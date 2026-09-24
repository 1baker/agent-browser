/**
 * Admit recovery of a source-binary publication stopped partway through its
 * handoff loop only when every unprepared lane is browserless and either gone
 * or still running the exact installed source executable inode.
 */
export function verifyPartialSourceHandoffInventory({
  expectedSessions,
  preparedSessions,
  readBrowser,
  readDaemonPid,
  isProcessLive,
  hasHandoffRecord,
  sameSourceExecutable,
}) {
  if (!Array.isArray(expectedSessions) || expectedSessions.length === 0
    || new Set(expectedSessions).size !== expectedSessions.length
    || !Array.isArray(preparedSessions)
    || new Set(preparedSessions).size !== preparedSessions.length) return false;
  const expected = new Set(expectedSessions);
  const prepared = new Set(preparedSessions);
  if ([...prepared].some((session) => !expected.has(session))) return false;
  for (const sessionName of expected) {
    if (prepared.has(sessionName)) continue;
    if (hasHandoffRecord(sessionName)) return false;
    const browser = readBrowser(sessionName);
    if (browser && (
      (Number.isInteger(browser.pid) && isProcessLive(browser.pid))
      || (typeof browser.cdpEndpoint === 'string' && browser.cdpEndpoint.length > 0)
      || (Array.isArray(browser.activeSessionIds) && browser.activeSessionIds.length > 0)
    )) return false;
    const daemonPid = readDaemonPid(sessionName);
    if (isProcessLive(daemonPid) && !sameSourceExecutable(daemonPid)) return false;
  }
  return true;
}
