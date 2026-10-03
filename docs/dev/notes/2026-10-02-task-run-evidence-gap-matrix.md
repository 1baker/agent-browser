# Task run evidence gap matrix

Date: 2026-10-02
Scope: source trace for plan 0133. This note does not assert live runtime state.

## Existing paths

- `cli/src/native/service_model.rs` defines `Challenge`, its states, the
  `ServiceState.challenges` map, and `BrowserTab.challenge_id`. The only
  `.challenges.insert` found in `cli/src/native/` is a fixture in
  `stream/http.rs`; production browser actions read the collection through
  `actions.rs::handle_service_challenges`. The access planner reads
  `WaitingForHuman` and sets `manual_action_required` in `service_access.rs`.
  Therefore the model and read path exist, but a page observation to persisted
  challenge writer is missing from the traced production path.
- `cli/src/native/service_config.rs::update_profile_seeding_handoff` persists
  the manual sign-in lifecycle. `record_profile_seeding_handoff_launch`
  records a detached browser, `refresh_profile_seeding_handoff_lifecycles`
  detects its exit, and a freshness update advances the handoff to `Fresh` or
  `VerificationPending`. This is a profile readiness path, not a per-tab
  challenge detector.
- `cli/src/native/task_authority.rs` persists issued authority, ordered step
  receipts, terminal outcome hashes, and pending confirmations in its own
  ledger. An admitted step without a terminal outcome is `indeterminate`.
  This path applies to authority-issued tasks, not every labeled service job.
- `cli/src/native/service_jobs.rs` retains at most 200 jobs. Several service
  event writers cap the event log at 100. These records support a current or
  recent run projection, but cannot guarantee full historical recovery.
- `cli/src/native/service_model.rs::ServiceTabHandle` binds browser, profile,
  session, tab, target, lease, and trace labels. It does not contain a durable
  task run ID. A `taskName` label is not a unique run identifier.

## Reconstruction matrix

| State | Existing source | Reliable after restart? | Gap |
| --- | --- | --- | --- |
| Current queued or running job | Service job state | Yes while retained | Job cap and no unique run ID |
| Recent success or failure | Service job result/error | Yes while retained | Job cap; browser command success is not task success |
| Authority-issued step outcome | Task authority ledger | Yes | Applies only to issued authorities |
| Manual profile seeding | Profile seeding handoff | Yes | Not linked to a task run or tab challenge |
| Current browser and tab | Service browser/tab records | Yes when state is retained | Closed targets may be pruned |
| Page challenge awaiting a human | Challenge model and access-plan reader | No observed production writer | Need bounded detector, persisted binding and pause |
| Human resolution and safe resume | Existing confirmation and handoff paths are specialized | Not for generic page challenge | Need a durable resolution fact and recheck |
| Complete task outcome and cleanup | No general task receipt | No | Add only the task facts not covered by authority receipts |

## Implementation consequence

Build a read-only view of current and recent labeled jobs first. Label it
`recent`, not durable history. Introduce a distinct run identity when a client
starts a governed task, then persist only the identity, human-gate transitions,
verified outcome, and cleanup evidence needed to reconstruct that task after
the bounded job and event logs rotate. Keep browser lifecycle in Service State.
