#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const explicit = args.some((arg) => arg.startsWith('--expect-retained-'));
const requirementFlag = args.indexOf('--retained-requirement');
if (requirementFlag >= 0 && !args[requirementFlag + 1]) {
  console.error('Missing value for --retained-requirement');
  process.exit(2);
}
const requirementPath = resolve(
  requirementFlag >= 0 && args[requirementFlag + 1]
    ? args[requirementFlag + 1]
    : process.env.AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT
      || resolve(homedir(), '.agent-browser', 'publications', 'local-dashboard-retained-browser.json'),
);
const json = args.includes('--json');
const statusOnlyArgs = requirementFlag >= 0
  ? args.filter((_, index) => index !== requirementFlag && index !== requirementFlag + 1)
  : args;
const noOpEligible = statusOnlyArgs.every((arg) => arg === '--json');

if (
  !explicit
  && noOpEligible
  && !existsSync(requirementPath)
  && !existsSync(`${requirementPath}.required`)
) {
  const result = {
    success: true,
    operation: 'retained_browser_status',
    status: 'not_configured',
    retainedBrowserRequirement: {
      path: requirementPath,
      exists: false,
    },
  };
  if (json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log('Retained browser publication guard: not configured');
  }
  process.exit(0);
}

const scriptDirectory = fileURLToPath(new URL('.', import.meta.url));
const publisher = resolve(scriptDirectory, 'publish-local-dashboard-runtime.js');
const result = spawnSync(process.execPath, [publisher, '--retained-browser-status', ...args], {
  cwd: resolve(scriptDirectory, '..'),
  env: process.env,
  encoding: 'utf8',
  stdio: 'inherit',
});
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 1);
