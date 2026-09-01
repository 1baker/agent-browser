#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-rust-tests-'));
const isolatedHome = join(root, 'home');
const agentHome = join(isolatedHome, '.agent-browser');
const socketDir = join(root, 'sockets');
const runtimeDir = join(root, 'runtime');
const realHome = homedir();
for (const path of [isolatedHome, agentHome, socketDir, runtimeDir]) {
  mkdirSync(path, { recursive: true, mode: 0o700 });
}

const separator = process.argv.indexOf('--');
const requested = separator >= 0 ? process.argv.slice(separator + 1) : process.argv.slice(2);
const cargoArgs = requested[0] === 'test'
  ? [...requested]
  : ['test', '--manifest-path', 'cli/Cargo.toml', ...requested];
if (cargoArgs[0] !== 'test') {
  console.error('Isolated Rust validation only accepts cargo test arguments');
  process.exit(2);
}
if (!cargoArgs.includes('--test-threads=1')) {
  const harnessSeparator = cargoArgs.indexOf('--');
  if (harnessSeparator >= 0) cargoArgs.push('--test-threads=1');
  else cargoArgs.push('--', '--test-threads=1');
}

const env = {
  ...process.env,
  HOME: isolatedHome,
  AGENT_BROWSER_HOME: agentHome,
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  XDG_RUNTIME_DIR: runtimeDir,
  AGENT_BROWSER_TEST_ISOLATED: '1',
};
if (!env.CARGO_HOME && existsSync(join(realHome, '.cargo'))) {
  env.CARGO_HOME = join(realHome, '.cargo');
}
if (!env.RUSTUP_HOME && existsSync(join(realHome, '.rustup'))) {
  env.RUSTUP_HOME = join(realHome, '.rustup');
}

let status = 1;
try {
  console.log(`Isolated Rust test root: ${root}`);
  const result = spawnSync('cargo', cargoArgs, {
    cwd: new URL('..', import.meta.url).pathname,
    env,
    stdio: 'inherit',
  });
  status = result.status ?? 1;
  if (result.error) console.error(result.error.message);
} finally {
  rmSync(root, { recursive: true, force: true });
}
process.exit(status);
