# Isolated Dashboard Publisher Orchestration Fault Fixture

Date: 2026-08-15
Plan: P117

## Finding

P116 made standalone listener quiescence and restart testable with real
processes, but the executable still owned build, backup, replacement, handoff,
readiness, and rollback sequencing directly. Proving a backup restore therefore
still depended on a live publication or on duplicating production control flow
inside a test.

## Resolution

The publisher now delegates the complete mutation sequence to an injected
orchestration module. The executable retains concrete commands and filesystem,
daemon, listener, and smoke implementations; the orchestrator owns their order
and rollback decision.

The isolated fixture uses real temporary files for the installed runtime,
replacement runtime, backup, and atomic staging path. It proves:

- successful publication keeps replacement bytes and the original backup;
- failure after replacement but before browser handoff restores original bytes
  before the rollback restart;
- committed handoff evidence prevents unsafe restoration and keeps the
  replacement available for recovery;
- rollback restart failure is recorded separately while the initiating error
  remains primary;
- build failure before mutation creates no backup and performs no lifecycle or
  install operation.

Fast Linux Dashboard CI runs smoke policy, real-process lifecycle, and
real-file orchestration fixtures in order before the dashboard build.

No live installed runtime, listener, daemon, browser, profile, target, page,
prompt, credential, external service, or GitHub state is mutated by the
fixture.

## Verified Result

Focused and widened publisher, convergence, capability, parity, dashboard,
client-type, syntax, production-build, package, YAML, skill-parity, plan-audit,
cleanup, and diff gates pass. Docs-wide ESLint retains one pre-existing React
effect error and one pre-existing search-route warning in untouched files.

Read-only live evidence still reports retained browser PID 1046742, its exact
CDP endpoint, ready health, Workshop URL, and `Architecture Review Boundaries`
title. The installed runtime and workspace reference binary remain at SHA-256
`07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`;
the live dashboard manifest remains ready with dashboard SHA-256
`867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`.

The next safety gap is restore evidence. A later slice should record and verify
source, backup, replacement, and restored hashes and preserve the initiating
publication failure even if the restore copy itself fails.
