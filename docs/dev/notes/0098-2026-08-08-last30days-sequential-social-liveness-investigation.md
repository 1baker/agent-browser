# Last30Days Sequential Social Liveness Investigation

Date: 2026-08-08

## Purpose

Investigate the Last30Days report that a fresh Facebook target could be created
after an X acquisition, but the immediately following authentication evaluation
timed out while the retained browser still reported ready.

This note is evidence and a cross-repo contract handoff. It does not authorize
closing retained PID 96078, removing its tabs, restarting its browser, creating
a replacement profile lane, automating authentication, or weakening Facebook
content quality gates.

## Supplied Failure

The installed Last30Days 0.3.26 provider attempt recorded this privacy-safe
sequence:

- service reads completed in 224 ms and 4,048 ms;
- tab listing completed in 8,286 ms;
- two retained-target selections each reached their 8-second caller timeout;
- `tab new https://www.facebook.com/` completed in 8,426 ms;
- the following authentication eval reached its 20-second caller timeout;
- that eval carried a 15-second `--job-timeout-ms` deadline;
- the browser remained live and a later direct Facebook run accepted real posts.

The failed attempt was
`provider-attempt-3b81211ca0a73be9481fa6262d1b59f8` in tick
`tick-d08819fc38346ad98f8eb070267d1076`.

## Source Findings

Plan 0097 and commit `91dd2abb2fe9aa4eb161a959716e3690eda6f60b`
already repair the prior unconditional browser cleanup and prove that a timed
out dispatched job releases the serialized worker without replacing a live
browser.

The remaining deadline layers are different:

1. The CLI caller deadline includes process startup, daemon connection, queue
   wait, command execution, and response handling.
2. `jobTimeoutMs` starts only after the worker dequeues and dispatches the
   request in `cli/src/native/control_plane.rs`.
3. Therefore a 15-second worker deadline with a 20-second caller deadline has
   only 5 seconds for all pre-dispatch and response overhead.
4. The supplied attempt already observed 8.2 to 8.4 seconds on successful
   commands in the same sequence. The caller could exit before a 15-second
   worker timeout response becomes observable.
5. CDP event maintenance remains a plausible source of queue wait because the
   worker awaits `drain_cdp_events_background()` between requests. The supplied
   receipt does not retain the failed eval job ID or queue timing, so it does
   not prove that event maintenance, target main-thread load, or another queue
   occupant was the exact contributor.

The correct immediate client contract is to leave enough caller-side grace for
observed setup and queue latency. A future agent-browser enhancement may make
submission-to-dispatch queue consumption explicit in job timing or deadline
semantics, but that is not required to correct the disproven 15-second versus
20-second Last30Days layering.

## Current Ownership Drift

A later readback found retained Chrome PID 96078 still ready with 17 tabs, but
the browser record is now `session:plan0058` with active session `plan0058` and
profile metadata `default`. The configured `last30days-facebook` session still
points to that exact browser ID. It is therefore an alias to the retained owner,
not authority to launch another default-profile browser.

The Last30Days compatibility guard currently accepts default-profile drift only
when the browser ID is literally `session:last30days-facebook`. It rejects the
current safe alias even though the configured session points to the same ready
browser and a target-service tab is retained. The downstream repair should:

- require the configured alias session and selected product profile to match;
- require that alias session to point to one ready default-labeled browser;
- require a writable ready CDP stream and a retained tab for Facebook;
- resolve an actual active owner session that points back to the same browser;
- route subsequent commands through that owner session;
- fail closed on ambiguity or any mismatched browser, profile, target, or
  ownership evidence.

## Diagnostic Side Effect And Cleanup

A bare `tab list` against the drifted `last30days-facebook` daemon auto-launched
default-profile PID 47946 at 2026-08-09T00:53:03Z because that daemon no longer
owned PID 96078. After exact process, event, session, and browser-ID attribution,
only PID 47946 was closed. Retained PID 96078 remained ready and unchanged.

This confirms that bare session commands are unsafe when retained ownership has
drifted. Future diagnostics must resolve the broker or alias owner before any
ordinary browser command, including commands that appear read-only.

## Acceptance Handoff

Last30Days may resume with one distinct successor only after tests prove both:

1. fresh-target auth evaluation keeps the 15-second worker deadline but gives
   the caller materially more than the observed 8.4-second setup and queue
   envelope; and
2. alias-owner routing selects `session:plan0058` and owner session `plan0058`
   from the current exact state without accepting an unrelated default browser.

The manual proof remains successful Facebook acquisition with at least one
accepted post, or verified genuine no-results. Transport success and
`quality_gate_failed` are not acceptance.

## Resolution

No agent-browser source change was required. After PID 96078 disappeared
outside this investigation, the distinct Last30Days manual proof legitimately
started one canonical `last30days-facebook` browser, PID 63205. Installed
Last30Days 0.3.27 then retained the failed job IDs needed to close the timing
question:

- both 3-second tab-switch jobs succeeded after their 8-second callers had
  already exited, proving caller queue grace was insufficient;
- the fresh evaluation reached the worker and independently timed out after its
  15-second `jobTimeoutMs`, returning through the repaired 30-second caller
  bound;
- an exact post-tick diagnostic succeeded on authenticated Facebook in 10.5
  seconds for tab switch and 8.4 seconds for auth when each 3-second worker job
  received a 15-second caller bound.

Last30Days 0.3.28 consumes that established agent-browser contract: retained
tab/auth jobs keep 3-second worker limits with 15-second caller bounds, while a
genuinely fresh auth target receives 30-second worker / 45-second caller bounds.
Manual tick `tick-f273eb12d642b31d49a7f12959b93b87` accepted Facebook via
attempt `provider-attempt-5e5205b623e52dfd122dbbf2e4e668af`: 19 observed,
two accepted, 17 rejected, and every recorded browser operation succeeded.

The accepted downstream commit is
`24474f62e5e11f1c51d5ab5adf0f0933764dce91` on
`CochranResearchGroup/last30days-skill` `main`. PID 63205 remains ready on the
canonical profile/session with 17 tabs. No duplicate browser was launched and
no retained tab was closed by the accepted proof.
