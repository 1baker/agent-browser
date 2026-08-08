# Last30Days Remote-Control Duplicate-Pressure Investigation Handoff

Date: 2026-08-07

## Purpose

Investigate why non-actionable duplicate-profile pressure anywhere in the
agent-browser service state makes the single-route remote-control contract
unready for Last30Days, even when every route, display, gateway, and browser
launch prerequisite is ready.

This note is an investigation packet, not authority to close browsers, rewrite
profiles, interact with Facebook, or weaken genuine readiness failures.

## User Impact

Last30Days service 0.3.13 now detects recurring Facebook authentication and
checkpoint incidents and can send an operator handoff through its configured
Slack Receipts route. It requires both of these proofs before including a link:

1. `agent-browser doctor remote-view --json` reports
   `remoteControl.status=ready`.
2. `remote-view open` reports `operatorVisible.state=ready` with an external
   HTTPS URL.

The first proof currently fails. Last30Days therefore behaves safely by
sending an intervention-required notification without a link.

Durable source evidence:

- Last30Days commit `94e66a11ab876d1a0b08a5d532b0b5a8086dfb5f`.
- Last30Days receipt
  `/home/ecochran76/workspace.local/last30days-skill/docs/dev/notes/0097-plan0026-recurring-browser-reauthentication-notifications.json`.
- Last30Days Plan 0026
  `/home/ecochran76/workspace.local/last30days-skill/docs/dev/plans/0026-2026-08-07-recurring-browser-reauthentication-notifications.md`.

## Current Reproduction

Installed runtime: agent-browser 0.28.0.

After `agent-browser install workstation --apply --json` and
`pnpm converge:local-runtime -- --apply --json`, current readback is:

```json
{
  "install_doctor": {
    "success": false,
    "issues": ["service_duplicate_profile_pressure"],
    "candidateCount": 0,
    "readinessImpactingCandidates": 0,
    "duplicateProfilePressureWarnings": 2,
    "runtimeConvergence": "converged",
    "workstationPayloadReady": true
  },
  "remote_control": {
    "status": "needs_browser_launch_prerequisites",
    "ready": false,
    "installReady": false,
    "routePoolReady": true,
    "rdpGatewayReady": true,
    "browserLaunchReady": true,
    "nextAction": "repair_install_drift"
  }
}
```

Current `service status` contains no session or browser whose profile is
`last30days-facebook`. The duplicate-pressure warnings are therefore not a
current collision on the profile requesting the future handoff. The default
profile presently has multiple live session/browser records, but the resource
monitor reports zero readiness-impacting cleanup candidates.

Reproduce with metadata-only output:

```bash
agent-browser install doctor --json | jq '{
  success,
  issues: [.data.issues[]? | {code, message, nextAction}],
  serviceResources: {
    candidateCount: .data.serviceResources.candidateCount,
    readinessImpactingCandidates: .data.serviceResources.readinessImpactingCandidates,
    duplicateProfilePressureWarnings: .data.serviceResources.duplicateProfilePressureWarnings
  },
  runtimeConvergence: .data.runtimeConvergence.status,
  workstationPayloadReady: .data.workstationPayload.ready
}'

agent-browser doctor remote-view --json | jq '{
  success,
  status: .data.status,
  remoteControl: {
    status: .data.remoteControl.status,
    ready: .data.remoteControl.ready,
    installReady: .data.remoteControl.installReady,
    routePoolReady: .data.remoteControl.routePoolReady,
    rdpGatewayReady: .data.remoteControl.rdpGatewayReady,
    browserLaunchReady: .data.remoteControl.browserLaunchReady,
    nextAction: .data.remoteControl.nextAction
  }
}'

agent-browser service status --json | jq '{
  last30daysSessions: [
    .data.service_state.sessions | to_entries[] |
    select(.value.profileId == "last30days-facebook") |
    {session: .key, browserCount: (.value.browserIds | length)}
  ],
  last30daysBrowsers: [
    .data.service_state.browsers | to_entries[] |
    select(.value.profileId == "last30days-facebook") |
    {browserId: .key, health: .value.health}
  ]
}'
```

Do not copy full doctor output into issues or notes. It contains route and
workstation details that are unnecessary for this defect.

## Source-Level Failure Chain

The current behavior is deterministic:

1. `cli/src/install.rs:1582` treats
   `readinessImpactingCandidates > 0` as
   `service_resource_candidates_ready`.
2. Independently, `cli/src/install.rs:1593` treats every
   `duplicateProfilePressureWarnings > 0` as the issue
   `service_duplicate_profile_pressure`, even when
   `readinessImpactingCandidates == 0`.
3. `cli/src/install.rs:1122` defines install-doctor success as
   `issues.is_empty()`.
4. The existing test at `cli/src/install.rs:3834` explicitly freezes this
   behavior: one duplicate warning and zero readiness-impacting candidates
   must still produce `service_duplicate_profile_pressure`.
5. `cli/src/remote_view_doctor.rs:993` derives `installReady` only from the
   install doctor's top-level `success` value.
6. `cli/src/remote_view_doctor.rs:1051` requires `installReady` in the
   conjunction for `remoteControl.ready`.
7. When all route/display prerequisites are ready but install is false,
   `cli/src/remote_view_doctor.rs:1059` reports
   `needs_browser_launch_prerequisites`, and line 1081 recommends
   `repair_install_drift`.

The apparent contract mismatch is that the resource monitor distinguishes
warnings from readiness-impacting candidates, while install doctor promotes
every duplicate-pressure warning into a fatal issue. Remote-view doctor then
consumes only the aggregate install success for its readiness conjunction.

## Investigation Questions

1. Is `service_duplicate_profile_pressure` intended to be diagnostic warning,
   install drift, or a remote-control interlock?
2. Should unrelated profile pressure block a handoff for a different profile?
3. If duplicate pressure has zero readiness-impacting candidates, what safe
   operator action can satisfy `repair_install_drift`? The current issue has no
   `nextAction` or remedy.
4. Should install doctor expose separate `issues` and `warnings` collections,
   or should remote-view doctor classify issue codes by relevance?
5. Does the interlock need a target profile argument so genuine same-profile
   lease conflict stays fatal without making unrelated pressure global?
6. Can the status label distinguish install provenance drift from resource
   policy pressure? `needs_browser_launch_prerequisites` currently obscures
   that every launch prerequisite is already true.

## Required Safety Properties

- Genuine readiness-impacting stale resources remain fatal.
- A real same-profile browser or lease conflict remains fail-closed.
- No doctor or repair path automatically closes a retained browser, removes a
  profile, revokes a lease, or kills an unrelated session.
- Remote control never becomes ready from URL presence alone.
- Route pool, gateway, display, display access, browser launch, and external
  operator-route proofs remain required.
- The fix must not special-case Last30Days or the `last30days-facebook`
  profile name.
- Diagnosis and tests must not launch Facebook or perform authentication.

## Suggested Acceptance Matrix

1. `readinessImpactingCandidates=0`,
   `duplicateProfilePressureWarnings=0`: install and remote control may be
   ready when all other prerequisites pass.
2. `readinessImpactingCandidates=0`,
   `duplicateProfilePressureWarnings>0` on an unrelated profile: the warning
   remains observable, but does not make an otherwise ready target handoff
   globally unready.
3. `readinessImpactingCandidates>0`: install and remote control remain
   fail-closed with a concrete next action.
4. Duplicate pressure on the requested target profile with a real ownership or
   lease ambiguity remains fail-closed and identifies the target-scoped
   conflict without exposing private browser state.
5. A nonblocking warning alone does not produce the misleading
   `repair_install_drift` action.
6. Existing route, gateway, display-access, browser-launch, stale-runtime, and
   workstation-payload failure tests continue to pass.

Candidate focused validation commands:

```bash
cargo test --manifest-path cli/Cargo.toml install_doctor_flags_duplicate_profile_pressure
cargo test --manifest-path cli/Cargo.toml remote_control
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm validation:select -- --base fb10aca262b5a4e7fa398fa3c69a75fc6e0dd942
```

Any implementation should add a red regression that spans install-doctor issue
classification through remote-control readiness. Unit tests at only one side
of the seam are insufficient.

## Hard Stops

- Do not close or restart live sessions merely to make doctor green.
- Do not run resource-monitor apply mode without independently proven stale
  candidates and explicit authority.
- Do not create another browser/profile lane.
- Do not retry Facebook, complete its checkpoint, or perform any login action.
- Do not publish local dashboard URLs, route IDs, cookies, credentials, page
  content, or full service-state dumps.
- Stop on target-profile ambiguity rather than treating similar session names
  as the same owner.

## Repository State At Handoff

- Repository: `/home/ecochran76/workspace.local/agent-browser`.
- Branch: `main`.
- HEAD and `origin/main`: `fb10aca262b5a4e7fa398fa3c69a75fc6e0dd942`.
- Pre-existing untracked paths `--full-page` and `docs/dev/evaluation/` belong
  to another slice. Preserve them and do not stage them with this note or a
  future repair.
- CodeGraph was healthy at 418 indexed files, 14,177 nodes, and 42,605 edges.
- Graphiti runtime was healthy. A focused search in `agent_browser_main`
  returned no directly relevant prior fact for this exact warning-to-readiness
  seam, so current repo source and runtime readbacks are authoritative.

## Suggested Skills

- `graphiti-discovery` for focused prior-decision recall in
  `agent_browser_main`.
- `codegraph-workspace` for the install-doctor to remote-control dependency
  path and blast radius.
- `diagnosing-bugs` for a falsifiable issue-severity and target-scope
  investigation.
- `agent-browser` for the operator-handoff safety contract and live read-only
  validation sequence.

## Next Bounded Action

Write one red cross-seam regression for the unrelated-warning case, decide and
document the warning versus fatal-issue contract, implement the narrowest
generic fix, and validate it without mutating live browsers. Only after source
and fixture validation should a reviewed installed-runtime readback confirm
that `remoteControl.status=ready` when no target-relevant blocker exists.
