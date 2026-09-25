import { isTerminalPublicationPhase } from './local-dashboard-publication-journal.js';
import { requireExpectedPublicationSessions, validatePrebuiltPublicationOptions } from './local-dashboard-prebuilt-candidate.js';
import { COLD_RESTART_SCHEMA, runColdRestart } from './local-dashboard-cold-restart.js';

/**
 * Opt-in publisher integration. No operator flag is exposed until a reviewed
 * runtime adapter implements exact admission, cold-close and launch custody.
 * Missing runtime support fails before staging, interlock changes or shutdown.
 */
export async function runColdRestartPublication({ options, report, adapters, acquireMaintenance, existingJournal }) {
  const runtime = adapters.coldRuntime;
  const required = ['captureIntent', 'verifyIntent', 'fenceSource', 'restoreSourceFence',
    'closeSource', 'requireIdle', 'backupProfile', 'verifyBackups', 'launch', 'qualify',
    'fenceCandidate', 'closeCandidate', 'restoreProfile', 'verifyFinal'];
  if (!runtime || required.some((name) => typeof runtime[name] !== 'function')) {
    throw new Error('Cold restart has no reviewed exact-custody runtime adapter; no live changes admitted');
  }
  if (typeof adapters.acquireMaintenance !== 'function') {
    throw new Error('Cold restart requires mandatory maintenance custody');
  }
  const requireMaintenance = () => {
    const custody = acquireMaintenance();
    if (!custody?.receipt || typeof custody.receipt.id !== 'string' || !custody.receipt.id
      || typeof custody.release !== 'function' || typeof custody.retainForRecovery !== 'function') {
      throw new Error('Cold restart did not acquire positive maintenance custody');
    }
  };
  let record = existingJournal;
  const recovering = record?.coldRestart && !isTerminalPublicationPhase(record.phase);
  if (recovering && !options.recoverOnly) throw new Error('Cold publication requires explicit recovery');
  if (options.recoverOnly && !recovering) throw new Error('No nonterminal cold publication exists for recovery');
  if (record && !isTerminalPublicationPhase(record.phase) && !record.coldRestart) {
    throw new Error('An unrelated publication requires its own recovery');
  }
  if (!recovering) {
    if (!validatePrebuiltPublicationOptions(options) || options.syncReferenceBinaries
      || !options.retainedBrowserExpectation || options.skipSmoke) {
      throw new Error('Cold publication requires a digest-bound prebuilt candidate, retained identity, readiness checks and no reference sync');
    }
    requireExpectedPublicationSessions(options.expectedSessions, adapters.runtimeSessionNames());
    const before = await adapters.verifyRetainedBrowserExpectation(report.installBin, {
      expectation: options.retainedBrowserExpectation, stage: 'cold_pre_mutation',
    });
    if (before?.verified !== true) throw new Error('Cold publication retained identity is unverified');
    const builtBin = await adapters.stagePrebuiltCandidate();
    if (adapters.sha256File(builtBin) !== options.expectedSha256) throw new Error('Cold candidate digest mismatch');
    const sourceSha256 = adapters.sha256File(report.installBin);
    const intent = await runtime.captureIntent({
      options, installBin: report.installBin, builtBin, sourceSha256,
      candidateSha256: options.expectedSha256, retainedEvidence: before,
    });
    if (intent?.approved !== true) throw new Error('Cold restart intent is not approved');
    requireMaintenance();
    report.service.before = adapters.serviceStatus();
    requireExpectedPublicationSessions(options.expectedSessions, adapters.runtimeSessionNames());
    const backup = await adapters.backupInstalledBinary(report.installBin);
    if (!backup || adapters.sha256File(backup.path) !== sourceSha256
      || adapters.sha256File(report.installBin) !== sourceSha256) throw new Error('Cold source backup failed verification');
    const provenance = await adapters.prepareWorkstationProvenance({ installBin: report.installBin, builtBin });
    if (!provenance) throw new Error('Cold restart requires paired workstation provenance');
    record = adapters.publicationJournal.create({
      installBin: report.installBin, builtBin, backupPath: backup.path, installMode: backup.mode,
      workstationProvenance: provenance, candidateSessions: options.expectedSessions,
      handoffs: [], syncReferenceBinaries: false,
      artifactEvidence: {
        source: { path: report.installBin, sha256: sourceSha256 },
        built: { path: builtBin, sha256: options.expectedSha256 },
        backup: { ...backup, sha256: sourceSha256, verified: true },
      },
      retainedBrowserExpectation: { required: true, before, final: null },
      coldRestart: { schemaVersion: COLD_RESTART_SCHEMA, intent, profileBackup: null },
    });
  } else {
    requireMaintenance();
    if (record.installBin !== report.installBin) throw new Error('Cold recovery installation identity mismatch');
  }
  const verifyArtifacts = (current) => {
    const evidence = current.artifactEvidence;
    if (adapters.sha256File(current.backupPath) !== evidence.backup.sha256
      || evidence.source.sha256 !== evidence.backup.sha256
      || adapters.sha256File(current.builtBin) !== evidence.built.sha256) {
      throw new Error('Cold publication artifact receipt mismatch');
    }
    const selected = adapters.sha256File(current.installBin);
    if (![evidence.backup.sha256, evidence.built.sha256].includes(selected)) {
      throw new Error('Cold publication found an unreviewed installed executable');
    }
  };
  const result = await runColdRestart({
    journal: adapters.publicationJournal, record, recover: Boolean(recovering),
    adapters: {
      ...runtime,
      verifyIntent: async (current) => {
        verifyArtifacts(current);
        return runtime.verifyIntent(current);
      },
      verifyBackups: async (current) => {
        verifyArtifacts(current);
        return runtime.verifyBackups(current);
      },
      installPair: async (current, selection) => {
        verifyArtifacts(current);
        const source = selection === 'source' ? current.backupPath : current.builtBin;
        const expected = selection === 'source'
          ? current.artifactEvidence.backup.sha256 : current.artifactEvidence.built.sha256;
        await adapters.installBinaryAtomically(source, current.installBin, current.installMode, expected);
        await adapters.applyWorkstationProvenance(current.workstationProvenance, {
          selection, installBin: current.installBin,
        });
        await adapters.verifyWorkstationProvenance(current.workstationProvenance, {
          selection, installBin: current.installBin,
        });
        return { verified: true };
      },
      restoreDashboard: (current) => adapters.restartOrStartDashboard(current.installBin, {
        restoring: current.coldRestart.sourceLaunchAdmitted === true,
      }),
      verifyFinal: async (current, selection) => {
        await adapters.verifyWorkstationProvenance(current.workstationProvenance, { selection, installBin: current.installBin });
        const retained = await runtime.verifyFinal(current, selection);
        if (retained?.verified !== true) throw new Error('Cold runtime final verification failed');
        report.smoke = await adapters.runHttpReadinessSmoke(current.installBin);
        report.runtimeManifest = await adapters.verifyRuntimeManifestReadback(current.installBin, report.smoke.runtimeManifest);
        // Doctor sees this transaction as nonterminal until all evidence agrees.
        report.installDoctor = await adapters.verifyInstalledDoctor(current.installBin, { journalRecord: current });
        return { verified: true, retained, installedSha256: adapters.sha256File(current.installBin) };
      },
    },
  });
  report.coldRestart = { phase: result.phase, result: result.coldRestart.result };
  report.publicationJournal = { path: adapters.publicationJournal.path, transactionId: result.transactionId,
    revision: result.revision, phase: result.phase, terminal: result.terminal };
  return result;
}
