# Plan 0109 | Indeterminate Task Authority Reconciliation

State: CLOSED
Roadmap: P109
Plan version: 2
Date: 2026-08-14

## Objective

Replace the manual revoke-and-reissue recovery recipe with one confirmation-
gated broker transaction that preserves the exact indeterminate predecessor
step, revokes its stranded authority, and mints a fresh exact-target authority
without inferring success or making the consumed step replayable.

## Scope

- accept a stable caller reconciliation ID, exact predecessor authority and
  unresolved step IDs, exact current target and URL, issuer and approval
  evidence, bounded lifetime, and explicit replacement steps;
- require exactly one indeterminate receipt and require it to match the named
  predecessor step, command, target, immutable plan, and consumed cursor;
- bind predecessor authority, step, command, outcome state, and reconciliation
  ID into the replacement authority envelope and its SHA-256 identity;
- durably revoke the predecessor before publishing a replacement, resume an
  interrupted transaction idempotently, and reject conflicting retries;
- require fresh exact-target confirmation and fail closed if target or URL
  changes before confirmation;
- expose the transaction through HTTP, MCP, generated client helpers, contract
  metadata, status, user docs, and an isolated public live regression.

## Non-Goals

- no inference that the indeterminate action succeeded or failed, no replay of
  its step ID or command ID, no automatic browser action during reconciliation,
  no authenticated-site work, prompt submission, page mutation, credential
  work, retained ChatGPT target change, or GitHub write.

## Acceptance Criteria

1. Reconciliation rejects terminal, missing, ambiguous, unconsumed, tampered,
   wrong-session, wrong-target, wrong-URL, expired-confirmation, and conflicting
   predecessor evidence.
2. The predecessor is durably revoked before the replacement becomes
   publishable; a crash after that revocation resumes to the same replacement
   ID, while a different request under the same reconciliation ID fails closed.
3. The replacement envelope cryptographically binds complete predecessor
   lineage and receives fresh ordered step IDs; the old step and command remain
   consumed and rejected.
4. HTTP, MCP, generated client, schema, help, README, skill, docs, runbook,
   roadmap, note, and status surfaces agree on the operation.
5. Focused and widened tests, builds, installed-runtime parity, disposable
   public live proof, and retained no-prompt lane identity verification pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P108 durably classifies a crash-stranded admission as
  indeterminate and keeps it consumed, but recovery still requires three
  separate manual operations with no broker-bound lineage or crash-safe
  replacement transaction.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to implement the best next
  agentic-browser recommendation; P108 explicitly names broker reconciliation
  and replacement authority as its next priority.
- next_action_or_stop_reason: implement the atomic ledger transition, public
  surfaces, regressions, documentation, install, and safe public live proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: reconciliation validates exactly one named indeterminate receipt,
  durably revokes its predecessor with a pending transaction record, creates
  one deterministic replacement with complete predecessor lineage, and marks
  the transaction complete. Interrupted and identical requests resume
  idempotently; changed evidence fails closed.
- regression: crash-after-revocation, identical retry, conflicting retry,
  terminal predecessor, ambiguous predecessor, target drift, old-command
  replay, and fresh replacement-step identity paths passed. The serial Rust
  suite passed 1,844 tests with 57 ignored.
- validation: focused tests, formatting, strict production Clippy, optimized
  release build, API/MCP parity, no-launch service contracts, generated client
  contracts and helpers, docs and dashboard production builds, JavaScript
  checks, direct plan audit, repository/installed skill parity, installed live
  smoke, retained-lane readback, and diff hygiene passed.
- live_proof: the installed disposable public smoke deliberately stranded a
  read-only wait after durable admission, observed the indeterminate receipt,
  required exact-target confirmation, revoked before replacement, bound exact
  step/command lineage, rejected predecessor replay, executed one fresh read,
  and cleaned up. It used no authentication, page mutation, or prompt.
- retained_lane: browser PID `1046742`, CDP port `39377`, profile
  `chatgpt-pro`, session `auracall-chatgpt-broker-v7`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title remained exact. No prompt was sent.
- installed_runtime: installed and workspace reference executable SHA-256 is
  `e03bd1ca76409476fcfb9afa1c875118756c5b115e7a58b5414d019adbf6ec36`;
  live dashboard bundle SHA-256 is
  `3336d77c1a00965371f65389eff9c6d41d9687c2935d213b2f73da5aad6fb4df`.
  All live daemon listeners use the current executable, runtime convergence has
  zero stale daemons, and the retained-browser handoff preserved browser PID,
  CDP endpoint, profile, session, and target.
- environment_note: this host has no user-systemd bus, so the guarded publisher
  could not restart the unmanaged dashboard listener. The dashboard's own
  stop/start command replaced the deleted-executable listener and manifest
  readback then matched the installed binary. Install doctor remains partial
  only for the pre-existing `workstation_payload_partial_or_drifted` issue.
- discovery_note: CodeGraph was not initialized and the repository-requested
  Graphiti runtime command was unavailable; focused source, plan, contract, and
  test inspection supplied the required discovery evidence.
- plan_audit: no `plans:audit` package script exists; plan, roadmap, runbook,
  note, contract, help, README, user docs, and repository/installed skill were
  checked directly.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P109.
- next_action_or_stop_reason: project reconciliation lineage and pending state
  into the operator dashboard with a reviewed replacement-plan confirmation
  flow.
