import assert from 'node:assert/strict';
import fs from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { createHash } from 'node:crypto';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { applyWorkstationProvenance, prepareWorkstationProvenance, verifyWorkstationProvenance } from './lib/local-dashboard-workstation-provenance.js';

const hash = (value) => createHash('sha256').update(value).digest('hex');
let count = 0;
function fixture(test) {
  const root = mkdtempSync(join(tmpdir(), 'workstation-provenance-test-'));
  const put = (path, value) => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, value); };
  const support = join(root, '.local/lib/agent-browser/1.2.3');
  const installBin = join(root, '.local/bin/agent-browser');
  const builtBin = join(root, 'candidate');
  const journalPath = join(root, 'journal.json');
  const manifestPath = join(support, 'manifest.json');
  const asset = join(support, 'scripts/controller.js');
  const bundle = { schema: { path: 'init.sql', sha256: hash('sql') }, files: [{ path: 'compose.yml', sha256: hash('compose') }] };
  const bundleBytes = JSON.stringify(bundle);
  const manifest = { schemaVersion: 'agent-browser.workstation-payload.v1', version: '1.2.3', binary: { sha256: hash('source') }, controllerAssets: { files: [{ path: 'scripts/controller.js', sha256: hash('controller') }] }, units: [{ name: 'test.service', sha256: hash('unit') }], guacamoleBundle: bundle, guacamoleBundleManifestSha256: hash(bundleBytes), futureField: { preserve: ['exact', 42] } };
  put(installBin, 'source'); put(builtBin, 'candidate'); put(asset, 'controller');
  put(join(root, '.config/systemd/user/test.service'), 'unit');
  put(join(support, 'guacamole/manifest.json'), bundleBytes);
  put(join(support, 'guacamole/compose.yml'), 'compose');
  put(join(support, 'guacamole/init.sql'), 'sql');
  const original = JSON.stringify(manifest, null, 4);
  put(manifestPath, original);
  const prepare = (controllerUpdates) => prepareWorkstationProvenance({ root, version: '1.2.3', installBin, builtBin, journalPath, controllerUpdates });
  try { test({ root, support, installBin, builtBin, manifestPath, manifest, original, asset, prepare, put }); count++; }
  finally { rmSync(root, { recursive: true, force: true }); }
}

fixture(({ prepare, installBin, manifestPath, original, put, manifest }) => {
  const record = JSON.parse(JSON.stringify(prepare()));
  assert.equal(verifyWorkstationProvenance(record, { selection: 'source', installBin }), true);
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'candidate', installBin }), /selected binary/);
  put(installBin, 'candidate');
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'candidate', installBin }), /incomplete/);
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  const committed = JSON.parse(readFileSync(manifestPath));
  committed.binary.sha256 = manifest.binary.sha256;
  assert.deepEqual(committed, manifest);
  put(installBin, 'source');
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /incomplete/);
  assert.equal(applyWorkstationProvenance(record, { selection: 'source', installBin }), true);
  assert.equal(readFileSync(manifestPath, 'utf8'), original);
  assert.equal(lstatSync(manifestPath).mode & 0o777, record.sourceMode);
});
fixture(({ prepare, installBin, put }) => { put(installBin, 'drift'); assert.throws(prepare, /installed binary digest/); });
fixture(({ prepare, asset, put }) => { put(asset, 'drift'); assert.throws(prepare, /asset digest/); });
fixture(({ prepare, manifest, manifestPath, put }) => { manifest.version = '2.0.0'; put(manifestPath, JSON.stringify(manifest)); assert.throws(prepare, /version/); });
fixture(({ prepare, manifest, manifestPath, put }) => { manifest.controllerAssets.files[0].path = '../../outside'; put(manifestPath, JSON.stringify(manifest)); assert.throws(prepare, /unsafe asset/); });
fixture(({ prepare, asset, root }) => { unlinkSync(asset); symlinkSync(join(root, 'candidate'), asset); assert.throws(prepare, /unsafe path/); });
fixture(({ prepare, manifestPath }) => { unlinkSync(manifestPath); assert.equal(prepare(), null); });
fixture(({ prepare, manifestPath, root }) => { unlinkSync(manifestPath); symlinkSync(join(root, 'missing'), manifestPath); assert.throws(prepare, /unsafe path/); });
fixture(({ prepare, installBin, manifestPath, put }) => { const record = prepare(); put(installBin, 'candidate'); put(manifestPath, '{}'); assert.throws(() => applyWorkstationProvenance(record, { selection: 'candidate', installBin }), /unreviewed/); });
fixture(({ prepare, installBin, asset, put }) => { const record = prepare(); put(installBin, 'candidate'); put(asset, 'tamper'); assert.throws(() => applyWorkstationProvenance(record, { selection: 'candidate', installBin }), /asset digest/); });
fixture(({ prepare, installBin, put }) => { const record = prepare(); put(join(record.snapshotDir, 'candidate.json'), '{}'); assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /snapshot digest/); });
fixture(({ prepare, installBin }) => { const record = prepare(); chmodSync(record.snapshotDir, 0o755); assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /unsafe snapshot/); });
fixture(({ prepare, installBin, root }) => { const record = prepare(); record.manifestPath = join(root, 'other'); assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /manifest path/); });
fixture(({ prepare, root }) => { const record = prepare(); assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin: join(root, 'candidate') }), /installation path/); });
fixture(({ prepare, installBin }) => { const record = prepare(); assert.throws(() => verifyWorkstationProvenance(record, { selection: 'unknown', installBin }), /selection/); });
fixture(({ root, installBin, builtBin }) => {
  assert.throws(() => prepareWorkstationProvenance({ root, version: '1.2.4', installBin, builtBin, journalPath: join(root, 'journal.json') }), /different workstation version/);
});
fixture(({ prepare, support, put }) => { put(join(support, 'guacamole/init.sql'), 'tamper'); assert.throws(prepare, /asset digest/); });
fixture(({ prepare, root, put }) => { put(join(root, '.config/systemd/user/test.service'), 'tamper'); assert.throws(prepare, /asset digest/); });
fixture(({ prepare, installBin, put }) => {
  const record = prepare();
  const snapshot = join(record.snapshotDir, 'candidate.json');
  const candidate = JSON.parse(readFileSync(snapshot));
  candidate.futureField = 'unauthorized';
  const changed = JSON.stringify(candidate);
  put(snapshot, changed);
  record.candidateManifestSha256 = hash(changed);
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /preserved fields/);
});
fixture(({ prepare, installBin, root }) => {
  const record = prepare();
  const snapshot = join(record.snapshotDir, 'source.json');
  unlinkSync(snapshot);
  symlinkSync(join(root, 'candidate'), snapshot);
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /unsafe path/);
});
assert.equal(applyWorkstationProvenance(null, {}), null);
assert.equal(verifyWorkstationProvenance(null, {}), null);

function controllerFixture(test) {
  fixture((context) => {
    const sourcePath = join(context.root, 'new-controller.js');
    context.put(sourcePath, 'new-controller');
    const updates = [{ path: 'scripts/controller.js', sourcePath, expectedSha256: hash('new-controller') }];
    test({ ...context, sourcePath, updates });
  });
}

controllerFixture(({ prepare, updates, installBin, asset, manifestPath, original, put }) => {
  chmodSync(asset, 0o755);
  const record = JSON.parse(JSON.stringify(prepare(updates)));
  assert.equal(record.controllerUpdates.length, 1);
  for (const suffix of ['source', 'candidate']) {
    assert.equal(lstatSync(join(record.snapshotDir, `controller-0-${suffix}`)).mode & 0o777, 0o600);
  }
  put(installBin, 'candidate');
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  assert.equal(readFileSync(asset, 'utf8'), 'new-controller');
  assert.equal(lstatSync(asset).mode & 0o777, 0o755);
  assert.equal(JSON.parse(readFileSync(manifestPath)).controllerAssets.files[0].sha256, hash('new-controller'));
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  put(installBin, 'source');
  assert.equal(applyWorkstationProvenance(record, { selection: 'source', installBin }), true);
  assert.equal(readFileSync(asset, 'utf8'), 'controller');
  assert.equal(readFileSync(manifestPath, 'utf8'), original);
  assert.equal(applyWorkstationProvenance(record, { selection: 'source', installBin }), true);
});

for (const selection of ['source', 'candidate']) {
  controllerFixture(({ prepare, updates, installBin, asset, manifestPath, original, put }) => {
    const record = prepare(updates);
    put(installBin, 'candidate');
    const rename = fs.renameSync;
    fs.renameSync = (from, to) => {
      if (to === manifestPath) throw new Error('simulated manifest commit failure');
      return rename(from, to);
    };
    syncBuiltinESMExports();
    try {
      assert.throws(() => applyWorkstationProvenance(record, { selection: 'candidate', installBin }), /simulated manifest commit failure/);
    } finally { fs.renameSync = rename; syncBuiltinESMExports(); }
    assert.equal(readFileSync(asset, 'utf8'), 'new-controller');
    assert.equal(readFileSync(manifestPath, 'utf8'), original);
    assert.throws(() => verifyWorkstationProvenance(record, { selection: 'candidate', installBin }), /incomplete/);
    put(installBin, selection);
    assert.equal(applyWorkstationProvenance(record, { selection, installBin }), true);
    assert.equal(readFileSync(asset, 'utf8'), selection === 'source' ? 'controller' : 'new-controller');
  });
}

controllerFixture(({ prepare, updates, installBin, asset, manifestPath, put }) => {
  const record = prepare(updates);
  put(installBin, 'candidate');
  put(manifestPath, readFileSync(join(record.snapshotDir, 'candidate.json')));
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'candidate', installBin }), /incomplete/);
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  assert.equal(readFileSync(asset, 'utf8'), 'new-controller');
});

controllerFixture(({ prepare, updates, installBin, asset, manifest, manifestPath, root, put }) => {
  const second = 'scripts/second.js';
  put(join(dirname(asset), 'second.js'), 'old-second');
  manifest.controllerAssets.files.push({ path: second, sha256: hash('old-second') });
  put(manifestPath, JSON.stringify(manifest));
  const sourcePath = join(root, 'second.js');
  put(sourcePath, 'new-second');
  updates.push({ path: second, sourcePath, expectedSha256: hash('new-second') });
  const record = prepare(updates);
  put(asset, 'new-controller');
  put(installBin, 'candidate');
  assert.equal(applyWorkstationProvenance(record, { selection: 'candidate', installBin }), true);
  assert.equal(readFileSync(join(dirname(asset), 'second.js'), 'utf8'), 'new-second');
  put(installBin, 'source');
  assert.equal(applyWorkstationProvenance(record, { selection: 'source', installBin }), true);
});

for (const suffix of ['source', 'candidate']) {
  controllerFixture(({ prepare, updates, installBin, put }) => {
    const record = prepare(updates);
    put(join(record.snapshotDir, `controller-0-${suffix}`), 'tampered');
    assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /controller snapshot digest/);
  });
}
controllerFixture(({ prepare, updates, installBin, asset, put }) => {
  const record = prepare(updates); put(asset, 'tampered');
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /unreviewed current controller/);
});
controllerFixture(({ prepare, updates, installBin, root, put }) => {
  const record = prepare(updates); put(join(root, '.config/systemd/user/test.service'), 'tampered');
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /preserved asset digest/);
});
controllerFixture(({ prepare, updates, installBin, support, manifest, manifestPath, asset, put }) => {
  const preserved = join(support, 'scripts/preserved.js');
  put(preserved, 'preserved');
  manifest.controllerAssets.files.push({ path: 'scripts/preserved.js', sha256: hash('preserved') });
  put(manifestPath, JSON.stringify(manifest));
  const record = prepare(updates); put(preserved, 'tampered'); put(installBin, 'candidate');
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'candidate', installBin }), /preserved asset digest/);
  assert.equal(readFileSync(asset, 'utf8'), 'controller');
});
controllerFixture(({ prepare, updates, installBin }) => {
  const record = prepare(updates); record.controllerUpdates.push({ ...record.controllerUpdates[0] });
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /duplicate controller/);
});
controllerFixture(({ prepare, updates, installBin }) => {
  const record = prepare(updates); chmodSync(join(record.snapshotDir, 'controller-0-candidate'), 0o644);
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /unsafe manifest or snapshot/);
});
controllerFixture(({ prepare, updates, installBin, asset }) => {
  const record = prepare(updates); chmodSync(asset, 0o755);
  assert.throws(() => applyWorkstationProvenance(record, { selection: 'source', installBin }), /controller mode changed/);
});
controllerFixture(({ prepare, updates }) => { assert.throws(() => prepare([...updates, ...updates]), /duplicate controller/); });
controllerFixture(({ prepare, updates, manifest, manifestPath, put }) => {
  manifest.controllerAssets.files.push({ ...manifest.controllerAssets.files[0] }); put(manifestPath, JSON.stringify(manifest));
  assert.throws(() => prepare(updates), /exactly one/);
});
for (const path of ['scripts/new.js', '../escape', 'scripts/../controller.js', '/tmp/controller.js', 'manifest.json', 'scripts//controller.js']) {
  controllerFixture(({ prepare, updates }) => { updates[0].path = path; assert.throws(() => prepare(updates), /existing manifest entry|unsafe/); });
}
for (const expectedSha256 of [undefined, 'bad', hash('wrong'), 'A'.repeat(64)]) {
  controllerFixture(({ prepare, updates }) => { updates[0].expectedSha256 = expectedSha256; assert.throws(() => prepare(updates), /SHA-256|candidate controller digest/); });
}
for (const mode of [0o666, 0o4755, 0o755]) {
  controllerFixture(({ prepare, updates, sourcePath }) => { chmodSync(sourcePath, mode); assert.throws(() => prepare(updates), /controller mode/); });
}
controllerFixture(({ prepare, updates, sourcePath, root }) => {
  unlinkSync(sourcePath); symlinkSync(join(root, 'candidate'), sourcePath);
  assert.throws(() => prepare(updates), /unsafe path/);
});
controllerFixture(({ prepare, updates, sourcePath }) => {
  updates[0].sourcePath = `${dirname(sourcePath)}/./new-controller.js`;
  assert.throws(() => prepare(updates), /noncanonical/);
});
controllerFixture(({ prepare, updates, manifestPath }) => { unlinkSync(manifestPath); assert.throws(() => prepare(updates), /existing workstation manifest/); });
controllerFixture(({ prepare, updates, installBin, put }) => {
  const record = prepare(updates);
  const path = join(record.snapshotDir, 'candidate.json');
  const manifest = JSON.parse(readFileSync(path)); manifest.controllerAssets.files.push({ path: 'scripts/unapproved.js', sha256: hash('extra') });
  const changed = JSON.stringify(manifest); put(path, changed); record.candidateManifestSha256 = hash(changed);
  assert.throws(() => verifyWorkstationProvenance(record, { selection: 'source', installBin }), /preserved fields/);
});
console.log(`Workstation provenance: ${count} isolated cases passed`);
