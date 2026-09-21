# Disposable crash recovery test correction

## Follow-up result

The candidate fix and corrected fixture passed the bounded daemon-crash test;
see the 2026-09-12 owned-Chrome reuse section in RUNBOOK.md for hashes, exact
process/target identity, checks and limitations. The blocker description below
is retained as test history, not the current candidate result. Production has
not been updated. Reviewer `crash_path_review` completed read-only source review;
the primary independently verified the result and executed live validation.

## Earlier test history

Scope: test installed CLI recovery without touching retained Workshop input or
the deferred MCP default-daemon setup issue. Existing dirty changes preserved.

Installed binary SHA-256:
`3772ea17f4bc27655e196a93ce2d4cebdfd48acb78691be9e48f9cf447b4eeed`.

The earlier SIGKILL test invoked same-session `handoff resume` without prepare.
That command requires a prepared descriptor in `actions.rs`; its refusal does
not establish failure of the ordinary retained-session auto-launch reconnect
path. Do not report that result as proof that all crash recovery is broken.

The corrective disposable `recovery-qa` run opened the loopback fixture, but
the first click was refused before effects: requested `stock_chrome`, retained
build `unknown`. The service browser row had `browserBuild: null` and
`browserBuildProof.applied: false`, reason `explicit_executable_path`.
Configuration selected the installed Chrome 153 executable explicitly and the
wrapper passed stock-build and local-headless flags. No crash was injected in
this corrective run. No build guard was weakened or metadata fabricated.

The exact disposable browser closed successfully. The retained-browser status
check returned `verified: true`, without launch. No ChatGPT submission,
runtime installation, GitHub write, or production code modification occurred.

Next: reconcile explicit-executable launch proof before repeating the ordinary
read-command crash test. Acceptance requires unchanged browser PID, exact target
and URL, readable committed result, and backend submission count one. A read
success alone is insufficient; current generic reconnect uses `connect_cdp`
and needs target identity verification. Full host/browser crash, production
broker recovery, and in-flight uncertain-effect replay remain unproven.

The loopback fixture and command wrapper are intentionally ephemeral under
`/tmp/agent-browser-transfer-qa-lCR5YT`; this note preserves outcome and installed
identity, not a reusable regression suite. No new automated regression passed.
