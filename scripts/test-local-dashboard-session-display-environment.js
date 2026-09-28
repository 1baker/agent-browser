import assert from 'node:assert/strict';
import {
  readSessionDisplayEnvironment,
  withSessionDisplayEnvironment,
} from './lib/local-dashboard-session-display-environment.js';

const captured = process.platform === 'linux'
  ? readSessionDisplayEnvironment(123, () => Buffer.from(
    'DISPLAY=:180\0AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY=:180\0TOKEN=private\0',
  ))
  : { display: ':180', osClickIsolatedDisplay: ':180' };
assert.deepEqual(captured, { display: ':180', osClickIsolatedDisplay: ':180' });
assert.deepEqual(
  withSessionDisplayEnvironment({ DISPLAY: ':99', AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY: ':99', KEEP: 'yes' }, captured),
  { DISPLAY: ':180', AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY: ':180', KEEP: 'yes' },
);
assert.deepEqual(
  withSessionDisplayEnvironment({ DISPLAY: ':99', AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY: ':99' }, {
    display: null,
    osClickIsolatedDisplay: null,
  }),
  {},
);
if (process.platform === 'linux') {
  assert.throws(
    () => readSessionDisplayEnvironment(123, () => Buffer.from('DISPLAY=:180\0AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY=:181\0')),
    /does not match DISPLAY/,
  );
  assert.throws(
    () => readSessionDisplayEnvironment(123, () => Buffer.from('DISPLAY=:180\0DISPLAY=:181\0')),
    /repeats DISPLAY/,
  );
}
console.log('Local dashboard session display environment tests passed');
