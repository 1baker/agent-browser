// Executes the production entrypoint with an isolated HOME and fake browser
// executables. Never connects to a browser, DBus, desktop or provider.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync,
  rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { probeDependencies, recoveryEnvironment } from './workshop-boot-recovery.mjs';

const source = fileURLToPath(new URL('./workshop-boot-recovery.mjs', import.meta.url));
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const json = (path) => JSON.parse(readFileSync(path, 'utf8'));
const write = (path, value) => writeFileSync(path, JSON.stringify(value), { mode: 0o600 });
const expected = { sessionName: 'chatgpt-pro', profileId: 'chatgpt-pro', targetId: 'OLD',
  url: 'https://chatgpt.com/g/g-p-00000000000000000000000000000000/c/00000000-0000-0000-0000-000000000000' };

const fakeBinary = `#!/usr/bin/env node
const fs = require('node:fs');
const crypto = require('node:crypto');
const root = process.env.HOME;
const dir = root + '/.agent-browser';
const args = process.argv.slice(2);
const reqPath = dir + '/publications/local-dashboard-retained-browser.json';
const req = () => JSON.parse(fs.readFileSync(reqPath, 'utf8'));
const log = (kind) => fs.appendFileSync(root + '/events', JSON.stringify({ kind, args }) + '\\n');
if (args.includes('retained-browser-status')) {
  log('status');
  if (fs.existsSync(root + '/healthy')) {
    console.log(JSON.stringify({ success: true, retainedBrowserRequirement: { verified: true } }));
  } else {
    console.log(JSON.stringify({ success: false, error: 'Retained browser requirement verification failed: retained_daemon_missing' }));
    process.exitCode = 1;
  }
} else if (args[0] === 'service' && args[1] === 'reconcile') {
  log('reconcile'); console.log(JSON.stringify({ success: true }));
} else if (args.includes('prepare-retained-browser')) {
  const pending = JSON.parse(fs.readFileSync(dir + '/workshop-boot-recovery/state.json', 'utf8'));
  if (!pending.attempt) throw Error('effect_without_durable_receipt');
  log('prepare');
  const value = req();
  if (args[args.indexOf('--url') + 1] !== value.expectation.url) throw Error('wrong_url');
  if (args[args.indexOf('--runtime-profile') + 1] !== value.expectation.profileId) throw Error('wrong_profile');
  const oldHash = crypto.createHash('sha256').update(fs.readFileSync(reqPath)).digest('hex');
  if (args[args.indexOf('--rotate-stale-requirement-sha256') + 1] !== oldHash) throw Error('wrong_digest');
  value.expectation.targetId = 'NEW';
  fs.writeFileSync(reqPath, JSON.stringify(value));
  const markerPath = reqPath + '.required';
  const marker = JSON.parse(fs.readFileSync(markerPath, 'utf8'));
  marker.requirementSha256 = crypto.createHash('sha256').update(fs.readFileSync(reqPath)).digest('hex');
  fs.writeFileSync(markerPath, JSON.stringify(marker));
  fs.writeFileSync(root + '/healthy', 'yes');
  if (fs.existsSync(root + '/interrupt-after-commit')) process.exit(17);
  console.log(JSON.stringify({ success: true }));
} else throw Error('unexpected_browser_action');
`;

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'workshop-recovery-cli-'));
  const scripts = join(root, '.local/lib/agent-browser/0.28.0/scripts');
  const bin = join(root, '.local/bin');
  const reqPath = join(root, '.agent-browser/publications/local-dashboard-retained-browser.json');
  const helper = join(root, '.local/lib/agent-browser-workshop-recovery/workshop-boot-recovery.mjs');
  for (const dir of [scripts, bin, dirname(reqPath), dirname(helper)]) mkdirSync(dir, { recursive: true });
  writeFileSync(helper, readFileSync(source));
  writeFileSync(join(bin, 'agent-browser'), fakeBinary, { mode: 0o700 });
  symlinkSync(process.execPath, join(bin, 'node'));
  const dependencyCommand = `#!/usr/bin/env node
const fs = require('node:fs');
const command = require('node:path').basename(process.argv[1]);
const root = process.env.HOME;
const state = JSON.parse(fs.readFileSync(root + '/.agent-browser/workshop-boot-recovery/state.json'));
if (state.attempt) throw Error('readiness_after_attempt');
fs.appendFileSync(root + '/events', JSON.stringify({kind:'dependency', command})+'\\n');
if (command === 'sudo' && fs.existsSync(root + '/dependency-refused')) process.exit(1);
if (command === 'systemctl' && process.argv.includes('xrdp.service') && fs.existsSync(root + '/xrdp-unavailable')) process.exit(3);
if (command === 'systemctl' && fs.existsSync(root + '/dependency-delayed')) {
  fs.unlinkSync(root + '/dependency-delayed'); process.exit(3);
}
if (command === 'curl') process.stdout.write('200');
if (command === 'docker' && process.argv[2] === 'inspect') process.stdout.write('true\\ntrue\\ntrue\\n');
`;
  for (const command of ['systemctl', 'docker', 'curl', 'sudo']) {
    writeFileSync(join(bin, command), dependencyCommand, { mode: 0o700 });
  }
  writeFileSync(join(scripts, 'open-rdp-guac-route-displays.js'),
    `const fs = require('node:fs');
fs.appendFileSync(process.env.HOME + '/events', JSON.stringify({kind:'desktop'})+'\\n');
if (fs.existsSync(process.env.HOME + '/desktop-failure')) {
  console.log(JSON.stringify({success:false,error:'open Guacamole route A failed using PRIVATE_TOKEN_DO_NOT_LOG'}));
  process.exit(1);
}
console.log('{"success":true}');`);
  writeFileSync(join(scripts, 'grant-rdp-route-display-access.sh'), 'exit 0\n');
  write(reqPath, { schemaVersion: 'agent-browser.local-dashboard-retained-browser-requirement.v1',
    createdAt: new Date().toISOString(), expectation: expected });
  write(reqPath + '.required', { schemaVersion: 'agent-browser.local-dashboard-retained-browser-enforcement.v1',
    requirementSha256: digest(readFileSync(reqPath)) });
  writeFileSync(join(root, 'healthy'), 'yes');
  const statePath = join(root, '.agent-browser/workshop-boot-recovery/state.json');
  const execute = (arg = '--run') => {
    const result = spawnSync(process.execPath, [helper, arg], {
      env: { HOME: root, PATH: '/usr/bin:/bin' }, encoding: 'utf8', timeout: 15000,
    });
    assert.equal(result.error, undefined);
    return { code: result.status, output: result.stdout + result.stderr };
  };
  const arm = execute('--arm'); assert.equal(arm.code, 0, arm.output);
  const current = json(statePath);
  const [kernel, ticks] = current.boot.split(':');
  const archivedTicks = BigInt(ticks) === 1n ? '2' : String(BigInt(ticks) - 1n);
  write(statePath, { ...current, boot: kernel + ':' + archivedTicks });
  rmSync(join(root, 'healthy'));
  writeFileSync(join(root, 'events'), '');
  return { root, execute, statePath, reqPath,
    events: () => readFileSync(join(root, 'events'), 'utf8').trim().split('\n').filter(Boolean).map(JSON.parse),
    cleanup: () => rmSync(root, { recursive: true }),
  };
}

test('desktop failure emits safe category and preserves receipt against replay', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, 'desktop-failure'), 'yes');
    const result = f.execute();
    assert.equal(result.code, 1);
    const response = JSON.parse(result.output);
    assert.equal(response.error, 'desktop_restore_failed');
    assert.equal(response.diagnostic.category, 'route_viewer_open_failed');
    assert.equal(response.diagnostic.exitCode, 1);
    assert.ok(!result.output.includes('PRIVATE_TOKEN_DO_NOT_LOG'));
    const receipt = readFileSync(f.statePath, 'utf8');
    assert.ok(json(f.statePath).attempt);
    const retry = f.execute();
    assert.equal(retry.code, 1);
    assert.match(retry.output, /uncertain_attempt_requires_review/);
    assert.equal(readFileSync(f.statePath, 'utf8'), receipt);
    assert.equal(f.events().filter((event) => event.kind === 'desktop').length, 1);
    assert.equal(f.events().filter((event) => event.kind === 'prepare').length, 0);
  } finally { f.cleanup(); }
});

test('production CLI recovers a WSL-style epoch change and repeated run has no effects', () => {
  const f = fixture();
  try {
    const result = f.execute(); assert.equal(result.code, 0, result.output);
    assert.match(result.output, /recovered_and_verified/);
    assert.deepEqual(f.events().filter((event) => event.kind !== 'dependency').map((event) => event.kind),
      ['status', 'status', 'desktop', 'reconcile', 'prepare', 'status']);
    assert.equal(json(f.statePath).attempt, null);
    assert.equal(json(f.statePath).digest, digest(readFileSync(f.reqPath)));
    assert.equal(f.execute().code, 0);
    assert.equal(f.events().filter((event) => event.kind === 'prepare').length, 1);
  } finally { f.cleanup(); }
});

test('production CLI waits for slow startup before any effect receipt', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, 'dependency-delayed'), 'yes');
    const result = f.execute(); assert.equal(result.code, 0, result.output);
    assert.equal(f.events().filter((e) => e.kind === 'dependency' && e.command === 'systemctl').length, 4);
    assert.equal(f.events().filter((e) => e.kind === 'prepare').length, 1);
  } finally { f.cleanup(); }
});

test('production CLI dependency refusal leaves no uncertain attempt or browser effect', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, 'dependency-refused'), 'yes');
    const result = f.execute();
    assert.equal(result.code, 1); assert.match(result.output, /dependency_preflight_refused/);
    assert.equal(json(f.statePath).attempt, null);
    assert.equal(f.events().filter((e) => ['desktop', 'reconcile', 'prepare'].includes(e.kind)).length, 0);
  } finally { f.cleanup(); }
});

test('Docker ready cannot conceal unavailable XRDP', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, 'xrdp-unavailable'), 'yes');
    const result = probeDependencies(recoveryEnvironment(f.root, 'operator',
      join(f.root, '.local/lib/agent-browser/0.28.0')), 5000);
    assert.equal(result.ready, false);
    assert.deepEqual(result.missing, ['xrdp.service']);
    assert.equal(json(f.statePath).attempt, null);
  } finally { f.cleanup(); }
});

test('production CLI does not replay after committed effect loses its response', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, 'interrupt-after-commit'), 'yes');
    assert.equal(f.execute().code, 1);
    assert.ok(json(f.statePath).attempt);
    const retry = f.execute();
    assert.equal(retry.code, 1); assert.match(retry.output, /uncertain_attempt/);
    assert.equal(f.events().filter((event) => event.kind === 'prepare').length, 1);
  } finally { f.cleanup(); }
});

test('production CLI rejects private-state permission drift before effects', () => {
  const f = fixture();
  try {
    chmodSync(f.statePath, 0o644);
    assert.match(f.execute().output, /unsafe_private_file/);
    assert.equal(f.events().length, 0);
  } finally { f.cleanup(); }
});

test('production CLI refuses added configuration and preserves the receipt', () => {
  const f = fixture();
  try {
    writeFileSync(join(f.root, '.agent-browser/.env'), 'AGENT_BROWSER_RETAINED_PINNER_SCRIPT=unreviewed');
    assert.match(f.execute().output, /installed_artifacts_changed/);
    assert.equal(f.events().length, 0);
    assert.equal(json(f.statePath).attempt, null);
    assert.equal(existsSync(join(dirname(f.statePath), 'lock')), false);
  } finally { f.cleanup(); }
});
