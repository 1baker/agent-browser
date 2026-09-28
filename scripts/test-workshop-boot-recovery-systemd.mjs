// Opt-in real user-systemd lifecycle test. Only disposable sleep processes are
// launched; no browser, recovery receipt, installed unit or provider is touched.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const source = readFileSync(new URL('./agent-browser-workshop-recovery.service', import.meta.url), 'utf8');
const setting = (name) => source.match(new RegExp(`^${name}=(.+)$`, 'm'))?.[1];
const env = { ...process.env, XDG_RUNTIME_DIR: `/run/user/${process.getuid()}`,
  DBUS_SESSION_BUS_ADDRESS: `unix:path=/run/user/${process.getuid()}/bus` };
const command = (program, args) => spawnSync(program, args, {
  env, encoding: 'utf8', timeout: 15000,
});
const alive = (pid) => {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[0] !== 'Z';
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
};

test('recovery unit retains successful children but preserves stop cleanup', () => {
  assert.equal(setting('Type'), 'oneshot');
  assert.equal(setting('RemainAfterExit'), 'yes');
  assert.equal(setting('KillMode'), 'control-group');
  assert.equal(setting('Restart'), 'no');
});

for (const scenario of [
  { name: 'old oneshot kills detached child at helper exit', remain: 'no', fail: false, survives: false },
  { name: 'fixed oneshot retains child after exit and repeated start; stop cleans up', remain: setting('RemainAfterExit'), fail: false, survives: true },
  { name: 'failed helper still cleans up child without restart', remain: setting('RemainAfterExit'), fail: true, survives: false },
]) {
  test(scenario.name, { skip: process.env.WORKSHOP_SYSTEMD_TEST !== '1' }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'workshop-systemd-fixture-'));
    const unit = `workshop-lifetime-fixture-${process.pid}-${Date.now()}.service`;
    const pidPath = join(root, 'child.pid');
    let childPid;
    // A detached child reproduces daemonization without escaping the cgroup.
    const script = `const {spawn}=require('node:child_process');
      const fs=require('node:fs');
      if(fs.existsSync(process.argv[1])) process.exit(23);
      const child=spawn('/bin/sleep',['120'],{detached:true,stdio:'ignore'});
      fs.writeFileSync(process.argv[1],String(child.pid));child.unref();
      process.exitCode=${scenario.fail ? 17 : 0};`;
    try {
      const start = command('systemd-run', ['--user', '--quiet', `--unit=${unit}`,
        `--property=Type=${setting('Type')}`, `--property=RemainAfterExit=${scenario.remain}`,
        `--property=KillMode=${setting('KillMode')}`, `--property=Restart=${setting('Restart')}`,
        '--property=TimeoutStartSec=10s', '--property=TimeoutStopSec=5s',
        process.execPath, '-e', script, pidPath]);
      if (scenario.fail) assert.notEqual(start.status, 0, start.stderr);
      else assert.equal(start.status, 0, start.stderr);
      const pid = Number(readFileSync(pidPath, 'utf8'));
      childPid = pid;
      assert.ok(Number.isSafeInteger(pid) && pid > 1);
      await delay(1000);
      assert.equal(alive(pid), scenario.survives);
      if (scenario.survives) {
        const show = command('systemctl', ['--user', 'show', unit,
          '-p', 'ActiveState', '-p', 'SubState', '-p', 'ExecMainStatus']);
        assert.equal(show.status, 0, show.stderr);
        assert.match(show.stdout, /ActiveState=active/);
        assert.match(show.stdout, /SubState=exited/);
        assert.match(show.stdout, /ExecMainStatus=0/);
        assert.match(readFileSync(`/proc/${pid}/cgroup`, 'utf8'), new RegExp(unit.replaceAll('.', '\\.')));
        const again = command('systemctl', ['--user', 'start', unit]);
        assert.equal(again.status, 0, again.stderr);
        assert.equal(Number(readFileSync(pidPath, 'utf8')), pid);
        assert.equal(alive(pid), true);
        const stop = command('systemctl', ['--user', 'stop', unit]);
        assert.equal(stop.status, 0, stop.stderr);
        assert.equal(alive(pid), false);
      }
    } finally {
      const cleanup = command('systemctl', ['--user', 'stop', unit]);
      command('systemctl', ['--user', 'reset-failed', unit]);
      rmSync(root, { recursive: true, force: true });
      // Successful transient units can already have been garbage-collected.
      assert.ok(cleanup.status === 0 || (cleanup.status === 5 && /not loaded/.test(cleanup.stderr)), cleanup.stderr);
      if (childPid) assert.equal(alive(childPid), false, 'fixture child survived cleanup');
    }
  });
}
