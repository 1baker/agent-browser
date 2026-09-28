#!/usr/bin/env node

import assert from 'node:assert/strict';

import { resolveRuntimeDaemonClientBinary } from './lib/runtime-daemon-client-binary.js';

const fallback = '/opt/agent-browser/current';
const dependencies = {
  platform: 'linux',
  pathExists: () => true,
  readLink: () => '/opt/agent-browser/old',
};

assert.equal(
  resolveRuntimeDaemonClientBinary(123, fallback, dependencies),
  '/proc/123/exe',
);
assert.equal(
  resolveRuntimeDaemonClientBinary(123, fallback, {
    ...dependencies,
    readLink: () => '/opt/agent-browser/old (deleted)',
  }),
  fallback,
);
assert.equal(
  resolveRuntimeDaemonClientBinary(123, fallback, {
    ...dependencies,
    readLink: () => { throw new Error('stale proc entry'); },
  }),
  fallback,
);
assert.equal(
  resolveRuntimeDaemonClientBinary(123, fallback, {
    ...dependencies,
    pathExists: () => false,
  }),
  fallback,
);

console.log('Runtime daemon client binary tests passed');
