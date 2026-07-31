#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  selectRouteOwnedStream,
  serviceEvaluateValue,
} from './lib/remote-view-live-state.js';

const routeId = 'guacamole:1';
const currentBrowserId = 'session:current';
const staleStream = {
  id: 'stale-stream',
  routeId,
  remoteReadiness: { state: 'released' },
};
const currentStream = {
  id: 'current-stream',
  routeId,
  remoteReadiness: { state: 'ready' },
};

const duplicateRouteState = {
  browsers: {
    'session:stale': {
      id: 'session:stale',
      viewStreams: [staleStream],
    },
    [currentBrowserId]: {
      id: currentBrowserId,
      viewStreams: [currentStream],
    },
  },
};

assert.equal(
  selectRouteOwnedStream(duplicateRouteState, routeId, currentBrowserId),
  currentStream,
  'a stale browser stream with the same route ID must not outrank the route-owned browser stream',
);

const attributedTopLevelStream = {
  browserId: currentBrowserId,
  id: 'attributed-top-level-stream',
  routeId,
};
assert.equal(
  selectRouteOwnedStream({
    browsers: { [currentBrowserId]: { id: currentBrowserId, viewStreams: [] } },
    viewStreams: { current: attributedTopLevelStream },
  }, routeId, currentBrowserId),
  attributedTopLevelStream,
  'an explicitly browser-attributed top-level stream remains a valid fallback',
);

assert.equal(
  selectRouteOwnedStream({
    browsers: { [currentBrowserId]: { id: currentBrowserId, viewStreams: [] } },
    viewStreams: {
      stale: staleStream,
      foreign: { ...currentStream, browserId: 'session:foreign' },
    },
  }, routeId, currentBrowserId),
  null,
  'unattributed and differently owned streams must not satisfy route-owned evidence',
);

const evaluatedPage = { url: 'https://www.linkedin.com/', title: 'LinkedIn' };
assert.deepEqual(
  serviceEvaluateValue({ data: { result: { result: { value: evaluatedPage } } } }),
  evaluatedPage,
  'current nested CDP evaluate responses must expose their by-value result',
);
assert.deepEqual(
  serviceEvaluateValue({ data: { result: { value: evaluatedPage } } }),
  evaluatedPage,
  'compatibility evaluate responses must expose their by-value result',
);
assert.deepEqual(
  serviceEvaluateValue({ data: { result: evaluatedPage } }),
  evaluatedPage,
  'already normalized evaluate responses must remain usable',
);

console.log('Remote-view live-state attribution tests passed');
