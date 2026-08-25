# Isolated Dashboard Publisher Lifecycle Fixture

Date: 2026-08-14
Plan: P116

## Finding

P115 fixed standalone dashboard restart after publication and proved it with a
live release build. The lifecycle decision and `/proc` identity checks still
lived inside the top-level publisher, so rollback coverage depended on another
expensive and disruptive runtime handoff.

## Resolution

The production publisher now delegates standalone quiescence and restart
selection to a reusable lifecycle module. A Linux fixture copies the current
Node executable to exact temporary `agent-browser` and deliberately mismatched
names, starts them as orphaned disposable processes with the dashboard-mode
environment, writes isolated PID metadata, and exercises the production module.

The fixture proves:

- normal restart returns `restart-standalone`;
- rollback restart returns `restart-standalone-after-restore`;
- both signal only the exact verified original PID and start one replacement;
- an absent dashboard remains `not-installed` without explicit start authority;
- stale PID metadata is removed without a signal;
- a wrong-command process fails closed and remains live until exact cleanup.

The fast Linux Dashboard CI job runs both the smoke-policy regression and this
real-process lifecycle fixture before the dashboard build, making the boundary
automatic rather than a manual-only package command.

No live dashboard, installed binary, retained browser, browser profile, target,
prompt, page, credential, external service, or GitHub state is mutated.

## Verified Result

The focused and widened gates pass, CI workflow syntax and ordering are
validated, and cleanup inspection finds zero temporary processes or directories.
Read-only live evidence shows the retained browser PID, CDP endpoint, URL, title,
and dashboard runtime manifest unchanged from P115.
