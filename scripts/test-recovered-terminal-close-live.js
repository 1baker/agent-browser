#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, mkdirSync, readFileSync, readlinkSync, symlinkSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createSmokeContext, createMcpStdioClient, runCli } from './smoke-utils.js';

// Explicit binaries, isolated HOME/socket/profile, loopback-only page. Never
// discover, attach to, or signal a production browser or daemon.
const binary = resolve(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD || 'cli/target/debug/agent-browser');
const chrome = process.env.AGENT_BROWSER_SMOKE_CHROME_PATH;
assert.ok(chrome && existsSync(chrome), 'Set AGENT_BROWSER_SMOKE_CHROME_PATH to installed Linux Chrome');
const context = createSmokeContext({ prefix: 'ab-terminal-close-', sessionPrefix: 'terminal-close' });
const chromeCache = join(context.agentHome, 'browsers', 'chrome-153.0.8010.36');
mkdirSync(chromeCache, { recursive: true });
symlinkSync(resolve(chrome), join(chromeCache, 'chrome'));
const profile = join(context.tempHome, 'profile');
const config = join(context.tempHome, 'config.json');
writeFileSync(config, JSON.stringify({ executablePath: chrome, profile, service: { defaultBrowserBuild: 'stock_chrome' } }));
Object.assign(context.env, {
  AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD: binary,
  AGENT_BROWSER_CONFIG: config,
  AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'disabled',
  AGENT_BROWSER_BROWSER_HOST: 'local_headless',
  AGENT_BROWSER_HEADED: '0',
});
const call = async (...args) => {
  const output = await runCli(context, ['--json', '--session', context.session, ...args], 30000);
  const response = JSON.parse(output.stdout);
  assert.equal(response.success, true, output.stdout);
  return response.data;
};
const statePath = join(context.agentHome, 'service', 'state.json');
const readState = () => JSON.parse(readFileSync(statePath, 'utf8'));
const browserId = `session:${context.session}`;
const targetAt = async endpoint => {
  const address = new URL(endpoint);
  const targets = await (await fetch(`http://${address.host}/json/list`)).json();
  return targets.find(tab => tab.url === url)?.id;
};
const alive = pid => { try { process.kill(pid, 0); return true; } catch (error) { if (error.code === 'ESRCH') return false; throw error; } };
const waitFor = async (predicate, label) => {
  const end = Date.now() + 10000;
  while (!predicate()) {
    assert.ok(Date.now() < end, label);
    await new Promise(resolveWait => setTimeout(resolveWait, 50));
  }
};
let commits = 0;
const server = createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/commit') {
    commits++;
    res.end(JSON.stringify({ count: commits }));
    return;
  }
  res.setHeader('Content-Type', 'text/html');
  res.end('<!doctype html><title>Disposable recovery close QA</title><button id="commit">Commit once</button><output id="result">Waiting</output><script>document.querySelector("#commit").onclick=async()=>{document.querySelector("#result").textContent=await(await fetch("/commit",{method:"POST"})).text();}</script>');
});
await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
const url = `http://127.0.0.1:${server.address().port}/`;
let mcp;
let completed = false;
try {
  await call('service', 'access-plan', '--service-name', 'RecoveryCloseQA', '--agent-name', 'fixture', '--task-name', context.session, '--url', url, '--browser-host', 'local_headless');
  await call('--browser-host', 'local_headless', 'open', url);
  await call('click', '#commit');
  await waitFor(() => commits === 1, 'first commit missing');
  const before = readState().browsers[browserId];
  assert.ok(before.pid && before.cdpEndpoint, 'persisted browser identity required');
  const target = await targetAt(before.cdpEndpoint);
  assert.ok(target, 'exact target required');
  const daemonPid = Number(readFileSync(join(context.socketDir, `${context.session}.pid`), 'utf8'));
  assert.equal(readlinkSync(`/proc/${daemonPid}/exe`), binary);
  const daemonEnv = readFileSync(`/proc/${daemonPid}/environ`, 'utf8').split('\0');
  assert.ok(daemonEnv.includes(`AGENT_BROWSER_SOCKET_DIR=${context.socketDir}`));
  assert.ok(daemonEnv.includes(`HOME=${context.tempHome}`));
  process.kill(daemonPid, 'SIGKILL');
  await waitFor(() => !alive(daemonPid), 'disposable daemon did not exit');
  await call('tab', 'list', '--verbose');
  await call('tab', 'list', '--verbose');
  assert.equal(await targetAt(before.cdpEndpoint), target);
  assert.equal((await call('get', 'browser-pid')).pid, before.pid);
  assert.equal((await call('get', 'cdp-url')).cdpUrl, before.cdpEndpoint);
  assert.equal(JSON.parse((await call('get', 'text', '#result')).text).count, 1);
  await call('close');
  assert.equal(alive(before.pid), true, 'ordinary detach must preserve Chrome');
  const detached = readState().browsers[browserId];
  assert.ok(detached, 'detach must preserve browser mapping');
  for (const field of ['pid', 'profileId', 'cdpEndpoint']) assert.equal(detached[field], before[field], field);
  await call('tab', 'list', '--verbose');
  await call('tab', 'list', '--verbose');
  assert.equal(await targetAt(before.cdpEndpoint), target);
  mcp = createMcpStdioClient({ context, args: ['--session', context.session, 'mcp', 'serve'], onFatal: message => { throw new Error(message); } });
  await mcp.send('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'recovery-close-fixture', version: '1' } });
  mcp.notify('notifications/initialized');
  const result = await mcp.send('tools/call', { name: 'service_request', arguments: {
    serviceName: 'RecoveryCloseQA', agentName: 'fixture', taskName: context.session,
    sessionName: context.session, browserId, action: 'service_browser_close',
    params: { browserId }, jobTimeoutMs: 20000,
  } });
  const terminal = JSON.parse(result.content[0].text);
  assert.equal(terminal.success, true, JSON.stringify(terminal));
  await waitFor(() => !alive(before.pid), 'terminal close did not stop Chrome');
  assert.throws(() => lstatSync(join(profile, 'SingletonLock')), { code: 'ENOENT' });
  assert.equal(readState().browsers[browserId], undefined, 'terminal mapping must be removed');
  assert.equal(commits, 1, 'recovery or cleanup replayed the action');
  await call('close');
  completed = true;
  console.log(JSON.stringify({ success: true, binarySha256: createHash('sha256').update(readFileSync(binary)).digest('hex'), browserPid: before.pid, target, commits, detachedMappingPreserved: true, terminalProcessExitVerified: true, profileLockReleased: true }));
} finally {
  if (mcp) await mcp.close();
  await new Promise(resolveClose => server.close(resolveClose));
  if (completed) context.cleanupTempHome();
  else console.error(`Preserved failed fixture for exact recovery: ${context.tempHome}`);
}
