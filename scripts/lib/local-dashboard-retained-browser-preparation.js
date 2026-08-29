import {
  normalizeRetainedExactUrl,
  normalizeRetainedUrlPrefix,
  retainedUrlMatchesPrefix,
} from './local-dashboard-retained-browser-discovery.js';

const SAFE_PROFILE_ID = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const SAFE_BROWSER_BUILDS = new Set(['stock_chrome', 'stealthcdp_chromium']);

/**
 * Normalize the authority for one exact, navigation-only retained lane setup.
 * The resulting operation can launch or navigate a route-bound tab and pin its
 * identity. It exposes no page interaction or prompt-submission primitive.
 */
export function normalizeRetainedBrowserPreparationRequest(input) {
  const exactUrl = normalizeRetainedExactUrl(input.url).href;
  const urlPrefix = normalizeRetainedUrlPrefix(input.urlPrefix).href;
  if (!retainedUrlMatchesPrefix(exactUrl, urlPrefix)) {
    throw preparationError(
      'retained_browser_preparation_url_outside_prefix',
      'The exact retained URL is outside the reviewed origin and path prefix',
    );
  }
  const runtimeProfile = String(input.runtimeProfile || '').trim();
  if (!SAFE_PROFILE_ID.test(runtimeProfile)) {
    throw preparationError(
      'retained_browser_preparation_profile_invalid',
      'A safe runtime profile id is required',
    );
  }
  const browserBuild = String(input.browserBuild || 'stock_chrome').trim();
  if (!SAFE_BROWSER_BUILDS.has(browserBuild)) {
    throw preparationError(
      'retained_browser_preparation_browser_build_invalid',
      'The retained browser build must be stock_chrome or stealthcdp_chromium',
    );
  }
  return {
    url: exactUrl,
    urlPrefix,
    runtimeProfile,
    sessionName: runtimeProfile,
    browserBuild,
    serviceName: boundedLabel(input.serviceName, 'AuraCall'),
    agentName: boundedLabel(input.agentName, 'codex'),
    taskName: boundedLabel(input.taskName, 'prepare-retained-browser-lane'),
    jobTimeoutMs: normalizeTimeout(input.jobTimeoutMs),
  };
}

export function buildRetainedBrowserRemoteViewArgs(request) {
  return [
    '--json',
    '--session',
    request.sessionName,
    'remote-view',
    'open',
    request.url,
    '--runtime-profile',
    request.runtimeProfile,
    '--browser-build',
    request.browserBuild,
    '--view-stream-provider',
    'rdp_gateway',
    '--service-name',
    request.serviceName,
    '--agent-name',
    request.agentName,
    '--task-name',
    request.taskName,
    '--job-timeout-ms',
    String(request.jobTimeoutMs),
  ];
}

export function verifyRetainedBrowserRemoteViewResult(payload, request) {
  const data = payload?.data;
  const target = data?.operatorVisible?.target;
  const profile = data?.routeBoundHandoff?.profile;
  const shared = data?.sharedAcquisition;
  if (payload?.success !== true || data?.status !== 'opened') {
    throw preparationError(
      'retained_browser_preparation_open_failed',
      'The route-bound browser did not report a successful open',
    );
  }
  if (data?.dryRun === true || data?.operatorVisible?.state !== 'ready') {
    throw preparationError(
      'retained_browser_preparation_not_visible',
      'The route-bound browser is not verified operator-visible',
    );
  }
  if (
    data?.intent?.url !== request.url
    || target?.url !== request.url
    || target?.expectedUrl !== request.url
    || target?.urlReadiness !== 'ready'
    || target?.state !== 'ready'
  ) {
    throw preparationError(
      'retained_browser_preparation_url_mismatch',
      'The rendered canonical URL does not exactly match the requested URL',
    );
  }
  if (
    data?.intent?.runtimeProfile !== request.runtimeProfile
    || profile?.id !== request.runtimeProfile
    || profile?.runtimeProfile !== request.runtimeProfile
    || (target?.profileId != null && target.profileId !== request.runtimeProfile)
    || shared?.profileId !== request.runtimeProfile
  ) {
    throw preparationError(
      'retained_browser_preparation_profile_mismatch',
      'The acquired target does not use the requested runtime profile',
    );
  }
  if (
    !target?.targetId
    || !data?.browserId
    || data?.sessionName !== request.sessionName
    || shared?.browserId !== data.browserId
    || shared?.sessionName !== request.sessionName
  ) {
    throw preparationError(
      'retained_browser_preparation_identity_incomplete',
      'The route-bound browser response lacks one stable target identity',
    );
  }
  return {
    browserId: data.browserId,
    sessionName: data.sessionName,
    profileId: request.runtimeProfile,
    targetId: target.targetId,
    url: target.url,
    handoffUrl: data.handoffUrl || data.externalUrl || null,
  };
}

export function buildRetainedBrowserPinArgs(request) {
  return [
    '--write-retained-requirement',
    '--discover-retained-exact-url',
    request.url,
    '--discover-retained-profile',
    request.runtimeProfile,
    '--json',
  ];
}

function boundedLabel(value, fallback) {
  const label = String(value || fallback).trim();
  if (!label || label.length > 128 || /[\r\n\0]/.test(label)) {
    throw preparationError(
      'retained_browser_preparation_label_invalid',
      'Service, agent, and task labels must be bounded single-line values',
    );
  }
  return label;
}

function normalizeTimeout(value) {
  const timeout = Number(value ?? 120000);
  if (!Number.isSafeInteger(timeout) || timeout <= 0 || timeout > 600000) {
    throw preparationError(
      'retained_browser_preparation_timeout_invalid',
      'The remote-view job timeout must be between 1 and 600000 milliseconds',
    );
  }
  return timeout;
}

function preparationError(code, message) {
  const error = new Error(`${code}: ${message}`);
  error.code = code;
  return error;
}
