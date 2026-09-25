#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createSmokeContext, createMcpStdioClient } from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-cold-profile-', sessionPrefix: 'profile' });
const mcp = createMcpStdioClient({ context, args: ['--session', context.session, 'mcp', 'serve'] });
const call = (id, profile) => mcp.send('tools/call', {
  name: 'service_profile_upsert',
  arguments: { id, profile, serviceName: 'ColdProfileQA', agentName: 'fixture', taskName: 'profile-only' },
});
try {
  await mcp.send('initialize', { protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'cold-profile-test', version: '1' } });
  mcp.notify('notifications/initialized');
  const result = await call('cold-profile', { id: 'cold-profile', name: 'Cold QA' });
  const payload = JSON.parse(result.content[0].text);
  assert.equal(payload.success, true, JSON.stringify(payload));
  assert.equal(payload.data.upserted, true);
  assert.equal(payload.data.profile.id, 'cold-profile');
  assert.deepEqual(readdirSync(context.socketDir), [], 'no daemon artifacts');
  const statePath = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  assert.ok(state.profiles['cold-profile']);
  assert.deepEqual(Object.keys(state.browsers ?? {}), []);
  const jobs = Object.values(state.jobs ?? {});
  assert.equal(jobs.filter(job => job.action === 'service_profile_upsert').length, 1);
  assert.ok(jobs.some(job => job.state === 'succeeded'));

  const denied = await call('requested-id', { id: 'different-id', name: 'Invalid identity' });
  assert.equal(denied.isError, true, 'normal profile identity validation must still reject');
  const beforeStale = JSON.parse(readFileSync(statePath, 'utf8'));
  assert.equal(beforeStale.profiles['requested-id'], undefined);
  assert.equal(beforeStale.profiles['different-id'], undefined);
  assert.deepEqual(readdirSync(context.socketDir), []);

  // Existing metadata is deliberately not repaired or bypassed.
  const marker = join(context.socketDir, `${context.session}.pid`);
  writeFileSync(marker, '999999999');
  await assert.rejects(call('must-not-exist', { id: 'must-not-exist', name: 'Blocked' }));
  assert.equal(readFileSync(marker, 'utf8'), '999999999');
  const after = JSON.parse(readFileSync(statePath, 'utf8'));
  assert.equal(after.profiles['must-not-exist'], undefined);
  assert.equal(Object.keys(after.jobs).length, Object.keys(beforeStale.jobs).length);
  console.log('Cold MCP profile creation passed: one queued operation, no daemon/browser; stale metadata refused.');
} finally {
  await mcp.close();
  context.cleanupTempHome();
}
