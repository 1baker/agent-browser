#!/usr/bin/env node

import { createReadStream } from 'node:fs';
import { existsSync, mkdirSync, readFileSync, readlinkSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { join } from 'node:path';

import { getServiceAccessPlan } from '../packages/client/src/service-observability.js';
import {
  postServiceRequest,
  requestServiceExternalByopAdopt,
} from '../packages/client/src/service-request.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-external-byop-adopt-',
  sessionPrefix: 'external-byop-adopt',
});

const { agentHome, session, tempHome } = context;
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
if (!process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD && existsSync('/usr/bin/google-chrome')) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = '/usr/bin/google-chrome';
}
const externalSession = `${session}-source`;
const serviceName = 'ExternalByopAdoptSmoke';
const agentName = 'smoke-agent';
const taskName = 'adoptExistingChrome';
const targetServiceId = 'auracall-smoke';
const profileId = `external-byop-smoke-${process.pid}`;
const userDataDir = join(tempHome, 'external-byop-user-data');
const externalChrome = process.env.AGENT_BROWSER_SMOKE_EXTERNAL_CHROME;
const expectedBrowserBuild = process.env.AGENT_BROWSER_SMOKE_EXTERNAL_BROWSER_BUILD || 'stock_chrome';
const adoptUrl = externalChrome
  ? `data:text/html,${encodeURIComponent('<title>External BYOP Adopt Smoke</title><button style="position:absolute;left:100px;top:100px;width:200px;height:80px" onclick="document.title=\'OS_CLICKED\'">Click with OS</button><div id="fixture-editor" contenteditable="true" style="position:absolute;left:100px;top:220px">agent-browser .com input test</div>')}`
  : smokeDataUrl('External BYOP Adopt Smoke', 'External BYOP Adopt Smoke');
if (externalChrome) context.env.AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY = context.env.DISPLAY;
let externalBrowser = null;

const timeout = setTimeout(() => {
  fail('Timed out waiting for external BYOP adopt smoke to complete');
}, 120000);

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [profileId]: {
            id: profileId,
            name: 'External BYOP smoke profile',
            profileOrigin: 'external_byop',
            ...(externalChrome ? { browserBuild: expectedBrowserBuild } : {}),
            userDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
            cleanup: 'detach',
            registration: {
              serviceName,
              agentName,
              taskName,
            },
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function seedObservedBuildProof(browserPid) {
  const executablePath = readlinkSync(`/proc/${browserPid}/exe`);
  const digest = createHash('sha256');
  for await (const chunk of createReadStream(executablePath)) digest.update(chunk);
  const statePath = join(agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.browserCapabilityRegistry = {
    browserHosts: [{ id: 'external-local-smoke', hostKind: 'local', reachable: true, lifecycleOwner: 'external' }],
    browserExecutables: [{ id: 'source-chrome-smoke', hostId: 'external-local-smoke', buildLabel: expectedBrowserBuild, executablePath, sha256: digest.digest('hex') }],
    browserCapabilities: [{ id: 'source-cdp-smoke', hostId: 'external-local-smoke', executableId: 'source-chrome-smoke', cdpSupported: true }],
    profileCompatibility: [{ id: 'source-profile-smoke', profileId, hostId: 'external-local-smoke', executableId: 'source-chrome-smoke', compatible: true }],
    validationEvidence: [{ id: 'source-validation-smoke', hostId: 'external-local-smoke', executableId: 'source-chrome-smoke', capabilityId: 'source-cdp-smoke', kind: 'cdp_attach', state: 'passed' }],
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

async function streamPort() {
  const statusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(statusResult.stdout, 'stream status');
  assert(stream.success === true, `stream status failed: ${statusResult.stdout}${statusResult.stderr}`);
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    if (externalBrowser) {
      if (externalBrowser.exitCode === null) {
        externalBrowser.kill('SIGTERM');
        await Promise.race([
          new Promise((resolve) => externalBrowser.once('exit', resolve)),
          new Promise((resolve) => setTimeout(resolve, 3000)),
        ]);
      }
    } else {
      await closeSession({ ...context, session: externalSession });
    }
  } catch (error) {
    console.error(`External-browser cleanup warning: ${error instanceof Error ? error.message : error}`);
  }
  try {
    await closeSession(context);
  } catch (error) {
    console.error(`Main-session cleanup warning: ${error instanceof Error ? error.message : error}`);
  } finally {
    if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
      console.error(`Keeping smoke home: ${tempHome}`);
    } else {
      context.cleanupTempHome();
    }
  }
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

try {
  seedServiceState();
  let cdpUrl;
  let browserPid;
  if (externalChrome) {
    externalBrowser = spawn(externalChrome, [
      '--no-sandbox', '--disable-gpu', '--no-first-run', '--remote-debugging-port=0',
      // Chromium may rewrite argv into one process title. Keep every token
      // after the executable as a switch so physical adoption stays provable.
      `--user-data-dir=${userDataDir}`,
    ], { env: context.env, stdio: 'ignore' });
    browserPid = externalBrowser.pid;
    const portFile = join(userDataDir, 'DevToolsActivePort');
    for (let attempt = 0; attempt < 100 && !existsSync(portFile) && externalBrowser.exitCode === null; attempt += 1) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    assert(existsSync(portFile), 'external Chrome did not create DevToolsActivePort');
    const [cdpPort, browserPath] = readFileSync(portFile, 'utf8').trim().split('\n');
    cdpUrl = `ws://127.0.0.1:${cdpPort}${browserPath}`;
  } else {
    const sourceOpen = await runCli(context, ['--json', '--session', externalSession, 'open', adoptUrl], 120000);
    const sourceOpenJson = parseJsonOutput(sourceOpen.stdout, 'source open');
    assert(sourceOpenJson.success === true, `source open failed: ${sourceOpen.stdout}${sourceOpen.stderr}`);
    const cdpResult = await runCli(context, ['--json', '--session', externalSession, 'get', 'cdp-url']);
    const cdp = parseJsonOutput(cdpResult.stdout, 'source cdp-url');
    cdpUrl = cdp.data?.cdpUrl;
    const pidResult = await runCli(context, ['--json', '--session', externalSession, 'get', 'browser-pid']);
    const pid = parseJsonOutput(pidResult.stdout, 'source browser-pid');
    browserPid = pid.data?.pid;
  }
  assert(typeof cdpUrl === 'string' && cdpUrl.startsWith('ws://'), 'source cdp-url missing');
  assert(Number.isInteger(browserPid) && browserPid > 0, 'source browser-pid missing');
  if (externalChrome) await seedObservedBuildProof(browserPid);

  const port = await streamPort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const adopted = await requestServiceExternalByopAdopt({
    baseUrl,
    serviceName,
    agentName,
    taskName,
    runtimeProfile: profileId,
    cdpUrl,
    browserPid,
    url: adoptUrl,
  });
  assert(adopted.success === true, `adoption request failed: ${JSON.stringify(adopted)}`);
  assert(adopted.data?.adopted === true, `adoption did not report adopted: ${JSON.stringify(adopted)}`);
  if (externalChrome) {
    assert(adopted.data?.browserBuildProof?.applied === true, `adoption lacks verified build proof: ${JSON.stringify(adopted.data)}`);
  }
  assert(
    adopted.data?.serviceTabHandle?.profileOrigin === 'external_byop',
    `adoption returned wrong profile origin: ${JSON.stringify(adopted.data?.serviceTabHandle)}`,
  );

  const plan = await getServiceAccessPlan({
    baseUrl,
    serviceName,
    agentName,
    taskName: 'verifyReuseAfterAdopt',
    targetServiceId,
  });
  assert(
    plan.decision?.profileReuse?.recommendedAction === 'reuse_existing_browser',
    `access plan did not recommend reuse: ${JSON.stringify(plan.decision?.profileReuse)}`,
  );
  assert(
    plan.decision?.profileReuse?.reusableBrowserId === `session:${session}`,
    `access plan reused wrong browser: ${JSON.stringify(plan.decision?.profileReuse)}`,
  );
  assert(
    plan.decision?.serviceRequest?.request?.sessionName === session,
    `access plan did not include reusable session route: ${JSON.stringify(plan.decision?.serviceRequest)}`,
  );
  if (externalChrome) {
    const followUp = await postServiceRequest({
      baseUrl,
      request: {
        action: 'tab_list', serviceName, agentName, taskName: 'verifyProvenBuildDispatch',
        browserId: `session:${session}`, sessionName: session,
        browserBuild: expectedBrowserBuild, runtimeProfile: profileId,
      },
    });
    assert(followUp.success === true, `proven-build follow-up dispatch failed: ${JSON.stringify(followUp)}`);
    const rejected = await postServiceRequest({
      baseUrl,
      request: {
        action: 'ui_action', serviceName, agentName, taskName: 'rejectOutOfBoundsOsClick',
        serviceTabHandle: adopted.data.serviceTabHandle,
        browserId: `session:${session}`, sessionName: session,
        browserBuild: expectedBrowserBuild, runtimeProfile: profileId,
        timeoutMs: 5000,
        uiAction: { steps: [{ type: 'os_click', viewportX: 100000, viewportY: 140 }] },
      },
    });
    assert(rejected.success === true && rejected.data?.ok === false, `out-of-bounds OS click was not rejected: ${JSON.stringify(rejected)}`);
    assert(rejected.data?.after?.title === 'External BYOP Adopt Smoke', 'rejected OS click changed the fixture');
    const osClick = await postServiceRequest({
      baseUrl,
      request: {
        action: 'ui_action', serviceName, agentName, taskName: 'verifyOsClick',
        serviceTabHandle: adopted.data.serviceTabHandle,
        browserId: `session:${session}`, sessionName: session,
        browserBuild: expectedBrowserBuild, runtimeProfile: profileId,
        timeoutMs: 5000,
        uiAction: { steps: [{ type: 'os_click', viewportX: 200, viewportY: 140 }] },
      },
    });
    assert(osClick.success === true && osClick.data?.ok === true, `OS click failed: ${JSON.stringify(osClick)}`);
    assert(osClick.data?.steps?.[0]?.result?.inputSource === 'xdotool_x11_xtest', 'OS click receipt lacks xdotool input source');
    const displayEvidence = osClick.data?.steps?.[0]?.result?.displayBindingEvidence;
    const processEnvironment = readFileSync(`/proc/${browserPid}/environ`);
    const expectedDisplayEvidence = processEnvironment.length > 0 && processEnvironment.every((byte) => byte === 0)
      ? 'scrubbed_process_environ_with_x11_proof'
      : 'process_environ';
    assert(displayEvidence === expectedDisplayEvidence, `OS click display proof mismatch: ${displayEvidence}`);
    assert(osClick.data?.after?.title === 'OS_CLICKED', `OS click did not change fixture title: ${JSON.stringify(osClick.data)}`);
    const clearEditable = await postServiceRequest({
      baseUrl,
      request: {
        action: 'ui_action', serviceName, agentName, taskName: 'clearContenteditable',
        serviceTabHandle: adopted.data.serviceTabHandle,
        browserId: `session:${session}`, sessionName: session,
        browserBuild: expectedBrowserBuild, runtimeProfile: profileId,
        timeoutMs: 5000,
        uiAction: { steps: [{ type: 'clear', selector: '#fixture-editor' }] },
      },
    });
    assert(clearEditable.success === true && clearEditable.data?.ok === true, `contenteditable clear failed: ${JSON.stringify(clearEditable)}`);
    const editorState = await postServiceRequest({
      baseUrl,
      request: {
        action: 'evaluate', serviceName, agentName, taskName: 'verifyContenteditableClear',
        serviceTabHandle: adopted.data.serviceTabHandle,
        browserId: `session:${session}`, sessionName: session,
        browserBuild: expectedBrowserBuild, runtimeProfile: profileId,
        expression: `!(document.querySelector('#fixture-editor')?.textContent || '').trim()`,
        timeoutMs: 5000, maxReturnBytes: 100,
      },
    });
    assert(editorState.success === true && editorState.data?.result === true, `contenteditable retained text after clear: ${JSON.stringify(editorState)}`);
  }

  const browsers = await httpJson(port, 'GET', '/api/service/browsers');
  assert(browsers.success === true, `browser readback failed: ${JSON.stringify(browsers)}`);
  const adoptedBrowser = browsers.data?.browsers?.find((browser) => browser.id === `session:${session}`);
  assert(adoptedBrowser, `adopted browser missing from readback: ${JSON.stringify(browsers.data)}`);
  assert(adoptedBrowser.profileId === profileId, `adopted browser profile mismatch: ${JSON.stringify(adoptedBrowser)}`);
  assert(adoptedBrowser.host === 'attached_existing', `adopted browser host mismatch: ${JSON.stringify(adoptedBrowser)}`);
  assert(adoptedBrowser.pid === browserPid, `adopted browser PID mismatch: ${JSON.stringify(adoptedBrowser)}`);
  if (externalChrome) {
    assert(adoptedBrowser.browserBuild === expectedBrowserBuild, `adopted browser build mismatch: ${JSON.stringify(adoptedBrowser)}`);
    assert(adoptedBrowser.browserBuildProof?.applied === true, `adopted browser proof missing: ${JSON.stringify(adoptedBrowser)}`);
  }
  const cdpStream = adoptedBrowser.viewStreams?.find((stream) => stream?.provider === 'cdp_screencast');
  assert(cdpStream, `adopted browser missing CDP stream: ${JSON.stringify(adoptedBrowser)}`);
  assert(cdpStream.url === `http://127.0.0.1:${port}/`, `adopted stream URL mismatch: ${JSON.stringify(cdpStream)}`);
  assert(cdpStream.controlInput === 'cdp_input', `adopted stream control mismatch: ${JSON.stringify(cdpStream)}`);
  assert(cdpStream.readOnly === false, `adopted stream should be controllable: ${JSON.stringify(cdpStream)}`);
  assert(
    cdpStream.readiness?.state === 'ready' && cdpStream.readiness?.reason === 'stream_server_ready',
    `adopted stream readiness mismatch: ${JSON.stringify(cdpStream)}`,
  );
  const frame = await httpJson(port, 'GET', `/api/stream/${port}/frame`);
  assert(frame.success === true, `adopted stream frame endpoint failed: ${JSON.stringify(frame)}`);
  assert(typeof frame.frame === 'string' && frame.frame.length > 100, `adopted stream did not return a frame: ${JSON.stringify(frame)}`);

  await cleanup();
  console.log('External BYOP adopt live smoke passed');
} catch (err) {
  await fail(err instanceof Error ? err.stack || err.message : String(err));
}
