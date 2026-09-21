// Production opener with isolated HOME, fake provider/browser commands and
// synthetic display observations. No real browser, login or desktop is touched.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

function run(mode) {
  const root = mkdtempSync(join(tmpdir(), 'route-desktop-startup-'));
  try {
    const bin = join(root, 'bin');
    mkdirSync(bin);
    const log = join(root, 'calls');
    writeFileSync(log, '');
    writeFileSync(join(bin, 'curl'), `#!${process.execPath}
const auth = process.argv.some(v=>v.startsWith('Remote-User:'));
console.log(JSON.stringify({authToken:process.env.MODE==='password'&&auth?undefined:'synthetic-token'}));
process.stdout.write('200');`, { mode: 0o700 });
    const browser = join(bin, 'browser');
    writeFileSync(browser, `#!${process.execPath}
const fs=require('node:fs'); const args=process.argv.slice(2);
const session=args[args.indexOf('--session')+1];
const action=args.includes('set')?'headers':args.includes('about:blank')?'launch':'navigate';
fs.appendFileSync(process.env.CALLS,JSON.stringify({session,action})+'\\n');
if(process.env.MODE===action+'-timeout') { setTimeout(()=>{},3000); }
else {
  const rejected=process.env.MODE===action+'-rejected';
  if(action==='navigate') fs.writeFileSync(process.env.HOME+'/'+session,'ready');
  console.log(JSON.stringify({success:!rejected,error:rejected?'synthetic denial':undefined}));
}`, { mode: 0o700 });
    writeFileSync(join(root, 'inspect-rdp-route-displays.js'), `
const fs=require('node:fs'); const rows={};
for(const [label,n] of [['A',10],['B',11]]) if(fs.existsSync(process.env.HOME+'/rdp-guac-route-'+label.toLowerCase()+'-viewer')) rows[label]={displayName:':'+n};
console.log(JSON.stringify({success:Object.keys(rows).length===2,routeSpecificUsers:rows}));`);
    const routes = ['a', 'b'].map((label, i) => ({ id: `guacamole-rdp-${label}`,
      connectionId: String(i + 1), frameUrl: `http://127.0.0.1:1/guacamole/#/client/${label}` }));
    const opener = process.env.AGENT_BROWSER_TEST_ROUTE_OPENER
      || fileURLToPath(new URL('./open-rdp-guac-route-displays.js', import.meta.url));
    const r = spawnSync(process.execPath, [opener,
      '--wait-ms', '0', '--route-navigation-timeout-ms', '200', '--route-display-timeout-ms', '0'], {
      env: { HOME: root, USER: 'fixture', PATH: `${bin}:/usr/bin:/bin`, MODE: mode, CALLS: log,
        AGENT_BROWSER_HOME: root, AGENT_BROWSER_GUACAMOLE_SECRET_FILE: join(root, 'absent'),
        AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT: root, AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD: browser,
        AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(routes),
        GUACAMOLE_ADMIN_USERNAME: 'synthetic', GUACAMOLE_ADMIN_PASSWORD: 'synthetic' },
      encoding: 'utf8', timeout: 10000,
    });
    assert.equal(r.error, undefined);
    return { status: r.status, result: JSON.parse(r.stdout),
      calls: readFileSync(log, 'utf8').trim().split('\n').filter(Boolean).map(JSON.parse) };
  } finally { rmSync(root, { recursive: true, force: true }); }
}

test('cold desktop startup opens and verifies both exact route viewers', () => {
  const r = run('success');
  assert.equal(r.status, 0);
  assert.equal(r.result.success, true);
  assert.deepEqual(r.calls.map(c=>c.action), ['launch','headers','navigate','launch','headers','navigate']);
  assert.deepEqual(r.result.openedRoutes.map(r=>r.displayName), [':10', ':11']);
});
for (const phase of ['launch', 'headers', 'navigate']) {
  test(`${phase} JSON rejection stops at the failed command`, () => {
    const r = run(`${phase}-rejected`);
    assert.equal(r.status, 1);
    assert.equal(r.result.success, false);
    assert.match(r.result.error, /command_rejected/);
    assert.equal(r.calls.at(-1).action, phase);
    assert.ok(r.calls.every(c=>c.session==='rdp-guac-route-a-viewer'));
  });
}
test('unsupported password-only authentication is refused before browser launch', () => {
  const r = run('password');
  assert.equal(r.status, 1);
  assert.deepEqual(r.calls, []);
});
test('navigation timeout is reported as uncertain without display polling or retry', () => {
  const r = run('navigate-timeout');
  assert.equal(r.status, 1);
  assert.match(r.result.error, /navigation_outcome_unknown/);
  assert.deepEqual(r.calls.map(c=>c.action), ['launch','headers','navigate']);
});
