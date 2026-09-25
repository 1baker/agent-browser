# Plan 0107 | Ordered Broker Task Plan

State: CLOSED
Roadmap: P107
Plan version: 2
Date: 2026-08-14

## Objective

Bind every v2 broker-issued task authority to one immutable ordered plan and a
durable step cursor so action multiplicity, sequence, exact URL state, and
per-step evidence reservations cannot be substituted after approval or reset
by daemon restart.

## Scope

- assign a unique broker step ID and zero-based index to every approved step;
- bind the envelope to the complete ordered plan hash;
- persist the next-step cursor and one admission receipt before command
  dispatch, including command ID, target, URLs, action, evidence, and time;
- require an exact `taskStepId`, action, pre-action URL, requested URL, and
  evidence reservation match for every v2 broker-issued command;
- reject missing, repeated, stale, out-of-order, wrong-action, wrong-URL, and
  wrong-budget steps without advancing the cursor;
- preserve v1 aggregate-envelope compatibility only outside required broker
  mode; replan explicitly means revoke the old authority and issue a newly
  confirmed immutable plan;
- expose ordered plan, next step, completed receipts, and remaining steps
  through status, HTTP, MCP, generated clients, docs, and live public proof.

## Non-Goals

- no in-place plan mutation, implicit replanning, prompt submission,
  authenticated-site mutation, credential work, or GitHub write;
- no change to the retained ChatGPT target or its browser/profile identity.

## Acceptance Criteria

1. Two occurrences of the same action have distinct step IDs and cannot be
   exchanged or replayed.
2. The daemon durably advances the cursor before dispatch, and a fresh process
   rejects the already-admitted step or command instead of executing it again.
3. Wrong order, action, current URL, requested URL, evidence reservation,
   target, or plan hash fails closed and leaves the cursor unchanged.
4. Status returns the immutable ordered plan, current next step, completed
   admission receipts, remaining steps, and aggregate compatibility totals.
5. Focused and widened tests, builds, installed runtime parity, a disposable
   public read-only live proof, and retained-lane identity verification pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P106 persists only unique allowed actions and aggregate budgets;
  its ledger stores totals rather than ordered steps. An approved sequence can
  therefore be replaced by repeated use of another allowed action.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to implement the best next
  agentic-browser recommendation; P106 explicitly names the ordered cursor and
  per-step receipts as its next priority.
- next_action_or_stop_reason: implement the v2 plan binding, cursor, receipts,
  public contracts, regressions, and safe read-only live proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: new broker issuances persist the complete ordered plan, unique
  step IDs, exact normalized pre-action/requested URLs, fixed evidence, and a
  plan hash bound into the envelope. A validated v2 ledger persists the cursor
  and command-bound receipts atomically before dispatch.
- regression: missing, stale, repeated, out-of-order, wrong-action, wrong-URL,
  wrong-evidence, plan-hash, target, and receipt-ledger drift fail closed. HTTP,
  MCP `service_request`, and generated clients preserve all authority fields.
- validation: focused authority/MCP/HTTP/client/contract tests passed; the full
  serial Rust suite passed 1,837 tests with 57 ignored. Formatting, strict
  production Clippy, release build, service contract smoke, API/MCP parity,
  client types/contracts, docs and dashboard production builds, JavaScript
  syntax, direct planning-surface audit, and diff hygiene passed.
- live_proof: debug and installed disposable public proofs ran read-only
  `title -> url -> title`, rejected an out-of-order duplicate action, handed
  the daemon off after step 1, rejected the admitted command after restart,
  completed each remaining step once, and cleaned up exactly. No login, page
  mutation, authenticated profile, or prompt was used.
- installed_runtime: installed, workspace reference, retained daemons, and the
  live dashboard manifest converge on executable SHA-256
  `e760f6ad07012d9fc083a77b790f5f2aa2ee4d320e8b38f9c47815cf74311220`;
  dashboard bundle SHA-256 is
  `b4322cfab3888e1ce9f32aecc37229674641b644f7217d38398ed247df84eb82`.
- retained_lane: browser PID `1046742`, profile `chatgpt-pro`, CDP endpoint,
  target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title remained exact through handoff.
- environment_note: generic `test:service-request-live` still fails before page
  work because its disposable auto-launch exits with Chrome code 21 even
  though the fixture configures `--no-sandbox`. The P107-specific debug and
  installed live proofs pass. The unavailable user systemd bus also required
  replacing only the exact deleted-executable dashboard listener. Doctor is
  partial only for the existing workstation-payload provenance and privileged
  remote-view installation boundaries.
- plan_audit: no `plans:audit` script exists; plan, roadmap, runbook, note,
  contracts, user docs, README, and repo/installed skill were checked directly.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P107.
- next_action_or_stop_reason: add durable terminal outcome receipts so status
  distinguishes admitted, completed, failed, and indeterminate steps without
  ever replaying a consumed step automatically.
