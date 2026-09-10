# Plan 0114 | Dashboard Publisher Readiness Classification

State: CLOSED
Roadmap: P114
Plan version: 2
Date: 2026-08-14

## Objective

Make installed dashboard HTTP, bundle, marker, and runtime-manifest readiness
the authoritative local publish gate while preserving rendered-page browser QA
as a distinct diagnostic. A known disposable-Chrome launch failure before
renderer acquisition is advisory by default on WSL; every rendered-page or
contract failure remains fatal.

## Scope

- run required HTTP, asset marker, and runtime-manifest verification before any
  disposable browser attempt;
- classify only known browser launch-unavailable evidence in the pre-render
  phase as advisory by default;
- retain fail-closed behavior for HTTP, manifest, marker, authentication, DOM,
  workspace, and post-launch browser failures;
- expose an explicit required-browser mode for release-style validation;
- return structured readiness and browser-smoke evidence separately.
- replace a standalone dashboard listener only after exact PID, user, command,
  and dashboard-mode evidence matches; fail closed on ambiguous metadata.

## Non-Goals

- no `--no-sandbox` launch default;
- no weakening of managed Chrome sandbox policy;
- no retained browser, profile, target, prompt, page, or external mutation;
- no GitHub write or formal release.

## Acceptance Criteria

1. A passing HTTP, marker, and exact runtime-manifest gate commits publication
   readiness independently of the disposable browser.
2. The observed pre-render Chrome exit without DevTools returns success with a
   structured `browser_launch_unavailable` diagnostic by default.
3. `--require-browser-smoke` makes the same launch failure fatal.
4. Marker, manifest, authentication, rendered DOM, and workspace failures stay
   fatal regardless of default advisory launch handling.
5. Focused policy tests, script syntax, documentation, formatting, lint, build,
   install, live manifest readback, and retained-lane preservation pass.
6. A non-systemd standalone dashboard listener is retired and replaced without
   broad process matching, while wrong-user, wrong-command, and wrong-mode PID
   evidence is rejected.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the P113 publisher completed live HTTP, bundle, authenticated API,
  and runtime-manifest checks but exited nonzero because the same combined smoke
  could not launch disposable Chrome under WSL sandboxing.
- discovery_status: the documented Graphiti executable is absent and CodeGraph
  reports the checkout as not initialized. Repository policy prohibits silent
  initialization, so direct source, contract, and test inspection is the
  verified fallback.
- subagent_status: not_spawned; active system policy prohibits delegation.
- next_action_or_stop_reason: split required HTTP readiness from classified
  browser rendering diagnostics and preserve fatal post-launch validation.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: publication now completes required HTTP, asset-marker, and
  exact runtime-manifest verification before attempting disposable-browser QA.
  Browser results are separate structured evidence. Only a known pre-render
  launch-unavailable result is advisory by default; required-browser mode and
  every rendered-page failure remain fatal.
- standalone_listener: when the user systemd bus is unavailable, the publisher
  accepts only the exact PID-file process with matching current UID,
  `agent-browser` command, and `AGENT_BROWSER_DASHBOARD=1`. Stale metadata is
  removable; identity mismatch fails closed; an exact listener receives one
  SIGTERM and must exit before replacement continues.
- regression: focused tests cover the observed Chrome exit 21 disposition,
  required-browser escalation, late-phase fail-closed behavior, passing browser
  evidence, exact standalone identity, and wrong-user, wrong-command, and
  wrong-mode rejection.
- live_publish: the first attempt correctly failed on a real old-listener
  manifest mismatch. After exact-listener convergence was implemented, publish
  succeeded with required marker and manifest checks, then returned
  `browserSmoke.status=unavailable`,
  `classification=browser_launch_unavailable`, `advisory=true`, and
  `fatal=false` for the pre-render WSL Chrome exit.
- installed_validation: installed and release executable SHA-256 values match
  `8afeb3a270ce54c85cc25a14292e75e6299eee4c5dcc087c9aaf2342e992929e`.
  The live dashboard reports bundle SHA-256
  `ae1565768a00643425b703ae38c9fd8a992eb1e02534c5c78e76a7635de14027`,
  exact executable agreement, and the required marker. The new exact
  standalone dashboard PID is 2448046.
- retained_lane: executable handoff preserved browser PID 1046742 and CDP URL
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`.
  Read-only batch evidence returned URL
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2` and title
  `Architecture Review Boundaries`.
- validation: focused smoke policy, dashboard workspace inspector, local
  convergence contract and fixture, API/MCP parity, JavaScript syntax,
  formatting, production-target strict clippy, dashboard release build, docs
  build, direct plan audit, repository/installed skill parity, and diff hygiene
  passed.
- doctor: runtime inventory is converged with zero stale runtimes and the live
  dashboard is ready. Overall doctor remains nonzero for the pre-existing
  workstation payload drift; runtime convergence calls the expected standalone
  dashboard a diagnostic runtime and therefore remains `partial`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, credential operation, or retained-page mutation occurred.
- next_action_or_stop_reason: P114 acceptance criteria are met. Next, resolve a
  compatible disposable browser through installed host capabilities so WSL
  rendered-page QA can pass without adding a general `--no-sandbox` default.
