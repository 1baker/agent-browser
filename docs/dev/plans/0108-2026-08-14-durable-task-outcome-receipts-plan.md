# Plan 0108 | Durable Task Outcome Receipts

State: CLOSED
Roadmap: P108
Plan version: 2
Date: 2026-08-14

## Objective

Finalize every admitted v2 broker step with durable completed or failed
evidence before publishing its response, while projecting any crash-stranded
admission as indeterminate and never replaying that consumed step.

## Scope

- persist a terminal state, response digest and byte count, post-action target
  and URL, and finalization time on the existing command-bound receipt;
- make identical finalization idempotent and reject conflicting outcomes;
- fail closed before response publication when terminal evidence cannot be
  persisted;
- classify admitted-only receipts as indeterminate after restart while keeping
  the ordered cursor advanced;
- expose admitted, completed, failed, and indeterminate views and counts through
  authority status across HTTP, MCP, and generated client status helpers;
- extend the disposable public smoke with both a successful and deterministic
  failed read plus a real daemon handoff and replay rejection.

## Non-Goals

- no automatic retry or replay, inferred success after a crash, prompt
  submission, authenticated-site work, page mutation, credential work, or
  GitHub write;
- no change to the retained ChatGPT target or its browser/profile identity.

## Acceptance Criteria

1. Successful and failed command responses are hashed and persisted before
   publication, and status distinguishes both terminal classes.
2. A crash after admission but before finalization is reported as
   indeterminate after restart; the same step and command remain rejected.
3. Identical finalization is idempotent, conflicting finalization and ledger
   tampering fail closed, and a finalization write failure replaces the
   otherwise publishable response with an error.
4. Every post-admission exit, including launch/recovery/profile/backend errors,
   crosses the same terminal finalizer.
5. Focused and widened tests, builds, installed runtime parity, a disposable
   public read-only live proof, and retained-lane identity verification pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P107 durably consumes ordered steps before dispatch but retains no
  terminal result, so restart status cannot distinguish success, action failure,
  and a crash between admission and response publication.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to implement the best next
  agentic-browser recommendation; P107 explicitly names durable terminal
  outcome receipts as its next priority.
- next_action_or_stop_reason: complete the common finalizer, public status,
  regressions, documentation, install, and safe public live proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: every ordered post-admission exit now crosses one idempotent
  terminal finalizer before stream broadcast or response publication. Receipts
  persist completed/failed state, exact response SHA-256 and byte count,
  post-action target/URL evidence, and finalization time. Admitted-only receipts
  project as indeterminate and remain consumed.
- regression: successful, failed, idempotent, conflicting, missing-ledger,
  tampered-ledger, crash-after-admission/restart, replay, and finalization-write
  failure paths fail closed as designed. The full serial Rust suite passed
  1,841 tests with 57 ignored.
- validation: focused tests, formatting, strict production Clippy, optimized
  release build, API/MCP parity, no-launch service contracts, generated client
  contracts/types/examples, docs and dashboard production builds, JavaScript
  syntax, direct plan audit, repo/installed skill parity, and diff hygiene
  passed. MDX ESLint has no matching configuration and reported only its two
  existing ignored-file warnings; the docs production build typechecked both
  changed pages.
- live_proof: debug and installed disposable public proofs completed a read,
  handed the daemon off, rejected replay and out-of-order steps, durably
  classified a deterministic missing-selector read as failed, completed later
  reads once, exposed response digests, revoked, and cleaned up exactly. No
  authentication, page mutation, prompt, or retained profile was used.
- installed_runtime: installed and workspace reference executable SHA-256 is
  `58023942b0c1de84b2c38aef23b9dbc440796ab76576f0b67669fa585193e130`;
  optimized release SHA-256 is
  `396452aca9aaf0fe53d3b5bfa71d473e854abd0b2ca1c64f1e9943ea02c2a547`;
  live dashboard bundle SHA-256 is
  `881e5a5203d9971063caa278eba2d32103b4db4ce8cce7f0a95584e98599b634`.
- retained_lane: browser PID `1046742`, CDP port `39377`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title remained exact. No prompt was sent.
- environment_note: the publisher's generic browser smoke still creates a
  Linux-side disposable profile for the WSL Windows Chromium executable and
  exits with Chrome code 21 before page work. The P108 fixture uses a Windows-
  mounted disposable profile and passed. Because the user systemd bus is
  unavailable, only the verified deleted-executable dashboard listener was
  replaced; manifest/marker smoke then passed without launching another
  browser. Install doctor remains partial for
  `workstation_payload_partial_or_drifted`; remote-view doctor separately
  reports existing workstation, Guacamole route/schema/login, route-display,
  group, helper, membership, and sudoers blockers.
- plan_audit: no `plans:audit` script exists; plan, roadmap, runbook, note,
  contract, user docs, README, and repository/installed skill were checked
  directly.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P108.
- next_action_or_stop_reason: add an explicit broker reconciliation/replan
  operation that links an indeterminate receipt to a freshly confirmed
  replacement authority without inferring success or replaying the old step.
