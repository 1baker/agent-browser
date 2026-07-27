# P78 Guacamole Route Fixture Recovery Attempt

Date: 2026-07-27
Plan: `docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md`
Outcome: blocked before live route mutation

## Pre-State

- Guacamole PostgreSQL, schema, web, header login, guacd, XRDP, public ingress,
  installed runtime, and daemon inventory were healthy.
- The database contained zero RDP connection rows and zero selected connection
  permissions.
- Remote-view doctor was blocked on
  `provision_second_guacamole_rdp_connection`.
- The recurring timer was enabled. It was paused before the controlled attempt
  so it could not race the packet's one live gate.
- Both participating repositories were clean and synchronized before P78
  implementation began.

## Deterministic Repair

The controller now recognizes only the exact supported fixture-provisioning
doctor action. In apply mode it invokes the existing-user route-pool sync once,
then reruns install and remote-view doctors before considering display
restoration. Dry-run mode remains read-only, and unrelated doctor actions do
not select the remedy.

The fixture-backed regression drives the public convergence command through
isolated command adapters. It proves:

- exactly one guarded existing-user route sync;
- service reconciliation before fixture sync;
- refreshed doctor evidence before display restoration;
- no runtime mutation in dry-run mode;
- no fixture provisioning for unrelated doctor actions.

## Live Attempt

The single authorized command was:

```text
pnpm --silent converge:local-runtime -- --apply --skip-publish --json
```

It stopped at `provision_rdp_guac_route_fixtures` with status 2 because the
plan supplied `--apply` to
`sync:rdp-guac-existing-user-route-pool`. That script applies by default and
accepts `--dry-run` as its only mode flag. Argument parsing failed before SQL,
route-display restoration, or access grants ran.

The retained receipt is:

```text
~/.agent-browser/convergence/local-runtime-latest.json
```

It records the unsuccessful fixture step and no later mutation steps. A
post-attempt report-only readiness check still found zero RDP connections.

The controller, fixture regression, and Plan 0078 example now use the correct
apply-by-default invocation. The corrected live command has not been run
because Plan 0078 permits only one live attempt.

## Backup And Reset Evidence

- The PostgreSQL container bind-mounts
  `~/.agent-browser/guacamole/data/postgres`.
- No usable PostgreSQL dump, archive, snapshot timer, or documented restore
  procedure was found in the inspected user-scoped runtime and repo surfaces.
- `~/.agent-browser/guacamole/init/001-initdb.sql` recreates schema only. It is
  not a data backup and cannot restore route rows or permissions.
- Container history records fresh initialization events on July 4, 15, 21,
  26, and 27.
- No repo-owned script was found that deletes the bind mount.

The current database is recoverable only through deterministic fixture
recreation, not historical restore. The recurring external reset cause remains
unresolved.

## Historical Resume Gate

Explicitly authorize one replacement Packet C live attempt using the corrected
sync invocation. Keep the timer paused until that controlled attempt completes.
If the attempt succeeds, verify exactly two distinct authorized routes, both
route displays and X11 sockets, retained route-pool availability, remote-view
control readiness, install convergence, and one later recurring timer pass.

Do not launch an application browser or inspect source authentication during
this gate.

This gate was consumed by the replacement attempt recorded below.

## Replacement Attempt

The operator authorized one replacement Packet C attempt on 2026-07-27. The
corrected convergence command provisioned exactly two Guacamole RDP
connections. Both connections have the required read grants, distinct
configured target identity keys, reachable backends, and connection IDs 1 and
2.

The attempt then failed at `restore_rdp_route_displays` with status 1. XRDP
created display `:10` for route A. When route B connected with the same
`agent-browser-rdp` account, XRDP recorded a reconnection to the existing
display `:10` instead of allocating display `:11`. Report-only readiness now
reports route A's X11 socket ready and route B's expected `:11` socket missing,
with next action `repair_rdp_route_display_session`.

The host runs XRDP 0.9.24 with `Policy=Default`, which allocates sessions by
user and negotiated bit depth. Although the two Guacamole connection records
request color depths 24 and 32, the live XRDP allocation treated them as the
same session key. The nominal color-depth distinction is therefore not a
working isolation mechanism on this runtime.

No second route sync or display-open attempt was made. The interlock timer
remains enabled but inactive, no application browser was launched, and the
retained receipt remains
`~/.agent-browser/convergence/local-runtime-latest.json`.

The next safe action is a new bounded isolation plan. It must review and choose
between the already-present route-specific users and an explicit XRDP session
policy before authorizing any further live display mutation.
