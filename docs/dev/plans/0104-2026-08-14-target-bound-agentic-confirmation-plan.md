# Plan 0104 | Target-Bound Agentic Confirmation

State: CLOSED
Roadmap: P104
Plan version: 1
Date: 2026-08-14

## Objective

Turn the existing confirmation stub into a deterministic human approval
boundary suitable for retained-target agentic work. Pending actions must carry
an explicit consequence classification, an unforgeable command identifier, a
bounded lifetime, and the exact browser target identity observed when approval
was requested.

## Scope

- classify browser actions into stable consequence categories;
- allow `--confirm-actions` to select either exact actions or consequence
  categories;
- return the category, explanation, target binding, and expiry with every
  approval request;
- reject wrong, missing, expired, superseded, or target-mismatched approvals;
- prevent a second action from silently replacing an unexpired pending action;
- exercise a disposable retained public page with read-only inspection allowed
  and an external mutation stopped before execution.

## Non-Goals

- no authenticated-site mutation, provider prompt, form submission, upload,
  download, login, or challenge approval;
- no change to browser actions when confirmation policy is not configured;
- no GitHub write or unrelated dirty-worktree reconciliation.

## Acceptance Criteria

1. Exact-action and consequence-category configuration both require approval.
2. Confirmation responses include a non-empty confirmation ID, consequence
   class, description, expected target binding, and 60-second expiry.
3. Wrong IDs, expired approvals, and changed target identity fail closed and do
   not execute the pending command.
4. A live disposable retained-target task proves read-only evidence succeeds,
   a classified mutation stops at approval, denial executes no mutation, and
   exact cleanup succeeds.
5. Focused tests, full relevant Rust gates, formatting, strict Clippy,
   documentation checks, validation selection, build, and diff checks pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the existing daemon stores only action and raw command. It ignores
  the supplied confirmation ID, has no expiry, overwrites pending work, does
  not bind approval to the active target, and omits the category and description
  already promised by CLI output and documentation.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to continue the best agentic-browser
  recommendations; ROADMAP P101 explicitly names this approval boundary.
- next_action_or_stop_reason: implement the fail-closed pending-confirmation
  contract, validate it provider-free, then run one disposable no-mutation live
  proof.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: exact-action and consequence-category policy now stage one bounded
  request carrying its confirmation ID, consequence description, target ID,
  URL, and 60-second lifetime. Missing, wrong, expired, overwritten, and
  target-mismatched approvals fail closed; a matching approval executes once.
- regression: five focused confirmation tests and thirteen policy tests pass.
  The deterministic serial suite passed 1,824 tests with 57 ignored and zero
  failures; formatting, strict Clippy, docs TypeScript/build, JavaScript syntax,
  validation selection, release build, and diff hygiene passed.
- live_proof: both rebuilt debug and installed binaries passed the disposable
  public-page smoke. Read-only URL/title evidence succeeded, `external_mutation`
  stopped before execution, denial preserved the URL, navigation invalidated a
  later approval, and cleanup was exact. No authenticated profile or prompt was
  used.
- installed_runtime: release, installed, and reference binaries share SHA-256
  `0f57a4b060d68473d13b07155cd6fc502393124244d18def9340c5e7f083b468`.
  Executable handoff preserved retained browser PID `1046742`, its CDP endpoint,
  target count, canonical conversation URL, and rendered title. Restarting only
  the stale standalone dashboard listener reconciled its runtime manifest.
- safety_note: two disposable pre-build public smoke attempts ran through the
  stale debug executable and followed the Example Domain link. They touched no
  authenticated state and cleaned up exactly; the rebuilt and installed proofs
  both stopped that action before execution.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none.
- environment_note: install doctor verifies the new current/workspace binary
  hashes, live dashboard readiness, and zero stale runtimes, but remains partial
  for the separate workstation-payload provenance and remote-view privileged
  helper installation that requires interactive sudo.
- next_action_or_stop_reason: stop this slice; the next priority is a durable
  task authority envelope above these per-action confirmation boundaries.
