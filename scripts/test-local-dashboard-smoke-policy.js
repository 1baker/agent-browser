#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  classifyLocalDashboardBrowserSmokeFailure,
  classifyStandaloneDashboardProcess,
  evaluateLocalDashboardBrowserSmokeResult,
} from './lib/local-dashboard-smoke-policy.js';

const publisher = readFileSync('scripts/publish-local-dashboard-runtime.js', 'utf8');
const publisherLifecycle = readFileSync('scripts/lib/local-dashboard-publisher-lifecycle.js', 'utf8');
const publisherOrchestration = readFileSync('scripts/lib/local-dashboard-publisher-orchestration.js', 'utf8');
const publicationJournal = readFileSync('scripts/lib/local-dashboard-publication-journal.js', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));

const wslLaunch = classifyLocalDashboardBrowserSmokeFailure({
  phase: 'open dashboard url',
  error: 'Chrome exited early (exit code: 21) without exposing DevTools',
});
assert.deepEqual(
  {
    status: wslLaunch.status,
    classification: wslLaunch.classification,
    advisory: wslLaunch.advisory,
    fatal: wslLaunch.fatal,
  },
  {
    status: 'unavailable',
    classification: 'browser_launch_unavailable',
    advisory: true,
    fatal: false,
  },
  'known pre-render WSL Chrome launch failure must be advisory by default',
);

const requiredWslLaunch = classifyLocalDashboardBrowserSmokeFailure({
  phase: 'open dashboard url',
  error: 'Chrome exited early without exposing DevTools',
  required: true,
});
assert.equal(requiredWslLaunch.fatal, true, 'required browser smoke must fail on launch unavailability');
assert.equal(requiredWslLaunch.advisory, false);

const publisherAdvisory = evaluateLocalDashboardBrowserSmokeResult({
  processStatus: 1,
  parsed: {
    success: false,
    phase: 'open dashboard url',
    error: 'Chrome exited early (exit code: 21) without exposing DevTools',
  },
});
assert.equal(publisherAdvisory.status, 'unavailable');
assert.equal(publisherAdvisory.fatal, false);

const publisherPass = evaluateLocalDashboardBrowserSmokeResult({
  processStatus: 0,
  parsed: { success: true, browser: { smokeUrl: 'http://127.0.0.1:4848/' } },
});
assert.equal(publisherPass.status, 'passed');
assert.equal(publisherPass.evidence.smokeUrl, 'http://127.0.0.1:4848/');

assert.deepEqual(
  classifyStandaloneDashboardProcess({
    pid: 123,
    running: true,
    processUid: 1000,
    currentUid: 1000,
    command: '/home/operator/.local/bin/agent-browser',
    dashboardMode: '1',
  }),
  { state: 'owned_dashboard', safeToRetire: true, safeToRemovePidFile: false },
);
assert.equal(
  classifyStandaloneDashboardProcess({
    pid: 124,
    running: true,
    processUid: 1000,
    currentUid: 1000,
    command: '/home/operator/bin/agent-browser-linux-x64',
    dashboardMode: '1',
  }).safeToRetire,
  true,
  'packaged platform binary names must retain exact dashboard ownership',
);
for (const mismatch of [
  { processUid: 1001, command: '/home/operator/.local/bin/agent-browser', dashboardMode: '1' },
  { processUid: 1000, command: '/usr/bin/node', dashboardMode: '1' },
  { processUid: 1000, command: '/home/operator/.local/bin/agent-browser', dashboardMode: null },
]) {
  const identity = classifyStandaloneDashboardProcess({
    pid: 123,
    running: true,
    currentUid: 1000,
    ...mismatch,
  });
  assert.equal(identity.state, 'identity_mismatch');
  assert.equal(identity.safeToRetire, false);
}
assert.equal(
  classifyStandaloneDashboardProcess({ pid: 123, running: false }).safeToRemovePidFile,
  true,
  'stale dashboard PID metadata may be removed without signaling a process',
);

for (const failure of [
  { phase: 'fetch runtime manifest', error: 'connection reset' },
  { phase: 'fetch dashboard chunk /_next/chunk.js', error: 'marker missing' },
  { phase: 'verify dashboard login', error: 'Chrome exited early without exposing DevTools' },
  { phase: 'wait for app chrome 20', error: 'Dashboard browser smoke did not see the expected app chrome' },
  { phase: 'read workspace detail', error: 'Workspace tab did not expose dense selected-workspace detail' },
]) {
  const disposition = classifyLocalDashboardBrowserSmokeFailure(failure);
  assert.equal(disposition.fatal, true, `${failure.phase} must remain fatal`);
  assert.equal(disposition.classification, 'browser_validation_failed');
}

assert.match(
  publisherOrchestration,
  /report\.smoke = await adapters\.runHttpReadinessSmoke\(installBin\)[\s\S]*adapters\.verifyRuntimeManifestReadback\([\s\S]*report\.smoke\.runtimeManifest[\s\S]*report\.browserSmoke = await adapters\.runBrowserSmokeDiagnostic\(installBin\)/,
  'publisher must commit HTTP and manifest readiness before optional browser diagnostics',
);
assert.match(
  publisher,
  /requireBrowserSmoke: false[\s\S]*arg === '--require-browser-smoke'[\s\S]*evaluateLocalDashboardBrowserSmokeResult/,
  'publisher must expose explicit required browser-smoke mode and use the classifier',
);
assert.match(
  publisher,
  /quiesceStandaloneDashboardForRuntimeHandoff\([\s\S]*runtimeSocketDir: runtimeSocketDir\(\)[\s\S]*service: report\.service/,
  'publisher must route standalone quiescence through the tested lifecycle module',
);
assert.match(
  publisherLifecycle,
  /classifyStandaloneDashboardProcess[\s\S]*process\.kill\(pid, 'SIGTERM'\)[\s\S]*resumeOwnedStandalone = service\.quiesced[\s\S]*restart-standalone-after-restore/,
  'publisher must restart the exact standalone dashboard that it quiesced without requiring start-if-missing',
);
assert.match(
  publisher,
  /artifactEvidence: \{[\s\S]*built: null,[\s\S]*source: null,[\s\S]*backup: null,[\s\S]*replacement: null,[\s\S]*restoration: null,[\s\S]*pathExists: existsSync,[\s\S]*sha256File,/,
  'publisher must expose artifact evidence and wire production existence and SHA-256 adapters',
);
assert.match(
  publisherOrchestration,
  /backupSha256 === sourceSha256[\s\S]*installedReplacementSha256 === builtSha256[\s\S]*Restored binary hash mismatch[\s\S]*report\.restoreError[\s\S]*installed_artifact_unverified_after_publication_failure/,
  'publisher orchestration must verify backup, replacement, and restoration hashes without hiding restore failure',
);
assert.match(
  publisherOrchestration,
  /publicationJournal\.acquire\(\)[\s\S]*recoverIncompletePublication[\s\S]*quiesce_admitted[\s\S]*handoff_admitted[\s\S]*replacement_installed[\s\S]*handoffs_resumed[\s\S]*dashboard_restarted[\s\S]*recovered_ready/,
  'publisher orchestration must durably checkpoint and recover every mutation boundary',
);
assert.match(
  publicationJournal,
  /LOCAL_DASHBOARD_PUBLICATION_SCHEMA[\s\S]*journal revision conflict[\s\S]*openSync\(staged, 'wx', 0o600\)[\s\S]*fsyncSync\(descriptor\)[\s\S]*renameSync\(staged, path\)/,
  'publication journal must use secured atomic fsync writes and optimistic revision checks',
);
assert.match(
  publicationJournal,
  /inspectLocalDashboardPublicationJournal[\s\S]*journal\.lockStatus\(\)[\s\S]*classifyInstalledArtifact[\s\S]*wait_for_active_publisher[\s\S]*investigate_installed_artifact[\s\S]*recover_only/,
  'read-only journal status must classify lock and installed artifact evidence into exact next actions',
);
assert.match(
  publisher,
  /arg === '--journal-status'[\s\S]*arg === '--recover-only'[\s\S]*if \(options\.journalStatus\)[\s\S]*inspectLocalDashboardPublicationJournal[\s\S]*runLocalDashboardPublisherOrchestration/,
  'journal status must return before publisher orchestration can acquire a lock or build',
);
assert.match(
  publisherOrchestration,
  /if \(options\.recoverOnly\)[\s\S]*result: 'nothing_to_recover'[\s\S]*return;[\s\S]*adapters\.buildDashboard\(\)/,
  'recovery-only must return without falling through to a new build',
);
assert.match(
  publisher,
  /!\['closed', 'not_started'\]\.includes\(browser\.health\)[\s\S]*alreadyResumed: true[\s\S]*function discoverPreparedRuntimeHandoffs[\s\S]*\.handoff\.json[\s\S]*descriptor\.schemaVersion \?\? descriptor\.schema_version/,
  'production recovery must discover exact handoff descriptors and reconcile already-resumed sessions',
);
assert.match(
  publisher,
  /serviceBrowserForSession\([\s\S]*prepared\.sessionName,[\s\S]*prepared,[\s\S]*expectedBrowser\.browserPid[\s\S]*browser\?\.pid == null[\s\S]*browser\?\.pid === expectedBrowser\.browserPid[\s\S]*expectedBrowser\.cdpUrl[\s\S]*browser\?\.cdpEndpoint === expectedBrowser\.cdpUrl/,
  'handoff recovery must recognize an externally adopted browser by exact PID and CDP identity when its stable browser id differs from the daemon session',
);
assert.equal(
  packageJson.scripts['test:local-dashboard-smoke-policy'],
  'node scripts/test-local-dashboard-smoke-policy.js',
  'package.json must expose the focused smoke policy regression',
);
assert.equal(
  packageJson.scripts['test:local-dashboard-publisher-lifecycle'],
  'node scripts/test-local-dashboard-publisher-lifecycle.js',
  'package.json must expose the isolated publisher lifecycle fixture',
);
assert.equal(
  packageJson.scripts['test:local-dashboard-publisher-orchestration'],
  'node scripts/test-local-dashboard-publisher-orchestration.js',
  'package.json must expose the isolated publisher orchestration fixture',
);
assert.equal(
  packageJson.scripts['test:local-dashboard-publication-journal'],
  'node scripts/test-local-dashboard-publication-journal.js',
  'package.json must expose the durable publication journal fixture',
);
assert.equal(
  packageJson.scripts['test:local-dashboard-publication-operations'],
  'node scripts/test-local-dashboard-publication-operations.js',
  'package.json must expose the status and recovery-only authorization fixture',
);
assert.equal(
  packageJson.scripts['status:local-dashboard-publication'],
  'node scripts/publish-local-dashboard-runtime.js --journal-status --json',
  'package.json must expose read-only publication status',
);
assert.equal(
  packageJson.scripts['recover:local-dashboard-publication'],
  'node scripts/publish-local-dashboard-runtime.js --recover-only',
  'package.json must expose explicit recovery-only operation',
);

console.log('Local dashboard smoke policy tests passed');
