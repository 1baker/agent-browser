#!/usr/bin/env node
// Operator-installed recovery for one explicitly armed retained conversation.
// No browser input or prompt submission primitive is available here.
import { createHash } from 'node:crypto';
import { closeSync, existsSync, fsyncSync, lstatSync, mkdirSync, openSync,
  readFileSync, readdirSync, renameSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { homedir, userInfo } from 'node:os';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const schema = 'agent-browser.workshop-boot-recovery.v1';
const epochPattern = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}:[1-9][0-9]*$/;

/** Journal only fixed categories and process metadata, never child output or auth. */
export function commandFailureDetails(result) {
  let parsed;
  try { parsed = JSON.parse(result.stdout); } catch { /* Child output is untrusted. */ }
  const message = typeof parsed?.error === 'string' ? parsed.error : '';
  const categories = [
    [/^guacamole_credentials_missing:/, 'guacamole_credentials_missing'],
    [/^guacamole_route_[ab]_login_failed:/, 'guacamole_login_failed'],
    [/^guacamole_route_[ab]_header_auth_required/, 'guacamole_header_auth_required'],
    [/^route_pool_missing:/, 'route_pool_missing'],
    [/^route_[ab]_display_timeout:/, 'route_display_timeout'],
    [/^open Guacamole route [AB] failed using /, 'route_viewer_open_failed'],
    [/^configure Guacamole header authentication for route [AB] failed using /, 'route_header_configuration_failed'],
    [/^reload authenticated Guacamole route [AB] failed using /, 'route_navigation_failed'],
    [/^route display inspector JSON parse failed:/, 'route_inspection_invalid_response'],
  ];
  return {
    category: categories.find(([pattern]) => pattern.test(message))?.[1] || 'unclassified_child_failure',
    exitCode: Number.isInteger(result.status) ? result.status : null,
    signal: ['SIGTERM', 'SIGKILL', 'SIGABRT', 'SIGSEGV'].includes(result.signal) ? result.signal : null,
    spawnError: ['ETIMEDOUT', 'ENOENT', 'EACCES', 'ENOBUFS'].includes(result.error?.code) ? result.error.code : null,
    jsonResponse: parsed !== null && typeof parsed === 'object',
  };
}

/** Only read-only probes may be retried. No attempt receipt is consumed here. */
export async function waitForDependencies({ probe, timeoutMs = 180000,
  now = () => performance.now(), sleep = (ms) => new Promise((done) => setTimeout(done, ms)) }) {
  const deadline = now() + timeoutMs;
  let missing = [];
  while (now() < deadline) {
    const result = await probe(Math.max(1, Math.floor(deadline - now())));
    if (result.ready && now() <= deadline) return result;
    missing = result.missing;
    if (result.terminal) throw Error(`dependency_preflight_refused:${result.missing.join(',')}`);
    if (now() < deadline) await sleep(Math.min(2000, deadline - now()));
  }
  throw Error(`dependency_readiness_timeout_no_recovery_attempt:${missing.join(',')}`);
}

/** Local service, database and HTTP reads only: never opens a desktop or browser. */
export function probeDependencies(env, budgetMs) {
  const deadline = performance.now() + budgetMs;
  const checks = [
    ...['docker.service', 'xrdp.service', 'xrdp-sesman.service']
      .map((unit) => [unit, 'systemctl', ['is-active', '--quiet', unit]]),
    ['containers', 'docker', ['inspect', '-f', '{{.State.Running}}',
      'agent-browser-guacamole', 'agent-browser-guacamole-postgres', 'agent-browser-guacd'],
    (output) => output.trim().split(/\s+/).length === 3 && output.trim().split(/\s+/).every((s) => s === 'true')],
    ['database', 'docker', ['exec', 'agent-browser-guacamole-postgres', 'pg_isready', '-U', 'guacamole_user', '-d', 'guacamole_db']],
    ['guacamole_http', 'curl', ['--silent', '--fail', '--max-time', '4', '--output', '/dev/null',
      '--write-out', '%{http_code}', 'http://127.0.0.1:8092/guacamole/'], (output) => output === '200'],
    ['privileged_helper', 'sudo', ['-n', '/usr/local/libexec/agent-browser/agent-browser-privileged-helper', 'check']],
  ];
  for (const [name, command, args, accepts] of checks) {
    const remaining = Math.floor(deadline - performance.now());
    if (remaining <= 0) return { ready: false, missing: [name] };
    const result = spawnSync(command, args, { env, cwd: env.HOME, encoding: 'utf8',
      timeout: Math.min(5000, remaining), maxBuffer: 65536 });
    if (result.error || result.status !== 0 || (accepts && !accepts(result.stdout))) {
      return { ready: false, missing: [name], terminal: name === 'privileged_helper' };
    }
  }
  return { ready: true, missing: [] };
}

/** WSL distro termination restarts PID 1 without necessarily restarting its kernel. */
export function distributionEpoch(kernelBootId, initStat) {
  if (!/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(kernelBootId)
      || !initStat.startsWith('1 (')) throw Error('invalid_distribution_epoch');
  // comm (field 2) may contain spaces or parentheses. starttime is field 22.
  const startTicks = initStat.slice(initStat.lastIndexOf(')') + 1).trim().split(/\s+/)[19];
  const epoch = `${kernelBootId}:${startTicks}`;
  if (!epochPattern.test(epoch)) throw Error('invalid_distribution_epoch');
  return epoch;
}

export function sameIdentity(a, b) {
  return ['sessionName', 'profileId', 'url'].every((key) => a?.[key] === b?.[key]);
}

/** Pure admission and injected effects permit crash/replay regression tests. */
export async function recover({ state, boot, requirement, status, save, effects, verify,
  beforeEffects = async () => ({ status }) }) {
  if (state.schema !== schema || !epochPattern.test(boot)) throw Error('invalid_recovery_state');
  if (state.attempt) throw Error('uncertain_attempt_requires_review');
  if (!sameIdentity(state.identity, requirement.expectation)
      || state.digest !== requirement.digest) throw Error('requirement_changed');
  if (status.verified) {
    await save({ ...state, boot });
    return 'verified_no_action';
  }
  if (!epochPattern.test(state.boot)) throw Error('legacy_epoch_requires_verified_rearm');
  if (boot === state.boot) throw Error('same_boot_recovery_refused');
  if (!['retained_daemon_missing', 'retained_browser_missing'].includes(status.reason)) {
    throw Error('old_authority_unproven');
  }
  // Startup waits happen before the durable uncertain-effect boundary.
  const fresh = await beforeEffects();
  if (fresh.status.verified) {
    await save({ ...state, boot });
    return 'verified_no_action';
  }
  if (!['retained_daemon_missing', 'retained_browser_missing'].includes(fresh.status.reason)) {
    throw Error('old_authority_unproven_after_readiness');
  }
  // This durable record remains even if the process dies after dispatch.
  const pending = { ...state, attempt: { boot, digest: state.digest } };
  await save(pending);
  await effects(state);
  const next = await verify();
  if (!next.verified || !sameIdentity(state.identity, next.expectation)) {
    throw Error('recovered_identity_unproven');
  }
  await save({ ...state, boot, digest: next.digest, attempt: null });
  return 'recovered_and_verified';
}

function privateJson(path) {
  const st = lstatSync(path);
  if (!st.isFile() || st.isSymbolicLink() || st.uid !== process.getuid()
      || (st.mode & 0o077) || st.size > 1024 * 1024) throw Error('unsafe_private_file');
  return JSON.parse(readFileSync(path, 'utf8'));
}

function durableSave(path, value) {
  const temporary = `${path}.${process.pid}.tmp`;
  const fd = openSync(temporary, 'wx', 0o600);
  try { writeFileSync(fd, JSON.stringify(value)); fsyncSync(fd); } finally { closeSync(fd); }
  renameSync(temporary, path);
  const directory = openSync(dirname(path), 'r');
  try { fsyncSync(directory); } finally { closeSync(directory); }
}

function fileHashes(root) {
  return readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((entry) => {
      const path = join(root, entry.name);
      if (entry.isSymbolicLink()) throw Error('support_symlink_refused');
      return entry.isDirectory() ? fileHashes(path) : [[path, hash(readFileSync(path))]];
    });
}

export function artifactManifest(paths, scripts) {
  return [...paths, ...fileHashes(scripts).map(([path]) => path)]
    .map((path) => [path, existsSync(path) ? hash(readFileSync(path)) : null]);
}

export function recoveryEnvironment(root, username, support) {
  return { HOME: root, USER: username, LOGNAME: username,
    PATH: `${join(root, '.local/bin')}:/usr/local/bin:/usr/bin:/bin`, LANG: 'C.UTF-8',
    XDG_RUNTIME_DIR: `/run/user/${process.getuid()}`,
    DBUS_SESSION_BUS_ADDRESS: `unix:path=/run/user/${process.getuid()}/bus`,
    AGENT_BROWSER_RETAINED_PINNER_SCRIPT: '',
    AGENT_BROWSER_HOME: join(root, '.agent-browser'),
    AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT: join(root, '.agent-browser/publications/local-dashboard-retained-browser.json'),
    AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT: join(support, 'scripts'),
    AGENT_BROWSER_GUACAMOLE_SECRET_FILE: join(root, '.agent-browser/guacamole/secrets/guacamole.env'),
    AGENT_BROWSER_RDP_ROUTE_VIEWER_BROWSER_BUILD: 'stock_chrome',
    AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD: join(root, '.local/bin/agent-browser'),
  };
}

export function preparationArgs(approved) {
  return ['install', 'workstation', 'prepare-retained-browser',
    '--url', approved.identity.url, '--url-prefix', approved.identity.url,
    '--runtime-profile', approved.identity.profileId,
    '--rotate-stale-requirement-sha256', approved.digest, '--json'];
}

async function main() {
  const root = homedir();
  const folder = join(root, '.agent-browser/workshop-boot-recovery');
  const statePath = join(folder, 'state.json');
  const binary = join(root, '.local/bin/agent-browser');
  const support = join(root, '.local/lib/agent-browser/0.28.0');
  const requirementPath = join(root, '.agent-browser/publications/local-dashboard-retained-browser.json');
  const boot = distributionEpoch(readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim(),
    readFileSync('/proc/1/stat', 'utf8'));
  // Do not inherit loader, shell, proxy, provider or identity overrides.
  const env = recoveryEnvironment(root, userInfo().username, support);
  const run = (command, args, label, json = true, allowFailure = false) => {
    const r = spawnSync(command, args, { cwd: support, env, encoding: 'utf8',
      timeout: 240000, maxBuffer: 32 * 1024 * 1024 });
    if (r.error || (!allowFailure && r.status !== 0)) {
      const error = Error(`${label}_failed`);
      error.diagnostic = commandFailureDetails(r);
      throw error;
    }
    if (!json) return null;
    let parsed;
    try { parsed = JSON.parse(r.stdout); } catch { throw Error(`${label}_invalid_response`); }
    if (!allowFailure && parsed?.success !== true) throw Error(`${label}_rejected`);
    return parsed;
  };
  const readRequirement = () => {
    if (existsSync(`${requirementPath}.rotation.json`)) throw Error('pending_rotation_requires_review');
    const req = privateJson(requirementPath);
    const marker = privateJson(`${requirementPath}.required`);
    const digest = hash(readFileSync(requirementPath));
    if (req.schemaVersion !== 'agent-browser.local-dashboard-retained-browser-requirement.v1'
        || marker.schemaVersion !== 'agent-browser.local-dashboard-retained-browser-enforcement.v1'
        || marker.requirementSha256 !== digest
        || req.expectation.sessionName !== req.expectation.profileId
        || !/^https:\/\/chatgpt\.com\/g\/g-p-[a-f0-9]{32}(?:-[a-z0-9-]+)?\/c\/[a-f0-9-]{36}$/.test(req.expectation.url)) {
      throw Error('invalid_enforced_requirement');
    }
    return { digest, expectation: req.expectation };
  };
  const status = () => {
    const result = run(binary, ['install', 'workstation', 'retained-browser-status', '--json'], 'status', true, true);
    return { verified: result.success === true && result.retainedBrowserRequirement?.verified === true,
      reason: /verification failed: (retained_daemon_missing|retained_browser_missing)$/.exec(result.error || '')?.[1] };
  };
  const artifacts = () => artifactManifest([binary, resolve(process.argv[1]), process.execPath,
    join(root, '.agent-browser/.env'), join(root, '.agent-browser/config.json'),
    join(root, '.agent-browser/agent-browser.json'), join(support, 'agent-browser.json')], join(support, 'scripts'));
  mkdirSync(folder, { recursive: true, mode: 0o700 });
  const dir = lstatSync(folder);
  if (!dir.isDirectory() || dir.isSymbolicLink() || dir.uid !== process.getuid() || (dir.mode & 0o077)) {
    throw Error('unsafe_state_directory');
  }
  // Never remove another invocation's lock, including one left by a crash.
  const lock = join(folder, 'lock');
  const fd = openSync(lock, 'wx', 0o600);
  try {
    writeFileSync(fd, `${process.pid}\n`); fsyncSync(fd);
    const requirement = readRequirement();
    if (['--arm', '--rearm-verified'].includes(process.argv[2])) {
      if (process.argv[2] === '--rearm-verified') {
        const previous = privateJson(statePath);
        if (previous.attempt || !sameIdentity(previous.identity, requirement.expectation)) {
          throw Error('rearm_identity_or_attempt_requires_review');
        }
        if (!status().verified) throw Error('arm_requires_live_verified_target');
        // Explicit operator migration retains the old receipt; never fabricate a prior epoch.
        const backup = `${statePath}.before-rearm-${Date.now()}`;
        const backupFd = openSync(backup, 'wx', 0o600);
        try { writeFileSync(backupFd, JSON.stringify(previous)); fsyncSync(backupFd); }
        finally { closeSync(backupFd); }
      } else if (existsSync(statePath)) throw Error('already_armed_requires_review');
      if (!status().verified) throw Error('arm_requires_live_verified_target');
      durableSave(statePath, { schema, boot, identity: requirement.expectation,
        digest: requirement.digest, artifacts: artifacts(), attempt: null });
      console.log(JSON.stringify({ success: true, state: 'armed_for_next_boot' }));
      return;
    }
    if (!['--run', '--check-readiness'].includes(process.argv[2])) throw Error('expected_arm_run_or_check_readiness');
    const state = privateJson(statePath);
    if (JSON.stringify(artifacts()) !== JSON.stringify(state.artifacts)) {
      throw Error('installed_artifacts_changed_requires_review');
    }
    const beforeEffects = async () => {
      await waitForDependencies({ probe: (budget) => probeDependencies(env, budget) });
      const current = readRequirement();
      if (state.attempt || current.digest !== state.digest || !sameIdentity(current.expectation, state.identity)
          || JSON.stringify(artifacts()) !== JSON.stringify(state.artifacts)) {
        throw Error('authority_changed_during_readiness');
      }
      return { status: status() };
    };
    if (process.argv[2] === '--check-readiness') {
      const fresh = await beforeEffects();
      console.log(JSON.stringify({ success: true, state: 'dependencies_ready', retainedVerified: fresh.status.verified }));
      return;
    }
    const outcome = await recover({ state, boot, requirement, status: status(),
      beforeEffects,
      save: (value) => durableSave(statePath, value),
      effects: (approved) => {
        run(process.execPath, [join(support, 'scripts/open-rdp-guac-route-displays.js'),
          '--agent-browser-timeout-ms', '45000', '--route-navigation-timeout-ms', '45000',
          '--route-display-timeout-ms', '45000', '--wait-ms', '0'], 'desktop_restore');
        run('/bin/bash', ['scripts/grant-rdp-route-display-access.sh', '--apply'], 'display_access', false);
        run(binary, ['service', 'reconcile', '--json'], 'service_reconcile');
        // Recheck the exact pinned authority after desktop setup, before navigation.
        const current = readRequirement();
        if (current.digest !== approved.digest || !sameIdentity(current.expectation, approved.identity)) {
          throw Error('requirement_changed_before_navigation');
        }
        run(binary, preparationArgs(approved), 'browser_restore');
      },
      verify: () => ({ ...readRequirement(), ...status() }),
    });
    console.log(JSON.stringify({ success: true, state: outcome }));
  } finally { closeSync(fd); unlinkSync(lock); }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch((error) => { console.error(JSON.stringify({ success: false, error: error.message,
    ...(error.diagnostic ? { diagnostic: error.diagnostic } : {}) })); process.exitCode = 1; });
}
