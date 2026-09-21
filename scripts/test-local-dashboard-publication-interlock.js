#!/usr/bin/env node
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { acquireLocalDashboardPublicationInterlock as acquire, INTERLOCK_TIMER, INTERLOCK_SERVICE } from './lib/local-dashboard-publication-interlock.js';

const directories = [];
function fixture({ timer = 'active', service = 'inactive', lockBusy = false, startFails = false, stopFails = false, realFlock = false, native = false, runtimeRoot = null } = {}) {
  const runtimeDir = runtimeRoot || fs.mkdtempSync(join(tmpdir(), 'publication-interlock-test-'));
  directories.push(runtimeDir);
  const receiptPath = join(runtimeDir, 'receipt.json');
  const calls = [];
  const runCommand = (bin, args, options) => {
    calls.push([bin, ...args]);
    if (bin === 'flock') {
      if (realFlock) return spawnSync(bin, args, options);
      return { status: lockBusy ? 1 : 0, stdout: '', stderr: '' };
    }
    assert.equal(bin, 'systemctl');
    const [, action, unit, property] = args;
    if (action === 'show-environment') return { status: 0, stdout: `HOME=${runtimeDir}` };
    assert.ok([INTERLOCK_TIMER, INTERLOCK_SERVICE].includes(unit));
    if (action === 'show') {
      const stdout = property === '--property=LoadState' ? 'loaded' :
        property === '--property=ExecStart' ? (native ?
          `{ path=${runtimeDir}/agent-browser ; argv[]=${runtimeDir}/agent-browser install workstation reconcile --json ; ignore_errors=no ; }` :
          `{ path=/usr/bin/flock ; argv[]=/usr/bin/flock --nonblock ${runtimeDir}/agent-browser-runtime-interlock.lock pnpm converge ; ignore_errors=no ; }`) :
          property === '--property=ActiveState' ? (unit === INTERLOCK_TIMER ? timer : service) : '';
      return { status: 0, stdout, stderr: '' };
    }
    assert.equal(unit, INTERLOCK_TIMER, 'never stop the interlock service');
    if ((action === 'start' && startFails) || (action === 'stop' && stopFails)) return { status: 1, stderr: 'fixture failure' };
    timer = action === 'start' ? 'active' : 'inactive';
    return { status: 0, stdout: '' };
  };
  return { receiptPath, runtimeDir, runCommand, calls, timer: () => timer,
    ...(native ? { installBin: join(runtimeDir, 'agent-browser'), workstationRoot: runtimeDir } : {}) };
}

if (process.argv.includes('--native-crash-child')) {
  const f = fixture({ native: true });
  const crash = () => {
    const receipt = JSON.parse(fs.readFileSync(f.receiptPath, 'utf8'));
    process.stdout.write(JSON.stringify({ runtimeDir: f.runtimeDir, receiptId: receipt.id, timer: f.timer() }));
    process.exit(0);
  };
  const boundary = process.argv.at(-1);
  f.fsAdapter = { ...fs,
    linkSync: (...args) => { if (boundary === 'before-link') crash(); return fs.linkSync(...args); },
    unlinkSync: (path) => {
      fs.unlinkSync(path);
      if (boundary === 'after-unlink' && path.endsWith('/workstation.lock')) crash();
    },
  };
  const guard = acquire(f);
  if (boundary === 'after-unlink') guard.release();
  crash(); // Simulate process loss without releasing custody.
}

try {
  for (const boundary of ['before-link', 'after-link', 'after-unlink']) {
    const child = spawnSync(process.execPath, [process.argv[1], '--native-crash-child', boundary], { encoding: 'utf8' });
    assert.equal(child.status, 0, child.stderr);
    const crashed = JSON.parse(child.stdout);
    const f = fixture({ native: true, timer: crashed.timer, runtimeRoot: crashed.runtimeDir });
    const guard = acquire({ ...f, recoverReceiptId: crashed.receiptId });
    guard.release();
    assert.equal(f.timer(), 'active', 'real exited publisher can be recovered without PID override');
  }
  for (const initial of ['active', 'inactive']) {
    const f = fixture({ native: true, timer: initial });
    const guard = acquire(f);
    const lock = guard.receipt.lockPath;
    assert.equal(guard.receipt.version, 2);
    assert.throws(() => fs.openSync(lock, 'wx'), /EEXIST/);
    assert.ok(Number.isNaN(Number(fs.readFileSync(lock, 'utf8'))), 'native PID parser cannot reclaim crash-persistent marker');
    guard.retainForRecovery();
    assert.ok(fs.existsSync(lock));
    assert.throws(() => acquire({ ...f, recoverReceiptId: guard.receipt.id }), /still alive/);
    const recovered = acquire({ ...f, recoverReceiptId: guard.receipt.id, ownerIsAlive: () => false });
    recovered.release();
    recovered.release();
    assert.equal(fs.existsSync(lock), false);
    assert.equal(f.timer(), initial);
  }
  for (const payload of ['1234\n', 'not-a-pid\n']) {
    const f = fixture({ native: true });
    const dir = join(f.runtimeDir, '.agent-browser/convergence');
    fs.mkdirSync(dir, { recursive: true });
    const lock = join(dir, 'workstation.lock');
    fs.writeFileSync(lock, payload);
    assert.throws(() => acquire(f), /EEXIST/);
    assert.equal(fs.readFileSync(lock, 'utf8'), payload, 'never reclaim foreign locks');
    assert.equal(f.timer(), 'active');
  }
  for (const property of ['Environment', 'EnvironmentFiles', 'RootDirectory', 'RootImage', 'PAMName', 'User',
    'BindPaths', 'BindReadOnlyPaths', 'TemporaryFileSystem', 'MountImages']) {
    const f = fixture({ native: true });
    const runner = f.runCommand;
    f.runCommand = (bin, args, options) => args.includes(`--property=${property}`) ?
      { status: 0, stdout: property === 'Environment' ? 'AGENT_BROWSER_WORKSTATION_ROOT=/elsewhere' : '/elsewhere' } : runner(bin, args, options);
    assert.throws(() => acquire(f), /unsupported/);
    assert.equal(fs.existsSync(f.receiptPath), false);
    assert.equal(f.timer(), 'active');
  }
  {
    const f = fixture({ native: true });
    const guard = acquire(f);
    fs.unlinkSync(guard.receipt.lockPath);
    fs.writeFileSync(guard.receipt.lockPath, 'foreign');
    assert.throws(() => guard.release(), /identity changed/);
    assert.equal(f.timer(), 'inactive');
    assert.ok(fs.existsSync(f.receiptPath));
    assert.equal(fs.readFileSync(guard.receipt.lockPath, 'utf8'), 'foreign');
  }
  {
    const f = fixture({ native: true, startFails: true });
    const guard = acquire(f);
    assert.throws(() => guard.release(), /command failed/);
    assert.equal(fs.existsSync(guard.receipt.lockPath), false);
    const recovered = acquire({ ...f, recoverReceiptId: guard.receipt.id, ownerIsAlive: () => false });
    assert.ok(fs.existsSync(guard.receipt.lockPath), 'recovery reacquires after marker removal crash boundary');
    recovered.retainForRecovery();
  }
  {
    const f = fixture({ lockBusy: true, timer: 'inactive' });
    const receipt = { version: 1, id: 'recovery-failure', ownerPid: 1234,
      timer: INTERLOCK_TIMER, service: INTERLOCK_SERVICE,
      lockPath: join(f.runtimeDir, 'agent-browser-runtime-interlock.lock'), priorTimerState: 'active' };
    fs.writeFileSync(f.receiptPath, JSON.stringify(receipt));
    assert.throws(() => acquire({ ...f, recoverReceiptId: receipt.id, ownerIsAlive: () => false }), /command failed/);
    assert.equal(f.timer(), 'inactive', 'failed recovery cannot rearm timer');
    assert.deepEqual(JSON.parse(fs.readFileSync(f.receiptPath)), receipt);
    assert.ok(!f.calls.some(call => call[2] === 'start'));
  }
  for (const initial of ['active', 'inactive']) {
    const f = fixture({ timer: initial });
    const guard = acquire(f);
    assert.equal(f.timer(), 'inactive');
    assert.equal(JSON.parse(fs.readFileSync(f.receiptPath)).priorTimerState, initial);
    guard.release();
    guard.release();
    assert.equal(f.timer(), initial);
    assert.equal(fs.existsSync(f.receiptPath), false);
  }
  for (const service of ['active', 'activating', 'deactivating']) {
    const f = fixture({ service });
    assert.throws(() => acquire(f), /service is busy/);
    assert.equal(fs.existsSync(f.receiptPath), false);
    assert.ok(f.calls.every((c) => c[2] === 'show'));
  }
  for (const options of [{ lockBusy: true }, { stopFails: true }]) {
    const f = fixture(options);
    assert.throws(() => acquire(f), /command failed/);
    assert.equal(f.timer(), 'active');
    assert.equal(fs.existsSync(f.receiptPath), false);
  }
  {
    const f = fixture();
    const realRunner = f.runCommand;
    f.runCommand = (bin, args, options) => args.includes('--property=ExecStart') ?
      { status: 0, stdout: `{ path=/bin/echo ; argv[]=/bin/echo --nonblock ${f.runtimeDir}/agent-browser-runtime-interlock.lock pnpm converge ; }` } :
      realRunner(bin, args, options);
    assert.throws(() => acquire(f), /expected nonblocking lock/);
    assert.equal(f.timer(), 'active');
    assert.equal(fs.existsSync(f.receiptPath), false);
  }
  {
    const f = fixture({ lockBusy: true, startFails: true });
    assert.throws(() => acquire(f), AggregateError);
    assert.equal(fs.existsSync(f.receiptPath), true, 'restore failure keeps durable custody intent');
    assert.equal(f.timer(), 'inactive');
  }
  {
    const f = fixture({ startFails: true });
    const guard = acquire(f);
    assert.throws(() => guard.release(), /command failed/);
    assert.equal(fs.existsSync(f.receiptPath), true);
    assert.throws(() => acquire(f), /explicit recovery/);
    assert.throws(() => acquire({ ...f, recoverReceiptId: guard.receipt.id }), /owner is still alive/);
  }
  {
    const f = fixture();
    const original = acquire(f);
    original.retainForRecovery();
    assert.equal(f.timer(), 'inactive');
    assert.equal(fs.existsSync(f.receiptPath), true);
    original.release();
    assert.equal(f.timer(), 'inactive', 'retained handle cannot later restore the timer');
    // Emulate a completed failed publisher using an injected dead-owner check.
    const recovered = acquire({ ...f, recoverReceiptId: original.receipt.id, ownerIsAlive: () => false });
    assert.equal(recovered.receipt.priorTimerState, 'active');
    recovered.release();
    assert.equal(f.timer(), 'active');
    assert.equal(fs.existsSync(f.receiptPath), false);
  }
  if (process.platform === 'linux') {
    const f = fixture({ realFlock: true });
    const guard = acquire(f);
    const lock = join(f.runtimeDir, 'agent-browser-runtime-interlock.lock');
    assert.equal(spawnSync('flock', ['--nonblock', lock, 'true']).status, 1,
      'parent retains lock after synchronous flock child exits');
    await Promise.resolve();
    assert.equal(spawnSync('flock', ['--nonblock', lock, 'true']).status, 1);
    guard.release();
    assert.equal(spawnSync('flock', ['--nonblock', lock, 'true']).status, 0);
  }
  console.log('Local dashboard publication interlock tests passed (mock systemd, isolated lock).');
} finally {
  for (const directory of directories) fs.rmSync(directory, { recursive: true, force: true });
}
