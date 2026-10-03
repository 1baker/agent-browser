#!/usr/bin/env node

import { createServer } from 'node:http';
import { mkdirSync, readFileSync, existsSync, symlinkSync } from 'node:fs';
import { basename, dirname, join, resolve, extname } from 'node:path';
import { once } from 'node:events';
import { assert, closeSession, createSmokeContext, httpJson } from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';
import { findLatestInstalledSmokeBrowser, isolateSmokeBrowserEnvironment } from './lib/smoke-browser-fixture.js';

// A synthetic gate controlled by the fixture server. This never contacts a
// CAPTCHA provider, handles real credentials, or claims human verification.
const context = createSmokeContext({ prefix: 'ab-workbench-', sessionPrefix: 'workbench' });
isolateSmokeBrowserEnvironment(context);
const executable = findLatestInstalledSmokeBrowser();
assert(executable, 'An installed Chrome for Testing is required');
context.env.AGENT_BROWSER_EXECUTABLE_PATH = executable;
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY = 'disabled';
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');
const fixtureVersion = join(context.agentHome, 'browsers', basename(dirname(executable)));
mkdirSync(fixtureVersion, { recursive: true, mode: 0o700 });
symlinkSync(executable, join(fixtureVersion, basename(executable)));
const serviceName = 'WorkbenchLiveFixture';
const labels = { serviceName, agentName: 'fixture', taskName: 'plan0133GateJourney' };
let port;
let stage = 'gate';
let browserPid;
let daemonIdentity;
let uiHandle;
let taskHandle;
let closed = false;
const report = { checks: [] };
const dashboardRoot = resolve('packages/dashboard/out');
const evidenceDir = process.env.AGENT_BROWSER_WORKBENCH_EVIDENCE_DIR;

const server = createServer(async (req, res) => {
  try {
    if (req.url.startsWith('/api/')) {
      const chunks = [];
      for await (const chunk of req) chunks.push(chunk);
      const body = Buffer.concat(chunks);
      const upstream = await fetch(`http://127.0.0.1:${port}${req.url}`, {
        method: req.method,
        headers: { ...(body.length ? { 'content-type': 'application/json' } : {}), ...(req.headers.cookie ? { cookie: req.headers.cookie } : {}) },
        ...(body.length ? { body } : {}),
      });
      res.writeHead(upstream.status, { 'content-type': upstream.headers.get('content-type') || 'application/json', ...(upstream.headers.get('set-cookie') ? { 'set-cookie': upstream.headers.get('set-cookie') } : {}) });
      res.end(Buffer.from(await upstream.arrayBuffer()));
      return;
    }
    if (req.url === '/fixture-stage') {
      res.setHeader('content-type', 'application/json');
      res.end(JSON.stringify({ stage }));
      return;
    }
    if (req.url === '/fixture') {
      res.setHeader('content-type', 'text/html');
      res.end(`<!doctype html><title>Synthetic gate</title><main id="content"></main>
<script>
let previous;
async function refresh() {
 const {stage} = await (await fetch('/fixture-stage')).json();
 if (stage === previous) return; previous = stage;
 document.title = stage === 'gate' ? 'Synthetic gate' : 'Fixture article';
 document.querySelector('#content').innerHTML = stage === 'gate'
  ? '<section id="human-gate">Synthetic operator handoff</section>'
  : stage === 'ready' ? '<article id="ready"><h1>Fixture article</h1><button id="finish" onclick="this.textContent=&#39;Completed&#39;">Finish task</button></article>'
  : '<p id="unknown">Waiting for page evidence</p>';
}
refresh(); setInterval(refresh, 100);
</script>`);
      return;
    }
    const pathname = new URL(req.url, 'http://fixture').pathname;
    let path = resolve(dashboardRoot, `.${pathname}`);
    if (!path.startsWith(`${dashboardRoot}/`)) { res.writeHead(404); res.end(); return; }
    if (!extname(path) && existsSync(`${path}.html`)) path += '.html';
    const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.txt': 'text/plain' };
    res.setHeader('content-type', types[extname(path)] || 'application/octet-stream');
    res.end(readFileSync(path));
  } catch {
    res.writeHead(404); res.end();
  }
});

async function request(action, options = {}, expectSuccess = true) {
  const response = await httpJson(port, 'POST', '/api/service/request', {
    ...labels, action, jobTimeoutMs: 20000, ...options,
  });
  if (expectSuccess) assert(response.success === true, `${action}: ${JSON.stringify(response)}`);
  return response;
}

async function probe(handle) {
  return request('probe', {
    serviceTabHandle: handle, timeoutMs: 2000, maxReturnBytes: 512,
    probe: { observePageGate: true, detectors: [{ id: 'page', type: 'url_title' }] },
  });
}

async function evaluate(handle, expression) {
  const r = await request('evaluate', { serviceTabHandle: handle, expression, timeoutMs: 2000, maxReturnBytes: 4096 });
  assert(r.data.ok, `evaluate failed: ${JSON.stringify(r.data)}`);
  return r.data.result;
}

async function until(fn, label) {
  const deadline = Date.now() + 12000;
  while (Date.now() < deadline) {
    if (await fn()) return;
    await new Promise((done) => setTimeout(done, 200));
  }
  throw new Error(`Timed out: ${label}`);
}

function passed(name) { report.checks.push(name); console.log(`PASS ${name}`); }

function processIdentity(pid) {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    return `${pid}:${stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19]}`;
  } catch { return null; }
}

function currentDaemonIdentity() {
  const pid = Number(readFileSync(join(context.socketDir, `${context.session}.pid`), 'utf8'));
  const identity = processIdentity(pid);
  assert(identity, 'Fixture daemon identity unavailable');
  return { pid, identity };
}

async function verifiedClose() {
  await closeSession(context);
  if (daemonIdentity) await until(async () => processIdentity(daemonIdentity.pid) !== daemonIdentity.identity, 'fixture daemon termination');
  if (browserPid) await until(async () => !processIdentity(browserPid), 'fixture browser termination');
}

async function cleanup() {
  if (closed) return;
  closed = true;
  try {
    await verifiedClose();
  } catch (error) {
    throw new Error(`${error.message}; fixture home preserved at ${context.tempHome}`);
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
  if (browserPid) {
    let alive = true;
    try { process.kill(browserPid, 0); } catch { alive = false; }
    assert(!alive, `Fixture browser PID ${browserPid} survived cleanup; home preserved at ${context.tempHome}`);
  }
  context.cleanupTempHome();
}

const timeout = setTimeout(() => {
  console.error('Workbench fixture exceeded 180 seconds');
  cleanup().finally(() => process.exit(1));
}, 180000);

try {
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const origin = `http://127.0.0.1:${server.address().port}`;
  port = await ensureStreamPort(context);
  daemonIdentity = currentDaemonIdentity();
  const sessions = await httpJson(port, 'GET', '/api/sessions');
  assert(Array.isArray(sessions) && sessions.every((s) => s.session === context.session), 'Fixture discovered a foreign browser');
  passed('session inventory excludes foreign browsers');
  const plan = await httpJson(port, 'GET', `/api/service/access-plan?serviceName=${serviceName}&agentName=fixture&taskName=plan0133GateJourney&url=${encodeURIComponent(origin + '/fixture')}&browserHost=local_headless`);
  assert(plan.success, 'No-launch access plan failed');
  const policy = await httpJson(port, 'POST', '/api/service/site-policies/workbench-fixture', {
    originPattern: origin, browserHost: 'local_headless',
    pageGateObserver: { challengeTitleContains: ['Synthetic gate'], challengeSelectors: ['#human-gate'], readySelectors: ['#ready'] },
  });
  assert(policy.success, `Policy failed: ${JSON.stringify(policy)}`);
  taskHandle = (await request('tab_new', { params: { url: `${origin}/fixture`, headless: true, waitUntil: 'load' } })).data.serviceTabHandle;
  assert(taskHandle?.valid, 'Missing exact task handle');
  await until(async () => (await probe(taskHandle)).data.pageGate.classification === 'challenge', 'gate observation');
  let state = JSON.parse(readFileSync(join(context.agentHome, 'service/state.json'), 'utf8'));
  browserPid = state.browsers[taskHandle.browserId].pid;
  passed('exact browser acquisition and observed human gate');
  const blocked = await request('click', { serviceTabHandle: taskHandle, params: { selector: '#human-gate' } }, false);
  assert(!blocked.success && JSON.stringify(blocked).includes('paused for human gate'), 'Waiting gate did not pause click');
  passed('agent mutation paused on gated tab');
  uiHandle = (await request('tab_new', { params: { url: `${origin}/tasks`, headless: true, waitUntil: 'load' } })).data.serviceTabHandle;
  const auth = await httpJson(port, 'GET', '/api/dashboard-auth/status');
  assert(auth.authenticated === false, `Disposable auth bootstrap failed: ${JSON.stringify(auth)}`);
  await until(async () => evaluate(uiHandle, '!!document.querySelector("input[autocomplete=username]:not(:disabled)")'), 'disposable dashboard login');
  const credentials = Object.fromEntries(readFileSync(join(context.agentHome, 'dashboard-auth.env'), 'utf8')
    .split(/\r?\n/).map((line) => line.match(/^([A-Z0-9_]+)="(.*)"$/)).filter(Boolean).map((match) => [match[1], match[2]]));
  await request('fill', { serviceTabHandle: uiHandle, params: { selector: 'input[autocomplete="username"]', value: credentials.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME } });
  await request('fill', { serviceTabHandle: uiHandle, params: { selector: 'input[autocomplete="current-password"]', value: credentials.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD } });
  await request('click', { serviceTabHandle: uiHandle, params: { selector: 'button[type="submit"]' } });
  await until(async () => (await evaluate(uiHandle, 'document.body.innerText')).includes('Needs you (1)'), 'dashboard human handoff');
  passed('disposable dashboard login through real authentication endpoint');
  passed('rendered dashboard shows Needs you and exact tab');
  assert((await evaluate(uiHandle, 'document.querySelector("aside[aria-label=\\"Selected browser work\\"]").innerText')).includes(taskHandle.tabId), 'Dashboard selected a different tab');
  const recheckSelector = 'aside[aria-label="Selected browser work"] button:last-of-type';
  assert(await evaluate(uiHandle, `document.querySelector(${JSON.stringify(recheckSelector)}).textContent`) === 'Done, re-check page', 'Expected rendered recheck button');
  await request('click', { serviceTabHandle: uiHandle, params: { selector: recheckSelector } });
  await until(async () => (await evaluate(uiHandle, 'document.body.innerText')).includes('The page still needs a human action.'), 'still gated dashboard notice');
  passed('dashboard recheck preserves an unresolved handoff');
  stage = 'unknown';
  await until(async () => (await probe(taskHandle)).data.pageGate.classification === 'unknown', 'unknown page');
  const waiting = await httpJson(port, 'GET', '/api/service/runs');
  assert(waiting.data.runs.some((r) => r.kind === 'human_challenge' && r.targetId === taskHandle.targetId), 'Unknown cleared handoff');
  passed('inconclusive page evidence cannot clear handoff');
  stage = 'ready';
  // Server state is a simulated operator action, not a browser CAPTCHA bypass.
  await new Promise((done) => setTimeout(done, 300));
  await request('click', { serviceTabHandle: uiHandle, params: { selector: recheckSelector } });
  await until(async () => (await evaluate(uiHandle, 'document.body.innerText')).includes('A ready page was observed.'), 'ready-page dashboard notice');
  const clear = await httpJson(port, 'GET', '/api/service/runs');
  assert(!clear.data.runs.some((r) => r.kind === 'human_challenge'), 'Ready recheck did not resolve handoff');
  assert(clear.data.durableTaskHistory === false && clear.data.runs.every((r) => !r.taskOutcomeVerified), 'Workbench falsely promoted command success');
  passed('dashboard button resolves only positive ready evidence');
  await request('click', { serviceTabHandle: taskHandle, params: { selector: '#finish' } });
  assert(await evaluate(taskHandle, 'document.querySelector("#finish").textContent') === 'Completed', 'Task did not resume');
  passed('same exact target resumes and resulting DOM is verified');
  for (const [width, height] of [[1280, 900], [390, 844]]) {
    await request('viewport', { serviceTabHandle: uiHandle, params: { width, height } });
    const layout = await evaluate(uiHandle, '({width:innerWidth, scroll:document.documentElement.scrollWidth, workbench:!!document.querySelector("[aria-label=\\"Task workbench\\"]")})');
    assert(layout.workbench && layout.scroll <= layout.width, `Dashboard overflow: ${JSON.stringify(layout)}`);
    if (evidenceDir) {
      await request('screenshot', { serviceTabHandle: uiHandle, params: { path: join(evidenceDir, `workbench-${width}.png`) } });
    }
    passed(`dashboard layout ${width}x${height}`);
  }
  const released = await request('tab_handle_release', { serviceTabHandle: taskHandle, params: { requirePhysicalClose: true } });
  assert(released.success, 'Task tab release failed');
  const tabs = await request('tab_list', { params: { verbose: true } });
  assert(!tabs.data.tabs.some((t) => t.targetId === taskHandle.targetId), 'Released target still exists');
  assert(tabs.data.tabs.some((t) => t.targetId === uiHandle.targetId), 'Cleanup removed the other tab');
  passed('physical close removes only the task tab');
  const reopened = await request('tab_reopen', { params: { tabId: taskHandle.tabId } });
  const newHandle = reopened.data.serviceTabHandle;
  assert(newHandle?.valid && newHandle.targetId !== taskHandle.targetId && newHandle.profileId === taskHandle.profileId, 'Reopen did not preserve profile and change target');
  assert((await request('url', { serviceTabHandle: newHandle })).data.url === `${origin}/fixture`, 'Reopen restored the wrong URL');
  passed('reopen restores URL in same profile with a new target');
  const oldDaemon = daemonIdentity;
  await verifiedClose();
  port = await ensureStreamPort(context);
  daemonIdentity = currentDaemonIdentity();
  assert(daemonIdentity.identity !== oldDaemon.identity, 'Persistence check reused the original daemon');
  const restarted = await httpJson(port, 'GET', '/api/service/runs');
  assert(restarted.success && restarted.data.runs.length > 0, 'Recent work did not survive daemon restart');
  state = JSON.parse(readFileSync(join(context.agentHome, 'service/state.json'), 'utf8'));
  assert(state.challenges[`page-gate:${taskHandle.tabId}`]?.state === 'resolved', 'Resolved gate did not persist');
  assert(restarted.data.runs.every((r) => !r.taskOutcomeVerified), 'Restart invented verified task outcome');
  passed('resolved gate and recent work persist across daemon restart');
  await cleanup();
  passed('fixture browser, daemon, and disposable home cleaned up');
  console.log(JSON.stringify({ success: true, ...report }));
} catch (error) {
  console.error(error.stack || error.message);
  await cleanup();
  process.exitCode = 1;
} finally {
  clearTimeout(timeout);
}
