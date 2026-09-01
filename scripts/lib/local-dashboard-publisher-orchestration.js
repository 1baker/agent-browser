import { isTerminalPublicationPhase } from './local-dashboard-publication-journal.js';
import { pinRetainedBrowserExpectation } from './local-dashboard-retained-browser-guard.js';

export async function runLocalDashboardPublisherOrchestration({
  options,
  report,
  adapters,
}) {
  adapters.publicationJournal.acquire();
  try {
    return await runLockedLocalDashboardPublisherOrchestration({ options, report, adapters });
  } finally {
    adapters.publicationJournal.release();
  }
}

async function runLockedLocalDashboardPublisherOrchestration({ options, report, adapters }) {
  validateOptions(options);

  const installBin = adapters.resolveInstallBin();
  report.installBin = installBin;
  adapters.guardInstallPath(installBin);

  const existingJournal = adapters.publicationJournal.read();
  if (existingJournal && !isTerminalPublicationPhase(existingJournal.phase)) {
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

  await adapters.buildDashboard();
  await adapters.buildRuntime({ release: options.release });

  const builtBin = adapters.resolveBuiltBin({ release: options.release });
  if (!adapters.builtBinaryExists(builtBin)) {
    throw new Error(`Built binary was not found: ${builtBin}`);
  }
  report.builtBin = builtBin;
  const artifactEvidence = ensureArtifactEvidence(report);
  const builtSha256 = adapters.sha256File(builtBin);
  artifactEvidence.built = {
    path: builtBin,
    sha256: builtSha256,
  };

  report.service.before = adapters.serviceStatus();
  let backup = null;
  let dashboardQuiesced = false;
  let journalRecord = null;
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

    journalRecord = adapters.publicationJournal.create({
      installBin,
      builtBin,
      backupPath: backup?.path ?? null,
      installMode: backup?.mode ?? 0o755,
      artifactEvidence: cloneJson(artifactEvidence),
      candidateSessions: adapters.runtimeSessionNames(),
      handoffs: [],
      retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
      dashboardQuiesced: false,
      failure: null,
    });
    report.publicationJournal = journalSummary(journalRecord, adapters.publicationJournal.path);

    commitJournal('quiesce_admitted', { dashboardQuiesceAdmitted: true });
    await adapters.quiesceDashboardForRuntimeHandoff();
    dashboardQuiesced = true;
    commitJournal('quiesced', { dashboardQuiesced: true });

    try {
      commitJournal('handoff_admitted');
      await adapters.prepareRuntimeHandoffs(builtBin, installBin);
      commitJournal('handoff_prepared', {
        handoffs: cloneJson(report.handoffs.prepared),
      });
      commitJournal('replacement_admitted');
      await adapters.installBinaryAtomically(
        builtBin,
        installBin,
        backup?.mode ?? 0o755,
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
      commitJournal('ready', {
        artifactEvidence: cloneJson(artifactEvidence),
        retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        readiness: {
          smoke: cloneJson(report.smoke),
          runtimeManifest: cloneJson(report.runtimeManifest),
          browserSmoke: cloneJson(report.browserSmoke),
        },
      });
    } catch (error) {
      safeCommitJournal('publication_failed', {
        failedAtPhase: journalRecord?.phase ?? null,
        failure: errorMessage(error),
        artifactEvidence: cloneJson(artifactEvidence),
        handoffs: cloneJson(report.handoffs.prepared),
      });
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
          await adapters.installBinaryAtomically(backup.path, installBin, backup.mode);
          restoration.actualSha256 = adapters.sha256File(installBin);
          restoration.verified = restoration.actualSha256 === restoration.expectedSha256;
          if (!restoration.verified) {
            throw new Error(
              `Restored binary hash mismatch: expected=${restoration.expectedSha256} ` +
              `actual=${restoration.actualSha256}`,
            );
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
          await adapters.restartOrStartDashboard(installBin, { restoring: true });
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
  report.artifactEvidence = artifactEvidence;
  report.builtBin = journalRecord.builtBin ?? null;
  report.backupPath = journalRecord.backupPath ?? null;
  const retainedBrowserExpectation = journalRecord.retainedBrowserExpectation ?? null;
  let pinnedRetainedBrowserExpectation = null;
  if (retainedBrowserExpectation?.required === true) {
    pinnedRetainedBrowserExpectation = retainedBrowserExpectation.pinned
      ?? pinRetainedBrowserExpectation(retainedBrowserExpectation.before);
    report.retainedBrowserExpectation = cloneJson(retainedBrowserExpectation);
  }
  report.service.before = adapters.serviceStatus();
  try {
    const discoveredHandoffs = adapters.discoverPreparedRuntimeHandoffs(
      Array.isArray(journalRecord.candidateSessions) ? journalRecord.candidateSessions : [],
    );
    report.handoffs.prepared = mergePreparedHandoffs(
      Array.isArray(journalRecord.handoffs) ? journalRecord.handoffs : [],
      discoveredHandoffs,
    );

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
        commit('recovery_blocked', {
          recoveryError: 'retained_browser_expectation_failed',
          retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        });
        throw error;
      }
    }

    if (matchesReplacement) {
      if (options.syncReferenceBinaries) {
        commit('recovery_reference_sync_admitted');
        report.referenceBinaries = await adapters.syncReferenceBinaries(installBin);
        commit('recovery_references_synced', {
          referenceBinaries: cloneJson(report.referenceBinaries),
        });
      }
      commit('recovery_dashboard_restart_admitted');
      await adapters.restartOrStartDashboard(installBin, { restoring: false });
      commit('recovery_dashboard_restarted');
      if (!options.skipSmoke) {
        commit('recovery_readiness_admitted');
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
      commit('recovered_ready', {
        artifactEvidence: cloneJson(artifactEvidence),
        retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
        readiness: {
          smoke: cloneJson(report.smoke),
          runtimeManifest: cloneJson(report.runtimeManifest),
          browserSmoke: cloneJson(report.browserSmoke),
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
    commit('recovered_rolled_back', {
      artifactEvidence: cloneJson(artifactEvidence),
      retainedBrowserExpectation: cloneJson(report.retainedBrowserExpectation),
      installedSha256,
    });
    report.restoredBackup = true;
    report.recovery = {
      transactionId: journalRecord.transactionId,
      result: 'recovered_rolled_back',
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
