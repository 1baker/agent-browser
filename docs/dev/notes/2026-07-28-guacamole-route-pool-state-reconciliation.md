# Guacamole Route-Pool State Reconciliation Validation

Date: 2026-07-28
Plan: P81
Result: PASS

## Incident

A guarded Plan 0012 route-open selected legacy retained route
`guacamole:4` on display `:10`, while current route readiness and remote-view
doctor selected `guacamole:1` on `:11`. The acquisition stopped before browser
launch and rolled its lease back. No source authentication probe, canary, or
request ID was consumed.

## Root Cause

The local convergence interlock ran the authoritative Guacamole route-pool
readiness check but discarded its JSON output. Its later `service reconcile`
call performed health reconciliation only, so stable retained entry
`guacamole-rdp-a` continued to point at a legacy route definition.

The remote-view selector behaved correctly for its retained input. The defect
was missing readiness-to-state projection, not route ranking, source
authentication, Guacamole authentication, or browser launch.

## Repair

- `service reconcile` now accepts
  `--authoritative-route-pool-json <json-array>`.
- Reconciliation validates unique nonempty stable IDs and route IDs.
- Stale inactive definitions are replaced by stable entry ID.
- Same-route active lease state is preserved.
- A conflicting active allocation is not redirected and is reported through
  `skippedActiveConflictEntryIds`.
- Repository persistence preserves any route lease or allocation that changes
  while reconciliation is probing, so a concurrent checkout cannot be
  overwritten by an older snapshot.
- The response exposes `routePoolRefresh`, with matching JSON schema and
  generated client declarations.
- Local convergence passes only successful route-readiness JSON into
  reconciliation.

## Deterministic Validation

- focused Rust tests passed for CLI parsing, stale available definition
  refresh, active and incomplete conflict preservation, concurrent checkout
  preservation, and newer route-release preservation;
- `pnpm test:local-runtime-convergence` passed its source contract and behavior
  fixtures;
- Rust format and clippy passed;
- route-confusion no-launch gates passed;
- service API and MCP parity passed;
- the complete service-client suite passed;
- docs production build passed;
- patch whitespace validation passed.

## Installed And Live Evidence

- Installed version: `agent-browser 0.27.0`.
- Installed executable SHA-256:
  `f016e7579b9e8b9a5f10548e683e3cbf4192b1a6344ddd97311622b1d3835f18`.
- Pre-repair retained-state backup:
  `/home/ecochran76/.agent-browser/backups/service-state/20260728-p81-pre-reconcile-state.json`.
- Backup SHA-256:
  `47b0c39aede63a6f8af55f319414c4cc434e113847e5e59bfe04c93b706c1bd8`.
- Applied convergence receipt:
  `/home/ecochran76/.agent-browser/convergence/local-runtime-latest.json`.
- The applied convergence and the next scheduled interlock pass both
  completed successfully with no failed steps.
- Install doctor reports six converged runtimes and zero stale runtimes.
- Remote-view doctor reports ready and selects `guacamole-rdp-a`,
  `guacamole:1`, and `:11`.

Retained route-pool state after convergence:

- `guacamole-rdp-a`: route `guacamole:1`, connection `1`, display `:11`,
  state `available`, no current allocation;
- `guacamole-rdp-b`: route `guacamole:2`, connection `2`, display `:12`,
  state `available`, no current allocation.

## No-Launch Selector Proof

The normal stable-entry command used the `last30days-facebook` runtime profile,
`stealthcdp_chromium`, RDP gateway view provider, and
`--route-pool-entry-id guacamole-rdp-a` with `--dry-run`.

It returned:

- selected route-pool entry `guacamole-rdp-a`;
- selected route `guacamole:1`;
- connection `1`;
- display and launch display `:11`;
- `browserLaunchRequested: false`;
- `routeCheckoutRequested: false`;
- `tabOpenRequested: false`.

No browser, source authentication probe, source canary, or Plan 0012 request
ran. The next Plan 0012 attempt remains behind fresh explicit authorization.
