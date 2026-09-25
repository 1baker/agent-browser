// Synthetic command-boundary coverage. Never invokes a real service manager,
// browser, container, authentication endpoint, or user's workstation state.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { isAbsolute, join } from 'node:path';

const candidate = process.env.AGENT_BROWSER_TEST_CANDIDATE_BIN;
assert.ok(candidate && isAbsolute(candidate), 'Explicit absolute candidate binary required');
const root = mkdtempSync(join(tmpdir(), 'agent-browser-healthy-reconcile-'));
const bin = join(root, '.local/bin');
const support = join(root, '.local/lib/agent-browser/0.28.0');
const log = join(root, 'commands.jsonl');
const fixture = join(root, 'probe.json');
const stateFile = join(root, '.agent-browser/service/state.json');
const envFile = join(root, '.agent-browser/.env');
const json = (path, value) => writeFileSync(path, JSON.stringify(value));
const routes = ['a', 'b'].map((label, index) => ({
  id: `guacamole-rdp-${label}`, provider: 'rdp_gateway', routeId: `guacamole:${index + 1}`,
  state: 'available', frameUrl: 'http://127.0.0.1:8092/guacamole/',
  target: { displayName: `:${10 + index}`, displayAllocationId: `display-${label}` },
}));
const good = () => ({
  install: { success: true, data: { workstationPayload: { ready: true }, issues: [] } },
  remote: { success: true, data: { install: { data: { success: true,
    data: { workstationPayload: { ready: true }, issues: [] } } }, issues: [], remoteControl: { ready: true },
    guacamole: { routePool: { success: true, data: { routePoolJson: routes } } } } },
});
const environmentText = [
  'AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway',
  'AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/',
  'AGENT_BROWSER_GUACAMOLE_HEADER_USER=fixture',
  'AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10',
  'AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11', '',
].join('\n');

try {
  for (const path of [bin, join(support, 'scripts'), join(support, 'guacamole'),
    join(root, '.agent-browser/service'), join(root, '.agent-browser/guacamole/secrets'),
    join(root, '.config/systemd/user')]) mkdirSync(path, { recursive: true });
  for (const path of ['manifest.json', 'guacamole/compose.yml',
    'scripts/smoke-rdp-guac-route-pool-readiness.js',
    'scripts/prepare-local-dashboard-retained-browser.js']) writeFileSync(join(support, path), '{}');
  writeFileSync(join(root, '.agent-browser/guacamole/secrets/guacamole.env'), '');
  writeFileSync(join(root, '.agent-browser/service/state.json.lock'), '');
  writeFileSync(join(root, '.config/systemd/user/agent-browser-dashboard.service'), '[Service]\n');
  const shim = `#!${process.execPath}
const fs = require('node:fs');
const path = require('node:path');
const name = path.basename(process.argv[1]);
const args = process.argv.slice(2);
fs.appendFileSync(process.env.QA_COMMAND_LOG, JSON.stringify({name, args}) + '\\n');
const probe = JSON.parse(fs.readFileSync(process.env.QA_PROBE));
if (name === 'id') { console.log('fixture agent-browser docker'); process.exit(0); }
if (name === 'systemctl' && args[1] === 'is-active') {
  console.log(probe.unitActive || 'active'); process.exit(probe.unitActive === 'inactive' ? 3 : 0);
}
if (name === 'systemctl' && args[1] === 'is-enabled') { console.log('enabled'); process.exit(0); }
if (name === 'agent-browser' && args.join(' ') === 'install doctor --json') {
  console.log(JSON.stringify(probe.install)); process.exit(probe.install.success ? 0 : 1);
}
if (name === 'agent-browser' && args.join(' ') === 'doctor remote-view --json') {
  console.log(JSON.stringify(probe.remote)); process.exit(0);
}
console.error('fixture refused mutation or unknown command'); process.exit(91);
`;
  for (const name of ['id', 'systemctl', 'agent-browser', 'docker', 'sudo', 'node', 'bash']) {
    writeFileSync(join(bin, name), shim, { mode: 0o755 });
  }
  const reset = () => {
    json(fixture, good());
    json(stateFile, { routePool: Object.fromEntries(routes.map(route => [route.id, route])) });
    writeFileSync(envFile, environmentText);
    writeFileSync(log, '');
  };
  const run = () => spawnSync(candidate, ['install', 'workstation', 'reconcile', '--json'], {
    cwd: root, env: { HOME: root, USER: 'fixture', PATH: `${bin}:/usr/bin:/bin`,
      AGENT_BROWSER_WORKSTATION_ROOT: root, AGENT_BROWSER_HOME: join(root, '.agent-browser'),
      QA_COMMAND_LOG: log, QA_PROBE: fixture },
    encoding: 'utf8', timeout: 30000, maxBuffer: 1024 * 1024,
  });
  const commands = () => readFileSync(log, 'utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
  const mutations = () => commands().filter(({name, args}) =>
    !(name === 'id' || (name === 'systemctl' && ['is-active', 'is-enabled'].includes(args[1])) ||
      (name === 'agent-browser' && ['install doctor --json', 'doctor remote-view --json'].includes(args.join(' ')))));
  const cases = [];
  reset();
  for (let pass = 0; pass < 2; pass++) {
    const result = run();
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const report = JSON.parse(result.stdout);
    assert.ok(report.steps.some(step => step.name === 'healthy-runtime-preserved'));
    assert.deepEqual(JSON.parse(readFileSync(report.receiptPath, 'utf8')), report);
    assert.equal(mutations().length, 0);
    assert.equal(readFileSync(envFile, 'utf8'), environmentText);
  }
  cases.push('healthy repeated reconcile does not enter mutation commands');
  for (const code of ['dashboard_publication_active', 'unknown_issue']) {
    reset();
    const probe = good();
    probe.install = { success: false, data: { workstationPayload: { ready: true }, issues: [{ code }] } };
    json(fixture, probe);
    assert.notEqual(run().status, 0);
    assert.equal(mutations().length, 0);
    cases.push(`${code} blocks before mutation`);
  }
  reset();
  const drift = good();
  drift.install.data.workstationPayload.ready = false;
  json(fixture, drift);
  assert.notEqual(run().status, 0);
  assert.equal(mutations().length, 0);
  cases.push('unverified payload blocks before mutation');
  reset();
  json(stateFile, { routePool: { [routes[0].id]: { ...routes[0], state: 'checked_out',
    currentRouteAllocationId: 'active-fixture', target: { ...routes[0].target, displayName: ':99' } } } });
  assert.notEqual(run().status, 0);
  assert.equal(mutations().length, 0);
  cases.push('active route mismatch blocks before mutation');
  const failedReady = good();
  failedReady.remote.data.issues = [{ code: 'route_display_access_missing' }];
  failedReady.remote.data.remoteControl.ready = false;
  json(fixture, failedReady);
  writeFileSync(log, '');
  assert.notEqual(run().status, 0);
  assert.equal(mutations().length, 0, 'known readiness failure must not bypass active route conflict');
  cases.push('known readiness failure still blocks active route conflict');
  reset();
  const missingRoutes = good();
  missingRoutes.remote.data.guacamole.routePool.data.routePoolJson = [];
  json(fixture, missingRoutes);
  assert.notEqual(run().status, 0);
  assert.equal(mutations().length, 0);
  cases.push('missing canonical evidence blocks despite earlier healthy receipt');
  reset();
  const incomplete = good();
  delete incomplete.remote.data.install;
  json(fixture, incomplete);
  assert.notEqual(run().status, 0);
  assert.equal(mutations().length, 0);
  cases.push('incomplete nested doctor evidence blocks');
  reset();
  json(fixture, { ...good(), unitActive: 'inactive' });
  assert.notEqual(run().status, 0, 'fixture must refuse the repair command');
  assert.ok(mutations().some(({name, args}) => name === 'systemctl' && args.includes('daemon-reload')));
  cases.push('stopped service reaches guarded repair boundary');
  reset();
  writeFileSync(envFile, environmentText.replace('DISPLAY_NAME=:10', 'DISPLAY_NAME=:12'));
  assert.notEqual(run().status, 0, 'fixture must refuse the repair command');
  assert.ok(mutations().some(({name, args}) => name === 'systemctl' && args.includes('daemon-reload')));
  cases.push('inactive configuration drift reaches guarded repair boundary');
  console.log(JSON.stringify({ success: true, cases, isolated: true, liveServicesTouched: false }));
} finally {
  rmSync(root, { recursive: true, force: true });
}
