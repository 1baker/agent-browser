#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import {
  buildRetainedBrowserRemoteViewArgs,
  normalizeRetainedBrowserPreparationRequest,
  verifyRetainedBrowserRemoteViewResult,
} from './lib/local-dashboard-retained-browser-preparation.js';
import {
  discoverVerifyAndPinRetainedBrowser,
} from './lib/local-dashboard-retained-browser-live.js';

const options = {
  agentBrowserBin: process.env.AGENT_BROWSER_INSTALL_BIN
    || resolve(homedir(), '.local', 'bin', 'agent-browser'),
  agentName: 'codex',
  browserBuild: 'stock_chrome',
  jobTimeoutMs: 120000,
  json: false,
  runtimeProfile: '',
  retainedRequirement: process.env.AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT
    || resolve(homedir(), '.agent-browser', 'publications', 'local-dashboard-retained-browser.json'),
  rotateStaleRequirementSha256: '',
  serviceName: 'AuraCall',
  taskName: 'prepare-retained-browser-lane',
  url: '',
  urlPrefix: '',
};

for (let index = 0; index < process.argv.slice(2).length; index += 1) {
  const args = process.argv.slice(2);
  const arg = args[index];
  if (arg === '--') continue;
  if (arg === '--agent-browser-bin') options.agentBrowserBin = requiredValue(args, ++index, arg);
  else if (arg === '--agent-name') options.agentName = requiredValue(args, ++index, arg);
  else if (arg === '--browser-build') options.browserBuild = requiredValue(args, ++index, arg);
  else if (arg === '--job-timeout-ms') options.jobTimeoutMs = requiredValue(args, ++index, arg);
  else if (arg === '--json') options.json = true;
  else if (arg === '--runtime-profile') options.runtimeProfile = requiredValue(args, ++index, arg);
  else if (arg === '--retained-requirement') options.retainedRequirement = requiredValue(args, ++index, arg);
  else if (arg === '--rotate-stale-requirement-sha256') {
    options.rotateStaleRequirementSha256 = requiredValue(args, ++index, arg);
  }
  else if (arg === '--service-name') options.serviceName = requiredValue(args, ++index, arg);
  else if (arg === '--task-name') options.taskName = requiredValue(args, ++index, arg);
  else if (arg === '--url') options.url = requiredValue(args, ++index, arg);
  else if (arg === '--url-prefix') options.urlPrefix = requiredValue(args, ++index, arg);
  else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else fail(`Unknown argument: ${arg}`);
}

try {
  const request = normalizeRetainedBrowserPreparationRequest(options);
  if (!existsSync(options.agentBrowserBin)) {
    fail(`Installed agent-browser binary not found: ${options.agentBrowserBin}`);
  }
  const openPayload = runJson(options.agentBrowserBin, buildRetainedBrowserRemoteViewArgs(request));
  const opened = verifyRetainedBrowserRemoteViewResult(openPayload, request);
  const pinPayload = process.env.AGENT_BROWSER_RETAINED_PINNER_SCRIPT
    ? runJson(process.execPath, [
      process.env.AGENT_BROWSER_RETAINED_PINNER_SCRIPT,
      '--exact-url',
      request.url,
      '--profile-id',
      request.runtimeProfile,
      '--requirement-path',
      resolve(options.retainedRequirement),
      ...(options.rotateStaleRequirementSha256 ? [
        '--rotate-stale-requirement-sha256',
        options.rotateStaleRequirementSha256,
      ] : []),
    ]).data
    : await discoverVerifyAndPinRetainedBrowser({
      agentBrowserBin: options.agentBrowserBin,
      exactUrl: request.url,
      profileId: request.runtimeProfile,
      requirementPath: resolve(options.retainedRequirement),
      expectedOpenedIdentity: opened,
      rotateExpectedSha256: options.rotateStaleRequirementSha256 || null,
    });
  if (
    pinPayload?.discovery?.matchedCandidateCount !== 1
    || pinPayload?.discovery?.exactUrl !== request.url
    || pinPayload?.discovery?.profileId !== request.runtimeProfile
    || pinPayload?.requirement?.exists !== true
  ) {
    fail('Exact retained browser requirement was not durably pinned');
  }
  output({
    success: true,
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-preparation.v1',
    state: 'ready_and_pinned',
    opened,
    discovery: pinPayload.discovery,
    requirement: pinPayload.requirement,
  });
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}

function runJson(command, args) {
  const result = spawnSync(command, args, {
    cwd: resolve(new URL('..', import.meta.url).pathname),
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    timeout: 660000,
  });
  let payload = null;
  try {
    payload = JSON.parse(result.stdout || '{}');
  } catch {
    // The bounded error below intentionally excludes raw browser output.
  }
  if (result.status !== 0 || payload?.success !== true) {
    const childError = String(
      payload?.error || result.error?.message || 'unknown structured child failure',
    ).replace(/[\r\n\0]+/g, ' ').slice(0, 512);
    throw new Error(`Guarded retained-browser step failed for ${command}: ${childError}`);
  }
  return payload;
}

function requiredValue(args, index, flag) {
  const value = args[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function output(payload) {
  if (options.json) console.log(JSON.stringify(payload));
  else console.log(`Retained browser preparation: ${payload.state}`);
}

function fail(message) {
  if (options.json) console.log(JSON.stringify({ success: false, error: message }));
  else console.error(message);
  process.exit(1);
}

function printHelp() {
  console.log(`Usage: node scripts/prepare-local-dashboard-retained-browser.js [options]

Open one exact URL in a route-bound managed browser, verify its rendered URL,
profile, target, session, and operator-visible route, then uniquely discover and
durably pin the retained identity. This command has no page interaction or
prompt-submission action.

Options:
  --url <url>                 Exact canonical URL to open and retain.
  --url-prefix <url>          Reviewed origin and path boundary containing the exact URL.
  --runtime-profile <id>      Required managed runtime profile.
  --retained-requirement <path>
                              Private durable requirement path.
  --rotate-stale-requirement-sha256 <sha256>
                              Rotate a proven-dead prior requirement matching this digest.
  --browser-build <build>     stock_chrome or stealthcdp_chromium. Default: stock_chrome.
  --service-name <name>       Service attribution. Default: AuraCall.
  --agent-name <name>         Agent attribution. Default: codex.
  --task-name <name>          Task attribution. Default: prepare-retained-browser-lane.
  --job-timeout-ms <ms>       Bounded remote-view open timeout. Default: 120000.
  --agent-browser-bin <path>  Installed agent-browser binary.
  --json                      Print structured JSON.
`);
}
