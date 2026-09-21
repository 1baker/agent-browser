import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { prepareColdProfileBackup, verifyColdProfileBackup, restoreColdProfileBackup } from './lib/local-dashboard-cold-profile.js';

const root = fs.mkdtempSync(join(tmpdir(), 'cold-profile-test-'));
let cases = 0;
function fixture(name) {
  const base = join(root, name);
  const profile = join(base, 'profile');
  const backups = join(base, 'backups');
  fs.mkdirSync(profile, { recursive: true, mode: 0o700 });
  fs.mkdirSync(backups, { mode: 0o700 });
  fs.mkdirSync(join(profile, 'Default'), { mode: 0o700 });
  fs.writeFileSync(join(profile, 'Default', 'Preferences'), 'fixture bytes', { mode: 0o600 });
  fs.symlinkSync('../absent', join(profile, 'relative-link'));
  fs.symlinkSync('/unrelated/not-readable', join(profile, 'absolute-link'));
  return { profilePath: profile, backupRoot: backups, requireProfileIdle: () => true };
}
function test(name, fn) { fn(fixture(name)); cases += 1; }

try {
  test('roundtrip', (args) => {
    let checks = 0;
    args.requireProfileIdle = () => { checks += 1; };
    const receipt = prepareColdProfileBackup(args);
    assert.ok(checks >= 2);
    assert.equal(verifyColdProfileBackup(receipt), true);
    fs.writeFileSync(join(args.profilePath, 'Default', 'Preferences'), 'candidate changes');
    const result = restoreColdProfileBackup(receipt, args);
    assert.equal(fs.readFileSync(join(args.profilePath, 'Default', 'Preferences'), 'utf8'), 'fixture bytes');
    assert.equal(fs.readFileSync(join(result.displacedPath, 'Default', 'Preferences'), 'utf8'), 'candidate changes');
    assert.equal(fs.readlinkSync(join(args.profilePath, 'absolute-link')), '/unrelated/not-readable');
    assert.equal(fs.readlinkSync(join(args.profilePath, 'relative-link')), '../absent');
    assert.equal(fs.statSync(join(args.profilePath, 'Default', 'Preferences')).mode & 0o777, 0o600);
    assert.deepEqual(restoreColdProfileBackup(receipt, args), result);
    fs.writeFileSync(join(args.profilePath, 'new-work'), 'new');
    assert.throws(() => restoreColdProfileBackup(receipt, args), /drifted/);
  });
  for (const tamper of ['bytes', 'mode', 'name', 'link', 'manifest', 'receipt']) {
    test(`tamper-${tamper}`, (args) => {
      const receipt = prepareColdProfileBackup(args);
      const file = join(receipt.backupPath, 'Default', 'Preferences');
      if (tamper === 'bytes') fs.writeFileSync(file, 'different');
      if (tamper === 'mode') fs.chmodSync(file, 0o644);
      if (tamper === 'name') fs.renameSync(file, `${file}-renamed`);
      if (tamper === 'link') { fs.unlinkSync(join(receipt.backupPath, 'relative-link')); fs.symlinkSync('changed', join(receipt.backupPath, 'relative-link')); }
      if (tamper === 'manifest') fs.writeFileSync(join(receipt.backupDirectory, 'manifest.json'), '[]');
      if (tamper === 'receipt') receipt.profilePath = join(root, 'other-profile');
      assert.throws(() => verifyColdProfileBackup(receipt));
      assert.equal(fs.readFileSync(join(args.profilePath, 'Default', 'Preferences'), 'utf8'), 'fixture bytes');
    });
  }
  test('unsafe', (args) => {
    const linked = join(root, 'linked-parent');
    fs.symlinkSync(join(root, 'unsafe'), linked);
    assert.throws(() => prepareColdProfileBackup({ ...args, profilePath: join(linked, 'profile') }), /real directory/);
    for (const path of ['/', '/home', root, process.cwd()]) {
      assert.throws(() => prepareColdProfileBackup({ ...args, profilePath: path }));
    }
    fs.writeFileSync(join(args.profilePath, '.git'), 'worktree marker');
    assert.throws(() => prepareColdProfileBackup(args), /workspace/);
  });
  test('private-backup', (args) => {
    fs.chmodSync(args.backupRoot, 0o755);
    assert.throws(() => prepareColdProfileBackup(args), /private/);
  });
  test('backup-symlink', (args) => {
    const link = join(root, 'backup-link');
    fs.symlinkSync(args.backupRoot, link);
    assert.throws(() => prepareColdProfileBackup({ ...args, backupRoot: link }), /real directory/);
    const nested = join(args.profilePath, 'nested-backups');
    fs.mkdirSync(nested, { mode: 0o700 });
    assert.throws(() => prepareColdProfileBackup({ ...args, backupRoot: nested }), /disjoint/);
  });
  test('idle-failure', (args) => {
    assert.throws(() => prepareColdProfileBackup({ ...args, requireProfileIdle: () => false }), /idle/);
    assert.throws(() => prepareColdProfileBackup({ ...args, requireProfileIdle: async () => true }), /synchronously/);
    const receipt = prepareColdProfileBackup(args);
    assert.throws(() => restoreColdProfileBackup(receipt, { requireProfileIdle: () => { throw new Error('busy'); } }), /busy/);
    assert.ok(fs.existsSync(args.profilePath));
  });
  test('changed-during-backup', (args) => {
    let checks = 0;
    assert.throws(() => prepareColdProfileBackup({ ...args, requireProfileIdle: () => {
      if (++checks === 2) fs.writeFileSync(join(args.profilePath, 'changed'), 'concurrent');
    } }), /changed while backing up/);
  });
  test('after-move-failure', (args) => {
    const receipt = prepareColdProfileBackup(args);
    fs.writeFileSync(join(args.profilePath, 'candidate-work'), 'keep me');
    assert.throws(() => restoreColdProfileBackup(receipt, { requireProfileIdle: (path) => {
      if (!fs.existsSync(path)) throw new Error('injected after move');
    } }), /injected after move/);
    const journal = JSON.parse(fs.readFileSync(join(receipt.backupDirectory, 'restore.json'), 'utf8'));
    assert.equal(journal.phase, 'moved');
    assert.equal(fs.readFileSync(join(journal.displacedPath, 'candidate-work'), 'utf8'), 'keep me');
    const result = restoreColdProfileBackup(receipt, args);
    assert.equal(result.displacedPath, journal.displacedPath);
    assert.ok(fs.existsSync(join(args.profilePath, 'Default', 'Preferences')));
  });
  test('move-admission-drift', (args) => {
    const receipt = prepareColdProfileBackup(args);
    let calls = 0;
    assert.throws(() => restoreColdProfileBackup(receipt, { requireProfileIdle: () => {
      if (++calls === 2) fs.writeFileSync(join(args.profilePath, 'unexpected-work'), 'preserve');
    } }), /drift at move/);
    assert.equal(fs.readFileSync(join(args.profilePath, 'unexpected-work'), 'utf8'), 'preserve');
  });
  test('reappeared-before-commit', (args) => {
    const receipt = prepareColdProfileBackup(args);
    let calls = 0;
    assert.throws(() => restoreColdProfileBackup(receipt, { requireProfileIdle: () => {
      if (++calls === 4) fs.mkdirSync(args.profilePath, { mode: 0o700 });
    } }), /reappeared/);
    const journal = JSON.parse(fs.readFileSync(join(receipt.backupDirectory, 'restore.json'), 'utf8'));
    assert.ok(fs.existsSync(journal.displacedPath));
    assert.ok(fs.existsSync(journal.stagedPath));
    assert.ok(fs.existsSync(args.profilePath));
  });
  console.log(`Cold profile isolated fixtures passed: ${cases}`);
} finally {
  // This is exclusively the mkdtemp-owned test tree, never an operator profile.
  fs.rmSync(root, { recursive: true, force: true });
}
