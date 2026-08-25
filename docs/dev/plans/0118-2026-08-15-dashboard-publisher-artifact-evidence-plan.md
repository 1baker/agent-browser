# Plan 0118 | Dashboard Publisher Artifact Evidence

State: CLOSED
Roadmap: P118
Plan version: 2
Date: 2026-08-15

## Objective

Bind dashboard publication and rollback to SHA-256 evidence for every binary
transition, preserve the initiating publication error when restore also fails,
and never restart an unverified installed artifact.

## Scope

- hash the built runtime before any live mutation boundary;
- hash the installed source before backup and again after backup copy;
- verify the backup hash exactly matches the stable installed source;
- verify the installed replacement exactly matches the built runtime;
- verify a restored binary exactly matches the previously verified backup;
- retain structured built, source, backup, replacement, restoration, and safe
  restart evidence in the publisher report;
- keep the initiating publication error primary when backup disappearance,
  restore copy failure, or restore hash mismatch occurs;
- restart after a restore failure only when the currently installed artifact
  still matches a verified backup or verified replacement;
- extend the isolated real-file fixture and fast smoke-policy contract.

## Non-Goals

- no live build, publication, install, daemon handoff, listener restart, or
  browser lifecycle action;
- no retained page, profile, target, prompt, credential, or external service
  operation;
- no GitHub write or release action.

## Acceptance Criteria

1. Production and fixture paths use the same artifact-evidence orchestration.
2. Successful publication records verified built, source, backup, and
   replacement SHA-256 evidence.
3. Backup mismatch fails before quiescence, replacement, or restart.
4. Replacement mismatch enters rollback and proves the restored source hash.
5. Successful rollback records the restored hash and marks it verified before
   restart.
6. Missing backup and restore-copy failure preserve the initiating error,
   record `restoreError`, and restart only a currently verified replacement or
   backup.
7. Restore hash mismatch preserves the initiating error, records the mismatch,
   and skips restart of the unverified artifact.
8. Focused and widened validation pass without changing live runtime identity.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: Graphiti's documented runtime is unavailable and CodeGraph
  reports this checkout as not initialized. Current source, P117 evidence, and
  focused tests are the documented fallback; no index was created.
- implementation: orchestration now records and verifies built, source, backup,
  replacement, restoration, and safe-restart artifact hashes. Restore failures
  are secondary report evidence and never replace the initiating error.
- regression: the real-file fixture covers verified success and rollback,
  backup mismatch, replacement mismatch, missing backup, restore-copy failure,
  restore hash mismatch, committed handoff preservation, rollback restart
  failure, and pre-mutation failure.
- safety: an unverified installed artifact is not executed after restore hash
  failure. A verified replacement may be restarted after the backup vanishes or
  the restore copy fails before replacement.
- focused_validation: orchestration fixture, publisher smoke policy, local
  convergence, and JavaScript syntax checks pass.
- live_boundary: no installed runtime, listener, daemon, or retained browser
  action occurred.
- next_action_or_stop_reason: run widened parity, capability, dashboard, type,
  build, documentation, audit, skill-parity, cleanup, diff, and read-only live
  checks before closing P118.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: the production publisher supplies real `existsSync` and
  `sha256File` adapters to the shared orchestration. Every rollback restart now
  re-hashes the current installed file and requires a match to verified backup
  or replacement evidence, including committed-handoff and fresh-install paths.
- regression: the real-file fixture proves verified success and rollback,
  backup corruption before quiescence, replacement corruption with and without
  committed handoff evidence, missing backup, restore-copy failure, restore-hash
  mismatch, rollback restart failure, and pre-mutation failure. Unknown restored
  bytes are not executed.
- error_precedence: readiness or replacement failure remains the thrown primary
  error. Backup disappearance, restore-copy failure, restore mismatch, and
  rollback restart failure remain separate structured report fields.
- validation: publisher policy, lifecycle, orchestration, local convergence,
  browser capability, API/MCP parity, dashboard incident and workspace
  contracts, service-client JavaScript types, JavaScript syntax, dashboard and
  docs production builds, package JSON, CI YAML, repository/installed skill
  parity, direct plan audit, cleanup, and scoped diff hygiene pass.
- lint_baseline: docs-wide ESLint retains the pre-existing
  `theme-toggle.tsx` effect error and one search-route warning. Neither file is
  modified by this slice.
- unavailable_optional_tools: `actionlint` and Ruby are not installed and are
  not claimed as completed gates.
- cleanup: post-test inspection finds zero orchestration or lifecycle fixture
  directories.
- retained_lane: read-only service evidence still reports browser PID 1046742,
  CDP endpoint
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`,
  ready health, profile `chatgpt-pro`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, the exact Workshop URL, and title
  `Architecture Review Boundaries`.
- live_runtime: no publication occurred. Installed and workspace reference
  SHA-256 remains
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`;
  the live dashboard returns HTTP 200 with dashboard SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, browser lifecycle, credential, attach, detach, daemon handoff,
  install, or live dashboard action occurred.
- next_action_or_stop_reason: P118 acceptance criteria are met. Next, persist a
  hash-bound publication transaction journal before quiescence and update it
  atomically through replacement, handoff, readiness, and rollback so a process
  crash can be reconciled idempotently instead of losing this evidence.
