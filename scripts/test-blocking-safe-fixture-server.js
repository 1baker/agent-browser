#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

import { startBlockingSafeFixtureServer } from './lib/blocking-safe-fixture-server.js';

const marker = `BLOCKING SAFE FIXTURE ${process.pid}`;
const fixture = await startBlockingSafeFixtureServer({
  html: `<title>${marker}</title><h1>${marker}</h1>`,
});

try {
  const url = `http://${fixture.host}:${fixture.port}/`;
  const probe = spawnSync(process.execPath, [
    '-e',
    'fetch(process.argv[1]).then(async response => { if (!response.ok) process.exit(2); process.stdout.write(await response.text()); }).catch(error => { console.error(error); process.exit(1); })',
    url,
  ], {
    encoding: 'utf8',
    timeout: 5000,
  });
  assert.equal(
    probe.status,
    0,
    `fixture server did not respond while the parent thread was blocked:\n${probe.stdout}${probe.stderr}`,
  );
  assert.match(probe.stdout, new RegExp(marker));
  console.log('Blocking-safe fixture server test passed');
} finally {
  await fixture.close();
}
