import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, delimiter } from 'node:path';
import { spawnSync } from 'node:child_process';

const script = resolve('scripts/setup-rdp-guac-temporary-route.js');
function fixture(extra = {}) {
  const root = mkdtempSync(join(tmpdir(), 'temporary-rdp-route-fixture-'));
  const bin = join(root, 'bin');
  mkdirSync(bin);
  const program = `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const command = path.basename(process.argv[1]);
const args = process.argv.slice(2);
const input = fs.readFileSync(0, 'utf8');
fs.appendFileSync(process.env.FIXTURE_LOG, JSON.stringify({command,args,input})+'\\n', {mode:384});
if(command==='getent') process.exit(process.env.FIXTURE_USER_EXISTS ||
  (process.env.FIXTURE_LATE_USER && fs.existsSync(process.env.FIXTURE_LOG+'.backup')) ? 0 : 2);
if(command==='sudo') {
  if(process.env.FIXTURE_SUDO_FAIL) { console.error('SECRET_FAILURE_TEXT'); process.exit(1); }
  process.exit(0);
}
if(command==='agent-browser') { fs.writeFileSync(process.env.FIXTURE_LOG+'.backup', '1'); console.log(JSON.stringify({success: !process.env.FIXTURE_BACKUP_FAIL})); process.exit(0); }
if(command==='docker') {
  if(input.includes('BEGIN;')) {
    if(process.env.FIXTURE_SQL_FAIL) { console.error(input); process.exit(1); }
    console.log('9');
  } else if(input.includes('count(*)')) console.log('1');
  else if(process.env.FIXTURE_CONNECTION_EXISTS) console.log('8');
}
`;
  for (const command of ['docker', 'sudo', 'getent', 'agent-browser']) {
    writeFileSync(join(bin, command), program, { mode: 0o700 });
  }
  const env = { ...process.env, PATH: bin + delimiter + process.env.PATH, USER: 'fixtureuser',
    AGENT_BROWSER_HOME: join(root, 'agent-home'), FIXTURE_LOG: join(root, 'calls.jsonl'), ...extra };
  const run = (...args) => spawnSync(process.execPath, [script, ...args], { env, encoding: 'utf8' });
  const calls = () => existsSync(env.FIXTURE_LOG)
    ? readFileSync(env.FIXTURE_LOG, 'utf8').trim().split('\n').map(JSON.parse) : [];
  return { root, env, run, calls };
}
const dry = fixture();
const entry = {id:'guacamole-rdp-c',connectionName:'Agent Browser RDP Temporary Route C',
  frameUrl:'http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=',
  connectionId:'3',routeId:'guacamole:3',target:{routeUser:'agent-browser-rdp-c'}};
const selection = spawnSync(process.execPath, [resolve('scripts/open-rdp-guac-route-displays.js'), '--route-label', 'C', '--dry-run'], {
  encoding:'utf8', env:{...dry.env, AGENT_BROWSER_RDP_ROUTE_POOL_JSON:JSON.stringify([{id:'guacamole-rdp-b'},entry,{id:'guacamole-rdp-a'}])},
});
assert.equal(selection.status, 0, selection.stdout);
assert.equal(JSON.parse(selection.stdout).selectedRoutes[0].label, 'C');
assert.equal(JSON.parse(selection.stdout).selectedRoutes.length, 1);
assert.deepEqual(dry.calls(), []);
assert.equal(dry.run('--label', 'C', '--dry-run').status, 0);
assert.equal(existsSync(dry.env.AGENT_BROWSER_HOME), false);
assert(!dry.calls().some(c => c.args.includes('ensure-rdp-route-user') || c.command==='agent-browser' || c.input.includes('INSERT')));
for (const extra of [{FIXTURE_USER_EXISTS:'1'}, {FIXTURE_CONNECTION_EXISTS:'1'}, {FIXTURE_SUDO_FAIL:'1'}, {FIXTURE_BACKUP_FAIL:'1'}, {FIXTURE_LATE_USER:'1'}]) {
  const f = fixture(extra);
  const result = f.run('--label', 'C', '--apply');
  assert.notEqual(result.status, 0);
  assert.doesNotMatch(result.stdout + result.stderr, /SECRET_FAILURE_TEXT/);
  assert(!f.calls().some(c => c.args.includes('ensure-rdp-route-user') || c.input.includes('INSERT')));
}
for (const flag of ['--apply', '--dry-run']) {
  const f = fixture();
  assert.notEqual(f.run('--label', 'A', flag).status, 0);
  assert.deepEqual(f.calls(), []);
}
const success = fixture();
const result = success.run('--label', 'C', '--apply');
assert.equal(result.status, 0, result.stdout);
assert.equal(JSON.parse(result.stdout).status, 'provisioned_not_browser_ready');
const receipt = JSON.parse(readFileSync(join(success.env.AGENT_BROWSER_HOME, 'temporary-routes/C.json')));
assert(!result.stdout.includes(receipt.password));
assert(!success.calls().some(c => JSON.stringify(c.args).includes(receipt.password)));
assert(!success.calls().some(c => c.args.includes('restart') || c.args.includes('compose') || c.args.includes('reconcile')));
assert.equal(success.calls().filter(c => c.args.includes('ensure-rdp-route-user')).length, 1);
assert.notEqual(success.run('--label', 'C', '--apply').status, 0);
assert.equal(success.calls().filter(c => c.args.includes('ensure-rdp-route-user')).length, 1);
const failed = fixture({FIXTURE_SQL_FAIL:'1'});
const failure = failed.run('--label', 'C', '--apply');
assert.notEqual(failure.status, 0);
assert(!failure.stdout.includes('INSERT'));
assert(existsSync(join(failed.env.AGENT_BROWSER_HOME, 'temporary-routes/C.json')));
assert.notEqual(failed.run('--label', 'C', '--apply').status, 0);
console.log('temporary route process fixtures passed (dry-run, isolation, failure redaction, fail-closed retry)');
