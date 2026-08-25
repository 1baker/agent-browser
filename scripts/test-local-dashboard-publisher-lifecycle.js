#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  processIsLive,
  quiesceStandaloneDashboardForRuntimeHandoff,
  restartOrStartDashboardRuntime,
} from './lib/local-dashboard-publisher-lifecycle.js';

if (process.platform !== 'linux') {
  console.log('Local dashboard publisher lifecycle fixture skipped: Linux /proc is required');
  process.exit(0);
}

const root = mkdtempSync(join(tmpdir(), 'agent-browser-dashboard-publisher-lifecycle-'));
const ownedBinary = join(root, 'agent-browser');
const mismatchedBinary = join(root, 'not-agent-browser');
const trackedPids = new Set();
const ciWorkflow = readFileSync('.github/workflows/ci.yml', 'utf8');

copyFileSync(process.execPath, ownedBinary);
copyFileSync(process.execPath, mismatchedBinary);
chmodSync(ownedBinary, 0o755);
chmodSync(mismatchedBinary, 0o755);

try {
  runRestartScenario({ restoring: false, expectedAction: 'restart-standalone' });
  runRestartScenario({ restoring: true, expectedAction: 'restart-standalone-after-restore' });
  runAbsentScenario();
  runStalePidScenario();
  runMismatchScenario();
  assertFastCiCoverage();
} finally {
  for (const pid of trackedPids) stopExactProcess(pid);
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard publisher lifecycle fixture passed');

function runRestartScenario({ restoring, expectedAction }) {
  const scenario = restoring ? 'rollback' : 'normal';
  const socketDir = join(root, scenario);
  mkdirSync(socketDir, { recursive: true });
  const originalPid = launchDashboardProcess(ownedBinary, socketDir);
  const service = { action: 'none', quiesced: false, standaloneDashboard: null };

  const quiesced = quiesceStandaloneDashboardForRuntimeHandoff({
    runtimeSocketDir: socketDir,
    service,
    timeoutMs: 5000,
  });
  trackedPids.delete(originalPid);
  assert.deepEqual(quiesced, { state: 'owned_dashboard', pid: originalPid });
  assert.equal(processIsLive(originalPid), false, `${scenario} fixture must stop the exact original PID`);
  assert.equal(service.action, 'stop-standalone-for-runtime-handoff');
  assert.equal(service.quiesced, true);
  assert.equal(service.standaloneDashboard.command, ownedBinary);

  const commands = [];
  const restarted = restartOrStartDashboardRuntime({
    installBin: ownedBinary,
    restoring,
    startIfMissing: false,
    service,
    serviceStatus: () => ({ loadState: 'unknown', activeState: 'unknown' }),
    runCommand: (command, args) => {
      commands.push({ command, args });
      assert.equal(command, ownedBinary);
      assert.deepEqual(args, ['dashboard', 'start']);
      launchDashboardProcess(ownedBinary, socketDir);
    },
  });
  const replacementPid = readPid(socketDir);
  assert.notEqual(replacementPid, originalPid);
  assert.equal(processIsLive(replacementPid), true);
  assert.deepEqual(commands, [{ command: ownedBinary, args: ['dashboard', 'start'] }]);
  assert.deepEqual(restarted, {
    action: expectedAction,
    started: true,
    mode: 'standalone-resume',
  });
  assert.equal(service.action, expectedAction);

  stopExactProcess(replacementPid);
  trackedPids.delete(replacementPid);
  rmSync(join(socketDir, 'dashboard.pid'), { force: true });
}

function runAbsentScenario() {
  const socketDir = join(root, 'absent');
  mkdirSync(socketDir, { recursive: true });
  const service = { action: 'none', quiesced: false, standaloneDashboard: null };
  assert.deepEqual(
    quiesceStandaloneDashboardForRuntimeHandoff({ runtimeSocketDir: socketDir, service }),
    { state: 'absent', pid: null },
  );
  let commandCount = 0;
  assert.deepEqual(
    restartOrStartDashboardRuntime({
      installBin: ownedBinary,
      startIfMissing: false,
      service,
      serviceStatus: () => ({ loadState: 'unknown', activeState: 'unknown' }),
      runCommand: () => { commandCount += 1; },
    }),
    { action: 'not-installed', started: false, mode: 'absent' },
  );
  assert.equal(commandCount, 0, 'an absent dashboard must still require explicit start authority');
}

function runStalePidScenario() {
  const socketDir = join(root, 'stale');
  mkdirSync(socketDir, { recursive: true });
  const pidPath = join(socketDir, 'dashboard.pid');
  writeFileSync(pidPath, '999999999\n');
  const service = { action: 'none', quiesced: false, standaloneDashboard: null };
  assert.deepEqual(
    quiesceStandaloneDashboardForRuntimeHandoff({ runtimeSocketDir: socketDir, service }),
    { state: 'stale', pid: 999999999 },
  );
  assert.equal(existsSync(pidPath), false, 'stale PID metadata must be removed without signaling a process');
  assert.equal(service.quiesced, false);
}

function runMismatchScenario() {
  const socketDir = join(root, 'mismatch');
  mkdirSync(socketDir, { recursive: true });
  const mismatchPid = launchDashboardProcess(mismatchedBinary, socketDir);
  const service = { action: 'none', quiesced: false, standaloneDashboard: null };
  assert.throws(
    () => quiesceStandaloneDashboardForRuntimeHandoff({
      runtimeSocketDir: socketDir,
      service,
      timeoutMs: 500,
    }),
    /process identity did not match/,
  );
  assert.equal(processIsLive(mismatchPid), true, 'identity mismatch must not signal the recorded process');
  assert.equal(service.standaloneDashboard.state, 'identity_mismatch');
  assert.equal(service.quiesced, false);
  stopExactProcess(mismatchPid);
  trackedPids.delete(mismatchPid);
  rmSync(join(socketDir, 'dashboard.pid'), { force: true });
}

function assertFastCiCoverage() {
  const dashboardJobStart = ciWorkflow.indexOf('\n  dashboard:\n');
  const nextJobStart = ciWorkflow.indexOf('\n  service-client:\n', dashboardJobStart);
  assert.ok(dashboardJobStart >= 0 && nextJobStart > dashboardJobStart, 'CI must contain a bounded Dashboard job');
  const dashboardJob = ciWorkflow.slice(dashboardJobStart, nextJobStart);
  const smokePolicyIndex = dashboardJob.indexOf('pnpm test:local-dashboard-smoke-policy');
  const lifecycleIndex = dashboardJob.indexOf('pnpm test:local-dashboard-publisher-lifecycle');
  const orchestrationIndex = dashboardJob.indexOf('pnpm test:local-dashboard-publisher-orchestration');
  const journalIndex = dashboardJob.indexOf('pnpm test:local-dashboard-publication-journal');
  const operationsIndex = dashboardJob.indexOf('pnpm test:local-dashboard-publication-operations');
  const projectionIndex = dashboardJob.indexOf('pnpm test:dashboard-publication-status-projection');
  const buildIndex = dashboardJob.indexOf('\n      - name: Build dashboard\n');
  assert.ok(smokePolicyIndex >= 0, 'fast Dashboard CI must run the dashboard smoke policy');
  assert.ok(lifecycleIndex > smokePolicyIndex, 'fast Dashboard CI must run the lifecycle fixture after policy coverage');
  assert.ok(orchestrationIndex > lifecycleIndex, 'fast Dashboard CI must run orchestration faults after lifecycle coverage');
  assert.ok(journalIndex > orchestrationIndex, 'fast Dashboard CI must run durable journal coverage after orchestration faults');
  assert.ok(operationsIndex > journalIndex, 'fast Dashboard CI must prove status and recovery-only authorization after journal coverage');
  assert.ok(projectionIndex > operationsIndex, 'fast Dashboard CI must prove installed doctor and dashboard status projection after operation coverage');
  assert.ok(buildIndex > projectionIndex, 'publisher lifecycle gates must run before the dashboard build');
}

function launchDashboardProcess(binary, socketDir) {
  const pidPath = join(socketDir, 'dashboard.pid');
  const childSource = `
process.on('SIGTERM', () => process.exit(0));
process.on('SIGINT', () => process.exit(0));
setInterval(() => {}, 1000);
`;
  const launcherSource = `
const { spawn } = require('node:child_process');
const { writeFileSync } = require('node:fs');
const child = spawn(process.env.FIXTURE_BINARY, ['-e', process.env.FIXTURE_CHILD_SOURCE], {
  detached: true,
  env: { ...process.env, AGENT_BROWSER_DASHBOARD: '1' },
  stdio: 'ignore',
});
writeFileSync(process.env.FIXTURE_PID_PATH, String(child.pid) + '\\n');
child.unref();
`;
  const launched = spawnSync(process.execPath, ['-e', launcherSource], {
    env: {
      ...process.env,
      FIXTURE_BINARY: binary,
      FIXTURE_CHILD_SOURCE: childSource,
      FIXTURE_PID_PATH: pidPath,
    },
    encoding: 'utf8',
    timeout: 5000,
  });
  assert.equal(launched.status, 0, launched.stderr || launched.stdout);
  const pid = readPid(socketDir);
  trackedPids.add(pid);
  waitForProcess(pid, true);
  return pid;
}

function readPid(socketDir) {
  return Number.parseInt(readFileSync(join(socketDir, 'dashboard.pid'), 'utf8').trim(), 10);
}

function stopExactProcess(pid) {
  if (!processIsLive(pid)) return;
  process.kill(pid, 'SIGTERM');
  waitForProcess(pid, false);
}

function waitForProcess(pid, expectedLive) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (processIsLive(pid) === expectedLive) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25);
  }
  throw new Error(`Timed out waiting for PID ${pid} live=${expectedLive}`);
}
