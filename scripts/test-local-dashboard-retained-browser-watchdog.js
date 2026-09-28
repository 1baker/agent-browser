#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const home = mkdtempSync(join(tmpdir(), 'agent-browser-retained-watchdog-'));
const script = resolve('scripts/check-local-dashboard-retained-browser.js');
const requirement = join(
  home,
  '.agent-browser',
  'publications',
  'local-dashboard-retained-browser.json',
);
const enforcement = `${requirement}.required`;

try {
  const absent = run(['--json']);
  assert.equal(absent.status, 0, absent.stderr);
  assert.equal(JSON.parse(absent.stdout).status, 'not_configured');
  assert.equal(existsSync(join(home, '.agent-browser')), false);

  const unknown = run(['--unknown', '--json']);
  assert.equal(unknown.status, 2);
  assert.match(unknown.stderr, /Unknown argument/);
  assert.equal(existsSync(join(home, '.agent-browser')), false);

  mkdirSync(join(home, '.agent-browser', 'publications'), { recursive: true, mode: 0o700 });
  writeFileSync(enforcement, `${JSON.stringify({
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-enforcement.v1',
    createdAt: '2026-08-15T12:00:00.000Z',
    requirementSha256: '0'.repeat(64),
  })}\n`, { mode: 0o600 });
  chmodSync(enforcement, 0o600);
  const enforcedMissingBefore = runtimeTree();
  const enforcedMissing = run(['--json']);
  assert.equal(enforcedMissing.status, 2);
  assert.match(enforcedMissing.stderr, /Required retained browser requirement is missing/);
  assert.deepEqual(runtimeTree(), enforcedMissingBefore);
  assert.doesNotMatch(enforcedMissing.stderr, /build:dashboard|cargo build|dashboard start/);

  writeFileSync(requirement, '{}\n', { mode: 0o600 });
  chmodSync(requirement, 0o600);
  const stateBefore = runtimeTree();
  const invalid = run(['--json']);
  assert.equal(invalid.status, 2);
  assert.match(invalid.stderr, /Unsupported retained browser requirement schema/);
  assert.deepEqual(runtimeTree(), stateBefore);
  assert.doesNotMatch(invalid.stderr, /build:dashboard|cargo build|dashboard start/);

  const validRequirement = `${JSON.stringify({
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-requirement.v1',
    createdAt: '2026-08-15T12:00:00.000Z',
    expectation: {
      sessionName: 'watchdog-fixture',
      profileId: 'watchdog-profile',
      targetId: 'watchdog-target',
      url: 'https://example.test/watchdog',
    },
  })}\n`;
  writeFileSync(requirement, validRequirement);
  chmodSync(requirement, 0o600);
  writeFileSync(enforcement, `${JSON.stringify({
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-enforcement.v1',
    createdAt: '2026-08-15T12:00:00.000Z',
    requirementSha256: createHash('sha256').update(validRequirement).digest('hex'),
  })}\n`, { mode: 0o600 });
  chmodSync(enforcement, 0o600);
  const missing = run(['--json']);
  assert.equal(missing.status, 1);
  const payload = JSON.parse(missing.stdout);
  assert.equal(payload.retainedBrowserExpectation.before.reason, 'retained_daemon_missing');
  assert.deepEqual(runtimeTree(), stateBefore);
  assert.doesNotMatch(missing.stderr, /build:dashboard|cargo build|dashboard start/);
} finally {
  rmSync(home, { recursive: true, force: true });
}

console.log('Local dashboard retained browser watchdog fixture passed');

function run(args) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd: resolve('.'),
    env: { ...process.env, HOME: home },
    encoding: 'utf8',
    timeout: 10_000,
    maxBuffer: 4 * 1024 * 1024,
  });
}

function runtimeTree() {
  const root = join(home, '.agent-browser');
  return existsSync(root)
    ? readdirSync(root, { recursive: true }).map(String).sort()
    : [];
}
