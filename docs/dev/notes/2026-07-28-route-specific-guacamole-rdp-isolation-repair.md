# Route-Specific Guacamole RDP Isolation Repair

Date: 2026-07-28
Plan: P79
Status: complete

## Incident

The post-reboot route substrate had two Guacamole RDP connection records and
the required read grants, but only one live XRDP display. Guacamole and guacd
authenticated both routes. XRDP created display `:10` for route A and treated
route B as a reconnection to the same user's `:10` session.

The installed XRDP 0.9.24 runtime therefore disproved the fixture assumption
that Guacamole color depths 24 and 32 would produce distinct same-user session
allocation keys under sesman `Policy=Default`.

## Selected Repair

- use the existing `agent-browser-rdp-a` and `agent-browser-rdp-b` Linux users;
- migrate exact supported legacy Guacamole rows in place to preserve their
  connection ids;
- fail closed on duplicate or mixed canonical and legacy rows;
- delete `color-depth` from the two managed connections;
- leave unrelated connections unchanged;
- teach convergence to migrate before display restoration for missing fixtures
  or the typed collapsed-display repair;
- prefer current Xorg-inferred display allocation over persisted display hints;
- route the old existing-user sync spelling to the safe migration.

No XRDP policy, Linux user, password, browser, profile, or authentication state
is changed by the deterministic repair.

## Test-Driven Evidence

Observed red failures:

- convergence selected `sync:rdp-guac-existing-user-route-pool`;
- the route-specific sync command did not exist;
- the live display-selection module did not exist;
- the legacy command still required the existing-user secret.

Green checks:

```text
pnpm test:local-runtime-convergence
pnpm test:rdp-guac-route-specific-user-sync
pnpm test:rdp-route-display-selection
pnpm test:rdp-guac-postgres-hardening
node --check scripts/converge-local-runtime.js
node --check scripts/smoke-rdp-guac-route-pool-readiness.js
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml output -- --test-threads=1
pnpm --dir docs build
git diff --check
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

`shellcheck` was not installed, so the behavior fixture and Bash strict mode
remain the shell validation authorities.

## Live Result

- Pre-state and post-state receipts are retained under
  `~/.agent-browser/convergence/p79-2026-07-28/`.
- The two failed viewer sessions were closed before migration.
- The guarded transaction preserved connection ids `1` and `2`, renamed the
  rows to `Agent Browser RDP Route A/B`, configured
  `agent-browser-rdp-a/b`, deleted `color-depth`, and retained two read grants
  on each route.
- Display restoration opened route A on `:11` and route B on `:12`; both have
  live abstract X11 sockets and display access.
- The first aggregate result was unsuccessful even though migration and display
  restoration succeeded. The installed doctor ran the readiness helper with
  `scripts/` as cwd, and the helper used a cwd-relative inspector path. That
  disabled live inference only through the doctor and exposed stale configured
  display `:10`.
- A failing cwd-independence regression was added. The helper now resolves the
  inspector from `import.meta.url`; no second migration or display restoration
  was run.
- The installed doctor then reported route `:11`, remote control ready, and
  many-to-many prerequisites ready. A read-only convergence pass succeeded.
- The recurring interlock service completed at 2026-07-28 10:00:11 CDT with
  result `success`, exit status 0, and no route fixture, display restoration,
  or access-grant steps. The timer is enabled, active, and waiting.
- `agent-browser install doctor` reports zero issues and seven converged
  runtimes.
- The installed binary was not replaced. The interlock uses the repaired repo
  scripts from its configured working directory.
- No application browser or source-authentication surface was opened.

Source commits `2dcac761` and `641f45ae` are pushed to `origin/main`.

## Residual Durability Risk

No usable Guacamole PostgreSQL backup was found during P78. The cause of the
repeated database reinitialization events is still unexplained. Backup,
retention, restore validation, and reset attribution remain a separate bounded
durability packet.
