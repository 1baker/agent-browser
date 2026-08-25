#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const publicationStatus = readFileSync('cli/src/native/publication_status.rs', 'utf8');
const installDoctor = readFileSync('cli/src/install.rs', 'utf8');
const streamHttp = readFileSync('cli/src/native/stream/http.rs', 'utf8');
const dashboardHttp = readFileSync('cli/src/native/stream/dashboard.rs', 'utf8');
const serviceContracts = readFileSync('cli/src/native/service_contracts.rs', 'utf8');
const mcp = readFileSync('cli/src/mcp.rs', 'utf8');
const client = readFileSync('packages/client/src/service-observability.js', 'utf8');
const dashboard = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const schema = JSON.parse(
  readFileSync('docs/dev/contracts/local-dashboard-publication-status.v1.schema.json', 'utf8'),
);

assert.match(
  publicationStatus,
  /local_dashboard_publication_status_for_path[\s\S]*if !journal_path\.exists\(\)[\s\S]*recommendedAction[\s\S]*investigate_installed_artifact[\s\S]*recover_only/,
  'installed status must remain read-only and derive recovery from verified journal, lock, and artifact evidence',
);
assert.match(
  publicationStatus,
  /MAX_JOURNAL_BYTES[\s\S]*MAX_ARTIFACT_BYTES[\s\S]*must be a regular file[\s\S]*sha256_file/,
  'installed status must bound input size, reject non-regular files, and redact failure detail',
);
assert.match(publicationStatus, /bounded_failure/, 'publication failure detail must be bounded');
assert.match(
  installDoctor,
  /"localDashboardPublication": local_dashboard_publication[\s\S]*dashboard_publication_recovery_required[\s\S]*requiresExplicitOperatorConfirmation[\s\S]*pnpm recover:local-dashboard-publication/,
  'install doctor must project status and mark recovery as an explicit operator command',
);
assert.match(
  streamHttp,
  /LOCAL_DASHBOARD_PUBLICATION_HTTP_ROUTE[\s\S]*local_dashboard_publication_status\(\)[\s\S]*"success": true/,
  'session service HTTP must expose the local publication status',
);
assert.match(
  dashboardHttp,
  /method == "GET" && path == LOCAL_DASHBOARD_PUBLICATION_HTTP_ROUTE[\s\S]*local_dashboard_publication_status\(\)[\s\S]*if path == "\/api\/service" \|\| path\.starts_with\("\/api\/service\/"\)/,
  'standalone dashboard must serve publication status locally before generic backend proxying',
);
assert.doesNotMatch(
  `${streamHttp}\n${dashboardHttp}`,
  /method == "POST" && path == LOCAL_DASHBOARD_PUBLICATION_HTTP_ROUTE/,
  'publication projection must not expose an HTTP recovery mutation',
);
assert.match(
  serviceContracts,
  /localDashboardPublicationStatus[\s\S]*recoveryAuthorized": false/,
  'service metadata must declare that status does not authorize recovery',
);
assert.match(
  mcp,
  /LOCAL_DASHBOARD_PUBLICATION_MCP_RESOURCE[\s\S]*local_dashboard_publication_status\(\)\?/,
  'MCP must expose the same read-only installed status',
);
assert.match(
  client,
  /function getLocalDashboardPublicationStatus[\s\S]*\/api\/service\/publications\/local-dashboard/,
  'the generated-client package must expose the read-only HTTP helper',
);
assert.match(
  dashboard,
  /\/publications\/local-dashboard[\s\S]*label="Publication"[\s\S]*Dashboard publication needs review[\s\S]*pnpm recover:local-dashboard-publication/,
  'the dashboard must surface status and the reviewed recovery-only command',
);
assert.doesNotMatch(
  dashboard,
  /fetch\([^\n]*publications\/local-dashboard[^\n]*method:\s*"POST"/,
  'the dashboard must not turn status visibility into recovery authority',
);
assert.equal(
  schema.$id,
  'https://agent-browser.local/contracts/local-dashboard-publication-status.v1.schema.json',
);
assert.deepEqual(schema.properties.recommendedAction.enum, [
  'none',
  'wait_for_active_publisher',
  'recover_only',
  'investigate_installed_artifact',
  'investigate_retained_browser',
]);

console.log('Dashboard publication status projection tests passed');
