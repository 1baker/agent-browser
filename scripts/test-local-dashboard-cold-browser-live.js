// Opt-in real legacy -> candidate -> candidate custody migration fixture.
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
async function rawTool(name, args) {
  if (fatalError) throw fatalError;
  const result = await bounded(mcp.send('tools/call', { name, arguments: { ...labels, ...args } }));
  return { result, value: JSON.parse(result.content.find((entry) => entry.type === 'text').text) };
}
async function tool(name, args) {
  const { result, value } = await rawTool(name, args);
  assert.notEqual(result.isError, true, JSON.stringify(result));
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
async function cli(args) {
  const result = await runCli(context, ['--json', '--session', context.session, ...args], 30000);
  const value = JSON.parse(result.stdout);
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
async function prepare(expectedSchema) {
  const receipt = await cli(['handoff', 'prepare']);
  assert.equal(receipt.prepared, true);
  const descriptor = JSON.parse(fs.readFileSync(receipt.handoffPath, 'utf8'));
  assert.equal(descriptor.schemaVersion, expectedSchema,
    'handoff schema must reflect the source daemon generation and verified custody');
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
  const handle = { ...acquired.serviceTabHandle, leaseState: 'shared' };
  assert.equal(handle?.valid, true, JSON.stringify(acquired));
  const diagnostics = () => tool('service_request', { action: 'diagnostics', sessionName: context.session,
    browserId: `session:${context.session}`, serviceTabHandle: handle, includeScreenshot: false });
  const before = await diagnostics();
  browserPid = before.browser?.pid;
  assert.ok(Number.isInteger(browserPid));
  await prepare(1);
  await startMcp(candidate);
  await cli(['handoff', 'resume']);
  const middle = await diagnostics();
  assert.equal(middle.browser?.pid, browserPid);
  assert.notEqual(middle.controlPlaneAttestation?.complete, true, 'v1 must not fabricate complete custody');
  const second = await tool('service_request', {
    action: 'tab_new', sessionName: context.session, browserId: `session:${context.session}`,
    runtimeProfile: context.session, profile, browserBuild: 'stock_chrome',
    url: 'data:text/html,<title>Second preserved tab</title>', params: { headless: true },
  });
  assert.notEqual(second.serviceTabHandle?.targetId, handle.targetId);
  await prepare(4);
  await startMcp(candidate);
  await cli(['handoff', 'resume']);
  const after = await diagnostics();
  assert.equal(after.browser?.pid, browserPid);
  assert.equal(after.targetId, before.targetId);
  assert.equal(after.controlPlaneAttestation?.complete, true,
    JSON.stringify(after.controlPlaneAttestation));
  // Reject metadata mutations even when valid=true and genuine custody exists.
  // Each request points at the second tab, so unchanged active URL also proves
  // rejection happened before diagnostics selected or read the foreign handle.
  const negativeHandles = [];
  for (const leaseState of ['released', 'expired', 'foreign', 'human_takeover']) {
    negativeHandles.push({ ...second.serviceTabHandle, leaseState });
  }
  for (const key of ['leaseState', 'leaseId', 'ownerSessionId', 'sessionName']) {
    for (const value of ['foreign', null, 17]) {
      negativeHandles.push({ ...second.serviceTabHandle, [key]: value });
    }
    const missing = { ...second.serviceTabHandle };
    delete missing[key];
    negativeHandles.push(missing);
  }
  const activeUrl = await cli(['get', 'url']);
  for (const invalidHandle of negativeHandles) {
    let response;
    try {
      response = await rawTool('service_request', {
        action: 'diagnostics', sessionName: context.session,
        browserId: `session:${context.session}`, serviceTabHandle: invalidHandle, includeScreenshot: false,
      });
    } catch (error) {
      // A foreign session can be refused even earlier by the MCP route parser.
      // Do not accept unrelated transport errors as a passing negative check.
      assert.match(error.message, /tools\/call failed: .*"code":-32602.*serviceTabHandle sessionName and browserId routes conflict/);
      assert.equal(invalidHandle.sessionName, 'foreign');
    }
    if (response) {
      assert.equal(response.value.success, false, JSON.stringify(response.value));
      assert.notEqual(response.value.data?.controlPlaneAttestation?.complete, true);
    }
    assert.deepEqual(await cli(['get', 'url']), activeUrl, 'invalid handle must not select another tab');
  }
  const rechecked = await diagnostics();
  assert.equal(rechecked.controlPlaneAttestation?.complete, true);
  assert.equal(rechecked.browser?.pid, browserPid);
  const exclusive = await tool('service_request', { action: 'diagnostics', sessionName: context.session,
    browserId: `session:${context.session}`, serviceTabHandle: { ...handle, leaseState: 'exclusive' },
    includeScreenshot: false });
  assert.equal(exclusive.controlPlaneAttestation?.complete, true);
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
console.log('Isolated real-browser v1 to v4 custody migration and 20 negative handle checks passed');
