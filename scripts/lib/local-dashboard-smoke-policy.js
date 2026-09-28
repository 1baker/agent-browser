const PRE_RENDER_BROWSER_PHASES = new Set([
  'open dashboard url',
]);

const BROWSER_LAUNCH_UNAVAILABLE_PATTERNS = [
  /Chrome exited early/i,
  /without exposing DevTools/i,
  /Chrome process failed/i,
  /browser executable.+(?:missing|not found)/i,
  /spawn .+ ENOENT/i,
];

/**
 * Classify a browser smoke failure without weakening rendered-page validation.
 * Only a known launch-unavailable error before renderer acquisition is
 * advisory by default. Every HTTP, manifest, marker, authentication, DOM, and
 * workspace failure remains fatal.
 */
export function classifyLocalDashboardBrowserSmokeFailure({ phase, error, required = false }) {
  const normalizedPhase = typeof phase === 'string' ? phase.trim() : '';
  const normalizedError = typeof error === 'string' ? error.trim() : String(error ?? '');
  const launchUnavailable = PRE_RENDER_BROWSER_PHASES.has(normalizedPhase) &&
    BROWSER_LAUNCH_UNAVAILABLE_PATTERNS.some((pattern) => pattern.test(normalizedError));

  if (launchUnavailable) {
    return {
      status: 'unavailable',
      classification: 'browser_launch_unavailable',
      advisory: !required,
      fatal: required,
      phase: normalizedPhase,
      error: normalizedError,
    };
  }

  return {
    status: 'failed',
    classification: 'browser_validation_failed',
    advisory: false,
    fatal: true,
    phase: normalizedPhase || null,
    error: normalizedError,
  };
}

export function evaluateLocalDashboardBrowserSmokeResult({
  processStatus,
  parsed,
  stderr = '',
  stdout = '',
  required = false,
}) {
  if (processStatus === 0 && parsed?.success === true) {
    return {
      requested: true,
      required,
      status: 'passed',
      classification: 'rendered_page_verified',
      advisory: false,
      fatal: false,
      phase: null,
      error: null,
      evidence: parsed.browser ?? null,
    };
  }

  return {
    requested: true,
    required,
    ...classifyLocalDashboardBrowserSmokeFailure({
      phase: parsed?.phase,
      error: parsed?.error || stderr || stdout,
      required,
    }),
  };
}

export function classifyStandaloneDashboardProcess({
  pid,
  running,
  processUid,
  currentUid,
  command,
  dashboardMode,
}) {
  if (!Number.isInteger(pid) || pid <= 0 || !running) {
    return { state: 'stale', safeToRetire: false, safeToRemovePidFile: true };
  }
  const sameUser = Number.isInteger(processUid) && Number.isInteger(currentUid) && processUid === currentUid;
  const agentBrowserCommand = typeof command === 'string' &&
    /(?:^|[/\\])agent-browser(?:-(?:linux|darwin|windows)-(?:x64|arm64))?(?:\.exe)?$/i.test(command);
  if (sameUser && agentBrowserCommand && dashboardMode === '1') {
    return { state: 'owned_dashboard', safeToRetire: true, safeToRemovePidFile: false };
  }
  return { state: 'identity_mismatch', safeToRetire: false, safeToRemovePidFile: false };
}
