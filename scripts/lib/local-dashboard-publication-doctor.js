const fail = (message) => { throw new Error(`Publication doctor: ${message}`); };
const INODE_ISSUES = new Set(['daemon_socket_current_executable_mismatch', 'daemon_socket_deleted_executable']);
const SHA256 = /^[a-f0-9]{64}$/;

function shape(doctor) {
  if (!doctor || typeof doctor.success !== 'boolean' || !Array.isArray(doctor.data?.issues)) fail('invalid report');
  if (doctor.data.workstationPayload?.ready !== true) fail('workstation payload is not ready');
  if (doctor.success !== (doctor.data.issues.length === 0)) fail('inconsistent success flag');
  return doctor.data;
}

export function validateStrictPublicationDoctor(doctor) {
  const data = shape(doctor);
  if (!doctor.success || data.issues.length) fail('strict installed doctor failed');
  return { success: true, rawSuccess: true, transactionScoped: false, degraded: false, repairRequired: false, workstationPayloadReady: true };
}

/** Evaluate a doctor while the caller still owns the journal lock. This does
 * not turn the raw doctor into a strict pass: a later unlocked doctor is required.
 * Evidence callbacks must read executable bytes and process start identity from
 * the live PID, not from session metadata. No state is changed here. */
export function validatePublicationDoctor(doctor, {
  journalPath, journalRecord, ownerPid, installedSha256,
  allowSourceRollbackDegraded = false, verifyListenerEvidence,
}) {
  const data = shape(doctor);
  if (!SHA256.test(installedSha256) || !Number.isInteger(ownerPid) || ownerPid < 1) fail('invalid expected identity');
  const status = data.localDashboardPublication;
  const txn = status?.transaction;
  if (!journalRecord || typeof journalRecord.transactionId !== 'string' || !Number.isInteger(journalRecord.revision) || journalRecord.terminal !== false) fail('invalid journal record');
  if (status?.journalPath !== journalPath || status?.exists !== true ||
      txn?.transactionId !== journalRecord.transactionId || txn?.revision !== journalRecord.revision ||
      txn?.phase !== journalRecord.phase || txn?.terminal !== false || txn?.installBin !== journalRecord.installBin) fail('foreign or stale transaction');
  if (status.lock?.path !== `${journalPath}.lock` || status.lock?.present !== true ||
      status.lock?.ownerPid !== ownerPid || status.lock?.live !== true || status.lock?.stale !== false ||
      status.recommendedAction !== 'wait_for_active_publisher') fail('publisher lock is not owned');
  if (status.installedArtifact?.verified !== true || status.installedArtifact?.sha256 !== installedSha256 ||
      status.installedArtifact?.path !== journalRecord.installBin || data.currentExecutable?.sha256 !== installedSha256) fail('installed artifact mismatch');
  const issues = data.issues;
  if (issues.filter(issue => issue?.code === 'dashboard_publication_active').length !== 1) fail('missing or duplicate own-publication warning');
  const remaining = issues.filter(issue => issue?.code !== 'dashboard_publication_active');
  if (remaining.some(issue => !INODE_ISSUES.has(issue?.code))) fail(`unrelated doctor issue: ${remaining.map(issue => issue?.code).join(', ')}`);
  const degraded = remaining.length > 0;
  const listenerEvidence = [];
  if (degraded) {
    const backup = journalRecord.artifactEvidence?.backup;
    if (!allowSourceRollbackDegraded || journalRecord.handoffOutcomeUncertain !== false ||
        !['prepared', 'quiesce_admitted', 'quiesced', 'handoff_admitted'].includes(journalRecord.failedAtPhase) || backup?.verified !== true || backup.sha256 !== installedSha256 ||
        journalRecord.artifactEvidence?.source?.sha256 !== installedSha256 ||
        !Array.isArray(journalRecord.handoffs) || journalRecord.handoffs.length ||
        (journalRecord.resumedHandoffs ?? []).length || txn.preparedHandoffCount !== 0 || txn.resumedHandoffCount !== 0) fail('source rollback degradation not admitted');
    const inventory = data.daemonListenerInventory;
    if (!Array.isArray(inventory?.listeners) || inventory.defaultSocketListenerCount !== 1 ||
        inventory.defaultSocketDeletedExecutableCount !== 1 || inventory.defaultSocketCurrentExecutableMatchCount !== 0 ||
        typeof verifyListenerEvidence !== 'function') fail('ambiguous listener evidence');
    const defaults = inventory.listeners.filter(row => typeof row.socketPath === 'string' && row.socketPath.endsWith('/default.sock'));
    if (defaults.length !== 1 || defaults[0].deletedExecutable !== true) fail('default listener is not the deleted source');
    const rows = [...inventory.listeners];
    if (!Array.isArray(data.runtimeInventory?.runtimes)) fail('missing runtime inventory');
    for (const runtime of data.runtimeInventory.runtimes) {
      if (runtime.pidRunning === true) rows.push(runtime);
    }
    const seen = new Set();
    for (const row of rows) {
      if (!Number.isInteger(row.pid) || row.pid < 1) fail('invalid listener PID');
      if (seen.has(row.pid)) continue;
      seen.add(row.pid);
      const evidence = verifyListenerEvidence(row);
      if (!evidence || evidence.pid !== row.pid || evidence.sha256 !== installedSha256 ||
          typeof evidence.executablePath !== 'string' || evidence.executablePath.replace(/ \(deleted\)$/, '') !== journalRecord.installBin ||
          typeof evidence.processStartTimeBefore !== 'string' || !evidence.processStartTimeBefore ||
          evidence.processStartTimeBefore !== evidence.processStartTimeAfter) fail('live source executable evidence mismatch');
      listenerEvidence.push(evidence);
    }
  }
  return { success: !degraded, rawSuccess: doctor.success, transactionScoped: true, degraded,
    repairRequired: degraded, ignoredIssues: issues, listenerEvidence, workstationPayloadReady: true };
}
