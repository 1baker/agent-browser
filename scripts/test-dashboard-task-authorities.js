#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  parseTaskAuthorityReplacementPlan,
  taskAuthorityOutcomeCounts,
  taskAuthorityRecoveryEligibility,
  taskAuthoritySuggestedReplacementSteps,
} from '../packages/dashboard/src/lib/service-task-authorities.ts';

const indeterminateStep = {
  stepId: 'authority-1:step-0',
  stepIndex: 0,
  commandId: 'command-1',
  action: 'title',
  currentUrl: 'https://example.com/',
  outcome: { state: 'indeterminate', success: null },
};
const authority = {
  id: 'authority-1',
  sessionId: 'retained-lane',
  state: 'exhausted',
  approvalReference: 'plan-109',
  envelope: {
    taskName: 'inspect-source',
    serviceName: 'concierge',
    agentName: 'codex',
    targetBinding: { targetId: 'target-1', initialUrl: 'https://example.com/' },
  },
  approvedPlan: {
    steps: [
      { stepId: 'authority-1:step-0', index: 0, action: 'title', evidenceBytes: 1024 },
      { stepId: 'authority-1:step-1', index: 1, action: 'url', evidenceBytes: 2048 },
    ],
  },
  usage: {
    nextStepIndex: 1,
    completedSteps: [],
    failedSteps: [],
    indeterminateSteps: [indeterminateStep],
    outcomeSummary: { completed: 0, failed: 0, indeterminate: 1 },
  },
};
const exactTabs = [{ id: 'target:target-1', targetId: 'target-1', lifecycle: 'ready', url: 'https://example.com/' }];

assert.deepEqual(taskAuthorityOutcomeCounts(authority), { completed: 0, failed: 0, indeterminate: 1 });
assert.deepEqual(taskAuthoritySuggestedReplacementSteps(authority), [{ action: 'url', evidenceBytes: 2048 }]);
const eligible = taskAuthorityRecoveryEligibility(authority, exactTabs);
assert.equal(eligible.eligible, true);
assert.equal(eligible.indeterminateStep?.commandId, 'command-1');
assert.deepEqual(eligible.target, { targetId: 'target-1', url: 'https://example.com/', tabId: 'target:target-1' });
assert.equal(JSON.stringify(eligible.suggestedSteps).includes('step-0'), false);
assert.equal(JSON.stringify(eligible.suggestedSteps).includes('stepId'), false);

assert.equal(taskAuthorityRecoveryEligibility(authority, []).eligible, false);
assert.match(taskAuthorityRecoveryEligibility(authority, []).reason, /found 0/);
assert.equal(taskAuthorityRecoveryEligibility(authority, [...exactTabs, { ...exactTabs[0], id: 'duplicate' }]).eligible, false);
assert.match(taskAuthorityRecoveryEligibility(authority, [...exactTabs, { ...exactTabs[0], id: 'duplicate' }]).reason, /found 2/);
assert.equal(taskAuthorityRecoveryEligibility({ ...authority, usage: { ...authority.usage, indeterminateSteps: [] } }, exactTabs).eligible, false);
assert.equal(taskAuthorityRecoveryEligibility({
  ...authority,
  usage: { ...authority.usage, indeterminateSteps: [indeterminateStep, { ...indeterminateStep, stepId: 'step-2' }] },
}, exactTabs).eligible, false);
assert.equal(taskAuthorityRecoveryEligibility({
  ...authority,
  revocation: { reconciliation: { reconciliationId: 'reconcile-1', state: 'pending' } },
}, exactTabs).eligible, false);

assert.deepEqual(
  parseTaskAuthorityReplacementPlan('[{"action":"title","evidenceBytes":1024}]'),
  { success: true, steps: [{ action: 'title', evidenceBytes: 1024 }] },
);
assert.equal(parseTaskAuthorityReplacementPlan('[]').success, false);
assert.equal(parseTaskAuthorityReplacementPlan('[{"action":"navigate"}]').success, false);
assert.equal(parseTaskAuthorityReplacementPlan('[{"action":"title","evidenceBytes":0}]').success, false);
assert.equal(parseTaskAuthorityReplacementPlan('[{"action":"title","stepId":"replayed"}]').success, false);
assert.equal(parseTaskAuthorityReplacementPlan('[{"action":"title","commandId":"replayed"}]').success, false);

const workspaceSource = readFileSync('packages/dashboard/src/components/task-authority-workspace.tsx', 'utf8');
const servicePanelSource = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
assert.match(workspaceSource, /AlertDialogTitle>Confirm this replacement authority/);
assert.match(workspaceSource, /decision: "deny"/);
assert.match(workspaceSource, /expectedAction: "task_authority_reconcile"/);
assert.doesNotMatch(workspaceSource, /decidedBy:/);
assert.doesNotMatch(workspaceSource, /issuer:/);
assert.match(workspaceSource, /Confirmation receipts/);
assert.match(workspaceSource, /receipt\.executionState === "indeterminate"/);
assert.match(workspaceSource, /Receipt retention review/);
assert.match(workspaceSource, /task-authorities\/confirmations\/cleanup/);
assert.match(workspaceSource, /Apply this exact receipt cleanup/);
assert.match(workspaceSource, /legacyPendingMigration/);
assert.match(workspaceSource, /tombstoneLedger\?\.integrityState !== "verified"/);
assert.match(workspaceSource, /reviewSha256/);
assert.match(workspaceSource, /if \(confirmation\)[\s\S]*decideConfirmation\("deny"\)/);
assert.match(workspaceSource, /data\.confirmation_required !== true/);
assert.match(workspaceSource, /targetId !== target\.targetId \|\| url !== target\.url/);
assert.doesNotMatch(workspaceSource, /\b(?:alert|confirm|prompt)\s*\(/);
assert.match(servicePanelSource, /value: "authorities" as const/);
assert.match(servicePanelSource, /task-authorities[\s\S]*sessionName/);
assert.match(servicePanelSource, /<TaskAuthorityWorkspace/);

console.log('Dashboard task authority recovery tests passed');
