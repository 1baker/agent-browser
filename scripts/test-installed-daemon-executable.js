#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { daemonUsesInstalledExecutable, installedDaemonVerified } from './lib/installed-daemon-executable.js';

assert.equal(daemonUsesInstalledExecutable(process.pid, process.execPath), true);
assert.equal(daemonUsesInstalledExecutable(0, process.execPath), false);
const observed = [];
const stat = (path) => {
  observed.push(path);
  return path === '/installed' ? { dev: 1, ino: 2 } : { dev: 1, ino: 3 };
};
assert.equal(daemonUsesInstalledExecutable(42, '/installed', stat), false);
assert.deepEqual(observed, ['/installed', '/proc/42/exe']);
assert.equal(daemonUsesInstalledExecutable(42, '/installed', () => { throw new Error('unreadable'); }), false);
assert.equal(installedDaemonVerified(null, '/installed', () => true, () => true), false);
assert.equal(installedDaemonVerified(42, '/installed', () => false, () => true), false);
assert.equal(installedDaemonVerified(42, '/installed', () => true, () => false), false);
assert.equal(installedDaemonVerified(42, '/installed', () => true, () => true), true);

const publisher = readFileSync('scripts/publish-local-dashboard-runtime.js', 'utf8');
const resume = publisher.slice(publisher.indexOf('function resumeRuntimeHandoffs('));
const initialCheck = resume.indexOf('assertInstalledRuntimeDaemon(prepared.sessionName, installBin)');
assert.ok(initialCheck >= 0);
assert.ok(resume.indexOf('existsSync(join(runtimeSocketDir(), `${prepared.sessionName}.sock`))')
  < initialCheck);
assert.ok(initialCheck < resume.indexOf('const existing = serviceBrowserForSession('));
assert.ok(initialCheck < resume.indexOf("process.kill(existingDaemonPid, 'SIGTERM')"));
const alreadyResumed = resume.indexOf('alreadyResumed: true');
assert.ok(resume.lastIndexOf('assertInstalledRuntimeDaemon(prepared.sessionName, installBin)', alreadyResumed)
  > resume.indexOf('if (existing.success && existing.browser)'));
assert.ok(resume.lastIndexOf('assertInstalledRuntimeDaemon(prepared.sessionName, installBin)', alreadyResumed)
  < resume.lastIndexOf('removeVerifiedRuntimeHandoffRecord(prepared)', alreadyResumed));
const successResume = resume.indexOf('const data = resumed.json.data || {}');
assert.ok(resume.indexOf('assertInstalledRuntimeDaemon(prepared.sessionName, installBin)', successResume)
  < resume.indexOf('report.handoffs.resumed.push({', successResume));
assert.ok(resume.includes('const pid = readRuntimePid(sessionName);'));
assert.ok(resume.includes('!installedDaemonVerified(pid, installBin, browserProcessIsLive)'));

console.log('installed daemon executable guard: ok');
