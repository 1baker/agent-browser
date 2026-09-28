import { createHash } from 'node:crypto';
import {
  closeSync, constants, copyFileSync, chmodSync, fchmodSync, fsyncSync, fstatSync, lstatSync,
  mkdirSync, mkdtempSync, openSync, readFileSync, renameSync, rmSync, writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';

export function validatePrebuiltPublicationOptions(options) {
  const supplied = Boolean(options.prebuiltBin || options.expectedSha256);
  if (!supplied) return false;
  if (!isAbsolute(options.prebuiltBin || '') || !/^[a-f0-9]{64}$/.test(options.expectedSha256 || '')) {
    throw new Error('Prebuilt publication requires --prebuilt-bin <absolute-path> and --expected-sha256 <lowercase-sha256>');
  }
  if (options.release || options.syncReferenceBinaries || options.smokeBrowser || options.skipSmoke) {
    throw new Error('Prebuilt publication requires --skip-reference-sync and --skip-browser, and rejects --release or --skip-smoke');
  }
  if (!Array.isArray(options.expectedSessions)) {
    throw new Error('Prebuilt publication requires --expected-sessions <comma-separated-names|none>');
  }
  const names = options.expectedSessions;
  if (new Set(names).size !== names.length || names.some(name => !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name))) {
    throw new Error('Expected sessions must be unique valid session names');
  }
  return true;
}

export function requireExpectedPublicationSessions(expected, observed) {
  if (JSON.stringify([...expected].sort()) !== JSON.stringify([...observed].sort())) {
    throw new Error('Publication session inventory changed; inspect and approve the exact session set again');
  }
}

/** Snapshot the reviewed bytes without executing or rebuilding them. Keep the
 * private snapshot for journal recovery; it is not an installed generation. */
export function stagePrebuiltPublicationCandidate({ sourcePath, expectedSha256, installPath, journalPath }) {
  if (resolve(sourcePath) === resolve(installPath)) throw new Error('Prebuilt source cannot be the installed target');
  const fd = openSync(sourcePath, constants.O_RDONLY | constants.O_NOFOLLOW);
  let bytes;
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || !(stat.mode & 0o111) || stat.size > 1024 ** 3) {
      throw new Error('Prebuilt source must be a bounded regular executable file');
    }
    bytes = readFileSync(fd);
  } finally {
    closeSync(fd);
  }
  if (createHash('sha256').update(bytes).digest('hex') !== expectedSha256) {
    throw new Error('Prebuilt candidate SHA-256 mismatch before publication');
  }
  const parent = dirname(journalPath);
  mkdirSync(parent, { recursive: true, mode: 0o700 });
  const stageDir = mkdtempSync(join(parent, 'prebuilt-'));
  const path = join(stageDir, 'agent-browser');
  const output = openSync(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL, 0o500);
  try {
    writeFileSync(output, bytes);
    fchmodSync(output, 0o500);
    fsyncSync(output);
  } finally {
    closeSync(output);
  }
  const dirFd = openSync(stageDir, constants.O_RDONLY);
  try { fsyncSync(dirFd); } finally { closeSync(dirFd); }
  const parentFd = openSync(parent, constants.O_RDONLY);
  try { fsyncSync(parentFd); } finally { closeSync(parentFd); }
  return path;
}

/** Verify the copied bytes before rename, not just the mutable source path. */
export function installPublicationBinaryAtomically(source, target, mode, expectedSha256 = null) {
  requireLegacyPublicationTarget(target);
  mkdirSync(dirname(target), { recursive: true });
  const stageDir = mkdtempSync(join(dirname(target), '.agent-browser-replacement-'));
  const staged = join(stageDir, 'agent-browser');
  try {
    copyFileSync(source, staged, constants.COPYFILE_EXCL);
    chmodSync(staged, mode);
    const actual = createHash('sha256').update(readFileSync(staged)).digest('hex');
    if (expectedSha256 && actual !== expectedSha256) throw new Error('Candidate copy SHA-256 mismatch before installed binary replacement');
    const fd = openSync(staged, constants.O_RDONLY);
    try { fsyncSync(fd); } finally { closeSync(fd); }
    requireLegacyPublicationTarget(target);
    renameSync(staged, target);
    const parentFd = openSync(dirname(target), constants.O_RDONLY);
    try { fsyncSync(parentFd); } finally { closeSync(parentFd); }
  } finally {
    rmSync(stageDir, { recursive: true, force: true });
  }
}

/** A legacy publisher must never replace a generation-selector symlink. */
export function requireLegacyPublicationTarget(path) {
  try {
    if (!lstatSync(path).isFile()) throw new Error('Publication requires a regular legacy installed binary, not a selector or symlink');
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}
