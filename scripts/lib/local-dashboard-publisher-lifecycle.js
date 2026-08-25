import {
  existsSync,
  readFileSync,
  rmSync,
} from 'node:fs';
import { join } from 'node:path';

import { classifyStandaloneDashboardProcess } from './local-dashboard-smoke-policy.js';

export function quiesceStandaloneDashboardForRuntimeHandoff({
  runtimeSocketDir,
  service,
  timeoutMs = 5000,
} = {}) {
  const pidPath = join(runtimeSocketDir, 'dashboard.pid');
  if (!existsSync(pidPath)) return { state: 'absent', pid: null };

  const pid = Number.parseInt(readFileSync(pidPath, 'utf8').trim(), 10);
  const running = Number.isInteger(pid) && pid > 0 && processIsLive(pid);
  const processUid = running ? linuxProcessUid(pid) : null;
  const currentUid = typeof process.getuid === 'function' ? process.getuid() : null;
  const command = running ? linuxProcessCommand(pid) : null;
  const dashboardMode = running ? linuxProcessEnvironment(pid).AGENT_BROWSER_DASHBOARD ?? null : null;
  const identity = classifyStandaloneDashboardProcess({
    pid,
    running,
    processUid,
    currentUid,
    command,
    dashboardMode,
  });
  service.standaloneDashboard = {
    pid: Number.isInteger(pid) && pid > 0 ? pid : null,
    state: identity.state,
    command,
  };

  if (identity.safeToRemovePidFile) {
    rmSync(pidPath, { force: true });
    return { state: identity.state, pid: service.standaloneDashboard.pid };
  }
  if (!identity.safeToRetire) {
    throw new Error(
      `Refusing to retire dashboard PID ${Number.isInteger(pid) ? pid : 'invalid'} because its process identity did not match the user-owned agent-browser dashboard`,
    );
  }

  process.kill(pid, 'SIGTERM');
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline && processIsLive(pid)) sleep(25);
  if (processIsLive(pid)) {
    throw new Error(`Standalone dashboard PID ${pid} did not exit after SIGTERM`);
  }
  let recordedPid = null;
  try {
    recordedPid = Number.parseInt(readFileSync(pidPath, 'utf8').trim(), 10);
  } catch {
    // The exiting dashboard may already have removed its own PID metadata.
  }
  if (recordedPid === pid) rmSync(pidPath, { force: true });
  service.quiesced = true;
  service.action = 'stop-standalone-for-runtime-handoff';
  return { state: identity.state, pid };
}

export function restartOrStartDashboardRuntime({
  installBin,
  restoring = false,
  startIfMissing = false,
  service,
  serviceStatus,
  runCommand,
} = {}) {
  const status = serviceStatus();
  if (status.loadState === 'loaded') {
    service.action = restoring ? 'restart-after-restore' : 'restart';
    runCommand('systemctl', ['--user', 'restart', 'agent-browser-dashboard.service']);
    return { action: service.action, started: true, mode: 'systemd' };
  }

  const resumeOwnedStandalone = service.quiesced
    && service.standaloneDashboard?.state === 'owned_dashboard';
  if (!startIfMissing && !resumeOwnedStandalone) {
    service.action = 'not-installed';
    return { action: service.action, started: false, mode: 'absent' };
  }

  service.action = resumeOwnedStandalone
    ? restoring ? 'restart-standalone-after-restore' : 'restart-standalone'
    : restoring ? 'start-after-restore' : 'start';
  runCommand(installBin, ['dashboard', 'start']);
  return {
    action: service.action,
    started: true,
    mode: resumeOwnedStandalone ? 'standalone-resume' : 'explicit-start',
  };
}

export function processIsLive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function linuxProcessUid(pid) {
  try {
    const match = readFileSync(`/proc/${pid}/status`, 'utf8').match(/^Uid:\s+(\d+)/m);
    return match ? Number.parseInt(match[1], 10) : null;
  } catch {
    return null;
  }
}

function linuxProcessCommand(pid) {
  try {
    return readFileSync(`/proc/${pid}/cmdline`, 'utf8').split('\0')[0] || null;
  } catch {
    return null;
  }
}

function linuxProcessEnvironment(pid) {
  try {
    return Object.fromEntries(
      readFileSync(`/proc/${pid}/environ`, 'utf8')
        .split('\0')
        .filter(Boolean)
        .map((entry) => {
          const index = entry.indexOf('=');
          return index > 0 ? [entry.slice(0, index), entry.slice(index + 1)] : [entry, ''];
        }),
    );
  } catch {
    return {};
  }
}

function sleep(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
