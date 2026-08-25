#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  discoverVerifyAndPinRetainedBrowser,
} from './lib/local-dashboard-retained-browser-live.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-retained-live-'));
const requirementPath = join(root, 'publications', 'retained.json');
const exactUrl = 'https://chatgpt.com/g/g-p-workshop/c/conversation-id';
const browser = {
  id: 'session:workshop',
  pid: process.pid,
  profileId: 'chatgpt-pro',
  health: 'ready',
  cdpEndpoint: 'ws://127.0.0.1:9444/devtools/browser/workshop',
};
const target = { id: 'target-workshop', type: 'page', url: exactUrl };
const opened = {
  browserId: browser.id,
  sessionName: 'workshop',
  profileId: browser.profileId,
  targetId: target.id,
  url: exactUrl,
};
const adapters = {
  sessionNames: ['workshop'],
  readDaemonPid: () => process.pid,
  isProcessLive: (pid) => pid === process.pid,
  readBrowser: async () => ({ success: true, browser }),
  readCdpTargets: async () => [target],
};

try {
  const result = await discoverVerifyAndPinRetainedBrowser({
    agentBrowserBin: '/fixture/agent-browser',
    exactUrl,
    profileId: 'chatgpt-pro',
    requirementPath,
    expectedOpenedIdentity: opened,
    adapters,
  });
  assert.equal(result.discovery.matchedCandidateCount, 1);
  assert.equal(result.requirement.exists, true);
  assert.equal(statSync(requirementPath).mode & 0o777, 0o600);
  assert.equal(statSync(`${requirementPath}.required`).mode & 0o777, 0o600);
  const stored = JSON.parse(readFileSync(requirementPath, 'utf8'));
  assert.deepEqual(stored.expectation, {
    sessionName: 'workshop',
    profileId: 'chatgpt-pro',
    targetId: 'target-workshop',
    url: exactUrl,
  });

  await assert.rejects(
    discoverVerifyAndPinRetainedBrowser({
      agentBrowserBin: '/fixture/agent-browser',
      exactUrl,
      profileId: 'chatgpt-pro',
      requirementPath: join(root, 'wrong', 'retained.json'),
      expectedOpenedIdentity: { ...opened, targetId: 'wrong-target' },
      adapters,
    }),
    /retained_browser_preparation_open_discovery_mismatch/,
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard retained browser live pin fixture passed');
