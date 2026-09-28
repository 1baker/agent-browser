# Plan 0106 | Broker Task Authority Issuer

State: CLOSED
Roadmap: P106
Plan version: 2
Date: 2026-08-14

## Objective

Turn P105's caller-supplied authority envelope into a broker-issued,
revocable, inspectable capability. The broker must derive the smallest action,
origin, evidence, target, and lifetime scope from an approved plan and expose
durable issuer, revocation, and usage evidence before authenticated mutation
work is considered.

## Scope

- issue an authority only for the exact live retained target and caller labels;
- derive the action allowlist, origin set, action count, and evidence budget
  from explicit bounded plan steps rather than accepting caller-computed totals;
- require fresh exact-target confirmation for issue and revoke operations;
- persist issuer identity, approval reference, immutable envelope hash,
  approved-plan summary, issuance time, and revocation evidence atomically;
- require broker issuance in task-authority required mode and reject revoked or
  altered records before browser command dispatch;
- expose read-only authority status and usage receipts through daemon, HTTP,
  MCP, and generated client surfaces;
- prove the flow on a disposable public retained target without login,
  external mutation, file transfer, or prompt submission.

## Non-Goals

- no authenticated mutation workflow, remote policy distribution, credential
  storage, or cryptographic multi-party signature system;
- no ChatGPT prompt or composer action;
- no GitHub write, release publication, privileged workstation installation,
  or unrelated dirty-worktree reconciliation.

## Acceptance Criteria

1. Issuance derives one immutable envelope from the exact active target and
   explicit plan steps; callers cannot widen actions, origins, budgets, target,
   or expiry after issuance.
2. Required mode accepts only a matching active broker record and rejects
   caller-fabricated, revoked, expired, drifted, wrong-target, and over-budget
   authority before execution.
3. Issue and revoke stop at target-bound confirmation; denial changes no
   authority state and a changed target invalidates approval.
4. Read-only status reports issuer, approval reference, plan summary, state,
   budget usage, remaining allowance, expiry, and revocation evidence without
   launching or mutating a browser.
5. HTTP, MCP, service contracts, generated client helpers, docs, focused tests,
   widened Rust/client/docs gates, release build, installed-runtime parity, and
   disposable public live proof pass while preserving the retained ChatGPT
   target exactly.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P105 validates and accounts for immutable envelopes, but any
  caller can still construct one directly. There is no broker issuance record,
  action allowlist, issuer or approval provenance, revocation state, or
  read-only usage/status contract.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to continue the best agentic-browser
  recommendations; P105 explicitly names broker-issued minimal authority as
  the next priority.
- next_action_or_stop_reason: implement issuance and status in the existing
  task-authority module, then wire confirmation-gated control surfaces and
  provider-free proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: issue/revoke are first-class exact-target confirmation-gated broker
  operations; status is no-launch/read-only. Issuance derives action/origin
  scope and exact budgets from bounded plan steps and persists issuer, approval,
  plan, hash, timestamps, and revocation evidence atomically.
- regression: focused authority, HTTP, MCP schema, caller/profile-lease,
  generated client, contract parity, and service-contract tests pass. The
  serial suite passed 1,834 tests with 57 ignored and zero failures.
- live_proof: debug and installed disposable Example Domain proofs issued one
  read-only title capability, read status without target change, debited and
  exhausted its budget, and revoked it after fresh confirmation. Both cleaned
  up exactly; no authenticated profile, prompt, composer action, or page
  mutation was used.
- installed_runtime: installed, reference, and live-dashboard hashes are
  `86380304f45f9d8c6affc3c9caaffe85bbae26aaa18dbf260eae0d8b05cf7868`.
  Handoff preserved browser PID `1046742`, the CDP endpoint, exact retained
  target, canonical conversation URL, and title.
- validation: Rust formatting, strict production Clippy, client contract/types
  and examples, API/MCP parity, docs/dashboard production builds, release
  build, validation selection, JavaScript syntax, targeted MDX lint, and diff
  hygiene passed. All-target Clippy reports ten unrelated pre-existing
  test-style warnings in preserved dirty work; no `plans:audit` script exists.
- environment_note: the missing user systemd bus left the old dashboard
  listener active after successful handoff. Replacing only that exact
  deleted-executable listener reconciled the live manifest. Doctor remains
  partial only for separate workstation-payload provenance and privileged
  remote-view installation gates.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P106.
- next_action_or_stop_reason: continue the broader agentic-browser roadmap with
  an ordered broker plan cursor and per-step receipts so allowed-action
  multiplicity and sequence cannot drift.
