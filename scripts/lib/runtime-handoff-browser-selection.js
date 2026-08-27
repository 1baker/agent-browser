import { existsSync, readFileSync, unlinkSync } from 'node:fs';

export function selectRuntimeHandoffBrowser({
  browsers,
  sessionName,
  expectedBrowser = null,
}) {
  const candidates = Array.isArray(browsers) ? browsers : [];
  const sessionBrowsers = candidates.filter(
    (browser) => browser?.id === `session:${sessionName}`,
  );
  if (sessionBrowsers.length > 1) {
    return selectionFailure(
      `Multiple service browsers claim daemon session '${sessionName}'`,
    );
  }
  if (sessionBrowsers.length === 1) {
    const browser = sessionBrowsers[0];
    if (expectedBrowser && !matchesExpectedBrowser(browser, expectedBrowser)) {
      return selectionFailure(
        `The service browser for '${sessionName}' does not match the prepared handoff identity`,
      );
    }
    return { browser, matchKind: 'session', error: null };
  }
  if (!expectedBrowser) {
    return { browser: null, matchKind: null, error: null };
  }

  const aliases = candidates.filter((browser) =>
    matchesExpectedBrowser(browser, expectedBrowser));
  if (aliases.length > 1) {
    return selectionFailure(
      `Multiple service browsers match the prepared handoff identity for '${sessionName}'`,
    );
  }
  return {
    browser: aliases[0] ?? null,
    matchKind: aliases.length === 1 ? 'identity_alias' : null,
    error: null,
  };
}

export function isRuntimeHandoffBrowserActive({
  browser,
  expectedBrowser,
  isProcessLive,
}) {
  if (!browser || ['closed', 'not_started'].includes(browser.health)) return false;
  const effectivePid = browser.pid ?? expectedBrowser?.browserPid ?? null;
  if (Number.isInteger(effectivePid) && effectivePid > 0) {
    return isProcessLive(effectivePid);
  }
  return typeof browser.cdpEndpoint === 'string' && browser.cdpEndpoint.length > 0;
}

export function removeVerifiedRuntimeHandoffRecord(
  expectedBrowser,
  dependencies = {},
) {
  const pathExists = dependencies.pathExists || existsSync;
  const readFile = dependencies.readFile || readFileSync;
  const removeFile = dependencies.removeFile || unlinkSync;
  const handoffPath = expectedBrowser?.handoffPath;
  if (!handoffPath) return false;
  if (!pathExists(handoffPath)) return true;

  let descriptor;
  try {
    descriptor = JSON.parse(readFile(handoffPath, 'utf8'));
  } catch (error) {
    throw new Error(
      `Runtime handoff retry record for '${expectedBrowser.sessionName}' is invalid: ` +
      `${error instanceof Error ? error.message : String(error)}`,
    );
  }
  const identity = {
    schemaVersion: descriptor.schemaVersion ?? descriptor.schema_version,
    sessionName: descriptor.sessionName ?? descriptor.session_name,
    browserPid: descriptor.browserPid ?? descriptor.browser_pid ?? null,
    cdpUrl: descriptor.cdpUrl ?? descriptor.cdp_url ?? null,
    runtimeProfile: descriptor.runtimeProfile ?? descriptor.runtime_profile ?? null,
  };
  if (
    identity.schemaVersion !== 1
    || identity.sessionName !== expectedBrowser.sessionName
    || identity.browserPid !== (expectedBrowser.browserPid ?? null)
    || identity.cdpUrl !== (expectedBrowser.cdpUrl ?? null)
    || (
      expectedBrowser.runtimeProfile != null
      && identity.runtimeProfile !== expectedBrowser.runtimeProfile
    )
  ) {
    throw new Error(
      `Runtime handoff retry record for '${expectedBrowser.sessionName}' changed identity`,
    );
  }
  removeFile(handoffPath);
  return true;
}

function matchesExpectedBrowser(browser, expectedBrowser) {
  const expectedPid = expectedBrowser.browserPid ?? null;
  const expectedCdp = expectedBrowser.cdpUrl ?? null;
  const pidMatches = expectedPid == null
    || browser?.pid === expectedPid
    || (browser?.pid == null && typeof expectedCdp === 'string' && expectedCdp.length > 0);
  return pidMatches
    && (!expectedCdp || browser?.cdpEndpoint === expectedCdp);
}

function selectionFailure(error) {
  return { browser: null, matchKind: null, error };
}
