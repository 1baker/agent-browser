# Authenticated Confirmation Principal and Receipt Retention

Date: 2026-08-14
Roadmap: P112
Plan: `docs/dev/plans/0112-2026-08-14-authenticated-confirmation-principal-retention-plan.md`

## Outcome

Task-authority HTTP control mutations now require an authenticated dashboard
superuser and use that session username as requester and decider evidence. MCP
uses a stable OS-owned stdio transport principal. Conflicting caller identity
claims fail before daemon relay.

Terminal confirmation receipts now have a deterministic preview-first cleanup
path. Apply requires the exact review digest, writes retired IDs before receipt
removal, preserves pending and confirmed/dispatched indeterminate evidence, and
does not make old confirmation IDs reusable.

## Verification

Focused authority, HTTP, MCP, generated-client, and dashboard tests passed. The
widened Rust suite passed 1,850 tests with 57 ignored. A final parallel run
exposed three unrelated shared-temp-state flakes; all three passed when rerun
individually with one test thread. Formatting, production-target strict clippy,
release, dashboard and docs builds, parity, client, schema, install, and diff
checks passed.

The installed binary exactly matches the release artifact at SHA-256
`b8416f50b572a9c4a7e9640e5cdae7b97eb226b06961b9a6630cc425bd9e9774`.
Installed HTTP and MCP cleanup previews were successful and non-mutating. The
retained ChatGPT session remained on the exact expected conversation and was
read back without prompt submission or any page mutation. Install doctor
reported converged runtime state with zero stale runtimes; its remaining
nonzero condition is the pre-existing workstation payload drift.
