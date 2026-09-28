#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

import { startBlockingSafeFixtureServer } from './lib/blocking-safe-fixture-server.js';

const profileId = process.env.AGENT_BROWSER_RETAINED_PREPARATION_FIXTURE_PROFILE
  || 'remote-view-open-live-413134-profile';
const agentBrowserBin = resolve(
  process.env.AGENT_BROWSER_RETAINED_PREPARATION_FIXTURE_BIN
    || 'cli/target/debug/agent-browser',
);
const root = mkdtempSync(join(tmpdir(), 'agent-browser-retained-preparation-live-'));
const requirementPath = join(root, 'publications', 'retained.json');
const marker = `RETAINED PREPARATION LIVE ${process.pid}`;
const fixture = await startBlockingSafeFixtureServer({
  html: `<title>${marker}</title><h1>${marker}</h1>`,
});
let openedSessionName = null;

try {
  assert.equal(existsSync(agentBrowserBin), true, `candidate binary is missing: ${agentBrowserBin}`);
  const before = runtimeStatus();
  assert.notEqual(
    before?.browserAlive,
    true,
    `fixture profile is already live and cannot be used safely: ${profileId}`,
  );
  const exactUrl = `http://${fixture.host}:${fixture.port}/retained-preparation/${process.pid}`;
  const urlPrefix = `http://${fixture.host}:${fixture.port}/retained-preparation/`;
  const result = spawnSync(process.execPath, [
    resolve('scripts/prepare-local-dashboard-retained-browser.js'),
    '--url', exactUrl,
    '--url-prefix', urlPrefix,
    '--runtime-profile', profileId,
    '--retained-requirement', requirementPath,
    '--agent-browser-bin', agentBrowserBin,
    '--json',
  ], {
    cwd: resolve('.'),
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    timeout: 180000,
    env: process.env,
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.success, true);
  assert.equal(payload.state, 'ready_and_pinned');
  assert.equal(payload.opened.url, exactUrl);
  assert.equal(payload.opened.profileId, profileId);
  assert.equal(payload.discovery.exactUrl, exactUrl);
  assert.equal(payload.discovery.profileId, profileId);
  assert.equal(payload.discovery.matchedCandidateCount, 1);
  assert.equal(payload.requirement.exists, true);
  openedSessionName = payload.opened.sessionName;
  const requirement = JSON.parse(readFileSync(requirementPath, 'utf8'));
  assert.deepEqual(requirement.expectation, {
    sessionName: openedSessionName,
    profileId,
    targetId: payload.opened.targetId,
    url: exactUrl,
  });
  assert.equal(existsSync(`${requirementPath}.required`), true);
  console.log(JSON.stringify({
    success: true,
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-preparation-live.v1',
    state: 'fixture_ready_and_pinned',
    profileId,
    sessionName: openedSessionName,
    targetId: payload.opened.targetId,
    exactUrl,
  }));
} finally {
  const status = runtimeStatus();
  if (status?.browserAlive === true) {
    const sessionName = openedSessionName || 'default';
    const closed = spawnSync(agentBrowserBin, [
      '--json',
      '--session', sessionName,
      'close',
    ], {
      cwd: resolve('.'),
      encoding: 'utf8',
      timeout: 30000,
    });
    if (closed.status !== 0) {
      console.error(`Fixture browser cleanup failed for verified profile ${profileId}`);
    }
  }
  await fixture.close();
  rmSync(root, { recursive: true, force: true });
}

function runtimeStatus() {
  const result = spawnSync(agentBrowserBin, [
    '--json',
    '--runtime-profile', profileId,
    'runtime',
    'status',
  ], {
    cwd: resolve('.'),
    encoding: 'utf8',
    timeout: 30000,
  });
  if (result.status !== 0) return null;
  try {
    return JSON.parse(result.stdout);
  } catch {
    return null;
  }
}
