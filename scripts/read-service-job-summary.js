import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const RESOURCE = 'agent-browser://jobs';
const TERMINAL = new Set(['succeeded', 'failed', 'cancelled']);

// Read the complete resource locally before projecting it into tool output.
// An empty snapshot is an observation, never a lease or permission to attach.
export function summarizeJobs(raw) {
  const response = JSON.parse(raw);
  const contents = response.data?.contents;
  if (response.success !== true || response.data?.uri !== RESOURCE ||
      !Array.isArray(contents?.jobs) || contents.count !== contents.jobs.length) {
    throw new Error('Incomplete jobs resource');
  }
  const counts = new Map();
  const ids = new Set();
  let nonterminalCount = 0;
  let unknownCount = 0;
  for (const job of contents.jobs) {
    if (!job || typeof job.id !== 'string' || !job.id || ids.has(job.id) ||
        typeof job.state !== 'string') throw new Error('Invalid job record');
    ids.add(job.id);
    const terminal = TERMINAL.has(job.state);
    const known = terminal || ['queued', 'running', 'waiting'].includes(job.state);
    const bucket = known ? job.state : 'unknown';
    counts.set(bucket, (counts.get(bucket) ?? 0) + 1);
    if (!terminal) nonterminalCount++;
    if (!known) unknownCount++;
  }
  return {
    schema: 'agent-browser.job-summary.v1',
    complete: true,
    retainedCount: contents.count,
    stateCounts: Object.fromEntries(counts),
    nonterminalCount,
    unknownCount,
    observedIdle: nonterminalCount === 0,
    grantsBrowserAuthority: false,
  };
}

export function readJobSummary(binary, run = execFileSync) {
  const raw = run(binary, ['--json', 'mcp', 'read', RESOURCE], {
    encoding: 'utf8', timeout: 15000, maxBuffer: 16 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return summarizeJobs(raw);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (process.argv.length !== 3) throw new Error('Expected binary path');
    const summary = readJobSummary(process.argv[2]);
    console.log(JSON.stringify(summary));
    if (!summary.observedIdle) process.exitCode = 2;
  } catch {
    // Never echo raw job payloads, stderr, or parse-error excerpts.
    console.log(JSON.stringify({ complete: false, observedIdle: false,
      grantsBrowserAuthority: false, error: 'job_summary_unavailable' }));
    process.exitCode = 1;
  }
}
