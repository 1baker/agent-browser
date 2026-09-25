#!/usr/bin/env node

// Pure in-memory protocol fixtures: no browser, profile, process, or install writes.
import assert from 'node:assert/strict';
import { runColdRestart } from './lib/local-dashboard-cold-restart.js';

const clone = (value) => structuredClone(value);
const operations = [
  'verifyIntent', 'fenceSource', 'closeSource', 'requireIdle', 'backupProfile',
  'verifyBackups', 'installPair', 'launch', 'qualify', 'fenceCandidate', 'closeCandidate',
  'restoreProfile', 'restoreDashboard', 'verifyFinal',
];

function fixture({ fail = null, recover = false, seed = null, commitFault = null, durableCommitFault = null } = {}) {
  let record = seed ? clone(seed) : {
    phase: 'prepared',
    coldRestart: {
      schemaVersion: 'agent-browser.cold-restart.v1',
      intent: {
        approved: true, sessionName: 'default', profileId: 'chatgpt-pro',
        canonicalProfile: '/fixture/profile', url: 'https://chatgpt.com/',
        sourceSha256: 'a'.repeat(64), candidateSha256: 'b'.repeat(64),
        sourceRecordSha256: 'c'.repeat(64), candidateRecordSha256: 'd'.repeat(64),
      },
      profileBackup: null, sourceClosed: false, candidateLaunchAdmitted: false,
    },
  };
  const events = [];
  const commits = [];
  const counts = new Map();
  let failUsed = false;
  let commitFailureUsed = false;
  let running = !recover || !record.coldRestart.closeAdmitted || record.coldRestart.sourceLaunchAdmitted
    ? 'source' : record.coldRestart.candidateLaunchAdmitted ? 'candidate' : null;
  let installed = running === 'candidate' ? 'candidate' : 'source';
  const trip = (name) => {
    counts.set(name, (counts.get(name) || 0) + 1);
    events.push(name);
    if (!failUsed && fail === name) {
      failUsed = true;
      throw new Error(`fixture fault: ${name}`);
    }
  };
  const journal = {
    commit(_previous, phase, patch = {}) {
      events.push(`commit:${phase}`);
      assert.deepEqual(_previous, record, 'journal revision conflict: caller is stale');
      if (!commitFailureUsed && commitFault === commits.length + 1) {
        commitFailureUsed = true;
        throw new Error(`fixture journal fault: ${phase}`);
      }
      record = { ...clone(_previous), ...clone(patch), phase };
      commits.push(clone(record));
      if (durableCommitFault === commits.length) {
        events.push('durable-commit-response-lost');
        throw new Error(`fixture durable journal response lost: ${phase}`);
      }
      return clone(record);
    },
  };
  const adapters = Object.fromEntries(operations.map((name) => [name, async (current, selection) => {
    const key = ['installPair', 'launch', 'qualify', 'verifyFinal'].includes(name)
      ? `${name}:${selection}` : name;
    trip(key);
    switch (name) {
      case 'verifyIntent':
        assert.equal(current.coldRestart?.schemaVersion, 'agent-browser.cold-restart.v1');
        assert.equal(current.coldRestart.intent.approved, true);
        assert.equal(current.coldRestart.intent.sessionName, 'default');
        assert.equal(current.coldRestart.intent.profileId, 'chatgpt-pro');
        for (const field of ['sourceSha256', 'candidateSha256', 'sourceRecordSha256', 'candidateRecordSha256']) {
          assert.match(current.coldRestart.intent[field], /^[a-f0-9]{64}$/);
        }
        return { verified: true };
      case 'closeSource':
        assert.equal(running, 'source');
        running = null;
        return { closed: true };
      case 'requireIdle':
        assert.equal(running, null, 'must prove no browser before cold profile/install work');
        return { idle: true };
      case 'backupProfile':
        assert.equal(running, null);
        return { schemaVersion: 'fixture.backup.v1', path: '/fixture/backup', sha256: 'e'.repeat(64) };
      case 'installPair':
        assert.equal(running, null, 'cannot install over a running browser');
        assert.ok(['source', 'candidate'].includes(selection));
        installed = selection;
        return { verified: true };
      case 'launch':
        if (recover && selection === 'source' && current.coldRestart.sourceLaunchAdmitted && running === 'source') {
          return { selection, pid: 101, reused: true };
        }
        assert.equal(running, null, 'duplicate launch forbidden');
        assert.equal(installed, selection);
        if (selection === 'candidate') {
          assert.equal(record.coldRestart.candidateLaunchAdmitted, true, 'launch intent must be durable before spawn');
        }
        running = selection;
        return { selection, pid: selection === 'source' ? 101 : 202 };
      case 'qualify':
      case 'verifyFinal':
        assert.equal(running, selection);
        return { verified: true };
      case 'closeCandidate':
        assert.equal(current.coldRestart.candidateLaunchAdmitted, true);
        assert.ok(running === null || running === 'candidate');
        running = null;
        return { closed: true };
      case 'restoreProfile':
        assert.equal(running, null);
        assert.ok(current.coldRestart.profileBackup);
        return { restored: true };
      default:
        return { verified: true };
    }
  }]));
  adapters.restoreSourceFence = async () => { trip('restoreSourceFence'); return { restored: true }; };
  const input = { journal, record: clone(record), adapters, recover };
  return { input, events, commits, counts, adapters, current: () => clone(record), running: () => running };
}

async function settled(f) {
  try { return { value: await runColdRestart(f.input) }; }
  catch (error) { return { error }; }
}

function before(events, first, second) {
  assert.ok(events.indexOf(first) >= 0, `missing ${first}: ${events}`);
  assert.ok(events.indexOf(second) > events.indexOf(first), `${first} must precede ${second}: ${events}`);
}

const success = fixture();
const good = await settled(success);
assert.ifError(good.error);
assert.equal(success.current().phase, 'ready');
assert.equal(success.running(), 'candidate');
for (const [first, second] of [
  ['verifyIntent', 'fenceSource'], ['fenceSource', 'closeSource'],
  ['closeSource', 'requireIdle'], ['requireIdle', 'backupProfile'],
  ['verifyBackups', 'closeSource'], ['backupProfile', 'installPair:candidate'],
  ['installPair:candidate', 'launch:candidate'], ['launch:candidate', 'qualify:candidate'],
  ['qualify:candidate', 'restoreDashboard'], ['restoreDashboard', 'verifyFinal:candidate'],
]) before(success.events, first, second);
assert.equal(success.counts.get('launch:candidate'), 1);
assert.equal(success.counts.has('launch:source'), false);

for (const fail of ['verifyIntent', 'fenceSource']) {
  const f = fixture({ fail });
  const result = await settled(f);
  assert.ok(result.error || f.current().phase === 'rolled_back', fail);
  assert.equal(f.running(), 'source', fail);
  assert.equal(f.events.includes('closeSource'), false, fail);
  assert.equal(f.events.some((event) => /^(launch|installPair|restoreProfile)/.test(event)), false, fail);
}

// A close error is an ambiguous external outcome, never proof of cold state.
const ambiguousClose = fixture({ fail: 'closeSource' });
assert.ok((await settled(ambiguousClose)).error);
assert.equal(ambiguousClose.current().phase, 'recovery_blocked');
assert.equal(ambiguousClose.events.some((event) => event.startsWith('launch:')), false);

for (const fail of [
  'requireIdle', 'backupProfile', 'verifyBackups', 'installPair:candidate',
  'launch:candidate', 'qualify:candidate', 'restoreDashboard', 'verifyFinal:candidate',
]) {
  const f = fixture({ fail });
  await settled(f);
  assert.ok(['rolled_back', 'recovery_blocked'].includes(f.current().phase), `${fail}: ${f.current().phase}`);
  assert.ok((f.counts.get('launch:candidate') || 0) <= 1, fail);
  assert.ok((f.counts.get('launch:source') || 0) <= 1, fail);
  if (f.current().phase === 'rolled_back' && f.events.includes('closeSource')) {
    assert.equal(f.running(), 'source', fail);
    before(f.events, 'installPair:source', 'launch:source');
    before(f.events, 'launch:source', 'qualify:source');
    before(f.events, 'qualify:source', 'verifyFinal:source');
  }
  if (['qualify:candidate', 'restoreDashboard', 'verifyFinal:candidate'].includes(fail)) {
    before(f.events, 'closeCandidate', 'installPair:source');
  }
}

// Seed recovery at every durable nonterminal checkpoint of a successful run.
for (const seed of success.commits.filter((record) => !['ready', 'rolled_back'].includes(record.phase))) {
  const f = fixture({ recover: true, seed });
  await settled(f);
  assert.equal(f.events.includes('launch:candidate'), false, seed.phase);
  assert.ok((f.counts.get('launch:source') || 0) <= 1, seed.phase);
  assert.ok(['rolled_back', 'recovery_blocked'].includes(f.current().phase), seed.phase);
  if (f.events.includes('launch:source')) {
    before(f.events, 'verifyIntent', 'launch:source');
    before(f.events, 'requireIdle', 'launch:source');
    before(f.events, 'verifyBackups', 'launch:source');
  }
}

for (const mutate of [
  (record) => { record.coldRestart.intent.approved = false; },
  (record) => { record.coldRestart.schemaVersion = 'unknown'; },
  (record) => { record.coldRestart.intent.sourceSha256 = 'unverified'; },
  (record) => { record.coldRestart.intent.profileId = 'unapproved-profile'; },
]) {
  const seed = success.commits.find((record) => record.coldRestart.sourceClosed);
  assert.ok(seed);
  const invalid = clone(seed);
  mutate(invalid);
  const f = fixture({ recover: true, seed: invalid });
  assert.ok((await settled(f)).error);
  assert.equal(f.events.some((event) => /^(launch|installPair|restoreProfile|closeCandidate)/.test(event)), false);
}

const candidateSeed = success.commits.find((record) => record.coldRestart.candidateLaunchAdmitted);
assert.ok(candidateSeed);
for (const fail of [
  'verifyIntent', 'closeCandidate', 'requireIdle', 'verifyBackups', 'restoreProfile',
  'installPair:source', 'launch:source', 'qualify:source', 'restoreDashboard', 'verifyFinal:source',
]) {
  const f = fixture({ recover: true, seed: candidateSeed, fail });
  assert.ok((await settled(f)).error, fail);
  assert.equal(f.current().phase, 'recovery_blocked', fail);
  assert.equal(f.events.includes('launch:candidate'), false, fail);
  assert.ok((f.counts.get('launch:source') || 0) <= 1, fail);
  if (['verifyIntent', 'closeCandidate', 'requireIdle', 'verifyBackups', 'restoreProfile', 'installPair:source'].includes(fail)) {
    assert.equal(f.events.includes('launch:source'), false, fail);
  }
}

// A lost candidate launch response must be treated as possibly running.
const lostLaunch = fixture();
const actualLaunch = lostLaunch.adapters.launch;
lostLaunch.adapters.launch = async (record, selection) => {
  const result = await actualLaunch(record, selection);
  if (selection === 'candidate') throw new Error('fixture lost launch response');
  return result;
};
await settled(lostLaunch);
assert.equal(lostLaunch.current().phase, 'rolled_back');
assert.equal(lostLaunch.running(), 'source');
before(lostLaunch.events, 'closeCandidate', 'launch:source');
assert.equal(lostLaunch.counts.get('launch:candidate'), 1);

// Failed candidate closure leaves its live browser untouched by source launch.
const uncertainCandidate = fixture({ recover: true, seed: candidateSeed, fail: 'closeCandidate' });
await settled(uncertainCandidate);
assert.equal(uncertainCandidate.running(), 'candidate');
assert.equal(uncertainCandidate.events.includes('launch:source'), false);

// Restarting recovery after source launch admission reuses that exact source.
const sourceSeed = clone(candidateSeed);
sourceSeed.phase = 'cold_source_launch_admitted';
sourceSeed.coldRestart.sourceLaunchAdmitted = true;
const sourceResume = fixture({ recover: true, seed: sourceSeed });
assert.ifError((await settled(sourceResume)).error);
assert.equal(sourceResume.current().phase, 'rolled_back');
assert.equal(sourceResume.events.includes('closeCandidate'), false);
assert.equal(sourceResume.events.includes('restoreProfile'), false);
assert.equal(sourceResume.events.includes('installPair:source'), false);
assert.equal(sourceResume.counts.get('launch:source'), 1);

// A nonthrowing negative final check must never be released as success.
const negativeFinal = fixture();
negativeFinal.adapters.verifyFinal = async () => ({ verified: false });
await settled(negativeFinal);
assert.equal(negativeFinal.current().phase, 'recovery_blocked');

// Durable-write failure must never duplicate launch or claim unverified readiness.
for (let commitFault = 1; commitFault <= success.commits.length; commitFault++) {
  const f = fixture({ commitFault });
  await settled(f);
  assert.ok((f.counts.get('launch:candidate') || 0) <= 1, `commit ${commitFault}`);
  assert.ok((f.counts.get('launch:source') || 0) <= 1, `commit ${commitFault}`);
  if (f.current().phase === 'ready') assert.equal(f.running(), 'candidate');
}

// Structural admission is checked before any adapter or journal side effect.
const impossible = [
  { closeAdmitted: true, fenceAdmitted: false },
  { sourceClosed: true, closeAdmitted: false },
  { profileBackup: { fixture: true }, sourceClosed: false },
  { candidateInstallAdmitted: true, sourceClosed: false },
  { candidateInstallAdmitted: true, sourceClosed: true, closeAdmitted: true, fenceAdmitted: true, profileBackup: null },
  { candidateLaunchAdmitted: true, candidateInstallAdmitted: false },
  { sourceLaunchAdmitted: true, closeAdmitted: false },
];
for (const flag of ['fenceAdmitted', 'closeAdmitted', 'sourceClosed', 'candidateInstallAdmitted', 'candidateLaunchAdmitted', 'sourceLaunchAdmitted']) {
  for (const value of ['true', 1, null, {}, []]) impossible.push({ [flag]: value });
}
for (const coldPatch of impossible) {
  const seed = fixture().input.record;
  Object.assign(seed.coldRestart, coldPatch);
  const f = fixture({ recover: true, seed });
  assert.ok((await settled(f)).error, JSON.stringify(coldPatch));
  assert.deepEqual(f.events, [], 'malformed admission must fail before all adapter effects');
}

for (const name of ['verifyIntent', 'fenceSource', 'requireIdle', 'installPair', 'qualify', 'verifyBackups']) {
  for (const receipt of [undefined, false, {}, { verified: false, idle: false }]) {
    const f = fixture();
    const delegate = f.adapters[name];
    f.adapters[name] = async (...args) => {
      // Retain real fixture state changes when installPair ran but proof was lost.
      await delegate(...args);
      f.events.push(`negative-proof:${name}`);
      return receipt;
    };
    await settled(f);
    const record = f.current();
    assert.notEqual(record.phase, 'ready', name);
    if (['verifyIntent', 'fenceSource', 'verifyBackups'].includes(name)) {
      assert.equal(f.events.includes('closeSource'), false, name);
    }
    if (name === 'requireIdle') {
      assert.equal(f.events.includes('backupProfile'), false);
      assert.equal(f.events.some((event) => event.startsWith('installPair:')), false);
    }
    if (name === 'installPair') {
      assert.equal(f.events.some((event) => event.startsWith('launch:')), false);
    }
    if (name === 'qualify') {
      assert.equal(f.events.includes('verifyFinal:candidate'), false);
      assert.equal(f.events.includes('verifyFinal:source'), false);
    }
  }
}

// CAS journal model: a committed write whose response is lost must stop effects.
// A stale caller cannot overwrite that durable checkpoint during error handling.
for (let durableCommitFault = 1; durableCommitFault <= success.commits.length; durableCommitFault++) {
  const f = fixture({ durableCommitFault });
  assert.ok((await settled(f)).error);
  assert.equal(f.commits.length, durableCommitFault);
  const afterLoss = f.events.slice(f.events.indexOf('durable-commit-response-lost') + 1);
  assert.ok(afterLoss.every((event) => event.startsWith('commit:')), `effect after lost durable write: ${afterLoss}`);
  assert.ok((f.counts.get('launch:candidate') || 0) <= 1);
  assert.ok((f.counts.get('launch:source') || 0) <= 1);
}

console.log('Local dashboard cold restart protocol fixtures passed');
