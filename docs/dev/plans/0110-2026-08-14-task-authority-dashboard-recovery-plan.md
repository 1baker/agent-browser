# Plan 0110 | Task Authority Dashboard Recovery

State: CLOSED
Roadmap: P110
Plan version: 2
Date: 2026-08-14

## Objective

Make durable task-authority status and indeterminate-receipt recovery legible
and operable from the Service dashboard without requiring an operator to read
raw ledger files or reconstruct a confirmation command.

## Scope

- add an Authorities workspace backed by the selected daemon session's
  no-launch authority collection;
- show authority state, caller, target, plan progress, outcome counts,
  predecessor/replacement lineage, and pending/completed reconciliation state;
- derive a replacement preview only from unconsumed planned steps, never from
  the indeterminate consumed step;
- require exactly one indeterminate receipt and one exact live target/URL match
  before enabling a recovery request;
- stage reconciliation first, display its exact target-bound confirmation
  evidence, and require a separate operator confirmation or denial;
- expose confirmation decisions through HTTP, MCP, generated client metadata,
  docs, and focused no-launch regressions;
- install and verify the dashboard with disposable public authority evidence
  while preserving the retained Workshop target without sending a prompt.

## Non-Goals

- no automatic inference or replay of an indeterminate step, automatic
  confirmation, authenticated-site action, page mutation, prompt submission,
  real-account credential work, retained target change, or GitHub write. A
  generated disposable local dashboard bootstrap credential may be used only
  inside the isolated rendered-page smoke and is destroyed with that fixture.

## Acceptance Criteria

1. The Authorities tab renders empty, loading, error, active, exhausted,
   revoked, terminal-outcome, indeterminate, pending reconciliation, completed
   reconciliation, and replacement-lineage states without relying on color.
2. Recovery remains disabled unless the selected session has exactly one
   indeterminate receipt and exactly one retained tab matches the authority's
   target ID with a nonempty current URL.
3. The proposed replacement excludes every consumed step, assigns no step IDs,
   validates action/URL/evidence fields, and is fully visible before staging.
4. Staging returns one exact confirmation ID and target binding. Confirmation
   and denial route to the same exact session, and closing a staged dialog
   denies rather than leaving an undisclosed pending action.
5. Focused dashboard, HTTP, MCP, generated-client, accessibility-oriented
   source, widened build/test, installed runtime, disposable public live, and
   retained no-prompt verification pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P109 provides durable no-launch status and crash-safe
  reconciliation, but the dashboard has no authority collection, lineage
  view, replacement preview, or exact-session confirmation control.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- discovery_status: CodeGraph is not initialized and the repo-requested
  Graphiti discovery skill is unavailable, so focused source, test, contract,
  plan, and live-state reads are the active fallback.
- authority_classification: inherited goal to continue the best recorded
  recommendation; P109 names dashboard lineage and reviewed replacement-plan
  confirmation as its next action.
- next_action_or_stop_reason: implement the pure authority projection, Service
  workspace, exact-session confirmation surface, regressions, docs, install,
  and safe live proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: the selected-session Authorities workspace renders authority
  state, exact target, immutable plan progress, outcome counts, pending and
  completed reconciliation, and complete predecessor/replacement lineage.
  Recovery derives only unconsumed steps and remains disabled unless exactly
  one indeterminate receipt and one exact live target match.
- confirmation: staging returns an exact target-bound confirmation preview;
  HTTP and MCP decisions require the same session, confirmation ID, and
  expected task-authority action. Closing a staged flow denies it, and an
  action mismatch consumes the pending confirmation fail closed.
- auth_isolation: an explicit `AGENT_BROWSER_DASHBOARD_AUTH_FILE` now relocates
  the generated bootstrap credential and `.env` beside the store. The rendered
  smoke initializes and uses only that disposable credential directory.
- regression: pure dashboard eligibility/parser/source tests, generated-client
  tests, HTTP and MCP routing, expected-action isolation, relocated-auth path,
  exact collection readback, denial preservation, and rendered lineage passed.
  The serial Rust suite passed 1,846 tests with 57 ignored.
- validation: focused gates, formatting, strict Clippy, optimized release
  build, API/MCP parity, no-launch contracts, generated client checks, docs and
  dashboard production builds, JavaScript syntax, validation selection, direct
  plan audit, repository/installed skill parity, and diff hygiene passed. The
  targeted MDX lint command produced only the repository's no-matcher warnings;
  the docs production build supplied the parser/type gate.
- live_proof: debug and installed disposable public smokes created one
  indeterminate read receipt, reconciled it once against the exact target,
  rejected predecessor replay, denied a staged revoke without changing the
  authority, rendered predecessor and replacement IDs in the dashboard, and
  cleaned up. They used no authenticated profile, page mutation, or prompt.
- installed_runtime: installed, release, and workspace reference executable
  SHA-256 is
  `66fcca318e238ee6bf027ffd8ad8a38676c2e237acf59dad23465ee764b6f258`;
  live dashboard bundle SHA-256 is
  `61465d64813a55daf57701fc214a73abcc6df57d386f4cf16d9fbab3a7622534`.
  The unmanaged dashboard listener required one documented stop/start after
  publication; manifest readback then matched the installed executable.
- retained_lane: browser PID `1046742`, CDP port `39377`, profile
  `chatgpt-pro`, session `auracall-chatgpt-broker-v7`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title remained exact. No prompt was sent.
- doctor: runtime inventory is converged with zero stale runtimes and no
  deleted or stale daemon listeners. The only install issue remains the
  pre-existing `workstation_payload_partial_or_drifted` condition.
- discovery_note: CodeGraph was not initialized and the repo-requested
  Graphiti discovery skill was unavailable; focused source, contract, policy,
  plan, test, and live-state reads supplied discovery evidence.
- plan_audit: no `plans:audit` package script exists; plan, roadmap, runbook,
  note, contract, help, README, user docs, and repository/installed skill were
  checked directly.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P110.
- next_action_or_stop_reason: persist single-use pending confirmation intent
  and decision receipts across daemon restart, bound to exact operator,
  session, action, target, URL, request digest, and expiry.
