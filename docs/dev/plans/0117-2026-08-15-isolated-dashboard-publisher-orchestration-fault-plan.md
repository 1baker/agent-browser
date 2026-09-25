# Plan 0117 | Isolated Dashboard Publisher Orchestration Fault Fixture

State: CLOSED
Roadmap: P117
Plan version: 2
Date: 2026-08-15

## Objective

Prove the local dashboard publisher's full build, backup, quiesce, replacement,
handoff, restart, readiness, and rollback sequence without rebuilding or
replacing the live runtime.

## Scope

- extract top-level publisher sequencing into a reusable orchestration module;
- inject production build, filesystem, handoff, dashboard, and readiness
  adapters at the executable boundary;
- use real temporary old, replacement, backup, staged, and restored binaries;
- prove exact successful phase ordering and artifact results;
- fault after replacement before browser handoff and prove backup restoration
  occurs before rollback restart;
- fault after a committed browser handoff and prove the replacement plus
  recovery evidence are preserved;
- prove a rollback restart failure is recorded without replacing the original
  publication fault;
- add the focused fixture to fast Linux Dashboard CI before the dashboard build.

## Non-Goals

- no live build, installed-binary replacement, listener restart, or daemon
  handoff;
- no retained browser, page, profile, target, prompt, credential, or external
  service operation;
- no GitHub write or release action.

## Acceptance Criteria

1. The production publisher invokes the same orchestration module used by the
   fixture.
2. The success fixture proves build, backup, quiesce, replacement, handoff,
   restart, HTTP readiness, manifest readback, browser diagnostic, and final
   status ordering against real temporary binary files.
3. A pre-handoff fault restores the original bytes and mode before exactly one
   rollback restart.
4. A fault after committed handoff evidence does not restore the old binary and
   preserves the replacement for retained-browser reconciliation.
5. Rollback restart failure is reported separately while the initiating error
   remains primary.
6. Pre-mutation build failure leaves the original binary unchanged and performs
   no backup, quiescence, install, or restart action.
7. Focused and widened validation pass without changing live runtime identity.
8. Fast CI executes policy, lifecycle, and orchestration gates in order before
   its dashboard build.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: Graphiti's documented runtime is absent and CodeGraph
  reports this checkout as not initialized. Focused source, policy, prior-plan,
  and test inspection is the documented fallback; no index was created.
- implementation: top-level sequencing now lives in
  `scripts/lib/local-dashboard-publisher-orchestration.js`, while the executable
  supplies production build, filesystem, lifecycle, handoff, and readiness
  adapters.
- fixture: isolated real files prove the success sequence, pre-handoff restore,
  committed-handoff preservation, rollback restart error reporting, and
  pre-mutation failure boundary.
- focused_validation: orchestration fixture, lifecycle fixture, smoke-policy
  contract, and JavaScript syntax checks pass.
- live_boundary: no live installed binary, dashboard listener, daemon session,
  or retained browser was touched.
- next_action_or_stop_reason: run widened convergence, parity, build,
  documentation, direct plan audit, skill parity, cleanup, diff, and read-only
  retained-lane checks before closing P117.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: the executable wires its real build, backup, atomic install,
  reference sync, daemon handoff, dashboard lifecycle, and readiness functions
  into the shared orchestration module. Production sequencing no longer has a
  separate untested control-flow copy.
- regression: real temporary files prove exact successful order and executable
  mode retention, restore-before-restart on a pre-handoff fault, no restore
  after committed handoff evidence, secondary rollback restart error capture,
  and zero mutation after a build fault.
- ci_enforcement: fast Linux Dashboard CI runs smoke policy, real-process
  lifecycle, and real-file orchestration fixtures in that order before build;
  the lifecycle fixture asserts exact placement and the workflow parses as
  YAML.
- validation: focused publisher tests, local runtime convergence, browser
  capability fixture, API/MCP parity, dashboard incident and workspace
  contracts, service-client JavaScript types, JavaScript syntax, dashboard
  production build, docs production build, package JSON parse, CI YAML parse,
  repository/installed skill parity, direct plan audit, cleanup inspection, and
  scoped diff hygiene pass.
- lint_baseline: docs-wide ESLint still reports the pre-existing
  `theme-toggle.tsx` effect error and one search-route warning. Neither file is
  modified by this slice; both production builds and the touched-surface checks
  pass.
- unavailable_optional_tools: `actionlint` and Ruby are not installed and are
  not claimed as completed gates.
- cleanup: post-test inspection reports zero orchestration or lifecycle fixture
  directories.
- retained_lane: read-only service evidence still reports browser PID 1046742,
  CDP endpoint
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`,
  health `ready`, URL
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`, and title
  `Architecture Review Boundaries`.
- live_runtime: no publication occurred. The installed and workspace reference
  binaries remain at SHA-256
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`,
  and the ready dashboard manifest remains at SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, browser lifecycle, credential, attach, detach, daemon handoff, or
  live dashboard action occurred.
- next_action_or_stop_reason: P117 acceptance criteria are met. Next, make
  backup restoration itself evidence-bearing by recording and verifying source,
  backup, replacement, and restored hashes and by preserving the initiating
  publication failure if the restore operation also fails.
