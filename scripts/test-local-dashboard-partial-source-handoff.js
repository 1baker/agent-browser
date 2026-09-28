import assert from 'node:assert/strict';
import { verifyPartialSourceHandoffInventory } from './lib/local-dashboard-partial-source-handoff.js';

const fixture = {
  expectedSessions: ['retained', 'idle', 'zombie'],
  preparedSessions: ['retained'],
  readBrowser: () => null,
  readDaemonPid: (session) => session === 'idle' ? 200 : session === 'zombie' ? 300 : null,
  isProcessLive: (pid) => pid === 200,
  hasHandoffRecord: () => false,
  sameSourceExecutable: (pid) => pid === 200,
};
assert.equal(verifyPartialSourceHandoffInventory(fixture), true);
assert.equal(verifyPartialSourceHandoffInventory({
  ...fixture, readBrowser: (session) => session === 'idle' ? { cdpEndpoint: 'ws://127.0.0.1:9' } : null,
}), false);
assert.equal(verifyPartialSourceHandoffInventory({
  ...fixture, hasHandoffRecord: (session) => session === 'zombie',
}), false);
assert.equal(verifyPartialSourceHandoffInventory({
  ...fixture, sameSourceExecutable: () => false,
}), false);
assert.equal(verifyPartialSourceHandoffInventory({
  ...fixture, preparedSessions: ['retained', 'unknown'],
}), false);
console.log('Local dashboard partial source handoff tests passed');
