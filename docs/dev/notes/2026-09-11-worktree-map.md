# Active worktree boundaries

This checkout contains multiple unfinished changes, not disposable build debris.
At inspection it had 32 modified tracked files and 37 untracked files. This
inventory is a routing aid, not a claim that every file has completed review.
Do not publish the entire checkout as the desktop-startup fix.

## Workshop desktop and boot recovery

- Runtime: scripts/workshop-boot-recovery.mjs and the matching service/timer.
- Desktop repair: scripts/open-rdp-guac-route-displays.js.
- Tests: scripts/test-route-desktop-startup.mjs and
  scripts/test-workshop-boot-recovery*.mjs.
- Evidence: 2026-09-11-workshop-recovery-history.md and the short RUNBOOK entry.
- Validation: pnpm test:workshop-recovery; opt-in real user-systemd coverage:
  pnpm test:workshop-recovery:systemd.

The operator boot helper is separately installed. The desktop opener belongs
to the packaged controller payload. These are different deployment boundaries.
Keep the current runtime filenames: the installer embeds the opener, tests
reference those paths, and installed operator configuration pins helper bytes.

The scoped rebuild now lives in the separate detached checkout
`/home/bak3r/projects/agent-browser-startup-20260911`; its generated build cache
and candidate are not changes to this worktree. The publisher controller-update
option and provenance snapshots support paired installation and rollback.
RUNBOOK records the candidate digest, fixture-install/reinstall proof and the
September 12 guarded installation after the quiet-window gate was satisfied.
Do not publish the separate Plan 0131 changes as part of that candidate.

## Publication and cold rollback

scripts/lib/local-dashboard-* and matching test-local-dashboard-* files include
prebuilt publication, manifest provenance, interlock, quiescence and cold
rollback work. See 2026-09-11-cold-restart-rollback-core.md. The cold adapter is
not complete; its fixtures are not permission to replace the signed-in browser.
Do not fold this entire group into the small desktop-command patch.

## Separate active work

- Temporary route provisioning: temporary-rdp-route modules, setup and tests;
  Plan 0129. The desktop opener has overlapping changes from this work.
- Private credential execution: Plan 0130, runtime_attach_proof.rs and related
  actions/service-health changes.
- Background exit evidence: Plan 0131 and control_plane.rs.
- Retained identity/readiness: retained_browser_requirement.rs,
  service_access.rs and related tests.

README, RUNBOOK, package.json, CLI help, docs and the browser skill contain
changes from several groups. Review hunks, not just filenames, before staging.

## Cleanup performed

Moved the 271-line Workshop history out of the main runbook without discarding
its content. Added one named isolated test suite and a separate opt-in systemd
suite. Left executable paths, installed files, live sessions and unrelated
source unchanged. No stash, reset, deletion, commit or push was performed.
No delegation: this cleanup was a bounded documentation move and test routing
change; independent runtime review from earlier work was not reused as proof
of new behavior.

The checkout intentionally remains dirty. A clean Git status requires reviewed
workstream commits or an explicitly agreed archival separation; hiding these
files in ignore rules would not resolve the unfinished work.
