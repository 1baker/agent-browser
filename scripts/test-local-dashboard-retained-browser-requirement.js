#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
  LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
  readRetainedBrowserRequirement,
  retainedBrowserEnforcementPath,
  retainedBrowserRotationJournalPath,
  resolveRetainedBrowserExpectation,
  rotateRetainedBrowserRequirement,
  writeRetainedBrowserRequirement,
} from './lib/local-dashboard-retained-browser-requirement.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-retained-requirement-'));
const path = join(root, 'publications', 'retained-browser.json');
const expectation = {
  sessionName: 'retained-fixture',
  profileId: 'fixture-profile',
  targetId: 'fixture-target',
  url: 'https://example.test/conversation',
};
const observed = {
  ...expectation,
  browserId: 'session:retained-fixture',
  browserPid: 4242,
  cdpUrl: 'ws://127.0.0.1:9444/devtools/browser/fixture',
  health: 'ready',
  title: 'Fixture conversation',
  cdpTargetCount: 1,
};
const evidence = {
  required: true,
  verified: true,
  stage: 'read_only_preflight',
  reason: 'retained_browser_exact_match',
  expected: expectation,
  observed,
};

try {
  assert.equal(readRetainedBrowserRequirement(path).exists, false);
  const written = writeRetainedBrowserRequirement({
    path,
    evidence,
    now: () => '2026-08-15T12:00:00.000Z',
  });
  assert.equal(written.written, true);
  assert.equal(written.schemaVersion, LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA);
  assert.deepEqual(written.expectation, { ...expectation, browserId: null, browserPid: null, cdpUrl: null });
  assert.equal(readFileSync(path, 'utf8').includes('4242'), false);
  assert.equal(readFileSync(path, 'utf8').includes('9444'), false);
  const enforcementPath = retainedBrowserEnforcementPath(path);
  assert.equal(written.enforcement.exists, true);
  assert.equal(
    written.enforcement.schemaVersion,
    LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
  );
  assert.equal(existsSync(enforcementPath), true);
  assert.equal(written.enforcement.requirementSha256, written.sha256);

  const repeated = writeRetainedBrowserRequirement({ path, evidence });
  assert.equal(repeated.written, false);
  assert.equal(
    resolveRetainedBrowserExpectation({ explicit: null, requirement: repeated }).targetId,
    'fixture-target',
  );
  assert.equal(
    resolveRetainedBrowserExpectation({
      explicit: { sessionName: 'retained-fixture' },
      requirement: repeated,
    }).profileId,
    'fixture-profile',
  );
  assert.throws(
    () => resolveRetainedBrowserExpectation({
      explicit: { ...expectation, targetId: 'other-target' },
      requirement: repeated,
    }),
    /conflicts with durable requirement/,
  );

  chmodSync(path, 0o644);
  assert.throws(() => readRetainedBrowserRequirement(path), /permissions must be 0600/);
  chmodSync(path, 0o600);
  const symlink = join(root, 'requirement-link.json');
  symlinkSync(path, symlink);
  assert.throws(() => readRetainedBrowserRequirement(symlink), /non-symlink/);
  const invalid = join(root, 'invalid.json');
  writeFileSync(invalid, '{}\n', { mode: 0o600 });
  assert.throws(() => readRetainedBrowserRequirement(invalid), /Unsupported retained browser/);
  const transient = join(root, 'transient.json');
  writeFileSync(transient, `${JSON.stringify({
    schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
    createdAt: '2026-08-15T12:00:00.000Z',
    expectation: { ...expectation, cdpUrl: observed.cdpUrl },
  })}\n`, { mode: 0o600 });
  assert.throws(
    () => readRetainedBrowserRequirement(transient),
    /only stable identity fields/,
  );
  const invalidWrite = join(root, 'invalid-write.json');
  assert.throws(
    () => writeRetainedBrowserRequirement({
      path: invalidWrite,
      evidence,
      now: () => 'not-a-timestamp',
    }),
    /valid createdAt timestamp/,
  );
  assert.equal(readRetainedBrowserRequirement(invalidWrite).exists, false);

  const crashPath = join(root, 'crash', 'retained-browser.json');
  assert.throws(
    () => writeRetainedBrowserRequirement({
      path: crashPath,
      evidence,
      now: () => '2026-08-15T12:00:00.000Z',
      afterEnforcementCommit: () => {
        throw new Error('fixture crash after enforcement');
      },
    }),
    /fixture crash after enforcement/,
  );
  assert.equal(existsSync(crashPath), false);
  assert.equal(existsSync(retainedBrowserEnforcementPath(crashPath)), true);
  const crashedEnforcement = JSON.parse(
    readFileSync(retainedBrowserEnforcementPath(crashPath), 'utf8'),
  );
  assert.equal(crashedEnforcement.createdAt, '2026-08-15T12:00:00.000Z');
  assert.throws(
    () => readRetainedBrowserRequirement(crashPath),
    /Required retained browser requirement is missing/,
  );
  assert.throws(
    () => writeRetainedBrowserRequirement({
      path: crashPath,
      evidence: {
        ...evidence,
        expected: { ...expectation, targetId: 'replacement-target' },
        observed: { ...observed, targetId: 'replacement-target' },
      },
    }),
    /enforcement conflicts with requirement commit/,
  );
  assert.equal(existsSync(crashPath), false);
  const recovered = writeRetainedBrowserRequirement({ path: crashPath, evidence });
  assert.equal(recovered.written, true);
  assert.equal(recovered.enforcement.exists, true);
  assert.equal(recovered.createdAt, crashedEnforcement.createdAt);
  assert.equal(recovered.enforcement.requirementSha256, recovered.sha256);

  const rotationPath = join(root, 'rotation', 'retained-browser.json');
  const rotationOld = writeRetainedBrowserRequirement({
    path: rotationPath,
    evidence,
    now: () => '2026-08-15T12:00:00.000Z',
  });
  const replacementEvidence = {
    ...evidence,
    expected: {
      ...expectation,
      sessionName: 'replacement-session',
      targetId: 'replacement-target',
      url: 'https://example.test/replacement',
    },
    observed: {
      ...observed,
      sessionName: 'replacement-session',
      browserId: 'session:replacement-session',
      targetId: 'replacement-target',
      url: 'https://example.test/replacement',
    },
  };
  assert.throws(
    () => rotateRetainedBrowserRequirement({
      path: rotationPath,
      evidence: replacementEvidence,
      expectedSha256: rotationOld.sha256,
      staleEvidence: { confirmed: false, reason: 'retained_daemon_missing' },
    }),
    /bounded proof that old authority is stale/,
  );
  assert.throws(
    () => rotateRetainedBrowserRequirement({
      path: rotationPath,
      evidence: replacementEvidence,
      expectedSha256: rotationOld.sha256,
      staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
      now: () => '2026-08-16T12:00:00.000Z',
      afterPhase: (phase) => {
        if (phase === 'requirement_replaced') throw new Error('fixture crash during rotation');
      },
    }),
    /fixture crash during rotation/,
  );
  assert.equal(existsSync(retainedBrowserRotationJournalPath(rotationPath)), true);
  assert.throws(
    () => readRetainedBrowserRequirement(rotationPath),
    /does not match enforcement digest/,
  );
  const rotated = rotateRetainedBrowserRequirement({
    path: rotationPath,
    evidence: replacementEvidence,
    expectedSha256: rotationOld.sha256,
    staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
    now: () => '2099-01-01T00:00:00.000Z',
  });
  assert.equal(rotated.rotated, true);
  assert.equal(rotated.previousSha256, rotationOld.sha256);
  assert.equal(rotated.expectation.sessionName, 'replacement-session');
  assert.equal(rotated.expectation.targetId, 'replacement-target');
  assert.equal(rotated.enforcement.requirementSha256, rotated.sha256);
  assert.equal(existsSync(retainedBrowserRotationJournalPath(rotationPath)), false);

  for (const phase of ['prepared', 'requirement_replaced', 'enforcement_replaced', 'committed']) {
    const phasePath = join(root, `rotation-${phase}`, 'retained-browser.json');
    const phaseOld = writeRetainedBrowserRequirement({
      path: phasePath,
      evidence,
      now: () => '2026-08-15T12:00:00.000Z',
    });
    assert.throws(
      () => rotateRetainedBrowserRequirement({
        path: phasePath,
        evidence: replacementEvidence,
        expectedSha256: phaseOld.sha256,
        staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
        now: () => '2026-08-16T12:00:00.000Z',
        afterPhase: (observedPhase) => {
          if (observedPhase === phase) throw new Error(`fixture crash at ${phase}`);
        },
      }),
      new RegExp(`fixture crash at ${phase}`),
    );
    assert.equal(existsSync(retainedBrowserRotationJournalPath(phasePath)), true);
    if (phase === 'prepared') {
      assert.throws(
        () => rotateRetainedBrowserRequirement({
          path: phasePath,
          evidence: {
            ...replacementEvidence,
            observed: { ...replacementEvidence.observed, targetId: 'changed-retry-target' },
          },
          expectedSha256: phaseOld.sha256,
          staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
        }),
        /conflicts with requested replacement/,
      );
    }
    const recoveredPhase = rotateRetainedBrowserRequirement({
      path: phasePath,
      evidence: replacementEvidence,
      expectedSha256: phaseOld.sha256,
      staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
    });
    assert.equal(recoveredPhase.rotated, true);
    assert.equal(recoveredPhase.expectation.targetId, 'replacement-target');
    assert.equal(existsSync(retainedBrowserRotationJournalPath(phasePath)), false);
  }

  const digestConflictPath = join(root, 'rotation-digest-conflict', 'retained-browser.json');
  writeRetainedBrowserRequirement({ path: digestConflictPath, evidence });
  assert.throws(
    () => rotateRetainedBrowserRequirement({
      path: digestConflictPath,
      evidence: replacementEvidence,
      expectedSha256: '0'.repeat(64),
      staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
    }),
    /expectedSha256 does not match current requirement/,
  );

  const requirementBytes = readFileSync(path);
  const replacedValue = JSON.parse(readFileSync(path, 'utf8'));
  replacedValue.expectation.targetId = 'replacement-target';
  writeFileSync(path, `${JSON.stringify(replacedValue, null, 2)}\n`, { mode: 0o600 });
  assert.throws(
    () => readRetainedBrowserRequirement(path),
    /does not match enforcement digest/,
  );
  writeFileSync(path, requirementBytes, { mode: 0o600 });
  chmodSync(path, 0o600);

  rmSync(path);
  assert.throws(
    () => readRetainedBrowserRequirement(path),
    /Required retained browser requirement is missing/,
  );
  writeFileSync(path, requirementBytes, { mode: 0o600 });
  chmodSync(path, 0o600);

  chmodSync(enforcementPath, 0o644);
  assert.throws(
    () => readRetainedBrowserRequirement(path),
    /Retained browser enforcement permissions must be 0600/,
  );
  chmodSync(enforcementPath, 0o600);
  const enforcementBytes = readFileSync(enforcementPath);
  const enforcementTarget = join(root, 'enforcement-target.json');
  writeFileSync(enforcementTarget, enforcementBytes, { mode: 0o600 });
  rmSync(enforcementPath);
  symlinkSync(enforcementTarget, enforcementPath);
  assert.throws(
    () => readRetainedBrowserRequirement(path),
    /Retained browser enforcement must be a regular non-symlink file/,
  );
  rmSync(enforcementPath);
  writeFileSync(enforcementPath, enforcementBytes, { mode: 0o600 });
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard retained browser requirement fixture passed');
