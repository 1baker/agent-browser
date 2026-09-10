# Plan 0115 | Capability-Resolved Dashboard Browser Smoke

State: CLOSED
Roadmap: P115
Plan version: 2
Date: 2026-08-14

## Objective

Make required rendered-page dashboard QA select a verified installed browser
capability and an isolated disposable profile, while preserving the retained
authenticated browser and avoiding a general `--no-sandbox` launch default.

## Scope

- read `service status` launch capability without launching a browser;
- select the configured patched Chromium only when its manifest, executable,
  artifact smoke, and WSL profile-smoke evidence agree;
- create Windows-hosted Chromium profiles under the matching user temp root;
- pass the selected browser build through the existing CLI launch contract;
- close the exact smoke session before deleting only its generated profile;
- report browser build, executable, selection source, profile source, and
  unsafe-argument evidence in structured smoke output.

## Non-Goals

- no retained target navigation, prompt, click, typing, or profile reuse;
- no global sandbox weakening or direct Chrome launch;
- no GitHub write or formal release.

## Acceptance Criteria

1. Ready installed `stealthcdp_chromium` evidence selects that exact build and
   executable with a generated Windows-mounted disposable profile.
2. Missing, stale, failed, or mismatched manifest and executable evidence fails
   before launch instead of silently claiming the configured build.
3. The smoke launch contains no general `--no-sandbox` argument.
4. A strict live smoke renders the dashboard app chrome and workspace pane once,
   closes its exact session, and removes the generated profile.
5. The retained ChatGPT browser PID, CDP endpoint, target URL, and title remain
   unchanged.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_live_validation_started
- discovery_status: Graphiti is absent and CodeGraph reports this checkout as
  not initialized. Direct source and contract inspection is the policy-safe
  fallback; no repository index was created.
- capability_evidence: installed service status reports
  `defaultBrowserBuild=stealthcdp_chromium`, a valid ready manifest, an existing
  Windows-mounted executable, a passed artifact smoke, and applicable WSL
  profile-smoke evidence.
- implementation: the dashboard smoke resolves that evidence, creates a
  throwaway profile under the executable owner's Windows temp root, passes the
  first-class browser-build flag, closes the exact session, and removes the
  profile with bounded retries.
- focused_validation: resolver, stale-evidence, explicit-profile, unsafe-arg,
  script syntax, and existing dashboard smoke-policy tests pass.
- live_validation: a disposable session rendered `http://127.0.0.1:4848/`, saw
  Agent Browser app chrome and Workspaces, closed, and left no generated profile.
- publisher_recovery: the first guarded release publish exposed that a verified
  standalone listener was quiesced but not restarted when no systemd user bus
  existed and `--start-if-missing` was absent. The publisher now resumes the
  exact owned standalone listener after normal handoff or rollback while still
  requiring explicit start authority for a genuinely absent dashboard.
- retained_lane: read-only status and page identity evidence still reports PID
  1046742, the retained CDP endpoint, URL `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`,
  and title `Architecture Review Boundaries`.
- next_action_or_stop_reason: run widened validation, install the updated
  runtime, execute the publisher in required-browser mode, and close the plan
  only after exact runtime and retained-lane proof.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: rendered-page dashboard QA consumes the installed no-launch
  capability, validates exact patched-Chromium readiness, selects the existing
  browser-build contract, creates a Windows-mounted disposable profile, and
  removes it only after closing the unique smoke session. Stale or mismatched
  readiness fails before launch; no general sandbox override is generated.
- lifecycle_fix: a publisher that quiesces an exact owned standalone dashboard
  now resumes it after handoff or rollback without requiring
  `--start-if-missing`. A previously absent dashboard still needs that explicit
  authority.
- required_live_publish: release publication without `--start-if-missing`
  returned `browserSmoke.status=passed` and
  `classification=rendered_page_verified`, rendered Agent Browser app chrome
  and Workspaces at `http://127.0.0.1:4848/`, and reported
  `service.action=restart-standalone`.
- installed_validation: installed, release, and repository reference binaries
  match SHA-256
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`.
  The live manifest reports dashboard SHA-256
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`,
  service contract `service-ui-runtime.v1`, and the exact installed executable.
- retained_lane: before/after readback proves PID 1046742 and CDP endpoint
  `ws://127.0.0.1:39377/devtools/browser/7016bf50-2a61-466c-b3e1-627afeaf1529`
  unchanged and healthy. Read-only URL and title remain the exact Workshop
  conversation and `Architecture Review Boundaries`.
- cleanup: no `agent-browser-dashboard-smoke-*` profile remains in the matching
  Windows temp root.
- validation: focused resolver and smoke policy, local runtime convergence,
  workspace navigator and inspector, API/MCP parity, request-client parity,
  JavaScript syntax, Rust formatting, production binary strict clippy,
  dashboard build, docs build, and diff hygiene passed. Docs lint remains
  nonzero only for the pre-existing theme-toggle effect error and unused search
  variable warning outside this slice.
- doctor: the live dashboard is ready with zero stale runtimes. Overall
  convergence remains `partial` because the standalone dashboard is a known
  diagnostic runtime; this is not runtime drift.
- publication: no GitHub or external write occurred. No prompt, typing, click,
  retained-page navigation, or credential action occurred.
- next_action_or_stop_reason: P115 acceptance criteria are met. Next, add an
  isolated publisher integration fixture that exercises normal and rollback
  standalone restart transitions without a live release build.
