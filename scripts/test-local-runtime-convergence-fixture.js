#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-p78-convergence-'));
const controller = resolve('scripts/converge-local-runtime.js');
const fakeAgentBrowser = join(fixtureRoot, 'fake-agent-browser.mjs');
const fakePnpm = join(fixtureRoot, 'fake-pnpm.mjs');

writeFileSync(fakeAgentBrowser, `#!/usr/bin/env node
import { appendFileSync, readFileSync } from 'node:fs';

const args = process.argv.slice(2);
const state = JSON.parse(readFileSync(process.env.P78_FIXTURE_STATE, 'utf8'));
appendFileSync(process.env.P78_FIXTURE_LOG, JSON.stringify({ command: 'agent-browser', args }) + '\\n');

if (args.join(' ') === 'install doctor --json') {
  console.log(JSON.stringify({
    success: true,
    data: {
      issues: [],
      runtimeInventory: { status: 'converged', runtimeCount: 0, staleCount: 0, convergedCount: 0 },
      daemonListenerInventory: { state: 'authoritative', listeners: [] },
    },
  }));
  process.exit(0);
}

if (args.join(' ') === 'doctor remote-view --json') {
  let nextAction = 'provision_second_guacamole_rdp_connection';
  if (state.scenario === 'unrelated') nextAction = 'inspect_unrelated_runtime_state';
  if (state.phase === 'provisioned') {
    nextAction = 'open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor';
  } else if (state.phase === 'displays_open') {
    nextAction = 'grant_route_display_access';
  } else if (state.phase === 'ready') {
    nextAction = 'none';
  }
  const ready = state.phase === 'ready';
  console.log(JSON.stringify({
    success: true,
    data: {
      status: ready ? 'ready' : 'blocked',
      nextAction,
      remoteControl: { ready },
      runtimeInventory: { status: 'converged', runtimeCount: 0, staleCount: 0, convergedCount: 0 },
    },
  }));
  process.exit(0);
}

if (args.join(' ') === '--json service reconcile') {
  console.log(JSON.stringify({ success: true, data: {} }));
  process.exit(0);
}

console.error('unexpected fake agent-browser command: ' + args.join(' '));
process.exit(2);
`);

writeFileSync(fakePnpm, `#!/usr/bin/env node
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
const state = JSON.parse(readFileSync(process.env.P78_FIXTURE_STATE, 'utf8'));
appendFileSync(process.env.P78_FIXTURE_LOG, JSON.stringify({ command: 'pnpm', args }) + '\\n');

const action = args[0];
if (action === 'ensure:rdp-guac-postgres' || action === 'test:rdp-guac-route-pool-readiness') {
  process.exit(0);
}
if (action === 'sync:rdp-guac-existing-user-route-pool') {
  if (args.length !== 1) {
    console.error('existing-user route sync is apply-by-default and accepts no apply flag');
    process.exit(3);
  }
  state.phase = 'provisioned';
  writeFileSync(process.env.P78_FIXTURE_STATE, JSON.stringify(state));
  process.exit(0);
}
if (action === 'open:rdp-route-displays') {
  state.phase = 'displays_open';
  writeFileSync(process.env.P78_FIXTURE_STATE, JSON.stringify(state));
  process.exit(0);
}
if (action === 'grant:rdp-route-display-access') {
  assertApply();
  state.phase = 'ready';
  writeFileSync(process.env.P78_FIXTURE_STATE, JSON.stringify(state));
  process.exit(0);
}

console.error('unexpected fake pnpm command: ' + args.join(' '));
process.exit(2);

function assertApply() {
  if (!args.includes('--apply')) {
    console.error('fixture mutation command omitted --apply: ' + args.join(' '));
    process.exit(3);
  }
}
`);

chmodSync(fakeAgentBrowser, 0o755);
chmodSync(fakePnpm, 0o755);

try {
  const recovered = runScenario('empty-route-fixtures', ['--apply', '--skip-publish', '--json']);
  assert.equal(
    recovered.result.status,
    0,
    `authorized fixture recovery must converge: ${recovered.result.stdout}${recovered.result.stderr}`,
  );
  assert.equal(recovered.payload.success, true);

  const syncIndexes = commandIndexes(
    recovered.commands,
    'pnpm',
    'sync:rdp-guac-existing-user-route-pool',
  );
  assert.deepEqual(syncIndexes.length, 1, 'fixture recovery must run exactly one guarded route sync');
  const syncIndex = syncIndexes[0];
  const reconcileIndex = commandIndexes(recovered.commands, 'agent-browser', '--json')[0];
  const doctorAfterSyncIndex = recovered.commands.findIndex(
    (entry, index) => index > syncIndex
      && entry.command === 'agent-browser'
      && entry.args.join(' ') === 'doctor remote-view --json',
  );
  const displayRestoreIndex = commandIndexes(
    recovered.commands,
    'pnpm',
    'open:rdp-route-displays',
  )[0];
  assert.ok(reconcileIndex < syncIndex, 'retained service reconciliation must precede route sync');
  assert.ok(
    syncIndex < doctorAfterSyncIndex && doctorAfterSyncIndex < displayRestoreIndex,
    'doctors must be rerun after fixture sync and before display restoration',
  );

  const dryRun = runScenario('empty-route-fixtures-dry-run', ['--json']);
  assert.equal(dryRun.result.status, 1, 'dry-run must report the blocked empty-fixture state');
  assertNoMutations(dryRun.commands, 'dry-run');

  const unrelated = runScenario('unrelated', ['--apply', '--skip-publish', '--json'], {
    scenario: 'unrelated',
  });
  assert.equal(unrelated.result.status, 1, 'an unrelated doctor action must remain blocked');
  assert.equal(
    commandIndexes(unrelated.commands, 'pnpm', 'sync:rdp-guac-existing-user-route-pool').length,
    0,
    'an unrelated doctor action must not provision route fixtures',
  );

  console.log('Local runtime convergence fixture behavior passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function runScenario(name, controllerArgs, state = {}) {
  const scenarioRoot = join(fixtureRoot, name);
  const statePath = `${scenarioRoot}.state.json`;
  const logPath = `${scenarioRoot}.commands.jsonl`;
  const evidencePath = `${scenarioRoot}.evidence.json`;
  writeFileSync(statePath, JSON.stringify({ phase: 'missing', ...state }));
  writeFileSync(logPath, '');
  const result = spawnSync(process.execPath, [
    controller,
    ...controllerArgs,
    '--evidence-path',
    evidencePath,
  ], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: fixtureRoot,
      AGENT_BROWSER_BIN: fakeAgentBrowser,
      PNPM_BIN: fakePnpm,
      P78_FIXTURE_LOG: logPath,
      P78_FIXTURE_STATE: statePath,
    },
  });
  const commands = readFileSync(logPath, 'utf8')
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  return {
    result,
    payload: JSON.parse(result.stdout),
    commands,
  };
}

function commandIndexes(commands, command, firstArg) {
  return commands.flatMap((entry, index) => (
    entry.command === command && entry.args[0] === firstArg ? [index] : []
  ));
}

function assertNoMutations(commands, label) {
  const mutations = new Set([
    'ensure:rdp-guac-postgres',
    'sync:rdp-guac-existing-user-route-pool',
    'open:rdp-route-displays',
    'grant:rdp-route-display-access',
  ]);
  assert.equal(
    commands.some((entry) => entry.command === 'pnpm' && mutations.has(entry.args[0])),
    false,
    `${label} must not run a runtime mutation command`,
  );
}
