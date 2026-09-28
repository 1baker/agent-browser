import { isDeepStrictEqual } from 'node:util';

const SCHEMA = 'agent-browser.local-dashboard-session-quiesce.v1';
const BACKEND = 'dashboard-service-backend';
const fail = (message) => { throw new Error(`Dashboard session quiesce: ${message}`); };

function names(values) {
  if (!Array.isArray(values) || values.some((value) => typeof value !== 'string' || !value || value.trim() !== value) || new Set(values).size !== values.length) fail('invalid session set');
  return [...values].sort();
}

function sameNames(actual, expected) {
  if (!isDeepStrictEqual(names(actual), names(expected))) fail('session set drift');
}

function cgroup(value) {
  if (typeof value !== 'string' || !value.startsWith('/') || value === '/' || value.includes('\\') || value.slice(1).split('/').some((part) => !part || part === '.' || part === '..')) fail('missing or invalid cgroup evidence');
  return value;
}

function pid(value) {
  if (!Number.isSafeInteger(value) || value <= 0) fail('invalid daemon or browser PID');
  return value;
}

function indexEvidence(evidence, expectedSessions) {
  if (!Array.isArray(evidence) || evidence.some((item) => !item || typeof item !== 'object')) fail('missing session evidence');
  sameNames(evidence.map((item) => item.sessionName), expectedSessions);
  return new Map(evidence.map((item) => [item.sessionName, item]));
}

function affected(value, dashboardCgroup) {
  return value === dashboardCgroup || value.startsWith(`${dashboardCgroup}/`);
}

/** Refresh the complete idle snapshot without dropping earlier browser ownership proof. */
export function refreshDashboardSessionEvidence({ evidence, state, readCgroup }) {
  if (!Array.isArray(evidence) || evidence.length === 0 || typeof readCgroup !== 'function') fail('missing refresh evidence');
  names(evidence.map((item) => item?.sessionName));
  const rows = (key) => {
    const value = state?.[key];
    if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`missing current ${key} evidence`);
    const result = Object.values(value);
    if (result.some((row) => !row || typeof row !== 'object' || Array.isArray(row))) fail(`invalid current ${key} row`);
    return result;
  };
  const browsers = rows('browsers');
  const countActive = (key, terminal) => rows(key).filter((row) => {
    if (typeof row.state !== 'string' || !row.state) fail(`missing current ${key} state`);
    return !terminal.includes(row.state);
  }).length;
  const activeJobs = countActive('jobs', ['succeeded', 'failed', 'cancelled', 'canceled']);
  const activeViewerLeases = countActive('viewerLeases', ['disconnected', 'expired', 'released', 'failed']);
  if (activeJobs !== 0) fail('current snapshot is not idle');
  const groups = new Map();
  const groupFor = (processPid) => {
    pid(processPid);
    if (!groups.has(processPid)) groups.set(processPid, cgroup(readCgroup(processPid)));
    return groups.get(processPid);
  };
  const associated = (browser, sessionName) => browser.id === `session:${sessionName}` || browser.sessionName === sessionName || browser.session === sessionName;
  const observedBrowserCgroups = [];
  for (const browser of browsers) {
    if (typeof browser.id !== 'string' || !browser.id) fail('missing current browser identity');
    if (browser.pid != null) observedBrowserCgroups.push(groupFor(browser.pid));
    else if (browser.cdpEndpoint && !evidence.some((item) => associated(browser, item.sessionName))) fail('unassociated browser has no process evidence');
  }
  return evidence.map((item) => {
    if (!Array.isArray(item.browserPids) || !Array.isArray(item.browserCgroups) || item.browserPids.length !== item.browserCgroups.length) fail('missing prior browser ownership evidence');
    const currentBrowsers = browsers.filter((browser) => associated(browser, item.sessionName));
    const browserPids = [...new Set([...item.browserPids, ...currentBrowsers.filter((browser) => browser.pid != null).map((browser) => browser.pid)])];
    return {
      ...item,
      daemonCgroup: groupFor(item.daemonPid),
      browserPids,
      browserCgroups: browserPids.map(groupFor),
      observedBrowserCgroups: [...new Set([...(item.observedBrowserCgroups || []), ...observedBrowserCgroups])],
      hasPersistedBrowser: item.hasPersistedBrowser === true || currentBrowsers.length > 0 ? true : item.hasPersistedBrowser,
      activeJobs,
      activeViewerLeases,
    };
  });
}

/** Pure pre-stop authorization. The adapter must supply fresh process and browser evidence. */
export function prepareDashboardSessionQuiesce({ expectedSessions, currentSessions, dashboardCgroup, evidence }) {
  const expected = names(expectedSessions);
  sameNames(currentSessions, expected);
  const dashboard = cgroup(dashboardCgroup);
  if (!dashboard.endsWith('/agent-browser-dashboard.service')) fail('unexpected dashboard ControlGroup');
  const indexed = indexEvidence(evidence, expected);
  const retiredSessions = [];
  const reviewedEvidence = [];
  const daemonPids = new Set();
  for (const sessionName of expected) {
    const item = indexed.get(sessionName);
    const daemonPid = pid(item.daemonPid);
    if (daemonPids.has(daemonPid)) fail('duplicate session daemon PID');
    daemonPids.add(daemonPid);
    const daemonCgroup = cgroup(item.daemonCgroup);
    if (!Array.isArray(item.browserPids) || !Array.isArray(item.browserCgroups) || item.browserPids.length !== item.browserCgroups.length) fail('missing browser ownership evidence');
    item.browserPids.forEach(pid);
    item.browserCgroups.forEach(cgroup);
    const observedBrowserCgroups = item.observedBrowserCgroups ?? [];
    if (!Array.isArray(observedBrowserCgroups)) fail('invalid global browser evidence');
    observedBrowserCgroups.forEach(cgroup);
    if ([...item.browserCgroups, ...observedBrowserCgroups].some((group) => affected(group, dashboard))) fail('browser would be stopped with dashboard');
    if (affected(daemonCgroup, dashboard)) {
      if (sessionName !== BACKEND || daemonCgroup !== dashboard) fail('unrelated session would be stopped with dashboard');
      if (item.browserPids.length !== 0 || item.hasPersistedBrowser !== false || item.activeJobs !== 0 || item.activeViewerLeases !== 0) fail(`backend is not proven idle and browser-free (browsers=${item.browserPids.length}, persisted=${item.hasPersistedBrowser}, jobs=${item.activeJobs}, viewers=${item.activeViewerLeases})`);
      retiredSessions.push(sessionName);
    }
    // Keep only the bounded proof fields; do not journal arbitrary adapter data.
    reviewedEvidence.push({ sessionName, daemonPid, daemonCgroup, browserPids: [...item.browserPids], browserCgroups: [...item.browserCgroups], observedBrowserCgroups: [...observedBrowserCgroups], ...(sessionName === BACKEND ? { hasPersistedBrowser: item.hasPersistedBrowser, activeJobs: item.activeJobs, activeViewerLeases: item.activeViewerLeases } : {}) });
  }
  return { schemaVersion: SCHEMA, expectedSessions: expected, dashboardCgroup: dashboard, retiredSessions, evidence: reviewedEvidence };
}

/** Verify the exact planned post-stop set and each original daemon's observed fate. */
export function verifyDashboardSessionQuiesce(record, { currentSessions, evidence }) {
  if (!record || record.schemaVersion !== SCHEMA) fail('invalid quiesce record');
  const reviewed = prepareDashboardSessionQuiesce({ ...record, currentSessions: record.expectedSessions });
  if (!isDeepStrictEqual(record.retiredSessions, reviewed.retiredSessions)) fail('invalid retirement selection');
  const survivors = reviewed.expectedSessions.filter((name) => !reviewed.retiredSessions.includes(name));
  sameNames(currentSessions, survivors);
  const observed = indexEvidence(evidence, reviewed.expectedSessions);
  for (const original of reviewed.evidence) {
    const item = observed.get(original.sessionName);
    if (item.daemonPid !== original.daemonPid) fail('session daemon was replaced');
    const retired = reviewed.retiredSessions.includes(original.sessionName);
    if (item.daemonAlive !== !retired || item.socketPresent !== !retired) fail(retired ? 'retired backend still alive or socket remains' : 'unrelated daemon or socket disappeared');
  }
  return { verified: true, expectedSessions: survivors, retiredSessions: [...reviewed.retiredSessions] };
}
