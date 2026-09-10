#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { retirePreparedDaemon } from './lib/prepared-daemon-retirement.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-prepared-retirement-'));
const handoffPath = join(root, 'retained.handoff.json');
const prepared = {
  sessionName: 'retained',
  daemonPid: 101,
  browserPid: 202,
  cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
  handoffPath,
};

try {
  writeFileSync(handoffPath, JSON.stringify({
    schemaVersion: 1,
    sessionName: prepared.sessionName,
    browserPid: prepared.browserPid,
    cdpUrl: prepared.cdpUrl,
  }));

  const live = new Set([prepared.daemonPid, prepared.browserPid]);
  const signals = [];
  const signal = retirePreparedDaemon(prepared, {
    isProcessLive: (pid) => live.has(pid),
    signalProcess: (pid, value) => signals.push([pid, value]),
    waitForExit: (pid, _timeout, isLive) => {
      if (signals.at(-1)?.[1] === 'SIGKILL') live.delete(pid);
      return !isLive(pid);
    },
  });
  assert.equal(signal, 'SIGKILL');
  assert.deepEqual(signals, [
    [prepared.daemonPid, 'SIGTERM'],
    [prepared.daemonPid, 'SIGKILL'],
  ]);
  assert.equal(live.has(prepared.browserPid), true);

  writeFileSync(handoffPath, JSON.stringify({
    schemaVersion: 1,
    sessionName: 'wrong-session',
    browserPid: prepared.browserPid,
    cdpUrl: prepared.cdpUrl,
  }));
  assert.throws(
    () => retirePreparedDaemon(prepared, {
      isProcessLive: () => true,
      signalProcess: () => assert.fail('mismatched descriptor must not signal'),
    }),
    /mismatched durable descriptor/,
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Prepared daemon retirement compatibility tests passed');
