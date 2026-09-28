import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, writeFileSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseControllerUpdate, verifyControllerCandidate } from './lib/local-dashboard-controller-candidate.js';

const root = mkdtempSync(join(tmpdir(), 'controller-candidate-test-'));
const hash = value => createHash('sha256').update(value).digest('hex');
try {
  const sourcePath = join(root, 'opener.js');
  const builtBin = join(root, 'candidate');
  const content = 'console.log("reviewed controller");\n';
  writeFileSync(sourcePath, content);
  writeFileSync(builtBin, `synthetic-binary-prefix\0${content}\0suffix`);
  const spec = `scripts/open-rdp-guac-route-displays.js=${sourcePath}=${hash(content)}`;
  const update = parseControllerUpdate(spec);
  verifyControllerCandidate(builtBin, [update]);
  verifyControllerCandidate('/not/read/without/updates', []);
  for (const value of [spec.replace('scripts/', '../'), spec.replace(sourcePath, './relative'),
    spec.replace(hash(content), 'not-a-digest'), `${spec}=extra`,
    spec.replace('scripts/', 'scripts/../'), spec.replace(sourcePath, `${root}/../opener.js`)]) {
    assert.throws(() => parseControllerUpdate(value));
  }
  assert.throws(() => verifyControllerCandidate(builtBin, [update, update]), /Duplicate/);
  writeFileSync(sourcePath, 'drift');
  assert.throws(() => verifyControllerCandidate(builtBin, [update]), /digest mismatch/);
  writeFileSync(sourcePath, content);
  writeFileSync(builtBin, 'old-binary-without-fix');
  assert.throws(() => verifyControllerCandidate(builtBin, [update]), /does not embed/);
  const link = join(root, 'link');
  symlinkSync(sourcePath, link);
  assert.throws(() => verifyControllerCandidate(builtBin, [{ ...update, sourcePath: link }]));
  console.log('Controller candidate: valid pair and 10 rejection cases passed');
} finally { rmSync(root, { recursive: true, force: true }); }
