#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';

const repoRoot = resolve(import.meta.dirname, '..');
const agentBrowser = process.env.AGENT_BROWSER_TEST_BINARY
  ? resolve(process.env.AGENT_BROWSER_TEST_BINARY)
  : join(repoRoot, 'cli/target/debug/agent-browser');
const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-electron-relay-'));
const configRoot = join(fixtureRoot, 'xdg');
const powershell = join(fixtureRoot, 'powershell.exe');
const serverScript = join(fixtureRoot, 'fixture-server.js');
const localPort = await reservePort();
const common = [
  'electron',
  'relay',
];
const installArgs = [
  ...common,
  'install',
  '--name',
  'termius',
  '--process-name',
  'Termius.exe',
  '--local-port',
  String(localPort),
  '--remote-port',
  '9222',
  '--json',
];

mkdirSync(dirname(powershell), { recursive: true });
writeFileSync(
  powershell,
  `#!/usr/bin/env node
const states = {
  absent: { ProcessCount: 0, Pids: [], ListenerCount: 0, Owners: [], Addresses: [] },
  ready: { ProcessCount: 1, Pids: [42], ListenerCount: 1, Owners: [42], Addresses: ['127.0.0.1'] },
  ambiguous: { ProcessCount: 2, Pids: [42, 43], ListenerCount: 1, Owners: [42], Addresses: ['127.0.0.1'] },
  wrong_owner: { ProcessCount: 1, Pids: [42], ListenerCount: 1, Owners: [43], Addresses: ['127.0.0.1'] },
  exposed: { ProcessCount: 1, Pids: [42], ListenerCount: 1, Owners: [42], Addresses: ['0.0.0.0'] },
};
process.stdout.write(JSON.stringify(states[process.env.ELECTRON_RELAY_FIXTURE_STATE || 'absent']));
`,
);
chmodSync(powershell, 0o755);
writeFileSync(
  serverScript,
  `import { createServer } from 'node:net';
const mode = process.argv[2];
const port = Number(process.argv[3]);
const server = createServer((socket) => {
  socket.once('data', () => {
    const isCdp = mode === 'cdp' || mode === 'cdp-keepalive';
    const body = isCdp ? JSON.stringify({ webSocketDebuggerUrl: 'ws://127.0.0.1/devtools/browser/fixture' }) : 'not cdp';
    const response = 'HTTP/1.1 200 OK\\r\\nContent-Length: ' + Buffer.byteLength(body) + '\\r\\nConnection: ' + (mode === 'cdp-keepalive' ? 'keep-alive' : 'close') + '\\r\\n\\r\\n' + body;
    if (mode === 'cdp-keepalive') socket.write(response);
    else socket.end(response);
  });
});
server.listen(port, '127.0.0.1', () => process.stdout.write('ready\\n'));
`,
);

const env = {
  ...process.env,
  AGENT_BROWSER_ELECTRON_RELAY_CONFIG_ROOT: configRoot,
  AGENT_BROWSER_ELECTRON_RELAY_POWERSHELL: powershell,
  AGENT_BROWSER_ELECTRON_RELAY_SKIP_INTEROP_CHECK: '1',
  AGENT_BROWSER_ELECTRON_RELAY_SKIP_SYSTEMCTL: '1',
};

function run(args, extraEnv = {}) {
  return spawnSync(agentBrowser, args, {
    cwd: repoRoot,
    env: { ...env, ...extraEnv },
    encoding: 'utf8',
  });
}

function json(result) {
  return JSON.parse(result.stdout.trim());
}

async function reservePort() {
  const server = createServer();
  await new Promise((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveListen);
  });
  const { port } = server.address();
  await new Promise((resolveClose) => server.close(resolveClose));
  return port;
}

async function startFixtureServer(mode, port) {
  const child = spawn(process.execPath, [serverScript, mode, String(port)], {
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  await new Promise((resolveReady, reject) => {
    child.once('error', reject);
    child.stdout.once('data', resolveReady);
    child.once('exit', (code) => reject(new Error(`fixture server exited ${code}`)));
  });
  return child;
}

async function stopFixtureServer(child) {
  if (child.exitCode !== null) return;
  const exited = new Promise((resolveExit) => child.once('exit', resolveExit));
  child.kill();
  await exited;
}

try {
  const dryRun = run([...installArgs, '--dry-run']);
  assert.equal(dryRun.status, 0, dryRun.stderr);
  assert.equal(json(dryRun).mutated, false);
  assert.equal(existsSync(configRoot), false, 'dry-run must not create config state');

  const apply = run([...installArgs, '--apply']);
  assert.equal(apply.status, 0, apply.stderr);
  assert.equal(json(apply).mutated, true);

  const idempotent = run([...installArgs, '--apply']);
  assert.equal(idempotent.status, 0, idempotent.stderr);
  assert.equal(json(idempotent).idempotent, true);
  assert.equal(json(idempotent).mutated, false);

  const servicePath = join(
    configRoot,
    'systemd/user/agent-browser-electron-relay-termius.service',
  );
  const service = readFileSync(servicePath, 'utf8');
  assert.match(service, /electron relay run --name termius/);
  assert.doesNotMatch(service, /Termius\.exe|powershell|bash|ssh|0\.0\.0\.0/);

  const absent = run([...common, 'doctor', '--name', 'termius', '--json']);
  assert.equal(absent.status, 0, absent.stderr);
  assert.equal(json(absent).state, 'not_running');
  assert.equal(json(absent).ready, false);

  const globalJson = run(['--json', ...common, 'doctor', '--name', 'termius']);
  assert.equal(globalJson.status, 0, globalJson.stderr);
  assert.equal(json(globalJson).state, 'not_running');

  const absentRun = run([...common, 'run', '--name', 'termius', '--json']);
  assert.equal(absentRun.status, 0, absentRun.stderr);
  assert.equal(json(absentRun).started, false);

  const cdpServer = await startFixtureServer('cdp', localPort);
  const ready = run(
    [...common, 'doctor', '--name', 'termius', '--json'],
    {
      ELECTRON_RELAY_FIXTURE_STATE: 'ready',
      AGENT_BROWSER_ELECTRON_RELAY_FIXTURE_SERVICE_ACTIVE: '1',
    },
  );
  assert.equal(ready.status, 0, ready.stderr);
  assert.equal(json(ready).state, 'ready');
  assert.equal(json(ready).ready, true);

  const unmanaged = run(
    [...common, 'doctor', '--name', 'termius', '--json'],
    { ELECTRON_RELAY_FIXTURE_STATE: 'ready' },
  );
  assert.notEqual(unmanaged.status, 0, 'unmanaged CDP endpoint must fail closed');
  assert.match(
    json(unmanaged).error,
    /not owned by the active managed relay service/,
  );
  await stopFixtureServer(cdpServer);

  const keepaliveCdpServer = await startFixtureServer('cdp-keepalive', localPort);
  const keepaliveReady = run(
    [...common, 'doctor', '--name', 'termius', '--json'],
    {
      ELECTRON_RELAY_FIXTURE_STATE: 'ready',
      AGENT_BROWSER_ELECTRON_RELAY_FIXTURE_SERVICE_ACTIVE: '1',
    },
  );
  assert.equal(keepaliveReady.status, 0, keepaliveReady.stderr);
  assert.equal(json(keepaliveReady).state, 'ready');
  assert.equal(json(keepaliveReady).ready, true);
  await stopFixtureServer(keepaliveCdpServer);

  const conflictServer = await startFixtureServer('other', localPort);
  const conflict = run(
    [...common, 'doctor', '--name', 'termius', '--json'],
    {
      ELECTRON_RELAY_FIXTURE_STATE: 'ready',
      AGENT_BROWSER_ELECTRON_RELAY_FIXTURE_SERVICE_ACTIVE: '1',
    },
  );
  await stopFixtureServer(conflictServer);
  assert.notEqual(conflict.status, 0, 'non-CDP local listener must fail closed');
  assert.match(json(conflict).error, /occupied by a non-CDP listener/);

  for (const state of ['ambiguous', 'wrong_owner', 'exposed']) {
    const rejected = run(
      [...common, 'doctor', '--name', 'termius', '--json'],
      { ELECTRON_RELAY_FIXTURE_STATE: state },
    );
    assert.notEqual(rejected.status, 0, `${state} must fail closed`);
    assert.equal(json(rejected).success, false);
  }

  writeFileSync(servicePath, `${service}# operator drift\n`);
  const driftedRemoval = run([
    ...common,
    'uninstall',
    '--name',
    'termius',
    '--apply',
    '--json',
  ]);
  assert.notEqual(driftedRemoval.status, 0, 'drifted uninstall must fail closed');
  assert.equal(existsSync(servicePath), true, 'drifted artifact must be preserved');
  writeFileSync(servicePath, service);

  const removalPreview = run([
    ...common,
    'uninstall',
    '--name',
    'termius',
    '--dry-run',
    '--json',
  ]);
  assert.equal(removalPreview.status, 0, removalPreview.stderr);
  assert.equal(json(removalPreview).mutated, false);

  const removal = run([
    ...common,
    'uninstall',
    '--name',
    'termius',
    '--apply',
    '--json',
  ]);
  assert.equal(removal.status, 0, removal.stderr);
  assert.equal(existsSync(servicePath), false);

  const unsafeName = run([
    ...common,
    'install',
    '--name',
    '../escape',
    '--process-name',
    'Termius.exe',
    '--local-port',
    String(localPort),
    '--remote-port',
    '9222',
    '--dry-run',
    '--json',
  ]);
  assert.notEqual(unsafeName.status, 0, 'unsafe name must be rejected');

  console.log('electron relay lifecycle fixture passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}
