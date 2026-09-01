import { statSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

export function resolveRuntimeSocketDir({
  env = process.env,
  platform = process.platform,
  uid = typeof process.getuid === 'function' ? process.getuid() : null,
  home = homedir(),
  linuxRuntimeRoot = '/run/user',
  stat = statSync,
} = {}) {
  if (env.AGENT_BROWSER_SOCKET_DIR) return resolve(env.AGENT_BROWSER_SOCKET_DIR);
  if (env.XDG_RUNTIME_DIR) return resolve(env.XDG_RUNTIME_DIR, 'agent-browser');

  if (platform === 'linux' && Number.isInteger(uid) && uid >= 0) {
    const candidate = resolve(linuxRuntimeRoot, String(uid));
    try {
      const metadata = stat(candidate, { bigint: false });
      if (
        metadata.isDirectory()
        && metadata.uid === uid
        && (metadata.mode & 0o077) === 0
      ) {
        return join(candidate, 'agent-browser');
      }
    } catch {
      // The home fallback remains authoritative when no secure user runtime
      // directory is available.
    }
  }

  return resolve(home, '.agent-browser');
}
