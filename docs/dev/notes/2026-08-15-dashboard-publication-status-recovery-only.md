# Dashboard Publication Status and Recovery-Only Operations

Date: 2026-08-15
Plan: P120

## Finding

P119 automatically recovers an incomplete journal when the publisher runs, but
operators and automation could not inspect that state without invoking the same
entry point that normally authorizes a new build. A successful recovery was
safe, yet an absent or terminal journal allowed the command to continue into a
new publication. Inspection and reconciliation therefore lacked distinct
authority boundaries.

## Resolution

`--journal-status` returns before orchestration. It does not acquire the journal
lock, create directories, build, back up, install, restart, or touch browser
state. It reports:

- whether a journal and lock exist;
- lock owner and live or stale classification;
- transaction ID, revision, phase, terminal state, and timestamps;
- bounded handoff candidate, prepared, and resumed counts;
- current installed path, SHA-256, and verified artifact classification;
- whether recovery is safe and one exact recommended action.

`--recover-only` uses the existing secured lock and P119 recovery state machine
for a nonterminal transaction. With no incomplete transaction it returns
`nothing_to_recover` before any build adapter. The two flags are mutually
exclusive.

Package commands are:

```bash
pnpm status:local-dashboard-publication
pnpm recover:local-dashboard-publication
```

## Verified Result

Focused and widened status, recovery-only, journal, orchestration, lifecycle,
policy, convergence, capability, parity, dashboard, client-type, syntax,
production-build, package, YAML, skill-parity, plan-audit, cleanup, and diff
gates pass. Docs-wide ESLint retains one pre-existing React effect error and one
pre-existing search-route warning in untouched files.

The live read-only package command returned an absent schema-v1 journal, absent
lock, no transaction, no recoverable operation, and recommended action `none`.
The journal and lock remained absent afterward. Retained browser and installed
runtime identity remained unchanged.

The next visibility gap is the installed operator plane. A later slice should
project the same bounded read-only status into install doctor and the dashboard
service surface. Any recovery action there must retain explicit confirmation
and must not weaken recovery-only authority.
