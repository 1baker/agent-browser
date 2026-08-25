#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  runLocalDashboardPublisherOrchestration,
} from './lib/local-dashboard-publisher-orchestration.js';
import {
  createLocalDashboardPublicationJournal,
} from './lib/local-dashboard-publication-journal.js';

const roots = new Set();

try {
  await runSuccessScenario();
  await runRetainedGuardSuccessScenario();
  await runRetainedGuardPreMutationFailureScenario();
  await runRetainedGuardPostHandoffFailureScenario();
  await runPreHandoffRollbackScenario();
  await runCommittedHandoffFailureScenario();
  await runCommittedHandoffReplacementMismatchScenario();
  await runRollbackRestartFailureScenario();
  await runBackupHashMismatchScenario();
  await runReplacementHashMismatchScenario();
  await runMissingBackupRollbackScenario();
  await runRestoreCopyFailureScenario();
  await runRestoreHashMismatchScenario();
  await runPreMutationFailureScenario();
  await runReplacementRecoveryScenario();
  await runPostInstallPreCheckpointRecoveryScenario();
  await runRolledBackRecoveryScenario();
  await runDiscoveredHandoffRecoveryScenario();
  await runAlreadyResumedRecoveryScenario();
  await runRetainedGuardRecoveryFailureScenario();
  await runUnverifiedRecoveryScenario();
  await runRecoverOnlyNoopScenario();
} finally {
  for (const root of roots) rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard publisher orchestration fixture passed');

async function runSuccessScenario() {
  const fixture = createFixture();
  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.deepEqual(fixture.actions, [
    'resolve-install',
    'guard-install',
    'build-dashboard',
    'build-runtime:debug',
    'resolve-built:debug',
    'built-exists',
    'hash:built',
    'service-status:before',
    'exists:installed',
    'hash:installed',
    'backup',
    'hash:backup',
    'hash:installed',
    'quiesce',
    'prepare-handoffs',
    'install-replacement',
    'hash:installed',
    'sync-references',
    'resume-handoffs',
    'restart:normal',
    'http-readiness',
    'manifest-readback',
    'browser-smoke',
    'service-status:after',
  ]);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(readFileSync(fixture.backupPath, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(statSync(fixture.backupPath).mode & 0o777, 0o751);
  assert.equal(fixture.report.backupPath, fixture.backupPath);
  assert.equal(fixture.report.restoredBackup, undefined);
  assert.deepEqual(fixture.report.referenceBinaries, [{ path: 'reference', synced: true }]);
  assert.equal(fixture.report.browserSmoke.status, 'passed');
  assert.equal(fixture.report.service.after.sequence, 2);
  assert.deepEqual(fixture.report.artifactEvidence, {
    built: { path: fixture.builtBin, sha256: fixture.replacementSha256 },
    source: { path: fixture.installBin, sha256: fixture.originalSha256 },
    backup: {
      path: fixture.backupPath,
      mode: 0o751,
      sha256: fixture.originalSha256,
      sourceSha256: fixture.originalSha256,
      sourceAfterSha256: fixture.originalSha256,
      verified: true,
    },
    replacement: {
      path: fixture.installBin,
      sourcePath: fixture.builtBin,
      expectedSha256: fixture.replacementSha256,
      actualSha256: fixture.replacementSha256,
      verified: true,
    },
    restoration: null,
  });
}

async function runRetainedGuardSuccessScenario() {
  const fixture = createFixture({ prepareHandoff: true, retainedExpectation: true });
  await runLocalDashboardPublisherOrchestration(fixture.input);

  const pre = fixture.actions.indexOf('retained-guard:pre_mutation');
  const build = fixture.actions.indexOf('build-dashboard');
  const backup = fixture.actions.indexOf('backup');
  const post = fixture.actions.indexOf('retained-guard:post_handoff');
  const resume = fixture.actions.indexOf('resume-handoffs');
  const final = fixture.actions.indexOf('retained-guard:final_readiness');
  const smoke = fixture.actions.indexOf('browser-smoke');
  assert.ok(pre >= 0 && pre < build, 'retained guard must run before any build');
  assert.ok(pre < backup, 'retained guard must run before backup or quiescence');
  assert.ok(post > resume, 'retained guard must rerun after handoff resume');
  assert.ok(final > smoke, 'retained guard must rerun at final readiness');
  assert.equal(fixture.report.retainedBrowserExpectation.before.verified, true);
  assert.equal(fixture.report.retainedBrowserExpectation.afterHandoff.verified, true);
  assert.equal(fixture.report.retainedBrowserExpectation.final.verified, true);
  assert.equal(
    fixture.publicationJournal.read().retainedBrowserExpectation.final.verified,
    true,
  );
}

async function runRetainedGuardPreMutationFailureScenario() {
  const fixture = createFixture({
    retainedExpectation: true,
    retainedGuardFaultAt: 'pre_mutation',
  });
  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:pre_mutation',
  );

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('build-runtime:debug'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.includes('quiesce'), false);
  assert.equal(fixture.actions.includes('prepare-handoffs'), false);
  assert.equal(fixture.publicationJournal.read(), null);
}

async function runRetainedGuardPostHandoffFailureScenario() {
  const fixture = createFixture({
    prepareHandoff: true,
    retainedExpectation: true,
    retainedGuardFaultAt: 'post_handoff',
  });
  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:post_handoff',
  );

  assert.equal(fixture.report.handoffs.prepared.length, 1);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.includes('retained-guard:final_readiness'), false);
  assert.equal(
    fixture.publicationJournal.read().phase,
    'publication_failed_replacement_retained',
  );
}

async function runPreHandoffRollbackScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness' });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(fixture.report.restoredBackup, true);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    path: fixture.installBin,
    sourcePath: fixture.backupPath,
    expectedSha256: fixture.originalSha256,
    actualSha256: fixture.originalSha256,
    status: 'verified',
    verified: true,
    error: null,
  });
  assert.equal(fixture.actions.filter((action) => action === 'install-replacement').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restore-backup').length, 1);
  assert.ok(
    fixture.actions.indexOf('restart:rollback') > fixture.actions.indexOf('restore-backup'),
    'rollback restart must occur after the original binary is restored',
  );
  assert.equal(fixture.actions.at(-1), 'service-status:after');
}

async function runCommittedHandoffFailureScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', prepareHandoff: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(readFileSync(fixture.backupPath, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(fixture.report.restoredBackup, undefined);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
  assert.equal(fixture.report.handoffs.prepared.length, 1);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    status: 'skipped',
    reason: 'browser_handoff_started',
  });
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
}

async function runCommittedHandoffReplacementMismatchScenario() {
  const fixture = createFixture({ corruptReplacement: true, prepareHandoff: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed replacement hash mismatch:');

  assert.equal(fixture.report.artifactEvidence.replacement.verified, false);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    status: 'skipped',
    reason: 'browser_handoff_started',
  });
  assert.equal(fixture.report.restoreRestartArtifact.verified, false);
  assert.equal(
    fixture.report.restoreRestartSkipped,
    'installed_artifact_unverified_after_publication_failure',
  );
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 0);
}

async function runRollbackRestartFailureScenario() {
  const fixture = createFixture({
    faultAt: 'http-readiness',
    rollbackRestartFault: 'fault:rollback-restart',
  });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.restoredBackup, true);
  assert.equal(fixture.report.restoreRestartError, 'fault:rollback-restart');
  assert.equal(fixture.actions.at(-1), 'service-status:after');
}

async function runBackupHashMismatchScenario() {
  const fixture = createFixture({ corruptBackup: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed binary backup hash mismatch:');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.artifactEvidence.backup.verified, false);
  assert.equal(fixture.actions.includes('quiesce'), false);
  assert.equal(fixture.actions.includes('install-replacement'), false);
  assert.equal(fixture.actions.includes('restart:rollback'), false);
}

async function runReplacementHashMismatchScenario() {
  const fixture = createFixture({ corruptReplacement: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed replacement hash mismatch:');

  assert.equal(fixture.report.artifactEvidence.replacement.verified, false);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'verified');
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.restoredBackup, true);
}

async function runMissingBackupRollbackScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', removeBackupBeforeRollback: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.match(fixture.report.restoreError, /Verified backup is no longer available/);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.error, fixture.report.restoreError);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runRestoreCopyFailureScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', restoreCopyFault: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.equal(fixture.report.restoreError, 'fault:restore-copy');
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.error, 'fault:restore-copy');
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
  assert.equal(fixture.actions.filter((action) => action === 'restore-backup').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runRestoreHashMismatchScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', corruptRestore: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.match(fixture.report.restoreError, /Restored binary hash mismatch/);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.verified, false);
  assert.notEqual(
    fixture.report.artifactEvidence.restoration.actualSha256,
    fixture.report.artifactEvidence.restoration.expectedSha256,
  );
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'corrupted-restoration\n');
  assert.equal(fixture.report.restoreRestartArtifact.verified, false);
  assert.equal(
    fixture.report.restoreRestartSkipped,
    'installed_artifact_unverified_after_publication_failure',
  );
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 0);
}

async function runPreMutationFailureScenario() {
  const fixture = createFixture({ faultAt: 'build-runtime' });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:build-runtime');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(existsSync(fixture.backupPath), false);
  assert.deepEqual(fixture.actions, [
    'resolve-install',
    'guard-install',
    'build-dashboard',
    'build-runtime:debug',
  ]);
  assert.equal(fixture.report.service.before, null);
  assert.equal(fixture.report.service.after, null);
}

async function runReplacementRecoveryScenario() {
  const fixture = createFixture();
  fixture.input.options.recoverOnly = true;
  seedIncompleteJournal(fixture, { phase: 'replacement_installed', installed: 'replacement' });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_ready');
  assert.equal(fixture.publicationJournal.read().terminal, true);
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
}

async function runRecoverOnlyNoopScenario() {
  const fixture = createFixture();
  fixture.input.options.recoverOnly = true;
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.deepEqual(fixture.report.recovery, {
    transactionId: null,
    result: 'nothing_to_recover',
    terminalPhase: null,
  });
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.some((action) => action.startsWith('restart:')), false);
  assert.equal(existsSync(fixture.installBin), true);
}

async function runPostInstallPreCheckpointRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, {
    phase: 'handoff_prepared',
    installed: 'replacement',
    replacementEvidence: false,
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.equal(fixture.report.artifactEvidence.replacement.verified, true);
  assert.equal(fixture.report.artifactEvidence.replacement.recoveredFromBuiltEvidence, true);
  assert.equal(fixture.actions.filter((action) => action === 'sync-references').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_ready');
}

async function runRolledBackRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, { phase: 'quiesced', installed: 'source' });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_rolled_back');
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_rolled_back');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
}

async function runDiscoveredHandoffRecoveryScenario() {
  const handoff = fixtureHandoff();
  const fixture = createFixture({ discoveredHandoffs: [handoff] });
  seedIncompleteJournal(fixture, {
    phase: 'handoff_admitted',
    installed: 'source',
    candidateSessions: [handoff.sessionName],
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_rolled_back');
  assert.deepEqual(fixture.report.handoffs.prepared, [handoff]);
  assert.equal(fixture.actions.filter((action) => action === 'resume-handoffs').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runAlreadyResumedRecoveryScenario() {
  const handoff = fixtureHandoff();
  const resumed = { ...handoff, alreadyResumed: true };
  const fixture = createFixture();
  seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    handoffs: [handoff],
    resumedHandoffs: [resumed],
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.deepEqual(fixture.report.handoffs.resumed, [resumed]);
  assert.equal(fixture.actions.includes('resume-handoffs'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
}

async function runRetainedGuardRecoveryFailureScenario() {
  const handoff = fixtureHandoff();
  const fixture = createFixture({ retainedGuardFaultAt: 'recovery_post_handoff' });
  seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    handoffs: [handoff],
    retainedBrowserExpectation: fixtureRetainedExpectationRecord(),
  });
  fixture.actions.length = 0;

  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:recovery_post_handoff',
  );

  assert.equal(fixture.actions.includes('resume-handoffs'), true);
  assert.equal(fixture.actions.includes('restart:normal'), false);
  assert.equal(fixture.publicationJournal.read().phase, 'recovery_blocked');
  assert.equal(
    fixture.publicationJournal.read().recoveryError,
    'retained_browser_expectation_failed',
  );
}

async function runUnverifiedRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, { phase: 'replacement_installed', installed: 'replacement' });
  writeFileSync(fixture.installBin, 'unknown-runtime\n');
  fixture.actions.length = 0;

  await assertRejectsWithOriginalFault(
    fixture.input,
    'Publication recovery found an unverified installed binary:',
  );

  assert.equal(fixture.publicationJournal.read().phase, 'recovery_blocked');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.some((action) => action.startsWith('restart:')), false);
}

function createFixture({
  corruptBackup = false,
  corruptReplacement = false,
  corruptRestore = false,
  faultAt = null,
  prepareHandoff = false,
  removeBackupBeforeRollback = false,
  restoreCopyFault = false,
  rollbackRestartFault = null,
  discoveredHandoffs = [],
  retainedExpectation = false,
  retainedGuardFaultAt = null,
} = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-dashboard-publisher-orchestration-'));
  roots.add(root);
  const installBin = join(root, 'installed-agent-browser');
  const builtBin = join(root, 'built-agent-browser');
  const backupPath = join(root, 'installed-agent-browser.backup');
  const journalPath = join(root, 'publication', 'local-dashboard.json');
  writeFileSync(installBin, 'original-runtime\n', { mode: 0o751 });
  writeFileSync(builtBin, 'replacement-runtime\n', { mode: 0o755 });
  const originalSha256 = hashFile(installBin);
  const replacementSha256 = hashFile(builtBin);

  const actions = [];
  const report = createReport();
  const publicationJournal = createLocalDashboardPublicationJournal({ journalPath });
  let serviceStatusCount = 0;
  const act = (name, value) => {
    actions.push(name);
    if (faultAt === value) throw new Error(`fault:${value}`);
  };

  const adapters = {
    resolveInstallBin: () => {
      act('resolve-install', 'resolve-install');
      return installBin;
    },
    guardInstallPath: () => act('guard-install', 'guard-install'),
    buildDashboard: () => act('build-dashboard', 'build-dashboard'),
    buildRuntime: ({ release }) => act(`build-runtime:${release ? 'release' : 'debug'}`, 'build-runtime'),
    resolveBuiltBin: ({ release }) => {
      act(`resolve-built:${release ? 'release' : 'debug'}`, 'resolve-built');
      return builtBin;
    },
    builtBinaryExists: (path) => {
      act('built-exists', 'built-exists');
      return existsSync(path);
    },
    pathExists: (path) => {
      actions.push(`exists:${path === backupPath ? 'backup' : 'installed'}`);
      return existsSync(path);
    },
    sha256File: (path) => {
      actions.push(`hash:${path === builtBin ? 'built' : path === backupPath ? 'backup' : 'installed'}`);
      return hashFile(path);
    },
    serviceStatus: () => {
      serviceStatusCount += 1;
      actions.push(`service-status:${serviceStatusCount === 1 ? 'before' : 'after'}`);
      return { sequence: serviceStatusCount };
    },
    backupInstalledBinary: () => {
      act('backup', 'backup');
      copyFileSync(installBin, backupPath);
      const mode = statSync(installBin).mode & 0o777;
      chmodSync(backupPath, mode);
      if (corruptBackup) writeFileSync(backupPath, 'corrupted-backup\n');
      return { path: backupPath, mode };
    },
    quiesceDashboardForRuntimeHandoff: () => act('quiesce', 'quiesce'),
    prepareRuntimeHandoffs: () => {
      act('prepare-handoffs', 'prepare-handoffs');
      if (prepareHandoff) report.handoffs.prepared.push({ sessionName: 'retained-fixture' });
    },
    installBinaryAtomically: (source, target, mode) => {
      const restoring = source === backupPath;
      act(restoring ? 'restore-backup' : 'install-replacement', restoring ? 'restore-backup' : 'install-replacement');
      if (restoring && restoreCopyFault) throw new Error('fault:restore-copy');
      const staged = join(root, restoring ? 'restore.next' : 'replacement.next');
      copyFileSync(source, staged);
      if (restoring && corruptRestore) writeFileSync(staged, 'corrupted-restoration\n');
      if (!restoring && corruptReplacement) writeFileSync(staged, 'corrupted-replacement\n');
      chmodSync(staged, mode);
      renameSync(staged, target);
    },
    syncReferenceBinaries: () => {
      act('sync-references', 'sync-references');
      return [{ path: 'reference', synced: true }];
    },
    resumeRuntimeHandoffs: () => {
      act('resume-handoffs', 'resume-handoffs');
      for (const handoff of report.handoffs.prepared) {
        report.handoffs.resumed.push({ ...handoff });
      }
    },
    restartOrStartDashboard: (_path, { restoring }) => {
      actions.push(`restart:${restoring ? 'rollback' : 'normal'}`);
      if (restoring && rollbackRestartFault) throw new Error(rollbackRestartFault);
      if (!restoring && faultAt === 'restart-normal') throw new Error('fault:restart-normal');
    },
    runHttpReadinessSmoke: () => {
      if (removeBackupBeforeRollback) rmSync(backupPath, { force: true });
      act('http-readiness', 'http-readiness');
      return { runtimeManifest: { schemaVersion: 'fixture.v1' } };
    },
    verifyRuntimeManifestReadback: (_path, manifest) => {
      act('manifest-readback', 'manifest-readback');
      return { ...manifest, verified: true };
    },
    verifyRetainedBrowserExpectation: (_path, { expectation, stage }) => {
      actions.push(`retained-guard:${stage}`);
      if (retainedGuardFaultAt === stage) {
        throw new Error(`fault:retained-guard:${stage}`);
      }
      return {
        required: true,
        verified: true,
        stage,
        reason: 'retained_browser_exact_match',
        expected: expectation,
        observed: {
          sessionName: expectation.sessionName,
          browserId: 'session:retained-fixture',
          browserPid: 5678,
          cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
          profileId: 'fixture-profile',
          health: 'ready',
          targetId: 'target-fixture',
          url: 'https://example.test/conversation',
          title: 'Fixture conversation',
          cdpTargetCount: 1,
        },
      };
    },
    runBrowserSmokeDiagnostic: () => {
      act('browser-smoke', 'browser-smoke');
      return { status: 'passed' };
    },
    runtimeSessionNames: () => [],
    discoverPreparedRuntimeHandoffs: (candidateSessions) => discoveredHandoffs
      .filter((handoff) => candidateSessions.includes(handoff.sessionName))
      .map((handoff) => ({ ...handoff })),
    publicationJournal,
  };

  return {
    actions,
    backupPath,
    builtBin,
    installBin,
    journalPath,
    originalSha256,
    replacementSha256,
    publicationJournal,
    input: {
      adapters,
      options: {
        release: false,
        recoverOnly: false,
        requireBrowserSmoke: false,
        skipSmoke: false,
        smokeBrowser: true,
        syncReferenceBinaries: true,
        retainedBrowserExpectation: retainedExpectation
          ? {
            sessionName: 'retained-fixture',
            targetId: 'target-fixture',
            url: 'https://example.test/conversation',
          }
          : null,
      },
      report,
    },
    report,
  };
}

function createReport() {
  return {
    installBin: null,
    builtBin: null,
    backupPath: null,
    service: { before: null, after: null },
    smoke: null,
    browserSmoke: { status: 'pending' },
    runtimeManifest: null,
    artifactEvidence: {
      built: null,
      source: null,
      backup: null,
      replacement: null,
      restoration: null,
    },
    referenceBinaries: [],
    handoffs: { prepared: [], resumed: [] },
  };
}

async function assertRejectsWithOriginalFault(input, message) {
  await assert.rejects(
    () => runLocalDashboardPublisherOrchestration(input),
    (error) => error instanceof Error && error.message.startsWith(message),
  );
}

function hashFile(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function seedIncompleteJournal(fixture, {
  candidateSessions = [],
  handoffs = [],
  installed,
  phase,
  replacementEvidence = true,
  resumedHandoffs = [],
  retainedBrowserExpectation = null,
}) {
  copyFileSync(fixture.installBin, fixture.backupPath);
  chmodSync(fixture.backupPath, 0o751);
  if (installed === 'replacement') {
    copyFileSync(fixture.builtBin, fixture.installBin);
    chmodSync(fixture.installBin, 0o751);
  }
  const artifactEvidence = {
    built: { path: fixture.builtBin, sha256: fixture.replacementSha256 },
    source: { path: fixture.installBin, sha256: fixture.originalSha256 },
    backup: {
      path: fixture.backupPath,
      mode: 0o751,
      sha256: fixture.originalSha256,
      sourceSha256: fixture.originalSha256,
      sourceAfterSha256: fixture.originalSha256,
      verified: true,
    },
    replacement: replacementEvidence
      ? {
        path: fixture.installBin,
        sourcePath: fixture.builtBin,
        expectedSha256: fixture.replacementSha256,
        actualSha256: fixture.replacementSha256,
        verified: true,
      }
      : null,
    restoration: null,
  };
  fixture.publicationJournal.acquire();
  try {
    let record = fixture.publicationJournal.create({
      installBin: fixture.installBin,
      builtBin: fixture.builtBin,
      backupPath: fixture.backupPath,
      installMode: 0o751,
      artifactEvidence,
      candidateSessions,
      handoffs,
      resumedHandoffs,
      retainedBrowserExpectation,
      dashboardQuiesceAdmitted: phase !== 'prepared',
      dashboardQuiesced: phase !== 'prepared',
      failure: null,
    });
    if (phase !== 'prepared') {
      record = fixture.publicationJournal.commit(record, phase, {
        handoffs,
        resumedHandoffs,
      });
    }
    return record;
  } finally {
    fixture.publicationJournal.release();
  }
}

function fixtureHandoff() {
  return {
    sessionName: 'retained-fixture',
    daemonPid: 1234,
    browserPid: 5678,
    cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
    runtimeProfile: 'fixture-profile',
    handoffPath: '/tmp/retained-fixture.handoff.json',
  };
}

function fixtureRetainedExpectationRecord() {
  const observed = {
    sessionName: 'retained-fixture',
    browserId: 'session:retained-fixture',
    browserPid: 5678,
    cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
    profileId: 'fixture-profile',
    health: 'ready',
    targetId: 'target-fixture',
    url: 'https://example.test/conversation',
    title: 'Fixture conversation',
    cdpTargetCount: 1,
  };
  return {
    required: true,
    pinned: {
      sessionName: observed.sessionName,
      browserId: observed.browserId,
      browserPid: observed.browserPid,
      cdpUrl: observed.cdpUrl,
      profileId: observed.profileId,
      targetId: observed.targetId,
      url: observed.url,
    },
    before: {
      required: true,
      verified: true,
      stage: 'pre_mutation',
      reason: 'retained_browser_exact_match',
      expected: {
        sessionName: observed.sessionName,
        targetId: observed.targetId,
        url: observed.url,
      },
      observed,
    },
    afterHandoff: null,
    final: null,
  };
}
