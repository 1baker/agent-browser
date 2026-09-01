# Ordered Broker Task Plan

Date: 2026-08-14
Plan: P107

## Finding

P106 correctly made task authority broker-issued, target-bound, revocable, and
durable, but issuance reduced the approved plan to a unique action set plus
aggregate action and evidence totals. The ledger also retained only aggregate
usage. That representation could not distinguish `title` step 1 from `title`
step 3 and could not prove which approved step a crash had consumed.

## Decision

New broker issuances use `agent-browser.task-authority-issuance.v2`. The broker
assigns every step an ID and index, resolves its exact expected pre-action URL
and optional requested URL, fixes its evidence reservation, hashes the entire
ordered step array, and includes that hash in the immutable envelope.

Every command must provide the broker-assigned `taskStepId`. Admission matches
the current cursor, action, target, exact normalized URL state, requested URL,
and evidence reservation. Before dispatch it atomically writes a v2 ledger
receipt containing the step and command identities. A restart therefore sees
the advanced cursor and cannot replay the admitted command.

Replanning never mutates an issued record. The explicit path is to revoke the
old authority, obtain fresh exact-target confirmation, and issue a new plan.
Legacy v1 caller envelopes remain compatible only where broker-required mode
is disabled.

## Verification

- focused Rust authority, HTTP, and MCP regressions passed;
- generated client request, contract, type, and API/MCP parity tests passed;
- the serial Rust suite passed 1,837 tests with 57 ignored;
- strict production Clippy, release build, service-contract smoke, docs and
  dashboard builds, JavaScript syntax, and diff hygiene passed;
- debug and installed disposable public smokes proved ordered multiplicity,
  out-of-order rejection, a real daemon handoff, replay rejection after
  restart, durable receipts, exact cleanup, and no page mutation or prompt;
- installed/reference/live-dashboard executable SHA-256 is
  `e760f6ad07012d9fc083a77b790f5f2aa2ee4d320e8b38f9c47815cf74311220`.

The broader generic service-request live smoke still exits before page work
with the existing disposable Chrome code 21 sandbox failure even though that
fixture configures `--no-sandbox`; it is not the P107 ordered-authority path.
