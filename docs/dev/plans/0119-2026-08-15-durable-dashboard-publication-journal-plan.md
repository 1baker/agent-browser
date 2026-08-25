# Plan 0119 | Durable Dashboard Publication Journal

State: CLOSED
Roadmap: P119
Plan version: 2
Date: 2026-08-15

## Objective

Persist the local dashboard publisher's hash-bound transaction before
quiescence and reconcile an interrupted replacement, handoff, restart, or
readiness sequence idempotently on the next invocation.

## Scope

- create one mode-0600, schema-versioned publication journal under
  `~/.agent-browser/publications/`;
- serialize publication ownership with an exact PID lock and recover stale lock
  metadata without broad process operations;
- publish each revision with staged write, file fsync, atomic rename, and
  directory fsync where supported;
- enforce transaction ID and revision continuity and fail closed on invalid or
  conflicting journal state;
- checkpoint admission and completion around quiescence, runtime handoff,
  replacement, handoff resume, dashboard restart, readiness, and rollback;
- discover exact candidate-session handoff descriptors after a crash and reject
  descriptor identity mismatch;
- reconcile already-resumed exact browser PID and CDP evidence without replaying
  handoff resume;
- recognize a replacement installed immediately before a crash from the durable
  built-artifact hash and repeat reference sync safely;
- recover verified replacement state to readiness, recover verified source state
  to rolled back, and block unknown installed bytes;
- add isolated real-file journal and recovery coverage to fast Dashboard CI.

## Non-Goals

- no live publisher invocation, install, listener restart, daemon handoff, or
  browser lifecycle operation;
- no prompt, page, profile, target, credential, or external service mutation;
- no GitHub write or release action.

## Acceptance Criteria

1. Production and fixture orchestration use the same journal and recovery state
   machine.
2. Journal files and locks are user-only, atomically replaced, fsynced, and
   revision checked.
3. A live owner lock blocks concurrent publication; an exact stale PID lock is
   recoverable.
4. Every mutation boundary has a durable admission or completion checkpoint.
5. Replacement recovery completes readiness without rebuilding, backing up, or
   reinstalling.
6. Source recovery resumes discovered handoffs when necessary and restarts only
   after quiescence was admitted.
7. An already-resumed exact handoff is not replayed.
8. A crash after replacement but before its checkpoint recognizes the installed
   built hash, repeats reference sync idempotently, and reaches readiness.
9. Unknown installed bytes fail closed without build, handoff, or restart.
10. Focused and widened validation pass without changing live runtime identity.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: Graphiti's documented runtime is unavailable and CodeGraph
  reports the checkout as not initialized. Current source, P118 evidence, Rust
  handoff implementation, and focused tests are the documented fallback; no
  index was created.
- implementation: a secured atomic journal now owns publication concurrency,
  phase revisions, artifact evidence, handoff candidates, recovery evidence,
  readiness, rollback, and terminal state.
- recovery: verified replacement resumes to readiness; verified source resumes
  to rolled back; exact `.handoff.json` descriptors recover admissions not yet
  checkpointed; already-resumed PID/CDP identity is accepted without command
  replay; unknown bytes block.
- crash_window: installed bytes matching the durable built hash synthesize
  verified replacement evidence when a crash occurred after atomic rename but
  before the replacement checkpoint. Reference binaries are then re-synced
  idempotently from the verified installed binary.
- focused_validation: journal, orchestration, lifecycle, smoke-policy, and
  JavaScript syntax fixtures pass.
- live_boundary: no installed artifact, listener, daemon, browser, profile,
  page, or external service was touched.
- next_action_or_stop_reason: run widened convergence, parity, capability,
  dashboard, client-type, build, documentation, plan-audit, skill-parity,
  cleanup, diff, and read-only retained-lane checks before closing P119.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- persistence: journal directory is mode 0700, records and locks are mode 0600,
  staged records are file-fsynced before atomic rename, and the containing
  directory is fsynced where supported. Exact transaction and revision conflict
  checks fail closed.
- checkpointing: explicit admission and completion phases cover quiescence,
  handoff prepare, replacement, reference sync, handoff resume, dashboard
  restart, readiness, rollback restore, and rollback restart.
- process_death: a child process commits a nonterminal journal and exits without
  releasing its lock. The parent recovers the exact stale lock and durable phase,
  then advances the same transaction to a terminal recovered state.
- recovery: real-file fixtures recover verified replacement to ready, source to
  rolled back, exact discovered handoff evidence to resume, and prior resumed
  evidence without replay. Crash-after-rename synthesizes replacement identity
  from durable built SHA and re-runs reference sync. Unknown bytes produce
  `recovery_blocked` with no build or restart.
- production_handoff: resume first reads service browser identity. Exact active
  PID and CDP evidence is recorded as `alreadyResumed`; changed PID or endpoint
  fails closed. Exact candidate `.handoff.json` descriptors recover a prepare
  response lost with the publisher process.
- validation: journal, orchestration, lifecycle, publisher policy, local
  convergence, browser capability, API/MCP parity, dashboard incident and
  workspace contracts, service-client JavaScript types, JavaScript syntax,
  dashboard and docs production builds, package JSON, CI YAML,
  repository/installed skill parity, direct plan audit, cleanup, and scoped diff
  hygiene pass.
- lint_baseline: docs-wide ESLint retains the pre-existing
  `theme-toggle.tsx` effect error and one search-route warning. Neither file is
  modified by this slice.
- unavailable_optional_tools: `actionlint` and Ruby are not installed and are
  not claimed as completed gates.
- cleanup: post-test inspection finds zero journal, orchestration, or lifecycle
  fixture directories. No production publication journal or lock was created by
  tests.
- retained_lane: read-only evidence still reports browser PID 1046742, CDP
  endpoint
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`,
  ready health, profile `chatgpt-pro`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, exact Workshop URL, and title
  `Architecture Review Boundaries`.
- live_runtime: no publication occurred. Installed and workspace reference
  SHA-256 remains
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`;
  the live dashboard returns HTTP 200 with dashboard SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, browser lifecycle, credential, attach, detach, daemon handoff,
  install, recovery, or live dashboard action occurred.
- next_action_or_stop_reason: P119 acceptance criteria are met. Next, expose the
  durable journal through a read-only status surface and add explicit
  recovery-only operation so automation and operators can inspect or reconcile
  an incomplete transaction without authorizing a new build.
