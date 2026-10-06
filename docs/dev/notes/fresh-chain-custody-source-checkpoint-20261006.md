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

The attachment uses only the exact pinned IPv4 loopback browser WebSocket,
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

No existing action calls this API. Domains are not enabled and the attachment
cannot be handed to existing ready-runtime consumers. Any future operation
entrypoint, live supersession, detach/re-attach or installation requires a separate
explicit scope and all applicable custody, quiescence and publisher gates.

## Validation scope

Only synthetic process/profile fixtures and a fake WebSocket browser are used.
Focused tests cover exact attach/session echo, unchanged predecessor and peers,
save failure, concurrent lease/receipt/tab/session changes, invalid handles,
former-owner liveness/PID reuse, physical-lock failure, truthful diagnostics,
legacy refusal compatibility and opaque projection. Production HOME, runtime,
PID, mount and network lanes remain masked during owner verification. No full or
live browser suite is part of this source checkpoint.
