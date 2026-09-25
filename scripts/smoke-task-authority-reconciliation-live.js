#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { createDisposableSmokeProfile, isWslWindowsBrowserExecutable, selectSmokeBrowserExecutable } from './lib/smoke-browser-fixture.js';
import { assert, closeSession, createSmokeContext, httpJson, parseJsonOutput, runCli } from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({ prefix: 'ab-task-authority-reconcile-', sessionPrefix: 'task-authority-reconcile' });
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = process.env.AGENT_BROWSER_TASK_AUTHORITY_RECONCILE_AGENT_BROWSER_CMD || 'agent-browser';
context.env.AGENT_BROWSER_TASK_AUTHORITY_DIR = join(context.tempHome, 'task-authority-ledger');
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');
let profilePath;
let ledgerSessionDir;
let ledgerPath;
let ledgerBackupPath;
let cleanupComplete = false;

try {
  const browserExecutable = resolveBrowserExecutable();
  profilePath = createDisposableSmokeProfile({ browserExecutable, defaultRoot: context.tempHome, windowsTempRoot: resolveWindowsTempRoot(browserExecutable), prefix: 'task-authority-reconcile-profile-' });
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = browserExecutable;
  context.env.AGENT_BROWSER_PROFILE = profilePath;
  const opened = await command(['--json', '--session', context.session, '--profile', profilePath, 'open', 'https://example.com/', '--headed', '--timeout', '20000'], 'open disposable public target');
  assert(opened.success === true, `open failed: ${JSON.stringify(opened)}`);
  const streamPort = await ensureStreamPort(context);
  const tabs = await httpJson(streamPort, 'POST', '/api/command', { action: 'tab_list', verbose: true });
  const target = tabs.data?.tabs?.find((tab) => tab.active === true) || tabs.data?.tabs?.[0];
  assert(target?.targetId && target.url === 'https://example.com/', 'public target identity drifted');

  const pendingIssue = await httpJson(streamPort, 'POST', '/api/service/task-authorities/issue', {
    sessionName: context.session,
    taskName: 'reconcile-public-read', serviceName: 'agent-browser-qa', agentName: 'codex',
    expectedTargetId: target.targetId, expectedUrl: target.url,
    issuer: { kind: 'operator', id: 'live-qa' }, approvalReference: 'plan-0109-live-proof',
    expiresInSeconds: 300, steps: [{ action: 'wait', evidenceBytes: 4096 }],
  });
  assert(pendingIssue.data?.confirmation_required === true, 'issuance bypassed confirmation');
  const issued = await decideAuthorityConfirmation(streamPort, pendingIssue.data.confirmation_id, 'task_authority_issue', 'confirm');
  const predecessor = issued.data?.result?.data;
  assert(predecessor?.state === 'active', `initial issuance failed: ${JSON.stringify(issued)}`);
  const predecessorId = predecessor.id;
  const predecessorStepId = predecessor.approvedPlan.steps[0].stepId;
  const commandId = `reconcile-stranded-read-${Date.now()}`;
  ledgerSessionDir = join(context.env.AGENT_BROWSER_TASK_AUTHORITY_DIR, context.session);
  ledgerPath = join(ledgerSessionDir, `${predecessorId}.json`);
  ledgerBackupPath = `${ledgerPath}.blocked`;
  const pendingRead = httpJson(streamPort, 'POST', '/api/command', {
    id: commandId, action: 'wait', timeout: 3000,
    taskName: predecessor.envelope.taskName, serviceName: predecessor.envelope.serviceName,
    agentName: predecessor.envelope.agentName, taskAuthority: predecessor.envelope,
    taskStepId: predecessorStepId, taskEvidenceBytes: 4096,
  });
  await waitFor(() => {
    if (!existsSync(ledgerPath)) return false;
    const ledger = JSON.parse(readFileSync(ledgerPath, 'utf8'));
    return ledger.stepReceipts?.some((receipt) => receipt.commandId === commandId && !receipt.outcome);
  }, 2500, 'durable admission receipt');
  renameSync(ledgerPath, ledgerBackupPath);
  mkdirSync(ledgerPath);
  const strandedResponse = await pendingRead;
  rmSync(ledgerPath, { recursive: true, force: true });
  renameSync(ledgerBackupPath, ledgerPath);
  assert(strandedResponse.success === false && /outcome finalization failed/i.test(strandedResponse.error || ''), `finalization failure did not fail closed: ${JSON.stringify(strandedResponse)}`);
  const strandedStatus = await httpJson(streamPort, 'GET', `/api/service/task-authorities/${predecessorId}?sessionName=${encodeURIComponent(context.session)}`);
  assert(strandedStatus.data?.usage?.outcomeSummary?.indeterminate === 1, 'stranded receipt was not indeterminate');

  const pendingReconcile = await httpJson(streamPort, 'POST', `/api/service/task-authorities/${predecessorId}/reconcile`, {
    sessionName: context.session,
    reconciliationId: 'p109-live-reconciliation-1', unresolvedStepId: predecessorStepId,
    taskName: predecessor.envelope.taskName, serviceName: predecessor.envelope.serviceName,
    agentName: predecessor.envelope.agentName, expectedTargetId: target.targetId,
    expectedUrl: target.url, issuer: { kind: 'operator', id: 'live-qa' },
    approvalReference: 'plan-0109-replacement', expiresInSeconds: 300,
    steps: [{ action: 'title', evidenceBytes: 4096 }],
  });
  assert(pendingReconcile.data?.confirmation_required === true, 'reconciliation bypassed confirmation');
  const reconciled = await decideAuthorityConfirmation(streamPort, pendingReconcile.data.confirmation_id, 'task_authority_reconcile', 'confirm');
  const receipt = reconciled.data?.result?.data;
  assert(receipt?.predecessor?.state === 'revoked', `predecessor was not revoked: ${JSON.stringify(receipt)}`);
  assert(receipt?.replacement?.state === 'active', 'replacement was not minted');
  assert(receipt?.lineage?.predecessorCommandId === commandId, 'replacement lost command lineage');
  assert(receipt?.replacement?.envelope?.lineage?.predecessorStepId === predecessorStepId, 'replacement envelope lost step lineage');
  assert(receipt?.replacement?.approvedPlan?.steps?.[0]?.stepId !== predecessorStepId, 'replacement replayed the consumed step ID');

  const oldReplay = await httpJson(streamPort, 'POST', '/api/command', {
    id: commandId, action: 'wait', timeout: 1,
    taskName: predecessor.envelope.taskName, serviceName: predecessor.envelope.serviceName,
    agentName: predecessor.envelope.agentName, taskAuthority: predecessor.envelope,
    taskStepId: predecessorStepId, taskEvidenceBytes: 4096,
  });
  assert(oldReplay.success === false && /revoked/.test(oldReplay.error || ''), 'consumed predecessor replay was not rejected');

  const replacement = receipt.replacement;
  const replacementRead = await httpJson(streamPort, 'POST', '/api/command', {
    id: `replacement-title-${Date.now()}`, action: 'title',
    taskName: replacement.envelope.taskName, serviceName: replacement.envelope.serviceName,
    agentName: replacement.envelope.agentName, taskAuthority: replacement.envelope,
    taskStepId: replacement.approvedPlan.steps[0].stepId, taskEvidenceBytes: 4096,
  });
  assert(replacementRead.success === true && /example domain/i.test(replacementRead.data?.title || ''), 'replacement read failed');
  const pendingDeny = await httpJson(streamPort, 'POST', `/api/service/task-authorities/${replacement.id}/revoke`, {
    sessionName: context.session,
    revokedBy: 'live-qa',
    reason: 'verify dashboard denial path',
  });
  assert(pendingDeny.data?.confirmation_required === true, 'revocation denial fixture bypassed confirmation');
  const denied = await decideAuthorityConfirmation(streamPort, pendingDeny.data.confirmation_id, 'task_authority_revoke', 'deny');
  assert(denied.data?.denied === true, `confirmation denial failed: ${JSON.stringify(denied)}`);
  const activeAfterDenial = await httpJson(streamPort, 'GET', `/api/service/task-authorities/${replacement.id}?sessionName=${encodeURIComponent(context.session)}`);
  assert(activeAfterDenial.data?.state === 'exhausted' || activeAfterDenial.data?.state === 'active', 'denial changed the replacement authority');
  assert((await readUrl()) === target.url, 'reconciliation changed the retained public target');
  const authorityCollection = await httpJson(streamPort, 'GET', `/api/service/task-authorities?sessionName=${encodeURIComponent(context.session)}`);
  assert(authorityCollection.data?.authorities?.some((authority) => authority.id === predecessorId), 'authority collection omitted the predecessor before dashboard rendering');
  assert(authorityCollection.data?.authorities?.some((authority) => authority.id === replacement.id), 'authority collection omitted the replacement before dashboard rendering');

  const dashboardUrl = `http://127.0.0.1:${streamPort}/service?view=service:authorities&port=${streamPort}`;
  const dashboardTab = await httpJson(streamPort, 'POST', '/api/browser/new-tab', { url: dashboardUrl });
  assert(dashboardTab.success === true, `dashboard tab failed: ${JSON.stringify(dashboardTab)}`);
  const dashboardAuth = await httpJson(streamPort, 'GET', '/api/dashboard-auth/status');
  assert(dashboardAuth.authenticated === false, 'disposable dashboard unexpectedly reused an authenticated session');
  await httpJson(streamPort, 'POST', '/api/browser/wait', { timeout: 1500 });
  const dashboardCredentials = readDisposableDashboardCredentials();
  await httpJson(streamPort, 'POST', '/api/browser/fill', { selector: 'input[autocomplete="username"]', value: dashboardCredentials.username });
  await httpJson(streamPort, 'POST', '/api/browser/fill', { selector: 'input[autocomplete="current-password"]', value: dashboardCredentials.password });
  await httpJson(streamPort, 'POST', '/api/browser/click', { selector: 'button[type="submit"]' });
  let renderedDashboard = '';
  await waitFor(async () => {
    await httpJson(streamPort, 'POST', '/api/browser/wait', { timeout: 500 });
    const dashboardSnapshot = await httpJson(streamPort, 'POST', '/api/browser/snapshot', { selector: 'main' });
    renderedDashboard = JSON.stringify(dashboardSnapshot);
    return renderedDashboard.includes(predecessorId) && renderedDashboard.includes(replacement.id);
  }, 12000, 'rendered dashboard authority lineage');
  assert(renderedDashboard.includes('Task authorities'), `dashboard did not render the Authorities workspace: ${renderedDashboard.slice(0, 4000)}`);
  assert(renderedDashboard.includes(predecessorId), `dashboard omitted predecessor authority evidence: ${renderedDashboard.slice(0, 6000)}`);
  assert(renderedDashboard.includes(replacement.id), `dashboard omitted replacement lineage evidence: ${renderedDashboard.slice(0, 6000)}`);

  await cleanup();
  console.log(JSON.stringify({
    schema: 'agent-browser.task-authority-reconciliation-smoke.v1', success: true,
    posture: { publicOnly: true, authenticatedProfileUsed: false, disposableLocalDashboardAuthUsed: true, pageMutationExecuted: false, promptSubmitted: false, cleanupComplete },
    evidence: { indeterminateReceiptObserved: true, reconciliationRequiredExactTargetConfirmation: true, httpConfirmationDecisionUsed: true, denialPreservedAuthority: true, dashboardRenderedAuthorityLineage: true, predecessorRevokedBeforeReplacement: true, predecessorStepAndCommandLineageBound: true, replacementStepIdFresh: true, predecessorReplayRejected: true, replacementExecutedOnce: true },
  }, null, 2));
} catch (error) {
  if (ledgerPath && existsSync(ledgerPath) && ledgerBackupPath && existsSync(ledgerBackupPath)) {
    rmSync(ledgerPath, { recursive: true, force: true });
    renameSync(ledgerBackupPath, ledgerPath);
  }
  await cleanup();
  console.error(error.stack || error.message);
  process.exitCode = 1;
}

async function command(args, label) {
  return parseJsonOutput((await runCli(context, args, 30000)).stdout, label);
}
async function decideAuthorityConfirmation(streamPort, confirmationId, expectedAction, decision) {
  return httpJson(streamPort, 'POST', '/api/service/task-authorities/confirmation', {
    sessionName: context.session,
    confirmationId,
    expectedAction,
    decision,
    decidedBy: { kind: 'operator', id: 'live-qa' },
  });
}
function readDisposableDashboardCredentials() {
  const path = join(context.tempHome, '.agent-browser', 'dashboard-auth.env');
  const values = new Map();
  for (const line of readFileSync(path, 'utf8').split(/\r?\n/)) {
    const match = line.match(/^([A-Z0-9_]+)="(.*)"$/);
    if (match) values.set(match[1], match[2].replaceAll('\\"', '"'));
  }
  const username = values.get('AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME');
  const password = values.get('AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD');
  assert(username && password, 'disposable dashboard bootstrap credential was unavailable');
  return { username, password };
}
async function readUrl() {
  return (await command(['--json', '--session', context.session, 'get', 'url'], 'read current URL')).data?.url;
}
async function waitFor(predicate, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`Timed out waiting for ${label}`);
}
function resolveBrowserExecutable() {
  const selected = selectSmokeBrowserExecutable({ configuredExecutable: process.env.AGENT_BROWSER_TASK_AUTHORITY_RECONCILE_BROWSER_EXECUTABLE });
  if (selected) { assert(existsSync(selected), `browser missing: ${selected}`); return selected; }
  const doctor = spawnSync(context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD, ['install', 'doctor', '--json'], { encoding: 'utf8', env: process.env });
  const executable = parseJsonOutput(doctor.stdout, 'agent-browser install doctor').data?.launchConfig?.executablePath;
  assert(executable && existsSync(executable), 'install doctor returned no usable browser executable');
  return executable;
}
function resolveWindowsTempRoot(browserExecutable) {
  if (!isWslWindowsBrowserExecutable(browserExecutable)) return null;
  const windowsTemp = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '[Console]::Out.Write([IO.Path]::GetTempPath())'], { encoding: 'utf8' }).trim();
  return execFileSync('wslpath', ['-u', windowsTemp], { encoding: 'utf8' }).trim();
}
async function cleanup() {
  if (cleanupComplete) return;
  cleanupComplete = true;
  await closeSession(context);
  if (profilePath) rmSync(profilePath, { recursive: true, force: true });
  context.cleanupTempHome();
}
