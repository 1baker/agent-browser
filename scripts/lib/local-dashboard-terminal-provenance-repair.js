import { createHash } from 'node:crypto';
import {
  constants,
  closeSync,
  fchmodSync,
  fstatSync,
  fsyncSync,
  lstatSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const SCHEMA = 'agent-browser.local-dashboard-terminal-provenance-repair.v1';
const MANIFEST_SCHEMA = 'agent-browser.workstation-payload.v1';
const LIMIT = 4 * 1024 * 1024;
const SHA256 = /^[a-f0-9]{64}$/;

const fail = (message) => {
  throw new Error(`Terminal workstation provenance repair: ${message}`);
};

const digest = (value) => createHash('sha256').update(value).digest('hex');

function requireSha256(value, label) {
  if (typeof value !== 'string' || !SHA256.test(value)) fail(`invalid ${label} SHA-256`);
  return value;
}

function safePath(path, { missing = false } = {}) {
  if (typeof path !== 'string' || !isAbsolute(path) || resolve(path) !== path) {
    fail('noncanonical path');
  }
  const parts = path.split('/').filter(Boolean);
  let current = '/';
  for (let index = 0; index < parts.length; index += 1) {
    current = join(current, parts[index]);
    let stat;
    try {
      stat = lstatSync(current);
    } catch (error) {
      if (missing && error?.code === 'ENOENT') return false;
      throw error;
    }
    if (stat.isSymbolicLink() || (index < parts.length - 1 && !stat.isDirectory())) {
      fail('unsafe path');
    }
  }
  return true;
}

function readBytes(path, { privateFile = false } = {}) {
  safePath(path);
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || stat.size > LIMIT) fail('unsafe manifest or snapshot');
    if (privateFile && (stat.uid !== process.getuid() || (stat.mode & 0o077))) {
      fail('unsafe private snapshot');
    }
    return readFileSync(fd);
  } finally {
    closeSync(fd);
  }
}

function hashFile(path) {
  safePath(path);
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    if (!fstatSync(fd).isFile()) fail('asset is not a regular file');
    const hash = createHash('sha256');
    const chunk = Buffer.alloc(65536);
    let count;
    while ((count = readSync(fd, chunk, 0, chunk.length, null)) > 0) {
      hash.update(chunk.subarray(0, count));
    }
    return hash.digest('hex');
  } finally {
    closeSync(fd);
  }
}

function syncDirectory(path) {
  const fd = openSync(path, constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
  try {
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
}

function exclusiveWrite(path, value, mode = 0o600) {
  const fd = openSync(
    path,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600,
  );
  try {
    writeFileSync(fd, value);
    fchmodSync(fd, mode);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
}

function relativeAssetPath(base, value) {
  if (
    typeof value !== 'string'
    || !value
    || value.includes('\\')
    || isAbsolute(value)
    || value.split('/').some((part) => !part || part === '.' || part === '..')
  ) {
    fail('unsafe asset path');
  }
  return join(base, value);
}

function verifyPreservedAssets(manifest, { supportRoot, unitsRoot, version }) {
  if (manifest.schemaVersion !== MANIFEST_SCHEMA || manifest.version !== version) {
    fail('manifest version or schema mismatch');
  }
  requireSha256(manifest.binary?.sha256, 'manifest binary');
  const controllers = manifest.controllerAssets?.files;
  const units = manifest.units;
  const bundle = manifest.guacamoleBundle;
  if (
    !Array.isArray(controllers)
    || !controllers.length
    || !Array.isArray(units)
    || !units.length
    || !Array.isArray(bundle?.files)
    || !bundle.files.length
    || !bundle.schema
  ) {
    fail('missing asset receipts');
  }
  const verify = (base, item, key = 'path') => {
    const expected = requireSha256(item?.sha256, 'asset');
    if (hashFile(relativeAssetPath(base, item?.[key])) !== expected) {
      fail('preserved asset digest mismatch');
    }
  };
  controllers.forEach((item) => verify(supportRoot, item));
  units.forEach((item) => {
    if (typeof item?.name !== 'string' || item.name.includes('/')) fail('unsafe unit name');
    verify(unitsRoot, item, 'name');
  });
  const guacamoleRoot = join(supportRoot, 'guacamole');
  const bundleBytes = readBytes(join(guacamoleRoot, 'manifest.json'));
  if (
    digest(bundleBytes) !== requireSha256(manifest.guacamoleBundleManifestSha256, 'bundle manifest')
    || !isDeepStrictEqual(JSON.parse(bundleBytes), bundle)
  ) {
    fail('Guacamole manifest mismatch');
  }
  bundle.files.forEach((item) => verify(guacamoleRoot, item));
  verify(guacamoleRoot, bundle.schema);
}

function validateJournal(journalRecord, installBin) {
  if (!journalRecord || journalRecord.terminal !== true) fail('publication journal is not terminal');
  if (!['ready', 'recovered_ready', 'publication_failed_replacement_retained'].includes(journalRecord.phase)) {
    fail('publication journal does not retain an admitted replacement');
  }
  if (journalRecord.installBin !== installBin) fail('publication install path mismatch');
  const built = journalRecord.artifactEvidence?.built;
  const replacement = journalRecord.artifactEvidence?.replacement;
  const installedSha256 = hashFile(installBin);
  if (
    replacement?.verified !== true
    || replacement.path !== installBin
    || requireSha256(replacement.expectedSha256, 'expected replacement') !== installedSha256
    || requireSha256(replacement.actualSha256, 'actual replacement') !== installedSha256
    || requireSha256(built?.sha256, 'built artifact') !== installedSha256
  ) {
    fail('installed binary is not the exact journaled replacement');
  }
  return installedSha256;
}

export function repairTerminalWorkstationProvenance({
  root,
  version,
  installBin,
  journalPath,
  journalRecord,
}) {
  safePath(root);
  if (typeof version !== 'string' || !/^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][a-zA-Z0-9.-]+)?$/.test(version)) {
    fail('invalid package version');
  }
  if (installBin !== join(root, '.local/bin/agent-browser')) fail('unexpected installation path');
  safePath(dirname(journalPath));
  if (relative(root, journalPath).startsWith('..')) fail('journal outside workstation root');

  const installedSha256 = validateJournal(journalRecord, installBin);
  const supportRoot = join(root, '.local/lib/agent-browser', version);
  const unitsRoot = join(root, '.config/systemd/user');
  const manifestPath = join(supportRoot, 'manifest.json');
  safePath(manifestPath);
  const manifestStat = lstatSync(manifestPath);
  const manifestMode = manifestStat.mode & 0o777;
  if (!manifestStat.isFile() || manifestStat.uid !== process.getuid() || (manifestMode & 0o022)) {
    fail('unsafe workstation manifest');
  }
  const sourceBytes = readBytes(manifestPath);
  const source = JSON.parse(sourceBytes);
  verifyPreservedAssets(source, { supportRoot, unitsRoot, version });

  if (source.binary.sha256 === installedSha256) {
    return {
      schemaVersion: SCHEMA,
      transactionId: journalRecord.transactionId,
      changed: false,
      installedSha256,
      manifestPath,
      manifestSha256: digest(sourceBytes),
    };
  }

  const candidate = structuredClone(source);
  candidate.binary.sha256 = installedSha256;
  const candidateBytes = Buffer.from(`${JSON.stringify(candidate, null, 2)}\n`);
  const snapshotDir = mkdtempSync(join(dirname(journalPath), '.terminal-provenance-repair-'));
  const sourceSnapshot = join(snapshotDir, 'source.json');
  const candidateSnapshot = join(snapshotDir, 'candidate.json');
  exclusiveWrite(sourceSnapshot, sourceBytes);
  exclusiveWrite(candidateSnapshot, candidateBytes);
  syncDirectory(snapshotDir);
  syncDirectory(dirname(snapshotDir));

  const temporary = join(supportRoot, `.manifest-provenance-repair-${process.pid}-${Date.now()}`);
  try {
    if (!isDeepStrictEqual(JSON.parse(readBytes(sourceSnapshot, { privateFile: true })), source)) {
      fail('source snapshot changed');
    }
    if (digest(readBytes(manifestPath)) !== digest(sourceBytes)) fail('manifest changed during repair');
    if (hashFile(installBin) !== installedSha256) fail('installed binary changed during repair');
    verifyPreservedAssets(source, { supportRoot, unitsRoot, version });
    exclusiveWrite(temporary, candidateBytes, manifestMode);
    renameSync(temporary, manifestPath);
    syncDirectory(supportRoot);
  } finally {
    if (safePath(temporary, { missing: true })) unlinkSync(temporary);
  }

  const committedBytes = readBytes(manifestPath);
  const committed = JSON.parse(committedBytes);
  verifyPreservedAssets(committed, { supportRoot, unitsRoot, version });
  if (committed.binary.sha256 !== installedSha256 || hashFile(installBin) !== installedSha256) {
    fail('binary and manifest pair is incomplete');
  }
  const receipt = {
    schemaVersion: SCHEMA,
    transactionId: journalRecord.transactionId,
    changed: true,
    installedSha256,
    manifestPath,
    sourceManifestSha256: digest(sourceBytes),
    candidateManifestSha256: digest(candidateBytes),
    snapshotDir,
  };
  exclusiveWrite(join(snapshotDir, 'receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`);
  syncDirectory(snapshotDir);
  return receipt;
}
