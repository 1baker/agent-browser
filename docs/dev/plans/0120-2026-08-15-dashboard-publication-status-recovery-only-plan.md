# Plan 0120 | Dashboard Publication Status and Recovery-Only Operations

State: CLOSED
Roadmap: P120
Plan version: 2
Date: 2026-08-15

## Objective

Separate publication-journal inspection and recovery authorization so a status
request is strictly read-only and a recovery request can never fall through to
a new dashboard build.

## Scope

- add `--journal-status` to the local dashboard publisher;
- read journal and lock evidence without acquiring a lock or creating runtime
  directories;
- hash the journal-bound installed binary and classify it as verified
  replacement, built replacement, backup, missing, or unknown;
- return exact recommended actions for no journal, active publisher,
  recoverable transaction, and unverified installed artifact;
- redact status to transaction metadata, artifact identity, handoff counts, and
  failure classification rather than full handoff descriptors;
- add `--recover-only` that recovers one incomplete transaction or returns a
  no-op and never starts a new build;
- reject combined status and recovery authorization;
- expose package commands for status and recovery-only;
- prove command-level no-write and no-build behavior in an isolated home;
- run the operations fixture in fast Dashboard CI.

## Non-Goals

- no live recovery, publication, installation, listener restart, daemon handoff,
  or browser lifecycle action;
- no dashboard UI, HTTP service, MCP, or installed CLI contract in this slice;
- no prompt, page, profile, target, credential, or external service mutation;
- no GitHub write or release action.

## Acceptance Criteria

1. Status returns before orchestration, lock acquisition, build, backup, or
   lifecycle code.
2. Status on an absent journal creates no filesystem state.
3. Status reports live-lock wait, verified recovery-only, and unknown-artifact
   investigation recommendations from authoritative evidence.
4. Recovery-only recovers a nonterminal transaction through the existing P119
   state machine.
5. Recovery-only with no journal or a terminal journal returns
   `nothing_to_recover` without build, backup, install, or restart.
6. Status and recovery-only cannot be combined.
7. Package scripts and fast CI expose and enforce both operations.
8. Focused and widened validation pass without changing live runtime identity.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: Graphiti's documented runtime is unavailable and CodeGraph
  reports this checkout as not initialized. Current P119 source, plan, and
  fixtures are the documented fallback; no index was created.
- status: `inspectLocalDashboardPublicationJournal` reads atomic journal, lock,
  and installed SHA evidence without creating state. It returns bounded
  transaction metadata, counts, artifact classification, recoverability, and
  one exact recommended action.
- recovery_only: orchestration recovers an existing nonterminal journal first;
  otherwise `recoverOnly` returns `nothing_to_recover` before the first build
  adapter.
- command_fixture: a disposable HOME proves status creates no `.agent-browser`
  directory, recovery-only creates no installed binary or journal and leaves no
  lock, neither emits build or dashboard-start commands, and combined flags exit
  with status 2.
- focused_validation: journal status, recovery-only, orchestration, journal,
  lifecycle, smoke-policy, and JavaScript syntax fixtures pass.
- live_boundary: no production status, recovery, build, install, listener,
  daemon, or browser action has occurred yet.
- next_action_or_stop_reason: update operator documentation and skill parity,
  run widened validation and builds, then safely live-verify only
  `--journal-status` before closing P120.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- status_contract: successful JSON contains only `success`, `operation`, and
  `publicationJournalStatus`. No service, smoke, build, handoff, or mutation
  placeholders appear in the read-only envelope.
- status_evidence: journal inspection reports schema, exact paths, lock state,
  bounded transaction metadata and handoff counts, installed SHA classification,
  recoverability, and one of `none`, `wait_for_active_publisher`,
  `recover_only`, or `investigate_installed_artifact`.
- recovery_authority: recovery-only handles a nonterminal transaction through
  P119. An absent or terminal journal returns `nothing_to_recover` before the
  first build adapter. Status and recovery flags together exit with status 2.
- command_fixture: disposable HOME execution proves status creates no runtime
  directory. Recovery-only creates no installed binary or journal, releases its
  exact lock, and emits no build or dashboard-start command.
- ci_enforcement: fast Dashboard CI runs smoke policy, lifecycle,
  orchestration, journal, and operations fixtures in order before build; the
  lifecycle fixture asserts exact placement.
- validation: focused status, recovery-only, journal, orchestration, lifecycle,
  publisher policy, local convergence, browser capability, API/MCP parity,
  dashboard incident and workspace contracts, service-client JavaScript types,
  JavaScript syntax, dashboard and docs production builds, package JSON, CI
  YAML, repository/installed skill parity, direct plan audit, cleanup, and
  scoped diff hygiene pass.
- lint_baseline: docs-wide ESLint retains the pre-existing
  `theme-toggle.tsx` effect error and one search-route warning. Neither file is
  modified by this slice.
- unavailable_optional_tools: `actionlint` and Ruby are not installed and are
  not claimed as completed gates.
- live_status: `pnpm status:local-dashboard-publication` returned success,
  operation `journal_status`, schema v1, no journal, no lock, no transaction,
  no installed artifact classification, `recoverable=false`, and recommended
  action `none`. Journal and lock were absent before and after the command.
- cleanup: post-test inspection finds zero status, journal, orchestration, or
  lifecycle fixture directories and no production journal or lock.
- retained_lane: read-only evidence still reports browser PID 1046742, exact
  CDP endpoint, ready health, profile `chatgpt-pro`, exact target, Workshop URL,
  and title `Architecture Review Boundaries`.
- live_runtime: no recovery or publication occurred. Installed and workspace
  reference SHA-256 remains
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`;
  the live dashboard returns HTTP 200 with dashboard SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.
- publication: no GitHub or external write occurred. No prompt, click, typing,
  navigation, browser lifecycle, credential, attach, detach, daemon handoff,
  install, recovery, or live dashboard action occurred.
- next_action_or_stop_reason: P120 acceptance criteria are met. Next, project
  this read-only status into install doctor and the dashboard service surface so
  operators can see incomplete or blocked publication state without shell
  access; keep recovery as an explicit confirmed operation.
