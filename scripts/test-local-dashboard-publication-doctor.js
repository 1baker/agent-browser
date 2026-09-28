import assert from 'node:assert/strict';
import { validatePublicationDoctor, validateStrictPublicationDoctor } from './lib/local-dashboard-publication-doctor.js';

const sha = 'a'.repeat(64);
function fixture() {
  const journalRecord = { transactionId: 'txn', revision: 4, phase: 'recovery_rollback_restart_admitted', terminal: false, installBin: '/home/test/.local/bin/agent-browser', handoffOutcomeUncertain: false, failedAtPhase: 'handoff_admitted', handoffs: [], resumedHandoffs: [], artifactEvidence: { source: { sha256: sha }, backup: { verified: true, sha256: sha } } };
  const options = { journalPath: '/home/test/journal.json', journalRecord, ownerPid: 123, installedSha256: sha };
  const doctor = { success: false, data: { issues: [{ code: 'dashboard_publication_active' }], workstationPayload: { ready: true }, currentExecutable: { sha256: sha }, localDashboardPublication: { journalPath: options.journalPath, exists: true, transaction: { ...journalRecord, preparedHandoffCount: 0, resumedHandoffCount: 0 }, lock: { path: `${options.journalPath}.lock`, present: true, ownerPid: 123, live: true, stale: false }, recommendedAction: 'wait_for_active_publisher', installedArtifact: { path: journalRecord.installBin, verified: true, sha256: sha } } } };
  return { doctor, options };
}
let count = 0;
function rejects(change) { const f = fixture(); change(f); assert.throws(() => validatePublicationDoctor(f.doctor, f.options)); count++; }
const clean = fixture();
assert.equal(validatePublicationDoctor(clean.doctor, clean.options).rawSuccess, false);
assert.equal(validatePublicationDoctor(clean.doctor, clean.options).degraded, false);
for (const field of ['transactionId', 'revision', 'phase', 'installBin']) rejects(({ doctor }) => { doctor.data.localDashboardPublication.transaction[field] = 'foreign'; });
rejects(({ doctor }) => { doctor.data.localDashboardPublication.journalPath = '/foreign'; });
rejects(({ doctor }) => { doctor.data.localDashboardPublication.lock.ownerPid = 999; });
rejects(({ doctor }) => { doctor.data.localDashboardPublication.lock.live = false; });
rejects(({ doctor }) => { doctor.data.workstationPayload.ready = false; });
rejects(({ doctor }) => { doctor.data.issues.push({ code: 'active_runtime_stale_stream_backend' }); });
rejects(({ doctor }) => { doctor.data.currentExecutable.sha256 = 'b'.repeat(64); });
rejects(({ doctor }) => { doctor.data.issues.push({ code: 'dashboard_publication_active' }); });

function degradedFixture() {
  const f = fixture();
  f.options.allowSourceRollbackDegraded = true;
  f.doctor.data.issues.push({ code: 'daemon_socket_deleted_executable' }, { code: 'daemon_socket_current_executable_mismatch' });
  f.doctor.data.daemonListenerInventory = { defaultSocketListenerCount: 1, defaultSocketDeletedExecutableCount: 1, defaultSocketCurrentExecutableMatchCount: 0, listeners: [{ pid: 44, socketPath: '/run/user/1000/agent-browser/default.sock', deletedExecutable: true }] };
  f.doctor.data.runtimeInventory = { runtimes: [{ pid: 45, pidRunning: true }] };
  f.options.verifyListenerEvidence = ({ pid }) => ({ pid, sha256: sha, executablePath: `${f.options.journalRecord.installBin} (deleted)`, processStartTimeBefore: '1234', processStartTimeAfter: '1234' });
  return f;
}
const degraded = degradedFixture();
const receipt = validatePublicationDoctor(degraded.doctor, degraded.options);
assert.equal(receipt.success, false);
assert.equal(receipt.degraded, true);
assert.equal(receipt.listenerEvidence.length, 2);
for (const phase of ['prepared', 'quiesce_admitted', 'quiesced', 'handoff_admitted']) {
  const f = degradedFixture();
  f.options.journalRecord.failedAtPhase = phase;
  assert.equal(validatePublicationDoctor(f.doctor, f.options).degraded, true, phase);
  count++;
  for (const uncertainty of [true, undefined]) {
    f.options.journalRecord.handoffOutcomeUncertain = uncertainty;
    assert.throws(() => validatePublicationDoctor(f.doctor, f.options), /not admitted/, `${phase}: uncertainty ${uncertainty}`);
    count++;
  }
}
for (const phase of ['handoff_prepared', 'replacement_admitted', 'replacement_installed', 'ready', undefined]) {
  const f = degradedFixture();
  f.options.journalRecord.failedAtPhase = phase;
  assert.throws(() => validatePublicationDoctor(f.doctor, f.options), /not admitted/);
  count++;
}
for (const change of [
  f => { f.options.allowSourceRollbackDegraded = false; },
  f => { delete f.options.journalRecord.handoffOutcomeUncertain; },
  f => { f.options.journalRecord.handoffs.push({}); },
  f => { f.doctor.data.daemonListenerInventory.defaultSocketListenerCount = 2; },
  f => { f.options.verifyListenerEvidence = () => ({ pid: 44, sha256: sha, executablePath: f.options.journalRecord.installBin, processStartTimeBefore: '1', processStartTimeAfter: '2' }); },
  f => { f.options.verifyListenerEvidence = () => null; },
  f => { f.options.journalRecord.artifactEvidence.backup.sha256 = 'b'.repeat(64); },
]) {
  const f = degradedFixture(); change(f); assert.throws(() => validatePublicationDoctor(f.doctor, f.options)); count++;
}
assert.throws(() => validateStrictPublicationDoctor(clean.doctor));
assert.equal(validateStrictPublicationDoctor({ success: true, data: { issues: [], workstationPayload: { ready: true } } }).success, true);
console.log(`Publication doctor: ${count + 5} isolated checks passed`);
