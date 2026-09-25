# Indeterminate Task Authority Reconciliation

An admitted browser step can become indeterminate when execution crosses the
durable admission boundary but the daemon cannot persist a terminal response.
The step must remain consumed, but manual revoke and reissue left the causal
relationship between that receipt and a replacement plan outside the broker.

P109 adds one confirmation-gated reconciliation transaction. It accepts the
exact predecessor authority and unresolved step, a stable caller
`reconciliationId`, the current target and canonical URL, issuer and approval
evidence, and an explicit replacement plan. The broker verifies that exactly
one receipt is indeterminate, durably records a pending reconciliation while
revoking the predecessor, mints a deterministic replacement authority with
full predecessor step and command lineage, and then marks reconciliation
complete.

An interrupted transaction resumes to the same replacement identity. An
identical completed retry is idempotent. Changed request evidence, a terminal
or ambiguous predecessor, target or URL drift, ledger tampering, and reuse of
the reconciliation ID for unrelated lineage fail closed. The consumed step and
command are never replayable; replacement steps always receive fresh IDs.

The operation is available through HTTP
`POST /api/service/task-authorities/<id>/reconcile`, MCP
`service_task_authority_reconcile`, and the generated service client. It does
not launch a browser and always requires fresh exact-target confirmation.

## Live Proof

The disposable public smoke used `https://example.com/` and a read-only bounded
wait. It deliberately made the outcome ledger path unwritable after durable
admission, observed the indeterminate receipt, restored the ledger, reconciled
through the installed HTTP surface, rejected predecessor replay, and executed
one fresh replacement read exactly once. It used no authenticated profile,
page mutation, prompt submission, or retained ChatGPT target.

The retained Workshop lane remained browser PID `1046742`, CDP port `39377`,
profile `chatgpt-pro`, session `auracall-chatgpt-broker-v7`, target
`B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
`Architecture Review Boundaries` title. No prompt was sent.

The installed and reference executable SHA-256 is
`e03bd1ca76409476fcfb9afa1c875118756c5b115e7a58b5414d019adbf6ec36`.
The converged live dashboard bundle SHA-256 is
`3336d77c1a00965371f65389eff9c6d41d9687c2935d213b2f73da5aad6fb4df`.

## Validation

- focused reconciliation, authority, action, HTTP, MCP, policy, and client
  regressions;
- serial Rust suite: 1,844 passed, 57 ignored;
- Rust formatting, strict Clippy, optimized build, API/MCP parity, no-launch
  service contracts, generated client checks, docs and dashboard builds;
- debug and installed disposable public reconciliation smoke;
- repository and installed skill parity, direct planning audit, runtime
  manifest readback, retained-lane identity proof, and diff hygiene.

CodeGraph was not initialized in this checkout, and the repository-requested
Graphiti discovery command was unavailable, so discovery used focused source,
contract, plan, and test reads. Those tooling gaps did not affect runtime
verification.
