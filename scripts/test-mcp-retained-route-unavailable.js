#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { createSmokeContext, createMcpStdioClient } from './smoke-utils.js';

// Real stdio transport, isolated runtime: an existing-route hint is not launch authority.
const context = createSmokeContext({
  prefix: 'ab-mcp-cold-route-',
  sessionPrefix: 'cold-route',
});
const mcp = createMcpStdioClient({
  context,
  args: ['--session', context.session, 'mcp', 'serve'],
});

try {
  await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'cold-route-regression', version: '1' },
  });
  mcp.notify('notifications/initialized');
  for (const route of [
    { sessionName: 'absent-retained-session' },
    { browserId: 'session:absent-retained-session' },
    { serviceTabHandle: { sessionName: 'absent-retained-session' } },
    { serviceTabHandle: { browserId: 'session:absent-retained-session' } },
    { serviceTabHandle: { sessionName: 'absent-retained-session', browserId: 'session:absent-retained-session' } },
  ]) {
    await assert.rejects(
      mcp.send('tools/call', {
        name: 'service_request',
        arguments: {
          serviceName: 'ColdRouteTest', agentName: 'fixture', taskName: 'no-launch',
          action: 'url', ...route,
        },
      }),
      (error) => {
        const marker = 'tools/call failed: ';
        assert.ok(error.message.startsWith(marker));
        const response = JSON.parse(error.message.slice(marker.length));
        assert.equal(response.code, -32603);
        assert.equal(response.data.diagnosticCode, 'retained_daemon_unavailable');
        assert.equal(response.data.session, 'absent-retained-session');
        assert.equal(response.data.autoLaunchAttempted, false);
        return true;
      },
    );
  }
  for (const serviceTabHandle of [
    {}, { browserId: 'opaque-browser' },
    { sessionName: 'one', browserId: 'session:two' },
  ]) {
    await assert.rejects(mcp.send('tools/call', {
      name: 'service_request',
      arguments: { action: 'url', serviceTabHandle,
        serviceName: 'ColdRouteTest', agentName: 'fixture', taskName: 'reject-invalid-route' },
    }), error => {
      const response = JSON.parse(error.message.slice('tools/call failed: '.length));
      assert.equal(response.code, -32602);
      return true;
    });
  }
  assert.deepEqual(readdirSync(context.socketDir), [], 'must not create daemon socket or PID files');
  const statePath = join(context.agentHome, 'service', 'state.json');
  if (existsSync(statePath)) {
    const state = JSON.parse(readFileSync(statePath, 'utf8'));
    for (const collection of ['jobs', 'browsers', 'sessions']) {
      assert.deepEqual(Object.keys(state[collection] ?? {}), [], `unexpected ${collection}`);
    }
  }
  console.log('MCP absent retained routes: actionable failure, no daemon, browser, or job created.');
} finally {
  await mcp.close();
  context.cleanupTempHome();
}
