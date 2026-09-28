#!/usr/bin/env node

import {
  closeSync,
  fsyncSync,
  lstatSync,
  openSync,
  readFileSync,
  realpathSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

function fail(message) {
  process.stderr.write(`${JSON.stringify({ success: false, error: message }, null, 2)}\n`);
  process.exit(1);
}

function parseArgs(argv) {
  const options = { apply: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--apply') options.apply = true;
    else if (argument === '--source-gone') options.sourceGone = true;
    else if (argument === '--json') continue;
    else if (argument.startsWith('--')) {
      const value = argv[index + 1];
      if (!value || value.startsWith('--')) fail(`missing value for ${argument}`);
      options[argument.slice(2)] = value;
      index += 1;
    } else fail(`unexpected argument: ${argument}`);
  }
  for (const required of ['session', 'receipt', 'state']) {
    if (!options[required]) fail(`--${required} is required`);
  }
  if (options.apply && !options['expected-state-sha256'] && !options['expected-receipt-sha256']) {
    fail('--expected-state-sha256 or --expected-receipt-sha256 is required with --apply');
  }
  if (options['restore-handoff'] && !options.sourceGone) {
    fail('--restore-handoff requires --source-gone');
  }
  return options;
}

function sha256(payload) {
  return createHash('sha256').update(payload).digest('hex');
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function processIdentity(pid) {
  const proc = `/proc/${pid}`;
  const stat = readFileSync(`${proc}/stat`, 'utf8');
  const close = stat.lastIndexOf(')');
  if (close < 0) fail(`process ${pid} stat is malformed`);
  const startTicks = Number(stat.slice(close + 1).trim().split(/\s+/)[19]);
  const executable = statSync(`${proc}/exe`);
  const status = readFileSync(`${proc}/status`, 'utf8');
  const uidLine = status.split('\n').find((line) => line.startsWith('Uid:'));
  const uid = Number(uidLine?.trim().split(/\s+/)[1]);
  return {
    pid,
    bootId: readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim(),
    startTicks,
    executableDevice: Number(executable.dev),
    executableInode: Number(executable.ino),
    uid,
  };
}

function equalIdentity(actual, expected, label) {
  for (const key of ['pid', 'bootId', 'startTicks', 'executableDevice', 'executableInode', 'uid']) {
    if (actual[key] !== expected?.[key]) fail(`${label} ${key} mismatch`);
  }
}

function requireIdentityGone(identity, label) {
  const currentBoot = readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim();
  if (currentBoot !== identity.bootId) return;
  try {
    const stat = readFileSync(`/proc/${identity.pid}/stat`, 'utf8');
    const close = stat.lastIndexOf(')');
    const startTicks = Number(stat.slice(close + 1).trim().split(/\s+/)[19]);
    if (startTicks !== identity.startTicks) return;
  } catch (error) {
    if (error?.code === 'ENOENT') return;
    throw error;
  }
  fail(`${label} is still alive`);
}

function validateProfile(browser) {
  const canonical = realpathSync(browser.canonicalProfile);
  if (canonical !== browser.canonicalProfile) fail('browser canonical profile mismatch');
  const metadata = statSync(canonical);
  if (!metadata.isDirectory()) fail('browser profile is not a directory');
  if (Number(metadata.dev) !== browser.profileDevice || Number(metadata.ino) !== browser.profileInode) {
    fail('browser profile identity mismatch');
  }
  const rawCmdline = readFileSync(`/proc/${browser.process.pid}/cmdline`).toString('utf8');
  let cmdline = rawCmdline
    .split('\0')
    .filter(Boolean);
  // Chrome may replace its argv vector with a single process-title string after
  // launch.  Keep accepting only a title that starts with the exact live exe,
  // then recover its whitespace-delimited flags for the no-space managed path.
  if (cmdline.length === 1
      && rawCmdline.endsWith('\0')
      && rawCmdline.slice(0, -1).includes('\0') === false) {
    const executablePath = realpathSync(`/proc/${browser.process.pid}/exe`);
    if (!cmdline[0].startsWith(`${executablePath} `)) {
      fail('browser process title does not bind the live executable');
    }
    cmdline = cmdline[0].split(/\s+/);
  }
  const profileArgs = cmdline
    .map((value, index) => value.startsWith('--user-data-dir=')
      ? value.slice('--user-data-dir='.length)
      : value === '--user-data-dir' ? cmdline[index + 1] : null)
    .filter(Boolean);
  if (profileArgs.length !== 1 || realpathSync(profileArgs[0]) !== canonical) {
    fail('browser process profile argument mismatch');
  }
  const endpoint = new URL(browser.cdpEndpoint);
  if (endpoint.protocol !== 'ws:' || endpoint.hostname !== '127.0.0.1' || !endpoint.port
      || !endpoint.pathname.startsWith('/devtools/browser/')) {
    fail('browser CDP endpoint is not a bounded loopback browser endpoint');
  }
  const [port, path] = readFileSync(`${canonical}/DevToolsActivePort`, 'utf8').trim().split('\n');
  if (port !== endpoint.port || path !== endpoint.pathname) fail('browser active CDP endpoint mismatch');
}

function validateReceipt(receiptPath, receipt, state, sessionName, sourceGone) {
  const receiptMetadata = lstatSync(receiptPath);
  if (!receiptMetadata.isFile() || (receiptMetadata.mode & 0o077) !== 0 || receiptMetadata.uid !== process.getuid()) {
    fail('receipt must be a private regular file owned by the current user');
  }
  if (receipt.schemaVersion !== 2 || receipt.phase !== 'committed') fail('receipt is not committed schema v2');
  if (!/^[a-f0-9]{64}$/.test(receipt.descriptorSha256 ?? '')) fail('receipt descriptor digest is invalid');
  if (sourceGone) requireIdentityGone(receipt.destination, 'former daemon destination');
  else equalIdentity(processIdentity(receipt.destination.pid), receipt.destination, 'daemon destination');
  equalIdentity(processIdentity(receipt.browser.process.pid), receipt.browser.process, 'browser process');
  validateProfile(receipt.browser);

  const session = state.sessions?.[sessionName];
  const browserId = `session:${sessionName}`;
  const browser = state.browsers?.[browserId];
  const tabId = `target:${receipt.targetId}`;
  const tab = state.tabs?.[tabId];
  const profile = state.profiles?.[session?.profileId];
  if (!session || session.lease !== 'exclusive') fail('exact exclusive service session is required');
  if (!browser || browser.pid !== receipt.browser.process.pid
      || browser.cdpEndpoint !== receipt.browser.cdpEndpoint
      || browser.profileId !== session.profileId) fail('service browser identity mismatch');
  if (!session.browserIds?.includes(browserId) || !session.tabIds?.includes(tabId)
      || tab?.browserId !== browserId || tab?.ownerSessionId !== sessionName
      || tab?.targetId !== receipt.targetId || tab?.lifecycle !== 'ready') {
    fail('service target custody mismatch');
  }
  if (!profile?.userDataDir || realpathSync(profile.userDataDir) !== receipt.browser.canonicalProfile) {
    fail('service profile identity mismatch');
  }
}

function restoredHandoffDescriptor(receipt, state, sessionName) {
  const session = state.sessions[sessionName];
  const browser = state.browsers[`session:${sessionName}`];
  return {
    schemaVersion: 2,
    sessionName,
    cdpUrl: receipt.browser.cdpEndpoint,
    browserPid: receipt.browser.process.pid,
    runtimeProfile: session.profileId,
    engine: 'chrome',
    host: browser.host,
    closeBrowserOnClose: session.cleanup === 'close_browser',
    activeTargetId: receipt.targetId,
    preparedAt: new Date().toISOString(),
    custody: {
      source: receipt.destination,
      browser: receipt.browser,
    },
  };
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

function atomicWriteJson(path, value) {
  const temporary = `${path}.custody-repair-${process.pid}`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
  const file = openSync(temporary, 'r');
  fsyncSync(file);
  closeSync(file);
  renameSync(temporary, path);
  const directory = openSync(dirname(path), 'r');
  fsyncSync(directory);
  closeSync(directory);
}

const options = parseArgs(process.argv.slice(2));
const statePath = resolve(options.state);
const receiptPath = resolve(options.receipt);
const lockPath = `${statePath}.lock`;
if (process.env.AGENT_BROWSER_CUSTODY_REPAIR_LOCKED !== '1') {
  const child = spawnSync('flock', [
    '--exclusive', lockPath, process.execPath, process.argv[1], ...process.argv.slice(2),
  ], {
    stdio: 'inherit',
    env: { ...process.env, AGENT_BROWSER_CUSTODY_REPAIR_LOCKED: '1' },
  });
  process.exit(child.status ?? 1);
}

const rawState = readFileSync(statePath);
const stateSha256 = sha256(rawState);
const state = JSON.parse(rawState);
const receipt = JSON.parse(readFileSync(receiptPath, 'utf8'));
const receiptSha256 = sha256(readFileSync(receiptPath));
validateReceipt(receiptPath, receipt, state, options.session, options.sourceGone === true);
const existing = state.runtimeCustodyReceipts?.[options.session];
if (existing && stableJson(existing) !== stableJson(receipt)) {
  fail('a different runtime custody receipt is already recorded for this session');
}
if (options.apply) {
  if (options['expected-state-sha256'] && options['expected-state-sha256'] !== stateSha256) {
    fail('service state changed after preview');
  }
  if (options['expected-receipt-sha256']
      && options['expected-receipt-sha256'] !== receiptSha256) {
    fail('custody receipt changed after review');
  }
  state.runtimeCustodyReceipts ??= {};
  state.runtimeCustodyReceipts[options.session] = receipt;
  atomicWriteJson(statePath, state);
  if (options['restore-handoff']) {
    writePrivateJsonExclusive(
      resolve(options['restore-handoff']),
      restoredHandoffDescriptor(receipt, state, options.session),
    );
  }
}
process.stdout.write(`${JSON.stringify({
  success: true,
  schemaVersion: 'agent-browser.runtime-custody-repair.v1',
  mode: options.apply ? 'apply' : 'dry-run',
  session: options.session,
  statePath,
  stateSha256,
  receiptPath,
  receiptSha256,
  destinationPid: receipt.destination.pid,
  browserPid: receipt.browser.process.pid,
  targetId: receipt.targetId,
  restored: options.apply,
  handoffRestored: Boolean(options.apply && options['restore-handoff']),
}, null, 2)}\n`);
