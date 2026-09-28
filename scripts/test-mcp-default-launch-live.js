#!/usr/bin/env node
import assert from 'node:assert/strict';
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createSmokeContext, createMcpStdioClient, runCli } from './smoke-utils.js';

// Fresh HOME/socket/profile, no explicit session: exercise real MCP defaults.
const context = createSmokeContext({ prefix: 'ab-mcp-default-', session: 'default' });
const chrome = process.env.AGENT_BROWSER_SMOKE_CHROME_PATH;
assert.ok(chrome && existsSync(chrome), 'Explicit disposable Linux Chrome executable required');
const chromeCache = join(context.agentHome, 'browsers', 'chrome-153.0.8010.36');
mkdirSync(chromeCache, { recursive: true });
symlinkSync(resolve(chrome), join(chromeCache, 'chrome'));
const config = join(context.tempHome, 'config.json');
const profile = join(context.tempHome, 'profile');
const secondProfile = join(context.tempHome, 'second-profile');
const useProfileAlias = process.env.AGENT_BROWSER_SMOKE_PROFILE_ALIAS === '1';
writeFileSync(config, JSON.stringify({ executablePath: chrome, profile,
  ...(useProfileAlias ? { runtimeProfile: 'configured-unused' } : {}),
  service: { defaultBrowserBuild: 'stock_chrome' } }));
Object.assign(context.env, { AGENT_BROWSER_CONFIG: config,
  AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'disabled',
  AGENT_BROWSER_BROWSER_HOST: 'local_headless', AGENT_BROWSER_HEADED: '0' });
delete context.env.AGENT_BROWSER_SESSION;
delete context.env.AGENT_BROWSER_RUNTIME_PROFILE;
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'] });
const labels = { serviceName: 'DefaultDaemonQA', agentName: 'fixture', taskName: 'cold-launch' };
const acquisitionAction = process.env.AGENT_BROWSER_SMOKE_COLD_ACTION || 'navigate';
assert.ok(['navigate', 'tab_new'].includes(acquisitionAction));
const call = async (name, args) => {
  const response = await client.send('tools/call', { name, arguments: { ...labels, ...args } });
  const payload = JSON.parse(response.content[0].text);
  assert.equal(payload.success, true, JSON.stringify(payload));
  return payload.data;
};
let complete = false;
try {
  await client.send('initialize', { protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'default-daemon-fixture', version: '1' } });
  client.notify('notifications/initialized');
  await call('service_profile_upsert', { id: 'fresh-profile', profile: {
    id: 'fresh-profile', name: 'Disposable profile', userDataDir: profile,
  } });
  assert.deepEqual(readdirSync(context.socketDir), [], 'profile creation must not start a daemon');
  const url = 'data:text/html,<title>Default MCP QA</title><p>Fresh profile ready</p>';
  await call('service_access_plan', { url, runtimeProfile: 'fresh-profile', browserHost: 'local_headless' });
  await call('service_request', { action: acquisitionAction,
    ...(useProfileAlias ? { profileId: 'fresh-profile' } : { runtimeProfile: 'fresh-profile' }), params: {
    url, browserHost: 'local_headless', headless: true,
  } });
  const state = JSON.parse(readFileSync(join(context.agentHome, 'service', 'state.json'), 'utf8'));
  assert.ok(state.browsers['session:default']?.pid, 'default daemon must own its browser');
  assert.equal(state.browsers['session:default'].profileId, 'fresh-profile');
  assert.equal(Object.keys(state.browsers).length, 1, 'only one disposable browser');
  assert.deepEqual(readdirSync(context.socketDir).filter(name => name.endsWith('.pid')), ['default.pid']);
  const result = await call('service_request', { action: 'title' });
  assert.equal(result.title, 'Default MCP QA');
  const repeated = await call('service_request', { action: 'title', sessionName: 'default' });
  assert.equal(repeated.title, 'Default MCP QA');
  const afterRead = JSON.parse(readFileSync(join(context.agentHome, 'service', 'state.json'), 'utf8'));
  assert.equal(afterRead.browsers['session:default'].pid, state.browsers['session:default'].pid);
  await call('service_profile_upsert', { id: 'second-profile', profile: {
    id: 'second-profile', name: 'Second disposable profile', userDataDir: secondProfile,
  } });
  const secondUrl = 'data:text/html,<title>Second MCP QA</title><p>Isolated profile ready</p>';
  const secondPlan = await call('service_access_plan', {
    url: secondUrl, runtimeProfile: 'second-profile', browserHost: 'local_headless',
  });
  assert.equal(secondPlan.decision.profileReuse.recommendedAction, 'launch_new_browser');
  const second = await call('service_request', { action: 'tab_new', runtimeProfile: 'second-profile',
    params: { url: secondUrl, browserHost: 'local_headless', headless: true },
  });
  const secondSession = second.serviceTabHandle?.sessionName;
  assert.match(secondSession, /^mcp-cold-[0-9a-f]{32}$/);
  const withSecond = JSON.parse(readFileSync(join(context.agentHome, 'service', 'state.json'), 'utf8'));
  assert.equal(withSecond.browsers['session:default'].pid, state.browsers['session:default'].pid);
  assert.equal(withSecond.browsers[`session:${secondSession}`].profileId, 'second-profile');
  assert.equal(Object.keys(withSecond.browsers).length, 2, 'exactly two profile-isolated browsers');
  const secondTitle = await call('service_request', { action: 'title',
    serviceTabHandle: second.serviceTabHandle });
  assert.equal(secondTitle.title, 'Second MCP QA');
  await runCli(context, ['--session', secondSession, '--json', 'close']);
  await runCli(context, ['--json', 'close']);
  assert.throws(() => lstatSync(join(profile, 'SingletonLock')), { code: 'ENOENT' });
  assert.throws(() => lstatSync(join(secondProfile, 'SingletonLock')), { code: 'ENOENT' });
  complete = true;
  console.log(`Fresh MCP default daemon (${acquisitionAction}) and occupied-default isolated cold launch passed.`);
} finally {
  await client.close();
  if (complete) context.cleanupTempHome();
  else console.error(`Preserved failed isolated fixture: ${context.tempHome}`);
}
