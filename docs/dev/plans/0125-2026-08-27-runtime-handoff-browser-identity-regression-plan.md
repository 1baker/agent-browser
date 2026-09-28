# Plan 0125 | Runtime Handoff Browser Identity Regression

State: CLOSED
Roadmap: P125
Plan version: 1
Date: 2026-08-27

## Objective

Replace source-pattern-only coverage of already-resumed runtime handoffs with a
behaviorally tested browser selector, authoritative process-liveness gate, and
verified idempotent retry-record cleanup.

## Frozen Acceptance Criteria

1. Recovery selects the exact daemon session browser when its persisted PID and
   CDP endpoint match the prepared handoff.
2. A bridge alias with a missing attached-existing PID is accepted only when
   its CDP endpoint matches exactly and the prepared browser PID remains live.
3. A known stale prepared browser PID cannot be treated as active merely
   because a CDP endpoint string remains in service state.
4. PID drift, CDP drift, duplicate session ownership, and multiple matching
   aliases fail closed.
5. An already-resumed handoff removes its durable retry record only after an
   exact session, PID, CDP endpoint, and runtime-profile readback. Cleanup is
   idempotent, while a changed record remains present for investigation.
6. Retained-browser preparation and publication share the same session-browser
   selection helper.
7. Focused fixtures, local convergence, publisher orchestration, lint,
   CodeGraph synchronization, validation selection, and diff hygiene pass.
8. No browser is opened, navigated, closed, or prompted during this slice.

## Execution Graph

1. Extract browser selection, liveness, and retry-record cleanup into a small
   dependency-injected helper.
2. Replace the publisher's inline selector and align retained-browser live
   discovery on the same helper.
3. Add behavioral fixtures and keep one lightweight production-wiring guard.
4. Run focused and widened validation, audit the final diff, and checkpoint
   only the Plan 0125 files.

## Delegation Receipt

- State: `not_spawned`
- Reason: one helper, its two direct consumers, and its fixtures form a small
  overlapping write surface.
- Runtime handle: none

## Evidence Log

- 2026-08-27: code inspection found that a known stale browser PID could pass
  the already-resumed check when a leftover CDP endpoint string was present.
- 2026-08-27: the existing alias regression was a regular-expression assertion
  over publisher source rather than executable behavior.
- 2026-08-27: the new behavioral fixture passes exact-session and bridge-alias
  selection, stale and closed process rejection, PID and endpoint mismatch,
  duplicate session and alias ambiguity, exact cleanup, idempotent cleanup, and
  mismatched-record preservation.
- 2026-08-27: local convergence, retained-browser live discovery, smoke policy,
  publisher lifecycle and orchestration, publication journal and operations,
  retained discovery and guard, route-confusion gates, lint, release-asset
  verification, CodeGraph synchronization, validation selection, and diff
  hygiene pass.
- 2026-08-27: the validation selector also recommended Rust gates because an
  unrelated `cli/src/mcp.rs` edit appeared during this turn. That edit is not
  part of Plan 0125 and remains untouched.

## Closeout

The publisher and retained-browser preparation now share one behaviorally
tested browser selector. Recovery requires a known prepared PID to remain live,
accepts exactly one matching PID/CDP alias when the service record omits PID,
and removes only an exact durable retry record. No live browser or installed
runtime mutation was performed.
