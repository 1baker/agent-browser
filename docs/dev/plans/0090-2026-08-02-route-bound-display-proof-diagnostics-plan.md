# Plan 0090: Route-Bound Display Proof Diagnostics

State: AWAITING_REVIEW

Created: 2026-08-02

Lane: last30days X successor hard-stop diagnosis

Source evidence:

- last30days Plan 0018 checkpoint C63
- last30days receipt `docs/dev/notes/0034-x-profile-handoff-repair-and-successor-stop.json`
- retained agent-browser acquisition leases at 2026-08-02T21:22:07Z and
  2026-08-02T21:22:51Z

## Goal

Remove the redundant ambient-display discovery gate from an already
route-bound visible-window proof and preserve its exact bounded probe failure
through CLI error rendering.

## Bounds

- Keep arbitrary dashboard and unbound display probes guarded by configured
  route discovery.
- Allow only an exact display carried by a normalized route binding to use the
  bound-route probe path.
- Preserve the capped underlying display-probe reason in visible-window proof
  failures.
- Preserve typed remote-view proof errors in AI-friendly CLI output.
- Add focused no-launch Rust regressions before implementation.
- One implementation pass, one review and rework cycle, and no live browser or
  last30days source attempt in this packet.
- Preserve the operator-owned untracked `--full-page` file.
- No push, release, tag, or publication.

## Acceptance

1. A route-bound display probe does not fail merely because ambient route
   discovery does not list that exact display.
2. Unbound display probes retain their current configured-route guard.
3. A display-probe failure reports its capped underlying reason.
4. AI-friendly error rendering preserves the structured remote-view proof
   failure rather than replacing it with a generic page timeout.
5. Focused tests, formatting, strict Clippy, and relevant Rust validation pass.

## Next Action

- Obtain independent review of the candidate repair. If it passes, close the
  plan without launching a browser or consuming another last30days source
  attempt.

## Candidate Result

- The already normalized remote-view binding now probes its exact display
  without repeating ambient route discovery; unbound probes retain the
  configured-route guard.
- Visible-window failures preserve a sanitized, 240-character maximum probe
  reason, and AI-friendly rendering preserves the structured failure.
- Forty-seven focused tests and all twenty-nine `remote_view_open` tests pass.
  Rust formatting and strict Clippy pass.
- The first independent review found one blocking coverage gap: the probe-error
  regression did not exercise sanitization or the 240-character cap. The one
  allowed rework cycle now covers multiline, quoted, over-limit input and
  asserts the sanitized bounded result; thirty remote-view unit tests and all
  twenty-nine `remote_view_open` tests pass after rework.
- The complete Rust suite reported 1,755 passed, 57 ignored, and two failures.
  The authenticated-target failure passed alone. The unknown-command fixture
  remains sensitive to the installed browser-recovery retry state and still
  fails alone before reaching its expected unknown-command assertion; it does
  not exercise the files or paths changed by this plan.
- No browser or source attempt was launched. The operator-owned untracked
  `--full-page` file remains untouched.
