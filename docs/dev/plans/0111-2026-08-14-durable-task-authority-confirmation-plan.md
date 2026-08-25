# Plan 0111 | Durable Task Authority Confirmation

State: CLOSED
Roadmap: P111
Plan version: 2
Date: 2026-08-14

## Objective

Persist task-authority control confirmation intent and decision receipts across
daemon restart without permitting a confirmed action to replay automatically.

## Scope

- persist only `task_authority_issue`, `task_authority_reconcile`, and
  `task_authority_revoke` confirmation commands in a private session ledger;
- bind each pending record to its exact session, action, target, URL, request
  digest, requester, creation time, and expiry;
- require an explicit decision actor and exact confirmation ID and action;
- archive the decision before dispatch and record the terminal result digest;
- treat a restart after decision commit as indeterminate and never resume,
  confirm, or restage it automatically;
- expose redacted pending and terminal confirmation status beside durable task
  authority status in HTTP, MCP, client, and dashboard views;
- retain existing in-memory confirmation behavior for all other actions so
  commands that may contain credentials or page data are never persisted.

## Non-Goals

- no prompt submission, authenticated-site mutation, automatic decision,
  automatic task replay, credential persistence, retained-target replacement,
  or GitHub write.

## Acceptance Criteria

1. A pending task-authority confirmation survives daemon restart and can be
   decided once only by an explicitly identified operator.
2. Session, confirmation ID, expected action, target ID, URL, request digest,
   or expiry drift consumes the pending record fail closed and executes no
   authority mutation.
3. The decision receipt is durable before dispatch. A simulated crash after
   that commit exposes an indeterminate execution state and a retry cannot
   execute or restage the command.
4. Public status omits the persisted command while showing requester, decision
   actor, target binding, request digest, expiry, decision, and execution state.
5. Focused restart and mismatch regressions, widened Rust and client/dashboard
   gates, installed-runtime checks, and a safe no-prompt retained-target check
   pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P110 exposes exact-session confirmation controls, but
  `DaemonState.pending_confirmation` is process memory and contains neither a
  durable request receipt nor a durable decision receipt.
- discovery_status: CodeGraph is not initialized and the repo-requested
  Graphiti discovery skill is unavailable. Focused source, test, contract,
  plan, and live-state reads are the active fallback.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited user direction to fix the next recorded
  agent-browser gap.
- next_action_or_stop_reason: implement private single-use confirmation
  storage, exact decision binding, redacted status, and restart regressions.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- implementation: task-authority issue, reconcile, and revoke confirmations
  now persist privately under the authority ledger. Each record binds the exact
  session, action, target, URL, request digest, requester, creation time, and
  expiry. Generic browser confirmations remain memory-only.
- single_use_boundary: confirmation and denial require an exact ID, expected
  action, and `decidedBy` identity matching the staged requester. The decision
  receipt is archived before dispatch. A crash after that commit projects an
  indeterminate execution state, and neither decision retry nor command retry
  can replay or restage the record.
- status: no-launch authority status includes redacted pending and terminal
  confirmation receipts without the persisted command. The dashboard shows
  pending, denied, expired, invalidated, indeterminate, failed, and completed
  execution evidence.
- regression: restart, decision-commit crash, finalization, ID, action,
  decision-actor, target, URL, session, expiry, request-digest, redaction, and
  competing-confirmation tests passed. The installed disposable public smoke
  proved pending restart preservation, zero restart execution, one decision,
  and replay rejection.
- validation: 1,849 Rust tests passed with 57 ignored. Formatting, strict
  Clippy, optimized release build, HTTP/MCP parity, generated client and type
  checks, dashboard tests, dashboard and docs production builds, contract JSON,
  JavaScript syntax, direct plan audit, installed-skill parity, validation
  selection, and diff hygiene passed.
- installed_runtime: installed, release, and workspace reference executable
  SHA-256 is
  `aef835d1fddcd9e5cb469901edb7667e531f38fd707d661fdff0ecbe418e85a8`;
  live dashboard bundle SHA-256 is
  `c79ef6e6e503792444229ce7d73fd52ca3bb5094ec6223ff67b30f65f408e741`.
- install_note: the publisher preserved the retained browser and CDP endpoint,
  but its marker check initially reached the pre-existing unmanaged dashboard
  listener because this shell has no user systemd bus. The exact listener was
  stopped and relaunched detached with the installed binary; manifest and
  marker readback then passed.
- retained_lane: browser PID `1046742`, profile `chatgpt-pro`, session
  `auracall-chatgpt-broker-v7`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title remained exact. No prompt, typing,
  click, navigation, or page mutation was sent.
- doctor: runtime convergence is ready with zero stale runtimes. The only
  top-level issue remains the pre-existing
  `workstation_payload_partial_or_drifted` condition.
- discovery_note: CodeGraph was not initialized and the repo-requested
  Graphiti discovery skill was unavailable; focused source, contract, policy,
  plan, test, and live-state reads supplied discovery evidence.
- plan_audit: no `plans:audit` package script exists; plan, roadmap, runbook,
  note, contract, help, README, user docs, repository/installed skill, and patch
  checks were audited directly.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P111.
- next_action_or_stop_reason: bind the recorded requester and decider labels to
  authenticated dashboard/MCP transport principals and add bounded receipt
  retention without weakening single-use restart behavior.
