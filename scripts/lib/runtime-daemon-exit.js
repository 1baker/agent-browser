// Process retirement and session vacancy are different observations. A stale
// PID file must not prolong retirement, and a new owner must never be signaled.
export function waitForRuntimeDaemonExit(sessionName, priorPid, {
  readRuntimePid,
  isProcessLive,
  now = Date.now,
  sleep = (ms) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms),
  timeoutMs = 5000,
}) {
  const deadline = now() + timeoutMs;
  for (;;) {
    const currentPid = readRuntimePid(sessionName);
    if (currentPid !== priorPid && Number.isInteger(currentPid) && isProcessLive(currentPid)) {
      const error = new Error(`Daemon session '${sessionName}' was reacquired by PID ${currentPid} during executable handoff; no replacement process was signaled`);
      error.code = 'runtime_session_reacquired';
      throw error;
    }
    if (!isProcessLive(priorPid)) return;
    if (now() >= deadline) {
      const error = new Error(`Daemon session '${sessionName}' did not exit for executable handoff`);
      error.code = 'runtime_daemon_exit_timeout';
      throw error;
    }
    sleep(50);
  }
}

export function verifyRuntimeSessionsRetired({ sessionNames, readRuntimePid, isProcessLive }) {
  for (const sessionName of sessionNames) {
    const pid = readRuntimePid(sessionName);
    if (Number.isInteger(pid) && isProcessLive(pid)) {
      throw new Error(`Daemon session '${sessionName}' is live before executable replacement (PID ${pid}); publication must recover before retrying`);
    }
  }
}
