#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  discoverVerifyAndPinRetainedBrowser,
} from './lib/local-dashboard-retained-browser-live.js';
import {
  rotateRetainedBrowserRequirement,
  writeRetainedBrowserRequirement,
} from './lib/local-dashboard-retained-browser-requirement.js';

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

  const oldExpectation = {
    sessionName: 'old-workshop',
    profileId: 'chatgpt-pro',
    targetId: 'old-target',
    url: 'https://chatgpt.com/g/g-p-workshop/c/old-conversation',
  };
  const oldEvidence = {
    ...evidenceFor(oldExpectation),
    observed: {
      ...oldExpectation,
      browserId: 'session:old-workshop',
      browserPid: process.pid,
      cdpUrl: 'ws://127.0.0.1:9444/devtools/browser/old-workshop',
      health: 'ready',
    },
  };
  const staleRotationPath = join(root, 'rotation', 'retained.json');
  const oldRequirement = writeRetainedBrowserRequirement({
    path: staleRotationPath,
    evidence: oldEvidence,
    now: () => '2026-08-15T12:00:00.000Z',
  });
  const deadOldAdapters = {
    ...adapters,
    readDaemonPid: (sessionName) => sessionName === 'old-workshop' ? null : process.pid,
  };
  const rotated = await discoverVerifyAndPinRetainedBrowser({
    agentBrowserBin: '/fixture/agent-browser',
    exactUrl,
    profileId: 'chatgpt-pro',
    requirementPath: staleRotationPath,
    expectedOpenedIdentity: opened,
    rotateExpectedSha256: oldRequirement.sha256,
    adapters: deadOldAdapters,
  });
  assert.equal(rotated.requirement.rotated, true);
  assert.equal(rotated.requirement.previousSha256, oldRequirement.sha256);

  const recoveryPath = join(root, 'rotation-recovery', 'retained.json');
  const recoveryOld = writeRetainedBrowserRequirement({
    path: recoveryPath,
    evidence: oldEvidence,
    now: () => '2026-08-15T12:00:00.000Z',
  });
  const candidateEvidence = {
    required: true,
    verified: true,
    stage: 'requirement_write_preflight',
    reason: 'retained_browser_exact_match',
    expected: {
      sessionName: 'workshop',
      profileId: 'chatgpt-pro',
      targetId: target.id,
      url: exactUrl,
    },
    observed: {
      sessionName: 'workshop',
      browserId: browser.id,
      browserPid: process.pid,
      cdpUrl: browser.cdpEndpoint,
      profileId: 'chatgpt-pro',
      health: 'ready',
      targetId: target.id,
      url: exactUrl,
    },
  };
  assert.throws(
    () => rotateRetainedBrowserRequirement({
      path: recoveryPath,
      evidence: candidateEvidence,
      expectedSha256: recoveryOld.sha256,
      staleEvidence: { confirmed: true, reason: 'retained_daemon_missing' },
      now: () => '2026-08-16T12:00:00.000Z',
      afterPhase: (phase) => {
        if (phase === 'requirement_replaced') throw new Error('fixture interrupted rotation');
      },
    }),
    /fixture interrupted rotation/,
  );
  const recoveredRotation = await discoverVerifyAndPinRetainedBrowser({
    agentBrowserBin: '/fixture/agent-browser',
    exactUrl,
    profileId: 'chatgpt-pro',
    requirementPath: recoveryPath,
    expectedOpenedIdentity: opened,
    rotateExpectedSha256: recoveryOld.sha256,
    adapters: deadOldAdapters,
  });
  assert.equal(recoveredRotation.requirement.rotated, true);
  assert.equal(recoveredRotation.requirement.previousSha256, recoveryOld.sha256);

  const liveRotationPath = join(root, 'live-rotation', 'retained.json');
  const liveRequirement = writeRetainedBrowserRequirement({
    path: liveRotationPath,
    evidence: oldEvidence,
    now: () => '2026-08-15T12:00:00.000Z',
  });
  const oldBrowser = {
    id: 'session:old-workshop',
    pid: process.pid,
    profileId: 'chatgpt-pro',
    health: 'ready',
    cdpEndpoint: 'ws://127.0.0.1:9444/devtools/browser/old-workshop',
  };
  await assert.rejects(
    discoverVerifyAndPinRetainedBrowser({
      agentBrowserBin: '/fixture/agent-browser',
      exactUrl,
      profileId: 'chatgpt-pro',
      requirementPath: liveRotationPath,
      expectedOpenedIdentity: opened,
      rotateExpectedSha256: liveRequirement.sha256,
      adapters: {
        ...adapters,
        readBrowser: async (sessionName) => ({
          success: true,
          browser: sessionName === 'old-workshop' ? oldBrowser : browser,
        }),
      },
    }),
    /retained_browser_rotation_old_authority_live/,
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard retained browser live pin fixture passed');

function evidenceFor(expected) {
  return {
    required: true,
    verified: true,
    stage: 'read_only_preflight',
    reason: 'retained_browser_exact_match',
    expected,
  };
}
