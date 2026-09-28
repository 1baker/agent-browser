// Isolated on-disk journal/profile/pair fixtures; runtime custody is simulated.
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runLocalDashboardPublisherOrchestration } from './lib/local-dashboard-publisher-orchestration.js';
import { runColdRestartPublication } from './lib/local-dashboard-cold-publication.js';
import { createLocalDashboardPublicationJournal } from './lib/local-dashboard-publication-journal.js';
import { prepareColdProfileBackup, verifyColdProfileBackup, restoreColdProfileBackup } from './lib/local-dashboard-cold-profile.js';

const root = fs.mkdtempSync(join(tmpdir(), 'cold-publication-test-'));
const digest = (path) => createHash('sha256').update(fs.readFileSync(path)).digest('hex');
let cases = 0;

async function scenario(fault = null) {
  const directory = fs.mkdtempSync(join(root, 'case-'));
  const installBin = join(directory, 'installed');
  const builtBin = join(directory, 'candidate');
  const backupPath = join(directory, 'source-backup');
  const manifestPath = join(directory, 'manifest');
  const profilePath = join(directory, 'profile');
  const backupRoot = join(directory, 'backups');
  fs.mkdirSync(profilePath, { mode: 0o700 });
  fs.mkdirSync(backupRoot, { mode: 0o700 });
  fs.writeFileSync(join(profilePath, 'Preferences'), 'original fixture state');
  fs.writeFileSync(installBin, 'old fixture executable');
  fs.writeFileSync(builtBin, 'candidate fixture executable');
  const sourceSha256 = digest(installBin);
  const candidateSha256 = digest(builtBin);
  fs.writeFileSync(manifestPath, sourceSha256);
  const journal = createLocalDashboardPublicationJournal({ journalPath: join(directory, 'publication.json') });
  const events = [];
  let running = 'source';
  let fenced = false;
  let injected = false;
  const trip = (where) => {
    if (fault === where && !injected) { injected = true; throw new Error(`injected ${where}`); }
  };
  const idle = () => { assert.equal(running, null); assert.equal(fenced, true); return true; };
  const options = {
    coldRestart: true, prebuiltBin: builtBin, expectedSha256: candidateSha256,
    expectedSessions: ['default'], syncReferenceBinaries: false, skipSmoke: false,
    retainedBrowserExpectation: { sessionName: 'default', profileId: 'fixture', url: 'https://example.invalid/' },
  };
  const report = { service: {} };
  const runtime = {
    captureIntent: async () => ({ approved: true, sessionName: 'default', profilePath, sourceSha256, candidateSha256 }),
    verifyIntent: async (current) => {
      assert.equal(current.coldRestart.intent.profilePath, profilePath);
      return { verified: true };
    },
    fenceSource: async () => { events.push('fence_source'); fenced = true; trip('fence'); return { verified: true }; },
    restoreSourceFence: async () => { fenced = false; return { restored: true }; },
    closeSource: async () => { assert.equal(fenced, true); assert.equal(running, 'source'); running = null; return { closed: true }; },
    requireIdle: async () => ({ idle: idle() }),
    backupProfile: async () => prepareColdProfileBackup({ profilePath, backupRoot, requireProfileIdle: idle }),
    verifyBackups: async (current) => {
      if (current.coldRestart.profileBackup) verifyColdProfileBackup(current.coldRestart.profileBackup);
      return { verified: true };
    },
    launch: async (_current, selection) => {
      idle();
      assert.equal(fs.readFileSync(manifestPath, 'utf8'), digest(installBin));
      running = selection;
      events.push(`launch_${selection}`);
      if (selection === 'candidate') {
        fs.writeFileSync(join(profilePath, 'Preferences'), 'candidate fixture changes');
        trip('lost_launch');
      }
      return { selection };
    },
    qualify: async (_current, selection) => {
      assert.equal(running, selection);
      if (selection === 'candidate') trip('qualification');
      return { verified: true };
    },
    fenceCandidate: async () => { events.push('fence_candidate'); fenced = true; return { verified: true }; },
    closeCandidate: async () => { assert.equal(fenced, true); assert.ok(running === null || running === 'candidate'); running = null; },
    restoreProfile: async (current) => restoreColdProfileBackup(current.coldRestart.profileBackup, { requireProfileIdle: idle }),
    verifyFinal: async (_current, selection) => {
      assert.equal(running, selection);
      return { verified: true, stage: 'fixture_final' };
    },
  };
  const adapters = {
    publicationJournal: journal, coldRuntime: runtime,
    resolveInstallBin: () => installBin, guardInstallPath: () => {},
    runtimeSessionNames: () => ['default'],
    verifyRetainedBrowserExpectation: async () => ({ verified: true }),
    stagePrebuiltCandidate: async () => builtBin, sha256File: digest,
    acquireMaintenance: () => fault === 'no_custody' ? null : ({ receipt: { id: 'isolated-fixture' },
      release: () => events.push('interlock_release'), retainForRecovery: () => events.push('interlock_retain') }),
    serviceStatus: () => ({ fixture: true }),
    backupInstalledBinary: async () => { fs.copyFileSync(installBin, backupPath); return { path: backupPath, mode: 0o700 }; },
    prepareWorkstationProvenance: async () => ({ sourceSha256, candidateSha256 }),
    installBinaryAtomically: async (source, target, _mode, expected) => {
      idle(); assert.equal(digest(source), expected); fs.copyFileSync(source, target);
      if (source === builtBin) trip('after_binary');
    },
    applyWorkstationProvenance: async (_receipt, { selection }) => {
      fs.writeFileSync(manifestPath, selection === 'source' ? sourceSha256 : candidateSha256);
      if (selection === 'candidate') trip('after_manifest');
    },
    verifyWorkstationProvenance: async (_receipt, { selection }) => {
      const expected = selection === 'source' ? sourceSha256 : candidateSha256;
      assert.equal(digest(installBin), expected);
      assert.equal(fs.readFileSync(manifestPath, 'utf8'), expected);
      return true;
    },
    restartOrStartDashboard: async () => { fenced = false; events.push('dashboard_restart'); },
    runHttpReadinessSmoke: async () => { if (running === 'candidate') trip('readiness'); return { runtimeManifest: {} }; },
    verifyRuntimeManifestReadback: async () => ({ fixture: true }),
    verifyInstalledDoctor: async () => ({ verified: true }),
  };
  if (fault === 'no_custody') {
    await assert.rejects(runLocalDashboardPublisherOrchestration({ options, report, adapters }), /positive maintenance custody/);
    assert.equal(journal.read(), null);
    assert.equal(running, 'source');
    assert.equal(digest(installBin), sourceSha256);
    assert.equal(events.length, 0);
    cases += 1;
    return;
  }
  await runLocalDashboardPublisherOrchestration({ options, report, adapters });
  const result = journal.read();
  assert.equal(result.phase, fault ? 'rolled_back' : 'ready', result.coldRestart.failure);
  assert.equal(result.terminal, true);
  assert.equal(result.retainedBrowserExpectation.final.verified, true);
  assert.equal(events.at(-1), 'interlock_release');
  assert.equal(digest(installBin), fault ? sourceSha256 : candidateSha256);
  assert.equal(fs.readFileSync(manifestPath, 'utf8'), digest(installBin));
  assert.equal(fs.readFileSync(join(profilePath, 'Preferences'), 'utf8'), fault ? 'original fixture state' : 'candidate fixture changes');
  assert.ok(events.filter((event) => event === 'launch_candidate').length <= 1);
  if (['lost_launch', 'qualification', 'readiness'].includes(fault)) {
    assert.ok(events.indexOf('fence_candidate') < events.indexOf('launch_source'));
  }
  cases += 1;
}

try {
  for (const fault of [null, 'no_custody', 'fence', 'after_binary', 'after_manifest', 'lost_launch', 'qualification', 'readiness']) await scenario(fault);
  const required = ['captureIntent', 'verifyIntent', 'fenceSource', 'restoreSourceFence',
    'closeSource', 'requireIdle', 'backupProfile', 'verifyBackups', 'launch', 'qualify',
    'fenceCandidate', 'closeCandidate', 'restoreProfile', 'verifyFinal'];
  let calls = 0;
  const coldRuntime = Object.fromEntries(required.map((name) => [name, () => { calls += 1; }]));
  await assert.rejects(runColdRestartPublication({ options: {}, report: {},
    adapters: { coldRuntime }, acquireMaintenance: () => { calls += 1; }, existingJournal: null,
  }), /mandatory maintenance custody/);
  assert.equal(calls, 0);
  cases += 1;
  for (const existingJournal of [null, { phase: 'ready', coldRestart: {} }]) {
    await assert.rejects(runColdRestartPublication({
      options: { coldRestart: true, recoverOnly: true }, report: {},
      adapters: { coldRuntime, acquireMaintenance: () => { calls += 1; } },
      acquireMaintenance: () => { calls += 1; }, existingJournal,
    }), /No nonterminal cold publication/);
    assert.equal(calls, 0);
    cases += 1;
  }
  // Live CLI intentionally supplies no adapter until its custody implementation
  // is reviewed. Even programmatic opt-in must fail before maintenance changes.
  let mutated = false;
  const journal = createLocalDashboardPublicationJournal({ journalPath: join(root, 'no-adapter.json') });
  await assert.rejects(runLocalDashboardPublisherOrchestration({
    options: { coldRestart: true }, report: {}, adapters: {
      publicationJournal: journal, resolveInstallBin: () => '/fixture/installed', guardInstallPath: () => {},
      acquireMaintenance: () => { mutated = true; },
    },
  }), /no reviewed exact-custody runtime adapter/);
  assert.equal(mutated, false);
  assert.equal(journal.read(), null);
  cases += 1;
  console.log(`Cold publication on-disk fixtures passed: ${cases}`);
} finally {
  // Only the unique fixture tree is disposable. No operator profile is touched.
  fs.rmSync(root, { recursive: true, force: true });
}
