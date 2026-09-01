import { existsSync, readFileSync } from 'node:fs';

export function retirePreparedDaemon(prepared, dependencies = {}) {
  const isProcessLive = dependencies.isProcessLive || processIsLive;
  const signalProcess = dependencies.signalProcess || process.kill.bind(process);
  const waitForExit = dependencies.waitForExit || waitForProcessExit;

  if (
    !prepared.handoffPath
    || !existsSync(prepared.handoffPath)
    || !isProcessLive(prepared.daemonPid)
  ) {
    throw new Error(
      `Prepared daemon session '${prepared.sessionName}' lacks a verifiable retirement boundary`,
    );
  }
  const descriptor = JSON.parse(readFileSync(prepared.handoffPath, 'utf8'));
  if (
    descriptor?.schemaVersion !== 1
    || descriptor?.sessionName !== prepared.sessionName
    || descriptor?.cdpUrl !== prepared.cdpUrl
    || (descriptor?.browserPid ?? null) !== prepared.browserPid
    || (
      Number.isInteger(prepared.browserPid)
      && !isProcessLive(prepared.browserPid)
    )
  ) {
    throw new Error(
      `Prepared daemon session '${prepared.sessionName}' has a mismatched durable descriptor`,
    );
  }

  signalProcess(prepared.daemonPid, 'SIGTERM');
  let signal = 'SIGTERM';
  if (!waitForExit(prepared.daemonPid, 1500, isProcessLive)) {
    signalProcess(prepared.daemonPid, 'SIGKILL');
    signal = 'SIGKILL';
    if (!waitForExit(prepared.daemonPid, 1500, isProcessLive)) {
      throw new Error(
        `Prepared daemon session '${prepared.sessionName}' remained live after ${signal}`,
      );
    }
  }
  if (
    Number.isInteger(prepared.browserPid)
    && !isProcessLive(prepared.browserPid)
  ) {
    throw new Error(
      `Retained browser PID ${prepared.browserPid} exited while retiring prepared daemon ` +
      `'${prepared.sessionName}'`,
    );
  }
  return signal;
}

function processIsLive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function waitForProcessExit(pid, timeoutMs, isProcessLive) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!isProcessLive(pid)) return true;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
  }
  return !isProcessLive(pid);
}
