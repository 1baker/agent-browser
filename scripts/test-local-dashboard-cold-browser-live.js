// Opt-in real old -> candidate -> old handoff compatibility fixture.
// Uses the existing isolated smoke environment and a synthetic page only.
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import { dirname, join, isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { createSmokeContext, createMcpStdioClient, runCli } from './smoke-utils.js';

const oldBinary = process.env.AGENT_BROWSER_COLD_TEST_OLD_BIN;
const candidate = process.env.AGENT_BROWSER_COLD_TEST_CANDIDATE_BIN;
const chrome = process.env.AGENT_BROWSER_COLD_TEST_CHROME;
for (const path of [oldBinary, candidate, chrome]) {
  assert.ok(path && isAbsolute(path) && fs.statSync(path).isFile(), 'explicit installed old/candidate/Chrome paths required');
}
const hash = (path) => createHash('sha256').update(fs.readFileSync(path)).digest('hex');
const originalHashes = [hash(oldBinary), hash(candidate)];
const context = createSmokeContext({ prefix: 'cold-browser-live-', sessionPrefix: 'cold-v1' });
const profile = join(context.tempHome, 'profile');
fs.mkdirSync(profile, { mode: 0o700 });
fs.mkdirSync(join(context.agentHome, 'browsers'), { recursive: true });
fs.symlinkSync(dirname(chrome), join(context.agentHome, 'browsers', 'chrome-fixture'));
for (const key of ['AGENT_BROWSER_CDP', 'AGENT_BROWSER_AUTO_CONNECT', 'AGENT_BROWSER_PROVIDER',
  'AGENT_BROWSER_HEADED', 'AGENT_BROWSER_CONFIG', 'AGENT_BROWSER_SESSION']) delete context.env[key];
context.env.AGENT_BROWSER_CDP_TRANSPORT = 'websocket';
context.env.AGENT_BROWSER_PROFILE = profile;
context.env.AGENT_BROWSER_EXECUTABLE_PATH = chrome;
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = oldBinary;
const labels = { serviceName: 'ColdFixture', agentName: 'fixture', taskName: 'legacy-roundtrip' };
const pageUrl = 'data:text/html,<title>Cold rollback fixture</title><input id="probe">';
let mcp;
let browserPid;
let passed = false;
let fatalError;
let failure;
let cleanupFailure;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function bounded(promise) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error('Fixture MCP deadline exceeded')), 30000);
  })]); } finally { clearTimeout(timer); }
}
async function startMcp(binary) {
  if (mcp) mcp.close();
  context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = binary;
  mcp = createMcpStdioClient({ context, args: ['--session', context.session, 'mcp', 'serve'],
    onFatal: (message) => { fatalError = new Error(message); } });
  await bounded(mcp.send('initialize', { protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'cold-browser-fixture', version: '1' } }));
  mcp.notify('notifications/initialized');
}
async function tool(name, args) {
  if (fatalError) throw fatalError;
  const result = await bounded(mcp.send('tools/call', { name, arguments: { ...labels, ...args } }));
  assert.notEqual(result.isError, true, JSON.stringify(result));
  const value = JSON.parse(result.content.find((entry) => entry.type === 'text').text);
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
async function cli(args) {
  const result = await runCli(context, ['--json', '--session', context.session, ...args], 30000);
  const value = JSON.parse(result.stdout);
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
async function prepare() {
  const receipt = await cli(['handoff', 'prepare']);
  assert.equal(receipt.prepared, true);
  const descriptor = JSON.parse(fs.readFileSync(receipt.handoffPath, 'utf8'));
  assert.equal(descriptor.schemaVersion, 1, 'legacy-owned lane must not claim upgraded custody');
  const pidPath = join(context.socketDir, `${context.session}.pid`);
  for (let attempt = 0; attempt < 100 && fs.existsSync(pidPath); attempt += 1) await sleep(50);
  assert.equal(fs.existsSync(pidPath), false, 'source daemon must exit');
  return descriptor;
}
try {
  await cli(['--runtime-profile', context.session, '--profile', profile,
    '--executable-path', chrome, '--browser-build', 'stock_chrome', 'open', 'about:blank']);
  await startMcp(oldBinary);
  await tool('service_access_plan', { runtimeProfile: context.session, url: pageUrl });
  const acquired = await tool('service_request', {
    action: 'tab_new', sessionName: context.session, browserId: `session:${context.session}`,
    runtimeProfile: context.session, profile, browserBuild: 'stock_chrome', url: pageUrl,
    params: { headless: true },
  });
  const handle = acquired.serviceTabHandle;
  assert.equal(handle?.valid, true, JSON.stringify(acquired));
  const diagnostics = () => tool('service_request', { action: 'diagnostics', sessionName: context.session,
    browserId: `session:${context.session}`, serviceTabHandle: handle, includeScreenshot: false });
  const before = await diagnostics();
  browserPid = before.browser?.pid;
  assert.ok(Number.isInteger(browserPid));
  await prepare();
  await startMcp(candidate);
  await cli(['handoff', 'resume']);
  const middle = await diagnostics();
  assert.equal(middle.browser?.pid, browserPid);
  assert.notEqual(middle.controlPlaneAttestation?.complete, true, 'v1 must not fabricate complete custody');
  await prepare();
  await startMcp(oldBinary);
  await cli(['handoff', 'resume']);
  const after = await diagnostics();
  assert.equal(after.browser?.pid, browserPid);
  assert.equal(after.targetId, before.targetId);
  assert.deepEqual([hash(oldBinary), hash(candidate)], originalHashes);
  passed = true;
} catch (error) {
  failure = error;
} finally {
  if (mcp) mcp.close();
  // Close only this throwaway test session. Preserve all evidence if its exit
  // cannot be proved; never clean up a still-running profile.
  try {
    await cli(['close']);
    if (browserPid) {
      for (let attempt = 0; attempt < 100 && fs.existsSync(`/proc/${browserPid}`); attempt += 1) await sleep(50);
      assert.equal(fs.existsSync(`/proc/${browserPid}`), false, 'fixture browser still live');
    }
    context.cleanupTempHome();
  } catch (error) {
    console.error(`Fixture cleanup unverified; preserved ${context.tempHome}`);
    cleanupFailure = error;
  }
}
if (cleanupFailure) throw new AggregateError([failure, cleanupFailure].filter(Boolean), 'Fixture cleanup failed');
if (failure) throw failure;
assert.equal(passed, true);
console.log('Isolated real-browser v1 old/candidate/old rollback passed');
