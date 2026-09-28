# Plan 0113 | Confirmation Ledger Checkpoints and Cleanup Review

State: CLOSED
Roadmap: P113
Plan version: 2
Date: 2026-08-14

## Objective

Preserve exact single-use confirmation evidence while keeping each retired-ID
ledger artifact bounded, make ledger integrity and cleanup review evidence
visible to authenticated dashboard operators, and remove the shared test-state
races exposed by the P112 widened run.

## Scope

- migrate the mutable retired-ID set to a bounded active manifest plus
  fixed-capacity immutable hash-chained segments;
- verify segment order, linkage, counts, duplicate absence, and head digest on
  every membership read and fail closed on mismatch;
- preserve exact membership for every retired confirmation ID without a
  probabilistic false-negative path;
- return compact verified-ledger evidence from cleanup preview and apply;
- add dashboard controls for retention policy preview, digest review, candidate
  evidence, and exact-digest apply;
- isolate Chrome environment tests and remote-view handoff stores from parallel
  test mutation.

## Non-Goals

- no deletion of historical single-use membership evidence;
- no prompt submission, browser navigation, page mutation, credential handling,
  external write, or GitHub write.

## Acceptance Criteria

1. Every segment contains no more than the configured capacity, uses an exact
   predecessor digest, and is committed by the bounded manifest head.
2. Missing, reordered, modified, duplicate, count-mismatched, or head-mismatched
   segment evidence fails closed before a confirmation ID can be staged.
3. Legacy tombstones migrate on the next reviewed cleanup apply, and every old
   ID remains single use.
4. Dashboard preview displays policy, candidates, authenticated requester,
   exact review digest, and verified ledger evidence; apply requires the same
   digest and an explicit alert-dialog decision.
5. The previously flaky Chrome argument and remote-view handoff tests pass in
   repeated parallel execution without shared ambient paths.
6. Focused tests, client and dashboard contracts, parity, formatting, strict
   production lint, widened Rust tests, builds, install, and safe retained-lane
   readback pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: P112 stores every retired ID in one mutable JSON set, the dashboard
  does not expose cleanup preview or ledger integrity, two Chrome argument tests
  read ambient HOME without the shared environment lock, and remote-view tests
  place independent state files beside one shared `/tmp/remote-view-handoffs.json`.
- discovery_status: CodeGraph is not initialized and the repository-required
  Graphiti discovery skill is unavailable. Direct source, plan, contract, and
  test inspection is the verified fallback.
- subagent_status: not_spawned; active policy prohibits proactive delegation.
- authority_classification: active user goal to continue implementing the best
  recommendations after P112.
- next_action_or_stop_reason: implement checkpointed exact membership,
  dashboard review, and parallel-safe test fixtures.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- implementation: confirmation tombstones now use a bounded v2 manifest and
  fixed-capacity immutable hash-linked segments. Membership reads verify every
  segment, predecessor, count, duplicate constraint, active digest, and head
  digest before accepting an ID. Cleanup apply migrates legacy v1 evidence.
- operator_surface: the authenticated Authorities workspace previews retention
  policy, exact candidates, requester, review digest, and verified ledger
  evidence, then requires an explicit alert-dialog decision with the same
  digest before apply.
- deterministic_fixtures: Chrome argument tests share the environment guard;
  remote-view rollback state uses a unique per-test directory.
- focused_validation: task-authority focused tests, bounded/hash-linked and
  fail-closed integrity variants, dashboard authority assertions, generated
  client and request contracts, HTTP/MCP parity, schema parsing, formatting,
  release build, dashboard build, docs build, and production-target strict
  clippy passed.
- widened_validation: `cargo test --manifest-path cli/Cargo.toml` passed 1,852
  tests with 57 ignored and zero failures under normal parallel execution.
- installed_validation: installed and release executables match SHA-256
  `efc8a0dba40989a757fd5660e93221ae3e92cd41c967feb0b0bbcdf87fa2506f`;
  the live dashboard served bundle manifest SHA-256
  `ad4630a4aba2b6382b28090f5f8508b0c4a8cd9122b112ddd91c5cf1af4659b2`;
  authenticated HTTP and MCP cleanup previews returned schema v2 with verified
  integrity and did not apply cleanup.
- retained_lane: the exact retained ChatGPT target remained
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2` with title
  `Architecture Review Boundaries`; no prompt, typing, click, navigation, or
  page mutation occurred.
- runtime_diagnostics: inventory is converged with zero stale runtimes. Doctor
  labels convergence `partial` only because the healthy live dashboard process
  is diagnostic rather than daemon-addressable. The optional disposable
  browser marker smoke could not launch Chrome under WSL sandboxing; direct
  authenticated loopback marker, manifest, preview, and bundle checks passed.
- residuals: strict all-target/all-feature clippy still reports 12 pre-existing
  test-style warnings outside P113. Doctor also retains the pre-existing
  `workstation_payload_partial_or_drifted` finding.
- plan_audit: this checkout has no `plans:audit` package script; direct checks
  verified the closed plan/version, closed roadmap entry, Turn 160 outcome,
  validation note, schema parse, installed-skill parity, and diff hygiene.
- publication: no GitHub or external write occurred.
- next_action_or_stop_reason: P113 acceptance criteria are met. The next bounded
  increment should make publisher smoke classify dashboard HTTP readiness
  independently from optional disposable-Chrome readiness so a successful
  install is not reported as failed solely by WSL Chrome sandboxing.
