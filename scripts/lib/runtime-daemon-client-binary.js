import { existsSync, readlinkSync } from 'node:fs';

export function resolveRuntimeDaemonClientBinary(
  daemonPid,
  fallbackBin,
  dependencies = {},
) {
  const platform = dependencies.platform || process.platform;
  const pathExists = dependencies.pathExists || existsSync;
  const readLink = dependencies.readLink || readlinkSync;
  if (platform !== 'linux' || !Number.isInteger(daemonPid) || daemonPid <= 0) {
    return fallbackBin;
  }
  const procExecutable = `/proc/${daemonPid}/exe`;
  if (!pathExists(procExecutable)) return fallbackBin;
  try {
    // Linux keeps /proc/<pid>/exe executable for the lifetime of the process
    // even after its original pathname was atomically replaced. That retained
    // image is the only client guaranteed to match an old daemon protocol.
    readLink(procExecutable);
  } catch {
    return fallbackBin;
  }
  return procExecutable;
}
