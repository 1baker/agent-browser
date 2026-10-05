import { statSync } from 'node:fs';

// A live daemon must execute the file just installed by the publisher.
// A missing or inaccessible process identity is not acceptance evidence.
export function daemonUsesInstalledExecutable(pid, installBin, stat = statSync) {
  if (!Number.isSafeInteger(pid) || pid <= 0) return false;
  try {
    const installed = stat(installBin);
    const running = stat(`/proc/${pid}/exe`);
    return installed.dev === running.dev && installed.ino === running.ino;
  } catch {
    return false;
  }
}

export function installedDaemonVerified(pid, installBin, isLive, usesInstalled = daemonUsesInstalledExecutable) {
  return Number.isSafeInteger(pid) && pid > 0 && isLive(pid) && usesInstalled(pid, installBin);
}
