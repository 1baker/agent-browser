import {
  closeSync,
  chmodSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname } from 'node:path';

export const LOCAL_DASHBOARD_PUBLICATION_SCHEMA =
  'agent-browser.local-dashboard-publication.v1';

const TERMINAL_PHASES = new Set([
  'ready',
  'rolled_back',
  'recovered_ready',
  'recovered_rolled_back',
]);

export function createLocalDashboardPublicationJournal({
  journalPath,
  now = () => new Date().toISOString(),
  ownerPid = process.pid,
} = {}) {
  if (typeof journalPath !== 'string' || journalPath.length === 0) {
    throw new Error('Local dashboard publication journal path is required');
  }
  const lockPath = `${journalPath}.lock`;
  let lockHeld = false;

  return {
    path: journalPath,
    lockPath,
    lockStatus() {
      if (!existsSync(lockPath)) {
        return { path: lockPath, present: false, ownerPid: null, live: false, stale: false };
      }
      const lockOwnerPid = readOwnerPid(lockPath);
      const live = lockOwnerPid !== null && processIsLive(lockOwnerPid);
      return {
        path: lockPath,
        present: true,
        ownerPid: lockOwnerPid,
        live,
        stale: !live,
      };
    },
    acquire() {
      if (lockHeld) return;
      mkdirSync(dirname(journalPath), { recursive: true, mode: 0o700 });
      chmodSync(dirname(journalPath), 0o700);
      acquireExactLock(lockPath, ownerPid);
      lockHeld = true;
    },
    release() {
      if (!lockHeld) return;
      releaseExactLock(lockPath, ownerPid);
      lockHeld = false;
    },
    read() {
      if (!existsSync(journalPath)) return null;
      let value;
      try {
        value = JSON.parse(readFileSync(journalPath, 'utf8'));
      } catch (error) {
        throw new Error(`Local dashboard publication journal is invalid: ${errorMessage(error)}`);
      }
      validateRecord(value);
      return value;
    },
    create(payload) {
      requireLock(lockHeld);
      const existing = this.read();
      if (existing && !isTerminalPublicationPhase(existing.phase)) {
        throw new Error(
          `Incomplete local dashboard publication transaction requires recovery: ` +
          `${existing.transactionId} phase=${existing.phase}`,
        );
      }
      const createdAt = now();
      const transactionId = `local-dashboard-${randomUUID()}`;
      const record = {
        ...payload,
        schemaVersion: LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
        transactionId,
        revision: 1,
        phase: 'prepared',
        terminal: false,
        createdAt,
        updatedAt: createdAt,
      };
      validateRecord(record);
      atomicWriteJson(journalPath, record, ownerPid);
      return record;
    },
    commit(record, phase, patch = {}) {
      requireLock(lockHeld);
      validateRecord(record);
      const current = this.read();
      if (
        !current
        || current.transactionId !== record.transactionId
        || current.revision !== record.revision
      ) {
        throw new Error(
          `Local dashboard publication journal revision conflict for ${record.transactionId}`,
        );
      }
      const next = {
        ...record,
        ...patch,
        schemaVersion: LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
        transactionId: record.transactionId,
        revision: record.revision + 1,
        phase,
        terminal: isTerminalPublicationPhase(phase),
        updatedAt: now(),
      };
      validateRecord(next);
      atomicWriteJson(journalPath, next, ownerPid);
      return next;
    },
  };
}

export function isTerminalPublicationPhase(phase) {
  return TERMINAL_PHASES.has(phase);
}

export function inspectLocalDashboardPublicationJournal({
  journal,
  pathExists,
  sha256File,
}) {
  const lock = journal.lockStatus();
  const record = journal.read();
  if (!record) {
    return {
      schemaVersion: LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
      journalPath: journal.path,
      exists: false,
      lock,
      transaction: null,
      installedArtifact: null,
      recoverable: false,
      recommendedAction: lock.live ? 'wait_for_active_publisher' : 'none',
    };
  }

  const installBin = typeof record.installBin === 'string' ? record.installBin : null;
  let installedArtifact = {
    path: installBin,
    exists: false,
    sha256: null,
    classification: 'missing',
    verified: false,
  };
  if (installBin && pathExists(installBin)) {
    const sha256 = sha256File(installBin);
    const classification = classifyInstalledArtifact(record.artifactEvidence, sha256);
    installedArtifact = {
      path: installBin,
      exists: true,
      sha256,
      classification,
      verified: classification !== 'unknown',
    };
  }

  const terminal = isTerminalPublicationPhase(record.phase);
  const retainedBrowserExpectationRequired =
    record.retainedBrowserExpectation?.required === true;
  const retainedBrowserExpectationVerified = retainedBrowserExpectationRequired
    ? record.retainedBrowserExpectation?.final?.verified === true
    : null;
  const terminalRetainedBrowserUnverified = terminal
    && retainedBrowserExpectationRequired
    && !retainedBrowserExpectationVerified;
  const recoverable = !terminal && installedArtifact.verified && !lock.live;
  let recommendedAction = 'none';
  if (lock.live) {
    recommendedAction = 'wait_for_active_publisher';
  } else if (terminalRetainedBrowserUnverified) {
    recommendedAction = 'investigate_retained_browser';
  } else if (!terminal && !installedArtifact.verified) {
    recommendedAction = 'investigate_installed_artifact';
  } else if (recoverable) {
    recommendedAction = 'recover_only';
  }

  return {
    schemaVersion: LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
    journalPath: journal.path,
    exists: true,
    lock,
    transaction: {
      transactionId: record.transactionId,
      revision: record.revision,
      phase: record.phase,
      terminal,
      createdAt: record.createdAt ?? null,
      updatedAt: record.updatedAt ?? null,
      installBin,
      builtBin: record.builtBin ?? null,
      backupPath: record.backupPath ?? null,
      candidateSessionCount: Array.isArray(record.candidateSessions)
        ? record.candidateSessions.length
        : 0,
      preparedHandoffCount: Array.isArray(record.handoffs) ? record.handoffs.length : 0,
      resumedHandoffCount: Array.isArray(record.resumedHandoffs)
        ? record.resumedHandoffs.length
        : 0,
      retainedBrowserExpectationRequired:
        retainedBrowserExpectationRequired,
      retainedBrowserExpectationVerified:
        retainedBrowserExpectationVerified,
      retainedBrowserExpectationStage:
        record.retainedBrowserExpectation?.final?.stage
        ?? record.retainedBrowserExpectation?.afterHandoff?.stage
        ?? record.retainedBrowserExpectation?.before?.stage
        ?? null,
      failure: record.failure ?? record.originalFailure ?? null,
      recoveryError: record.recoveryError ?? null,
    },
    installedArtifact,
    recoverable,
    recommendedAction,
  };
}

function classifyInstalledArtifact(artifactEvidence, sha256) {
  if (
    artifactEvidence?.replacement?.verified === true
    && sha256 === artifactEvidence.replacement.actualSha256
  ) {
    return 'replacement';
  }
  if (
    artifactEvidence?.backup?.verified === true
    && sha256 === artifactEvidence.backup.sha256
  ) {
    return 'backup';
  }
  if (
    typeof artifactEvidence?.built?.sha256 === 'string'
    && sha256 === artifactEvidence.built.sha256
  ) {
    return 'built_replacement';
  }
  return 'unknown';
}

function validateRecord(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Local dashboard publication journal must be an object');
  }
  if (value.schemaVersion !== LOCAL_DASHBOARD_PUBLICATION_SCHEMA) {
    throw new Error(`Unsupported local dashboard publication journal schema: ${value.schemaVersion}`);
  }
  if (typeof value.transactionId !== 'string' || value.transactionId.length === 0) {
    throw new Error('Local dashboard publication journal transactionId is required');
  }
  if (!Number.isInteger(value.revision) || value.revision < 1) {
    throw new Error('Local dashboard publication journal revision must be positive');
  }
  if (typeof value.phase !== 'string' || value.phase.length === 0) {
    throw new Error('Local dashboard publication journal phase is required');
  }
  if (value.terminal !== isTerminalPublicationPhase(value.phase)) {
    throw new Error('Local dashboard publication journal terminal state does not match phase');
  }
}

function atomicWriteJson(path, value, ownerPid) {
  const directory = dirname(path);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const staged = `${path}.next-${ownerPid}`;
  let descriptor = null;
  try {
    rmSync(staged, { force: true });
    descriptor = openSync(staged, 'wx', 0o600);
    writeFileSync(descriptor, `${JSON.stringify(value, null, 2)}\n`);
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = null;
    renameSync(staged, path);
    fsyncDirectory(directory);
  } catch (error) {
    if (descriptor !== null) closeSync(descriptor);
    rmSync(staged, { force: true });
    throw error;
  }
}

function fsyncDirectory(directory) {
  let descriptor = null;
  try {
    descriptor = openSync(directory, 'r');
    fsyncSync(descriptor);
  } catch (error) {
    if (!['EINVAL', 'EISDIR', 'EPERM', 'ENOTSUP'].includes(error?.code)) throw error;
  } finally {
    if (descriptor !== null) closeSync(descriptor);
  }
}

function acquireExactLock(path, ownerPid) {
  try {
    const descriptor = openSync(path, 'wx', 0o600);
    writeFileSync(descriptor, `${ownerPid}\n`);
    fsyncSync(descriptor);
    closeSync(descriptor);
    return;
  } catch (error) {
    if (error?.code !== 'EEXIST') throw error;
  }

  const currentOwner = readOwnerPid(path);
  if (currentOwner !== null && processIsLive(currentOwner)) {
    throw new Error(`Local dashboard publication is already active in process ${currentOwner}`);
  }
  rmSync(path, { force: true });
  const descriptor = openSync(path, 'wx', 0o600);
  writeFileSync(descriptor, `${ownerPid}\n`);
  fsyncSync(descriptor);
  closeSync(descriptor);
}

function releaseExactLock(path, ownerPid) {
  if (readOwnerPid(path) === ownerPid) rmSync(path, { force: true });
}

function readOwnerPid(path) {
  try {
    const value = Number.parseInt(readFileSync(path, 'utf8').trim(), 10);
    return Number.isInteger(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

function processIsLive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function requireLock(lockHeld) {
  if (!lockHeld) throw new Error('Local dashboard publication journal lock is not held');
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}
