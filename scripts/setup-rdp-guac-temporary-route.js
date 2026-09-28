#!/usr/bin/env node
// Add one isolated desktop account and connection; never restart or reconfigure A/B.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync, lstatSync, rmdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { temporaryRoute, quote, connectionSql } from './lib/temporary-rdp-route.js';

const helper = '/usr/local/libexec/agent-browser/agent-browser-privileged-helper';
function run(command, args, input) {
  const result = spawnSync(command, args, { input, encoding: 'utf8', timeout: 60000 });
  if (result.status !== 0) throw new Error(`${command} failed; sensitive output withheld`);
  return result.stdout.trim();
}
function sql(query) {
  return run('docker', ['exec', '-i', 'agent-browser-guacamole-postgres', 'psql',
    '-U', 'guacamole_user', '-d', 'guacamole_db', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], query);
}
function privatePath(path, directory = false) {
  const stat = lstatSync(path);
  if (stat.isSymbolicLink() || stat.uid !== process.getuid() || (stat.mode & 0o077) ||
      (directory ? !stat.isDirectory() : !stat.isFile())) throw new Error('unsafe private state path');
}
function main() {
  const args = process.argv.slice(2).filter(x => x !== '--');
  const labelIndex = args.indexOf('--label');
  const label = labelIndex >= 0 ? args[labelIndex + 1] : undefined;
  const route = temporaryRoute(label);
  const remaining = args.filter((_, i) => i !== labelIndex && i !== labelIndex + 1);
  if (remaining.some(x => !['--apply', '--dry-run', '--json'].includes(x)) ||
      remaining.filter(x => ['--apply', '--dry-run'].includes(x)).length !== 1) {
    throw new Error('use --label C with exactly one of --dry-run or --apply');
  }
  const apply = remaining.includes('--apply');
  const operator = process.env.USER;
  connectionSql(route, 'a'.repeat(64), operator); // Validate before any mutation.
  const root = join(process.env.AGENT_BROWSER_HOME || join(homedir(), '.agent-browser'), 'temporary-routes');
  const journal = join(root, `${label}.json`);
  const existing = sql(`SELECT connection_id FROM guacamole_connection WHERE connection_name = ${quote(route.name)};`);
  const userExists = spawnSync('getent', ['passwd', route.user], { encoding: 'utf8' });
  if (![0, 2].includes(userExists.status)) throw new Error('unable to inspect route account');
  if (existing || userExists.status === 0 || existsSync(journal)) {
    throw new Error('route identity or journal already exists; inspect before retry, never overwrite');
  }
  run('sudo', ['-n', helper, 'check']);
  const operatorPresent = sql(`SELECT count(*) FROM guacamole_entity WHERE name = ${quote(operator)} AND type = 'USER';`);
  if (operatorPresent !== '1') throw new Error('unique Guacamole operator identity required');
  if (!apply) return { success: true, status: 'dry_run', ...route, wouldRestartServices: false,
    nextStep: 'Apply adds only this account and connection; browser/display readiness is a separate gate.' };

  mkdirSync(root, { recursive: true, mode: 0o700 });
  privatePath(root, true);
  const lock = join(root, '.provision.lock');
  mkdirSync(lock, { mode: 0o700 }); // Fail closed on concurrent or interrupted provisioning.
  try {
    // Standard installed backup verifies database continuity before any host/DB write.
    const backup = JSON.parse(run('agent-browser', ['install', 'workstation', 'backup', '--json']));
    if (backup.success !== true) throw new Error('workstation backup did not succeed');
    // Backup can be slow. Recheck under our provisioning lock immediately before
    // using the existing helper, which would rotate a preexisting user's password.
    const currentUser = spawnSync('getent', ['passwd', route.user], { encoding: 'utf8' });
    if (currentUser.status !== 2 || existsSync(journal) ||
        sql(`SELECT connection_id FROM guacamole_connection WHERE connection_name = ${quote(route.name)};`)) {
      throw new Error('route identity changed during preflight; refusing mutation');
    }
    const password = randomBytes(32).toString('hex');
    const receipt = { schemaVersion: 1, ...route, operator, password, state: 'prepared', createdAt: new Date().toISOString() };
    writeFileSync(journal, JSON.stringify(receipt) + '\n', { mode: 0o600, flag: 'wx' });
    privatePath(journal);
    run('sudo', ['-n', helper, 'ensure-rdp-route-user', '--user', route.user], password + '\n');
    const result = sql(connectionSql(route, password, operator));
    const connectionId = result.split('\n').filter(x => /^\d+$/.test(x)).at(-1);
    if (!connectionId) throw new Error('connection created but identifier not confirmed; inspect journal');
    receipt.state = 'provisioned';
    receipt.connectionId = connectionId;
    writeFileSync(journal, JSON.stringify(receipt) + '\n', { mode: 0o600 });
    // Read back only nonsensitive identity. Do not export credentials or raw provider URLs.
    const verified = sql(`SELECT count(*) FROM guacamole_connection c JOIN guacamole_connection_parameter p
      ON c.connection_id=p.connection_id WHERE c.connection_id=${connectionId}
      AND c.connection_name=${quote(route.name)} AND p.parameter_name='username' AND p.parameter_value=${quote(route.user)};`);
    if (verified !== '1') throw new Error('connection identity readback failed');
    return { success: true, status: 'provisioned_not_browser_ready', ...route, connectionId,
      servicesRestarted: false, nextStep: 'Establish only the new RDP desktop, verify isolated display, then request a managed browser handoff.' };
  } finally { rmdirSync(lock); }
}
try { console.log(JSON.stringify(main())); }
catch (error) { console.log(JSON.stringify({ success: false, error: error.message })); process.exitCode = 1; }
