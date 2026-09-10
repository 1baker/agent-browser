#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-retained-preparation-'));
const auditPath = join(root, 'audit.jsonl');
const fakeAgentBrowser = join(root, 'agent-browser');
  const fakePinner = join(root, 'pinner.js');
const url = 'https://chatgpt.com/g/g-p-workshop/c/conversation-id';

try {
  writeFileSync(fakeAgentBrowser, `#!/usr/bin/env node
const fs = require('fs');
fs.appendFileSync(process.env.PREPARATION_AUDIT, JSON.stringify({ kind: 'open', args: process.argv.slice(2) }) + '\\n');
const url = process.argv[process.argv.indexOf('open') + 1];
const profile = process.argv[process.argv.indexOf('--runtime-profile') + 1];
const session = process.argv[process.argv.indexOf('--session') + 1];
console.log(JSON.stringify({ success: true, data: {
  status: 'opened', dryRun: false, browserId: 'session:workshop', sessionName: session,
  handoffUrl: 'https://desktop.example.test/remote-view/r1',
  intent: { url, runtimeProfile: profile },
  operatorVisible: { state: 'ready', target: { expectedUrl: url, profileId: profile, state: 'ready', targetId: 'target-workshop', url, urlReadiness: 'ready' } },
  routeBoundHandoff: { profile: { id: profile, runtimeProfile: profile } },
  sharedAcquisition: { browserId: 'session:workshop', profileId: profile, sessionName: session }
} }));
`, { mode: 0o755 });
  writeFileSync(fakePinner, `#!/usr/bin/env node
const fs = require('fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.PREPARATION_AUDIT, JSON.stringify({ kind: 'pin', args }) + '\\n');
const exactUrl = args[args.indexOf('--exact-url') + 1];
const profileId = args[args.indexOf('--profile-id') + 1];
console.log(JSON.stringify({ success: true, data: {
  discovery: { exactUrl, profileId, matchedCandidateCount: 1 },
  requirement: { exists: true, written: true, path: '/fixture/requirement.json' }
} }));
`, { mode: 0o755 });

  const result = spawnSync(process.execPath, [
    resolve('scripts/prepare-local-dashboard-retained-browser.js'),
    '--url', url,
    '--url-prefix', 'https://chatgpt.com/g/g-p-workshop/',
    '--runtime-profile', 'chatgpt-pro',
    '--agent-browser-bin', fakeAgentBrowser,
    '--json',
  ], {
    cwd: resolve('.'),
    encoding: 'utf8',
    env: {
      ...process.env,
      AGENT_BROWSER_RETAINED_PINNER_SCRIPT: fakePinner,
      PREPARATION_AUDIT: auditPath,
    },
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.success, true);
  assert.equal(payload.state, 'ready_and_pinned');
  const audit = readFileSync(auditPath, 'utf8').trim().split('\n').map(JSON.parse);
  assert.equal(audit.length, 2);
  assert.deepEqual(audit.map((entry) => entry.kind), ['open', 'pin']);
  assert.deepEqual(audit[0].args.slice(0, 6), [
    '--json', '--session', 'chatgpt-pro', 'remote-view', 'open', url,
  ]);
  assert.equal(audit[1].args.includes('--exact-url'), true);
  assert.equal(audit[1].args.includes('--profile-id'), true);
  for (const entry of audit) {
    for (const forbidden of ['click', 'type', 'fill', 'evaluate', 'send', 'submit']) {
      assert.equal(entry.args.includes(forbidden), false);
    }
  }
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Local dashboard retained browser preparation command fixture passed');
