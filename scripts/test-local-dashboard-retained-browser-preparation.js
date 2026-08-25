#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  buildRetainedBrowserPinArgs,
  buildRetainedBrowserRemoteViewArgs,
  normalizeRetainedBrowserPreparationRequest,
  verifyRetainedBrowserRemoteViewResult,
} from './lib/local-dashboard-retained-browser-preparation.js';

const request = normalizeRetainedBrowserPreparationRequest({
  url: 'https://chatgpt.com/g/g-p-workshop/c/conversation-id',
  urlPrefix: 'https://chatgpt.com/g/g-p-workshop/',
  runtimeProfile: 'chatgpt-pro',
});
const remoteArgs = buildRetainedBrowserRemoteViewArgs(request);
assert.deepEqual(remoteArgs.slice(0, 6), [
  '--json',
  '--session',
  'chatgpt-pro',
  'remote-view',
  'open',
  request.url,
]);
for (const forbidden of ['click', 'type', 'fill', 'evaluate', 'send', 'submit']) {
  assert.equal(remoteArgs.includes(forbidden), false);
}
assert.deepEqual(buildRetainedBrowserPinArgs(request), [
  '--write-retained-requirement',
  '--discover-retained-exact-url',
  request.url,
  '--discover-retained-profile',
  'chatgpt-pro',
  '--json',
]);

const payload = {
  success: true,
  data: {
    status: 'opened',
    dryRun: false,
    browserId: 'session:workshop',
    sessionName: 'chatgpt-pro',
    handoffUrl: 'https://desktop.example.test/remote-view/r1',
    intent: { url: request.url, runtimeProfile: 'chatgpt-pro' },
    operatorVisible: {
      state: 'ready',
      target: {
        expectedUrl: request.url,
        profileId: 'chatgpt-pro',
        state: 'ready',
        targetId: 'target-workshop',
        url: request.url,
        urlReadiness: 'ready',
      },
    },
    routeBoundHandoff: {
      profile: { id: 'chatgpt-pro', runtimeProfile: 'chatgpt-pro' },
    },
    sharedAcquisition: {
      browserId: 'session:workshop',
      profileId: 'chatgpt-pro',
      sessionName: 'chatgpt-pro',
    },
  },
};
assert.equal(verifyRetainedBrowserRemoteViewResult(payload, request).targetId, 'target-workshop');

await assert.rejects(
  async () => normalizeRetainedBrowserPreparationRequest({
    ...request,
    url: 'https://chatgpt.com/c/wrong-project',
  }),
  /retained_browser_preparation_url_outside_prefix/,
);
for (const changed of [
  { operatorVisible: { ...payload.data.operatorVisible, state: 'wrong_tab' } },
  { operatorVisible: { ...payload.data.operatorVisible, target: { ...payload.data.operatorVisible.target, url: 'https://chatgpt.com/c/wrong' } } },
  { routeBoundHandoff: { profile: { id: 'wrong-profile', runtimeProfile: 'wrong-profile' } } },
  { sharedAcquisition: { ...payload.data.sharedAcquisition, sessionName: 'wrong-session' } },
]) {
  assert.throws(
    () => verifyRetainedBrowserRemoteViewResult({
      ...payload,
      data: { ...payload.data, ...changed },
    }, request),
    /retained_browser_preparation_/,
  );
}

console.log('Local dashboard retained browser preparation fixture passed');
