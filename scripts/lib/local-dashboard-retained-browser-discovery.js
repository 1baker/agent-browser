import {
  isLoopbackDevToolsUrl,
  normalizeRetainedBrowserExpectation,
} from './local-dashboard-retained-browser-guard.js';

/**
 * Discover one exact, ready retained browser target under an operator-reviewed
 * URL prefix. Discovery is read-only and fails closed on unreadable live lanes,
 * missing identity, no match, or more than one match.
 */
export async function discoverRetainedBrowserExpectation({
  urlPrefix,
  exactUrl,
  profileId,
  sessionNames,
  readDaemonPid,
  isProcessLive,
  readBrowser,
  readCdpTargets,
}) {
  if (Boolean(urlPrefix) === Boolean(exactUrl)) {
    throw discoveryError(
      'retained_browser_discovery_selector_invalid',
      'Retained browser discovery requires exactly one URL prefix or exact URL',
    );
  }
  const prefix = urlPrefix ? normalizeRetainedUrlPrefix(urlPrefix) : null;
  const exact = exactUrl ? normalizeRetainedExactUrl(exactUrl) : null;
  const expectedProfileId = String(profileId || '').trim();
  const candidates = [];
  let inspectedSessionCount = 0;

  for (const sessionName of [...new Set(sessionNames)].sort()) {
    const daemonPid = readDaemonPid(sessionName);
    if (!isProcessLive(daemonPid)) continue;
    inspectedSessionCount += 1;
    const readback = await readBrowser(sessionName, daemonPid);
    if (!readback?.success) {
      throw discoveryError(
        'retained_browser_discovery_service_read_failed',
        `Could not inspect live retained session '${sessionName}'`,
      );
    }
    const browser = readback.browser;
    if (!browser) continue;
    if (browser.id !== `session:${sessionName}`) {
      throw discoveryError(
        'retained_browser_discovery_session_mismatch',
        `Live retained session '${sessionName}' reported a different browser identity`,
      );
    }
    if (browser.health !== 'ready') continue;
    if (!browser.profileId || !browser.cdpEndpoint || !isProcessLive(browser.pid)) {
      throw discoveryError(
        'retained_browser_discovery_identity_incomplete',
        `Ready retained session '${sessionName}' has incomplete live browser identity`,
      );
    }
    if (!isLoopbackDevToolsUrl(browser.cdpEndpoint)) {
      throw discoveryError(
        'retained_browser_discovery_cdp_not_loopback',
        `Ready retained session '${sessionName}' does not expose loopback DevTools`,
      );
    }
    let targets;
    try {
      targets = await readCdpTargets(browser.cdpEndpoint);
    } catch (error) {
      throw discoveryError(
        'retained_browser_discovery_cdp_unreadable',
        `Could not inspect targets for ready retained session '${sessionName}': ${error instanceof Error ? error.message : String(error)}`,
      );
    }
    if (!Array.isArray(targets)) {
      throw discoveryError(
        'retained_browser_discovery_cdp_invalid',
        `Ready retained session '${sessionName}' returned an invalid target inventory`,
      );
    }
    for (const target of targets) {
      if (target?.type !== 'page' || !target.id || !target.url) continue;
      if (exact
        ? !retainedUrlMatchesExact(target.url, exact)
        : !retainedUrlMatchesPrefix(target.url, prefix)) continue;
      candidates.push({
        sessionName,
        browserId: browser.id,
        browserPid: browser.pid,
        cdpUrl: browser.cdpEndpoint,
        profileId: browser.profileId,
        targetId: target.id,
        url: target.url,
      });
    }
  }

  if (candidates.length !== 1) {
    throw discoveryError(
      candidates.length === 0
        ? 'retained_browser_discovery_no_match'
        : 'retained_browser_discovery_ambiguous',
      `Retained browser discovery matched ${candidates.length} ready targets for the reviewed URL selector`,
      { matchedCandidateCount: candidates.length, inspectedSessionCount },
    );
  }
  if (expectedProfileId && candidates[0].profileId !== expectedProfileId) {
    throw discoveryError(
      'retained_browser_discovery_profile_mismatch',
      'The uniquely matched retained browser target does not use the required profile',
      { matchedCandidateCount: 1, inspectedSessionCount },
    );
  }

  return {
    expectation: normalizeRetainedBrowserExpectation(candidates[0]),
    urlPrefix: prefix?.href ?? null,
    exactUrl: exact?.href ?? null,
    profileId: expectedProfileId || null,
    inspectedSessionCount,
    matchedCandidateCount: 1,
  };
}

export function normalizeRetainedExactUrl(value) {
  let parsed;
  try {
    parsed = new URL(String(value || ''));
  } catch {
    throw discoveryError(
      'retained_browser_discovery_exact_url_invalid',
      'Retained browser discovery exact URL must be an absolute HTTP or HTTPS URL',
    );
  }
  if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) {
    throw discoveryError(
      'retained_browser_discovery_exact_url_invalid',
      'Retained browser discovery exact URL must be HTTP or HTTPS without credentials',
    );
  }
  if (parsed.search) {
    throw discoveryError(
      'retained_browser_discovery_exact_url_invalid',
      'Retained browser discovery exact URL cannot contain a query string',
    );
  }
  return parsed;
}

export function retainedUrlMatchesExact(value, exactValue) {
  let candidate;
  try {
    candidate = new URL(value);
  } catch {
    return false;
  }
  const exact = exactValue instanceof URL
    ? exactValue
    : normalizeRetainedExactUrl(exactValue);
  return candidate.href === exact.href;
}

export function normalizeRetainedUrlPrefix(value) {
  let parsed;
  try {
    parsed = new URL(String(value || ''));
  } catch {
    throw discoveryError(
      'retained_browser_discovery_prefix_invalid',
      'Retained browser discovery URL prefix must be an absolute HTTP or HTTPS URL',
    );
  }
  if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) {
    throw discoveryError(
      'retained_browser_discovery_prefix_invalid',
      'Retained browser discovery URL prefix must be an absolute HTTP or HTTPS URL without credentials',
    );
  }
  if (parsed.search || parsed.hash) {
    throw discoveryError(
      'retained_browser_discovery_prefix_invalid',
      'Retained browser discovery URL prefix cannot contain a query or fragment',
    );
  }
  return parsed;
}

export function retainedUrlMatchesPrefix(value, prefixValue) {
  let candidate;
  try {
    candidate = new URL(value);
  } catch {
    return false;
  }
  const prefix = prefixValue instanceof URL
    ? prefixValue
    : normalizeRetainedUrlPrefix(prefixValue);
  if (candidate.origin !== prefix.origin) return false;
  const prefixPath = prefix.pathname;
  return candidate.pathname === prefixPath
    || (prefixPath.endsWith('/')
      ? candidate.pathname.startsWith(prefixPath)
      : candidate.pathname.startsWith(`${prefixPath}/`));
}

function discoveryError(code, message, evidence = {}) {
  const error = new Error(`${code}: ${message}`);
  error.code = code;
  error.discoveryEvidence = { code, ...evidence };
  return error;
}
