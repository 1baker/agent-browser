#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { existsSync, mkdirSync, readFileSync, readlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  evaluateServiceTab,
  releaseServiceTabHandle,
  reopenServiceTab,
  requestServiceExternalByopAdopt,
  requestServiceTab,
} from '../packages/client/src/service-request.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  smokeDataUrl,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-tab-parking-',
  sessionPrefix: 'service-tab-parking',
});
const serviceName = 'ServiceTabParkingSmoke';
const agentName = 'parking-smoke-agent';
const profileId = `service-tab-parking-${process.pid}`;
const userDataDir = join(context.tempHome, 'external-browser-profile');
let port = null;
let externalBrowser = null;

function resolveBrowserExecutable() {
  const candidates = [
    process.env.AGENT_BROWSER_SMOKE_EXTERNAL_CHROME,
    join(process.env.HOME || '', '.local/share/agent-browser/fortress-150/chrome'),
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
  ];
  return candidates.find((candidate) => candidate && existsSync(candidate)) ?? null;
}

function seedExternalProfile() {
  const serviceDir = join(context.agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify({
      profiles: {
        [profileId]: {
          id: profileId,
          name: 'Service tab parking external profile',
          profileOrigin: 'external_byop',
          browserBuild: 'stock_chrome',
          userDataDir,
          targetServiceIds: ['parking-smoke'],
          authenticatedServiceIds: ['parking-smoke'],
          sharedServiceIds: [serviceName],
          persistent: true,
          cleanup: 'detach',
          registration: { serviceName, agentName, taskName: 'adopt-parking-fixture' },
        },
      },
    }, null, 2)}\n`,
  );
}

async function seedObservedBuildProof(browserPid) {
  const executablePath = readlinkSync(`/proc/${browserPid}/exe`);
  const digest = createHash('sha256');
  for await (const chunk of createReadStream(executablePath)) digest.update(chunk);
  const statePath = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.browserCapabilityRegistry = {
    browserHosts: [{ id: 'parking-smoke-host', hostKind: 'local', reachable: true, lifecycleOwner: 'external' }],
    browserExecutables: [{ id: 'parking-smoke-browser', hostId: 'parking-smoke-host', buildLabel: 'stock_chrome', executablePath, sha256: digest.digest('hex') }],
    browserCapabilities: [{ id: 'parking-smoke-cdp', hostId: 'parking-smoke-host', executableId: 'parking-smoke-browser', cdpSupported: true }],
    profileCompatibility: [{ id: 'parking-smoke-profile', profileId, hostId: 'parking-smoke-host', executableId: 'parking-smoke-browser', compatible: true }],
    validationEvidence: [{ id: 'parking-smoke-validation', hostId: 'parking-smoke-host', executableId: 'parking-smoke-browser', capabilityId: 'parking-smoke-cdp', kind: 'cdp_attach', state: 'passed' }],
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

async function launchExternalBrowser(executable) {
  externalBrowser = spawn(executable, [
    '--headless=new',
    '--no-sandbox',
    '--disable-gpu',
    '--no-first-run',
    '--remote-debugging-port=0',
    `--user-data-dir=${userDataDir}`,
  ], { env: context.env, stdio: 'ignore' });
  const portFile = join(userDataDir, 'DevToolsActivePort');
  for (let attempt = 0; attempt < 100 && !existsSync(portFile) && externalBrowser.exitCode === null; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert(existsSync(portFile), 'external browser did not create DevToolsActivePort');
  const [cdpPort, browserPath] = readFileSync(portFile, 'utf8').trim().split('\n');
  return `ws://127.0.0.1:${cdpPort}${browserPath}`;
}

async function cleanup() {
  await closeSession(context).catch(() => undefined);
  if (externalBrowser?.exitCode === null) {
    externalBrowser.kill('SIGTERM');
    await Promise.race([
      new Promise((resolve) => externalBrowser.once('exit', resolve)),
      new Promise((resolve) => setTimeout(resolve, 3000)),
    ]);
  }
  context.cleanupTempHome();
}

try {
  context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
  const browserExecutable = resolveBrowserExecutable();
  assert(browserExecutable, 'no isolated Chromium executable is available for the parking smoke');
  seedExternalProfile();
  const cdpUrl = await launchExternalBrowser(browserExecutable);
  await seedObservedBuildProof(externalBrowser.pid);
  port = await ensureStreamPort(context);
  const baseUrl = `http://127.0.0.1:${port}`;
  const adopted = await requestServiceExternalByopAdopt({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'adopt-parking-fixture',
    runtimeProfile: profileId,
    cdpUrl,
    browserPid: externalBrowser.pid,
    url: 'about:blank',
  });
  assert(adopted.success === true && adopted.data?.adopted === true, `fixture adoption failed: ${JSON.stringify(adopted)}`);
  assert(adopted.data?.browserBuildProof?.applied === true, `fixture adoption lacks build proof: ${JSON.stringify(adopted)}`);
  const first = await requestServiceTab({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'open-parkable-tab',
    browserId: `session:${context.session}`,
    sessionName: context.session,
    runtimeProfile: profileId,
    url: smokeDataUrl('Parkable Tab', 'Parkable Tab'),
    jobTimeoutMs: 120000,
  });
  const second = await requestServiceTab({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'open-peer-tab',
    browserId: `session:${context.session}`,
    sessionName: context.session,
    runtimeProfile: profileId,
    url: smokeDataUrl('Peer Tab', 'Peer Tab'),
    jobTimeoutMs: 120000,
  });
  assert(first.success === true, `first tab open failed: ${JSON.stringify(first)}`);
  assert(second.success === true, `peer tab open failed: ${JSON.stringify(second)}`);
  const firstHandle = first.data?.serviceTabHandle;
  const secondHandle = second.data?.serviceTabHandle;
  assert(firstHandle?.valid === true, `first tab missing handle: ${JSON.stringify(first)}`);
  assert(secondHandle?.valid === true, `peer tab missing handle: ${JSON.stringify(second)}`);

  const parked = await releaseServiceTabHandle({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'park-first-tab',
    serviceTabHandle: firstHandle,
    requirePhysicalClose: true,
    jobTimeoutMs: 120000,
  });
  assert(parked.success === true, `tab park failed: ${JSON.stringify(parked)}`);
  assert(
    parked.data?.tabReleased === true && parked.data?.physicalTabClosed === true,
    `tab park did not close and retain the exact target: ${JSON.stringify(parked)}`,
  );

  const peerProof = await evaluateServiceTab({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'prove-peer-survives-park',
    serviceTabHandle: secondHandle,
    script: 'document.title',
    returnByValue: true,
    timeoutMs: 5000,
    maxReturnBytes: 128,
    jobTimeoutMs: 120000,
  });
  assert(peerProof.success === true && peerProof.data?.ok === true, `peer tab failed after park: ${JSON.stringify(peerProof)}`);
  assert(
    peerProof.data?.result?.result?.value === 'Peer Tab' || peerProof.data?.result === 'Peer Tab',
    `park affected the peer tab: ${JSON.stringify(peerProof)}`,
  );

  const reopened = await reopenServiceTab({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'reopen-parked-tab',
    browserId: firstHandle.browserId,
    sessionName: firstHandle.sessionName,
    tabId: firstHandle.tabId,
    jobTimeoutMs: 120000,
  });
  assert(reopened.success === true, `tab reopen failed: ${JSON.stringify(reopened)}`);
  const reopenedHandle = reopened.data?.serviceTabHandle;
  assert(
    reopened.data?.reopened === true &&
      reopened.data?.reopenedFromTabId === firstHandle.tabId &&
      reopened.data?.reopenedAsTabId !== firstHandle.tabId &&
      reopenedHandle?.valid === true &&
      reopenedHandle.profileOrigin === 'external_byop' &&
      reopenedHandle.browserId === firstHandle.browserId &&
      reopenedHandle.sessionName === firstHandle.sessionName &&
      reopenedHandle.targetId !== firstHandle.targetId,
    `tab reopen did not return a fresh attached target on the same route: ${JSON.stringify(reopened)}`,
  );

  const reopenProof = await evaluateServiceTab({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'prove-reopened-tab',
    serviceTabHandle: reopenedHandle,
    script: 'document.title',
    returnByValue: true,
    timeoutMs: 5000,
    maxReturnBytes: 128,
    jobTimeoutMs: 120000,
  });
  assert(reopenProof.success === true && reopenProof.data?.ok === true, `reopened tab was not attachable: ${JSON.stringify(reopenProof)}`);
  assert(
    reopenProof.data?.result?.result?.value === 'Parkable Tab' || reopenProof.data?.result === 'Parkable Tab',
    `reopened tab did not restore the recorded URL: ${JSON.stringify(reopenProof)}`,
  );

  const status = await httpJson(port, 'GET', '/api/service/status');
  const tabs = status.data?.service_state?.tabs ?? {};
  assert(tabs[firstHandle.tabId]?.lifecycle === 'closed', `parked history was not retained: ${JSON.stringify(tabs[firstHandle.tabId])}`);
  assert(tabs[reopenedHandle.tabId]?.lifecycle === 'ready', `reopened target was not retained as ready: ${JSON.stringify(tabs[reopenedHandle.tabId])}`);
  assert(tabs[reopenedHandle.tabId]?.serviceTabHandle?.valid === true, 'reopened retained tab does not expose a valid handle');

  console.log('Service tab Park/Reopen live smoke passed');
} finally {
  await cleanup();
}
