# Task Authority Dashboard Recovery

P110 makes durable authority evidence usable without reconstructing broker
commands from raw ledger JSON. The selected-session Service dashboard now shows
authority state, caller, exact target and URL, approval evidence, immutable
plan progress, terminal and indeterminate outcome counts, reconciliation
state, and predecessor/replacement lineage.

Recovery stays fail closed. It requires exactly one indeterminate receipt and
exactly one nonclosed live tab with the stored target ID and current URL. The
suggested replacement contains only unconsumed planned steps and never copies
step IDs or command IDs. The operator can edit and review the full bounded
replacement plan before staging.

Staging does not approve anything. It returns one confirmation ID plus the
exact target and URL. The dashboard then requires a separate confirmation or
denial routed to the same daemon session with `expectedAction` set to
`task_authority_reconcile`. A mismatched action cannot approve another pending
control, and closing a staged recovery denies it.

The confirmation surface is available through HTTP
`POST /api/service/task-authorities/confirmation`, MCP
`service_task_authority_confirmation`, and the generated service client. The
request binds `sessionName`, `confirmationId`, `expectedAction`, and a decision
of `confirm` or `deny`.

## Isolation Repair

The rendered smoke exposed that `AGENT_BROWSER_DASHBOARD_AUTH_FILE` relocated
only the JSON store while bootstrap credentials still used the OS account's
default directory. The auth path contract now places the bootstrap credential
and generated `.env` beside an explicitly configured store. The disposable
smoke initializes that path through the status endpoint, logs in locally, and
destroys the entire isolated home during cleanup.

## Live Proof

Debug and installed smokes used only `https://example.com/`. They created one
durable indeterminate read receipt, reconciled it once against the exact live
target, rejected predecessor replay, executed one fresh read, staged and
denied revocation without changing authority, and rendered both predecessor
and replacement IDs in the Authorities workspace. Cleanup completed. No
authenticated profile, prompt submission, or page mutation was used.

The retained Workshop lane stayed at browser PID `1046742`, CDP port `39377`,
profile `chatgpt-pro`, session `auracall-chatgpt-broker-v7`, target
`B0EC77F279E5434E33FEA97AB1742B1A`, the pinned canonical conversation URL, and
the `Architecture Review Boundaries` title. No prompt was sent.

Installed, release, and workspace reference executable SHA-256 is
`66fcca318e238ee6bf027ffd8ad8a38676c2e237acf59dad23465ee764b6f258`.
The live dashboard bundle SHA-256 is
`61465d64813a55daf57701fc214a73abcc6df57d386f4cf16d9fbab3a7622534`.

## Validation

- focused dashboard eligibility, parser, accessibility-oriented source,
  client, HTTP, MCP, contract, confirmation-isolation, and auth-path tests;
- serial Rust suite: 1,846 passed, 57 ignored;
- Rust formatting, strict Clippy, optimized build, API/MCP parity, no-launch
  contracts, generated client checks, docs/dashboard builds, and JavaScript
  syntax;
- debug and installed disposable rendered-page smokes, runtime manifest
  readback, repository/installed skill parity, validation selection, direct
  planning audit, retained-lane identity proof, and diff hygiene.

CodeGraph was not initialized and the repository-requested Graphiti discovery
skill was unavailable, so discovery used focused source, contract, policy,
plan, test, and live-state reads. Those tooling gaps did not affect runtime
verification.
