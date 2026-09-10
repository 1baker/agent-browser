#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-workstation-install-'));
const fakeBin = join(fixtureRoot, 'bin');
const commandLog = join(fixtureRoot, 'commands.jsonl');
const installRoot = join(fixtureRoot, 'workstation');
const failedInstallRoot = join(fixtureRoot, 'failed-workstation');
const lockedInstallRoot = join(fixtureRoot, 'locked-workstation');
const retainedBlockedRoot = join(fixtureRoot, 'retained-blocked-workstation');
const enforcedMissingRoot = join(fixtureRoot, 'enforced-missing-workstation');
const home = join(fixtureRoot, 'home');
const xdgRoot = join(fixtureRoot, 'xdg');
const agentBrowser = resolveAgentBrowser();

const expectedUnits = new Set([
  'agent-browser-dashboard.service',
  'agent-browser-runtime-interlock.service',
  'agent-browser-runtime-interlock.timer',
  'agent-browser-guacamole-postgres-backup.service',
  'agent-browser-guacamole-postgres-backup.timer',
]);

mkdirSync(fakeBin, { recursive: true });
mkdirSync(home, { recursive: true });
writeFileSync(commandLog, '');
installCommandShim('systemctl');
installCommandShim('sudo');

try {
  assertWorkstationInterface();
  assertNativeRetainedRequirementOrdering();
  const configuredEnvRequirement = join(fixtureRoot, 'env-override', 'missing.json');
  const configuredEnvFile = join(fixtureRoot, 'retained-status.env');
  writeFileSync(
    configuredEnvFile,
    `AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT=${configuredEnvRequirement}\n`,
  );
  const beforeNativeStatus = treeManifest(fixtureRoot, ignoredFixturePaths());
  const nativeStatus = runInstaller(
    installRoot,
    ['retained-browser-status', '--json'],
  );
  assert.equal(
    nativeStatus.status,
    0,
    `native retained-browser status must succeed:\n${nativeStatus.stdout}${nativeStatus.stderr}`,
  );
  const nativeStatusJson = JSON.parse(nativeStatus.stdout);
  assert.equal(
    nativeStatusJson.schemaVersion,
    'agent-browser.workstation-retained-browser-status.v1',
  );
  assert.equal(
    nativeStatusJson.retainedBrowserRequirement.state,
    'not_configured',
  );
  assert.deepEqual(
    treeManifest(fixtureRoot, ignoredFixturePaths()),
    beforeNativeStatus,
    'native retained-browser status must not create workstation or browser state',
  );

  const overrideRequirement = join(fixtureRoot, 'override', 'missing.json');
  const overrideStatus = runInstaller(
    installRoot,
    ['retained-browser-status', '--json'],
    { AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT: overrideRequirement },
  );
  assert.equal(
    overrideStatus.status,
    0,
    `native retained-browser override status must succeed:\n${overrideStatus.stdout}${overrideStatus.stderr}`,
  );
  assert.equal(
    JSON.parse(overrideStatus.stdout).retainedBrowserRequirement.requirementPath,
    overrideRequirement,
    'native status must honor the configured durable requirement path',
  );
  assert.equal(
    existsSync(dirname(overrideRequirement)),
    false,
    'reading an absent configured requirement must not create its parent',
  );

  const envFileStatus = runInstaller(
    installRoot,
    ['retained-browser-status', '--json'],
    { AGENT_BROWSER_ENV_FILE: configuredEnvFile },
  );
  assert.equal(
    envFileStatus.status,
    0,
    `native retained-browser env-file status must succeed:\n${envFileStatus.stdout}${envFileStatus.stderr}`,
  );
  assert.equal(
    JSON.parse(envFileStatus.stdout).retainedBrowserRequirement.requirementPath,
    configuredEnvRequirement,
    'native status must load the configured requirement path from the normal env file',
  );
  assert.equal(
    existsSync(dirname(configuredEnvRequirement)),
    false,
    'env-file status must not create the configured requirement parent',
  );

  const enforcedRequirement = join(
    enforcedMissingRoot,
    '.agent-browser',
    'publications',
    'local-dashboard-retained-browser.json',
  );
  const enforcedMarker = `${enforcedRequirement}.required`;
  mkdirSync(dirname(enforcedMarker), { recursive: true, mode: 0o700 });
  writeFileSync(enforcedMarker, `${JSON.stringify({
    schemaVersion: 'agent-browser.local-dashboard-retained-browser-enforcement.v1',
    createdAt: '2026-08-15T12:00:00.000Z',
    requirementSha256: '0'.repeat(64),
  })}\n`, { mode: 0o600 });
  chmodSync(enforcedMarker, 0o600);
  writeFileSync(commandLog, '');
  const enforcedMissingStatus = runInstaller(
    enforcedMissingRoot,
    ['retained-browser-status', '--json'],
  );
  assert.notEqual(
    enforcedMissingStatus.status,
    0,
    'native status must reject an enforcement record with no requirement',
  );
  assert.match(
    `${enforcedMissingStatus.stdout}${enforcedMissingStatus.stderr}`,
    /retained_browser_requirement_missing/,
  );
  assert.deepEqual(
    regularFiles(enforcedMissingRoot),
    [enforcedMarker],
    'enforced-missing status must not create state',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'enforced-missing status must not invoke service commands',
  );
  const beforeDryRun = treeManifest(fixtureRoot, ignoredFixturePaths());
  const dryRun = runInstaller(installRoot, ['--dry-run', '--json']);
  assert.equal(
    dryRun.status,
    0,
    `workstation install dry-run must succeed:\n${dryRun.stdout}${dryRun.stderr}`,
  );
  assertJsonSuccess(dryRun.stdout, 'dry-run');
  assert.deepEqual(
    treeManifest(fixtureRoot, ignoredFixturePaths()),
    beforeDryRun,
    'dry-run must not mutate HOME, XDG paths, or the workstation install root',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'dry-run must not invoke systemctl or sudo',
  );

  const firstApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.equal(
    firstApply.status,
    0,
    `workstation install apply must succeed:\n${firstApply.stdout}${firstApply.stderr}`,
  );
  assertJsonSuccess(firstApply.stdout, 'first apply');
  assert.ok(existsSync(installRoot), 'apply must create the isolated workstation root');

  const firstManifest = treeManifest(installRoot);
  assert.equal(
    firstManifest.some((entry) => entry.type === 'symlink'),
    false,
    'a source-free payload must not contain symlinks back to external state',
  );
  const installedFiles = regularFiles(installRoot);
  const installedBasenames = new Set(installedFiles.map((path) => basename(path)));
  const installedBinary = installedFiles.find((path) => (
    basename(path) === 'agent-browser'
    && path.includes(`${join('.local', 'bin')}/`)
  ));
  assert.ok(installedBinary, 'apply must install the agent-browser executable');
  assert.notEqual(
    lstatSync(installedBinary).mode & 0o111,
    0,
    'the installed agent-browser payload must be executable',
  );
  for (const unit of expectedUnits) {
    assert.ok(installedBasenames.has(unit), `apply must install ${unit}`);
  }
  const dashboardUnit = installedFiles.find(
    (path) => basename(path) === 'agent-browser-dashboard.service',
  );
  const dashboardSource = readFileSync(dashboardUnit, 'utf8');
  assert.match(
    dashboardSource,
    /EnvironmentFile=-%h\/.agent-browser\/\.env/,
    'dashboard service must load the managed agent-browser environment',
  );
  const payloadManifestPath = installedFiles.find(
    (path) => basename(path) === 'manifest.json' && !path.includes(`${join('guacamole')}/`),
  );
  const payloadRoot = dirname(payloadManifestPath);
  const payloadManifest = JSON.parse(readFileSync(payloadManifestPath, 'utf8'));
  assert.equal(
    sha256(installedBinary),
    payloadManifest.binary.sha256,
    'payload manifest must bind the installed binary hash',
  );
  for (const file of payloadManifest.controllerAssets.files) {
    const path = join(payloadRoot, file.path);
    assert.equal(sha256(path), file.sha256, `controller asset hash mismatch: ${file.path}`);
  }
  const retainedPreparationScript = join(
    payloadRoot,
    'scripts',
    'prepare-local-dashboard-retained-browser.js',
  );
  assert.equal(
    existsSync(retainedPreparationScript),
    true,
    'source-free payload must include the retained-browser preparation controller',
  );
  assert.match(
    readFileSync(retainedPreparationScript, 'utf8'),
    /discoverVerifyAndPinRetainedBrowser/,
    'installed preparation controller must use exact live discovery and pinning',
  );
  const installedPreparationHelp = spawnSync(installedBinary, [
    'install',
    'workstation',
    'prepare-retained-browser',
    '--help',
  ], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      PATH: `${fakeBin}:/usr/bin:/bin`,
      AGENT_BROWSER_WORKSTATION_ROOT: installRoot,
    },
  });
  assert.equal(
    installedPreparationHelp.status,
    0,
    `installed preparation help must run source-free:\n${installedPreparationHelp.stdout}${installedPreparationHelp.stderr}`,
  );
  assert.match(
    `${installedPreparationHelp.stdout}${installedPreparationHelp.stderr}`,
    /source-free workstation preparation command/,
  );
  const installedControllerHelp = spawnSync('node', [retainedPreparationScript, '--help'], {
    cwd: payloadRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      PATH: `${fakeBin}:/usr/bin:/bin`,
    },
  });
  assert.equal(
    installedControllerHelp.status,
    0,
    `installed preparation controller help must run without the checkout:\n${installedControllerHelp.stdout}${installedControllerHelp.stderr}`,
  );
  assert.match(
    `${installedControllerHelp.stdout}${installedControllerHelp.stderr}`,
    /This command has no page interaction or\s+prompt-submission action/,
  );
  const guacamoleRoot = join(payloadRoot, 'guacamole');
  assert.equal(
    sha256(join(guacamoleRoot, 'manifest.json')),
    payloadManifest.guacamoleBundleManifestSha256,
    'payload manifest must bind the Guacamole manifest hash',
  );
  for (const file of payloadManifest.guacamoleBundle.files) {
    assert.equal(
      sha256(join(guacamoleRoot, file.path)),
      file.sha256,
      `Guacamole asset hash mismatch: ${file.path}`,
    );
  }

  const payloadFiles = installedFiles.filter((path) => (
    path.includes(`${join('lib', 'agent-browser')}/`)
    && !expectedUnits.has(basename(path))
  ));
  assert.ok(
    payloadFiles.length > 0,
    'apply must install at least one immutable runtime payload under lib/agent-browser',
  );

  const textArtifacts = installedFiles.filter((path) => (
    expectedUnits.has(basename(path))
    || /\.(?:json|service|sh|timer|txt|yaml|yml)$/.test(path)
  ));
  for (const path of textArtifacts) {
    const source = readFileSync(path, 'utf8');
    assert.equal(
      source.includes(repoRoot),
      false,
      `${path} must not reference the source checkout`,
    );
    assert.equal(
      /\bpnpm\b/i.test(source),
      false,
      `${path} must not require pnpm`,
    );
  }

  const interlockUnit = installedFiles.find(
    (path) => basename(path) === 'agent-browser-runtime-interlock.service',
  );
  const interlockSource = readFileSync(interlockUnit, 'utf8');
  assert.match(
    interlockSource,
    /ExecStart=.+/,
    'the installed interlock unit must declare an executable payload',
  );
  assert.equal(
    /WorkingDirectory=/.test(interlockSource),
    false,
    'the installed interlock unit must not depend on a mutable working directory',
  );
  for (const path of installedFiles.filter((candidate) => candidate.endsWith('.service'))) {
    const unitSource = readFileSync(path, 'utf8');
    const execStart = unitSource.match(/^ExecStart=(.+)$/m)?.[1];
    assert.ok(execStart, `${path} must declare ExecStart`);
    assert.ok(
      execStart.startsWith(installRoot),
      `${path} must execute only the isolated installed payload`,
    );
  }

  const routeOpener = installedFiles.find(
    (path) => basename(path) === 'open-rdp-guac-route-displays.js',
  );
  const routeOpenerSource = readFileSync(routeOpener, 'utf8');
  assert.equal(
    routeOpenerSource.includes('${args.join'),
    false,
    'route-opener failures must not echo reversible authentication arguments',
  );
  assert.match(
    routeOpenerSource,
    /'set',\s*'headers'/,
    'Guacamole route sessions must apply header authentication before navigation',
  );
  assert.equal(
    /'eval'|--base64|--stdin/.test(routeOpenerSource),
    false,
    'Guacamole authentication must not inject tokens through eval arguments or input',
  );
  assert.match(
    routeOpenerSource,
    /waitForRouteDisplay/,
    'long-lived route navigation must require positive Xorg display proof',
  );
  assert.equal(
    /if \(inspection\.success && displayName\)/.test(routeOpenerSource),
    false,
    'route A readiness must not deadlock on two-route global readiness',
  );
  assert.match(
    routeOpenerSource,
    /AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT/,
    'the installed route opener must resolve its inspector from the installed script root',
  );
  assert.match(
    routeOpenerSource,
    /AGENT_BROWSER_GUACAMOLE_BASE_URL/,
    'database-derived route URLs must use an explicit Guacamole base URL',
  );
  assert.equal(
    /const baseUrl = process\.env\.AGENT_BROWSER_REMOTE_VIEW_URL/.test(routeOpenerSource),
    false,
    'a selected route viewer URL must never become the base for another route',
  );
  assert.match(
    routeOpenerSource,
    /Object\.hasOwn\(process\.env, key\)/,
    'an explicitly cleared route-pool env value must not reload stale persisted state',
  );
  assert.match(
    routeOpenerSource,
    /'--data-binary', '@-'/,
    'Guacamole password fallback must pass form data through stdin',
  );
  assert.equal(
    routeOpenerSource.includes('`password=${auth.password}`'),
    false,
    'Guacamole passwords must not appear in curl argv',
  );
  const readinessScript = installedFiles.find(
    (path) => basename(path) === 'smoke-rdp-guac-route-pool-readiness.js',
  );
  const readinessSource = readFileSync(readinessScript, 'utf8');
  assert.match(readinessSource, /'--data-binary',\s*'@-'/);
  assert.equal(readinessSource.includes('`password=${password}`'), false);
  const routeSync = installedFiles.find(
    (path) => basename(path) === 'sync-rdp-guac-route-specific-user-pool.sh',
  );
  const routeSyncSource = readFileSync(routeSync, 'utf8');
  assert.match(routeSyncSource, /python3 \/dev\/fd\/3 3<<'PY'/);
  assert.equal(
    /python3 - \\\n[^]*?\$PASS_A/.test(routeSyncSource),
    false,
    'route passwords must not be passed to Python through argv',
  );
  assert.match(
    routeOpenerSource,
    /agentBrowserTimeoutMs[^]*?\?\? 600000/,
    'slow route-browser commands must retain the bounded ten-minute budget',
  );
  const agentHome = join(home, '.agent-browser');
  mkdirSync(agentHome, { recursive: true });
  writeFileSync(
    join(agentHome, '.env'),
    `AGENT_BROWSER_RDP_ROUTE_POOL_JSON=${JSON.stringify([
      { id: 'legacy-a', routeId: 'guacamole:99', frameUrl: 'http://legacy/a' },
      { id: 'legacy-b', routeId: 'guacamole:100', frameUrl: 'http://legacy/b' },
    ])}\n`,
  );
  writeFileSync(
    join(fakeBin, 'docker'),
    '#!/usr/bin/env bash\nprintf "1\\tAgent Browser RDP Route A\\n2\\tAgent Browser RDP Route B\\n"\n',
  );
  chmodSync(join(fakeBin, 'docker'), 0o755);
  const persistedRouteDriftProbe = spawnSync(
    process.execPath,
    [routeOpener, '--dry-run'],
    {
      encoding: 'utf8',
      env: {
        ...process.env,
        HOME: home,
        PATH: `${fakeBin}:/usr/bin:/bin`,
        AGENT_BROWSER_RDP_ROUTE_POOL_JSON: '',
        AGENT_BROWSER_GUACAMOLE_BASE_URL: 'http://127.0.0.1:8092/guacamole/',
      },
    },
  );
  assert.equal(
    persistedRouteDriftProbe.status,
    0,
    `${persistedRouteDriftProbe.stdout}${persistedRouteDriftProbe.stderr}`,
  );
  assert.deepEqual(
    JSON.parse(persistedRouteDriftProbe.stdout).selectedRoutes.map(
      (route) => route.routeId,
    ),
    ['guacamole:1', 'guacamole:2'],
    'an explicit empty route-pool env value must suppress stale persisted route JSON',
  );
  const installedBinaryInode = statSync(installedBinary).ino;
  const routeOpenerInode = statSync(routeOpener).ino;

  const secondApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.equal(
    secondApply.status,
    0,
    `second workstation install apply must succeed:\n${secondApply.stdout}${secondApply.stderr}`,
  );
  assertJsonSuccess(secondApply.stdout, 'second apply');
  assert.deepEqual(
    treeManifest(installRoot),
    firstManifest,
    'a second apply must leave byte content and file modes unchanged',
  );
  assert.equal(
    statSync(installedBinary).ino,
    installedBinaryInode,
    'a no-op second apply must preserve the installed binary inode',
  );
  assert.equal(
    statSync(routeOpener).ino,
    routeOpenerInode,
    'a no-op second apply must preserve versioned support-file inodes',
  );

  writeFileSync(commandLog, '');
  const failedApply = runInstaller(
    failedInstallRoot,
    ['--apply', '--json'],
    { AGENT_BROWSER_WORKSTATION_FAIL_AFTER: 'units-staged' },
  );
  assert.notEqual(
    failedApply.status,
    0,
    'failure injection after units-staged must return a nonzero status',
  );
  assert.equal(
    regularFiles(failedInstallRoot).filter(
      (path) => basename(path) !== 'workstation.lock',
    ).length,
    0,
    'a failed first apply must roll back all staged payload files',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8')
      .split('\n')
      .filter((line) => /\b(enable|start|restart)\b/.test(line))
      .length,
    0,
    'a failed apply must not activate systemd units',
  );

  const lockDir = join(lockedInstallRoot, '.agent-browser', 'convergence');
  mkdirSync(lockDir, { recursive: true });
  writeFileSync(join(lockDir, 'workstation.lock'), `${process.pid}\n`);
  writeFileSync(commandLog, '');
  const lockedApply = runInstaller(lockedInstallRoot, ['--apply', '--json']);
  assert.notEqual(lockedApply.status, 0, 'a concurrent apply must fail before staging');
  assert.match(`${lockedApply.stdout}${lockedApply.stderr}`, /already active/);
  assert.equal(
    regularFiles(lockedInstallRoot).filter(
      (path) => basename(path) !== 'workstation.lock',
    ).length,
    0,
    'a lock-rejected apply must not stage payload files',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'a lock-rejected apply must not quiesce or activate user units',
  );

  const retainedRequirement = join(
    retainedBlockedRoot,
    '.agent-browser',
    'publications',
    'local-dashboard-retained-browser.json',
  );
  mkdirSync(dirname(retainedRequirement), { recursive: true });
  writeFileSync(
    retainedRequirement,
    `${JSON.stringify({ schemaVersion: 'unsupported.fixture' })}\n`,
    { mode: 0o600 },
  );
  chmodSync(retainedRequirement, 0o600);
  writeFileSync(commandLog, '');
  const retainedBlockedApply = runInstaller(
    retainedBlockedRoot,
    ['--apply', '--json'],
  );
  assert.notEqual(
    retainedBlockedApply.status,
    0,
    'a configured invalid retained lane must fail workstation apply',
  );
  assert.match(
    `${retainedBlockedApply.stdout}${retainedBlockedApply.stderr}`,
    /retained_browser_requirement_invalid/,
  );
  assert.deepEqual(
    regularFiles(retainedBlockedRoot),
    [retainedRequirement],
    'retained-lane refusal must occur before lock or payload staging',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'retained-lane refusal must not quiesce or activate user units',
  );

  console.log('Workstation install source-free fixture passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function resolveAgentBrowser() {
  const configured = process.env.AGENT_BROWSER_FIXTURE_BIN;
  const candidates = [
    configured,
    join(repoRoot, 'cli', 'target', 'debug', 'agent-browser'),
    join(repoRoot, 'cli', 'target', 'release', 'agent-browser'),
    join(repoRoot, 'bin', `agent-browser-${process.platform}-${process.arch}`),
  ].filter(Boolean);
  const candidate = candidates.find((path) => existsSync(path));
  assert.ok(
    candidate,
    'Set AGENT_BROWSER_FIXTURE_BIN to the agent-browser binary under test',
  );
  return resolve(candidate);
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function runInstaller(root, flags, extraEnv = {}) {
  return spawnSync(agentBrowser, ['install', 'workstation', ...flags], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      XDG_CONFIG_HOME: join(xdgRoot, 'config'),
      XDG_DATA_HOME: join(xdgRoot, 'data'),
      XDG_STATE_HOME: join(xdgRoot, 'state'),
      XDG_CACHE_HOME: join(xdgRoot, 'cache'),
      XDG_RUNTIME_DIR: join(xdgRoot, 'runtime'),
      PATH: `${fakeBin}:/usr/bin:/bin`,
      AGENT_BROWSER_WORKSTATION_ROOT: root,
      AGENT_BROWSER_WORKSTATION_COMMAND_LOG: commandLog,
      ...extraEnv,
    },
  });
}

function assertWorkstationInterface() {
  const help = spawnSync(agentBrowser, ['install', '--help'], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      PATH: `${fakeBin}:/usr/bin:/bin`,
    },
  });
  assert.equal(
    help.status,
    0,
    `install help must succeed before running the fixture:\n${help.stdout}${help.stderr}`,
  );
  assert.match(
    `${help.stdout}${help.stderr}`,
    /install workstation/,
    'binary does not expose the required `agent-browser install workstation` interface',
  );
  assert.match(
    `${help.stdout}${help.stderr}`,
    /retained-browser-status/,
    'binary does not expose the native retained-browser status interface',
  );
  assert.match(
    `${help.stdout}${help.stderr}`,
    /prepare-retained-browser/,
    'binary does not expose the source-free retained-browser preparation interface',
  );
}

function assertNativeRetainedRequirementOrdering() {
  const source = readFileSync(
    join(repoRoot, 'cli', 'src', 'workstation_install.rs'),
    'utf8',
  );
  const start = source.indexOf('fn reconcile_workstation_locked(');
  const end = source.indexOf('\nfn ensure_guacamole_header_user(', start);
  assert.notEqual(start, -1, 'native workstation reconcile implementation must exist');
  assert.notEqual(end, -1, 'native workstation reconcile boundary must be discoverable');
  const reconcile = source.slice(start, end);
  const verify = reconcile.indexOf('verify_retained_browser_requirement_for_root(root)?');
  const quiesce = reconcile.indexOf('quiesce_existing_user_units(paths)?');
  assert.notEqual(
    verify,
    -1,
    'binary-owned reconcile must verify the durable retained browser requirement',
  );
  assert.ok(
    verify < quiesce,
    'retained browser verification must precede the first reconcile mutation',
  );

  const installStart = source.indexOf('fn run_workstation_install(');
  const installEnd = source.indexOf('\n#[derive(Debug, Serialize)]', installStart);
  const install = source.slice(installStart, installEnd);
  const installVerify = install.indexOf('verify_retained_browser_requirement_for_root(&root)');
  const installLock = install.indexOf('WorkstationLock::acquire(&root)');
  const installQuiesce = install.indexOf('quiesce_existing_user_units(&paths)');
  assert.notEqual(
    installVerify,
    -1,
    'workstation apply must verify the durable retained browser requirement',
  );
  assert.ok(
    installVerify < installLock && installVerify < installQuiesce,
    'retained browser verification must precede apply lock, staging, and quiescence',
  );
}

function assertJsonSuccess(stdout, label) {
  assert.ok(stdout.trim(), `${label} must emit JSON`);
  let payload;
  try {
    payload = JSON.parse(stdout);
  } catch (error) {
    assert.fail(`${label} emitted invalid JSON: ${error.message}\n${stdout}`);
  }
  assert.equal(payload.success, true, `${label} JSON must report success`);
}

function installCommandShim(command) {
  const path = join(fakeBin, command);
  writeFileSync(path, `#!/usr/bin/env bash
set -euo pipefail
printf '%s' '${command}' >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
printf ' %q' "$@" >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
printf '\\n' >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
`);
  chmodSync(path, 0o755);
}

function regularFiles(root) {
  if (!existsSync(root)) return [];
  const paths = [];
  visit(root, (path) => {
    if (lstatSync(path).isFile()) paths.push(path);
  });
  return paths.sort();
}

function treeManifest(root, ignored = new Set()) {
  if (!existsSync(root)) return [];
  const manifest = [];
  visit(root, (path) => {
    if (ignored.has(path)) return 'skip';
    const stat = lstatSync(path);
    const relative = path.slice(root.length + 1);
    const entry = {
      path: relative,
      mode: stat.mode & 0o777,
      type: stat.isDirectory() ? 'directory' : stat.isSymbolicLink() ? 'symlink' : 'file',
    };
    if (stat.isFile()) {
      entry.sha256 = createHash('sha256').update(readFileSync(path)).digest('hex');
    }
    manifest.push(entry);
  });
  return manifest.sort((left, right) => left.path.localeCompare(right.path));
}

function visit(root, callback) {
  if (!existsSync(root)) return;
  for (const entry of readdirSync(root).sort()) {
    const path = join(root, entry);
    const decision = callback(path);
    if (decision !== 'skip' && statSync(path).isDirectory()) {
      visit(path, callback);
    }
  }
}

function ignoredFixturePaths() {
  return new Set([
    commandLog,
    agentBrowser.startsWith(fixtureRoot) ? agentBrowser : '',
  ]);
}
