# Fresh-chain custody source checkpoint

Status: OPERATOR ENTRYPOINT SOURCE CHECKPOINT / NOT INSTALLED. The original
attachment-only API stays internal. The later cold operator entrypoint is
described below; source validation is not an install or live go/no-go gate.

## Proof and storage

`bootstrap` requires an injected service-state repository and an exact current
handle, a pinned canonical profile path/device/inode, and the expected predecessor
digest. The digest is `sha256:` followed by lowercase hexadecimal SHA-256 over
compact JSON with recursively sorted object keys; array order remains significant.
The predecessor must be a committed schema-4 receipt with complete identities.
The former destination process must be observed absent by boot/PID/start identity.
That observation is explicitly unauthenticated; it is not a detach acknowledgment.

The current browser process identity, executable inode/device, physical profile,
SingletonLock, DevToolsActivePort, exclusive persisted profile lease and exact tab
must agree. A conflicting live lease, released/foreign handle, changed browser or
profile, malformed/replayed predecessor, or live former owner fails closed.

The attachment-only `bootstrap` uses the exact pinned IPv4 loopback browser WebSocket,
without discovery or fallback. It requires one page target in inventory, attaches
with flatten enabled, then verifies the target through the returned session with
no targetId parameter. It enables no domains and sends no activation, resume,
creation, navigation, closure, retry or recovery command. It owns no browser
process. Dropping the staged result drops its connection only.

An owned shared PublicLease privacy barrier is required through attachment,
revalidation, persistence and result publication. It is separate from the
exclusive persisted profile lease and is not browser lifecycle authority.

All admitted state, process and physical facts are rechecked inside the locked
mutation. Schema 5 has kind `fresh_chain_bootstrap`, a fresh random version-4
chainId, generation 1, phase committed, and the current destination identity.
It has no source or fabricated old-chain continuity. Its embedded predecessor
contains the unchanged old body, canonical digest, and ownerExit
`observed_absent_unauthenticated`, in the existing opaque receipt-map value.
No ServiceState field or additional map key is introduced.

The opaque guarded result is returned only after repository persistence succeeds.
An error, including an uncertain save, never publishes success or installs the
manager. An uncertain save can have written the receipt; no unchanged-byte
guarantee, retry or automatic rollback is inferred. The generic repository controls persistence guarantees; this checkpoint
does not claim installation durability, recovery, or rollback readiness.

## Compatibility and boundary

Diagnostics distinguish `fresh_chain_exact_attach` from schema-4 handoff continuity
and recheck current owner, profile, lease and exact tab. Existing handoff consumers
refuse schema 5 before connecting and again before receipt removal/replacement;
supported schema-2 and schema-4 behavior is retained. Exact-tab URL projection
fences and preserves the receipt as opaque data and never promotes its proof.

At the original source checkpoint no action called either API. The original attachment-only
result cannot be handed to existing ready-runtime consumers. Any future operation
entrypoint, live supersession, detach/re-attach or installation still requires
its explicit scope and all applicable custody, quiescence and publisher gates.

## Internal protocol-readiness subunit

The separate `bootstrap_ready` variant keeps identical admission and the same
single locked receipt insert. Before committing, on the original exact socket
and session, it sends only `Page.enable`, `Runtime.enable`, `Network.enable`, then
one session-scoped `Target.getTargetInfo` with no targetId parameter. Each exchange
has a five-second timeout. It does not activate, resume, evaluate, auto-attach,
navigate, create or close a target. The original attachment-only wire remains
unchanged.

Explicit pause responses and exact-session pause events veto progress. Passive
event observation is retained during initialization, before commit, after
readback and at consuming adoption. A lagged, closed or continuously replenished
event queue also vetoes publication; its drain is bounded to 1024 events. That
conservative veto is not proof of a pause or browser exit. Absence of a pause
indication does not prove executable page script, visible pixels or login.

After saving, the ready variant verifies the exact receipt, the fence captured
inside the commit and current process/profile/lease/tab proof. Readback failure
or disagreement returns uncertainty without publishing a manager, retrying or
rolling back the receipt. Pre-commit failures request zero saves. A write-then-
error can leave the new receipt persisted, but returns no usable result.

Only an opaque, non-Clone `CommittedReadyBootstrap` can be consumed through
`BrowserManager::adopt_committed_ready`. The shared privacy guard stays held
through consumption and the returned manager never owns Chrome. The original
attachment-only result and pre-commit protocol-ready stage expose no manager.
Future action adoption must retain that guard and revalidate its current
authority; this internal token is not a reusable lifecycle permission.

At that checkpoint the subunit had no CLI, action, daemon-startup or install wiring. Those checks,
the detached publisher packet, final peer review and operational go/no-go are
unfinished. Neither source success nor synthetic tests establish live custody.

## Cold operator entrypoint, 2026-10-06

`handoff bootstrap --request-file <path>` is Linux-only and not exposed through
service_request/MCP schema additions. The regular-file, no-symlink input is
bounded to 64KiB, denies unknown fields and binds the CLI session to an exact
handle, predecessor digest and canonical profile/device/inode. It is parsed
before startup. Cold admission refuses existing daemon metadata and handoff
descriptors under the startup lock; no cleanup, prepare, resume or retry path.

The child environment suppresses launch/capture hints while preserving privacy
and policy routing. Direct bootstrap-mode startup also requires cold admission
and ignores streaming, private execution, expiration and all background/idle
timers. Its dedicated worker message bypasses scheduler/job persistence,
cancellation/timeout wrappers and health follow-up. Ordinary pre-success
requests are rejected before normal submit. One entry latch is consumed even
on rejection; existing managers are never replaced or retired on refusal.

The action uses the existing protocol-ready core, retains the public privacy
guard and adopts an unowned manager with detach behavior. It neither upserts
leases nor writes generic browser health nor starts handlers/streaming. A
precommit failure retires only the new browserless daemon, even if delivery
fails. The possibility of a durable write is conservatively latched before
commit; errors thereafter stay rejecting requests with no manager. Failed
publication drops the unowned connection only, never the browser or receipt.
No retry, rollback or fabricated continuity follows from any error.

The agreed plan incorporates peer C1 (stream-server suppression) and R1-R6
(dedicated worker, one-shot failure disposition, early request gate, timers,
inherited environment, symbol-based wiring). The owner implements this unit;
default peer limits remain unchanged. Existing internal proofs, schema-4
compatibility and W1 downgrade guards are not weakened. A mechanical false
initializer in mcp.rs does not add an MCP interface. The lifecycle consequence
category covers bootstrap so confirmation policy cannot misclassify it.

No install or retained-browser action belongs to this source unit. The detached
publisher packet, final install peer check and separate operational go/no-go
remain mandatory. Source tests do not establish live custody or rendered UI.

### Independent findings and bounded correction

Independent review raised ECR-01 (direct startup admission), ECR-02 (publication
admission race), and ECR-03 (rejected duplicate inheriting cleanup authority).
The owner accepted all three as blocking. ECR-02/03 now bind shared admission and
cleanup to the admitted first success and successful socket publication; failed
publication fences admission before dropping the unowned connection, and a
duplicate error owns no cleanup baton.

The first ECR-01 remediation failed the full isolated CLI fixture (35/36 focused
tests passed) because it refused the launcher's own pre-spawn token reservation.
That failed attempt is retained, not reported as a passing checkpoint. Peer
drift review accepted one bounded parent-reservation compatibility correction
inside the existing cold-only plan, with no new task, limit extension or live
permission. The parent still refuses all metadata. The child reuses the same
suffix list, excepts only its exact inherited reservation, opens it without
following links, checks the opened file's ownership/private mode/link count and
size, and performs a fixed-bound comparison without echoing contents. Other
metadata, a handoff, missing or foreign reservation still fail before writes.
Only the cold launcher's shared create_new reservation helper is used in tests.
The bounded correction passed the full fixture and closed-world ECR-01 source
review. The independent reviewer identified no critical regression in the
correction and confirmed ECR-02/03 remained intact; executed owner QA is
recorded below. No additional discovery or remediation loop was opened.

### Validation boundary

Only synthetic process/profile fixtures and a fake WebSocket browser are used.
Focused tests cover exact attach/session echo, unchanged predecessor and peers,
save failure, concurrent lease/receipt/tab/session changes, invalid handles,
former-owner liveness/PID reuse, physical-lock failure, truthful diagnostics,
legacy refusal compatibility and opaque projection. Production HOME, runtime,
PID, mount and network lanes remain masked during owner verification. No full or
live browser suite is part of this source checkpoint.

## Protocol-readiness validation, 2026-10-06

Owner verification passed on the ext4 checkout against source base da5434c5.
Every execution used the reviewed bwrap fixture with production HOME, runtime,
PID, mount and host-loopback lanes masked. The negative host-listener control
passed; no retained browser, profile contents or live daemon was inspected.

Final focused Rust results: 102 passed, zero failed. Filters and counts were
runtime_custody_bootstrap::tests (21, including the original 17),
runtime_attestation::tests (18), runtime_handoff (10), handoff_guard::tests (4),
tab_handle_refresh (22), and output::tests (27), run serially. The final log
SHA-256 is a02714f58a02615a80d18266c8f4118682aee69bc645c51175139b1c1a0f0a29.
Rust format check, normal clippy with warnings denied, root pnpm lint and docs
build also passed. The docs build retains the existing multiple-lockfile
workspace-root warning; no config or lockfile was changed to suppress it.

The owner inspected and explicitly applied the scoped peer patch, then fixed
two accepted blockers: late pause-event observation and current-binding
readback after commit. Independent closed-world read-only source review found
no remaining blockers in those fixes or the guarded consumption subunit. The
reviewed source SHA-256 values are:

- browser.rs: 272e1c0cd814cbb581c5e17ec71ce6970240dcf4fc22e86a2aa1728141488dce.
- runtime_custody_bootstrap.rs:
  60d78ac18420f41dfb3d74a125ed2a4b5b6502f856ba49f06ed3e6f66abca301.

The validation selector also recommends installer-family fixtures because of
the broad output.rs match. No installer, provisioning, privilege, asset or
runtime route code changed; those fixtures are not this subunit's verification
surface. Its skill-sync recommendation points to an obsolete other-user home;
this source-only scope does not install or synchronize workstation skills.
Full Rust, live E2E, installer and consumer acceptance remain unrun. This note
is a source checkpoint, not release readiness or an operational go/no-go.

## Cold entrypoint validation, 2026-10-06

The later operator-entrypoint unit was verified against base 9a27bd93 with the
same reviewed ext4 isolation wrapper. All four final QA result receipts report
exit code 0. The host-listener negative control passed in every execution;
production browser/process/profile/runtime lanes were unavailable. The CLI
fixture invoked the built candidate against a fake WebSocket browser, not Chrome.

Across the focused Rust filters, 555 distinct tests passed with zero failures.
The `custody_bootstrap` filter passed 37 cases, including the cold CLI-to-daemon
path, private parent reservation, seven-command exact attachment, single receipt
write, no stream/job/health mutation, precommit retirement, publication fencing,
and rejected-duplicate cleanup ownership. Compatibility filters passed for
attestation, runtime handoff, downgrade guard, handle refresh, parser, connection,
daemon, policy, output and screencast view. The ordinary worker filter passed
33 cases; its pre-existing live-browser fixture stayed ignored. All four ignored
private-worker fixtures were explicitly run in isolation and passed. Counts
above deduplicate overlapping filters rather than adding their totals.

Rust format check and normal Clippy with warnings denied passed. Root ESLint,
the production docs build, API/MCP parity, generated service-client contract
check, and all eight route-confusion fixtures passed. The docs build's existing
multiple-lockfile warning remains; no lockfile or configuration changed. Owner
final inspection constrained the direct-startup fixture to Linux, matching its
Linux-only admission API; non-Linux execution is not claimed.

The final log SHA-256 values are:

- bootstrap-reservation-final:
  636bce3b385084112c4885b86a56246ecf4c6a5079ed4f492bd3a17fe4019709.
- bootstrap-worker-regression:
  710939db59be78452ecac6de09b80018f21d40611c3c43c725250d3a8898c814.
- bootstrap-reservation-docs:
  c9e1ab8362a791b07a82bd20ca70162f1d5daff0f04fb5bf324d84ca4015d218.
- bootstrap-entrypoint-contracts:
  652fe2bd4f8a7e6dc1b4ec4ee29874c3afe7bd08e89bdaf5c69648c75b8f4734.
- bootstrap-portability-check:
  c3285c86294a9bf3838b9b1e9c94a3653108bdf8ceb5f6e686c45193d31fc527.

The fifth result receipt also reports exit code 0: after the final Linux fixture
annotation, the candidate was rebuilt and all 37 bootstrap cases, format check
and strict Clippy passed again. Only documentation evidence was added afterward.

No installer/provisioning/runtime publisher code, public schema, dependencies,
version or credential material changed. Broad selector installer recommendations
come from shared output/help paths, not an installer mutation; live streaming
and broad browser suites are outside this source-only boundary. Installed skills
were not synchronized. Final install review, detached executor/artifact packet,
fresh operational go/no-go and live consumer acceptance remain separate and
unfinished. This checkpoint neither authorizes nor claims installation or live
custody continuity.
