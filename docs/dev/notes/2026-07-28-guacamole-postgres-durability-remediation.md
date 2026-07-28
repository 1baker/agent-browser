# Guacamole PostgreSQL Durability Remediation

Date: 2026-07-28
Plan: P80

## Outcome

Guacamole PostgreSQL now uses a Docker named volume, has a recorded cluster
identity, produces atomic checksummed daily backups, and has a passing isolated
restore drill. The current two-route database survived the migration unchanged.

## Root Cause

The repeated initialization events were caused by a stale Docker Desktop WSL
bind attachment, not by PostgreSQL, repo recovery scripts, or deletion of the
host directory.

Docker still declared
`~/.agent-browser/guacamole/data/postgres` as the bind source, but the
long-running container mounted `/var/lib/postgresql/data` from `tmpfs` at
device and inode `32:26`. The host path and a fresh probe container saw ext4 at
device and inode `2128:38068951`. The host-visible cluster identifier was
`7640969869501161507` with its last checkpoint on July 26, while the running
container held cluster `7667172823649325092` created July 27. July 26 and July
27 initializations aligned with WSL boots.

The existing schema guard compounded the incident by treating a wholly empty
database as safe to initialize after prior durable use.

## Remediation

- Captured and restore-validated the current two-route database before compose
  mutation.
- Replaced the PostgreSQL WSL bind with external named volume
  `agent-browser-guacamole-postgres-data`.
- Restored the verified dump with clean replacement into the isolated fresh
  database while Guacamole remained stopped.
- Required exact post-restore counts before restarting Guacamole.
- Added `scripts/guacamole-postgres-durability.sh` for continuity status,
  atomic custom-format backup, identity recording, retention, and isolated
  restore drill.
- Made schema assurance fail closed on stale WSL `tmpfs`, recorded cluster
  identity mismatch, partial schema, and absent schema for a recorded identity.
- Added a persistent daily user timer with 14-backup default retention.
- Installed a source-checkout-independent helper with bounded PostgreSQL
  readiness wait, failure retry, backup locking, nanosecond names, complete-pair
  selection, and protected-retention markers.
- Added a reproducible named-volume compose override under `config/`.

## Validation

- Pre-migration dump:
  `~/.agent-browser/backups/guacamole-postgres/guacamole-postgres-20260728T164852Z.dump`
- SHA-256:
  `dd881cb6ca8e936a59f1a105ee0fedfa86942339471c6f53be45a83656e04a72`
- Retention: protected by a paired `.keep` marker
- Restore catalog entries: 193
- Restored required tables: 5
- Restored connections: 2
- Restored connection parameters: 22
- Restored connection permissions: 4
- Current cluster identity: `7667622067821133864`
- Current declared mount type: `volume`
- Current running mount filesystem: `ext4`
- Backup service: success, exit status 0
- Backup timer: enabled and active
- Runtime interlock: success, exit status 0
- Route readiness: ready
- Remote control: ready
- Many-to-many: ready
- Install doctor: success with zero issues

The old host bind directory was not deleted. It remains a non-authoritative
forensic artifact. The named volume is the live database authority and paired
checksummed dump and manifest files are the recovery authority.

An independent read-only review initially failed the slice on cleanup,
identity, publication, retention, fixture, and scheduled-runtime edges. The
bounded remediation pass fixed each finding. The fresh re-review passed with
no residual blocking findings and confirmed zero temporary restore databases.

## Recovery Contract

```text
pnpm status:rdp-guac-postgres
pnpm backup:rdp-guac-postgres
pnpm drill:rdp-guac-postgres-restore
```

Do not bypass a continuity failure with schema import. Preserve the newest
validated dump and its JSON manifest together.

An emergency production restore is a consequential manual operation, not a
timer action. Stop Guacamole web access, validate the selected manifest
checksum and `pg_restore --list` catalog, retain a safety dump when the current
cluster is readable, restore with `pg_restore --clean --if-exists` only into
the reviewed `guacamole_db` target, and require the manifest's connection and
permission counts plus all five required tables before restarting Guacamole.
The isolated drill proves that artifact first without replacing production.
