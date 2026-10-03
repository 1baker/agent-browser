#!/usr/bin/env node

import assert from 'node:assert/strict';
import { getServiceRuns } from '../packages/client/src/service-runs.js';

const calls = [];
const runs = await getServiceRuns({
  baseUrl: 'http://127.0.0.1:4848/api/service/',
  fetchImpl: async (url) => {
    calls.push(url);
    return {
      ok: true,
      json: async () => ({
        success: true,
        data: {
          runs: [{ id: 'job:1', state: 'activity_succeeded', taskOutcomeVerified: false }],
          count: 1,
          coverage: 'recent_jobs_and_retained_human_gates',
          durableTaskHistory: false,
        },
      }),
    };
  },
});
assert.deepEqual(calls, ['http://127.0.0.1:4848/api/service/runs']);
assert.equal(runs.runs[0].taskOutcomeVerified, false);
assert.equal(runs.durableTaskHistory, false);

await assert.rejects(
  getServiceRuns({
    baseUrl: 'http://127.0.0.1:4848/api/service',
    fetchImpl: async () => ({
      ok: false,
      status: 503,
      json: async () => ({ success: false, error: 'backend unavailable' }),
    }),
  }),
  /backend unavailable/,
);

console.log('service runs client passed');
