#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  discoverRetainedBrowserExpectation,
  retainedUrlMatchesExact,
  retainedUrlMatchesPrefix,
} from './lib/local-dashboard-retained-browser-discovery.js';

const prefix = 'https://chatgpt.com/g/g-p-workshop/';
const browser = {
  id: 'session:workshop-retained',
  pid: 4242,
  profileId: 'chatgpt-pro',
  health: 'ready',
  cdpEndpoint: 'ws://127.0.0.1:9444/devtools/browser/workshop',
};
const target = {
  id: 'target-workshop',
  type: 'page',
  url: 'https://chatgpt.com/g/g-p-workshop/c/conversation-id',
};

assert.equal(retainedUrlMatchesPrefix(target.url, prefix), true);
assert.equal(
  retainedUrlMatchesPrefix('https://chatgpt.com/g/g-p-workshop-other/c/wrong', prefix),
  false,
);
assert.equal(
  retainedUrlMatchesPrefix('https://chatgpt.com.evil.test/g/g-p-workshop/c/wrong', prefix),
  false,
);
assert.equal(retainedUrlMatchesExact(target.url, target.url), true);
assert.equal(
  retainedUrlMatchesExact(`${target.url}/wrong`, target.url),
  false,
);

const adapters = ({ browsers = { 'workshop-retained': browser }, targets = [target] } = {}) => ({
  urlPrefix: prefix,
  sessionNames: Object.keys(browsers),
  readDaemonPid: () => 31337,
  isProcessLive: (pid) => pid === 31337 || pid === 4242,
  readBrowser: async (sessionName) => ({ success: true, browser: browsers[sessionName] }),
  readCdpTargets: async () => targets,
});

const exact = await discoverRetainedBrowserExpectation(adapters());
assert.deepEqual(exact.expectation, {
  sessionName: 'workshop-retained',
  browserId: 'session:workshop-retained',
  browserPid: 4242,
  cdpUrl: browser.cdpEndpoint,
  profileId: 'chatgpt-pro',
  targetId: 'target-workshop',
  url: target.url,
});
assert.equal(exact.matchedCandidateCount, 1);

const exactUrl = await discoverRetainedBrowserExpectation({
  ...adapters(),
  urlPrefix: undefined,
  exactUrl: target.url,
  profileId: 'chatgpt-pro',
});
assert.equal(exactUrl.exactUrl, target.url);
assert.equal(exactUrl.urlPrefix, null);
assert.equal(exactUrl.profileId, 'chatgpt-pro');

await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    urlPrefix: undefined,
    exactUrl: target.url,
    profileId: 'wrong-profile',
  }),
  /retained_browser_discovery_profile_mismatch/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    exactUrl: target.url,
  }),
  /retained_browser_discovery_selector_invalid/,
);

await assert.rejects(
  discoverRetainedBrowserExpectation(adapters({ targets: [] })),
  /retained_browser_discovery_no_match/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation(adapters({
    targets: [target, { ...target, id: 'target-duplicate' }],
  })),
  /retained_browser_discovery_ambiguous/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation(adapters({
    browsers: {
      'workshop-retained': { ...browser, health: 'degraded' },
    },
  })),
  /retained_browser_discovery_no_match/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    readCdpTargets: async () => {
      throw new Error('fixture target read failed');
    },
  }),
  /retained_browser_discovery_cdp_unreadable/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    readBrowser: async () => ({ success: true, browser: { ...browser, profileId: null } }),
  }),
  /retained_browser_discovery_identity_incomplete/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    readBrowser: async () => ({
      success: true,
      browser: { ...browser, id: 'session:wrong-session' },
    }),
  }),
  /retained_browser_discovery_session_mismatch/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    readBrowser: async () => ({
      success: true,
      browser: { ...browser, cdpEndpoint: 'ws://192.0.2.25:9444/devtools/browser/wrong' },
    }),
  }),
  /retained_browser_discovery_cdp_not_loopback/,
);
await assert.rejects(
  discoverRetainedBrowserExpectation({
    ...adapters(),
    urlPrefix: 'https://user@example.test/private',
  }),
  /retained_browser_discovery_prefix_invalid/,
);

console.log('Local dashboard retained browser discovery fixture passed');
