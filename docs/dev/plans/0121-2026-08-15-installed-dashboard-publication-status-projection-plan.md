# Plan 0121 | Installed Dashboard Publication Status Projection

State: CLOSED
Roadmap: P121
Plan version: 12
Date: 2026-08-15

## Objective

Project P120's bounded, read-only local dashboard publication evidence into the
source-free installed runtime, install doctor, authenticated dashboard service,
MCP, software client, and operator UI without allowing status inspection to
authorize recovery.

## Scope

- implement one Rust inspector for the secured journal, exact PID lock,
  bounded transaction metadata, and installed-artifact SHA-256 classification;
- reject invalid, oversized, non-regular, missing, or unknown evidence safely;
- expose status under install doctor and authenticated HTTP/MCP reads;
- publish a versioned JSON schema and service contract metadata;
- add a typed client helper;
- show publication health and exact next action in the Service dashboard;
- keep recovery outside HTTP, MCP, and dashboard mutation surfaces;
- preserve `pnpm recover:local-dashboard-publication` as the explicit reviewed
  recovery-only operation;
- prove isolated reads create no publication directory or browser state.

## Non-Goals

- no live recovery, build, install, dashboard restart, daemon handoff, or
  browser lifecycle action;
- no POST recovery route, MCP recovery tool, automatic doctor remedy, or
  dashboard recovery button;
- no prompt, page, profile, target, credential, or external service mutation;
- no GitHub write or release action.

## Acceptance Criteria

1. Rust and Node status agree on absent, active-lock, recoverable verified,
   unknown-artifact, and terminal/no-op meanings.
2. The Rust inspector bounds journal and artifact reads, requires regular
   files, validates the journal schema, and never exposes handoff descriptors.
3. Install doctor reports status and emits distinct active, recovery-required,
   unknown-artifact, and unreadable-journal issues.
4. Recovery-required doctor evidence names the exact recovery-only command and
   marks it as requiring explicit operator confirmation.
5. Authenticated session HTTP, standalone dashboard HTTP, MCP, and client reads
   return the same schema without acquiring a lock or creating publication
   state.
6. The dashboard displays status and review guidance but has no recovery POST
   or button.
7. Focused, widened, build, contract, plan, cleanup, and diff gates pass.
8. Installed live readback is safe and retains the exact browser identity.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_focused_validation_complete
- discovery_status: CodeGraph reports this checkout is not initialized. Per
  repository policy, no index was created; focused current source, contracts,
  tests, and live P120 evidence supplied the fallback.
- installed_authority: `native/publication_status.rs` validates the journal,
  reads exact lock liveness, streams bounded artifact hashing, projects counts
  instead of handoff descriptors, and fails closed on invalid evidence.
- doctor: `data.localDashboardPublication` and distinct issue codes expose
  status. The recovery remedy names only the explicit recovery-only command and
  carries `requiresExplicitOperatorConfirmation=true`.
- service_projection: authenticated session and standalone dashboard HTTP,
  MCP resource, service-contract metadata, JSON schema, and generated client
  helper expose the same read-only status.
- dashboard: the Service view polls the read-only route, displays a Publication
  status light, and shows reviewed guidance. It has no recovery POST or button.
- focused_validation: five Rust inspector tests, two install-doctor authority
  tests, service-contract metadata, MCP resource listing, generated client
  helper tests, dashboard projection source contract, publisher lifecycle CI
  ordering, isolated no-launch HTTP/client reads, and dashboard production build
  pass.
- live_boundary: no live build, install, recovery, handoff, restart, browser
  interaction, or external write has occurred in P121.
- next_action_or_stop_reason: update operator documentation and skill parity,
  run widened validation and builds, then install and verify only read-only
  doctor/HTTP/MCP status while preserving the retained browser.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> OPEN
- progress_classification: installed_status_verified_retained_browser_blocked
- widened_validation: all 1,859 non-ignored Rust tests pass, with 57 ignored;
  focused publication, doctor, HTTP, MCP, contract, generated client,
  dashboard, publisher lifecycle, orchestration, journal, operations, parity,
  typecheck, production build, and documentation build gates pass.
- installed_publication: guarded release publication completed transaction
  `local-dashboard-383011f9-c4eb-4da7-ae96-dbe1532bc3bf` at revision 15,
  terminal phase `ready`. Installed and release binaries share SHA-256
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- installed_readback: repository status, install doctor, authenticated HTTP,
  and MCP all report schema `agent-browser.local-dashboard-publication.v1`,
  a verified `replacement`, no live lock, no recoverable transaction, and
  recommended action `none`. The dashboard runtime manifest reports 80 assets
  with SHA-256
  `01a2676380d4c7b0b7ba9b5281e77ff02740fcfd675adf9fcd623e62843c78af`.
- authority_boundary: no recovery route, MCP mutation tool, dashboard recovery
  control, prompt submission, navigation, typing, or click was used. The
  authenticated live HTTP check created only a bounded dashboard login session.
- retained_browser_blocker: the previously retained Workshop Chrome PID and
  CDP endpoint are no longer live, and current service state contains no
  matching browser, session, or target record. The exact retained target cannot
  be read back, and starting a replacement would conceal the preservation
  failure and violate the no-navigation verification boundary.
- audit: diff hygiene, JSON parsing, workflow YAML parsing, repository and
  installed skill parity, direct plan/roadmap/runbook/note consistency, binary
  hashes, manifest readback, and publication-lock absence pass. The repository
  has no package plan-audit command. Docs-wide lint retains the existing error
  and warning in untouched docs files recorded by prior receipts.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Diagnose
  the retained-browser disappearance with an isolated publisher and test-HOME
  boundary, then repeat exact target readback against an operator-established
  retained lane without sending a prompt.

## Execution Receipt C03

- plan_version: 3
- state_transition: OPEN -> OPEN
- progress_classification: retention_loss_isolated_and_fail_closed_guard_ready
- timeline: service evidence records the Workshop browser process exit at
  `2026-08-15T05:55:19.961014375Z`. The guarded publication transaction began
  at `2026-08-15T06:03:01.334Z`, so publication did not cause the exit. Kernel,
  system journal, and Chrome stderr contain no termination cause. The publisher
  nevertheless treated the surviving browserless daemon as idle and could not
  prove the acceptance target existed before mutation.
- retained_browser_guard: publication can now require an exact session,
  profile, target, URL, and optional initial CDP endpoint. Before backup or
  quiescence it reads the live CDP target inventory, then pins browser PID, CDP
  endpoint, profile, target, and URL. The same identity is mandatory after
  handoff, at final readiness, and during crash recovery.
- read_only_preflight: `pnpm check:local-dashboard-retained-browser --` runs the
  same exact-identity check without acquiring the publication lock, building,
  launching a daemon or browser, or changing journal state. Missing daemon PID
  evidence fails before invoking agent-browser, preventing auto-launch.
- validation_isolation: `pnpm test:rust-isolated` supplies a disposable HOME,
  agent root, socket directory, and runtime directory, retains the real Cargo
  and Rustup stores, and forces one test thread. The full suite passes 1,861
  tests with 57 ignored and leaves live publication and Workshop inventory
  hashes unchanged.
- status_projection: installed status contracts add only bounded guard-required,
  guard-verified, and stage fields. Exact session, PID, endpoint, profile,
  target, and URL stay private. A terminal transaction lacking final proof now
  recommends `investigate_retained_browser` and emits a distinct doctor issue.
- regressions: focused guard evaluation covers missing browser, PID, CDP,
  profile, target, URL, and unreachable inventory; orchestration covers
  pre-mutation refusal, post-handoff drift, final success, and recovery refusal.
  Read-only operation coverage proves no state-tree change for a missing daemon.
- widened_validation: focused Node and Rust gates, full isolated Rust suite,
  formatting, strict Clippy, API/MCP parity, generated clients, service
  contracts, dashboard and docs production builds, publisher lifecycle,
  orchestration, journal, operations, and smoke policy pass. Docs-wide lint
  retains the existing untouched warning and error recorded by C02.
- release_candidate: current release binary SHA-256 is
  `03efdddf4bf80cdeb377e549f2d1ad818e493e6248a92aa661a32afdda9d39e9`.
  It is intentionally not installed; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_fail_closed_proof: the exact Workshop preflight returns
  `retained_daemon_missing` while service state, publication journal, session
  inventory, and lock state remain unchanged. No replacement browser, prompt,
  navigation, typing, or click occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once the
  operator-established retained Workshop lane exists again, run the read-only
  exact preflight, then the guarded publish with the same identity flags and
  prove installed status plus exact retained-target readback without a prompt.

## Execution Receipt C04

- plan_version: 4
- state_transition: OPEN -> OPEN
- progress_classification: durable_critical_lane_authority_and_watchdog_ready
- durable_requirement: a verified live exact lane can be pinned into a private,
  bounded, owner-only, non-symlink v1 record. The record contains stable
  session, profile, target, and canonical URL only; PID, browser ID, and CDP
  endpoint are rejected.
- automatic_enforcement: normal publication loads the durable requirement and
  rejects missing, invalid, or conflicting identity before dashboard or Rust
  build. Explicit flags can strengthen but cannot weaken the durable identity.
- watchdog: the read-only check returns `not_configured` without creating state
  when no critical lane is pinned. The source-checkout user-service interlock
  runs the check before convergence and fails before mutation when a configured
  lane no longer matches.
- focused_validation: requirement security, idempotence, conflict, watchdog,
  publication operation, orchestration ordering, and local convergence fixtures
  pass. Durable missing-lane publication is proven to fail before build.
- widened_validation: all 1,861 non-ignored Rust tests pass with 57 ignored;
  strict Clippy, Rust formatting, client typecheck, dashboard build, docs build,
  schema and workflow parsing, shell syntax, installed skill parity, and diff
  hygiene pass. Root JavaScript lint is unavailable because the repository has
  no root ESLint configuration. Docs lint retains the existing untouched error
  and warning recorded by C02 and C03.
- release_candidate: current release binary SHA-256 is
  `68303e1b9c89aa7e8a1f81cf7e646ac0f570208cf5c2559635cb186f95f40381`.
  It is intentionally not installed; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_boundary: the Workshop lane remains absent. No requirement was pinned,
  no runtime was installed, and no browser launch, prompt, navigation, typing,
  or click occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once an
  operator-established Workshop lane exists, verify and pin it, then perform
  the guarded publication and exact no-prompt installed readback.

## Execution Receipt C05

- plan_version: 5
- state_transition: OPEN -> OPEN
- progress_classification: native_source_free_retention_interlock_ready
- authority_gap_closed: `install workstation --apply` now checks the durable
  requirement before lock acquisition, sudo, payload staging, or service
  quiescence. Binary-owned `install workstation reconcile` repeats the native
  check immediately before its first mutation, so the installed timer no
  longer depends on repository Node tooling.
- native_verification: bounded owner-only non-symlink requirement and service
  state reads require one exact ready browser, active session, valid service-tab
  handle, profile, live PID, loopback DevTools target, and canonical URL. The
  verifier calls no service command and cannot auto-launch a daemon or browser.
- regressions: nine focused Rust tests prove exact success plus changed browser,
  session, service handle, profile, non-ready health, changed rendered URL,
  ambiguous or missing target, insecure permission, and symlink refusal. The
  source-free workstation
  fixture proves both apply and reconcile ordering and proves an invalid durable
  lane creates no lock, payload, or service command.
- widened_validation: all 1,870 non-ignored Rust tests pass with 57 ignored;
  strict Clippy, Rust formatting, retained-lane Node fixtures, source-free and
  host workstation fixtures, service-client type and contract checks, dashboard
  and docs production builds, installed skill parity, release fixture, and diff
  hygiene pass. Targeted MDX lint reports only that MDX has no matching ESLint
  configuration.
- release_candidate: current release binary SHA-256 is
  `b8eaac13578f3380aca701e4ea127d2653b4d2a22fdbb73a1172e8738fc474dc`.
  It is intentionally not installed; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_boundary: the durable requirement and publication lock are absent. No
  installation, browser launch, prompt, navigation, typing, click, or GitHub
  write occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once an
  operator-established Workshop lane exists, verify and pin it, then perform
  the guarded publication and exact no-prompt installed readback.

## Execution Receipt C06

- plan_version: 6
- state_transition: OPEN -> OPEN
- progress_classification: source_free_retention_status_and_config_parity_ready
- native_status: `agent-browser install workstation
  retained-browser-status --json` exposes the installed controller's exact
  native check without a workstation lock, apply, reconcile, daemon call,
  browser launch, or state creation. Output contains bounded configured,
  verified, state, path, and no-launch fields only.
- config_parity: native status, apply, and reconcile honor
  `AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT`. The normal dotenv loader now
  admits that key from `AGENT_BROWSER_ENV_FILE` or
  `~/.agent-browser/.env`, matching repository publication tooling without
  manual flags or exports.
- regressions: ten native verifier tests cover default and overridden paths,
  exact identity, drift, ambiguity, unsafe files, and no-launch status. The
  source-free workstation fixture proves default, direct-env, and dotenv path
  resolution create no state and that invalid configured identity still blocks
  before apply mutation.
- widened_validation: all 1,871 non-ignored Rust tests pass with 57 ignored;
  strict Clippy, Rust formatting, source-free workstation fixture, dashboard
  build, docs build, installed skill parity, and diff hygiene pass.
- live_no_launch_readback: the optimized native status command reports
  `not_configured` at the default private path. Service state and publication
  journal SHA-256 values are byte-identical before and after the command.
- release_candidate: current release binary SHA-256 is
  `ce5a783994a8744de27e6f59c0be145ee64de78a4379248c50868b3dcddd7835`.
  It remains uninstalled; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_boundary: no installation, browser launch, prompt, navigation, typing,
  click, or GitHub write occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once an
  operator-established Workshop lane exists, verify and pin it, then perform
  the guarded publication and exact no-prompt installed readback.

## Execution Receipt C07

- plan_version: 7
- state_transition: OPEN -> OPEN
- progress_classification: marker_first_retention_enforcement_ready
- crash_ordering: pinning durably creates a separate private `.required`
  enforcement record before the stable-identity requirement. A simulated crash
  after the first commit is retryable and cannot be misclassified as an
  unconfigured lane.
- deletion_interlock: once the marker exists, a missing requirement fails
  closed in repository publication, the read-only watchdog, native status,
  workstation apply, and recurring reconcile. Invalid marker schema,
  permissions, ownership, size, or symlink shape also fail closed.
- regression_scope: focused Node coverage proves marker-first crash recovery,
  idempotent retry, requirement deletion refusal, unsafe marker permissions,
  marker symlink refusal, and no-build watchdog ordering. Native tests and the
  source-free workstation fixture prove the same enforced-missing and
  unsafe-marker boundary before state creation or service commands.
- widened_validation: all 1,873 non-ignored Rust tests pass with 57 ignored;
  Rust formatting, production-binary strict Clippy, retained-browser and
  publisher fixtures, dashboard and docs production builds, schema parsing,
  installed skill parity, and diff hygiene pass. Full all-target strict Clippy
  remains blocked by 12 unrelated warnings in existing test code outside this
  slice.
- live_no_launch_readback: the optimized native status command reports
  `not_configured`, `enforcementConfigured=false`, and the private marker path.
  Service-state SHA-256
  `fa8067d2756dba729fadce2fce174290fd5b06c749cc84b8569dfd890f664ecc`
  and publication-journal SHA-256
  `a3efa8ef2e6a0e3bb1aec6247a8328fdfb9793b65b5bbdad87c5118c45a79fa7`
  are byte-identical before and after.
- release_candidate: current release binary SHA-256 is
  `ab15748e93330c2c4d61d1bc6b0ff2de18b86be3852869a2f83b657424b13165`.
  It remains uninstalled; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- authority_boundary: loss of only the requirement is detected. Deliberately
  deleting both private authority files removes enforcement and remains an
  administrative operation outside this crash-loss interlock.
- live_boundary: no installation, browser launch, prompt, navigation, typing,
  click, or GitHub write occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once an
  operator-established Workshop lane exists, verify and pin it, then perform
  the guarded publication and exact no-prompt installed readback.

## Execution Receipt C08

- plan_version: 8
- state_transition: OPEN -> OPEN
- progress_classification: digest_bound_retention_enrollment_ready
- pair_authority: the marker-first enforcement record now precommits the
  lowercase SHA-256 of the exact allowed requirement bytes. Node and native
  readers compare that digest before consulting service state, so a stale
  marker cannot authorize a replaced stable-identity record.
- crash_retry: an interrupted writer reuses the committed marker timestamp and
  digest. The same verified identity completes idempotently; different evidence
  conflicts with the commit and leaves the requirement absent.
- compatibility_boundary: a legacy requirement without an enforcement marker
  remains readable and is digest-bound when next pinned. Every v1 enforcement
  record now requires `requirementSha256`; this branch has not installed the
  prior marker-only candidate and live state contains neither authority file.
- regressions: Node coverage proves exact digest publication, crash retry,
  changed-evidence refusal, byte replacement refusal, and watchdog behavior.
  Thirteen native focused tests include digest mismatch before service-state
  access. The source-free installer fixture validates enforced-missing behavior
  against the rebuilt release binary.
- widened_validation: all 1,874 non-ignored Rust tests pass with 57 ignored;
  production-binary strict Clippy, Rust formatting, publisher operations and
  orchestration, source-free workstation, dashboard and docs production builds,
  schema parsing, installed skill parity, and diff hygiene pass.
- live_no_launch_readback: optimized status reports `not_configured` with no
  enforcement. Service-state SHA-256
  `c7b92949339e831e65e6f317bace89dc0796bb0065a93a743d095e78d4353c7a`
  and publication-journal SHA-256
  `a3efa8ef2e6a0e3bb1aec6247a8328fdfb9793b65b5bbdad87c5118c45a79fa7`
  are unchanged before and after.
- release_candidate: current release binary SHA-256 is
  `f129a45f9e7b70506808723637432038c6b9e94ccf9327ab24cb8c858f6b88b3`.
  It remains uninstalled; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_boundary: no installation, browser launch, prompt, navigation, typing,
  click, or GitHub write occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Once an
  operator-established Workshop lane exists, verify and pin it, then perform
  the guarded publication and exact no-prompt installed readback.

## Execution Receipt C09

- plan_version: 9
- state_transition: OPEN -> OPEN
- progress_classification: workstation_remote_view_substrate_and_live_gate_ready
- workstation_reconciliation: the post-relogin installed controller completed
  with distinct route-specific XRDP desktops on `:10` and `:11`. Install doctor
  and remote-view doctor report ready with no issues; the dashboard, runtime
  interlock timer, and PostgreSQL backup timer are active.
- route_viewer_binding: source reconciliation now binds the Guacamole viewer
  daemons to the installed Linux Chrome after browser installation. This keeps
  an ambient Windows StealthCDP default from being selected for a Linux XRDP
  display. A focused Rust regression proves replacement is exact and
  duplicate-free.
- fixture_liveness: the local remote-view fixture server now runs in a worker,
  so the smoke's synchronous CLI child cannot block HTTP responses. A
  deterministic regression fetches the fixture from a synchronous child while
  the parent thread is blocked.
- rendered_live_proof: the optimized release candidate passes
  `pnpm test:remote-view-open-live -- --fixture` with an explicit installed
  Linux browser binding. Evidence includes exact fixture URL and title, matching
  X11 browser PID, `route_bound_ready`, `browser_window_visible`, one active
  intended target after CLI and HTTP repeat opens, OCR marker proof, and
  automatic disposal of the isolated session and profile.
- focused_validation: 20 workstation Rust tests, the blocking-safe fixture
  regression, source-free workstation fixture, JavaScript syntax checks, Rust
  formatting, production-binary strict Clippy, and diff hygiene pass.
- release_candidate: current release binary SHA-256 is
  `a886febac4a3794bd0b20387e603406b319e9419c2047cd2b1b15cd404450eec`.
  It remains uninstalled; the installed binary remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
- live_boundary: only disposable local fixture browsers were launched. No
  ChatGPT navigation, prompt, typing, click, retained-lane pin, or GitHub write
  occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. Use an
  operator-established Workshop conversation URL to create the exact retained
  lane, pin it, install the guarded release candidate, and prove the same target
  survives with no prompt submission.

## Execution Receipt C10

- plan_version: 10
- state_transition: OPEN -> OPEN
- progress_classification: unique_prefix_retained_lane_enrollment_ready
- operator_handoff: requirement pinning now accepts one reviewed origin and
  path prefix, discovers current live retained daemon and CDP targets, and
  derives the exact stable session, profile, target, and canonical URL.
- fail_closed_selection: enrollment requires exactly one `page` target owned by
  a live `health=ready` session. Zero or multiple matches, service or CDP read
  failure, mismatched browser/session identity, incomplete profile or PID
  identity, degraded health, and origin or path-prefix drift stop before either
  private authority file is written.
- guard_parity: repository publication now requires `health=ready`, matching
  the native installed verifier instead of accepting a merely nonterminal
  browser health value.
- focused_validation: discovery selection, durable requirement, and JavaScript
  syntax fixtures pass. Source-free workstation, Rust format and strict Clippy,
  docs production build, and optimized release build also pass. Current
  no-launch service state contains no ChatGPT target and no authority was
  pinned.
- fixture_isolation: widened convergence validation exposed an inherited
  `XDG_RUNTIME_DIR` leak that let the temporary-home fixture see an unrelated
  live `etf-paper-dashboard` token. The fixture now binds an explicit isolated
  `AGENT_BROWSER_SOCKET_DIR`, and its full rerun passes without consulting live
  session metadata.
- release_candidate: current release binary SHA-256 is
  `0e2186376c0795d6df684797f779ce94400f99fb8d582c68b03741918f306acf`.
  Live drift reconciliation found a separate terminal publisher transaction at
  13:58 that installed an earlier unguarded candidate with SHA-256
  `758c9f4d4e89941799dbe053357e50597e13b633e3e6013f8a5e7259d39b2984`.
  No retained browser existed during that transaction.
- live_boundary: no browser launch, ChatGPT navigation, prompt, typing, click,
  retained-lane pin, installation, or GitHub write occurred.
- next_action_or_stop_reason: keep P121 open at acceptance criterion 8. After
  the operator opens the intended Workshop conversation, enroll it using the
  reviewed Workshop project URL prefix, install the guarded release candidate,
  and prove the exact target survives without prompt submission.

## Execution Receipt C11

- plan_version: 11
- state_transition: OPEN -> OPEN
- progress_classification: manual_navigation_gap_split_to_p122
- finding: the managed Route B browser was ready, but repeated operator handoff
  landed in an ambient browser and left the managed profile on `about:blank`.
  Prefix discovery correctly returned no match and wrote no authority files.
- scope_boundary: P121 retains its read-only discovery non-goal. P122 owns a
  separate guarded exact-URL open and pin operation with no prompt authority.
- next_action_or_stop_reason: complete P122 validation, then use its guarded
  operation for the live Workshop lane only after live navigation is allowed.

## Execution Receipt C12

- plan_version: 12
- state_transition: OPEN -> CLOSED
- progress_classification: installed_exact_browser_readback_complete_via_p122
- dependency_closure: P122 established and pinned the exact retained Workshop
  lane, then published the guarded release with that identity required through
  final readiness.
- installed_readback: install doctor reports publication transaction
  `local-dashboard-5a1ca8ea-e8f6-4f91-88f7-2d4ea6b26d21` terminal `ready`,
  retained expectation required and verified at `final_readiness`, installed
  artifact classification `replacement`, and recommended action `none`.
- exact_browser: read-only retained status reports PID `1326080`, session and
  profile `chatgpt-pro`, CDP target `91DBB20C67DFB0398978722D6B6FA85A`,
  ready health, and canonical Workshop conversation
  `6a80e64e-e830-83ea-b21f-9079abf27a1d`.
- runtime: release and installed SHA-256 both equal
  `7704b89a579e6bb1678d43cbe3d3ea402197a411525cab83e9f6a641228755bb`;
  live dashboard manifest and remote-view doctor are ready, with zero stale
  daemon runtimes.
- acceptance_audit: criterion 8 is now proven and all P121 acceptance criteria
  are satisfied. No recovery surface or GitHub write was added.
