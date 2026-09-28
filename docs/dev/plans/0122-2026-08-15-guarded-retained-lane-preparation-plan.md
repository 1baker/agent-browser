# Plan 0122 | Guarded Retained Lane Preparation

State: CLOSED
Roadmap: P122
Plan version: 8
Date: 2026-08-15

## Objective

Replace the repeated operator navigation and target-copy handoff with one
guarded command that opens an exact reviewed URL in the required managed
profile, verifies the route-bound rendered identity, uniquely discovers the
same exact live lane, and commits the existing marker-first retained-browser
requirement.

## Scope

- require an exact canonical HTTP or HTTPS URL, reviewed origin and path
  prefix, managed runtime profile, and supported browser build;
- use only `remote-view open` for browser acquisition and navigation;
- require operator-visible readiness plus exact URL, profile, target, browser,
  and session evidence from the route-bound response;
- rediscover exactly one ready live page at the exact URL and required profile
  before the existing digest-bound requirement transaction;
- provide isolated fixtures for URL and profile drift, incomplete identity,
  ambiguous discovery, and command composition;
- document the automation boundary and retain existing publication guards.

## Non-Goals

- no click, type, fill, evaluate, upload, send, submit, or prompt action;
- no authentication automation or credential handling;
- no automatic recovery from an ambiguous or mismatched target;
- no GitHub write or release action;
- no live ChatGPT execution while the earlier no-navigation and no-submission
  instruction remains active.

## Acceptance Criteria

1. One command composes route-bound exact-URL open and marker-first pinning.
2. The exact URL must be within the reviewed origin and path prefix.
3. Rendered URL, profile, target, browser, session, and operator-visible proof
   must agree before discovery proceeds.
4. Discovery requires exactly one ready exact-URL target and the requested
   profile before either private authority file is written.
5. Isolated command evidence proves the operation invokes only `remote-view
   open` and the retained requirement writer, with no page-interaction verb.
6. Focused and widened validation, documentation, plan audit, and diff hygiene
   pass.
7. Live no-prompt verification proves the exact target is pinned and survives
   guarded installation once live navigation is authorized.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_fixture_validation
- implementation: exact-URL and profile-aware discovery now complements the
  existing prefix selector. `prepare:local-dashboard-retained-browser`
  normalizes the reviewed request, invokes route-bound `remote-view open`,
  verifies exact rendered identity, then invokes unique exact discovery and
  the existing digest-bound requirement writer.
- authority_boundary: the preparation argv contains no page interaction or
  prompt-submission action. An isolated two-process fixture records only the
  expected open and pin operations.
- focused_validation: discovery, preparation contract, command orchestration,
  and JavaScript syntax fixtures pass.
- live_boundary: no ChatGPT navigation, prompt submission, typing, click,
  retained-lane pin, installation, or GitHub write occurred in this receipt.
- next_action_or_stop_reason: finish documentation and widened validation, then
  request removal of the live no-navigation boundary before running the exact
  Workshop preparation command.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_widened_validation_ready
- focused_validation: retained guard, requirement, exact discovery,
  preparation contract, preparation command, watchdog, publication operations,
  and publisher orchestration fixtures pass.
- widened_validation: service-client typecheck, dashboard production build,
  docs production build, runtime convergence contract and isolated fixture,
  package JSON parsing, skill parity, JavaScript syntax, and full diff hygiene
  pass.
- lint: the full docs lint remains blocked by the pre-existing synchronous
  `setMounted(true)` effect in `docs/src/components/theme-toggle.tsx`; the docs
  production TypeScript build passes and the changed MDX has no configured
  standalone lint parser. Root JavaScript ESLint is not configured, so the
  changed scripts use Node syntax and executable fixture validation.
- live_readback: both retained authority files remain absent. The temporary
  `chatgpt-pro` blank browser has exited and has no targets. No live ChatGPT
  navigation or retained authority write occurred.
- acceptance_audit: criteria 1 through 5 are proven. Criterion 6 is proven
  except for the unrelated existing docs lint baseline. Criterion 7 remains
  intentionally open pending authority to navigate the exact Workshop URL.
- next_action_or_stop_reason: run the single exact Workshop preparation command
  after the active no-navigation boundary is explicitly lifted, then install
  and verify retained identity without submitting a prompt.

## Execution Receipt C03

- plan_version: 3
- state_transition: OPEN -> OPEN
- progress_classification: source_free_preparation_candidate_ready
- source_free_controller: preparation no longer calls the repository publisher.
  A bounded live module uniquely discovers the exact URL and profile, requires
  agreement with the route-bound opened target, reverifies daemon, browser,
  CDP, and target evidence, then invokes the existing marker-first writer.
- installed_surface: the workstation bundle embeds the preparation controller
  and its five library dependencies. The installed binary exposes
  `install workstation prepare-retained-browser`, forwards only its reviewed
  arguments, and binds the installed agent-browser executable.
- fresh_install_proof: the isolated workstation fixture verifies every embedded
  controller hash, invokes installed command help, invokes controller help from
  the staged support directory without the checkout, and passes.
- lint_repair: the unrelated docs baseline blocker was removed by replacing the
  mount-state effect with hydration-safe `useSyncExternalStore` and projecting
  search results without an unused score binding. Full docs lint and production
  build now pass.
- widened_validation: preparation and live pin fixtures, source-free workstation
  fixture, 20 native workstation tests, strict production-binary Clippy, Rust
  formatting, docs lint/build, and optimized release build pass.
- release_candidate: optimized SHA-256 is
  `954a01d3783912a129a1a23f6dc6c606dc455ce1991d380a64b0a898d2f29389`.
  The installed binary remains intentionally unchanged at
  `072ba7f998de7da35d94fb6f0f4a3782ea012b7ba1711d913527788c7134ac03`.
- acceptance_audit: criteria 1 through 6 are proven. Criterion 7 remains open
  solely because live automatic ChatGPT navigation is still forbidden.
- next_action_or_stop_reason: once that exact navigation boundary is lifted,
  use the repository controller to create and pin the Workshop lane, publish
  the source-free candidate under the retained guard, and verify the installed
  command and unchanged target without prompt submission.

## Execution Receipt C04

- plan_version: 4
- state_transition: OPEN -> OPEN
- progress_classification: live_fixture_fail_closed_on_route_capacity
- live_fixture: a disposable localhost page, inactive prior remote-view smoke
  profile, temporary requirement path, and debug candidate exercised the real
  preparation entrypoint. Route-bound acquisition stopped before launch with
  `service_remote_view_route_preflight requires ... an available route pool
  entry`.
- capacity_evidence: Route A is retained by `away-auth-handoff`; Route B is
  retained by `nyse-developer-route`. Both display allocations are classified
  live. No route can be taken without parking or displacing another retained
  operator surface.
- safety_result: the preparation wrapper created no fixture browser, no target,
  no requirement, and no enforcement marker. It did not close, park, or mutate
  either retained route. Structured child failures are now preserved in one
  bounded single-line diagnostic instead of being collapsed to an opaque step
  failure.
- regression_artifact:
  `smoke:local-dashboard-retained-browser-preparation-live` provides the
  disposable live gate for the next available route window.
- next_action_or_stop_reason: live closure requires both an available route or
  an explicitly reviewed non-destructive parking policy and authority to
  navigate the exact Workshop URL. Do not evict either current retained route.

## Execution Receipt C05

- plan_version: 5
- state_transition: OPEN -> OPEN
- progress_classification: final_candidate_relinked_after_live_diagnostic
- artifact_provenance: the bounded structured child-error change is embedded in
  the workstation controller via `include_str!`, so the optimized candidate was
  relinked after the live smoke. Current SHA-256 is
  `fc4fd837feeadbe9da3ffa9c2bab903a4af64005ad2504330e4e46498675bf0e`.
- installed_boundary: installed SHA-256 remains
  `072ba7f998de7da35d94fb6f0f4a3782ea012b7ba1711d913527788c7134ac03`;
  no installation or service restart occurred.
- next_action_or_stop_reason: preserve the candidate and current live routes.
  Live closure remains gated by route capacity and the prior no-navigation
  instruction.

## Execution Receipt C06

- plan_version: 6
- state_transition: OPEN -> BLOCKED
- progress_classification: repeated_external_capacity_and_authority_blocker
- third_live_readback: Route A remains `checked_out` to
  `away-auth-handoff`; Route B remains `checked_out` to
  `nyse-developer-route`. Retained display inventory reports zero apply-safe
  allocations and classifies both route-linked allocations as live.
- authority_readback: both retained-browser authority files remain absent. The
  candidate SHA-256 remains
  `fc4fd837feeadbe9da3ffa9c2bab903a4af64005ad2504330e4e46498675bf0e`;
  installed SHA-256 remains
  `072ba7f998de7da35d94fb6f0f4a3782ea012b7ba1711d913527788c7134ac03`.
- safety_boundary: no route was parked or evicted, no ChatGPT navigation was
  attempted, and no installation or service restart occurred.
- unblock_condition: either free one Guacamole route or explicitly authorize a
  reviewed parking policy, and explicitly lift the prior automatic ChatGPT
  navigation prohibition for the exact Workshop URL. Prompt submission remains
  outside preparation authority.

## Execution Receipt C07

- plan_version: 7
- state_transition: BLOCKED -> OPEN -> BLOCKED
- progress_classification: live_route_proof_complete_exact_conversation_missing
- route_policy: Route A was parked with `parkForRouteSwitch=true` only after
  verifying zero viewer and controller leases. The `away-auth-handoff` browser
  remained ready at PID `521790` with its LinkedIn target unchanged. Route B
  and the `nyse-developer-route` browser remained live and were not displaced.
- deterministic_route_fixes: canonical Guacamole route definitions now carry
  stable route-specific display allocation ids. Workstation validation rejects
  missing or collapsed ids. Checkout now accepts only its exact pending
  browser, session, route, allocation, and acquisition-lease reservation.
- deterministic_session_fix: retained preparation binds the daemon session to
  the runtime-profile id and verifies that exact session in the route response,
  preventing an ambient `default` daemon from satisfying the proof.
- live_fixture: after reconciling the corrected authoritative route pool, the
  disposable localhost preparation completed `fixture_ready_and_pinned` and
  cleaned up its browser and temporary authority.
- live_workshop: navigation to conversation
  `6a7e0704-214c-83ea-8e85-79b3750ae6c5` succeeded through Route A, but ChatGPT
  canonically redirected it to the Workshop `/project` page. Exact live
  reverification failed with `retained_target_url_changed`; no requirement or
  enforcement marker was written and no prompt action was invoked.
- widened_validation: 31 native remote-view tests, 20 native workstation tests,
  focused retained-lane and source-free fixtures, strict Clippy, Rust format,
  client generator check, runtime convergence, dashboard build, docs lint and
  build, JavaScript syntax, package JSON parse, skill parity, optimized build,
  and diff hygiene pass.
- release_candidate: optimized SHA-256 is
  `2bbfe6d53e7553424e88773d67f222f0105d2dc791fe67d5098b2022bd8a3e88`.
  Installed SHA-256 remains
  `072ba7f998de7da35d94fb6f0f4a3782ea012b7ba1711d913527788c7134ac03`.
- unblock_condition: establish or explicitly authorize creation of one real
  Workshop conversation and provide its resulting exact URL. Prompt submission
  remains outside this plan, so publication and installation stay fail-closed.

## Execution Receipt C08

- plan_version: 8
- state_transition: BLOCKED -> OPEN -> CLOSED
- progress_classification: authorized_conversation_creation_and_guarded_publication_complete
- prompt_authority: the operator explicitly authorized exactly one short
  Workshop prompt. One human-scale prompt was typed and submitted once from the
  empty project composer; no retry or second user turn was sent.
- exact_identity: the retained `chatgpt-pro` browser remained PID `1326080`,
  profile `chatgpt-pro`, CDP port `38405`, and page target
  `91DBB20C67DFB0398978722D6B6FA85A`. The resulting canonical conversation is
  `https://chatgpt.com/g/g-p-6a7e016622e48191a60c4bc34366b537-codex-chatgpt-workshop/c/6a80e64e-e830-83ea-b21f-9079abf27a1d`.
- authority_preservation: the pre-existing default retained requirement for
  `nyse-developer-route` was preserved. Workshop authority was committed to the
  separate private requirement
  `local-dashboard-retained-browser-workshop.json`, with SHA-256
  `80d2f5883c47f7bc7ef7370ec0a5b32a11a468a6a668bd440baaa9df98c326f7`;
  both requirement and enforcement files are mode `0600`.
- publication: guarded release publication completed with transaction
  `local-dashboard-5a1ca8ea-e8f6-4f91-88f7-2d4ea6b26d21` terminal at `ready`.
  Retained identity was verified at `final_readiness`; four daemon handoffs
  were prepared and resumed without replacing the retained Chrome process.
- installed_runtime: release and installed SHA-256 both equal
  `7704b89a579e6bb1678d43cbe3d3ea402197a411525cab83e9f6a641228755bb`.
  Runtime convergence reports five current daemon sessions, zero stale
  runtimes, and a ready live dashboard manifest. Remote-view doctor reports
  ready.
- residual_non_blocker: install doctor recommends refreshing the versioned
  source-free workstation payload with `agent-browser install workstation
  --apply --json`; this does not invalidate the current installed binary,
  dashboard runtime, retained lane, or remote-view readiness.
- acceptance_audit: all seven acceptance criteria are satisfied. No GitHub
  write occurred.
