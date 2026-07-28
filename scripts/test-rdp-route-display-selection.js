#!/usr/bin/env node

import assert from 'node:assert/strict';
import { selectRouteDisplayName } from './lib/rdp-route-display-selection.js';

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

console.log('RDP route display selection behavior passed');
