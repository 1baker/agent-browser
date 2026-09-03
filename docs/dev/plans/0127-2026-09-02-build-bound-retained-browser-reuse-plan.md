# Plan 0127 | Build-Bound Retained Browser Reuse

State: IMPLEMENTED; REVIEW FAILED CLOSED
Roadmap: P127
Plan version: 1
Date: 2026-09-02

## Objective

Make ordinary service-owned agent-browser calls prove the retained browser's
selected build and executable, and fail closed instead of silently attaching a
stealth-required request to a retained stock Chromium process.

## Immutable Intake

Persist authoritative browser-build and executable launch proof on retained
browser records. Require access-plan reuse and daemon auto-attach to match the
requested or policy-selected browser build exactly. Treat missing proof as
incompatible whenever a build is required. Preserve the retained browser and
profile rather than closing it or launching a duplicate lane. Add focused
regressions, synchronize operator documentation, validate and install the
candidate, and perform a safe live no-prompt verification without navigating
the retained FigureLabs target.

## Frozen Acceptance Criteria

1. A retained browser record carries the selected browser build, executable
   path when known, and durable launch-selection proof.
2. Build proof survives health-only updates and service-state round trips.
3. Access-plan reuse requires an exact selected-build match. A same-profile,
   healthy retained browser with a different or unknown build is not reusable.
4. Daemon auto-attach applies the same exact-build rule after service defaults,
   site policy, profile selection, and explicit request overrides are resolved.
5. A mismatched retained profile lane is preserved and blocks a duplicate
   launch through the existing profile-lane guard.
6. Explicit non-service direct attachment remains available when no governed
   browser build is required.
7. Focused tests cover exact match, mismatch, missing proof, preserved proof,
   no duplicate process, and request/default resolution.
8. README, CLI help, docs, roadmap, runbook, and agent-browser skill guidance
   use the same fail-closed contract.
9. Focused and widened Rust tests, formatting, strict Clippy, TypeScript
   typecheck, targeted lint, production builds, plan audit, CodeGraph sync, and
   diff hygiene pass.
10. Installed-runtime verification is read-only and sends no application prompt,
    performs no retained-tab navigation, and does not close a browser.
11. The bilateral workflow binds this intake, approves both complete packets,
    performs exactly one ordered Pro synthesis transit, and records any browser
    authority blocker without launching a substitute lane.

## Execution Graph

1. Inspect the persisted lifecycle and every retained-browser reuse path.
2. Collect architecture, security, compatibility, and operator evidence in the
   bilateral workflow; Codex approves both packets before Pro submission.
3. Extend the retained browser record and launch persistence with build proof.
4. Apply one shared compatibility predicate to access-plan and daemon attach.
5. Add regressions and synchronize public documentation.
6. Run focused and widened validation, then ordered bilateral synthesis/review.
7. Install through the guarded handoff and perform a safe no-prompt live check.

## Delegation Receipt

- State: `not_spawned`
- Reason: current runtime policy does not authorize sub-agent delegation, and
  the persistence and reuse guards share one tightly coupled authority boundary.
- Runtime handle: none

## Evidence Log

- 2026-09-02: service status resolves `stealthcdp_chromium` as the configured
  default and reports the installed Windows executable plus ready manifest.
- 2026-09-02: retained `session:default` is healthy and controllable but its
  persisted browser record has no build or launch metadata; its running process
  is bundled Linux Chromium, not the configured stealth executable.
- 2026-09-02: `profile_reuse_decision`,
  `shared_profile_attach_target_for_auto_launch`, and
  `retained_session_attach_target_for_auto_launch` filter profile and posture
  but do not compare a retained browser's build.
- 2026-09-02: `BrowserProcess` has no browser-build or executable-proof fields;
  only remote display allocation opportunistically copies a build label.
- 2026-09-02: implementation adds durable `browserBuild`, `executablePath`, and
  `browserBuildProof`, preserves authoritative proof through health updates,
  and applies exact-build matching to access-plan reuse, retained-session
  attach, shared-profile attach, and active command dispatch.
- 2026-09-02: a fresh `stealthcdp_chromium` launch now fails closed before
  browser start when the browser-capability resolver cannot apply a validated
  executable binding. Stock Chrome keeps its existing discovery behavior.
- 2026-09-02: focused tests pass for exact match, wrong build, missing proof,
  durable proof preservation, no duplicate profile lane, and unproven stealth
  launch rejection. Explicit CLI build routing now survives both ordinary
  command parsing and the synthetic prestart launch, and explicit launch itself
  enforces the same proof gate before browser start. The full serialized Rust
  suite passes 1,902 tests with 57
  ignored; strict Clippy, ESLint, service-client type checks and contract gates,
  docs build, and dashboard build pass.
- 2026-09-02: installed runtime hash
  `a970b3267c56b7dace8e6cb2055fc101f5e1a1e877d20fb4a45dcedceb1f1984`
  reached terminal journal state `recovered_ready`; six active sessions reattached,
  eight retained browser records remain visible, and the exact ICE SSO target
  survived the handoff. Idle daemon sessions were retired without closing their
  shared or retained browser processes.
- 2026-09-02: an installed-runtime negative check requested
  `stealthcdp_chromium` with a disposable session and profile. It failed with
  `Stealth browser launch proof unavailable` and
  `no_matching_preference_binding`; retained browser inventory remained eight
  before and after. The disposable daemon was closed, with no browser launch,
  retained-tab navigation, or application prompt.
- 2026-09-02: before the idle default session was retired, no-launch live access
  planning rejected its healthy unproven browser with
  `retained_browser_build_mismatch_or_missing_proof`. Browser-capability
  preflight still reports `no_matching_preference_binding`; the current access
  plan recommends `launch_new_browser`, but the command path will stop before
  that fresh stealth launch until validated registry evidence is added.
- 2026-09-02: bilateral collection completed and the initiating Pro result was
  sufficient. The reviewing Pro run
  `resp_95b9612f04c74764ae93b740fb0604c9` failed closed before Send because the
  requested `chatgpt-pro` profile did not match the retained
  `auracall-chatgpt-live` bridge daemon profile. No retry was submitted.
- 2026-09-02: the validated `stealthcdp-win-150` registry binding now resolves
  `stealthcdp_chromium` to the exact installed executable. Stale listeners were
  retired without replacing the authenticated retained browser.
- 2026-09-02: attached-existing browser verification now accepts a missing
  local PID only when the live daemon socket, loopback CDP endpoint,
  browser/session/profile identity, one target, exact URL, and target-bound
  handle all agree. Locally owned browsers still require their live PID.
- 2026-09-02: publication recovery persists its original browser-smoke policy,
  so a recover-only handoff cannot introduce a disposable browser after an
  intentionally browser-free publish attempt.
- 2026-09-02: service-tab handles now derive profile identity from the
  access-plan command before the manager fallback and never from ambient
  environment state. This repaired broker acquisition for an attached-existing
  `chatgpt-pro` browser whose local manager has no runtime-profile name.
- 2026-09-02: focused regression tests pass for the attached-existing lifecycle,
  smoke-policy recovery, and access-plan-owned handle profile. The widened Rust
  suite passes 1,906 tests with 57 ignored; strict Clippy, formatting, ESLint,
  service-client and HTTP/MCP parity checks, route-confusion gates, docs and
  dashboard builds, native release build, planning audit, CodeGraph sync, and
  diff hygiene pass.
- 2026-09-02: guarded publication installed executable SHA-256
  `2dff309146ce62e2383ceaf6c2fe9169d92093049b5b29b9e3cb6260a5e2c9fe`
  and reached terminal journal state `recovered_ready`. Follow-up workstation
  doctor reported convergence with no issues. A live no-prompt broker request
  returned `session:default`, profile `chatgpt-pro`, the exact conversation URL,
  and a valid target-bound handle, then released the tab while preserving the
  retained browser.
- 2026-09-02: reviewing Pro run
  `resp_f08eb21d87c84f1db2756f646beb7ec8` received a durable response id, but
  AuraCall restarted while waiting and expired the lease. Recovery terminated
  with `runner_execution_failed` because the submitted WebSocket was closed and
  explicitly refused prompt replay. Read-only retained-page inspection found no
  nonce-matched user message, so the prompt was not promoted as committed and
  was not resubmitted. The retained browser remains open on one Workshop target
  plus non-application blank/new-tab surfaces.

## Closeout

The implementation, installed runtime, and deterministic validation are
complete. The earlier stealth-binding and broker-profile blockers are closed.
Final review publication failed closed after the AuraCall API restart because
the submitted WebSocket could not be reattached. The retained page contains no
matching committed turn, and recovery correctly refused prompt replay. Do not
retry against a substitute target or launch a duplicate authenticated lane.
