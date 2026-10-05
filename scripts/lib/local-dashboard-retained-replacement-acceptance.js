export function validateRetainedReplacementAcceptance({
  journal, installedSha256, backupSha256, currentSessions, daemonEvidence,
  retainedEvidence, processStartTicks, expectedStartTicks, doctorIssues, doctorStatus,
  doctorLock, reason,
}) {
  const fail = (message) => { throw new Error(`Retained replacement acceptance refused: ${message}`); };
  if (!['publication_failed_replacement_retained', 'accept_admitted'].includes(journal?.phase)
      || journal.terminal !== false) fail('wrong journal phase');
  if (typeof reason !== 'string' || !reason.trim() || reason.length > 240) fail('bounded operator reason required');
  const replacement = journal.artifactEvidence?.replacement?.actualSha256;
  const backup = journal.artifactEvidence?.backup?.sha256;
  if (!journal.artifactEvidence?.replacement?.verified || !journal.artifactEvidence?.backup?.verified
      || !/^[a-f0-9]{64}$/.test(replacement ?? '') || !/^[a-f0-9]{64}$/.test(backup ?? '')) fail('journal artifact evidence incomplete');
  if (journal.artifactEvidence.replacement.path !== journal.installBin
      || journal.artifactEvidence.backup.path !== journal.backupPath) fail('journal artifact path mismatch');
  if (replacement === backup || installedSha256 !== replacement) fail('installed replacement digest mismatch');
  if (backupSha256 !== backup) fail('journaled backup digest mismatch');
  if (journal.failedAtPhase !== 'readiness_admitted'
      || !/^Local dashboard HTTP readiness smoke failed: Dashboard HTML .* did not look like the Agent Browser dashboard\.$/.test(journal.failure ?? '')) fail('failure is not the dashboard HTML smoke');
  const expectedSessions = journal.candidateSessions;
  if (!Array.isArray(expectedSessions) || expectedSessions.length === 0
      || new Set(expectedSessions).size !== expectedSessions.length
      || JSON.stringify([...expectedSessions].sort()) !== JSON.stringify([...currentSessions].sort())) fail('runtime session census mismatch');
  const prepared = journal.handoffs;
  const resumed = journal.resumedHandoffs;
  if (!Array.isArray(prepared) || prepared.length === 0 || !Array.isArray(resumed)
      || prepared.length !== resumed.length
      || new Set(prepared.map((entry) => entry.sessionName)).size !== prepared.length
      || new Set(resumed.map((entry) => entry.sessionName)).size !== resumed.length
      || prepared.some((entry) => !resumed.some((item) => item.sessionName === entry.sessionName
        && item.daemonPid > 0 && item.browserPid === entry.browserPid && item.cdpUrl === entry.cdpUrl))) fail('retained handoff not fully resumed');
  if (!Array.isArray(daemonEvidence) || daemonEvidence.length !== expectedSessions.length
      || new Set(daemonEvidence.map((entry) => entry.sessionName)).size !== daemonEvidence.length
      || expectedSessions.some((sessionName) => !daemonEvidence.some((entry) => entry.sessionName === sessionName
        && Number.isInteger(entry.pid) && entry.pid > 0 && entry.exeSha256 === replacement))) fail('runtime daemon digest mismatch');
  const pin = journal.retainedBrowserExpectation?.pinned;
  const observed = retainedEvidence?.observed;
  if (retainedEvidence?.verified !== true || !pin || !observed
      || ['sessionName', 'browserPid', 'cdpUrl', 'profileId', 'targetId', 'url'].some((key) => observed[key] !== pin[key])) fail('retained browser pin mismatch');
  if (!Number.isSafeInteger(expectedStartTicks) || expectedStartTicks <= 0
      || processStartTicks !== expectedStartTicks) fail('retained browser process start mismatch');
  const expectedDoctor = JSON.stringify(doctorIssues) === JSON.stringify(['dashboard_publication_active'])
    && doctorStatus === 1 && doctorLock?.live === true && doctorLock.ownerPid === process.pid;
  if (!expectedDoctor) {
    const codes = Array.isArray(doctorIssues) ? [...new Set(doctorIssues)].slice(0, 10) : [];
    fail(`unexpected installation doctor issue: codes=${JSON.stringify(codes)}, status=${doctorStatus}, binary=${journal.installBin}`);
  }
  if (journal.phase === 'accept_admitted') {
    const admitted = journal.retainedReplacementAcceptance;
    if (!admitted || admitted.replacementSha256 !== replacement || admitted.backupSha256 !== backup
        || admitted.browserPid !== observed.browserPid || admitted.browserStartTicks !== processStartTicks
        || admitted.browserCdpUrl !== observed.cdpUrl || admitted.reason !== reason.trim()
        || admitted.smokeFailure !== journal.failure) fail('admitted acceptance evidence mismatch');
  }
  return {
    reason: reason.trim(), replacementSha256: replacement, backupSha256: backup,
    backupPath: journal.backupPath, expectedSessions: [...expectedSessions],
    browserPid: observed.browserPid, browserStartTicks: processStartTicks,
    browserCdpUrl: observed.cdpUrl, smokeWaived: true,
    doctorDuringLock: { issueCodes: doctorIssues, status: doctorStatus, ownerPid: doctorLock.ownerPid },
    retainedReadback: retainedEvidence,
    smokeFailure: journal.failure,
  };
}
