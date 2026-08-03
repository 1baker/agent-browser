# Systemd Interlock Self-Quiesce Repair

Date: 2026-08-03
Plan: 0091
Status: Source repaired; installed runtime gate blocked

## Incident

The public dashboard returned HTTP 502 after
`agent-browser-runtime-interlock.service` ran the installed workstation
reconciler. The reconciler passed its own service name to `systemctl --user
stop`, so systemd terminated it with signal 15 before it could reactivate the
dashboard and timers.

The Guacamole connection-not-found report had a separate permission cause.
Connections `1` and `2` existed, but the governed `admin` entity lacked READ.
A guarded sync restored three of three required grants on each connection. The
prechange PostgreSQL backup is:

`~/.agent-browser/backups/guacamole-postgres/guacamole-postgres-20260803T162312-171716168Z.dump`

Its SHA-256 is
`95a0bdabc3e3f57fbb92fa5ef2b16baae4891268fd2cce1f824aa704c5682a75`.

## Repair

The reconcile quiesce set is now a source-local constant. It still stops the
dashboard, interlock timer, and PostgreSQL backup timer, but it does not stop
`agent-browser-runtime-interlock.service`. This keeps timer-triggered
reconciliation alive through the later activation step without changing the
public command contract.

The regression was recorded red before the behavioral change. It failed on
the assertion that the running service must be absent, then passed after the
one-line set repair.

## Deterministic Validation

Green checks:

- `cargo test --manifest-path cli/Cargo.toml systemd_reconcile_quiesce_set_preserves_its_running_service -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml workstation_install -- --nocapture --test-threads=1`, fifteen passed
- `pnpm test:workstation-install-fixture`
- `pnpm test:workstation-host-provision`
- `pnpm test:fresh-workstation-vm-harness`
- `pnpm test:workstation-guacamole-assets`
- `pnpm test:guacamole-postgres-durability`
- `pnpm test:rdp-guac-route-specific-user-sync`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `git diff --check`

`pnpm validation:select -- --base 7dd12436` selected the focused checks above.
No plan-audit package script exists in this checkout, so no such command is
claimed as validation.

## Installed and Live State

The corrected candidate and installed executable share SHA-256
`23e71f0ffd8e75355719896a71d09849f57bf6c7e5c417eaf366e8489405d684`
and report version `0.28.0`. The installed source-free payload is ready, its
binary provenance matches, and all five unit hashes match the manifest.

The workstation apply stopped at the existing interactive sudo boundary
because the root-owned privilege helper differs from the bundled helper. The
user payload had already been materialized. Install doctor consequently
initially reported twenty `active_runtime_stale_executable` issues and the corresponding
`daemon_socket_current_executable_mismatch` and
`daemon_socket_deleted_executable` issues. Closing those sessions would have
exceeded this incident scope and disrupted live owners, including
`wsl-chrome-3`.

The operator then reported `wsl-chrome-3` unreachable. Read-only diagnosis
showed that its `default` service session retained the profile lease but had no
browser, its prior Chrome PID was absent, and no live tab records remained.
One Route A recovery relaunched the durable profile on `:10`. Chrome restored
its persistent tabs. A second route operation reused the restored ChatGPT
target and rebound the route to the browser's canonical
`display:shared_display:10` allocation, avoiding a retained legacy allocation
whose profile label belonged to an older browser. The browser PID remained
stable through the reattachment.

Final read-only state:

- dashboard service enabled and active;
- local and public dashboard routes return HTTP 200;
- public Guacamole ingress is ready;
- connections `1` and `2` each retain three of three required READ grants;
- Route A remains `guacamole:1` on display `:10` and Route B remains
  `guacamole:2` on display `:11`;
- `wsl-chrome-3` is healthy on session `default`, its Chrome window is visible
  on `:10`, its Route A allocation is ready, and the restored ChatGPT target is
  active;
- PostgreSQL backup timer enabled and active;
- recurring interlock timer disabled fail-closed;
- no other browser owner was closed, relaunched, or navigated.

## Remaining Gate

Use one owner-coordinated maintenance window to refresh the root-owned helper
from an interactive sudo-capable terminal and hand off all nineteen remaining
stale daemon sessions. `wsl-chrome-3` is already on the corrected runtime. Run
one installed interlock service pass to exit status 0, verify both doctors and
public routes, then re-enable the timer. Until that gate is complete, the
dashboard is reachable but recurring reconciliation is not healthy or
authorized.
