#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { repairTerminalWorkstationProvenance } from './lib/local-dashboard-terminal-provenance-repair.js';

const root = mkdtempSync(join(tmpdir(), 'terminal-provenance-repair-'));
try {
  const version = '1.2.3';
  const installBin = join(root, '.local/bin/agent-browser');
  const supportRoot = join(root, '.local/lib/agent-browser', version);
  const unitsRoot = join(root, '.config/systemd/user');
  const journalPath = join(root, '.agent-browser/publications/local-dashboard-publication.json');
  mkdirSync(join(supportRoot, 'scripts'), { recursive: true });
  mkdirSync(join(supportRoot, 'guacamole'), { recursive: true });
  mkdirSync(unitsRoot, { recursive: true });
  mkdirSync(join(root, '.local/bin'), { recursive: true });
  mkdirSync(join(root, '.agent-browser/publications'), { recursive: true });

  put(installBin, 'replacement-runtime\n', 0o755);
  put(join(supportRoot, 'scripts/controller.js'), 'controller\n');
  put(join(unitsRoot, 'agent-browser.service'), 'unit\n');
  put(join(supportRoot, 'guacamole/compose.yml'), 'compose\n');
  put(join(supportRoot, 'guacamole/schema.sql'), 'schema\n');
  const bundle = {
    files: [{ path: 'compose.yml', sha256: hash('compose\n') }],
    schema: { path: 'schema.sql', sha256: hash('schema\n') },
  };
  const bundleBytes = Buffer.from(`${JSON.stringify(bundle, null, 2)}\n`);
  put(join(supportRoot, 'guacamole/manifest.json'), bundleBytes);
  const installedSha256 = hash('replacement-runtime\n');
  const sourceBinarySha256 = hash('older-runtime\n');
  const manifest = {
    schemaVersion: 'agent-browser.workstation-payload.v1',
    version,
    binary: { sha256: sourceBinarySha256 },
    controllerAssets: {
      files: [{ path: 'scripts/controller.js', sha256: hash('controller\n') }],
    },
    units: [{ name: 'agent-browser.service', sha256: hash('unit\n') }],
    guacamoleBundle: bundle,
    guacamoleBundleManifestSha256: hash(bundleBytes),
    futureField: { preserved: true },
  };
  const manifestPath = join(supportRoot, 'manifest.json');
  put(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 0o644);
  const journalRecord = {
    transactionId: 'local-dashboard-fixture',
    terminal: true,
    phase: 'recovered_ready',
    installBin,
    artifactEvidence: {
      built: { sha256: installedSha256 },
      replacement: {
        path: installBin,
        expectedSha256: installedSha256,
        actualSha256: installedSha256,
        verified: true,
      },
    },
  };

  const receipt = repairTerminalWorkstationProvenance({
    root,
    version,
    installBin,
    journalPath,
    journalRecord,
  });
  assert.equal(receipt.changed, true);
  assert.equal(receipt.installedSha256, installedSha256);
  const repaired = JSON.parse(readFileSync(manifestPath, 'utf8'));
  assert.equal(repaired.binary.sha256, installedSha256);
  assert.deepEqual(repaired.futureField, { preserved: true });
  assert.deepEqual(
    JSON.parse(readFileSync(join(receipt.snapshotDir, 'source.json'), 'utf8')),
    manifest,
  );

  const idempotent = repairTerminalWorkstationProvenance({
    root,
    version,
    installBin,
    journalPath,
    journalRecord,
  });
  assert.equal(idempotent.changed, false);

  const unverified = structuredClone(journalRecord);
  unverified.artifactEvidence.replacement.verified = false;
  assert.throws(
    () => repairTerminalWorkstationProvenance({
      root, version, installBin, journalPath, journalRecord: unverified,
    }),
    /exact journaled replacement/,
  );

  console.log('Terminal workstation provenance repair: 3 cases passed');
} finally {
  rmSync(root, { recursive: true, force: true });
}

function put(path, value, mode = 0o644) {
  writeFileSync(path, value, { mode });
  chmodSync(path, mode);
}

function hash(value) {
  return createHash('sha256').update(value).digest('hex');
}
