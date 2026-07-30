#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const path = 'scripts/vm/fresh-workstation-vm.sh';
const source = readFileSync(path, 'utf8');

for (const command of ['prepare', 'start', 'wait', 'stage', 'status', 'stop', 'reset']) {
  assert.match(source, new RegExp(`\\b${command}\\b`));
}
for (const contract of [
  'AGENT_BROWSER_VM_STATE_DIR',
  'AGENT_BROWSER_VM_BASE_IMAGE',
  'AGENT_BROWSER_VM_CANDIDATE',
  'AGENT_BROWSER_VM_QEMU_FIRMWARE_DIR',
  'AGENT_BROWSER_VM_QEMU_BIOS_PATH',
  'Refusing to reset a running VM',
  '127.0.0.1',
  'workstation --apply --json',
]) {
  assert.match(source, new RegExp(contract.replaceAll('-', '\\-')));
}
assert.doesNotMatch(source, /rm -rf/);
assert.doesNotMatch(source, /hostfwd=tcp:0\.0\.0\.0/);

const help = spawnSync('bash', [path, '--help'], { encoding: 'utf8' });
assert.equal(help.status, 0, help.stderr);
assert.match(help.stdout, /operator-visible|visible SSH/);

const broadReset = spawnSync('bash', [path, 'reset'], {
  encoding: 'utf8',
  env: {
    ...process.env,
    AGENT_BROWSER_VM_STATE_DIR: '/',
    AGENT_BROWSER_VM_BASE_IMAGE: '/does/not/exist',
  },
});
assert.notEqual(broadReset.status, 0);
assert.match(broadReset.stderr, /explicit narrow directory/);

console.log('Fresh workstation VM harness contract passed');
