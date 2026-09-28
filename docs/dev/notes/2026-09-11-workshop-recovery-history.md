# Workshop recovery history

This is the complete prior Workshop section moved from RUNBOOK.md during
documentation cleanup. Entries are historical evidence, not current runtime
authority; later entries can supersede earlier readiness claims. Consult the
short RUNBOOK status before performing an operation. No evidence was discarded.

## 2026-09-11 | Opt-in Workshop boot recovery

Exact Workshop restoration at 21:53:14 UTC: no active jobs or chatgpt-pro
browser existed. The no-launch broker plan selected chatgpt-pro, no conflicting
lease, and launch_new_browser. The fixed passwordless helper granted the
operator access to the already verified A/B route desktops. One installed
prepare-retained-browser call used the original exact conversation URL and
stale requirement digest
`1c52f77aa9edcb6efafa6675cbcb34ef50db54ae5e61e164eed3263bb72c1f54`.
It returned ready_and_pinned, exactly one matching candidate, session/profile
chatgpt-pro, target `4C4278BF20BBD5794A62582E9622E7D8`, and new requirement digest
`e38ea5c5bd9ff8230d63d69fe0cc45f1a3905726c8a736c95429cb266aa7fc97`.
Native no-launch readback independently verified the new pin. No prompt was sent.

Manual attempt reconciliation: the failed original attempt stopped at desktop
restore; the separate desktop operation established both routes, and the above
single preparation verified the unchanged logical conversation/profile/session.
Preserved the old pending receipt as private mode-0600
`state.reconciled-20260911-215314.json` in the recovery state directory, then
used the installed --arm entrypoint to create a fresh verified current-boot
receipt. The original receipt's pre-archive digest was
`c8de47b2f442b9b1d22b1191db34ee832f0924202f095e4f2790e4a7381e711b`;
archive preserves its JSON content, with newline normalization from apply_patch.
This was explicit postcondition-based operator reconciliation, not automatic
replay or deletion of failure history. Installed --check-readiness returned
dependencies_ready and retainedVerified=true. Publication retained-identity
preflight was rerun. No WSL restart or packaged runtime replacement occurred;
the staged desktop command-outcome source fix still awaits guarded installation.

Desktop-only live reconciliation at 16:50 CDT: the shared default/chatgpt-pro
browser and active jobs were absent in both Service inventory and the process
resource census. The failed boot receipt remained pending. After no-launch
access planning and exact A/B route dry-run, the main agent invoked the existing
installed desktop opener once, in the recovery helper's sanitized environment,
without invoking Workshop recovery. It returned exit zero and success=true;
route A and B both reported openedSuccess=true, reused=false and distinct
displays :10 and :11. Viewer PIDs 495544 and 496206 remained alive after helper
completion. These are the canonical separate custom viewer profiles, not the
chatgpt-pro profile. Pathname sockets were not visible in the caller namespace;
both Linux abstract X11 listeners were independently present. Fresh inspection
confirmed route-user Xorg PIDs 495951 (:10) and 496635 (:11), success=true.

The original boot-attempt receipt was preserved, and no Workshop preparation,
pin rotation, prompt, WSL restart or runtime publication was performed. This
proves desktop-only startup on the running host with the old installed opener;
it does not prove the staged command-outcome fix is installed, nor explain the
lost boot-time child error. Do not treat the ready viewers as completed browser
recovery. The next scoped operation is exact Workshop restoration and receipt
reconciliation, followed by guarded installation of the tested opener changes.

Desktop command-outcome repair (source only): isolated subprocess reproduction
proved that launch and header commands returning success=false with exit zero
could still produce overall ready. Navigation JSON was unchecked and timeout
was discarded; password-only authentication was rejected only after launching
the viewer. The opener now requires explicit success at each command, reports
navigation_outcome_unknown without retry on timeout, and checks supported
authentication before launch. No historical root cause is inferred from these
new reproductions; the earlier child error was lost.

`node --test scripts/test-route-desktop-startup.mjs` passes six cases, including
successful cold orchestration, three explicit rejections, unsupported auth and
navigation timeout. Every provider/browser command and display observation is
synthetic in an isolated HOME. Targeted ESLint passed. Independent read-only
review `/root/desktop_start_review` confirmed the command-success and timeout
defects; the main agent implemented the fix and independently ran the tests.
CodeGraph located the opener and its wrapper. The shared LitScout/ProposalWriter
browser and existing dirty cold-rollback work remain untouched. No install,
restart, real browser effect, receipt clearing or rearm belongs to this slice.
Widened primary validation passed all 38 desktop/recovery tests, the temporary
route subprocess fixtures, both retained requirement/live-pin fixtures, and
diff checks. Added `pnpm test:route-desktop-startup` for repeatable focused use.
The packaged opener cannot be edited in place without updating installed
provenance; this fix awaits guarded installation and real desktop-only proof.

Cold-start failure follow-up: the 14:26:19 CDT recovery failed at 14:26:22 with
`desktop_restore_failed`, before Workshop preparation. The old helper discarded
the child response on nonzero exit. No preserved child error was found; its
specific historical cause remains unknown. Do not represent the earlier
dependency preflight or systemd fixture as proof of cold desktop restoration.

Added and installed allowlisted command-failure diagnostics: category, exit code,
recognized signal/spawn error and JSON-response presence only. Raw stdout/stderr,
arbitrary errors and authentication payloads never reach the journal. Unknown
errors remain unclassified. Installed/source helper SHA-256:
`b73acf0d238e8a2b060fe2387a26f8f29321276c3559569cf804eb0c52f9a340`.
All 32 recovery tests passed, including real systemd lifecycle fixtures and a
production-entrypoint desktop failure proving safe reporting, durable receipt
preservation, one desktop dispatch and zero browser preparation/replay. Targeted
ESLint, source/installed parity, CodeGraph sync and diff checks passed. CodeGraph
located the desktop command wrapper; installed source was read for runtime detail.

Live no-launch dry-run in the recovery environment found both routes. Remote-view
doctor completed but reports blocked: route X11 sockets :10 and :11 are absent,
with two distinct desktop displays missing. Viewer prerequisites report ready.
This identifies current missing desktop sessions, not the lost original error.
The pending attempt and recovery receipt were not changed; updated helper bytes
also require reviewed rearming after reconciliation. No live desktop replay,
WSL restart, browser launch or prompt was performed. Next action is controlled
desktop-only failure reproduction after reconciling the uncertain attempt,
not another WSL restart. No delegation for this bounded diagnostic slice;
no compiled CLI surface changed.

Verified rearm follow-up: native no-launch retained status verified replacement
target `4DED51312C85F09289C5D643C5CA5757` in the same chatgpt-pro session/profile
and exact Workshop conversation. The installed artifact manifest had zero
changed or removed entries, source/installed helper and service matched, and
no uncertain attempt was present. Explicit installed `--rearm-verified` returned
`armed_for_next_boot`, preserving `state.json.before-rearm-1789154429935`.
The new requirement digest is
`1c52f77aa9edcb6efafa6675cbcb34ef50db54ae5e61e164eed3263bb72c1f54`.
Installed `--check-readiness` returned `dependencies_ready` and
`retainedVerified:true`; installed doctor exited zero with success=true.
The recovery timer is enabled and active; the loaded service retains
RemainAfterExit=yes and KillMode=control-group. No service start, WSL restart,
browser launch or prompt occurred. This clears the stale-receipt preflight
blocker, not the still-pending full restart acceptance. No delegation was needed
for this bounded operator rearm; prior independent lifecycle review still applies.

Service-lifetime correction: the second WSL recovery reported
`recovered_and_verified` at 12:24:06 CDT and its oneshot finished at 12:24:08.
The retained daemon was subsequently missing. The unit's defaults were
`RemainAfterExit=no` and `KillMode=control-group`. A real user-systemd fixture
reproduces detached-child termination on successful helper exit under those
settings. The corrected unit sets `RemainAfterExit=yes` and explicitly retains
`KillMode=control-group`: successful children remain under service custody,
repeated start is a no-op, and deliberate stop or failed startup cleans them up.
Never restart this service as a health check. Disable only its timer to prevent
future recovery without closing existing children. `active (exited)` is not
browser-health evidence.

Primary validation: `WORKSHOP_SYSTEMD_TEST=1 node --test` with all three
`scripts/test-workshop-boot-recovery*.mjs` files passed 30 tests, including the
real systemd old/fixed/failure lifecycle cases. Disposable fixture processes and
temporary files were cleaned up. Both existing retained requirement/live-pin
fixtures, targeted ESLint, systemd unit validation and diff checks passed.
CodeGraph was current and synchronized for the added test. The graph-discovery
skill was unavailable; read-only Graphiti retrieval supplied advisory history.
No compiled CLI behavior changed, so native help/build surfaces are unchanged.
The source and installed service match SHA-256
`9a91c0bdfeecb8760b133f15ae71ad772a9523a15277a2183761037d3451b6cf`.
User daemon-reload loaded the corrected properties without restarting recovery;
ExecMainStartTimestamp remained 12:23:41 CDT. No WSL restart or browser action
was performed by this slice.

Live handoff is blocked on changed authority, not the unit fix: a no-launch
native check now verifies the same Workshop conversation under target
`4DED51312C85F09289C5D643C5CA5757`, with requirement mtime 19:02:35 UTC. This
slice did not restore it. The older recovery receipt remains unchanged, and
installed `--check-readiness` refuses `authority_changed_during_readiness`.
Review that replacement target and explicitly rearm only after verifying it;
do not restart based on fixture success. No receipt or lock was bypassed.
Delegation: `/root/oneshot_lifetime_review` completed a bounded read-only
lifecycle review with no blocking findings. The primary independently ran the
tests and accepted the review's stop/restart and health-state caveats above.

Readiness remediation: added a bounded, read-only three-minute gate before the
durable effect receipt. It checks Docker, XRDP and XRDP-sesman individually,
three running containers, PostgreSQL readiness, HTTP 200 from local Guacamole
and the configured passwordless helper. Timeout does not consume an attempt;
permission refusal stops immediately. After the wait, exact requirement digest,
identity, installed artifacts and live retained status are rechecked. An already
restored target is not opened again. JSON effect responses must explicitly
report success. Removed the nonexistent user network-online target dependency.

Added `--check-readiness` for pre-restart verification with no browser actions.
All 26 focused/real-entrypoint fixture tests passed, including delayed readiness,
dependency refusal without receipt consumption and Docker-ready/XRDP-unavailable.
Existing requirement/live-pin fixtures, targeted ESLint and diff checks passed.
The main agent verified live readiness `{ready:true,missing:[]}`, native retained
identity and a completed ready installation transaction. The reviewed installed
binary SHA-256 is
`90e5dffdbc360f8fe3f1dfef34ffe0f4c5370ac4f11b1a7d602c68d075529272`.
Closed-world reviewer `restart_preflight_review` confirmed the individual-service
fix after flagging multi-unit is-active semantics. The main agent ran the tests;
reviewer evidence is source inspection, not an independent test run.

Installed helper SHA-256:
`c6562f47da34d2df1c9dc0d7b8d7fb4e527f83e6fefc78dc5d1f061c827818d7`.
Explicit verified rearming preserved the prior receipt. The installed readiness
command returned `dependencies_ready` with `retainedVerified:true`; service
Result=success/ExecMainStatus=0, timer enabled, and final installed doctor returned
ready with zero issues. No WSL restart, browser launch or prompt submission occurred
in this remediation turn. A controlled real restart remains the final end-to-end test.

Pre-restart hold: a later installed-binary change invalidated the armed artifact
manifest. Main-agent execution returned `installed_artifacts_changed_requires_review`;
do not restart on the basis of the earlier green no-action check. The user
`network-online.target` is absent; a 90-second delay is not proof that desktop
dependencies are ready before the durable attempt is consumed.

Added `test-workshop-boot-recovery-cli.mjs`: four passing isolated-HOME tests
execute the real helper entrypoint with fake browser/desktop commands. They
cover unchanged-kernel/new-distribution epoch recovery, repeat invocation with
one preparation total, committed-effect/response-loss refusal to replay,
private-file permissions and added configuration rejection. These prove helper
orchestration, not real Guacamole or XRDP startup. Targeted ESLint and diff
checks passed. No WSL restart or browser action was performed for this hold.

Independent read-only preflight (`restart_preflight_review`) identified artifact
drift and missing dependency readiness, confirmed by the main agent. Its claim
that the installed desktop opener exits zero on failure was rejected after
main-agent inspection verified the final nonzero-exit block. Required next work:
bounded no-effect dependency readiness before consuming an attempt, then review
and rearm against the current installed binary and verify the installed service.

WSL restart regression: terminating Ubuntu restarted its PID 1 but preserved the
shared kernel boot ID, so the first real test refused `same_boot_recovery_refused`.
Recovery epochs now combine kernel boot ID with `/proc/1/stat` field 22, using
the final comm delimiter rather than splitting a process name on spaces.
The timer now waits 90 seconds after user-manager startup, not kernel startup.
Kernel-only legacy receipts cannot authorize missing-browser recovery. Explicit
`--rearm-verified` requires a live verified same-identity lane and no pending
attempt, backs up the previous receipt, then pins the reviewed artifacts and new
epoch. It never invents a pre-restart PID 1 identity. This correction is tested
for unchanged kernel/restarted distro, full kernel reboot, same-epoch refusal,
malformed process observations, legacy refusal and interrupted-operation replay.

Correction validation: 16 focused tests, existing requirement/live-pin fixtures,
targeted ESLint, syntax and diff checks passed. Restored the exact Workshop
conversation through digest-bound preparation with one verified match, then
installed helper SHA-256
`ddaab9dbf1b7b8049f4d6630d152bca4199bc9396211afa5def062a881400ef4`.
Backed up and migrated the legacy receipt only after live same-identity
verification. The installed service returned `verified_no_action`; the timer
remains enabled. No prompt was sent. A second real WSL restart was not performed
in this correction turn, so corrected unattended recovery is fixture-tested,
not yet proven by a second restart. No delegation for this tightly coupled
epoch-parser correction; earlier helper review remains limited to its original
scope. Existing dirty source work was preserved.

The stale-runtime warning cleared after the separately published installation
converged. Installed doctor returned ready with zero issues; the exact retained
Workshop requirement remained verified. No browser was closed for this work.

The operator recovery helper is intentionally separate from the package payload:
install `scripts/workshop-boot-recovery.mjs` under
`~/.local/lib/agent-browser-workshop-recovery/`, then run that installed file with
`--arm` only while the intended retained lane is verified. Install the matching
service and timer templates under the user systemd directory and enable the
timer. Private state lives under `~/.agent-browser/workshop-boot-recovery/`.

Arming binds boot ID, enforced requirement digest, profile/session/conversation,
installed binary, support-script closure, helper, Node and configuration presence.
New boots use existing desktop restoration, display grants, service reconcile
and exact digest-bound preparation. A durable pre-effect attempt blocks replay
after a crash, including on later boots. Same-boot failure, identity/digest drift,
pending rotation, unknown old authority and installed-artifact drift fail closed.
No submit, fill, click or prompt API exists in this helper. OS shell/tool binaries
remain part of the trusted workstation boundary.

After any upgrade, review the new artifacts and current exact lane before
rearming. Do not delete a pending attempt or stale lock simply to retry: first
reconcile whether the prior operation opened a target or committed a rotation.
Rollback is disabling the new timer; preserve its private receipt for review.

Validation: 13 focused fixtures plus existing retained requirement and live-pin
fixtures passed; targeted ESLint and syntax checks passed. Independent review
identified environment inheritance and incomplete artifact inventories; both
were fixed and covered by focused tests. Real reboot validation remains pending;
do not reboot the shared workstation merely to prove this operator helper.

Installed and armed the separate operator helper and enabled its user timer.
Systemd unit validation passed. The installed service completed with
`verified_no_action` and exit status zero against the retained Workshop lane;
the live check did not reopen a browser or send a prompt. Source and installed
helper digests are compared at handoff. Existing dirty changes are preserved;
no package publication, compiled CLI change or GitHub write belongs to this slice.
Delegation: `reboot_recovery_review` performed read-only discovery and one
closed-world review; the main agent applied its two blocking findings, ran all
tests and live checks, and accepted its confirmation of those fixes.
