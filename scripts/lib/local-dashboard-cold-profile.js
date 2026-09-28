import * as fs from 'node:fs';
import { createHash, randomUUID } from 'node:crypto';
import { dirname, basename, isAbsolute, join, resolve, relative } from 'node:path';
import { homedir } from 'node:os';

const SCHEMA = 'agent-browser.cold-profile-backup.v1';
const fail = (message) => { throw new Error(`Cold profile: ${message}`); };
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const json = (value) => `${JSON.stringify(value)}\n`;
const exists = (path) => { try { fs.lstatSync(path); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; } };

function canonical(path, missing = false) {
  if (process.platform !== 'linux') fail('Linux is required');
  if (typeof path !== 'string' || !isAbsolute(path) || resolve(path) !== path) fail('canonical absolute path required');
  let current = '/';
  for (const part of path.split('/').filter(Boolean)) {
    current = join(current, part);
    if (missing && current === path && !exists(current)) return;
    const stat = fs.lstatSync(current);
    if (!stat.isDirectory() || stat.isSymbolicLink()) fail('directory or ancestor is not a real directory');
  }
}

function profilePath(path, missing = false) {
  canonical(path, missing);
  if (path.split('/').filter(Boolean).length < 3 || [homedir(), process.cwd()].includes(path)
    || ['.agent-browser', '.codex', 'runtime-profiles', 'projects', 'workspace', 'workspaces'].includes(basename(path))
    || exists(join(path, '.git'))) fail('broad root or workspace is not a profile');
}

function privateDirectory(path) {
  canonical(path);
  const stat = fs.lstatSync(path);
  if ((stat.mode & 0o077) || stat.uid !== process.getuid()) fail('backup directory must be private and owned');
}

function idle(callback, path) {
  if (typeof callback !== 'function') fail('requireProfileIdle callback required');
  const result = callback(path);
  if (result === false || result?.then) fail('idle check must synchronously prove absence or throw');
}

function syncDirectory(path) {
  const fd = fs.openSync(path, fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}

function writePrivate(path, value, replace = false) {
  const temporary = replace ? `${path}.${randomUUID()}` : path;
  const fd = fs.openSync(temporary, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, 0o600);
  try { fs.writeFileSync(fd, json(value)); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  if (replace) fs.renameSync(temporary, path);
  syncDirectory(dirname(path));
}

function readPrivate(path) {
  canonical(dirname(path));
  const fd = fs.openSync(path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try {
    const stat = fs.fstatSync(fd);
    if (!stat.isFile() || (stat.mode & 0o077) || stat.uid !== process.getuid() || stat.size > 32 * 1024 * 1024) fail('unsafe receipt file');
    return JSON.parse(fs.readFileSync(fd, 'utf8'));
  } finally { fs.closeSync(fd); }
}

// Hash names, modes, link text and bytes; never follow a profile symlink.
function inventory(root, durable = false) {
  canonical(root);
  const entries = [];
  function visit(path, name) {
    const stat = fs.lstatSync(path);
    const entry = { name, mode: stat.mode & 0o7777 };
    if (stat.isSymbolicLink()) {
      entries.push({ ...entry, type: 'symlink', target: fs.readlinkSync(path) });
    } else if (stat.isDirectory()) {
      entries.push({ ...entry, type: 'directory' });
      for (const child of fs.readdirSync(path).sort()) visit(join(path, child), name ? `${name}/${child}` : child);
      if (durable) syncDirectory(path);
    } else if (stat.isFile()) {
      const fd = fs.openSync(path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
      try {
        const before = fs.fstatSync(fd);
        if (before.ino !== stat.ino || before.dev !== stat.dev) fail('file changed during inventory');
        const digest = createHash('sha256');
        const chunk = Buffer.alloc(65536);
        let count;
        while ((count = fs.readSync(fd, chunk, 0, chunk.length, null))) digest.update(chunk.subarray(0, count));
        const after = fs.fstatSync(fd);
        if (before.size !== after.size || before.mtimeMs !== after.mtimeMs || before.ctimeMs !== after.ctimeMs) fail('file changed during inventory');
        if (durable) fs.fsyncSync(fd);
        entries.push({ ...entry, type: 'file', size: stat.size, sha256: digest.digest('hex') });
      } finally { fs.closeSync(fd); }
    } else fail('unsupported special file in profile');
  }
  visit(root, '');
  return { entries, sha256: hash(json(entries)) };
}

function copyTree(source, target) {
  if (exists(target)) fail('copy target already exists');
  const metadata = inventory(source);
  fs.cpSync(source, target, { recursive: true, dereference: false, verbatimSymlinks: true, preserveTimestamps: true, errorOnExist: true, force: false });
  // Node's recursive copy can create directories with default modes. Apply
  // exact source modes only to verified non-link entries in the new tree.
  for (const entry of [...metadata.entries].reverse()) {
    if (entry.type === 'symlink') continue;
    const path = entry.name ? join(target, entry.name) : target;
    canonical(dirname(path));
    const stat = fs.lstatSync(path);
    if (stat.isSymbolicLink() || (entry.type === 'file' ? !stat.isFile() : !stat.isDirectory())) fail('copy entry type changed');
    fs.chmodSync(path, entry.mode);
  }
}

/** Copies only an idle exact profile into a private task-owned backup. */
export function prepareColdProfileBackup({ profilePath: profile, backupRoot, requireProfileIdle }) {
  profilePath(profile);
  privateDirectory(backupRoot);
  const outside = (value) => value === '..' || value.startsWith('../');
  if (!outside(relative(profile, backupRoot)) || !outside(relative(backupRoot, profile))) fail('backup and profile must be disjoint');
  idle(requireProfileIdle, profile);
  const before = inventory(profile);
  const directory = fs.mkdtempSync(join(backupRoot, 'cold-profile-'));
  const backupPath = join(directory, 'profile');
  copyTree(profile, backupPath);
  idle(requireProfileIdle, profile);
  const copied = inventory(backupPath, true);
  if (before.sha256 !== copied.sha256 || inventory(profile).sha256 !== before.sha256) fail('profile changed while backing up');
  const receipt = { schemaVersion: SCHEMA, profilePath: profile, backupDirectory: directory, backupPath, manifestSha256: copied.sha256, entryCount: copied.entries.length };
  writePrivate(join(directory, 'manifest.json'), copied.entries);
  writePrivate(join(directory, 'receipt.json'), receipt);
  syncDirectory(backupRoot);
  return receipt;
}

/** Checks the private receipt, manifest and all backup bytes without effects. */
export function verifyColdProfileBackup(receipt) {
  if (receipt?.schemaVersion !== SCHEMA || !/^[a-f0-9]{64}$/.test(receipt.manifestSha256)) fail('invalid backup receipt');
  profilePath(receipt.profilePath, true);
  privateDirectory(receipt.backupDirectory);
  if (!/^cold-profile-[A-Za-z0-9]+$/.test(basename(receipt.backupDirectory)) || receipt.backupPath !== join(receipt.backupDirectory, 'profile')) fail('invalid backup binding');
  if (json(readPrivate(join(receipt.backupDirectory, 'receipt.json'))) !== json(receipt)) fail('receipt changed');
  const manifest = readPrivate(join(receipt.backupDirectory, 'manifest.json'));
  if (!Array.isArray(manifest) || manifest.length !== receipt.entryCount || hash(json(manifest)) !== receipt.manifestSha256) fail('manifest changed');
  if (inventory(receipt.backupPath).sha256 !== receipt.manifestSha256) fail('backup bytes or metadata changed');
  return true;
}

/** Retains the replaced profile; retries recover a receipted interrupted move. */
export function restoreColdProfileBackup(receipt, { requireProfileIdle } = {}) {
  verifyColdProfileBackup(receipt);
  const profile = receipt.profilePath;
  idle(requireProfileIdle, profile);
  const journalPath = join(receipt.backupDirectory, 'restore.json');
  let journal;
  if (exists(journalPath)) {
    journal = readPrivate(journalPath);
    if (journal.manifestSha256 !== receipt.manifestSha256 || journal.profilePath !== profile
      || !/^[a-f0-9-]{36}$/.test(journal.id)
      || journal.displacedPath !== join(dirname(profile), `.${basename(profile)}.cold-displaced-${journal.id}`)
      || journal.stagedPath !== join(dirname(profile), `.${basename(profile)}.cold-restored-${journal.id}`)
      || !['prepared', 'moved', 'restored'].includes(journal.phase)) fail('invalid restore journal');
  } else {
    if (!exists(profile)) fail('unreceipted absent profile');
    const id = randomUUID();
    journal = { id, profilePath: profile, manifestSha256: receipt.manifestSha256, previousSha256: inventory(profile).sha256,
      displacedPath: join(dirname(profile), `.${basename(profile)}.cold-displaced-${id}`),
      stagedPath: join(dirname(profile), `.${basename(profile)}.cold-restored-${id}`), phase: 'prepared' };
    writePrivate(journalPath, journal);
  }
  if (journal.phase === 'restored') {
    if (inventory(profile).sha256 !== receipt.manifestSha256) fail('restored profile has drifted');
    return { restored: true, profilePath: profile, displacedPath: journal.displacedPath, manifestSha256: receipt.manifestSha256 };
  }
  if (!exists(journal.displacedPath)) {
    if (journal.phase !== 'prepared' || inventory(profile).sha256 !== journal.previousSha256) fail('profile drift before move');
    idle(requireProfileIdle, profile);
    if (inventory(profile).sha256 !== journal.previousSha256 || exists(journal.displacedPath)) fail('profile drift at move admission');
    fs.renameSync(profile, journal.displacedPath);
    syncDirectory(dirname(profile));
  } else if (inventory(journal.displacedPath).sha256 !== journal.previousSha256) fail('displaced profile has drifted');
  journal.phase = 'moved';
  writePrivate(journalPath, journal, true);
  idle(requireProfileIdle, profile);
  verifyColdProfileBackup(receipt);
  if (!exists(journal.stagedPath) && !exists(profile)) copyTree(receipt.backupPath, journal.stagedPath);
  if (!exists(profile)) {
    if (inventory(journal.stagedPath, true).sha256 !== receipt.manifestSha256) fail('staged restore is incomplete; preserved for inspection');
    idle(requireProfileIdle, profile);
    canonical(dirname(profile));
    if (exists(profile)) fail('profile reappeared before restore commit');
    fs.renameSync(journal.stagedPath, profile);
    syncDirectory(dirname(profile));
  }
  idle(requireProfileIdle, profile);
  if (inventory(profile).sha256 !== receipt.manifestSha256) fail('restored profile mismatch');
  journal.phase = 'restored';
  writePrivate(journalPath, journal, true);
  return { restored: true, profilePath: profile, displacedPath: journal.displacedPath, manifestSha256: receipt.manifestSha256 };
}
