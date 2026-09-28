# Plan 0105 | Durable Task Authority Envelope

State: CLOSED
Roadmap: P105
Plan version: 1
Date: 2026-08-14

## Objective

Add a durable, fail-closed task authority boundary above individual browser
actions. An agentic task must remain bound to one immutable authority identity,
caller task, allowed origins, retained target, evidence budget, consequence
ceiling, and expiry across commands and daemon restarts.

## Scope

- accept a structured `taskAuthority` envelope on browser commands;
- validate its immutable identity, task/caller binding, allowed origins,
  retained target, evidence budget, consequence ceiling, and expiry;
- persist an atomic per-session authority ledger so action and evidence
  reservations survive daemon restart;
- reject envelope drift, origin drift, target drift, expiry, and exhausted
  budgets before command execution;
- route actions above the consequence ceiling through P104's fresh exact-target
  confirmation path;
- support an explicit required mode for agentic runs while preserving exact
  cleanup and existing non-agentic compatibility;
- prove the behavior on a disposable public target without authenticated state
  or external mutation.

## Non-Goals

- no ChatGPT prompt, authenticated-site action, login, upload, download, form
  submission, or external mutation;
- no cryptographic operator signing or remote policy-distribution system;
- no GitHub write or unrelated dirty-worktree reconciliation;
- no workstation privileged-helper installation.

## Acceptance Criteria

1. A valid envelope admits bounded read-only work only for its exact task,
   target, origin, and unexpired immutable authority identity.
2. Ledger usage survives a fresh daemon state and rejects envelope drift or an
   exhausted action/evidence budget before execution.
3. Origin, target, task, expiry, malformed envelope, and missing required-mode
   authority all fail closed.
4. An action above the consequence ceiling stages P104 confirmation and can run
   only after a fresh matching confirmation against the same target.
5. Focused tests, deterministic full Rust tests, formatting, strict Clippy,
   docs checks, release build, validation selection, diff hygiene, installed
   runtime parity, and a disposable public no-mutation proof pass.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: current commands carry trace-only `taskName` labels and P104
  protects individual actions, but no immutable task envelope, durable usage
  ledger, allowed-origin set, retained-target constraint, evidence budget,
  consequence ceiling, or task expiry exists.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: inherited goal to continue the best agentic-browser
  recommendations; P104 explicitly recommends this task-level boundary.
- next_action_or_stop_reason: implement the envelope and durable ledger at the
  common command-admission boundary, then validate it provider-free.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: one strict envelope now binds exact task/caller labels, retained
  target and initial URL, allowed origins, action/evidence budgets, consequence
  ceiling, and expiry. Its full hash and usage are persisted atomically before
  dispatch; restart preserves usage and envelope drift fails closed.
- regression: four focused authority tests prove durable reuse, drift and
  budget rejection, target/origin/task/expiry rejection, required-mode
  rejection, and above-ceiling confirmation. HTTP propagation, confirmation,
  service contract/types/examples, and API/MCP parity tests pass. The serial
  suite passed 1,828 tests with 57 ignored and zero failures.
- live_proof: debug and installed binaries passed the disposable Example
  Domain proof. It admitted two authorized reads, rejected drift and wrong
  origin, staged then denied an above-ceiling click without mutation, rejected
  exhausted budget, and cleaned up exactly. No authenticated profile or prompt
  was used.
- installed_runtime: release, installed, reference, and live-dashboard hashes
  are `9fe62980912e20c0e5c1db2c2b5538edcadba9adf106ab9a0d1ef3da6133c9df`.
  Handoff preserved retained browser PID `1046742`, the same CDP endpoint,
  exact target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical URL, and title.
- validation: formatting, strict Clippy, docs build and TypeScript, dashboard
  build, release build, validation selection, JavaScript syntax, and diff
  hygiene passed. Targeted MDX lint had no errors and three ignored-file
  warnings. No `plans:audit` package script exists in this checkout.
- environment_note: install doctor confirms a ready current live dashboard and
  zero stale runtimes, but remains partial for the separate workstation
  payload provenance and missing privileged remote-view helper/group/sudoers.
  Those require operator installation and interactive sudo.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- material_blockers: none for P105.
- next_action_or_stop_reason: stop this slice; next issue a minimal envelope as
  a first-class broker operation with read-only issuer/revocation/usage status.
