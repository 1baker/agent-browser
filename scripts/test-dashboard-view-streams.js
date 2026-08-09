#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  canControlViewStream,
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  controlInputLabel,
  viewStreamCapabilityLabel,
  viewStreamControlTitle,
  viewStreamDashboardFrameUrl,
  viewStreamLabel,
  viewStreamOpenTitle,
  viewStreamReadinessLabel,
  viewStreamRouteLabel,
  viewStreamRouteSummary,
  viewStreamLeaseLabel,
} from '../packages/dashboard/src/lib/service-view-streams.ts';
import {
  mergeWorkspaceViewStreams,
  selectWorkspaceViewStream,
  workspaceViewRecoveryAction,
  workspaceViewStreamKey,
} from '../packages/dashboard/src/lib/workspace-view-stream-selection.ts';
import {
  compactWorkspaceViewportReadinessComponents,
  deriveWorkspaceViewportReadiness,
  deriveWorkspaceViewportUxState,
  workspaceViewportReadinessStatusLabel,
  workspaceViewportUxStateLabel,
} from '../packages/dashboard/src/lib/workspace-viewport-state.ts';
import {
  selectedWorkspaceContextCanRenderViewport,
  serviceViewStreamForSelectedWorkspaceContext,
  serviceBrowserForWorkspaceSelection,
} from '../packages/dashboard/src/lib/workspace-browser-selection.ts';
import {
  borrowForeignCdpControl,
  dispatchForeignCdpInput,
  fetchForeignCdpScreenshot,
  foreignCdpScreenshotUrl,
  readForeignCdpControlStatus,
  releaseForeignCdpControl,
} from '../packages/dashboard/src/lib/foreign-cdp-control.ts';

const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const workspaceNavigator = readFileSync('packages/dashboard/src/components/workspace-navigator.tsx', 'utf8');
const workspaceViewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const css = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const rdpAutologinSetup = readFileSync('scripts/setup-rdp-autologin-user.sh', 'utf8');

const rdpGatewayStream = {
  id: 'remote-headed-view',
  provider: 'rdp_gateway',
  controlInput: 'manual_attached_desktop',
  url: 'http://127.0.0.1:8080/rdp/session',
  frameUrl: 'http://127.0.0.1:8080/guacamole/#/client/route-a',
  externalUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
  routeDescriptor: {
    localEmbedUrl: 'http://127.0.0.1:8080/guacamole/#/client/route-a',
    publicOperatorUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
    dashboardEmbedUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
    healthUrl: 'http://127.0.0.1:8080/guacamole/',
  },
  routeId: 'route-a',
  displayAllocationId: 'display-a',
  connectionId: 'guac-a',
  connectionName: 'Browser A',
  providerMode: 'simultaneous_view',
  viewerLeaseIds: ['viewer-a', 'viewer-b'],
  controllerLeaseId: 'viewer-a',
  remoteReadiness: { state: 'ready' },
  readOnly: false,
};

const selectableCdpScreencastStream = {
  id: 'cdp-live-view',
  provider: 'cdp_screencast',
  controlInput: 'cdp_input',
  url: 'http://127.0.0.1:9223/',
  readiness: { state: 'ready' },
  readOnly: false,
};

assert.equal(
  selectedWorkspaceContextCanRenderViewport({
    node: { id: 'manual-runtime:im-receipts-google-messages-main:37820' },
    stream: {
      provider: 'rdp_gateway',
      url: 'https://agent-browser.example.test/guacamole/#/client/route-b',
      embeddable: true,
    },
  }),
  true,
  'A detected manual-runtime RDP stream can drive the workspace viewport even without a service-owned browser record',
);
assert.equal(
  selectedWorkspaceContextCanRenderViewport({
    node: { id: 'manual-runtime:missing-stream:1' },
    stream: { provider: 'rdp_gateway', url: null, embeddable: true },
  }),
  false,
  'A selected workspace context still requires an embeddable stream URL',
);
assert.deepEqual(
  serviceViewStreamForSelectedWorkspaceContext(
    'manual-runtime:im-receipts-google-messages-main:37820',
    {
      provider: 'rdp_gateway',
      url: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
      routeId: 'guacamole:2',
      embeddable: true,
      controllable: true,
      controlInput: 'manual_attached_desktop',
      operatorVisibleState: 'controllable',
    },
    {
      id: 'guacamole:2',
      provider: 'rdp_gateway',
      frameUrl: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
      externalUrl: 'https://agent-browser.example.test/guacamole/#/client/route-b',
      routeDescriptor: {
        localEmbedUrl: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
        publicOperatorUrl: 'https://agent-browser.example.test/guacamole/#/client/route-b',
      },
    },
  ),
  {
    id: 'selected:manual-runtime:im-receipts-google-messages-main:37820:rdp_gateway',
    provider: 'rdp_gateway',
    controlInput: 'manual_attached_desktop',
    url: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
    frameUrl: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
    externalUrl: 'https://agent-browser.example.test/guacamole/#/client/route-b',
    routeDescriptor: {
      localEmbedUrl: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
      publicOperatorUrl: 'https://agent-browser.example.test/guacamole/#/client/route-b',
    },
    routeId: 'guacamole:2',
    displayAllocationId: null,
    connectionId: null,
    connectionName: null,
    routeSource: null,
    providerMode: null,
    viewerLeaseIds: undefined,
    controllerLeaseId: null,
    readOnly: undefined,
    readiness: { state: 'controllable', reason: undefined },
  },
  'A manual-runtime selection keeps the route public operator URL instead of embedding its loopback-only Guacamole URL on public ingress',
);

assert.equal(
  foreignCdpScreenshotUrl(45011, 'target-a', 'png'),
  '/api/session-screenshot?port=45011&format=png&targetId=target-a',
  'Foreign CDP capture uses the selected page target and an explicit image format',
);

let captureRequest = null;
const captured = await fetchForeignCdpScreenshot({
  port: 45011,
  targetId: 'target-a',
  format: 'png',
  fetcher: async (input, init) => {
    captureRequest = { input, init };
    return new Response(JSON.stringify({
      success: true,
      targetId: 'target-a',
      title: 'Detected page',
      format: 'png',
      dataUrl: 'data:image/png;base64,AA==',
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  },
});
assert.equal(captureRequest.input, '/api/session-screenshot?port=45011&format=png&targetId=target-a');
assert.equal(captureRequest.init.credentials, 'include');
assert.equal(captured.dataUrl, 'data:image/png;base64,AA==');

const controlRequests = [];
const controlFetcher = async (input, init) => {
  controlRequests.push({ input, init });
  if (input.startsWith('/api/foreign-cdp/control')) {
    return new Response(JSON.stringify({ active: false, lifecycleOwnership: false }), { status: 200 });
  }
  if (input === '/api/foreign-cdp/borrow') {
    return new Response(JSON.stringify({
      active: true,
      grantId: 'grant-a',
      owner: 'operator-a',
      expiresAt: '2026-08-05T12:05:00Z',
      allowedOperations: ['pointer', 'keyboard', 'wheel'],
      lifecycleOwnership: false,
    }), { status: 200 });
  }
  return new Response(JSON.stringify({ success: true, active: false, lifecycleOwnership: false }), { status: 200 });
};
await readForeignCdpControlStatus({ port: 45011, targetId: 'target-a', fetcher: controlFetcher });
const borrow = await borrowForeignCdpControl({
  port: 45011,
  targetId: 'target-a',
  reason: 'Investigate checkout failure',
  ttlSeconds: 300,
  fetcher: controlFetcher,
});
assert.equal(borrow.grantId, 'grant-a');
await dispatchForeignCdpInput({
  port: 45011,
  targetId: 'target-a',
  grantId: 'grant-a',
  input: { kind: 'mouse', eventType: 'mousePressed', x: 10, y: 20, button: 'left', clickCount: 1 },
  fetcher: controlFetcher,
});
await releaseForeignCdpControl({
  port: 45011,
  targetId: 'target-a',
  grantId: 'grant-a',
  fetcher: controlFetcher,
});
assert.equal(
  controlRequests[0].input,
  '/api/foreign-cdp/control?port=45011&targetId=target-a',
  'Borrow status is scoped to the selected foreign page target',
);
assert.deepEqual(
  JSON.parse(controlRequests[1].init.body),
  { port: 45011, targetId: 'target-a', reason: 'Investigate checkout failure', ttlSeconds: 300 },
);
assert.equal(controlRequests[2].input, '/api/foreign-cdp/input');
assert.equal(controlRequests[3].input, '/api/foreign-cdp/release');
assert.ok(controlRequests.every((request) => request.init.credentials === 'include'));
assert.match(
  workspaceViewport,
  /Borrow control[\s\S]*Release control/,
  'The foreign snapshot viewport exposes explicit Borrow and Release controls',
);
assert.match(
  workspaceViewport,
  /dispatchForeignCdpInput[\s\S]*canControl=\{foreignBorrow\?\.active === true\}/,
  'Pointer and keyboard input is enabled only while a visible Borrow grant is active',
);
assert.match(
  workspaceViewport,
  /onKeyDown/,
  'The borrowed snapshot viewport supports narrow keyboard and wheel input',
);
assert.match(workspaceViewport, /kind: "keyboard"/);
assert.match(workspaceViewport, /onWheel/);
assert.match(workspaceViewport, /kind: "wheel"/);

assert.deepEqual(
  mergeWorkspaceViewStreams([rdpGatewayStream], [selectableCdpScreencastStream]),
  [rdpGatewayStream, selectableCdpScreencastStream],
  'Workspace view merges service-owned RDP and daemon-owned CDP sources for one browser',
);
assert.deepEqual(
  mergeWorkspaceViewStreams([rdpGatewayStream], [rdpGatewayStream]),
  [rdpGatewayStream],
  'Workspace view does not duplicate the same source when browser projections overlap',
);

assert.equal(
  selectWorkspaceViewStream([selectableCdpScreencastStream, rdpGatewayStream]),
  rdpGatewayStream,
  'Workspace view defaults to the highest-readiness remote control stream',
);
assert.equal(
  selectWorkspaceViewStream(
    [selectableCdpScreencastStream, rdpGatewayStream],
    workspaceViewStreamKey(selectableCdpScreencastStream),
  ),
  selectableCdpScreencastStream,
  'Workspace view honors an explicit remembered stream choice over automatic scoring',
);
assert.equal(
  workspaceViewRecoveryAction({ browserAttachability: null, streamAttachability: null }),
  'service_remote_view_browser_reattach',
  'A retained browser with no reported stream defaults to non-launching browser reattachment',
);
assert.equal(
  workspaceViewRecoveryAction({
    browserAttachability: { recommendedAction: 'service_remote_view_route_switch' },
    streamAttachability: null,
  }),
  'service_remote_view_route_switch',
  'A retained browser uses route switch when service attachability explicitly recommends it',
);

const linkedRdpBrowser = {
  id: 'session:last30days-facebook',
  activeSessionIds: ['last30days-facebook'],
  viewStreams: [rdpGatewayStream],
};
assert.equal(
  serviceBrowserForWorkspaceSelection(
    [linkedRdpBrowser],
    {
      workspaceId: 'daemon-session:last30days-facebook',
      browserId: null,
      sessionId: 'last30days-facebook',
    },
  ),
  linkedRdpBrowser,
);
assert.equal(
  serviceBrowserForWorkspaceSelection(
    [linkedRdpBrowser],
    {
      workspaceId: 'daemon-session:last30days-facebook',
      browserId: null,
      sessionId: null,
    },
  ),
  linkedRdpBrowser,
);

assert.equal(viewStreamLabel(rdpGatewayStream), 'rdp gateway');
assert.equal(controlInputLabel(rdpGatewayStream), 'manual attached desktop');
assert.equal(viewStreamCapabilityLabel(rdpGatewayStream), 'rdp gateway / manual attached desktop');
assert.equal(canEmbedViewStream(rdpGatewayStream), true);
assert.equal(canControlViewStream(rdpGatewayStream), true);
assert.equal(canOpenViewStream(rdpGatewayStream), true);
assert.equal(canOpenControlViewStream(rdpGatewayStream), true);
assert.equal(viewStreamOpenTitle(rdpGatewayStream), 'Open rdp gateway in the dashboard.');
assert.equal(viewStreamControlTitle(rdpGatewayStream), 'Focus the browser and open manual attached desktop control.');
assert.equal(viewStreamRouteLabel(rdpGatewayStream), 'route-a');
assert.equal(viewStreamLeaseLabel(rdpGatewayStream), '2 viewers, controller leased');
assert.equal(viewStreamReadinessLabel(rdpGatewayStream), 'ready');
assert.equal(
  viewStreamDashboardFrameUrl(rdpGatewayStream, 'http://127.0.0.1:4848/workspace'),
  'http://127.0.0.1:8080/guacamole/#/client/route-a',
);
assert.equal(
  viewStreamDashboardFrameUrl(rdpGatewayStream, 'https://agent-browser.example.test/workspace'),
  'https://agent-browser.example.test/guacamole/#/client/route-a',
);
assert.equal(
  viewStreamDashboardFrameUrl({
    provider: 'rdp_gateway',
    frameUrl: 'http://127.0.0.1:8092/guacamole/#/client/route-b',
    externalUrl: 'https://configured.example.test/guacamole/#/client/route-b',
  }, 'https://operator.example.test/workspace'),
  'https://configured.example.test/guacamole/#/client/route-b',
);
assert.equal(
  viewStreamDashboardFrameUrl({
    provider: 'rdp_gateway',
    frameUrl: 'https://private.example.test/guacamole/#/client/route-c',
  }, 'https://operator.example.test/workspace'),
  'https://private.example.test/guacamole/#/client/route-c',
);
assert.equal(
  viewStreamRouteSummary(rdpGatewayStream),
  'route-a / display display-a / simultaneous view / 2 viewers, controller leased / ready',
);

assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
  }),
  'connected',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    recoveredStaleTarget: true,
  }),
  'stale_target_recovered',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
  }),
  'takeover_ready',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'taken-over',
  }),
  'taken_over',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
    takeoverPending: true,
  }),
  'reconnecting',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'process_exited',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
  }),
  'browser_unavailable',
);
assert.equal(workspaceViewportUxStateLabel('stale_target_recovered'), 'stale target recovered');
assert.equal(workspaceViewportReadinessStatusLabel('action_required'), 'action required');

assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'cdp_disconnected',
    hasStream: false,
    canEmbed: false,
    canControl: false,
    mode: 'control',
    preflightStatus: 'idle',
  }),
  'browser_unavailable',
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'cdp_disconnected',
    hasStream: false,
    canEmbed: false,
    canControl: false,
    mode: 'control',
    preflightStatus: 'idle',
  }),
  {
    component: 'browser',
    status: 'blocked',
    evidence: 'browser health is cdp_disconnected',
    nextAction: 'relaunch_browser',
    title: 'Browser unavailable',
    recoveryCopy: 'The selected browser process or CDP endpoint is unhealthy. Relaunch the browser or inspect browser health before opening the remote desktop stream.',
  },
);

assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }),
  {
    component: 'rdp_gateway',
    status: 'ready',
    evidence: 'stream URL is present and preflight is ready',
    nextAction: 'none',
    title: 'Stream ready',
    recoveryCopy: 'The selected browser and remote stream are ready.',
  },
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'login-required',
    preflightMessage: 'The remote stream rejected the current dashboard session.',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }).nextAction,
  'sign_in_again',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }).nextAction,
  'take_over',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: {
      components: [
        {
          component: 'guacamole_connection',
          status: 'failed',
          evidence: 'connection missing',
          nextAction: 'inspect_readiness',
          recovery: 'Create or grant the Guacamole connection before opening the workspace stream.',
        },
      ],
    },
  }).component,
  'guacamole_connection',
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: {
      component: 'remote_view_open_visible_window',
      state: 'terminal_only_route',
      evidence: 'route display :10 contains only xterm and no browser window',
      nextAction: 'inspect_readiness',
      recovery: 'Open the browser on the selected route display before treating this route as ready.',
    },
  }),
  {
    component: 'remote_view_open_visible_window',
    status: 'blocked',
    evidence: 'route display :10 contains only xterm and no browser window',
    nextAction: 'inspect_readiness',
    title: 'remote view open visible window readiness failed',
    recoveryCopy: 'Open the browser on the selected route display before treating this route as ready.',
  },
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'blocked',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: {
      components: [
        {
          component: 'privileged_helper_status',
          status: 'blocked',
          evidence: 'installed remote-view helper does not report the current route desktop and display-access capability contract',
          nextAction: 'install_privileged_helper',
        },
      ],
    },
  }),
  {
    component: 'privileged_helper_status',
    status: 'blocked',
    evidence: 'installed remote-view helper does not report the current route desktop and display-access capability contract',
    nextAction: 'refresh_remote_view_helper',
    title: 'privileged helper status readiness failed',
    recoveryCopy: 'Refresh the installed remote-view privileged helper from an interactive terminal, then rerun route preflight before opening the workspace stream.',
  },
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running after a later focus succeeded',
        nextAction: 'inspect_readiness',
      },
    ],
  }).status,
  'ready',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'checking',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running before stream readiness is proven',
        nextAction: 'inspect_readiness',
      },
    ],
  }).status,
  'action_required',
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    recoveredStaleTarget: true,
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running after a later focus succeeded',
        nextAction: 'inspect_readiness',
      },
    ],
  }),
  {
    component: 'selected_target',
    status: 'ready',
    evidence: 'retained target identity was stale and a live tab was selected',
    nextAction: 'none',
    title: 'Recovered stale selected tab identity',
    recoveryCopy: 'The retained target identity was stale, but Agent Browser selected a current live tab before opening the workspace viewport.',
  },
);
assert.deepEqual(
  compactWorkspaceViewportReadinessComponents({
    components: [{ component: 'public_ingress', status: 'failed', evidence: 'timeout' }],
  }),
  [{ component: 'public_ingress', status: 'failed', evidence: 'timeout', nextAction: null, recovery: null, message: null }],
);

assert.equal(
  canEmbedViewStream({
    provider: 'rdp_gateway',
    url: null,
  }),
  false,
);
assert.equal(
  canEmbedViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:8080/cdp/session',
  }),
  true,
);
const cdpScreencastStream = {
  provider: 'cdp_screencast',
  controlInput: 'cdp_input',
  url: 'http://127.0.0.1:44841/',
  frameUrl: 'http://127.0.0.1:44841/',
  readiness: { state: 'ready', reason: 'stream_server_ready' },
  readOnly: false,
};
assert.equal(canEmbedViewStream(cdpScreencastStream), true);
assert.equal(canOpenControlViewStream(cdpScreencastStream), true);
assert.equal(viewStreamOpenTitle(cdpScreencastStream), 'Open cdp screencast in the dashboard.');
assert.equal(viewStreamControlTitle(cdpScreencastStream), 'Focus the browser and open cdp input control.');
assert.equal(
  canOpenViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:44841/',
  }),
  true,
);
assert.equal(
  canOpenViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:44841/',
    readiness: { state: 'unknown' },
  }),
  true,
);
assert.equal(
  canOpenViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:44841/',
    readiness: { state: 'probing' },
  }),
  true,
);
for (const state of ['unreachable', 'auth_expired', 'stale_target', 'invalid_payload', 'unsupported_provider']) {
  const blockedStream = {
    provider: 'cdp_screencast',
    controlInput: 'cdp_input',
    url: 'http://127.0.0.1:44841/',
    readiness: { state, reason: `${state}_reason` },
  };
  assert.equal(canEmbedViewStream(blockedStream), true);
  assert.equal(canOpenViewStream(blockedStream), false);
  assert.equal(canOpenControlViewStream(blockedStream), false);
  assert.equal(viewStreamOpenTitle(blockedStream), `cdp screencast is unavailable: ${state.replaceAll('_', ' ')} reason.`);
}
assert.equal(
  canOpenViewStream({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/session',
    remoteReadiness: {
      components: [
        { component: 'proxy', state: 'ready' },
        { component: 'frame', state: 'invalid_payload', reason: 'empty backend response' },
      ],
    },
  }),
  false,
);
assert.equal(
  canOpenViewStream({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/terminal-only',
    remoteReadiness: { state: 'route_bound_terminal_only', reason: 'Remote route display is terminal-only.' },
  }),
  false,
);
assert.equal(
  canOpenViewStream({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/display-content-terminal',
    remoteReadiness: { state: 'ready' },
    displayContent: { state: 'terminal_only' },
  }),
  false,
);
assert.equal(
  viewStreamOpenTitle({
    provider: 'cdp_screencast',
    url: null,
    readiness: { state: 'unavailable', reason: 'missing_stream_server' },
    readOnly: true,
  }),
  'cdp screencast is unavailable: missing stream server.',
);
assert.equal(
  canControlViewStream({
    provider: 'rdp_gateway',
    readOnly: true,
    controlInput: 'manual_attached_desktop',
  }),
  false,
);
assert.equal(
  canOpenControlViewStream({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/session',
    readOnly: true,
    controlInput: 'manual_attached_desktop',
  }),
  false,
);
assert.equal(
  viewStreamControlTitle({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/session',
    readOnly: true,
  }),
  'The service marked this stream as view-only or did not report a control input provider.',
);
assert.equal(controlInputLabel({ readOnly: true }), 'view only');
assert.equal(viewStreamLabel({}), 'view stream');

assert.match(
  dashboardPage,
  /import \{ WorkspaceRemoteViewport \} from "@\/components\/workspace-remote-viewport";[\s\S]*<WorkspaceRemoteViewport fallback=\{<Viewport \/>\} selectedWorkspaceContext=\{selectedWorkspace\.context\} \/>/,
  'Dashboard viewport route must render the workspace remote viewport wrapper with selected workspace context before falling back to CDP screencast',
);

assert.match(
  dashboardPage,
  /readWorkspaceViewportRoute[\s\S]*view === "workspace:tile"[\s\S]*DASHBOARD_WORKSPACE_SELECTION_EVENT[\s\S]*!hasSessions && activeSection !== "service" && !hasWorkspaceViewportRoute/,
  'Dashboard empty state must yield to service-owned workspace viewport URLs even when no daemon sessions are active',
);

assert.match(
  workspaceNavigator,
  /function pushWorkspaceViewportUrl\(node: WorkspaceNode, mode: "view" \| "control"\)[\s\S]*url\.pathname = "\/"[\s\S]*url\.searchParams\.set\("view", `workspace:\$\{mode\}`\)[\s\S]*DASHBOARD_WORKSPACE_QUERY_KEYS[\s\S]*new PopStateEvent\("popstate"/,
  'Workspace navigator View and Control actions must push a stable workspace viewport URL and notify route listeners',
);

assert.match(
  workspaceNavigator,
  /function pushWorkspaceTileUrl[\s\S]*url\.searchParams\.set\("view", "workspace:tile"\)[\s\S]*DASHBOARD_WORKSPACE_QUERY_KEYS[\s\S]*aria-label="Open tiled workspace view"/,
  'Workspace navigator must expose a tiled remote workspace route that does not depend on one selected browser',
);

assert.match(
  workspaceNavigator,
  /action\.id === "control" && node\.viewStream\?\.controllable[\s\S]*pushWorkspaceViewportUrl\(node, "control"\)[\s\S]*action\.id === "view" && node\.viewStream\?\.embeddable[\s\S]*pushWorkspaceViewportUrl\(node, "view"\)/,
  'Workspace navigator primary View and Control actions must open the dashboard-owned workspace viewport',
);

assert.match(
  workspaceViewport,
  /view === "workspace:control"[\s\S]*view === "workspace:view"[\s\S]*daemonSessionFromSelection[\s\S]*daemonBrowserFromSession[\s\S]*primaryViewStream[\s\S]*chooseWorkspaceViewportBrowser[\s\S]*isBlankWorkspaceViewportTab[\s\S]*workspaceViewportTabScore[\s\S]*daemonSessionNameForBrowser[\s\S]*const serviceBrowser = viewportSelection[\s\S]*serviceBrowserForWorkspaceSelection[\s\S]*const browser = chooseWorkspaceViewportBrowser\(serviceBrowser, selectedContextBrowser \?\? daemonBrowser\)[\s\S]*const params = targetId[\s\S]*sessionName[\s\S]*action: "view_focus"[\s\S]*taskName: "workspace-viewport-control"[\s\S]*params,/,
  'Workspace remote viewport must restore URL selection, resolve daemon-session URLs back to linked service browsers, synthesize fallback daemon streams, choose a live non-blank service-owned target, and queue view_focus before control embedding',
);

assert.match(
  workspaceViewport,
  /function chooseWorkspaceViewportBrowser[\s\S]*workspaceViewportBrowsersShareSession\(serviceBrowser, daemonBrowser\)[\s\S]*mergeWorkspaceViewStreams\(serviceBrowser\.viewStreams, daemonBrowser\.viewStreams\)[\s\S]*hasOpenWorkspaceViewportStream\(serviceBrowser\)[\s\S]*return serviceBrowser[\s\S]*hasOpenWorkspaceViewportStream\(daemonBrowser\)[\s\S]*return daemonBrowser/,
  'Workspace remote viewport must merge RDP and CDP projections for one session, then preserve the openable fallback behavior for distinct browsers',
);

assert.match(
  workspaceViewport,
  /view === "workspace:tile"[\s\S]*workspaceViewportTiles[\s\S]*tileStreams = viewportSelection\?\.mode === "tile"/,
  'Workspace remote viewport must derive tile mode from the URL and service-owned route state',
);
assert.match(
  workspaceViewport,
  /workspace-remote-viewport-tile-grid[\s\S]*tileStreams\.map\(\(tile\)[\s\S]*workspace-remote-viewport-tile-card[\s\S]*tile\.sharedRoute[\s\S]*shared route[\s\S]*<iframe/,
  'Workspace remote viewport must render a tiled view with two service-owned remote routes and visible shared-route warnings',
);

assert.match(
  workspaceViewport,
  /recoveredFromStaleSelection[\s\S]*deriveWorkspaceViewportUxState[\s\S]*Recovered stale selected tab identity/,
  'Workspace remote viewport must expose the Slice A UX state vocabulary and recover stale retained tab identity as a state, not as browser failure',
);
assert.match(
  workspaceViewport,
  /selectedIsLive[\s\S]*selectedIsBlank[\s\S]*selectedFocusable = selected && selectedIsLive \? selected : undefined[\s\S]*recoveredFromStaleSelection: Boolean\(selectedWasStale && \(selectedIsBlank \|\| tab\.id !== selected\?\.id\)\)/,
  'Workspace remote viewport must honor an explicitly selected live blank tab while still marking it as recovered stale selection evidence',
);
assert.match(
  workspaceViewport,
  /tabSelection\.recoveredFromStaleSelection[\s\S]*if \(viewportSelection\.selection\.tabId === tabSelection\.tab\.id\) return[\s\S]*tabId: tabSelection\.tab\.id[\s\S]*writeDashboardWorkspaceUrlSelection\(nextSelection, "replace"\)[\s\S]*mode: viewportSelection\.mode[\s\S]*Recovered stale selected tab identity/,
  'Workspace remote viewport must replace missing or dead stale tab URL selections with the current live tab before rendering control mode',
);
assert.match(
  workspaceViewport,
  /rows\.find\(\(tab\) => tab\.id === selection\.tabId \|\| tab\.targetId === selection\.tabId \|\| \(tab\.targetId \? `target:\$\{tab\.targetId\}` === selection\.tabId : false\)\)[\s\S]*selectedWasStale = Boolean\(selection\.tabId && \(!selected \|\| !selectedIsLive \|\| selectedIsBlank\)\)/,
  'Workspace remote viewport must treat missing, dead, or blank target-shaped tab URL selections as stale recovery evidence',
);
assert.match(
  workspaceViewport,
  /deriveWorkspaceViewportReadiness[\s\S]*streamReadiness: stream\?\.remoteReadiness \?\? stream\?\.readiness[\s\S]*data-readiness-status=\{viewportReadiness\.status\}[\s\S]*viewStreamRouteSummary\(stream\)[\s\S]*viewportReadiness\.recoveryCopy/,
  'Workspace remote viewport must derive compact readiness and render actionable recovery copy for auth, provider, browser, viewer, and retained-job states',
);
assert.match(
  workspaceViewport,
  /data-ux-state=\{viewportUxState\}/,
  'Workspace remote viewport must expose the derived UX state on the viewport shell',
);
assert.match(
  workspaceViewport,
  /workspaceViewportUxStateLabel\(viewportUxState\)/,
  'Workspace remote viewport must render the service-derived UX state vocabulary',
);

assert.match(
  workspaceViewport,
  /function resolveWorkspaceStreamUrl[\s\S]*viewStreamExternalUrl\(stream\)[\s\S]*viewStreamDashboardFrameUrl\(stream, dashboardHref\)[\s\S]*new URL\(streamUrl, window\.location\.href\)\.toString\(\)[\s\S]*resolved\.origin === window\.location\.origin[\s\S]*dispatchViewportController\(\{ type: "preflight_succeeded", targetToken: preflightTargetToken \}\)/,
  'Workspace remote viewport must resolve service-owned frame and external stream URLs with hosted-dashboard loopback protection and allow cross-origin iframe rendering instead of treating CORS preflight failure as stream unavailability',
);
assert.match(
  workspaceViewport,
  /function detectWorkspaceFrameFailure[\s\S]*catch \{\s*return null;\s*\}[\s\S]*return null;/,
  'Workspace remote viewport must not classify cross-origin Guacamole frame inspection limits as browser-error failures',
);

assert.match(
  workspaceViewport,
  /WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH[\s\S]*process_exited[\s\S]*function browserCanRenderWorkspaceViewport[\s\S]*!WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH\.has\(health\)[\s\S]*const canEmbed = stream \? canOpenViewStream\(stream\)[\s\S]*const canRenderSelectedBrowser = browserCanRenderWorkspaceViewport\(browser\)[\s\S]*const canRenderCdpStream = canRenderSelectedBrowser[\s\S]*const canRenderFrame = canRenderSelectedBrowser/,
  'Workspace remote viewport must not embed retained, terminal, or readiness-blocked streams',
);

assert.match(
  workspaceViewport,
  /function workspaceViewportTiles[\s\S]*!browserCanRenderWorkspaceViewport\(browser\) && !browserCanRecoverWorkspaceViewport\(browser\)[\s\S]*frameUrl: browserCanRenderWorkspaceViewport\(browser\) && stream && frameUrl && canOpenViewStream\(stream\) \? frameUrl : null/,
  'Workspace tile mode must keep recoverable terminal cards visible without embedding their stale Guacamole URLs',
);

assert.doesNotMatch(
  workspaceViewport,
  /params: \{ index: tabIndex, maximize: true \}/,
  'Workspace remote viewport must not rely only on retained tab indexes when target IDs are available',
);

assert.match(
  workspaceViewport,
  /installGuacamoleTouchClickBridge[\s\S]*sendMouse\(touch, true\)[\s\S]*sendMouse\(touch, false\)[\s\S]*<iframe[\s\S]*ref=\{viewportFrameRef\}[\s\S]*className="workspace-remote-viewport-frame"[\s\S]*allow="clipboard-read; clipboard-write; fullscreen; pointer-lock"/,
  'Workspace remote viewport must embed service-owned streams behind dashboard chrome with input capabilities enabled',
);

assert.match(
  workspaceViewport,
  /function isCdpScreencastStream[\s\S]*provider\?\.trim\(\)\.toLowerCase\(\) === "cdp_screencast"[\s\S]*function workspaceCdpWebSocketUrl[\s\S]*\/api\/stream\/\$\{encodeURIComponent\(resolved\.port\)\}[\s\S]*resolved\.protocol = resolved\.protocol === "https:" \? "wss:" : "ws:"[\s\S]*function WorkspaceCdpStreamCanvas[\s\S]*new WebSocket\(websocketUrl\)[\s\S]*case "frame":[\s\S]*drawFrame\(msg\.data\)[\s\S]*type: "input_mouse"[\s\S]*type: "input_keyboard"/,
  'Workspace remote viewport must render CDP screencast streams through a native WebSocket canvas instead of iframing the stream server HTTP root',
);

assert.match(
  workspaceViewport,
  /const canRenderCdpStream = canRenderSelectedBrowser && isCdpScreencastStream\(stream\)[\s\S]*const canRenderFrame = canRenderSelectedBrowser && !isCdpScreencastStream\(stream\)[\s\S]*<WorkspaceCdpStreamCanvas[\s\S]*: stream && canRenderFrame \? \(/,
  'Workspace remote viewport must route CDP screencasts to the native canvas before considering the iframe path',
);

assert.match(
  workspaceViewport,
  /const viewportTargetToken = workspaceViewportTargetToken\(viewportTarget\);[\s\S]*const streamPreflight: WorkspaceViewportPreflightState =[\s\S]*viewportController\.targetToken === viewportTargetToken[\s\S]*\? viewportController\.preflight[\s\S]*: \{ status: "idle", message: "" \};[\s\S]*const canRenderCdpStream = canRenderSelectedBrowser[\s\S]*streamPreflight\.status === "ready"[\s\S]*const canRenderFrame = canRenderSelectedBrowser[\s\S]*streamPreflight\.status === "ready"/,
  'Workspace remote viewport must not reuse a previous target ready preflight state while the selected target token is changing',
);

assert.match(
  css,
  /\.workspace-cdp-stream[\s\S]*\.workspace-cdp-stream-canvas[\s\S]*\.workspace-cdp-stream-footer/,
  'Workspace remote viewport must style the native CDP canvas so stream readiness is visible without embedding the dashboard login shell',
);

assert.match(
  workspaceViewport,
  /function openGuacamoleInteractionSettings[\s\S]*#guac-menu[\s\S]*scope\.menu!\.shown = true[\s\S]*#keyboard-settings[\s\S]*#mouse-settings[\s\S]*aria-label="Open Guacamole interaction settings"/,
  'Workspace remote viewport must expose a control that opens Guacamole keyboard and mouse interaction settings',
);

assert.match(
  workspaceViewport,
  /remote-disconnected[\s\S]*you have been disconnected[\s\S]*Another dashboard or Guacamole popout is using this remote desktop[\s\S]*Take over/,
  'Workspace remote viewport must identify Guacamole single-viewer disconnects and expose a takeover action',
);

assert.match(
  workspaceViewport,
  /requestWorkspaceTakeover[\s\S]*browserId: browser\.id[\s\S]*streamId: stream\.id[\s\S]*openMode[\s\S]*action: "view_takeover"[\s\S]*taskName: "workspace-viewport-takeover"[\s\S]*setStreamRefreshNonce\(Date\.now\(\)\)/,
  'Workspace remote viewport Take over must queue a service-owned view_takeover request and reconnect the iframe',
);

assert.match(
  workspaceViewport,
  /postWorkspaceRecoveryRequest[\s\S]*action: ServiceRequestAction[\s\S]*workspaceViewRecoveryAction[\s\S]*workspace-viewport-route-switch[\s\S]*workspace-viewport-browser-reattach[\s\S]*service_viewer_lease_request[\s\S]*workspace-viewport-viewer-reconnect[\s\S]*service_controller_lease_takeover[\s\S]*workspace-viewport-controller-takeover[\s\S]*service_viewer_lease_release[\s\S]*workspace-viewport-viewer-release/,
  'Workspace remote viewport must expose explicit browser reattach, route switch, viewer reconnect, controller takeover, and viewer release recovery actions',
);

assert.match(
  workspaceViewport,
  /aria-label="Reattach remote browser route"[\s\S]*aria-label="Reconnect viewer lease"[\s\S]*aria-label="Take controller lease"[\s\S]*aria-label="Release viewer leases"/,
  'Workspace remote viewport recovery actions must be visible as stable icon-button controls',
);

assert.match(
  workspaceViewport,
  /workspaceViewStreamChoices\(browser\?\.viewStreams\)[\s\S]*workspaceViewStreamKey\(option[\s\S]*aria-pressed=\{selected\}[\s\S]*Use \{viewStreamLabel\(option\)\}/,
  'Workspace remote viewport must expose every reported stream as an explicit operator-selectable source',
);

assert.match(
  workspaceViewport,
  /captureForeignCdpScreenshot[\s\S]*fetchForeignCdpScreenshot[\s\S]*format: "png"[\s\S]*download = document\.createElement\("a"\)[\s\S]*download\.click\(\)[\s\S]*aria-label="Capture foreign browser screenshot"[\s\S]*Capture PNG/,
  'Foreign CDP viewport must expose a target-specific PNG download instead of treating Screenshot as selection only',
);

assert.match(
  workspaceViewport,
  /function WorkspaceCdpSnapshotViewer[\s\S]*window\.setInterval\(fetchSnapshot, 750\)[\s\S]*aria-live="polite"[\s\S]*Foreign CDP watch live/,
  'Foreign CDP viewport must present snapshot polling as an obvious responsive live watch feed',
);

assert.match(
  workspaceViewport,
  /recoverWorkspaceBrowser[\s\S]*if \(!targetBrowser\) return[\s\S]*workspaceViewRecoveryAction[\s\S]*!stream && browser[\s\S]*Wake stream/,
  'A selected retained browser with no stream must expose a non-launching wake action',
);

assert.match(
  workspaceViewport,
  /type WorkspaceViewportTile = \{[\s\S]*stream: ServiceViewStream \| null[\s\S]*function workspaceViewportTiles[\s\S]*tile\.stream && tileFrameUrl[\s\S]*recoverWorkspaceBrowser\(tile\.browser, tile\.stream\)[\s\S]*Wake stream/,
  'Tile mode must retain recoverable browser cards without a usable stream and expose wake-up in place',
);

assert.match(
  workspaceViewport,
  /openWorkspaceStreamExternally[\s\S]*const accepted = await requestWorkspaceTakeover\("external"\)[\s\S]*if \(!accepted\) return[\s\S]*window\.open\(externalStreamUrl, "_blank", "noopener,noreferrer"\)/,
  'Workspace remote viewport external open must await service-owned takeover acceptance before opening the external route',
);

assert.doesNotMatch(
  workspaceViewport,
  /onClick=\{refreshWorkspaceViewport\}[\s\S]*Take over/,
  'Workspace remote viewport Take over must not be a local iframe-only refresh',
);

assert.doesNotMatch(
  workspaceViewport,
  /sandbox=/,
  'Workspace remote viewport must not sandbox first-party Guacamole streams because that breaks operator input capture',
);

assert.match(
  workspaceViewport,
  /if \(state\.httpFallback\)[\s\S]*wsRef\.current\?\.close\(\)[\s\S]*return/,
  'HTTPS frame fallback must stop the failing WebSocket reconnect loop',
);

assert.match(
  workspaceViewport,
  /const poll = async \(\)[\s\S]*AbortController[\s\S]*window\.setTimeout\(\(\) => void poll\(\), 900\)[\s\S]*controller\?\.abort\(\)/,
  'HTTPS frame fallback must serialize polls and abort the active request during cleanup',
);

assert.doesNotMatch(
  workspaceViewport,
  /setInterval\(poll, 900\)/,
  'HTTPS frame fallback must not accumulate overlapping frame requests',
);

assert.match(
  workspaceViewport,
  /readWorkspaceApiResponse[\s\S]*await response\.text\(\)[\s\S]*response\.statusText[\s\S]*postWorkspaceRecoveryRequest[\s\S]*readWorkspaceApiResponse/,
  'Workspace recovery must turn a non-JSON gateway response into a readable HTTP error',
);

assert.match(
  rdpAutologinSetup,
  /INSERT INTO guacamole_connection_permission[\s\S]*SELECT entity_id, \{connection_id\}, 'READ'::guacamole_object_permission_type[\s\S]*WHERE type = 'USER'[\s\S]*ON CONFLICT DO NOTHING/,
  'XRDP autologin setup must grant current Guacamole users READ on the configured remote desktop connection',
);

assert.match(
  css,
  /\.workspace-remote-viewport[\s\S]*grid-template-rows: auto auto minmax\(0, 1fr\)[\s\S]*\.workspace-remote-viewport-stage[\s\S]*min-height: 0[\s\S]*touch-action: none[\s\S]*\.workspace-remote-viewport-frame[\s\S]*height: 100%[\s\S]*touch-action: none[\s\S]*\.workspace-remote-viewport-tile-grid[\s\S]*grid-template-columns: repeat\(2, minmax\(0, 1fr\)\)[\s\S]*\.workspace-remote-viewport-tile-stage/,
  'Workspace remote viewport CSS must keep compact chrome and a stable iframe stage',
);

assert.match(
  css,
  /\.service-view-stream-route-strip[\s\S]*grid-template-columns: repeat\(auto-fit, minmax\(7rem, 1fr\)\)/,
  'Service stream cards must render route metadata in stable responsive columns',
);

console.log('Dashboard view stream contract smoke passed');
