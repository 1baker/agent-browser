# Agent-browser Cloudflare and ChatGPT E2E evidence

## Defects repaired

- Retained-browser preparation previously collapsed four distinct identities: runtime profile, daemon session, retained browser id, and remote-view route-owner session. The preparation command now accepts and validates separate values for each, plus an optional route-pool entry.
- Remote-view reuse required the route-owner session to equal the daemon session. That condition was invalid for the deployed ChatGPT lane, where daemon `auracall-destination-smoke` owns browser `session:auracall-destination-smoke` and route owner `chatgpt-pro` owns Guacamole route 2. The reuse gate now binds the browser to the active daemon and relies on the already-completed acquisition plan to verify the distinct route owner.
- The full suite exposed a separate build-attestation regression: a later unproven health refresh could either erase a previously verified retained-build proof or, if preservation were too broad, conceal an explicit failed re-verification. Health reconciliation now preserves an existing verified proof only when the incoming refresh has no verification result; an explicit verification failure clears the build and path.
- Documentation and CLI help now explain the identity separation.

## Local verification

- Rust formatting passed.
- The canonical partitioned Rust suite passed: 1,483 parallel-safe tests plus every serial partition, including all 18 browser-parity tests, with zero failures.
- Clippy with warnings denied and Rust formatting passed on the final source.
- Focused managed-runtime CDP attachment and same-owner checked-out route reuse tests passed.
- Both opposing build-proof cases passed: an unproven metadata refresh preserves a verified build, while an explicit failed re-verification clears it.
- JavaScript lint and the retained-browser preparation fixture and command tests passed.
- JavaScript lint, both retained-browser preparation fixtures, and `git diff --check` passed.
- The release binary was rebuilt from the final reviewed source and published transactionally.

## Installed runtime and custody proof

- Reviewed source base: `1afdaa680411db2e4c15a86396ffa5435c92b192`; scoped implementation diff SHA-256 before this evidence-only update: `770c378bc3a580c26a63d56703003af8935c3e66384bd539f930a45d5cb22cc1`. The unrelated pre-existing `cli/src/commands.rs` modification is excluded from both the scoped diff and release commit.
- Final optimized executable SHA-256: `fd351626bb4240506251f8d081683269cae43d3ffc02844a63c5280bba0ad53b`. The build output, ignored workspace reference binary, and installed `/home/bak3r/.local/bin/agent-browser` all matched this digest byte-for-byte.
- Transaction `local-dashboard-a757fc74-fb29-4246-a5b6-6b434afd65f1` published that exact digest and ended at terminal phase `ready`; its replacement record reports expected digest equals actual digest and `verified: true`.
- Publication ended in terminal `ready`; the runtime interlock timer was restored.
- The exact retained ChatGPT Chrome process remained PID 2696783 with start ticks 7628344. Pre-mutation, post-handoff, and final-readiness checks all reported `retained_browser_exact_match`, the same CDP browser endpoint, profile `chatgpt-pro`, and the exact required target.
- Three active attached daemons were resumed on their pre-publication CDP endpoints; no retained browser was launched, killed, or replaced.
- The final installed doctor reports `ready`, zero issues, four converged runtimes, and zero stale runtimes.

## Inspectable authorization path and negative coverage

- `cli/src/native/actions.rs:14648-14667` permits reuse only when the requested browser id equals the daemon-derived browser id, that daemon already has a browser, and the acquisition plan contains the exact `route_pool_entry/same_owner_checked_out_route` authorization decision.
- `cli/src/native/actions.rs:14784-14894` proves sequencing: normalize exact browser and route-owner identities, build the plan from current persisted state, begin the durable acquisition lease, establish display access, and only then evaluate reuse. There is no reuse call before acquisition.
- `scripts/lib/local-dashboard-retained-browser-preparation.js:38-98` keeps route owner, daemon session, browser id, and route-pool entry separate, while requiring browser id to match the daemon session.
- Positive tests cover ordinary and split-identity reuse: `acquisition_plan_reuses_checked_out_same_owner_route`, `acquisition_plan_reuses_same_owner_checked_out_route`, `test_remote_view_open_dry_run_reuses_checked_out_same_route`, and `reused_browser_launch_result_records_route_owner_evidence`.
- Negative tests cover foreign ownership, exact pending-reservation mismatch, exhausted/ambiguous pool selection, browser/profile mismatch, route target mismatch, contention, unavailable routes, stale acquisition readiness, malformed Guacamole tokens, and failed public operator access. Representative tests are `acquisition_plan_rejects_checked_out_route_for_other_owner`, `acquisition_plan_accepts_only_its_exact_pending_reservation`, `acquisition_plan_reports_route_pool_exhausted_for_unpinned_checked_out_inline_entry`, `test_remote_view_route_checkout_rejects_pool_target_mismatch_and_contention`, and `test_active_browser_profile_mismatch_rejects_wrong_runtime_profile`.

## Live remote-view proof

- The first live attempt safely rolled back after exposing the route-owner/daemon-session reuse defect.
- After the correction and republish, `remote_view_open` succeeded against browser `session:auracall-destination-smoke`, daemon `auracall-destination-smoke`, route owner `chatgpt-pro`, and route-pool entry `guacamole-rdp-b`.
- A fresh post-final-publication open at 2026-09-22T14:21:30Z again returned `browserReused: true`, `browserLaunchRequested: false`, `tabAcquisitionDecision: reused_compatible_target`, route state `ready`, attachability `attached_ready`, display `:11`, and visible Chrome-window proof. It selected the exact `guacamole-rdp-b` entry and `guacamole:2` route for owner `chatgpt-pro` while controlling the daemon-owned browser.
- The public operator route is `https://agent-browser.bwkuehl.com/guacamole/#/client/MgBjAHBvc3RncmVzcWw=`.
- Public TLS validation returned verify result 0. Anonymous requests to `/`, `/guacamole/`, `/api/service/status`, and `/websocket-tunnel` all returned Cloudflare-served HTTP 302 redirects to `auth.bwkuehl.com`; none exposed dashboard, API, Guacamole, or tunnel content.
- The authenticated operator had already loaded the public page. The fresh route result independently reported the local authenticated Guacamole backend HTTP 200, public operator URL present, route/display/browser agreement, and visible Chrome window. This pairs the external authentication boundary with backend transport/readiness evidence without copying authentication cookies into test logs.

## Review request

Review the identity and reuse logic, build-attestation semantics, preservation evidence, and test sufficiency. Return the required strict JSON verdict. Do not infer installed-runtime identity until the final publication digest below is populated and checked against the installed executable.

## AuraCall target-binding remediation

- The first two create-once review attempts failed before provider input: one new-project request lacked an unambiguous cold-start browser, and one exact-conversation request exposed that AuraCall was pinned to closed target `15DC8CC5AC3AD247BCCA5655C2F808E4`.
- No failed response was resubmitted. AuraCall's configuration was changed only to the already verified retained target `466ED9475217ECF6B73089625AD6C1BA` and its exact matching conversation URL.
- All recent AuraCall runtime records were terminal before restart. The user service restarted successfully and loaded the corrected target binding.
- Review round 2 then completed through the retained browser as response `resp_idem_e3a08a92b930c3575e762d200c88a234`, proving post-restart submission and response readback on the pinned target. Its 74/100 fail identified the missing evidence now supplied in this revision; it was not resubmitted.
- After the review completed, ChatGPT returned the active retained tab to `https://chatgpt.com/` under new target `A7B97C577F5A83BEBC46D1AC0CA7EA62`. The final live remote-view check reused that exact target. It was then navigated back to the canonical review conversation without changing the target id; direct evaluation there proves `hasComposer: true` and `hasLoginLink: false`. AuraCall was repinned to target `A7B97C577F5A83BEBC46D1AC0CA7EA62` plus the canonical conversation URL and restarted before the final review round.
