#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';

import {
  createDisposableSmokeProfile,
  isWslWindowsBrowserExecutable,
  selectSmokeBrowserExecutable,
} from './lib/smoke-browser-fixture.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-durable-task-authority-',
  sessionPrefix: 'durable-task-authority',
});
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD =
  process.env.AGENT_BROWSER_TASK_AUTHORITY_AGENT_BROWSER_CMD || 'agent-browser';
context.env.AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS = '0';
context.env.AGENT_BROWSER_TASK_AUTHORITY_DIR = join(context.tempHome, 'task-authority-ledger');

let profilePath;
let cleanupComplete = false;

try {
  const browserExecutable = resolveBrowserExecutable();
  profilePath = createDisposableSmokeProfile({
    browserExecutable,
    defaultRoot: context.tempHome,
    windowsTempRoot: resolveWindowsTempRoot(browserExecutable),
    prefix: 'durable-task-authority-profile-',
  });
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = browserExecutable;
  context.env.AGENT_BROWSER_PROFILE = profilePath;

  const browserArgs = String(process.env.AGENT_BROWSER_TASK_AUTHORITY_BROWSER_ARGS || '').trim();
  const opened = await command([
    '--json', '--session', context.session, '--profile', profilePath,
    ...(browserArgs ? ['--args', browserArgs] : []),
    'open', 'https://example.com/', '--headed', '--timeout', '20000',
  ], 'open retained public target');
  assert(opened.success === true, `open failed: ${JSON.stringify(opened)}`);

  const streamPort = await ensureStreamPort(context);
  const tabs = await httpJson(streamPort, 'POST', '/api/command', {
    action: 'tab_list',
    verbose: true,
  });
  const target = tabs.data?.tabs?.find((tab) => tab.active === true) || tabs.data?.tabs?.[0];
  assert(target?.targetId, `tab list omitted target id: ${JSON.stringify(tabs)}`);
  assert(target.url === 'https://example.com/', `unexpected target URL: ${target?.url}`);

  const authority = {
    id: `public-read-${Date.now()}`,
    taskName: 'inspect-public-example',
    serviceName: 'agent-browser-qa',
    agentName: 'codex',
    allowedOrigins: ['https://example.com'],
    targetBinding: {
      targetId: target.targetId,
      initialUrl: target.url,
    },
    evidenceBudget: {
      maxActions: 2,
      maxEvidenceBytes: 8192,
    },
    consequenceCeiling: 'read_only',
    expiresAt: new Date(Date.now() + 5 * 60 * 1000).toISOString(),
  };
  const authorized = (action, extra = {}) => ({
    id: `${action}-${Date.now()}-${Math.random()}`,
    action,
    serviceName: authority.serviceName,
    agentName: authority.agentName,
    taskName: authority.taskName,
    taskAuthority: authority,
    taskEvidenceBytes: 4096,
    ...extra,
  });

  const firstRead = await httpJson(streamPort, 'POST', '/api/command', authorized('title'));
  assert(firstRead.success === true, `authorized title failed: ${JSON.stringify(firstRead)}`);
  assert(/example domain/i.test(firstRead.data?.title || ''), 'authorized title evidence drifted');

  const changedAuthority = structuredClone(authority);
  changedAuthority.allowedOrigins = ['https://example.com', 'https://example.org'];
  const drift = await httpJson(streamPort, 'POST', '/api/command', {
    ...authorized('title'),
    taskAuthority: changedAuthority,
  });
  assert(drift.success === false, 'changed envelope was admitted');
  assert(/changed after first use/i.test(String(drift.error)), `wrong drift error: ${JSON.stringify(drift)}`);

  const wrongOrigin = await httpJson(
    streamPort,
    'POST',
    '/api/command',
    authorized('title', { url: 'https://example.org/' }),
  );
  assert(wrongOrigin.success === false, 'wrong origin was admitted');
  assert(/origin .* is not allowed/i.test(String(wrongOrigin.error)), 'wrong origin error drifted');

  const pending = await httpJson(
    streamPort,
    'POST',
    '/api/command',
    authorized('click', { selector: 'a' }),
  );
  assert(pending.data?.confirmation_required === true, `click bypassed ceiling: ${JSON.stringify(pending)}`);
  assert(pending.data?.consequenceClass === 'external_mutation', 'click consequence drifted');
  assert(pending.data?.taskAuthority?.id === authority.id, 'confirmation omitted authority id');
  assert(await readUrl() === target.url, 'guarded click executed before approval');

  const denied = await command([
    '--json', '--session', context.session, 'deny', pending.data.confirmation_id,
  ], 'deny above-ceiling action');
  assert(denied.data?.denied === true, 'denial was not recorded');
  assert(await readUrl() === target.url, 'denied action changed the target');

  const secondRead = await httpJson(streamPort, 'POST', '/api/command', authorized('title'));
  assert(secondRead.success === true, 'second authorized read failed');
  const exhausted = await httpJson(streamPort, 'POST', '/api/command', authorized('title'));
  assert(exhausted.success === false, 'exhausted action budget was admitted');
  assert(/action budget exhausted/i.test(String(exhausted.error)), 'budget error drifted');

  await cleanup();
  console.log(JSON.stringify({
    schema: 'agent-browser.durable-task-authority.v1',
    success: true,
    posture: {
      publicOnly: true,
      authenticatedProfileUsed: false,
      mutationExecuted: false,
      promptSubmitted: false,
      cleanupComplete,
    },
    evidence: {
      immutableEnvelope: true,
      exactTaskAndTargetBinding: true,
      allowedOriginEnforced: true,
      durableBudgetLedger: true,
      consequenceCeilingRequiredConfirmation: true,
      denialPreservedTarget: true,
      actionBudgetExhaustionRejected: true,
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

async function readUrl() {
  const result = await command([
    '--json', '--session', context.session, 'get', 'url',
  ], 'read current URL');
  return result.data?.url;
}

function resolveBrowserExecutable() {
  const selected = selectSmokeBrowserExecutable({
    configuredExecutable: process.env.AGENT_BROWSER_TASK_AUTHORITY_BROWSER_EXECUTABLE,
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
