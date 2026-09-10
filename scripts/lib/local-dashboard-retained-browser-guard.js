const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

export function normalizeRetainedBrowserExpectation(value) {
  if (value == null) return null;
  if (typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Retained browser expectation must be an object');
  }
  const expectation = {
    sessionName: optionalString(value.sessionName),
    browserId: optionalString(value.browserId),
    browserPid: optionalPositiveInteger(value.browserPid, 'browserPid'),
    cdpUrl: optionalString(value.cdpUrl),
    profileId: optionalString(value.profileId),
    targetId: optionalString(value.targetId),
    url: optionalString(value.url),
  };
  if (!expectation.sessionName) {
    throw new Error('Retained browser expectation requires sessionName');
  }
  if (!SESSION_ID_PATTERN.test(expectation.sessionName)) {
    throw new Error(`Invalid retained browser session name: ${expectation.sessionName}`);
  }
  if (expectation.browserId && expectation.browserId !== `session:${expectation.sessionName}`) {
    throw new Error(
      `Retained browser ID must match session:${expectation.sessionName}: ${expectation.browserId}`,
    );
  }
  return expectation;
}

export function evaluateRetainedBrowserExpectation({
  browser,
  cdpTargets,
  expectation: rawExpectation,
  stage,
}) {
  const expectation = normalizeRetainedBrowserExpectation(rawExpectation);
  const failure = (reason, message) => ({
    required: true,
    verified: false,
    stage,
    reason,
    message,
    expected: expectation,
    observed: publicBrowserEvidence(browser, cdpTargets, expectation),
  });
  if (!browser) {
    return failure(
      'retained_browser_missing',
      `Required retained browser session '${expectation.sessionName}' is missing`,
    );
  }
  const expectedBrowserId = expectation.browserId || `session:${expectation.sessionName}`;
  if (browser.id !== expectedBrowserId) {
    return failure(
      'retained_browser_id_changed',
      `Required retained browser ID changed: ${expectedBrowserId} -> ${browser.id || 'missing'}`,
    );
  }
  if (browser.health !== 'ready') {
    return failure(
      'retained_browser_not_live',
      `Required retained browser is not ready: ${browser.health || 'missing'}`,
    );
  }
  if (expectation.browserPid != null && browser.pid !== expectation.browserPid) {
    return failure(
      'retained_browser_pid_changed',
      `Required retained browser PID changed: ${expectation.browserPid} -> ${browser.pid ?? 'missing'}`,
    );
  }
  if (expectation.cdpUrl && browser.cdpEndpoint !== expectation.cdpUrl) {
    return failure(
      'retained_browser_cdp_changed',
      `Required retained browser CDP endpoint changed: ${expectation.cdpUrl} -> ${browser.cdpEndpoint || 'missing'}`,
    );
  }
  if (expectation.profileId && browser.profileId !== expectation.profileId) {
    return failure(
      'retained_browser_profile_changed',
      `Required retained browser profile changed: ${expectation.profileId} -> ${browser.profileId || 'missing'}`,
    );
  }
  if (!browser.cdpEndpoint) {
    return failure(
      'retained_browser_cdp_missing',
      'Required retained browser has no CDP endpoint',
    );
  }
  if (!isLoopbackDevToolsUrl(browser.cdpEndpoint)) {
    return failure(
      'retained_browser_cdp_not_loopback',
      'Required retained browser DevTools endpoint is not loopback',
    );
  }
  if (!Array.isArray(cdpTargets)) {
    return failure(
      'retained_browser_cdp_unreachable',
      'Required retained browser CDP target inventory is unavailable',
    );
  }
  let target = null;
  if (expectation.targetId) {
    const matches = cdpTargets.filter((candidate) => candidate?.id === expectation.targetId);
    if (matches.length !== 1) {
      return failure(
        matches.length === 0 ? 'retained_target_missing' : 'retained_target_ambiguous',
        `Required retained target '${expectation.targetId}' matched ${matches.length} CDP targets`,
      );
    }
    [target] = matches;
  }
  if (expectation.url && target?.url !== expectation.url) {
    return failure(
      'retained_target_url_changed',
      `Required retained target URL changed: ${expectation.url} -> ${target?.url || 'missing'}`,
    );
  }
  return {
    required: true,
    verified: true,
    stage,
    reason: 'retained_browser_exact_match',
    expected: expectation,
    observed: publicBrowserEvidence(browser, cdpTargets, expectation),
  };
}

export function pinRetainedBrowserExpectation(evidence) {
  if (evidence?.verified !== true || !evidence.observed) {
    throw new Error('Cannot pin an unverified retained browser expectation');
  }
  return normalizeRetainedBrowserExpectation({
    sessionName: evidence.observed.sessionName,
    browserId: evidence.observed.browserId,
    browserPid: evidence.observed.browserPid,
    cdpUrl: evidence.observed.cdpUrl,
    profileId: evidence.observed.profileId,
    targetId: evidence.observed.targetId,
    url: evidence.observed.url,
  });
}

export function isLoopbackDevToolsUrl(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    return false;
  }
  if (!['http:', 'https:', 'ws:', 'wss:'].includes(parsed.protocol)) return false;
  const hostname = parsed.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  if (hostname === 'localhost' || hostname === '::1') return true;
  const octets = hostname.split('.');
  return octets.length === 4
    && octets.every((octet) => /^\d{1,3}$/.test(octet) && Number(octet) <= 255)
    && Number(octets[0]) === 127;
}

function publicBrowserEvidence(browser, cdpTargets, expectation) {
  const target = expectation?.targetId && Array.isArray(cdpTargets)
    ? cdpTargets.find((candidate) => candidate?.id === expectation.targetId)
    : null;
  return {
    sessionName: expectation?.sessionName ?? null,
    browserId: browser?.id ?? null,
    browserPid: Number.isInteger(browser?.pid) ? browser.pid : null,
    cdpUrl: browser?.cdpEndpoint ?? null,
    profileId: browser?.profileId ?? null,
    health: browser?.health ?? null,
    targetId: target?.id ?? expectation?.targetId ?? null,
    url: target?.url ?? null,
    title: target?.title ?? null,
    cdpTargetCount: Array.isArray(cdpTargets) ? cdpTargets.length : null,
  };
}

function optionalString(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function optionalPositiveInteger(value, field) {
  if (value == null) return null;
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`Retained browser ${field} must be a positive integer`);
  }
  return value;
}
