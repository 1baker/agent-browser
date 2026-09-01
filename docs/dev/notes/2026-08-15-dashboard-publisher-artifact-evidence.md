# Dashboard Publisher Artifact Evidence

Date: 2026-08-15
Plan: P118

## Finding

P117 proved the publisher's control-flow and rollback ordering, but byte
identity was asserted only by the fixture. Production reports did not retain
the built, source, backup, replacement, or restored hashes, and a restore-copy
failure could replace the initiating publication error. The publisher also had
no explicit prohibition against restarting a binary whose restore hash was
wrong.

## Resolution

The shared orchestration now makes artifact identity part of the publication
transaction:

- the built runtime is hashed before the mutation boundary;
- the installed source is hashed before and after backup creation;
- the backup must match that stable source before dashboard quiescence;
- the installed replacement must match the built runtime;
- rollback is complete only when the restored runtime matches the verified
  backup;
- restore disappearance, copy failure, and hash mismatch are recorded as
  `restoreError` while the initiating error remains primary;
- a restart after restore failure requires the current binary to match either
  the verified backup or verified replacement. Otherwise restart is skipped.

The real-file fixture corrupts each relevant transition independently and
proves the fail-closed behavior without invoking any live lifecycle adapter.

## Verified Result

Focused and widened publisher, convergence, capability, parity, dashboard,
client-type, syntax, production-build, package, YAML, skill-parity, plan-audit,
cleanup, and diff gates pass. Docs-wide ESLint retains one pre-existing React
effect error and one pre-existing search-route warning in untouched files.

The fixture also proves the committed-handoff edge: a corrupted replacement is
not restored because handoff evidence exists, but it is also not restarted
because it matches neither verified artifact. Read-only live evidence confirms
the retained browser identity, exact target, ready dashboard manifest, and
installed binary SHA remain unchanged.

The next reliability gap is crash durability. Artifact evidence currently lives
in the process report. A later slice should atomically persist a hash-bound
publication transaction journal before quiescence and make restart recovery
idempotently reconcile its last committed phase.
