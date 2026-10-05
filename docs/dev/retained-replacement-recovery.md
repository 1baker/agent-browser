# Retained replacement recovery

When a local dashboard publication has already handed off retained sessions
and the replacement binary fails only the dashboard HTML smoke because its
embedded dashboard was not built, the normal recovery repeats that failure.
The guarded `--recover-only --accept-retained-replacement` mode can close that
exact failed transaction without changing the binary, daemons, or Chrome.

Supply `--reason`, `--expected-sessions`, the exact retained session/profile/
target/URL/CDP flags, and `--expect-retained-start-ticks`. The mode requires
the installed replacement and every runtime daemon to have the journaled
replacement digest, every prepared handoff to be resumed, a matching saved
backup, the exact retained browser pin, and no doctor issue beyond the pending
publication. It records the dashboard smoke failure as waived for this
transaction only. A fresh publication must build the dashboard into its
binary and pass all normal readiness checks.

The accepted journal phase is the existing terminal `recovered_ready` phase,
with `readiness.smokeWaived` and `retainedReplacementAcceptance` recording why
the dashboard smoke did not pass. It releases the publication interlock
and allows a new publication to take its own backup and perform one guarded
handoff. Do not use this mode for a handoff, browser identity, doctor, or
unknown readiness failure.

If the process stops after `accept_admitted`, rerun this mode with the same
reason and the exact interlock receipt ID using `--recover-interlock-receipt`.
The guards recheck the live evidence before completing the terminal record.
