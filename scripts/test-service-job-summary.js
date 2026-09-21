import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readJobSummary, summarizeJobs } from './read-service-job-summary.js';

const resource = jobs => JSON.stringify({ success: true, data: {
  uri: 'agent-browser://jobs', contents: { count: jobs.length, jobs },
} });

test('large historical payload produces a small payload-free summary', () => {
  const raw = resource(Array.from({ length: 200 }, (_, i) => ({
    id: `job-${i}`, state: i % 2 ? 'failed' : 'succeeded',
    result: { secret: 'private-data'.repeat(1000) },
  })));
  assert.ok(raw.length > 1000000);
  const summary = summarizeJobs(raw);
  assert.equal(summary.observedIdle, true);
  assert.equal(summary.retainedCount, 200);
  assert.equal(summary.grantsBrowserAuthority, false);
  assert.ok(JSON.stringify(summary).length < 500);
  assert.ok(!JSON.stringify(summary).includes('private-data'));
});

test('active oldest job is not hidden by more recent terminal records', () => {
  for (const state of ['queued', 'running', 'waiting', 'future_state']) {
    const jobs = [{ id: 'oldest', state }, ...Array.from({ length: 200 },
      (_, i) => ({ id: `${i}`, state: 'succeeded' }))];
    assert.equal(summarizeJobs(resource(jobs)).nonterminalCount, 1);
    assert.equal(summarizeJobs(resource(jobs)).observedIdle, false);
  }
});

test('unknown states stay bounded and block idle', () => {
  const result = summarizeJobs(resource([{ id: 'a', state: 'private'.repeat(10000) }]));
  assert.equal(result.unknownCount, 1);
  assert.deepEqual(result.stateCounts, { unknown: 1 });
});

test('malformed, partial, failed and duplicate records fail closed', () => {
  for (const raw of ['{"truncated', '{}',
    resource([null]), resource([{ id: 'a' }]),
    resource([{ id: 'a', state: 'failed' }, { id: 'a', state: 'failed' }]),
    resource([]).replace('"count":0', '"count":1'),
    resource([]).replace('"success":true', '"success":false'),
    resource([]).replace('agent-browser://jobs', 'agent-browser://events')]) {
    assert.throws(() => summarizeJobs(raw));
  }
});

test('reader uses only the no-launch resource command with bounded capture', () => {
  assert.equal(readJobSummary('/installed/agent-browser', (binary, args, options) => {
    assert.equal(binary, '/installed/agent-browser');
    assert.deepEqual(args, ['--json', 'mcp', 'read', 'agent-browser://jobs']);
    assert.equal(options.timeout, 15000);
    assert.equal(options.maxBuffer, 16 * 1024 * 1024);
    return resource([]);
  }).observedIdle, true);
  assert.throws(() => readJobSummary('/installed/agent-browser', () => {
    throw new Error('timeout or buffer limit');
  }));
});
