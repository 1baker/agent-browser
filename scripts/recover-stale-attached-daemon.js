#!/usr/bin/env node

import {
  closeSync,
  fsyncSync,
  lstatSync,
  openSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';

function fail(message) {
  process.stderr.write(`${JSON.stringify({ success: false, error: message }, null, 2)}\n`);
  process.exit(1);
}

function parseArgs(argv) {
  const options = { apply: false, 'stale-snapshot-recovery': false, 'replace-existing-handoff': false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--apply') options.apply = true;
    else if (argument === '--stale-snapshot-recovery') options['stale-snapshot-recovery'] = true;
    else if (argument === '--replace-existing-handoff') options['replace-existing-handoff'] = true;
    else if (argument === '--json') continue;
    else if (argument.startsWith('--')) {
      const value = argv[index + 1];
      if (!value || value.startsWith('--')) fail(`missing value for ${argument}`);
      options[argument.slice(2)] = value;
      index += 1;
    } else fail(`unexpected argument: ${argument}`);
  }
  for (const required of ['session', 'daemon-pid', 'state', 'handoff', 'target-id']) {
    if (!options[required]) fail(`--${required} is required`);
  }
  if (options['stale-snapshot-recovery'] && !options['daemon-client']) {
    fail('--daemon-client is required with --stale-snapshot-recovery');
  }
  if (options['prior-receipt'] && !options['stale-snapshot-recovery']) {
    fail('--prior-receipt requires --stale-snapshot-recovery');
  }
  if (options['replace-existing-handoff'] && !options['stale-snapshot-recovery']) {
    fail('--replace-existing-handoff requires --stale-snapshot-recovery');
  }
  if (options.apply && !options['expected-plan-digest']) {
    fail('--expected-plan-digest is required with --apply');
  }
  return options;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function startTicks(pid) {
  const raw = readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = raw.lastIndexOf(')');
  if (close < 0) fail(`process ${pid} stat is malformed`);
  return Number(raw.slice(close + 1).trim().split(/\s+/)[19]);
}

function processIdentity(pid) {
  const executable = statSync(`/proc/${pid}/exe`);
  const status = readFileSync(`/proc/${pid}/status`, 'utf8');
  const uidLine = status.split('\n').find((line) => line.startsWith('Uid:'));
  return {
    pid,
    bootId: readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim(),
    startTicks: startTicks(pid),
    executableDevice: Number(executable.dev),
    executableInode: Number(executable.ino),
    executableLink: readlinkSync(`/proc/${pid}/exe`),
    uid: Number(uidLine?.trim().split(/\s+/)[1]),
  };
}

function processProfile(pid) {
  const commandLine = readFileSync(`/proc/${pid}/cmdline`);
  let arguments_ = commandLine.toString('utf8')
    .split('\0')
    .filter(Boolean);
  if (arguments_.length === 1 && commandLine.at(-1) === 0
      && !commandLine.subarray(0, -1).includes(0)) {
    const executable = readlinkSync(`/proc/${pid}/exe`);
    if (!arguments_[0].startsWith(`${executable} `)) {
      fail('browser process title does not bind to the live executable');
    }
    arguments_ = arguments_[0].trim().split(/\s+/);
  }
  const profiles = arguments_
    .map((value, index) => value.startsWith('--user-data-dir=')
      ? value.slice('--user-data-dir='.length)
      : value === '--user-data-dir' ? arguments_[index + 1] : null)
    .filter(Boolean);
  if (profiles.length !== 1) fail('browser process profile argument is not unique');
  return realpathSync(profiles[0]);
}

async function browserTargets(cdpEndpoint) {
  const endpoint = new URL(cdpEndpoint);
  if (endpoint.protocol !== 'ws:' || endpoint.hostname !== '127.0.0.1' || !endpoint.port
      || !endpoint.pathname.startsWith('/devtools/browser/')) {
    fail('CDP endpoint is not a bounded loopback browser endpoint');
  }
  const response = await fetch(`http://127.0.0.1:${endpoint.port}/json/list`, {
    signal: AbortSignal.timeout(3000),
  });
  if (!response.ok) fail(`CDP target inventory returned HTTP ${response.status}`);
  return response.json();
}

function identityWithoutLink(identity) {
  const bound = { ...identity };
  delete bound.executableLink;
  return bound;
}

function recoveryStateBinding(state, sessionName, targetId, staleTargetId) {
  const session = state.sessions?.[sessionName];
  const browser = state.browsers?.[`session:${sessionName}`];
  const profile = state.profiles?.[session?.profileId];
  const target = state.tabs?.[`target:${targetId}`];
  const staleTarget = staleTargetId ? state.tabs?.[`target:${staleTargetId}`] : null;
  return {
    session: session && {
      profileId: session.profileId, browserIds: session.browserIds, tabIds: session.tabIds,
      cleanup: session.cleanup, browserCapabilityLaunch: session.browserCapabilityLaunch,
    },
    browser: browser && {
      profileId: browser.profileId, browserBuild: browser.browserBuild,
      executablePath: browser.executablePath, host: browser.host, health: browser.health,
      pid: browser.pid, cdpEndpoint: browser.cdpEndpoint,
    },
    profile: profile && { userDataDir: profile.userDataDir },
    target: target && {
      browserId: target.browserId, targetId: target.targetId, lifecycle: target.lifecycle,
      url: target.url, ownerSessionId: target.ownerSessionId,
    },
    staleTarget: staleTarget && {
      browserId: staleTarget.browserId, targetId: staleTarget.targetId,
      lifecycle: staleTarget.lifecycle, url: staleTarget.url,
      ownerSessionId: staleTarget.ownerSessionId,
    },
    projectedReceipt: state.runtimeCustodyReceipts?.[sessionName] ?? null,
  };
}

function requireCurrentSnapshotMismatch(options) {
  const result = spawnSync(options['daemon-client'], [
    '--json', '--session', options.session, 'service', 'browsers',
  ], { encoding: 'utf8', timeout: 12000 });
  let payload;
  try { payload = JSON.parse(result.stdout); } catch {
    fail(`daemon mismatch check did not return JSON: ${String(result.stderr || result.error || 'empty').trim()}`);
  }
  if (result.status === 0 || payload?.error !== 'handoff_snapshot_identity_mismatch') {
    fail('daemon is not in the exact stale handoff snapshot failure state');
  }
}

function requireIntermediateDaemonUnattached(options, daemonPid, cdpEndpoint) {
  const result = spawnSync(options['daemon-client'], [
    '--json', '--session', options.session, 'handoff', 'resume',
  ], { encoding: 'utf8', timeout: 12000 });
  let payload;
  try { payload = JSON.parse(result.stdout); } catch {
    fail(`intermediate daemon check did not return JSON: ${String(result.stderr || result.error || 'empty').trim()}`);
  }
  const error = String(payload?.error || '');
  if (result.status === 0 || (!error.startsWith('No prepared runtime handoff is available')
      && error !== 'No such file or directory (os error 2)'
      && error !== 'migration_explicit_resume_required')) {
    fail('intermediate daemon is not proven unattached');
  }
  const endpoint = new URL(cdpEndpoint);
  const port = Number(endpoint.port).toString(16).toUpperCase().padStart(4, '0');
  // /proc/<pid>/fd is a directory; inspect its links without trusting a
  // process-wide socket table that can include other daemons.
  const fdInodes = new Set();
  for (const entry of readdirSync(`/proc/${daemonPid}/fd`)) {
    const target = readlinkSync(`/proc/${daemonPid}/fd/${entry}`);
    const match = /^socket:\[(\d+)\]$/.exec(target);
    if (match) fdInodes.add(match[1]);
  }
  const connected = readFileSync(`/proc/${daemonPid}/net/tcp`, 'utf8').split('\n').some((line) => {
    const fields = line.trim().split(/\s+/);
    return fields[2] === `0100007F:${port}` && fields[3] === '01' && fdInodes.has(fields[9]);
  });
  if (connected) fail('intermediate daemon still has an exact CDP connection');
}

function processIsGone(identity) {
  try { return startTicks(identity.pid) !== identity.startTicks; }
  catch (error) {
    if (error?.code === 'ENOENT') return true;
    throw error;
  }
}

function readPrivateCommittedReceipt(path) {
  const resolved = resolve(path);
  const metadata = lstatSync(resolved);
  if (!metadata.isFile() || metadata.uid !== process.getuid() || (metadata.mode & 0o077) !== 0) {
    fail('prior receipt must be a private regular file owned by the current user');
  }
  const raw = readFileSync(resolved);
  const receipt = JSON.parse(raw);
  if (receipt.schemaVersion !== 2 || receipt.phase !== 'committed') {
    fail('prior receipt is not committed schema v2');
  }
  return { path: resolved, raw, receipt, sha256: sha256(raw) };
}

function atomicWritePrivateJson(path, value, expectedSha256 = null) {
  if (expectedSha256 !== null && sha256(readFileSync(path)) !== expectedSha256) {
    fail('recovery target changed after review');
  }
  const temporary = `${path}.recovery-${process.pid}`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
  const file = openSync(temporary, 'r');
  fsyncSync(file);
  closeSync(file);
  renameSync(temporary, path);
  const directory = openSync(dirname(path), 'r');
  fsyncSync(directory);
  closeSync(directory);
}

function restoreReceiptWithStateLock(plan) {
  const helper = String.raw`
const fs = require('node:fs');
const crypto = require('node:crypto');
const [statePath, expectedBindingSha, receiptPath, expectedReceiptSha, sessionName, targetId] = process.argv.slice(1);
const sha = value => crypto.createHash('sha256').update(value).digest('hex');
const stateRaw = fs.readFileSync(statePath);
const receiptRaw = fs.readFileSync(receiptPath);
if (sha(receiptRaw) !== expectedReceiptSha) throw new Error('prior receipt changed after review');
const state = JSON.parse(stateRaw);
const receipt = JSON.parse(receiptRaw);
const session = state.sessions?.[sessionName];
const browser = state.browsers?.['session:' + sessionName];
const profile = state.profiles?.[session?.profileId];
const target = state.tabs?.['target:' + targetId];
const staleTarget = state.tabs?.['target:' + receipt.targetId];
const binding = {
  session: session && { profileId: session.profileId, browserIds: session.browserIds, tabIds: session.tabIds, cleanup: session.cleanup, browserCapabilityLaunch: session.browserCapabilityLaunch },
  browser: browser && { profileId: browser.profileId, browserBuild: browser.browserBuild, executablePath: browser.executablePath, host: browser.host, health: browser.health, pid: browser.pid, cdpEndpoint: browser.cdpEndpoint },
  profile: profile && { userDataDir: profile.userDataDir },
  target: target && { browserId: target.browserId, targetId: target.targetId, lifecycle: target.lifecycle, url: target.url, ownerSessionId: target.ownerSessionId },
  staleTarget: staleTarget && { browserId: staleTarget.browserId, targetId: staleTarget.targetId, lifecycle: staleTarget.lifecycle, url: staleTarget.url, ownerSessionId: staleTarget.ownerSessionId },
  projectedReceipt: state.runtimeCustodyReceipts?.[sessionName] ?? null,
};
if (sha(JSON.stringify(binding)) !== expectedBindingSha) throw new Error('recovery state binding changed after review');
state.runtimeCustodyReceipts ??= {};
state.runtimeCustodyReceipts[sessionName] = receipt;
const temporary = statePath + '.custody-recovery-' + process.pid;
fs.writeFileSync(temporary, JSON.stringify(state, null, 2) + '\n', { mode: 0o600, flag: 'wx' });
const file = fs.openSync(temporary, 'r'); fs.fsyncSync(file); fs.closeSync(file);
fs.renameSync(temporary, statePath);
const directory = fs.openSync(require('node:path').dirname(statePath), 'r');
fs.fsyncSync(directory); fs.closeSync(directory);
`;
  const result = spawnSync('flock', [
    '--exclusive', `${plan.statePath}.lock`, process.execPath, '-e', helper,
    plan.statePath, plan.stateBindingSha256, plan.priorReceiptPath,
    plan.priorReceiptSha256, plan.sessionName, plan.target.id,
  ], { encoding: 'utf8' });
  if (result.status !== 0) {
    fail(`custody projection restore failed: ${String(result.stderr || result.error || 'unknown').trim()}`);
  }
}

async function buildPlan(options, priorDaemon = null) {
  const statePath = resolve(options.state);
  const rawState = readFileSync(statePath);
  const state = JSON.parse(rawState);
  const sessionName = options.session;
  const daemonPid = Number(options['daemon-pid']);
  const session = state.sessions?.[sessionName];
  const browserId = `session:${sessionName}`;
  const browser = state.browsers?.[browserId];
  const profile = state.profiles?.[session?.profileId];
  const targetId = options['target-id'];
  const tab = state.tabs?.[`target:${targetId}`];
  const launch = session?.browserCapabilityLaunch;
  if (!session || !browser || !profile || !launch) fail('retained service identity is incomplete');
  const staleSnapshotRecovery = options['stale-snapshot-recovery'];
  if (browser.health !== 'ready'
      || (!staleSnapshotRecovery && (browser.host !== 'attached_existing' || browser.pid !== null))
      || (staleSnapshotRecovery && browser.pid !== Number(launch.browserPid))) {
    fail('recovery browser host or PID does not match its bounded mode');
  }
  if (session.profileId !== browser.profileId || launch.profileId !== session.profileId
      || !session.browserIds?.includes(browserId) || !session.tabIds?.includes(`target:${targetId}`)
      || tab?.browserId !== browserId || tab?.ownerSessionId !== sessionName
      || tab?.targetId !== targetId || tab?.lifecycle !== 'ready') {
    fail('service session, browser, profile, or target identity mismatch');
  }
  const browserPid = Number(launch.browserPid);
  const browserProcess = processIdentity(browserPid);
  if (browserProcess.startTicks !== launch.processStartTicks
      || realpathSync(launch.executablePath) !== realpathSync(`/proc/${browserPid}/exe`)
      || launch.cdpEndpoint !== browser.cdpEndpoint
      || realpathSync(launch.userDataDir) !== realpathSync(profile.userDataDir)
      || processProfile(browserPid) !== realpathSync(profile.userDataDir)) {
    fail('live browser no longer matches verified launch proof');
  }
  const activePort = readFileSync(`${realpathSync(profile.userDataDir)}/DevToolsActivePort`, 'utf8')
    .trim().split('\n');
  const endpoint = new URL(browser.cdpEndpoint);
  if (activePort[0] !== endpoint.port || activePort[1] !== endpoint.pathname) {
    fail('browser DevToolsActivePort no longer matches service state');
  }
  const targets = await browserTargets(browser.cdpEndpoint);
  const cdpTarget = targets.find((target) => target.id === targetId && target.type === 'page');
  if (!cdpTarget || cdpTarget.url !== tab.url) fail('exact retained target is absent from CDP');
  const daemon = priorDaemon ?? processIdentity(daemonPid);
  if (!priorDaemon) {
    const socketPidPath = `/run/user/${process.getuid()}/agent-browser/${sessionName}.pid`;
    if (Number(readFileSync(socketPidPath, 'utf8').trim()) !== daemonPid) {
      fail('daemon PID metadata mismatch');
    }
    if (!staleSnapshotRecovery && !daemon.executableLink.endsWith(' (deleted)')) {
      fail('daemon executable is not the reviewed replaced inode');
    }
  }
  const externalReceipt = options['prior-receipt']
    ? readPrivateCommittedReceipt(options['prior-receipt'])
    : null;
  const projectedReceipt = state.runtimeCustodyReceipts?.[sessionName];
  if (externalReceipt && projectedReceipt
      && !isDeepStrictEqual(projectedReceipt, externalReceipt.receipt)) {
    fail('prior receipt conflicts with the shared custody projection');
  }
  const receipt = projectedReceipt ?? externalReceipt?.receipt;
  let custodySource = identityWithoutLink(daemon);
  if (staleSnapshotRecovery) {
    const receiptDestination = receipt?.destination;
    const daemonMatchesReceipt = isDeepStrictEqual(receiptDestination, identityWithoutLink(daemon));
    if (!priorDaemon) {
      if (daemonMatchesReceipt) requireCurrentSnapshotMismatch(options);
      else {
        if (!receiptDestination || !processIsGone(receiptDestination)) {
          fail('intermediate daemon is not eligible while the receipt-bound daemon remains live');
        }
        requireIntermediateDaemonUnattached(options, daemon.pid, browser.cdpEndpoint);
      }
    }
    const profileStat = statSync(realpathSync(profile.userDataDir));
    const browserCustody = {
      canonicalProfile: realpathSync(profile.userDataDir),
      cdpEndpoint: browser.cdpEndpoint,
      process: identityWithoutLink(browserProcess),
      profileDevice: Number(profileStat.dev),
      profileInode: Number(profileStat.ino),
    };
    const staleTargetId = receipt?.targetId;
    if (receipt?.schemaVersion !== 2 || receipt?.phase !== 'committed'
        || (!daemonMatchesReceipt && !processIsGone(receipt.destination))
        || !isDeepStrictEqual(receipt.browser, browserCustody)
        || !staleTargetId || staleTargetId === targetId
        || state.tabs?.[`target:${staleTargetId}`]?.lifecycle === 'ready') {
      fail('stale receipt does not bind exactly to the failed daemon and absent old target');
    }
    custodySource = receipt.destination;
  }
  const handoffPath = resolve(options.handoff);
  let existingHandoffSha256 = null;
  try {
    const metadata = lstatSync(handoffPath);
    if (!options['replace-existing-handoff']) fail('handoff descriptor already exists');
    if (!metadata.isFile() || metadata.uid !== process.getuid() || (metadata.mode & 0o077) !== 0) {
      fail('existing handoff must be a private regular file owned by the current user');
    }
    const rawHandoff = readFileSync(handoffPath);
    const existing = JSON.parse(rawHandoff);
    if (existing.schemaVersion !== 1 || existing.sessionName !== sessionName
        || existing.browserPid !== browserPid || existing.cdpUrl !== browser.cdpEndpoint
        || existing.runtimeProfile !== session.profileId || existing.activeTargetId !== targetId
        || (existing.custody !== undefined && existing.custody !== null)) {
      fail('existing handoff descriptor does not bind the exact current recovery target');
    }
    existingHandoffSha256 = sha256(rawHandoff);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  return {
    schemaVersion: 'agent-browser.stale-attached-daemon-recovery-plan.v1',
    sessionName,
    daemon,
    browser: {
      id: browserId,
      process: browserProcess,
      profileId: session.profileId,
      profilePath: realpathSync(profile.userDataDir),
      cdpEndpoint: browser.cdpEndpoint,
      browserBuild: browser.browserBuild,
    },
    target: { id: targetId, url: tab.url, title: tab.title },
    statePath,
    stateBindingSha256: sha256(JSON.stringify(recoveryStateBinding(
      state, sessionName, targetId, receipt?.targetId,
    ))),
    priorReceiptPath: externalReceipt?.path ?? null,
    priorReceiptSha256: externalReceipt?.sha256 ?? null,
    existingHandoffSha256,
    handoffPath,
    descriptor: {
      schemaVersion: staleSnapshotRecovery ? 3 : 1,
      sessionName,
      cdpUrl: browser.cdpEndpoint,
      browserPid: staleSnapshotRecovery ? browserPid : null,
      runtimeProfile: session.profileId,
      engine: 'chrome',
      host: staleSnapshotRecovery ? browser.host : 'attached_existing',
      closeBrowserOnClose: session.cleanup === 'close_browser',
      activeTargetId: targetId,
      preparedAt: new Date().toISOString(),
      ...(staleSnapshotRecovery ? { custody: {
        source: custodySource,
        browser: receipt.browser,
      } } : {}),
    },
  };
}

function stablePlan(plan) {
  const copy = structuredClone(plan);
  copy.descriptor.preparedAt = '<apply-time>';
  return copy;
}

function planDigest(plan) {
  return sha256(JSON.stringify(stablePlan(plan)));
}

function waitForGone(pid, expectedStartTicks) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    try {
      if (startTicks(pid) !== expectedStartTicks) return;
    } catch (error) {
      if (error?.code === 'ENOENT') return;
      throw error;
    }
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
  }
  fail('exact stale daemon did not exit after revocation');
}

function writePrivateJsonExclusive(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
  const file = openSync(path, 'r');
  fsyncSync(file);
  closeSync(file);
  const directory = openSync(dirname(path), 'r');
  fsyncSync(directory);
  closeSync(directory);
}

const options = parseArgs(process.argv.slice(2));
const startupLock = `/run/user/${process.getuid()}/agent-browser/.${options.session}.startup-lock`;
if (process.env.AGENT_BROWSER_STALE_RECOVERY_LOCKED !== '1') {
  const child = spawnSync('flock', [
    '--exclusive', startupLock, process.execPath, process.argv[1], ...process.argv.slice(2),
  ], {
    stdio: 'inherit',
    env: { ...process.env, AGENT_BROWSER_STALE_RECOVERY_LOCKED: '1' },
  });
  process.exit(child.status ?? 1);
}
const plan = await buildPlan(options);
const digest = planDigest(plan);
if (options.apply) {
  if (options['expected-plan-digest'] !== digest) fail('recovery plan changed after review');
  process.kill(plan.daemon.pid, 'SIGKILL');
  waitForGone(plan.daemon.pid, plan.daemon.startTicks);
  const after = await buildPlan(options, plan.daemon);
  if (!isDeepStrictEqual(after.browser, plan.browser)
      || !isDeepStrictEqual(after.target, plan.target)) {
    fail('browser or target changed after daemon revocation');
  }
  if (plan.priorReceiptPath) {
    restoreReceiptWithStateLock(plan);
  }
  plan.descriptor.preparedAt = new Date().toISOString();
  if (plan.existingHandoffSha256) {
    atomicWritePrivateJson(plan.handoffPath, plan.descriptor, plan.existingHandoffSha256);
  } else {
    writePrivateJsonExclusive(plan.handoffPath, plan.descriptor);
  }
}
process.stdout.write(`${JSON.stringify({
  success: true,
  mode: options.apply ? 'apply' : 'dry-run',
  planDigest: digest,
  plan,
  daemonRevoked: options.apply,
  handoffPrepared: options.apply,
}, null, 2)}\n`);
