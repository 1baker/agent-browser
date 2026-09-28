# Legacy retained-browser custody migration

## Frozen objective

User objective: `dont stop until the end to end test works with a chatgpt prompt being sent`.
The user approved a local-only engineering exception to the circular Pro-review
dependency. Completion requires one authorized, exact-target ChatGPT submission
and response readback, not merely passing migration tests.

## Verified starting point

The retained `chatgpt-pro` daemon PID 167143 matches the installed binary SHA-256
`3d2e7954865610e9f4fccb006a411aadb6b218379b4f82f2eca67b04e3f861e4`.
The publication record names the startup checkout as its source. That checkout
does not implement `controlPlaneAttestation`. The recovery checkout implements
it using committed custody receipts, but deliberately does not manufacture
custody on legacy resume.

The isolated test `test_legacy_resume_does_not_establish_v2_custody` passed on
2026-09-13. It proves that legacy resume followed by another prepare still emits
schema 1 without custody. Two ordinary handoffs are not a repair.

## Narrow implementation contract

1. Before changing the source, witness its exact Linux Unix-socket peer process
   using kernel PID/UID plus boot, start ticks and executable identity. Use the
   same authenticated connection for prepare; never reconnect between witness
   and command dispatch.
2. Capture the exact browser process, canonical profile inode, endpoint and
   listener ownership. Require the current exclusive unexpired session lease,
   exact target, URL and display proof. Reject ambiguous or foreign owners.
3. Bind the original legacy descriptor bytes and digest to the witnessed source.
   Preserve the legacy descriptor unchanged. Label new evidence as prospective
   migration, never historical v2 custody.
4. After prepare, prove that the exact old source process is gone. Recheck all
   browser/profile/target bindings before acquiring the destination lease.
5. Attach the original retained target and commit a migration receipt only after
   fresh verification. Missing acknowledgements remain uncertain; do not replay
   prepare, input or Send through generic transport retries.
6. Reject changed source, browser, profile, target, URL, conflicting leases and
   incomplete teardown. Preserve existing tabs and the Chrome process.

The existing connection helper retries EOF/reset failures. Migration must use a
single-dispatch transport and retain indeterminate outcomes instead.

## Acceptance and non-goals

Require isolated tests for successful migration, identity drift, source still
alive, uncertain prepare, and restart evidence. Then review the exact candidate
and ownership-preserving deployment before any live change. Verify complete
input and display attestation, send exactly one short prompt in one deliberately
selected conversation, and read its completed response. No GitHub writes,
credential changes, blanket cleanup or installation of the whole dirty checkout.

## Independent review

Delegation: spawned `/root/migration_review`, read-only design review completed.
Primary independently inspected the cited paths and ran the regression above.
Accepted finding: two existing handoffs cannot establish v2 custody. The same
agent now owns the Linux peer-witness primitive, single-dispatch migration
coordinator and focused tests. The primary owns documentation, independent
validation and deployment integration. `/root/activation_preflight` is reviewing
the narrow backport onto the installed startup source without live mutations.

Status: implementation in progress; no live migration or prompt submission.

## First implementation checkpoint

`ProcessIdentity::capture_unix_peer` now witnesses Linux peer PID/UID through
SO_PEERCRED, binds those credentials to the process identity and rechecks it.
The primary independently read the implementation and reran the isolated
`unix_peer` tests: two passed. The delegated formatting check passed, and the
primary whole-worktree diff check passed. Existing dirty changes were preserved.
This primitive grants no browser authority and is not yet wired into the
single-dispatch migration coordinator. It has not been installed.

## Deployment checkpoint

The primary independently ran `test-local-dashboard-prebuilt-candidate.js`
(passed) and `test-local-dashboard-workstation-provenance.js` (52 isolated cases
passed). These fixtures do not touch the retained browser. The installed source
checkout has existing dirty startup fixes, including `runtime_attach_proof.rs`.
Preserve those fixes; neither the whole main checkout nor the recovery checkout's
unrelated dirty broker/pipe work is an approved deployment candidate.

Planned operator-local interface: `handoff migration-plan <target-id> <url>`,
`handoff migration-prepare <target-id> <url> <plan-digest>`, then explicit
`handoff migration-resume`. Plan is read-only. Prepare must witness and dispatch
on the same authenticated socket exactly once. Resume must consume separately
labeled prospective enrollment without rewriting the legacy descriptor as v2.
This interface is under implementation, not an installed capability.

The exact committed custody delta `b385fd18` applied with `apply_patch` onto the
startup checkout without replacing its existing dirty files. The primary's
isolated baseline run passed 90 handoff-related tests; both initially ignored
private-worker tests also passed when explicitly run in isolation. The mocked
systemd publication-interlock fixture passed. These checks cover the combined
startup/custody baseline, not the unfinished migration integration.

Independent activation review accepted a two-stage sequence: first ordinary
transactional publication without any enrollment, preserving the same retained
browser but granting no legacy custody; then explicit migration using that same
installed candidate, only for `chatgpt-pro`. This avoids ordinary publisher
rollback crossing prospective enrollment. Hold the publication journal lock and
a separate migration-owned native maintenance receipt during stage two. Proven
success releases exclusion and restores the original timer state. An uncertain
post-dispatch outcome retains the receipt and paused timer for reconciliation.
No stage-two mutation has occurred.

## Migration review checkpoint

`/root/activation_scope` performed a read-only closed-world review. Two findings
were accepted and corrected: explicit resume now returns before generic
auto-connect/provider launch paths, and migration rejects duplicate browser rows
or competing profile aliases for the same physical browser/profile. The reviewer
identified no remaining concrete source blocker and requested migration-specific
restart coverage. The implementation agent reports seven focused migration tests
and strict Clippy passed after those corrections. Primary verification of the
final startup backport remains pending.

Primary client-generator contract checks passed. The candidate's service-request
and service-observability JavaScript passed typechecking using the installed main
checkout TypeScript toolchain, and the modified generator passed targeted ESLint.
The normal example-wide typecheck cannot run in the dependency-unmaterialized
startup worktree: its workspace package links and local TypeScript are absent.
No dependencies were installed and no example-wide pass is claimed.

A read-only live tab inventory found the pinned Reliability Gap conversation
present but inactive; Create Brief Deliverables is currently active. No tab was
switched. The single test conversation must be deliberately selected and its
exact canonical URL checked; migration must not silently fall back between them.

## Installed result and remaining blocker

Primary final startup validation passed 2116 full isolated Rust tests, with 73
ignored and zero failures, plus strict Clippy, build, format, contract and diff
checks. Real-host compatibility fixes cover conservatively validated joined
Chrome process titles and exact same-name abstract X sockets only on filesystem
ENOENT. No permission failure triggers an alternate transport. Candidate digest
`2e522208680e7586a98c984b355efbf9f129ebb90392c74d60ad9a7ead159c57` was published
transactionally, final doctor passed, and original Chrome PID 505779/endpoint/
physical targets remained. Publication record is
`local-dashboard-473a57c6-43fe-49b9-91ea-d891f1c9b40a`.

The explicitly selected current Create Brief Deliverables target was migrated
once: source 564799 exited, destination 575804 attached, original v1 descriptor
digest `c17c3c6d0f5c82ff8204281b7a7278223afcb56ecdbb0b9eb8e3331cca53abf5`
remains preserved. Initial broker diagnostics proved complete custody and display
ownership. Maintenance exclusion was released with the timer restored.

Before any typing, the next read-only request failed with
`handoff_custody_receipt_snapshot_mismatch`. The private committed receipt remains
but the shared receipt map is empty. A 12-second directory watch proved that old
QA workers 1974838 (transfer-qa) and 3762797 (installed-transfer-qa) actively replace
the live service-state file. Their temporary AGENT_BROWSER_HOME does not isolate
the HOME-based service-state resolver; both retain HOME=/home/bak3r. Their old
binary models drop custody evidence. No other daemon was stopped or modified.

End-to-end completion remains blocked before typing/Send. Next scoped work is
retiring only those legacy QA writers while preserving browsers, then governed
owner-side receipt reconciliation. Do not replay migration or restore authority
by manually editing shared state. Full installed checkpoint is in the startup
checkout RUNBOOK.md. No ChatGPT turn or GitHub write was made.
