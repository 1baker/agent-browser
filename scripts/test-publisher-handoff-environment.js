#!/usr/bin/env node
// Publisher handoff authorization is scoped to its one pre-install `handoff prepare`.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PUBLISHER_HANDOFF_ENVIRONMENT, agentCommandEnvironment } from './lib/agent-command-environment.js';

// 1. The authorization is exact and immutable.
assert.deepEqual({ ...PUBLISHER_HANDOFF_ENVIRONMENT }, { AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE: '1' });
assert.ok(Object.isFrozen(PUBLISHER_HANDOFF_ENVIRONMENT));

// 2. Without an extra environment the base object is passed through untouched.
const base = Object.freeze({ PATH: '/usr/bin', HOME: '/home/fixture' });
assert.equal(agentCommandEnvironment(base), base);
assert.equal(agentCommandEnvironment(base, undefined).AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE, undefined);

// 3. With it, a NEW object carries the override; neither input nor process.env is mutated.
const envBefore = JSON.stringify(process.env);
const merged = agentCommandEnvironment(base, PUBLISHER_HANDOFF_ENVIRONMENT);
assert.notEqual(merged, base);
assert.equal(merged.AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE, '1');
assert.equal(merged.PATH, '/usr/bin');
assert.equal(base.AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE, undefined);
assert.equal(JSON.stringify(process.env), envBefore);
assert.equal(process.env.AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE, undefined);

// 4. In the publisher, exactly one runAgentJson call passes the authorization, and it is
//    the `handoff prepare` call; runAgentJson merges it only into that spawn's env.
const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const publisher = readFileSync(join(root, 'scripts/publish-local-dashboard-runtime.js'), 'utf8');
const calls = [...publisher.matchAll(/runAgentJson\(([\s\S]*?)\);/g)].map((match) => match[1]);
const authorized = calls.filter((args) => args.includes('PUBLISHER_HANDOFF_ENVIRONMENT'));
assert.equal(authorized.length, 1, 'exactly one authorized subprocess');
assert.match(authorized[0], /\['handoff', 'prepare'\]/);
for (const args of calls.filter((args) => !args.includes('PUBLISHER_HANDOFF_ENVIRONMENT'))) {
  assert.doesNotMatch(args, /SIDEGRADE/);
}
assert.match(publisher, /env: agentCommandEnvironment\(baseEnvironment, extraEnvironment\)/);
assert.doesNotMatch(publisher, /process\.env\.AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE\s*=/);

console.log('publisher handoff environment: ok');
