import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve } from 'node:path';

// A local model fixture exercises the actual CLI without credentials or browser work.
const binary = resolve(process.env.AGENT_BROWSER_TEST_BIN || 'cli/target/debug/agent-browser');
let requests = 0;
let mode = 'exhausted';
const server = createServer((request, response) => {
  request.resume();
  requests += 1;
  response.writeHead(200, { 'Content-Type': 'text/event-stream' });
  const delta = mode === 'complete'
    ? { content: 'Fixture complete' }
    : { tool_calls: [{ index: 0, id: `call_${requests}`, function: {
      name: 'agent_browser', arguments: JSON.stringify({ command: 'not-a-browser-command' }),
    } }] };
  response.write(`data: ${JSON.stringify({ choices: [{ delta }] })}\n\n`);
  response.end(mode === 'truncated' ? '' : 'data: [DONE]\n\n');
});
await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));

async function run(scenario) {
  mode = scenario;
  requests = 0;
  const child = spawn(binary, ['--json', 'chat', 'Run the local completion fixture'], {
    env: { ...process.env, AI_GATEWAY_API_KEY: 'local-fixture-only',
      AI_GATEWAY_URL: `http://127.0.0.1:${server.address().port}` },
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: 30000,
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', (chunk) => { stdout += chunk; });
  child.stderr.on('data', (chunk) => { stderr += chunk; });
  const code = await new Promise((resolveExit, reject) => {
    child.on('error', reject);
    child.on('close', resolveExit);
  });
  assert.ok(stdout.trim(), stderr);
  return { code, output: JSON.parse(stdout), requests };
}

try {
  const exhausted = await run('exhausted');
  assert.equal(exhausted.code, 1);
  assert.equal(exhausted.requests, 50);
  assert.equal(exhausted.output.success, false);
  assert.match(exhausted.output.error, /step limit/);
  const truncated = await run('truncated');
  assert.equal(truncated.code, 1);
  assert.equal(truncated.requests, 1);
  assert.match(truncated.output.error, /partial tool calls were not executed/);
  const complete = await run('complete');
  assert.equal(complete.code, 0);
  assert.equal(complete.requests, 1);
  assert.equal(complete.output.success, true);
  assert.equal(complete.output.text, 'Fixture complete');
  console.log('CLI chat completion: exhaustion, truncated stream, and completed response passed.');
} finally {
  server.closeAllConnections();
  await new Promise((resolveClose) => server.close(resolveClose));
}
