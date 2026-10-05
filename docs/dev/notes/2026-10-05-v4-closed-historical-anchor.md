# Schema-v4 closed historical anchor repair

Date: 2026-10-05. Scope: source repair, not installed or live acceptance.

## Desired result

Legitimate closure of the historical handoff tab must not permanently poison
otherwise proven browser custody. The current requested tab must still verify.
Never infer ownership merely from CDP reachability or fabricate a receipt.

## Agreed scope and decisions

Codex and Claude accepted this bounded source slice in peer task
`agent-browser-v4-closed-anchor-repair-20261005`, revision 4. It is distinct from
the maintenance-window task: its existing phase limits remain unchanged.

- Keep schema-v4 committed receipt, positive generation, exact current daemon
  identity, prior-source exit, exact Chrome identity, profile inode and lock,
  configured physical profile, exclusive lease and conflicting-owner checks.
- Validate the current target normally. Closed, closing or crashed current
  records cannot be used, even if incorrectly left in session membership.
- A historical anchor is either the active target, another normally bound
  target, or an exact owned closed tombstone absent from every session's tabs.
  Missing, foreign, inconsistently listed and merely unlisted live records fail.
- Do not rewrite the stored receipt's target. Add a `priorAnchor` audit field to
  newly prepared custody descriptors and validate it on resume. A live anchor
  may become a verified closed tombstone during handoff, but not the reverse.
  Older descriptors without the field retain existing compatibility checks.
- Use checked generation arithmetic. No new flag, command or service action.

## Plan

1. Separate current target binding from historical anchor disposition while
   retaining all physical custody checks.
2. Add descriptor lineage and diagnostics evidence with negative fixtures.
3. Validate isolated focused Rust tests, formatting, Clippy, lint, docs build,
   staged secret scan, and an independent closed-world review before publishing.

## Completion checks

Fixture-only positive and negative checks cover owned closure, current target
rejection, missing and foreign records, conflicting leases, mismatched process
and profile identities, stale destination, overflow and resume compatibility.
Process/file fixtures use private temporary profiles and reaped shell children;
they are not Chromium tests or live end-to-end acceptance.

Primary validation: 17 attestation tests, eight handoff tests and four downgrade
guard tests passed (29 distinct; the two v2 verifier cases also passed separately
and are not counted twice). A `diagnostics_handle` filter selected zero tests
and is excluded; the actual metadata negative gates are in the attestation suite.
Formatting, production Clippy with warnings denied, ESLint and docs build passed.
Five workstation fixtures passed, using the pre-existing fixture binary, not a
newly built repair candidate. Host provisioning failed at the already-known
missing apt candidate for AppArmor; no installer/privilege code changed here.
The validation selector's installed-skill comparison refers to a different
workstation path; skill installation remains deferred with the runtime install.

Claude revision 6 and independent Codex closed-world review found no blocking
G2 defect. Primary independently ran validation; neither reviewer ran tests.
The first test compile assumed an unavailable temporary-directory dependency;
the fixture was corrected to use the existing UUID dependency and standard
library only. Claude final revision 8 accepted the source outcome. Final format,
production Clippy, lint and focused attestation tests passed again after fixture
hardening. The complete staged seven-file credential-pattern scan passed after
three unchanged documentation examples were explicitly reviewed as placeholders.
No real credential value was exposed or added. No blanket ignore was applied.

## Known fail-closed limits

Applying `service prune-retained` closed-tab pruning deletes the historical
anchor tombstone, so custody then fails with
`attestation_historical_anchor_missing`. Preserve it during a pending handoff;
changing prune policy is outside this slice.

The pre-existing prepare helper returns no attestation when session, profile or
browser PID metadata is absent. The action handler independently rejects that
result when a custody receipt exists (`attestation_existing_receipt_cannot_downgrade`).
This slice does not weaken or broaden that guard.

## Installation limitation

The installed older source contains the same historical-target check as the
pre-repair code. Prepare runs inside that old daemon, so this repair alone
cannot repair its poisoned receipt or produce a valid upgrade handoff.

A reviewed migration still needs an exact owner-granted current target,
socket-peer-bound acknowledgement from the real former owner, proof of that
owner's exit, unchanged browser/profile/lease/tab inventory, and a typed native
recovery entrypoint preserving receipt history. The old explicit leave-open
handler is only a conditional release primitive: generic CLI close can fall
back to force cleanup after communication failure and is not an acceptable
migration client. Reconnect alone cannot replace a stale destination receipt.

No migration, receipt edit, daemon/browser restart, tab manipulation or provider
Send is part of this source slice. Live installation remains blocked on the
owner-target reply, native-compatible custody gate, fresh installer exclusion
and proven lifecycle failure handling. Existing pause approval remains valid;
it does not waive those technical gates.
