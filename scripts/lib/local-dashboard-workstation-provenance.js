import { createHash } from 'node:crypto';
import { constants, closeSync, fchmodSync, fstatSync, fsyncSync, lstatSync, mkdtempSync, openSync, readFileSync, readdirSync, readSync, renameSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const SCHEMA = 'agent-browser.local-dashboard-workstation-provenance.v1';
const LIMIT = 4 * 1024 * 1024;
const fail = (message) => { throw new Error(`Workstation provenance: ${message}`); };
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sha = (value) => { if (!/^[a-f0-9]{64}$/.test(value)) fail('invalid SHA-256'); return value; };

function safePath(path, missing = false) {
  if (typeof path !== 'string' || !isAbsolute(path) || resolve(path) !== path) fail('noncanonical path');
  const parts = path.split('/').filter(Boolean);
  let current = '/';
  for (let i = 0; i < parts.length; i++) {
    current = join(current, parts[i]);
    let stat;
    try { stat = lstatSync(current); } catch (error) {
      if (missing && error.code === 'ENOENT') return false;
      throw error;
    }
    if (stat.isSymbolicLink() || (i < parts.length - 1 && !stat.isDirectory())) fail('unsafe path');
  }
  return true;
}

function bytes(path, privateFile = false) {
  safePath(path);
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || stat.size > LIMIT || (privateFile && ((stat.mode & 0o077) || stat.uid !== process.getuid()))) fail('unsafe manifest or snapshot');
    return readFileSync(fd);
  } finally { closeSync(fd); }
}

function hashFile(path) {
  safePath(path);
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    if (!fstatSync(fd).isFile()) fail('asset is not a regular file');
    const hash = createHash('sha256');
    const chunk = Buffer.alloc(65536);
    let count;
    while ((count = readSync(fd, chunk, 0, chunk.length, null)) > 0) hash.update(chunk.subarray(0, count));
    return hash.digest('hex');
  } finally { closeSync(fd); }
}

function syncDirectory(path) {
  const fd = openSync(path, constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
  try { fsyncSync(fd); } finally { closeSync(fd); }
}

function durableWrite(path, value, mode = 0o600) {
  const fd = openSync(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW, 0o600);
  try { writeFileSync(fd, value); fchmodSync(fd, mode); fsyncSync(fd); } finally { closeSync(fd); }
}

function paths(root, version, installBin) {
  safePath(root);
  if (typeof version !== 'string' || !/^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][a-zA-Z0-9.-]+)?$/.test(version)) fail('invalid version');
  if (installBin !== join(root, '.local/bin/agent-browser')) fail('unexpected installation path');
  const support = join(root, '.local/lib/agent-browser', version);
  return { support, manifest: join(support, 'manifest.json'), units: join(root, '.config/systemd/user') };
}

function assetPath(base, value) {
  if (typeof value !== 'string' || !value || value.includes('\\') || isAbsolute(value) || value.split('/').some((x) => !x || x === '.' || x === '..')) fail('unsafe asset path');
  return join(base, value);
}

function verifyAssets(manifest, locations, version, updates = []) {
  if (manifest.schemaVersion !== 'agent-browser.workstation-payload.v1' || manifest.version !== version) fail('manifest version or schema mismatch');
  sha(manifest.binary?.sha256);
  const controllers = manifest.controllerAssets?.files;
  const units = manifest.units;
  const bundle = manifest.guacamoleBundle;
  if (!Array.isArray(controllers) || !controllers.length || !Array.isArray(units) || !units.length || !Array.isArray(bundle?.files) || !bundle.files.length || !bundle.schema) fail('missing asset receipts');
  const verify = (base, item, key = 'path') => {
    const path = assetPath(base, item[key]);
    if (hashFile(path) !== sha(item.sha256)) fail('preserved asset digest mismatch');
  };
  controllers.forEach((item) => {
    const update = updates.find((entry) => entry.path === item.path);
    if (!update) return verify(locations.support, item);
    const path = assetPath(locations.support, item.path);
    if (![update.sourceSha256, update.candidateSha256].includes(hashFile(path))) fail('unreviewed current controller asset digest');
    if ((lstatSync(path).mode & 0o7777) !== update.sourceMode) fail('controller mode changed');
  });
  units.forEach((item) => {
    if (item.name.includes('/')) fail('unsafe unit name');
    verify(locations.units, item, 'name');
  });
  const guac = join(locations.support, 'guacamole');
  const bundleBytes = bytes(join(guac, 'manifest.json'));
  if (digest(bundleBytes) !== sha(manifest.guacamoleBundleManifestSha256) || !isDeepStrictEqual(JSON.parse(bundleBytes), bundle)) fail('Guacamole manifest mismatch');
  bundle.files.forEach((item) => verify(guac, item));
  verify(guac, bundle.schema);
}

// The caller verifies candidate --version and journals this record before replacing the binary.
export function prepareWorkstationProvenance({ root, version, installBin, builtBin, journalPath, controllerUpdates = [] }) {
  if (!Array.isArray(controllerUpdates)) fail('invalid controller updates');
  const locations = paths(root, version, installBin);
  if (!safePath(locations.manifest, true)) {
    if (controllerUpdates.length) fail('controller updates require an existing workstation manifest');
    const library = join(root, '.local/lib/agent-browser');
    if (safePath(library, true) && readdirSync(library).some((entry) => safePath(join(library, entry, 'manifest.json'), true))) fail('different workstation version requires full installation');
    return null;
  }
  const sourceMode = lstatSync(locations.manifest).mode & 0o777;
  if (sourceMode & 0o022) fail('writable manifest permissions');
  const sourceBytes = bytes(locations.manifest);
  const source = JSON.parse(sourceBytes);
  verifyAssets(source, locations, version);
  const sourceBinarySha256 = hashFile(installBin);
  if (source.binary.sha256 !== sourceBinarySha256) fail('installed binary digest mismatch');
  const candidateBinarySha256 = hashFile(builtBin);
  const candidate = structuredClone(source);
  candidate.binary.sha256 = candidateBinarySha256;
  const seen = new Set();
  const updates = controllerUpdates.map((update) => {
    const path = controllerPath(locations, update?.path);
    if (seen.has(update.path)) fail('duplicate controller update');
    seen.add(update.path);
    const entry = controllerEntry(source, update.path);
    const sourceBytes = bytes(path);
    const sourceMode = lstatSync(path).mode & 0o7777;
    controllerMode(sourceMode);
    const candidateBytes = bytes(update.sourcePath);
    const candidateMode = lstatSync(update.sourcePath).mode & 0o7777;
    controllerMode(candidateMode);
    if (candidateMode & ~sourceMode) fail('controller mode widening');
    if (digest(sourceBytes) !== entry.sha256) fail('source controller digest mismatch');
    const candidateSha256 = sha(update.expectedSha256);
    if (digest(candidateBytes) !== candidateSha256) fail('candidate controller digest mismatch');
    controllerEntry(candidate, update.path).sha256 = candidateSha256;
    return { path: update.path, sourceMode, sourceSha256: entry.sha256, candidateSha256, sourceBytes, candidateBytes };
  });
  const candidateBytes = Buffer.from(`${JSON.stringify(candidate, null, 2)}\n`);
  safePath(dirname(journalPath));
  if (relative(root, journalPath).startsWith('..') || !isAbsolute(journalPath)) fail('journal outside workstation root');
  const snapshotDir = mkdtempSync(join(dirname(journalPath), '.workstation-provenance-'));
  durableWrite(join(snapshotDir, 'source.json'), sourceBytes);
  durableWrite(join(snapshotDir, 'candidate.json'), candidateBytes);
  updates.forEach((update, index) => {
    durableWrite(join(snapshotDir, `controller-${index}-source`), update.sourceBytes);
    durableWrite(join(snapshotDir, `controller-${index}-candidate`), update.candidateBytes);
  });
  syncDirectory(snapshotDir);
  syncDirectory(dirname(snapshotDir));
  return { schemaVersion: SCHEMA, root, version, manifestPath: locations.manifest, snapshotDir, sourceMode, sourceManifestSha256: digest(sourceBytes), candidateManifestSha256: digest(candidateBytes), sourceBinarySha256, candidateBinarySha256,
    ...(updates.length ? { controllerUpdates: updates.map(({ path, sourceMode, sourceSha256, candidateSha256 }) => ({ path, sourceMode, sourceSha256, candidateSha256 })) } : {}) };
}

function controllerPath(locations, path) {
  const target = assetPath(locations.support, path);
  if (!path.startsWith('scripts/')) fail('unsafe controller path');
  return target;
}

function controllerMode(mode) {
  if (!Number.isInteger(mode) || mode < 0 || mode > 0o777 || (mode & 0o022)) fail('unsafe controller mode');
}

function controllerEntry(manifest, path) {
  const entries = manifest.controllerAssets?.files?.filter((entry) => entry.path === path);
  if (entries?.length !== 1) fail('controller update requires exactly one existing manifest entry');
  return entries[0];
}

function review(record, { selection, installBin }) {
  if (!['source', 'candidate'].includes(selection)) fail('invalid selection');
  if (record.schemaVersion !== SCHEMA) fail('invalid record schema');
  const locations = paths(record.root, record.version, installBin);
  if (record.manifestPath !== locations.manifest) fail('manifest path mismatch');
  safePath(record.snapshotDir);
  const snapshotStat = lstatSync(record.snapshotDir);
  if (!snapshotStat.isDirectory() || snapshotStat.uid !== process.getuid() || relative(record.root, record.snapshotDir).startsWith('..') || !/\.workstation-provenance-[^/]+$/.test(record.snapshotDir) || (snapshotStat.mode & 0o077)) fail('unsafe snapshot directory');
  if (!Number.isInteger(record.sourceMode) || record.sourceMode < 0 || record.sourceMode > 0o777 || (record.sourceMode & 0o022)) fail('invalid manifest mode');
  const sourceBytes = bytes(join(record.snapshotDir, 'source.json'), true);
  const candidateBytes = bytes(join(record.snapshotDir, 'candidate.json'), true);
  if (digest(sourceBytes) !== sha(record.sourceManifestSha256) || digest(candidateBytes) !== sha(record.candidateManifestSha256)) fail('snapshot digest mismatch');
  const source = JSON.parse(sourceBytes);
  const candidate = JSON.parse(candidateBytes);
  if (source.binary?.sha256 !== sha(record.sourceBinarySha256) || candidate.binary?.sha256 !== sha(record.candidateBinarySha256)) fail('binary receipt mismatch');
  const comparison = structuredClone(candidate);
  comparison.binary.sha256 = source.binary.sha256;
  const updates = record.controllerUpdates ?? [];
  if (!Array.isArray(updates)) fail('invalid controller updates');
  const seen = new Set();
  const reviewedUpdates = updates.map((update, index) => {
    const path = controllerPath(locations, update?.path);
    if (seen.has(update.path)) fail('duplicate controller update');
    seen.add(update.path);
    controllerMode(update.sourceMode);
    if (controllerEntry(source, update.path).sha256 !== sha(update.sourceSha256) || controllerEntry(candidate, update.path).sha256 !== sha(update.candidateSha256)) fail('controller receipt mismatch');
    const sourceBytes = bytes(join(record.snapshotDir, `controller-${index}-source`), true);
    const candidateBytes = bytes(join(record.snapshotDir, `controller-${index}-candidate`), true);
    if (digest(sourceBytes) !== update.sourceSha256 || digest(candidateBytes) !== update.candidateSha256) fail('controller snapshot digest mismatch');
    controllerEntry(comparison, update.path).sha256 = update.sourceSha256;
    return { ...update, target: path, selectedBytes: selection === 'source' ? sourceBytes : candidateBytes };
  });
  if (!isDeepStrictEqual(comparison, source)) fail('candidate changes preserved fields');
  verifyAssets(source, locations, record.version, updates);
  if (hashFile(installBin) !== record[`${selection}BinarySha256`]) fail('selected binary is not installed');
  const current = digest(bytes(locations.manifest));
  if ((lstatSync(locations.manifest).mode & 0o7777) !== record.sourceMode) fail('manifest mode changed');
  if (![record.sourceManifestSha256, record.candidateManifestSha256].includes(current)) fail('unreviewed current manifest');
  return { locations, current, updates: reviewedUpdates, selectedBytes: selection === 'source' ? sourceBytes : candidateBytes };
}

export function verifyWorkstationProvenance(record, options) {
  if (record == null) return null;
  const { current, updates } = review(record, options);
  if (updates.some((update) => hashFile(update.target) !== update[`${options.selection}Sha256`])) fail('binary, controllers and manifest are incomplete');
  if (current !== record[`${options.selection}ManifestSha256`]) fail('binary and manifest pair is incomplete');
  return true;
}

export function applyWorkstationProvenance(record, options) {
  if (record == null) return null;
  // Caller holds both publisher and native workstation locks throughout apply/recovery.
  const { locations, current, selectedBytes, updates } = review(record, options);
  for (const update of updates) {
    if (hashFile(update.target) === update[`${options.selection}Sha256`]) continue;
    const temporary = join(dirname(update.target), `.controller-publication-${process.pid}-${Date.now()}`);
    durableWrite(temporary, update.selectedBytes, update.sourceMode);
    try {
      review(record, options);
      renameSync(temporary, update.target);
      syncDirectory(dirname(update.target));
    } finally {
      if (safePath(temporary, true)) unlinkSync(temporary);
    }
  }
  if (current !== record[`${options.selection}ManifestSha256`]) {
    const temporary = join(locations.support, `.manifest-publication-${process.pid}-${Date.now()}`);
    durableWrite(temporary, selectedBytes, record.sourceMode);
    try {
      review(record, options);
      renameSync(temporary, locations.manifest);
      syncDirectory(locations.support);
    } finally {
      if (safePath(temporary, true)) unlinkSync(temporary);
    }
  }
  return verifyWorkstationProvenance(record, options);
}
