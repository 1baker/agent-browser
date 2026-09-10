# Plan 0116 | Isolated Dashboard Publisher Lifecycle Fixture

State: CLOSED
Roadmap: P116
Plan version: 2
Date: 2026-08-14

## Objective

Prove the standalone dashboard publisher's normal and rollback restart
transitions with real process identity and signal behavior, without rebuilding,
installing, stopping the live dashboard, or touching a retained browser.

## Scope

- extract standalone quiescence and restart selection into a reusable module;
- preserve exact PID, UID, executable-name, and dashboard-mode ownership gates;
- launch temporary real processes whose `/proc` identity matches or deliberately
  differs from the allowed dashboard executable identity;
- exercise normal and rollback restart actions after exact quiescence;
- cover absent dashboards, stale PID metadata, and identity mismatch;
- expose one focused package test command.
- run the smoke policy and lifecycle fixture in the fast Linux Dashboard CI job.

## Non-Goals

- no release build, installed-binary replacement, or live listener restart;
- no retained browser, profile, target, prompt, page, or credential operation;
- no GitHub write or formal release.

## Acceptance Criteria

1. The production publisher calls the same extracted lifecycle functions used
   by the fixture.
2. Normal and rollback scenarios stop only the exact owned PID, remove its
   stale PID metadata, issue one exact `dashboard start`, and report the
   distinct expected restart action.
3. An absent dashboard remains absent unless explicit start authority exists.
4. Stale PID metadata is removed without signaling a process.
5. A live wrong-command process is rejected and remains alive until the fixture
   explicitly cleans up its tracked PID.
6. Focused and widened validation passes without changing live runtime identity.
7. Fast CI executes the smoke policy and real-process fixture before the
   dashboard build.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: Graphiti is unavailable and CodeGraph reports the checkout
  as not initialized. Direct source inspection is the documented fallback; no
  graph store or index was created.
- implementation: standalone quiescence and restart selection now live in
  `scripts/lib/local-dashboard-publisher-lifecycle.js`, and the production
  publisher supplies its real service report, status probe, and command runner.
- fixture: temporary copied Node executables provide real Linux `/proc`
  executable, UID, environment, PID, and SIGTERM behavior. Each PID is recorded
  explicitly and cleaned up by exact identifier.
- focused_validation: normal restart, rollback restart, absent, stale PID,
  identity mismatch, script syntax, and existing smoke-policy tests pass.
- ci_enforcement: the Linux Dashboard job now runs both publisher policy and
  lifecycle fixture gates before building the dashboard.
- live_boundary: no live dashboard, retained browser, or installed artifact was
  touched by the fixture.
- next_action_or_stop_reason: run widened convergence, parity, formatting,
  documentation, plan audit, skill parity, diff, and read-only retained-lane
  checks before closing P116.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: production standalone quiescence and restart decisions use
  the same extracted lifecycle module as the fixture. Exact PID, UID,
  executable-name, dashboard-mode, and bounded SIGTERM checks are preserved.
- regression: normal restart reports `restart-standalone`; rollback reports
  `restart-standalone-after-restore`; each stops one exact original PID and
  starts one replacement. Absent, stale-PID, and wrong-command paths behave as
  specified, and the mismatched live process is not signaled by production code.
- ci_enforcement: the fast Linux Dashboard job runs smoke policy followed by
  lifecycle fixture before its dashboard build. The fixture asserts that exact
  workflow ordering, and the workflow parses as YAML.
- validation: lifecycle fixture, smoke policy, local runtime convergence,
  capability fixture, API/MCP parity, dashboard incident contract, dashboard
  build, docs build, JavaScript syntax, package JSON parse, YAML parse, skill
  parity, and diff hygiene pass.
- unavailable_optional_tools: neither `actionlint`, repository Prettier, nor
  Ruby is installed. They were not treated as completed gates; Python PyYAML
  provided syntax parsing and the fixture validates exact CI placement.
- cleanup: post-test inspection reports zero fixture processes and zero fixture
  directories.
- retained_lane: read-only evidence still reports retained browser PID 1046742,
  CDP endpoint
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`,
  health `ready`, the exact Workshop URL, and title
  `Architecture Review Boundaries`.
- live_runtime: the fixture did not rebuild or install. The live executable and
  all local reference binaries remain at SHA-256
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`,
  and the dashboard manifest remains ready with SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, browser lifecycle, credential, or live dashboard action occurred.
- next_action_or_stop_reason: P116 acceptance criteria are met. Next, extract
  the publisher's top-level build, replace, readiness, and rollback orchestration
  behind injectable adapters so one end-to-end fault fixture can prove backup
  restoration and restart sequencing without a live install.
