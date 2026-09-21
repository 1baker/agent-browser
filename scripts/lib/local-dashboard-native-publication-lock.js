import * as fs from 'node:fs';
import { dirname, join } from 'node:path';

// WorkstationLock treats a non-PID payload as busy, even after publisher death.
// This is intentional: only receipt-bound recovery may release this marker.
export function nativePublicationLock({ lockPath, receipt, recovering, fsAdapter = fs }) {
  const stagePath = `${lockPath}.publication-${receipt.id}`;
  const payload = `${JSON.stringify({ publication: receipt.id })}\n`;
  const syncDirectory = () => {
    const fd = fsAdapter.openSync(dirname(lockPath), 'r');
    try { fsAdapter.fsyncSync(fd); } finally { fsAdapter.closeSync(fd); }
  };
  const verify = (path) => {
    const fd = fsAdapter.openSync(path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
    try {
      const info = fsAdapter.fstatSync(fd);
      if (!info.isFile() || info.uid !== process.getuid() || info.nlink > 2 ||
          info.dev !== receipt.nativeIdentity.dev || info.ino !== receipt.nativeIdentity.ino ||
          fsAdapter.readFileSync(fd, 'utf8') !== payload) {
        throw new Error('Native publication lock identity changed; refusing cleanup');
      }
    } finally { fsAdapter.closeSync(fd); }
  };
  if (!recovering) {
    fsAdapter.mkdirSync(dirname(lockPath), { recursive: true, mode: 0o700 });
    const fd = fsAdapter.openSync(stagePath, 'wx', 0o600);
    try {
      fsAdapter.writeFileSync(fd, payload);
      fsAdapter.fsyncSync(fd);
      const info = fsAdapter.fstatSync(fd);
      receipt.nativeIdentity = { dev: info.dev, ino: info.ino };
    } finally { fsAdapter.closeSync(fd); }
    syncDirectory();
  } else {
    if (!receipt.nativeIdentity || !/^[a-f0-9-]{36}$/.test(receipt.id)) {
      throw new Error('Native publication recovery identity is missing');
    }
    verify(stagePath);
  }
  let held = false;
  return {
    acquire() {
      verify(stagePath);
      try { fsAdapter.linkSync(stagePath, lockPath); }
      catch (error) {
        if (!recovering || error.code !== 'EEXIST') throw error;
        verify(lockPath);
      }
      held = true;
      verify(lockPath);
      syncDirectory();
    },
    release() {
      if (!held) return;
      verify(stagePath);
      verify(lockPath);
      fsAdapter.unlinkSync(lockPath);
      syncDirectory();
      held = false;
    },
    cleanup() {
      verify(stagePath);
      fsAdapter.unlinkSync(stagePath);
      syncDirectory();
    },
  };
}

export function verifyNativePublicationRoot({ root, show, command, fsAdapter = fs }) {
  for (const property of ['RootDirectory', 'RootImage', 'PAMName', 'User',
    'BindPaths', 'BindReadOnlyPaths', 'TemporaryFileSystem', 'MountImages']) {
    if (show(property)) throw new Error(`Native interlock ${property} override is unsupported`);
  }
  // Reject root-changing overrides rather than trying to emulate systemd's
  // environment parser. Values are inspected privately, never reported.
  const rejectOverrides = (value) => {
    if (/AGENT_BROWSER_WORKSTATION_ROOT\s*=/.test(value)) {
      throw new Error('Native interlock workstation root override is unsupported');
    }
    const homes = [...value.matchAll(/(?:^|[\s"])HOME=([^\s"]+)/g)];
    if (homes.some((match) => match[1] !== root)) throw new Error('Native interlock HOME differs from publication root');
    if ((value.match(/\bHOME\s*=/g) || []).length !== homes.length) {
      throw new Error('Native interlock HOME syntax is unsupported');
    }
  };
  const managerEnvironment = command('systemctl', ['--user', 'show-environment']);
  rejectOverrides(managerEnvironment);
  if (!managerEnvironment.split('\n').includes(`HOME=${root}`)) {
    throw new Error('Native interlock manager HOME is unproven');
  }
  rejectOverrides(show('Environment'));
  if (show('UnsetEnvironment') || show('PassEnvironment')) {
    throw new Error('Native interlock environment directives are unsupported');
  }
  const envPath = join(root, '.agent-browser/.env');
  const files = show('EnvironmentFiles');
  if (files && files !== `${envPath} (ignore_errors=yes)`) {
    throw new Error('Native interlock environment file contract is unsupported');
  }
  if (files && fsAdapter.existsSync(envPath)) {
    const value = fsAdapter.readFileSync(envPath, 'utf8');
    if (/HOME|AGENT_BROWSER_WORKSTATION_ROOT|\\/.test(value)) {
      throw new Error('Native interlock environment file may change the workstation root');
    }
  }
}
