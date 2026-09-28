import { createHash } from 'node:crypto';
import { constants, closeSync, fstatSync, openSync, readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';

// Opt-in payload updates are explicitly named and digest-bound. No directory
// scan can pull unrelated worktree files into the installed controller bundle.
export function parseControllerUpdate(value) {
  const parts = value.split('=');
  if (parts.length !== 3) throw new Error('Controller update requires path=absolute-source=sha256');
  const [path, sourcePath, expectedSha256] = parts;
  if (!/^scripts\/(?:[a-zA-Z0-9_-]+\/)*[a-zA-Z0-9_.-]+\.js$/.test(path)
      || path.split('/').some(part => part === '.' || part === '..')
      || !isAbsolute(sourcePath) || resolve(sourcePath) !== sourcePath
      || !/^[a-f0-9]{64}$/.test(expectedSha256)) {
    throw new Error('Invalid controller update path or digest');
  }
  return { path, sourcePath, expectedSha256 };
}

function regularBytes(path, limit) {
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || stat.size < 1 || stat.size > limit) throw new Error('Invalid controller candidate file');
    return readFileSync(fd);
  } finally { closeSync(fd); }
}

/** Supplement isolated installer extraction tests with a pre-publication check
 * that the pinned binary contains the exact explicitly reviewed script bytes.
 * This is a build-coherence check, not execution of the candidate installer. */
export function verifyControllerCandidate(builtBin, updates) {
  if (!updates.length) return;
  if (new Set(updates.map(update => update.path)).size !== updates.length) {
    throw new Error('Duplicate controller update');
  }
  const binary = regularBytes(builtBin, 1024 ** 3);
  for (const update of updates) {
    parseControllerUpdate(`${update.path}=${update.sourcePath}=${update.expectedSha256}`);
    const bytes = regularBytes(update.sourcePath, 4 * 1024 * 1024);
    if (createHash('sha256').update(bytes).digest('hex') !== update.expectedSha256) {
      throw new Error('Controller candidate digest mismatch');
    }
    if (!binary.includes(bytes)) throw new Error('Candidate binary does not embed the reviewed controller');
  }
}
