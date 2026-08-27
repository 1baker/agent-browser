import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { resolveRuntimeSocketDir } from './runtime-socket-dir.js';
import {
  resolveRuntimeDaemonClientBinary as runtimeDaemonClientBinary,
} from './runtime-daemon-client-binary.js';
import { selectRuntimeHandoffBrowser } from './runtime-handoff-browser-selection.js';

import {
  discoverRetainedBrowserExpectation,
} from './local-dashboard-retained-browser-discovery.js';
import {
  evaluateRetainedBrowserExpectation,
  isLoopbackDevToolsUrl,
} from './local-dashboard-retained-browser-guard.js';
import {
  writeRetainedBrowserRequirement,
} from './local-dashboard-retained-browser-requirement.js';

/**
 * Uniquely rediscover, reverify, and marker-first pin one exact live lane.
 * This module reads daemon and CDP state but has no browser action primitive.
 */
export async function discoverVerifyAndPinRetainedBrowser({
  agentBrowserBin,
  exactUrl,
  profileId,
  requirementPath,
  expectedOpenedIdentity,
  socketDir = runtimeSocketDir(),
  adapters,
}) {
  const liveAdapters = adapters || {
    exactUrl,
    profileId,
    sessionNames: runtimeSessionNames(socketDir),
    readDaemonPid: (sessionName) => readRuntimePid(socketDir, sessionName),
    isProcessLive,
    readBrowser: async (sessionName, daemonPid) => serviceBrowserForSession(
      runtimeDaemonClientBinary(daemonPid, agentBrowserBin),
      sessionName,
    ),
    readCdpTargets: readCdpTargetInventory,
  };
  const discovery = await discoverRetainedBrowserExpectation({
    ...liveAdapters,
    exactUrl,
    profileId,
  });
  requireOpenedIdentityAgreement(discovery.expectation, expectedOpenedIdentity);

  const daemonPid = liveAdapters.readDaemonPid(discovery.expectation.sessionName);
  if (!liveAdapters.isProcessLive(daemonPid)) {
    throw liveError(
      'retained_browser_preparation_daemon_lost',
      'The uniquely discovered daemon exited before requirement commit',
    );
  }
  const readback = await liveAdapters.readBrowser(discovery.expectation.sessionName, daemonPid);
  if (!readback.success || !readback.browser?.cdpEndpoint) {
    throw liveError(
      'retained_browser_preparation_service_lost',
      'The uniquely discovered browser became unreadable before requirement commit',
    );
  }
  const targets = await liveAdapters.readCdpTargets(readback.browser.cdpEndpoint);
  const evidence = evaluateRetainedBrowserExpectation({
    browser: readback.browser,
    cdpTargets: targets,
    expectation: discovery.expectation,
    stage: 'requirement_write_preflight',
  });
  if (!evidence.verified) {
    throw liveError(
      evidence.reason || 'retained_browser_preparation_reverification_failed',
      evidence.message || 'The exact retained lane changed before requirement commit',
    );
  }
  const requirement = writeRetainedBrowserRequirement({
    path: requirementPath,
    evidence,
  });
  return {
    discovery: {
      exactUrl: discovery.exactUrl,
      profileId: discovery.profileId,
      inspectedSessionCount: discovery.inspectedSessionCount,
      matchedCandidateCount: discovery.matchedCandidateCount,
    },
    requirement: {
      path: requirement.path,
      exists: requirement.exists === true,
      sha256: requirement.sha256 ?? null,
      createdAt: requirement.createdAt ?? null,
      written: requirement.written === true,
    },
  };
}

export function runtimeSocketDir() {
  return resolveRuntimeSocketDir();
}

function runtimeSessionNames(socketDir) {
  if (!existsSync(socketDir)) return [];
  const suffix = process.platform === 'win32' ? '.port' : '.sock';
  return readdirSync(socketDir)
    .filter((name) => name.endsWith(suffix))
    .map((name) => name.slice(0, -suffix.length))
    .filter((name) => /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name))
    .sort();
}

function readRuntimePid(socketDir, sessionName) {
  try {
    const value = Number.parseInt(
      readFileSync(join(socketDir, `${sessionName}.pid`), 'utf8').trim(),
      10,
    );
    return Number.isInteger(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

function isProcessLive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function serviceBrowserForSession(binary, sessionName) {
  const result = spawnSync(binary, [
    '--json',
    '--session',
    sessionName,
    'service',
    'browsers',
  ], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 16 * 1024 * 1024,
    timeout: 30000,
  });
  let payload = null;
  try {
    payload = JSON.parse(String(result.stdout || '').trim());
  } catch {
    // The bounded failure below intentionally excludes raw browser output.
  }
  const browsers = payload?.data?.browsers || [];
  const selection = selectRuntimeHandoffBrowser({ browsers, sessionName });
  return {
    success: result.status === 0 && payload?.success === true && selection.error === null,
    browser: selection.browser,
  };
}

async function readCdpTargetInventory(cdpUrl) {
  if (!isLoopbackDevToolsUrl(cdpUrl)) {
    throw liveError(
      'retained_browser_preparation_cdp_not_loopback',
      'The retained browser DevTools endpoint must use loopback',
    );
  }
  const endpoint = new URL(cdpUrl);
  endpoint.protocol = endpoint.protocol === 'wss:' ? 'https:' : 'http:';
  endpoint.pathname = '/json/list';
  endpoint.search = '';
  endpoint.hash = '';
  const response = await fetch(endpoint, { signal: AbortSignal.timeout(5000) });
  if (!response.ok) {
    throw liveError(
      'retained_browser_preparation_cdp_unreadable',
      `The retained browser target inventory returned HTTP ${response.status}`,
    );
  }
  const targets = await response.json();
  if (!Array.isArray(targets)) {
    throw liveError(
      'retained_browser_preparation_cdp_invalid',
      'The retained browser target inventory is invalid',
    );
  }
  return targets;
}

function requireOpenedIdentityAgreement(expectation, opened) {
  if (
    !opened
    || expectation.sessionName !== opened.sessionName
    || expectation.browserId !== opened.browserId
    || expectation.profileId !== opened.profileId
    || expectation.targetId !== opened.targetId
    || expectation.url !== opened.url
  ) {
    throw liveError(
      'retained_browser_preparation_open_discovery_mismatch',
      'Unique live discovery does not match the route-bound opened identity',
    );
  }
}

function liveError(code, message) {
  const error = new Error(`${code}: ${message}`);
  error.code = code;
  return error;
}
