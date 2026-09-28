// Run only with an explicit reviewed binary. Both installs use a disposable
// workstation root: native fixture mode skips host provisioning and services.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const candidate = process.env.AGENT_BROWSER_TEST_CANDIDATE_BIN;
assert.ok(candidate && isAbsolute(candidate), 'Explicit absolute candidate binary required');
const root = mkdtempSync(join(tmpdir(), 'controller-payload-install-'));
const scripts = dirname(fileURLToPath(import.meta.url));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
try {
  const environment = { HOME: root, PATH: '/usr/bin:/bin',
    AGENT_BROWSER_WORKSTATION_ROOT: root, AGENT_BROWSER_HOME: join(root, '.agent-browser') };
  const install = binary => {
    const result = spawnSync(binary, ['install', 'workstation', '--apply', '--json'], {
      env: environment, cwd: root, encoding: 'utf8', timeout: 60000, maxBuffer: 1024 * 1024,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);
    assert.equal(report.success, true);
    assert.equal(report.hostPlan.fixtureRoot, true);
    assert.equal(report.hostPrepared, false);
    assert.equal(report.state, 'payload_installed');
    assert.equal(report.reconcileReceipt, null);
    return report;
  };
  const report = install(candidate);
  const support = report.paths.supportDir;
  const opener = join(support, 'scripts/open-rdp-guac-route-displays.js');
  const expected = readFileSync(join(scripts, 'open-rdp-guac-route-displays.js'));
  const verify = () => {
    const manifest = JSON.parse(readFileSync(join(support, 'manifest.json')));
    assert.equal(manifest.binary.sha256, hash(readFileSync(candidate)));
    assert.equal(hash(readFileSync(report.paths.binary)), manifest.binary.sha256);
    for (const asset of manifest.controllerAssets.files) {
      assert.equal(hash(readFileSync(join(support, asset.path))), asset.sha256, asset.path);
    }
    assert.deepEqual(readFileSync(opener), expected);
    return hash(readFileSync(join(support, 'manifest.json')));
  };
  const firstManifest = verify();
  // Exercise reinstallation from the extracted installed executable, not the
  // source executable, proving the fix survives its own future installer.
  install(report.paths.binary);
  assert.equal(verify(), firstManifest);
  const fixture = spawnSync(process.execPath, ['--test', join(scripts, 'test-route-desktop-startup.mjs')], {
    env: { ...environment, AGENT_BROWSER_TEST_ROUTE_OPENER: opener },
    cwd: root, encoding: 'utf8', timeout: 60000, maxBuffer: 1024 * 1024,
  });
  assert.equal(fixture.error, undefined);
  assert.equal(fixture.status, 0, fixture.stdout + fixture.stderr);
  assert.match(fixture.stdout, /# pass 6/);
  console.log(JSON.stringify({ success: true, candidateSha256: hash(readFileSync(candidate)),
    openerSha256: hash(expected), isolatedInstall: true, isolatedReinstall: true,
    manifestVerified: true, extractedOpenerTestsPassed: 6, liveBrowserTouched: false }));
} finally { rmSync(root, { recursive: true, force: true }); }
