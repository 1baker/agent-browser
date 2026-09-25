#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createSmokeContext, createMcpStdioClient } from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-cold-admission-', session: 'default' });
delete context.env.AGENT_BROWSER_SESSION;
delete context.env.AGENT_BROWSER_CONFIG;
delete context.env.AGENT_BROWSER_RUNTIME_PROFILE;
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'] });
const labels = { serviceName: 'ColdAdmissionQA', agentName: 'fixture', taskName: 'deny-effects' };
const request = args => client.send('tools/call', { name: 'service_request',
  arguments: { ...labels, runtimeProfile: 'owned-fixture', ...args } });
const denied = async args => {
  let response;
  try { response = await request(args); }
  catch (error) {
    assert.ok(error.message.startsWith('tools/call failed: '), error.message);
    return;
  }
  const payload = JSON.parse(response.content[0].text);
  assert.ok(response.isError === true || payload.success === false, JSON.stringify(payload));
};
let complete = false;
try {
  await client.send('initialize', { protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'cold-admission-fixture', version: '1' } });
  client.notify('notifications/initialized');
  const created = await client.send('tools/call', { name: 'service_profile_upsert', arguments: {
    ...labels, id: 'owned-fixture', profile: { id: 'owned-fixture',
      name: 'Owned fixture', userDataDir: join(context.tempHome, 'profile') },
  } });
  assert.equal(JSON.parse(created.content[0].text).success, true);
  const statePath = join(context.agentHome, 'service', 'state.json');
  const original = readFileSync(statePath, 'utf8');
  const navigate = { action: 'navigate', params: { url: 'data:text/html,forbidden', browserHost: 'local_headless' } };
  for (const args of [
    { action: 'title' },
    { action: 'click', params: { selector: 'button' } },
    { ...navigate, sessionName: 'missing-retained' },
    { ...navigate, browserId: 'session:missing-retained' },
    { ...navigate, serviceTabHandle: { sessionName: 'missing-retained' } },
  ]) {
    await denied(args);
    assert.deepEqual(readdirSync(context.socketDir), [], 'denied request started a daemon');
    assert.equal(readFileSync(statePath, 'utf8'), original, 'denied request mutated service state');
  }
  const leased = JSON.parse(original);
  leased.sessions['competing-owner'] = { id: 'competing-owner', profileId: 'owned-fixture',
    lease: 'exclusive', browserIds: [], activeTabIds: [] };
  writeFileSync(statePath, JSON.stringify(leased));
  const leasedBefore = readFileSync(statePath, 'utf8');
  await denied(navigate);
  assert.deepEqual(readdirSync(context.socketDir), [], 'lease conflict started a daemon');
  assert.equal(readFileSync(statePath, 'utf8'), leasedBefore);
  writeFileSync(statePath, original);
  const marker = join(context.socketDir, 'default.pid');
  writeFileSync(marker, '999999999');
  await denied(navigate);
  assert.equal(readFileSync(marker, 'utf8'), '999999999');
  assert.deepEqual(readdirSync(context.socketDir), ['default.pid']);
  assert.equal(readFileSync(statePath, 'utf8'), original);
  complete = true;
  console.log('Cold MCP admission: read/input, retained routes, profile lease and stale metadata denied without effects.');
} finally {
  await client.close();
  if (complete) context.cleanupTempHome();
  else console.error(`Preserved failed isolated fixture: ${context.tempHome}`);
}
