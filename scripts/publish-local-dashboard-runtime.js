#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  readFileSync,
  readlinkSync,
  readdirSync,
  statSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import {
  stagePrebuiltPublicationCandidate,
  requireExpectedPublicationSessions,
  requireLegacyPublicationTarget,
  installPublicationBinaryAtomically,
} from './lib/local-dashboard-prebuilt-candidate.js';
import { acquireLocalDashboardPublicationInterlock, readLocalDashboardPublicationInterlockReceipt } from './lib/local-dashboard-publication-interlock.js';
import { prepareWorkstationProvenance, applyWorkstationProvenance, verifyWorkstationProvenance } from './lib/local-dashboard-workstation-provenance.js';
import { parseControllerUpdate, verifyControllerCandidate } from './lib/local-dashboard-controller-candidate.js';
import { prepareDashboardSessionQuiesce, verifyDashboardSessionQuiesce, refreshDashboardSessionEvidence } from './lib/local-dashboard-quiesce-sessions.js';
import { validatePublicationDoctor, validateStrictPublicationDoctor } from './lib/local-dashboard-publication-doctor.js';
import {
  evaluateLocalDashboardBrowserSmokeResult,
} from './lib/local-dashboard-smoke-policy.js';
import {
  quiesceStandaloneDashboardForRuntimeHandoff,
  restartOrStartDashboardRuntime,
} from './lib/local-dashboard-publisher-lifecycle.js';
import {
  runLocalDashboardPublisherOrchestration,
} from './lib/local-dashboard-publisher-orchestration.js';
import {
  createLocalDashboardPublicationJournal,
  inspectLocalDashboardPublicationJournal,
} from './lib/local-dashboard-publication-journal.js';
import {
  evaluateRetainedBrowserExpectation,
  isLoopbackDevToolsUrl,
  normalizeRetainedBrowserExpectation,
  retainedTargetUrlsMatch,
} from './lib/local-dashboard-retained-browser-guard.js';
import {
  discoverRetainedBrowserExpectation,
} from './lib/local-dashboard-retained-browser-discovery.js';
import {
  readRetainedBrowserRequirement,
  resolveRetainedBrowserExpectation,
  writeRetainedBrowserRequirement,
} from './lib/local-dashboard-retained-browser-requirement.js';
import { resolveRuntimeSocketDir } from './lib/runtime-socket-dir.js';
import { verifyPartialSourceHandoffInventory } from './lib/local-dashboard-partial-source-handoff.js';
import {
  readSessionDisplayEnvironment,
  withSessionDisplayEnvironment,
} from './lib/local-dashboard-session-display-environment.js';
import { retirePreparedDaemon } from './lib/prepared-daemon-retirement.js';
import {
  isProcessLive as browserProcessIsLive,
  waitForRuntimeDaemonExit,
  verifyRuntimeSessionsRetired,
} from './lib/runtime-daemon-exit.js';
import {
  resolveRuntimeDaemonClientBinary as runtimeDaemonClientBinary,
} from './lib/runtime-daemon-client-binary.js';
import {
  isRuntimeHandoffBrowserActive,
  removeVerifiedRuntimeHandoffRecord,
  selectRuntimeHandoffBrowser,
} from './lib/runtime-handoff-browser-selection.js';

const rootDir = new URL('..', import.meta.url).pathname;
const args = process.argv.slice(2);
const options = {
  allowOutsideHome: false,
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'http://127.0.0.1:4848/',
  discoverRetainedExactUrl: '',
  discoverRetainedProfile: '',
  discoverRetainedUrlPrefix: '',
  expectMarkers: [],
  expectRetainedCdpUrl: '',
  expectRetainedProfile: '',
  expectRetainedSession: '',
  expectRetainedTarget: '',
  expectRetainedUrl: '',
  installBin: process.env.AGENT_BROWSER_INSTALL_BIN || '',
  json: false,
  journalStatus: false,
  browserBuild: '',
  browserProfile: '',
  release: false,
  prebuiltBin: '',
  controllerUpdates: [],
  expectedSha256: '',
  expectedSessions: null,
  recoverOnly: false,
  recoverInterlockReceipt: null,
  recoverReplacedRetainedBrowser: null,
  retainedBrowserStatus: false,
  retainedRequirementPath: process.env.AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT
    || resolve(homedir(), '.agent-browser', 'publications', 'local-dashboard-retained-browser.json'),
  requireBrowserSmoke: false,
  skipSmoke: false,
  syncReferenceBinaries: true,
  smokeBrowser: true,
  startIfMissing: false,
  writeRetainedRequirement: false,
  workspaceSession: '',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--allow-outside-home') {
    options.allowOutsideHome = true;
  } else if (arg === '--dashboard-url') {
    options.dashboardUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--discover-retained-url-prefix') {
    options.discoverRetainedUrlPrefix = requiredValue(args, ++index, arg);
  } else if (arg === '--discover-retained-exact-url') {
    options.discoverRetainedExactUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--discover-retained-profile') {
    options.discoverRetainedProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-marker') {
    options.expectMarkers.push(requiredValue(args, ++index, arg));
  } else if (arg === '--expect-retained-cdp-url') {
    options.expectRetainedCdpUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-retained-profile') {
    options.expectRetainedProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-retained-session') {
    options.expectRetainedSession = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-retained-target') {
    options.expectRetainedTarget = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-retained-url') {
    options.expectRetainedUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--browser-build') {
    options.browserBuild = requiredValue(args, ++index, arg);
  } else if (arg === '--browser-profile') {
    options.browserProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--install-bin') {
    options.installBin = requiredValue(args, ++index, arg);
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--journal-status') {
    options.journalStatus = true;
  } else if (arg === '--release') {
    options.release = true;
  } else if (arg === '--prebuilt-bin') {
    options.prebuiltBin = requiredValue(args, ++index, arg);
  } else if (arg === '--controller-update') {
    options.controllerUpdates.push(parseControllerUpdate(requiredValue(args, ++index, arg)));
  } else if (arg === '--expected-sha256') {
    options.expectedSha256 = requiredValue(args, ++index, arg);
  } else if (arg === '--expected-sessions') {
    const value = requiredValue(args, ++index, arg);
    options.expectedSessions = value === 'none' ? [] : value.split(',');
  } else if (arg === '--recover-only') {
    options.recoverOnly = true;
  } else if (arg === '--recover-interlock-receipt') {
    options.recoverInterlockReceipt = requiredValue(args, ++index, arg);
  } else if (arg === '--recover-replaced-retained-browser') {
    options.recoverReplacedRetainedBrowser = requiredValue(args, ++index, arg);
  } else if (arg === '--retained-browser-status') {
    options.retainedBrowserStatus = true;
  } else if (arg === '--retained-requirement') {
    options.retainedRequirementPath = requiredValue(args, ++index, arg);
  } else if (arg === '--require-browser-smoke') {
    options.requireBrowserSmoke = true;
  } else if (arg === '--skip-browser') {
    options.smokeBrowser = false;
  } else if (arg === '--skip-reference-sync') {
    options.syncReferenceBinaries = false;
  } else if (arg === '--skip-smoke') {
    options.skipSmoke = true;
  } else if (arg === '--start-if-missing') {
    options.startIfMissing = true;
  } else if (arg === '--write-retained-requirement') {
    options.writeRetainedRequirement = true;
  } else if (arg === '--workspace-session') {
    options.workspaceSession = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}
const selectedOperations = [
  options.journalStatus,
  options.recoverOnly,
  options.retainedBrowserStatus,
  options.writeRetainedRequirement,
].filter(Boolean).length;
if (selectedOperations > 1) {
  fail('Journal status, recovery, retained browser status, and requirement write cannot be combined');
}
if (options.recoverReplacedRetainedBrowser && !options.recoverOnly) {
  fail('--recover-replaced-retained-browser requires --recover-only');
}
try {
  const retainedExpectationRequested = [
    options.expectRetainedCdpUrl,
    options.expectRetainedProfile,
    options.expectRetainedSession,
    options.expectRetainedTarget,
    options.expectRetainedUrl,
  ].some(Boolean);
  options.explicitRetainedBrowserExpectation = retainedExpectationRequested
    ? normalizeRetainedBrowserExpectation({
      sessionName: options.expectRetainedSession,
      cdpUrl: options.expectRetainedCdpUrl,
      profileId: options.expectRetainedProfile,
      targetId: options.expectRetainedTarget,
      url: options.expectRetainedUrl,
    })
    : null;
  options.retainedRequirementPath = resolve(options.retainedRequirementPath);
  options.retainedBrowserRequirement = options.journalStatus || options.recoverOnly
    ? null
    : readRetainedBrowserRequirement(options.retainedRequirementPath);
  options.retainedBrowserExpectation = resolveRetainedBrowserExpectation({
    explicit: options.explicitRetainedBrowserExpectation,
    requirement: options.retainedBrowserRequirement,
  });
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
if (options.retainedBrowserStatus && !options.retainedBrowserExpectation) {
  fail('--retained-browser-status requires an explicit or durable retained browser requirement');
}
if (options.writeRetainedRequirement && !options.explicitRetainedBrowserExpectation) {
  if (!options.discoverRetainedUrlPrefix && !options.discoverRetainedExactUrl) {
    fail('--write-retained-requirement requires explicit retained browser identity flags or a discovery URL selector');
  }
}
if ((options.discoverRetainedUrlPrefix || options.discoverRetainedExactUrl) && !options.writeRetainedRequirement) {
  fail('Retained browser discovery requires --write-retained-requirement');
}
if (options.discoverRetainedUrlPrefix && options.discoverRetainedExactUrl) {
  fail('--discover-retained-url-prefix and --discover-retained-exact-url cannot be combined');
}
if ((options.discoverRetainedUrlPrefix || options.discoverRetainedExactUrl) && options.explicitRetainedBrowserExpectation) {
  fail('Retained browser discovery cannot be combined with explicit retained browser identity flags');
}
if (options.discoverRetainedProfile && !options.discoverRetainedExactUrl) {
  fail('--discover-retained-profile requires --discover-retained-exact-url');
}
if (
  options.writeRetainedRequirement
  && !options.discoverRetainedUrlPrefix
  && !options.discoverRetainedExactUrl
  && ['sessionName', 'profileId', 'targetId', 'url']
    .some((field) => !options.explicitRetainedBrowserExpectation[field])
) {
  fail('--write-retained-requirement requires session, profile, target, and URL');
}

const report = {
  operation: options.journalStatus
    ? 'journal_status'
    : options.recoverOnly
      ? 'recover_only'
      : options.retainedBrowserStatus
        ? 'retained_browser_status'
        : options.writeRetainedRequirement ? 'write_retained_browser_requirement' : 'publish',
  dashboardUrl: options.dashboardUrl,
  mode: options.release ? 'release' : 'debug',
  installBin: null,
  builtBin: null,
  backupPath: null,
  service: {
    before: null,
    after: null,
    action: 'none',
    quiesced: false,
    standaloneDashboard: null,
  },
  smoke: null,
  browserSmoke: {
    requested: options.smokeBrowser
      && !options.skipSmoke
      && !options.retainedBrowserStatus
      && !options.writeRetainedRequirement,
    required: options.requireBrowserSmoke,
    status: options.smokeBrowser
      && !options.skipSmoke
      && !options.retainedBrowserStatus
      && !options.writeRetainedRequirement
      ? 'pending'
      : 'skipped',
    classification: options.retainedBrowserStatus || options.writeRetainedRequirement
      ? options.retainedBrowserStatus
        ? 'retained_browser_status_only'
        : 'retained_browser_requirement_write_only'
      : options.skipSmoke
      ? 'all_smoke_skipped'
      : options.smokeBrowser ? null : 'browser_smoke_skipped',
  },
  runtimeManifest: null,
  retainedBrowserExpectation: null,
  retainedBrowserDiscovery: null,
  retainedBrowserRequirement: publicRetainedBrowserRequirement(
    options.retainedBrowserRequirement,
  ),
  publicationJournal: null,
  artifactEvidence: {
    built: null,
    source: null,
    backup: null,
    replacement: null,
    restoration: null,
  },
  referenceBinaries: [],
  handoffs: {
    prepared: [],
    resumed: [],
    rollbackResumed: [],
    retiredIdleSessions: [],
    unsupportedActiveSessions: [],
  },
};
const publicationJournal = createLocalDashboardPublicationJournal({
  journalPath: resolve(
    homedir(),
    '.agent-browser',
    'publications',
    'local-dashboard-publication.json',
  ),
});

try {
  await run();
  output(options.journalStatus
    ? {
      success: true,
      operation: report.operation,
      publicationJournalStatus: report.publicationJournalStatus,
    }
    : { success: true, ...report });
} catch (error) {
  const errorMessage = error instanceof Error ? error.message : String(error);
  output(options.journalStatus
    ? { success: false, operation: report.operation, error: errorMessage }
    : { success: false, error: errorMessage, ...report });
  process.exit(1);
}

async function run() {
  if (options.controllerUpdates.length && (!options.prebuiltBin || options.recoverOnly
      || options.journalStatus || options.retainedBrowserStatus || options.writeRetainedRequirement)) {
    throw new Error('Controller updates require a fresh reviewed prebuilt publication');
  }
  if (options.journalStatus) {
    report.publicationJournalStatus = inspectLocalDashboardPublicationJournal({
      journal: publicationJournal,
      pathExists: existsSync,
      sha256File,
    });
    const receiptPath = `${publicationJournal.path}.interlock.json`;
    report.publicationJournalStatus.interlockCustody = existsSync(receiptPath)
      ? readLocalDashboardPublicationInterlockReceipt(receiptPath) : null;
    return;
  }
  if (options.retainedBrowserStatus || options.writeRetainedRequirement) {
    report.installBin = resolveInstallBin();
    if (options.writeRetainedRequirement && (
      options.discoverRetainedUrlPrefix || options.discoverRetainedExactUrl
    )) {
      let discovery;
      try {
        discovery = await discoverRetainedBrowserExpectation({
          urlPrefix: options.discoverRetainedUrlPrefix,
          exactUrl: options.discoverRetainedExactUrl,
          profileId: options.discoverRetainedProfile,
          sessionNames: runtimeSessionNames(),
          readDaemonPid: readRuntimePid,
          isProcessLive: browserProcessIsLive,
          readBrowser: async (sessionName, daemonPid) => serviceBrowserForSession(
            runtimeDaemonClientBinary(daemonPid, report.installBin),
            sessionName,
          ),
          readCdpTargets: readCdpTargetInventory,
        });
      } catch (error) {
        report.retainedBrowserDiscovery = error?.discoveryEvidence ?? null;
        throw error;
      }
      options.explicitRetainedBrowserExpectation = discovery.expectation;
      options.retainedBrowserExpectation = resolveRetainedBrowserExpectation({
        explicit: discovery.expectation,
        requirement: options.retainedBrowserRequirement,
      });
      report.retainedBrowserDiscovery = {
        urlPrefix: discovery.urlPrefix,
        exactUrl: discovery.exactUrl,
        profileId: discovery.profileId,
        inspectedSessionCount: discovery.inspectedSessionCount,
        matchedCandidateCount: discovery.matchedCandidateCount,
      };
    }
    report.retainedBrowserExpectation = {
      required: true,
      pinned: null,
      before: null,
      afterHandoff: null,
      final: null,
    };
    try {
      report.retainedBrowserExpectation.before =
        await verifyRetainedBrowserExpectation(report.installBin, {
          expectation: options.retainedBrowserExpectation,
          stage: options.writeRetainedRequirement
            ? 'requirement_write_preflight'
            : 'read_only_preflight',
        });
    } catch (error) {
      report.retainedBrowserExpectation.before = error?.retainedBrowserEvidence ?? null;
      throw error;
    }
    if (options.writeRetainedRequirement) {
      const requirement = writeRetainedBrowserRequirement({
        path: options.retainedRequirementPath,
        evidence: report.retainedBrowserExpectation.before,
      });
      report.retainedBrowserRequirement = publicRetainedBrowserRequirement(requirement);
    }
    return;
  }
  await runLocalDashboardPublisherOrchestration({
    options,
    report,
    adapters: {
      resolveInstallBin,
      guardInstallPath,
      buildDashboard: () => runCommand('pnpm', ['build:dashboard']),
      buildRuntime: ({ release }) => {
        const cargoArgs = ['build', '--manifest-path', 'cli/Cargo.toml'];
        if (release) cargoArgs.push('--release');
        runCommand('cargo', cargoArgs);
      },
      resolveBuiltBin: ({ release }) => resolve(
        rootDir,
        'cli',
        'target',
        release ? 'release' : 'debug',
        'agent-browser',
      ),
      builtBinaryExists: existsSync,
      stagePrebuiltCandidate: () => stagePrebuiltPublicationCandidate({
        sourcePath: options.prebuiltBin,
        expectedSha256: options.expectedSha256,
        installPath: report.installBin,
        journalPath: publicationJournal.path,
      }),
      acquireMaintenance: () => acquireLocalDashboardPublicationInterlock({
        receiptPath: `${publicationJournal.path}.interlock.json`,
        recoverReceiptId: options.recoverInterlockReceipt,
        installBin: report.installBin,
        workstationRoot: homedir(),
      }),
      hasMaintenanceReceipt: () => existsSync(`${publicationJournal.path}.interlock.json`),
      prepareWorkstationProvenance: ({ installBin, builtBin }) => {
        const version = JSON.parse(readFileSync(resolve(rootDir, 'package.json'), 'utf8')).version;
        const candidateVersion = execFileSync(builtBin, ['--version'], { encoding: 'utf8', timeout: 15000 }).trim();
        if (candidateVersion !== `agent-browser ${version}`) throw new Error('Candidate package version differs from publication checkout');
        verifyControllerCandidate(builtBin, options.controllerUpdates);
        return prepareWorkstationProvenance({ root: homedir(), version, installBin, builtBin,
          journalPath: publicationJournal.path, controllerUpdates: options.controllerUpdates });
      },
      applyWorkstationProvenance,
      verifyWorkstationProvenance,
      verifyInstalledDoctor: (installBin, context = {}) => {
        const result = spawnSync(installBin, ['install', 'doctor', '--json'], {
          encoding: 'utf8', timeout: 90000, maxBuffer: 8 * 1024 * 1024, cwd: homedir(),
        });
        if (result.error || ![0, 1].includes(result.status)) throw new Error('Installation doctor did not complete');
        const value = JSON.parse(result.stdout);
        if (context.strict) return validateStrictPublicationDoctor(value);
        return validatePublicationDoctor(value, {
          ...context, journalPath: publicationJournal.path, ownerPid: process.pid,
          installedSha256: sha256File(installBin), verifyListenerEvidence,
        });
      },
      inspectRollbackDoctorDegradation: ({ installBin, journalRecord, expectedSha256 }) => {
        const result = spawnSync(installBin, ['install', 'doctor', '--json'], {
          encoding: 'utf8', timeout: 90000, maxBuffer: 8 * 1024 * 1024, cwd: homedir(),
        });
        if (result.error || result.status !== 1) return null;
        let value;
        try {
          value = JSON.parse(result.stdout);
        } catch {
          return null;
        }
        const issueCodes = value.data?.issues?.map((issue) => issue?.code).sort();
        const status = value.data?.localDashboardPublication;
        if (value.success !== false
          || JSON.stringify(issueCodes) !== JSON.stringify([
            'dashboard_publication_active', 'service_resource_candidates_ready',
          ])
          || value.data?.serviceResources?.readinessImpactingCandidates !== 1
          || value.data?.currentExecutable?.sha256 !== expectedSha256
          || value.data?.workstationPayload?.ready !== true
          || value.data?.liveDashboardRuntime?.ready !== true
          || status?.transaction?.transactionId !== journalRecord.transactionId
          || status?.lock?.ownerPid !== process.pid
          || status?.lock?.live !== true) return null;
        return {
          degraded: true,
          rawSuccess: false,
          transactionScoped: true,
          repairRequired: true,
          issueCodes,
          readinessImpactingCandidateCount: 1,
          workstationPayloadReady: true,
        };
      },
      serviceStatus,
      backupInstalledBinary,
      quiesceDashboardForRuntimeHandoff,
      prepareQuiesceSessions,
      verifyQuiesceSessions,
      prepareRuntimeHandoffs,
      verifyRuntimeSessionsRetired: () => verifyRuntimeSessionsRetired({
        sessionNames: runtimeSessionNames(), readRuntimePid, isProcessLive: browserProcessIsLive,
      }),
      verifyPartialSourceHandoffInventory: ({ expectedSessions, preparedSessions, sourceSha256, installBin }) => {
        if (sha256File(installBin) !== sourceSha256) return false;
        const state = JSON.parse(readFileSync(join(homedir(), '.agent-browser', 'service', 'state.json'), 'utf8'));
        const sourceStat = statSync(installBin);
        return verifyPartialSourceHandoffInventory({
          expectedSessions,
          preparedSessions,
          readBrowser: (sessionName) => state.browsers?.[`session:${sessionName}`] ?? null,
          readDaemonPid: readRuntimePid,
          isProcessLive: browserProcessIsLive,
          hasHandoffRecord: (sessionName) => existsSync(join(runtimeSocketDir(), `${sessionName}.handoff.json`)),
          sameSourceExecutable: (pid) => {
            try {
              const executable = statSync(`/proc/${pid}/exe`);
              return executable.dev === sourceStat.dev && executable.ino === sourceStat.ino;
            } catch {
              return false;
            }
          },
        });
      },
      installBinaryAtomically,
      syncReferenceBinaries,
      resumeRuntimeHandoffs,
      restartOrStartDashboard,
      runHttpReadinessSmoke,
      verifyRuntimeManifestReadback,
      verifyRetainedBrowserExpectation,
      verifyRecoveredRetainedBrowserExpectation,
      runBrowserSmokeDiagnostic,
      pathExists: existsSync,
      sha256File,
      runtimeSessionNames,
      discoverPreparedRuntimeHandoffs,
      publicationJournal,
    },
  });
}

function backupInstalledBinary(installBin) {
  if (!existsSync(installBin)) return null;
  const beforeStat = statSync(installBin);
  const mode = beforeStat.mode & 0o777;
  const backupPath = `${installBin}.pre-local-dashboard-${timestamp()}`;
  copyFileSync(installBin, backupPath);
  chmodSync(backupPath, mode);
  return { path: backupPath, mode };
}

function syncReferenceBinaries(builtBin) {
  const references = [];
  const seen = new Set([resolve(builtBin), resolve(report.installBin || '')]);
  for (const target of referenceBinaryCandidates()) {
    const resolved = resolve(target);
    if (seen.has(resolved)) continue;
    seen.add(resolved);
    if (!existsSync(resolved)) {
      references.push({
        path: resolved,
        synced: false,
        reason: 'missing',
      });
      continue;
    }
    guardInstallPath(resolved);
    const before = sha256File(resolved);
    const mode = statSync(resolved).mode & 0o777;
    installBinaryAtomically(builtBin, resolved, mode);
    references.push({
      path: resolved,
      synced: true,
      beforeSha256: before,
      afterSha256: sha256File(resolved),
    });
  }
  return references;
}

function referenceBinaryCandidates() {
  const candidates = [
    resolve(rootDir, 'bin', platformBinaryName()),
  ];
  const pnpmRoot = commandOutput('pnpm', ['root', '-g']).trim();
  if (pnpmRoot) {
    candidates.push(resolve(pnpmRoot, 'agent-browser', 'bin', platformBinaryName()));
  }
  return candidates;
}

function platformBinaryName() {
  const platform = process.platform === 'win32' ? 'windows' : process.platform;
  const arch = process.arch === 'x64' ? 'x64' : process.arch;
  const extension = process.platform === 'win32' ? '.exe' : '';
  return `agent-browser-${platform}-${arch}${extension}`;
}

function installBinaryAtomically(source, target, mode, expectedSha256 = null) {
  installPublicationBinaryAtomically(source, target, mode, expectedSha256);
}

function runtimeSocketDir() {
  return resolveRuntimeSocketDir();
}

function runtimeSessionNames() {
  const socketDir = runtimeSocketDir();
  if (!existsSync(socketDir)) return [];
  const suffix = process.platform === 'win32' ? '.port' : '.sock';
  return readdirSync(socketDir)
    .filter((name) => name.endsWith(suffix))
    .map((name) => name.slice(0, -suffix.length))
    .filter((name) => /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name))
    .sort();
}

function prepareRuntimeHandoffs(clientBin, rollbackBin, expectedSessions = null) {
  try {
    const sessions = runtimeSessionNames();
    if (expectedSessions) requireExpectedPublicationSessions(expectedSessions, sessions);
    for (const sessionName of sessions) {
      const daemonPid = readRuntimePid(sessionName);
      const displayEnvironment = readSessionDisplayEnvironment(daemonPid);
      const daemonClientBin = runtimeDaemonClientBinary(daemonPid, rollbackBin);
      const prepared = runAgentJson(clientBin, sessionName, ['handoff', 'prepare']);
      if (prepared.status === 0 && prepared.json?.success === true) {
        const data = prepared.json.data || {};
        if (data.prepared === true) {
          const preparedHandoff = {
            sessionName,
            daemonPid,
            browserPid: data.browserPid ?? null,
            cdpUrl: data.cdpUrl ?? null,
            runtimeProfile: data.runtimeProfile ?? null,
            host: data.host ?? null,
            handoffPath: data.handoffPath ?? null,
            displayEnvironment,
            strandedDaemonTermination: null,
          };
          report.handoffs.prepared.push(preparedHandoff);
          try {
            waitForDaemonExit(sessionName, daemonPid);
          } catch (error) {
            if (error.code !== 'runtime_daemon_exit_timeout') throw error;
            preparedHandoff.strandedDaemonTermination =
              retirePreparedDaemon(preparedHandoff);
            waitForDaemonExit(sessionName, daemonPid);
          }
        } else {
          report.handoffs.retiredIdleSessions.push({ sessionName, daemonPid });
          waitForDaemonExit(sessionName, daemonPid);
        }
        continue;
      }

      const serviceReadback = serviceBrowserForSession(daemonClientBin, sessionName);
      if (!serviceReadback.success) {
        throw new Error(
          `Could not prove whether daemon session '${sessionName}' owns a browser before executable replacement: ` +
          serviceReadback.error,
        );
      }
      const browser = serviceReadback.browser;
      const browserAppearsActive = browser
        && (
          browserProcessIsLive(browser.pid)
          || (
            typeof browser.cdpEndpoint === 'string'
            && browser.cdpEndpoint.length > 0
            && !['closed', 'not_started'].includes(browser.health)
          )
        );
      if (!browserAppearsActive) {
        const closed = runAgentJson(daemonClientBin, sessionName, ['close']);
        if (closed.status !== 0 || closed.json?.success !== true) {
          if (!browserProcessIsLive(daemonPid)) {
            report.handoffs.retiredIdleSessions.push({
              sessionName,
              daemonPid,
              compatibilityClose: true,
              alreadyExited: true,
            });
            continue;
          }
          throw new Error(
            `Could not retire idle daemon session '${sessionName}' before executable replacement: ${closed.error}`,
          );
        }
        waitForDaemonExit(sessionName, daemonPid);
        report.handoffs.retiredIdleSessions.push({
          sessionName,
          daemonPid,
          compatibilityClose: true,
        });
        continue;
      }

      report.handoffs.unsupportedActiveSessions.push({
        sessionName,
        daemonPid,
        browserPid: browser.pid ?? null,
        cdpUrl: browser.cdpEndpoint ?? null,
        error: prepared.error,
      });
      throw new Error(
        `Installed daemon cannot hand off active browser session '${sessionName}'. ` +
        'The publish was stopped before replacing the executable.',
      );
    }
  } catch (error) {
    for (const prepared of report.handoffs.prepared) {
      const resumed = runAgentJson(
        rollbackBin,
        prepared.sessionName,
        ['handoff', 'resume'],
        prepared.displayEnvironment,
      );
      report.handoffs.rollbackResumed.push({
        sessionName: prepared.sessionName,
        success: resumed.status === 0 && resumed.json?.success === true,
        error: resumed.status === 0 && resumed.json?.success === true ? null : resumed.error,
      });
    }
    throw error;
  }
}

function resumeRuntimeHandoffs(installBin) {
  for (const prepared of report.handoffs.prepared) {
    if (prepared.host === 'attached_existing'
      && prepared.browserPid != null
      && prepared.displayEnvironment === undefined) {
      throw new Error(
        `Retained external browser session '${prepared.sessionName}' lacks display environment custody`,
      );
    }
    // Capture daemon existence before the service-state readback. The readback
    // command can start an otherwise idle daemon for this session, and that
    // newly spawned process is not evidence that handoff resume completed.
    const existingDaemonPid = readRuntimePid(prepared.sessionName);
    const existing = serviceBrowserForSession(
      installBin,
      prepared.sessionName,
      prepared,
      prepared.displayEnvironment,
    );
    if (
      prepared.host === 'attached_existing'
      && prepared.browserPid == null
      && prepared.cdpUrl
      && !cdpEndpointIsReachable(prepared.cdpUrl)
    ) {
      const retryRecordRemoved = removeVerifiedRuntimeHandoffRecord(prepared);
      if (Number.isInteger(existingDaemonPid) && browserProcessIsLive(existingDaemonPid)) {
        process.kill(existingDaemonPid, 'SIGTERM');
        waitForDaemonExit(prepared.sessionName, existingDaemonPid);
      }
      report.handoffs.resumed.push({
        sessionName: prepared.sessionName,
        browserPid: null,
        attachedBrowserPid: null,
        staleBrowserPidDropped: false,
        staleAttachedEndpointDropped: true,
        cdpUrl: prepared.cdpUrl,
        runtimeProfile: prepared.runtimeProfile ?? null,
        targetsReattached: 0,
        retryRecordRemoved,
        daemonPid: null,
      });
      continue;
    }
    if (existing.success && existing.browser) {
      const browser = existing.browser;
      if (
        prepared.browserPid !== null
        && browser.pid !== null
        && browser.pid !== prepared.browserPid
      ) {
        throw new Error(
          `Runtime handoff recovery found a different browser PID for session ` +
          `'${prepared.sessionName}': ${prepared.browserPid} -> ${browser.pid}`,
        );
      }
      if (prepared.cdpUrl && browser.cdpEndpoint !== prepared.cdpUrl) {
        throw new Error(
          `Runtime handoff recovery found a different CDP endpoint for session ` +
          `'${prepared.sessionName}': ${prepared.cdpUrl} -> ${browser.cdpEndpoint}`,
        );
      }
      if (
        Number.isInteger(existingDaemonPid)
        && browserProcessIsLive(existingDaemonPid)
        && isRuntimeHandoffBrowserActive({
          browser,
          expectedBrowser: prepared,
          isProcessLive: browserProcessIsLive,
        })
      ) {
        const retryRecordRemoved = removeVerifiedRuntimeHandoffRecord(prepared);
        report.handoffs.resumed.push({
          sessionName: prepared.sessionName,
          browserPid: browser.pid ?? prepared.browserPid ?? null,
          cdpUrl: browser.cdpEndpoint ?? null,
          runtimeProfile: browser.profileId ?? prepared.runtimeProfile ?? null,
          targetsReattached: Array.isArray(browser.tabHandles)
            ? browser.tabHandles.filter((tab) => tab?.valid === true).length
            : null,
          retryRecordRemoved,
          daemonPid: existingDaemonPid,
          alreadyResumed: true,
        });
        continue;
      }
    }
    let resumed;
    for (let attempt = 1; attempt <= 3; attempt += 1) {
      resumed = runAgentJson(installBin, prepared.sessionName, ['handoff', 'resume'], prepared.displayEnvironment);
      if (resumed.status === 0 && resumed.json?.success === true) break;
      if (attempt < 3) sleep(250);
    }
    if (resumed.status !== 0 || resumed.json?.success !== true) {
      throw new Error(
        `Replacement daemon could not resume browser session '${prepared.sessionName}'. ` +
        `The browser and retry record remain available: ${resumed.error}`,
      );
    }
    const data = resumed.json.data || {};
    const staleAttachedPidDropped = prepared.host === 'attached_existing'
      && data.browserPid == null
      && data.preparedBrowserPid === prepared.browserPid
      && data.staleBrowserPidDropped === true;
    if (
      prepared.browserPid !== null
      && data.browserPid !== prepared.browserPid
      && !staleAttachedPidDropped
    ) {
      throw new Error(
        `Runtime handoff changed browser PID for session '${prepared.sessionName}': ` +
        `${prepared.browserPid} -> ${data.browserPid}`,
      );
    }
    if (prepared.cdpUrl && data.cdpUrl !== prepared.cdpUrl) {
      throw new Error(
        `Runtime handoff changed CDP endpoint for session '${prepared.sessionName}': ` +
        `${prepared.cdpUrl} -> ${data.cdpUrl}`,
      );
    }
    report.handoffs.resumed.push({
      sessionName: prepared.sessionName,
      browserPid: staleAttachedPidDropped
        ? prepared.browserPid
        : data.browserPid ?? null,
      attachedBrowserPid: data.browserPid ?? null,
      staleBrowserPidDropped: staleAttachedPidDropped,
      cdpUrl: data.cdpUrl ?? null,
      runtimeProfile: data.runtimeProfile ?? null,
      targetsReattached: data.targetsReattached ?? null,
      retryRecordRemoved: data.retryRecordRemoved === true,
      daemonPid: readRuntimePid(prepared.sessionName),
    });
  }
}

function discoverPreparedRuntimeHandoffs(candidateSessions) {
  const handoffs = [];
  for (const sessionName of candidateSessions) {
    if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(sessionName)) {
      throw new Error(`Publication journal contains an invalid session name: ${sessionName}`);
    }
    const path = join(runtimeSocketDir(), `${sessionName}.handoff.json`);
    if (!existsSync(path)) continue;
    let descriptor;
    try {
      descriptor = JSON.parse(readFileSync(path, 'utf8'));
    } catch (error) {
      throw new Error(
        `Prepared runtime handoff is invalid for session '${sessionName}': ` +
        `${error instanceof Error ? error.message : String(error)}`,
      );
    }
    const schemaVersion = descriptor.schemaVersion ?? descriptor.schema_version;
    const descriptorSessionName = descriptor.sessionName ?? descriptor.session_name;
    if (![1, 2].includes(schemaVersion) || descriptorSessionName !== sessionName) {
      throw new Error(`Prepared runtime handoff identity mismatch for session '${sessionName}'`);
    }
    handoffs.push({
      sessionName,
      daemonPid: readRuntimePid(sessionName),
      browserPid: descriptor.browserPid ?? descriptor.browser_pid ?? null,
      cdpUrl: descriptor.cdpUrl ?? descriptor.cdp_url ?? null,
      runtimeProfile: descriptor.runtimeProfile ?? descriptor.runtime_profile ?? null,
      host: descriptor.host ?? null,
      handoffPath: path,
    });
  }
  return handoffs;
}

function runAgentJson(binary, sessionName, commandArgs, displayEnvironment = undefined) {
  const result = spawnSync(binary, ['--json', '--session', sessionName, ...commandArgs], {
    cwd: rootDir,
    env: displayEnvironment === undefined
      ? process.env
      : withSessionDisplayEnvironment(process.env, displayEnvironment),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 16 * 1024 * 1024,
    timeout: 30_000,
  });
  let json = null;
  try {
    json = JSON.parse(String(result.stdout || '').trim());
  } catch {
    // The compatibility path uses the structured error below.
  }
  return {
    status: result.status,
    json,
    error: json?.error || result.error?.message || result.stderr?.trim() || result.stdout?.trim() || 'unknown error',
  };
}

function serviceBrowserForSession(binary, sessionName, expectedBrowser = null, displayEnvironment = undefined) {
  const result = runAgentJson(binary, sessionName, ['service', 'browsers'], displayEnvironment);
  const browsers = result.json?.data?.browsers || [];
  const selection = selectRuntimeHandoffBrowser({
    browsers,
    sessionName,
    expectedBrowser,
  });
  return {
    success: result.status === 0
      && result.json?.success === true
      && selection.error === null,
    browser: selection.browser,
    error: selection.error || result.error,
  };
}

function processIdentity(pid) {
  const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = stat.lastIndexOf(')');
  if (close < 0) throw new Error('retained custody process stat is malformed');
  const status = readFileSync(`/proc/${pid}/status`, 'utf8');
  const uidLine = status.split('\n').find(line => line.startsWith('Uid:'));
  const executable = statSync(`/proc/${pid}/exe`);
  return {
    pid,
    bootId: readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim(),
    startTicks: Number(stat.slice(close + 1).trim().split(/\s+/)[19]),
    executableDevice: Number(executable.dev),
    executableInode: Number(executable.ino),
    uid: Number(uidLine?.trim().split(/\s+/)[1]),
  };
}

function sameProcessIdentity(actual, expected) {
  return ['pid', 'bootId', 'startTicks', 'executableDevice', 'executableInode', 'uid']
    .every(key => actual[key] === expected?.[key]);
}

function validateReceiptBackedProfile(browser) {
  const profile = browser.canonicalProfile;
  const metadata = statSync(profile);
  if (!metadata.isDirectory()
    || Number(metadata.dev) !== browser.profileDevice
    || Number(metadata.ino) !== browser.profileInode) {
    throw new Error('retained custody profile identity changed');
  }
  const raw = readFileSync(`/proc/${browser.process.pid}/cmdline`).toString('utf8');
  let args = raw.split('\0').filter(Boolean);
  if (args.length === 1 && raw.endsWith('\0') && !raw.slice(0, -1).includes('\0')) {
    const executable = readlinkSync(`/proc/${browser.process.pid}/exe`);
    if (!args[0].startsWith(`${executable} `)) {
      throw new Error('retained custody process title is not bound to live executable');
    }
    args = args[0].split(/\s+/);
  }
  const profiles = args.map((value, index) => value.startsWith('--user-data-dir=')
    ? value.slice('--user-data-dir='.length)
    : value === '--user-data-dir' ? args[index + 1] : null).filter(Boolean);
  if (profiles.length !== 1 || resolve(profiles[0]) !== profile) {
    throw new Error('retained custody browser profile argument changed');
  }
  const endpoint = new URL(browser.cdpEndpoint);
  const [port, path] = readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').trim().split('\n');
  if (endpoint.protocol !== 'ws:' || endpoint.hostname !== '127.0.0.1'
    || port !== endpoint.port || path !== endpoint.pathname) {
    throw new Error('retained custody DevTools identity changed');
  }
}

async function receiptBackedBrowserForSession(sessionName, expectation) {
  const statePath = join(homedir(), '.agent-browser', 'service', 'state.json');
  const before = readFileSync(statePath);
  const state = JSON.parse(before);
  const receipt = state.runtimeCustodyReceipts?.[sessionName];
  const browserId = `session:${sessionName}`;
  const session = state.sessions?.[sessionName];
  const browser = state.browsers?.[browserId];
  const profile = state.profiles?.[session?.profileId];
  const receiptTab = state.tabs?.[`target:${receipt?.targetId}`];
  const expectedTab = state.tabs?.[`target:${expectation.targetId}`];
  if (receipt?.schemaVersion !== 2 || receipt.phase !== 'committed'
    || session?.lease !== 'exclusive' || session.profileId !== expectation.profileId
    || !browser || browser.id !== browserId || browser.health !== 'ready'
    || browser.pid !== receipt.browser?.process?.pid
    || browser.cdpEndpoint !== receipt.browser?.cdpEndpoint
    || browser.profileId !== session.profileId
    || !profile?.userDataDir || resolve(profile.userDataDir) !== receipt.browser?.canonicalProfile
    || !session.browserIds?.includes(browserId)
    || !session.tabIds?.includes(`target:${receipt.targetId}`)
    || receiptTab?.browserId !== browserId || receiptTab?.ownerSessionId !== sessionName
    || receiptTab?.lifecycle !== 'ready'
    || !session.tabIds?.includes(`target:${expectation.targetId}`)
    || expectedTab?.browserId !== browserId || expectedTab?.ownerSessionId !== sessionName
    || expectedTab?.lifecycle !== 'ready' || expectedTab.url !== expectation.url) {
    throw new Error('receipt-backed retained browser identity is incomplete or changed');
  }
  if (!sameProcessIdentity(processIdentity(browser.pid), receipt.browser.process)
    || !sameProcessIdentity(processIdentity(receipt.destination.pid), receipt.destination)) {
    throw new Error('receipt-backed retained process identity changed');
  }
  validateReceiptBackedProfile(receipt.browser);
  const targets = await readCdpTargetInventory(browser.cdpEndpoint);
  if (!targets.some(target => target?.id === receipt.targetId && target.type === 'page' && target.url === receiptTab.url)
    || !targets.some(target => target?.id === expectation.targetId && target.type === 'page' && target.url === expectation.url)) {
    throw new Error('receipt-backed retained target is absent or changed');
  }
  if (!before.equals(readFileSync(statePath))) {
    throw new Error('retained service state changed during receipt-backed verification');
  }
  return { browser, targets };
}

async function verifyRetainedBrowserExpectation(_binary, { expectation, stage }) {
  const daemonPid = readRuntimePid(expectation.sessionName);
  if (!browserProcessIsLive(daemonPid)) {
    const evidence = evaluateRetainedBrowserExpectation({
      browser: null,
      cdpTargets: null,
      expectation,
      stage,
    });
    evidence.reason = 'retained_daemon_missing';
    evidence.message =
      `Required retained daemon session '${expectation.sessionName}' is not running`;
    const error = new Error(
      `Retained browser guard failed at ${stage}: ${evidence.reason}: ${evidence.message}`,
    );
    error.retainedBrowserEvidence = evidence;
    throw error;
  }
  const daemonClientBin = runtimeDaemonClientBinary(daemonPid, report.installBin);
  let serviceReadback = serviceBrowserForSession(
    daemonClientBin,
    expectation.sessionName,
  );
  if (!serviceReadback.success
    && ['pre_mutation', 'read_only_preflight'].includes(stage)
    && serviceReadback.error === 'migration_profile_target_url_display_changed') {
    try {
      const receiptBacked = await receiptBackedBrowserForSession(expectation.sessionName, expectation);
      serviceReadback = {
        success: true,
        browser: receiptBacked.browser,
        error: null,
        cdpTargets: receiptBacked.targets,
        custodyFallback: 'committed_schema_v2_receipt',
      };
    } catch (error) {
      serviceReadback.error = `migration receipt fallback failed: ${error instanceof Error ? error.message : String(error)}`;
    }
  }
  if (!serviceReadback.success) {
    const error = new Error(
      `Retained browser guard could not read session '${expectation.sessionName}' ` +
      `at ${stage}: ${serviceReadback.error}`,
    );
    error.retainedBrowserEvidence = {
      required: true,
      verified: false,
      stage,
      reason: 'retained_browser_service_read_failed',
      message: serviceReadback.error,
    };
    throw error;
  }
  let cdpTargets = serviceReadback.cdpTargets || null;
  let cdpError = null;
  if (!cdpTargets && serviceReadback.browser?.cdpEndpoint) {
    try {
      cdpTargets = await readCdpTargetInventory(serviceReadback.browser.cdpEndpoint);
    } catch (error) {
      cdpError = error instanceof Error ? error.message : String(error);
    }
  }
  const evidence = evaluateRetainedBrowserExpectation({
    browser: serviceReadback.browser,
    cdpTargets,
    expectation,
    stage,
  });
  if (cdpError) evidence.cdpError = cdpError;
  if (!evidence.verified) {
    const error = new Error(
      `Retained browser guard failed at ${stage}: ${evidence.reason}: ${evidence.message}` +
      (cdpError ? ` (${cdpError})` : ''),
    );
    error.retainedBrowserEvidence = evidence;
    throw error;
  }
  return evidence;
}

async function verifyRecoveredRetainedBrowserExpectation(binary, { expectation, stage }) {
  const expected = normalizeRetainedBrowserExpectation(expectation);
  const daemonPid = readRuntimePid(expected.sessionName);
  const serviceReadback = serviceBrowserForSession(
    browserProcessIsLive(daemonPid)
      ? runtimeDaemonClientBinary(daemonPid, binary)
      : binary,
    expected.sessionName,
  );
  if (!serviceReadback.success || !serviceReadback.browser?.cdpEndpoint) {
    throw new Error(
      `Recovered retained browser '${expected.sessionName}' is unavailable: ${serviceReadback.error || 'missing CDP endpoint'}`,
    );
  }
  const targets = await readCdpTargetInventory(serviceReadback.browser.cdpEndpoint);
  const matches = targets.filter((target) =>
    target?.type === 'page' && retainedTargetUrlsMatch(expected.url, target.url));
  if (matches.length !== 1) {
    throw new Error(
      `Recovered retained conversation matched ${matches.length} page targets; expected exactly one`,
    );
  }
  const replacement = normalizeRetainedBrowserExpectation({
    sessionName: expected.sessionName,
    browserId: expected.browserId || `session:${expected.sessionName}`,
    profileId: expected.profileId,
    targetId: matches[0].id,
    url: expected.url,
  });
  const evidence = evaluateRetainedBrowserExpectation({
    browser: serviceReadback.browser,
    cdpTargets: targets,
    expectation: replacement,
    stage,
  });
  if (!evidence.verified) {
    const error = new Error(
      `Recovered retained browser verification failed: ${evidence.reason}: ${evidence.message}`,
    );
    error.retainedBrowserEvidence = evidence;
    throw error;
  }
  return evidence;
}

async function readCdpTargetInventory(cdpUrl) {
  if (!isLoopbackDevToolsUrl(cdpUrl)) {
    throw new Error('CDP target inventory endpoint must use loopback');
  }
  const endpoint = new URL(cdpUrl);
  endpoint.protocol = endpoint.protocol === 'wss:' ? 'https:' : 'http:';
  endpoint.pathname = '/json/list';
  endpoint.search = '';
  endpoint.hash = '';
  const response = await fetch(endpoint, { signal: AbortSignal.timeout(5000) });
  if (!response.ok) {
    throw new Error(`CDP target inventory returned HTTP ${response.status}`);
  }
  const targets = await response.json();
  if (!Array.isArray(targets)) {
    throw new Error('CDP target inventory is not an array');
  }
  return targets;
}

function readRuntimePid(sessionName) {
  try {
    const value = Number.parseInt(
      readFileSync(join(runtimeSocketDir(), `${sessionName}.pid`), 'utf8').trim(),
      10,
    );
    return Number.isInteger(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

function cdpEndpointIsReachable(cdpUrl) {
  let endpoint;
  try {
    endpoint = new URL(cdpUrl);
  } catch {
    return false;
  }
  if (
    !['ws:', 'wss:'].includes(endpoint.protocol)
    || !['127.0.0.1', 'localhost', '[::1]'].includes(endpoint.hostname)
  ) {
    return false;
  }
  endpoint.protocol = endpoint.protocol === 'wss:' ? 'https:' : 'http:';
  endpoint.pathname = '/json/version';
  endpoint.search = '';
  endpoint.hash = '';
  const probe = spawnSync(process.execPath, [
    '--input-type=module',
    '-e',
    'const response = await fetch(process.argv[1], { signal: AbortSignal.timeout(1000) }); process.exit(response.ok ? 0 : 1);',
    endpoint.href,
  ], {
    encoding: 'utf8',
    stdio: ['ignore', 'ignore', 'ignore'],
    timeout: 2000,
  });
  return probe.status === 0;
}

function waitForDaemonExit(sessionName, priorPid) {
  return waitForRuntimeDaemonExit(sessionName, priorPid, {
    readRuntimePid, isProcessLive: browserProcessIsLive,
  });
}

function sleep(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function resolveInstallBin() {
  if (options.installBin) return resolve(options.installBin);
  const defaultPath = resolve(homedir(), '.local/bin/agent-browser');
  if (existsSync(defaultPath)) return defaultPath;
  const pathValue = commandOutput('sh', ['-lc', 'command -v agent-browser']).trim();
  if (pathValue) return resolve(pathValue);
  return defaultPath;
}

function guardInstallPath(path) {
  requireLegacyPublicationTarget(path);
  if (options.allowOutsideHome) return;
  const home = resolve(homedir());
  const resolved = resolve(path);
  if (resolved !== home && !resolved.startsWith(`${home}/`)) {
    throw new Error(`Refusing to replace a binary outside the current user's home without --allow-outside-home: ${resolved}`);
  }
}

function quiesceDashboardForRuntimeHandoff() {
  if (
    report.service.before?.loadState === 'loaded'
    && report.service.before?.activeState === 'active'
  ) {
    runCommand('systemctl', ['--user', 'stop', 'agent-browser-dashboard.service']);
    report.service.quiesced = true;
    report.service.action = 'stop-for-runtime-handoff';
    return;
  }
  if (process.platform === 'linux') {
    quiesceStandaloneDashboardForRuntimeHandoff({
      runtimeSocketDir: runtimeSocketDir(),
      service: report.service,
    });
  }
}

// Prove the only session consumed by systemd stop is its idle backend. A
// session-name exception alone could terminate a retained browser unnoticed.
async function prepareQuiesceSessions(expectedSessions) {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      return await prepareQuiesceSessionsOnce(expectedSessions);
    } catch (error) {
      const procRace = error?.code === 'ENOENT'
        && typeof error?.path === 'string'
        && error.path.startsWith('/proc/');
      if (!procRace || attempt === 2) throw error;
      requireExpectedPublicationSessions(expectedSessions, runtimeSessionNames());
      await new Promise(resolve => setTimeout(resolve, 25));
    }
  }
  throw new Error('Quiescence evidence retry exhausted');
}

async function prepareQuiesceSessionsOnce(expectedSessions) {
  if (report.service.before?.activeState !== 'active' || process.platform !== 'linux') return null;
  const dashboardCgroup = commandOutput('systemctl', ['--user', 'show', 'agent-browser-dashboard.service', '--property=ControlGroup', '--value']).trim();
  const state = JSON.parse(readFileSync(join(homedir(), '.agent-browser/service/state.json'), 'utf8'));
  for (const key of ['browsers', 'jobs', 'viewerLeases']) {
    if (!state[key] || typeof state[key] !== 'object') throw new Error(`Missing quiescence evidence: ${key}`);
  }
  const activeViewerLeases = Object.values(state.viewerLeases).filter(row => !['disconnected', 'expired', 'released', 'failed'].includes(row.state)).length;
  const evidence = expectedSessions.map(sessionName => {
    const daemonPid = readRuntimePid(sessionName);
    const readback = serviceBrowserForSession(runtimeDaemonClientBinary(daemonPid, report.installBin), sessionName);
    if (!readback.success) throw new Error(`Cannot verify browser ownership before dashboard stop: ${sessionName}`);
    const browserRows = Object.values(state.browsers).filter(browser => browser.id === `session:${sessionName}`);
    if (readback.browser) browserRows.push(readback.browser);
    const browserPids = [...new Set(browserRows.map(browser => browser.pid).filter(pid => Number.isInteger(pid) && pid > 0))];
    return {
      sessionName, daemonPid, daemonCgroup: processCgroup(daemonPid), browserPids,
      browserCgroups: browserPids.map(processCgroup), hasPersistedBrowser: browserRows.length > 0,
      activeJobs: null, activeViewerLeases,
    };
  });
  // Also inspect browser rows outside the socket inventory before killing any
  // cgroup. A missing daemon socket is not proof that its browser is absent.
  for (const browser of Object.values(state.browsers)) {
    if (Number.isInteger(browser.pid) && browser.pid > 0) {
      const group = processCgroup(browser.pid);
      if (group === dashboardCgroup || group.startsWith(`${dashboardCgroup}/`)) {
        throw new Error('Dashboard cgroup contains a retained browser; refusing quiescence');
      }
    }
  }
  // Dashboard event polling itself is a short-lived service job. Observe a
  // bounded idle point after readback rather than reusing an older busy snapshot.
  const idleDeadline = Date.now() + 5000;
  let activeJobs;
  let current;
  do {
    current = JSON.parse(readFileSync(join(homedir(), '.agent-browser/service/state.json'), 'utf8'));
    if (!current.jobs || typeof current.jobs !== 'object') throw new Error('Missing current job evidence');
    activeJobs = Object.values(current.jobs).filter(row => !['succeeded', 'failed', 'cancelled', 'canceled'].includes(row.state)).length;
    if (activeJobs === 0) break;
    await new Promise(resolve => setTimeout(resolve, 50));
  } while (Date.now() < idleDeadline);
  const refreshedEvidence = refreshDashboardSessionEvidence({ evidence, state: current, readCgroup: processCgroup });
  return prepareDashboardSessionQuiesce({ expectedSessions, currentSessions: runtimeSessionNames(), dashboardCgroup, evidence: refreshedEvidence });
}

function verifyQuiesceSessions(record) {
  return verifyDashboardSessionQuiesce(record, {
    currentSessions: runtimeSessionNames(),
    evidence: record.evidence.map(row => ({
      sessionName: row.sessionName,
      daemonPid: readRuntimePid(row.sessionName) ?? row.daemonPid,
      daemonAlive: browserProcessIsLive(row.daemonPid),
      socketPresent: existsSync(join(runtimeSocketDir(), `${row.sessionName}.sock`)),
    })),
  }).expectedSessions;
}

function processCgroup(pid) {
  const rows = readFileSync(`/proc/${pid}/cgroup`, 'utf8').trim().split('\n');
  const unified = rows.find(row => row.startsWith('0::'));
  if (!unified) throw new Error(`Cannot verify unified cgroup for PID ${pid}`);
  return unified.slice(3);
}

function verifyListenerEvidence({ pid }) {
  const startTime = () => readFileSync(`/proc/${pid}/stat`, 'utf8').split(') ').at(-1).split(' ')[19];
  const processStartTimeBefore = startTime();
  const executablePath = readlinkSync(`/proc/${pid}/exe`).replace(/ \(deleted\)$/, '');
  const sha256 = sha256File(`/proc/${pid}/exe`);
  const processStartTimeAfter = startTime();
  return { pid, executablePath, sha256, processStartTimeBefore, processStartTimeAfter };
}

async function restartOrStartDashboard(installBin, { restoring = false } = {}) {
  const restart = restartOrStartDashboardRuntime({
    installBin,
    restoring,
    startIfMissing: options.startIfMissing,
    service: report.service,
    serviceStatus,
    runCommand,
  });
  if (!restart.started) return;
  const deadline = Date.now() + 30000;
  let lastError;
  // systemctl restart acknowledges process startup, not HTTP readiness.
  while (Date.now() < deadline) {
    try {
      const response = await fetch(new URL('/api/runtime/manifest', options.dashboardUrl), { signal: AbortSignal.timeout(3000) });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      verifyRuntimeManifestReadback(installBin, await response.json());
      return;
    } catch (error) { lastError = error; }
    await new Promise(resolve => setTimeout(resolve, 250));
  }
  throw new Error(`Dashboard did not become ready after restart: ${lastError?.message}`);
}

function serviceStatus() {
  const result = spawnSync('systemctl', [
    '--user',
    'show',
    'agent-browser-dashboard.service',
    '--property=LoadState',
    '--property=ActiveState',
    '--property=MainPID',
    '--property=ActiveEnterTimestamp',
  ], {
    cwd: rootDir,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    return {
      loadState: 'unknown',
      activeState: 'unknown',
      mainPid: null,
      activeEnterTimestamp: null,
      error: (result.stderr || result.stdout || '').trim(),
    };
  }
  const values = {};
  for (const line of result.stdout.split(/\r?\n/)) {
    const index = line.indexOf('=');
    if (index <= 0) continue;
    values[line.slice(0, index)] = line.slice(index + 1);
  }
  return {
    loadState: values.LoadState || 'unknown',
    activeState: values.ActiveState || 'unknown',
    mainPid: Number(values.MainPID || 0) || null,
    activeEnterTimestamp: values.ActiveEnterTimestamp || null,
  };
}

function smokeArgs(installBin, { skipBrowser }) {
  const smokeArgs = [
    'scripts/smoke-local-dashboard-runtime.js',
    '--dashboard-url',
    options.dashboardUrl,
    '--agent-browser-bin',
    installBin,
    '--json',
  ];
  for (const marker of options.expectMarkers) {
    smokeArgs.push('--expect-marker', marker);
  }
  if (skipBrowser) smokeArgs.push('--skip-browser');
  if (options.browserBuild) smokeArgs.push('--browser-build', options.browserBuild);
  if (options.browserProfile) smokeArgs.push('--browser-profile', options.browserProfile);
  if (options.workspaceSession) smokeArgs.push('--workspace-session', options.workspaceSession);
  return smokeArgs;
}

function runSmokeProcess(installBin, { skipBrowser }) {
  const commandArgs = smokeArgs(installBin, { skipBrowser });

  const result = spawnSync('node', commandArgs, {
    cwd: rootDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const parsed = parseJson(result.stdout, 'local dashboard runtime smoke');
  return { result, parsed };
}

function runHttpReadinessSmoke(installBin) {
  const { result, parsed } = runSmokeProcess(installBin, { skipBrowser: true });
  if (result.status !== 0 || !parsed.success) {
    throw new Error(`Local dashboard HTTP readiness smoke failed: ${parsed.error || result.stderr || result.stdout}`);
  }
  return parsed;
}

function runBrowserSmokeDiagnostic(installBin) {
  const { result, parsed } = runSmokeProcess(installBin, { skipBrowser: false });
  const disposition = evaluateLocalDashboardBrowserSmokeResult({
    processStatus: result.status,
    parsed,
    stderr: result.stderr,
    stdout: result.stdout,
    required: options.requireBrowserSmoke,
  });
  if (disposition.fatal) {
    report.browserSmoke = disposition;
    const error = new Error(`Local dashboard browser smoke failed: ${disposition.error}`);
    throw error;
  }
  return disposition;
}

function verifyRuntimeManifestReadback(installBin, manifest) {
  if (!manifest || manifest.schemaVersion !== 'agent-browser.runtime-manifest.v1') {
    throw new Error(`Live runtime manifest is missing or invalid: ${JSON.stringify(manifest)}`);
  }
  if (manifest.serviceContractVersion !== 'service-ui-runtime.v1') {
    throw new Error(`Live runtime manifest contract mismatch: ${manifest.serviceContractVersion}`);
  }
  const installedSha = sha256File(installBin);
  const manifestSha = manifest.executable?.sha256;
  if (manifestSha !== installedSha) {
    throw new Error(`Live runtime manifest executable sha mismatch: manifest=${manifestSha || 'missing'} installed=${installedSha}`);
  }
  if (typeof manifest.dashboard?.sha256 !== 'string' || manifest.dashboard.sha256.length !== 64) {
    throw new Error(`Live runtime manifest dashboard sha is missing: ${JSON.stringify(manifest.dashboard)}`);
  }
  const features = new Set(Array.isArray(manifest.supportedUiFeatures) ? manifest.supportedUiFeatures : []);
  for (const feature of ['workspace.detectedBrowsers', 'workspace.foreignCdpBorrow', 'workspace.noRetainedLiveRail']) {
    if (!features.has(feature)) {
      throw new Error(`Live runtime manifest missing feature ${feature}`);
    }
  }
  return {
    schemaVersion: manifest.schemaVersion,
    packageVersion: manifest.packageVersion,
    serviceContractVersion: manifest.serviceContractVersion,
    dashboardSha256: manifest.dashboard.sha256,
    dashboardAssetCount: manifest.dashboard.assetCount,
    executablePath: manifest.executable?.path ?? null,
    executableSha256: manifestSha,
    installedSha256: installedSha,
    supportedUiFeatures: [...features].sort(),
  };
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function runCommand(command, commandArgs) {
  log(`$ ${command} ${commandArgs.join(' ')}`);
  const result = spawnSync(command, commandArgs, {
    cwd: rootDir,
    env: process.env,
    encoding: 'utf8',
    stdio: options.json ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  });
  if (options.json && result.stdout) process.stderr.write(result.stdout);
  if (options.json && result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`${command} ${commandArgs.join(' ')} failed with status ${result.status}`);
  }
}

function commandOutput(command, commandArgs, extra = {}) {
  try {
    return execFileSync(command, commandArgs, {
      cwd: rootDir,
      encoding: 'utf8',
      ...extra,
    });
  } catch {
    return '';
  }
}

function parseJson(text, label) {
  try {
    return JSON.parse(String(text).trim());
  } catch (error) {
    throw new Error(`Failed to parse ${label} JSON: ${error instanceof Error ? error.message : String(error)}\n${text}`);
  }
}

function timestamp() {
  const now = new Date();
  return now.toISOString().replace(/[-:]/g, '').replace(/\..+/, '').replace('T', '');
}

function log(message) {
  if (options.json) {
    process.stderr.write(`${message}\n`);
  }
}

function output(payload) {
  if (options.json) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }
  if (!payload.success) {
    console.error(payload.error);
    return;
  }
  if (payload.operation === 'journal_status') {
    const status = payload.publicationJournalStatus;
    console.log(`Publication journal: ${status.exists ? status.transaction.phase : 'absent'}`);
    console.log(`Recommended action: ${status.recommendedAction}`);
    return;
  }
  if (payload.operation === 'recover_only' && payload.recovery?.result === 'nothing_to_recover') {
    console.log('No incomplete local dashboard publication requires recovery.');
    return;
  }
  if (payload.operation === 'retained_browser_status') {
    console.log('Retained browser publication guard: verified');
    return;
  }
  if (payload.operation === 'write_retained_browser_requirement') {
    console.log(`Retained browser publication requirement: ${payload.retainedBrowserRequirement.written ? 'written' : 'already pinned'}`);
    return;
  }
  console.log(`Published local dashboard runtime to ${payload.installBin}`);
  console.log(`Backup: ${payload.backupPath ?? 'none'}`);
  console.log(`Dashboard: ${payload.dashboardUrl}`);
  console.log(`Service PID: ${payload.service?.after?.mainPid ?? 'none'}`);
  if (payload.smoke?.browser) {
    console.log(`Browser smoke: ${payload.smoke.browser.smokeUrl}`);
  } else if (payload.browserSmoke?.status === 'passed') {
    console.log(`Browser smoke: ${payload.browserSmoke.evidence?.smokeUrl ?? 'passed'}`);
  } else if (payload.browserSmoke?.status === 'unavailable') {
    console.log(`Browser smoke: unavailable (${payload.browserSmoke.classification})`);
  }
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function publicRetainedBrowserRequirement(requirement) {
  if (!requirement) return null;
  return {
    path: requirement.path,
    exists: requirement.exists === true,
    sha256: requirement.sha256 ?? null,
    createdAt: requirement.createdAt ?? null,
    written: requirement.written === true,
  };
}

function fail(message) {
  console.error(message);
  process.exit(2);
}

function printHelp() {
  console.log(`Usage: node scripts/publish-local-dashboard-runtime.js [options]

Build and install the dashboard-embedded local agent-browser binary, restart the
user dashboard service, hand active browser sessions to replacement daemons
without changing their browser PIDs or CDP endpoints, and verify the externally
visible dashboard runtime.

Options:
  --recover-interlock-receipt <id>
                              With --recover-only, recover exact prior dead-publisher timer custody.
  --recover-replaced-retained-browser <transaction-id>
                              With --recover-only, explicitly acknowledge an exited retained
                              process after verifying one replacement target at the same session,
                              profile, and conversation URL. The identity loss remains journaled.
  --prebuilt-bin <absolute-path>
                              Publish reviewed embedded-dashboard bytes without rebuilding.
  --expected-sha256 <sha256>  Required lowercase digest for --prebuilt-bin.
  --controller-update <path=absolute-source=sha256>
                              Repeatable, prebuilt-only existing scripts/ asset update.
                              Candidate must embed those exact bytes. Journal snapshots
                              restore scripts, binary and manifest together on rollback.
  --expected-sessions <names|none>
                              Required exact comma-separated session inventory for prebuilt mode.
                              Prebuilt mode requires --skip-reference-sync --skip-browser;
                              --release and --skip-smoke are rejected.
  --dashboard-url <url>       Dashboard URL to smoke. Default: http://127.0.0.1:4848/
  --discover-retained-url-prefix <url>
                              With requirement write, discover exactly one ready target under this reviewed prefix.
  --discover-retained-exact-url <url>
                              With requirement write, discover exactly one ready target at this canonical URL.
  --discover-retained-profile <id>
                              Require the exact-URL discovery target to use this runtime profile.
  --expect-marker <text>      Require served HTML or JS bundle to contain text. Repeatable.
  --expect-retained-session <name>
                              Require this daemon session before mutation and after handoff.
  --expect-retained-profile <id>
                              Require and pin this retained runtime profile.
  --expect-retained-target <id>
                              Require exactly this CDP page target.
  --expect-retained-url <url> Require the exact URL on the retained target.
  --expect-retained-cdp-url <url>
                              Require this browser DevTools endpoint before pinning.
  --browser-build <build>     Require a verified build for disposable browser smoke.
  --browser-profile <path>    Use an isolated runtime profile for browser smoke.
  --install-bin <path>        Installed binary path. Default: ~/.local/bin/agent-browser.
  --journal-status            Read publication journal and artifact status without locking or mutation.
  --retained-browser-status   Verify only the required retained identity; no lock, build, or mutation.
  --retained-requirement <path>
                              Override the private durable retained-lane requirement path.
  --write-retained-requirement
                              Verify and privately pin explicit or uniquely discovered retained identity.
  --recover-only              Recover one incomplete transaction; never start a new build.
  --release                   Build cli/target/release/agent-browser instead of debug.
  --skip-browser              Skip browser smoke, keep required HTTP and bundle readiness.
  --require-browser-smoke     Fail when the disposable browser cannot launch.
  --skip-reference-sync        Do not sync ignored workspace and pnpm package binaries.
  --skip-smoke                Build, install, and restart without smoke.
  --start-if-missing          Start dashboard if the user service is not installed.
  --workspace-session <name>  Smoke a workspace viewport route for a daemon session.
  --json                      Print structured JSON.
`);
}
