#!/usr/bin/env node

import assert from 'node:assert/strict';
import './test-local-dashboard-session-display-environment.js';
import './test-local-dashboard-partial-source-handoff.js';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  runLocalDashboardPublisherOrchestration,
} from './lib/local-dashboard-publisher-orchestration.js';
import {
  createLocalDashboardPublicationJournal,
} from './lib/local-dashboard-publication-journal.js';

const roots = new Set();

try {
  const reacquired = createFixture({ prepareHandoff: true });
  reacquired.input.adapters.verifyRuntimeSessionsRetired = () => {
    throw new Error('fixture session reacquired before replacement');
  };
  await assert.rejects(runLocalDashboardPublisherOrchestration(reacquired.input), /session reacquired/);
  assert.equal(reacquired.actions.includes('install-replacement'), false);
  assert.equal(reacquired.actions.includes('resume-handoffs'), true);
  assert.ok(reacquired.actions.indexOf('resume-handoffs') < reacquired.actions.indexOf('restart:rollback'));
  const failedResume = createFixture({ prepareHandoff: true, faultAt: 'resume-handoffs' });
  failedResume.input.adapters.verifyRuntimeSessionsRetired = () => {
    throw new Error('fixture session reacquired before replacement');
  };
  await assert.rejects(runLocalDashboardPublisherOrchestration(failedResume.input), /handoff recovery failed/);
  assert.equal(failedResume.actions.includes('install-replacement'), false);
  assert.equal(failedResume.actions.some((action) => action.startsWith('restart:')), false);
  assert.equal(failedResume.publicationJournal.read().phase, 'recovery_blocked');

  await runQuiesceAndKnownRollbackScenarios();
  await runWorkstationProvenanceScenarios();
  await runMaintenanceScenarios();
  await runPrebuiltScenarios();
  await runSuccessScenario();
  await runRetainedGuardSuccessScenario();
  await runRetainedGuardPreMutationFailureScenario();
  await runRetainedGuardPostHandoffFailureScenario();
  await runPreHandoffRollbackScenario();
  await runCommittedHandoffFailureScenario();
  await runCommittedHandoffReplacementMismatchScenario();
  await runRollbackRestartFailureScenario();
  await runBackupHashMismatchScenario();
  await runReplacementHashMismatchScenario();
  await runMissingBackupRollbackScenario();
  await runRestoreCopyFailureScenario();
  await runRestoreHashMismatchScenario();
  await runPreMutationFailureScenario();
  await runReplacementRecoveryScenario();
  await runPostInstallPreCheckpointRecoveryScenario();
  await runRolledBackRecoveryScenario();
  await runDiscoveredHandoffRecoveryScenario();
  await runAlreadyResumedRecoveryScenario();
  await runRecoveryPreservesOriginalSkipBrowserPolicyScenario();
  await runRetainedGuardRecoveryFailureScenario();
  await runRetainedGuardReplacementRecoveryScenario();
  await runUnverifiedRecoveryScenario();
  await runRecoverOnlyNoopScenario();
} finally {
  for (const root of roots) rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard publisher orchestration fixture passed');

async function runQuiesceAndKnownRollbackScenarios() {
  {
    const f = createFixture();
    const originalInode = statSync(f.installBin).ino;
    f.input.adapters.prepareQuiesceSessions = () => { throw new Error('backend still busy'); };
    await assert.rejects(runLocalDashboardPublisherOrchestration(f.input), /backend still busy/);
    assert.equal(statSync(f.installBin).ino, originalInode);
    for (const action of ['quiesce', 'prepare-handoffs', 'install-replacement', 'restore-backup', 'restart:rollback', 'restart:normal']) {
      assert.ok(!f.actions.includes(action), `pre-stop refusal must not perform ${action}`);
    }
    const record = f.publicationJournal.read();
    assert.equal(record.failedAtPhase, 'prepared');
    assert.equal(record.handoffOutcomeUncertain, false);
    assert.notEqual(record.dashboardQuiesceAdmitted, true);
    assert.equal(record.phase, 'rolled_back');
  }
  for (const phase of ['prepared', 'quiesce_admitted', 'quiesced', 'handoff_admitted']) {
    const f = createFixture();
    const originalInode = statSync(f.installBin).ino;
    seedIncompleteJournal(f, { installed: 'source', phase, candidateSessions: ['retained-fixture'] });
    f.publicationJournal.acquire();
    try {
      f.publicationJournal.commit(f.publicationJournal.read(), 'recovery_blocked', {
        failedAtPhase: phase, handoffOutcomeUncertain: false, workstationProvenance: { fixture: true },
      });
    } finally { f.publicationJournal.release(); }
    f.input.adapters.applyWorkstationProvenance = (_record, context) => assert.equal(context.selection, 'source');
    f.input.adapters.verifyWorkstationProvenance = (_record, context) => assert.equal(context.selection, 'source');
    let doctorCalls = 0;
    f.input.adapters.verifyInstalledDoctor = (_binary, context) => {
      doctorCalls++;
      assert.equal(context.allowSourceRollbackDegraded, true, phase);
      assert.ok(!context.strict, 'degraded source rollback must not claim a strict final doctor');
      return { success: false, rawSuccess: false, degraded: true, repairRequired: true };
    };
    await runLocalDashboardPublisherOrchestration(f.input);
    assert.equal(doctorCalls, 1);
    assert.equal(f.publicationJournal.read().phase, 'recovered_rolled_back');
    assert.equal(f.publicationJournal.read().installDoctor.degraded, true);
    assert.equal(statSync(f.installBin).ino, originalInode);
    assert.ok(!f.actions.includes('prepare-handoffs'));
    assert.ok(!f.actions.includes('resume-handoffs'));
    assert.ok(!f.actions.includes('restore-backup'));
    assert.equal(f.actions.includes('restart:rollback'), phase !== 'prepared', `${phase}: restart only after quiesce admission`);
  }
  {
    const f = createFixture();
    const originalInode = statSync(f.installBin).ino;
    f.input.adapters.verifyQuiesceSessions = () => { throw new Error('quiesce drift'); };
    f.input.adapters.prepareQuiesceSessions = () => ({ fixture: true });
    await assert.rejects(runLocalDashboardPublisherOrchestration(f.input), /quiesce drift/);
    assert.equal(statSync(f.installBin).ino, originalInode, 'unchanged source must keep its executable inode');
    assert.ok(!f.actions.includes('prepare-handoffs'));
    assert.ok(f.actions.includes('restart:rollback'));
    assert.equal(f.publicationJournal.read().phase, 'rolled_back');
  }
  {
    const f = createFixture();
    let stopped = false;
    f.input.options.prebuiltBin = f.builtBin;
    f.input.options.expectedSha256 = f.replacementSha256;
    f.input.options.expectedSessions = ['default', 'dashboard-service-backend'];
    f.input.options.syncReferenceBinaries = false;
    f.input.options.smokeBrowser = false;
    f.input.adapters.stagePrebuiltCandidate = () => f.builtBin;
    f.input.adapters.runtimeSessionNames = () => stopped ? ['default'] : ['default', 'dashboard-service-backend'];
    f.input.adapters.prepareQuiesceSessions = () => ({ fixture: true });
    f.input.adapters.quiesceDashboardForRuntimeHandoff = () => { stopped = true; };
    f.input.adapters.verifyQuiesceSessions = () => ['default'];
    f.input.adapters.prepareRuntimeHandoffs = (_built, _installed, expected) => assert.deepEqual(expected, ['default']);
    await runLocalDashboardPublisherOrchestration(f.input);
    assert.deepEqual(f.publicationJournal.read().handoffSessions, ['default']);
  }
  for (const uncertainty of [false, true, undefined]) {
    const f = createFixture();
    seedIncompleteJournal(f, { installed: 'source', phase: 'handoff_admitted', candidateSessions: ['retained-fixture'] });
    f.publicationJournal.acquire();
    try {
      f.publicationJournal.commit(f.publicationJournal.read(), 'recovery_blocked', {
        failedAtPhase: 'handoff_admitted', handoffOutcomeUncertain: uncertainty,
      });
    } finally { f.publicationJournal.release(); }
    if (uncertainty === false) {
      await runLocalDashboardPublisherOrchestration(f.input);
      assert.equal(f.publicationJournal.read().phase, 'recovered_rolled_back');
      assert.ok(!f.actions.includes('prepare-handoffs'));
    } else {
      await assert.rejects(runLocalDashboardPublisherOrchestration(f.input), /handoff outcome is uncertain/);
      assert.ok(!f.actions.includes('restart:rollback'));
    }
  }
}

async function runWorkstationProvenanceScenarios() {
  const configure = fixture => {
    let selection = 'source';
    const record = { fixture: true };
    fixture.input.adapters.prepareWorkstationProvenance = () => record;
    fixture.input.adapters.applyWorkstationProvenance = (_record, options) => {
      assert.deepEqual(fixture.publicationJournal.read().workstationProvenance, record,
        'both manifest snapshots must be journaled before pair mutation');
      selection = options.selection;
      fixture.actions.push(`provenance:${selection}`);
    };
    fixture.input.adapters.verifyWorkstationProvenance = (_record, options) => {
      assert.equal(selection, options.selection);
      fixture.actions.push(`provenance-verified:${selection}`);
    };
    fixture.input.adapters.verifyInstalledDoctor = (_binary, context) => {
      assert.equal(fixture.publicationJournal.lockStatus().live, !context.strict,
        'strict doctor runs only after publication lock release');
      fixture.actions.push('install-doctor');
      return { success: true };
    };
    return record;
  };
  for (const faultAt of [null, 'http-readiness']) {
    const f = createFixture({ faultAt });
    configure(f);
    if (faultAt) await assert.rejects(runLocalDashboardPublisherOrchestration(f.input), /http-readiness/);
    else await runLocalDashboardPublisherOrchestration(f.input);
    assert.ok(f.actions.indexOf('provenance:candidate') > f.actions.indexOf('install-replacement'));
    assert.ok(f.actions.indexOf('provenance:candidate') < f.actions.indexOf('resume-handoffs'));
    assert.ok(f.actions.includes('install-doctor'));
    if (faultAt) {
      assert.ok(f.actions.indexOf('provenance:source') < f.actions.indexOf('restart:rollback'));
      assert.equal(f.publicationJournal.read().phase, 'rolled_back');
    }
  }
  for (const installed of ['source', 'replacement']) {
    const f = createFixture();
    const record = configure(f);
    seedIncompleteJournal(f, { installed, phase: 'replacement_admitted', replacementEvidence: false });
    f.publicationJournal.acquire();
    try { f.publicationJournal.commit(f.publicationJournal.read(), 'replacement_admitted', { workstationProvenance: record }); }
    finally { f.publicationJournal.release(); }
    await runLocalDashboardPublisherOrchestration(f.input);
    const selected = installed === 'source' ? 'source' : 'candidate';
    const restart = installed === 'source' ? 'restart:rollback' : 'restart:normal';
    assert.ok(f.actions.indexOf(`provenance:${selected}`) < f.actions.indexOf(restart));
    assert.ok(!f.actions.includes('prepare-handoffs'), 'recovery must not replay handoff preparation');
    assert.ok(f.actions.includes('install-doctor'));
  }
  {
    const f = createFixture({ prepareHandoff: true });
    configure(f);
    f.input.adapters.applyWorkstationProvenance = () => { throw new Error('manifest write failed'); };
    await assert.rejects(runLocalDashboardPublisherOrchestration(f.input), /manifest write failed/);
    assert.ok(!f.actions.includes('resume-handoffs'));
    assert.ok(!f.actions.includes('restart:normal'));
    assert.ok(!f.actions.includes('restart:rollback'), 'do not restart a mismatched binary/manifest pair');
    assert.equal(f.publicationJournal.read().phase, 'recovery_blocked');
  }
}

async function runMaintenanceScenarios() {
  const crashed = createFixture();
  seedIncompleteJournal(crashed, { phase: 'handoff_admitted', installed: 'source', candidateSessions: ['retained-fixture'] });
  crashed.publicationJournal.acquire();
  try {
    crashed.publicationJournal.commit(crashed.publicationJournal.read(), 'publication_failed', { failedAtPhase: 'handoff_admitted' });
  } finally { crashed.publicationJournal.release(); }
  await assert.rejects(runLocalDashboardPublisherOrchestration(crashed.input), /handoff outcome is uncertain/);
  assert.ok(!crashed.actions.includes('restart:rollback'));
  for (const failure of [null, 'prepare-handoffs', 'resume-handoffs']) {
    const fixture = createFixture({ prepareHandoff: true, faultAt: failure });
    const maintenance = [];
    fixture.input.adapters.acquireMaintenance = () => {
      maintenance.push('acquire');
      return { receipt: { id: 'fixture' }, release: () => maintenance.push('release'), retainForRecovery: () => maintenance.push('retain') };
    };
    if (failure) await assert.rejects(runLocalDashboardPublisherOrchestration(fixture.input));
    else await runLocalDashboardPublisherOrchestration(fixture.input);
    assert.deepEqual(maintenance, ['acquire', failure ? 'retain' : 'release']);
    if (failure === 'prepare-handoffs') {
      assert.equal(fixture.publicationJournal.read().handoffOutcomeUncertain, true);
      assert.ok(!fixture.actions.includes('restore-backup'));
      const priorPrepares = fixture.actions.filter(action => action === 'prepare-handoffs').length;
      await assert.rejects(runLocalDashboardPublisherOrchestration(fixture.input), /handoff outcome is uncertain/);
      assert.equal(fixture.actions.filter(action => action === 'prepare-handoffs').length, priorPrepares);
    }
    assert.equal(fixture.publicationJournal.lockStatus().present, false);
  }
  const refusal = createFixture();
  refusal.input.adapters.acquireMaintenance = () => { throw new Error('busy interlock'); };
  await assert.rejects(runLocalDashboardPublisherOrchestration(refusal.input), /busy interlock/);
  assert.ok(!refusal.actions.includes('quiesce'));
  assert.ok(!refusal.actions.includes('backup'));
  const failedRelease = createFixture();
  failedRelease.input.adapters.acquireMaintenance = () => ({ release: () => { throw new Error('restore timer failed'); } });
  await assert.rejects(runLocalDashboardPublisherOrchestration(failedRelease.input), /restore timer failed/);
  assert.equal(failedRelease.publicationJournal.lockStatus().present, false);
}

async function runPrebuiltScenarios() {
  const configure = fixture => {
    Object.assign(fixture.input.options, {
      prebuiltBin: fixture.builtBin,
      expectedSha256: fixture.replacementSha256,
      expectedSessions: [],
      syncReferenceBinaries: false,
      smokeBrowser: false,
    });
    fixture.input.adapters.stagePrebuiltCandidate = () => fixture.builtBin;
  };
  const success = createFixture();
  configure(success);
  await runLocalDashboardPublisherOrchestration(success.input);
  for (const forbidden of ['build-dashboard', 'build-runtime:debug', 'sync-references', 'browser-smoke']) {
    assert.ok(!success.actions.includes(forbidden), forbidden);
  }
  assert.ok(success.actions.includes('http-readiness'));
  assert.equal(success.report.artifactEvidence.replacement.actualSha256, success.replacementSha256);
  assert.equal(success.publicationJournal.read().syncReferenceBinaries, false);

  for (const fault of ['hash', 'sessions', 'late-sessions', 'candidate-drift']) {
    const fixture = createFixture();
    configure(fixture);
    if (fault === 'hash') fixture.input.options.expectedSha256 = '0'.repeat(64);
    if (fault === 'sessions') fixture.input.adapters.runtimeSessionNames = () => ['unreviewed'];
    if (fault === 'late-sessions') {
      let reads = 0;
      fixture.input.adapters.runtimeSessionNames = () => ++reads > 2 ? ['unreviewed'] : [];
    }
    if (fault === 'candidate-drift') {
      const backup = fixture.input.adapters.backupInstalledBinary;
      fixture.input.adapters.backupInstalledBinary = () => {
        const result = backup();
        writeFileSync(fixture.builtBin, 'changed after admission');
        return result;
      };
    }
    await assert.rejects(runLocalDashboardPublisherOrchestration(fixture.input), /SHA-256 mismatch|inventory changed|candidate changed/);
    assert.ok(!fixture.actions.includes('quiesce'), fault);
    assert.ok(!fixture.actions.includes('install-replacement'), fault);
    assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  }
}

async function runSuccessScenario() {
  const fixture = createFixture();
  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.deepEqual(fixture.actions, [
    'resolve-install',
    'guard-install',
    'build-dashboard',
    'build-runtime:debug',
    'resolve-built:debug',
    'built-exists',
    'hash:built',
    'service-status:before',
    'exists:installed',
    'hash:installed',
    'backup',
    'hash:backup',
    'hash:installed',
    'quiesce',
    'prepare-handoffs',
    'install-replacement',
    'hash:installed',
    'sync-references',
    'resume-handoffs',
    'restart:normal',
    'http-readiness',
    'manifest-readback',
    'browser-smoke',
    'service-status:after',
  ]);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(readFileSync(fixture.backupPath, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(statSync(fixture.backupPath).mode & 0o777, 0o751);
  assert.equal(fixture.report.backupPath, fixture.backupPath);
  assert.equal(fixture.report.restoredBackup, undefined);
  assert.deepEqual(fixture.report.referenceBinaries, [{ path: 'reference', synced: true }]);
  assert.equal(fixture.report.browserSmoke.status, 'passed');
  assert.equal(fixture.report.service.after.sequence, 2);
  assert.deepEqual(fixture.report.artifactEvidence, {
    built: { path: fixture.builtBin, sha256: fixture.replacementSha256 },
    source: { path: fixture.installBin, sha256: fixture.originalSha256 },
    backup: {
      path: fixture.backupPath,
      mode: 0o751,
      sha256: fixture.originalSha256,
      sourceSha256: fixture.originalSha256,
      sourceAfterSha256: fixture.originalSha256,
      verified: true,
    },
    replacement: {
      path: fixture.installBin,
      sourcePath: fixture.builtBin,
      expectedSha256: fixture.replacementSha256,
      actualSha256: fixture.replacementSha256,
      verified: true,
    },
    restoration: null,
  });
}

async function runRetainedGuardSuccessScenario() {
  const fixture = createFixture({ prepareHandoff: true, retainedExpectation: true });
  await runLocalDashboardPublisherOrchestration(fixture.input);

  const pre = fixture.actions.indexOf('retained-guard:pre_mutation');
  const build = fixture.actions.indexOf('build-dashboard');
  const backup = fixture.actions.indexOf('backup');
  const post = fixture.actions.indexOf('retained-guard:post_handoff');
  const resume = fixture.actions.indexOf('resume-handoffs');
  const final = fixture.actions.indexOf('retained-guard:final_readiness');
  const smoke = fixture.actions.indexOf('browser-smoke');
  assert.ok(pre >= 0 && pre < build, 'retained guard must run before any build');
  assert.ok(pre < backup, 'retained guard must run before backup or quiescence');
  assert.ok(post > resume, 'retained guard must rerun after handoff resume');
  assert.ok(final > smoke, 'retained guard must rerun at final readiness');
  assert.equal(fixture.report.retainedBrowserExpectation.before.verified, true);
  assert.equal(fixture.report.retainedBrowserExpectation.afterHandoff.verified, true);
  assert.equal(fixture.report.retainedBrowserExpectation.final.verified, true);
  assert.equal(
    fixture.publicationJournal.read().retainedBrowserExpectation.final.verified,
    true,
  );
}

async function runRetainedGuardPreMutationFailureScenario() {
  const fixture = createFixture({
    retainedExpectation: true,
    retainedGuardFaultAt: 'pre_mutation',
  });
  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:pre_mutation',
  );

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('build-runtime:debug'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.includes('quiesce'), false);
  assert.equal(fixture.actions.includes('prepare-handoffs'), false);
  assert.equal(fixture.publicationJournal.read(), null);
}

async function runRetainedGuardPostHandoffFailureScenario() {
  const fixture = createFixture({
    prepareHandoff: true,
    retainedExpectation: true,
    retainedGuardFaultAt: 'post_handoff',
  });
  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:post_handoff',
  );

  assert.equal(fixture.report.handoffs.prepared.length, 1);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.includes('retained-guard:final_readiness'), false);
  assert.equal(
    fixture.publicationJournal.read().phase,
    'publication_failed_replacement_retained',
  );
}

async function runPreHandoffRollbackScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness' });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(fixture.report.restoredBackup, true);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    path: fixture.installBin,
    sourcePath: fixture.backupPath,
    expectedSha256: fixture.originalSha256,
    actualSha256: fixture.originalSha256,
    status: 'verified',
    verified: true,
    error: null,
  });
  assert.equal(fixture.actions.filter((action) => action === 'install-replacement').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restore-backup').length, 1);
  assert.ok(
    fixture.actions.indexOf('restart:rollback') > fixture.actions.indexOf('restore-backup'),
    'rollback restart must occur after the original binary is restored',
  );
  assert.equal(fixture.actions.at(-1), 'service-status:after');
}

async function runCommittedHandoffFailureScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', prepareHandoff: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(readFileSync(fixture.backupPath, 'utf8'), 'original-runtime\n');
  assert.equal(statSync(fixture.installBin).mode & 0o777, 0o751);
  assert.equal(fixture.report.restoredBackup, undefined);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
  assert.equal(fixture.report.handoffs.prepared.length, 1);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    status: 'skipped',
    reason: 'browser_handoff_started',
  });
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
}

async function runCommittedHandoffReplacementMismatchScenario() {
  const fixture = createFixture({ corruptReplacement: true, prepareHandoff: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed replacement hash mismatch:');

  assert.equal(fixture.report.artifactEvidence.replacement.verified, false);
  assert.deepEqual(fixture.report.artifactEvidence.restoration, {
    status: 'skipped',
    reason: 'browser_handoff_started',
  });
  assert.equal(fixture.report.restoreRestartArtifact.verified, false);
  assert.equal(
    fixture.report.restoreRestartSkipped,
    'installed_artifact_unverified_after_publication_failure',
  );
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 0);
}

async function runRollbackRestartFailureScenario() {
  const fixture = createFixture({
    faultAt: 'http-readiness',
    rollbackRestartFault: 'fault:rollback-restart',
  });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.restoredBackup, true);
  assert.equal(fixture.report.restoreRestartError, 'fault:rollback-restart');
  assert.equal(fixture.actions.at(-1), 'service-status:after');
}

async function runBackupHashMismatchScenario() {
  const fixture = createFixture({ corruptBackup: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed binary backup hash mismatch:');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.artifactEvidence.backup.verified, false);
  assert.equal(fixture.actions.includes('quiesce'), false);
  assert.equal(fixture.actions.includes('install-replacement'), false);
  assert.equal(fixture.actions.includes('restart:rollback'), false);
}

async function runReplacementHashMismatchScenario() {
  const fixture = createFixture({ corruptReplacement: true });
  await assertRejectsWithOriginalFault(fixture.input, 'Installed replacement hash mismatch:');

  assert.equal(fixture.report.artifactEvidence.replacement.verified, false);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'verified');
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(fixture.report.restoredBackup, true);
}

async function runMissingBackupRollbackScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', removeBackupBeforeRollback: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.match(fixture.report.restoreError, /Verified backup is no longer available/);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.error, fixture.report.restoreError);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
  assert.equal(fixture.actions.includes('restore-backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runRestoreCopyFailureScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', restoreCopyFault: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.equal(fixture.report.restoreError, 'fault:restore-copy');
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.error, 'fault:restore-copy');
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
  assert.equal(fixture.report.restoreRestartArtifact.matched, 'replacement');
  assert.equal(fixture.report.restoreRestartArtifact.verified, true);
  assert.equal(fixture.actions.filter((action) => action === 'restore-backup').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runRestoreHashMismatchScenario() {
  const fixture = createFixture({ faultAt: 'http-readiness', corruptRestore: true });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:http-readiness');

  assert.equal(fixture.report.restoredBackup, undefined);
  assert.match(fixture.report.restoreError, /Restored binary hash mismatch/);
  assert.equal(fixture.report.artifactEvidence.restoration.status, 'failed');
  assert.equal(fixture.report.artifactEvidence.restoration.verified, false);
  assert.notEqual(
    fixture.report.artifactEvidence.restoration.actualSha256,
    fixture.report.artifactEvidence.restoration.expectedSha256,
  );
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'corrupted-restoration\n');
  assert.equal(fixture.report.restoreRestartArtifact.verified, false);
  assert.equal(
    fixture.report.restoreRestartSkipped,
    'installed_artifact_unverified_after_publication_failure',
  );
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 0);
}

async function runPreMutationFailureScenario() {
  const fixture = createFixture({ faultAt: 'build-runtime' });
  await assertRejectsWithOriginalFault(fixture.input, 'fault:build-runtime');

  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
  assert.equal(existsSync(fixture.backupPath), false);
  assert.deepEqual(fixture.actions, [
    'resolve-install',
    'guard-install',
    'build-dashboard',
    'build-runtime:debug',
  ]);
  assert.equal(fixture.report.service.before, null);
  assert.equal(fixture.report.service.after, null);
}

async function runReplacementRecoveryScenario() {
  const fixture = createFixture();
  fixture.input.options.recoverOnly = true;
  seedIncompleteJournal(fixture, { phase: 'replacement_installed', installed: 'replacement' });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_ready');
  assert.equal(fixture.publicationJournal.read().terminal, true);
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'replacement-runtime\n');
}

async function runRecoverOnlyNoopScenario() {
  const fixture = createFixture();
  fixture.input.options.recoverOnly = true;
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.deepEqual(fixture.report.recovery, {
    transactionId: null,
    result: 'nothing_to_recover',
    terminalPhase: null,
  });
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.includes('backup'), false);
  assert.equal(fixture.actions.some((action) => action.startsWith('restart:')), false);
  assert.equal(existsSync(fixture.installBin), true);
}

async function runPostInstallPreCheckpointRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, {
    phase: 'handoff_prepared',
    installed: 'replacement',
    replacementEvidence: false,
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.equal(fixture.report.artifactEvidence.replacement.verified, true);
  assert.equal(fixture.report.artifactEvidence.replacement.recoveredFromBuiltEvidence, true);
  assert.equal(fixture.actions.filter((action) => action === 'sync-references').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_ready');
}

async function runRolledBackRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, { phase: 'quiesced', installed: 'source' });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_rolled_back');
  assert.equal(fixture.publicationJournal.read().phase, 'recovered_rolled_back');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
  assert.equal(readFileSync(fixture.installBin, 'utf8'), 'original-runtime\n');
}

async function runDiscoveredHandoffRecoveryScenario() {
  const handoff = fixtureHandoff();
  const fixture = createFixture({ discoveredHandoffs: [handoff] });
  seedIncompleteJournal(fixture, {
    phase: 'handoff_admitted',
    installed: 'source',
    candidateSessions: [handoff.sessionName],
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_rolled_back');
  assert.deepEqual(fixture.report.handoffs.prepared, [handoff]);
  assert.equal(fixture.actions.filter((action) => action === 'resume-handoffs').length, 1);
  assert.equal(fixture.actions.filter((action) => action === 'restart:rollback').length, 1);
}

async function runAlreadyResumedRecoveryScenario() {
  const handoff = fixtureHandoff();
  const resumed = { ...handoff, alreadyResumed: true };
  const fixture = createFixture();
  seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    handoffs: [handoff],
    resumedHandoffs: [resumed],
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.deepEqual(fixture.report.handoffs.resumed, [resumed]);
  assert.equal(fixture.actions.includes('resume-handoffs'), false);
  assert.equal(fixture.actions.filter((action) => action === 'restart:normal').length, 1);
}

async function runRecoveryPreservesOriginalSkipBrowserPolicyScenario() {
  const fixture = createFixture();
  fixture.input.options.recoverOnly = true;
  seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    smokePolicy: {
      skipSmoke: false,
      smokeBrowser: false,
      requireBrowserSmoke: false,
    },
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  assert.equal(fixture.report.recovery.result, 'recovered_ready');
  assert.equal(fixture.actions.includes('http-readiness'), true);
  assert.equal(fixture.actions.includes('browser-smoke'), false);
}

async function runRetainedGuardRecoveryFailureScenario() {
  const handoff = fixtureHandoff();
  const fixture = createFixture({ retainedGuardFaultAt: 'recovery_post_handoff' });
  seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    handoffs: [handoff],
    retainedBrowserExpectation: fixtureRetainedExpectationRecord(),
  });
  fixture.actions.length = 0;

  await assertRejectsWithOriginalFault(
    fixture.input,
    'fault:retained-guard:recovery_post_handoff',
  );

  assert.equal(fixture.actions.includes('resume-handoffs'), true);
  assert.equal(fixture.actions.includes('restart:normal'), false);
  assert.equal(fixture.publicationJournal.read().phase, 'recovery_blocked');
  assert.equal(
    fixture.publicationJournal.read().recoveryError,
    'retained_browser_expectation_failed',
  );
}

async function runRetainedGuardReplacementRecoveryScenario() {
  const handoff = fixtureHandoff();
  const fixture = createFixture({ retainedGuardFaultAt: 'recovery_post_handoff' });
  const record = seedIncompleteJournal(fixture, {
    phase: 'publication_failed_replacement_retained',
    installed: 'replacement',
    handoffs: [handoff],
    retainedBrowserExpectation: fixtureRetainedExpectationRecord(),
  });
  fixture.input.options.recoverOnly = true;
  fixture.input.options.recoverReplacedRetainedBrowser = record.transactionId;
  fixture.input.adapters.verifyRecoveredRetainedBrowserExpectation = (_path, { expectation, stage }) => ({
    required: true,
    verified: true,
    stage,
    reason: 'retained_browser_exact_match',
    expected: expectation,
    observed: {
      sessionName: expectation.sessionName,
      browserId: expectation.browserId,
      browserPid: 8765,
      cdpUrl: 'ws://127.0.0.1:9333/devtools/browser/recovered',
      profileId: expectation.profileId,
      health: 'ready',
      targetId: 'target-recovered',
      url: expectation.url,
      title: 'Recovered fixture conversation',
      cdpTargetCount: 1,
    },
  });
  fixture.actions.length = 0;

  await runLocalDashboardPublisherOrchestration(fixture.input);

  const recovered = fixture.publicationJournal.read();
  assert.equal(recovered.phase, 'recovered_ready');
  assert.equal(recovered.retainedBrowserRecovery.acknowledgedTransactionId, record.transactionId);
  assert.equal(recovered.retainedBrowserRecovery.replacement.observed.browserPid, 8765);
  assert.equal(recovered.retainedBrowserExpectation.final.verified, true);
}

async function runUnverifiedRecoveryScenario() {
  const fixture = createFixture();
  seedIncompleteJournal(fixture, { phase: 'replacement_installed', installed: 'replacement' });
  writeFileSync(fixture.installBin, 'unknown-runtime\n');
  fixture.actions.length = 0;

  await assertRejectsWithOriginalFault(
    fixture.input,
    'Publication recovery found an unverified installed binary:',
  );

  assert.equal(fixture.publicationJournal.read().phase, 'recovery_blocked');
  assert.equal(fixture.actions.includes('build-dashboard'), false);
  assert.equal(fixture.actions.some((action) => action.startsWith('restart:')), false);
}

function createFixture({
  corruptBackup = false,
  corruptReplacement = false,
  corruptRestore = false,
  faultAt = null,
  prepareHandoff = false,
  removeBackupBeforeRollback = false,
  restoreCopyFault = false,
  rollbackRestartFault = null,
  discoveredHandoffs = [],
  retainedExpectation = false,
  retainedGuardFaultAt = null,
} = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-dashboard-publisher-orchestration-'));
  roots.add(root);
  const installBin = join(root, 'installed-agent-browser');
  const builtBin = join(root, 'built-agent-browser');
  const backupPath = join(root, 'installed-agent-browser.backup');
  const journalPath = join(root, 'publication', 'local-dashboard.json');
  writeFileSync(installBin, 'original-runtime\n', { mode: 0o751 });
  writeFileSync(builtBin, 'replacement-runtime\n', { mode: 0o755 });
  const originalSha256 = hashFile(installBin);
  const replacementSha256 = hashFile(builtBin);

  const actions = [];
  const report = createReport();
  const publicationJournal = createLocalDashboardPublicationJournal({ journalPath });
  let serviceStatusCount = 0;
  const act = (name, value) => {
    actions.push(name);
    if (faultAt === value) throw new Error(`fault:${value}`);
  };

  const adapters = {
    resolveInstallBin: () => {
      act('resolve-install', 'resolve-install');
      return installBin;
    },
    guardInstallPath: () => act('guard-install', 'guard-install'),
    buildDashboard: () => act('build-dashboard', 'build-dashboard'),
    buildRuntime: ({ release }) => act(`build-runtime:${release ? 'release' : 'debug'}`, 'build-runtime'),
    resolveBuiltBin: ({ release }) => {
      act(`resolve-built:${release ? 'release' : 'debug'}`, 'resolve-built');
      return builtBin;
    },
    builtBinaryExists: (path) => {
      act('built-exists', 'built-exists');
      return existsSync(path);
    },
    pathExists: (path) => {
      actions.push(`exists:${path === backupPath ? 'backup' : 'installed'}`);
      return existsSync(path);
    },
    sha256File: (path) => {
      actions.push(`hash:${path === builtBin ? 'built' : path === backupPath ? 'backup' : 'installed'}`);
      return hashFile(path);
    },
    serviceStatus: () => {
      serviceStatusCount += 1;
      actions.push(`service-status:${serviceStatusCount === 1 ? 'before' : 'after'}`);
      return { sequence: serviceStatusCount };
    },
    backupInstalledBinary: () => {
      act('backup', 'backup');
      copyFileSync(installBin, backupPath);
      const mode = statSync(installBin).mode & 0o777;
      chmodSync(backupPath, mode);
      if (corruptBackup) writeFileSync(backupPath, 'corrupted-backup\n');
      return { path: backupPath, mode };
    },
    quiesceDashboardForRuntimeHandoff: () => act('quiesce', 'quiesce'),
    prepareRuntimeHandoffs: () => {
      act('prepare-handoffs', 'prepare-handoffs');
      if (prepareHandoff) report.handoffs.prepared.push({ sessionName: 'retained-fixture' });
    },
    installBinaryAtomically: (source, target, mode) => {
      const restoring = source === backupPath;
      act(restoring ? 'restore-backup' : 'install-replacement', restoring ? 'restore-backup' : 'install-replacement');
      if (restoring && restoreCopyFault) throw new Error('fault:restore-copy');
      const staged = join(root, restoring ? 'restore.next' : 'replacement.next');
      copyFileSync(source, staged);
      if (restoring && corruptRestore) writeFileSync(staged, 'corrupted-restoration\n');
      if (!restoring && corruptReplacement) writeFileSync(staged, 'corrupted-replacement\n');
      chmodSync(staged, mode);
      renameSync(staged, target);
    },
    syncReferenceBinaries: () => {
      act('sync-references', 'sync-references');
      return [{ path: 'reference', synced: true }];
    },
    resumeRuntimeHandoffs: () => {
      act('resume-handoffs', 'resume-handoffs');
      for (const handoff of report.handoffs.prepared) {
        report.handoffs.resumed.push({ ...handoff });
      }
    },
    restartOrStartDashboard: (_path, { restoring }) => {
      actions.push(`restart:${restoring ? 'rollback' : 'normal'}`);
      if (restoring && rollbackRestartFault) throw new Error(rollbackRestartFault);
      if (!restoring && faultAt === 'restart-normal') throw new Error('fault:restart-normal');
    },
    runHttpReadinessSmoke: () => {
      if (removeBackupBeforeRollback) rmSync(backupPath, { force: true });
      act('http-readiness', 'http-readiness');
      return { runtimeManifest: { schemaVersion: 'fixture.v1' } };
    },
    verifyRuntimeManifestReadback: (_path, manifest) => {
      act('manifest-readback', 'manifest-readback');
      return { ...manifest, verified: true };
    },
    verifyRetainedBrowserExpectation: (_path, { expectation, stage }) => {
      actions.push(`retained-guard:${stage}`);
      if (retainedGuardFaultAt === stage) {
        throw new Error(`fault:retained-guard:${stage}`);
      }
      return {
        required: true,
        verified: true,
        stage,
        reason: 'retained_browser_exact_match',
        expected: expectation,
        observed: {
          sessionName: expectation.sessionName,
          browserId: 'session:retained-fixture',
          browserPid: 5678,
          cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
          profileId: 'fixture-profile',
          health: 'ready',
          targetId: 'target-fixture',
          url: 'https://example.test/conversation',
          title: 'Fixture conversation',
          cdpTargetCount: 1,
        },
      };
    },
    runBrowserSmokeDiagnostic: () => {
      act('browser-smoke', 'browser-smoke');
      return { status: 'passed' };
    },
    runtimeSessionNames: () => [],
    discoverPreparedRuntimeHandoffs: (candidateSessions) => discoveredHandoffs
      .filter((handoff) => candidateSessions.includes(handoff.sessionName))
      .map((handoff) => ({ ...handoff })),
    publicationJournal,
  };

  return {
    actions,
    backupPath,
    builtBin,
    installBin,
    journalPath,
    originalSha256,
    replacementSha256,
    publicationJournal,
    input: {
      adapters,
      options: {
        release: false,
        recoverOnly: false,
        requireBrowserSmoke: false,
        skipSmoke: false,
        smokeBrowser: true,
        syncReferenceBinaries: true,
        retainedBrowserExpectation: retainedExpectation
          ? {
            sessionName: 'retained-fixture',
            targetId: 'target-fixture',
            url: 'https://example.test/conversation',
          }
          : null,
      },
      report,
    },
    report,
  };
}

function createReport() {
  return {
    installBin: null,
    builtBin: null,
    backupPath: null,
    service: { before: null, after: null },
    smoke: null,
    browserSmoke: { status: 'pending' },
    runtimeManifest: null,
    artifactEvidence: {
      built: null,
      source: null,
      backup: null,
      replacement: null,
      restoration: null,
    },
    referenceBinaries: [],
    handoffs: { prepared: [], resumed: [] },
  };
}

async function assertRejectsWithOriginalFault(input, message) {
  await assert.rejects(
    () => runLocalDashboardPublisherOrchestration(input),
    (error) => error instanceof Error && error.message.startsWith(message),
  );
}

function hashFile(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function seedIncompleteJournal(fixture, {
  candidateSessions = [],
  handoffs = [],
  installed,
  phase,
  replacementEvidence = true,
  resumedHandoffs = [],
  retainedBrowserExpectation = null,
  smokePolicy = null,
}) {
  copyFileSync(fixture.installBin, fixture.backupPath);
  chmodSync(fixture.backupPath, 0o751);
  if (installed === 'replacement') {
    copyFileSync(fixture.builtBin, fixture.installBin);
    chmodSync(fixture.installBin, 0o751);
  }
  const artifactEvidence = {
    built: { path: fixture.builtBin, sha256: fixture.replacementSha256 },
    source: { path: fixture.installBin, sha256: fixture.originalSha256 },
    backup: {
      path: fixture.backupPath,
      mode: 0o751,
      sha256: fixture.originalSha256,
      sourceSha256: fixture.originalSha256,
      sourceAfterSha256: fixture.originalSha256,
      verified: true,
    },
    replacement: replacementEvidence
      ? {
        path: fixture.installBin,
        sourcePath: fixture.builtBin,
        expectedSha256: fixture.replacementSha256,
        actualSha256: fixture.replacementSha256,
        verified: true,
      }
      : null,
    restoration: null,
  };
  fixture.publicationJournal.acquire();
  try {
    let record = fixture.publicationJournal.create({
      installBin: fixture.installBin,
      builtBin: fixture.builtBin,
      backupPath: fixture.backupPath,
      installMode: 0o751,
      artifactEvidence,
      candidateSessions,
      handoffs,
      resumedHandoffs,
      retainedBrowserExpectation,
      smokePolicy,
      dashboardQuiesceAdmitted: phase !== 'prepared',
      dashboardQuiesced: phase !== 'prepared',
      failure: null,
    });
    if (phase !== 'prepared') {
      record = fixture.publicationJournal.commit(record, phase, {
        handoffs,
        resumedHandoffs,
      });
    }
    return record;
  } finally {
    fixture.publicationJournal.release();
  }
}

function fixtureHandoff() {
  return {
    sessionName: 'retained-fixture',
    daemonPid: 1234,
    browserPid: 5678,
    cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
    runtimeProfile: 'fixture-profile',
    handoffPath: '/tmp/retained-fixture.handoff.json',
  };
}

function fixtureRetainedExpectationRecord() {
  const observed = {
    sessionName: 'retained-fixture',
    browserId: 'session:retained-fixture',
    browserPid: 5678,
    cdpUrl: 'ws://127.0.0.1:9222/devtools/browser/fixture',
    profileId: 'fixture-profile',
    health: 'ready',
    targetId: 'target-fixture',
    url: 'https://example.test/conversation',
    title: 'Fixture conversation',
    cdpTargetCount: 1,
  };
  return {
    required: true,
    pinned: {
      sessionName: observed.sessionName,
      browserId: observed.browserId,
      browserPid: observed.browserPid,
      cdpUrl: observed.cdpUrl,
      profileId: observed.profileId,
      targetId: observed.targetId,
      url: observed.url,
    },
    before: {
      required: true,
      verified: true,
      stage: 'pre_mutation',
      reason: 'retained_browser_exact_match',
      expected: {
        sessionName: observed.sessionName,
        targetId: observed.targetId,
        url: observed.url,
      },
      observed,
    },
    afterHandoff: null,
    final: null,
  };
}
