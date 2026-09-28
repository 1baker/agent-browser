# Confirmation Ledger Checkpoints and Dashboard Review

Date: 2026-08-14
Plan: P113 / Plan 0113

## Outcome

Retired task-authority confirmation IDs retain exact single-use membership in
fixed-capacity immutable hash-linked segments. A bounded manifest commits the
segment count, total confirmation count, active IDs and digest, and historical
head digest. Reads fail closed when a segment is missing, reordered, modified,
duplicated, over capacity, count mismatched, or disconnected from the committed
head. Legacy v1 tombstones migrate during reviewed cleanup apply.

The authenticated dashboard now provides a receipt-retention review surface.
It displays policy, candidates, requester, exact review digest, and verified
ledger evidence. Apply uses an explicit alert dialog and the preview digest;
the browser cannot synthesize requester identity or apply an unreviewed set.

## Verification

- focused authority and integrity regression tests passed;
- dashboard authority, generated client, request contract, schema, and HTTP/MCP
  parity checks passed;
- the normal parallel Rust suite passed 1,852 tests, with 57 ignored and zero
  failures;
- formatting, production-target strict clippy, release, dashboard, and docs
  builds passed;
- direct planning-surface audit, schema parse, repository/installed skill
  parity, and diff hygiene passed; this checkout has no `plans:audit` script;
- installed and release executable SHA-256 values both equal
  `efc8a0dba40989a757fd5660e93221ae3e92cd41c967feb0b0bbcdf87fa2506f`;
- authenticated live HTTP and MCP previews returned schema v2, verified ledger
  integrity, a 64-character review digest, and `apply: false`;
- the retained ChatGPT target and title were read back without a prompt, click,
  typing, navigation, or page mutation.

The publisher's optional disposable-Chrome marker smoke remains unable to
launch under WSL's Chrome sandbox settings. Direct authenticated loopback
marker, manifest, bundle, and preview checks prove the installed dashboard is
ready. Runtime inventory is converged with no stale runtime; doctor calls the
overall convergence partial because the live dashboard process is intentionally
diagnostic rather than a daemon-addressable browser session.

Strict all-target/all-feature clippy still reports 12 pre-existing test-style
warnings outside this plan. The existing workstation payload drift doctor
finding also remains outside this increment.
