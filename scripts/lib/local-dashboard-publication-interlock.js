import * as fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { basename, dirname, join, isAbsolute } from 'node:path';
import { nativePublicationLock, verifyNativePublicationRoot } from './local-dashboard-native-publication-lock.js';

export const INTERLOCK_TIMER = 'agent-browser-runtime-interlock.timer';
export const INTERLOCK_SERVICE = 'agent-browser-runtime-interlock.service';

export function readLocalDashboardPublicationInterlockReceipt(receiptPath) {
  return JSON.parse(fs.readFileSync(receiptPath, 'utf8'));
}

/**
 * The caller must hold the publication journal lock before calling this helper,
 * including recovery. The receipt is immutable intent, durable before timer stop.
 * Recovery requires the exact receipt ID and a dead previous owner; it never
 * infers custody from a PID alone or overwrites an unresolved receipt.
 *
 * Linux flock locks belong to open file descriptions. The short-lived flock
 * child inherits the parent's descriptor at fd 3; closing the child does not
 * release the lock while the parent descriptor remains open. Thus this entirely
 * synchronous guard also covers asynchronous publication in the caller.
 */
export function acquireLocalDashboardPublicationInterlock({
  receiptPath,
  runtimeDir = process.env.XDG_RUNTIME_DIR || `/run/user/${process.getuid()}`,
  runCommand = spawnSync,
  fsAdapter = fs,
  recoverReceiptId = null,
  installBin = null,
  workstationRoot = null,
  ownerIsAlive = (pid) => {
    try { process.kill(pid, 0); return true; }
    catch (error) { if (error.code === 'ESRCH') return false; throw error; }
  },
}) {
  if (!receiptPath || !isAbsolute(receiptPath) || !isAbsolute(runtimeDir)) {
    throw new Error('Interlock custody requires absolute receipt and runtime paths');
  }
  let lockPath = join(runtimeDir, 'agent-browser-runtime-interlock.lock');
  const syncReceiptDirectory = () => {
    const directoryFd = fsAdapter.openSync(dirname(receiptPath), 'r');
    try { fsAdapter.fsyncSync(directoryFd); } finally { fsAdapter.closeSync(directoryFd); }
  };
  const command = (bin, args, options = {}) => {
    const result = runCommand(bin, args, { encoding: 'utf8', timeout: 15000, ...options });
    if (result.error || result.status !== 0) {
      throw new Error(`Interlock command failed: ${bin} ${args.join(' ')}: ${result.error?.message || result.stderr || result.status}`);
    }
    return String(result.stdout || '').trim();
  };
  const show = (unit, property) => command('systemctl', ['--user', 'show', unit, `--property=${property}`, '--value']);
  const state = (unit) => {
    if (show(unit, 'LoadState') !== 'loaded') throw new Error(`Interlock unit is not loaded: ${unit}`);
    return show(unit, 'ActiveState');
  };
  const idleService = () => {
    if (!['inactive', 'failed'].includes(state(INTERLOCK_SERVICE))) {
      throw new Error('Interlock service is busy; refusing publication without stopping its oneshot');
    }
  };
  const execStart = show(INTERLOCK_SERVICE, 'ExecStart');
  const configuredCommand = execStart.match(/^\{ path=(\S+) ; argv\[\]=([^;]+) ;/);
  const configuredArgs = configuredCommand?.[2].trim().split(/\s+/);
  const native = installBin && workstationRoot && isAbsolute(installBin) && isAbsolute(workstationRoot) &&
    configuredCommand?.[1] === installBin &&
    JSON.stringify(configuredArgs) === JSON.stringify([installBin, 'install', 'workstation', 'reconcile', '--json']);
  if (!configuredCommand || (execStart.match(/\{ path=/g) || []).length !== 1 ||
      (!native && (basename(configuredCommand[1]) !== 'flock' ||
      configuredArgs[0] !== configuredCommand[1] || configuredArgs[1] !== '--nonblock' ||
      configuredArgs[2] !== lockPath || configuredArgs.length < 4))) {
    throw new Error('Interlock service does not declare the expected nonblocking lock or exact native reconcile command');
  }
  if (native) {
    verifyNativePublicationRoot({ root: workstationRoot,
      show: (property) => show(INTERLOCK_SERVICE, property), command, fsAdapter });
    lockPath = join(workstationRoot, '.agent-browser/convergence/workstation.lock');
  }
  let receipt;
  let recovering = false;
  if (fsAdapter.existsSync(receiptPath)) {
    receipt = JSON.parse(fsAdapter.readFileSync(receiptPath, 'utf8'));
    if (!recoverReceiptId || receipt.id !== recoverReceiptId || receipt.version !== (native ? 2 : 1) ||
        receipt.timer !== INTERLOCK_TIMER || receipt.service !== INTERLOCK_SERVICE ||
        (native && receipt.nativeExecutable !== installBin) ||
        receipt.lockPath !== lockPath || !['active', 'inactive'].includes(receipt.priorTimerState) ||
        !Number.isSafeInteger(receipt.ownerPid) || receipt.ownerPid <= 0) {
      throw new Error(`Unresolved interlock custody receipt requires explicit recovery: ${receiptPath}`);
    }
    if (ownerIsAlive(receipt.ownerPid)) throw new Error('Interlock custody owner is still alive; recovery refused');
    recovering = true;
  } else if (recoverReceiptId) {
    throw new Error('Interlock recovery receipt is missing');
  }
  const timerState = state(INTERLOCK_TIMER);
  if (!['active', 'inactive'].includes(timerState)) throw new Error(`Interlock timer state is not stable: ${timerState}`);
  idleService();
  let nativeLock;
  if (!recovering) {
    receipt = { version: native ? 2 : 1, id: randomUUID(), ownerPid: process.pid,
      createdAt: new Date().toISOString(), timer: INTERLOCK_TIMER,
      service: INTERLOCK_SERVICE, lockPath, priorTimerState: timerState };
    if (native) receipt.nativeExecutable = installBin;
    if (native) nativeLock = nativePublicationLock({ lockPath, receipt, recovering, fsAdapter });
    const receiptFd = fsAdapter.openSync(receiptPath, 'wx', 0o600);
    try {
      fsAdapter.writeFileSync(receiptFd, `${JSON.stringify(receipt, null, 2)}\n`);
      fsAdapter.fsyncSync(receiptFd);
    } finally { fsAdapter.closeSync(receiptFd); }
    syncReceiptDirectory();
  } else if (native) {
    nativeLock = nativePublicationLock({ lockPath, receipt, recovering, fsAdapter });
  }
  let lockFd = null;
  let released = false;
  const restore = () => {
    if (released) return;
    // Release exclusion before rearming a persistent timer, which may fire now.
    if (lockFd !== null) { fsAdapter.closeSync(lockFd); lockFd = null; }
    const saved = JSON.parse(fsAdapter.readFileSync(receiptPath, 'utf8'));
    if (saved.id !== receipt.id) throw new Error('Interlock custody receipt changed; refusing restoration');
    nativeLock?.release();
    const current = state(INTERLOCK_TIMER);
    if (current !== receipt.priorTimerState) {
      command('systemctl', ['--user', receipt.priorTimerState === 'active' ? 'start' : 'stop', INTERLOCK_TIMER]);
    }
    if (state(INTERLOCK_TIMER) !== receipt.priorTimerState) throw new Error('Interlock timer restoration verification failed; custody receipt retained');
    fsAdapter.unlinkSync(receiptPath);
    syncReceiptDirectory();
    released = true;
    nativeLock?.cleanup();
  };
  try {
    if (timerState === 'active') command('systemctl', ['--user', 'stop', INTERLOCK_TIMER]);
    if (state(INTERLOCK_TIMER) !== 'inactive') throw new Error('Interlock timer did not become inactive');
    idleService();
    if (nativeLock) {
      nativeLock.acquire();
    } else {
      lockFd = fsAdapter.openSync(lockPath, fs.constants.O_CREAT | fs.constants.O_RDWR | fs.constants.O_NOFOLLOW, 0o600);
      const info = fsAdapter.fstatSync(lockFd);
      if (!info.isFile() || info.uid !== process.getuid() || info.nlink !== 1) throw new Error('Interlock lock identity is unsafe');
      command('flock', ['--nonblock', '3'], { stdio: ['ignore', 'pipe', 'pipe', lockFd] });
      const currentLock = fsAdapter.lstatSync(lockPath);
      if (currentLock.ino !== info.ino || currentLock.dev !== info.dev || currentLock.isSymbolicLink()) {
        throw new Error('Interlock lock path changed during acquisition');
      }
    }
    idleService();
    if (state(INTERLOCK_TIMER) !== 'inactive') throw new Error('Interlock timer reactivated during lock acquisition');
    return {
      receipt,
      release: restore,
      retainForRecovery() {
        if (released) throw new Error('Interlock custody has already been released');
        const saved = JSON.parse(fsAdapter.readFileSync(receiptPath, 'utf8'));
        if (saved.id !== receipt.id) throw new Error('Interlock custody receipt changed; refusing retention');
        if (state(INTERLOCK_TIMER) !== 'inactive') throw new Error('Interlock timer is not inactive; recovery retention verification failed');
        if (lockFd !== null) { fsAdapter.closeSync(lockFd); lockFd = null; }
        // The journal remains nonterminal. Only an explicit recovery may
        // restore the recorded timer intent; this handle must not do so later.
        released = true;
      },
    };
  } catch (error) {
    if (recovering) {
      // Existing custody may protect an uncertain handoff. Failed recovery
      // admission must not rearm maintenance or erase that original intent.
      if (lockFd !== null) { fsAdapter.closeSync(lockFd); lockFd = null; }
      throw error;
    }
    try { restore(); }
    catch (restoreError) { throw new AggregateError([error, restoreError], 'Interlock acquisition refused and restoration failed; custody receipt retained'); }
    throw error;
  }
}
