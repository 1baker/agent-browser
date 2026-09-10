#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  evaluateRetainedBrowserExpectation,
  normalizeRetainedBrowserExpectation,
  pinRetainedBrowserExpectation,
} from './lib/local-dashboard-retained-browser-guard.js';

const expectation = {
  sessionName: 'retained-fixture',
  targetId: 'target-fixture',
  url: 'https://example.test/conversation',
};
const browser = {
  id: 'session:retained-fixture',
  pid: 4242,
  cdpEndpoint: 'ws://127.0.0.1:9444/devtools/browser/fixture',
  profileId: 'fixture-profile',
  health: 'ready',
};
const targets = [{
  id: 'target-fixture',
  type: 'page',
  title: 'Fixture conversation',
  url: 'https://example.test/conversation',
}];

const before = evaluateRetainedBrowserExpectation({
  browser,
  cdpTargets: targets,
  expectation,
  stage: 'pre_mutation',
});
assert.equal(before.verified, true);
assert.deepEqual(pinRetainedBrowserExpectation(before), {
  sessionName: 'retained-fixture',
  browserId: 'session:retained-fixture',
  browserPid: 4242,
  cdpUrl: 'ws://127.0.0.1:9444/devtools/browser/fixture',
  profileId: 'fixture-profile',
  targetId: 'target-fixture',
  url: 'https://example.test/conversation',
});

for (const [reason, changedBrowser, changedTargets] of [
  ['retained_browser_missing', null, targets],
  ['retained_browser_pid_changed', { ...browser, pid: 4343 }, targets],
  ['retained_browser_cdp_changed', { ...browser, cdpEndpoint: 'ws://127.0.0.1:9555/devtools/browser/other' }, targets],
  ['retained_browser_profile_changed', { ...browser, profileId: 'other-profile' }, targets],
  ['retained_browser_not_live', { ...browser, health: 'degraded' }, targets],
  ['retained_browser_cdp_unreachable', browser, null],
  ['retained_target_missing', browser, []],
  ['retained_target_url_changed', browser, [{ ...targets[0], url: 'https://example.test/other' }]],
]) {
  const result = evaluateRetainedBrowserExpectation({
    browser: changedBrowser,
    cdpTargets: changedTargets,
    expectation: pinRetainedBrowserExpectation(before),
    stage: 'post_handoff',
  });
  assert.equal(result.verified, false);
  assert.equal(result.reason, reason);
}

const nonLoopbackCdp = 'ws://192.0.2.25:9444/devtools/browser/other';
const nonLoopback = evaluateRetainedBrowserExpectation({
  browser: { ...browser, cdpEndpoint: nonLoopbackCdp },
  cdpTargets: targets,
  expectation: {
    ...pinRetainedBrowserExpectation(before),
    cdpUrl: nonLoopbackCdp,
  },
  stage: 'post_handoff',
});
assert.equal(nonLoopback.verified, false);
assert.equal(nonLoopback.reason, 'retained_browser_cdp_not_loopback');

assert.throws(
  () => normalizeRetainedBrowserExpectation({ targetId: 'missing-session' }),
  /requires sessionName/,
);
assert.throws(
  () => normalizeRetainedBrowserExpectation({ sessionName: '../unsafe' }),
  /Invalid retained browser session name/,
);

console.log('Local dashboard retained browser guard fixture passed');
