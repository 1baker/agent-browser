#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
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

const root = mkdtempSync(join(tmpdir(), 'agent-browser-dashboard-publication-operations-'));
const script = resolve('scripts/publish-local-dashboard-runtime.js');
const installBin = join(root, '.local', 'bin', 'agent-browser');

try {
  const status = runPublisher(['--journal-status', '--json']);
  assert.equal(status.result.status, 0, status.result.stderr || status.result.stdout);
  assert.equal(status.payload.success, true);
  assert.equal(status.payload.operation, 'journal_status');
  assert.equal(status.payload.publicationJournalStatus.exists, false);
  assert.equal(status.payload.publicationJournalStatus.recommendedAction, 'none');
  assert.deepEqual(Object.keys(status.payload).sort(), [
    'operation',
    'publicationJournalStatus',
    'success',
  ]);
  assert.equal(
    existsSync(join(root, '.agent-browser')),
    false,
    'read-only journal status must not create runtime state',
  );
  assert.doesNotMatch(status.result.stderr, /build:dashboard|cargo build|dashboard start/);

  const recover = runPublisher(['--recover-only', '--install-bin', installBin, '--json']);
  assert.equal(recover.result.status, 0, recover.result.stderr || recover.result.stdout);
  assert.equal(recover.payload.success, true);
  assert.equal(recover.payload.operation, 'recover_only');
  assert.deepEqual(recover.payload.recovery, {
    transactionId: null,
    result: 'nothing_to_recover',
    terminalPhase: null,
  });
  assert.equal(existsSync(installBin), false, 'recovery-only no-op must not create an installed binary');
  assert.equal(
    existsSync(join(root, '.agent-browser', 'publications', 'local-dashboard-publication.json')),
    false,
  );
  assert.equal(
    existsSync(join(root, '.agent-browser', 'publications', 'local-dashboard-publication.json.lock')),
    false,
  );
  assert.doesNotMatch(recover.result.stderr, /build:dashboard|cargo build|dashboard start/);

  const retainedStateBefore = runtimeTree();
  const retained = runPublisher([
    '--retained-browser-status',
    '--expect-retained-session',
    'missing-retained-session',
    '--expect-retained-target',
    'missing-target',
    '--expect-retained-url',
    'https://example.test/conversation',
    '--json',
  ]);
  assert.equal(retained.result.status, 1);
  assert.equal(retained.payload.success, false);
  assert.equal(retained.payload.operation, 'retained_browser_status');
  assert.equal(
    retained.payload.retainedBrowserExpectation.before.reason,
    'retained_daemon_missing',
  );
  assert.equal(retained.payload.browserSmoke.requested, false);
  assert.equal(retained.payload.browserSmoke.status, 'skipped');
  assert.deepEqual(
    runtimeTree(),
    retainedStateBefore,
    'retained browser status must not change runtime state',
  );
  assert.doesNotMatch(retained.result.stderr, /build:dashboard|cargo build|dashboard start/);

  const requirementPath = join(
    root,
    '.agent-browser',
    'publications',
    'local-dashboard-retained-browser.json',
  );
  mkdirSync(join(root, '.agent-browser', 'publications'), { recursive: true, mode: 0o700 });
  writeFileSync(requirementPath, `${JSON.stringify({
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-requirement.v1',
    createdAt: '2026-08-15T12:00:00.000Z',
    expectation: {
      sessionName: 'durable-retained-session',
      profileId: 'durable-profile',
      targetId: 'durable-target',
      url: 'https://example.test/durable-conversation',
    },
  }, null, 2)}\n`, { mode: 0o600 });
  chmodSync(requirementPath, 0o600);
  const durableStateBefore = runtimeTree();

  const durableStatus = runPublisher(['--retained-browser-status', '--json']);
  assert.equal(durableStatus.result.status, 1);
  assert.equal(durableStatus.payload.success, false);
  assert.equal(
    durableStatus.payload.retainedBrowserExpectation.before.expected.sessionName,
    'durable-retained-session',
  );
  assert.equal(durableStatus.payload.retainedBrowserRequirement.exists, true);
  assert.equal(durableStatus.payload.retainedBrowserRequirement.path, requirementPath);
  assert.deepEqual(runtimeTree(), durableStateBefore);
  assert.doesNotMatch(durableStatus.result.stderr, /build:dashboard|cargo build|dashboard start/);

  const guardedPublish = runPublisher([
    '--install-bin',
    installBin,
    '--skip-smoke',
    '--json',
  ]);
  assert.equal(guardedPublish.result.status, 1);
  assert.equal(guardedPublish.payload.operation, 'publish');
  assert.ok(
    guardedPublish.payload.retainedBrowserExpectation,
    JSON.stringify(guardedPublish.payload, null, 2),
  );
  assert.equal(
    guardedPublish.payload.retainedBrowserExpectation.before.reason,
    'retained_daemon_missing',
  );
  assert.deepEqual(runtimeTree(), durableStateBefore);
  assert.doesNotMatch(guardedPublish.result.stderr, /build:dashboard|cargo build|dashboard start/);

  const durableConflict = runPublisher([
    '--retained-browser-status',
    '--expect-retained-session',
    'durable-retained-session',
    '--expect-retained-target',
    'different-target',
    '--json',
  ], { parse: false });
  assert.equal(durableConflict.result.status, 2);
  assert.match(durableConflict.result.stderr, /conflicts with durable requirement/);

  const conflicting = runPublisher(['--journal-status', '--recover-only', '--json'], { parse: false });
  assert.equal(conflicting.result.status, 2);
  assert.match(conflicting.result.stderr, /cannot be combined/);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard publication status and recovery-only fixture passed');

function runPublisher(args, { parse = true } = {}) {
  const result = spawnSync(process.execPath, [script, ...args], {
    cwd: resolve('.'),
    env: {
      ...process.env,
      HOME: root,
    },
    encoding: 'utf8',
    timeout: 10_000,
    maxBuffer: 4 * 1024 * 1024,
  });
  return {
    result,
    payload: parse ? JSON.parse(result.stdout) : null,
  };
}

function runtimeTree() {
  const agentRoot = join(root, '.agent-browser');
  return existsSync(agentRoot)
    ? readdirSync(agentRoot, { recursive: true }).map(String).sort()
    : [];
}
