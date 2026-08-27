#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
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
import { retirePreparedDaemon } from './lib/prepared-daemon-retirement.js';
import {
  resolveRuntimeDaemonClientBinary as runtimeDaemonClientBinary,
} from './lib/runtime-daemon-client-binary.js';

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
  recoverOnly: false,
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
  } else if (arg === '--recover-only') {
    options.recoverOnly = true;
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
  if (options.journalStatus) {
    report.publicationJournalStatus = inspectLocalDashboardPublicationJournal({
      journal: publicationJournal,
      pathExists: existsSync,
      sha256File,
    });
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
      serviceStatus,
      backupInstalledBinary,
      quiesceDashboardForRuntimeHandoff,
      prepareRuntimeHandoffs,
      installBinaryAtomically,
      syncReferenceBinaries,
      resumeRuntimeHandoffs,
      restartOrStartDashboard,
      runHttpReadinessSmoke,
      verifyRuntimeManifestReadback,
      verifyRetainedBrowserExpectation,
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

function installBinaryAtomically(source, target, mode) {
  mkdirSync(dirname(target), { recursive: true });
  const staged = `${target}.next-${timestamp()}-${process.pid}`;
  try {
    copyFileSync(source, staged);
    chmodSync(staged, mode);
    renameSync(staged, target);
  } catch (error) {
    rmSync(staged, { force: true });
    throw error;
  }
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

function prepareRuntimeHandoffs(clientBin, rollbackBin) {
  try {
    for (const sessionName of runtimeSessionNames()) {
      const daemonPid = readRuntimePid(sessionName);
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
            handoffPath: data.handoffPath ?? null,
            strandedDaemonTermination: null,
          };
          report.handoffs.prepared.push(preparedHandoff);
          try {
            waitForDaemonExit(sessionName, daemonPid);
          } catch {
            preparedHandoff.strandedDaemonTermination =
              retirePreparedDaemon(preparedHandoff);
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
      const resumed = runAgentJson(rollbackBin, prepared.sessionName, ['handoff', 'resume']);
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
    const existing = serviceBrowserForSession(
      installBin,
      prepared.sessionName,
      prepared,
    );
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
        !['closed', 'not_started'].includes(browser.health)
        && (
          browserProcessIsLive(browser.pid ?? prepared.browserPid)
          || (typeof browser.cdpEndpoint === 'string' && browser.cdpEndpoint.length > 0)
        )
      ) {
        report.handoffs.resumed.push({
          sessionName: prepared.sessionName,
          browserPid: browser.pid ?? prepared.browserPid ?? null,
          cdpUrl: browser.cdpEndpoint ?? null,
          runtimeProfile: browser.profileId ?? prepared.runtimeProfile ?? null,
          targetsReattached: Array.isArray(browser.tabHandles)
            ? browser.tabHandles.filter((tab) => tab?.valid === true).length
            : null,
          retryRecordRemoved: !existsSync(prepared.handoffPath || ''),
          daemonPid: readRuntimePid(prepared.sessionName),
          alreadyResumed: true,
        });
        continue;
      }
    }
    let resumed;
    for (let attempt = 1; attempt <= 3; attempt += 1) {
      resumed = runAgentJson(installBin, prepared.sessionName, ['handoff', 'resume']);
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
    if (prepared.browserPid !== null && data.browserPid !== prepared.browserPid) {
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
      browserPid: data.browserPid ?? null,
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
    if (schemaVersion !== 1 || descriptorSessionName !== sessionName) {
      throw new Error(`Prepared runtime handoff identity mismatch for session '${sessionName}'`);
    }
    handoffs.push({
      sessionName,
      daemonPid: readRuntimePid(sessionName),
      browserPid: descriptor.browserPid ?? descriptor.browser_pid ?? null,
      cdpUrl: descriptor.cdpUrl ?? descriptor.cdp_url ?? null,
      runtimeProfile: descriptor.runtimeProfile ?? descriptor.runtime_profile ?? null,
      handoffPath: path,
    });
  }
  return handoffs;
}

function runAgentJson(binary, sessionName, commandArgs) {
  const result = spawnSync(binary, ['--json', '--session', sessionName, ...commandArgs], {
    cwd: rootDir,
    env: process.env,
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

function serviceBrowserForSession(binary, sessionName, expectedBrowser = null) {
  const result = runAgentJson(binary, sessionName, ['service', 'browsers']);
  const browsers = result.json?.data?.browsers || [];
  const sessionBrowser = browsers.find(
    (browser) => browser?.id === `session:${sessionName}`,
  ) || null;
  const exactIdentityBrowsers = expectedBrowser
    ? browsers.filter((browser) =>
      (
        expectedBrowser.browserPid == null
        || browser?.pid == null
        || browser?.pid === expectedBrowser.browserPid
      )
      && (!expectedBrowser.cdpUrl || browser?.cdpEndpoint === expectedBrowser.cdpUrl))
    : [];
  const browser = sessionBrowser || (
    exactIdentityBrowsers.length === 1 ? exactIdentityBrowsers[0] : null
  );
  return {
    success: result.status === 0 && result.json?.success === true,
    browser,
    error: exactIdentityBrowsers.length > 1
      ? `Multiple service browsers match the prepared PID/CDP identity for '${sessionName}'`
      : result.error,
  };
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
  const serviceReadback = serviceBrowserForSession(
    daemonClientBin,
    expectation.sessionName,
  );
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
  let cdpTargets = null;
  let cdpError = null;
  if (serviceReadback.browser?.cdpEndpoint) {
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

function browserProcessIsLive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function waitForDaemonExit(sessionName, priorPid) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    const currentPid = readRuntimePid(sessionName);
    if (currentPid === null && !browserProcessIsLive(priorPid)) return;
    sleep(50);
  }
  throw new Error(`Daemon session '${sessionName}' did not exit for executable handoff`);
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

async function restartOrStartDashboard(installBin, { restoring = false } = {}) {
  restartOrStartDashboardRuntime({
    installBin,
    restoring,
    startIfMissing: options.startIfMissing,
    service: report.service,
    serviceStatus,
    runCommand,
  });
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
