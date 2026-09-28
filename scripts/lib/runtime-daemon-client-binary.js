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
    const target = readLink(procExecutable);
    if (target.endsWith(' (deleted)')) return fallbackBin;
  } catch {
    return fallbackBin;
  }
  return procExecutable;
}
