import { createHash } from 'node:crypto';
import {
  chmodSync,
  closeSync,
  constants,
  fstatSync,
  fsyncSync,
  lstatSync,
  linkSync,
  mkdirSync,
  openSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname } from 'node:path';

import {
  normalizeRetainedBrowserExpectation,
  pinRetainedBrowserExpectation,
} from './local-dashboard-retained-browser-guard.js';

export const LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA =
  'agent-browser.local-dashboard-retained-browser-requirement.v1';
export const LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA =
  'agent-browser.local-dashboard-retained-browser-enforcement.v1';

const MAX_REQUIREMENT_BYTES = 16 * 1024;

/** Read one private, bounded, durable retained-lane requirement without mutation. */
export function readRetainedBrowserRequirement(path, { allowEnforcedMissing = false } = {}) {
  if (typeof path !== 'string' || !path) {
    throw new Error('Retained browser requirement path is required');
  }
  const enforcement = readRetainedBrowserEnforcement(path);
  if (!pathEntryExists(path)) {
    if (enforcement.exists && !allowEnforcedMissing) {
      throw new Error('Required retained browser requirement is missing');
    }
    return {
      schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
      path,
      exists: false,
      sha256: null,
      expectation: null,
      createdAt: null,
      enforcement,
    };
  }
  let descriptor = null;
  try {
    descriptor = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  } catch (error) {
    if (error?.code === 'ELOOP') {
      throw new Error('Retained browser requirement must be a regular non-symlink file');
    }
    throw error;
  }
  let bytes;
  try {
    const metadata = fstatSync(descriptor);
    if (!metadata.isFile()) {
      throw new Error('Retained browser requirement must be a regular non-symlink file');
    }
    if (metadata.size > MAX_REQUIREMENT_BYTES) {
      throw new Error(`Retained browser requirement exceeds ${MAX_REQUIREMENT_BYTES} bytes`);
    }
    if (process.platform !== 'win32' && (metadata.mode & 0o077) !== 0) {
      throw new Error('Retained browser requirement permissions must be 0600 or stricter');
    }
    if (
      process.platform !== 'win32'
      && typeof process.getuid === 'function'
      && metadata.uid !== process.getuid()
    ) {
      throw new Error('Retained browser requirement must be owned by the current user');
    }
    bytes = readFileSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
  let value;
  try {
    value = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    throw new Error(
      `Retained browser requirement is invalid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (value?.schemaVersion !== LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA) {
    throw new Error(`Unsupported retained browser requirement schema: ${value?.schemaVersion}`);
  }
  if (
    typeof value !== 'object'
    || Array.isArray(value)
    || Object.keys(value).some((field) => !['schemaVersion', 'createdAt', 'expectation'].includes(field))
  ) {
    throw new Error('Retained browser requirement contains unsupported fields');
  }
  validateCreatedAt(value.createdAt);
  if (
    typeof value.expectation !== 'object'
    || value.expectation == null
    || Array.isArray(value.expectation)
    || Object.keys(value.expectation)
      .some((field) => !['sessionName', 'profileId', 'targetId', 'url'].includes(field))
  ) {
    throw new Error('Durable retained browser expectation may contain only stable identity fields');
  }
  const expectation = normalizeRetainedBrowserExpectation(value.expectation);
  for (const field of ['profileId', 'targetId', 'url']) {
    if (!expectation[field]) {
      throw new Error(`Durable retained browser requirement requires expectation.${field}`);
    }
  }
  validateHttpUrl(expectation.url);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  if (enforcement.exists && enforcement.requirementSha256 !== sha256) {
    throw new Error('Retained browser requirement does not match enforcement digest');
  }
  return {
    schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
    path,
    exists: true,
    sha256,
    expectation,
    createdAt: value.createdAt,
    enforcement,
  };
}

export function retainedBrowserEnforcementPath(requirementPath) {
  return `${requirementPath}.required`;
}

/** Read the separately committed fail-closed enforcement authority. */
export function readRetainedBrowserEnforcement(requirementPath) {
  const path = retainedBrowserEnforcementPath(requirementPath);
  if (!pathEntryExists(path)) {
    return {
      schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
      path,
      exists: false,
      createdAt: null,
      requirementSha256: null,
    };
  }
  const bytes = readPrivateBoundedFile(path, 'Retained browser enforcement', MAX_REQUIREMENT_BYTES);
  let value;
  try {
    value = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    throw new Error(
      `Retained browser enforcement is invalid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (value?.schemaVersion !== LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA) {
    throw new Error(`Unsupported retained browser enforcement schema: ${value?.schemaVersion}`);
  }
  if (
    typeof value !== 'object'
    || Array.isArray(value)
    || Object.keys(value)
      .some((field) => !['schemaVersion', 'createdAt', 'requirementSha256'].includes(field))
  ) {
    throw new Error('Retained browser enforcement contains unsupported fields');
  }
  validateCreatedAt(value.createdAt);
  validateSha256(value.requirementSha256, 'Retained browser enforcement requirementSha256');
  return {
    schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
    path,
    exists: true,
    createdAt: value.createdAt,
    requirementSha256: value.requirementSha256,
  };
}

/** Merge optional command-line strengthening with the durable identity authority. */
export function resolveRetainedBrowserExpectation({ explicit, requirement }) {
  const explicitExpectation = explicit
    ? normalizeRetainedBrowserExpectation(explicit)
    : null;
  if (!requirement?.exists) return explicitExpectation;
  const required = requirement.expectation;
  if (!explicitExpectation) return required;
  for (const field of ['sessionName', 'profileId', 'targetId', 'url']) {
    if (
      explicitExpectation[field] != null
      && explicitExpectation[field] !== required[field]
    ) {
      throw new Error(
        `Explicit retained browser ${field} conflicts with durable requirement`,
      );
    }
  }
  const merged = { ...required };
  for (const [field, value] of Object.entries(explicitExpectation)) {
    if (value != null) merged[field] = value;
  }
  return normalizeRetainedBrowserExpectation(merged);
}

/** Persist only stable identity from verified live evidence, never PID or CDP data. */
export function writeRetainedBrowserRequirement({
  path,
  evidence,
  now = () => new Date().toISOString(),
  afterEnforcementCommit = () => {},
}) {
  const pinned = pinRetainedBrowserExpectation(evidence);
  const expectation = normalizeRetainedBrowserExpectation({
    sessionName: pinned.sessionName,
    profileId: pinned.profileId,
    targetId: pinned.targetId,
    url: pinned.url,
  });
  for (const field of ['profileId', 'targetId', 'url']) {
    if (!expectation[field]) {
      throw new Error(`Verified retained browser evidence is missing ${field}`);
    }
  }
  const existing = readRetainedBrowserRequirement(path, { allowEnforcedMissing: true });
  if (existing.exists) {
    const resolved = resolveRetainedBrowserExpectation({
      explicit: expectation,
      requirement: existing,
    });
    ensureRetainedBrowserEnforcement(path, existing.createdAt, existing.sha256);
    return {
      ...readRetainedBrowserRequirement(path),
      expectation: resolved,
      written: false,
    };
  }
  const createdAt = existing.enforcement.exists ? existing.enforcement.createdAt : now();
  validateCreatedAt(createdAt);
  validateHttpUrl(expectation.url);
  const value = {
    schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
    createdAt,
    expectation: {
      sessionName: expectation.sessionName,
      profileId: expectation.profileId,
      targetId: expectation.targetId,
      url: expectation.url,
    },
  };
  const requirementSha256 = sha256Json(value);
  ensureRetainedBrowserEnforcement(path, createdAt, requirementSha256);
  afterEnforcementCommit();
  atomicWritePrivateJson(path, value);
  return { ...readRetainedBrowserRequirement(path), written: true };
}

function ensureRetainedBrowserEnforcement(requirementPath, createdAt, requirementSha256) {
  const existing = readRetainedBrowserEnforcement(requirementPath);
  if (existing.exists) {
    validateEnforcementCommit(existing, createdAt, requirementSha256);
    return existing;
  }
  try {
    atomicWritePrivateJson(retainedBrowserEnforcementPath(requirementPath), {
      schemaVersion: LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
      createdAt,
      requirementSha256,
    });
  } catch (error) {
    if (error?.code !== 'EEXIST') throw error;
  }
  const committed = readRetainedBrowserEnforcement(requirementPath);
  validateEnforcementCommit(committed, createdAt, requirementSha256);
  return committed;
}

function validateEnforcementCommit(enforcement, createdAt, requirementSha256) {
  if (
    enforcement.createdAt !== createdAt
    || enforcement.requirementSha256 !== requirementSha256
  ) {
    throw new Error('Retained browser enforcement conflicts with requirement commit');
  }
}

function sha256Json(value) {
  return createHash('sha256').update(serializePrivateJson(value)).digest('hex');
}

function readPrivateBoundedFile(path, label, maxBytes) {
  let descriptor = null;
  try {
    descriptor = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  } catch (error) {
    if (error?.code === 'ELOOP') {
      throw new Error(`${label} must be a regular non-symlink file`);
    }
    throw error;
  }
  try {
    const metadata = fstatSync(descriptor);
    if (!metadata.isFile()) {
      throw new Error(`${label} must be a regular non-symlink file`);
    }
    if (metadata.size > maxBytes) {
      throw new Error(`${label} exceeds ${maxBytes} bytes`);
    }
    if (process.platform !== 'win32' && (metadata.mode & 0o077) !== 0) {
      throw new Error(`${label} permissions must be 0600 or stricter`);
    }
    if (
      process.platform !== 'win32'
      && typeof process.getuid === 'function'
      && metadata.uid !== process.getuid()
    ) {
      throw new Error(`${label} must be owned by the current user`);
    }
    return readFileSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function pathEntryExists(path) {
  try {
    lstatSync(path);
    return true;
  } catch (error) {
    if (error?.code === 'ENOENT') return false;
    throw error;
  }
}

function atomicWritePrivateJson(path, value) {
  const directory = dirname(path);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  chmodSync(directory, 0o700);
  const staged = `${path}.next-${process.pid}`;
  let descriptor = null;
  try {
    rmSync(staged, { force: true });
    descriptor = openSync(staged, 'wx', 0o600);
    writeFileSync(descriptor, serializePrivateJson(value));
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = null;
    linkSync(staged, path);
    rmSync(staged);
    fsyncDirectory(directory);
  } catch (error) {
    if (descriptor !== null) closeSync(descriptor);
    rmSync(staged, { force: true });
    throw error;
  }
}

function serializePrivateJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function fsyncDirectory(path) {
  let descriptor = null;
  try {
    descriptor = openSync(path, 'r');
    fsyncSync(descriptor);
  } catch (error) {
    if (!['EINVAL', 'ENOTSUP', 'EISDIR'].includes(error?.code)) throw error;
  } finally {
    if (descriptor !== null) closeSync(descriptor);
  }
}

function validateCreatedAt(value) {
  if (typeof value !== 'string' || Number.isNaN(Date.parse(value))) {
    throw new Error('Retained browser requirement requires a valid createdAt timestamp');
  }
}

function validateHttpUrl(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error('Durable retained browser requirement URL is invalid');
  }
  if (!['http:', 'https:'].includes(parsed.protocol)) {
    throw new Error('Durable retained browser requirement URL must use HTTP or HTTPS');
  }
}

function validateSha256(value, label) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    throw new Error(`${label} must be a lowercase SHA-256 digest`);
  }
}
