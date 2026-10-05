# Full E2E and installation checkpoint, 2026-10-05

Status: **installation blocked; source checkpoint only**. The native live suite
did not pass, and no installed binary, shared service, retained browser, profile,
or tab was replaced. No provider message was submitted.

## Source identity

- Integrated published checkpoints: `0a7415be`, `93aa5e7c`, and `06dfa596`.
- Integration commits: `350b24e8`, then `8061a145`.
- Test-fixture checkpoint: `0703daa3367b615b6da1eff1a2fab9852b7bddac`.
- Tested `e2e_tests.rs` SHA-256:
  `2755880ab412c017fbf8f0ab07555f66be50290abff4aeb8185c3ed67558a787`.
- The bounded browser-family probe remains byte-identical to `0a7415be`.

## Verified results

| Check | Result |
| --- | --- |
| Ordinary Rust suite, optimized CI profile and repository serial partitions | 2,196 passed, including a rerun after the fixture changes |
| Native real-browser suite | 57 passed, 1 failed; not a green E2E result |
| Isolated ignored privacy/control-plane tests | 14 passed on `8061a145`; production code is unchanged by the fixture commit; guarded LitScout interoperability was not run |
| Required format, default-target clippy with warnings denied, JavaScript lint | Passed |
| Required CI no-launch smokes | All ten passed unchanged |
| Service API/MCP parity, service client contracts/types/examples, dashboard contracts | Passed |
| Dashboard and documentation production builds | Passed |
| Publisher, custody, journal, quiesce, provenance and rollback contract fixtures | Passed; these are not live installation proof |
| Source-free workstation installation | Passed with the exact-source external debug binary |
| VM harness, Guacamole assets, PostgreSQL durability/hardening, route user sync, release-verifier fixtures | Passed |

Three native cases return early when their optional engine is not configured:
the custom stealth-browser case and two Lightpanda cases. Accordingly, the
native result represents **54 executed Chrome cases passing, one failing, and
three optional-engine cases not certified**, not 57 independently executed
browser scenarios. No custom-engine certification or guarded installation was
attempted after the failure.

## Diagnosed fixture failures and remaining failure

The first native runs failed three cases. Two recovery fixtures changed `HOME`
and hid the installed Chrome cache. The existing production build-identity gate
correctly rejected their unproven browser. The fix exposes only the cached
version directory in the disposable home, verifies canonical identity, and
positively asserts `applied`, `fresh_installed_chrome_launch`, and the exact
executable path. The production gate was not changed.

The snapshot/reference-click case depended on example.com's former heading and
link order. The observed public DOM instead contained translated paragraphs
and a single `Learn more` link. An owned loopback fixture now supplies the exact
heading and link; the test still checks heading `e1`, link `e2`, a real click,
and exact navigation to `/clicked`. Other native cases retain public HTTPS
coverage. Failure diagnostics are deliberately retained; the cancellation
diagnostic reports only its synthetic job rather than the entire state store.

The resulting live run still failed
`e2e_service_detects_browser_crash_and_recovers_on_next_command` at
`cli/src/native/e2e_tests.rs:605`:
“terminated browser operational state should be removed after crash evidence
is recorded.” Valid launch proof and the process-exit event were reached, but
the browser operational record remained. **Cause undiagnosed; this may be
production behavior.** The saved browser record at this final assertion was
not captured. The follow-up recovery portion was not reached. The assertion
was not relaxed, skipped, or retried to obtain a passing result.

The latest inspected successful main CI run, `36460615622`, skipped Native E2E.
Earlier live status is therefore unattested, not previously green.

## Additional failed checks and environment limits

- Extra all-targets clippy failed on three findings. A read-only check of main
  `0a7415be` reproduced `chat.rs:669` (`unused_io_amount`) and
  `private_coordinator.rs:316` (`items_after_test_module`). The third,
  `actions.rs:43545` (`field_reassign_with_default` in the broker collector test),
  arrived with the integrated collector work. The required default-target CI
  clippy check passed; the broader check is not claimed green.
- The additional, non-CI `smoke-service-status-no-launch.js` failed because it
  requires `NotStarted` while a cold local status returned empty browser-health
  metadata. The actual ten CI no-launch scripts passed. This additional check
  remains unresolved; it was not changed or waived.
- The host-provision fixture could not find an AppArmor apt candidate in the
  intentionally restricted test filesystem. Host package indexes were not
  exposed. Earlier recipe omissions of `USER` and `/etc/os-release`, and the
  ambiguous checkout alias `/work` matching `/workstation`, were recorded and
  corrected in the private sandbox only. No production provisioning occurred.

## Isolation, review and next gate

Tests used private filesystem, PID, IPC and network namespaces, a disposable
home mounted at the unchanged home path, selected read-only toolchains/browser
files, and an external Cargo target. Host-loopback and WSL-gateway listener
negative controls passed before each run. Signed-in browser directories,
desktop forwarding, runtime sockets and provider authority were not exposed.
Network isolation is not claimed to block every possible LAN address.

Detailed recipes and logs intentionally remain local; this sanitized receipt
contains no credentials, private host paths, provider chat content or tab IDs.
Reproduction requires an equivalently isolated environment, an installed
Chrome cache, a disposable profile, the repository CI Rust runner, and
`cargo test --profile ci --manifest-path cli/Cargo.toml e2e` with ignored tests
enabled and one test thread. Never run the broad suite against a live user home.

Delegation was used for an independent integrated-source review and a
closed-world review of the fixture repair. Both returned no source blockers;
neither independently ran the tests. The owner verified the runtime results.
Claude agreed on the fixture plan and the blocked checkpoint, with the explicit
limitation that its file access covered main rather than the successor diff.

Installation is blocked both by the live crash-cleanup failure and by unresolved
source provenance for retained legacy clients, including running artifact
`0697d956ca4cf6caab2de894858b6de5fe05a0c7d7ba4250a7174b0b953fb477`.
Its artifact manifest identifies bytes and version but not trustworthy source
behavior. A full executable census found additional legacy MCP owners beyond
the initial two-client check. They were preserved, not stopped or restarted;
the whole active writer population remains an installation prerequisite.

Next: diagnose the crash-cleanup failure in an isolated scope, resolve legacy
client compatibility with their owners, then rerun the failed gates before
building a release candidate and attempting the guarded prebuilt publisher.
This checkpoint is not an installation, formal release, or live consumer
acceptance claim.
