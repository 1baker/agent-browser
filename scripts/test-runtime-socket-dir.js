#!/usr/bin/env node

import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { resolveRuntimeSocketDir } from './lib/runtime-socket-dir.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-socket-'));
const uid = typeof process.getuid === 'function' ? process.getuid() : 1000;
const runtimeRoot = join(root, 'run-user');
const runtimeDir = join(runtimeRoot, String(uid));
const home = join(root, 'home');

try {
  mkdirSync(runtimeDir, { recursive: true });
  chmodSync(runtimeDir, 0o700);

  assert.equal(resolveRuntimeSocketDir({
    env: { AGENT_BROWSER_SOCKET_DIR: join(root, 'explicit') },
    platform: 'linux',
    uid,
    home,
    linuxRuntimeRoot: runtimeRoot,
  }), join(root, 'explicit'));

  assert.equal(resolveRuntimeSocketDir({
    env: { XDG_RUNTIME_DIR: join(root, 'xdg') },
    platform: 'linux',
    uid,
    home,
    linuxRuntimeRoot: runtimeRoot,
  }), join(root, 'xdg', 'agent-browser'));

  assert.equal(resolveRuntimeSocketDir({
    env: {},
    platform: 'linux',
    uid,
    home,
    linuxRuntimeRoot: runtimeRoot,
  }), join(runtimeDir, 'agent-browser'));

  chmodSync(runtimeDir, 0o777);
  assert.equal(resolveRuntimeSocketDir({
    env: {},
    platform: 'linux',
    uid,
    home,
    linuxRuntimeRoot: runtimeRoot,
  }), join(home, '.agent-browser'));

  console.log('runtime socket directory tests passed');
} finally {
  rmSync(root, { recursive: true, force: true });
}
