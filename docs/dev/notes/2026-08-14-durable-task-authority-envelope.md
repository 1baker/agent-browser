# Durable Task Authority Envelope

Date: 2026-08-14
Plan: P105 / Plan 0105

## Outcome

Agentic browser work can now be constrained by one immutable, durable
`taskAuthority` envelope instead of relying only on per-action policy. The
common daemon admission boundary verifies exact task/caller labels, retained
target and first-use URL, allowed HTTP(S) origins, action/evidence budgets,
consequence ceiling, and expiry before dispatch.

The complete envelope hash and consumed budgets are written atomically to a
per-session ledger. A fresh daemon context continues the same allowance;
changed authority fields and exhausted budgets fail closed. Above-ceiling work
uses the P104 exact-target confirmation path and reserves budget only after a
matching approval.

## Verification

- focused authority, service HTTP propagation, and confirmation regressions;
- 1,828 serial Rust tests passed, 57 ignored, zero failed;
- strict Clippy, Rust formatting, service API/MCP parity, generated client
  contracts/types/examples, docs build/TypeScript, dashboard and release
  builds, validation selection, and diff hygiene passed;
- debug and installed public-only live smokes passed with no authenticated
  state, mutation, prompt, or cleanup residue;
- installed runtime hash:
  `9fe62980912e20c0e5c1db2c2b5538edcadba9adf106ab9a0d1ef3da6133c9df`.

The retained ChatGPT lane remained PID `1046742`, CDP port `39377`, target
`B0EC77F279E5434E33FEA97AB1742B1A`, URL
`https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`, and title
`Architecture Review Boundaries`. No prompt or composer action occurred.

## Remaining Environment Boundary

Install doctor reports a ready live dashboard and zero stale runtimes. Its
overall result remains partial because workstation-payload binary provenance
is older and the privileged remote-view helper, group, and sudoers policy are
not installed. That separate installation requires operator authority and an
interactive sudo session.
