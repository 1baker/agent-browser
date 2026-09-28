#!/usr/bin/env node
import assert from 'node:assert/strict';
import {
  isProcessLive,
  waitForRuntimeDaemonExit,
  verifyRuntimeSessionsRetired,
} from './lib/runtime-daemon-exit.js';

const priorPid = 101;
assert.equal(isProcessLive(priorPid, {
  kill: () => {}, readProcStat: () => '101 (agent-browser) Z 1 101 101', platform: 'linux',
}), false);
assert.equal(isProcessLive(priorPid, {
  kill: () => {}, readProcStat: () => '101 (agent-browser) S 1 101 101', platform: 'linux',
}), true);
assert.equal(isProcessLive(priorPid, {
  kill: () => {}, readProcStat: () => { throw Object.assign(new Error('gone'), { code: 'ENOENT' }); }, platform: 'linux',
}), false);
for (const recordedPid of [null, priorPid, 303]) {
  waitForRuntimeDaemonExit('fixture', priorPid, {
    readRuntimePid: () => recordedPid,
    isProcessLive: () => false,
    sleep: () => assert.fail('dead original must not wait for stale metadata'),
  });
}
for (const priorStillLive of [false, true]) {
  assert.throws(() => waitForRuntimeDaemonExit('fixture', priorPid, {
    readRuntimePid: () => 303,
    isProcessLive: (pid) => pid === 303 || (pid === priorPid && priorStillLive),
    sleep: () => assert.fail('reacquisition must be reported immediately'),
  }), { code: 'runtime_session_reacquired' });
}
let elapsed = 0;
assert.throws(() => waitForRuntimeDaemonExit('fixture', priorPid, {
  readRuntimePid: () => priorPid,
  isProcessLive: () => true,
  now: () => elapsed,
  sleep: (ms) => { elapsed += ms; },
  timeoutMs: 100,
}), { code: 'runtime_daemon_exit_timeout' });
assert.equal(elapsed, 100);
waitForRuntimeDaemonExit('fixture', priorPid, {
  readRuntimePid: () => priorPid,
  isProcessLive: () => elapsed < 150,
  now: () => elapsed,
  sleep: (ms) => { elapsed += ms; },
});
assert.throws(() => verifyRuntimeSessionsRetired({
  sessionNames: ['fixture'], readRuntimePid: () => 303, isProcessLive: () => true,
}), /is live before executable replacement/);
verifyRuntimeSessionsRetired({
  sessionNames: ['fixture'], readRuntimePid: () => priorPid, isProcessLive: () => false,
});
console.log('Runtime daemon exit and session vacancy fixtures passed');
