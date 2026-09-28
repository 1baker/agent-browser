# Mixed-generation runtime custody recovery

## Problem

During an installed binary replacement, an older daemon may still own a live
retained browser. One custody-aware generation wrote
`runtimeCustodyReceipts` into service state, but the replacement generation did
not model that field. A normal service-state rewrite therefore removed the
receipt. The older daemon then rejected both handoff and close with
`handoff_custody_receipt_snapshot_mismatch`, even though its Chrome process,
profile, DevTools endpoint, and targets remained healthy.

This is a compatibility failure between executable generations. It is not
permission to close or relaunch the retained browser.

## Compatibility behavior

- `ServiceState` now round-trips `runtimeCustodyReceipts` as opaque JSON so a
  generation that does not consume a receipt cannot erase it.
- Runtime handoff can read schema-v2 descriptors written by the custody-aware
  generation. Linux resume verifies the exact committed receipt, source
  process exit, browser process identity, profile inode, loopback DevTools
  endpoint, selected target, and exclusive service projection before attach.
- A browserless `handoff prepare` refuses to replace an existing recovery
  descriptor.
- `scripts/repair-runtime-custody-receipt.js` can restore a reviewed exact
  receipt when the receipt bytes or the expected service-state digest are
  available. Apply requires the dry-run digest and revalidates live identity.
- `scripts/recover-stale-attached-daemon.js` is the narrower last-resort path
  for an `attached_existing` browser when no receipt exists. It requires the
  exact stale deleted daemon PID, exact live browser launch proof, profile,
  loopback DevTools endpoint, and selected target. Apply is digest-bound,
  revokes only that exact daemon, revalidates the browser and target, and
  writes a schema-v1 descriptor for the current binary. It does not terminate
  or relaunch Chrome.

Neither repair script is a general stale-process cleanup command. A changed
PID, inode, process start time, profile, endpoint, target, service projection,
or reviewed digest fails closed.

## Live verification

On 2026-09-19, five retained browser lanes were transferred across executable
generations without changing their Chrome PIDs or DevTools endpoints. Four
used normal or schema-v2 handoff. The LitScout lane had no recoverable receipt,
so the digest-bound attached-existing recovery revoked only stale daemon PID
`1431403`, preserved Chrome PID `804180`, and reattached all five targets.

After recovery, `agent-browser install doctor --compact --json` reported:

- `status: ready`
- `runtimeConvergence: converged`
- `runtimeCount: 6`
- `staleRuntimeCount: 0`
- `liveDashboardReady: true`
- `workstationPayloadReady: true`

The follow-up isolated file-transfer smoke exposed two independent fixture and
request-path gaps. Service auto-launch accepted explicit arguments but did not
apply an explicit `executablePath`, and the shared smoke helper allowed the
temporary agent home to be created with a group-writable mode that the privacy
gate correctly rejects. Auto-launch now preserves the explicit reviewed
executable path, with a focused Rust regression test. The smoke helper creates
the agent home as mode 0700 and can bind an explicitly supplied installed Chrome
directory into its isolated home. The live smoke then passed upload and download
SHA-256 checks, provider/expected filename agreement, byte and metadata checks,
and the diagnostic failure path.

After those fixes, formatting, Clippy, focused JavaScript lint, the local runtime
convergence suite, and the full Rust test suite passed again.

The live receipt is host-specific and ephemeral. Reproduce current proof with
the no-launch install doctor instead of relying on these historical PIDs.
