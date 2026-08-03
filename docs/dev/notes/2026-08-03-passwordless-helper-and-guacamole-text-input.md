# Passwordless Helper and Guacamole Text Input Validation

Date: 2026-08-03
Plan: 0092
Lane: P92

## Contract

Recurring route-user creation, XRDP restart, and X11 display-access grants may
cross privilege boundaries only through the fixed root-owned
`/usr/local/libexec/agent-browser/agent-browser-privileged-helper` path. A
compatible installed helper is ready when passwordless `check` and
`status-json` calls prove the route desktop session and bounded display-access
capabilities. Bundled-byte differences remain provenance evidence, not an
automatic reason to request credentials.

The managed Chrome AppArmor profile is required only when the kernel enables
AppArmor and restricts unprivileged user namespaces. On other kernels,
including this WSL2 host with AppArmor reported as disabled, the policy is not
an effective dependency and must not trigger a repeat bootstrap.

Guacamole text input is a browser-origin preference rather than an RDP
connection parameter. The extension performs migration version `1`: when the
version marker is absent, it preserves existing preferences, sets
`inputMethod` to `text`, and writes the marker. Once marked, later user changes
remain untouched.

## Deterministic Evidence

The privilege clean fixture proves a byte-different compatible helper causes
only passwordless `check` and `status-json` calls on rerun, with no `sudo -v`,
helper install, or sudoers install. The host-provision fixture proves the same
for a compatible local AppArmor annotation and then removes the profile under
an AppArmor-disabled WSL-like kernel without causing another authorization.
Missing or incompatible helpers still retain the one-time bootstrap.

The Guacamole asset fixture checks the extension manifest, migration behavior
for empty and prior-default storage, preservation of post-migration overrides,
compose mount, and generated JAR contents. The Rust workstation installer tests
verify the materialized JAR contains both extension files.

## Live Evidence

The source privilege installer was run with `--apply` and
`--with-workstation-deps`. It exited successfully with
`No privileged changes were needed` and did not request an interactive sudo
authorization. Installed doctor readback reports helper version
`2026-06-23.p44-route-desktop-v2`, helper readiness true, and
`requiresInteractiveSudo: false` despite the unavailable newer
`verify-install` command.

Before Guacamole reconciliation, the installed backup command published:

- path:
  `~/.agent-browser/backups/guacamole-postgres/guacamole-postgres-20260803T172928-655720657Z.dump`
- mode: `0600`
- size: `126300` bytes
- SHA-256:
  `84543945df97b756b2cee9ed529622fd5f5e7755a5ee419e4db6cd978f06c4b7`

Only `agent-browser-guacamole` was recreated. Container identity checks proved
`agent-browser-guacamole-postgres` and `agent-browser-guacd` unchanged.
Guacamole logged `agent-browser workstation defaults` loaded from
`agent-browser-defaults.jar`. The served `app.js` includes
`AGENT_BROWSER_GUAC_DEFAULTS_VERSION` and the text-input assignment. A fresh
disposable browser origin at `http://127.0.0.1:8092` read back:

```json
{
  "preferences": {
    "inputMethod": "text"
  },
  "migration": "1"
}
```

The disposable browser and profile were closed and removed. PostgreSQL
readback remained 2 connections, 22 connection parameters, and 6 connection
permissions. Connections `1` and `2` retained their Route A and Route B names,
host, port 3389, and route-specific users. Public ingress returned the expected
authentication redirect and an HTTP 200 login page.

Final route verification found that the durable `wsl-chrome-3` profile had a
dead retained DevTools port and no browser process. One operator-directed
Route A recovery reused the restored ChatGPT target. Its exact proof returned
profile `wsl-chrome-3`, session `default`, browser `session:default`, display
`:10`, allocation `display:shared_display:10`, route `guacamole:1`, pool entry
`guacamole-rdp-a`, visible Chrome window, selected target, stream, and operator
access all ready.

## State Boundary

Source implementation, the shared installed agent skill, and the live
Guacamole extension are current. The installed 0.28.0 binary remains the prior
reviewed candidate with SHA-256
`23e71f0ffd8e75355719896a71d09849f57bf6c7e5c417eaf366e8489405d684`.
Replacing it would make current daemon owners stale, so candidate installation
and the remaining stale-daemon handoffs stay in the coordinated Plan 0091
maintenance window. Four stale daemon owners remained at final readback. The
recurring interlock timer remains disabled; the daily PostgreSQL backup timer
remains enabled.
