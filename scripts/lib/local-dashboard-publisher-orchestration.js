import { isTerminalPublicationPhase } from './local-dashboard-publication-journal.js';
import { pinRetainedBrowserExpectation } from './local-dashboard-retained-browser-guard.js';
import { validatePrebuiltPublicationOptions, requireExpectedPublicationSessions } from './local-dashboard-prebuilt-candidate.js';
import { runColdRestartPublication } from './local-dashboard-cold-publication.js';

export async function runLocalDashboardPublisherOrchestration({
  options,
  report,
  adapters,
}) {
  adapters.publicationJournal.acquire();
  let maintenance = null;
  const acquireMaintenance = () => {
    maintenance ??= adapters.acquireMaintenance?.();
    if (maintenance) report.interlockCustody = maintenance.receipt;
    return maintenance;
  };
  try {
    await runLockedLocalDashboardPublisherOrchestration({ options, report, adapters, acquireMaintenance });
  } finally {
    try {
      if (maintenance) {
        const record = adapters.publicationJournal.read();
        if (!record || isTerminalPublicationPhase(record.phase)) {
          maintenance.release();
          report.interlockRestored = true;
        } else {
          maintenance.retainForRecovery();
          report.interlockRestored = false;
        }
      }
    } finally {
      adapters.publicationJournal.release();
    }
  }
  if (report.installDoctor && !report.installDoctor.degraded) {
    report.finalInstallDoctor = await adapters.verifyInstalledDoctor(report.installBin, { strict: true });
  }
}

async function runLockedLocalDashboardPublisherOrchestration({ options, report, adapters, acquireMaintenance }) {
  validateOptions(options);

  const installBin = adapters.resolveInstallBin();
  report.installBin = installBin;
  adapters.guardInstallPath(installBin);

  const existingJournal = adapters.publicationJournal.read();
  if (options.coldRestart || (existingJournal?.coldRestart && !isTerminalPublicationPhase(existingJournal.phase))) {
    await runColdRestartPublication({ options, report, adapters, acquireMaintenance, existingJournal });
    return;
  }
  if (existingJournal && !isTerminalPublicationPhase(existingJournal.phase)) {
    acquireMaintenance();
    await recoverIncompletePublication({
      adapters,
      installBin,
      journalRecord: existingJournal,
      options,
      report,
    });
    return;
  }
  if (options.recoverOnly) {
    if (options.recoverInterlockReceipt || adapters.hasMaintenanceReceipt?.()) acquireMaintenance();
    report.publicationJournal = existingJournal
      ? journalSummary(existingJournal, adapters.publicationJournal.path)
      : null;
    report.recovery = {
      transactionId: existingJournal?.transactionId ?? null,
      result: 'nothing_to_recover',
      terminalPhase: existingJournal?.phase ?? null,
    };
    return;
  }

  let pinnedRetainedBrowserExpectation = null;
  if (options.retainedBrowserExpectation) {
    let before;
    try {
      before = await adapters.verifyRetainedBrowserExpectation(installBin, {
        expectation: options.retainedBrowserExpectation,
        stage: 'pre_mutation',
      });
    } catch (error) {
      report.retainedBrowserExpectation = {
        required: true,
        pinned: null,
        before: error?.retainedBrowserEvidence ?? null,
        afterHandoff: null,
        final: null,
      };
      throw error;
    }
    pinnedRetainedBrowserExpectation = pinRetainedBrowserExpectation(before);
    report.retainedBrowserExpectation = {
      required: true,
      pinned: pinnedRetainedBrowserExpectation,
      before,
      afterHandoff: null,
      final: null,
    };
  }

  const prebuilt = validatePrebuiltPublicationOptions(options);
  if (prebuilt) requireExpectedPublicationSessions(options.expectedSessions, adapters.runtimeSessionNames());
  if (!prebuilt) {
    await adapters.buildDashboard();
    await adapters.buildRuntime({ release: options.release });
  }
  const builtBin = prebuilt
    ? await adapters.stagePrebuiltCandidate()
    : adapters.resolveBuiltBin({ release: options.release });
  if (!adapters.builtBinaryExists(builtBin)) {
    throw new Error(`Built binary was not found: ${builtBin}`);
  }
  report.builtBin = builtBin;
  const artifactEvidence = ensureArtifactEvidence(report);
  const builtSha256 = adapters.sha256File(builtBin);
  if (prebuilt && builtSha256 !== options.expectedSha256) {
    throw new Error('Staged prebuilt candidate SHA-256 mismatch');
  }
  artifactEvidence.built = {
    path: builtBin,
    sha256: builtSha256,
  };

  acquireMaintenance();
  report.service.before = adapters.serviceStatus();
  let backup = null;
  let dashboardQuiesced = false;
  let journalRecord = null;
  let handoffPreparationAdmitted = false;
  let handoffPreparationCompleted = false;
  let handoffRecoveryUncertain = false;
  let workstationProvenance = null;
  let quiesceSessions = null;
  const commitJournal = (phase, patch = {}) => {
    journalRecord = adapters.publicationJournal.commit(journalRecord, phase, patch);
    report.publicationJournal = journalSummary(journalRecord, adapters.publicationJournal.path);
    return journalRecord;
  };
  const safeCommitJournal = (phase, patch = {}) => {
    try {
      return commitJournal(phase, patch);
    } catch (error) {
      report.publicationJournalError = errorMessage(error);
      return journalRecord;
    }
  };
  try {
    const sourceExists = adapters.pathExists(installBin);
    const sourceSha256 = sourceExists ? adapters.sha256File(installBin) : null;
    artifactEvidence.source = sourceExists
      ? { path: installBin, sha256: sourceSha256 }
      : null;

    backup = await adapters.backupInstalledBinary(installBin);
    if (sourceExists && !backup) {
      throw new Error(`Installed binary backup was not created: ${installBin}`);
    }
    if (!sourceExists && backup) {
      throw new Error(`Backup was created for an absent installed binary: ${installBin}`);
    }
    if (backup) {
      report.backupPath = backup.path;
      const backupSha256 = adapters.sha256File(backup.path);
      const sourceAfterSha256 = adapters.sha256File(installBin);
      const verified = backupSha256 === sourceSha256 && sourceAfterSha256 === sourceSha256;
      artifactEvidence.backup = {
        path: backup.path,
        mode: backup.mode,
        sha256: backupSha256,
        sourceSha256,
        sourceAfterSha256,
        verified,
      };
      if (!verified) {
        throw new Error(
          `Installed binary backup hash mismatch: source=${sourceSha256} ` +
          `source_after=${sourceAfterSha256} backup=${backupSha256}`,
        );
      }
    }

    workstationProvenance = await adapters.prepareWorkstationProvenance?.({ installBin, builtBin }) ?? null;
    report.workstationProvenance = workstationProvenance;
    journalRecord = adapters.publicationJournal.create({
      installBin,
      builtBin,
      backupPath: backup?.path ?? null,
      installMode: backup?.mode ?? 0o755,
      artifactEvidence: cloneJson(artifactEvidence),
      candidateSessions: adapters.runtimeSessionNames(),
      prebuilt: prebuilt ? { sourcePath: options.prebuiltBin, expectedSha256: options.expectedSha256, expectedSessions: options.expectedSessions } : null,
      syncReferenceBinaries: options.syncReferenceBinaries,
      workstationProvenance: cloneJson(workstationProvenance),
      handoffs: [],
      retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
      smokePolicy: {
        skipSmoke: options.skipSmoke,
        smokeBrowser: options.smokeBrowser,
        requireBrowserSmoke: options.requireBrowserSmoke,
      },
      dashboardQuiesced: false,
      failure: null,
    });
    report.publicationJournal = journalSummary(journalRecord, adapters.publicationJournal.path);

    if (prebuilt) {
      requireExpectedPublicationSessions(options.expectedSessions, adapters.runtimeSessionNames());
      if (adapters.sha256File(builtBin) !== builtSha256) throw new Error('Prebuilt candidate changed before quiescence');
    }
    try {
      quiesceSessions = await adapters.prepareQuiesceSessions?.(journalRecord.candidateSessions) ?? null;
      commitJournal('quiesce_admitted', { dashboardQuiesceAdmitted: true, quiesceSessions });
      await adapters.quiesceDashboardForRuntimeHandoff();
      dashboardQuiesced = true;
      const handoffSessions = quiesceSessions
        ? await adapters.verifyQuiesceSessions(quiesceSessions)
        : prebuilt ? options.expectedSessions : journalRecord.candidateSessions;
      commitJournal('quiesced', { dashboardQuiesced: true, handoffSessions });
      commitJournal('handoff_admitted');
      if (prebuilt) requireExpectedPublicationSessions(handoffSessions, adapters.runtimeSessionNames());
      handoffPreparationAdmitted = true;
      await adapters.prepareRuntimeHandoffs(builtBin, installBin, prebuilt ? handoffSessions : null);
      handoffPreparationCompleted = true;
      commitJournal('handoff_prepared', {
        handoffs: cloneJson(report.handoffs.prepared),
      });
      try {
        await adapters.verifyRuntimeSessionsRetired?.();
      } catch (error) {
        // The candidate has not been installed. Reattach through the unchanged
        // installed runtime before the ordinary rollback/readiness path.
        try {
          await adapters.resumeRuntimeHandoffs(installBin);
        } catch (resumeError) {
          handoffRecoveryUncertain = true;
          throw new Error(`${errorMessage(error)}; installed-runtime handoff recovery failed: ${errorMessage(resumeError)}`);
        }
        throw error;
      }
      commitJournal('replacement_admitted');
      await adapters.installBinaryAtomically(
        builtBin,
        installBin,
        backup?.mode ?? 0o755,
        builtSha256,
      );
      const installedReplacementSha256 = adapters.sha256File(installBin);
      const replacementVerified = installedReplacementSha256 === builtSha256;
      artifactEvidence.replacement = {
        path: installBin,
        sourcePath: builtBin,
        expectedSha256: builtSha256,
        actualSha256: installedReplacementSha256,
        verified: replacementVerified,
      };
      if (!replacementVerified) {
        throw new Error(
          `Installed replacement hash mismatch: expected=${builtSha256} ` +
          `actual=${installedReplacementSha256}`,
        );
      }
      commitJournal('replacement_installed', {
        artifactEvidence: cloneJson(artifactEvidence),
      });
      if (workstationProvenance) {
        commitJournal('workstation_provenance_admitted');
        await adapters.applyWorkstationProvenance(workstationProvenance, { selection: 'candidate', installBin });
        commitJournal('workstation_provenance_installed');
      }
      if (options.syncReferenceBinaries) {
        commitJournal('reference_sync_admitted');
        report.referenceBinaries = await adapters.syncReferenceBinaries(builtBin);
        commitJournal('references_synced', {
          referenceBinaries: cloneJson(report.referenceBinaries),
        });
      }

      commitJournal('handoff_resume_admitted');
      await adapters.resumeRuntimeHandoffs(installBin);
      if (pinnedRetainedBrowserExpectation) {
        report.retainedBrowserExpectation.afterHandoff =
          await adapters.verifyRetainedBrowserExpectation(installBin, {
            expectation: pinnedRetainedBrowserExpectation,
            stage: 'post_handoff',
          });
      }
      commitJournal('handoffs_resumed', {
        handoffs: cloneJson(report.handoffs.prepared),
        resumedHandoffs: cloneJson(report.handoffs.resumed),
        retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
      });
      commitJournal('dashboard_restart_admitted');
      await adapters.restartOrStartDashboard(installBin, { restoring: false });
      commitJournal('dashboard_restarted');

      if (!options.skipSmoke) {
        commitJournal('readiness_admitted');
        report.smoke = await adapters.runHttpReadinessSmoke(installBin);
        report.runtimeManifest = await adapters.verifyRuntimeManifestReadback(
          installBin,
          report.smoke.runtimeManifest,
        );
        if (options.smokeBrowser) {
          report.browserSmoke = await adapters.runBrowserSmokeDiagnostic(installBin);
        }
      }
      if (pinnedRetainedBrowserExpectation) {
        report.retainedBrowserExpectation.final =
          await adapters.verifyRetainedBrowserExpectation(installBin, {
            expectation: pinnedRetainedBrowserExpectation,
            stage: 'final_readiness',
          });
      }
      if (workstationProvenance) {
        await adapters.verifyWorkstationProvenance(workstationProvenance, { selection: 'candidate', installBin });
        report.installDoctor = await adapters.verifyInstalledDoctor(installBin, { journalRecord });
      }
      commitJournal('ready', {
        artifactEvidence: cloneJson(artifactEvidence),
        retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        readiness: {
          smoke: cloneJson(report.smoke),
          runtimeManifest: cloneJson(report.runtimeManifest),
          browserSmoke: cloneJson(report.browserSmoke),
          installDoctor: cloneJson(report.installDoctor),
        },
      });
    } catch (error) {
      const handoffOutcomeUncertain = handoffRecoveryUncertain
        || (handoffPreparationAdmitted && !handoffPreparationCompleted);
      safeCommitJournal('publication_failed', {
        handoffOutcomeUncertain,
        failedAtPhase: journalRecord?.phase ?? null,
        failure: errorMessage(error),
        artifactEvidence: cloneJson(artifactEvidence),
        handoffs: cloneJson(report.handoffs.prepared),
      });
      if (handoffOutcomeUncertain) {
        safeCommitJournal('recovery_blocked', { handoffOutcomeUncertain: true });
        throw error;
      }
      const browserHandoffStarted = report.handoffs.prepared.length > 0;
      let rollbackRestartAllowed = true;
      if (!browserHandoffStarted && backup) {
        const restoration = {
          path: installBin,
          sourcePath: backup.path,
          expectedSha256: artifactEvidence.backup.sha256,
          actualSha256: null,
          status: 'pending',
          verified: false,
          error: null,
        };
        artifactEvidence.restoration = restoration;
        safeCommitJournal('rollback_admitted', {
          artifactEvidence: cloneJson(artifactEvidence),
        });
        try {
          if (!adapters.pathExists(backup.path)) {
            throw new Error(`Verified backup is no longer available: ${backup.path}`);
          }
          // A failed preflight must not replace an unchanged executable inode:
          // retained controllers may still be running that exact file.
          if (adapters.sha256File(installBin) !== restoration.expectedSha256) {
            await adapters.installBinaryAtomically(backup.path, installBin, backup.mode);
          }
          restoration.actualSha256 = adapters.sha256File(installBin);
          restoration.verified = restoration.actualSha256 === restoration.expectedSha256;
          if (!restoration.verified) {
            throw new Error(
              `Restored binary hash mismatch: expected=${restoration.expectedSha256} ` +
              `actual=${restoration.actualSha256}`,
            );
          }
          if (workstationProvenance) {
            await adapters.applyWorkstationProvenance(workstationProvenance, { selection: 'source', installBin });
          }
          restoration.status = 'verified';
          report.restoredBackup = true;
          safeCommitJournal('rollback_verified', {
            artifactEvidence: cloneJson(artifactEvidence),
          });
        } catch (restoreError) {
          restoration.status = 'failed';
          restoration.error = errorMessage(restoreError);
          report.restoreError = restoration.error;
          safeCommitJournal('rollback_failed', {
            artifactEvidence: cloneJson(artifactEvidence),
            restoreError: restoration.error,
          });
        }
      } else if (browserHandoffStarted) {
        artifactEvidence.restoration = {
          status: 'skipped',
          reason: 'browser_handoff_started',
        };
      } else {
        artifactEvidence.restoration = {
          status: 'skipped',
          reason: 'no_prior_install',
        };
      }
      const restartArtifact = inspectVerifiedRestartArtifact({
        adapters,
        installBin,
        artifactEvidence,
      });
      report.restoreRestartArtifact = restartArtifact;
      rollbackRestartAllowed = restartArtifact.verified;
      if (!rollbackRestartAllowed) {
        report.restoreRestartSkipped = 'installed_artifact_unverified_after_publication_failure';
      }
      if (rollbackRestartAllowed) {
        safeCommitJournal('rollback_restart_admitted', {
          restoreRestartArtifact: cloneJson(report.restoreRestartArtifact),
        });
        try {
          if (workstationProvenance) {
            await adapters.verifyWorkstationProvenance(workstationProvenance, {
              selection: restartArtifact.matched === 'backup' ? 'source' : 'candidate', installBin,
            });
          }
          if (dashboardQuiesced || journalRecord.dashboardQuiesceAdmitted) {
            await adapters.restartOrStartDashboard(installBin, { restoring: true });
          }
          if (pinnedRetainedBrowserExpectation) {
            report.retainedBrowserExpectation.final = await adapters.verifyRetainedBrowserExpectation(installBin, {
              expectation: pinnedRetainedBrowserExpectation, stage: 'rollback_final_readiness',
            });
          }
          if (workstationProvenance) report.installDoctor = await adapters.verifyInstalledDoctor(installBin, {
            journalRecord,
            allowSourceRollbackDegraded: !browserHandoffStarted && !handoffPreparationAdmitted,
          });
        } catch (restoreError) {
          report.restoreRestartError = errorMessage(restoreError);
        }
      }
      if (!rollbackRestartAllowed || report.restoreRestartError) {
        safeCommitJournal('recovery_blocked', {
          artifactEvidence: cloneJson(artifactEvidence),
          restoreError: report.restoreError ?? null,
          restoreRestartError: report.restoreRestartError ?? null,
          restoreRestartSkipped: report.restoreRestartSkipped ?? null,
        });
      } else if (report.restoredBackup) {
        safeCommitJournal('rolled_back', {
          artifactEvidence: cloneJson(artifactEvidence),
          retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
          originalFailure: errorMessage(error),
        });
      } else {
        safeCommitJournal('publication_failed_replacement_retained', {
          artifactEvidence: cloneJson(artifactEvidence),
          originalFailure: errorMessage(error),
        });
      }
      throw error;
    }
  } finally {
    if (dashboardQuiesced || backup !== null) {
      report.service.after = adapters.serviceStatus();
    }
  }
}

function validateOptions(options) {
  validatePrebuiltPublicationOptions(options);
  if (options.recoverInterlockReceipt && !options.recoverOnly) {
    throw new Error('--recover-interlock-receipt requires --recover-only');
  }
  if (options.recoverReplacedRetainedBrowser && !options.recoverOnly) {
    throw new Error('--recover-replaced-retained-browser requires --recover-only');
  }
  if (!options.smokeBrowser && options.requireBrowserSmoke) {
    throw new Error('--skip-browser and --require-browser-smoke cannot be used together');
  }
  if (options.skipSmoke && options.requireBrowserSmoke) {
    throw new Error('--skip-smoke and --require-browser-smoke cannot be used together');
  }
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function ensureArtifactEvidence(report) {
  report.artifactEvidence ??= {
    built: null,
    source: null,
    backup: null,
    replacement: null,
    restoration: null,
  };
  return report.artifactEvidence;
}

function inspectVerifiedRestartArtifact({ adapters, installBin, artifactEvidence }) {
  try {
    if (!adapters.pathExists(installBin)) {
      return { path: installBin, sha256: null, matched: null, verified: false };
    }
    const sha256 = adapters.sha256File(installBin);
    if (artifactEvidence.backup?.verified && sha256 === artifactEvidence.backup.sha256) {
      return { path: installBin, sha256, matched: 'backup', verified: true };
    }
    if (
      artifactEvidence.replacement?.verified
      && sha256 === artifactEvidence.replacement.actualSha256
    ) {
      return { path: installBin, sha256, matched: 'replacement', verified: true };
    }
    return { path: installBin, sha256, matched: null, verified: false };
  } catch (error) {
    return {
      path: installBin,
      sha256: null,
      matched: null,
      verified: false,
      error: errorMessage(error),
    };
  }
}

async function recoverIncompletePublication({
  adapters,
  installBin,
  journalRecord: initialJournalRecord,
  options,
  report,
}) {
  let journalRecord = initialJournalRecord;
  const commit = (phase, patch = {}) => {
    journalRecord = adapters.publicationJournal.commit(journalRecord, phase, patch);
    report.publicationJournal = journalSummary(journalRecord, adapters.publicationJournal.path);
  };

  report.publicationJournal = journalSummary(journalRecord, adapters.publicationJournal.path);
  if (journalRecord.installBin !== installBin) {
    throw new Error(
      `Publication journal install path mismatch: journal=${journalRecord.installBin} ` +
      `current=${installBin}`,
    );
  }
  const artifactEvidence = cloneJson(journalRecord.artifactEvidence);
  const workstationProvenance = journalRecord.workstationProvenance ?? null;
  report.workstationProvenance = cloneJson(workstationProvenance);
  report.artifactEvidence = artifactEvidence;
  report.builtBin = journalRecord.builtBin ?? null;
  report.backupPath = journalRecord.backupPath ?? null;
  const retainedBrowserExpectation = journalRecord.retainedBrowserExpectation ?? null;
  const smokePolicy = journalRecord.smokePolicy ?? {
    skipSmoke: options.skipSmoke,
    smokeBrowser: options.smokeBrowser,
    requireBrowserSmoke: options.requireBrowserSmoke,
  };
  let pinnedRetainedBrowserExpectation = null;
  if (retainedBrowserExpectation?.required === true) {
    pinnedRetainedBrowserExpectation = retainedBrowserExpectation.pinned
      ?? pinRetainedBrowserExpectation(retainedBrowserExpectation.before);
    report.retainedBrowserExpectation = cloneJson(retainedBrowserExpectation);
  }
  report.service.before = adapters.serviceStatus();
  let partialSourceRecoveryVerified = false;
  try {
    const discoveredHandoffs = adapters.discoverPreparedRuntimeHandoffs(
      Array.isArray(journalRecord.candidateSessions) ? journalRecord.candidateSessions : [],
    );
    report.handoffs.prepared = mergePreparedHandoffs(
      Array.isArray(journalRecord.handoffs) ? journalRecord.handoffs : [],
      discoveredHandoffs,
    );
    const knownPreHandoffFailure = journalRecord.handoffOutcomeUncertain === false
      && ['prepared', 'quiesce_admitted', 'quiesced', 'handoff_admitted'].includes(journalRecord.failedAtPhase)
      && report.handoffs.prepared.length === 0
      && (journalRecord.resumedHandoffs ?? []).length === 0;
    if (!knownPreHandoffFailure && (journalRecord.handoffOutcomeUncertain || journalRecord.phase === 'handoff_admitted' || journalRecord.failedAtPhase === 'handoff_admitted')) {
      const expected = journalRecord.handoffSessions ?? journalRecord.candidateSessions ?? [];
      const observed = report.handoffs.prepared.map(handoff => handoff.sessionName);
      const exactHandoffSet = expected.length > 0
        && new Set(observed).size === observed.length
        && JSON.stringify([...expected].sort()) === JSON.stringify([...observed].sort());
      const partialSourceRecovery = !exactHandoffSet
        && journalRecord.handoffOutcomeUncertain === true
        && journalRecord.failedAtPhase === 'handoff_admitted'
        && /^Daemon session '[A-Za-z0-9._-]+' did not exit for executable handoff$/.test(journalRecord.failure ?? '')
        && artifactEvidence.replacement == null
        && artifactEvidence.backup?.verified === true
        && artifactEvidence.source?.sha256 === artifactEvidence.backup.sha256
        && await adapters.verifyPartialSourceHandoffInventory?.({
          expectedSessions: expected,
          preparedSessions: observed,
          sourceSha256: artifactEvidence.source.sha256,
          installBin,
        }) === true;
      if (!exactHandoffSet && !partialSourceRecovery) {
        throw new Error('Publication handoff outcome is uncertain; inspect the exact sessions before recovery');
      }
      partialSourceRecoveryVerified = partialSourceRecovery;
    }

    if (!adapters.pathExists(installBin)) {
      commit('recovery_blocked', { recoveryError: 'installed_binary_missing' });
      throw new Error(`Publication recovery cannot find installed binary: ${installBin}`);
    }
    const installedSha256 = adapters.sha256File(installBin);
    const expectedReplacementSha256 = artifactEvidence.replacement?.verified === true
      ? artifactEvidence.replacement.actualSha256
      : artifactEvidence.built?.sha256;
    const matchesReplacement = typeof expectedReplacementSha256 === 'string'
      && installedSha256 === expectedReplacementSha256;
    const matchesBackup = artifactEvidence.backup?.verified === true
      && installedSha256 === artifactEvidence.backup.sha256;
    if (!matchesReplacement && !matchesBackup) {
      commit('recovery_blocked', {
        recoveryError: 'installed_artifact_unverified',
        installedSha256,
      });
      throw new Error(
        `Publication recovery found an unverified installed binary: ${installedSha256}`,
      );
    }

    if (matchesReplacement && artifactEvidence.replacement?.verified !== true) {
      artifactEvidence.replacement = {
        path: installBin,
        sourcePath: journalRecord.builtBin ?? null,
        expectedSha256: expectedReplacementSha256,
        actualSha256: installedSha256,
        verified: true,
        recoveredFromBuiltEvidence: true,
      };
      commit('recovery_replacement_verified', {
        artifactEvidence: cloneJson(artifactEvidence),
      });
    }

    // Recover the matching pair before any daemon or dashboard restart. The
    // journal stores both exact manifest snapshots before binary replacement.
    if (workstationProvenance) {
      commit('recovery_workstation_provenance_admitted');
      await adapters.applyWorkstationProvenance(workstationProvenance, {
        selection: matchesReplacement ? 'candidate' : 'source', installBin,
      });
      commit('recovery_workstation_provenance_verified');
    }

    const recordedResumedHandoffs = Array.isArray(journalRecord.resumedHandoffs)
      ? journalRecord.resumedHandoffs
      : [];
    if (
      report.handoffs.prepared.length > 0
      && !handoffsAlreadyResumed(report.handoffs.prepared, recordedResumedHandoffs)
    ) {
      commit('recovery_handoff_resume_admitted', {
        handoffs: cloneJson(report.handoffs.prepared),
      });
      await adapters.resumeRuntimeHandoffs(installBin);
      commit('recovery_handoffs_resumed', {
        handoffs: cloneJson(report.handoffs.prepared),
        resumedHandoffs: cloneJson(report.handoffs.resumed),
      });
    } else if (recordedResumedHandoffs.length > 0) {
      report.handoffs.resumed = cloneJson(recordedResumedHandoffs);
    }

    if (pinnedRetainedBrowserExpectation) {
      try {
        report.retainedBrowserExpectation.afterHandoff =
          await adapters.verifyRetainedBrowserExpectation(installBin, {
            expectation: pinnedRetainedBrowserExpectation,
            stage: 'recovery_post_handoff',
          });
      } catch (error) {
        const originalFailure = error?.retainedBrowserEvidence ?? {
          verified: false,
          reason: 'retained_browser_expectation_failed',
          message: errorMessage(error),
        };
        if (options.recoverReplacedRetainedBrowser !== journalRecord.transactionId) {
          commit('recovery_blocked', {
            recoveryError: 'retained_browser_expectation_failed',
            retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
          });
          throw error;
        }
        let replacement;
        try {
          replacement = await adapters.verifyRecoveredRetainedBrowserExpectation(
            installBin,
            {
              expectation: pinnedRetainedBrowserExpectation,
              stage: 'recovery_replacement_admission',
            },
          );
        } catch (replacementError) {
          commit('recovery_blocked', {
            recoveryError: 'retained_browser_replacement_unverified',
            retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
            retainedBrowserRecovery: {
              acknowledgedTransactionId: options.recoverReplacedRetainedBrowser,
              originalFailure,
              replacementFailure: replacementError?.retainedBrowserEvidence
                ?? { message: errorMessage(replacementError) },
            },
          });
          throw replacementError;
        }
        report.retainedBrowserExpectation.replacementAfterHandoff = replacement;
        report.retainedBrowserRecovery = {
          acknowledgedTransactionId: options.recoverReplacedRetainedBrowser,
          originalFailure,
          replacement,
        };
        pinnedRetainedBrowserExpectation = pinRetainedBrowserExpectation(replacement);
        commit('recovery_retained_browser_replacement_admitted', {
          retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
          retainedBrowserRecovery: cloneJson(report.retainedBrowserRecovery),
        });
      }
    }

    if (matchesReplacement) {
      if (journalRecord.syncReferenceBinaries ?? options.syncReferenceBinaries) {
        commit('recovery_reference_sync_admitted');
        report.referenceBinaries = await adapters.syncReferenceBinaries(installBin);
        commit('recovery_references_synced', {
          referenceBinaries: cloneJson(report.referenceBinaries),
        });
      }
      commit('recovery_dashboard_restart_admitted');
      await adapters.restartOrStartDashboard(installBin, { restoring: false });
      commit('recovery_dashboard_restarted');
      if (!smokePolicy.skipSmoke) {
        commit('recovery_readiness_admitted');
        report.smoke = await adapters.runHttpReadinessSmoke(installBin);
        report.runtimeManifest = await adapters.verifyRuntimeManifestReadback(
          installBin,
          report.smoke.runtimeManifest,
        );
        if (smokePolicy.smokeBrowser) {
          report.browserSmoke = await adapters.runBrowserSmokeDiagnostic(installBin);
        }
      }
      if (pinnedRetainedBrowserExpectation) {
        try {
          report.retainedBrowserExpectation.final =
            await adapters.verifyRetainedBrowserExpectation(installBin, {
              expectation: pinnedRetainedBrowserExpectation,
              stage: 'recovery_final_readiness',
            });
        } catch (error) {
          commit('recovery_blocked', {
            recoveryError: 'retained_browser_expectation_failed',
            retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
          });
          throw error;
        }
      }
      if (workstationProvenance) {
        await adapters.verifyWorkstationProvenance(workstationProvenance, { selection: 'candidate', installBin });
        report.installDoctor = await adapters.verifyInstalledDoctor(installBin, { journalRecord });
      }
      commit('recovered_ready', {
        artifactEvidence: cloneJson(artifactEvidence),
        retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        readiness: {
          smoke: cloneJson(report.smoke),
          runtimeManifest: cloneJson(report.runtimeManifest),
          browserSmoke: cloneJson(report.browserSmoke),
          installDoctor: cloneJson(report.installDoctor),
        },
      });
      report.recovery = {
        transactionId: journalRecord.transactionId,
        result: 'recovered_ready',
        installedSha256,
      };
      return;
    }

    if (journalRecord.dashboardQuiesceAdmitted || journalRecord.dashboardQuiesced) {
      commit('recovery_rollback_restart_admitted');
      await adapters.restartOrStartDashboard(installBin, { restoring: true });
    }
    if (pinnedRetainedBrowserExpectation) {
      try {
        report.retainedBrowserExpectation.final =
          await adapters.verifyRetainedBrowserExpectation(installBin, {
            expectation: pinnedRetainedBrowserExpectation,
            stage: 'recovery_final_readiness',
          });
      } catch (error) {
        commit('recovery_blocked', {
          recoveryError: 'retained_browser_expectation_failed',
          retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        });
        throw error;
      }
    }
    if (workstationProvenance) {
      await adapters.verifyWorkstationProvenance(workstationProvenance, { selection: 'source', installBin });
      try {
        report.installDoctor = await adapters.verifyInstalledDoctor(installBin, {
          journalRecord, allowSourceRollbackDegraded: knownPreHandoffFailure,
        });
      } catch (error) {
        if (!partialSourceRecoveryVerified) throw error;
        report.installDoctor = await adapters.inspectRollbackDoctorDegradation?.({
          installBin,
          journalRecord,
          expectedSha256: installedSha256,
        });
        if (report.installDoctor?.degraded !== true) throw error;
      }
    }
    commit('recovered_rolled_back', {
      artifactEvidence: cloneJson(artifactEvidence),
      retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
      installedSha256,
      installDoctor: cloneJson(report.installDoctor),
    });
    report.restoredBackup = true;
    report.recovery = {
      transactionId: journalRecord.transactionId,
      result: 'recovered_rolled_back',
      degraded: report.installDoctor?.degraded === true,
      installedSha256,
    };
  } finally {
    report.service.after = adapters.serviceStatus();
  }
}

function mergePreparedHandoffs(recorded, discovered) {
  const merged = new Map();
  for (const handoff of [...recorded, ...discovered]) {
    if (!handoff || typeof handoff.sessionName !== 'string') {
      throw new Error('Publication recovery found an invalid runtime handoff record');
    }
    const existing = merged.get(handoff.sessionName);
    if (
      existing
      && (
        existing.browserPid !== handoff.browserPid
        || existing.cdpUrl !== handoff.cdpUrl
        || existing.runtimeProfile !== handoff.runtimeProfile
      )
    ) {
      throw new Error(
        `Publication recovery found conflicting handoff evidence for '${handoff.sessionName}'`,
      );
    }
    merged.set(handoff.sessionName, { ...existing, ...handoff });
  }
  return [...merged.values()].sort((left, right) =>
    left.sessionName.localeCompare(right.sessionName));
}

function handoffsAlreadyResumed(prepared, resumed) {
  const resumedBySession = new Map(
    resumed.map((handoff) => [handoff?.sessionName, handoff]),
  );
  return prepared.every((handoff) => {
    const evidence = resumedBySession.get(handoff.sessionName);
    return evidence
      && (handoff.browserPid == null || evidence.browserPid === handoff.browserPid)
      && (!handoff.cdpUrl || evidence.cdpUrl === handoff.cdpUrl);
  });
}

function journalSummary(record, path) {
  return {
    path,
    transactionId: record.transactionId,
    revision: record.revision,
    phase: record.phase,
    terminal: record.terminal,
  };
}

function cloneJson(value) {
  return value == null ? value : structuredClone(value);
}
