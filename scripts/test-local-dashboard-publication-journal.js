#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  createLocalDashboardPublicationJournal,
  inspectLocalDashboardPublicationJournal,
  LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
} from './lib/local-dashboard-publication-journal.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-dashboard-publication-journal-'));
const journalPath = join(root, 'state', 'publication.json');
const timestamps = [
  '2026-08-15T01:00:00.000Z',
  '2026-08-15T01:00:01.000Z',
  '2026-08-15T01:00:02.000Z',
  '2026-08-15T01:00:03.000Z',
];
const now = () => timestamps.shift() || '2026-08-15T01:00:04.000Z';
const ownerPid = process.pid;

try {
  const journal = createLocalDashboardPublicationJournal({ journalPath, now, ownerPid });
  journal.acquire();
  let record;
  try {
    record = journal.create({ installBin: '/tmp/agent-browser', marker: 'fixture' });
    assert.equal(record.schemaVersion, LOCAL_DASHBOARD_PUBLICATION_SCHEMA);
    assert.equal(record.phase, 'prepared');
    assert.equal(record.revision, 1);
    assert.equal(record.terminal, false);
    assert.equal(statSync(journalPath).mode & 0o777, 0o600);
    assert.equal(JSON.parse(readFileSync(journalPath, 'utf8')).transactionId, record.transactionId);

    const competing = createLocalDashboardPublicationJournal({ journalPath, ownerPid });
    assert.throws(() => competing.acquire(), /already active in process/);

    const staleRevision = record;
    record = journal.commit(record, 'replacement_installed', { replacementSha256: 'a'.repeat(64) });
    assert.equal(record.revision, 2);
    assert.equal(record.phase, 'replacement_installed');
    assert.throws(
      () => journal.commit(staleRevision, 'ready'),
      /journal revision conflict/,
    );

    record = journal.commit(record, 'ready');
    assert.equal(record.revision, 3);
    assert.equal(record.terminal, true);
  } finally {
    journal.release();
  }

  const lockPath = `${journalPath}.lock`;
  writeFileSync(lockPath, '999999999\n', { mode: 0o600 });
  const staleLockRecovery = createLocalDashboardPublicationJournal({ journalPath, ownerPid });
  staleLockRecovery.acquire();
  staleLockRecovery.release();

  writeFileSync(journalPath, '{not-json\n');
  chmodSync(journalPath, 0o600);
  const invalid = createLocalDashboardPublicationJournal({ journalPath, ownerPid });
  invalid.acquire();
  try {
    assert.throws(() => invalid.read(), /journal is invalid/);
  } finally {
    invalid.release();
  }

  const crashJournalPath = join(root, 'crash', 'publication.json');
  const journalModuleUrl = new URL('./lib/local-dashboard-publication-journal.js', import.meta.url).href;
  const crashWorker = spawnSync(process.execPath, ['--input-type=module', '-e', `
const module = await import(process.env.FIXTURE_JOURNAL_MODULE_URL);
const journal = module.createLocalDashboardPublicationJournal({
  journalPath: process.env.FIXTURE_JOURNAL_PATH,
});
journal.acquire();
let record = journal.create({ installBin: '/tmp/agent-browser', crashFixture: true });
record = journal.commit(record, 'replacement_admitted');
process.exit(71);
`], {
    env: {
      ...process.env,
      FIXTURE_JOURNAL_MODULE_URL: journalModuleUrl,
      FIXTURE_JOURNAL_PATH: crashJournalPath,
    },
    encoding: 'utf8',
    timeout: 5000,
  });
  assert.equal(crashWorker.status, 71, crashWorker.stderr || crashWorker.stdout);
  assert.equal(existsSync(`${crashJournalPath}.lock`), true, 'crashed worker must leave exact lock evidence');
  const recovered = createLocalDashboardPublicationJournal({ journalPath: crashJournalPath });
  recovered.acquire();
  try {
    let crashRecord = recovered.read();
    assert.equal(crashRecord.phase, 'replacement_admitted');
    assert.equal(crashRecord.terminal, false);
    crashRecord = recovered.commit(crashRecord, 'recovered_rolled_back');
    assert.equal(crashRecord.terminal, true);
  } finally {
    recovered.release();
  }

  const absentPath = join(root, 'status-absent', 'publication.json');
  const absentJournal = createLocalDashboardPublicationJournal({ journalPath: absentPath });
  assert.deepEqual(
    inspectLocalDashboardPublicationJournal({
      journal: absentJournal,
      pathExists: () => false,
      sha256File: () => { throw new Error('status must not hash an absent artifact'); },
    }),
    {
      schemaVersion: LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
      journalPath: absentPath,
      exists: false,
      lock: {
        path: `${absentPath}.lock`,
        present: false,
        ownerPid: null,
        live: false,
        stale: false,
      },
      transaction: null,
      installedArtifact: null,
      recoverable: false,
      recommendedAction: 'none',
    },
  );
  assert.equal(existsSync(join(root, 'status-absent')), false, 'read-only status must not create its directory');

  const statusPath = join(root, 'status', 'publication.json');
  const installedPath = join(root, 'status', 'agent-browser');
  mkdirSync(join(root, 'status'), { recursive: true });
  writeFileSync(installedPath, 'verified-source\n', { mode: 0o755 });
  const installedSha256 = hashFile(installedPath);
  const statusJournal = createLocalDashboardPublicationJournal({ journalPath: statusPath });
  statusJournal.acquire();
  let statusRecord;
  try {
    statusRecord = statusJournal.create({
      installBin: installedPath,
      artifactEvidence: {
        built: { sha256: 'b'.repeat(64) },
        backup: { verified: true, sha256: installedSha256 },
        replacement: null,
      },
      candidateSessions: ['retained'],
      handoffs: [{ sessionName: 'retained' }],
      resumedHandoffs: [],
    });
    const activeStatus = inspectLocalDashboardPublicationJournal({
      journal: statusJournal,
      pathExists: existsSync,
      sha256File: hashFile,
    });
    assert.equal(activeStatus.lock.live, true);
    assert.equal(activeStatus.recommendedAction, 'wait_for_active_publisher');
  } finally {
    statusJournal.release();
  }
  const recoverableStatus = inspectLocalDashboardPublicationJournal({
    journal: statusJournal,
    pathExists: existsSync,
    sha256File: hashFile,
  });
  assert.equal(recoverableStatus.installedArtifact.classification, 'backup');
  assert.equal(recoverableStatus.recoverable, true);
  assert.equal(recoverableStatus.recommendedAction, 'recover_only');
  assert.equal(recoverableStatus.transaction.preparedHandoffCount, 1);
  assert.equal(recoverableStatus.transaction.retainedBrowserExpectationRequired, false);
  assert.equal(recoverableStatus.transaction.retainedBrowserExpectationVerified, null);

  writeFileSync(installedPath, 'unknown-source\n');
  const unknownStatus = inspectLocalDashboardPublicationJournal({
    journal: statusJournal,
    pathExists: existsSync,
    sha256File: hashFile,
  });
  assert.equal(unknownStatus.installedArtifact.classification, 'unknown');
  assert.equal(unknownStatus.recoverable, false);
  assert.equal(unknownStatus.recommendedAction, 'investigate_installed_artifact');

  const guardedJournal = createLocalDashboardPublicationJournal({
    journalPath: join(root, 'guarded-publication.json'),
  });
  guardedJournal.acquire();
  try {
    let guardedRecord = guardedJournal.create({
      installBin: installedPath,
      artifactEvidence: {
        backup: { verified: true, sha256: hashFile(installedPath) },
      },
      retainedBrowserExpectation: {
        required: true,
        before: { verified: true, stage: 'pre_mutation' },
        final: { verified: false, stage: 'final_readiness' },
      },
    });
    guardedRecord = guardedJournal.commit(guardedRecord, 'ready');
    assert.equal(guardedRecord.terminal, true);
  } finally {
    guardedJournal.release();
  }
  const guardedStatus = inspectLocalDashboardPublicationJournal({
    journal: guardedJournal,
    pathExists: existsSync,
    sha256File: hashFile,
  });
  assert.equal(guardedStatus.recoverable, false);
  assert.equal(guardedStatus.recommendedAction, 'investigate_retained_browser');
  assert.equal(guardedStatus.transaction.retainedBrowserExpectationRequired, true);
  assert.equal(guardedStatus.transaction.retainedBrowserExpectationVerified, false);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard publication journal fixture passed');

function hashFile(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}
