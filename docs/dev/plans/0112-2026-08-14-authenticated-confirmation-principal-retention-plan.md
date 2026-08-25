# Plan 0112 | Authenticated Confirmation Principal and Receipt Retention

State: CLOSED
Roadmap: P112
Plan version: 2
Date: 2026-08-14

## Objective

Bind durable task-authority request and decision identities to authenticated
HTTP or local MCP transport principals, and provide bounded, audited terminal
receipt cleanup without weakening restart or replay safety.

## Scope

- require an authenticated dashboard superuser for task-authority issue,
  reconcile, revoke, and confirmation HTTP mutations;
- derive the HTTP requester and decider from that authenticated session rather
  than dashboard form state;
- derive the MCP requester and decider from the OS-owned stdio transport rather
  than tool arguments;
- reject conflicting caller identity claims at either transport boundary;
- preview deterministic terminal-receipt cleanup with a review digest before
  an explicit apply;
- preserve pending records and confirmed, dispatched receipts whose execution
  outcome is indeterminate;
- keep confirmation IDs single-use even after bounded receipt cleanup.

## Non-Goals

- no browser launch, navigation, prompt submission, authenticated-site
  mutation, retained-target replacement, credential handling, or GitHub write.

## Acceptance Criteria

1. Unauthenticated HTTP mutations fail closed, and authenticated mutations
   persist the authenticated dashboard username as requester and decider.
2. MCP mutations persist a stable local stdio transport principal and reject a
   different structured identity claim.
3. Cleanup preview is non-mutating, deterministic, and bound to a digest that
   an explicit apply must match.
4. Cleanup never removes a pending record or a confirmed/dispatched
   indeterminate receipt, and removed confirmation IDs remain non-reusable.
5. Focused restart, transport identity, cleanup, HTTP/MCP parity, client,
   dashboard, widened Rust, build, install, and safe no-prompt checks pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the HTTP confirmation route currently relays before dashboard
  authentication, dashboard form state supplies `decidedBy`, and MCP tool
  arguments supply both requester and decider labels. Terminal receipts have
  no bounded cleanup path.
- discovery_status: CodeGraph is not initialized. Focused source, contract,
  test, and plan inspection is the active fallback.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited user direction to fix the next recorded
  agent-browser gap.
- next_action_or_stop_reason: bind transport principals, add tombstone-backed
  audited receipt cleanup, and verify the installed runtime without a browser
  mutation.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- implementation_evidence: authenticated HTTP and OS-owned MCP principals now
  replace caller-supplied actor labels; conflicting claims fail closed. Cleanup
  preview is deterministic and review-digest bound, retires IDs before receipt
  removal, and preserves pending plus dispatched-indeterminate evidence.
- regression_evidence: 23 focused task-authority tests, 11 dashboard-auth
  tests, 3 dashboard-gateway proxy tests, generated-client contract tests,
  dashboard authority tests, HTTP/MCP parity, client exports and types, and
  production-target clippy passed. One widened Rust run passed 1,850 tests with
  57 ignored. The final parallel run passed 1,847 tests and exposed three
  unrelated shared-temp-state flakes; each failed test passed in isolation
  with one test thread.
- build_evidence: release binary, dashboard production build, docs production
  build, task-authority schema parse, formatting, production-target strict
  clippy, and diff check passed. Strict all-target clippy remains blocked only
  by 11 pre-existing warnings in unrelated cumulative dirty test code.
- installed_evidence: installed and release binary SHA-256 are both
  `b8416f50b572a9c4a7e9640e5cdae7b97eb226b06961b9a6630cc425bd9e9774`.
  Installed authenticated HTTP preview returned the `admin` operator principal;
  installed MCP preview returned `mcp-stdio:uid:1000`. Both were non-mutating,
  successful previews with a 64-character review digest and zero candidates.
- browser_evidence: retained session `auracall-chatgpt-broker-v7` remained at
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2` with title
  `Architecture Review Boundaries`; verification performed no prompt, typing,
  click, navigation, or page mutation.
- residual_evidence: install doctor reports runtime convergence with zero stale
  runtimes. Its nonzero result is the pre-existing
  `workstation_payload_partial_or_drifted` condition, outside P112.
- authority_classification: inherited user direction to fix the recorded gap;
  no GitHub write was performed.
- next_action_or_stop_reason: P112 acceptance criteria are satisfied. Bound or
  checkpoint the compact retired-ID tombstone ledger and isolate shared test
  temp paths before the next retention increment.
