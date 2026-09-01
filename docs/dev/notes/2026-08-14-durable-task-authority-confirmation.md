# Durable Task Authority Confirmation

Date: 2026-08-14
Roadmap: P111
Plan: `docs/dev/plans/0111-2026-08-14-durable-task-authority-confirmation-plan.md`

## Outcome

Task-authority issue, reconcile, and revoke confirmation boundaries now survive
daemon restart without creating a replay path. Only these three reviewed
control commands are persisted. Generic confirmation commands remain in memory
so page values, credentials, and arbitrary browser command bodies are not added
to a durable ledger.

Each private pending record contains the exact session, action, target, URL,
request digest, requester, creation time, and expiry. A decision requires the
same confirmation ID, expected action, live target and URL for confirmation,
and a `decidedBy` principal exactly matching the staged requester. Any mismatch
is archived as invalidated and consumes the record.

The terminal decision receipt is written before dispatch. A process stop after
that commit therefore leaves an indeterminate receipt and no pending command.
Restart cannot execute, confirm, or restage it. Successful or failed dispatch
updates the same receipt with a result digest and terminal execution state.

## Operator Surface

No-launch task-authority status now includes a redacted confirmation collection
that never returns the stored command. HTTP, MCP, and the generated client
require `decidedBy`. The Authorities workspace renders pending, denied,
expired, invalidated, indeterminate, failed, and completed receipts with exact
target, requester, decider, and request-digest evidence.

## Verification

- focused ledger and daemon regressions covered restart, crash-after-commit,
  actor/session/action/target/URL/digest/expiry drift, redaction, and
  single-use finalization;
- 1,849 Rust tests passed with 57 ignored;
- formatting, strict Clippy, optimized release build, HTTP/MCP parity,
  generated client/type checks, dashboard tests, dashboard/docs builds,
  contract JSON, JavaScript syntax, installed-skill parity, and diff hygiene
  passed;
- debug and installed `example.com` smokes preserved a pending request through
  daemon handoff, observed zero authority before decision, completed one exact
  decision, and rejected replay;
- the installed executable SHA-256 is
  `aef835d1fddcd9e5cb469901edb7667e531f38fd707d661fdff0ecbe418e85a8`;
  the served dashboard bundle SHA-256 is
  `c79ef6e6e503792444229ce7d73fd52ca3bb5094ec6223ff67b30f65f408e741`;
- retained ChatGPT browser PID `1046742`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, conversation URL, and title remained
  exact. No prompt or page mutation was sent.

The only top-level install-doctor issue remains the pre-existing
`workstation_payload_partial_or_drifted` condition. Runtime convergence and the
live dashboard manifest are ready with zero stale runtimes.
