#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  isRuntimeHandoffBrowserActive,
  removeVerifiedRuntimeHandoffRecord,
  selectRuntimeHandoffBrowser,
} from './lib/runtime-handoff-browser-selection.js';

const expectedBrowser = {
  sessionName: 'retained',
  browserPid: 202,
  cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/retained',
  runtimeProfile: 'chatgpt-pro',
};
const exactBrowser = {
  id: 'session:retained',
  pid: expectedBrowser.browserPid,
  cdpEndpoint: expectedBrowser.cdpUrl,
  profileId: expectedBrowser.runtimeProfile,
  health: 'ready',
};

assert.deepEqual(
  selectRuntimeHandoffBrowser({
    browsers: [exactBrowser],
    sessionName: expectedBrowser.sessionName,
    expectedBrowser,
  }),
  { browser: exactBrowser, matchKind: 'session', error: null },
);

const bridgeAlias = {
  ...exactBrowser,
  id: 'bridge:retained-browser',
  pid: null,
};
assert.deepEqual(
  selectRuntimeHandoffBrowser({
    browsers: [bridgeAlias],
    sessionName: expectedBrowser.sessionName,
    expectedBrowser,
  }),
  { browser: bridgeAlias, matchKind: 'identity_alias', error: null },
);
assert.equal(
  isRuntimeHandoffBrowserActive({
    browser: bridgeAlias,
    expectedBrowser,
    isProcessLive: (pid) => pid === expectedBrowser.browserPid,
  }),
  true,
);
assert.equal(
  isRuntimeHandoffBrowserActive({
    browser: exactBrowser,
    expectedBrowser,
    isProcessLive: () => false,
  }),
  false,
  'a known stale PID must not become active from a leftover CDP string',
);
assert.equal(
  isRuntimeHandoffBrowserActive({
    browser: { ...exactBrowser, health: 'closed' },
    expectedBrowser,
    isProcessLive: () => true,
  }),
  false,
);

for (const changedBrowser of [
  { ...exactBrowser, pid: 303 },
  { ...exactBrowser, cdpEndpoint: 'ws://127.0.0.1:9333/devtools/browser/wrong' },
]) {
  const selected = selectRuntimeHandoffBrowser({
    browsers: [changedBrowser],
    sessionName: expectedBrowser.sessionName,
    expectedBrowser,
  });
  assert.equal(selected.browser, null);
  assert.match(selected.error, /does not match the prepared handoff identity/);
}

const ambiguous = selectRuntimeHandoffBrowser({
  browsers: [bridgeAlias, { ...bridgeAlias, id: 'bridge:second-alias' }],
  sessionName: expectedBrowser.sessionName,
  expectedBrowser,
});
assert.equal(ambiguous.browser, null);
assert.match(ambiguous.error, /Multiple service browsers match/);

const duplicateSession = selectRuntimeHandoffBrowser({
  browsers: [exactBrowser, { ...exactBrowser }],
  sessionName: expectedBrowser.sessionName,
  expectedBrowser,
});
assert.equal(duplicateSession.browser, null);
assert.match(duplicateSession.error, /Multiple service browsers claim daemon session/);

const root = mkdtempSync(join(tmpdir(), 'agent-browser-handoff-selection-'));
const handoffPath = join(root, 'retained.handoff.json');
try {
  const expectedWithPath = { ...expectedBrowser, handoffPath };
  writeFileSync(handoffPath, JSON.stringify({
    schemaVersion: 1,
    sessionName: expectedBrowser.sessionName,
    browserPid: expectedBrowser.browserPid,
    cdpUrl: expectedBrowser.cdpUrl,
    runtimeProfile: expectedBrowser.runtimeProfile,
  }));
  assert.equal(removeVerifiedRuntimeHandoffRecord(expectedWithPath), true);
  assert.equal(existsSync(handoffPath), false);
  assert.equal(
    removeVerifiedRuntimeHandoffRecord(expectedWithPath),
    true,
    'retry cleanup must be idempotent after the exact record is absent',
  );

  writeFileSync(handoffPath, JSON.stringify({
    schemaVersion: 1,
    sessionName: expectedBrowser.sessionName,
    browserPid: 999,
    cdpUrl: expectedBrowser.cdpUrl,
    runtimeProfile: expectedBrowser.runtimeProfile,
  }));
  assert.throws(
    () => removeVerifiedRuntimeHandoffRecord(expectedWithPath),
    /changed identity/,
  );
  assert.equal(
    existsSync(handoffPath),
    true,
    'a mismatched retry record must be preserved for investigation',
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Runtime handoff browser selection tests passed');
