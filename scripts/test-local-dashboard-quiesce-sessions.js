import assert from 'node:assert/strict';
import { prepareDashboardSessionQuiesce, refreshDashboardSessionEvidence, verifyDashboardSessionQuiesce } from './lib/local-dashboard-quiesce-sessions.js';

const dashboardCgroup = '/user.slice/user-1000.slice/user@1000.service/app.slice/agent-browser-dashboard.service';
const expectedSessions = ['dashboard-service-backend', 'retained-research'];
const make = () => ({ expectedSessions, currentSessions: [...expectedSessions], dashboardCgroup, evidence: [
  { sessionName: expectedSessions[0], daemonPid: 123, daemonCgroup: dashboardCgroup, browserPids: [], browserCgroups: [], hasPersistedBrowser: false, activeJobs: 0, activeViewerLeases: 0 },
  { sessionName: expectedSessions[1], daemonPid: 456, daemonCgroup: '/user.slice/retained.service', browserPids: [789], browserCgroups: ['/user.slice/browser.service'] },
] });
const observed = () => ({ currentSessions: ['retained-research'], evidence: [
  { sessionName: expectedSessions[0], daemonPid: 123, daemonAlive: false, socketPresent: false },
  { sessionName: expectedSessions[1], daemonPid: 456, daemonAlive: true, socketPresent: true },
] });
let cases = 0;
function test(fn) { fn(); cases++; }
test(() => {
  const input = make();
  const before = structuredClone(input);
  const record = prepareDashboardSessionQuiesce(input);
  assert.deepEqual(input, before);
  assert.deepEqual(record.retiredSessions, [expectedSessions[0]]);
  assert.deepEqual(verifyDashboardSessionQuiesce(JSON.parse(JSON.stringify(record)), observed()), { verified: true, expectedSessions: ['retained-research'], retiredSessions: [expectedSessions[0]] });
});
for (const mutate of [
  (x) => x.currentSessions.push('new-session'),
  (x) => x.currentSessions.pop(),
  (x) => x.evidence.pop(),
  (x) => { x.evidence[1].daemonCgroup = dashboardCgroup; },
  (x) => { x.evidence[1].daemonCgroup = `${dashboardCgroup}/child`; },
  (x) => { x.evidence[1].browserCgroups = [dashboardCgroup]; },
  (x) => { x.evidence[1].browserCgroups = [`${dashboardCgroup}/child`]; },
  (x) => { x.evidence[0].daemonCgroup = `${dashboardCgroup}/child`; },
  (x) => { x.evidence[0].hasPersistedBrowser = true; },
  (x) => { delete x.evidence[0].hasPersistedBrowser; },
  (x) => { x.evidence[0].activeJobs = 1; },
  (x) => { delete x.evidence[0].activeJobs; },
  (x) => { x.evidence[0].activeViewerLeases = 1; },
  (x) => { x.evidence[0].browserPids = [890]; x.evidence[0].browserCgroups = ['/other/browser']; },
  (x) => { delete x.evidence[1].browserPids; },
  (x) => { x.evidence[1].browserCgroups = []; },
  (x) => { x.dashboardCgroup = '/'; },
  (x) => { x.evidence[1].daemonCgroup = '/a/../b'; },
  (x) => { x.evidence[1].daemonPid = 123; },
  (x) => { x.evidence[1].daemonPid = 0; },
]) test(() => { const input = make(); mutate(input); assert.throws(() => prepareDashboardSessionQuiesce(input)); });
for (const mutate of [
  (x) => x.currentSessions.push('new-session'),
  (x) => x.currentSessions.pop(),
  (x) => x.currentSessions.push(expectedSessions[0]),
  (x) => { x.evidence[0].daemonAlive = true; },
  (x) => { x.evidence[0].socketPresent = true; },
  (x) => { x.evidence[0].daemonPid = 124; },
  (x) => { x.evidence[1].daemonPid = 457; },
  (x) => { x.evidence[1].daemonAlive = false; },
  (x) => { x.evidence[1].socketPresent = false; },
  (x) => x.evidence.pop(),
  (x) => { delete x.evidence[0].socketPresent; },
]) test(() => { const input = observed(); mutate(input); assert.throws(() => verifyDashboardSessionQuiesce(prepareDashboardSessionQuiesce(make()), input)); });
test(() => {
  const input = make();
  input.evidence[0].daemonCgroup = '/other/backend.service';
  const record = prepareDashboardSessionQuiesce(input);
  assert.deepEqual(record.retiredSessions, []);
  const post = observed();
  post.currentSessions = [...expectedSessions];
  post.evidence[0].daemonAlive = true;
  post.evidence[0].socketPresent = true;
  assert.equal(verifyDashboardSessionQuiesce(record, post).verified, true);
});
test(() => { const record = prepareDashboardSessionQuiesce(make()); record.retiredSessions.push('retained-research'); assert.throws(() => verifyDashboardSessionQuiesce(record, observed()), /retirement selection/); });
test(() => { const input = make(); input.evidence[1].daemonCgroup = `${dashboardCgroup}-unrelated`; assert.equal(prepareDashboardSessionQuiesce(input).retiredSessions.length, 1); });
const idleState = () => ({ browsers: {}, jobs: { finished: { state: 'succeeded' } }, viewerLeases: {} });
const readCgroup = (processPid) => processPid === 123 ? dashboardCgroup : '/other/retained.service';
function refreshed(state, evidence = make().evidence, reader = readCgroup) {
  return refreshDashboardSessionEvidence({ evidence, state, readCgroup: reader });
}
test(() => {
  const evidence = make().evidence;
  const original = structuredClone(evidence);
  const result = refreshed(idleState(), evidence);
  assert.deepEqual(evidence, original);
  assert.deepEqual(result[1].browserPids, [789]);
  assert.equal(result[1].daemonCgroup, '/other/retained.service');
  assert.equal(prepareDashboardSessionQuiesce({ ...make(), evidence: result }).retiredSessions.length, 1);
});
test(() => {
  const state = idleState();
  state.browsers.createdByFinishingJob = { id: 'session:dashboard-service-backend', pid: 890 };
  assert.throws(() => prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(state) }), /browser-free/);
});
test(() => {
  const state = idleState();
  state.viewerLeases.createdByFinishingJob = { state: 'connected' };
  assert.throws(() => prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(state) }), /viewers=1/);
});
test(() => {
  const state = idleState();
  state.browsers.orphan = { id: 'session:outside-inventory', pid: 890 };
  assert.throws(() => prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(state, make().evidence, (processPid) => processPid === 890 ? `${dashboardCgroup}/child` : readCgroup(processPid)) }), /browser would be stopped/);
});
test(() => {
  const state = idleState();
  state.browsers.attached = { id: 'session:dashboard-service-backend', pid: null };
  assert.throws(() => prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(state) }), /persisted=true/);
});
test(() => {
  const evidence = make().evidence;
  evidence[0].hasPersistedBrowser = true;
  assert.equal(refreshed(idleState(), evidence)[0].hasPersistedBrowser, true);
});
test(() => {
  const state = idleState();
  state.browsers.browser = { id: 'other-id', sessionName: 'dashboard-service-backend', pid: 890 };
  assert.deepEqual(refreshed(state)[0].browserPids, [890]);
});
for (const mutate of [
  (state) => { delete state.browsers; },
  (state) => { state.browsers = []; },
  (state) => { delete state.jobs; },
  (state) => { delete state.viewerLeases; },
  (state) => { state.jobs.pending = { state: 'running' }; },
  (state) => { state.jobs.pending = {}; },
  (state) => { state.viewerLeases.viewer = {}; },
  (state) => { state.browsers.unknown = { id: 'unknown', cdpEndpoint: 'http://127.0.0.1:9222' }; },
  (state) => { state.browsers.unknown = { pid: 890 }; },
  (state) => { state.browsers.unknown = { id: 'unknown', pid: 0 }; },
]) test(() => { const state = idleState(); mutate(state); assert.throws(() => refreshed(state)); });
test(() => { assert.throws(() => refreshed(idleState(), make().evidence, () => null), /cgroup evidence/); });
test(() => { const evidence = make().evidence; delete evidence[0].hasPersistedBrowser; assert.throws(() => prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(idleState(), evidence) }), /browser-free/); });
test(() => {
  const state = idleState();
  state.browsers.historical = { id: 'session:lab-record-research', pid: null, cdpEndpoint: null, health: 'degraded' };
  assert.deepEqual(prepareDashboardSessionQuiesce({ ...make(), evidence: refreshed(state) }).retiredSessions, ['dashboard-service-backend']);
});
console.log(`Dashboard session quiesce: ${cases} isolated cases passed`);
