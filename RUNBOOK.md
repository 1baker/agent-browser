# Runbook

## 2026-09-13 | Healthy maintenance installed and live-verified

Full partitioned Rust suite (`bash scripts/ci/rust-tests.sh`) passed after
synthetic gate isolation, without skips added or production privacy rules
weakened. Primary also repeated all 17 CDP tests five times in parallel mode
(85 successful executions), ran five private-execution tests, 29 workstation
tests, all ten command-boundary fixture scenarios, source-free installation
coverage, and isolated install/reinstall with six extracted controller tests.
Rust format, strict Clippy in both checkouts, scoped build, JS lint, and diff
checks passed. Existing ignored tests remain outside this unit-suite claim.

Guarded publication `local-dashboard-00a04b57-bace-4a7d-a367-e532e990f11b`
completed terminal `ready`, updating binary and installation provenance together.
Installed SHA: `3d2e7954865610e9f4fccb006a411aadb6b218379b4f82f2eca67b04e3f861e4`.
Manifest SHA: `e407882b28c3571b69d895cb9bd3f86007955b3e3e03832169b9a6f2394351c0`.
Rollback binary: `/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260913013608`.
Paired provenance: `/home/bak3r/.agent-browser/publications/.workstation-provenance-eNOx8m`.
Final publication doctor passed with raw success, no degradation, no repair
requirement, and ready payload. Workshop browser PID `505779`, exact target
`4C4278BF20BBD5794A62582E9622E7D8`, profile, URL, and CDP endpoint were preserved;
viewer browser PIDs `495544` and `496206` were preserved by verified handoffs.

Primary ran two installed reconcile commands followed by the actual
`agent-browser-runtime-interlock.service`. All three returned
`healthy-runtime-preserved`. Across every pass the dashboard PID remained
`167378`, its monotonic start timestamp remained `89700804247`, and the exact
four-session inventory was unchanged. No transient default daemon appeared.
The actual interlock completed with `ExecMainStatus=0`; its timer remained
active. The durable convergence receipt contains only retained verification
and healthy preservation steps. Final no-launch retained identity check passed.
No ChatGPT prompt, image-task interruption, or GitHub write occurred.

This closes the maintenance interruption and synthetic privacy-test blockers.
The wider integration goal remains open: next verify AuraCall control-plane
attestation and independently downloaded artifacts from the already completed
response, without replaying a prompt or taking a different conversation target.

## 2026-09-13 | Synthetic privacy-gate isolation

The three remaining failures were traced to synthetic localhost endpoints
using production gate storage. Gate identity intentionally uses scheme, host,
and port. Tests intentionally leave unfinished private intervals or pending
command receipts; OS port reuse then exposes another test to that durable
state. The observed `privacy_gate_locked` and `privacy_gate_pending` are
correct fail-closed results, not permission to clear operator gate records.
Representative failures passed alone, and an explicit same-URL fixture
regression now covers isolation without weakening sibling/reconnect guards.

Added test-only `TestCdpEndpoint` with UUID temporary gate storage and a real
endpoint-bound PrivacyGate. Sibling connections share the fixture gate; separate
fixtures never share it. Production endpoint resolution, leases, observers,
and private recovery checks are unchanged; only the post-resolution connection
body was extracted for reuse. Converted synthetic transport tests in CDP client,
clipboard, browser, inspect server, private attachment/execution, and stream
WebSocket modules. The private-journey end-to-end fixture retains its explicit
isolated-runner boundary and production endpoint resolution.

Delegated `/root/privacy_test_failure_review` supplied source-backed diagnosis
and implemented client/gate test infrastructure without builds or live actions.
Primary reviewed the production diff, migrated sibling test modules, and ran
17 CDP tests plus five private-execution tests successfully. No operator gate
files were deleted. Subsequent full-suite and installed validation are recorded
in the completed maintenance entry above.

## 2026-09-13 | Recurring workstation restart diagnosis

Primary live inspection confirmed the installed interlock invokes
`install workstation reconcile` every five minutes. The native reconciler
unconditionally quiesces the dashboard and both timers before provisioning,
even when the preceding install doctor reports success, no issues, and a ready
payload. During a scheduled pass, dashboard PID was zero and remote-view doctor
reported `install_dashboard_runtime_stale_or_unreadable` despite both canonical
routes passing readiness. After the pass, dashboard PID `4190117` was active.
This proves maintenance-induced interruption; it does not implicate the user's
image generator. No user browser or image task was stopped by this inspection.

Bounded remediation is implemented but not installed: check fresh installed
provenance, retained identity, service health, and canonical route authority
before any quiescence. A healthy pass preserves running services and browser
lifecycle and updates only its convergence receipt. Existing
remote-view doctor performs Guacamole authentication probes, so this is not a
claim of zero authentication-token writes. Unknown or unsafe evidence must not
authorize repair. The existing source-free workstation fixture passed before
the change. Delegated lane `/root/reconcile_fastpath` supplied the source-backed
diagnosis and implementation. Independent `/root/healthy_reconcile_review`
identified R1: unready remote evidence could bypass active route comparison.
Primary accepted R1; remediation now requires canonical route evidence and
active-conflict validation before every repair decision. The reviewer verified
R1 closed by source inspection; primary independently ran the command fixture.

Primary validation: all 29 workstation Rust tests passed. All ten synthetic command-boundary scenarios passed on the
scoped candidate, including repeated healthy checks, fresh receipt readback,
provenance/publication/unknown/incomplete evidence refusals, active conflicts
with both ready and unready remote state, and stopped-service/configuration
repair admission. The source-free install fixture and isolated install/reinstall
manifest verification passed; six extracted controller tests passed. Full JS
lint, Rust format, scoped strict Clippy/build, and diff check passed.

Candidate SHA: `6d62661bd3672c1b714de07e0a5cef1323ff0e7956165c017776063e80c4d0c5`
at `/home/bak3r/projects/agent-browser-startup-20260911/cli/target/debug/agent-browser`.
Only this slice's workstation source and help change were copied into that
scoped checkout; unrelated main-worktree changes were not bundled.

Installation held: partitioned parallel tests failed three privacy-gate tests.
An initial serial rerun overlapped a test-executable rebuild and is not accepted
as reliable subprocess-crash evidence. A second serial full-suite run with no
rebuild completed with 2,089 passed, three failed, and 74 ignored. Failures:
`native::cdp::client::tests::private_quiescence_cancels_live_pending_reply_without_publication`,
`native::cdp::client::tests::private_quiescence_old_proof_cannot_authorize_new_epoch`,
and `native::private_execution::tests::private_transport_admits_once_encrypts_result_and_never_broadcasts_secret`.
Observed errors include `privacy_gate_locked` and `privacy_gate_pending`.
Do not claim full-suite green or installed maintenance preservation until these
are reconciled and the candidate is published and live-tested. The installed
cold-start binary remains `088989f15d6c60c46af35a9b444b25e6f2e4220d53adc934e3ec5dd7da03b893`.
The latest inventory again included a transient default daemon while the
interlock was activating; service jobs had zero nonterminal entries. No
publication bypass, ChatGPT submission, GitHub write, or image-task stop occurred.

## 2026-09-13 | Cold MCP startup installed and verified

User confirmed proceeding after the inventory pause. Primary refreshed live
state: no nonterminal service jobs; the transient default daemon appeared and
disappeared while the scheduled runtime-interlock service was active. The
interlock's receipt records user-unit quiescence and reactivation; dashboard
journal restarts match its five-minute timer. This establishes maintenance
activity, not attribution to the user's image generator. No image task was
stopped. After the interlock finished, the original four-session inventory
matched and normal guarded publication proceeded without bypass.

Publication `local-dashboard-d2d4f366-a118-480b-a7ea-42dc262a36d1` completed
terminal `ready`. Installed binary SHA:
`088989f15d6c60c46af35a9b444b25e6f2e4220d53adc934e3ec5dd7da03b893`.
Matching installation manifest SHA:
`6ebb060b0d9dffd8168e7482ba62aeb0518e58a4cba113f7dfc32c048a881f85`.
Rollback binary:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260913004856`;
paired provenance:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-Ym2yiE`.
Final install doctor passed without degradation or repair requirements.
Workshop PID `505779`, exact target `4C4278BF20BBD5794A62582E9622E7D8`,
profile and CDP endpoint were preserved. Viewer PIDs `495544` and `496206`
were preserved. Post-test retained-browser-status independently verified.

Primary live-tested the installed executable with fresh isolated MCP clients:
default navigate, canonical tab_new, explicit profile alias overriding default,
repeat retained read and close all passed. Cold-admission negative fixture
passed without effects. Installed daemon crash/recover/detach/reconnect/terminal
close also passed: disposable Chrome PID `4099524`, target
`362F1FD7ED1796B74EF7359D0B97A066`, exactly one backend commit, retained mapping
preserved on detach, terminal process exit and profile-lock release confirmed.
No ChatGPT prompt or GitHub write. Existing MCP processes are not claimed to
have reloaded new code automatically.

Remaining integration work: investigate unnecessary maintenance churn and
verify AuraCall control-plane attestation plus independent document downloads.
The broader seamless-operation goal is not complete merely because cold startup
and the recovery regression now pass on the installed binary.

## 2026-09-13 | Cold MCP acquisition candidate verified

Implemented cold startup across `main.rs`, `mcp.rs`, `connection.rs` and
`native/daemon.rs`. Fresh validated `navigate`/`tab_new` requires a URL, exact
selected profile and an available, unblocked new-browser access plan with no
active lease, compatible browser or daemon metadata. Existing/retained reads
and input do not gain bootstrap authority. Standard configured launch values
are passed explicitly; explicit profile aliases and nested fields win defaults.
Cold startup uses OS-released startup serialization, exclusive metadata/token
creation, no socket unlink and exact child PID/token readiness. It dispatches
once without transport retry. Existing ordinary recovery is preserved.

Primary accepted delegated implementation from `/root/mcp_cold_bootstrap` and
independently ran seven focused tests plus the full partitioned Rust suite:
2,089 passed, 74 ignored, zero failures. Format, strict Clippy and new fixture
lint pass. Fresh reviewer `/root/cold_start_review` found one blocking profile
alias precedence issue; primary fixed it with nested-field precedence coverage.
Reviewer verified that remediation in source; primary reran the focused suite
and tested the explicit-alias/configured-default case live successfully.

Main candidate live proofs: fresh real MCP default `navigate` and canonical
`tab_new`, exact new profile, one browser, repeated read through retained default
route and shutdown/lock release all pass. The cold-admission no-launch fixture
passes: read/input, retained hints, competing lease and stale metadata refuse
without state changes. Recovered-terminal-close live regression also passes on
main SHA `acd1bb6af39a01bc7be8d1993baded4fd2f9f1e913fc1ea6f34b6115c66fc970`,
Chrome PID `4048422`, target `483FF044BFC7045163F91732BB80FC32`, one commit.

First positive fixture lacked isolated installed-Chrome cache identity and
correctly failed the stock-build proof gate after launch. The fixture now links
the real installed Chrome into its isolated cache, as the existing recovery
fixture does. The failed disposable browser was closed through its exact owned
session; no guard was weakened or production browser adopted. Test artifacts
are isolated from real HOME, socket, profile and service state.

Only the four startup files and cold-start help lines were applied to the
existing scoped installation checkout. Guarded publication/live installed
acceptance is pending below. No ChatGPT prompt or GitHub write.

Scoped candidate SHA:
`088989f15d6c60c46af35a9b444b25e6f2e4220d53adc934e3ec5dd7da03b893`.
Primary live-tested scoped navigate, tab_new and explicit profile alias; all
passed. Scoped cold-admission and cold-profile tests, strict Clippy, format and
diff checks passed. Main full JS lint passed; CodeGraph synchronized seven files.

Guarded installation was refused before mutation: expected four retained
sessions, but `default` appeared in the live daemon inventory during preflight.
An immediate read confirmed five sessions; a later read returned the original
four and default socket/PID metadata was absent. Its creator was not established.
No daemon was closed or silently added to the approved publication inventory.
Installed SHA remains
`9faf73ed3da8cc0366a329822813609a8a078e4b681e22c1101caeee1ba22086`.
Previous publication transaction remains terminal ready, with no new journal,
backup, replacement, handoff or dashboard restart. Retained Workshop identity
passed pre-mutation verification. A quiet, confirmed client window is needed
before repeating guarded installation; do not bypass inventory admission.

## 2026-09-13 | Seamless AuraCall goal: cold MCP navigation reproduced

Continuation checkpoint: canonical broker `tab_new` also reproduced the missing
default-daemon failure with exact `runtimeProfile=fresh-profile`; isolated
fixture `/tmp/ab-mcp-default-5NtKRw`. Both acquisition actions must pass, not
only custom navigation. New no-launch `test-mcp-cold-admission.js` passed on
installed runtime: read/input, missing retained route hints, competing profile
lease and stale metadata are refused with unchanged state and no daemon
artifacts. Both new fixtures pass targeted ESLint. Implementation is delegated
and in progress; no installation or successful cold-launch claim yet.

Previous goal slice made verified progress: installed daemon crash recovery and
explicit terminal shutdown passed. Full integration remains open, not flawless.
Primary reran installed `test-mcp-cold-profile.js` and
`test-mcp-retained-route-unavailable.js`: both pass. Thus profile creation and
retained-route refusal are resolved; old references to that setup gap are
superseded by this narrower first-navigation finding.

New `scripts/test-mcp-default-launch-live.js` uses real MCP stdio, isolated
HOME/socket/config/profile and an explicit installed Linux Chrome path. Profile
upsert and no-launch access planning succeed without a daemon. First supported
`service_request` navigation fails in `send_queued_tool_command`/`send_command`:
missing default socket, five connect retries, no daemon/browser launched.
Reproduced on installed SHA
`9faf73ed3da8cc0366a329822813609a8a078e4b681e22c1101caeee1ba22086`.
Failed isolated state `/tmp/ab-mcp-default-KeQqzp` has one succeeded profile job,
one profile, zero browsers and no socket artifacts. The first fixture revision
used unsupported action `launch`; that validation error is not a product bug.
The supported `navigate` reproduction is the accepted finding. The fixture now
also requests the exact registered runtime profile and will require that binding
once navigation reaches a daemon. It is a failing regression, not green coverage.

Next implementation must admit only clearly authorized cold launch requests
before transport, with fresh access-plan/profile/metadata checks. Do not retry
effects after transport uncertainty, bootstrap an absent retained route, repair
stale metadata implicitly or launch over another profile lease. Preserve config
host/build/profile choices. Root owns fixture and integration; delegated
`/root/mcp_cold_bootstrap` owns bounded design discovery and, after review,
the bounded MCP/startup seam. Design inspection found startup currently
overwrites PID/version/SHA metadata and unlinks the socket in daemon.rs.
Primary approved the minimal four-file extension (main, connection, MCP,
native daemon) for configured startup context and cold-only nonreplacement.
No runtime change or new ChatGPT prompt in this slice.

Fresh access-plan evidence for the unauthenticated fixture recommends
`launch_new_browser` with zero leases/browsers, while its top-level action is
`verify_or_seed_profile_before_authenticated_work`. Admission must use actual
available/unblocked launch capability plus reuse policy; requiring authenticated
freshness for a local unauthenticated page would be an incorrect new blocker.
Live Workshop no-launch access plan separately returned `wait_for_profile_lease`
with one active lease. Primary respected that result and did not attach or
navigate. This is not a failed recovery or permission to create another lane.

Primary also passed `pnpm test:service-client`: generated contract checks,
JavaScript typecheck, package exports, request/observability helpers, managed
profile workflow and broker/composed examples. These are no-launch/source
contracts, not proof that the cold MCP first navigation works. New fixture lint
and diff checks pass; its live acceptance remains red until the startup fix.

Independent provider-free AuraCall test lane `/root/auracall_test_map` reported
11 suites, 181 passed, one skipped across bridge, response binding, reattach,
attachment, retained-project routing and required document-set tests. Primary
accepted this as delegated source evidence, not installed integration proof.
Primary inspected current AuraCall Plan 0359: it remains OPEN for missing
control-plane attestation and independent DOCX/PDF downloads (ZIP alone was
materialized). These are next integration checks after cold navigation; no
existing provider response may be replayed. Broad live completion is unproven.

## 2026-09-13 | Recovered terminal close verified in scoped candidate

Supersedes the source-only terminal-cleanup limitation below. `actions.rs`
preserves retained identity on ordinary detach and provides explicit recovered
`service_browser_close` through the existing CDP connection, without PID signals.
`runtime_attach_proof.rs` exposes the existing exact Linux process verifier.
`service_health.rs` commits terminal cleanup only if custody is unchanged.
Borrowed/shared custody, process/profile/endpoint drift, uncertain exit or lock
release and persistence errors fail closed. Unsupported physical proof also
refuses shutdown rather than guessing ownership.

Reusable regression: `pnpm test:recovered-terminal-close-live`, with explicit
`AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD` and
`AGENT_BROWSER_SMOKE_CHROME_PATH`. It isolates HOME, service state, daemon socket,
profile and loopback page; only its exact verified daemon receives SIGKILL.
It proves one backend commit, unchanged Chrome PID/target/CDP after recovery,
detach mapping preservation, second reconnect, CDP terminal shutdown, process
exit, profile lock release and terminal mapping removal. Successful fixture
data is removed; failed evidence is retained without forced recovery.

Primary ran the final scoped candidate regression successfully:
SHA-256 `9faf73ed3da8cc0366a329822813609a8a078e4b681e22c1101caeee1ba22086`,
Chrome PID `3932402`, target `2BC7A826D226F58F57F136C088F89673`, commit count one.
Earlier main/scoped candidate runs passed as well. The first fixture revision
incorrectly expected targetId from CLI tab-list output; corrected to a read-only
exact disposable endpoint target listing. Its pre-crash browser was closed via
the owned session close path; no retained production target was involved.

Delegated implementation: `/root/terminal_close_impl`, actions and health only.
Primary reviewed and independently ran the focused tests, full partitioned Rust
suite, Rust format, strict Clippy, build, full JavaScript lint and diff check.
Fresh read-only `/root/terminal_close_review` identified an omitted newly shared
session check. Primary classified it blocking and added a fresh pre-close
identity check plus locked terminal exclusivity check and regression. Reviewer
verified that accepted fix in source; primary reran focused and live tests.
Final full-suite rerun passed: 2,082 passed, 74 ignored, zero failures. Final
main and scoped strict Clippy checks passed, as did format and full JS lint.

Guarded publication completed with transaction
`local-dashboard-14683cdf-3308-4b2f-970c-922d2c0fa2e1`, phase `ready`.
Installed binary SHA matches the final scoped candidate above; installation
manifest SHA is `8cae6c0131608a3957305ccbc25c29d775bdacd6599887600b8ce762a5e3bd56`.
Rollback binary is
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260913001826`,
with paired provenance snapshot
`/home/bak3r/.agent-browser/publications/.workstation-provenance-8PNgdP`.
Final publisher doctor and independent `install doctor --json` both succeeded.
Workshop Chrome PID `505779`, profile `chatgpt-pro`, exact target
`4C4278BF20BBD5794A62582E9622E7D8` and endpoint were unchanged before/after
handoff and at final readiness. Viewer Chrome PIDs `495544` and `496206` were
also preserved. Publication restored its runtime interlock.

Primary repeated the complete live regression against the installed binary:
Chrome PID `3944030`, target `510095692C10148FE6A8615D69D8C394`, one commit,
mapping preserved on detach, verified terminal process exit and lock release.
Result: installed recover/detach/reconnect/explicit-close sequence verified.
This is daemon-crash recovery, not proof of full-host or Chrome-crash recovery.
The separate default-profile MCP daemon work remains outside this slice.

Only the close fix and its process-proof wrapper were applied to the existing
scoped installation checkout; unrelated background-exit and other dirty work
remain preserved. The old unmapped disposable PID `3755158` remains outside
this close authority. No synthetic mapping, forced attach, ChatGPT prompt,
GitHub write or Duo/tmux helper tab was created.

## 2026-09-13 | Detach record preservation patched; terminal cleanup incomplete

Source-only repair in `cli/src/native/actions.rs`: `handle_close` now calls
terminal browser-health persistence only when CloseBrowser actually has a
manager or resolved attached process to shut down. Detach and repeated or fresh
no-browser close preserve the retained browser/session identity and lease.
The existing runtime-profile PID resolution is captured before cleanup.
No recovered or external attachment gains process-kill authority.

Primary regression
`test_detach_and_repeated_close_preserve_retained_browser_identity` passed. It
checks complete persisted state equality after detach, repeated close and a
fresh no-browser close. After the final profile-PID refinement, the new test,
nine existing `test_close` cases, Rust format, strict Clippy, debug build and
diff checks passed.
This unit fixture does not prove live recovered-manager or terminal shutdown.

Independent read-only `/root/close_review` confirmed the unconditional terminal
persistence defect and reviewed the patch. Primary accepted its profile-only
PID refinement. CodeGraph reported current index but did not resolve
`handle_close`; direct source inspection supplied the evidence. No live browser
or operating-system mutations were delegated.

Installation is deferred: `service_browser_close` still inherits Detach for a
recovered manager. Its stored-PID termination fallback does not establish exact
process ownership, so merely flipping that mode is unsafe. A verified terminal
shutdown path and live regression remain required. The old disposable Chrome
PID `3755158` remains preserved because the installed version already removed
its mapping; no synthetic record, forced attachment or process signal repairs
that missing authority in this slice. Signed-in sessions and dirty worktrees
remain untouched. Installed binary still has SHA-256 `1704d4c72a7a5b493cb7d6ca7ff8a76f8773dd1257afd826a1ee675c3ec8030b`.

## 2026-09-12 | Installed transfers and recovery passed; cleanup blocked

Primary tested installed SHA-256
`1704d4c72a7a5b493cb7d6ca7ff8a76f8773dd1257afd826a1ee675c3ec8030b`
using the loopback fixture at `http://127.0.0.1:36773/` and a fresh custom
profile under `/tmp/agent-browser-installed-qa-eTerCC`. The no-launch plan
selected no existing profile and advertised explicit throwaway behavior.
The agent-browser skill scoped this test to disposable no-login state.
No production browser input, prompts, installation or GitHub writes occurred.

Upload and download both passed: 87 bytes, SHA-256
`d8c5055518e8bae96b568b7c04108acab21405dccd7eff60ee6a33aa6207c888`.
Server evidence recorded exactly one upload and one download. After one commit,
the primary verified disposable daemon PID `3755098` against its executable,
PID file and isolated environment, then injected SIGKILL only into that daemon.
An ordinary tab-list read reconnected through daemon `3759087` without replay.
Chrome PID `3755158`, endpoint UUID `c69537a3-922f-4402-b581-0f44e1321cfe`,
target `8BC0363FCDEED75BC9039C34AE689A87` and exact URL remained unchanged.
Both rendered committed state and independent server counters stayed at one.
This proves post-commit daemon recovery, not browser/host crash or uncertain
in-flight effect recovery.

Cleanup failed after those checks. Ordinary `close` returned `closed=true` but
preserved recovered Chrome PID `3755158`. A subsequent read refused the live
custom-profile lock with ownership unknown and no mapped runtime/browser.
Exact-session MCP `service_browser_close` then returned
`Service browser session:installed-transfer-qa is not attached to this control plane`.
No force attachment, state edit or browser signal bypassed the ownership gate.
The disposable browser and its profile remain preserved pending scoped cleanup
recovery. Retained production browser status still verified without launch.
Overall outcome is partial: transfer and recovery passed, terminal cleanup
requires repair. The fixture and payload remain ephemeral in the directory
above; the exact outcome and artifact identity are recorded here durably.

## 2026-09-12 | Tested candidate installed; retained browsers preserved

Guarded local publication transaction
`local-dashboard-65d1acc5-f036-4a46-8a19-fa2949f28a13` completed in terminal
`ready` state. Preflight found the exact four expected sessions idle. The
publisher paired the binary replacement with workstation manifest provenance,
restored the runtime interlock and released its publication lock. No recovery
action is pending.

Installed binary SHA-256:
`1704d4c72a7a5b493cb7d6ca7ff8a76f8773dd1257afd826a1ee675c3ec8030b`.
Installed manifest SHA-256:
`6555000391a53c0136bd973c0c74bd19721d7f2fb1763e80670242ba201300ad`.
Verified prior binary backup:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912233147`,
SHA-256 `cbb1e4a728f343f4f1d7880f86fa69bb8b4d0430a32eaee0623806db4f178dbf`.
Paired prior provenance snapshot:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-sbecpx`.
Rollback artifacts are retained; this successful transaction did not exercise
a rollback.

All three retained browser sessions resumed with unchanged Chrome PIDs:
ChatGPT Pro `505779`, viewer A `495544`, viewer B `496206`. The publisher
verified the retained browser expectation at final readiness; a separate
installed retained-browser-status check also returned verified without launch.
No ChatGPT prompt, navigation or browser replacement occurred.

Fresh installed `install doctor` exited zero with success and no issues.
Both isolated installed-binary tests passed:
`scripts/test-mcp-cold-profile.js` and
`scripts/test-mcp-retained-route-unavailable.js`. They prove cold profile
creation without a daemon/browser, refusal of stale metadata and unavailable
retained routes without launching or creating jobs. The preceding three full
standard suite passes remain the source validation evidence; the 74 ignored
tests and cross-platform/live E2E coverage are not claimed as passing here.
Dirty worktrees were preserved; no GitHub writes occurred.

## 2026-09-12 | Controller lock repaired; final standard suite green locally

Controller `lock` now returns a must-use borrowed `DirectoryLock` with explicit
owner-PID-checked unlock. `run` retains it through socket cleanup and listener
destruction; the anchored directory closes last. Keys, durable authority,
existing-socket refusal and nonblocking admission remain unchanged. A duplicate
descriptor regression failed before the change and passed afterward; a second
regression proves a child-like destructor cannot unlock the parent's lease.

Validation also reproduced a test-only setup race in
`private_journey::tests::concurrent_same_consent_claim_has_exactly_one_winner`:
one worker could panic opening a nonblocking store before reaching the barrier,
leaving the other worker waiting. The fixture now opens two independent handles
before spawning; the unchanged barrier still races claims and requires exactly
one winner. A superseded pre-fixture-fix full run stalled there and was explicitly
terminated; it is incomplete, not passing evidence.

Final primary validation after both fixes: twenty fresh-home 16-thread private
suite repetitions passed 91 tests each (11 ignored). Three fresh-home complete
standard CI runs passed 2,079 tests each (74 ignored). No tests were excluded
beyond the existing ignored set, no CI partition changes or busy retries were
added. Rust format, strict Clippy, debug build, CodeGraph sync and diff checks
passed. The previously observed lock, expiry and installer test blockers are
resolved in this local candidate; ignored live/E2E and cross-platform coverage
remain unproven.

Both changes were carried into the scoped installation checkout only after
baseline-file hash comparison. There, six controller tests, the one-winner
claim test, formatting, strict Clippy, build and diff checks passed. Installed
runtime unchanged; retained-browser status verified without launch. No GitHub
writes, ChatGPT prompts or production account actions occurred; dirty worktrees
were preserved. Next is guarded candidate publication with paired rollback,
not another blind restart.

Delegation: `private_lock_review` completed independent closed-world source
reviews of the lock implementation and test-only setup repair. Primary accepted
the must-use/PID recommendations, inspected both diffs and ran the tests above.
Reviewer reported no critical regression. CodeGraph supplied lock/cleanup call
paths; read-only local context granted no browser or provider authority.
Source SHA-256 (`private_controller_socket.rs`):
`00ebf13b113b4aed25bd81c8a1261223bbe4e60049f62162fe6f26f5f7a1ada7`.
Source SHA-256 (`private_journey.rs`):
`ee883278e3a4c03878f37941a4d96c93f7eaa7e08a7189657a65c30a1d016af4`.

## 2026-09-12 | Installer port fixtures repaired; controller-lock blocker remains

Both released-port fixtures in `install.rs` now retain a bound, non-listening
`TcpSocket` until their probes finish. The helper asserts a competing listener
cannot bind and a bounded connection attempt fails. Before the fix, the new
reservation assertion deterministically failed the stale-inventory test.
Releasing the port allowed another HTTP test to claim it, consistent with the
paired stale-count/empty-User-Agent failures. The User-Agent test now also
requires HTTP 200 instead of discarding the request result. Production code and
CI partitions are unchanged; no retries or exclusions were added.

Primary validation: ten fresh-home, 16-thread installer repetitions passed
66 tests each. The first complete standard CI run passed 2,077 tests with 74
ignored. The second stopped in its parallel partition with 1,461 passed and
one failure: `native::private_controller_socket::linux::tests::duplicate_lock_and_existing_socket_fail_closed`
at `private_controller_socket.rs:639`, `lock(&directory).unwrap()` returned
`private_controller_unavailable`. Third repetition was not run after failure.
This is the remaining validation blocker; do not represent the suite as
reliably green or install the pending candidate yet.

Main build, strict Clippy, Rust formatting, CodeGraph sync and diff check passed.
The test-only patch was carried to the scoped installation checkout after its
`install.rs` baseline matched HEAD; its 66 installer tests, strict Clippy,
formatting, debug build and diff check also passed. No live
publication, browser interaction or GitHub write occurred. Retained-browser
status verified without launch; dirty worktrees were preserved.

Delegation receipt: `private_lock_review` completed read-only diagnosis and
closed-world review. Primary accepted the second released-port finding and
verified both reservation lifetimes and test results. Review found no critical
regression; cross-platform execution was not performed. CodeGraph supplied the
inventory/HTTP fixture paths; runtime evidence came from the primary tests.
`install.rs` SHA-256:
`7369fc1d1f876c9127dc2b5c3164d39c44aa11527f2d0cf785b116fcf7bf6c82`.

## 2026-09-12 | Expiry fixture synchronized; installer-test blocker remains

The private handoff EOF-expiry fixture no longer assumes a 60 ms Tokio sleep
proves a 30 ms SystemTime deadline elapsed. It prewrites the authenticated frame,
observes readable input and a pending receive, checks no staging occurred, then
withholds EOF until the actual authority deadline is observed expired. A five
second monotonic bound detects clock stalls. Rejection and unchanged storage
remain required. A separate direct `expired_validated_frame_never_stages` test
checks post-frame expiry without timing. Production expiry logic is unchanged;
no evidence establishes a production expiry bypass from the former assertion.

Primary validation: the corrected EOF fixture passed 30 fresh-home parallel
private-suite repetitions; the final 29 included the new direct test, with 89
passed and 11 ignored each. Two complete CI runs passed 2,077 tests with 74
ignored each. The third failed in the parallel partition: 1,460 passed and two
failed. Exact remaining failures are
`install::tests::active_runtime_inventory_classifies_unreachable_stream_backend_as_stale`
(`staleCount` was zero instead of one) and
`install::tests::http_client_sends_user_agent` (empty captured request).
The first fixture releases its reserved TCP listener before probing the port;
cross-test port reuse is a candidate explanation, not yet proven. Do not call
the full suite reliably green or install until these failures are reconciled.

Repeated stress runs now create a fresh isolated home each time. An earlier
reused-home diagnostic produced `privacy_gate_locked` in a private attachment
test after 14 successful repetitions; retained endpoint state can survive in
that harness and must not be mistaken for fresh-process isolation.

Main debug build, strict Clippy, formatting, CodeGraph sync and diff checks
passed. Existing dirty work was preserved. The three reviewed lock/expiry files
were also carried into the scoped installation checkout after verifying its
baseline files matched HEAD and its pre-build binary exactly matched installed
SHA `cbb1e4a728f343f4f1d7880f86fa69bb8b4d0430a32eaee0623806db4f178dbf`.
Scoped validation also passed: four lease regressions, all five handoff tests,
formatting, strict Clippy, debug build and diff check.
No publication or installation was attempted; retained-browser status verified
without launch. No ChatGPT prompts or GitHub writes occurred.

Delegation: `private_lock_review` completed read-only diagnosis and closed-world
fixture review, finding no weakened expiry assertion. Primary accepted the
ordering recommendation, inspected the code and ran validation. CodeGraph
provided receive/staging paths; local read-only context supplied no new authority.
Final `private_handoff.rs` SHA-256:
`bc6d5438b66d7be0c9351b2c01bcffcbed9f58b0aca6578887c38d38a472c587`.

## 2026-09-12 | Private flock lifetime fix verified locally, not installed

Reproduced `private_store_busy` in the parallel private-operation tests on
iteration 2 (private execution admission), matching the earlier broker resume
failure. UUID fixture roots did not prevent it. Close-only independent flock
descriptors can remain owned by a concurrent child's inherited open-file
description until exec. Two deterministic duplicate-descriptor regressions
both failed before the fix and passed afterward.

`private_secret_store.rs` now returns a non-clone `GateLease` for every
independent shared/exclusive lease. Owner Drop explicitly unlocks before close;
a PID mismatch prevents a child's destructor from unlocking its parent's
lease. `privacy_gate.rs` uses that owner for public, command and private permits.
No durable privacy marker, pending receipt, replay evidence or cleanup gate is
removed by Drop. Real competing leases still fail immediately. No retries were
added. A trial CI-serialization workaround failed outside its selected modules
and was fully removed; the original CI partitions are unchanged.

Four regression tests cover inherited-descriptor release, independent-reader
exclusion, child-PID ownership, and pending command receipt preservation.
Primary validation: ten 16-thread private-suite repetitions passed with 87
tests each, explicitly excluding the separately failing expiry test below;
three complete standard CI runs passed with 2,076 tests and 74 ignored each.
Rust format, strict Clippy, debug build, CodeGraph sync and diff checks passed.
No live/E2E or cross-platform coverage is claimed for this slice.

Delegation receipt: `private_lock_review` completed read-only diagnosis and
closed-world source review. Primary accepted the lock-lifetime diagnosis and
PID regression recommendation, inspected the patch, and ran all tests above.
Reviewer found no critical regression; its review was not runtime evidence.
CodeGraph supplied the lock/permit call paths, followed by source and test
verification. Graphiti-discovery skill was unavailable; read-only local context
was retrieved without provider submission.

Remaining blocker: `private_handoff::tests::expiry_during_eof_wait_prevents_staging`
failed both before this patch (serial run) and afterward (parallel run), although
it passed in the three complete CI runs. Its 30 ms wall-clock expiry / 60 ms
timer assertion is unresolved; the lock stress exclusion is diagnostic only,
not a CI exclusion or a claim that expiry behavior is correct. Investigate that
failure before publishing/installing this candidate.

Source SHA-256 (`private_secret_store.rs`):
`baf00b6a1172c36b44a8f69beb9736f33bd93e6bbbc92f076ea53f3bc6c00d32`.
Source SHA-256 (`privacy_gate.rs`):
`4327400e16dae6e0c17b81227f6f6cc56bb63db7847062440506337920b69d12`.
Local debug candidate SHA-256:
`3f1a1ab164ea1cea9be3c7bab08be8aad111180beec741a9db0495f548358612`.
Installed runtime unchanged; retained-browser status verified without launch.
Dirty worktrees preserved; no GitHub writes or ChatGPT prompts.

## 2026-09-12 | Cold MCP profile creation installed and verified

`service_profile_upsert` now uses a short-lived browserless control-plane worker
only when the requested session has no daemon metadata. It preserves canonical
validation, queued job evidence and profile persistence. It starts no socket
daemon, browser or ambient monitor. Existing, stale or unreadable metadata keeps
the guarded transport path; a failed send never authorizes fallback or replay.
This is a profile-upsert exception, not general cold-start browser authority.

Real MCP stdio fixtures `scripts/test-mcp-cold-profile.js` and
`scripts/test-mcp-retained-route-unavailable.js` passed against the main build,
scoped candidate and installed binary. They prove one persisted profile/job,
no daemon/browser, mismatched-ID rejection, unchanged stale metadata and
fail-closed absent retained routes. Both builds and strict Clippy passed;
targeted ESLint, Rust format, CodeGraph sync and diff checks passed. The Rust
`profile_upsert` test filter selected zero tests and is not regression evidence.
Independent source review by `crash_path_review` found no critical regression;
primary verified the implementation and executed the transport fixtures.

Publication `local-dashboard-f23bd272-b6fd-4acf-aabd-c644674e38a0` reached
terminal `ready`, with three prepared/resumed browser-preserving handoffs.
Installed SHA-256:
`cbb1e4a728f343f4f1d7880f86fa69bb8b4d0430a32eaee0623806db4f178dbf`.
Paired manifest SHA-256:
`4b30f65da48284f9331261287c95b5d67dc4ee3bcca83772e66535b9a07cff3c`.
Rollback binary:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912165509`;
paired provenance:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-XSsw6d`.
Use governed rollback, not an unpaired binary replacement.

Post-install doctor returned success with no issues and matching installed
provenance; all four daemon listeners matched the installed executable.
Retained-browser status independently verified the configured Workshop identity
without launch. No ChatGPT prompt or GitHub write occurred. Dirty worktrees
were preserved. Fresh MCP processes were tested; already-running MCP clients
are not claimed to have reloaded the new executable. The earlier broad serial
suite remains incomplete, as recorded below.

## 2026-09-12 | First-command recovery installed and verified

CLI prestart now lets implicit `tab_list`, `browser_pid` and `cdp_url` requests
reach daemon acquisition directly instead of sending a separate launch after
daemon restart. Explicit launch options, managed runtime attachments and input
commands retain prior behavior. This does not guarantee no launch when there
is no valid retained candidate, nor authorize replay of uncertain effects.

Focused routing tests passed in both checkouts. Both builds, strict Clippy,
format and diff checks passed. Reviewer `crash_path_review` recommended the
narrow metadata-only allowlist and preserving explicit build, proxy-bypass and
leave-open choices; primary accepted those findings and verified the code.
The optional broad serial suite was interrupted after roughly three minutes
without progress at `native::parity_tests::test_all_documented_actions_are_handled`.
It is incomplete, not a passing full-suite result.

Scoped installation transaction
`local-dashboard-8927f11e-0fef-4497-9a35-99a84afbb0ac` finished terminal ready,
with final strict doctor passing and retained Workshop identity preserved.
Installed binary SHA-256:
`dc4bc25f4790bba4133e97dc4067c84ef301eed036dd58f15b9925b53751233d`.
Manifest SHA-256:
`36185a7ba5a4e34a001ba760ca5cb057be8e7cc157fbef3675485e5e486bd225`.
Rollback binary:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912142131`;
paired provenance:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-nc4tvw`.

Candidate and installed real-browser tests both passed on the first command
after SIGKILL of only the disposable daemon. Installed Chrome PID 2117730,
endpoint UUID `718cee3b-c300-41e2-9af9-95fd38a11f4e`, fixture target
`729693209E95F490FED5F46998B44C7E`, loopback URL and peer target stayed exact.
DOM and independent server evidence both showed one committed action. No
retry, duplicate browser, duplicate submission or ChatGPT prompt occurred.
Disposable browser/server were stopped; final retained status verified.
Full Chrome crash and uncertain in-flight effect replay remain outside this
proof. The deferred new-profile/default-daemon setup issue is next.

## 2026-09-12 | Owned launch proof installed; first-command recovery gap

After operator pause confirmation, guarded publication
`local-dashboard-32625cf8-3f73-4d77-b40d-04d5e45b3459` completed terminal
`ready`. Installed binary SHA-256:
`38feafaf755e7a3ac784ef1b9ec7eb41996d443dee6d3d18da2aa24810085389`.
Paired manifest SHA-256:
`0589ccb032b205d9ee6724f88f9f0512682898ac02bb9ff2621979107043317a`.
Rollback binary is
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912141347`;
paired provenance is retained in
`/home/bak3r/.agent-browser/publications/.workstation-provenance-N4Oc1C`.
Use governed rollback/recovery, not manual binary replacement.

The scoped startup checkout's pre-patch binary matched production exactly. Only
the owned-launch reuse proof fix was applied there. Scoped build, two proof
tests, strict Clippy, format, diff and prebuilt publisher fixture passed. The
publisher preserved all three browser PIDs and Workshop's exact target, passed
dashboard HTTP/bundle checks and final strict doctor, and restored interlock
custody. No ChatGPT prompt or GitHub write occurred.

Installed disposable repeat-launch/commit passed. SIGKILL of only test daemon
2068418 exposed a remaining first-command recovery failure: tab-list attempted
launch and hit the profile-lock guard for surviving Chrome PID 2068453. The
following read reattached successfully with unchanged PID, endpoint UUID
`2aea0115-6a4b-4eb1-abe1-a634743f458e`, exact fixture target
`8F0D77B27A7292027208156ADC4AD85A`, URL and peer target. DOM and server both
reported one commit. This is recovery after an initial failed command, not
seamless first-command crash recovery. Earlier candidate success also followed
an initial configuration-related refusal; do not generalize it to first-call
success. Test browser/server were stopped. Fix first-command retained-session
routing before addressing the deferred MCP default-daemon issue.

## 2026-09-12 | Owned Chrome reuse proof and disposable crash test

Candidate-only fix: the unchanged live owned-manager branch now reapplies exact
installed-executable proof before persisting newly constructed launch metadata.
Repeated explicit launch flags previously replaced proof with an unverified
request. Attached managers, unknown executables, registry refusals and stealth
requests retain their existing guards. No runtime replacement was performed.

Primary checks passed: two stock-proof unit tests, two retained-session selection
tests, new real-Chrome repeated-launch E2E, build, strict Clippy, format and diff
checks. Candidate debug SHA-256 before help-only documentation update:
`9d89eed8ff3d5f8104e715e036bb11c06bfe00f974bd6e11ddada4a01f00c8ee`.

The disposable candidate daemon was killed with SIGKILL after one committed
fixture action. With the fixture's supported `service.defaultBrowserBuild`
configuration, a same-session tab-list reconnected. Chrome PID 2037908,
endpoint UUID `242d9df4-a547-4f86-bc9a-4a3a37d4a1d3`, target
`2D2E8F0699DA46F0F0A53B9F37E1B6FC`, loopback URL, and peer target were
unchanged. The committed DOM result and independent server counter both stayed
at one. The initial reconnect refusal was caused by the fixture's unsupported
top-level build field, not an authorization grant to weaken the build guard.

Reviewer `crash_path_review` independently identified the raw metadata overwrite
and reviewed the bounded helper reuse. Primary verified its source findings and
ran all live checks. No Chrome-process crash, uncertain in-flight replay,
production broker recovery or production installation is claimed. The default
MCP daemon issue remains deferred. Ephemeral fixture details and earlier test
correction are in `docs/dev/notes/2026-09-12-disposable-crash-recovery-test.md`.

## 2026-09-12 | MCP handle routing installed

After the operator confirmed browser workflows paused, scoped checkout
`/home/bak3r/projects/agent-browser-startup-20260911` received only the reviewed
`cli/src/mcp.rs` diff. Its previous binary matched the installed SHA exactly;
the other preexisting source changes were not modified in this slice. The
rebuilt candidate passed the real isolated MCP retained-route regression.

Guarded publication `local-dashboard-8369b807-a97a-4676-a875-3a6aa9ae752b`
finished terminal `ready`. Two earlier attempts stopped before mutation because
the ephemeral dashboard backend changed the expected session inventory. The
successful attempt captured and checked the immediate inventory against the
four approved sessions plus the optional dashboard backend, without weakening
the publisher's exact-set check.

Installed binary SHA-256:
`3772ea17f4bc27655e196a93ce2d4cebdfd48acb78691be9e48f9cf447b4eeed`.
Installed manifest SHA-256:
`58b47f3e03be01c2711cf27f3a9732ac49f975ddbcc8b60b136b050da6cd85f6`.
Rollback binary:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912133647`.
Paired manifest snapshots:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-H21EZN`.
Use governed recovery: automatic old-binary restoration is intentionally
restricted after browser handoff starts, to preserve browser ownership.

Primary verification: installed strict doctor exit 0, success true, zero issues,
payload ready and binary provenance matching; installed no-launch retained
status verified. Workshop Chrome PID 505779, exact pinned target/URL and all
three retained page targets survived the handoff. Both viewer browser PIDs
were also preserved. Dashboard HTTP/bundle/runtime smoke passed and interlock
custody was restored. Installed-binary MCP no-launch regression passed; 52
provenance and 40 publication-doctor fixture cases passed. The three routing
unit regressions were rerun in the main checkout. No new prompt was sent.

Read-only evaluator `routing_install_review` found no concrete code blocker;
it independently matched main/candidate routing diffs and reviewed rollback
boundaries. Full pre-patch source provenance for the candidate's other dirty
files was not independently reconstructed. Primary owns installation judgment.

Existing long-lived MCP stdio processes still reference deleted old executable
inodes. New CLI/MCP processes use the installed fix, but existing clients need
their agent-browser MCP connection refreshed before claiming this routing fix
is active in those clients. They were not killed or their chats restarted.
No GitHub writes, Chrome restart, profile replacement or unrelated cleanup.

## 2026-09-12 | MCP retained handle routing compatibility

The prior no-prompt Workshop attach/read/detach test succeeded with explicit
`browserId` and `sessionName`, preserving the browser and its three tabs.
Handle-only diagnostics instead selected `default`: the MCP dispatcher ignored
the handle's retained route. `cli/src/mcp.rs` now derives that route from the
handle, rejects conflicting or unusable identities before transport, and keeps
missing retained daemons on the no-launch error path. Daemon-side profile,
target, freshness and ownership checks remain authoritative.

Validation: `node scripts/run-rust-tests-isolated.js -- service_request` passed
87 tests, including three new handle-routing regressions. The real isolated
stdio test `node scripts/test-mcp-retained-route-unavailable.js` passed with
no daemon, browser or job created. Targeted ESLint, rustfmt for `mcp.rs`, and
`git diff --check` passed. `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
passed; the stdio regression compiled and exercised the debug candidate.
CodeGraph was synchronized after the source changes.
Help, README, commands documentation and the repo
skill describe the routing contract.

This is a source/candidate fix, not an installed-runtime claim. The running
installation and retained browser were not replaced, restarted or sent a
prompt. Existing unrelated worktree changes remain intact. Rollout must use
the guarded binary/installation-record publication with paired rollback;
continue copying explicit access-plan routing fields until that rollout.

## 2026-09-12 | Bounded retained job inspection

The MCP adapter output truncated a large historical jobs resource inside JSON.
The complete installed CLI resource read remained valid: 200 retained records,
157 succeeded and 43 failed, with no nonterminal records at observation time.
This was an output-size failure, not evidence that Chrome was stuck.

Use `node scripts/read-service-job-summary.js /home/bak3r/.local/bin/agent-browser`
from this checkout. The helper captures at most 16 MiB for at most 15 seconds,
validates the complete resource and unique records, and emits bounded counts
without job results, prompts, or errors. Unknown states and read failures block
idle. Exit 0 means observed idle; it is not an exclusive lease. Exit 2 means
nonterminal work and exit 1 means unavailable. No daemon replacement is needed.
Regression command: `node --test scripts/test-service-job-summary.js`.
Validation: all five regression tests passed, targeted ESLint passed, and the
helper returned complete observed-idle counts against the installed binary.
The worktree diff check passed. Existing dirty changes were preserved; no
runtime installation, browser effect, chat submission, or GitHub write occurred.
The separate live attach/read/detach test remains unverified by this check.

## 2026-09-12 | Desktop startup fix installed with paired rollback

After the operator confirmed other workflows paused, fresh preflight verified
the exact retained Workshop target, five socket-session names, unchanged source
and candidate digests, no unresolved publication, and no pending recovery
attempt. The running native interlock finished naturally before publication;
it was not killed. No interrupted publisher from the aborted turn was found.

Guarded prebuilt publication `local-dashboard-dc7ac324-d5fc-44a2-a0aa-ed0da247afad`
reached terminal `ready` at 2026-09-12T12:56:16.743Z. Installed binary SHA-256:
`2f6d8d80ddf077e11d00cdb8855a7049449229d0497ba7b78a85708beb4e8b79`.
Installed manifest SHA-256:
`00d3ec0ba5bb3fa9686bf89a4cf1ad4378f9874f61bf99d60277c7ef04dbb9e2`.
The only controller update was `scripts/open-rdp-guac-route-displays.js`, SHA-256
`aea5061dfa5bf8dccd5a1f8ef4ff71c78b622555613037ab6368e5a58841bfef`.
The paired provenance verifier passed after publication, including unchanged
assets and permissions. Strict unlocked doctor passed with no degradation;
HTTP/dashboard embedded-manifest verification passed and the dashboard service
and interlock timer are active.

All three handoffs preserved browser PIDs: Workshop 505779, route A 495544,
route B 496206. The exact Workshop target
`4C4278BF20BBD5794A62582E9622E7D8`, profile, URL and CDP endpoint passed final
retained verification. The proven idle default daemon was retired; no browser
was closed. No prompt, navigation, WSL restart, Git commit or GitHub write was
performed. No new independent review was spawned for this serialized deployment;
the prior bounded review was retained and live gates were run by the primary.

Main reran 52 provenance cases and the controller-candidate suite before apply,
then all six isolated startup tests against the actual installed opener. The
separate installed Workshop helper's digest remained unchanged. Its reviewed
`--rearm-verified` migration retained the prior receipt and updated artifact pins;
`--check-readiness` returned `dependencies_ready` and `retainedVerified=true`.
No reboot was performed, so a full cold-boot end-to-end claim remains unproved.
The historical cause of unrelated Chrome exits is not resolved by this patch.

Rollback binary:
`/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260912125526`.
Matching original/candidate manifest and controller snapshots:
`/home/bak3r/.agent-browser/publications/.workstation-provenance-BTtQ0j`.
Keep these paired and preserve the publication journal. Do not restore one file
independently; any subsequent rollback needs the same guarded handoff and a
reviewed recovery-helper rearm for the restored artifacts.

## 2026-09-11 | Scoped desktop controller publication candidate

Implemented a prebuilt-only, explicit `--controller-update` option. The publisher
checks the source digest and embedded bytes, snapshots old/new controller files
with the existing binary/manifest transaction, preserves modes and unrelated
assets, and recovers interrupted controller/manifest combinations before daemon
restart. Existing binary-only publication remains the default. No directory
discovery or blanket publication of dirty work is permitted.

Candidate checkout: `/home/bak3r/projects/agent-browser-startup-20260911`, detached
at c284351a with the documented already-installed native fixes and installed
controller bundle copied through reviewed patches. The corrected opener and
publisher-help text are included; the newer Plan 0131 control-plane change and
its actions visibility change are excluded. The original worktree is preserved.
The existing built dashboard assets were copied, not rebuilt from unrelated UI
work. Candidate debug executable SHA-256:
`2f6d8d80ddf077e11d00cdb8855a7049449229d0497ba7b78a85708beb4e8b79`.
Controller SHA-256:
`aea5061dfa5bf8dccd5a1f8ef4ff71c78b622555613037ab6368e5a58841bfef`.

Primary verified two native fixture-root installations, the second invoked from
the fixture-installed executable. Both retained the fixed opener, matching
binary/manifest hashes and all controller digests. All six startup regressions
passed against the extracted opener. No host provisioning, service activation
or browser launch occurs in this fixture. Reproduce with the explicit candidate
path in `AGENT_BROWSER_TEST_CANDIDATE_BIN` and `pnpm test:controller-payload-install`.

Primary validation: 52 provenance fixtures; controller-candidate positive and
10 rejection cases; 38 Workshop/systemd tests; existing publisher orchestration,
publication operations, interlock and prebuilt-candidate suites; service-client
typecheck; scoped ESLint; native build/format/Clippy; docs build; CodeGraph sync;
diff check. The candidate also passed 115 focused Rust tests: workstation
installation (26), runtime attach proof (9), service access (37), and service
health (43), run serially within each module. The docs build retains its existing
multiple-lockfile warning.
No new plan was opened; existing recovery acceptance and deployment boundaries
were audited directly (there is no plans:audit package command).

Delegation: `controller_provenance` implemented only provenance and its fixtures;
primary read the source and reran them. Fresh read-only evaluator
`controller_update_evaluator` independently reran both helper suites and returned
no blocking findings. Main owns candidate selection and installation judgment;
review of helper code is not live publication approval or installed proof.

At this candidate checkpoint, live installation was held: AuraCall used the retained lane for
`provider-frontend-response` and `full-joined-writer-progress` shortly before
preflight. Session `chatgpt-pro` still reports an exclusive AuraCall/codex lease.
An empty instantaneous job queue is not proof of a quiet maintenance window.
The operator was asked to pause other browser workflows. No handoff, runtime
replacement, pin rotation, helper rearm or prompt submission has occurred.
After that pause, recheck exact sessions/retained identity and run the reviewed
prebuilt publisher with only the opener update. Require unlocked doctor and
retained-target verification, then reviewed Workshop helper rearm/readiness.

## 2026-09-11 | Background browser exit evidence

Plan 0131 addresses lost diagnostics after the autonomous document workflow's
two retained-Chrome exits. Primary inspected broker events, process presence,
launch stderr and user-systemd timing. No recorded shutdown operation explains
the exits, and their underlying cause remains unknown. Default daemon logs are
discarded, so generic process-exited events are insufficient.

Worker `/root/conversation_capacity_detection` reused the existing owned-child
exit metadata helper in control-plane cleanup before closing the manager. Three
focused tests passed, covering exit code, signal, polling error and no evidence;
primary reviewed the diff and independently reran all three tests and Clippy
with warnings denied; both passed.
No policy, retry behavior, provider request or installed binary was changed.

While diagnosis ran, a separate controller restored session:chatgpt-pro with
PID 505779 and navigated the Workshop tab. Installation and live download
verification require reconciliation with that current owner; no replacement or
competing navigation was attempted. Full autonomous proposal acceptance remains
open, including actual source readiness and exact automatic document retrieval.

## 2026-09-11 | AuraCall access-plan readiness mismatch

The document smoke request resp_dde12ed7885d4ada94e7861ad3d041b9 remains
running with an expired lease after the runtime interruption. No replay was
submitted and no artifacts have been verified. A subsequent no-launch plan
selected chatgpt-pro while reading readiness from auracall-chatgpt, incorrectly
requiring manual seeding. Browser control remains paused at that gate.

Source repair makes default readiness follow the effective selected runtime
profile while preserving an explicit diagnostic readiness override. The focused
regression passes, including the explicit override retaining its manual gate.
This is source-only validation, not installed recovery or live document proof.
No browser processes or authentication records were changed for this repair.
Primary validation: all 37 service-access tests passed, cargo format check and
Clippy with warnings denied passed, and git diff whitespace checks passed.
CodeGraph was synchronized after the edit. Next is guarded installed-runtime
publication, then original-response recovery without prompt replay.

Installed at 14:44 UTC through guarded prebuilt publication. Binary digest
90e5dffdbc360f8fe3f1dfef34ffe0f4c5370ac4f11b1a7d602c68d075529272 and
workstation manifest digest
2aa2258a75b1c4e1a2470021548800c3eb9b1ed697b013621fa7e9c554191778
were published together. Strict unlocked doctor passed. Rollback binary is
/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260911144331;
paired manifest snapshot is publications/.workstation-provenance-8UP3M0.
Chrome PID 49015, CDP endpoint and Workshop target survived unchanged.
Installed CLI readiness now selects chatgpt-pro. Existing pre-upgrade MCP
server still computes the old readiness result; its queued actions use the
current broker. Matching the observed shared_display route yields reuse.
Broker download recovered both original response artifacts without replay;
retained-browser verification still passed afterwards.

This file records dated execution turns for repo governance, planning, release,
and operational handoff work. Detailed command output belongs in validation
notes or artifacts, not in this log.

## 2026-09-11 | Opt-in Workshop boot recovery

Status at this historical checkpoint: the exact signed-in Workshop conversation
was restored, verified and rearmed. Both route desktops were verified live.
The September 12 entry above records subsequent installation of the tested
desktop command failure fixes in the packaged runtime.

Historical blocker: the binary-only publisher preserved controller assets.
Installation requires a rollback-capable controller-payload update; do not
overwrite packaged files or modify their hashes independently.

Deployment preflight confirmed the installed binary embeds the old opener
(`5a95949c3cfc81013d6d9a2d3f5712867d4ee7297d5b88cfb948fcf9101d1cad`),
not the corrected source
(`aea5061dfa5bf8dccd5a1f8ef4ff71c78b622555613037ab6368e5a58841bfef`).
Its workstation installer regenerates controller assets from embedded bytes.
A script-and-manifest hotfix would therefore be reverted by reinstalling that
binary; doctor success alone would not establish embedded installer coherence.
The durable path needs a scoped binary-and-controller publication, with a
reviewed browser handoff if required, not publication of the entire dirty tree.
No installation or browser mutation was performed during this preflight.
Independent read-only review `payload_update_review` confirmed the four-hunk
installed-script delta and the existing matching temporary-route dependency.
The primary independently verified the embedded bytes and strict installed
doctor success; the native retained-browser check also passed without launch.

Validation entry point: `pnpm test:workshop-recovery` runs isolated desktop and
recovery fixtures without touching a browser. The separate opt-in
`pnpm test:workshop-recovery:systemd` uses disposable user-service processes only.

Preserve the signed-in browser, exact conversation pin and archived failure
receipts. Do not restart WSL to diagnose this deployment gap. Runtime paths
remain unchanged.

Full chronological evidence is preserved in
[Workshop recovery history](docs/dev/notes/2026-09-11-workshop-recovery-history.md).
Workstream boundaries are listed in
[the dirty-worktree map](docs/dev/notes/2026-09-11-worktree-map.md).

## 2026-09-11 | Verify Chrome rewritten process titles

AuraCall no-launch reuse found process_profile_mismatch despite the exact
installed Chrome executable and correct profile argument. Chrome PID 569667
exposes one space-joined /proc cmdline field. The verifier now accepts only a
strict unquoted switch-only title headed by the independently verified
executable, with no spaced executable/profile paths and a profile SingletonLock
naming the same PID. Existing runtime, executable inode, active-port, listener
ownership and process-start checks remain mandatory. Failed verifier evidence
can be re-observed; registry refusals remain ineligible.

Nine proof tests, production Clippy and build passed. Independent read-only
/root/handoff_race_review identified positional profile-suffix ambiguity;
primary accepted and rejected positional title tokens, added independent
SingletonLock validation and negative fixtures, then reran the tests.
Guarded publication installed SHA256
9cce482622c3633921d8447a2a0ce6de7b544209c25a8b28c5d704303fd80198 with manifest
cc385cc59af036411cec3740493a139e6afe5b93f9ea10f90093cf0195db0f2f.
Strict unlocked doctor and retained continuity passed. Live service readback
now reports stock_chrome and verified_installed_chrome_runtime_attach for the
unchanged retained PID/profile. AuraCall subsequently completed text nonce
request resp_b7c424366b154997a27d8c6c3eb44a1c without launching another browser.

## 2026-09-11 | Distinguish daemon exit from session reacquisition

Fixed publisher retirement classification: a dead original process with stale
PID metadata is retired; a different live session PID is reacquisition, not a
timeout and never a termination target. An original exiting between timeout
and retirement returns already_exited only after descriptor/browser checks.
The production publisher also rechecks live session inventory immediately
before replacement. On failure it resumes through the unchanged installed
runtime; failed resume is uncertain and prevents dashboard restart.
This is a bounded race repair, not a global daemon-start exclusion mechanism.

Primary validation: daemon-exit, prepared-retirement, publication-operations,
publisher-orchestration and the complete local-runtime-convergence suites pass;
scoped ESLint and git diff check pass. Updated the convergence source contract
to account for the existing three-argument handoff and new vacancy check.
CodeGraph refreshed the changed area.

Delegation receipt: spawned /root/handoff_race_review for read-only evidence
and closed-world review; completed. Accepted the source-level false timeout
finding and one recovery-ordering regression. Primary fixed failed resume
handling and independently ran failure-injection coverage proving no install
or restart on uncertain recovery and resume-before-restart on success.
No broad review loop or new browser-start permission was introduced.

Installed outcome: publication local-dashboard-c4b5a303-ad6d-4ae6-9a9f-bfeabf8365cf
reached ready at 2026-09-11T13:48:24Z. Installed binary SHA256 is
3323ff14edfae255d539c21566930b6e92e265bd388193c60c6b07b5ee55e868 and matching
workstation manifest SHA256 is
274fbebc21058bbcd9b8257ee9dd0e206154a87861322e50088bf081762c335a.
Strict unlocked doctor passed with rawSuccess=true, degraded=false and
workstationPayloadReady=true. Dashboard and interlock timer are active.
ChatGPT PID 569667 and both route viewer browsers (566675, 567977) survived
their three controller handoffs; exact ChatGPT target and CDP endpoint passed
final_readiness. Backup binary is
/home/bak3r/.local/bin/agent-browser.pre-local-dashboard-20260911134746;
matching source manifest is preserved by the publication provenance snapshot.
The default session had independently disappeared before this attempt; its
false-retirement regression is fixture-proven, not reproduced in this live
successful publication. Browser page-action smoke was skipped: this proves
installed runtime and retained continuity, not an AuraCall prompt/answer.
AuraCall end-to-end verification and subsequent runtime installation remain.

## 2026-09-11 | Exact acquisition selection and recovered publication attempt

Operator approved replacing the obsolete retained requirement. The fresh
profile's newly acquired target responded with a signed-in Pro composer.
Live discovery nevertheless rejected four same-URL targets. Preparation now
passes the complete opened identity into discovery: session, browser, profile,
target and URL must all agree. Unselected discovery remains strict; duplicate
exact identities and missing targets still fail closed. Live-pin, discovery
and requirement fixture suites pass, including the same-URL regression.

The prior chatgpt-pro lane was concurrently restored and its requirement now
pins target B3C22C284AE36F4E6FF9C69C551FA6B2. Read-only preflight verifies it;
this turn did not rotate or delete that live requirement.

Guarded prebuilt publication of candidate 3323ff14 failed before installation:
default lacked a verifiable retirement boundary. Old daemon PID 880 exited,
but a replacement default daemon was present afterward and the handoff file
was absent. This is consistent with concurrent reacquisition; causality is
not proven. Do not bypass retirement checks or kill the replacement daemon.
Transaction local-dashboard-1716a422-e8d9-4d0a-b577-1e072a942f5e reached
recovered_rolled_back. Exact receipt recovery restored interlock custody;
dashboard and timer are active, retained preflight passes, and installed
SHA256 remains d6bc35878dc456c0ec30456ea96dd68634f83c258cdfe777be331da0611d6329.
Candidate installation and AuraCall end-to-end/installed smoke remain undone.
Next repair must coordinate daemon reacquisition with publication, not repeat
approval requests or assume another blind publication attempt will succeed.

## 2026-09-11 | Reconciliation candidate built; activation held

Locked offline cargo build succeeded. Candidate binary SHA256 is
3323ff14edfae255d539c21566930b6e92e265bd388193c60c6b07b5ee55e868;
installed binary remains
d6bc35878dc456c0ec30456ea96dd68634f83c258cdfe777be331da0611d6329.
Source service_health.rs still matches the tested digest
e81652d6f0b6c738fca2012f3969cc0179a712a898c40540e372b9516f747b37.

Read-only publisher preflight failed retained_daemon_missing: the durable
requirement still pins chatgpt-pro target 3DDDE3A7210A0249DD650CA6674C0BB1.
No Chrome processes were listed. Actual live socket root is
/run/user/1000/agent-browser, containing dashboard-service-backend and default;
home-directory socket/PID files are stale and are not activation evidence.
No publication, daemon handoff, requirement edit, or AuraCall install occurred.
Resolve the obsolete retained-lane requirement with the operator before apply;
do not bypass it with an alternate path or fabricate a live target.

## 2026-09-11 | Reject obsolete process reconciliation results

Reproduced a race in merge_reconciled_service_state: a probe for an old PID
could mark a replacement browser under the same session ID as ProcessExited.
The initial regression failed with ProcessExited versus expected Ready.
Compare both PID and CDP endpoint with the pre-probe snapshot before merging;
reject obsolete health, tab/session updates and deletions, and scoped events.
Unchanged-process observations continue through the normal merge path.

Independent read-only evaluator /root/stale_health_review identified three
accepted companion protections: cleanup deletion, tab/session updates, and
obsolete events. Its bounded closed-world source review found all addressed
and no critical regression in this scope. Primary owns test verification.
The regression covers PID-only and endpoint-only replacement, operational
cleanup, and unchanged-process acceptance. Primary validation: all six merge
tests and all 43 service-health tests pass; cargo fmt check, production Clippy
with warnings denied, and git diff check pass. CodeGraph sync completed.

This demonstrates a source race, not the definitive cause of the observed live
default-session disappearance. No browser, daemon, or installed executable was
changed in this slice. AuraCall prompt/answer and subsequent installed-copy
smoke remain required before claiming the user's installation goal complete.

## 2026-09-10 | Transactional workstation record update installed

The bounded installation objective is live-verified. The guarded publisher
installed executable SHA256
d6bc35878dc456c0ec30456ea96dd68634f83c258cdfe777be331da0611d6329
and its matching same-version workstation manifest together. The manifest digest
is d0bbc0f41a4e29cda0f7a766da1f59fc1b6828619f951393603a4d20b5808d2b.
Unchanged support assets, unit files and Guacamole bundle were independently
hashed before admission; no broad workstation provisioning or secret changes
were used. Version remains 0.28.0, not a formal release.

Publication transaction local-dashboard-9a9cc9a0-da74-410e-9edc-cee2e24ade59 reached ready after
HTTP/runtime-manifest checks and exact retained-target verification. The ordinary
unlocked installed doctor returned success=true, rawSuccess=true, degraded=false,
workstationPayloadReady=true. Timer custody was restored. Chrome PIDs 785726,
777235 and 779674 were preserved while their daemon controllers were replaced.
Workshop target 3DDDE3A7210A0249DD650CA6674C0BB1 retained its profile, browser PID,
CDP endpoint and canonical conversation identity. No prompt, navigation or
credential action was performed. Idle default and AuraCall controller sessions
were retired through the existing compatibility path; no browser was attached to
those sessions. The dashboard's proven idle backend was recreated by its service.

Implemented source surfaces: local-dashboard-workstation-provenance.js and its
20-case isolated fixture, publisher orchestration and adapters, session-quiesce
proof helper, and transaction-scoped doctor helper. Binary and original/candidate
manifest snapshots are journaled before replacement; recovery pairs the manifest
with verified installed bytes before restart. Rollback preserves exact original
manifest bytes/mode and no longer replaces an unchanged executable inode.

Live attempts exposed and corrected dashboard-backend inventory removal,
unnecessary rollback inode replacement, doctor self-publication lock rejection,
HTTP startup timing, and transient dashboard event-poll job snapshots. Recovery
completed the failed source transactions with explicitly degraded receipts whose
live controller executable hashes and PID start identities matched the source;
those receipts were not represented as installed readiness. The subsequent
successful publication reconciled the controllers and passed strict doctor.
Only the exact owning publication warning is scoped out before commit; unrelated
doctor issues still fail. Source rollback degradation is limited to explicit
pre-handoff failure evidence and empty recorded/discovered/resumed handoffs.

Independent bounded review: payload_pair implemented isolated provenance and
quiescence helpers; pair_review implemented doctor validation and reviewed the
integration. Primary independently ran their tests and live verification. The
post-idle evidence freshness finding was accepted as blocking for the source
preflight and corrected by refreshing browser/viewer/cgroup proof. Its 55-case
fixture passed, and a final read-only live refresh admitted exactly the idle
dashboard backend from five current sessions, without stopping anything.
Historical unrelated rows with no PID or CDP endpoint do not imply ownership;
backend-associated persisted rows and live process evidence remain guarded. The unrelated
empty-session uncertain-handoff case remains nonblocking backlog for this
nonempty retained-session installation scope.

Primary validation: all eleven publisher/provenance/quiescence/doctor fixture
suites, 26 workstation Rust tests, service-client typecheck, ESLint, Rust format, Clippy with warnings denied,
native debug build, docs build, CodeGraph sync and whitespace checks. The docs
build retains its existing multiple-lockfile warning. Planning-contract audit
returned ok=true with no problems but applicable=false, not plan acceptance.
No GitHub writes, dirty-worktree cleanup, credential renewal or production
private-executor enablement occurred. Browser-driven task acceptance remains a
separate bounded no-prompt exercise; this result establishes the installation
and retained-browser continuity, not unrestricted autonomous account access.

## 2026-09-10 | Metadata-preserving attachment proof correction

### Guarded activation preflight: workstation provenance blocker

The requested installation/live demonstration stopped before mutation. Current
installed doctor returned success with no issues and ready workstation payload;
the native retained-browser requirement returned configured/verified. Candidate
dry-run returned planned, mutated=false, and verified the same requirement.
Installed SHA256 remains f95c3f19c00aaab1bc591da1ad59755e9de45730ce8bcb19eeaa8acbbf15c9f5;
candidate remains 154d08ea045c8a1b6a00d1b58bb230b0cce0738a5241d307c615ac030a081938.

Blocking: the legacy publisher replaces only the binary and validates the live
runtime manifest, but the installed workstation manifest separately binds the
old binary SHA256. install.rs requires that binding for payload readiness. This
publication would turn the currently ready installation into provenance drift.
Native reconciliation does not update that manifest. Do not manually rewrite
the expected hash or invoke broad provisioning to conceal the mismatch. The
next activation path needs a scoped transactional payload/provenance update,
rollback and a passing post-install doctor.

Read-only MCP access-plan for the exact Workshop URL and chatgpt-pro profile
reported one active profile lease, no compatible live browser and
wait_for_profile_lease; no tab request, attach, navigation or prompt followed.
Systemd inspection also found the interlock activating, so no oneshot was stopped.
The installed skill's preservation and exact-reuse rules kept browser work closed.

Delegation spawned: /root/activation_preflight independently verified the actual
manifest/hash mismatch and the absence of a publisher update or reconcile repair
path. Terminal result STOP was accepted and checked against install.rs and the
publisher by the primary agent. No code edits, build, installation, service
mutation, browser input or GitHub write occurred. Existing dirty work remains.
Documentation-only closeout passed git diff --check. Live browsing capability
has not been demonstrated by this preflight.

### Native interlock compatibility correction

The publisher now recognizes the exact installed native reconcile command as
well as the legacy flock wrapper. Native admission checks the service and manager
environment, supported environment file, and filesystem-remapping directives.
It refuses unproven roots, busy oneshots and all foreign lock files. No stale
PID-based takeover is used. A private staged non-PID marker is fsynced, its
device/inode and executable binding recorded in a version-2 custody receipt,
then exclusively hard-linked to the native workstation lock. Native reconciliation
rejects this marker even after publisher death. Exact receipt recovery checks
the dead owner and original file identity before reacquiring; verified release
removes only its own marker. Nonterminal failures retain the marker and timer
custody. A crash before receipt creation can leave an unpublished private stage;
one after receipt removal can leave an orphan stage. Neither blocks the native
lock, and no automatic orphan cleanup is claimed.

Delegation spawned: /root/native_lock_review performed read-only protocol review
and closed-world implementation review. Primary accepted its filesystem-remapping
finding, rejected those overrides, added fixtures and independently reran tests.
Reviewer terminal result: no remaining blocking findings in the bounded scope.

Primary validation passed: all eight publisher/retained-browser fixture suites,
26 workstation Rust tests (including native non-PID marker refusal), service-client
typecheck, ESLint, Rust format, Clippy with warnings denied, Rust build, docs build,
and diff whitespace checks. Crash fixtures use actual exited child processes at
before-link, after-link and after-unlink boundaries; systemd is mocked. CodeGraph
was synced. Docs retain the existing multiple-lockfile warning. No separate plan
audit runner was available in the current tool inventory; prior non-applicable
audit results are not promoted to acceptance.

Read-only installed root/mount-contract verification passed. Installed binary
SHA256 remains f95c3f19c00aaab1bc591da1ad59755e9de45730ce8bcb19eeaa8acbbf15c9f5.
Local debug binary SHA256: 154d08ea045c8a1b6a00d1b58bb230b0cce0738a5241d307c615ac030a081938.
Native helper SHA256: 2f999e6f50ab58d6a9d11fb88d6bfe9951836e6eac43b93718d554494770484a.
Interlock helper SHA256: 4add8e1a40a8a3bc51b6d7888c3b9fc653bee0fdda4dbe8ad2d5958861e06af2.
This completes the local compatibility slice, not live activation. No service
mutation, publication, install, browser input, renewal, credential operation or
GitHub write occurred. Existing dirty work was preserved. Live all-session
maintenance and retained-target verification remain a separate guarded step.

### Live preflight: native interlock contract mismatch

Follow-on live preflight confirmed candidate and publisher hashes below remain
unchanged. Read-only systemd inspection found the installed interlock executes
the installed binary with `install workstation reconcile --json`, not the older
flock wrapper used by the new helper fixtures. It was activating/running during
inspection; its timer was active. The helper requires a structured flock
ExecStart and therefore cannot admit this installed service even when it idles.
CodeGraph was current and located WorkstationLock in workstation_install.rs;
the native reconciler uses .agent-browser/convergence/workstation.lock, an
exclusive PID-bearing file, not the legacy runtime-directory flock contract.

This is an installed-layout coverage gap in the prior correction. Do not bypass
the helper, wrap only the timer, or substitute the old lock. Next source slice
must support the exact native reconcile command and native exclusion semantics,
including owner-safe release, crash recovery and fixture coverage for the actual
installed layout before live publication. No publisher apply, timer stop,
handoff, installation, browser control, credentials, renewal or GitHub write was
performed. Journal status remained terminal recovered_ready with no publication
lock or interlock custody receipt; installed SHA256 remains f95c3f19c00aaab1bc591da1ad59755e9de45730ce8bcb19eeaa8acbbf15c9f5.
Delegation not spawned: the concrete installed-command mismatch was on the
immediate critical path and ended this bounded maintenance preflight. Prior
fixture results remain valid for their tested legacy layout, not live acceptance.

### Publisher maintenance source correction

Implemented hash-bound prebuilt publication in the legacy publisher. Required
prebuilt path, digest and expected-session set reject incomplete options, builds,
reference sync and disposable browser smoke; HTTP and manifest checks stay on.
Private executable snapshots retain reviewed bytes independently of workspace
changes. Atomic installation hashes the copied file before rename and rejects
generation-selector symlinks. Source snapshots remain available for journal
recovery; no automatic snapshot cleanup is provided. Session-name checks bound
the observed batch but are not a global admission fence against other clients.

The publisher now coordinates the exact installed interlock timer/service and
declared flock. It refuses an active oneshot, stops only the timer, and holds a
parent-owned lock descriptor across asynchronous work. Immutable custody intent
is synced before timer mutation. Verified success or rollback restores prior
timer state; nonterminal failure retains stopped-timer custody. Exact receipt-ID
recovery requires the previous publisher to be dead and never implicitly claims
an unresolved receipt. Journal status exposes that receipt without mutation.
No-op recovery with no journal/custody performs no systemd operation.

Delegation: /root/interlock_guard implemented only the new guard and its fixture;
primary inspected/integrated it and reran tests. /root/publisher_review completed
independent read-only review. Primary accepted two blocking findings: failed
recovery admission restoring timer custody, and unknown first-handoff outcomes
being treated as safe rollback. Both are fixed. Uncertainty is recorded in the
first failure commit; recovery also recognizes legacy failedAtPhase evidence.
Incomplete prepared-session evidence refuses recovery without replay. Reviewer
closed-world verification passed, including the intermediate-journal crash gap.

Primary final validation: eight fixture suites passed (prebuilt candidate,
publication interlock, publisher orchestration, publisher lifecycle, publication
journal, publication operations, retained browser guard, retained live pin).
The interlock fixture uses mock systemd and real temporary-file flock to verify
exclusion across await. Service-client typecheck, ESLint, Rust format, Clippy with
warnings denied, Rust build and docs build passed. CodeGraph synced nine changed
files. The docs build retains its existing multiple-lockfile warning. An initial
planning audit reported no problems but applicable=false; it is not independent
plan acceptance. Final diff whitespace validation passed. An initial
operations fixture exposed an unnecessary read-only systemd check on no-op
recovery; corrected and additionally isolated its systemctl/PATH/runtime/bus.
An unused import initially failed lint and was removed before the passing rerun.
No live install, service stop, handoff, browser input, credential operation,
renewal or GitHub write occurred; existing dirty work remains intact.

SHA256 local debug binary after the publisher-help update (not installed):
f6cc50216e4b1afef1bcc172047ec719b803bf58c3dc5a35d75fca333be0a6e0
SHA256 scripts/publish-local-dashboard-runtime.js:
39e966171ff249974b23ff75388d9b00e42eab1ca6e3114fbe80bf945b0ed746
SHA256 scripts/lib/local-dashboard-prebuilt-candidate.js:
2fdb2c395f4fa370b029b101a90c5e94971399b2d11722bba33acf6668873215
SHA256 scripts/lib/local-dashboard-publication-interlock.js:
818e1e8b74ced5c798a49cf9fff98ea8f01b4000d055fe670b6c77f23aa590d9
SHA256 scripts/lib/local-dashboard-publisher-orchestration.js:
a4cfcac9120c47199894592c881ba6ae2537d2eb79e44b922502180aa63fa215

Next live gate remains a quiet, explicitly scoped all-session maintenance window
with fresh process/target/cgroup evidence and the exact candidate digest. These
fixtures do not prove deployed broker reuse, current sign-in or SAM renewal.

### Activation preflight: held before mutation

Follow-on maintenance inspection confirmed the source and local binary hashes
below are unchanged. Installed binary remains
f95c3f19c00aaab1bc591da1ad59755e9de45730ce8bcb19eeaa8acbbf15c9f5,
a regular legacy-layout file, not a generation selector. Installed dry-run
reported mutated=false and the retained requirement verified without launch.
Compact doctor reported five converged runtimes, no stale runtimes, installed
payload and dashboard ready, and zero cleanup candidates; exit 1 was solely
path_command_workspace_binary_mismatch. This is not activation acceptance.

No-launch MCP access-plan still selected litscout-sam-linux with zero compatible
live browsers and retained_browser_build_mismatch_or_missing_proof. No queued
request followed. Process/cgroup inspection found the four retained browser
roots for chatgpt-pro, default/SAM, and the two RDP viewer sessions outside the
dashboard cgroup. The dashboard uses KillMode=control-group. Retained state
showed no active jobs; this observation is not an admission fence.

Blocking maintenance gap: the publisher always rebuilds rather than consuming
the hash-pinned tested artifact, processes all socket sessions, and does not
coordinate the runtime-interlock timer. The interlock was running on first
inspection, subsequently finished, and its timer remains enabled. The publisher
also deliberately skips original-binary restoration after handoff begins.
The installed skill describes newer census/generation transactions absent from
this checkout's workstation installer; those guarantees cannot be assumed.
Journal inspection showed terminal recovered_ready history, no publication lock,
and no recovery recommendation; its installed-artifact classification is unknown,
not a verified rollback receipt for the current binary.

Independent read-only /root/activation_scope completed source review. Primary
accepted mandatory rebuild and uncoordinated interlock as blocking, verified
legacy layout and separate browser cgroups, and retained all-session/rollback
scope as required maintenance evidence. CodeGraph was current and identified
the publisher orchestration callers and existing fixture suite. No code edits,
install, service stop, handoff, browser input, credential operation, renewal or
GitHub write occurred. Next bounded slice: hash-bound prebuilt activation with
interlock coordination, exact affected-session admission, and explicit recovery
receipts, covered by isolated publisher fixtures before live maintenance.

Implemented exact retained-session proof refresh for metadata=None health writes.
The helper verifies the existing sole browser binding, profile, PID and endpoint
before invoking the existing process verifier; only build-proof fields change.
Registry refusals and non-stock builds remain unchanged. A failed process check
clears stale build/path projections. It does not create ownership or launch.

Independent read-only `/root/handoff_fix_review` found an initial two-write crash
window. Accepted as blocking and corrected: refresh now runs in the same health
repository mutation, before attachability derivation. Regression exercises the
integrated persistence path, idempotence, seven identity/policy mismatch cases,
process-listener loss and preservation of every non-proof state field.
No live handoff, installation, credential action or renewal has been performed.

Final corrected-source validation: isolated runtime_attach_proof filter passed
8/8; widened native filter passed 1,242 with 73 ignored and zero failures.
Both used CARGO_INCREMENTAL=0. Rust format, Clippy with warnings denied,
ESLint, service-client typecheck, docs build, local Rust build and diff check
passed. CodeGraph sync completed. Planning audit reported ok with no problems
but applicable=false; this is not independent plan acceptance. The docs build
retains the existing multiple-lockfile warning. Follow-up independent review
confirmed the atomicity correction and found no critical introduced regression.

SHA256 runtime_attach_proof.rs:
6204a6c48adb178ae05ca5522188e2aec128eedc3f52ffbc6d5d8bcb1d050f64
SHA256 local cli/target/debug/agent-browser (not installed):
d008a9f6e888364534229c6400ae34e5904cd5510c5e80460447916bbd6739bf

## 2026-09-10 | SAM attachment proof activation diagnosis

Read-only installed retained-browser-status verified the configured requirement.
Installed compact doctor reported runtime convergence, dashboard and payload
ready, no stale runtimes, and only path_command_workspace_binary_mismatch.
The retained SAM session still records stock_chrome with applied=false and
no_matching_preference_binding; browser health alone does not supply build proof.

Independent read-only reviewer `/root/sam_activation_review` completed. Primary
verified its blocking finding: handoff resume calls persist_current_browser_health
with metadata=None, while the new attachment verifier is inside metadata.map.
Therefore preserved-metadata handoff skips proof refresh. Handoff preservation
is not proof repair, and the shared publisher additionally affects other lanes.
No handoff, installer apply, attachment, browser replacement, credentials or
renewal was performed. Next bounded implementation must refresh only the exact
retained binding through the verifier and cover metadata=None plus identity drift
before any activation proposal.

Initial focused test command discovered zero tests after the prior publication
snapshot shared the Cargo output directory. This is not acceptance evidence.
CARGO_INCREMENTAL=0 with the isolated runner forced recompilation of the actual
checkout and passed all seven runtime_attach_proof tests. No production state
or ownership evidence was edited to clear the refusal.

## 2026-09-10 | Scoped private coordinator publication

This checkpoint publishes the disabled private controller, coordinator, recovery
and delivery source lane only. Retained-browser provenance, installer changes,
temporary RDP work and their implementation changes remain outside this commit.
Plan history mentioning those local experiments is not publication evidence.
The Python integration still depends on unpublished credential-consumer changes
in the separate LitScout checkout. This is not a standalone deployable release
or permission to activate renewal. No live recipe has been approved or executed.

Publication scope was independently inspected by `/root/publication_scope`
(read-only, completed). Primary accepted the dependency and mixed-hunk mapping,
and materialized the staged snapshot separately for clean-slice validation.

Primary clean-slice checks: Rust format and Clippy passed; private-filter tests
passed 115 with 16 ignored; all nine explicitly enabled private-journey tests
passed, including connected coordinator delivery and process-death recovery.
The working checkout's ESLint, service-client typecheck and docs build passed.
Python coordinator tests passed 12 and controller integration passed five using
the explicit local LitScout dependency and development test binary. Initial
invocations missing that Python path or binary setting failed during setup;
the corrected runs passed. Plan audit reported no problems but applicable=false.
Added-line credential-pattern scan found zero matches; it is not a complete
secret audit. Staged whitespace validation passed.

Read-only recipe preflight selected litscout-sam-linux but returned zero
compatible live browsers, active_profile_lease_conflict and
retained_browser_build_mismatch_or_missing_proof. It provided no reusable
browser/session route. No queued tab request, CDP attach, credential retrieval,
login, renewal, or executable recipe provisioning followed that refusal.

## 2026-09-10 | Executing coordinator and protected activation boundary

Added `private_coordinator.rs`, daemon startup integration, a read-only private
worker preflight, secure independent authority loading and duplex private ingress.
Added fixed `private-renewal-coordinator.py` provisioning/execution adapter and
offline tests. Updated help, README, bundled skill and security docs for the
opt-in root setting. The existing binding-only controller stays separate.

Private readiness is authenticated only after fresh daemon-owned broker/page
validation. The recipe comes from a separate immutable authority record, not
the incoming operations. A single consumed plan flows through binding, exact
private execution, verified cleanup and guarded installation acknowledgement.
Source refetch gets a bounded 60-second duplex-ingress window; an accepted
review finding corrected the original five-second mismatch. Primary ran the
synthetic connected coordinator test with a real 5.1-second ingress delay:
one attach/login/backup/renew/read/sanitize/detach, delivery ACK, retained target
preserved, public observation still locked and second binding claim rejected.

Focused receipts: three coordinator Rust tests passed; 12 Python coordinator
tests and six existing source-adapter tests passed; Python compilation passed.
All nine explicitly enabled isolated private-journey tests passed in 7.39
seconds. Format, Clippy, development build, ESLint, service-client typecheck and
docs build passed. Planning audit has no problems but is not applicable to the
repo's adopted contracts. CodeGraph refreshed; diff whitespace checks passed.

Initial widened native run had 1,240 passes, 72 ignored and one failure:
`private_recovery_reconnects_only_to_bound_endpoint_without_public_output`
reported `privacy_gate_locked`. It passed alone in a new isolated test root.
Endpoint/port reuse between persistent test gates is a possible cause, not a
confirmed live defect. No production gate was removed or relaxed; final widened
rerun evidence follows below.

Final widened rerun: 1,241 passed, zero failed, 72 ignored in 58.07 seconds.
The isolated private-recovery case also passed independently. Its initial
failure remains documented rather than being described as a fixed production
bug; no lasting change was made to the CDP recovery implementation or its test.

Independent delegates: `/root/coordinator_adapter` implementation complete;
`/root/coordinator_review` read-only review complete, accepted timeout finding
closed after source inspection. The primary reran the tests; no delegate
performed live browser, network, secret, or GitHub operations.

Live metadata-only check at `/home/bak3r/.agent-browser/private-controller`:
root and `controller.sock` exist; `execution.json` and `executor.sock` do not.
This is the concrete activation prerequisite, not authority to synthesize a
production recipe. No runtime installation, setting activation, browser change,
credential retrieval, login, backup-code consumption or SAM renewal occurred.
Retain the existing sessions and obtain/provision the reviewed exact live
recipe and plan before enabling the scoped daemon executor.

Coordinator source SHA256:
`544df5480412b9bb414fdb3abf5e69acfbc17ef7a6c8dfc8938df7197d818e7a`.
Python adapter SHA256:
`efd4f64585c74d00b9249c9164b1fc3df39f3e589f92d80e7e98592c62fb15e4`.
Uninstalled development binary SHA256:
`2ad1849473e0c099aa446462b38d2c0c7cd1f5c3ed00bea391477b40c54cc8fb`.

## 2026-09-10 | Protected SAM result delivery implementation

Added ABPD1 private result delivery from a typed bound/reconciled journey and a
matching internal LitScout consumer. Receiver-ready and installation-ACK MACs
are domain-separated and plan/frame-bound. Result admission and installation
reservation each precede their side effect; uncertain outcomes cannot replay.
The guarded installer checks authority immediately before atomic replacement.
No plaintext key enters an ordinary browser job result or public command.

Initial validation: three Rust transport tests passed; the explicitly enabled
isolated bound-ingress/CDP-worker/reconciliation/delivery regression passed;
1,238 widened native tests passed with 70 ignored. Clippy, development build,
ESLint and service-client typecheck passed. Planning audit returned no problems
but `applicable:false`, not plan-acceptance evidence. Touched Rust formatting and
diff checks passed. Whole-repository format checking found an existing unrelated
line-wrap difference in `cli/src/workstation_install.rs`, left unchanged.

Delegated consumer implementation and independent source review were scoped to
offline work. Accepted DR-1 (expiry at commit) and DR-2 (receiver authentication)
were fixed; reviewer reported no remaining blocking source findings. Primary
cross-language and final Python validation receipts follow below.

Fresh installed workstation dry-run returned `mutated:false`, retained
requirement verified, `ready:false`, substrate preparation required. This
supersedes the earlier missing-retained-browser refusal but does not authorize
an arbitrary runtime replacement or demonstrate private renewal readiness.
The installed probe/bind controller has no executing coordinator route to this
new internal capability, so renewal was not enabled and no live key was touched.

Final primary verification: the explicit isolated
`private_delivery_rust_to_litscout_guarded_install` test passed (1.78 seconds).
The Rust producer spoke ABPD1 to the real Python consumer, which used temporary
approval/installation DB records, the fixed validator with mock HTTP and the
real guarded atomic installer, then returned the verified ACK. This complements
the separate typed broker-journey/CDP-cleanup test; it is not a live combined
Slack/browser renewal. The primary widened Python run passed 119 tests in
72.83 seconds; an added ACK-loss regression is covered by the final delivery
rerun. Delegated combined consumer/approval checks passed 120 tests separately.
The final primary delivery rerun passed all 40 tests in 27.73 seconds, including
ACK loss after durable installation; Python compilation also passed.

Source SHA256: Rust `private_delivery.rs`
`437fecc4da56620936b02389a56d9e9b06c4ab0a2e42e6f23250750630e52aa6`;
Python `credential_delivery.py`
`f75dcafefbf20b5f9225c4bc7ffa82fc9314682d69adc7ede225e6850eb562fe`.
Uninstalled development binary SHA256:
`db99e4a7ae9aa3ca438161c412cd6bc359380aa94ab1f9451c9d589a4fe21485`.

Test isolation caveat: early delegated mock-provider tests did not redirect
SAM quota bookkeeping and may have incremented the local counter (approximately
eight validations; exact delta unverified). No live HTTP occurred and no quota
history was reset. All final delivery tests and the interoperability helper now
redirect both quota path and cached state to their temporary fixture.

## 2026-09-10 | Approved timer pause and broker-owned SAM recovery

User approved pausing the failing maintenance timer and broker-owned recovery.
Stopped only `agent-browser-runtime-interlock.timer`; verified inactive and no
running reconciliation MainPID before recovery. Workstation dry-run still
refused `retained_browser_missing`; no installer apply, requirement rotation,
ownership fabrication or forced shutdown was attempted. The timer remains
paused intentionally pending resolution of that separate retained requirement.

The no-launch broker access plan selected saved profile `litscout-sam-linux`,
reported zero live browsers/active leases and recommended `launch_new_browser`.
Exact scoped executable preflight passed for installed Linux Chrome
152.0.7977.82. Submitted one recommended service `tab_new` to SAM account details
with detach cleanup. The broker created browser/session `session:default` /
`default`, PID 467915, DevTools port 33729, using the existing user-data directory.
No duplicate lane, saved-profile deletion or alternate profile was requested.

The new SAM target C1538FDDC6F432F3130E7A9D8EFA7324 rendered `https://sam.gov/`
with title `Home | SAM.gov`, not the requested authenticated account-details
page. The saved profile also restored Login.gov target
2B1F56A6AD4FD80C1F7346A9D10B22A7. Retrieved its broker-issued handle from service
inventory and performed bounded metadata-only evaluation: canonical URL
`https://secure.login.gov/`, title `Sign in | Login.gov`, document complete,
email and password field presence true. No input values or page body were read.
The first evaluate was rejected before execution for missing positive timeout;
the corrected five-second request passed. A target-only handle-refresh request
was rejected for lacking a handle; used actual broker inventory, not a fabricated
handle. No input action or submission occurred.

Fresh access plan now reports one compatible live browser and
`reuse_existing_browser` for the exact profile. Service inventory proves stock
Chrome/executable metadata and valid handles to the two observed canonical URLs.
Browser recovery is verified; authentication and the full Slack/Login.gov/MFA/
SAM renewal/LitScout consumer sequence are not. Private-executor and consumer
installation gates remain in place, and stale pre-recovery private bindings
must not be reused for the new browser/target identity.

No binary installation, source implementation change, credential retrieval,
backup-code use, renewal, ChatGPT prompt or GitHub write. RUNBOOK and Plan 0130
only; diff hygiene checked. No delegation: this was a short sequential operation
with exact plan/preflight/handle checks, not an independent implementation lane.
The agent-browser skill's broker-first recovery and retained-profile safeguards
guided the operation. Keep the restored browser and paused timer as recorded.

## 2026-09-10 | Goal revalidation found the retained runtime missing

The preceding goal turn was progress: it completed the final optimized build
and established the shared-maintenance blocker. This continuation revalidated
current state rather than assuming that the previous browser still existed.
Installed retained-browser status now fails with `retained_daemon_missing`.
Supported `runtime status litscout-sam-linux` reports `browserAlive=false`,
`devtoolsReachable=false`, no port and no targets. PID 2774558 remains only in
the stale runtime record; its proc executable is absent. The recovery daemon
is also absent. The recorded WebSocket must not be reused as live authority.
The runtime interlock remains failed, exit status 1. No host-ingress ownership
metadata exists; do not manufacture it or use forced shutdown/reconciliation
to manufacture ownership for this legacy state.

The full acceptance objective remains protected Slack credential retrieval,
Login.gov sign-in and required MFA, SAM key renewal, protected delivery into
LitScout, and a verified consumer use of the installed key. None is established
by build success. Source inspection still finds `renewalEnabled=false` on the
private controller response, independent approval required by bound execution,
and LitScout `credential_approvals.py` rejecting the authentication-executor
installation path with `credential_authentication_executor_unavailable`.
No passwords, backup codes, Slack messages, API keys or page content were read.

The next dependency is now authorized broker-owned runtime recovery, not merely
refreshing an existing connection. The prior maintenance approval question is
still unanswered by the automatic goal continuation. Stop at that boundary;
no installation, launch, handoff, shutdown, lease deletion, credential retrieval
or renewal was attempted. No delegation this turn: current-state revalidation
was a short sequential check with no independent implementation lane. Only
RUNBOOK and Plan 0130 changed; diff hygiene was checked. The goal remains active
and incomplete; this is not an end-to-end pass.

## 2026-09-09 | Activation preflight stopped at shared-runtime maintenance drift

User approved proceeding with controlled activation. Read-only install doctor
reported seven existing issues: six stale executable daemon records
(`bwkuehl-headless-qa-20260907`, `chatgpt-pro`, `cloudflare-login-handoff`, and
the A/B/C route viewers), plus `workstation_payload_partial_or_drifted`.
Dashboard, service, launch config and remote-view privileges were ready;
the retained-browser requirement verified before and after inspection.

The active five-minute runtime-interlock timer invokes the shared installed
binary's workstation reconciler. Its naturally scheduled cycle ran during
inspection and failed with `install doctor did not report ready`, exit status 1.
The reconciler also manages canonical Guacamole routes/displays and user units.
Replacing the shared binary is therefore not a SAM-only activation. Do not
silently run the all-session publisher or full workstation apply under a narrow
session repair. A coordinated maintenance window must explicitly include the
timer, shared payload and affected retained connections before installation.

The no-launch SAM plan still sees one retained lease but zero compatible
browsers and reports `retained_browser_build_mismatch_or_missing_proof`.
Runtime status verified PID 2774558, port 42717, original browser WebSocket and
the original Login.gov and new-tab page targets. No auth probe, navigation,
prompt, capability mutation, handoff prepare, close or binary install was run.
The seven isolated provenance tests passed again and diff hygiene passed.
The refreshed optimized build completed successfully from the final verifier
source. Candidate SHA-256:
`11428079f6accfdc0c3ada017007ad1c84363ce6fd8ab03784c7a67a4572c20c`.
Installed binary remains unchanged, SHA-256:
`cb131da45e9e4ad0547b72d1e160fc018891bf7aba06980f0b0bbcfc6e58323d`.

Delegation: `/root/activation_boundary` completed read-only source review.
Primary confirmed that ordinary daemon startup does not enable private
controller execution, and that explicit `handoff prepare` persists exact
browser authority and stops only the daemon; `handoff resume` restores it.
`leave-open close` is not a close-time override. The full publisher is broader
than the selected-session path. These findings informed the stop decision;
they do not constitute a live handoff test. Existing dirty work remains intact.

## 2026-09-09 | Managed runtime attachment provenance repaired locally

Bounded local repair of the preceding SAM blocker, with independent security
review and no ChatGPT submission. Added `runtime_attach_proof.rs` and hooked it
into common managed-attachment metadata persistence. On Linux, installer proof
requires the recorded runtime/profile/PID, canonical profile directory, exact
connected browser WebSocket, DevToolsActivePort, process executable path and
inode, process start ticks, profile arguments, and process-owned loopback
listener to agree. Requested executable preferences alone are not proof.
Registry refusals and non-stock builds remain closed. Failed re-verification
clears persisted build/executable rather than keeping stale positive evidence.
The check never launches, navigates, closes, or changes a profile or binding.

Primary validation: seven focused tests passed; widened isolated native suite
passed 1,235 tests, with 70 ignored and the crash-child fixture excluded. Rust
format, Clippy with warnings denied, service-client typecheck, root ESLint, docs
production build, native development build, and diff check passed. After the
final guard preserving existing non-Linux/non-stock authority, both focused and
widened suites passed again. Current development binary SHA-256:
`2e34b8adae3a35e486f62e00e2b4bbb9f975a7e9626bb4d8f6979510ec5a6076`.
Verifier source SHA-256:
`b6c43871421600d0a4df69977d3a1376e46b0ff81bf8f4aa79f070de0eaf8c78`.
An optimized build also completed, but began before that final guard; rebuild
the optimized candidate before any installation. The first widened command had an
argument-separator error and ran no tests; the corrected invocation passed.
Planning audit returned no problems but `applicable=false`, not substantive
plan-contract certification. CodeGraph was synced; its affected-test suggestions
were not useful for this Rust surface, so direct focused/native suites govern.

Delegation receipt: spawned `/root/attach_proof_review` for read-only acceptance
and closed-world review. Completed with no critical remaining findings; primary
inspected its evidence and retained exact listener ownership and registry-gate
requirements. Added selected-executable mismatch coverage after its test-gap
note. Process-change race branches are inspected, not separately fault-injected.

Source/build staging only; no runtime installation or daemon replacement in
this slice. A read-only process/state check still found PID 2774558, Linux Chrome
152.0.7977.82 and the original recorded browser WebSocket/port 42717. This does
not prove live broker reuse of the new code. Activation must account for the
shared installed runtime and unrelated pre-existing dirty implementation before
a controlled leave-open connection refresh. Do not replace Chrome or erase a
lease to deploy this repair. Authentication, renewal, and GitHub writes remain
out of scope; the entire dirty worktree is preserved.

## 2026-09-09 | SAM retained runtime attached; build proof still blocks reuse

After explicit approval to reconcile the existing browser, inspected the
installed runtime help and attach implementation. Used the supported
`runtime attach litscout-sam-linux` with session `litscout-sam-recovery`, explicit
runtime profile and `leave-open`. The session name was absent from the prior
session inventory. Result: `attached=true`, `attachedToExistingBrowser=true`,
DevTools port 42717. This created an automation connection, not a browser.

Before/after runtime status preserved browser PID 2774558, exact browser
WebSocket identity, Login.gov page target FAA0D733399AC22A5C7F7A7F415593AF and
new-tab target 7BDFD0FE86F1740050DFE49489EFD933. Proc executable still resolves
to Linux Chrome 152.0.7977.82. Service now projects
`session:litscout-sam-recovery`, health ready, profile `litscout-sam-linux`, a
valid handle to `https://secure.login.gov/`, and cleanup `detach`.

Partial recovery only: access-plan refuses reuse with
`retained_browser_build_mismatch_or_missing_proof`, zero compatible browsers,
and `wait_for_profile_lease`. The projected browser has null `browserBuild`
and `executablePath`; `browserBuildProof.applied=false` with
`no_matching_preference_binding`. Runtime attach did not preserve the scoped
service/task attribution used by the routing rule. Do not solve this by
broadening the rule, fabricating proof, deleting the lease or launching again.
Next engineering boundary is verified existing-process build attribution in
the supported runtime-attach path, followed by no-prompt reuse verification.

No page snapshot, input, credential access, renewal, navigation, browser closure,
GitHub write or binary installation. Connection retained deliberately for
recovery; browser stays open. Only RUNBOOK and Plan 0130 changed in source.
Validation was installed attach receipt plus independent runtime, process,
service-browser/session and no-launch access-plan readback, not code tests.
Agent-browser skill guided preservation and refusal handling. Delegation:
`not_spawned`, bounded sequential operator recovery had no independent lane.

## 2026-09-09 | SAM inspection executable routing and retained-profile blocker

Authorized bounded routing repair used the installed registry upsert API, not
raw Service State edits. Added `browserPreferenceBindings` record
`litscout-sam-linux-readonly-inspection`: scope `task`, target `sam-gov`, service
`LitScout`, task `sam-readonly-page-recipe-review`, empty account filter,
build `stock_chrome`, priority 200. All three populated filters are conjunctive.
It names host `linux-local-headless`, executable `sam-chrome-152-7977-82-linux`,
and capability `sam-chrome-152-7977-82-cdp`. The exact profile compatibility row
is `litscout-sam-linux-chrome-152` for `litscout-sam-linux`, with reason
`same_browser_family` and no operator override. Other routing rules were preserved.

The executable record points to
`/home/bak3r/.agent-browser/browsers/chrome-152.0.7977.82/chrome`.
Its capability records observed headed/CDP support, not headless support.
`sam-chrome-152-7977-82-observed-launch` records only observation of an existing
launch at 2026-09-10T01:19:01Z: live PID 2774558, matching proc executable,
ELF x86-64 and version check, headed managed runtime and reachable DevTools.
It does not attest broker custody, new launch, authentication or renewal.
Initial routing used the existing .42 registry executable; after discovering
the live .82 process, both the binding and compatibility row were corrected to
.82 to avoid a future downgrade. No profile contents or global defaults changed.

Primary no-launch verification: exact request reports `validated_binding_applied`
and the .82 Linux path; changed task does not match; unregistered profile reports
`profile_compatibility_missing_or_blocked`. A success envelope on these negative
preflights is not launch authorization. All three report `wouldLaunch=false`.

One authorized `tab_new` attempt, before the .82 discovery, refused the occupied
profile and returned PID 2774558. No retry or raw attachment followed. The
prescribed runtime-status diagnostic reports a page titled `Sign in | Login.gov`
at `https://secure.login.gov/`, target `FAA0D733399AC22A5C7F7A7F415593AF`, plus a
new tab. This is metadata, not rendered form or successful authentication proof.
The service access plan had reported no compatible browser despite this live
profile owner. Preserve that process and stop at the requested sign-in boundary;
exact broker custody reconciliation is required before later automated inspection.

Delegation: `spawned`, `/root/sam_route_review`, completed read-only source review.
Primary accepted its evidence that preflight skips service-profile preparation
used by actual launch. Therefore the earlier Windows preflight path alone did
not prove an actual Windows launch. CodeGraph and agent-browser skills guided
the source trace and no-launch checks. No Rust/JS implementation changed, so
compilation and code suites were not rerun. No credentials, renewal, service
restart, browser closure, Pro submission or GitHub writes. Dirty work preserved.

## 2026-09-09 | Plan 0130 production handoff boundary assessment

Read-only source assessment confirmed that the bound executor has test callers
only. The saved browser binding does not supply `PrivateHandoffAuthority`.
LitScout's approval manifest identifies the account, renewal action, entry URL
and installation destination, but does not specify the four-stage approved page
sequence consumed by that authority. `reserve_credential_installation` still
rejects `authentication_executor_required`; deleting that guard would not prove
protected delivery or consumer activation.

Production activation remains blocked. Next acceptance packet must establish a
reviewed account/page/action recipe, carry the existing one-shot approval through
protected dispatch without reclaiming it, and deliver the reconciled result to
the approved installer with an authenticated, one-shot completion receipt.
Missing or changed authority and ambiguous delivery must remain fail-closed.
Do not fabricate page selectors, enable renewal, or reuse a consumed plan.

Delegation: `spawned`, `/root/handoff_boundary_check`, read-only three-boundary
review completed. Primary independently inspected the cited code and accepted
its production-blocked conclusion. CodeGraph skill guided callers inspection;
direct reads covered Python approval policy and Rust authority fields. The
unavailable graphiti-discovery skill was replaced by read-only local retrieval.

Primary reran `scripts/test-private-controller-integration.py`: five tests passed
using the existing debug binary with SHA-256
`90538cc4ca791fd3627ca0f40a32d26c264ca71e084126bedb2491c0ea111531`.
These are synthetic socket/binding tests, not production handoff or installation
proof. No implementation changes, rebuild, installed-service changes, credential
access, browser actions, Pro submissions or GitHub writes occurred. Existing
dirty changes were preserved. Only this execution receipt and Plan 0130 changed.

## 2026-09-09 | Plan 0130 stored binding to internal executor

Added `cli/src/native/private_bound_execution.rs`; updated
`private_handoff.rs`, `control_plane.rs`, `control_plane_private_tests.rs`,
`private_journey.rs`, test-only socket support in `private_controller_socket.rs`,
and `native/mod.rs`. Plan 0130 records the internal-only scope. No LitScout,
public service contract, installed unit, authentication key or live runtime changed.

The additional gate validates the immutable plan bookmark, strict saved binding,
exact digest/profile/browser/session/target, all four staged operations, expiry
ceiling and distinct stores. Admission is durable before returning the move-only
queue token. Reopened and repeated claims fail. Worker dequeue checks the binding
against actual endpoint/session before the existing controller and journey gates.
`submit_bound_private` is the production internal entry; legacy unbound submission
is now test-only. Existing independent handoff approval remains mandatory; stored
binding metadata never grants account/origin/action authority by itself.

The new Linux synthetic test sends an authenticated binding through the actual
ABPC1 handler, persists it, refuses a second claim, then uses the bound worker
for the single synthetic attach/login/backup/renew/read/sanitize/detach sequence
and reconciliation. It preserves observation closure. Queue-full/closed,
cancellation and expiration fixtures now use consumed bindings. Negative cases
reject staged-operation mismatch, wrong digest/endpoint/session, expired or
too-short bindings, and aliased stores before admission. Synthetic retained IDs
were adjusted to the socket's 32-uppercase-hex schema; no real target changed.

Delegation: `spawned`, `/root/binding_execution_gate` owned only the new module;
primary read the complete implementation and independently ran isolated tests.
The agent initially ran two pure parser tests through direct Cargo; this was
corrected to the isolated-runner convention and is not the accepted validation
basis. No filesystem/runtime fixture ran in that direct invocation.
Fresh `/root/bound_executor_review` returned no blocking implementation findings.
Primary accepted that disposition, then filled its nonblocking operation-mismatch,
expiry-ceiling and alias coverage gaps. A dedicated actual-worker endpoint/session
drift fixture remains nonblocking backlog; source ordering and pure drift checks
were reviewed. CodeGraph skill guided callers inspection and index refresh;
direct reads covered changed bodies. The unavailable graphiti-discovery skill
was replaced by a read-only local context query, not a provider submission.

Primary validation: focused private suite 78 passed, nine ignored; explicit
journey suite eight passed; bound queue/negative suite four passed; widened
isolated native suite 1,228 passed, 71 ignored. Rust format, clippy with warnings
denied, service-client typecheck, ESLint and docs build passed. Docs retain the
existing multiple-lockfile warning. Planning audit had no problems but
`applicable: false`; diff check and native build passed.

Source SHA-256: new gate
`49de4f2e5373e05cc1a85c0a9742f73d808fdf02bfb7dc1e91dab37b6c67eab7`;
handoff `be34b892ab585fec0ddfc9281932f31ec16342543af91bb0cfed1996b3039507`;
worker `1a602ce15d01cd117cb5204966838e28eb3d5f21a5f1d11f3186df55117ad67f`;
queue tests `cefdad3161da006f85748bc835571103a28eb520cefe3f57a2232bbb631397c5`;
journey `2893d166ebc2074eadc34ed5c44939a47621a1b6e52fc0a36ee8954d1e6aebd8`;
socket test seam `556bf9910491cc99ffd981821101f440dcd490a56bf75076d100a5f0824e8604`;
module registry `b677aed71bec6d96a8d82dc2a939862139bf0a9d29d0f4956eeac02361865a5e`.

No live approval, secret use, browser actions, installation/restart, Pro prompts
or GitHub writes. Dirty worktrees preserved. Next: protected production caller
for independent account/page/action authority and private material delivery,
followed by guarded consumer delivery proof. The installed socket remains
binding-only; internal source integration does not enable live renewal.

## 2026-09-09 | Plan 0130 authenticated local controller installed

User explicitly authorized a same-user Unix socket and separate authentication
key. Added `cli/src/native/private_controller_socket.rs`, early isolated routing
in `cli/src/main.rs`, registration in `native/mod.rs`, the Python
`scripts/private-controller-client.py`, adapter `bind_approved_controller` in
`scripts/private-sam-discovery.py`, cross-language
`scripts/test-private-controller-integration.py`, and
`scripts/agent-browser-private-controller.service`. Updated output help, README,
skill instructions, security MDX and Plan 0130 in the same slice. No LitScout
source edits were needed: its existing durable claim API is reused.

The Linux-only service accepts authenticated probe/bind messages only. Same-UID
checks, fresh challenge, domain-separated request/response HMAC, strict JSON,
16 KiB cap, five-second deadline and mandatory EOF precede dispatch. Random
key creation uses mode 0600 and durability sync under a mode-0700 root; unsafe
or missing keys in a used root are never repaired/rotated. Bindings are encrypted
and immutable per plan ID, including changed-digest attempts. The Python adapter
checks exact approved manifest digest and retained profile/browser/session/target
and ready state after LitScout's one-shot claim. The trusted caller supplies
the broker-resolved endpoint; key possession is not independent browser proof.

Binding metadata is not a credential payload, execution permit, recovery result,
authentication verdict or renewal receipt. The new socket has no route into
browser actions or the typed private journey/recovery worker. A lost response
leaves approval consumed and cannot authorize replay. Live renewal remains off.

Delegation: `spawned`, `/root/local_controller_transport` implemented only the
Rust transport and four tests. Primary read its complete source and independently
ran all accepted tests. Fresh read-only `/root/controller_socket_audit` reported
no blocking findings. Its nonblocking misleading test-name observation was
resolved by naming the test for second-server refusal, not server restart;
Rust separately reopens the store to verify immutable bindings survive reopen.
CodeGraph skill guided integration queries and was refreshed after edits. The
unavailable graphiti-discovery skill was replaced with a local read-only context
query, without provider submission or extracted-authority assumptions.

Primary tests: four transport unit tests passed; widened isolated native suite
1,226 passed, 69 ignored. Five real Python/Rust socket tests passed, including
captured-frame replay refusal, duplicate server/binding refusal, insecure key
refusal, and a synthetic LitScout claim committed before the socket opens.
The latter also rejects wrong retained identity without socket dispatch. Existing
private SAM adapter tests: six passed. LitScout broker/discovery/approval tests:
42 passed. Explicit isolated journey regressions: seven passed.
Rust format/clippy with warnings denied, native build, service-client typecheck,
ESLint and docs build passed. Existing docs multiple-lockfile warning remains.
Planning audit had no problems but `applicable: false`. Diff check passed.

Installed a separate binary, without replacing the normal agent-browser runtime:
`~/.local/lib/agent-browser-private-controller/20260909-v1/agent-browser`.
Installed SHA-256 matches the tested build:
`f7bab851b2b58c305b71d50940e2815c30574a4a5fa7a97105aba7e5bdbfd13d`.
Created and validated the new key privately under
`~/.agent-browser/private-controller/authentication.key`; no value or digest of
the key was printed. Verified root/key/socket permissions 0700/0600/0600 and
key size 32 bytes. Installed the matching user unit, verified its syntax after
binary installation, then enabled/started it. `ActiveState=active`,
`SubState=running`, `ExecMainStatus=0`, `UnitFileState=enabled`, `LimitCORE=0`,
`RestrictAddressFamilies=AF_UNIX`, `Restart=no`. A live authenticated Python probe
returned exactly `success: true, renewalEnabled: false`. No live plan was bound.
The user service manager was already degraded before this installation; this is
component-level verification, not a whole-workstation health claim.

An existing socket is never unlinked on startup. An unclean stop can leave one
that requires explicit ownership diagnosis before restart; automatic restart is
disabled to avoid repeated failures or takeover. No browser sessions, helper
tmux tabs, real Slack discovery, passwords, MFA, keys for external services,
ChatGPT prompts or GitHub writes occurred. Dirty worktrees remain preserved.

Source SHA-256: transport
`ab386b5a80a0e0efc39cf5433cd41b25a29c2cac2f63f6e3c9a57d5b84224834`;
client `519ddd1497473b81a311fce036e7f07a6609bcf1639440e068a8f8c0467f2759`;
adapter `f1de0e1b325fadb7a259ac625bbd5077f665071cbe32f2ccd963c0b1cd0953b8`;
integration tests `6ec4754bdf3bbb963d57903da8af23e441b7ef1552a739184acbe3c4cde4f8ab`;
unit `e1f1157fc8986cdab9ca72659e91b20c8b485886fb5f5bd781b9fcc735531ac1`.

Next: consume the persisted approved binding inside the typed private
journey/recovery coordinator, with synthetic end-to-end proof before live use.
Do not treat the installed binding service as enabling that coordinator.

## 2026-09-09 | Plan 0130 durable controller handoff

Implemented source-only controller retention and offline restart dispatch in
new `cli/src/native/private_controller.rs`. Updated `private_journey.rs`,
`control_plane.rs`, `control_plane_private_tests.rs`, `private_secret_store.rs`
and `native/mod.rs`; governing scope is in Plan 0130's latest section. Existing
recovery and unrelated dirty RDP/installer changes remain preserved.

The typed private worker requires independently pinned controller endpoint and
session, checked before journey execution. Reconciliation repeats scope checks,
stages and bookmarks the exact intent, then requires immutable encrypted
controller retention before admission. Authority and operation stores cannot
alias by anchored device/inode, including separate opens. The offline dispatcher
uses the saved authority digest/reference, not a digest minted from the examined
intent during restart. Errors never retry browser actions or unlock privacy.

The real SIGKILL fixture now recovers from the controller journal. Test handoff
metadata contains only epoch/endpoint, not a digest. Six phases cover prepared
intent, retained authority, admitted intent, broker commit, daemon update and
completion commit. The first two must refuse recovery with unchanged broker
state; the latter four recover twice while observation remains closed and the
single synthetic attach/login/backup/renew/read/sanitize/detach sequence stays
unchanged. A new private-worker fixture rejects wrong endpoint, wrong session
and aliased journals before any CDP command; all three record zero commands.
These are synthetic peers, not live browser or installed-service acceptance.

Delegation: `spawned`, `/root/controller_boundary_review` assessed the missing
producer then implemented only the controller module and four pure tests.
Primary inspected its full source and independently ran all accepted tests.
Fresh read-only `/root/controller_integration_audit` returned no blocking
findings for the frozen source-only contract; primary accepted that disposition.
No delegated validation result was substituted for primary test execution.
CodeGraph skill guided call-path inspection and was refreshed after edits;
exact source reads covered implementation bodies. The unavailable
graphiti-discovery skill was replaced with a read-only local context query;
its unrelated AuraCall excerpt supplied no implementation authority.

Primary validation: focused private tests 72 passed, seven ignored; final
explicit ignored journey suite seven passed, private queue suite three passed;
isolated widened native suite 1,222 passed, 69 ignored. Rust format and clippy
with warnings denied, service-client typecheck, ESLint and docs build passed.
Docs retained the pre-existing multiple-lockfile warning. Planning audit returned
no problems with `applicable: false`, not full planning-contract coverage.
Native build and final diff check passed.

Source SHA-256: `private_controller.rs`
`51b1aae8735e86d61d25b0e426ca58b3d1a4dc540cfa70a248f3987a118a81d5`;
`private_journey.rs`
`65cbc803ecf1972a3b5dab38e4d4c64abc1f263cc5777cf625957d6bedfd6040`;
`private_secret_store.rs`
`e99ed86724e909490980742b007a566b2a64bbf16e54577ec9ce54f57dd77c75`;
`control_plane.rs`
`6e6941094f9d2bbe4724f2712b728fa225ad62b61262d274b578aebf73993fb2`;
`control_plane_private_tests.rs`
`6905665b89b968f3c66b4665eb1ac28b950b9406a5ea6d28478f706cfb157ce4`;
`native/mod.rs`
`c99bc81297472f96e098a9cc1844a57338743580ae825a80be7290d72e3f704d`.

No credentials, live browser actions, Pro prompts, installation, GitHub writes,
privacy release or renewal enablement. Concrete remaining deployment blocker:
no installed trusted producer connects an approved LitScout plan to controller
provisioning and restart lookup. Next recommendation is that bounded producer
integration with synthetic end-to-end acceptance before any live activation.

## 2026-09-09 | Plan 0130 actual process-death recovery

Added `cli/src/native/private_recovery.rs`, registered in `native/mod.rs`.
`private_journey.rs` now records admitted v2 intents with verified detach
references; `private_attachment.rs` provides that opaque reference; and
`private_secret_store.rs` persists an immutable encrypted epoch bookmark.
Updated Plan 0130 with exact scope and production-controller prerequisites.

Recovery is offline broker-record repair, not browser attachment. It verifies
the independent controller digest/endpoint/epoch/session, immutable bookmark,
stage-3 scope, original read operation and encrypted result, complete owned
attachment/detach receipt chain, and exact pre/post broker identity. Repeated
recovery preserves the existing completion result; wrong/mixed/expired state
fails closed. The returned internal object retains the private permit and has
no key accessor or public serialization. Older v1 intents and crashes before
durable admission remain outside this bounded recovery path.

`/root/durable_recovery` implemented only the recovery module and pure tests.
Primary reviewed the full implementation, corrected post-state handle checks
for normal derived-handle rebuilding, and independently tested integration.
Fresh read-only `/root/restart_recovery_review` reported no blocking findings.
Its authority-delivery caveat remains explicit: the synthetic handoff file is
not a production trusted-controller integration.

The parent fixture kills only its own isolated synthetic child with SIGKILL at
four points: admitted intent, committed broker, updated daemon metadata, and
committed completion. A fresh process recovers twice per phase, keeps observation
locked, preserves the retained target record and fixed single-dispatch action
counts, and rejects changed authority, alternate intent reference, profile,
session, target, URL and expired lease. No destructor-only simulated restart is
used. The synthetic CDP peer does not model a surviving real Chrome process.
The child entrypoint requires parent-supplied isolated test state; exclude it
with `--skip reconciliation_crash_child` in direct ignored journey-suite runs.

Primary final validation: isolated native suite 1,218 passed, 68 ignored;
explicit ignored journey suite six passed (including the four-crash parent),
and private queue suite three passed. Earlier focused private run: 67 passed,
seven ignored; the later immutable-bookmark test is included in the widened
suite. Native build, Rust format/clippy with warnings denied, service-client
typecheck, ESLint, docs build and diff check passed. Docs retain the pre-existing
multiple-lockfile warning. Audit reported no problems but `applicable: false`.
CodeGraph refreshed. Initial test-only missing-tempfile dependency and misplaced
match-arm compilation errors were corrected before the accepted final runs;
no dependency or production validation bypass was added.

Source SHA-256: `private_recovery.rs`
`24316fd4002438e9a5b5975f4fd84cf18aff43a1575cc2badaa238174fe69a8a`;
`private_journey.rs`
`e505c0f9dcfb9217268ab6e6a9fb9f3e58b52cb68384e60cb69155206dc1e90c`;
`private_secret_store.rs`
`73fe5a678160700874283341629167625836365e8e554be7825808672fc119e7`;
`private_attachment.rs`
`31c29a7d6262ef7780476fe108eec3f8e780f796e0428f3f3d2608fdf67aa8e5`;
`native/mod.rs`
`24626a324ecd30ce491f3a5d6fbb6f98e81595c4911d0dd89530dec8e2ca64e8`.

No real credentials, live browser actions, installed runtime changes, Pro prompts,
GitHub writes or privacy release. Dirty semantic and unrelated RDP/installer work
preserved. Next: trusted-controller durable authority handoff and restart dispatch;
fresh private browser verification, guarded consumer key delivery and live renewal
remain separate gates.

## 2026-09-09 | Plan 0130 post-cleanup reconciliation

Implemented the internal success-path reconciliation seam in
`cli/src/native/private_journey.rs` and the typed worker response in
`cli/src/native/control_plane.rs`. `private_broker.rs` exposes its existing
snapshot validator internally; no validation rules were relaxed.
`private_attachment.rs` verifies completed detach against the closed transport
and encrypted acknowledgment without a second detach. `service_store.rs` adds
a private nonblocking durable mutation path while leaving ordinary writes intact.
Plan 0130 now records implemented behavior and the separate restart-recovery gate.

The coordinator admits an encrypted reconciliation intent, compares exact old
authority under broker locks, marks the retained browser connection disconnected,
clears stale target metadata/references, sanitizes local cached metadata, then
commits its private completion record. It keeps the manager, retained browser,
original target, and closed privacy authority; no browser launch/close, public
completion, key delivery or privacy release occurs. The broker's post-cleanup
URL is historical sanitation evidence, not a fresh observation after closure.

Delegation: `/root/closed_detach_proof` implemented only attachment verification
and tests; `/root/durable_reconcile_store` implemented only the repository method
and tests. Primary inspected both diffs and ran validation independently.
Fresh read-only evaluator `/root/reconcile_review` returned no blocking findings
within the frozen slice. Its caveat is retained: checkpoint failures simulate
in-process interruption, not subprocess crash/restart recovery. No reviewer
summary substitutes for primary execution evidence.

Primary focused results: 64 private tests passed, five ignored. Explicit isolated
journey fixtures: five passed, including actual socket-to-worker reconciliation,
post-login cancellation, changed profile/session/target/URL/lease refusal, and
failure after intent admission, broker persistence and daemon metadata update.
Explicit isolated private durable-store fixture: one passed, covering lock
contention, successful persistence, rejected mutation, uncertain write and invalid
state. The attachment tests include foreign client/epoch/proof, bad/missing
receipts and unfinished detach rejection, plus repeated read-only verification.
The three isolated private-queue regressions also passed explicitly, for nine
separately executed ignored fixtures in this slice.

Rust format and clippy with warnings denied, service-client typecheck, ESLint,
and docs build passed. Docs retain the existing multiple-lockfile warning.
Plan audit returned no problems but remains `applicable: false`. CodeGraph was
refreshed. Final isolated native suite: 1,214 passed, 66 ignored. Native build
and whitespace checks passed. Ignored live-browser E2E tests were not run.

Source SHA-256: `private_journey.rs`
`d93c0139ea6a0ea29e920772a98fbfcb5f022ad693aec80ad141e3478fc0ff71`;
`private_attachment.rs`
`8599f7f6e7a8382886d8078f8e2a54070a30ac286aae19844baadeb0dce2fa78`;
`private_broker.rs`
`5f509759d4a4223a03bcdaa5580389060dce5aafa93b053f7ce590c44d385d47`;
`control_plane.rs`
`4c9e7a2bb6457c41d8616f12798652179025fdbf588771104a98dcf468b77217`;
`service_store.rs`
`e7afbd8ef91f66e9d76d2e32a2c4ab27c53c8801348239bf1f465848906e8960`.

No real credentials, browser operations, installed-runtime changes, Pro prompts
or GitHub writes. Existing dirty RDP/installer changes preserved. Next: durable
intent recovery under actual subprocess restart, without replay or unlock.
Trusted consent production and guarded consumer installation/probe remain later
deployment gates; this slice does not enable renewal.

## 2026-09-09 | Push-protection fixture remediation

GitHub rejected the first private-broker checkpoint because three explicitly
synthetic test strings nevertheless matched Slack's token format. User approved
replacing the fixtures and amending the unpublished commit. Replaced them with
inert non-token placeholders in the three private Python test files; tests mock
only token acceptance for their offline transport, storage and PTY scenarios.
Production validation is unchanged. A separate unmocked regression proves the
placeholder and malformed values are rejected before network access.

Primary reran bootstrap, discovery and cross-repo adapter suites: 13, 17 and 6
passed. The PTY echo-suppression test still runs against a real synthetic terminal.
Only tests and this receipt changed since the prior validated source checkpoint;
no Rust or installed runtime changed. Preserve the prior Rust validation receipt
rather than implying a fresh Rust run. No secret-scanning exemption, allow URL,
force push or repository-rule change is authorized or used. Amend only the local
unpublished checkpoint, then retry the existing personal branch. Other dirty
RDP/installer work remains excluded. This bounded test-only correction stayed
local because delegation would duplicate its short validation path.

## 2026-09-09 | Private broker publication checkpoint

User requested personal GitHub publication and a next-step recommendation.
Selected `1baker/agent-browser`, existing
`agent/wsl-windows-chromium-paths` branch, through the verified personal helper
route. Staged only Plan 0130 private broker code, prerequisites, local Slack
helpers and matching documentation. RDP, installer and retained-URL changes
remain unstaged and preserved. This is a disabled-runtime source checkpoint,
not a release, installation or enabled renewal.

Validated an index-only exported snapshot (source tree
`a52ca22bdf9709c79daa358874a1fc7ce529dd69`, before this documentation receipt
and next-slice note): 1,211 native tests passed, 63 ignored; all six private
isolated fixtures explicitly passed separately. The difference from the prior
1,212 count is the excluded unrelated retained-URL test. Rust format and
clippy with warnings denied and native build passed. Python bootstrap/discovery/adapter suites
passed 12, 17 and 6 tests respectively; the cross-repo adapter check uses the
current local LitScout checkout, not a separately published dependency pin.
The isolated MCP absent-retained-route smoke also passed without creating a
daemon, browser or service job. ESLint and service-client typecheck passed in
the original checkout. Bounded
credential-pattern scan of 30 staged files found three explicitly synthetic
Slack token fixtures and no real token literal matching those patterns; this
is not a claim of exhaustive secret scanning. Audit had no problems but remains
`applicable: false`. No installed runtime or live browser was touched.

Read-only delegate `/root/next_renewal_slice` returned the missing post-cleanup
reconciliation seam; primary verified its source references and recorded the
bounded acceptance checks at the top of Plan 0130. Implementation is a future
slice. Durable crash recovery and guarded consumer delivery remain required
before live renewal. No automatic CI babysitting or additional GitHub writes
are part of this publication request.

## 2026-09-09 | Plan 0130 private socket-to-worker handoff

Added `cli/src/native/private_handoff.rs` and
`cli/src/native/control_plane_private_tests.rs`; integrated the separate private
worker message in `cli/src/native/control_plane.rs`, registered modules in
`cli/src/native/mod.rs`, and expanded `cli/src/native/private_journey.rs`
validation visibility and synthetic full-sequence tests. Updated Plan 0130 and
this Runbook. No public command, installed listener or approval producer exists.

Private ingress authenticates one bounded same-user connected Unix frame and
binds four operations to independently supplied consent, account, retained
identity and destinations before encrypted staging. The queue carries opaque
references, not credentials or ordinary service-job JSON. Expiry, queue failure
and cancellation fail closed without replay. Success returns only the internal
pending-reconciliation object after sanitation, owned detach and transport
closure. It does not release privacy or deliver/install the key.

`/root/private_integration_review` implemented the ingress and queue test files
only. Primary reviewed both and independently ran their tests. Independent
`/root/private_owned_attachment_review` identified active caller cancellation as
blocking; primary accepted and fixed it, and closed-world source review found
that finding resolved without a blocking regression. The initial cancellation
fixture failed because it expected lowercase `faulted` instead of the existing
`Faulted` status spelling; corrected and rerun successfully. No secret-bearing
type gained Debug or public serialization to satisfy a test assertion.

Primary validation: 62 focused private tests passed (three ignored). Explicit
isolated `native::private_journey::tests::` ignored run: three passed, including
authenticated socket-to-worker completion and cancellation after login with no
backup/renew/read dispatch. Explicit isolated
`native::control_plane::private_tests::` ignored run: three passed, covering full
and closed queues, expiry while queued and cancellation before dequeue, all
without ordinary job persistence. All tests use synthetic data and CDP peers.

Broader isolated native suite: 1,212 passed, 63 ignored. The six isolated-only
private fixtures above were explicitly run separately; other ignored live E2E
tests were not run. Service-client typecheck, ESLint, Rust format, clippy with
warnings denied, native build, docs build and whitespace check passed.
Docs retain the pre-existing multiple-lockfile warning. Planning audit reported
no problems but `applicable: false`, not full contract coverage. CodeGraph was
refreshed after source integration.

Source SHA-256: `private_handoff.rs`
`8a61b3e5d67176fcc21e6e8726a4a5dad5d8581bc0e53d551e14c697c0ab9c1f`;
`control_plane.rs`
`453f52ced08b9946a06541b134c79491657ce4d694a1936943192a68c7ea5935`;
`control_plane_private_tests.rs`
`8fbce1bc83806cc810602032f93095be59a6cb63b9be86a6c46d8d321ea6602b`;
`private_journey.rs`
`5134f3c701a7ad323e7130cc2d211106c3691d5fcf67703257bd93b516f666cd`.

No real credentials, live browser operations, runtime installation, ChatGPT
submissions or GitHub writes. Dirty work preserved. Next: bounded recovery and
daemon/broker reconciliation, then independently authorized ingress deployment
and guarded LitScout key installation with a real consumer probe. Renewal stays
disabled until those gates are verified; internal tests are not live acceptance.

## 2026-09-09 | Plan 0130 owned attachment integration

Added `native/private_attachment.rs` and routed private execution, transitions
and sanitation through its dedicated original-target session. Cleanup now
detaches the owned session before transport closure. It never treats the generic
borrowed-session `cdp_detach` acknowledgment as private cleanup proof. Existing
public attach/detach behavior is unchanged.

Changed `native/mod.rs`, `private_execution.rs`, `private_broker.rs`,
`private_journey.rs` and the internal permit check in `cdp/client.rs`.
Test-only constructors in `actions.rs` and `browser.rs` support an isolated
full-sequence fixture without launching Chrome or changing runtime authority.
The fixture refuses to run outside the repository isolated runner.

`/root/private_integration_review` implemented only the attachment module and
reported four passing tests. Primary inspected its code and independently ran
58 focused private tests (one ignored), then explicitly ran the ignored full
production-path synthetic sequence: one passed. Fresh closed-world evaluator
`/root/private_owned_attachment_review` found no blocking issue by source review.

Primary final gates: isolated native suite 1,208 passed, 58 ignored; Rust format
and clippy with warnings denied passed; service-client typecheck, ESLint, native
build, docs build and whitespace check passed. The ignored full-sequence fixture
was run explicitly and passed separately. Docs retain the existing multiple-
lockfile warning. Planning audit reported no problems but `applicable: false`,
not full contract coverage. CodeGraph was refreshed after the integration.

Source SHA-256: `private_attachment.rs`
`c9177df70f8aa790b7c86b4da45853836118033bc9e90da171577a371dec2a7e`;
`private_journey.rs`
`db3359b6f403e7120b717b06f99dc2181622b394f25ccafc22d9c2e0de8d0bca`.

No live credentials, retained-browser operations, runtime installation or GitHub
writes. The remaining ingress, daemon-state reconciliation and consumer-delivery
gates are explicit in Plan 0130; this is not enabled renewal or a Pro verdict.

## 2026-09-09 | Plan 0130 shared transport cleanup

Updated `native/cdp/client.rs`, `native/privacy_gate.rs` and
`native/private_journey.rs`: shared terminal sink state, cancellation-safe
worker joins, exact epoch/client closure proof, and an internal sanitation-to-
transport-closure handoff that still requires broker detach. Seven new synthetic
transport regressions cover retained handles, queued writes, cancellation,
concurrent cleanup, delayed replies, worker panic and wrong authority; the
existing sanitation fixture also exercises transport closure without unlocking.

Independent read-only review `/root/private_integration_review` completed,
including the `Arc<CdpClient>` revision; no blocking issue found in this slice.
Primary ran tests. No production ingress, broker detach, privacy release, key
installation or live renewal is claimed. Plan 0130 records remaining deployment
gates. No runtime installation, private browser mutation or GitHub writes.

Validation detail: 16 focused CDP-client tests passed, including seven new
transport cases. Rust format, clippy with warnings denied, service-client
typecheck, ESLint, native build and docs build passed. Docs retain the existing
multiple-lockfile warning. Planning audit returned no problems but
`applicable: false`; CodeGraph was refreshed and whitespace check passed.

The initial non-isolated wider run failed the existing renderer-deadline and
inspect-server shutdown checks and was terminated. Both checks passed in fresh
isolated environments. Persistent host/port-scoped privacy-gate contamination
is a plausible explanation, not a proven root cause. Final wider evidence uses
the repository's isolated runner: 1,204 passed, 57 ignored, zero failures. The
failed/stopped run is not accepted as a pass. No live privacy gate was cleared
to make tests pass.

Source SHA-256: `cdp/client.rs`
`b165e132819f1f04395982ff53cd884b4f6f85fb087dd29681dcc32a53720d95`;
`privacy_gate.rs`
`581e4bd192ab462e97f421b25ddb116166260a7f836ec476a44670f3257f40b6`;
`private_journey.rs`
`1904ea1b51a4c7965f620c02e05384049e972d512df92516a4e57121bb3094b1`.

## 2026-09-09 | Plan 0130 internal ordered execution

Added `cli/src/native/private_journey.rs`, registered it in `native/mod.rs`,
and added whole-consent reservation to `private_secret_store.rs`. All four
stages validate before admission; one private permit spans exact transitions.
Whole-run consumption survives fresh staging and subprocess death. Completion
remains pending cleanup, never public renewal or activation evidence.

Independent read-only review `/root/private_integration_review` completed and
its whole-consent replay finding was implemented. Closed-world review found no
blocking issue in this disabled internal slice; primary ran validation. The
first concurrency test expected only a consumed-receipt error and failed on
the store's intended nonblocking busy rejection. Corrected the test to require
exactly one winner, only either safe rejection, and subsequent consumed proof.

Primary final validation: 54 focused private tests passed; 1,197 wider native
tests passed, 57 ignored. Rust format, clippy with warnings denied, native debug
build, service-client typecheck, ESLint, docs build and whitespace check passed.
The initial wider run overlapped a test-binary rebuild and reported subprocess
executable-not-found failures plus the old concurrency assertion. That run is
not accepted evidence; the final wider run used the corrected built test binary
without another test rebuild. Docs build retains the existing multiple-lockfile
warning. Planning audit returned no problems but `applicable: false`, not full
planning-contract coverage. CodeGraph is current.

Source SHA-256 bindings: `private_journey.rs`
`60b4e8489643fcd01e0e7feb5e26401787dcd7607530a9dcab033c0313f07256`;
`private_secret_store.rs`
`34f5d2aa0823c5be3e5ec1044afa1c2572bdc1ae64c6cf4d036b0e32cbd80a47`.

No live credentials, browser actions, installation or GitHub writes. Remaining
deployment gates and evidence limits are in Plan 0130.

## Turn 183 | 2026-08-29

Scope: implement, publish, and safely migrate P126 without interrupting active
Termius SSH terminals.

Actions:

- Added the generic dry-run-first `electron relay` lifecycle, exact Windows
  process/listener authority checks, loopback-only foreground relay, bounded
  user timer, drift-guarded uninstall, docs, help, skill guidance, and isolated
  lifecycle fixture.
- Repaired attached-existing runtime handoff identity and liveness handling,
  stale intermediary PID recovery, a disappearing idle-daemon executable race,
  and persistent DevTools HTTP response validation found during live migration.
- Published the runtime through guarded browser handoff, migrated Termius to
  the managed unit, and quarantined rather than deleted the obsolete wrapper
  and units.
- Removed only tailnet TCP 2223. Preserved tailnet TCP 22 forwarding to the
  home SSH listener and every HTTPS/web route.

Validation:

- Electron relay fixture, runtime handoff selection, publisher orchestration,
  local convergence, Rust formatting, focused Rust tests, production-target
  strict Clippy, lint, typecheck, dashboard/docs builds, optimized build,
  direct plan audit, CodeGraph synchronization, and diff hygiene.
- All-target strict Clippy remains blocked by 12 existing test-only findings in
  unrelated `mcp.rs`, service, and control-plane test code; those user changes
  were preserved rather than folded into P126.
- Installed doctor reported `ready=true`, Windows PID 6672, active managed PID
  2831626, and loopback-only endpoints. Service stop/start preserved all three
  exact Termius target IDs.
- The publication journal reached terminal `ready`; retained browser handoffs
  and the exact Workshop conversation survived without a prompt submission.

Result:

- P126 is ready for its disruptive drill. A real Termius close/reopen and a
  coordinated reboot remain operator-gated because they close active SSH
  terminals; neither was performed during this turn.
- Bilateral DOCX/PDF submission stopped before Send because the document guard
  rejected the retained AuraCall session as not live even though read-only CDP
  and service inventory proved the exact browser and Workshop target healthy.
  No substitute browser was launched and no document prompt was submitted.

## Turn 182 | 2026-08-29

Scope: begin P126 managed WSL Electron relay productization.

Actions:

- Froze an immutable intake covering a generic loopback-only Windows Electron
  relay lifecycle, retained Termius preservation, close/reopen recovery, and a
  separate Tailscale cleanup audit.
- Reused Plan 0098's exact process, listener-owner, and private-loopback
  invariants instead of broadening Windows network exposure.
- Confirmed the bilateral controller can enforce complete per-side packet
  collection, Codex approval before Pro submission, and one bridge transit.
- Recorded a no-delegation receipt because this runtime does not authorize
  sub-agent spawning and the change has one overlapping lifecycle surface.

Validation:

- `codegraph status`
- `./bin/dual_hemisphere_thought --help`
- `git status --short --branch`

Result:

- P126 is active. No browser, relay, SSH, firewall, Tailscale, or installed
  runtime state changed during intake.

## Turn 181 | 2026-08-27

Scope: replace static runtime-handoff alias coverage with behavioral recovery
identity, liveness, and retry-record cleanup tests.

Actions:

- Extracted one fail-closed runtime handoff browser selector shared by guarded
  publication and retained-browser preparation.
- Made a known prepared browser PID authoritative over leftover CDP service
  text and retained the exact live-PID bridge-alias recovery case.
- Added exact, idempotent durable retry-record cleanup that preserves changed
  records for investigation.

Validation:

- runtime handoff selection and local convergence fixtures
- retained-browser live, discovery, and guard fixtures
- publisher smoke policy, lifecycle, orchestration, journal, and operation
  fixtures
- route-confusion gates, lint, release-asset verification, CodeGraph sync,
  validation selection, and diff hygiene

Result:

- Exact session and bridge-alias recovery behavior now has executable coverage.
  Known stale PIDs, identity mismatches, and ambiguous matches fail closed.
- Exact retry records are removed idempotently after already-resumed proof;
  changed records remain available for investigation.
- No browser or installed runtime was mutated. An unrelated `cli/src/mcp.rs`
  edit that appeared during the turn remains untouched and outside this slice.

## Turn 180 | 2026-08-27

Scope: repair split daemon namespaces and deterministic managed one-time profile
admission without disturbing retained browsers.

Actions:

- Added secure Linux runtime-directory inference shared by the Rust CLI,
  convergence, dashboard publication, retained-browser verification, and
  executable-handoff smoke discovery.
- Preserved explicit socket and XDG overrides and rejected unsafe inferred
  runtime directories before the existing home fallback.
- Registered only the exact deterministic managed one-time profile recommended
  by remote-view acquisition while retaining arbitrary-profile warnings.
- Updated README, sessions docs, and repository plus installed skill guidance.

Validation:

- focused Rust connection, remote-view, close, and CDP stream tests
- local convergence and dashboard publication fixture suites
- route-confusion gates, release verifier fixture, lint, docs build, strict
  Clippy, optimized build, CodeGraph sync, direct plan audit, and diff hygiene

Result:

- Source validation passes. Guarded publication found three legacy migration
  defects and rolled back safely each time: unbounded old-daemon shutdown,
  deleted proc-executable reuse, and broker ownership hidden behind a
  differently named bridge service record. Each now has a source fix and
  focused regression.
- Guarded publication and explicit recovery reached terminal
  `recovered_ready`. The source-free workstation payload is installed, compact
  install and remote-view doctors report zero issues, remote control and
  many-to-many readiness are ready, and runtime convergence reports five
  runtimes with zero stale entries.
- Installed and live dashboard executable SHA is
  `96e141a23e3da440d7757e0ff994e6eca0caf672f8b22df3a23c42cd675b4c42`.
  AuraCall, NYSE, both retained route viewers, and the default browser preserved
  their PIDs; exact retained CDP and target checks pass. No ChatGPT prompt or
  page interaction was sent.

## Turn 179 | 2026-08-24

Scope: add bounded doctor JSON for agent consumption without weakening the
authoritative no-launch probes or retained-browser publication guard.

Actions:

- Added command-specific `--compact --json` projections for install doctor and
  remote-view doctor with versioned schemas, total and omitted issue counts,
  at most 20 issue details, readiness fields, and actionable remedies.
- Updated CLI help, README, installation and command docs, and repository plus
  installed agent-browser skill guidance.
- Aligned native retained-browser verification with the publisher by requiring
  the named daemon PID to exist and be live before persisted browser or CDP
  evidence can verify an authority record.
- Preserved the dirty worktree and all pre-existing live browser processes
  during implementation validation.
- After explicit operator authorization, restored `chatgpt-pro` on the exact
  retained Workshop URL, archived the obsolete dead-target authority, and
  marker-first pinned the unique replacement target without page interaction.
- Guarded publication handed four active browser sessions to the replacement
  executable. Recovered the old dashboard manifest through the explicit
  recovery-only path with the user bus and canonical daemon socket directory.
- Refreshed the source-free workstation payload without interactive sudo.

Validation:

- 8 compact-focused Rust tests and 14 retained-requirement tests
- full install-doctor and remote-view-doctor focused suites
- Rust formatting and strict Clippy
- docs lint and production build
- optimized Rust build and native payload copy
- CodeGraph sync, installed-skill parity, direct plan audit, and diff hygiene

Result:

- Compact release output is approximately 78 percent smaller for install
  doctor and 95 percent smaller for remote-view doctor than full JSON.
- Guarded installation and recovery are complete. The publication journal is
  terminal `recovered_ready`; installed, release, and workspace-native binary
  SHA-256 are
  `143fc1f682c9c1ef7d7fd1d51175d1a66330e58eeb146803481721d8be0bb4f1`.
- Installed compact install and remote-view doctors report zero issues,
  runtime convergence and remote control are ready, and the exact Workshop
  target remains verified. No ChatGPT prompt or page interaction was sent.

## Turn 178 | 2026-08-24

Scope: reconcile the installed 0.28.0 workstation payload and repair
convergence dry-run visibility.

Actions:

- Refreshed the source-free workstation payload manifest to the installed
  `ff85f1f9` binary without interactive sudo.
- Handed `figurelabs-saber` from the deleted executable to the current binary
  while preserving its browser PID and exact CDP endpoint.
- Made convergence dry-run parse repairable nonzero doctor JSON so it can
  report `safeRemedies` without applying them.
- Added a fixture for the nonzero stale-daemon preview path and aligned README,
  installation docs, skill guidance, and roadmap evidence.

Validation:

- `pnpm test:local-runtime-convergence`
- JavaScript syntax checks for the controller and fixture
- targeted diff hygiene
- installed no-launch retained-browser status
- installed install doctor and remote-view doctor

Result:

- Retained-browser authority remains configured and verified without launch.
- Install doctor reports the payload installed, runtime convergence complete,
  zero stale runtimes, and no issues.
- Remote-view doctor reports remote control and many-to-many readiness.
- One optional source-checkout Guacamole schema ensure still looks for the old
  unversioned compose path, but route readiness and both final doctors pass.

## Turn 177 | 2026-08-15

Scope: remove the manual managed-browser navigation handoff that blocks exact
P121 retained-lane enrollment.

Actions:

- Opened P122 without weakening P121's read-only discovery contract.
- Added exact-URL and required-profile selectors to retained-browser discovery.
- Added a guarded preparation command that composes only route-bound
  `remote-view open` and digest-bound retained requirement pinning.
- Required exact rendered URL, profile, target, browser, session, and
  operator-visible route agreement before unique discovery.
- Added contract and isolated command fixtures that reject drift and prove no
  page-interaction or prompt-submission action is invoked.

Validation:

- `pnpm test:local-dashboard-retained-browser-discovery`
- `pnpm test:local-dashboard-retained-browser-preparation`
- `pnpm test:local-dashboard-retained-browser-preparation-command`
- JavaScript syntax checks for the new command and libraries
- retained guard, requirement, watchdog, publisher operations, and publisher
  orchestration fixtures
- service-client typecheck
- dashboard and docs production builds
- runtime convergence contract and isolated fixture
- package JSON parsing, installed skill SHA-256 parity, and full diff hygiene

Result:

- Focused fixtures pass. The live Workshop command remains intentionally
  unexecuted under the active no-navigation and no-submission boundary.
- Full docs lint remains red only on the pre-existing synchronous state update
  in `docs/src/components/theme-toggle.tsx`; production compilation passes.
- Next, finish widened validation and then run live exact-URL preparation only
  after that navigation boundary is explicitly lifted.

Follow-up:

- Replaced the preparation command's repository publisher dependency with a
  bounded live discovery, reverification, and marker-first writer module.
- Embedded the controller and five dependencies in the versioned workstation
  support payload and exposed
  `agent-browser install workstation prepare-retained-browser`.
- Repaired the docs lint baseline; full docs lint and build now pass.
- The source-free workstation fixture, 20 native workstation tests, strict
  Clippy, Rust formatting, preparation fixtures, and optimized build pass.
- Candidate SHA-256 is
  `954a01d3783912a129a1a23f6dc6c606dc455ce1991d380a64b0a898d2f29389`;
  installed runtime remains unchanged pending exact retained-lane enrollment.
- Added a disposable localhost live preparation smoke. It failed closed before
  browser launch because Route A belongs to `away-auth-handoff` and Route B to
  `nyse-developer-route`; both allocations are live.
- Preserved both retained browsers and routes. No fixture authority file or
  browser survived. The wrapper now returns the bounded structured child error
  so route-capacity failures are actionable.
- Relinked the optimized candidate after that embedded controller change.
  Current SHA-256 is
  `fc4fd837feeadbe9da3ffa9c2bab903a4af64005ad2504330e4e46498675bf0e`;
  the installed binary remains unchanged.
- A third live audit found both routes unchanged and zero apply-safe display
  allocations. P122 is now blocked rather than repeatedly requesting manual
  work. Unblock by freeing a route or approving a reviewed parking policy and
  lifting only the exact-Workshop navigation prohibition; prompt submission
  remains unauthorized.
- After explicit delegation, parked only Route A's viewer binding and preserved
  the `away-auth-handoff` and NYSE browser processes and targets.
- Added stable route-specific display allocation ids, exact pending-acquisition
  ownership checks, and runtime-profile-bound preparation daemon sessions.
- The disposable localhost preparation then passed and cleaned up. The exact
  Workshop navigation reached ChatGPT but redirected to `/project`, so exact
  verification failed closed and neither authority file was created.
- No prompt, click, type, fill, evaluate, upload, send, or submit action ran.
  The optimized candidate is
  `2bbfe6d53e7553424e88773d67f222f0105d2dc791fe67d5098b2022bd8a3e88`;
  installation remains blocked on a real exact Workshop conversation URL.
- After explicit authorization for exactly one short Workshop prompt, attached
  directly to the surviving `chatgpt-pro` Chrome DevTools endpoint, submitted
  one concise user turn, and captured canonical conversation
  `6a80e64e-e830-83ea-b21f-9079abf27a1d`. No retry or second prompt ran.
- Preserved the existing default NYSE retained requirement and committed a
  separate mode-0600 Workshop requirement for session/profile `chatgpt-pro`,
  target `91DBB20C67DFB0398978722D6B6FA85A`, and the exact new URL.
- Guarded release publication completed at `final_readiness`; installed and
  release SHA-256 both equal
  `7704b89a579e6bb1678d43cbe3d3ea402197a411525cab83e9f6a641228755bb`.
  Five daemon sessions are converged, the live dashboard manifest is ready,
  remote-view doctor is ready, and the retained Workshop PID and target remain
  exact. P122 is closed with no GitHub write.

## Turn 176 | 2026-08-15

Scope: remove error-prone retained-lane identity copying from the final P121
operator handoff.

Actions:

- added a read-only discovery mode for requirement pinning that accepts one
  reviewed origin and path prefix, scans live retained daemon and CDP evidence,
  and derives the exact session, profile, target, PID, endpoint, and canonical
  URL;
- required exactly one ready page target and failed closed on zero or multiple
  matches, service or CDP read failure, session mismatch, degraded health,
  incomplete identity, and origin or path-prefix spoofing;
- aligned the repository Node guard with the installed native verifier by
  requiring `health=ready` before pinning;
- isolated the convergence fixture's socket directory after widened validation
  caught it reading a live unrelated `etf-paper-dashboard` token through the
  inherited WSL runtime directory;
- preserved the no-prompt boundary and inspected only no-launch service state.

Validation:

- the focused discovery, durable requirement, and JavaScript syntax checks
  pass, and the widened publisher, publication-operations, watchdog,
  source-free workstation, and isolated convergence fixtures pass;
- current live discovery state contains zero ChatGPT targets, so no private
  requirement or enforcement record was written;
- the optimized source build and docs production build pass. The release
  candidate SHA-256 is
  `0e2186376c0795d6df684797f779ce94400f99fb8d582c68b03741918f306acf`;
- live drift reconciliation found a separate terminal publisher transaction at
  13:58 that installed an earlier unguarded candidate with SHA-256
  `758c9f4d4e89941799dbe053357e50597e13b633e3e6013f8a5e7259d39b2984`.
  No retained browser existed during that transaction.

Outcome: after the operator opens the intended Workshop page, one reviewed
project URL prefix is sufficient for exact fail-closed enrollment. P121 remains
open for that operator-established lane, guarded installation, and installed
no-prompt readback.

## Turn 175 | 2026-08-15

Scope: finish the post-relogin workstation reconciliation and make its
disposable remote-view live gate deterministic.

Actions:

- completed the source-free workstation reconciliation after the required WSL
  relogin and restored distinct route-specific XRDP desktops on `:10` and
  `:11`;
- made workstation reconciliation bind Guacamole viewer daemons to the
  installed Linux Chrome instead of inheriting an ambient Windows StealthCDP
  executable that cannot render on an XRDP display;
- moved the local remote-view fixture server to a worker so synchronous CLI
  probes cannot starve its HTTP event loop;
- added a blocking-parent regression and made OCR require the unique stable
  marker prefix while exact CDP URL and title checks retain full fixture
  identity;
- preserved the existing Workshop no-prompt boundary and used only disposable
  local fixture profiles.

Validation:

- install doctor and remote-view doctor report ready with no issues; dashboard,
  runtime interlock, and PostgreSQL backup timers are active;
- 20 workstation Rust tests, the blocking-safe fixture regression, the
  source-free workstation fixture, Rust formatting, strict Clippy, syntax, and
  diff hygiene pass;
- the optimized remote-view fixture gate passes with exact URL/title readback,
  matching X11 PID, `route_bound_ready`, visible-window and OCR proof, one
  active target after repeated opens, and automatic cleanup;
- release candidate SHA-256 is
  `a886febac4a3794bd0b20387e603406b319e9419c2047cd2b1b15cd404450eec`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: workstation remote control is operational and the release candidate's
rendered route gate is deterministic. P121 remains open for an
operator-established Workshop lane, durable pinning, guarded installation, and
exact no-prompt retained-target readback. No ChatGPT prompt or GitHub write
occurred.

## Turn 174 | 2026-08-15

Scope: bind the marker-first retained-browser enforcement record to exactly one
stable-identity requirement.

Actions:

- added `requirementSha256` to the private v1 enforcement contract;
- made crash retry reuse the marker's original timestamp and exact digest;
- made Node and native readers reject a stale marker paired with changed
  requirement bytes before service-state access;
- preserved legacy markerless requirement reads and digest-bound those records
  on the next idempotent pin;
- added changed-evidence, replaced-bytes, native pre-state, and source-free
  release-binary regressions;
- updated README, installation docs, contract guidance, repository and
  installed skills, P121, roadmap, and implementation evidence.

Validation:

- all 1,874 non-ignored Rust tests pass with 57 ignored; Rust formatting and
  production-binary strict Clippy pass;
- retained requirement/watchdog, publication operations/orchestration,
  source-free workstation, dashboard, docs, schema, skill-parity, and diff
  gates pass;
- live optimized status remains `not_configured` and leaves service state plus
  publication journal byte-identical;
- release candidate SHA-256 is
  `f129a45f9e7b70506808723637432038c6b9e94ccf9327ab24cb8c858f6b88b3`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: marker and requirement can no longer be mixed across enrollment
attempts. P121 remains open until an operator-established Workshop lane can be
pinned, safely published, and read back without a prompt. No browser or GitHub
write occurred.

## Turn 173 | 2026-08-15

Scope: prevent a crashed or partially lost retained-browser pin from silently
disabling the publication and workstation interlock.

Actions:

- added a private marker-first `.required` enforcement record with a bounded v1
  contract and durable atomic commit;
- made repository publication, read-only watchdog, native status, workstation
  apply, and recurring reconcile fail closed when enforcement exists but the
  identity requirement is missing;
- rejected unsafe enforcement permissions, ownership, size, schema, and
  symlink shape in both Node and native readers;
- added simulated crash-after-marker, idempotent retry, deletion, unsafe-file,
  no-build, no-state, and no-service-command regressions;
- updated CLI help, README, installation docs, repository and installed skill
  guidance, P121, roadmap, and implementation evidence.

Validation:

- all 1,873 non-ignored Rust tests pass with 57 ignored; Rust formatting and
  production-binary strict Clippy pass;
- focused retained-browser, watchdog, publisher operations/orchestration,
  source-free workstation, dashboard, docs, schema, skill-parity, and diff
  gates pass;
- live optimized status reports unconfigured enforcement and leaves service
  state plus publication journal byte-identical;
- release candidate SHA-256 is
  `ab15748e93330c2c4d61d1bc6b0ff2de18b86be3852869a2f83b657424b13165`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: a single missing requirement can no longer disable an enforced lane.
P121 remains open because the retained Workshop lane and durable requirement
are absent. No browser launch, prompt, navigation, typing, click, install, or
GitHub write occurred.

## Turn 172 | 2026-08-15

Scope: make the source-free retained-lane watchdog independently auditable and
configuration-equivalent to repository tooling.

Actions:

- added `install workstation retained-browser-status --json` as a native,
  no-lock, no-daemon, no-launch, identity-redacted audit surface;
- made native status, apply, and reconcile honor the configured durable
  requirement path;
- added the retained-requirement path to the normal dotenv allowlist so
  `AGENT_BROWSER_ENV_FILE` and `~/.agent-browser/.env` work without exports;
- added default, direct environment, dotenv, override, no-state, and invalid
  configured-path regressions;
- updated help, README, installation/configuration docs, repository and
  installed skill guidance, P121, roadmap, and implementation evidence.

Validation:

- all 1,871 non-ignored Rust tests pass with 57 ignored; formatting and strict
  Clippy pass;
- source-free workstation fixture, dashboard and docs builds, installed skill
  parity, and diff hygiene pass;
- live optimized status reports `not_configured` and leaves service state plus
  publication journal byte-identical;
- release candidate SHA-256 is
  `ce5a783994a8744de27e6f59c0be145ee64de78a4379248c50868b3dcddd7835`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: source-free retention status and configuration parity are ready. P121
remains open because the retained Workshop lane and durable requirement are
absent, so safe installation and exact no-prompt readback remain unproven. No
browser launch, prompt, navigation, typing, click, install, or GitHub write
occurred.

## Turn 171 | 2026-08-15

Scope: close the source-free workstation retention-watchdog bypass.

Actions:

- added a native bounded verifier for the private durable retained-browser
  requirement, persisted service identity, live PID, and loopback DevTools
  target inventory;
- made workstation apply check before lock, sudo, staging, or quiescence and
  made recurring binary-owned reconcile recheck before its first mutation;
- required one exact ready browser, active session, valid service-tab handle,
  profile, target, and canonical URL while keeping output identity-redacted;
- added isolated exact-match, drift, ambiguity, unsafe-file, no-mutation, and
  source-order regressions;
- updated CLI help, README, docs, repository and installed skill guidance, P121,
  and the implementation note.

Validation:

- all 1,870 non-ignored Rust tests pass with 57 ignored; formatting and strict
  Clippy pass;
- focused retained-browser, publication, convergence, source-free workstation,
  host-provision, fresh-install, Guacamole, PostgreSQL, route-user, release,
  service-client, dashboard, and docs gates pass;
- installed skill parity and diff hygiene pass. Targeted MDX lint reports only
  that the MDX file has no matching ESLint configuration;
- release candidate SHA-256 is
  `b8eaac13578f3380aca701e4ea127d2653b4d2a22fdbb73a1172e8738fc474dc`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: the binary-owned watchdog bypass is closed in source. P121 remains
open because the retained Workshop lane and durable requirement are absent, so
safe installation and exact no-prompt readback cannot yet be proven. No browser
launch, prompt, navigation, typing, click, install, or GitHub write occurred.

## Turn 170 | 2026-08-15

Scope: make the exact retained-browser publication gate durable and recurring
instead of dependent on repeated command-line flags.

Actions:

- added a private, bounded, owner-only, non-symlink requirement containing only
  stable session, profile, target, and canonical URL identity;
- made normal publication load and enforce that requirement before any build;
- added verified idempotent pinning that excludes browser PID, browser ID, and
  DevTools endpoint from durable state;
- made the read-only check report `not_configured` when no critical lane exists
  and fail closed on malformed or mismatched configured state;
- added the same read-only gate before source-checkout user-service interlock
  convergence.

Validation:

- retained requirement, guard, watchdog, publication operation,
  orchestration, and local convergence fixtures pass;
- a durable missing-lane fixture blocks publication before dashboard or Rust
  build and changes no retained runtime state;
- JSON schema parsing and script syntax pass.
- all 1,861 non-ignored Rust tests pass with 57 ignored; formatting, strict
  Clippy, client typecheck, dashboard and docs builds, workflow parsing,
  installed skill parity, and diff hygiene pass;
- release candidate SHA-256 is
  `68303e1b9c89aa7e8a1f81cf7e646ac0f570208cf5c2559635cb186f95f40381`;
  installed SHA-256 remains
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

Outcome: the flag-memory and recurring-watchdog gap is closed in source. P121
remains open because the Workshop lane is absent, so the durable requirement
cannot be safely pinned and installed acceptance cannot yet be proven. No
browser launch, prompt, navigation, typing, click, install, or GitHub write
occurred.

## Turn 169 | 2026-08-15

Scope: isolate the missing Workshop browser and make exact retained identity a
fail-closed publication and recovery gate.

Actions:

- established that the browser process exited about eight minutes before the
  P121 publication transaction; host and Chrome logs do not identify the
  terminating actor;
- added exact retained session, profile, target, URL, PID, and CDP pinning
  before publication mutation, after handoff, at final readiness, and during
  recovery;
- added a read-only no-lock, no-build preflight that refuses an absent daemon
  without auto-launching a replacement;
- added a serial Rust validation wrapper with disposable home, agent root,
  sockets, and runtime state;
- projected only required, verified, and stage guard fields to installed status
  and added a distinct terminal unverified classification.

Validation:

- retained guard, orchestration, recovery, journal, operations, lifecycle,
  smoke policy, publication status, doctor, client, contract, parity, dashboard,
  docs build, formatting, and strict Clippy gates pass;
- the isolated full Rust suite passes 1,861 tests with 57 ignored and does not
  change live publication or Workshop inventory hashes;
- the exact Workshop read-only preflight returns `retained_daemon_missing` and
  changes no service state, journal bytes, session inventory, or lock state;
- release binary SHA-256 is
  `03efdddf4bf80cdeb377e549f2d1ad818e493e6248a92aa661a32afdda9d39e9`.

Outcome: P121 remains open only for live acceptance criterion 8. The release
candidate is built but not installed because no retained Workshop lane exists
to satisfy the new guard. No replacement browser, prompt, navigation, typing,
click, recovery, installation, or GitHub write occurred.

## Turn 168 | 2026-08-15

Scope: expose durable dashboard publication status to installed operators while
preserving the recovery-only authority boundary.

Actions:

- added a source-free Rust journal, lock, transaction, and installed-artifact
  inspector with bounded reads and fail-closed validation;
- projected status into install doctor, authenticated HTTP, MCP, service
  contract metadata, generated client types, and the Service dashboard;
- added a versioned JSON schema and isolated no-launch endpoint coverage;
- kept recovery out of HTTP, MCP tools, and dashboard controls and marked the
  exact recovery-only command as requiring explicit operator confirmation.

Validation:

- focused and widened Rust, doctor, HTTP, MCP, contract, client, dashboard,
  publisher lifecycle, orchestration, journal, operations, parity, typecheck,
  production build, documentation build, JSON, YAML, skill-parity, plan, lock,
  and diff gates pass;
- guarded publication completed terminal phase `ready`; installed and release
  binary SHA-256 is
  `2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`;
- repository status, install doctor, authenticated HTTP, and MCP agree on a
  verified replacement with no lock, no recoverable state, and action `none`;
- no recovery, prompt submission, navigation, typing, click, or replacement
  browser launch occurred.

Outcome: installed publication projection is verified, but P121 remains open.
The prior Workshop Chrome PID and CDP endpoint are gone and service state has no
matching retained browser, session, or target. Exact retained-target readback is
therefore blocked until the disappearance is isolated and a retained lane is
available for no-prompt verification.

## Turn 167 | 2026-08-15

Scope: separate read-only publication inspection from explicit recovery
authority and prevent recovery-only from authorizing a new build.

Actions:

- added read-only journal, lock, transaction, and installed-artifact status;
- classified active publisher, recoverable transaction, absent journal, and
  unknown installed artifact into exact next actions;
- added recovery-only no-op before build and reused P119 recovery for incomplete
  transactions;
- rejected combined inspection and recovery flags;
- added package commands plus isolated command and CI coverage.

Validation:

- focused status, recovery-only, journal, orchestration, lifecycle,
  smoke-policy, and syntax checks pass;
- disposable status creates no runtime directory, and recovery-only leaves no
  binary, journal, or lock;
- no live status, recovery, build, install, listener, daemon, or browser action
  has occurred yet.

Outcome: P120 is closed. Focused and widened status, recovery-only, journal,
publisher, convergence, capability, parity, dashboard, client-type, syntax,
production-build, package, YAML, skill-parity, plan-audit, cleanup, and diff
gates pass. Docs-wide ESLint retains one existing effect error and one warning
in untouched files. Live status returned no journal, no lock, and action `none`
without creating state. Retained browser and installed runtime identity remain
unchanged. No GitHub write, prompt, browser interaction, lifecycle, handoff,
install, recovery, or live dashboard action occurred.

## Turn 166 | 2026-08-15

Scope: make dashboard publication artifact evidence durable and recover an
interrupted transaction before any new build or mutation.

Actions:

- added a mode-0600, fsynced, atomically replaced, revision-checked journal and
  exact PID publication lock;
- checkpointed quiescence, handoff, replacement, resume, restart, readiness,
  failure, and rollback phases;
- recovered exact handoff descriptors and reconciled already-resumed browser
  PID/CDP identity without replay;
- recognized crash-after-replacement from durable built SHA evidence and repeated
  reference sync from the verified installed binary;
- added isolated journal, recovery, stale-lock, revision-conflict, corruption,
  and fail-closed fixtures to fast Dashboard CI.

Validation:

- focused journal, orchestration, lifecycle, smoke-policy, and syntax checks
  pass;
- no live runtime, listener, daemon, browser, profile, or external service was
  touched.

Outcome: P119 is closed. Focused and widened journal, recovery, publisher,
convergence, capability, parity, dashboard, client-type, syntax,
production-build, package, YAML, skill-parity, plan-audit, cleanup, and diff
gates pass. Docs-wide ESLint retains one existing effect error and one warning
in untouched files. Read-only evidence shows the retained browser identity,
dashboard manifest, and installed SHA unchanged; no production journal or lock
was created. No GitHub write, prompt, browser interaction, lifecycle, handoff,
install, recovery, or live dashboard action occurred.

## Turn 165 | 2026-08-15

Scope: bind publisher replacement and rollback to cryptographic artifact
evidence and preserve the initiating error across restore failure.

Actions:

- added structured built, source, backup, replacement, restoration, and safe
  restart SHA-256 evidence to the publisher report;
- made backup mismatch fail before quiescence and replacement mismatch enter
  verified rollback;
- retained the original publication failure across missing-backup, restore-copy,
  and restore-hash failures;
- prohibited restart when the installed artifact matches neither the verified
  backup nor verified replacement;
- extended the existing isolated real-file and fast smoke-policy fixtures.

Validation:

- focused orchestration, publisher policy, convergence, and syntax checks pass;
- no live runtime, listener, daemon, browser, profile, or external service was
  touched.

Outcome: P118 is closed. Focused and widened publisher, convergence,
capability, parity, dashboard, client-type, syntax, production-build, package,
YAML, skill-parity, plan-audit, cleanup, and diff gates pass. Docs-wide ESLint
retains one existing effect error and one warning in untouched files. Read-only
evidence shows the retained browser identity, dashboard manifest, and installed
binary SHA unchanged. No GitHub write, prompt, browser interaction, lifecycle,
handoff, install, or live dashboard action occurred.

## Turn 164 | 2026-08-15

Scope: make full dashboard publication sequencing and rollback fault-testable
without a live build, install, service restart, or browser handoff.

Actions:

- extracted build, backup, quiesce, replacement, handoff, restart, readiness,
  and rollback ordering into an injected production orchestration module;
- added a real-file fixture for success, pre-handoff restore,
  committed-handoff preservation, rollback restart failure, and pre-mutation
  failure;
- added `pnpm test:local-dashboard-publisher-orchestration` after lifecycle
  coverage in the fast Linux Dashboard CI job.

Validation:

- focused orchestration, lifecycle, smoke-policy, and syntax checks pass;
- temporary test artifacts are isolated and removed by exact root;
- no live runtime, dashboard listener, daemon, retained browser, or external
  service was touched.

Outcome: P117 is closed. Focused publisher, convergence, capability, parity,
dashboard, client-type, syntax, production-build, package, YAML, skill-parity,
plan-audit, cleanup, and diff gates pass. Docs-wide ESLint retains one existing
effect error and one existing warning in untouched files. Read-only evidence
shows the retained browser identity, ready live dashboard manifest, and
installed binary SHA unchanged. No GitHub write, prompt, click, typing,
navigation, browser lifecycle, credential, attach, detach, daemon handoff,
install, or live dashboard action occurred.

## Turn 163 | 2026-08-14

Scope: make standalone dashboard publisher restart and rollback transitions
repeatably testable without a live release build or runtime handoff.

Actions:

- extracted exact standalone process inspection, quiescence, and restart
  selection into a production lifecycle module;
- added a Linux real-process fixture with isolated PID metadata and exact
  executable, UID, environment, and signal evidence;
- covered normal restart, rollback restart, absent dashboard, stale PID, and
  wrong-command rejection paths;
- exposed `pnpm test:local-dashboard-publisher-lifecycle` as the focused gate.
- added both publisher policy and lifecycle fixtures to the fast Linux
  Dashboard CI job before its build.

Validation:

- the lifecycle fixture, existing dashboard smoke policy, and JavaScript syntax
  checks pass;
- fixture cleanup uses only recorded exact PIDs and leaves no temporary process;
- no live runtime, installed binary, dashboard listener, or browser was touched.

Outcome: P116 is closed. The exact real-process lifecycle fixture and fast CI
ordering pass; post-test inspection reports zero process or directory leaks.
Convergence, capability, parity, dashboard, docs, syntax, YAML, package JSON,
skill parity, plan audit, and diff gates pass. Read-only evidence shows the
retained browser and live dashboard manifest unchanged. No GitHub write,
browser interaction, credential action, live runtime action, or external
mutation occurred.

## Turn 162 | 2026-08-14

Scope: resolve a verified disposable browser for strict dashboard rendered-page
QA under WSL without weakening the general browser sandbox posture.

Actions:

- consumed installed `launchConfig` manifest, executable, artifact-smoke, and
  WSL profile-smoke evidence before launch;
- selected the first-class patched Chromium build and created an isolated
  Windows-mounted temporary profile;
- closed the exact disposable session and removed only its generated profile;
- rejected stale or mismatched configured capability evidence before launch.
- repaired the exact standalone-listener transition so a listener quiesced by
  the publisher is resumed after handoff or rollback without widening
  `--start-if-missing` authority.

Validation:

- focused resolver, cleanup-contract, unsafe-argument, syntax, and existing
  dashboard smoke-policy tests passed;
- one disposable live session rendered the dashboard app chrome and Workspaces
  pane at `http://127.0.0.1:4848/` and left no generated profile;
- retained PID 1046742, CDP endpoint, URL, and title remained unchanged.

Outcome: P115 is closed. Required release publication rendered the dashboard,
resumed the exact standalone listener without `--start-if-missing`, and left no
disposable profile. Installed, release, and reference binary SHA-256 is
`07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`.
The retained browser PID and CDP endpoint remained unchanged and healthy.
Doctor reports the live dashboard ready with zero stale runtimes; convergence
remains partial only for the expected standalone diagnostic runtime. No GitHub
write, prompt, typing, click, retained-page navigation, or credential action
occurred.

## Turn 161 | 2026-08-14

Scope: separate authoritative dashboard publication readiness from disposable
browser launch diagnostics without weakening rendered-page validation.

Actions:

- split required HTTP, marker, and runtime-manifest readiness from browser QA;
- classified known pre-render launch unavailability as advisory by default and
  retained explicit required-browser and post-launch fatal behavior;
- added exact standalone dashboard PID, user, command, and mode verification
  before a non-systemd listener can be retired.

Validation:

- focused policy, existing convergence, workspace inspector, syntax, parity,
  formatting, production clippy, dashboard, docs, skill, and diff gates passed;
- live publication served the required marker and exact runtime manifest, then
  returned advisory `browser_launch_unavailable` for WSL Chrome exit 21;
- installed/release SHA-256 is
  `8afeb3a270ce54c85cc25a14292e75e6299eee4c5dcc087c9aaf2342e992929e`;
- exact retained browser PID, CDP endpoint, URL, and title survived handoff.

Outcome: P114 is closed. Runtime inventory is converged with zero stale
runtimes and the live dashboard is ready. Doctor retains the pre-existing
workstation payload drift and expected standalone-dashboard diagnostic state.
No GitHub write, prompt, click, typing, navigation, credential operation, or
retained-page mutation occurred.

## Turn 160 | 2026-08-14

Scope: checkpoint exact retired confirmation membership, expose cleanup review
in the dashboard, and remove shared-state test races.

Actions:

- implemented a bounded active manifest with fixed-capacity immutable
  hash-linked segments and fail-closed exact-membership integrity checks;
- added authenticated dashboard retention preview and explicit exact-digest
  apply with requester, candidate, and ledger proof;
- serialized ambient HOME access in Chrome tests and isolated remote-view
  handoff registries in unique per-test directories.

Validation:

- focused authority, integrity, dashboard, generated-client, contract, parity,
  schema, formatting, build, and production-target strict-clippy gates passed;
- the normal parallel Rust suite passed 1,852 tests with 57 ignored;
- installed/release SHA-256 is
  `efc8a0dba40989a757fd5660e93221ae3e92cd41c967feb0b0bbcdf87fa2506f`;
- authenticated live HTTP and MCP previews verified schema v2 integrity without
  applying cleanup, and the retained target was read without page mutation.

Outcome: P113 is closed. Runtime inventory is converged with zero stale
runtimes. The optional disposable-Chrome marker smoke remains blocked by WSL
sandbox launch behavior, while direct authenticated dashboard checks pass.
Strict all-target lint retains 12 pre-existing test warnings, and doctor retains
the pre-existing workstation payload drift finding. No GitHub write, prompt,
typing, click, navigation, or external mutation occurred.

## Turn 159 | 2026-08-14

Scope: bind durable task-authority actors to authenticated transports and add
bounded terminal confirmation receipt cleanup.

Actions:

- required dashboard superuser authentication for HTTP issue, reconcile,
  revoke, and confirmation mutations and injected the authenticated username;
- derived MCP actors from the OS-owned stdio transport and rejected conflicting
  structured caller identity claims;
- added deterministic cleanup preview and exact review-digest apply, writing
  retired confirmation IDs before receipt removal;
- preserved pending and confirmed/dispatched indeterminate receipts and kept
  retired confirmation IDs single-use.

Validation:

- focused authority, HTTP, MCP, generated-client, and dashboard tests passed;
- one widened Rust suite passed 1,850 tests with 57 ignored;
- a final parallel run exposed three unrelated shared-temp-state flakes, and
  every affected test passed in isolation with one test thread;
- formatting, production-target strict clippy, release, dashboard and docs
  builds, contract parity, schema parse, install, and diff checks passed;
- installed authenticated HTTP and MCP cleanup previews passed without apply;
- the retained ChatGPT session identity was read back without page mutation.

Outcome: P112 is closed. Installed and release binary SHA-256 are
`b8416f50b572a9c4a7e9640e5cdae7b97eb226b06961b9a6630cc425bd9e9774`.
Strict all-target clippy remains blocked by 11 pre-existing warnings in
unrelated cumulative dirty test code. Install doctor remains nonzero only for
the pre-existing `workstation_payload_partial_or_drifted` condition. No GitHub
write, prompt submission, typing, click, navigation, or page mutation occurred.

## Turn 158 | 2026-08-14

Scope: preserve exact task-authority confirmation intent and single-use
decisions across daemon restart.

Actions:

- added private durable pending and terminal confirmation records only for
  task-authority issue, reconcile, and revoke controls;
- bound each record to requester, session, action, target, URL, request digest,
  creation time, and expiry, and required the exact requester as `decidedBy`;
- archived decisions before dispatch, projected crash-after-commit as
  indeterminate, and prevented decision replay, automatic confirmation, and
  automatic restaging;
- exposed redacted receipt status through HTTP, MCP, generated client types,
  and the selected-session Authorities workspace.

Validation:

- restart, crash boundary, actor/session/action/target/URL/digest/expiry drift,
  redaction, and terminal finalization regressions passed;
- 1,849 Rust tests passed with 57 ignored; formatting, strict Clippy, release,
  client, parity, dashboard, docs, contract, direct plan, and diff gates passed;
- debug and installed disposable public smokes proved pending restart
  preservation, zero restart execution, one exact decision, and replay
  rejection with no authenticated profile, prompt, or page mutation;
- installed and retained-lane readback preserved browser PID, target, URL, and
  title. The known unmanaged dashboard listener required an exact detached
  restart after publication, then manifest and bundle-marker checks passed.

Outcome: P111 closed. The next boundary is authenticated transport-principal
binding plus bounded terminal-receipt retention.

## Turn 157 | 2026-08-14

Scope: make durable task-authority state and indeterminate recovery operable
from the selected-session Service dashboard.

Actions:

- added an Authorities workspace for state, target, plan progress, outcomes,
  pending/completed reconciliation, and predecessor/replacement lineage;
- derived replacement previews only from unconsumed steps and required exactly
  one indeterminate receipt plus one exact live target before staging;
- added exact-session HTTP, MCP, contract, and generated-client confirmation
  decisions with an expected-action interlock and deny-on-close behavior;
- colocated generated bootstrap credentials with an explicitly relocated
  dashboard auth store so disposable tests do not touch real user credentials.

Validation:

- focused authority UI, client, HTTP, MCP, contract, and isolated-auth tests
  passed; the serial Rust suite passed 1,846 tests with 57 ignored;
- Rust formatting, strict Clippy, optimized release build, service API/MCP
  parity, generated client checks, docs/dashboard production builds,
  JavaScript syntax, validation selection, direct plan audit, installed-skill
  parity, and diff hygiene passed;
- debug and installed disposable public smokes rendered exact authority
  lineage after local isolated login, denied a staged revoke without changing
  authority, and cleaned up with no authenticated profile, prompt, or page
  mutation.

Result:

- P110 is closed. Installed/reference executable SHA-256 is
  `66fcca318e238ee6bf027ffd8ad8a38676c2e237acf59dad23465ee764b6f258`;
  the live dashboard bundle SHA-256 is
  `61465d64813a55daf57701fc214a73abcc6df57d386f4cf16d9fbab3a7622534`;
- retained ChatGPT browser PID `1046742`, CDP port `39377`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, URL, and title remained exact. No prompt
  was sent and no GitHub write occurred;
- install doctor has zero stale runtimes/listeners and retains only the
  pre-existing `workstation_payload_partial_or_drifted` issue.

## Turn 156 | 2026-08-14

Scope: replace manual recovery of one indeterminate task receipt with a durable
broker reconciliation transaction.

Actions:

- added stable reconciliation IDs, deterministic replacement authority IDs,
  complete predecessor lineage, pending/completed revocation evidence, and
  crash-resumable idempotency;
- required exactly one matching indeterminate receipt and fresh exact-target
  confirmation, while rejecting terminal, ambiguous, changed, tampered, or
  replayed evidence;
- exposed reconciliation through HTTP, MCP, generated clients, contracts,
  help, README, docs, and repository plus installed skill guidance;
- added an isolated public smoke that strands a bounded read after admission,
  reconciles it, rejects predecessor replay, and executes one fresh read.

Validation:

- focused and serial Rust tests passed, with 1,844 passed and 57 ignored in the
  widened suite; formatting, strict Clippy, release build, service API/MCP and
  generated-client gates, docs/dashboard builds, direct plan audit, installed
  smoke, retained-lane proof, and diff hygiene passed;
- the live smoke used no authenticated profile, prompt, or page mutation.

Result:

- P109 is closed. The predecessor is durably revoked before replacement
  publication, interrupted reconciliation resumes to one deterministic
  replacement, and a consumed command is never replayable;
- installed/reference executable SHA-256 is
  `e03bd1ca76409476fcfb9afa1c875118756c5b115e7a58b5414d019adbf6ec36`;
  the live dashboard bundle SHA-256 is
  `3336d77c1a00965371f65389eff9c6d41d9687c2935d213b2f73da5aad6fb4df`;
- retained ChatGPT browser PID `1046742`, CDP port `39377`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, URL, and title remained exact. No prompt
  was sent and no GitHub write occurred.

## Turn 155 | 2026-08-14

Scope: add durable terminal outcomes to ordered broker task receipts.

Actions:

- added one idempotent terminal finalizer across every post-admission command
  exit, before result broadcast or response publication;
- persisted completed/failed state, exact response SHA-256 and byte count,
  post-action target/URL evidence, and finalization time;
- projected admitted-only receipts as indeterminate without moving the cursor
  backward or permitting replay, and added fail-closed receipt validation;
- expanded status, docs, plan/roadmap evidence, repository/installed skill, and
  the disposable public live smoke.

Validation:

- focused terminal-outcome regressions and the serial Rust suite passed 1,841
  tests with 57 ignored; formatting, strict Clippy, optimized release build,
  service contracts, API/MCP parity, generated client checks, docs/dashboard
  builds, JavaScript syntax, direct plan audit, and diff hygiene passed;
- debug and installed disposable public proofs passed completed/failed outcome
  persistence, real handoff, replay rejection, response digests, revocation,
  and cleanup without authentication or mutation;
- the generic dashboard browser smoke reproduced its existing WSL Windows
  Chromium/Linux-profile exit 21. The browser-free manifest/marker smoke and
  the P108 Windows-profile live fixture passed.

Result:

- P108 is closed. Installed/reference executable SHA-256 is
  `58023942b0c1de84b2c38aef23b9dbc440796ab76576f0b67669fa585193e130`;
  live dashboard bundle SHA-256 is
  `881e5a5203d9971063caa278eba2d32103b4db4ce8cce7f0a95584e98599b634`;
- retained ChatGPT browser PID `1046742`, CDP port `39377`, target
  `B0EC77F279E5434E33FEA97AB1742B1A`, URL, and title remain exact; no prompt was
  sent;
- doctor still reports the pre-existing workstation payload, Guacamole route,
  route-display, group/helper/membership/sudoers gaps. No GitHub write or
  unrelated dirty-worktree reconciliation occurred.

## Turn 154 | 2026-08-14

Scope: bind broker-issued authority to an ordered, crash-safe execution plan.

Actions:

- added v2 issuance with broker-assigned step IDs, exact pre-action/requested
  URL bindings, fixed step evidence, and an envelope-bound plan hash;
- added a validated durable cursor and per-command admission receipts written
  before dispatch, with fail-closed missing/repeated/out-of-order/action/URL/
  evidence/hash/ledger checks;
- propagated `taskAuthority`, `taskStepId`, and `taskEvidenceBytes` through
  HTTP, MCP `service_request`, generated clients, schemas, README, user docs,
  and the repository plus installed skill;
- documented immutable replanning as revoke plus newly confirmed issue;
- expanded the public smoke to `title -> url -> title` and a real daemon
  handoff after step 1, proving restart replay rejection without page mutation.

Validation:

- focused authority, HTTP, MCP, generated client, contract, and API/MCP parity
  tests passed;
- the serial Rust suite passed 1,837 tests with 57 ignored; formatting, strict
  production Clippy, release build, service-contract smoke, docs/dashboard
  production builds, JavaScript syntax, direct plan audit, and diff hygiene
  passed;
- debug and installed P107 disposable live proofs passed with exact cleanup,
  no authentication, page mutation, prompt, or retained-profile use;
- generic `test:service-request-live` remains blocked before page work by its
  pre-existing disposable Chrome exit 21 despite configured `--no-sandbox`.

Result:

- P107 is closed. Installed/reference/live-dashboard executable SHA-256 is
  `e760f6ad07012d9fc083a77b790f5f2aa2ee4d320e8b38f9c47815cf74311220`;
  dashboard bundle SHA-256 is
  `b4322cfab3888e1ce9f32aecc37229674641b644f7217d38398ed247df84eb82`;
- the exact retained ChatGPT browser PID `1046742`, profile, CDP endpoint,
  target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title were preserved; no prompt was sent;
- doctor has zero stale runtime listeners and a ready live dashboard, while
  remaining partial state is limited to the existing workstation-payload and
  interactive-sudo remote-view installation boundaries;
- no GitHub write or unrelated dirty-worktree reconciliation occurred.

## Turn 153 | 2026-08-14

Scope: replace caller-constructed authority with a broker-issued, revocable,
inspectable exact-target capability.

Actions:

- added confirmation-gated HTTP and MCP issue/revoke operations and no-launch
  collection/exact status operations;
- derived action/origin scope and action/evidence budgets from explicit bounded
  plan steps, persisted issuer/approval/plan/hash/revocation evidence privately
  and atomically, and required that record in fail-closed mode;
- added generated client helpers, schema/contract metadata, caller-label and
  profile-lease interlocks, allowed-action admission, docs, and installed skill;
- added a disposable public retained-target smoke covering issue, status,
  budget debit, exhaustion, and revoke without page mutation or prompt work.

Validation:

- focused authority, HTTP, MCP, service contract, generated client, and
  profile-lease regressions passed;
- the serial Rust suite passed 1,834 tests with 57 ignored; formatting, strict
  production Clippy, client types/examples, API/MCP parity, docs/dashboard
  production builds, release build, validation selection, targeted MDX lint,
  JavaScript syntax, and diff hygiene passed;
- all-target Clippy additionally reports ten unrelated pre-existing test-style
  warnings in the preserved dirty worktree; no unrelated test churn was made;
- debug and installed disposable public proofs passed with exact cleanup, no
  authentication, prompt, composer work, or page mutation.

Result:

- P106 is closed. Installed, reference, and live-dashboard executable SHA-256
  is `86380304f45f9d8c6affc3c9caaffe85bbae26aaa18dbf260eae0d8b05cf7868`;
- handoff preserved browser PID `1046742`, CDP endpoint `127.0.0.1:39377`,
  target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and
  `Architecture Review Boundaries` title;
- unavailable user systemd left the old 4848 process serving the prior binary.
  Replacing only that exact deleted-executable listener produced a ready live
  manifest with the installed hash;
- install doctor has zero stale runtimes and a ready dashboard, but remains
  partial for older workstation-payload provenance and the separate privileged
  remote-view helper/group/sudoers installation requiring operator action;
- no ChatGPT prompt, authenticated-site mutation, GitHub write, or unrelated
  dirty-worktree reconciliation occurred.

## Turn 152 | 2026-08-14

Scope: add a durable task-level authority boundary above individual browser
actions and P104 confirmation.

Actions:

- added a strict immutable `taskAuthority` contract for caller labels, allowed
  origins, retained target and initial URL, action/evidence budgets,
  consequence ceiling, and RFC 3339 expiry;
- added an atomic per-session ledger keyed by authority identity and full
  envelope hash so restart cannot reset budgets or accept envelope drift;
- enforced authority before dispatch at the common daemon boundary and routed
  above-ceiling actions through fresh exact-target confirmation;
- propagated authority and evidence reservations through service HTTP and the
  generated client contract, with required-mode and ledger-directory settings;
- documented the contract and synchronized the repository and installed skill.

Validation:

- four authority regressions, HTTP propagation, P104 confirmation, client
  contract/type/example, and API/MCP parity tests passed;
- the serial Rust suite passed 1,828 tests with 57 ignored; formatting, strict
  Clippy, docs production build/TypeScript, dashboard build, release build,
  validation selection, and diff hygiene passed;
- targeted MDX lint returned no errors and three no-matching-configuration
  warnings; this checkout has no `plans:audit` package script;
- debug and installed disposable public-page proofs passed with no login,
  mutation, prompt, or cleanup residue.

Result:

- P105 is closed. Release, installed, reference, and live-dashboard executable
  SHA-256 is
  `9fe62980912e20c0e5c1db2c2b5538edcadba9adf106ab9a0d1ef3da6133c9df`;
- executable handoff preserved retained browser PID `1046742`, CDP endpoint
  `127.0.0.1:39377`, target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical
  conversation URL, and `Architecture Review Boundaries` title;
- the unavailable user systemd bus left the prior dashboard listener active;
  replacing only that exact listener reconciled the live runtime manifest;
- install doctor reports zero stale runtimes and a ready live dashboard, but
  remains partial for separate workstation-payload provenance and privileged
  remote-view installation requiring operator action and interactive sudo;
- no ChatGPT prompt, authenticated-site mutation, or GitHub write occurred.

## Turn 151 | 2026-08-14

Scope: replace the confirmation stub with a target-bound human approval
boundary for retained-target agentic work.

Actions:

- classified browser actions into stable read-only, navigation, mutation,
  credential, script, lifecycle, and control-plane consequence classes;
- bound every pending approval to an exact confirmation ID, active target ID,
  active URL, and 60-second lifetime;
- rejected missing, wrong, expired, overwritten, and target-mismatched
  approvals while preserving single execution for a matching approval;
- added a disposable public-page smoke that proves read-only collection works,
  denial mutates nothing, and a later target change invalidates approval;
- documented category configuration and the fail-closed contract across CLI,
  README, user docs, and the installed skill.

Validation:

- five focused confirmation regressions and thirteen policy tests passed;
- the serial Rust suite passed 1,824 tests with 57 ignored; formatting, strict
  Clippy, docs TypeScript/build, JavaScript syntax, validation selection, and
  diff hygiene passed;
- debug and installed public-page live proofs returned success with no
  authenticated profile, mutation, prompt, or cleanup residue;
- optimized publish handed the retained session to the replacement daemon with
  browser PID `1046742`, the same CDP endpoint, and one reattached target;
- the publisher's first final check found the old standalone dashboard listener
  after the unavailable user systemd bus prevented restart. Terminating only
  that exact deleted-executable listener and starting the installed dashboard
  produced matching manifest and installed hashes.

Result:

- P104 is closed. Release, installed, and reference executable SHA-256 is
  `0f57a4b060d68473d13b07155cd6fc502393124244d18def9340c5e7f083b468`;
- install doctor confirms the current/workspace hash and ready live dashboard
  with zero stale runtimes, while the separate workstation payload and
  remote-view privileged helper remain partial pending interactive sudo;
- the retained target still resolves to `Architecture Review Boundaries` at
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`;
- two pre-build disposable public smoke attempts exposed a stale debug binary
  and followed the Example Domain link. They used no authenticated state and
  were cleaned exactly; rebuilt debug and installed proofs stopped mutation;
- no ChatGPT prompt, authenticated-site action, or GitHub write occurred.

## Turn 150 | 2026-08-14

Scope: close the WSL Windows launch gap in the isolated CDP tab-streaming live
fixture.

Actions:

- preserved explicit browser executable selection ahead of the fixture's
  `/usr/bin/google-chrome` fallback;
- placed Windows Chromium fixture profiles under Windows Temp and passed the
  exact profile as top-level service-request authority;
- added a no-browser precedence and profile-placement regression;
- kept all three live runs limited to disposable data-URL tabs and exact
  cleanup.

Validation:

- `pnpm test:service-cdp-tab-streaming-fixture` passed;
- the live streaming smoke passed through the debug executable and through the
  exact caller shape with only the Windows executable environment configured;
- syntax, package metadata, validation selection, and diff hygiene passed;
- no fixture profile or session remained after any run.

Result:

- P102 is closed and the previously failing WSL Windows streaming fixture now
  completes end to end;
- retained PID 184301 had already exited at `2026-08-14T20:48:18Z`, before the
  first P102 file change at `2026-08-14T20:59:17Z`; P102 did not relaunch that
  profile or send a prompt;
- no GitHub write occurred.

## Turn 149 | 2026-08-14

Scope: close P99-F1 with a bounded evidence reader that remains usable when a
monolithic page renderer stalls.

Actions:

- added `get page` with HTTP(S), domain-policy, byte, timeout, and explicit
  credential controls;
- implemented the fetch through one temporary background target using
  `Network.loadNetworkResource` and bounded `IO.read`, with verified stream and
  exact-target cleanup and no `Runtime.*` command;
- preserved active-target identity and returned source, status, MIME, byte,
  truncation, credential, and target metadata;
- routed the frozen WHATWG mission deterministically through the reader and
  retained the same reader as the single fallback after renderer evidence
  failure on other missions;
- documented, installed, and synchronized the repo and installed skill.

Validation:

- focused parser and mock-CDP tests passed, including success, truncation, read
  failure, timeout, cleanup, no-Runtime, credential default, and original-target
  checks;
- the isolated WHATWG run and frozen ten-mission run passed; the full score was
  10/10 in 48.468 seconds with zero retries and exact cleanup;
- the deterministic serial Rust suite passed 1,816 tests with 57 ignored;
  strict Clippy, formatting, docs TypeScript/build, route-confusion gates,
  JavaScript syntax, validation selection, and diff hygiene passed;
- targeted MDX lint had no errors and one no-matching-config warning. No package
  plan-audit command exists in this checkout.
- the generic CDP-tab-streaming live smoke remains blocked before navigation by
  its isolated WSL Chrome launch and did not exercise this reader.

Result:

- P101 and P99-F1 are closed. Installed/debug/reference executable SHA-256 is
  `739201f613f5b5cf987347618908528cd05a57ca8fe2c064b1699d2728f7552b`;
- exact executable handoff preserved retained browser PID 184301, its CDP
  endpoint, profile, target inventory, and `Architecture Review Boundaries`
  conversation identity;
- the installed public read returned HTTP 200 and 559 bytes with credentials
  disabled, then exact before/after inventory remained unchanged;
- no prompt, composer action, login, form submission, or GitHub write occurred.

## Turn 148 | 2026-08-14

Scope: close the delayed browser-health cascade exposed by the P99 public-web
research benchmark.

Actions:

- added a no-replay post-timeout circuit that separates browser-level CDP loss
  from active-renderer damage;
- prepared a ready blank target before closing exactly one affected ordinary
  owned target while preserving unrelated tabs and the browser process;
- preserved service-tab handles and externally attached browser lifecycle
  authority;
- skipped polite CDP close only after browser-level disconnection, then
  terminated the exact locally owned process and performed one blank relaunch;
- documented, built, installed, and synchronized the behavior into the repo and
  installed agent-browser skill.

Validation:

- focused tests, 1,811 Rust tests, formatting, strict Clippy, docs build and
  TypeScript, route-confusion gates, release build, validation selection, diff
  checks, and installed dashboard runtime smoke passed;
- the full frozen benchmark improved from 8/10 with delayed CDP refusal to 9/10
  with only the bounded WHATWG extraction failure;
- Python, Rust, Git, and Node passed after WHATWG in the same session, and the
  installed critical pair repeated that result;
- the generic CDP-tab-streaming smoke remains blocked before navigation by its
  disposable Linux Chrome launch configuration under WSL and did not exercise
  this change.

Result:

- P99-F2 is closed and installed binary SHA-256 is
  `6c6bcd338465639a3937b6a9c4c6f4a787b7e2847396cde451965603174968a8`;
- handoff preserved retained browser PID 184301 and one current ChatGPT target;
  read-only checks found `Architecture Review Boundaries` at its current URL and
  no prompt was sent;
- the next priority is P99-F1: a bounded precise-section or non-Runtime reader
  for monolithic standards pages.

## Turn 147 | 2026-08-14

Scope: establish a public, no-login Browser Research Concierge benchmark before
allowing authenticated or consequential agentic-browser missions.

Actions:

- added a ten-mission installed-runtime harness covering official IANA, RFC
  Editor, W3C, WHATWG, Python, Rust, Git, and Node.js sources;
- used one disposable Windows Chromium profile and named agent-browser session
  with canonical URL, page identity, evidence, duration, zero-retry, and exact
  cleanup checks;
- replaced title-only identity with title, heading, or bounded body evidence so
  valid RFC pages with empty HTML titles remain verifiable;
- added focused mission selection, source-drift diagnostics, post-navigation
  outcome verification, and one absolute mission deadline.

Validation:

- syntax, package JSON, and patch checks passed;
- the initial real run scored 6/10 and exposed one stale evidence phrase, two
  titleless RFC pages, and the monolithic WHATWG renderer stall;
- the strengthened pre-budget run scored 9/10;
- the final budgeted full run scored 8/10 with eight canonical evidence passes,
  one bounded WHATWG service-job timeout, and one delayed Node.js CDP refusal;
- the Node.js mission passed alone in 1.3 seconds, attributing its full-suite
  failure to delayed session damage rather than source availability;
- every run reported cleanup complete and no authenticated profile, prompt,
  form submission, download, or other mutation was used.

Result:

- the evaluation harness is complete and intentionally exits nonzero when any
  mission fails;
- agent-browser is reliable for ordinary public documentation but not yet safe
  for long-horizon agency after a renderer-stall timeout;
- the next repair is a post-timeout health circuit breaker plus a bounded
  non-Runtime or precise-section evidence reader.

## Turn 146 | 2026-08-14

Scope: retain WSL NAT and the home-network Bastion jump route while making
Windows Chromium DevTools privately reachable from agent-browser without SSH,
mirrored networking, a firewall rule, or a persistent terminal.

Actions:

- added an internal WSL-loopback relay that forwards each connection to
  Windows loopback through a bounded PowerShell/.NET binary pump;
- tied relay admission, health, handoff, and cleanup to one exact Windows
  browser PID and translated profile identity, failing closed on zero or
  multiple main-process matches;
- generalized the executable-handoff smoke for Windows Chromium and documented
  the NAT relay in CLI output, README, configuration docs, and the installed
  agent-browser skill;
- removed the active `srv1635328` SSH host block, its four authorized keys, and
  its host-key records while preserving the Tailscale Bastion and desktop jump
  routes.

Validation:

- disposable headed Windows Chromium launch/close passed under WSL NAT;
- executable handoff preserved browser/relay PID 255174 and CDP endpoint
  `127.0.0.1:37943`, then exact close removed both;
- 1,866 Rust tests, formatting, strict Clippy, dashboard build/typecheck, docs
  build, release build, focused path and handoff smokes, patch checks, and
  installed dashboard runtime smoke passed;
- retained LitScout target `F203CCD6CC8B212B3B55D16368150AEB` remained ready
  at its exact conversation URL and no prompt was sent.

Result:

- installed executable, reference, and live dashboard manifest SHA-256 are
  `0d1b1a771d57e2a084d10bcf5e74a47194dcf206af3b7f2c6dc8c428475f66bc`;
- the no-prompt broker attach request stopped with HTTP 502 before admission;
  zero matching `cdp_attach` jobs were committed, so there was no detach
  obligation and the request was not retried;
- install doctor is ready for launch configuration and live dashboard parity;
  its remaining failure is the older source-free workstation payload hash plus
  optional privileged RDP-helper prerequisites, outside this relay change.

## Turn 145 | 2026-08-09

Scope: repair the upstream renderer and command-delivery boundaries exposed by
three exhausted Last30Days Facebook candidates, without consuming a fourth
provider attempt or changing the retained Facebook browser.

Actions:

- used raw page and flattened browser CDP sessions to distinguish a
  Facebook-target Runtime stall from browser inventory and transport health;
- added Chromium renderer deadlines, browser-level navigation metadata,
  cached evaluation metadata, and response-before-health worker ordering;
- removed the pre-action health probe from the caller-bounded path;
- added a Linux same-inode daemon identity fast path while retaining SHA-256 as
  the rebuild, upgrade, and unavailable-procfs fallback;
- published the debug candidate through browser-preserving executable handoff,
  refreshed the source-free workstation payload, and closed the exact
  disposable investigation sessions plus stale session metadata advertised by
  doctor remedies.

Validation:

- focused red and green renderer-deadline, target-metadata, worker-response,
  and executable-identity regressions;
- 34 browser tests, 27 control-plane tests, 35 connection tests, 260 action
  tests, and three CDP stream tests;
- service CDP tab streaming live smoke and route-confusion no-launch gates;
- Rust formatting, required production Clippy, dashboard production build,
  selected validation, patch checks, installed binary/reference parity,
  install doctor, and remote-view doctor;
- the canonical partition passed 1,220 parallel-safe tests and the touched
  serial partitions before the unrelated untouched Chrome test
  `test_headed_display_fallback_not_used_when_display_set` rejected the current
  production behavior of returning `DISPLAY=:9`;
- broader all-target Clippy also surfaced twelve pre-existing test-only lint
  rejections. Required production Clippy passed.

Result:

- installed executable and reference SHA-256 are
  `17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b`;
- installed baseline evaluation returned `2` in 455 ms, the infinite loop
  returned a typed CDP failure in 4.546 seconds under a 6-second outer guard,
  and immediate recovery returned `2` in 1.482 seconds;
- install doctor is clean, runtime convergence is `converged`, remote-view and
  remote-control status are ready, and the dashboard payload is current;
- retained Facebook PID 63205 remained ready and was not restarted or closed;
  no Last30Days provider attempt was submitted.

Graphiti Write Status:

- compact source-backed closeout job
  `64a8beb5-806d-457a-84c6-2c7e4c51449d` was queued in
  `agent_browser_main`, then timed out after 120 seconds with no episode UUID;
  the durable memory write is therefore unconfirmed.

## Turn 144 | 2026-08-08

Scope: close the Last30Days sequential-social handoff with downstream installed
acceptance, without claiming an agent-browser source repair.

Actions:

- reconciled note 0098 with the retained agent-browser job IDs from installed
  Last30Days 0.3.27 and the accepted 0.3.28 proof;
- confirmed both prior tab-switch jobs succeeded after their 8-second callers
  exited, while the fresh eval independently reached its 15-second worker
  timeout;
- recorded the exact client contract proven live: 3-second retained worker jobs
  with 15-second callers, and 30/45 seconds only for a fresh auth target;
- bound the downstream result to pushed Last30Days commit
  `24474f62e5e11f1c51d5ab5adf0f0933764dce91`.

Validation:

- manual tick `tick-f273eb12d642b31d49a7f12959b93b87` accepted Facebook;
  attempt `provider-attempt-5e5205b623e52dfd122dbbf2e4e668af` observed 19,
  accepted two, rejected 17, and every browser operation succeeded;
- PID 63205 remains ready on canonical `session:last30days-facebook` with 17
  tabs; no duplicate browser launch or retained-tab closure occurred;
- no agent-browser source, generated client, dashboard, docs command surface,
  or installed executable changed in this closeout.

Result:

- the cross-repo blocker is closed as a downstream deadline-contract repair;
  queue timing observability may remain future nonblocking backlog.

Graphiti Write Status:

- pending the shared closeout memory after both repository commits are pushed.

## Turn 143 | 2026-08-08

Scope: investigate the Last30Days sequential X-to-Facebook evaluation timeout
without mutating retained PID 96078.

Actions:

- confirmed Plan 0097's timeout cleanup and queue-release repair is already in
  `origin/main`;
- separated the caller deadline from the dispatched worker deadline and found
  that Last30Days allowed only 5 seconds of grace despite observing successful
  commands taking 8.2 to 8.4 seconds in the same attempt;
- found later ownership drift: PID 96078 remains ready as
  `session:plan0058`, while `last30days-facebook` is an alias to that browser;
- wrote the privacy-safe cross-repo investigation note and downstream alias
  routing acceptance contract.

Validation:

- source inspection confirms `jobTimeoutMs` starts after worker dequeue;
- installed service state confirms PID 96078 is ready with 17 retained tabs;
- a diagnostic bare `tab list` accidentally auto-launched PID 47946 on the
  drifted daemon; exact attribution was established and only PID 47946 was
  closed. PID 96078 remained ready and unchanged.

Result:

- no agent-browser source repair is justified by the retained evidence yet;
  the immediate proven repair belongs in Last30Days deadline layering and
  exact alias-owner routing;
- note 0098 is the durable agent-browser investigation handoff.

Graphiti Write Status:

- no write; note 0098, current source, installed jobs, and service readbacks are
  sufficient durable evidence.

## Turn 142 | 2026-08-08

Scope: verify installed timeout/queue behavior during the explicit Last30Days
manual governed tick before pushing the completed remote-browser repair.

Actions:

- observed the installed Last30Days Facebook lane skip two frozen retained tabs,
  switch to a responsive tab, and complete bounded auth evaluation;
- observed query navigation job `r198316` fail at the page-operation timeout;
- verified the serialized queue released and later tab, eval, navigation, and
  service reconciliation jobs completed successfully;
- preserved retained Chrome PID 96078 and all eight tabs. No browser or tab was
  opened or closed.

Validation:

- manual tick `tick-848f61b8a22d7e603c7e473c16ba5fdf` terminalized
  `complete_degraded` with seven items, zero cost/model use, zero incidents,
  and zero notifications;
- failed job `r198316` reports the ordinary page-operation timeout rather than
  a queue stall; subsequent jobs `r998619`, `r498581`, `r461931`, `r922601`,
  `r422781`, `r500611`, and `r617918` all succeeded;
- current executable SHA-256 remains
  `e899753a27005a79fe820f9128420eb0ea80ed8ea59a8719c64d9bc14c278d5f`.

Result:

- Plan 0097 remains closed: cancellation and queue release are proven in the
  ordinary governed path. Last30Days now owns the separate post-auth navigation
  recovery in Plan 0027.
- Changes are ready for scoped commit and push; unrelated `--full-page` and
  declaration-campaign evaluation artifacts remain excluded.

Graphiti Write Status:

- no new write; agent-browser jobs plus both repo runbooks are the durable
  source-backed evidence.

## Turn 141 | 2026-08-08

Scope: complete durable authenticated remote-view handoffs, then repair ordinary
CLI timeout layering, retained-target handoff recovery, and interrupted-command
browser ownership for the Last30Days Facebook lane.

Actions:

- Added opaque authenticated `/remote-view/<handoff-id>` URLs, sidecar-backed
  durable handoff persistence, authenticated dashboard resolution, bounded
  route reacquisition, and explicit-close fail-closed behavior.
- Added global `--job-timeout-ms` parsing and top-level command JSON carriage
  while preserving action-specific timeout values and invalid-input rejection.
- Made runtime handoff persist an optional preferred target and probe retained
  targets under bounded Page, Runtime, and Network domain initialization,
  retaining schema-v1 backward compatibility.
- Changed control-plane cancellation cleanup so only an observed process exit
  discards browser ownership; a timeout or cancelled future preserves a live or
  reconnectable BrowserManager.
- Updated CLI help, README, docs site, source skill, and installed shared skill.
- Published and converged the release-mode executable without replacing the
  retained Last30Days browser.

Validation:

- durable URL, dashboard auth, handoff sidecar, legacy-writer, resolver,
  explicit-close, generated-client, and isolated live reacquisition gates
- focused red/green parser, command JSON, timeout queue-release, legacy
  handoff, target ordering, same-profile guard, and interruption cleanup tests
- canonical Rust suite: 1,789 passed, 57 ignored, zero failed
- Rust formatting, strict Clippy, dashboard/docs builds, patch checks,
  installed runtime and skill convergence, install doctor, and remote-view
  doctor
- live one-second never-resolving eval followed by a 466 ms tab-list command;
  retained PID 96078, seven tabs, and active index 3 were unchanged during the
  agent-browser proof

Result:

- Plans 0096/P96 and 0097/P97 are closed. Installed executable SHA-256 is
  `e899753a27005a79fe820f9128420eb0ea80ed8ea59a8719c64d9bc14c278d5f`;
  runtime convergence and `remoteControl.status=ready` are current.
- The first timeout proof exposed the prior unconditional cleanup branch and
  launched default-profile Chrome PID 97130. Exact daemon ownership and start
  time were proved; only that repair-created browser was closed. Retained PID
  96078 was never closed or restarted, and no default-profile Chrome remains.
- A later Last30Days proof opened one additional Facebook target, so current
  browser readback is eight tabs at active index 5; the final Last30Days repair
  preserves that complete set.
- Changes remain local and uncommitted. Unrelated untracked files, including
  `--full-page`, were untouched.

Graphiti Write Status:

- one compact closeout write was attempted as job
  `7e1b6b06-e449-4668-b469-99118eb1f14b`; it timed out after its explicit
  120-second bound before creating an episode UUID. No retry was submitted;
  this runbook, Plan 0097/C04, and installed readbacks remain authoritative.

## Turn 140 | 2026-08-07

Scope: repair the Last30days single-route remote-control readiness false block
without hiding duplicate-profile pressure or weakening target-profile launch
guards.

Actions:

- Added a fail-closed classifier that distinguishes raw embedded install-doctor
  success from effective single-route remote-control install readiness.
- Accepted `service_duplicate_profile_pressure` only when it is the complete
  structured issue set and readiness-impacting resource candidates equal zero.
- Applied the shared classification to nested remote-control status, top-level
  next-action recommendation, and remote-view issue projection.
- Added `installDoctorReady`, effective `installReady`, and
  `nonBlockingInstallIssueCodes` to the JSON contract while preserving the
  complete embedded install report.
- Updated CLI help, README, docs-site guidance, repo skill guidance, and the
  installed shared skill.
- Published and converged the installed `0.28.0` checkpoint without opening or
  closing a browser.

Validation:

- observed-red then green cross-seam regression matching the original two
  warning and zero readiness-candidate payload
- fail-closed mixed, malformed, timeout, and positive-candidate cases
- all 44 remote-view doctor tests and the same-profile retained-browser guard
- canonical partitioned Rust suite, Rust formatting, strict Clippy, docs
  production build, patch hygiene, installed help readback, and skill sync
- installed workstation payload, dashboard manifest, runtime convergence,
  remote-control readiness, daemon SHA-256, browser PID, URL, and service-state
  readbacks

Result:

- Plan 0095 is closed. Current remote control reports all single-route
  prerequisites ready and recommends `run_remote_view_open_live_gate`.
- The installed executable SHA-256 is
  `8582bf0900b4d974994846c4ff3985746dcbbf5ee2136699f68e56ea5e73726b`.
- Publish reported a `p0065` resume error after the replacement daemon had
  already attached. Direct readback proved the installed daemon still owns
  original browser PID `19675` and its existing URL. The LitScout lane also
  retained its existing ChatGPT target, and all four pre-publish ready browser
  records remain ready.
- Current resource warnings are zero because inactive daemon listeners were
  retired during publish. The exact original duplicate-pressure branch remains
  covered by its regression fixture.
- No browser was opened or closed, no profile lease was released, and no
  resource cleanup was applied. At this checkpoint the untracked `--full-page`,
  declaration-campaign evaluation artifacts, and Last30Days handoff note
  remained uncommitted and untouched; the handoff note is included only in the
  later scoped P95-P97 integration recorded by Turns 141-142.

## Turn 139 | 2026-08-06

Scope: open the exact guided launcher from actionable profile rows and close
any live service-owned browser from the dashboard without extending lifecycle
ownership to detected external browsers.

Actions:

- Routed profile-row Open browser through the existing browser/profile launcher,
  preserving exact profile selection and automatic no-launch access planning.
- Made workspace browser Close contract-aware and wired its confirmation to
  `service_browser_close` for browser records that do not carry daemon ports.
- Enabled Close in the Service browser table for every live service-owned
  browser instead of only the currently selected daemon-session browser.
- Added browser-specific confirmation copy that explains polite shutdown and
  retained lifecycle history.
- Preserved disabled lifecycle controls for detected non-owned browsers and
  explicit contract-unavailable reasons for service-owned rows.
- Updated CLI help, README, dashboard and service-mode docs, source skill, and
  installed shared skill, then published the embedded dashboard.

Validation:

- focused red then green workspace-node, workspace-navigator, and Service
  browser-table contracts
- dashboard view-stream, rendered browser-row action, selected-context,
  selected-chat, selected-console, launcher, and inspector tests
- route-confusion gates, 34 focused Rust output tests, Rust formatting, strict
  Clippy, dashboard and docs production builds, patch hygiene, and installed
  runtime smoke
- live dashboard inspection of all 485 stored profiles across 1,952 launcher
  combinations, exact disposable-profile preflight, disposable service-owned
  browser Close confirmation, session removal, and local/public HTTP 200

Result:

- Plan 0094 is closed. Actionable profile rows can wake the exact guided
  launcher, and live service-owned browser tiles can be politely closed from
  both lifecycle surfaces.
- Profiles without compatible browser evidence remain visible but blocked by
  the existing safety gate. Adding reviewed evidence from the UX is the next
  recommended slice.
- The installed dashboard SHA-256 is
  `b2d74b07f2d649f34858c67e3830fc41427818cbbbb4a6fb4b75b0c56fabbb16`;
  the executable SHA-256 is
  `32af83cf90e0940183f83e4e7f02ecd4f1b3b6ffaada96d6863f866c6485e3be`.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 138 | 2026-08-05

Scope: repair stored-profile launcher identity handling and open two bounded
stored-profile browser workspaces without disturbing retained browser owners.

Actions:

- Repaired nested browser-capability preflight parsing so explicit
  browser-build, runtime-profile, custom-profile, and headed flags survive
  global flag cleaning.
- Added exact compatibility rows for the Last30days stealth Chromium profile
  and the AuraCall stock Chrome profile, then proved both installed no-launch
  preflights selected passed validation evidence.
- Changed dashboard launcher session arguments to pass both the service
  runtime-profile identity and exact custom profile path when access-plan
  provides both.
- Launched `stored-last30days-social` on its exact profile as PID `90765`, CDP
  port `37077`, and display `:90`; direct CDP and screenshot readbacks showed
  the authenticated Facebook feed.
- Launched `stored-auracall-chatgpt` on its exact registered AuraCall path as
  PID `95241`, CDP port `38441`, and display `:91`; direct CDP and screenshot
  readbacks showed ChatGPT loaded but logged out.
- Released both Guacamole route checkouts created by rejected shared-display
  attempts. Route A and Route B returned to available. Preserved the existing
  `litscout-e3-auracall` CDP endpoint.
- Published the final embedded dashboard and executable. Both new browsers
  remain running.

Validation:

- focused red then green Rust preflight parser regression
- focused red then green dashboard launcher argument regression
- Rust formatting, strict Clippy, focused Rust tests, dashboard and docs
  production builds, patch hygiene, and installed preflight readbacks
- direct process command-line, runtime-state, CDP target, HTTP screenshot,
  service profile allocation, stream, dashboard local/public HTTP 200, route
  release, and retained-browser readbacks

Result:

- Plan 0093 is closed. Stored-profile launches now retain both service identity
  and exact browser-state paths, and the two requested browsers are available
  as controllable CDP stream workspaces.
- The installed executable SHA-256 is
  `2c07c043a2af5a7063a161159f856c1e9c3974e31ceaf95300f2a46383fae32b`.
- AuraCall's retained fresh ChatGPT authentication evidence is stale relative
  to the current logged-out page. Login repair remains a separate authorization
  boundary.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 137 | 2026-08-05

Scope: make detected non-owned CDP browser tiles useful for capture, responsive
watching, and explicit time-bounded interaction without taking lifecycle
ownership.

Actions:

- Added functional Screenshot and Watch live actions for foreign CDP page
  targets. Watch refreshes a bounded PNG frame every 750 milliseconds.
- Added authenticated Borrow status, grant, fixed-input, and Release endpoints.
  Grants bind one superuser, CDP port, and live page target for five minutes by
  default and no more than fifteen minutes.
- Restricted borrowed input to server-built pointer, keyboard, and wheel CDP
  commands. Arbitrary CDP, navigation, evaluation, Close, Kill, profile
  release, and adoption remain unavailable.
- Added viewport pointer, drag, wheel, and keyboard forwarding plus visible
  grant expiry and Release controls. Foreign rows no longer expose Close or
  Kill affordances.
- Updated help, README, dashboard docs, the agent-browser skill, runtime feature
  markers, tests, Plan 0041, and the roadmap.
- Published the embedded dashboard and executable while preserving retained
  browsers and daemon handoffs.

Validation:

- dashboard workspace navigator, view-stream, workspace-node, inspector,
  selected-context, chat-packet, console, service parity, service-client
  contract, and JavaScript type suites
- dashboard and docs production builds
- focused foreign-CDP Rust tests, formatting, strict Clippy, and the canonical
  partitioned Rust suite
- installed runtime marker smoke for `Borrow control`, `Capture PNG`, and
  `workspace.foreignCdpBorrow`
- disposable foreign Chrome proof of capture, Borrow, mouse and keyboard input,
  wheel input, Release, post-release HTTP 403, continued process health, and
  unchanged `foreign_cdp` ownership

Result:

- The installed dashboard at `http://127.0.0.1:4848/` can capture, watch, and
  temporarily interact with reachable non-owned browser targets without
  claiming their lifecycle.
- The installed dashboard SHA-256 is
  `d215c9b5fe7fc731abff307db240e383c31811060254d2a58514e3b4059d8cb4`;
  the executable SHA-256 is
  `f5d0c1ef6220415671f6e756e56bf9f18c6c6b5ade884ffbfacb0a4b264510fd`.
- Install doctor reports runtime convergence and the dashboard ready, but
  remains nonzero for the separate
  `workstation_payload_partial_or_drifted` source-free workstation payload
  issue. This slice did not broaden into workstation repair.
- Plan 0041 remains active for a native CDP screencast and durable Service or
  Activity audit history. The current Watch feed is a responsive screenshot
  stream.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 136 | 2026-08-03

Scope: make recurring workstation privilege actions passwordless through the
narrow installed helper, enable Guacamole text input by default, and restore
the named Route A browser after live drift.

Actions:

- Replaced byte-only helper readiness with root-owned capability checks while
  retaining exact helper provenance as advisory evidence.
- Removed direct sudo fallbacks from route-user and display-access maintenance.
- Made the managed Chrome AppArmor policy conditional on a kernel where
  AppArmor and restricted unprivileged user namespaces are both active.
- Added and packaged a Guacamole JavaScript extension that performs a
  versioned one-time browser-origin migration to text input while preserving
  later operator overrides.
- Published a prechange PostgreSQL backup, mounted the extension, and recreated
  only the Guacamole web container. PostgreSQL and guacd were not recreated.
- Found `wsl-chrome-3` with a dead retained DevTools port, then performed the
  requested Route A recovery on its canonical `:10` allocation and reused its
  restored ChatGPT target.

Validation:

- privilege clean and host-provision regressions, including compatible helper
  drift, compatible AppArmor annotation drift, and AppArmor-disabled WSL
- Guacamole asset, workstation install, fresh VM, PostgreSQL durability, and
  route-user fixture suites
- Rust workstation-install tests, formatting, strict Clippy, docs build, and
  selected validation
- live no-prompt privilege apply returned no privileged changes needed
- Guacamole logged the defaults extension loaded; served application code
  contains the migration; a fresh origin read back `inputMethod: text` and
  migration version `1`
- PostgreSQL retained 2 connections, 22 parameters, and 6 permissions; public
  ingress returned the expected authentication redirect and HTTP 200 login
- `wsl-chrome-3` operator-visible proof returned browser, display `:10`, Route
  A, stream, selected ChatGPT target, and operator access ready

Result:

- Guacamole text input is the live default for each browser origin on its first
  load after this deployment. A later user-selected input method is retained.
- Recurring route maintenance uses only the fixed passwordless helper. Missing
  or incompatible root state still stops at the one-time bootstrap boundary.
- The installed 0.28.0 candidate remains unchanged to avoid invalidating active
  daemon owners. Plan 0091 retains the coordinated candidate-install and
  remaining-daemon handoff gate; four stale daemon owners remained at final
  readback, and its recurring timer stays disabled.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 135 | 2026-08-03

Scope: restore the public agent-browser and Guacamole routes, repair the
systemd interlock self-quiesce defect, and attempt one bounded installed proof.

Actions:

- Restored missing Guacamole READ grants for the three governed users on
  connection ids `1` and `2` from a prechange PostgreSQL backup.
- Diagnosed the external HTTP 502 as the dashboard service being stopped by a
  timer-triggered reconcile that included its own running interlock service in
  the systemd stop request.
- Added a red regression for the quiesce set, removed only the running service
  from that set, and retained the dashboard, interlock timer, and PostgreSQL
  backup timer.
- Built and materialized the corrected `0.28.0` payload. The apply then stopped
  at the existing interactive sudo gate for the root-owned privilege helper.
- Preserved every active daemon and browser session. Restored the dashboard and
  backup timer directly, and kept the recurring interlock disabled fail-closed.
- After the operator reported `wsl-chrome-3` unreachable, confirmed its Chrome
  process was absent, relaunched the durable profile once on Route A display
  `:10`, and reattached the restored ChatGPT target through the browser's
  canonical display allocation.

Validation:

- focused red then green Rust regression
- fifteen workstation install Rust tests
- workstation install fixture, host provision, fresh VM harness, Guacamole
  assets, PostgreSQL durability, and route-user sync tests
- Rust formatting, strict Clippy, selected validation, and release build
- installed executable and payload manifest SHA-256
  `23e71f0ffd8e75355719896a71d09849f57bf6c7e5c417eaf366e8489405d684`
- dashboard local and public HTTP 200; Guacamole readiness `ready`
- connections `1` and `2` each at three of three READ grants; route displays
  remain `:10` and `:11`
- dashboard active, backup timer enabled and active, interlock timer disabled
- `wsl-chrome-3` browser healthy, visible on `:10`, Route A ready, and the
  Guacamole operator URL HTTP 200

Result:

- The external dashboard and exact Guacamole Route A URL are reachable again.
- The source defect is repaired and the corrected payload is installed, but
  Plan 0091 is blocked at the installed runtime gate. Install doctor reports
  nineteen stale active daemons after `wsl-chrome-3` moved to the corrected
  runtime.
- The remaining gate requires interactive sudo and an owner-coordinated handoff
  of all active daemon sessions. Do not re-enable the recurring interlock until
  one installed pass exits successfully.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 134 | 2026-08-02

Scope: reconcile the installed P90 workstation payload and prove the repaired
route-bound display path before returning authority to last30days.

Actions:

- Accepted the operator-completed interactive workstation reconciliation and
  re-read installed provenance rather than assuming the source push implied
  runtime readiness.
- Ran both installed-runtime doctors and confirmed executable, daemon,
  dashboard, route-pool, route-display, and external-ingress convergence.
- Ran the repository live fixture. Its route-bound visible-window proof passed,
  but its in-process HTTP fixture could not serve while the same Node process
  blocked on the synchronous CLI child, so navigation remained `about:blank`.
  Cleanup released every disposable resource.
- Ran the installed binary directly against the already-running dashboard on
  disposable Route B. It loaded the target, returned a direct external
  Guacamole URL, and aligned route, display, browser window, operator-visible,
  and attachability proof before clean close.

Validation:

- install doctor: success, zero issues, executable SHA-256
  `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`,
  one converged daemon, dashboard ready
- remote-view doctor: ready, runtime converged, both `:10` and `:11`
  accessible, route pool and external Guacamole ingress ready
- direct gate: job `r63183`, `remote_view_open` succeeded on
  `guacamole-rdp-b` / `guacamole:2` / `:11`; target dashboard loaded in one
  readback with `browser_window_visible`, operator-visible ready, and
  attachability ready
- post-close remote-view doctor remained ready and both route allocations were
  free

Result:

- P90 is closed both in source and in the installed runtime. The original C63
  route-bound proof defect is repaired and live-proven.
- The fixture self-server/synchronous-child interaction is a separate harness
  defect; it consumed no last30days source attempt and left no retained
  resource.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

Next bounded action:

- Return to last30days Plan 0018 and review one fresh X successor identity.
  Preserve the durable `last30days-facebook` binding and require any genuine
  human handoff to use the direct external Guacamole URL.

## Turn 133 | 2026-08-02

Scope: finish the reviewed P90 runtime installation and publish the repo fix
after the litscout owner paused its workflow.

Actions:

- Confirmed the blocking `litscout-plan0311` daemon had exited.
- Re-ran the no-browser local publisher. It installed executable SHA-256
  `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`,
  restarted the dashboard, passed dashboard smoke, and reattached ten retained
  targets in `litscout-0312`.
- Ran install doctor, which failed closed on workstation-payload provenance:
  the source-free manifest still records the previous executable and the
  installed root-owned privilege helper differs from the bundled helper.
- Attempted the doctor-prescribed workstation reconciliation. It stopped at
  the interactive sudo gate without changing the root-owned helper.

Validation:

- installed, built, and checkout-reference binaries share SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`
- dashboard runtime manifest reports the same executable SHA-256
- dashboard service is active and its no-browser smoke passed
- install doctor remains non-ready until interactive workstation reconciliation

Result:

- The reviewed P90 executable is installed, but the full workstation payload
  is not provenance-converged. Do not consume a last30days source proof until
  interactive reconciliation and install doctor pass.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 132 | 2026-08-02

Scope: diagnose and repair the last30days route-bound visible-window proof
timeout without launching another browser or consuming a source attempt.

Actions:

- Read the two retained failed acquisition leases. Both selected the expected
  `last30days-facebook` profile, route `guacamole:2`, and display `:11`, then
  failed after twenty-one visible-window probes reported
  `display_probe_unavailable`.
- Confirmed both Chrome processes launched on the route-B environment and that
  current `xwininfo` probes can read displays `:10` and `:11`.
- Removed redundant ambient route discovery from the already normalized,
  exact-display proof path while retaining that guard for unbound probes.
- Preserved a sanitized, bounded underlying display-probe reason through the
  visible-window proof and AI-friendly error renderer.
- Independent review found the initial probe-error test did not lock down
  sanitization and truncation. Reworked it with multiline, quoted, over-limit
  input and exact assertions for single-line output and the 240-character cap.

Validation:

- forty-seven focused display-proof and error-rendering tests
- thirty remote-view unit tests after the independent-review rework
- all twenty-nine `remote_view_open` tests
- Rust formatting and strict Clippy with warnings denied
- complete Rust suite: 1,755 passed, 57 ignored, two state-sensitive failures;
  the authenticated-target case passed alone, while the unknown-command case
  remains intercepted by installed browser-recovery retry state before its
  expected assertion
- independent re-review passed exact commit `116ee810`
- no-browser publisher built candidate SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`
  but rolled back when active daemon `litscout-plan0311` did not exit; the
  resumed prior runtime then passed install doctor with all active daemons
  matching installed SHA-256 `cc22abe43a069e55e2dd46598b3eaa4954ffd4b8859388f646d7761c6c05da60`

Result:

- P90 is closed with independent PASS. The repo repair is implemented and
  validated, while installation is deferred until the owning litscout workflow
  can quiesce its active daemon.
- No browser or last30days source attempt was launched.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 131 | 2026-07-30

Scope: finish exact-head release gating, publish `v0.28.0`, and prove the
public artifact.

Actions:

- Bound the release decision to candidate `412684f6`, exact-head fast CI
  `30552821524`, full CI `30553477964`, and an independent fresh evaluator.
- Merged PR 7 as `80f64885`.
- Let dry run `30575205599` fail closed when its inventory included the
  repository JavaScript shim beside the seven release binaries.
- Used the single authorized remediation to isolate workflow asset staging.
  Commit `4132e782` passed exact-head full CI `30576313066`, corrected dry run
  `30578774481`, and publication run `30579564702`.
- Downloaded the public Linux x64 binary and checksum manifest, then performed
  a source-free idempotent reinstall on the accepted disposable Ubuntu VM.

Validation:

- Public tag `v0.28.0` resolves to exact commit `4132e782`.
- The public and installed Linux x64 binaries share SHA-256
  `4af2aba4e3670b2ffcd9601ab0134ad24cd13ec9e8131212f42a5645cb9baa22`
  and report version `0.28.0`.
- Install doctor, remote-view doctor, and the no-launch Route A dry run passed.
  The dry run requested no browser launch, route checkout, or tab open.

Result:

- P82 is closed and `v0.28.0` is the supported public workstation baseline.
- Detailed evidence is in
  `docs/dev/notes/2026-07-30-v0-28-0-release-validation.md`.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 130 | 2026-07-30

Scope: repair the next exact-head Windows and native E2E release defects.

Actions:

- Confirmed exact-head fast CI run `30549724644` fully green at commit
  `98316d14`.
- Dispatched full CI run `30550334355`. Windows found three native-path
  assertions, two inventory classifications hidden by a non-Unix process
  liveness stub, and one repository test whose `HOME` isolation is
  Unix-specific.
- Changed path fixtures to compare native paths, constrained the
  Unix-specific repository isolation test, and implemented Windows process
  liveness through the existing `windows-sys` dependency.
- The native E2E lane passed 55 of 56 browser tests and exposed a lifecycle
  regression after an intentional Chrome crash. Reconciliation preserved the
  terminal health event and correctly removed operational browser state, but
  relaunch could no longer reconstruct the recovery tombstone. Recovery now
  rehydrates that bounded state from event history, preserves trace context,
  emits the recovery sequence, and resolves through a fresh ready browser.

Validation:

- five focused cross-platform regressions
- recovery-history unit regression
- real-browser crash and automatic relaunch E2E
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- All focused regressions, the real-browser E2E, formatting, strict Clippy,
  and the complete serialized Rust CI harness pass locally.
- Full CI run `30550334355` remains valid Windows and E2E failure evidence for
  commit `98316d14`. Matrix fail-fast cancelled both macOS lanes after those
  failures.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 129 | 2026-07-30

Scope: repair the next exact-head full-CI defect.

Actions:

- Confirmed exact-head fast CI run `30547407293` fully green at commit
  `60c784e3`.
- Dispatched full CI run `30548163584`. Its ordinary Linux Rust suite found
  both sequential private-display launches selecting `:90`.
- Traced the collision to `start_remote_headed_virtual_display`, which returned
  after a fixed 150 ms delay without checking that the spawned Xvfb owned a
  ready display.
- Serialized selection inside the daemon and replaced the fixed delay with
  ownership-backed readiness polling. Early child exit, inspection failure,
  and timeout now fail closed and reap the child.
- The completed matrix also found an Apple Silicon daemon-socket fixture
  inheriting a temporary path longer than macOS `SUN_LEN`. Moved that Unix
  fixture to a short, unique path under `/tmp`.

Validation:

- twenty consecutive distinct-live-display regressions
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- The repeated focused regressions, formatting, strict Clippy, and complete
  serialized Rust CI harness pass locally.
- Full CI run `30548163584` remains valid collision evidence for commit
  `60c784e3`. Its Apple Silicon lane independently found the overlong socket
  fixture; fail-fast cancelled the Windows and macOS x64 lanes.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 128 | 2026-07-30

Scope: continue exact-head cross-platform release gating.

Actions:

- Confirmed exact-head fast CI run `30545123372` fully green at commit
  `a5423d6e`.
- Dispatched full CI run `30545744595`. The Windows target compiled and ran
  tests for nearly ten minutes before
  `test_manifest_resolves_stealthcdp_executable_path` exited abnormally.
- Traced the exit to a test fixture that interpolated a native Windows path
  directly into JSON. Backslashes in the path became invalid JSON escape
  sequences, and the explicit-config failure path correctly terminated.
- Replaced string interpolation with structured JSON serialization so the
  fixture remains valid on Unix and Windows.

Validation:

- focused manifest-resolution regression
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- The focused regression, formatting, strict Clippy, and complete serialized
  Rust CI harness pass locally.
- Full CI run `30545744595` is valid Windows failure evidence for commit
  `a5423d6e`; its macOS jobs were cancelled by matrix fail-fast.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 127 | 2026-07-30

Scope: run exact-head release CI and repair the first cross-platform defect.

Actions:

- Confirmed exact-head fast CI run `30541737279` fully green at commit
  `0cbd1729`.
- Dispatched full CI run `30542411936` against that exact commit. Its
  Apple Silicon macOS Rust job failed compilation because
  `statvfs.f_bavail` is 32-bit on that target while the byte calculation
  assumed the Linux 64-bit field shape. Matrix fail-fast then cancelled the
  Windows job.
- Added a typed conversion helper that accepts both platform widths and
  saturates the byte multiplication. Added regression coverage for mixed
  32-bit and 64-bit inputs plus overflow.
- Target-gated the `Path` import used only by Linux `/proc` process sampling,
  removing the adjacent macOS warning.
- Exact-head fast CI run `30543600554` passed at repair commit `2db64424`.
- Full CI run `30544211166` moved Apple Silicon beyond the repaired compile
  site, then the Windows test build found a Linux-only WSL helper referenced
  through runtime `cfg!` and a Unix `libc::kill` call compiled inside
  workstation lock recovery. Matrix fail-fast cancelled both macOS jobs.
- Converted the WSL test to compile-time Linux gating and split stale-lock
  probing into Unix and fail-closed non-Unix implementations. Target-gated the
  adjacent Unix-only resource-monitor imports.

Validation:

- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness
- focused mixed-width and saturation regression

Result:

- The local portability repair is green.
- Full CI runs `30542411936` and `30544211166` remain valid macOS and Windows
  failure evidence for their exact commits. The current repair requires a new
  exact-head fast run and a new manually dispatched full run after commit and
  push.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 126 | 2026-07-30

Scope: prove the rebuilt doctor-discovery candidate on the clean Ubuntu host
and enter release gating.

Actions:

- Built commit `ce26f0f6` as release-mode SHA-256
  `06e3b85ebc734c914ad8937afe0f169107cd6e646f5c129ebe1d7afe29aacca2`
  and staged it on the clean rebooted Ubuntu 24.04 host.
- The first idempotent convergence stopped fail-closed because Route A and B
  viewer daemons still ran the preceding candidate. The diagnostic identified
  both exact sessions and supplied bounded close commands. After closing only
  those sessions, the retry passed with candidate and installed hashes equal,
  dashboard active, and interlock timer active.
- Ran standalone doctors from a new login shell. Install doctor and
  remote-view doctor both returned success with no issues. Remote-view doctor
  resolved
  `/home/agent/.local/lib/agent-browser/0.28.0/scripts` and reported remote
  control, many-to-many prerequisites, and the path-scoped managed Chrome
  sandbox policy ready.
- Opened `about:blank` through Route A using the exact installed binary. The
  response selected `guacamole:1`, connection `1`, display `:10`, and returned
  `operatorVisible.state=ready`.
- Closed the bounded browser session. Retained Route A returned to
  `available` with `currentRouteAllocationId=null`.
- Ran whole-slice local validation. The first full Rust pass found that the
  installer order test still searched for
  `install_remote_view_privileges()` after the helper gained explicit
  arguments. Production order remained privilege setup before dependency
  installation. Updated the assertion to the current signature and reran the
  serialized Rust CI harness successfully.
- Fast CI run `30540857427` passed version sync, Rust Quality, the full Rust
  suite, dashboard, service-client, and workstation fixture jobs. Its final
  no-launch packet found stale profile-lookup MCP template and selection-order
  assertions, followed by a fixture assumption that local service status
  starts a daemon and writes `state.json`.
- Consolidated the lookup expectations around the current generated contract.
  Updated shared no-launch setup to accept the offline empty control-plane
  snapshot and create a minimal service state only when no daemon-created file
  exists. The full ten-command no-launch CI packet passes locally.

Validation:

- exact release artifact and installed-binary SHA-256 parity
- fail-closed stale-runtime diagnosis and emitted remediation
- zero-prompt idempotent convergence retry
- standalone install doctor from a fresh login shell
- standalone remote-view doctor from versioned installed assets
- live Route A operator-visible open and cleanup
- Rust formatting, strict Clippy, and the complete serialized Rust suite
- source-free installer, host, VM harness, Guacamole asset, durability, route
  user, and release-verifier fixtures
- service API/MCP and generated-client contracts
- route-confusion gates and live CDP tab streaming
- dashboard contract packet and production build
- docs production build, version sync, planning audit, and shared-skill parity
- complete CI no-launch service smoke packet

Result:

- The clean-install, reboot, durability, conflict, sandbox, installed-helper
  discovery, and final live Route A acceptance evidence is complete.
- Release remains no-go until exact-head fast and full CI, release dry run,
  merge, publication, and public-asset reinstall pass.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 125 | 2026-07-30

Scope: finish clean-install durability and route evidence, then repair a
managed Chrome sandbox defect found by the live gate.

Actions:

- Proved the exact candidate's mutation-free dry run, one-sudo first apply,
  exit-75 reboot boundary, zero-prompt continuation, canonical two-route
  substrate, and final doctors on a clean Ubuntu 24.04 overlay.
- Proved idempotency by preserving binary, manifest, units, secret, named
  volume, PostgreSQL system identity, Guacamole row IDs, routes, displays,
  profiles, and active units across a same-artifact rerun.
- Created a checksummed PostgreSQL custom-format backup and passed its isolated
  temporary-database restore drill with the expected tables, connections, and
  permissions.
- Proved the no-launch Route A plan selects `guacamole:1`, connection `1`, and
  display `:10` while requesting no launch, checkout, or tab.
- The first live Route A open failed before DevTools because Ubuntu 24.04
  AppArmor denied the managed Chrome sandbox user namespace. Lease rollback
  restored the route.
- Added a path-scoped AppArmor `userns` profile to the one-sudo host installer.
  The repair keeps the host restriction and Chromium sandbox enabled.
  Remote-view doctor now reports this policy and blocks live-gate readiness
  when it is missing, inactive, or mismatched.
- Loaded the exact profile on the disposable VM and repeated the live open.
  Route A reached `operatorVisible=ready`.
- Submitted a conflicting `guacamole:999/:99` authoritative definition while
  Route A was checked out. Reconciliation reported
  `skippedActiveConflictEntryIds=["guacamole-rdp-a"]`; the retained entry's
  pre/post SHA-256 remained
  `207ff06af5a214ee29a6cce2f2a8385f39db1e63048e2802310f627fcaef164f`.
  Cleanup returned Route A to `available`.
- Rebuilt commit `a05e7ed0` as SHA-256
  `b929200b25a4104995e41ee64510ad3b650b81dc663e00f8d1bbfb459c4e072d`
  and installed it on a new immutable-base overlay. The embedded installer
  produced one prompt, exit 75, a distinct reboot ID, loaded AppArmor policy
  before and after reboot, and a zero-prompt ready continuation.
- A standalone doctor then fell back to `/home/agent/scripts` even though the
  payload correctly installed its helpers under the versioned support root.
  Doctor discovery now checks
  `~/.local/lib/agent-browser/<version>/scripts` without requiring a checkout
  or ambient override.

Validation:

- workstation host-provision fixture
- managed Chrome sandbox-policy doctor unit
- remote-control viewer-prerequisite doctor unit
- live AppArmor parser and idempotent installer pass on Ubuntu 24.04
- live Route A open with `operatorVisible=ready`
- live active-conflict preservation and cleanup
- clean embedded-policy install and post-reboot loaded-profile proof
- versioned installed support-root discovery regression

Result:

- The installer and doctor now cover the live Chrome sandbox prerequisite that
  the earlier no-launch gates missed.
- Release remains no-go pending the rebuilt doctor-discovery candidate,
  standalone live doctor and Route A proof, full CI, release dry run, merge,
  and public-asset reinstall.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 124 | 2026-07-30

Scope: build the `0.28.0` release candidate, execute the disposable VM lane,
and repair independent Packet F findings.

Actions:

- Built a release-mode `0.28.0` binary with 80 embedded dashboard assets and
  pushed release preparation commit `4a374bea` to PR 7.
- Rebooted the iterative Ubuntu VM to a new boot ID and ran the release
  artifact source-free. The first run stopped fail-closed at route opening; a
  redacted direct retry selected canonical connections 1 and 2 and opened
  distinct displays `:10` and `:11`.
- Completed three independent audits covering installer safety, evidence
  sufficiency, release workflow, and documentation parity.
- Repaired secret-bearing subprocess arguments, ambient route-pool override,
  install-wide locking, payload hash provenance, changelog/version binding,
  Cargo.lock version validation, and missing installer fixtures in selector
  and fast CI.
- The first authoritative clean overlay exposed a 3.5 GiB cloud-image capacity
  defect during apt unpack. Added a 24 GiB VM disk default and a real-host
  6 GiB free-space gate that fails before sudo, payload staging, or package
  mutation.
- The resized clean overlay reached the zero-prompt post-reboot continuation,
  then Guacamole's first JVM process crashed while concurrent header-auth
  requests raced automatic account creation. One account transaction
  succeeded and the other returned a duplicate-key 500. Reconciliation now
  waits for full application readiness, makes one creation request, and
  accepts only the exact database user postcondition.
- The next clean continuation passed header creation and route opening, then
  failed while resetting a newly written interlock service that systemd had
  not loaded yet. A resumed run proved file-derived `LoadState=loaded` is not
  manager-load evidence for a static unit. Activation now observes the
  state-bearing `is-failed` output independently of its exit status, resets
  only an exact `failed` result, and verifies the postcondition after a reset
  race.
- The resumed candidate reached executable handoff, where the retiring daemon
  removed the replacement daemon's rebound Unix socket and session metadata.
  Shutdown now compares the socket device and inode before cleaning any shared
  session artifact.
- A new exact-candidate clean overlay passed the mutation-free dry run,
  one-sudo host preparation, exit-75 relogin, reboot, zero-prompt route
  convergence, unit activation, and install doctor. Final remote-view doctor
  exposed missing `xdpyinfo` plus a legacy host-guacd readiness assumption.
  Host packages now include display and visual-proof tools; readiness accepts
  the pinned Guacd container and managed Chrome outside `PATH`.

Validation:

- focused workstation and payload-integrity Rust tests
- source-free install fixture with concurrent-lock rejection
- route-specific user sync fixture
- release asset and version-bound changelog fixture
- exact selector recommendations for the new installer gates
- disk-capacity boundary unit test and resized VM harness contract
- Guacamole header-user postcondition unit test
- systemd unit-load/reset boundary unit test
- retiring-daemon socket-ownership unit test
- live runtime executable-handoff smoke
- clean-overlay dry run, one-sudo, reboot, and zero-prompt continuation
- container-backed Guacd and viewer-prerequisite readiness fixture

Result:

- Focused repair gates are green.
- Release remains no-go until the rebuilt candidate passes a clean Ubuntu
  install, reboot, idempotent rerun, restore and conflict drills, full CI,
  release workflows, merge, and public-asset reinstall.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 123 | 2026-07-29

Scope: implement and locally validate the source-free workstation installer
before disposable-host testing.

Actions:

- Hardened the release workflow around exact assets, embedded versions,
  execution, checksums, and published-download verification.
- Embedded the pinned Guacamole Compose stack, normalized schema, controller
  helpers, binary, manifest, and systemd user units.
- Added Ubuntu 24.04 amd64 host preflight, one initial sudo authorization,
  noninteractive dependency and privilege work, service verification, and the
  resumable group-refresh boundary.
- Added installed reconciliation for Chrome, Guacamole, PostgreSQL continuity,
  route users, canonical rows, readiness-selected XRDP displays,
  readiness-authoritative service projection, unit activation, final doctors,
  and a private receipt.
- Added fail-closed validation for active legacy route conflicts and private,
  idempotent generated secrets.
- Updated the public install help, README, docs site, skill, roadmap, and Plan
  0082 checkpoint to match the two-stage fresh-login contract.

Validation:

- 8 focused workstation Rust tests
- focused installed-script-root doctor test
- Rust format and strict Clippy
- source-free payload fixture
- embedded Guacamole asset fixture
- workstation host-provision fixture
- clean privilege fixture

Result:

- The source-free payload and local mocked host path are green.
- The release remains no-go until a disposable Ubuntu host proves install,
  reboot, canonical routes, backup restore, idempotency, and the remaining
  release gates.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 122 | 2026-07-29

Scope: open the fresh-install productization and formal `v0.28.0` release lane.

Actions:

- Verified that current local runtime health comes from source-checkout
  convergence and is not reproducible by the public release installer.
- Compared public `v0.27.0` at `17a284f` with plan-intake `main` at `ffda60dd`;
  current `main` is 107 commits newer while still reporting version `0.27.0`.
- Confirmed that P81's canonical route-state projection is absent from the
  public release.
- Confirmed the working dashboard interlock references this repository and
  invokes pnpm, while the complete Guacamole compose and schema substrate is
  not distributable from the release binary.
- Assigned three independent read-only audits covering installer architecture,
  clean-host test design, and release/version eligibility.
- Selected `v0.28.0` as the feature-release target and opened branch
  `prepare-v0.28.0`.
- Added Plan 0082 with explicit clean-install, one-sudo, reboot, idempotency,
  P81 regression, CI, pull-request, dry-run, publication, and public-asset
  gates.

Validation:

- current repository, installed binary, doctor, systemd, GitHub release, CI,
  and workflow readbacks
- Graphiti advisory recall verified against current source and runtime evidence
- three independent read-only audit receipts
- `git diff --check`

Result:

- P82 is open and the release is currently no-go.
- Implementation begins with the existing Rust Quality repair and red
  source-free installer tests.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 121 | 2026-07-28

Scope: diagnose and repair post-reboot Guacamole route-selection state drift
without launching a browser or consuming a Plan 0012 attempt.

Actions:

- Proved route readiness was current at `guacamole:1/:11` and
  `guacamole:2/:12`, while retained stable entries still pointed at legacy
  routes `guacamole:4/:10` and `guacamole:5/:11`.
- Traced `remote-view open` to retained route-pool selection and proved local
  convergence discarded the readiness JSON before health reconciliation.
- Added red-green parser and reconciliation tests, including fail-closed
  active-conflict and concurrent-checkout cases.
- Added guarded authoritative route-pool refresh, structured response
  evidence, schema and client alignment, and interlock projection.
- Published the local `0.27.0` candidate through guarded daemon handoff after a
  mode-0600 retained-state backup.
- Ran one normal convergence apply and observed the next scheduled interlock
  pass complete successfully.
- Ran a stable-entry `remote-view open` dry run for
  `last30days-facebook`; it selected `guacamole:1/:11` and requested no browser
  launch, route checkout, or tab open.
- Updated CLI help, README, docs site, agent skill, plan, roadmap, and a dated
  validation note.

Validation:

- focused Rust parser and reconciliation tests
- `pnpm test:local-runtime-convergence`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- applied convergence plus scheduled interlock receipt
- install doctor, remote-view doctor, retained-state, and no-launch selector
  readbacks

Result:

- P81 is complete. Retained route A is `guacamole:1/:11`; retained route B is
  `guacamole:2/:12`; both are available with no allocation.
- Installed and recurring runtime paths are healthy.
- No browser, source authentication probe, source canary, or Plan 0012 request
  ran. The next Plan 0012 attempt remains explicitly unauthorized.

## Turn 120 | 2026-07-28

Scope: investigate and remediate repeated Guacamole PostgreSQL reinitialization.

Actions:

- Proved the running container retained a stale Docker Desktop WSL bind
  attachment as `tmpfs`, while the declared host path remained on ext4 with a
  different cluster identifier.
- Added a red-capable durability contract, then implemented continuity status,
  atomic checksummed custom-format backup, identity recording, retention, and
  isolated restore drill.
- Made schema assurance fail closed on stale mount, identity discontinuity,
  partial schema, and absent schema for a recorded identity.
- Captured and restore-drilled a current backup before migration.
- Stopped Guacamole web access, migrated PostgreSQL to a named volume, restored
  the verified dump, and required exact route and permission invariants before
  restarting Guacamole.
- Installed and ran the daily backup service, restored the recurring runtime
  interlock, and retained the old bind directory.
- Recorded subagent receipt `spawned`: independent read-only P80 review,
  handle `/root/p80_independent_review`, with no runtime or file mutation. Its
  first verdict found six consolidated durability gaps; the bounded
  remediation pass fixed them and its re-review returned `PASS` with no
  residual blocking findings.

Validation:

- `pnpm test:guacamole-postgres-durability`
- `pnpm test:rdp-guac-postgres-hardening`
- `pnpm ensure:rdp-guac-postgres -- --dry-run`
- pre-migration and post-migration isolated restore drills
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- named-volume mount, identity, route, parameter, and permission readbacks
- installed backup service and timer readbacks
- recurring interlock receipt
- route-pool readiness, remote-view doctor, and install doctor

Result:

- P80 is complete. PostgreSQL uses the named volume on ext4, continuity is
  ready, the database retains two routes, 22 parameters, and four permissions,
  and a verified recovery artifact exists.
- Backup and interlock services succeeded; both timers are enabled and active.
- Remote control and many-to-many readiness remain ready.
- No browser, source-authentication, or old-bind deletion occurred.

## Turn 119 | 2026-07-28

Scope: execute and close P79 route-specific isolation repair.

Actions:

- Added red-green convergence coverage for missing fixtures and typed
  same-user display collapse.
- Added a guarded transaction that migrates exact supported legacy rows in
  place, configures distinct route-specific users, removes `color-depth`,
  preserves permissions, and fails closed on ambiguous topology.
- Made the existing-user sync spelling a compatibility alias to the safe
  route-specific migration.
- Made live Xorg inference authoritative over persisted display hints.
- Updated CLI help, README, docs site, agent skill, roadmap, plan, and
  validation note; synchronized the installed shared agent skill.
- Committed and pushed the reviewed implementation as `2dcac761`.
- Captured live pre-state, closed only the two failed Guacamole viewer
  sessions, and ran one guarded convergence apply.
- Preserved Guacamole connection ids `1` and `2`, migrated to canonical route
  names and users `agent-browser-rdp-a/b`, and opened displays `:11` and `:12`.
- Diagnosed the final aggregate's residual false failure as a cwd-relative
  inspector path, added a failing regression, fixed module-relative
  resolution, and pushed `641f45ae`. No second migration or display
  restoration was run.
- Proved read-only convergence success, installed doctor success, one
  successful recurring interlock pass with no route mutation steps, and an
  enabled active waiting timer.

Validation:

- `pnpm test:local-runtime-convergence`
- `pnpm test:rdp-guac-route-specific-user-sync`
- `pnpm test:rdp-route-display-selection`
- `pnpm test:rdp-guac-postgres-hardening`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml output -- --test-threads=1`
- `pnpm --dir docs build`
- `agent-browser doctor remote-view --json`
- `agent-browser install doctor --json`
- read-only `pnpm converge:local-runtime -- --skip-publish --json`
- `systemctl --user start agent-browser-runtime-interlock.service`
- exact Guacamole row, parameter, permission, Xorg, and X11 socket readbacks

Result:

- P79 is complete. Remote control and many-to-many prerequisites are ready,
  route displays are isolated by user, and the recurring interlock is healthy.
- Source and installed runtime are reported separately: `origin/main` includes
  `641f45ae`; the binary was not replaced and install doctor reports seven
  converged runtimes with zero issues.
- No application browser or source-authentication state was touched.
- Guacamole PostgreSQL reset attribution and backup/restore durability remain a
  separate unresolved packet.

## Turn 118 | 2026-07-28

Scope: diagnose the failed post-reboot agent-browser route substrate and open
the authorized bounded repair.

Actions:

- Reproduced route readiness and confirmed that the two Guacamole RDP records
  and read permissions exist, while route B's expected display socket does
  not.
- Correlated Guacamole, guacd, and XRDP evidence. Both routes authenticated,
  but XRDP reconnected route B to the same user's display `:10`.
- Confirmed XRDP 0.9.24 with sesman `Policy=Default`; the configured 24/32
  color-depth distinction did not yield separate allocation keys.
- Confirmed the existing route-specific users and secret keys are ready and
  the route-specific setup isolation gate passes read-only.
- Identified two additional repair requirements: migrate the existing legacy
  rows in place to avoid four managed routes, and prefer live inferred display
  allocation over stale configured display hints.
- Opened P79 with one implementation/review cycle, one authorized live attempt,
  explicit rollback/stops, and no application-browser or authentication scope.
- Recorded subagent receipt `not_spawned`: this repair crosses one shared live
  Guacamole/XRDP route substrate and needs one critical-path owner.

Validation:

- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `node scripts/inspect-rdp-route-displays.js --windows`
- `pnpm --silent setup:rdp-guac-route-pool -- --dry-run`
- Guacamole PostgreSQL route and permission readbacks
- XRDP service configuration and journal readbacks

Result:

- Root cause is the same-user XRDP session collapse, not repository drift,
  Guacamole authentication, application-browser behavior, or source-account
  authentication.
- P79 is active. Live state remains unchanged while failing behavior tests and
  the guarded migration are implemented.

## Turn 117 | 2026-07-27

Scope: run the authorized replacement P78 Packet C attempt and diagnose the
first residual failure.

Actions:

- Confirmed the recurring interlock timer remained enabled but inactive and
  the pre-state still contained zero Guacamole routes.
- Ran exactly one corrected convergence apply with publication skipped.
- Provisioned two Guacamole RDP connections, their required read grants, and
  distinct configured target identities.
- Stopped after display restoration returned status 1. No second sync or
  display-open attempt ran.
- Diagnosed current route readiness, live XRDP processes, Guacamole and XRDP
  logs, installed XRDP policy, and the retained convergence receipt.

Validation:

- Report-only route readiness finds two connections and complete permissions.
- Route A has a live `:10` X11 socket.
- Route B has no `:11` X11 socket.
- XRDP logs show route A created display `:10`, then route B reconnected the
  same user to display `:10`.
- XRDP 0.9.24 is configured with `Policy=Default`, which allocates by user and
  negotiated bit depth. The configured Guacamole 24 and 32 color-depth values
  did not produce distinct live allocation keys.
- The retained convergence receipt records successful fixture provisioning,
  failed `restore_rdp_route_displays`, and final next action
  `repair_rdp_route_display_session`.

Result:

- Packet C remains blocked after its authorized replacement attempt. Packet D
  did not run.
- The database route-fixture defect is repaired, but the existing-user
  two-display isolation assumption is disproved on this runtime.
- The timer remains paused and no application browser or authentication
  surface was touched.

Next recommendation:

- open a new bounded isolation plan to choose between route-specific users and
  a reviewed XRDP session-policy change before any further live display
  mutation.

## Turn 116 | 2026-07-27

Scope: execute P78 through its single authorized live recovery attempt.

Actions:

- Added a behavior-level fixture harness for empty route fixtures, dry-run
  immutability, unrelated doctor actions, and doctor refresh ordering.
- Added the exact typed fixture-provisioning remedy to local-runtime
  convergence and updated README, CLI help, agent skill, docs site, and
  post-reboot operator guidance.
- Paused the enabled interlock timer to prevent a race with the controlled live
  attempt.
- Captured the empty pre-state and ran one authorized convergence apply with
  installed doctors and binaries.
- Stopped at the first typed failure. The existing-user sync rejected the
  plan's unsupported `--apply` argument before changing Guacamole rows.
- Corrected the controller, regression fixture, and plan example through the
  packet's one review/rework cycle. Did not run a second live sync.
- Audited backup truth. No usable PostgreSQL dump, archive, snapshot timer, or
  documented restore workflow was found for the bind mount.

Validation:

- `pnpm test:local-runtime-convergence` failed before the controller remedy,
  passed after it, failed again when the fixture enforced the real
  apply-by-default sync contract, and passed after the rework.
- `node --check scripts/converge-local-runtime.js` passed.
- `node --check scripts/test-local-runtime-convergence-fixture.js` passed.
- `pnpm test:rdp-guac-postgres-hardening` passed.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `cargo test --manifest-path cli/Cargo.toml output -- --test-threads=1`
  passed 34 focused tests.
- `pnpm --dir docs build` passed.
- The repo and installed agent-browser skill copies match.
- `git diff --check` passed after closeout documentation.
- The retained convergence receipt records
  `provision_rdp_guac_route_fixtures` status 2 and contains no later display or
  access steps.
- Post-attempt report-only readiness still reports zero RDP connections.

Result:

- Packets A and B are complete.
- Packet C is blocked after its single authorized attempt; Packet D remains
  blocked.
- The corrected live command has not run. A replacement Packet C attempt
  requires new explicit authorization.
- No application browser, authentication surface, source canary, or
  last30days acquisition was touched.

Next recommendation:

- explicitly authorize one replacement Plan 0078 Packet C live attempt, then
  complete installed timer proof and open a separate database durability
  packet.

## Turn 115 | 2026-07-27

Scope: bind P78 planning to commit, push, validation, and durable-memory
receipts.

Actions:

- Preserved `6d5cc908` as the coherent P78 roadmap, plan, and diagnostic
  runbook commit.
- Pushed the commit to `origin/main` and verified local, tracking, and remote
  refs agree.
- Queued one source-backed Graphiti closeout in `agent_browser_main` after the
  provider readiness probe passed.
- Did not provision route fixtures, open route displays, invoke convergence
  apply, retry remote-view acquisition, or launch a browser.

Validation:

- `node scripts/test-local-runtime-convergence.js` passed.
- `node scripts/test-rdp-guac-postgres-hardening.js` passed.
- `git diff --check` passed.
- local `HEAD`, `origin/main`, and the remote main ref agreed at
  `6d5cc908f1849970085d0ad059fbebfcdf8b9652`.

Result:

- P78 remains `PLANNED` and awaits explicit Plan 0078 Packet A authorization.
- Both agent-browser and the coordinating last30days Plan 0012 authority are
  durably pushed and clean.

Graphiti:

- job `6022b120-eb35-4bd9-8a50-0079a40b3782` completed in one attempt;
- episode `2e3a6d86-3d53-40b0-b4c9-0db6398d9264` is visible in
  `agent_browser_main` and passed the read-after-write check;
- the source description binds the episode to Plan 0078 commit `6d5cc908`.

Next recommendation:

- explicitly authorize Plan 0078 Packet A before any source or live route
  repair.

## Turn 114 | 2026-07-27

Scope: diagnose the post-reboot Guacamole route-preflight failure and create a
bounded implementation plan without mutating the live route substrate.

Actions:

- Reproduced the blocker with the report-only route-pool readiness and
  route-display inspection commands.
- Proved the current Guacamole database has zero connection and permission
  rows, while the retained route service state still refers to historical
  routes.
- Bound the data loss to PostgreSQL `initdb` at 2026-07-27 11:46:23 UTC; the
  reason the bind-mounted data directory was empty remains unresolved.
- Traced the remote-view acquisition preflight and confirmed it correctly
  fails before browser creation when no display allocation or available
  route-pool entry exists.
- Traced the convergence controller and found that it ensures the schema but
  has no remedy branch for
  `provision_second_guacamole_rdp_connection`; its display-recovery predicate
  recognizes only three later display-session actions.
- Opened planned P78/Plan 0078 for an idempotent existing-user route-fixture
  remedy, deterministic coverage, documentation, one separately authorized
  live recovery attempt, and installed interlock proof.
- Did not provision routes, open displays, retry remote-view acquisition,
  launch a browser, inspect authentication, or run a source canary.

Validation:

- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
  deterministically reported zero RDP connections and the provisioning next
  action.
- `pnpm --silent inspect:rdp-route-displays` reported no candidate route
  displays.
- Direct read-only Guacamole database counts reported zero connections and
  zero connection permissions.
- Current convergence receipt is unsuccessful, retains the provisioning next
  action, and contains no selected remedy.
- CodeGraph source tracing bound the missing action coverage to
  `routeDisplayRecoveryRequired()` and the apply sequence in
  `scripts/converge-local-runtime.js`.

Result:

- P78 is `PLANNED` and awaits explicit execution authorization.
- The failure class is route-fixture recovery, not application-browser or
  source authentication.
- The next safe action is Plan 0078 Packet A; no live repair is authorized by
  this planning turn.

Graphiti:

- Discovery in `agent_browser_main` supplied advisory history for prior
  post-reboot display reconciliation and earlier two-route live proof.
- Current repo source, runtime doctors, database counts, and retained
  convergence evidence remain authoritative.
- A closeout write should occur only after this planning slice has a durable
  commit.

## Turn 113 | 2026-07-25

Scope: reconnect a registered client session to its healthy retained service
browser instead of auto-launching an unrelated profile.

Actions:

- Reproduced the `last30days` X adapter failure below the content worker.
- Proved access planning and workspace acquisition selected
  `session:last30days-facebook`.
- Proved the next registered-session `tab_list` command attempted to launch
  the default profile and collided with `auracall-corel`.
- Added a same-session retained-browser resolver before ordinary-command
  auto-launch.
- Kept acquisition actions on the existing shared-profile fresh-tab path and
  made reconnected client teardown detach rather than close the browser.
- Rebuilt and installed the runtime through the executable handoff publisher.
- Ran one bounded no-navigation X authentication readback through the repaired
  installed session.

Validation:

- focused retained-session and existing shared-profile Rust tests
- Rust format and clippy
- CDP stream derivation tests
- `pnpm test:route-confusion-gates`
- `pnpm test:service-cdp-tab-streaming-live`
- installed SHA convergence and `agent-browser install doctor --json`
- installed registered-session X auth evaluation without navigation

Result:

- The installed `last30days-facebook` session reconnects to
  `session:last30days-facebook`.
- The existing X tab reports authenticated with no login form, checkpoint, or
  restriction.
- Browser PID `1669680`, the retained CDP endpoint, RDP provider, shared
  display, and Guacamole route `guacamole:4` remained intact.
- The runtime publisher reported a separate missing-handoff-file failure for
  `auracall-corel`; subsequent doctor passed, but its duplicate listener
  inventory remains out-of-scope follow-up.

Graphiti:

- Discovery found prior shared-profile attach/reuse context in
  `agent_browser_main`.
- Closeout memory should use this note after the source commit is durable.

## Turn 112 | 2026-07-25

Scope: repair the X workspace tile so its retained remote-headed browser opens
through Guacamole/RDP instead of an unrelated CDP snapshot.

Actions:

- Reproduced the failure with the exact dashboard selection
  `daemon-session:last30days-facebook`.
- Traced the selection path to `WorkspaceRemoteViewport`: only an explicit
  browser ID could resolve a retained service browser, so a daemon-session
  selection synthesized a CDP browser and let that snapshot win.
- Added one deterministic workspace-selection resolver that maps explicit
  browser IDs first and selected daemon sessions second.
- Made a linked retained service browser authoritative over a selected CDP
  snapshot while preserving CDP as the fallback for sessions with no linked
  service browser.
- Tightened the dashboard runtime smoke with an explicit expected-provider
  assertion so a Guacamole route cannot pass by rendering a CDP canvas.
- Recovered the retained X browser on the existing
  `last30days-facebook` profile and Route A Guacamole allocation after stale
  runtime evidence was reconciled.
- Published the dashboard bundle and converged the installed user-scoped
  runtime.

Validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:route-confusion-gates`
- `pnpm build:dashboard`
- `node --check scripts/smoke-local-dashboard-runtime.js`
- installed dashboard smoke with `--workspace-session
  last30days-facebook --expect-workspace-provider rdp_gateway`
- `agent-browser install doctor --json`

Result:

- The X tile resolves to `browser:session:last30days-facebook`.
- The rendered viewport is the Guacamole iframe for `guacamole:4`; no CDP
  canvas is present.
- The retained browser reports `rdp_gateway`,
  `manual_attached_desktop`, `remote_headed`, and ready operator visibility.
- The live and installed dashboard hashes match, runtime convergence reports
  zero stale runtimes, and install doctor reports no issues.
- This proves provider and route correctness, not X authentication state.
- The Graphiti closeout write remains pending because its bounded provider
  readiness probe timed out; no write was queued while the provider was
  degraded.

## Turn 111 | 2026-07-25

Scope: open, implement, validate, and close P77 profile discovery and manual
browser launch UX.

Actions:

- Kept the source note unchanged and converted its requirements into Plan
  0077.
- Audited current launcher, profile selector, HTTP, MCP, generated client,
  runtime state, resource discovery, and workspace inventory surfaces.
- Confirmed that the existing dashboard launcher and profile lookup are
  implementation inputs rather than satisfactory product behavior.
- Reproduced a no-launch selector defect where an X query chose
  `stealthcdp-default` by browser-build fallback instead of the exact
  authenticated `last30days-facebook` profile.
- Implemented one deterministic selector/catalog contract, one authoritative
  manual-browser runtime projection, a dedicated workspace class, and a
  server-backed dashboard workflow.
- Added CLI, HTTP, MCP, generated-client, and dashboard discovery parity with
  explicit ranked evidence and structured `not_found`.
- Added executable handoff so local runtime publication replaces daemons while
  preserving active browser PIDs, ports, targets, and streams.
- Tightened stale session lease expiration and merge persistence, repaired
  orphaned Route B pool checkouts, and made convergence run the real service
  reconcile action.
- Published the installed runtime and proved manual no-CDP visibility through
  both CLI status and the authenticated dashboard API.
- Recorded privacy boundaries, live acceptance criteria, and the explicit
  non-delegation reason.

Validation:

- Rust format, clippy, focused selector, status, lifecycle, route, model, and
  contract tests.
- Service client generation and API/MCP parity.
- Dashboard workspace, navigator, selected-context, launcher, docs, and
  production build checks.
- Live exact X and unmatched free-text lookup, detached no-CDP inventory and
  cleanup, executable handoff, Route B open, install doctor, remote-view
  doctor, and local runtime convergence.

Result:

- P77 is closed.
- Exact X lookup selected `last30days-facebook` by authenticated-target
  evidence; a free-text miss returned structured `not_found`.
- Five active sessions survived final daemon replacement with their browser
  PIDs and CDP endpoints unchanged.
- Final convergence reported zero stale runtimes, current authoritative
  listeners, ready remote control, and no install issues.

## Turn 110 | 2026-07-19

Scope: complete P76 source, contract, installed-runtime, and closeout gates.

Actions:

- Completed bounded clipboard-write capture, daemon-owned dependent batches,
  per-command timing fields, browser accessibility-tree role lookup, and
  bounded closed-tab status projection with full diagnostic retrieval.
- Updated every required CLI help, README, skill, docs-site, schema, contract,
  HTTP, MCP, generated-client, and inline documentation surface.
- Added and passed a real Chrome accessibility fixture for dynamically mounted
  `aria-labelledby` content and supported shadow-root lookup.
- Corrected two stale close-action tests to match intentional removal of
  `NotStarted` browser placeholders and empty released sessions.
- Used the installed smoke to find and repair missing
  `closedTabProjection` metadata on the CLI-local no-launch status path.
- Published the local dashboard runtime, retired stale daemon sessions, and
  verified a converged installed runtime with `agent-browser install doctor`.
- Queued one compact Graphiti closeout episode in `agent_browser_main` from the
  completed plan and redacted incident note after provider readiness passed.

Validation:

- `scripts/ci/rust-tests.sh`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused clipboard, CDP lifecycle, dependent batch, service projection, and
  real Chrome accessibility tests
- `pnpm --config.verify-deps-before-run=false test:service-client`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- repeated successful docs and dashboard production builds
- installed live capture, timeout recovery, dependent batch, status projection,
  and doctor readbacks with a temporary profile

Result:

- P76 is closed. All six slices are implemented and documented.
- The unresolved clipboard promise returned within the bounded deadline, a
  following evaluation succeeded on the same target, and opt-in write capture
  restored the patched method.
- Installed ordinary and full status modes returned their respective
  projection metadata. Final install doctor reported no issues and zero stale
  runtimes.
- The privacy-safe closeout evidence is recorded in Plan 0076 and the incident
  note. The temporary validation profile was removed.

## Turn 109 | 2026-07-19

Scope: open and execute P76 clipboard target recovery and interaction
performance remediation.

Actions:

- Reviewed the retained clipboard incident against the current clipboard,
  CDP timeout, evaluation, locator, batch, and service-status implementations.
- Opened Plan 0076 with six bounded slices covering evidence correction,
  cancellation-safe clipboard timeout and recovery, clipboard-write capture,
  timing and dependent batching, accessible locator repair, and closed-tab
  status projection.
- Made the CDP command lifecycle the deep module for deadline enforcement,
  pending-command cleanup, late responses, and timeout classification.
- Recorded review mediation so empty clipboard text remains successful, target
  recovery must be proved, locator coverage reproduces accessible-name
  behavior, and service-status compaction remains a projection rather than a
  mutation of persisted lifecycle authority.
- Completed Slice A by correcting causal language in the incident note,
  labeling historical observations, replacing the insufficient portal-only
  locator regression, and adding a privacy-safe validation artifact template.
- Completed Slice B source work with a cancellation-safe per-command CDP
  deadline, Chrome renderer timeout, execution termination fallback, normal
  evaluation health probe, successful empty-text output, stable failure codes,
  and explicit replacement-tab guidance.
- Updated CLI help, README, docs command and streaming pages, MCP tool
  description, and repo plus installed skill guidance for the bounded read
  contract.

Validation:

- `git diff --check`
- `cargo test --manifest-path cli/Cargo.toml clipboard -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::cdp::client::tests -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- `pnpm --config.verify-deps-before-run=false --dir docs build`
- `pnpm validation:select -- --base HEAD` was blocked before selection by
  pnpm 11 ignored-build enforcement. The underlying selector script completed
  directly without approving dependency build scripts.

Result:

- P76 is open and in progress. Slices A and B source work are complete. Slice C
  clipboard-write capture is the current execution boundary; Slice B installed
  retained-browser proof remains a final closeout gate.

## Turn 108 | 2026-07-06

Scope: close P69 Slice F live proof and fix live-discovered shared-profile and
route repeat-open failures.

Actions:

- Reproduced the in-use profile refusal through
  `scripts/open-rdp-guac-route-displays.js`: plain `open` against
  `/home/ecochran76/.agent-browser/guacamole-route-viewers/a` failed even
  though service state showed `session:rdp-guac-route-a-viewer` already owned
  the profile and exposed a CDP endpoint.
- Updated shared-profile auto-launch target selection so `open` participates in
  retained-browser attach/reuse and so the current session's live service
  browser can be selected when daemon metadata drift leaves `state.browser`
  empty.
- Reproduced the P69 route repeat bug in the full fixture smoke: first
  `remote_view open` checked out `guacamole-rdp-a`, while repeat open failed
  with `route_pool_entry_unavailable`.
- Updated remote-view acquisition to treat same-owner `checked_out` and
  reconciliation-stale `orphaned` route records as reusable when browser id,
  session id, route id, and display allocation still agree.
- Overrode stale route-display env for the live proof with inspected route
  displays `:10` and `:11`.

Validation:

- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_attach_target -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `pnpm test:service-client`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:rdp-guac-route-pool-readiness`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `git diff --check`

Result:

- Full fixture live proof passed with artifact
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T22-14-26-356Z`.
  It proved route `guacamole:4`, display allocation `remote-view-display:10`,
  display `:10`, `route_bound_ready`, `browser_window_visible`, one active
  intended target, and OCR text containing `REMOTE VIEW OPEN FIXTURE 55948`.
  P69 validation is complete.

## Turn 107 | 2026-07-06

Scope: audit P69 Slice C residual `remote_view_open` orchestration and remove
stale dispatcher rollback wrappers before live proof.

Actions:

- Audited the remaining `remote_view_open` route-bound sequence after the
  handoff recovery extraction.
- Confirmed the remaining action-local responsibilities are command dispatch,
  live browser side effects, timestamp supply, and repository/service plumbing.
- Removed stale `remote_view_open_rollback_acquisition_lease`.
- Removed stale `remote_view_open_update_acquisition_lease_cleanup`.
- Updated the acquisition-rollback test to call
  `remote_view_handoff::rollback_route_bound_handoff_acquisition` directly with
  an explicit observed timestamp.
- Updated P69 to mark Slice C ready for live proof rather than continuing
  unbounded micro-extractions.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Stale dispatcher rollback vocabulary is gone. P69 remains open for Slice F
  installed-runtime/live proof.

## Turn 106 | 2026-07-06

Scope: continue P69 Slice C by moving failure recovery cleanup-task selection
into the handoff recovery result.

Actions:

- Extended `remote_view_handoff::RouteBoundHandoffFailureRecovery` with
  `cleanup_task`.
- Updated `remote_view_handoff::begin_route_bound_handoff_failure_recovery` so
  it returns the selected cleanup task alongside rollback and cleanup-plan
  evidence.
- Rewired `remote_view_open_rollback_failure_after_cleanup` so `actions.rs`
  executes the handoff-selected cleanup task directly instead of interpreting
  `cleanup_plan` and `skipped_cleanup`.
- Updated the action cleanup test to exercise the task form.
- Extended handoff-module recovery coverage to assert the selected skipped
  cleanup task.
- Updated P69 to record this recovery-result extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure recovery now returns the cleanup task selected by the handoff module.
  P69 remains open for final sequencing assessment and Slice F live proof.

## Turn 105 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure cleanup task
vocabulary into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureCleanupTask`.
- Added `remote_view_handoff::route_bound_handoff_failure_cleanup_task`.
- Added
  `remote_view_handoff::route_bound_handoff_failure_cleanup_task_result`.
- Rewired `remote_view_open_cleanup_after_failure` so `actions.rs` still
  dispatches async tab-close or browser-close side effects, but no longer owns
  the close-tab command payload, close-browser task marker, skipped-cleanup
  payload, or cleanup result mapping.
- Added handoff-module coverage for close-tab, close-browser, and skipped
  cleanup task shapes.
- Updated P69 to record this cleanup task extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup task construction and cleanup result mapping are now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 104 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible proof record assembly
into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_operator_visible`.
- Moved route, display, browser, tab, stream, Guacamole, and URL-readiness
  response vocabulary out of `actions.rs`.
- Rewired `remote_view_open` dry-run, pre-checkout proof, final proof, and
  related route-bound tests to use the handoff-owned proof builder.
- Added handoff-module coverage for the operator-visible proof record shape.
- Updated P69 to record the proof assembly extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible route-bound proof vocabulary is now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 103 | 2026-07-06

Scope: continue P69 Slice C by moving the remaining simple rollback failure
descriptors into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_tab_open_failure`.
- Added `remote_view_handoff::route_bound_handoff_focus_failure`.
- Added `remote_view_handoff::route_bound_handoff_visible_window_proof_failure`.
- Rewired the `remote_view_open` tab, focus, and visible-window proof failure
  branches so `actions.rs` still executes async browser commands and rollback
  cleanup, but no longer owns those failure phase strings or rollback cleanup
  payloads.
- Added handoff-module coverage for the simple rollback failure descriptor
  shapes.
- Updated P69 to record this descriptor consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Tab-open, focus, visible-window proof, and checkout failure descriptors now
  share the handoff module shape. P69 remains open for broader sequencing
  consolidation and Slice F live proof.

## Turn 102 | 2026-07-06

Scope: continue P69 Slice C by moving checkout failure diagnostic preparation
into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffRollbackFailure`.
- Added `remote_view_handoff::route_bound_handoff_checkout_failure`.
- Rewired the `remote_view_open` checkout failure branch so `actions.rs` still
  executes the async checkout command and rollback cleanup, but no longer owns
  the checkout failure phase string or rollback cleanup payload.
- Added handoff-module coverage for the checkout failure phase and cleanup
  payload.
- Updated P69 to record this checkout failure diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Checkout failure phase and rollback cleanup payload construction now live in
  the handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 101 | 2026-07-06

Scope: continue P69 Slice C by moving post-checkout proof sequencing into the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProof`.
- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProofInput`.
- Added `remote_view_handoff::route_bound_handoff_post_checkout_proof`.
- Rewired `remote_view_open` so `actions.rs` supplies the final
  operator-visible proof calculation and executes rollback when needed, while
  the handoff module derives the final route binding, invokes the proof
  calculation, and applies the final proof readiness gate.
- Added handoff-module tests for ready and not-ready post-checkout proof
  results.
- Updated P69 to record this post-checkout proof sequencing extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Final route-binding derivation, final proof calculation invocation, and final
  proof readiness gating now run as one handoff step after checkout. P69 remains
  open for broader sequencing consolidation and Slice F live proof.

## Turn 100 | 2026-07-06

Scope: continue P69 Slice C by moving opened-response final route-binding
derivation into the handoff completion path.

Actions:

- Changed `remote_view_handoff::CompleteRouteBoundHandoffOpenInput` so callers
  no longer pass a precomputed final route binding.
- Updated `remote_view_handoff::complete_route_bound_handoff_open` to derive
  the final route binding from checkout readback before completing the lease and
  assembling the opened response.
- Rewired `remote_view_open` to pass only the planned route binding and checkout
  readback into the completion helper.
- Strengthened handoff-module coverage so the completion helper proves it uses
  checkout readback by returning `route-final` in the opened response.
- Updated P69 to record this final route-binding ownership move.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Opened-response final route-binding derivation now lives in the handoff
  completion path. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 99 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible readiness gating into
the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure_if_not_ready`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure_if_not_ready`.
- Rewired `remote_view_open` so `actions.rs` still computes operator-visible
  proof, but no longer interprets pre-checkout or final proof `state` values to
  decide whether rollback diagnostics are required.
- Added handoff-module tests for ready and not-ready pre-checkout proof gates
  and final proof context preservation.
- Updated P69 to record this readiness-gating extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible readiness decisions now live in the handoff module. P69
  remains open for broader sequencing consolidation and Slice F live proof.

## Turn 98 | 2026-07-06

Scope: continue P69 Slice C by moving successful route-bound open finalization
into the handoff module.

Actions:

- Added `remote_view_handoff::CompleteRouteBoundHandoffOpenInput`.
- Added `remote_view_handoff::complete_route_bound_handoff_open`.
- Rewired `remote_view_open` so `actions.rs` still performs command dispatch
  and final operator-visible proof, but no longer completes the route-bound
  lease, derives browser-build proof, serializes the lease, or assembles the
  opened response locally.
- Added handoff-module coverage proving the helper finalizes the lease and
  returns the opened `routeBoundHandoff` response surface.
- Updated P69 to record this successful-open finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Successful route-bound open finalization now lives in the handoff module.
  P69 remains open for broader sequencing consolidation and Slice F live proof.

## Turn 97 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure recovery staging into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureRecoveryInput` and
  `RouteBoundHandoffFailureRecovery`.
- Added
  `remote_view_handoff::begin_route_bound_handoff_failure_recovery` to perform
  rollback-before-cleanup sequencing and return the cleanup plan plus any
  skipped-cleanup payload.
- Added `remote_view_handoff::RouteBoundHandoffImmediateFailureInput` and
  `route_bound_handoff_immediate_failure` for pre-browser display/launch
  failures that only need rollback plus summary formatting.
- Rewired `remote_view_open` so `actions.rs` still executes async tab/browser
  close commands, but no longer derives cleanup plans from launch/tab evidence
  or builds immediate failure rollback summaries locally.
- Added handoff-module tests for failure recovery staging and immediate
  failures.
- Updated P69 to record this recovery-staging extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Rollback-before-cleanup ordering, cleanup-plan selection, skipped-cleanup
  detection, and immediate display/launch failure summaries now live in the
  handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 96 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible failure diagnostics into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffProofFailure`.
- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure`.
- Rewired `remote_view_open` operator-visible and final operator-visible
  failure branches to use those helpers for paired error text and rollback
  cleanup payloads.
- Added handoff-module tests proving the diagnostics preserve
  `routeBoundHandoff` and `preCheckoutOperatorVisible` labels.
- Updated P69 to record this failure-diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible proof failure error text and cleanup payload construction now
  lives in the handoff module. P69 remains open for full sequencing extraction
  and Slice F live proof.

## Turn 95 | 2026-07-06

Scope: continue P69 Slice C by moving pre-launch and launch-failure cleanup
payloads into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_pre_launch_failure_cleanup`.
- Added
  `remote_view_handoff::route_bound_handoff_launch_failure_cleanup`.
- Rewired the `remote_view_open` display-access failure branch and browser
  launch failure branch to use those handoff helpers instead of hand-built JSON.
- Added handoff-module coverage for the skipped-before-launch and
  skipped-after-launch cleanup payload shapes.
- Updated P69 to record this cleanup-payload extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Pre-launch and launch-failure cleanup JSON shapes now live in the handoff
  module. P69 remains open for deeper end-to-end orchestration and Slice F live
  proof.

## Turn 94 | 2026-07-06

Scope: continue P69 Slice C by moving reused-browser launch evidence into the
handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_reused_browser_launch_result`.
- Rewired `remote_view_open` to use that helper when the selected route is
  already checked out to the current browser/session.
- Removed the inline reused-launch JSON shape from `actions.rs`.
- Added handoff-module coverage for browser, session, route, display, and
  reason evidence in the reused-launch result.
- Updated P69 to record this launch-result vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Reused route-bound browser launch evidence now lives in the handoff module.
  P69 remains open for deeper end-to-end orchestration and Slice F live proof.

## Turn 93 | 2026-07-06

Scope: continue P69 Slice C by moving visible-window checkout command
finalization into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_checkout_command_with_visible_window_proof`.
- Rewired `remote_view_open` to finalize the route-checkout command through
  the handoff helper after visible-window proof.
- Removed the action-local checkout command mutation that attached readiness
  and display-content proof.
- Added handoff-module tests for checkout command finalization with and
  without display content.
- Updated P69 to record this checkout finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Visible-window proof enrichment of checkout commands now lives in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 92 | 2026-07-06

Scope: continue P69 Slice C by moving failure rollback cleanup payload
vocabulary into the handoff module.

Actions:

- Added handoff helpers for generic pending rollback cleanup, operator-visible
  failure cleanup, and final operator-visible failure cleanup payloads.
- Rewired `remote_view_open` tab, focus, visible-window proof, checkout,
  operator-visible, and final operator-visible failure branches to use the
  handoff cleanup payload helpers.
- Kept rollback execution and async browser cleanup in `actions.rs`.
- Added handoff-module tests for simple rollback cleanup and the two
  operator-visible proof cleanup surfaces.
- Updated P69 to record the failure-cleanup vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup JSON shapes for the route-bound handoff path now live in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 91 | 2026-07-06

Scope: continue P69 Slice C by moving plan-path acquisition begin/complete
adapters into the handoff module.

Actions:

- Added `begin_route_bound_handoff_plan_acquisition` to own the plan-path
  begin-acquisition call plus default control-input selection.
- Added `complete_route_bound_handoff_plan_acquisition` to restore a missing
  lease when needed and complete the acquisition in one handoff API.
- Rewired `remote_view_open` to call those handoff helpers while keeping
  timestamp generation in `actions.rs`.
- Removed the action-local begin, complete, and restore acquisition wrappers.
- Updated the lease rollback test to exercise the handoff begin helper.
- Updated P69 to record the acquisition helper consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- The route-bound plan path now enters and completes acquisition through named
  handoff-module APIs. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 90 | 2026-07-06

Scope: continue P69 Slice C by grouping route-bound plan artifacts behind the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPlan` and
  `route_bound_handoff_plan` to group the normalized route binding with launch,
  tab, and route-checkout command artifacts.
- Rewired `remote_view_open` to consume one handoff plan after acquisition-plan
  selection instead of normalizing the route binding and constructing command
  values locally.
- Removed the action-local route-binding normalization helper.
- Updated stale acquisition-pending readiness coverage to exercise the handoff
  plan path and added a handoff-module test for grouped plan artifacts.
- Updated P69 to record this plan-artifact consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound planning now has a named handoff-module API that returns the
  normalized route binding and command artifacts together. P69 remains open for
  deeper acquisition/finalization sequencing and Slice F live proof.

## Turn 89 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound command artifact
construction into the handoff module.

Actions:

- Added handoff-owned builders for route-bound launch, tab, focus, and
  route-checkout commands.
- Rewired `remote_view_open` to use the handoff command builders while keeping
  browser/service command execution in `actions.rs`.
- Removed the action-local route-bound command builders for launch, tab,
  focus, and checkout.
- Moved focus-command coverage into the handoff module and added coverage for
  launch/checkout route fields and tab default URL behavior.
- Updated P69 to record this command-artifact extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound command artifacts now have named handoff-module APIs. P69 remains
  open for deeper plan/acquire/finalize orchestration and Slice F live proof.

## Turn 88 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound browser-build proof
finalization into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_browser_build_proof` to own
  selected browser build, executable path, applied capability, and mismatch
  evidence for route-bound opened responses.
- Rewired `remote_view_open` to call the handoff helper before opened-response
  assembly.
- Removed the action-local browser-build proof helper and moved its mismatch
  regression coverage into the handoff module.
- Updated P69 to record this proof-finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml browser_build_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Browser-build proof finalization now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 87 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound final route-binding
derivation into the handoff module.

Actions:

- Added `remote_view_handoff::final_route_bound_handoff_route_binding` to own
  the merge of planned route binding, route checkout readback, and route-pool
  checkout readback.
- Rewired `remote_view_open` to use that handoff helper before final
  operator-visible proof and opened-response assembly.
- Removed the action-local final binding merge helper.
- Added handoff-module coverage for route and route-pool checkout readback
  overriding the planned binding, and kept the action-level stale-route proof
  coverage green.
- Updated P69 to record the finalization extraction.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_final_route_binding -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Final route binding derivation now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 86 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure rollback sequencing
behind the handoff module.

Actions:

- Added `remote_view_handoff::rollback_route_bound_handoff_failure` to restore
  a missing acquisition lease and roll route, display, route-pool, and browser
  display-allocation state back through one handoff API.
- Added `remote_view_handoff::complete_route_bound_handoff_failure_cleanup` to
  attach browser cleanup evidence to the rollback and produce the cleanup
  summary string.
- Rewired `remote_view_open` tab, focus, proof, checkout, and final-proof
  failure branches through one cleanup adapter. `actions.rs` still performs the
  async browser cleanup command, but no longer open-codes lease restoration,
  rollback mutation, cleanup attachment, and summary formatting in every branch.
- Added a repository-backed handoff-module test for restoring a missing lease,
  rolling pending state back to previous values, recording browser cleanup, and
  returning a parseable cleanup summary.
- Updated P69 to record this Slice C sequencing progress.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_cleanup -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Route-bound failure rollback and cleanup summary sequencing now has a named
  handoff-module interface. P69 remains open for deeper plan/acquire/finalize
  orchestration and Slice F live proof.

## Turn 85 | 2026-07-06

Scope: continue P69 Slice C by routing plain remote-headed `open` through the
shared acquisition-result surface.

Actions:

- Added a short-lived daemon response slot for launch-time shared-profile
  acquisition evidence.
- Reused `remote_view_handoff::shared_profile_acquisition_result` for plain
  `open`/`navigate` auto-launch when it attaches to a compatible retained
  same-profile browser and opens a tab there.
- Taught the subsequent navigation response to include `sharedAcquisition`
  with the selected retained owner browser/session, requested/planned profile,
  duplicate-process policy, and `routeHintSource: shared_profile_auto_launch`.
- Added focused Rust coverage for the plain-open owner evidence shape.
- Updated P69 to remove the plain remote-headed `open` acquisition-result gap.

Validation:

- `~/.local/bin/graphiti-runtime doctor`
- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_auto_launch_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Plain remote-headed `open` now participates in the same named
  `sharedAcquisition` response vocabulary as `remote_view_open` and HTTP/MCP
  `service_request` tab acquisition. P69 remains open for the full
  plan/acquire/finalize/rollback sequencing move and Slice F live proof.

## Turn 67 | 2026-07-06

Scope: execute P69 Slice A and the ordinary-open part of Slice B.

Actions:

- Added global `--browser-build` parsing and `clean_args` handling.
- Preserved explicit global launch-routing flags on plain `open`, `goto`, and
  `navigate` command payloads.
- Added a shared-profile auto-launch acquisition path that attaches to a
  compatible retained same-profile browser with a CDP endpoint, creates a fresh
  tab, and then lets the existing navigation handler load the requested URL.
- Updated the P69 plan, the `last30days` routing-failure note, CLI help,
  README, docs site commands page, and `skills/agent-browser/SKILL.md`.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_parse_global_browser_build_flag -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_navigate_preserves_explicit_global_launch_routing_flags -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/flags.rs cli/src/commands.rs cli/src/native/actions.rs cli/src/output.rs README.md docs/src/app/commands/page.mdx skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented for plain navigation commands.
- Slice B is partially implemented for ordinary `open`, `goto`, and `navigate`.
  HTTP/MCP `service_request` parity, public acquisition response fields,
  dashboard/client actionability, and live two-tab proof remain open P69 work.

## Turn 66 | 2026-07-06

Scope: write P69 for shared-profile routing and handoff deepening.

Actions:

- Reviewed the architecture review report, the `last30days` profile-routing
  failure note, the runtime-profile sharing plan, and P67/P68 profile identity
  follow-ups.
- Added
  `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
  to make plain `open` preserve explicit runtime identity, route compatible
  in-use profiles through retained-browser tab acquisition, deepen the
  route-bound handoff module, and align workspace inventory plus generated
  client contracts.
- Added the P69 roadmap entry so the new lane is discoverable from
  `ROADMAP.md`.

Validation run:

- Read-only policy, Graphiti, CodeGraph, roadmap, runbook, and source-note
  inspection only.

Result:

- P69 is open and ready for Slice A implementation.

## Turn 65 | 2026-06-27

Scope: implement P46 S10 harness and stop at the S10 retry lock.

Actions:

- Added S10 scenario metadata for a service-owned route-bound browser beside a
  zero-lease foreign CDP browser.
- Added a live S10 runner path that launches a foreign Chromium profile outside
  `~/.agent-browser`, captures authenticated dashboard inventory, and evaluates
  selected workspace action and route/display isolation.
- Ran two live S10 attempts from the installed binary lane. Both failed before
  S10 evaluation on dashboard inventory endpoint/auth issues, and both reset
  cleanly with zero active incidents.
- Repaired the harness to read `/api/sessions` and
  `/api/session-tabs?port=...` through the authenticated dashboard
  viewer-client session.
- Added P63 and locked P46 at S10 pending validation-backed retry clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`

Result:

- No-live checks pass after the authenticated inventory fix.
- Failed live artifacts:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z` and
  `/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z`.
- P46 is locked at S10 pending P63. Do not run another S10 retry until P63's
  green preflight authorizes exactly one retry.

## Turn 64 | 2026-06-27

Scope: complete P62 and clear P46 S9.

Actions:

- Repaired dashboard selected-target recovery so an explicitly selected live
  blank tab is preserved as the selected target, while missing or dead stale
  selections still recover to a live tab.
- Updated S9 viewer-client and evaluator checks to accept exact blank-target
  preservation or typed stale-target recovery before requiring final blank
  navigation.
- Rebuilt and installed the dashboard runtime with
  `pnpm publish:local-dashboard -- --skip-smoke --json`.
- Verified installed runtime convergence, then reran S9 from the installed
  binary authority.
- Marked P62 complete and advanced P46 to S10.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-dashboard-view-streams.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- packages/dashboard/src/components/workspace-remote-viewport.tsx scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-dashboard-view-streams.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`
- `agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- Installed runtime doctor passed with zero issues.
- S9 passed with artifact
  `/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z`.
- The pass proved exact initial blank-target selection, blank navigation to
  IANA, duplicate same-origin tab isolation, browser-window-visible route
  display, route-bound finalization, one default-profile browser row, and zero
  active incidents after reset-after.
- P46 is now in progress at S10.

## Turn 63 | 2026-06-27

Scope: implement P46 S9 stale target and duplicate tab stress, then record the
S9 lock.

Actions:

- Added S9 scenario metadata, live capture, evaluator checks, and no-live
  harness assertions.
- Added a narrow viewer-client stale selected-tab recovery option for the S9
  operator C blank-tab proof.
- Ran S9 live attempts from the explicit rebuilt-binary lane with reset-before
  and reset-after.
- Added P62 for the selected-target recovery follow-up.
- Updated P46 and the P46 execution note with the S9 lock.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`

Result:

- S9 did not pass. Corrected failure artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z`.
- The run proved stale blank-tab recovery notice and CLI navigation of the blank
  target, but the dashboard rewrote operator C back to duplicate target A when
  the harness re-requested the blank-tab dashboard URL.
- Reset-after reported zero sessions, zero browsers, zero tabs, and zero active
  incidents.
- P46 is locked at S9 pending P62. Do not run another S9 retry until P62
  records validation-backed retry authorization.

## Turn 62 | 2026-06-27

Scope: implement and clear P46 S8 display-access recovery.

Actions:

- Added P61 for the S8 display-access denial and recovery proof.
- Added S8 metadata, live capture, evaluator checks, and no-live harness
  assertions.
- Used a temporary `timeout` shim in `PATH` to safely simulate display-access
  denial without mutating host X11 permissions.
- Reran the same route-bound open with normal display access as the recovery
  proof.
- Updated P46 and the P46 execution note with S8 clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js`
- `node scripts/run-p46-stress-scenario.js --scenario s8 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S8 passed with artifact
  `/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z`.
- The pass proved typed `display_access_grant_failed` denial before browser
  launch, cleanup rollback of display allocation, remote-view route, and
  route-pool entry, no retained denied-profile browser row, terminal-free route
  displays after denial, successful recovery open with
  `displayAccessGrant.state: already_ready`, and zero active incidents after
  reset-after.
- P46 is now in progress at S9.

## Turn 61 | 2026-06-27

Scope: implement and clear P46 S7 route-pool exhaustion.

Actions:

- Added P60 for the S7 route-capacity diagnostic repair.
- Added S7 metadata, live capture, evaluator checks, and no-live harness
  assertions for third-demand route-pool exhaustion and retry after release.
- Tightened `plan_remote_view_acquisition` so unpinned route-bound demand that
  lands on a checked-out pool display owned by another session reports
  `route_pool_exhausted`.
- Rebuilt `./cli/target/debug/agent-browser`, restarted the stale default
  daemon, and ran the rebuilt-binary S7 verifier.
- Updated P46 and the P46 execution note with the S7 clearance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reports_route_pool_exhausted -- --nocapture`
- `node scripts/test-p47-scenario-harness.js`
- `cargo build --manifest-path cli/Cargo.toml`
- `node scripts/run-p46-stress-scenario.js --scenario s7 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S7 passed with artifact
  `/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z`.
- The pass proved both route-pool entries occupied, third demand failing with
  `route_pool_exhausted`, no retained profile C browser row after the failed
  demand, no terminal fallback on occupied displays, successful profile C retry
  after releasing profile A, and zero active incidents after reset-after.
- P46 is now in progress at S8.

## Turn 60 | 2026-06-27

Scope: clear P46 S6 and advance to S7.

Actions:

- Ran the P55-authorized S6 retry from the explicit rebuilt-binary lane.
- Manually closed the two retained S6 profile sessions after reset-after missed
  them.
- Added P56 to reconnect the external viewer-client CDP websocket after swapped
  dashboard navigation.
- Added a 32 MiB `spawnSync` output buffer to the P46 runner so large
  `service status` payloads remain parseable during reset.
- Added no-live coverage for swapped reconnect artifacts and reset buffer
  hardening.
- Ran one P56-authorized S6 retry after green preflight.
- Added P57 to require DevTools target-discovery evidence before another S6
  retry.
- Added P58 to wait for the swapped DevTools page URL before reconnecting.
- Added P59 to use same-origin `history.pushState` plus `popstate` for
  dashboard workspace swaps.
- Ran the P59-authorized S6 retry after green preflight.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`

Result:

- P55 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z`.
- The failure moved to post-swap state polling:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- P56 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z`.
- The failure moved to reconnect command enablement:
  `CDP command Page.enable timed out after 30000ms`.
- P56 reset-after closed both retained S6 profile sessions, and final readback
  showed zero sessions, zero browsers, zero tabs, zero active incidents, and
  both route-pool entries available.
- P57 retry showed the selected DevTools page URL still pointed at profile A
  after requesting profile B.
- P58 retry showed `location.assign()` did not change the selected DevTools
  page URL for the same-origin dashboard workspace swap.
- P59 retry passed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z`.
- S6 pass proved swapped selected-browser readback for both operators, working
  swapped refresh controls, swapped screenshots, distinct route-bound profile
  checkouts, profile B readiness after profile A closed, and clean reset-after.
- P46 is now in progress at S7.

## Turn 59 | 2026-06-27

Scope: repair P46 S5 viewer-client port allocation, pass S5, and start S6.

Actions:

- Added P54 for the S5 viewer-client DevTools port collision that locked P46
  after S5 attempt 2.
- Changed the external dashboard viewer-client launch path to use Chromium
  dynamic DevTools allocation with `--remote-debugging-port=0` by default.
- Added `DevToolsActivePort` readback before viewer-client `/json/version` and
  `/json` calls.
- Kept explicit viewer-client DevTools port overrides only for diagnostics.
- Extended the P47 viewer-client no-live test to cover dynamic launch metadata,
  `DevToolsActivePort` parsing, override validation, and absence of the old
  random fixed-port selector.
- Updated P46 and the P46 execution note with the P54 repair and S5 pass.
- Added S6 metadata, runner support, and no-live coverage for two-profile
  cross-observation with swapped dashboard selection.
- Added a CDP command timeout to the viewer-client adapter after S6 attempt 1
  hung before writing swapped selection artifacts.
- Updated reset handling to close retained browser rows from `activeSessionIds`
  and `session:<name>` browser IDs when session rows are missing.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/test-p47-viewer-client-separation.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `pnpm test:p47-viewer-client-separation`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/test-p47-viewer-client-separation.js scripts/run-p46-stress-scenario.js scripts/lib/p46-scenario-harness.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S5 passed with artifact
  `/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z`.
- The pass proved profile A on route `guacamole:3` and display `:13`, profile
  B on route `guacamole:4` and display `:14`, finalized route-bound checkouts
  for both profiles, working refresh controls for both external dashboard
  viewer clients, browser-visible route displays for both routes, and profile B
  staying ready after profile A closed.
- Reset-after and final readback showed zero sessions, zero browsers, zero
  tabs, and zero active incidents.
- S6 attempt 1 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z` proved both profile
  browsers and both initial dashboard viewers became ready, but the run hung
  before swapped dashboard selection artifacts were written.
- S6 attempt 2 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z` failed in bounded form
  with `CDP command Page.navigate timed out after 30000ms` during the swapped
  dashboard selection step.
- Manual cleanup closed
  `p46-s6-profile-a-2026-06-27T19-56-29-450Z` and
  `p46-s6-profile-b-2026-06-27T19-56-29-450Z`; final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, route-pool entries
  available, and idle displays.
- P54 is complete. P46 is locked at S6 by the two-consecutive-failure rule.

## Turn 57 | 2026-06-27

Scope: diagnose the P46 S4 lock and create the S4 topology follow-up plan.

Actions:

- Re-read P46, the P46 execution note, repo validation and memory policies, and
  the S4 attempt 2 artifact.
- Confirmed Graphiti was healthy, but the focused read did not add S4-specific
  authority beyond repo files and artifacts.
- Classified S4 attempt 2 as a same-profile topology and typed-blocker gap:
  window A reached `operatorVisible.state=ready` on `p46-s4-profile`, route A,
  and display `:13`; window B then tried the same runtime profile on route B,
  timed out, and left route-bound finalization cleanup evidence.
- Added P53 to decide and implement the supported S4 topology before any live
  S4 retry.
- Implemented the P53 Goal 1 no-live S4 topology guard. The S4 runner now
  writes `s4-topology-preflight.json` and stops with
  `same_profile_multi_process_unsupported` before launching window B for the
  current one-profile, two-session, two-route-pool-entry shape.
- Selected the P53 Goal 2 topology: one retained remote-headed browser process,
  one route lease, one runtime profile, and two top-level same-profile windows.
- Added `agent-browser window new [url] --same-profile` and rewired S4 window B
  to use that same-session window target instead of a second route-bound
  browser process.
- Switched S4 to a unique `p46-s4-window-<timestamp>` daemon session per run
  after the first P53-shaped retry reused a stale named session and exercised
  the older window handler.
- Updated P46 and the P46 execution note to keep the lock in place pending P53.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0053-2026-06-27-s4-single-profile-window-topology-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- Read-only service status: zero sessions, zero browsers, zero tabs, and zero
  active incidents.
- Read-only install doctor: success, zero issues, one matching default socket
  listener, and zero deleted default-socket listeners.

Result:

- P46 remains locked at S4 by its two-failure rule.
- No live S4 retry was run.

## Turn 56 | 2026-06-23

Scope: complete the P44 Slice H dashboard inventory class inspector and local
publish smoke.

Actions:

- Added `WorkspaceInventoryClass` to the shared dashboard workspace node model.
- Classified service-owned controllable browsers, service-owned view-only
  browsers, service-owned diagnostic browsers, detected non-owned browsers,
  viewer clients, retained history, service-owned sessions, and profile action
  rows.
- Exposed the inventory class through selected-workspace context, diagnostic
  bundles, and evidence rows so inspector, chat, console, and automation
  consumers do not infer ownership from URL shape.
- Added the selected Workspace inspector Class row backed by the canonical
  `WorkspaceInventoryClass` value.
- Published the dashboard runtime locally and ran the full local dashboard
  smoke against `/home/ecochran76/.local/bin/agent-browser`.
- Updated README, dashboard docs, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm publish:local-dashboard -- --expect-marker service-owned-controllable-browser --skip-browser --json`
- `pnpm smoke:local-dashboard-runtime -- --expect-marker service-owned-controllable-browser --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --json`
- `agent-browser install doctor --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- Focused dashboard workspace model, selected workspace, chat-packet, console,
  view-stream, navigator, inspector action, docs, dashboard build, local
  publish, runtime smoke, skill-sync, and hygiene checks passed.
- Local publish restarted `agent-browser-dashboard.service` and installed
  executable SHA
  `6c7c9b879c1b564130fb74e4d2abec7502252033be14e66586c20477e7762649`
  with dashboard bundle SHA
  `10177dc55ce0a76f29fbcce7ede2acf8e7b5cbb896d83987ddff2e2aaa193967`.
- Runtime smoke loaded `http://127.0.0.1:4848/`, found
  `service-owned-controllable-browser`, and confirmed the workspace pane in
  browser session `local-dashboard-runtime-smoke-1606766`.
- Closing stale daemon session `default` brought install doctor runtime
  convergence back to `converged` with stale daemon count `0`.
- Slice H dashboard inventory refactor is complete. P44 remains open for the
  installed privileged helper refresh and the later Slice I and Slice J work.
  Doctor still reports `remote_view_route_desktop_helper_stale`, which needs
  `agent-browser install --with-remote-view-privileges` from an interactive
  sudo shell, plus readiness-impacting stale resource candidates.

## Turn 55 | 2026-06-23

Scope: start P44 Slice H dashboard inventory actionability.

Actions:

- Moved non-ready RDP gateway operator-visible proof rows out of the active
  workspace control group and into `needs-attention`.
- Kept View and Control disabled with route-proof reasons while enabling Repair
  for actionable route-proof failures.
- Extended dashboard workspace fixture coverage for terminal-only, unbound,
  missing-proof, wrong-tab, unavailable-route, missing-CDP-target, and
  stale-route rows, while preserving active controllable service-owned rows and
  detected non-owned CDP rows.
- Updated README, dashboard docs, commands docs, Plan 0044, `ROADMAP.md`, and
  repo plus installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-view-streams`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-workspace-navigator`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- The focused dashboard workspace, adjacent view-stream, selected workspace,
  docs, dashboard build, skill-sync, and hygiene checks passed.
- P44 remains open. The installed helper refresh still needs interactive sudo,
  and Slice H still needs inspector and manual publish smoke coverage before it
  can be called complete.

## Turn 54 | 2026-06-23

Scope: start P44 Slice G fast route preflight.

Actions:

- Added `fastPreflight` to the existing
  `service_remote_view_route_preflight` no-launch action.
- The response now reports `ready`, `partial`, `stale`, or `blocked` from
  component evidence for acquisition planning, Guacamole route URL shape,
  retained Guacamole web/login/permission and RDP TCP readiness, display access,
  and route desktop state.
- Added HTTP `GET /api/service/remote-view/route-preflight`, MCP
  `service_remote_view_route_preflight`, and
  `getServiceRemoteViewRoutePreflight()` as first-class no-launch convenience
  surfaces over the same fast preflight response.
- Bounded the shared display-access probe with `timeout --kill-after=1 2` so
  fake or unreachable route displays cannot hang preflight or route-open display
  access checks.
- Updated README, CLI help, service-mode docs, service-request schema
  description, Plan 0044, `ROADMAP.md`, and repo plus installed skill guidance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_route_and_lease_actions_mutate_service_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:remote-view-route-preflight-timing`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

Result:

- Focused route preflight and route-action Rust tests, clippy, docs,
  service-client, API/MCP parity, skill-sync, and hygiene checks passed.
- P44 remains open. Slice G now has HTTP/MCP/client convenience surfaces and a
  bounded timing smoke; remaining live boundaries are still the installed helper
  refresh and guarded route-bound repeat-open smoke.

## Turn 53 | 2026-06-23

Scope: continue P44 Slice F route-bound repeat-open target convergence.

Actions:

- Routed `remote_view_open` tab acquisition through same-origin live target reuse
  before opening a new tab.
- Added `tabAcquisitionDecision` and `duplicateTargetCleanup` evidence to
  successful route-bound tab acquisition results.
- Extended the remote-view-open live smoke to assert that CLI first, CLI repeat,
  and HTTP helper opens converge to one active intended target in service state.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance for the repeat-open convergence contract.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_reusable_live_target -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `agent-browser install doctor --json`

Result:

- Static, docs, and focused Rust checks passed.
- The live route-bound repeat-open smoke is implemented but not run in this
  turn because install doctor still reports
  `remote_view_route_desktop_helper_stale`; refreshing the installed helper
  requires an interactive sudo boundary.
- P44 remains open. Slice D still needs the interactive helper refresh and cold
  route desktop proof; Slice F still needs the guarded live smoke run after that
  refresh.

## Turn 52 | 2026-06-23

Scope: continue P44 Slice F dashboard stale-target URL recovery.

Actions:

- Updated the workspace remote viewport to treat missing, closed, blank, or
  target-shaped stale `tab=target:*` URL selections as recoverable stale target
  identity for the selected browser.
- Replaced stale workspace tab URL selections with the current live service tab
  before control mode queues `view_focus`.
- Preserved the existing `stale_target_recovered` UX and readiness vocabulary
  while adding a focused recovery message that names the stale selection and
  current live tab.
- Added dashboard view-stream fixture assertions for stale URL replacement and
  target-shaped stale tab recovery.
- Updated README, docs site, Plan 0044, `ROADMAP.md`, and repo skill guidance.

Validation run:

- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs a route-bound repeat-open live proof
  that verifies one intended active target.
- Slice D remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 51 | 2026-06-23

Scope: start P44 Slice F tab acquisition cleanup with a duplicate-replacement
refresh policy.

Actions:

- Added `replace_duplicates` to `tab_handle_refresh` repair-policy validation in
  the daemon, HTTP ingress, MCP ingress, service schema, generated client
  template, and service-client helper.
- Implemented best-effort compatible duplicate cleanup for `replace_duplicates`.
  The refresh path reuses or opens one compatible target, preserves that selected
  target, closes other compatible live targets when possible, and returns
  `duplicateTargetCleanup` evidence.
- Added Rust coverage for compatible duplicate target selection and client
  coverage proving the new policy is accepted and forwarded.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm generate:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml tab_handle_refresh -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs dashboard stale-target URL recovery and a
  route-bound repeat-open live proof that verifies one intended active target.
- Slice D also remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 21 | 2026-06-23

Scope: start P44 Slice E by returning structured route-bound operator-visible
proof components.

Actions:

- Extended successful `remote_view_open` `operatorVisible` output with selected
  target evidence plus route, display, browser, tab, stream, and Guacamole
  component states while preserving the existing `proof` field.
- Updated `summarizeServiceRemoteViewOpenProof()` to prefer
  `operatorVisible.target` and `operatorVisible.components` before falling back
  to the tab result.
- Updated CLI help, README, docs site, and repo plus installed skill guidance
  for the richer `operatorVisible` proof shape.
- Added selected-target URL readiness so a visible browser with the wrong
  selected tab reports `operatorVisible.state=wrong_tab`, with
  `components.display.state=ready` and `components.tab.state=wrong_tab`.
- Added Guacamole route availability to the same proof vocabulary so ready
  display and tab evidence with a missing or non-ready operator route reports
  `operatorVisible.state=guacamole_route_unavailable`.
- Added CDP target availability to the selected-tab proof so URL-bearing tab
  results without a CDP `targetId` report
  `operatorVisible.state=cdp_target_unavailable`.
- Added retained route metadata to the route proof so stale or mismatched
  route-pool allocation records report
  `operatorVisible.state=stale_route_record`.
- Added dashboard readiness fixture coverage so workspace rows preserve
  `wrong_tab`, `guacamole_route_unavailable`, `cdp_target_unavailable`, and
  `stale_route_record` from structured stream readiness while keeping View and
  Control disabled with state-specific reasons.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_operator_visible_reports_ready_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_operator_visible -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `node --no-warnings --experimental-strip-types scripts/test-dashboard-workspace-nodes.js`
- `pnpm test:dashboard-view-streams`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Focused Rust proof coverage, the full `remote_view_open` filter, service
  model and CDP stream tests, clippy, service-client helpers, API/MCP parity,
  route-confusion gates, docs build, skill sync, diff hygiene, and the live CDP
  tab streaming smoke passed. Slice E remains open for failure-case proof
  vocabulary and dashboard readiness fixture coverage.

## Turn 20 | 2026-06-23

Scope: continue P44 Slice D by making stale installed route desktop helpers
visible in fast doctor surfaces.

Actions:

- Added `helperDesktopSession` inspection to `agent-browser install doctor` and
  `agent-browser doctor remote-view`; both parse the installed privileged
  helper's `.xsession` heredoc and classify terminal-first, missing, unreadable,
  incomplete, or browser-control-ready templates.
- Added `remote_view_route_desktop_helper_stale` issue reporting to both doctor
  surfaces when a root-owned helper exists but still writes a terminal-first
  route desktop.
- Added text output for route desktop helper state and focused Rust coverage for
  terminal-first rejection, idle Openbox acceptance, and stale-helper issue
  generation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_helper_desktop_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml reports_stale_remote_view_helper_desktop_template -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo build --manifest-path cli/Cargo.toml`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cli/target/debug/agent-browser --json install doctor`
- `cli/target/debug/agent-browser --json doctor remote-view`

Result:

- Both focused Rust test groups passed, route-confusion gates passed, debug build
  passed, and clippy passed.
- The rebuilt debug `install doctor` and `doctor remote-view` readbacks both
  reported `helperDesktopSession.state=terminal_first_template`,
  `terminalStartupDetected=true`, and issue code
  `remote_view_route_desktop_helper_stale` for the currently installed helper.
- The live route proof remains blocked on refreshing the root-owned helper from
  an interactive sudo shell and then starting a cold route session.

## Turn 19 | 2026-06-21

Scope: repair the Plan 0039 audit findings after closeout review.

Actions:

- Made `agent-browser remote-view open` accept the documented
  `--browser-build stealthcdp_chromium` and `--provider rdp_gateway` flags.
- Added post-launch failure cleanup to `remote_view_open`: tab open, focus, visible-window proof, or checkout failures now clean up before returning the typed error. New
  browser launches close the browser; reused retained browsers preserve the
  browser process and close only the opened tab when possible.
- Updated CLI help, README, docs command page, repo skill guidance, Plan 0039,
  and P16 roadmap text for the accepted flags and cleanup boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_builds_route_bound_service_action -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- The focused Plan 0039 parser and cleanup tests passed, the non-live Rust,
  client, docs, and dashboard gates above passed, and the installed skill copy
  matches the repo skill.
- The direct documented dry-run command
  `agent-browser remote-view open --runtime-profile stealthcdp-default
  --browser-build stealthcdp_chromium --provider rdp_gateway --url
  https://www.linkedin.com/ --dry-run` returned `success=true` and
  `status=planned`.
- The repo-wide planning audit still reports older unrelated drift, but the
  Plan 0039 row remains clean: `state=CLOSED`, `current_state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 18 | 2026-06-21

Scope: close Plan 0039 by making the route-specific `remote_view_open` lane the
documented default and proving it on the installed binary.

Actions:

- Added prelaunch route-display access repair to `remote_view_open`: it probes
  the selected route display, invokes the installed privileged helper when
  access is missing, and fails with typed display-access errors if access still
  cannot be proven.
- Fixed route binding selection so checked-out retained routes reuse their
  existing display allocation when no inline route material overrides them.
- Updated README, CLI help, docs site, service-request contract description,
  repo skill, installed skill, Plan 0039, ROADMAP, and downstream handoff note
  `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.
- Rebuilt and installed binary SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad` into
  `~/.local/bin/agent-browser`, `bin/agent-browser-linux-x64`, and the pnpm
  package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm test:service-client`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `agent-browser install doctor --json` passed with no issues and aligned SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
- `agent-browser doctor remote-view --json` reported `status=ready`,
  `remoteControl.status=ready`, `remoteControl.routeId=guacamole:3`,
  `remoteControl.displayName=:11`, and `manyToMany.status=ready`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
- `git diff --check` passed.
- The repo-wide planning audit still reports older unrelated planning-contract
  drift, but the Plan 0039 row is clean: `state=CLOSED`,
  `current_state_ok=true`, `wired_in_roadmap=true`, and
  `wired_in_runbook=true`.
- Plan 0039 and P16 are closed.

## Turn 17 | 2026-06-20

Scope: continue Plan 0039 remote-control ready command hardening after the
route-specific Guacamole/RDP lane exposed stale retained route state.

Actions:

- Repaired the retained service route pool from the current route-pool
  readiness report after backing up
  `~/.agent-browser/service/state.json.pre-route-pool-refresh-2026-06-21T00-56-42-211Z`.
- Changed `remote_view_open` route binding to prefer supplied/current
  route-pool identity over stale retained route id and display allocation
  state.
- Made requested route-pool entry id authoritative for allocation lookup and
  allowed top-level `readiness.state=ready` route-pool entries to be used even
  when informational nested components are not ready.
- Updated the remote-view open live smoke to use the selected route entry's
  display name and display isolation for CLI, HTTP, state, and X11 checks.
- Rebuilt and installed the local binary into `~/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the pnpm global package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/smoke-rdp-guac-route-pool-readiness.js`
- `node --check scripts/open-rdp-guac-route-displays.js`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `git diff --check`

Result:

- Route-specific `remote-view open` dry-run resolves `guacamole-rdp-a` to
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, and `manyToMany.status=ready`.
- Plan 0039 remains open only for Slice F documentation and downstream
  handoff closeout.

## Turn 1 | 2026-05-26

Scope: repair the planning contract after adopting Graphiti and CodeGraph
policy modules.

Actions:

- Added top-level `ROADMAP.md` as the planning index.
- Added top-level `RUNBOOK.md` as the dated execution log.
- Wired `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
  into both planning authorities.
- Changed plan 0001's deterministic plan state to `CLOSED` while preserving
  its `VALIDATED` outcome.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the planning-contract repair.

## Turn 2 | 2026-05-27

Scope: create the Guacamole remote-view routing hardening lane after roadmap
alignment review.

Actions:

- Added `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`.
- Added P02 to `ROADMAP.md`.
- Kept P01 closed and made the hardcoded Guacamole route, metadata-only
  `view_takeover`, and external-open race the explicit P02 scope.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the P02 planning turn.

## Turn 3 | 2026-05-27

Scope: implement the first Guacamole route hardening slices.

Actions:

- Added `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`.
- Added service-owned `ViewStream` route metadata: `frameUrl`,
  `externalUrl`, `routeId`, `connectionId`, `connectionName`, and
  `routeSource`.
- Removed production Guacamole client-hash repair from Rust service status
  handling and the dashboard workspace viewport.
- Changed dashboard external open to await `view_takeover` acceptance before
  opening `externalUrl`.
- Changed `view_takeover` to return typed acceptance metadata and persist a
  `viewer_takeover_requested` service event with `viewerLeaseId` and route
  details.
- Updated README, CLI help, docs site pages, service contracts, generated
  observability client, harness artifacts, and the repo plus installed
  `agent-browser` skill.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_headed_view_stream -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml guacamole -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml view_takeover -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_events -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_remote_headed_launch_env_hints -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_daemon_env_forwards_keychain_settings -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-inspector-actions`
- `node --check scripts/smoke-remote-headed-utils.js`
- `node --check scripts/test-rdp-guac-browser-switch-live.js`
- `node --check scripts/test-rdp-guac-viewer-transfer-live.js`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-viewer-transfer-live`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-browser-switch-live`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Local source and contract validation passed.
- Live readiness, viewer-transfer, and browser-switch validation passed for
  the configured shared Guacamole route.
- Viewer-transfer artifacts:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-27T19-40-36-319Z`
- Browser-switch artifacts:
  `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-27T19-41-29-855Z`

## Turn 4 | 2026-05-29

Scope: refactor the P05 handoff after maintainer clarification that the
Guacamole/RDP campaign is not ready for a formal release.

Actions:

- Reframed P05 as a validated installed-runtime checkpoint instead of a release
  preparation lane.
- Replaced the P05 plan with
  `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`.
- Added P06 in
  `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`.
- Removed the public docs changelog `v0.27.0` entry and kept current work under
  `## Unreleased` in `CHANGELOG.md`.
- Kept `CHANGELOG.md` release markers around the latest published `0.26.1`
  release entry.
- Changed `.github/workflows/release.yml` to manual dispatch only so ordinary
  pushes to `main` cannot publish checkpoint work as a GitHub release.
- Updated `AGENTS.md` and `ROADMAP.md` with the formal release boundary:
  release only after the hardened many-to-many Guacamole/RDP operational
  milestone, including one-time-sudo install and fully diagnostic doctors.

Validation run:

- `git diff --check`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm version:sync`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser --version`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Checks passed. The installed runtime reports `agent-browser 0.27.0`, install
  doctor is successful with matching installed, workspace, and pnpm package
  binary checksum
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`, and
  remote-view doctor reports route pool, route displays, display access,
  privileged helper, and simultaneous viewing readiness with
  `requiresInteractiveSudo=false`.

## Turn 5 | 2026-05-29

Scope: execute and refactor the first P06 slice after auditing the installed
checkpoint against the productization issues from P05.

Actions:

- Added install-doctor remote-view privilege readiness fields for helper,
  sudoers, group, membership, helper check, nested issues, and
  `requiresInteractiveSudo`.
- Added remote-view doctor top-level issue codes, remediations, viewer browser
  and OCR prerequisites, install drift propagation, sudoers readiness, and
  many-to-many prerequisite status.
- Changed the many-to-many live harness to prefer installed `agent-browser`,
  hydrate route-pool and route-display environment from remote-view doctor
  output, auto-discover common viewer browsers, and fail public Guacamole route
  URLs with `non_embeddable_guacamole_url`.
- Updated README, CLI help, docs site pages, the repo skill guidance, the P06
  plan, the roadmap, and the P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- Installed doctor and remote-view doctor passed with no issues. The installed
  runtime checksum is
  `1b67077ccdb5e80d8667d3bcc8327e9c2a1a8521417c25280f71d059bc3b1694`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` precondition diagnostic.
- The local embeddable Guacamole many-to-many gate passed from the installed
  command with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.
- P06 remains open for clean-machine first-install sudo proof and the
  install-doctor service-readiness ownership decision.

## Turn 6 | 2026-05-29

Scope: continue P06 by resolving the remaining install-doctor service
ownership decision and strengthening the already-provisioned privilege
installer re-run contract.

Actions:

- Added `data.service` to `agent-browser install doctor --json` using an
  isolated no-launch service-status probe.
- Made install doctor fail with `service_status_not_ready` when the no-launch
  service probe does not report ready.
- Changed `scripts/install-agent-browser-privileges.sh --apply` to exit before
  privileged changes when the helper source matches the installed helper, the
  sudoers file exists, the operator is in the `agent-browser` group, and
  `sudo -n <helper> check` succeeds.
- Updated CLI help, README, docs site installation/service-mode pages, skill
  guidance, the P06 plan, roadmap, and validation note.

Validation run:

- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `bash -n scripts/install-agent-browser-privileges.sh`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --dry-run`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --apply`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The source-build install doctor showed the new service probe as ready and
  no-launch, while still correctly reporting source/install binary drift.
- The already-provisioned helper installer re-run exited with "already ready"
  and made no privileged changes.
- The rebuilt installed runtime checksum is
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- Installed doctor and remote-view doctor passed with no issues, and install
  doctor reports `data.service.ready=true` plus `data.service.noLaunch=true`.
- P06 remains open for clean-host or equivalent reset-fixture proof that first
  install uses one clear sudo authorization boundary.

## Turn 7 | 2026-05-29

Scope: finish P06 by proving the first-install sudo boundary with an equivalent
clean reset fixture, validating route-pool restart durability, and running the
final installed gates.

Actions:

- Added `pnpm test:install-privileges-clean-fixture`, which runs the privilege
  installer against fake `sudo`, `getent`, `id`, `groupadd`, `usermod`, and
  `visudo` under a temp install root.
- Reordered the Linux install path so
  `agent-browser install --with-deps --with-remote-view-privileges` runs
  remote-view privilege setup before dependency installation.
- Added a Rust guard that keeps remote-view privilege setup before Linux
  dependency installation.
- Updated README, docs site installation guidance, skill guidance, P06 plan,
  roadmap, and P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `pnpm test:install-privileges-clean-fixture`
- `cargo test --manifest-path cli/Cargo.toml install_orders_remote_view_privileges_before_linux_deps -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `docker restart agent-browser-guacamole agent-browser-guacd && node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `pnpm sync:rdp-guac-existing-user-route-pool`
- `pnpm grant:rdp-route-display-access -- --apply`
- `agent-browser --json get title`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- The clean-fixture smoke proved first apply uses exactly one explicit
  `sudo -v` boundary and second apply does not add another prompt boundary or
  repeat privileged install commands.
- Installed doctor and remote-view doctor passed with no issues. The final
  P06 installed runtime checksum is
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- Route-pool readiness survived Guacamole web and guacd restarts.
- Route sync and route-display access grant reruns passed without interactive
  sudo.
- Default command attach passed.
- The local embeddable Guacamole many-to-many gate passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-39-55-085Z`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` diagnostic and artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-40-34-292Z`.
- P06 is closed. Formal release work remains a separate lane.

## Turn 8 | 2026-05-29

Scope: open the formal release lane now that P06 closed the Guacamole/RDP
productization blocker.

Actions:

- Created
  `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`.
- Moved `CHANGELOG.md` release extraction markers from `0.26.1` to `0.27.0`.
- Added the public docs changelog entry for `v0.27.0` dated May 29, 2026.
- Added P07 to `ROADMAP.md`.
- Added release-preparation validation note
  `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.

Validation run:

- `git log v0.26.1..HEAD --format='%an <%ae>' | sort -u`
- `pnpm version:sync`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Local release-preparation validation passed. The installed runtime checksum
  remains
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- P07 remains open for release PR merge, release workflow dry run, real
  release workflow run, and GitHub release asset verification.

## Turn 9 | 2026-05-29

Scope: respond to the first manual `Release` workflow dry-run failure.

Actions:

- Ran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed release-state precheck passed.
- Diagnosed the platform build failures as a Rust cfg leak in
  `cli/src/native/cdp/chrome.rs`.
- Kept the private remote-headed virtual-display fallback inside the Linux cfg
  block so non-Linux targets do not reference Linux-only helpers.
- Added
  `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml private_remote_display -- --test-threads=1`
- `cargo check --manifest-path cli/Cargo.toml --target x86_64-pc-windows-gnu`

Result:

- Format, clippy, and the focused private remote-display unit test passed.
- The local Windows cross-target check advanced past the previous missing
  symbols, then stopped because this workstation lacks
  `x86_64-w64-mingw32-gcc` for the `ring` build script.
- The release workflow dry run must be retried after this fix lands on `main`.

## Turn 10 | 2026-05-29

Scope: respond to the second manual `Release` workflow dry-run failure.

Actions:

- Reran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed Windows x64, macOS x64, and macOS ARM64 passed after the cfg fix.
- Diagnosed Linux target failures as release-time `-lX11` linking from the
  browser-focus helper.
- Changed the Linux X11 focus helper to load `libX11` dynamically with
  `dlopen` and `dlsym` at runtime instead of statically linking X11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml browser -- --test-threads=1`
- `git diff --check`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `rg -n "#\\[link\\(name = \\\"X11\\\"\\)|-lX11" cli/src`

Result:

- Local validation passed.
- No static X11 link remains in `cli/src`.
- The local machine does not have `cargo-zigbuild`, so the release workflow
  dry run must be retried after this fix lands on `main`.

## Turn 11 | 2026-05-29

Scope: publish and verify the formal `v0.27.0` GitHub release.

Actions:

- Reran the manual `Release` workflow with `dry_run=true`.
- Ran the manual `Release` workflow with `dry_run=false` after the dry run
  passed.
- Verified the public GitHub release and asset list.
- Closed P07 in the roadmap and plan surfaces.

Validation run:

- `gh run view 26648621169 --json conclusion,url,headSha`
- `gh run view 26649196974 --json conclusion,url,headSha`
- `gh release view v0.27.0 --json tagName,name,url,isDraft,isPrerelease,assets,targetCommitish`
- `git fetch --tags origin`
- `git rev-list -n1 v0.27.0`
- `git rev-parse origin/main`

Result:

- Dry run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26648621169`
- Real release run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26649196974`
- Release URL:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.27.0`
- Release commit and `origin/main` both resolve to
  `17a284f8624e6108473970e2ec2b380debf9f7ac`.
- The release is not a draft, is not a prerelease, and has seven assets:
  `agent-browser-darwin-arm64`, `agent-browser-darwin-x64`,
  `agent-browser-linux-arm64`, `agent-browser-linux-musl-arm64`,
  `agent-browser-linux-musl-x64`, `agent-browser-linux-x64`, and
  `agent-browser-win32-x64.exe`.

## Turn 12 | 2026-05-29

Scope: repair stale planning-audit residue after the `v0.27.0` release.

Actions:

- Normalized historical runbook headings to the deterministic
  `## Turn N | YYYY-MM-DD` format.
- Changed P02 plan state from `VALIDATED` to deterministic `CLOSED` while
  preserving `Outcome: VALIDATED`.
- Changed P03 plan state from `COMPLETE` to deterministic `CLOSED` while
  preserving `Outcome: COMPLETE`.
- Wired the existing P03 and P04 plan filenames into this runbook:
  `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md` and
  `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed. The planning audit now reports `ok: true`, no problems,
  no open roadmap lanes, deterministic state for every plan, and runbook plus
  roadmap wiring for every plan.

## Turn 13 | 2026-05-30

Scope: open the CDP tab streaming lane for non-remote browsers.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior CDP streaming
  context.
- Inspected the existing CDP stream server, stream WebSocket, service
  view-stream model, action-derived view streams, dashboard view-stream
  rendering, roadmap, and runbook surfaces.
- Added
  `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`.
- Added P08 to `ROADMAP.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Planning audit passed with `ok: true`, no problems, and P08 wired through the
  roadmap, runbook, and open plan file.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the documentation-only planning slice.

## Turn 14 | 2026-06-04

Scope: open a resource monitor and garbage collector lane after live
agent-browser resource pressure cleanup.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior resource
  cleanup and service lifecycle context.
- Confirmed the related retained orphan profile cleanup plan exists, but it
  covers service-state/profile metadata rather than live OS process pressure.
- Added
  `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`.
- Added P13 to `ROADMAP.md` with the dry-run-first resource monitor and GC
  recommendation.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- P13 is open for Slice A and Slice B: read-only resource inventory plus
  conservative stale classification before any apply-mode garbage collection.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` included the pre-existing dirty
  dashboard files in its recommendation set, so it selected dashboard checks in
  addition to the documentation-only change.
- The planning audit still fails due to pre-existing roadmap/runbook drift for
  older plans, but the new P13 plan is wired in both `ROADMAP.md` and
  `RUNBOOK.md`.

## Turn 15 | 2026-06-05

Scope: open and start the minimal runtime-profile reuse lane after Plan 0026
closed the resource-monitor and GC cleanup surface.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for profile reuse,
  service queue, lease, and access-plan context.
- Added
  `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`.
- Updated P13 in `ROADMAP.md` so Plan 0026 is the closed cleanup surface and
  Plan 0027 is the prevention surface.

Current target:

- Plan 0027 Slice A: add a read-only access-plan `profileReuse` advisory that
  recommends `reuse_existing_browser`, `wait_for_profile_lease`, or
  `launch_new_browser` before any launch mutates runtime state.

## Turn 16 | 2026-06-13

Scope: write an implementation handoff note for AuraCall-driven browser
service feature requests.

Actions:

- Ran Graphiti discovery against `agent_browser_main` and verified the local
  Graphiti runtime was healthy.
- Reviewed the existing access-plan service-request handoff note and the
  service request/client contract surfaces.
- Added
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`.
- Patched the note so AuraCall source paths are explicitly relative to the
  sibling `../auracall` repository.

Validation run:

- `git diff --check`
- Verified the listed agent-browser source surfaces exist in this repository.
- Verified the listed AuraCall source surfaces exist under the sibling
  `../auracall` repository.
- Ran Graphiti discovery against `agent_browser_main` for AuraCall CDP
  migration, BYOP, controlled CDP attach, bounded evaluate, and service tab
  handle context.

Result:

- The handoff note requests profile-origin and BYOP registration, a
  lease-backed service tab handle, controlled CDP attach, bounded evaluate
  jobs, readiness and identity probe recipes, tab reuse repair, diagnostic
  evidence bundles, and service-client ergonomics.
- The note keeps provider-specific ChatGPT, Gemini, Grok, and AuraCall
  semantics out of agent-browser and frames the work as service-owned browser
  primitives for a future implementation agent.

## Turn 17 | 2026-06-13

Scope: open a high-level upgrade plan suitable for subagents and goal-driven
execution.

Actions:

- Added
  `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`.
- Added P14 to `ROADMAP.md`.
- Structured the plan as a parent goal with slice-level subagent prompts,
  acceptance criteria, coordination rules, validation matrix, and open
  questions.

Validation run:

- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `pnpm validation:select -- --base HEAD`

Result:

- P14 is open for profile origin/BYOP, lease-backed service tab handles,
  controlled CDP attach, bounded evaluate, diagnostics/readiness evidence, and
  client ergonomics.
- The first recommended implementation slice is P14 Slice A: profile-origin
  schema plus explicit BYOP registration/readback.

## Turn 18 | 2026-06-13

Scope: implement P14 Slice A profile-origin and BYOP registration/readback.

Actions:

- Added durable service profile origin values:
  `agent_browser_owned`, `external_byop`, and `external_observed`.
- Added external profile registration metadata and browser compatibility
  evidence to service profile records.
- Added `registerExternalProfile()` to
  `@agent-browser/client/service-observability` for explicit BYOP or observed
  external profile registration.
- Exposed `profileOrigin` through service profile allocation readback and
  access-plan selected profiles.
- Hardened retained-state orphan profile pruning so `external_byop` and
  `external_observed` profiles are never pruned as owned profile data.
- Preserved profile origin and external metadata through the dashboard profile
  config save path.
- Updated service schemas, generated client types, README, docs site, and the
  installed agent-browser skill.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_removes_orphaned_custom_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-profile-allocation`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented as a no-launch contract slice.
- Access-plan and profile readback can distinguish owned, BYOP, and observed
  external profile lanes.
- Explicit external profile registration records caller identity, target
  identities, account ids, user-data directory, and browser compatibility
  evidence.
- The next recommended P14 slice is Slice B: lease-backed service tab handles.

## Turn 19 | 2026-06-13

Scope: implement P14 Slice B lease-backed service tab handles.

Actions:

- Added `ServiceTabHandle` and `ServiceTabHandleTraceFilter` to the service
  model.
- Derived tab handles from service state for `service tabs`, grouped browser
  `tabHandles`, and tab lifecycle trace event details.
- Extended direct `tab_new` responses with CDP target/session IDs and a
  conservative immediate `serviceTabHandle`.
- Added `getServiceTabHandle()` and `requireServiceTabHandle()` to
  `@agent-browser/client/service-request`.
- Updated service tab/browser schemas, generated client declarations, README,
  docs site, and the installed agent-browser skill.
- Added no-launch Rust and service-client fixtures for valid handles, binding
  fields, and stale-handle rejection.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice B is implemented as a no-launch contract slice.
- Software clients can use the returned service tab handle instead of
  rediscovering browser, session, profile, tab, target, lease, or trace
  identity.
- Stale handles fail closed through the client helper and expose explicit
  stale reasons in service readbacks.
- The selector recommended `pnpm test:service-cdp-tab-streaming-live` because
  browser/tab surfaces changed; that live smoke was deferred to Slice C unless
  live proof is requested before controlled CDP attach work starts.
- The next recommended P14 slice is Slice C: controlled CDP attach for leased
  service tab handles.

## Turn 20 | 2026-06-13

Scope: implement P14 Slice C controlled CDP attach for leased service tab
handles.

Actions:

- Added `cdp_attach` and `cdp_detach` to the service request action metadata,
  HTTP relay, MCP service request surface, Rust daemon dispatcher, JSON schema,
  generated client types, and `@agent-browser/client/service-request` helpers.
- Gated attach on a valid `serviceTabHandle`, `cdpAttachmentAllowed: true`,
  non-CDP-free posture, matching service session, handle freshness, and target
  identity.
- Returned a service-owned attach descriptor with browser, session, tab,
  target, profile, lease, cleanup, trace, websocket, and detach metadata.
- Made detach preserve the browser process by default and return explicit
  detach metadata.
- Updated README, docs site, repo skill, and installed agent-browser skill for
  the new attach/detach helper path.
- Updated P14 plan and ROADMAP so Slice D bounded evaluate is the next
  implementation target.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice C is implemented with no-launch policy and stale-handle coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-98925`,
  stream `37669`.
- A dedicated attach-read-detach live smoke remains as the validation gap before
  treating controlled attach as AuraCall migration proof.
- The next recommended P14 slice is Slice D: bounded evaluate against leased
  service tab handles.

## Turn 21 | 2026-06-13

Scope: implement P14 Slice D bounded evaluate against leased service tab
handles.

Actions:

- Added `evaluate` to the service request action metadata, HTTP relay, MCP
  service request surface, JSON schema, generated client types, and
  `@agent-browser/client/service-request` helpers.
- Required `serviceTabHandle`, `script` or `expression`, positive `timeoutMs`,
  and positive `maxReturnBytes` for service-owned evaluate requests.
- Made service-bound evaluate skip browser auto-launch, switch to the handle's
  CDP target, execute with a daemon-side timeout, cap serialized return data,
  and return URL/title plus truncation metadata.
- Added no-launch HTTP, MCP, and service-client coverage for missing handles,
  missing caps, stale handles, and helper request shape.
- Updated README, docs site, repo skill, installed agent-browser skill, P14
  plan, and ROADMAP for the new bounded evaluate helper path.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice D is implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-73918`,
  stream `37595`.
- A dedicated live bounded-evaluate smoke remains as the validation gap before
  treating bounded evaluate as AuraCall migration proof.
- Screenshot-on-failure capture is deferred to Slice E diagnostic bundles so
  screenshot storage, caps, and trace links are implemented in one evidence
  surface.
- The next recommended P14 slice is Slice E: diagnostics and readiness
  evidence.

## Turn 22 | 2026-06-13

Scope: implement the P14 Slice E diagnostic bundle sub-slice for leased service
tab handles.

Actions:

- Added `diagnostics` to service request action metadata, HTTP relay, MCP
  service request validation, Rust daemon dispatch, JSON schema, generated
  client types, and `@agent-browser/client/service-request` helpers.
- Required a valid `serviceTabHandle` and reused the service-owned queue and
  handle validation path rather than adding a caller-owned browser path.
- Returned a compact evidence bundle with URL/title, browser/session/tab
  identity, profile readiness, route/view metadata, browser health, console
  entries, page errors, recent request summaries, snapshot summary, caller
  context, trace filter, and optional screenshot path.
- Added no-launch client helper coverage for request shape, stale handles, and
  evidence count caps.
- Updated README, docs site, repo skill, P14 plan, and ROADMAP for the new
  diagnostic helper path.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm generate:service-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice E diagnostic bundles are implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-95746`,
  stream `36831`.
- Slice E remains open for readiness/freshness lifecycle gating and any focused
  live diagnostics smoke requested before AuraCall migration proof.

## Turn 23 | 2026-06-20

Scope: open the corrective planning lane for recurring Guacamole/RDP
false-ready states after the live LinkedIn manual-auth route repair.

Actions:

- Added
  `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`.
- Added P16 to `ROADMAP.md`.
- Made the combined readiness invariant explicit: a remote-control browser is
  ready only when the selected browser window is loaded, visible, and
  controllable through the selected external Guacamole/RDP route.
- Captured the two recurring failure classes as plan gates:
  - Guacamole unhappy document or internal error caused by schema, route, URL,
    or permission drift.
  - Terminal-only remote desktop caused by browser/display mismatch.
- Scoped the next fix as a generic one-command/API path,
  `agent-browser remote-view open` and service action `remote_view_open`,
  rather than a LinkedIn-specific or AuraCall-specific repair.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Focused Plan 0039 validation passed. The broad planning audit remains red
  from pre-existing historical plan drift, but it reports no Plan 0039
  problems.
- Implementation remains open under Plan 0039. Slice A and Slice B are the
  recommended parallel starting points.

## Turn 24 | 2026-06-22

Scope: open the runtime convergence lane after remote-view and dashboard
binary harmonization exposed remaining runtime identity confusion.

Actions:

- Added `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`.
- Added P42 to `ROADMAP.md`.
- Captured the missing invariant: the dashboard runtime manifest proves only
  the dashboard service identity, not every active daemon session, stream
  backend, route helper, retained browser row, or foreign CDP browser.
- Scoped executable slices for active runtime inventory, daemon executable
  SHA-256 convergence, actionable doctor remedies, idempotent remote-view
  bootstrap, live rail boundaries, and one-command local convergence.
- Kept P41 foreign CDP discovery as a separate dependency so non-owned browser
  addressability is not confused with lifecycle ownership.

Validation run:

- `git diff --check`

Result:

- P42 is active and not complete. Slice D is already in progress through the
  Guacamole Postgres/schema bootstrap guard. The next implementation slice is
  daemon executable SHA convergence and active runtime inventory.

## Turn 25 | 2026-06-22

Scope: execute P42 Slice B daemon executable SHA convergence.

Actions:

- Added daemon executable SHA metadata next to the existing daemon version
  metadata.
- Made daemon reuse compare the invoking executable SHA-256 against the daemon
  SHA metadata when the invoking executable can be hashed.
- Treated missing daemon SHA metadata as stale by default, with
  `AGENT_BROWSER_ALLOW_LEGACY_DAEMON_SHA_REUSE=1` as an explicit reviewed
  compatibility escape hatch.
- Extended stale daemon cleanup to remove `<session>.sha256`.
- Updated P42 Slice B completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml cleanup_stale_files_removes_version_and_executable_sha -- --nocapture`

Result:

- Focused daemon SHA convergence tests passed. P42 remains open for active
  runtime inventory, doctor remedies, live rail convergence boundaries, and
  one-command local convergence.

## Turn 26 | 2026-06-22

Scope: execute P42 Slice A active runtime inventory in doctor output.

Actions:

- Added `runtimeInventory` to `agent-browser install doctor --json`.
- The inventory scans the daemon socket metadata directory without launching
  Chrome and reports daemon session PID, PID liveness, package version match,
  executable SHA-256 match, stream port, and metadata presence.
- Added `active_runtime_stale_executable` install doctor issues for active
  daemon sessions whose metadata is stale or incomplete.
- Lifted the install doctor's runtime inventory into
  `agent-browser doctor remote-view --json` as top-level `runtimeInventory`.
- Updated P42 Slice A completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml runtime_inventory_from_install -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests and clippy passed.
- The rebuilt debug-binary install doctor reported
  `runtimeInventory.status=stale`, `runtimeCount=4`, and `staleCount=4`.
- The rebuilt debug-binary remote-view doctor lifted the same inventory and
  reported `runtimeInventory.status=stale`. This intentionally made the
  debug-binary readback not remote-control ready against the installed runtime,
  proving stale active runtimes are no longer omitted from readiness.

## Turn 27 | 2026-06-22

Scope: execute the first P42 Slice C convergence doctor remedy.

Actions:

- Added session-scoped remedy metadata to `active_runtime_stale_executable`
  install doctor issues.
- Each stale daemon issue now carries `session`,
  `nextAction=restart_stale_daemon_session`, and an argv-safe remedy for
  `agent-browser close --session <session>`.
- Made remote-view doctor prefer
  `restart_stale_daemon_sessions_then_rerun_doctor` when install readiness is
  blocked by stale active daemon sessions.
- Updated P42 Slice C progress notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests, clippy, and debug CLI build passed.
- The rebuilt debug-binary install doctor reported four
  `active_runtime_stale_executable` issues; the first issue carried
  `session=default`, `nextAction=restart_stale_daemon_session`, and
  `remedy.argv=["agent-browser","close","--session","default"]`.
- The rebuilt debug-binary remote-view doctor reported
  `nextAction=restart_stale_daemon_sessions_then_rerun_doctor` and a
  next-command explanation that points operators back to each issue's
  session-scoped `remedy.argv`.

## Turn 28 | 2026-06-22

Scope: execute P42 local binary/runtime convergence after publishing the
structured commits.

Actions:

- Extended `pnpm publish:local-dashboard` so it synchronizes the user-scoped
  install binary, ignored workspace package binary, and user pnpm package
  binary to the same freshly built executable by default.
- Added `--skip-reference-sync` for operator cases that intentionally do not
  want reference binaries changed.
- Published the current debug build to the local dashboard runtime and restarted
  `agent-browser-dashboard.service`.
- Applied the stale daemon restart path by invoking the three
  session-scoped remedies reported by install doctor. Those commands returned
  nonzero because `close --session` still routes through daemon restart, but
  the restart path did replace the stale daemon metadata and all active daemon
  rows converged.
- Reran publish after adding reference-binary sync so install doctor no longer
  failed on pnpm/workspace binary drift.

Validation run:

- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The publish report synchronized
  `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the user pnpm global package binary to
  `94d1d022b4f1315b2f3eb9ff08fdc3faa816d77960500c6b6854cab98161cfa8`.
- Installed `agent-browser install doctor --json` reported `success=true`,
  `runtimeInventory.status=converged`, `staleCount=0`, no issue codes, and
  matching PATH, pnpm, and workspace binary SHA-256 values.
- Installed `agent-browser doctor remote-view --json` reported `success=true`,
  `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=converged`, and
  `nextAction=run_many_to_many_live_gate`.
- Follow-up required: make stale daemon close remedies return success without
  depending on a daemon restart side effect.

## Turn 29 | 2026-06-22

Scope: finish P42 close/remedy and install-doctor probe convergence discovered
during local execution.

Actions:

- Added a `close --session` prestart path that targets an existing daemon
  before daemon convergence startup.
- Added explicit-session stale metadata cleanup for unauthorized or non-ready
  daemon close attempts, returning success with a warning instead of trying to
  start a replacement daemon.
- Classified running PID metadata without an addressable socket, stream, or
  port as `diagnostic` instead of stale active runtime inventory.
- Changed `service status` to execute locally before daemon startup.
- Changed install doctor service-status probing to use a unique owned session,
  terminate the owned probe daemon after reading status, and treat the isolated
  no-state probe as no-launch ready.
- Ran service GC apply for the orphaned Xvfb candidate that was blocking local
  install doctor readiness.
- Published the final local runtime and synchronized reference binaries.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml force_close_session_from_metadata -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml close_targets_existing_daemon_before_prestart -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_locally_before_daemon -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Final local publish succeeded and restarted
  `agent-browser-dashboard.service`.
- Final installed executable SHA-256:
  `19ba0d616388e1eb84241eea5ddcffa56a1803831c5085acc25abb01277b78e6`.
- Reference binaries in `~/.local/bin`, ignored workspace `bin/`, and user
  pnpm global package path matched the installed executable SHA-256.
- Final installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeInventory.status=none`,
  `runtimeCount=0`, and `staleCount=0`.
- Final installed `agent-browser doctor remote-view --json` reported
  `success=true`, `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=none`, `staleCount=0`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 30 | 2026-06-22

Scope: finish P42 live-rail and one-command local runtime convergence.

Actions:

- Added `pnpm converge:local-runtime` as a dry-run by default local operator
  convergence command.
- In apply mode, the command runs local dashboard publication, applies only
  doctor-reported `agent-browser close --session <name>` stale-daemon
  remedies, runs the Guacamole Postgres schema ensure, runs route-pool
  readiness, applies route display-access grants only when remote-view doctor
  asks for them, and reruns final doctors.
- Added `pnpm test:local-runtime-convergence` to lock the command contract,
  foreign-process refusal boundary, display-grant sequencing, and retained
  evidence behavior.
- Marked P42 Slice E done from the dashboard live-rail contract tests and
  Slice F done from command validation.

Validation run:

- `node --check scripts/converge-local-runtime.js`
- `node --check scripts/test-local-runtime-convergence.js`
- `pnpm test:local-runtime-convergence`
- `pnpm --silent converge:local-runtime -- --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-evidence.json`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`

Result:

- Dry-run convergence returned `success=true`, final install doctor ready,
  final remote-view ready, zero safe stale remedies, and zero skipped remedies.
- Apply convergence returned `success=true`, wrote
  `/tmp/agent-browser-converge-local-runtime-evidence.json`, final install
  doctor ready, final remote-view ready, and zero skipped remedies.
- Dashboard workspace tests passed, proving the live rail keeps retained and
  no-action attention rows out of the live control surface and groups
  reachable non-owned CDP browsers separately.
- P42 remains open for Slice C stale dashboard/stream classifications and
  Slice D bootstrap hardening.

## Turn 31 | 2026-06-22

Scope: continue P42 Slice C by classifying stale or unreadable live dashboard
runtime manifests.

Actions:

- Added an install-doctor live dashboard manifest probe for the local
  `/api/runtime/manifest` endpoint.
- Kept dashboard-not-running as non-drift, but classified a running dashboard
  that serves no readable manifest or a mismatched executable SHA-256 as
  `dashboard_runtime_stale_or_unreadable`.
- Added a bounded remedy pointing to
  `pnpm converge:local-runtime -- --apply --json`.
- Updated remote-view doctor so that dashboard runtime drift recommends
  `converge_local_runtime_then_rerun_doctor` before generic install drift.
- Updated `pnpm converge:local-runtime -- --apply --json` so initial nonzero
  doctor JSON is treated as repairable input in apply mode instead of aborting
  before local publish.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn31-final.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, clippy, and debug CLI build passed.
- The rebuilt debug install doctor reported
  `dashboard_runtime_stale_or_unreadable` with `state=stale_executable` when
  the running dashboard manifest executable SHA-256 did not match the debug
  executable.
- Convergence apply started with initial install issue
  `dashboard_runtime_stale_or_unreadable`, published the new local runtime, and
  ended with final install doctor ready, final remote-view ready, zero skipped
  remedies, and retained evidence at
  `/tmp/agent-browser-converge-local-runtime-turn31-final.json`.
- Direct installed `agent-browser install doctor --json` then reported
  `success=true`, no issue codes, `liveDashboardRuntime.ready=true`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend classification work.

## Turn 32 | 2026-06-22

Scope: continue P42 Slice C by adding explicit runtime convergence summary
states.

Actions:

- Added install-doctor `runtimeConvergence` with schema
  `agent-browser.runtime-convergence.v1`.
- Derived summary status from runtime inventory plus live dashboard manifest
  state, using `converged`, `partial`, `stale`, and
  `manual_review_required`.
- Lifted the summary into remote-view doctor and printed it in text output
  separately from raw runtime inventory status.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml runtime_convergence -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn32.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now locks the `converged`, `partial`, `stale`, and
  `manual_review_required` summary statuses plus remote-view summary lifting.
- Convergence apply published the summary-state build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn32.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend and diagnostic
  retained-row classification work.

## Turn 33 | 2026-06-22

Scope: finish P42 Slice C stale stream-backend classification.

Actions:

- Extended runtime inventory to probe advertised daemon stream ports.
- Added runtime row `streamReachable` and `driftReasons` evidence.
- Classified live daemon sessions with unreachable or invalid stream metadata
  as stale instead of converged.
- Added install-doctor issue code `active_runtime_stale_stream_backend` with
  the bounded `agent-browser close --session <session>` remedy.
- Updated remote-view doctor to treat stale stream backends as a
  session-scoped daemon restart prerequisite before generic install drift.
- Marked P42 Slice C done.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml stream_backend -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn33.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now proves unreachable stream metadata produces a stale runtime
  inventory row, install doctor emits
  `active_runtime_stale_stream_backend`, and remote-view doctor recommends the
  same session-scoped restart prerequisite.
- Convergence apply published the stream-backend build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn33.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `staleRuntimeCount=0`, and `runtimeInventory.status=none`.

## Turn 34 | 2026-06-22

Scope: close P42 by auditing and validating Slice D idempotent remote-view
bootstrap.

Actions:

- Verified `pnpm ensure:rdp-guac-postgres -- --apply` exists and is invoked by
  local convergence.
- Verified route-pool setup, existing-user route sync, and legacy autologin
  setup call the shared Guacamole Postgres schema guard before writing records.
- Verified the schema guard refuses partial `guacamole_*` relation state,
  imports only absent schema state, waits for Postgres readiness, and
  checkpoints after ready/imported states.
- Verified the live Guacamole compose file keeps explicit Postgres durability
  settings for WSL hard-stop resilience.
- Marked P42 `State: CLOSED`.

Validation run:

- `bash scripts/ensure-rdp-guac-postgres.sh --dry-run`
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `agent-browser doctor remote-view --json`

Result:

- Schema guard dry-run reported `Guacamole Postgres schema is ready.`
- Route-pool readiness reported `success=true`; Postgres, schema, Guacamole
  web/login, guacd, RDP connections, connection permissions, distinct targets,
  and both RDP backend TCP checks were ready.
- Direct installed remote-view doctor reported `success=true`, `status=ready`,
  `remoteControl.ready=true`, `runtimeConvergence.status=converged`,
  `runtimeInventory.status=none`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 35 | 2026-06-22

Scope: investigate the `last30days` Facebook remote-view friction and open the
next route-handoff audit lane.

Actions:

- Read the incident note at
  `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`.
- Used Graphiti discovery for advisory prior context and CodeGraph for the
  route-binding and dashboard stream helper joins.
- Captured live readbacks from `agent-browser doctor remote-view --json`,
  `agent-browser service browsers --json`, and
  `agent-browser service tabs --json`.
- Added P43 in
  `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`.
- Updated `ROADMAP.md` with the open P43 lane.

Findings:

- P42 binary/runtime convergence remains green. The failure sits above that
  layer.
- `session:default` owns the `last30days-facebook` browser on display `:11`
  with Facebook tabs and a generic Guacamole stream.
- `session:litscout-ai-smoke-clean` is a separate browser on display `:93`
  with several `127.0.0.1` tabs and its own generic Guacamole stream.
- The dashboard has stream metadata that can embed Guacamole, but it does not
  yet require row-bound proof that the stream is showing the intended browser
  instead of a terminal.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the docs-only change set.
- The planning-contract audit still fails on pre-existing older plan wiring and
  deterministic-state debt. The new P43 plan itself is reported with
  `filename_ok=true`, `lane_ok=true`, `state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 36 | 2026-06-22

Scope: execute P43 Slice A with a read-only route-handoff audit surface.

Actions:

- Added `scripts/audit-route-handoff.js`.
- Added package command `pnpm audit:route-handoff`.
- Added no-launch fixture coverage in `scripts/test-route-handoff-audit.js`.
- Added package command `pnpm test:route-handoff-audit`.
- Documented the audit command in `README.md`.
- Marked P43 Slice A done and updated `ROADMAP.md` next recommendation.

Validation run:

- `node --check scripts/audit-route-handoff.js`
- `node --check scripts/test-route-handoff-audit.js`
- `pnpm test:route-handoff-audit`
- `pnpm --silent audit:route-handoff -- --json --skip-doctor`
- `pnpm --silent audit:route-handoff -- --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:route-handoff-audit && pnpm --silent audit:route-handoff -- --json --skip-doctor | jq -e '.success == true and .data.summary.route_bound_ready == 2 and .data.summary.direct_remote_headed == 11'`

Result:

- Syntax checks passed.
- The fixture test passed and covers `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`
  classifications.
- The live read-only audit with `--skip-doctor` returned `success=true`,
  `collections.browsers=2`, `collections.tabs=13`, and summary
  `route_bound_ready=2`, `direct_remote_headed=11`.
- The full live audit also returned `success=true`, no collection errors,
  `runtime.convergenceStatus=converged`,
  `runtime.inventoryStatus=converged`, `runtime.runtimeCount=1`, and
  `runtime.remoteControlStatus=ready`.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` recommended `git diff --check` and
  `node scripts/dev/select-validation.js --base HEAD --json`; both passed.
- The combined fixture plus live summary assertion passed.

## Turn 37 | 2026-06-22

Scope: execute P43 Slice B one-line CLI contract and help.

Actions:

- Added command-specific `remote-view` help covering
  `agent-browser remote-view open`.
- Added the Facebook-style one-liner and flag placement guidance to CLI help.
- Changed `parse_remote_view_open` to copy global `--session-name` into the
  `remote_view_open` request.
- Added parser tests for post-subcommand `--runtime-profile`, `--session`,
  `--session-name`, `--browser-build`, and `--provider` placement.
- Updated `README.md`, `docs/src/app/commands/page.mdx`, and
  `skills/agent-browser/SKILL.md`.
- Marked P43 Slice B done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo run --quiet --manifest-path cli/Cargo.toml -- remote-view open --help | rg -n "Facebook|Global placement|--session selects|last30days-facebook"`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Rust format passed after applying `cargo fmt`.
- Focused Rust tests passed: 10 passed, 0 failed.
- Help output includes the global placement section, Facebook examples, and
  the `--session` versus `--session-name` distinction.
- Docs build passed.
- Clippy passed with `-D warnings`.
- Validation selector required the Rust format, focused Rust test, clippy,
  docs build, diff hygiene, and repo-installed skill sync checks.
- The repo and installed `agent-browser` skill copies now match.

## Turn 38 | 2026-06-22

Scope: execute P43 Slice C route allocation diagnostics.

Actions:

- Added compact route-pool diagnostic JSON for `route_pool_unavailable`.
- Added the same diagnostic context to stale explicit pool-entry failures:
  `route_pool_entry_missing` and `route_pool_entry_unavailable`.
- Included requested route, route-pool entry, display allocation, display name,
  display isolation, owner browser, owner session, profile, provider, matching
  pool entries, available pool entries, ready display allocation IDs, existing
  remote-view routes, and recommended commands.
- Kept the existing string error-code contract intact so callers that check
  `route_pool_unavailable` continue to work.
- Tightened route-pool tests to parse the diagnostic suffix and assert the
  actionable identity fields.
- Marked P43 Slice C done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml route_pool -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:service-cdp-tab-streaming-live`
- Direct temp-session probe:
  `HOME=<temp> AGENT_BROWSER_HOME=<temp>/.agent-browser AGENT_BROWSER_SOCKET_DIR=<temp>/s cargo run --quiet --manifest-path cli/Cargo.toml -- --json --session daemon-probe stream status`

Result:

- Focused route-pool Rust tests passed: 12 passed, 0 failed.
- Focused CDP stream Rust tests passed: 3 passed, 0 failed.
- Rust format, clippy, and diff hygiene passed.
- The new route-pool unavailable test verifies stable error code retention and
  machine-readable diagnostic fields for requested display identity, checked
  out matching pool entry, ready display allocations, and recommended repair
  command.
- The selector-recommended live CDP smoke did not reach the CDP path. It failed
  while starting a temporary daemon with
  `Daemon failed to start (socket: <temp>/s/<session>.sock)`. A direct
  temp-session `stream status` probe reproduced the same daemon-start failure
  and left only pid/token/version files. This appears independent of the
  route-pool diagnostic change and should be handled as a separate daemon
  startup validation issue.

## Turn 39 | 2026-06-22

Scope: execute P43 Slice D profile-lock ownership diagnostics.

Actions:

- Added profile-lock diagnostic JSON to Chrome profile lock failures while
  preserving the existing hard stop against launching a second Chrome process
  on the same user-data-dir.
- The diagnostic includes lock PID, user-data-dir, matching runtime profile
  state, matching service browser rows, primary owner, and safe remedies.
- Known service-owned locks now identify browser ID, active session, profile,
  host, health, PID, CDP endpoint, display, display allocation, and view stream
  IDs when persisted service state has them.
- Remedies include exact session-scoped service-status reuse and close commands
  for known owners, runtime-profile inspection and attach commands for matching
  runtime state, service-status inspection for unknown owners, and explicit
  separate-profile guidance for intentionally separate identities.
- Updated README, CLI runtime help, docs command page, and the
  `agent-browser` skill.
- Marked P43 Slice D done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml locked_profile -- --test-threads=1 --nocapture`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused profile-lock tests passed for known service/runtime owner diagnostics
  and unknown-owner diagnostics.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, diff hygiene, validation selector, and installed-skill sync passed.

## Turn 40 | 2026-06-22

Scope: execute P43 Slice E operator-visible success contract.

Actions:

- Added top-level `operatorVisible` to `remote-view open` dry-run and opened
  responses.
- Dry-runs report `operatorVisible.state=not_checked` with route, browser,
  session, display, provider, and display allocation identity.
- Successful opened responses report `operatorVisible.state=ready` and include
  the visible-window proof that already gates success.
- Added dry-run assertions and a pure ready-proof unit test for the
  `operatorVisible` contract.
- Updated README, CLI remote-view help, docs command page, and the
  `agent-browser` skill to tell clients to require
  `operatorVisible.state=ready`.
- Marked P43 Slice E done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused remote-view-open tests passed, including the new
  `operatorVisible` dry-run and ready-proof coverage.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, format check, diff hygiene, validation selector, installed-skill
  sync, and the no-launch CDP stream test passed.
- The selector still recommends `pnpm test:service-cdp-tab-streaming-live`
  because `actions.rs` changed; the same live smoke was already attempted in
  Turn 38 and failed before CDP validation while starting a temporary daemon.

## Turn 41 | 2026-06-22

Scope: execute P43 Slice F dashboard row binding and route-proof UX.

Actions:

- Added `operatorVisibleState` and `operatorVisibleReason` to dashboard
  workspace view-stream rows.
- Required current browser-window proof before RDP gateway View, Control, or
  external open actions are enabled.
- Kept terminal-only, idle-display, and missing-proof route rows in the live
  owned group as disabled diagnostics rather than moving them into a no-action
  attention category.
- Preserved detected non-owned browser grouping and retained-record filtering in
  the live workspace navigator.
- Updated README, docs dashboard/service/commands pages, the `agent-browser`
  skill, P43, and `ROADMAP.md`.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm publish:local-dashboard -- --expect-marker "operator-visible proof missing" --skip-browser --json`

Result:

- Dashboard workspace node, navigator, selected-context, chat-packet, and
  console smokes passed.
- Docs and dashboard builds passed with the known Next.js multiple-lockfile and
  static-export rewrite warnings.
- Diff hygiene, validation selector, selector JSON, and installed-skill sync
  passed.
- The local dashboard runtime was rebuilt into
  `/home/ecochran76/.local/bin/agent-browser`, `agent-browser-dashboard.service`
  restarted, `/api/runtime/manifest` matched the installed executable SHA
  `f626320b5d084f824917560bdad60c8111678896cf81299a602c2d3a35c9d0a6`, and the
  served chunk contained `operator-visible proof missing`.
- The full publish browser smoke was attempted first and failed at the known
  temp-daemon startup boundary:
  `Daemon failed to start (socket: /run/user/1000/agent-browser/local-dashboard-runtime-smoke-2846318.sock)`.
  The final publish used `--skip-browser`, so live browser launch remains
  covered by the separate temp-daemon startup blocker rather than this Slice F
  dashboard contract.

## Turn 42 | 2026-06-22

Scope: execute P43 Slice G downstream client contract and last30days handoff
guidance.

Actions:

- Made `requestServiceRemoteViewOpen` require `operatorVisible.state=ready`
  before returning non-dry-run handoff success.
- Added service-client helpers for reading operator-visible state, checking
  readiness, throwing on invalid handoff proof, and logging one compact route,
  tab, profile, and visual-proof summary line.
- Kept dry-run remote-view open responses allowed as `not_checked` and made
  infrastructure-only readiness an explicit client opt-in that is not posted to
  the service API.
- Updated README, docs commands page, service-client examples, generated client
  types, and the installed `agent-browser` skill.
- Updated `last30days` so Facebook uses the route-bound
  `agent-browser remote-view open` one-liner with the `last30days-facebook`
  runtime profile and rejects missing-proof, CDP-only, or terminal-only
  Guacamole/RDP handoff success.
- Marked P43 Slice G done and moved `ROADMAP.md` to Slice H live gates.

Validation run:

- `git diff --check`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `uv run pytest tests/test_facebook.py`
- `python3 -m py_compile skills/last30days/scripts/lib/facebook.py skills/last30days/scripts/lib/env.py`

Result:

- Service API/MCP parity, service-client contract/type/export/request/helper
  smokes, docs build, diff hygiene, and installed-skill sync passed.
- Focused last30days Facebook tests passed with 9 tests, including the
  terminal-only rejection case.
- P43 remains open for Slice H. The next gate needs no-launch route-confusion
  fixtures and an OCR-backed live route proof that fails on terminal-only
  route displays.

## Turn 43 | 2026-06-22

Scope: execute P43 Slice H live gates and close the route-handoff confusion
audit lane.

Actions:

- Added `pnpm test:route-confusion-gates` as the focused no-launch gate for
  route-handoff confusion regressions.
- Covered wrong flag placement, named-session route-pool mismatch,
  same-owner route-pool repeat checkout, known-owner profile-lock messaging,
  direct remote-headed audit classification, and dashboard missing-proof plus
  terminal-only row classification.
- Updated validation selection so route, dashboard stream, service-client, and
  remote-view command changes recommend the route-confusion gate.
- Strengthened the live `remote-view open` fixture smoke with isolated daemon
  session and runtime profile defaults, bounded daemon-start retry, available
  route-pool selection, repeat handoff through the first route/display
  identity, route-handoff audit assertion, and OCR of the route display.
- Fixed the route-pool checkout resolver so an already checked-out route is
  reusable only for the same ready route, browser, session, and display
  allocation. Other owners still receive `route_pool_unavailable`.
- Marked P43 complete in the plan and roadmap.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Diff hygiene, Rust formatting, validation selector JSON, clippy, no-launch
  CDP stream regressions, and the route-confusion gate passed.
- The OCR-backed live route gate passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-22T16-23-29-784Z`,
  route `guacamole:5`, display allocation `remote-view-display:12`,
  route-handoff classification `route_bound_ready`, visual state
  `browser_window_visible`, and fixture text
  `REMOTE VIEW OPEN FIXTURE 3815575`.
- `pnpm test:service-cdp-tab-streaming-live` was retried twice and failed
  before CDP validation at the known temporary-daemon startup boundary:
  `Daemon failed to start`.

## Turn 44 | 2026-06-22

Scope: diagnose and repair `pnpm test:service-cdp-tab-streaming-live`.

## Turn 45 | 2026-06-22

Scope: execute P44 Slice A intent normalization and remote-view provider
harmonization.

Actions:

- Added `RemoteViewOpenIntent` normalization for `remote_view_open` before
  route binding or launch.
- Made `viewStreamProvider` the canonical remote-view stream field and kept
  `provider=rdp_gateway` as a compatibility alias.
- Rejected provider/view-stream conflicts before acquisition.
- Updated CLI help, README, docs site, repo skill guidance, Plan 0044, and
  `ROADMAP.md`.
- Added focused CLI parser tests and remote-view intent normalization tests.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml normalize_remote_view_open_intent -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:browser-capability-registry-draft`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- All listed no-launch checks passed.
- `pnpm test:service-cdp-tab-streaming-live` was selected by validation but
  not rerun in this slice because the prior P43 closeout recorded the existing
  temporary-daemon startup boundary before CDP validation.

Actions:

- Reproduced the original failure with an isolated temp home and debug daemon
  logs. The client timed out before the daemon bound its socket, but the daemon
  stayed alive and became usable seconds later.
- Added daemon startup milestones under `--debug`.
- Moved Unix control-socket bind ahead of stream-server startup.
- Moved executable SHA calculation out of the daemon startup critical path by
  writing a short-lived `pending` marker and filling the real SHA in a
  background task. The client tolerates `pending` only during startup grace.
- Avoided hashing the current executable on fresh daemon startup unless an
  already-running daemon must be compared.
- Added a bounded smoke retry around first `stream status` daemon startup.
- Fixed service-owned `navigate` so it persists the active tab record and
  service tab handle, matching the existing `tab_new` retained-tab contract.
- Hardened the CDP tab streaming smoke diagnostics and allowed data-URL marker
  matching when Chrome has not populated a tab title yet.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml test_daemon_executable_sha_pending_is_startup_grace_only -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm validation:select -- --base HEAD --json`

Result:

- Isolated first-command startup dropped from roughly 10 to 13 seconds to
  about 145 ms on the debug binary.
- The original live smoke passed end to end:
  `Service CDP tab streaming live smoke passed`.

## Turn 46 | 2026-06-22

Scope: execute P44 Slice B no-mutation acquisition planner.

Actions:

- Added `RemoteViewAcquisitionPlan` for route-bound remote-view acquisition.
- Routed `remote_view_open`, route preflight, and route checkout through the
  planner before state mutation.
- Moved route/display fallback selection into named planner decisions and
  surfaced `acquisitionPlan` in dry-run, opened, preflight, and checkout
  responses.
- Added blockers and diagnostics for unavailable route-pool entries and
  named-session display-allocation mismatches.
- Added planner fixtures for stale browser fallback ordering, checked-out
  same-owner reuse, checked-out other-owner rejection, named-session mismatch
  diagnostics, and dry-run acquisition-plan output.
- Updated Plan 0044 and `ROADMAP.md` so Slice C is the next P44 boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `pnpm test:route-handoff-audit`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed no-launch checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2019837` and stream `38157`.

## Turn 47 | 2026-06-22

Scope: execute the P44 Slice C no-launch acquisition lease and rollback
foundation.

Actions:

- Added persisted `RemoteViewAcquisitionLease` state to `ServiceState`.
- Wrapped `remote_view_open` in an acquisition lease that marks selected
  route-pool entry, display allocation, and remote-view route records as
  pending before display access, browser launch, tab acquisition, proof, and
  checkout complete.
- Added rollback for display-access, launch, tab-open, focus, proof, and
  checkout failures.
- Changed failure cleanup summaries to typed JSON with cleanup and lease
  rollback evidence.
- Added a no-launch fixture proving a failed pending acquisition restores the
  available route-pool entry and removes pending display/route rows.
- Updated Plan 0044 and `ROADMAP.md` to record Slice C progress and keep the
  forced-proof live smoke as remaining Slice C work.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-handoff-audit`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2342690` and stream `38445`.
- Slice C is not complete yet. The focused forced-proof failing live smoke and
  remaining service-contract metadata coverage still need to run before moving
  to Slice D.

## Turn 48 | 2026-06-22

Scope: close the P44 Slice C no-launch service-contract metadata gap.

Actions:

- Added a service-state wire-contract assertion for
  `remoteViewAcquisitionLeases`.
- Extended the nested service-state round-trip fixture with an acquisition
  lease and previous route-pool, display-allocation, and remote-view-route
  snapshots.
- Strengthened route checkout assertions for acquisition-plan metadata,
  checked-out route-pool entry state, and route provider event metadata.
- Strengthened route release assertions for release status, released
  viewer-lease metadata shape, and route release provider event metadata.
- Updated Plan 0044 and `ROADMAP.md` so the remaining Slice C gap is the
  focused forced-proof failing live smoke.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml service_state_round_trips_nested_entities -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route_and_lease_actions_mutate_service_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2420462` and stream `37215`.
- Slice C still needs the focused forced-proof failing live smoke before it can
  be marked complete.

## Turn 49 | 2026-06-23

Scope: close the P44 Slice C forced-proof live-smoke gap.

Actions:

- Added a forced visible-window-proof failure hook behind
  `AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE`.
- Added `--force-proof-failure` support to
  `scripts/smoke-remote-view-open-live.js`, including assertions for typed
  cleanup JSON, route-pool rollback, display/route rollback, and failed
  acquisition-lease state.
- Fixed post-launch failure ordering so rollback happens before browser/tab
  cleanup can prune pending lease state, then records actual cleanup back onto
  the failed lease when the lease is still retained.
- Restored missing acquisition-lease snapshots before rollback and completion
  when service mutations overwrite pending lease state.
- Allowed released or orphaned display allocations from previous sessions to be
  reclaimed by a new acquisition.
- Allowed same-owner pending route reservations to be reused during checkout and
  repeat open.
- Updated Plan 0044 and `ROADMAP.md` to mark Slice C complete and point the next
  P44 slice at browser-only route desktop work.

Validation run:

- `node --check scripts/smoke-remote-view-open-live.js`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_acquisition_lease_rollback -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reclaims_released_display_allocation_from_previous_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner_pending_route_reservation -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live -- --force-proof-failure`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`

Result:

- All listed checks passed.
- The forced-proof live smoke passed with route `guacamole:4`, display
  allocation `remote-view-display:16`, cleanup state `closed_new_browser`,
  rollback state `rolled_back`, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-16-04-025Z`.
- The normal fixture smoke passed afterward with repeat open, HTTP helper, CDP
  readback, X11 PID proof, route-handoff classification `route_bound_ready`,
  visual state `browser_window_visible`, OCR proof, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-17-24-564Z`.
- P44 Slice C is complete. P44 remains open for Slice D and later closeout
  criteria.

## Turn 50 | 2026-06-23

Scope: start P44 Slice D browser-only route desktop work.

Actions:

- Removed foreground terminal startup from new route-pool XRDP user sessions.
- Updated the installed-helper source and the route-pool setup fallback so
  generated `.xsession` files start Openbox when available, keep XRDP alive with
  an idle sleep loop, and do not launch terminal UI.
- Added `scripts/test-rdp-route-xsession.js` to guard maintained route
  `.xsession` writers against terminal startup.
- Wired the xsession guard into `pnpm test:route-confusion-gates`.
- Added `terminal_topmost` route display classification and
  `terminal_topmost_route` proof failure coverage so visible browser proof
  cannot pass when a terminal is the top application window over the browser.
- Updated README, docs install page, Plan 0044, and `ROADMAP.md`.

Validation run:

- `bash -n scripts/libexec/agent-browser-privileged-helper scripts/setup-rdp-guac-route-pool.sh`
- `pnpm test:rdp-route-xsession`
- `cargo test --manifest-path cli/Cargo.toml display_content_rejects_terminal_topmost_over_browser -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-3021946` and stream `37741`.
- Slice D is not complete yet. The installed privileged helper still needs to be
  refreshed on the host, then a cold route session and route display inspection
  should prove the desktop is browser-control-ready instead of terminal-first.
- Installed helper readback showed
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` still
  writes the old `xterm` `.xsession`; `sudo -n true` failed with password
  required, so this session could not refresh the helper or run the cold-route
  proof.

## Turn 58 | 2026-06-27

Scope: close P53 and unlock P46 S4.

Actions:

- Added `agent-browser window new [url] --same-profile` and wired S4 to create
  the second top-level window inside the same retained browser process instead
  of launching a second same-profile Chrome process.
- Rebuilt and converged the local runtime with
  `pnpm converge:local-runtime -- --apply --json`.
- Updated the S4 harness to accept explicit-command runs with no pre-existing
  daemon listener, while still rejecting duplicated or mismatched daemon
  authority.
- Updated S4 evaluation to require one retained same-profile browser row for
  the P53 topology.
- Marked P53 complete and moved P46 to S5.

Validation run:

- `node scripts/test-p47-scenario-harness.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `cargo test --manifest-path cli/Cargo.toml test_window_new_same_profile_with_url`
- `node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S4 passed with artifact
  `/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z`.
- The pass proved one retained browser process
  `session:p46-s4-window-2026-06-27T19-12-53-709Z`, one runtime profile
  `p46-s4-profile`, one route `guacamole:3`, one display `:13`, two
  same-profile top-level windows, working refresh controls for both dashboard
  operators, and window B staying ready after closing window A.
- Reset-before and reset-after both ended with zero active incidents.

## Turn 66 | 2026-06-27

Scope: close P63 and unlock P46 S11 after the S10 foreign CDP inventory lock.

Actions:

- Added dashboard `/api/session-tabs?port=<foreign-cdp-port>` fallback from
  agent-browser `/api/tabs` to raw Chrome CDP `/json/list`.
- Changed the local dashboard proxy to stop reading once declared
  `Content-Length` is satisfied, with bounded per-read timeout and response
  size cap.
- Updated S10 to read dashboard inventory through the authenticated
  viewer-client session.
- Made foreign CDP browser cleanup best-effort so profile removal cannot mask
  the scenario failure.
- Updated selected workspace probing to accept viewport-route context when the
  optional detail panel is not mounted.
- Scoped S10 foreign route-borrow detection to selected-workspace evidence
  instead of global workspace-list text.
- Marked P63 complete and advanced P46 to S11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`
- `git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`
- `pnpm publish:local-dashboard -- --skip-smoke --json`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S10 passed with artifact
  `/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z`.
- The pass proved authenticated foreign CDP inventory, normalized foreign tab
  inventory, no service route/display borrowing, stable foreign and
  service-owned selected workspace context, service-owned control readiness,
  and complete route-bound finalization.
- Reset-before and reset-after ended with zero active incidents.
- Installed executable SHA:
  `502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a`.

## Turn 67 | 2026-06-27

Scope: implement P46 S11, clear P64, and advance P46 to S12.

Actions:

- Added S11 scenario metadata for one route-bound service-owned browser and one
  zero-lease dashboard viewer-client.
- Added S11 capture for dashboard reload, stale workspace URL navigation,
  viewer-client reconnect, viewport refresh, direct Guacamole frame URL
  readback, route display inspection, service status, incidents, and
  route-bound finalization evidence.
- Added S11 evaluator checks for stale target recovery, reconnect proof,
  refresh control function, direct Guacamole reachability, route display state,
  stream binding, finalization, and incident cleanliness.
- Ran two live S11 attempts from the installed binary authority. Both reset
  cleanly with zero active incidents but failed in the harness before S11
  evaluation.
- Added P64 to repair the stale URL live-target recovery acceptance boundary.
- Added `allowRecoveredLiveTab` for S11 so immediate rewrite from a stale target
  to a current live target can satisfy the stale URL recovery criterion without
  weakening default tab matching.
- Ran the P64-authorized S11 retry and marked P64 complete.
- Advanced P46 to S12.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/lib/p47-viewer-client.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command-match --require-agent-browser-daemon-command-match`

Result:

- First failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z`.
- Second failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z`.
- Both failures proved the dashboard rejected the stale target and recovered to
  a live target, but the harness expected either exact stale URL persistence or
  explicit stale-recovery notice text.
- S11 passed with artifact
  `/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z`.
- The pass proved dashboard reload restoration, stale URL recovery to a live
  target, viewer-client reconnect, viewport refresh, direct Guacamole HTTP 200,
  route display `browser_window_visible`, route-bound finalization, and zero
  active incidents after reset-after.
- Command metadata caveat: the pass used an explicit installed binary command
  and daemon realpath matching passed, but the explicit-command guard flag was
  misspelled, so the artifact reports `requireExplicit: false` while also
  reporting `explicit: true`.
- P46 is now in progress at S12.

## Turn 68 | 2026-06-27

Scope: implement P46 S12 soak harness and stop at the S12 lock.

Actions:

- Added S12 scenario metadata for repeated normal-use drift and reset soak.
- Added S12 runner support for ten cycles of route-bound open, dashboard
  reload, viewer-client reconnect, viewport refresh, navigate, tab creation,
  tab switch, direct Guacamole readback, route-bound finalization, close,
  reset, and cycle-boundary doctor and incident probes.
- Added active-pressure and route-pool-baseline evaluation for each cycle.
- Corrected the pressure classifier after the first S12 run showed completed
  acquisition-lease history was being counted as active pressure.
- Ran a second S12 attempt and stopped after it exposed real route-pool reset
  drift.
- Repaired the failed live state through authenticated `service_route_pool_repair`
  dry-run and apply, followed by service reconcile and incident resolution.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- First S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-20-14-868Z`.
- The first run completed ten cycles with zero active incidents, zero retained
  sessions/browsers/tabs, route-pool baseline true, and direct Guacamole HTTP
  200 in every cycle, but failed due to the harness counting completed
  acquisition-lease history as active pressure.
- Second S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-39-14-415Z`.
- The second run exposed real drift: cycle 3 left `guacamole:3` orphaned on
  `remote-view-display:13`, with `guacamole-rdp-a` still checked out; cycle 4
  then failed with `route_pool_entry_unavailable`.
- `service_route_pool_repair` dry-run found one stale checkout, one stale
  route, and one stale display allocation; apply repaired all three.
- Post-repair reconcile showed both route-pool entries available,
  `guacamole:3` released, and `remote-view-display:13` released.
- Final incident summary has no active incidents; the transient cycle-browser
  incident is recovered with an explicit resolution note.
- Historical state, superseded by the later repair and S12 pass: P46 was
  locked at S12 until a follow-up plan addressed orphaned route-bound display
  cleanup after normal close.

## Turn 69 | 2026-06-27

Scope: repair P46 S12 route cleanup and classify the remaining selector
failure.

Actions:

- Updated service-health reconciliation to preserve newer remote-view release
  mutations for display allocations, routes, and route-pool entries.
- Updated normal close cleanup to release session-owned display allocations and
  routes when process-exit reconcile removed the browser row before close
  persistence.
- Added regression coverage for the absent-browser close race.
- Rebuilt and converged the installed runtime after closing stale daemon
  listeners.
- Ran S12 against installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Classified the remaining S12 failure as a harness selector defect and patched
  S12 to prefer the current cycle's `tab new` result by service tab ID or exact
  returned index and URL.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml close_releases_session_owned_route_after_process_exit_removed_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::service_health::tests:: -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `pnpm converge:local-runtime -- --apply --json`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`

Result:

- Third S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T00-43-57-985Z`.
- The run completed ten cycles with zero active incidents at all boundaries,
  route-pool baseline true after every reset, no post-reset pressure increase,
  and direct Guacamole HTTP 200 in every cycle.
- The only failures were switched-tab URL assertions caused by stale positional
  tab selection under repeated-cycle tab accumulation.
- S12 is unlocked for one selector-repaired retry.

## Turn 70 | 2026-06-27

Scope: clear P46 S12 after selector repair.

Actions:

- Reran S12 with the selector-repaired harness against
  `/home/ecochran76/.local/bin/agent-browser`.
- Captured final install doctor, remote-view doctor, and incident-summary
  evidence.
- Updated the P46 plan and execution note to mark S12 cleared.

Validation run:

- `node scripts/run-p46-stress-scenario.js --scenario s12 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- S12 pass artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`.
- The pass reports `requireExplicit: true`, `explicit: true`, and daemon
  realpath matching passed.
- All ten cycles completed with route-pool baseline true after every reset.
- Active incidents stayed zero at every boundary and reset point.
- Post-reset pressure did not increase; checked-out route-pool, active
  remote-view routes, sessions, browsers, and tabs were zero after every reset.
- Direct Guacamole returned HTTP 200 in every cycle.
- Final install doctor succeeded with no issues and installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- P46 S12 is cleared.

## Turn 71 | 2026-06-27

Scope: close P46 after auditing completion criteria.

Actions:

- Re-read the P46 plan closeout criteria and audited current evidence against
  each required proof point.
- Updated the P46 plan state to `COMPLETE`.
- Added the missing S9 through S12 entries to the current execution ledger.
- Added the campaign summary, residual risks, and next hardening target to the
  P46 plan and execution note.
- Captured fresh final service status after the S12 pass.

Validation run:

- `~/.local/bin/graphiti-runtime doctor`
- `~/.local/bin/graphiti-runtime discover --group-id agent_browser_main --max-facts 8 --max-nodes 5 --max-episodes 5 "agent-browser P46 plan 0046 S12 route cleanup selector repair final closeout residual risk next hardening target"`
- `/home/ecochran76/.local/bin/agent-browser --json service status`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`

Result:

- P46 is complete through S12.
- Final service status artifact:
  `/tmp/agent-browser-p46-final-service-status.json`.
- Final install doctor artifact:
  `/tmp/agent-browser-p46-final-install-doctor.json`.
- Final remote-view doctor artifact:
  `/tmp/agent-browser-p46-final-remote-view-doctor.json`.
- Final incident summary artifact:
  `/tmp/agent-browser-p46-final-incidents-summary.json`.
- Final route-pool readiness artifact:
  `/tmp/agent-browser-p46-final-route-pool-readiness.json`.
- Final service status reported zero service browsers, zero service sessions,
  zero tabs, and zero active incidents.
- Final install doctor succeeded with no issues and runtime status `converged`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- Final route-pool readiness succeeded.
- Route-pool entries `guacamole-rdp-a` and `guacamole-rdp-b` were available
  with no current route allocation.
- Residual risk: historical orphaned display-allocation records remain visible
  in service status, but they are not live control rows and do not hold
  route-pool capacity.
- Next hardening target: retained-state compaction and doctor-surface cleanup
  for historical orphaned display allocations and stale metadata visibility.

## Turn 72 | 2026-06-28

Scope: plan the retained display-state compaction follow-up after P46.

Actions:

- Created `docs/dev/plans/0065-2026-06-28-retained-display-state-compaction-plan.md`.
- Scoped P65 to classify, explain, and safely compact retained historical
  display-allocation metadata without weakening P46 route-pool or live-control
  guarantees.
- Reused prior retained-state cleanup conventions: dry-run before apply,
  service-owned actions only, no manual service-state edits, and doctor/readback
  proof after live cleanup.

Result:

- P65 is `PLANNED`.
- First implementation slice should add the retained display-state classifier
  and focused tests before adding apply behavior.

## Turn 73 | 2026-06-28

Scope: execute P65 retained display-state compaction.

Actions:

- Added retained display-allocation classification to service state model.
- Extended `service prune-retained` with `--display-allocations` dry-run/apply.
- Added `retainedDisplayAllocations` to service status JSON and text output.
- Updated service request/status contracts, generated client types, README,
  docs site pages, and `skills/agent-browser/SKILL.md`.
- Rebuilt and installed the local debug binary, ran
  `pnpm publish:local-dashboard -- --skip-browser --json`, and removed two
  stale deleted-executable default daemon listeners reported by install doctor.

Validation:

- `cargo test --manifest-path cli/Cargo.toml service_prune_retained -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_classifies_display_allocations -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_service_status_via_actions_does_not_launch_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_status_text_includes_profile_and_session_summaries -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_and_collection_response_contracts_match_wire_shape -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`

Live proof:

- Artifact directory:
  `/tmp/agent-browser-p65-retained-display-20260628T174225Z`.
- `display-prune-dry-run.json` reported zero apply-safe display allocation
  candidates; apply was skipped.
- Final status retained 22 display allocations: 16 `diagnostic-retained`, 6
  `live`, 0 apply-safe.
- `final2-incidents-summary.json` reported incident count 0.
- `final2-install-doctor.json` succeeded with no issues.
- `final2-remote-view-doctor.json` succeeded with status `ready`.
- `final2-route-pool-readiness.json` succeeded with status `ready`.

Result:

- P65 is complete.
- No retained display allocation compaction is needed until a future dry-run
  reports apply-safe candidates.

## Turn 84 | 2026-07-06

Scope: continue P69 Slice C by sharing one acquisition-result builder.

Actions:

- Added `remote_view_handoff::shared_profile_acquisition_result` as the common
  JSON constructor for shared-profile acquisition evidence.
- Rewired route-bound `remote_view_open` shared acquisition records and
  `tab_new_shared_acquisition_evidence` in `cli/src/native/actions.rs` through
  that one builder.
- Extended the existing tab evidence tests to assert common acquisition-result
  fields such as `duplicateProcessPolicy`, `plannedProfile`, and
  `routeHintFields`.
- Updated P69 to narrow the remaining shared acquisition-result gap to plain
  remote-headed `open`.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml tab_new_shared_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- HTTP/MCP-routed `tab_new` responses and route-bound `remote_view_open`
  responses now share the same acquisition-result constructor. P69 remains open
  for plain remote-headed `open` convergence and Slice F live proof.

## Turn 83 | 2026-07-06

Scope: continue P69 Slice C shared acquisition-result convergence.

Actions:

- Added route-bound `sharedAcquisition` records to `remote_view_open` planned
  and opened responses through
  `remote_view_handoff::route_bound_handoff_shared_acquisition`.
- Kept `routeBoundHandoff` as the detailed route/display/operator-visible proof
  surface while exposing the same top-level acquisition-result name already
  used by access-plan and service-request tab responses.
- Extended `summarizeServiceSharedProfileAcquisition()` in
  `packages/client/src/service-request.js` to summarize route-bound
  `remote_view_open` responses from `data.intent`, `data.sharedAcquisition`,
  nested `data.tab`, and nested `serviceTabHandle`.
- Added focused Rust and service-client coverage for route-bound shared
  acquisition summaries.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/remote_view_handoff.rs packages/client/src/service-request.js scripts/test-service-request-client.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- P69 Slice C now has a common named acquisition-result record on
  `remote_view_open` responses and on the existing access-plan/tab response
  path. Remaining Slice C work is to route plain remote-headed `open`, HTTP
  `service_request`, and MCP `service_request` through that same acquisition
  result as a real shared planning artifact, not just a response field.
- P69 Slice F live proof remains open.

## Turn 82 | 2026-07-06

Scope: continue P69 Slice C retained-browser failure cleanup deepening.

Actions:

- Added handoff-owned cleanup decision helpers in
  `cli/src/native/remote_view_handoff.rs` for route-bound failure recovery:
  close only the opened tab for a reused retained browser, close a newly
  launched browser, or skip cleanup when no opened-tab index is available.
- Rewired `remote_view_open_cleanup_after_failure` in
  `cli/src/native/actions.rs` so the dispatcher executes the selected async
  browser command while the handoff module owns cleanup decision and result
  vocabulary.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns failure-cleanup recovery decisions for retained versus newly launched
  browsers, but the full shared acquisition-result routing across
  `remote-view open`, plain remote-headed `open`, HTTP `service_request`, and
  MCP `service_request` remains open.
- P69 Slice F live proof remains open.

## Turn 81 | 2026-07-06

Scope: continue P69 Slice C begin-acquisition lease reservation deepening.

Actions:

- Moved route-bound begin-acquisition lease reservation into
  `cli/src/native/remote_view_handoff.rs` behind
  `begin_route_bound_handoff_acquisition`.
- Rewired the `remote_view_open` begin-acquisition adapter in
  `cli/src/native/actions.rs` to supply the observation timestamp and
  provider-derived default control-input adapter, while the handoff module now
  owns pending route-pool, display-allocation, route, and lease repository
  mutations.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns begin-acquisition reservation, planned/opened response assembly,
  cleanup/rollback summary reporting, acquisition completion, lease
  restoration, rollback mutation, and cleanup update mutation. Retained-browser
  recovery sequencing and shared acquisition-result routing remain open.
- P69 Slice F live proof remains open.

## Turn 80 | 2026-07-06

Scope: continue P69 Slice C acquisition lease lifecycle deepening.

Actions:

- Moved route-bound acquisition completion, lease restoration, rollback
  mutation, and post-cleanup rollback update into
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` helper adapters in `cli/src/native/actions.rs` to
  delegate those service-state mutations to the handoff module while retaining
  timestamp generation and browser/repository orchestration in the dispatcher.
- Kept begin-acquisition lease reservation in `actions.rs` for now because it
  still constructs a pending `RemoteViewRoute` with the local
  `default_control_input_provider` helper. That is the next obvious Slice C
  sequencing move.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly, cleanup/rollback summary reporting,
  acquisition completion, lease restoration, rollback mutation, and cleanup
  update mutation. Begin-acquisition reservation and retained-browser recovery
  sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 79 | 2026-07-06

Scope: continue P69 Slice C handoff cleanup reporting.

Actions:

- Added `route_bound_handoff_cleanup_summary()` to
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` failure paths in `cli/src/native/actions.rs` to
  use the handoff module's cleanup summary for rollback and cleanup reporting.
- Removed the duplicate local cleanup-summary formatter from `actions.rs`.
- Added handoff-module coverage for cleanup summary shape.
- Updated P69 with the new Slice C progress. Repository rollback mutation still
  remains in `actions.rs` for the next deeper sequencing pass.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly and cleanup/rollback summary reporting.
  Full acquisition, finalization, rollback mutation, and retained-browser
  recovery sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 78 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `planned_route_bound_handoff_response()` and
  `opened_route_bound_handoff_response()` to `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` dry-run and opened success paths in
  `cli/src/native/actions.rs` to call the handoff response builders instead of
  assembling the authoritative profile, browser, session, route, display, tab,
  operator-visible proof, and verification fields in the command dispatcher.
- Preserved existing response shape for dry-run and opened handoffs while
  concentrating that shape behind the handoff module interface.
- Fixed `parse_remote_view_open` to preserve global `--browser-build` into
  the `remote_view_open` command payload. The broader `remote_view_open_`
  filter caught this as a P69 flag-preservation regression.
- Updated P69 and the routing-failure note.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly. Full acquisition, finalization,
  rollback, and retained-browser recovery sequencing still remains to move
  behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 77 | 2026-07-06

Scope: continue P69 Slice E shared-profile client ergonomics.

Actions:

- Added generated TypeScript declaration coverage for
  `ServiceSharedProfileAcquisitionSummary`.
- Added `summarizeServiceSharedProfileAcquisition()` to
  `@agent-browser/client/service-request`.
- The helper accepts either an access-plan response or a tab response and
  returns compact requested profile, planned profile, runtime profile, profile
  id, retained browser/session route hints, tab/target ids, service tab handle,
  acquisition mode, route-hint requirement, and duplicate-process policy.
- Added service-request client tests for summaries from both access-plan
  `decision.profileReuse.sharedAcquisition` and tab response
  `data.sharedAcquisition`.
- Updated P69, the routing-failure note, README, docs site service-mode
  guidance, and the agent-browser skill so software clients use the helper
  instead of parsing raw profile reuse state.

Validation:

- `node scripts/generate-service-request-client.js`
- `pnpm test:service-request-client`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`

Result:

- P69 Slice E shared-profile client ergonomics is implemented. P69 remains
  open for Slice C's full handoff-module sequencing and Slice F live proof.

## Turn 76 | 2026-07-06

Scope: continue P69 Slice D workspace inventory actionability.

Actions:

- Added `WorkspaceProfileActionability` to the dashboard workspace inventory
  projection.
- Marked compatible live service-owned retained browser rows with
  `openSharedProfileTab` and enabled their `add-tab` action as the recommended
  shared-profile operation.
- Marked profile-only lock rows with `waitForProfileHolder` or
  `rejectDuplicateProcess` so the dashboard distinguishes agent-browser-owned
  retained profile sharing from unknown or incompatible profile holders.
- Surfaced profile actionability in workspace navigator search and selected
  row detail.
- Wired the service-owned browser row `add-tab` action to HTTP
  `service_request` `tab_new` using the retained owner route hints, then
  refreshed service status and selected the returned browser/tab identity.
- Added viewer-controller lease and route-switch actionability to the same
  workspace inventory interface. Those rows now recommend `takeOverViewer` or
  `routeSwitch`, carry the lease or attachability reason, and keep `add-tab`
  disabled when opening another shared-profile tab is not the correct
  operation.
- Updated P69, the routing-failure note, README, dashboard docs, and the
  agent-browser skill.

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check -- packages/dashboard/src/lib/service-workspaces.ts packages/dashboard/src/components/workspace-navigator.tsx scripts/test-dashboard-workspace-nodes.js scripts/test-dashboard-workspace-navigator.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md RUNBOOK.md README.md docs/src/app/dashboard/page.mdx skills/agent-browser/SKILL.md`

Result:

- P69 Slice D no-launch workspace inventory actionability is implemented. The
  inventory projection now carries the retained-owner versus duplicate-process
  distinction, viewer takeover, and route-switch recommendations, and the
  executable service-owned browser `add-tab` dashboard flow uses the
  service-request tab creation path.
- P69 remains open for Slice C's full handoff-module sequencing and Slice E/F
  contract, client, and live proof work.

## Turn 75 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `cli/src/native/remote_view_handoff.rs` as the named module for the
  route-bound handoff proof record.
- Wired `remote_view_open` dry-run and success responses to publish
  `routeBoundHandoff` with one authoritative profile, browser, session, tab,
  route, display, and operator-visible proof surface.
- Reworked operator-visible proof failure diagnostics to include a focused
  `routeBoundHandoff` failure record for the failing route binding. Final
  post-checkout proof failures keep pre-checkout evidence separately labeled
  instead of blending it into the final proof record.
- Updated P69 and the profile-routing failure note to record Slice C progress
  and keep the remaining full handoff-module sequencing work open.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is partially implemented. The response proof vocabulary is now a
  named module, while the larger plan/acquire/finalize/rollback sequencing
  still needs to move behind the handoff module interface.

## Turn 74 | 2026-07-06

Scope: continue P69 shared-profile routing and service-request parity.

Actions:

- Added an access-plan-owned helper that applies shared-profile route hints to
  `tab_new` service requests when a compatible retained same-profile browser
  already owns the requested runtime profile.
- Wired HTTP `POST /api/service/request` through that helper with the live
  service state and taught non-focus relay to honor synthesized top-level
  command hints while continuing to ignore `params.sessionName`.
- Wired MCP `service_request` through the same persisted-plus-configured service
  state used by `agent-browser://access-plan` and routed hinted requests to the
  owner daemon session.
- Extended MCP `service_request` command/schema handling for
  `runtimeProfile`, `profileId`, `profile`, `browserHost`,
  `viewStreamProvider`, `controlInputProvider`, and `displayIsolation`.
- Repaired service-request contract drift for access-plan planned tab requests
  by adding `profileClass` to the JSON schema, generated client, HTTP adapter,
  and MCP adapter.

Validation:

- `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:service-request-live`
- `pnpm test:service-client-contract`
- `pnpm test:service-request-client`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice B now covers plain navigation plus HTTP/MCP `service_request`
  `tab_new` route-hint parity in no-launch tests and live service-request
  smoke proof.
- The live smoke opened two same-profile service tabs through one retained
  browser, released one physical target, preserved the browser/session route,
  and successfully evaluated the surviving tab handle.
- Remaining P69 work starts at Slice C route-bound handoff deepening.
# Turn 127 | 2026-09-02

Scope: bind retained-browser reuse to durable browser-build and executable
proof.

Current evidence:

- The installed service default and ready manifest select
  `stealthcdp_chromium`.
- The healthy retained `session:default` process is bundled Linux Chromium and
  its browser record has no build or launch proof.
- Access-plan reuse and daemon auto-attach currently accept that browser by
  profile and host alone.

Safety boundary:

- Do not navigate or close the retained FigureLabs target.
- Do not launch a duplicate profile lane.
- Treat missing or mismatched retained build proof as incompatible whenever the
  resolved request requires a browser build.
- When `stealthcdp_chromium` is required for a fresh launch, require
  `browserCapabilityLaunch.applied=true`; otherwise stop before browser start
  and repair the preflight binding, compatibility, or validation evidence.
- Keep ordinary non-service direct attachment behavior when no governed build
  is selected.

Plan: `docs/dev/plans/0127-2026-09-02-build-bound-retained-browser-reuse-plan.md`.

# Turn 128 | 2026-09-02

Scope: close the live Plan 0127 build-bound reuse blockers and clean the
worktree without replacing the authenticated browser.

Completed:

- Validated the exact `stealthcdp-win-150` executable binding and retired stale
  daemon listeners while preserving the retained browser.
- Allowed explicit `attached_existing` retained browsers without a local PID
  only after exact live daemon, CDP, browser, session, profile, target, URL, and
  handle verification; locally owned browsers remain PID-bound.
- Persisted publisher browser-smoke policy through recover-only handoff.
- Derived service-tab profile identity from the access-plan command before the
  manager fallback, with no ambient-environment fallback.
- Installed SHA-256
  `2dff309146ce62e2383ceaf6c2fe9169d92093049b5b29b9e3cb6260a5e2c9fe`;
  the publish journal reached `recovered_ready` and workstation doctor reported
  converged state with no issues.
- Proved a no-prompt broker acquisition returned the retained
  `session:default`, profile `chatgpt-pro`, exact target URL, and target-bound
  handle, then released the test tab without closing the browser.

Validation:

- Rust: 1,906 passed and 57 ignored; strict Clippy and formatting passed.
- Focused attached-existing, service-tab profile, retained-browser requirement,
  and publisher-recovery tests passed.
- ESLint, service-client checks, HTTP/MCP parity, route-confusion gates, docs and
  dashboard builds, native release build, plan audit, CodeGraph sync, and diff
  hygiene passed.

Remaining review boundary:

- AuraCall response `resp_f08eb21d87c84f1db2756f646beb7ec8` was created once,
  but its API restarted while waiting and the durable lease expired. Recovery
  then failed with `runner_execution_failed` because its submitted WebSocket was
  closed, explicitly refusing prompt replay. Read-only retained-page inspection
  found no matching nonce-bound user message. Preserve the response identity and
  do not resubmit or launch a substitute browser.

# Turn 129 | 2026-09-07

Scope: Plan 0128 autonomous headless execution, first checkpoint.

- Verified the initial clean worktree at `8e1ca4bb` and queried current broker
  access planning for bwkuehl.com.
- The disposable session `bwkuehl-headless-qa-20260907`, profile
  `bwkuehl-public-qa-20260907`, launched Linux Chromium headlessly but failed
  before navigation: capability metadata omitted the proven stock build when
  no preference binding matched. Preserve this exact probe for the routing fix.
- CLI chat now bounds gateway and tool work by the remaining turn deadline,
  reports exhausted steps as unfinished, rejects incomplete streamed tool calls,
  and stops on unknown tool outcomes.
- Three Rust local-HTTP stream tests and the real CLI fixture
  `node scripts/test-cli-chat-completion.js` passed. The fixture verifies 50-round
  exhaustion, truncated stream rejection, and normal completion without browser
  launch or an external model request.
- The full headless objective remains active. Dashboard parity, fresh-launch
  proof, task recovery, authentication, and live task outcomes are unverified.

# Turn 130 | 2026-09-07

Scope: Plan 0128 bounded blocker repair and clean-worktree checkpoint.

- Started clean at `8f4acea3`. Added exact fresh-owned installer Chrome
  provenance in `native/cdp/chrome.rs`, exposed it through `native/browser.rs`,
  and persisted it in both successful fresh-launch paths in `native/actions.rs`.
  Two focused regressions pass, including repository roundtrip and exclusion of
  custom, attached/no-process, stealth, missing-installer, and invalid-registry
  proof. No existing browser is relabeled.
- Dashboard Chat now errors on incomplete streams, 50-round exhaustion, tool
  timeout, and client disconnect. The whole turn, including compaction, has one
  deadline. Seven isolated socket regressions pass; the existing actual-CLI
  local gateway fixture passes all three scenarios.
- Delegated only the disjoint dashboard Rust file to
  `/root/dashboard_completion`, which completed its seven tests and returned no
  blocking findings on a closed-world review of the launch patch. Primary
  inspected the dashboard diff and owns combined validation.
- Native build, strict Clippy, formatting, targeted ESLint, dashboard TypeScript
  check, and diff check passed. CodeGraph synchronized with no pending changes;
  direct source reads filled unsupported graph queries. Planning audit reports
  `ok: true`, `applicable: false`, not independent plan acceptance.
- A first full isolated Rust run had 1916 passes, 57 ignored, and one child
  executable spawn failure in the large-output doctor fixture during concurrent
  Rust validation. The noncompeting full rerun passed: 1918 passed, zero failed,
  57 ignored. A final focused rerun covers the platform-canonical path assertion.
- Read-only deployment preflight fails `retained_daemon_missing` for the required
  `nyse-developer` lane. No pin removal, replacement browser, runtime installation,
  service restart, ChatGPT submission, or GitHub write was performed. Publication
  journal is already terminal `recovered_ready`; recovery is not the remedy.
- Local public tunnel unit is disabled/inactive; its loopback origin still
  listens on 8787. No public ingress change was authorized or attempted.

Next: restore the exact required retained lane, or explicitly approve retiring
its obsolete requirement, before guarded installation and fresh-launch live QA.
Durable task recovery and full autonomous/authenticated task coverage remain
separate unverified Plan 0128 work.

# Turn 131 | 2026-09-07

Scope: Plan 0128 CLI interactive compaction deadline closure.

- Started clean at `6ed97936`; classified the prior goal turn as verified
  implementation progress. Inspected current source and confirmed interactive
  CLI compaction ran outside the turn deadline.
- Moved compaction into `run_chat_turn`, using its existing absolute deadline.
  A timeout preserves history and returns before gateway tool work; success
  preserves system instructions and recent messages. No automatic retry occurs.
- Five CLI Rust tests pass, including two new socket regressions for stalled
  summary/history preservation and successful compaction. The actual built CLI
  completion fixture passes its three scenarios without external model calls.
- Full isolated Rust suite: 1920 passed, zero failed, 57 ignored. Native build,
  strict Clippy, formatting, targeted ESLint, dashboard TypeScript, and diff
  checks passed. Plan audit reports no problems but is not applicable to this
  repository's adopted planning contracts.
- `/root/dashboard_completion` performed a read-only closed-world review and
  returned no blocking findings; primary owns implementation and validation.
- Updated README, CLI help, skill, docs page, and Plan 0128 in the same slice.
  CodeGraph was synchronized; direct source reads covered exact control flow.
- Repeated read-only installation preflight still fails
  `retained_daemon_missing` for `nyse-developer`. No installation, pin changes,
  service restarts, account access, ChatGPT submission, or GitHub writes occurred.

This closes another bounded-wait gap, not the full autonomous-browser objective.
Durable task recovery and representative live task verification remain open.

# Turn 132 | 2026-09-07

Scope: user-authorized retirement of the obsolete NYSE installation requirement
and guarded local installation of commits through `2164f6d8`.

- Verified the exact NYSE requirement digest, then moved only its requirement
  and enforcement files into the private recoverable directory
  `~/.agent-browser/publications/retired-nyse-requirement-20260907/`.
  Profile/login data and the separate Workshop requirement were not changed.
- Publisher completed transaction
  `local-dashboard-d5a0771c-0bd8-4b8d-a0e2-9118711be673` as terminal `ready`.
  Installed and live dashboard executable SHA-256:
  `d2183f7679e4261f45305e5d1a3e9541f911a643b064169b3d4e1603d9b8fe09`.
  Previous executable is recoverable from the publisher's verified backup.
  Dashboard HTTP/bundle/manifest checks and three installed-CLI fixture scenarios
  passed. Browser smoke was deliberately skipped; no live autonomy claim.
- Workstation apply refreshed payload provenance, then failed opening the
  canonical Guacamole displays. The route viewer inherited a stealth build
  requirement without a matching binding. Source shows the installer pins a
  Linux viewer executable but the helper does not pin the matching browser build.
  The report-only helper still attempts route opening; it failed before launch.
- Install doctor subsequently returned exit zero with no issues and payload
  ready. Remote-view doctor still lists missing X11 displays 10 and 11 despite
  returning exit zero: full workstation reconciliation is not complete.
- Restored the previously serving dashboard after installer quiescence; repeated
  HTTP/manifest smoke passed. Restored the PostgreSQL backup timer. Runtime
  interlock remains paused pending route-launcher repair to avoid retry churn.
  No public tunnel changes, account login, ChatGPT prompt, or GitHub writes.

Next: bind the installed Guacamole viewer's executable and browser-build identity
together, add regression coverage, rerun workstation reconciliation, and then
perform the pending headless rendered-page verification.

# Turn 133 | 2026-09-07

Scope: fix the workstation viewer blocker and finish live reconciliation.

- `21d8ec18` binds the installer-selected stock Chrome executable and build
  identity on viewer launch, header setup, and navigation. The global browser
  preference and private authentication data are unchanged. The real command
  builders pass `scripts/test-route-viewer-build.js`; invalid build names reject
  before execution. Installer selection and update behavior have Rust coverage.
- Closed the failed Route A service attachment, then verified and gracefully
  terminated its remaining legacy installer-owned Chrome PID by UID, executable,
  and exact profile path. Profile data was preserved. Both route desktops then
  reached ready. The initial identity check deliberately stopped when the
  executable differed; inspection identified managed Chrome 149, not 152.
- `285538b5` fixes the subsequent final-doctor startup failure: at most three
  attempts for a sole unreadable dashboard manifest, with no service restart or
  relaxation of hash/other failure checks. The 22 installer tests, packaged
  source-free fixture, strict Clippy, formatting, targeted ESLint, TypeScript,
  build, and diff checks passed. CodeGraph was synchronized. Planning audit is
  clean but not applicable. Independent bounded reviews returned no blockers.
- Installed runtime SHA-256:
  `09288ffad7a132b888eea6e79d47d49d6302599bb39310b6204eea058d64afda`.
  Publisher transaction `local-dashboard-d2df51c6-1c1a-457f-9a6e-6f429ed924f2`
  finished ready and preserved both new viewer browser PIDs/CDP endpoints.
- Final `agent-browser install workstation --apply --json` exited zero with
  `success: true`, `complete: true`, `state: ready`, and `ready: true`.
  Durable local receipt: `~/.agent-browser/convergence/workstation-latest.json`.
  This supersedes Turn 132's incomplete reconciliation and paused watchdog.
- No GitHub writes, public tunnel changes, ChatGPT prompts, or user-profile
  deletion. This resolves the workstation blocker, not the entire autonomous
  browser objective; representative headless task verification remains next.

### Session continuation checkpoint | 2026-09-07

- Plan 0128 follow-through preserves the proven active build when only the global
  launch default differs. Explicit/site/profile/registry constraints still bind;
  this is not a new launch fallback or an exemption for missing build proof.
- MCP missing/refused retained routes return `retained_daemon_unavailable` and
  recovery guidance. No daemon startup or additional replay is added.
- Validation: 1928 isolated Rust tests passed, 57 ignored; format, strict Clippy,
  debug build, API/MCP parity, client contract/types, docs build, targeted ESLint,
  actual no-launch MCP fixture, and diff check passed. Planning audit is clean but
  not applicable. Independent scoped review returned no blocking findings.
- Reproduce the no-launch fixture with
  `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=/absolute/path/to/built/agent-browser node scripts/test-mcp-retained-route-unavailable.js`.
- Concurrent installer-recovery work is preserved. This slice is source-verified,
  not installed or published; reconcile that work before the next runtime update.

### Private credential broker checkpoint | 2026-09-08

- Plan 0130 implements only internal encrypted staging and durable per-reference
  admission in `cli/src/native/private_secret_store.rs`, registered by
  `cli/src/native/mod.rs`. No private browser command or public contract is
  exposed. The installed runtime and retained SAM/Workshop targets are untouched.
- Primary validation: nine focused tests; final full isolated Rust run 1937
  passed, 57 ignored; formatting, strict Clippy, debug build, ESLint, service
  client typecheck and diff checks passed. Planning audit has no problems but is
  not applicable. CodeGraph is synchronized. Exact source/build digests and
  independent delegation/review receipts are in Plan 0130.
- Independent review caught directory-enumeration errors being treated as EOF;
  fixed with explicit errno handling and unsupported-platform rejection.
  Concurrent independent-open and subprocess-exit tests reject reference replay.
  This is not browser at-most-once proof or cross-reference backup-code dedup.
- Live execution remains blocked: raw inspect children survive listener shutdown;
  background stream/replay paths bypass command guards; cancellation and recovery
  need a persistent shared privacy lock; existing handle validation is not full
  live identity proof. These are accepted B1-B4 in Plan 0130, not new permission
  requests. Do not use ordinary fill/evaluate/password-stdin for this SAM test.
- Next engineering slice: shared identity-bound observer/input barrier with
  cancellation/restart protection and exact-handle/origin enforcement, then
  synthetic sentinel tests before enabling private transport and installing.
  No Slack secrets, backup codes, API-key renewal, ChatGPT prompts, GitHub writes,
  or runtime installation occurred. Unrelated dirty RDP-route work is preserved.

### Private credential barrier continuation | 2026-09-08

- Plan 0130 now implements a cooperative endpoint-scoped OS privacy barrier,
  integrated into CDP command/observer paths, viewer replay/input, dispatcher
  admission and attached-browser recovery. Pending command receipts are durable
  before dispatch and survive cancellation/process death. Private locks have no
  automatic reset. Untracked raw traffic blocks private admission as uncertain.
- Inspect proxy connections are owned, aborted and joined during awaited
  shutdown. A rejected close/handoff cannot exit the daemon. Pure exact-target
  checks reject a healthy same-profile handle at a different requested URL.
- Final isolated suite: 1964 passed, 57 ignored; formatting, strict Clippy,
  ESLint, client typecheck, native debug and docs builds, and diff checks passed.
  Planning audit is clean but not applicable; CodeGraph is current. Synthetic
  viewer/CDP tests prove cached/live frame and event silence, input rejection,
  reconnect refusal and restart-persistent locks. Plan 0130 records exact source
  hashes, file ownership and independent review receipts.
- Not installed or live accepted: aliases/tunnels/external clients are not
  unified; no sanitized unlock/reconciliation or private secret transport exists;
  the identity helper still needs fresh execution-time observations. Matching
  CDP responses do not prove deferred page work stopped. Do not delete receipts
  to bypass these gates. Runtime-path and userinfo-URL compatibility constraints
  are documented in help, README, skill and security docs.
- Next: verified retained-browser authority across alternate routes and safe
  recovery, then private execution integration. No real Slack credential,
  backup code, SAM renewal, ChatGPT submission, install or GitHub write. Existing
  RDP work and retained browser sessions were preserved.

### 2026-09-08: private recovery transport checkpoint, not live acceptance

- Added permit-bound reconnect using the already locked store, authenticated
  recovery epochs, fixed-label private CDP errors and isolated-world page cleanup.
  Cleanup keeps observation locked even when the retained page is empty.
- Independent review identified delayed private messages after unlock and
  incomplete mutation-target binding. Both block live enablement. Alternate CDP
  routes also still need trusted broker identity mapping. Do not delete locks,
  replay submissions or freeze unrelated browser lanes to bypass these gaps.
- Primary validation: 6 focused CDP tests and 1971 full Rust tests passed,
  57 ignored; formatting, strict Clippy, ESLint, client typecheck and docs build
  passed. Planning audit reports no problems but is not applicable. CodeGraph
  refreshed with 482 files. Plan 0130 retains source hashes and review evidence.
- No installation, live browser mutation, credential retrieval, SAM renewal,
  LitScout credential update, ChatGPT prompt or GitHub write. Existing dirty
  worktrees and retained sessions remain preserved.

### 2026-09-08: private transport R1/R2 remediated, live integration pending

- Old CDP observer generations now remain revoked after private cleanup,
  including delayed responses/events not read during the private interval.
  Recovery sockets never gain public observation. Reader, command and inspect
  paths share the captured observer; new post-clean sockets capture a new epoch.
- Private admission requires a validated target token and persists its target
  and authorization digest. Recovery preserves scope. Private dispatch rejects
  unscoped permits and probes the actual session target before mutation; a
  sibling target cannot receive the command or authorize cleanup.
- Independent closed-world review passed R1/R2 by source inspection. Primary
  ran 9 CDP tests and the full suite: 1982 passed, 57 ignored. Formatting, strict
  Clippy, ESLint and service-client typecheck passed. See Plan 0130 for receipts.
- Live alternate-route broker binding and private executor integration remain
  unfinished. No runtime install, live credential use, SAM renewal, LitScout key
  change, ChatGPT prompt or GitHub write. Dirty worktrees and browsers preserved.

### 2026-09-08: daemon-owned private authority preflight

- `private_broker.rs` now rebuilds authority from broker records and probes the
  already-owned CDP page session. It rejects caller-provided authority, mismatched
  profile/session/browser/endpoint/target/URL, unhealthy or ambiguous targets,
  and expired or malformed configured leases. Cached embedded handles are ignored.
- This is preflight, not an enabled private executor. Revalidate under the private
  barrier before admission; alternate-route binding, private ingress/egress and
  replay-safe workflow coordination still gate installation and live credentials.
- No-launch checks retained the original LitScout login.gov tab. Its URL includes
  a request query; do not substitute the bare origin. The session has no expiry,
  so expiry does not explain differing lease counters. No browser changes or
  real credential operations occurred. See Plan 0130 for checkpoint evidence.

### 2026-09-08: locked admission and backup-code replay reservation

- Internal broker admission now composes daemon-owned preflight, scoped privacy
  lock and private-transport revalidation. Fresh identity must still match the
  original saved scope. Errors/cancellation keep the durable lock closed.
- Private storage now reserves a backup code and its exact reviewed source slot
  across restaging, new consent IDs and restart, using keyed fingerprints rather
  than raw values or plain code hashes. Canonical account/source provenance is
  still the future trusted operation parser's responsibility.
- These are integration components, not an enabled authentication executor.
  Alternate-route authority, private ingress/egress and cleanup coordination
  still gate runtime installation and the SAM.gov/LitScout end-to-end test.
  No real credentials or live browser/runtime state changed in this checkpoint.

### 2026-09-08: typed private single-operation execution

- New internal parser/executor connects encrypted staging to private isolated DOM
  execution and encrypted results. Login, backup, renewal and key-read operations
  have strict account/origin binding; durable admission precedes mutation.
- Effective form action/method/target checks prevent submitter overrides or base
  targets from leaking credentials into GET URLs or another window. Independent
  closed-world review accepted this fix by source inspection.
- Synthetic DOM and mock-CDP coverage is not live acceptance. Public entrypoints
  remain disabled pending trusted Slack ingress, consent-bound navigation across
  stages, alternate-route authority, cleanup and guarded LitScout egress.
- Do not use the old Slack search helper as a credential adapter: its desktop
  credential extraction and ordinary error/write paths are outside this private
  transport. No real Slack credential or SAM key was retrieved or changed.

### 2026-09-09: operator-approved local Slack bootstrap

- The approved private SABER channel is in Cochran Group (`polycy.slack.com`),
  workspace `TEG3AC109`, channel `C07CA08AKUH`, verified from Slack metadata only.
- Source helper `python3 scripts/setup-private-slack.py` uses hidden operator
  terminal input, probes only authentication identity/scopes, and creates an
  owner-only plaintext bootstrap file without replacing existing credentials.
  `groups:history` is required; the operator additionally approved canvas
  read/write. No other token scope is approved by this setup.
- This is not private executor installation or an end-to-end SAM login. Never
  print the connection file or ask for the token in chat. Use `--status` for local
  metadata and preserve the existing browser and terminal sessions.

- Follow-up live verification: the saved token passed a fresh constrained
  `auth.test` and matched the saved bot/workspace/scopes. No messages were read,
  no credential was printed and no config was rewritten. Twelve offline helper
  tests passed again. This verifies local Slack authentication only, not channel
  access, SAM login, renewal or LitScout activation; Plan 0130 retains evidence.

### 2026-09-09: bounded autonomous source discovery, not runtime activation

- The operator approved discovering relevant messages in the existing private
  SABER channel instead of manually supplying links. LitScout now supports a
  digest-bound discovery consent alternative with explicit account, browser,
  workspace/channel, timestamp window and page/message limits.
- `scripts/private-slack-discovery.py` is an internal, non-CLI library. It uses
  the approved dedicated bot, verifies fresh identity/scopes and walks bounded
  history without printing bodies. It returns exact candidate references/counts
  only after unambiguous explicit account-matched text and complete pagination.
- Missing completion metadata, changed scopes, ambiguous accounts/candidates,
  uninspected thread replies, retention limits and exhausted budgets fail closed.
  It does not fetch files/canvases, retry rate limits, widen scope, or use a model
  to interpret secret-bearing messages. Free-form text recognition remains limited.
- This helper is not wired into the installed daemon. Trusted consent-to-adapter
  integration, private extraction/staging, browser transitions/cleanup, renewal,
  guarded installation and a fresh consumer probe remain required. Do not run
  live credential discovery through ad hoc imports to bypass those gates.

### 2026-09-09: consent-to-private-discovery connection

Follow-up: the approved extraction and exact-navigation checkpoint is recorded
in Plan 0130. It adds an atomic broker-only approval claim, genuine in-process
extraction continuity and durable ordered destination advancement. Primary
validation passed 2,010 native tests (57 ignored), 140 LitScout tests, 6 private
adapter tests, 17 extraction tests and 12 bootstrap tests. These are source and
synthetic runtime receipts, not live renewal evidence. The installed executor,
private transport, cleanup and guarded consumer handoff remain unconnected;
renewal stays disabled and no live credential was used.

- Internal `scripts/private-sam-discovery.py` now connects an embedding service's
  authoritative LitScout context and approved digest to private Slack discovery.
  It opens the protected connection only after durable admission; it does not
  supply an operator-facing command, choose a DB, or approve a plan itself.
- LitScout's source schema adds `CredentialDiscovery`. Started/failed attempts
  are not retried; only in-scope refs/counts may be published, and publication
  atomically checks current approval. No live schema migration occurred.
- Private exact-source extraction now rejects edits after discovery and returns
  opaque in-memory material. It cannot serialize credentials into normal results
  or select/try a backup code. Password punctuation is literal; ambiguous
  boundary whitespace is rejected. Process-local extraction capabilities are
  consumed before I/O and cannot be rebuilt from saved public references.
- No renewal runtime is enabled. Encrypted staging, retained-browser transitions,
  verified privacy cleanup and the guarded key/consumer handoff remain required.

### 2026-09-19: mixed-generation retained-runtime convergence

- Fixed service-state compatibility so current writers preserve opaque
  `runtimeCustodyReceipts` required by an older live daemon.
- Added fail-closed schema-v2 handoff verification and two digest-bound internal
  recovery tools. The last-resort attached-existing path can revoke only an
  exact stale deleted daemon after proving the preserved browser, profile,
  loopback DevTools endpoint, target, and service projection; it never treats
  daemon staleness as authority to terminate or relaunch Chrome.
- Recovered the six retained runtime lanes without changing their Chrome
  processes. The final no-launch install doctor reported `ready`, runtime
  convergence `converged`, and zero stale runtimes. See
  `docs/dev/notes/2026-09-19-mixed-generation-runtime-custody-recovery.md` for
  the failure mode, safety boundary, and reproducible validation command.
- The isolated service file-transfer smoke then found and fixed explicit
  `executablePath` loss in auto-launch plus a test-only privacy-mode mismatch.
  The rerun passed exact upload/download digest and filename provenance checks,
  and the complete Rust suite passed after the fix.
