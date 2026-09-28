# Plan 0131: Background browser exit evidence

State: OPEN

## Current State

The autonomous Codex/AuraCall document workflow cannot complete its installed
download verification because retained Chrome exited twice. Broker events bind
PID 35051 to an exit at 2026-09-11T21:41:58Z and restored PID 479722 to an exit at
21:47:45Z. The first predates the API restart; the second predates the test
script. No recorded broker close/release job explains either. The owning daemon
logs to /dev/null. The underlying termination cause remains unproven.

Owned Chrome already exposes nonblocking exit observations. Active-command
diagnostics describe exit code/signal evidence, but control-plane background
cleanup currently persists generic process-exited health without that evidence.

## Bounded outcome

Preserve the existing owned-child exit observation in background cleanup events
before discarding the browser manager. Do not infer a signal from an absent PID,
change recovery policy, launch a browser, or replay a provider request.

Owner: primary integrates; `/root/conversation_capacity_detection` implements
the narrow control-plane change and tests. `/root/live_recovery_verify` completed
read-only broker/process-log diagnosis; primary independently checked lifecycle
events and user-systemd timer records. No timer termination was established.

## Acceptance

- Background cleanup records available exit code or signal using existing fields.
- Unknown/external process state does not fabricate owned-child evidence.
- Poll errors remain errors, not successful exit observations.
- Regression tests cover the persistence path, not merely a metadata helper.
- Preserve current ownership, cleanup, retry, privacy and prompt replay boundaries.
- Installed acceptance requires guarded binary/installation-record publication
  with rollback and an observed runtime result; source tests alone are partial.

## Non-goals

This diagnostic repair does not establish the cause of past browser exits,
prove autonomous downloads, or close proposal evidence/routing requirements.
No unrelated dirty worktree changes may be published accidentally.

## Next action

Source repair is implemented in control_plane.rs using the existing
process_exit_observation_details helper from actions.rs. The worker's three
focused Rust tests and formatting check passed; primary inspected the diff and
independently reran all three tests and Clippy with warnings denied; both passed.
No new schema or retry policy. Installed acceptance is still open.

The runtime boundary changed during diagnosis: another controller launched
session:chatgpt-pro, PID 505779, at 21:53:07Z with a remote-headed Chrome 153
browser and began navigating the retained Workshop target. Primary did not
launch or modify it. Do not replace that daemon or navigate the tab on the
assumption that the former default session is still authoritative. Reconcile
the current owner and guarded retained-session handoff before installation or
document materialization. Existing binary and installation record remain intact.
