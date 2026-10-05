# Executable handoff guard checkpoint, 2026-10-05

Status: verified source candidate; live installation blocked on continuous
client quiescence and remaining artifact-specific compatibility evidence.
No shared host was restarted, retained browser or tab changed, provider request
sent, installed executable replaced, or publisher apply started.

## Provenance and bounded changes

The user approved patch `679b2393d53d44d9324fd4140c3610ab3f8d88fb` on
`3bb9595f616ca9400024eaad893997ddec736e79`. Its supplied SHA-256 was verified:
`50b4f9da0f48fee4b80c510c0ec13cae1989a13c9da1ce7fb124e1695cfcc541`.
The owner verified a clean base, dry-run patch applicability, and all eight
unchanged source/test blob identities after applying with the patch tool.

The candidate adds Linux executable-inode authorization before handoff side
effects. Refusal keeps the daemon running and makes an older client return
before disconnecting or spawning its own replacement. Executable inspection
errors refuse even with authorization. Only JSON boolean authorization and
the exact client environment value `1` are accepted. The publisher scopes that
authorization to its own pre-install prepare subprocess without mutating the
parent environment.

The owner additionally gated the new Unix test helpers to Linux and updated
help, README, repository skill, MDX and inline documentation. Non-Linux behavior
is unchanged; no non-Linux toolchain was installed or cross-build certified.
Guard source SHA-256:
`eec11c71bbc61e952440b250658570284af08dcf3dff578728abfce1912d0f4f`.
External optimized candidate SHA-256:
`084967bd80ab61f178719e6e5138c6532d2c3905374b49307427262a947b482c`.
That artifact was built from this source diff, not installed.

## Primary-agent validation

Rust tests ran serially on ext4 in disposable mount, PID, IPC, user and network
namespaces. The production home and runtime paths were replaced with empty
private filesystems; only source and build output were mounted read-only.
No signed-in profile, credentials, live socket or host network was available.

| Check | Result |
| --- | --- |
| Executable handoff guard | 4 passed |
| Runtime handoff, including parser and refusal-before-side-effects | 8 passed |
| Private handoff | 5 passed |
| Private broker | 6 passed |
| Exact Grok refresh projection | 12 passed |
| Connection module | 38 passed |
| Refused handoff does not exit daemon | 1 passed |
| Focused Rust total | 74 passed, zero failed |
| Publisher orchestration | Passed |
| Scoped publisher environment fixture | Passed |
| Dashboard quiescence fixture | 55 isolated cases passed |
| Rebuilt source-free workstation installer fixture | Passed |
| Rust format and default clippy with warnings denied | Passed |
| Root ESLint and docs production build | Passed |
| Patch whitespace check | Passed |
| Candidate workstation dry-run | Passed, planned, mutated false |
| Publisher read-only retained-browser and journal preflight | Passed |
| Existing installed doctor | Passed, zero issues |

The empty-filter attempt for `successful_exit_response` selected zero tests and
is not counted. The actual `denied_close_or_handoff_does_not_exit_daemon` test
was then run and passed. An initial doctor invocation used unsupported
`--compact`; the correct `install doctor` invocation passed. Neither error
caused a runtime mutation.

The validation selector overselects live streaming and host provisioning for
the large actions source file. Those unrelated live/privileged suites, broader
Rust/E2E suites and provider acceptance were not rerun for this bounded patch.
Prior broader validation is historical, not a new pass. The installed-skill
sync path named by the selector is absent on this workstation; no shared skill
was overwritten.

## W1 evidence correction and remaining gate

`bf3eeff5` is ancestral to the `0.28.0` bump `4a374bea`. That release-source MCP
reads state and dispatches commands, but the version label does not identify all
later artifacts. Commit `229ff162dfa1665378801d6e7d11bb94503c50b1` subsequently
added cold MCP startup and an in-process profile worker without changing that
version. The worker in `cli/src/mcp.rs` delegates to `ControlPlaneWorker`, which
persists job audit through `service_jobs.rs` and `service_store.rs`; profile
operations can also persist configuration. Absence of direct filesystem writes
in `mcp.rs` therefore does not establish that every retained MCP process is a
state reader. Exact executable hashes still need source/behavior provenance.

The guard protects a running guarded daemon, not cold startup after that daemon
exits or crashes. A superseded daemon permits any prepare. The publisher's
prepare-to-resume interval must therefore have an evidenced continuous hold:
every affected client has no request in flight and performs no browser request,
state mutation, reconnect or cold launch. Snapshot job/viewer idleness, process
I/O idleness, a version label or the guard alone does not establish this hold.
No such all-client hold has been accepted; W1 is not claimed closed.

Read-only checks found the installed executable unchanged, a terminal ready
journal with no live publication lock, and preserved retained Chrome start
identities. Exact process/client census and coordination remain private. A full
publisher apply was deliberately not started because the external gate was
already unproven. A workstation dry-run is not publisher-window authorization.

## Review and next action

The own-thread stateful Claude task
`agent-browser-downgrade-guard-install-20261005` approved the corrected bounded
plan at revision 4 and the source checkpoint at revision 6. Its initial claim
that old clients must run the new guard was rejected with the daemon/client
call-path evidence and corrected by Claude. No peer editing baton was used.

Delegation receipt: spawned `downgrade_guard_audit`, completed. The independent
read-only Codex reviewer found no behavioral guard defect and identified the
version-label/indirect-worker distinction. The primary agent inspected the
source paths and verified the tests independently. Both reviews are source
judgment, not installed-runtime acceptance. CodeGraph is uninitialized;
direct source review was used. Graphiti retrieval remained advisory and made no
provider request or writeback.

Next: obtain exact affected-owner maintenance holds and artifact compatibility
dispositions, then refresh the complete session/custody/job/viewer/interlock
preflight. Only after those gates pass may the guarded publisher prepare,
atomically replace and resume the complete writer/session set. Verify installed
guard and Grok behavior, retained process/profile/tab identity, strict doctor
and journal completion before reporting installation or consumer readiness.
