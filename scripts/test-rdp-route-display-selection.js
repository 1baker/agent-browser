#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  routeDisplayInspectorPath,
  selectRouteDisplayName,
} from './lib/rdp-route-display-selection.js';

assert.equal(
  selectRouteDisplayName({
    configuredDisplayName: ':10',
    inferredDisplayName: ':21',
  }),
  ':21',
  'a live inferred display must override a stale configured display hint',
);

assert.equal(
  selectRouteDisplayName({
    configuredDisplayName: ':10',
    inferredDisplayName: null,
  }),
  ':10',
  'the configured display remains the fallback when live inference is unavailable',
);

assert.equal(
  selectRouteDisplayName({
    configuredDisplayName: '',
    inferredDisplayName: '',
  }),
  null,
  'missing live and configured displays must remain unassigned',
);

const inspectorPath = routeDisplayInspectorPath(import.meta.url);
assert.equal(
  inspectorPath.endsWith('/scripts/inspect-rdp-route-displays.js'),
  true,
  'route display inspection must use a script-relative absolute path so doctor cwd cannot disable inference',
);

console.log('RDP route display selection behavior passed');
