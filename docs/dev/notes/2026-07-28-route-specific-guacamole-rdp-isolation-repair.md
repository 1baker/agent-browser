# Route-Specific Guacamole RDP Isolation Repair

Date: 2026-07-28
Plan: P79
Status: deterministic repair validated; live gate pending

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

## Live Gate

The live gate is authorized but has not started at this checkpoint. It permits
one migration/convergence attempt after capturing pre-state and closing only
the two failed Guacamole viewer sessions. It stops before any application
browser or source-authentication work.
