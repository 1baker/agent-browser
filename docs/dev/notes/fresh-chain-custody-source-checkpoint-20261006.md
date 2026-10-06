# Fresh-chain custody source checkpoint

Status: INTERNAL UNEXPOSED. This is an internal Linux API, not an operation
entrypoint, ready runtime, install gate, or permission for a live receipt change.

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

No existing action calls either internal API. The original attachment-only
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

This subunit has no CLI, action, daemon-startup or install wiring. Those checks,
the detached publisher packet, final peer review and operational go/no-go are
unfinished. Neither source success nor synthetic tests establish live custody.

## Validation scope

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
