#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';

import {
  createDisposableSmokeProfile,
  isWslWindowsBrowserExecutable,
  selectSmokeBrowserExecutable,
} from './lib/smoke-browser-fixture.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-target-bound-confirmation-',
  sessionPrefix: 'target-bound-confirmation',
});
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD =
  process.env.AGENT_BROWSER_AGENTIC_CONFIRMATION_AGENT_BROWSER_CMD || 'agent-browser';
context.env.AGENT_BROWSER_CONFIRM_ACTIONS = 'external_mutation';
context.env.AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS = '0';

let profilePath;
let cleanupComplete = false;

try {
  const browserExecutable = resolveBrowserExecutable();
  profilePath = createDisposableSmokeProfile({
    browserExecutable,
    defaultRoot: context.tempHome,
    windowsTempRoot: resolveWindowsTempRoot(browserExecutable),
    prefix: 'agentic-confirmation-profile-',
  });
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = browserExecutable;
  context.env.AGENT_BROWSER_PROFILE = profilePath;

  const browserArgs = String(
    process.env.AGENT_BROWSER_AGENTIC_CONFIRMATION_BROWSER_ARGS || '',
  ).trim();
  const opened = await command([
    '--json', '--session', context.session, '--profile', profilePath,
    ...(browserArgs ? ['--args', browserArgs] : []),
    'open', 'https://example.com/', '--headed', '--timeout', '20000',
  ], 'open retained public target');
  assert(opened.success === true, `open failed: ${JSON.stringify(opened)}`);
  const initialUrl = await readField('url');
  const title = await readField('title');
  assert(initialUrl === 'https://example.com/', `unexpected initial URL: ${initialUrl}`);
  assert(/example domain/i.test(title), `unexpected title: ${title}`);

  const firstPending = await command([
    '--json', '--session', context.session, 'click', 'a',
  ], 'first guarded click');
  assert(
    firstPending.data?.confirmation_required === true,
    `click did not stop for approval: ${JSON.stringify(firstPending)}`,
  );
  assert(firstPending.data?.consequenceClass === 'external_mutation', 'click classification drifted');
  assert(firstPending.data?.expectedTargetBinding?.targetId, 'approval omitted target identity');
  assert(firstPending.data?.expectedTargetBinding?.url === initialUrl, 'approval omitted exact URL');
  assert(firstPending.data?.expiresInMs === 60000, 'approval expiry drifted');
  assert(await readField('url') === initialUrl, 'guarded click executed before approval');

  const denied = await command([
    '--json', '--session', context.session,
    'deny', firstPending.data.confirmation_id,
  ], 'deny guarded click');
  assert(denied.data?.denied === true, 'denial was not recorded');
  assert(await readField('url') === initialUrl, 'denied click changed the page');

  const secondPending = await command([
    '--json', '--session', context.session, 'click', 'a',
  ], 'second guarded click');
  const secondConfirmationId = secondPending.data?.confirmation_id;
  assert(secondConfirmationId, 'second approval omitted confirmation ID');

  const changed = await command([
    '--json', '--session', context.session,
    'open', 'https://example.org/', '--timeout', '20000',
  ], 'change target before approval');
  assert(changed.success === true, 'target change failed');
  const changedUrl = await readField('url');
  assert(changedUrl === 'https://example.org/', `unexpected changed URL: ${changedUrl}`);

  let targetMismatch = null;
  try {
    await runCli(context, [
      '--json', '--session', context.session, 'confirm', secondConfirmationId,
    ], 15000);
  } catch (error) {
    targetMismatch = error.message;
  }
  assert(
    targetMismatch && /target changed before approval/i.test(targetMismatch),
    `changed-target approval did not fail closed: ${targetMismatch || 'command succeeded'}`,
  );
  assert(await readField('url') === changedUrl, 'rejected approval executed the click');

  await cleanup();
  console.log(JSON.stringify({
    schema: 'agent-browser.target-bound-agentic-confirmation.v1',
    success: true,
    posture: {
      publicOnly: true,
      authenticatedProfileUsed: false,
      mutationExecuted: false,
      promptSubmitted: false,
      cleanupComplete,
    },
    evidence: {
      readOnlyInspectionSucceeded: true,
      consequenceClass: 'external_mutation',
      confirmationIdRequired: true,
      denialPreservedUrl: true,
      changedTargetRejected: true,
      exactTargetBindingPresent: true,
      expiryMs: 60000,
    },
  }, null, 2));
} catch (error) {
  await cleanup();
  console.error(error.stack || error.message);
  process.exitCode = 1;
}

async function command(args, label) {
  const result = await runCli(context, args, 30000);
  return parseJsonOutput(result.stdout, label);
}

async function readField(field) {
  const result = await command([
    '--json', '--session', context.session, 'get', field,
  ], `read ${field}`);
  assert(result.success === true, `read ${field} failed: ${JSON.stringify(result)}`);
  return result.data?.[field];
}

function resolveBrowserExecutable() {
  const selected = selectSmokeBrowserExecutable({
    configuredExecutable: process.env.AGENT_BROWSER_AGENTIC_CONFIRMATION_BROWSER_EXECUTABLE,
  });
  if (selected) {
    assert(existsSync(selected), `configured browser executable is missing: ${selected}`);
    return selected;
  }
  const doctor = spawnSync(
    context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,
    ['install', 'doctor', '--json'],
    { encoding: 'utf8', env: process.env },
  );
  const payload = parseJsonOutput(doctor.stdout, 'agent-browser install doctor');
  const executable = payload.data?.launchConfig?.executablePath;
  assert(executable && existsSync(executable), 'install doctor returned no usable browser executable');
  return executable;
}

function resolveWindowsTempRoot(browserExecutable) {
  if (!isWslWindowsBrowserExecutable(browserExecutable)) return null;
  const windowsTemp = execFileSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', '[Console]::Out.Write([IO.Path]::GetTempPath())'],
    { encoding: 'utf8' },
  ).trim();
  return execFileSync('wslpath', ['-u', windowsTemp], { encoding: 'utf8' }).trim();
}

async function cleanup() {
  if (cleanupComplete) return;
  cleanupComplete = true;
  await closeSession(context);
  if (profilePath) rmSync(profilePath, { recursive: true, force: true });
  context.cleanupTempHome();
}
