import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { recover, recoveryEnvironment, artifactManifest, preparationArgs, distributionEpoch, waitForDependencies, commandFailureDetails } from './workshop-boot-recovery.mjs';

test('child failure diagnostics classify without leaking arbitrary output', () => {
  const secret = 'PRIVATE_TOKEN_DO_NOT_LOG';
  const result = commandFailureDetails({ status: 1, stdout: JSON.stringify({
    error: `guacamole_route_a_login_failed: ${secret}`, token: secret,
  }), stderr: secret });
  assert.equal(result.category, 'guacamole_login_failed');
  assert.equal(result.exitCode, 1);
  assert.equal(result.jsonResponse, true);
  assert.ok(!JSON.stringify(result).includes(secret));
  for (const stdout of [secret, JSON.stringify({ error: secret }), 'null']) {
    const other = commandFailureDetails({ stdout, stderr: secret, status: null,
      signal: secret, error: { code: 'ETIMEDOUT', message: secret } });
    assert.equal(other.spawnError, 'ETIMEDOUT');
    assert.equal(other.category, 'unclassified_child_failure');
    assert.ok(!JSON.stringify(other).includes(secret));
  }
});

const kernel = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const before = `${kernel}:100`;
const after = `${kernel}:200`;
const identity = { profileId: 'chatgpt-pro', sessionName: 'chatgpt-pro', url: 'exact-url' };
function fixture(overrides = {}) {
  let saved;
  let effects = 0;
  return {
    args: { state: { schema: 'agent-browser.workshop-boot-recovery.v1', boot: before,
      identity, digest: 'old', attempt: null }, boot: after,
    requirement: { expectation: identity, digest: 'old' },
    status: { verified: false, reason: 'retained_daemon_missing' },
    save: (s) => { saved = s; }, effects: () => { effects++; assert.ok(saved.attempt); },
    verify: () => ({ verified: true, expectation: identity, digest: 'new' }), ...overrides },
    saved: () => saved, count: () => effects,
  };
}
test('fresh boot runs once and commits exact identity', async () => {
  const f = fixture();
  assert.equal(await recover(f.args), 'recovered_and_verified');
  assert.equal(f.count(), 1);
  assert.equal(f.saved().digest, 'new');
  assert.equal(f.saved().attempt, null);
});
test('healthy target requires zero effects', async () => {
  const f = fixture({ status: { verified: true } });
  assert.equal(await recover(f.args), 'verified_no_action');
  assert.equal(f.count(), 0);
});
test('same boot dead target is refused', async () => {
  const f = fixture({ boot: before });
  await assert.rejects(recover(f.args), /same_boot/);
  assert.equal(f.count(), 0);
});
for (const field of ['profileId', 'sessionName', 'url']) {
  test(`changed ${field} never dispatches`, async () => {
    const f = fixture({ requirement: { digest: 'old', expectation: { ...identity, [field]: 'changed' } } });
    await assert.rejects(recover(f.args), /requirement_changed/);
    assert.equal(f.count(), 0);
  });
}
test('changed digest and unproven authority never dispatch', async () => {
  for (const override of [{ requirement: { digest: 'changed', expectation: identity } },
    { status: { verified: false, reason: 'unreadable' } }]) {
    const f = fixture(override);
    await assert.rejects(recover(f.args));
    assert.equal(f.count(), 0);
  }
});
test('crash after dispatch cannot replay, even on another boot', async () => {
  const f = fixture({ effects: () => { throw Error('connection_lost'); } });
  await assert.rejects(recover(f.args), /connection_lost/);
  for (const boot of [after, 'cccccccc-cccc-cccc-cccc-cccccccccccc:300']) {
    const retry = fixture({ state: f.saved(), boot });
    await assert.rejects(recover(retry.args), /uncertain_attempt/);
    assert.equal(retry.count(), 0);
  }
});
test('same kernel with restarted distro is a new recovery epoch', () => {
  const stat = (ticks) => `1 (systemd (init)) S ${Array(18).fill('0').join(' ')} ${ticks} 0 0`;
  assert.equal(distributionEpoch(kernel, stat('100')), before);
  assert.equal(distributionEpoch(kernel, stat('200')), after);
  assert.throws(() => distributionEpoch(kernel, '1 (systemd) S'), /invalid/);
  assert.throws(() => distributionEpoch('bad', stat('100')), /invalid/);
  assert.throws(() => distributionEpoch(kernel, stat('0')), /invalid/);
});
test('full kernel reboot also admits recovery', async () => {
  const f = fixture({ boot: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb:100' });
  assert.equal(await recover(f.args), 'recovered_and_verified');
  assert.equal(f.count(), 1);
});
test('legacy kernel-only state cannot authorize a dead target recovery', async () => {
  const f = fixture(); f.args.state.boot = kernel;
  await assert.rejects(recover(f.args), /legacy_epoch/);
  assert.equal(f.count(), 0);
});
test('wrong rendered identity leaves durable uncertainty', async () => {
  const f = fixture({ verify: () => ({ verified: true, digest: 'new', expectation: { ...identity, url: 'wrong' } }) });
  await assert.rejects(recover(f.args), /unproven/);
  assert.ok(f.saved().attempt);
});
test('receipt failure prevents all browser effects', async () => {
  const f = fixture({ save: () => { throw Error('disk_failure'); } });
  await assert.rejects(recover(f.args), /disk_failure/);
  assert.equal(f.count(), 0);
});
test('child environment never inherits loader or identity overrides', () => {
  const env = recoveryEnvironment('/home/operator', 'operator', '/trusted/support');
  for (const key of ['NODE_OPTIONS', 'NODE_PATH', 'BASH_ENV', 'ENV', 'LD_PRELOAD',
    'LD_LIBRARY_PATH', 'GUACAMOLE_ADMIN_USERNAME', 'REMOTE_USER']) assert.equal(env[key], undefined);
  assert.equal(env.AGENT_BROWSER_RETAINED_PINNER_SCRIPT, '');
  assert.equal(env.USER, 'operator');
  assert.equal(env.PATH, '/home/operator/.local/bin:/usr/local/bin:/usr/bin:/bin');
});
test('artifact admission detects new configuration and new imported support files', () => {
  const root = mkdtempSync(join(tmpdir(), 'workshop-boot-fixture-'));
  try {
    const scripts = join(root, 'scripts'); mkdirSync(scripts);
    const config = join(root, '.env');
    const initial = artifactManifest([config], scripts);
    assert.deepEqual(initial, [[config, null]]);
    writeFileSync(config, 'new');
    const addedConfig = artifactManifest([config], scripts);
    assert.notDeepEqual(initial, addedConfig);
    writeFileSync(join(scripts, 'new.js'), 'new import');
    assert.notDeepEqual(addedConfig, artifactManifest([config], scripts));
  } finally { rmSync(root, { recursive: true }); }
});
test('production dispatch is exact navigation-only with stale digest', () => {
  const args = preparationArgs({ identity, digest: 'old' });
  assert.deepEqual(args, ['install', 'workstation', 'prepare-retained-browser',
    '--url', 'exact-url', '--url-prefix', 'exact-url', '--runtime-profile', 'chatgpt-pro',
    '--rotate-stale-requirement-sha256', 'old', '--json']);
});
test('readiness timeout leaves the durable recovery attempt untouched', async () => {
  let clock = 0;
  const f = fixture({ beforeEffects: () => waitForDependencies({ timeoutMs: 100,
    now: () => clock, sleep: (ms) => { clock += ms; },
    probe: () => ({ ready: false, missing: ['database'] }),
  }) });
  await assert.rejects(recover(f.args), /readiness_timeout_no_recovery_attempt/);
  assert.equal(f.saved(), undefined);
  assert.equal(f.count(), 0);
});
test('target restored by another caller during readiness is not opened again', async () => {
  const f = fixture({ beforeEffects: async () => ({ status: { verified: true } }) });
  assert.equal(await recover(f.args), 'verified_no_action');
  assert.equal(f.count(), 0);
});
test('authority loss after readiness blocks all effects', async () => {
  const f = fixture({ beforeEffects: async () => ({ status: { verified: false, reason: 'unreadable' } }) });
  await assert.rejects(recover(f.args), /unproven_after_readiness/);
  assert.equal(f.saved(), undefined);
  assert.equal(f.count(), 0);
});
