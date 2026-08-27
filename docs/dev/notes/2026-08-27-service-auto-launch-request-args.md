# Service auto-launch request arguments

## Objective

Repair the isolated CDP tab-streaming smoke without restoring ambient browser
argument inheritance across the service-owned launch boundary.

## Implementation

- Implicit service auto-launch now applies an explicit request `args` array to
  the effective launch options.
- Explicit request arguments replace ambient daemon arguments. An explicit
  empty array clears them.
- The CDP tab-streaming smoke no longer sets ambient `AGENT_BROWSER_ARGS`.
- The smoke selects the newest installed Agent Browser Chrome when neither an
  explicit executable nor the system Chrome fallback is available.
- Fixture coverage verifies numeric installed-browser version ordering and the
  Rust regression verifies request-scoped replacement and clearing.

## Validation

- `pnpm test:service-cdp-tab-streaming-live` passed with disposable session
  `cdp-tab-stream-1241009` and stream port `42885`.
- `pnpm test:service-cdp-tab-streaming-fixture` passed.
- The focused request-argument and CDP screencast Rust tests passed.
- The route-confusion no-launch gates passed.
- Rust formatting and clippy passed.
- `scripts/ci/rust-tests.sh` passed the full parallel-safe and serial test
  partitions with 1,941 current tests and the configured ignored tests left
  ignored.
- CodeGraph was synchronized with no pending files.

## Review and runtime evidence

Two read-only independent collection lanes reviewed the launch path and the
security boundary. The launch-path review supported request-scoped replacement.
The security review recommended a separate future policy for allowlisting
service-controlled Chrome arguments; that broader contract change is not part
of this compatibility repair.

The first initiating AuraCall Pro guard response
`resp_0c45a2a2ffd3416e9f546edd92bc6e9b` failed before a ChatGPT turn was sent.
At that time, the live `chatgpt-pro` profile reported both
`auracall-chatgpt-broker-v7` and `dashboard-service-backend` as exclusive
holders, so the broker correctly rejected the new tab request after its bounded
wait. A later read-only allocation check showed only the intended
`auracall-chatgpt-broker-v7` holder and no remaining conflict.

The initial retained-browser preflight reported `retained_daemon_missing` for
`nyse-developer-route` because the noninteractive shell omitted
`XDG_RUNTIME_DIR=/run/user/1000` and therefore inspected the wrong runtime
directory. Repeating the installed native status with the correct user runtime
directory verified the existing requirement exactly. The retained browser,
profile, target, URL, PID, loopback DevTools endpoint, and ready health were all
preserved; no requirement file or browser state was replaced.

These reconciliations were read-only. No ChatGPT prompt was sent, and neither
the Workshop nor NYSE page was navigated during the checks.

After those checks, initiating Pro guard
`agent-browser-wsl-sandbox-20260827-r2` submitted exactly once to the retained
Workshop conversation as response
`resp_c691988b11a247f99bfc1578b0204511`. The broker sent the prompt and then
reported `Remote Chrome connection lost before Aura-Call finished` before a
verdict was durably captured. The guard prompt did not contain a recovery nonce,
so no later rendered response can be proven to belong uniquely to this run.
The request was not retried, the reviewing Pro pass was not started, and runtime
publication remains blocked on a fresh, duplicate-safe review opportunity.

## Delegation receipt

- `launch_path_analysis`: completed read-only analysis; recommended honoring
  explicit request arguments in auto-launch and removing the smoke's ambient
  fallback. The primary agent verified and implemented the recommendation.
- `security_boundary_review`: completed read-only adversarial analysis;
  confirmed ambient recovery inheritance must remain closed and proposed a
  broader argument allowlist as a separate follow-up. The primary agent kept
  that proposal outside this bounded fix.
