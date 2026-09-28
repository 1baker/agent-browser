#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { validatePrebuiltPublicationOptions, stagePrebuiltPublicationCandidate, requireLegacyPublicationTarget, installPublicationBinaryAtomically } from './lib/local-dashboard-prebuilt-candidate.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-prebuilt-fixture-'));
try {
  const sourcePath = join(root, 'candidate');
  writeFileSync(sourcePath, 'reviewed executable bytes', { mode: 0o700 });
  const expectedSha256 = createHash('sha256').update(readFileSync(sourcePath)).digest('hex');
  const options = { prebuiltBin: sourcePath, expectedSha256, expectedSessions: [], syncReferenceBinaries: false, smokeBrowser: false };
  assert.equal(validatePrebuiltPublicationOptions(options), true);
  for (const patch of [
    { expectedSha256: '' }, { prebuiltBin: 'relative' }, { release: true },
    { syncReferenceBinaries: true }, { smokeBrowser: true }, { skipSmoke: true },
    { expectedSessions: null }, { expectedSessions: ['same', 'same'] },
  ]) assert.throws(() => validatePrebuiltPublicationOptions({ ...options, ...patch }));
  const input = { sourcePath, expectedSha256, installPath: join(root, 'installed'), journalPath: join(root, 'journal.json') };
  const staged = stagePrebuiltPublicationCandidate(input);
  assert.equal(readFileSync(staged, 'utf8'), 'reviewed executable bytes');
  assert.equal(statSync(staged).mode & 0o777, 0o500);
  writeFileSync(sourcePath, 'changed workspace candidate');
  writeFileSync(input.installPath, 'original installed bytes');
  assert.throws(() => installPublicationBinaryAtomically(sourcePath, input.installPath, 0o700, expectedSha256), /SHA-256 mismatch/);
  assert.equal(readFileSync(input.installPath, 'utf8'), 'original installed bytes');
  installPublicationBinaryAtomically(staged, input.installPath, 0o700, expectedSha256);
  assert.equal(readFileSync(input.installPath, 'utf8'), 'reviewed executable bytes');
  assert.equal(readFileSync(staged, 'utf8'), 'reviewed executable bytes');
  assert.throws(() => stagePrebuiltPublicationCandidate(input), /SHA-256 mismatch/);
  rmSync(input.installPath);
  symlinkSync(staged, input.installPath);
  assert.throws(() => requireLegacyPublicationTarget(input.installPath), /legacy installed binary/);
  assert.throws(() => stagePrebuiltPublicationCandidate({ ...input, sourcePath: input.installPath }));
  chmodSync(sourcePath, 0o600);
  assert.throws(() => stagePrebuiltPublicationCandidate(input), /regular executable/);
  console.log('Prebuilt publication candidate fixture passed');
} finally {
  rmSync(root, { recursive: true, force: true });
}
