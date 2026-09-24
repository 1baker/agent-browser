#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');

assert.match(
  dashboardPage,
  /remoteViewHandoffIdFromPath\([\s\S]*remote-view[\s\S]*function RemoteViewHandoffGate\([\s\S]*action: "service_remote_view_handoff_resolve"/,
  'authenticated dashboard routes must resolve opaque remote-view handoff IDs through the service queue',
);

assert.match(
  dashboardPage,
  /next\.startsWith\("\/guacamole\/"\)[\s\S]*window\.location\.assign\(next\)/,
  'post-auth forwarding to a direct Guacamole path must perform a real navigation',
);

assert.match(
  dashboardPage,
  /resolveHandoff\(true\)[\s\S]*Reopen tab/,
  'a deliberately closed handoff target must require an explicit reopen action',
);

assert.match(
  dashboardPage,
  /Remote view unavailable[\s\S]*resolveHandoff\(false\)[\s\S]*Recover and take control/,
  'a retained-owner failure must expose an explicit recovery action',
);

assert.match(
  dashboardPage,
  /cause\?: string \| null;[\s\S]*recourse\?: string \| null;[\s\S]*requestDispatched\?: boolean;/,
  'typed recovery failures must expose cause, recourse, and dispatch state',
);

assert.match(
  dashboardPage,
  /payload\.error[\s\S]*payload\.recourse[\s\S]*filter\(Boolean\)\.join\(" "\)/,
  'the recovery screen must present operator recourse returned by the service',
);

assert.match(
  dashboardPage,
  /params\.set\("view-provider", nextResolution\.viewStreamProvider\)[\s\S]*params\.set\("view", "workspace:control"\)/,
  'successful handoff resolution must preserve the intended view provider and open workspace control',
);

assert.doesNotMatch(
  dashboardPage,
  /window\.location\.assign\(nextResolution\.providerFallbackUrl\)/,
  'exact recovery must not redirect to a raw provider fallback URL',
);

assert.match(
  viewport,
  /params\?\.get\("view-provider"\)[\s\S]*stream\.provider\?\.trim\(\)\.toLowerCase\(\) === intendedProvider/,
  'workspace control must prefer the provider encoded by the durable handoff route',
);

console.log('dashboard durable remote-view handoff checks passed');
