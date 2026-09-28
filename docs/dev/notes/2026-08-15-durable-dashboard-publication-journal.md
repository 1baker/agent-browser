# Durable Dashboard Publication Journal

Date: 2026-08-15
Plan: P119

## Finding

P118 made every binary transition hash-verifiable, but the evidence existed
only in the publisher process and final report. A process death after dashboard
quiescence, runtime handoff, atomic replacement, or restart could therefore
lose the last committed phase. The next run had no authoritative way to decide
whether it should resume a retained browser, restore the source, finish
readiness, or stop.

## Resolution

The publisher now holds an exact PID lock and a schema-versioned transaction at
`~/.agent-browser/publications/local-dashboard-publication.json`. Each revision
is written mode 0600 to an exact staged file, fsynced, atomically renamed, and
followed by directory fsync where the platform supports it. Transaction ID and
revision mismatch fail closed.

The transaction checkpoints quiescence admission and completion, handoff
admission and evidence, replacement identity, handoff resumption, dashboard
restart, readiness, failure, and rollback. On the next invocation, an
incomplete transaction is recovered before any new build:

- installed bytes matching verified replacement evidence continue to readiness;
- bytes matching verified backup evidence recover to rolled back;
- a replacement matching only the durable built hash is recognized as the
  crash-after-rename window and gains recovered replacement evidence;
- exact candidate-session handoff descriptors recover a prepare response lost
  with the process;
- already-resumed exact PID and CDP evidence prevents handoff replay;
- unknown installed bytes produce `recovery_blocked` without restart.

Reference sync is safe to repeat from the verified installed replacement. A
terminal recovered journal remains as durable evidence and a later intentional
publication may replace it with a new transaction.

## Verified Result

Focused and widened journal, recovery, orchestration, lifecycle, policy,
convergence, capability, parity, dashboard, client-type, syntax,
production-build, package, YAML, skill-parity, plan-audit, cleanup, and diff
gates pass. Docs-wide ESLint retains one pre-existing React effect error and one
pre-existing search-route warning in untouched files.

A real child process exits after committing `replacement_admitted` and leaves
its exact lock. The parent proves the process is gone, safely replaces only that
stale lock, reads the durable revision, and commits the same transaction to a
terminal recovered state. Recovery fixtures additionally prove the exact
artifact and handoff branches without live adapters.

Read-only live evidence confirms the retained browser identity, target, URL,
title, dashboard manifest, and installed binary SHA are unchanged. Tests leave
no production journal or lock.

The next operator gap is visibility and intentional scope. A later slice should
expose journal status read-only and add recovery-only operation so checking or
reconciling an incomplete transaction never implies authorization for a new
build.
