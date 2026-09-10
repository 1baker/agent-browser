#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { createDisposableSmokeProfile, isWslWindowsBrowserExecutable, selectSmokeBrowserExecutable } from './lib/smoke-browser-fixture.js';
import { assert, closeSession, createSmokeContext, httpJson, parseJsonOutput, runCli } from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({ prefix: 'ab-broker-task-authority-', sessionPrefix: 'broker-task-authority' });
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = process.env.AGENT_BROWSER_BROKER_AUTHORITY_AGENT_BROWSER_CMD || 'agent-browser';
context.env.AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS = '0';
context.env.AGENT_BROWSER_TASK_AUTHORITY_DIR = join(context.tempHome, 'task-authority-ledger');
let profilePath;
let cleanupComplete = false;

try {
  const browserExecutable = resolveBrowserExecutable();
  profilePath = createDisposableSmokeProfile({ browserExecutable, defaultRoot: context.tempHome, windowsTempRoot: resolveWindowsTempRoot(browserExecutable), prefix: 'broker-task-authority-profile-' });
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = browserExecutable;
  context.env.AGENT_BROWSER_PROFILE = profilePath;
  const browserArgs = String(process.env.AGENT_BROWSER_BROKER_AUTHORITY_BROWSER_ARGS || '').trim();
  const opened = await command(['--json', '--session', context.session, '--profile', profilePath, ...(browserArgs ? ['--args', browserArgs] : []), 'open', 'https://example.com/', '--headed', '--timeout', '20000'], 'open retained public target');
  assert(opened.success === true, `open failed: ${JSON.stringify(opened)}`);
  let streamPort = await ensureStreamPort(context);
  const tabs = await httpJson(streamPort, 'POST', '/api/command', { action: 'tab_list', verbose: true });
  const target = tabs.data?.tabs?.find((tab) => tab.active === true) || tabs.data?.tabs?.[0];
  assert(target?.targetId && target.url === 'https://example.com/', `target drifted: ${JSON.stringify(tabs)}`);

  const pendingIssue = await httpJson(streamPort, 'POST', '/api/service/task-authorities/issue', {
    sessionName: context.session,
    taskName: 'inspect-public-example', serviceName: 'agent-browser-qa', agentName: 'codex',
    expectedTargetId: target.targetId, expectedUrl: target.url,
    issuer: { kind: 'operator', id: 'live-qa' }, approvalReference: 'plan-0108-live-proof',
    expiresInSeconds: 300, steps: [
      { action: 'title', evidenceBytes: 4096 },
      { action: 'gettext', evidenceBytes: 4096 },
      { action: 'url', evidenceBytes: 4096 },
      { action: 'title', evidenceBytes: 4096 },
    ],
  });
  assert(pendingIssue.data?.confirmation_required === true, 'issuance bypassed confirmation');
  assert(pendingIssue.data?.consequenceClass === 'control_plane', 'issuance class drifted');
  assert(pendingIssue.data?.durable === true && /^[a-f0-9]{64}$/.test(pendingIssue.data?.requestSha256 || ''), 'issuance omitted durable request evidence');
  assert(await readUrl() === target.url, 'pending issuance changed the target');
  await command(['--json', '--session', context.session, 'handoff', 'prepare'], 'prepare pending-confirmation daemon handoff');
  await command(['--json', '--session', context.session, 'handoff', 'resume'], 'resume pending-confirmation daemon handoff');
  streamPort = await ensureStreamPort(context);
  const afterPendingRestart = await httpJson(streamPort, 'GET', `/api/service/task-authorities?sessionName=${encodeURIComponent(context.session)}`);
  const retainedConfirmation = afterPendingRestart.data?.confirmationStatus?.confirmations?.find((record) => record.confirmationId === pendingIssue.data.confirmation_id);
  assert(retainedConfirmation?.state === 'pending' && retainedConfirmation.requestSha256 === pendingIssue.data.requestSha256, `pending confirmation did not survive restart: ${JSON.stringify(afterPendingRestart)}`);
  assert(afterPendingRestart.data?.count === 0, 'pending confirmation executed during restart');
  const issued = await decideAuthorityConfirmation(streamPort, pendingIssue.data.confirmation_id, 'task_authority_issue', 'confirm');
  const status = issued.data?.result?.data;
  assert(status?.state === 'active', `issuance failed: ${JSON.stringify(issued)}`);
  const authorityId = status.id;
  const envelope = status.envelope;
  assert(authorityId && envelope?.allowedActions?.join(',') === 'gettext,title,url', 'action scope was not minimal');
  assert(envelope.allowedOrigins?.join(',') === 'https://example.com', 'origin scope was not minimal');
  assert(envelope.evidenceBudget?.maxActions === 4 && envelope.evidenceBudget?.maxEvidenceBytes === 16384, 'budgets were not plan-derived');
  const steps = status.approvedPlan?.steps;
  assert(Array.isArray(steps) && steps.length === 4 && steps[0].stepId !== steps[3].stepId, `ordered plan IDs were not broker-derived: ${JSON.stringify(status)}`);

  const beforeStatusUrl = await readUrl();
  const readStatus = await httpJson(streamPort, 'GET', `/api/service/task-authorities/${authorityId}?sessionName=${encodeURIComponent(context.session)}`);
  assert(readStatus.data?.issuer?.id === 'live-qa' && readStatus.data?.approvalReference === 'plan-0108-live-proof', 'status omitted provenance');
  assert(await readUrl() === beforeStatusUrl, 'read-only status changed the target');
  const authorizedCommand = (id, action, taskStepId) => ({
    id, action, taskName: envelope.taskName, serviceName: envelope.serviceName,
    agentName: envelope.agentName, taskAuthority: envelope, taskStepId, taskEvidenceBytes: 4096,
  });
  const first = await httpJson(streamPort, 'POST', '/api/command', authorizedCommand(`broker-authority-title-1-${Date.now()}`, 'title', steps[0].stepId));
  assert(first.success === true && /example domain/i.test(first.data?.title || ''), `first ordered read failed: ${JSON.stringify(first)}`);
  await command(['--json', '--session', context.session, 'handoff', 'prepare'], 'prepare disposable daemon handoff');
  await command(['--json', '--session', context.session, 'handoff', 'resume'], 'resume disposable daemon handoff');
  streamPort = await ensureStreamPort(context);
  assert(await readUrl() === target.url, 'daemon handoff changed the retained public target');
  const replayAfterRestart = await httpJson(streamPort, 'POST', '/api/command', {
    ...authorizedCommand(first.id || `broker-authority-title-replay-${Date.now()}`, 'title', steps[0].stepId),
    id: first.id || `broker-authority-title-replay-${Date.now()}`,
  });
  assert(replayAfterRestart.success === false && /expected step|already admitted/.test(replayAfterRestart.error || ''), `restart replay was not rejected: ${JSON.stringify(replayAfterRestart)}`);
  const afterRestartStatus = await httpJson(streamPort, 'GET', `/api/service/task-authorities/${authorityId}?sessionName=${encodeURIComponent(context.session)}`);
  assert(afterRestartStatus.data?.usage?.outcomeSummary?.completed === 1 && afterRestartStatus.data?.usage?.outcomeSummary?.indeterminate === 0, `completed terminal receipt did not survive handoff: ${JSON.stringify(afterRestartStatus)}`);
  const outOfOrder = await httpJson(streamPort, 'POST', '/api/command', authorizedCommand(`broker-authority-title-wrong-${Date.now()}`, 'title', steps[3].stepId));
  assert(outOfOrder.success === false && /expected step/.test(outOfOrder.error || ''), `out-of-order read was not rejected: ${JSON.stringify(outOfOrder)}`);
  const failedRead = await httpJson(streamPort, 'POST', '/api/command', { ...authorizedCommand(`broker-authority-gettext-2-${Date.now()}`, 'gettext', steps[1].stepId), selector: '#agent-browser-p108-missing-selector' });
  assert(failedRead.success === false, `deterministic failed read unexpectedly succeeded: ${JSON.stringify(failedRead)}`);
  const second = await httpJson(streamPort, 'POST', '/api/command', authorizedCommand(`broker-authority-url-3-${Date.now()}`, 'url', steps[2].stepId));
  assert(second.success === true && second.data?.url === target.url, `third ordered read failed: ${JSON.stringify(second)}`);
  const third = await httpJson(streamPort, 'POST', '/api/command', authorizedCommand(`broker-authority-title-4-${Date.now()}`, 'title', steps[3].stepId));
  assert(third.success === true && /example domain/i.test(third.data?.title || ''), `fourth ordered read failed: ${JSON.stringify(third)}`);
  const usedStatus = await httpJson(streamPort, 'GET', `/api/service/task-authorities/${authorityId}?sessionName=${encodeURIComponent(context.session)}`);
  assert(usedStatus.data?.state === 'exhausted' && usedStatus.data?.usage?.remainingActions === 0, 'usage debit drifted');
  assert(usedStatus.data?.usage?.nextStepIndex === 4 && usedStatus.data?.usage?.completedSteps?.length === 3, 'completed terminal receipts drifted');
  assert(usedStatus.data?.usage?.failedSteps?.length === 1 && usedStatus.data?.usage?.indeterminateSteps?.length === 0, 'failed or indeterminate terminal receipts drifted');
  assert(usedStatus.data?.usage?.stepOutcomes?.every((receipt) => /^[a-f0-9]{64}$/.test(receipt.outcome?.responseSha256 || '')), 'terminal response digests were missing');

  const pendingRevoke = await httpJson(streamPort, 'POST', `/api/service/task-authorities/${authorityId}/revoke`, { sessionName: context.session, revokedBy: 'live-qa', reason: 'proof complete' });
  assert(pendingRevoke.data?.confirmation_required === true, 'revocation bypassed confirmation');
  const revoked = await decideAuthorityConfirmation(streamPort, pendingRevoke.data.confirmation_id, 'task_authority_revoke', 'confirm');
  assert(revoked.data?.result?.data?.state === 'revoked', 'revocation was not durable');
  assert(await readUrl() === target.url, 'authority lifecycle changed the target');

  await cleanup();
  console.log(JSON.stringify({ schema: 'agent-browser.broker-task-authority.v4', success: true, posture: { publicOnly: true, authenticatedProfileUsed: false, mutationExecuted: false, promptSubmitted: false, cleanupComplete }, evidence: { issueRequiredExactTargetConfirmation: true, pendingConfirmationSurvivedDaemonRestart: true, restartDidNotAutoConfirmOrRestage: true, decisionActorAndRequestDigestDurable: true, brokerDerivedOrderedStepsAndBudgets: true, repeatedActionMultiplicityPreserved: true, outOfOrderStepRejected: true, durableAdmissionReceipts: true, durableCompletedAndFailedOutcomes: true, terminalResponseDigests: true, daemonRestartPreservedCursorAndCompletedOutcome: true, replayAfterRestartRejected: true, readOnlyStatusPreservedTarget: true, usageWasDebited: true, revokeRequiredExactTargetConfirmation: true, revocationPersisted: true } }, null, 2));
} catch (error) {
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
async function readUrl() {
  return (await command(['--json', '--session', context.session, 'get', 'url'], 'read current URL')).data?.url;
}
function resolveBrowserExecutable() {
  const selected = selectSmokeBrowserExecutable({ configuredExecutable: process.env.AGENT_BROWSER_BROKER_AUTHORITY_BROWSER_EXECUTABLE });
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
