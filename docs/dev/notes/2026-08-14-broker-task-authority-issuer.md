# Broker Task Authority Issuer

Date: 2026-08-14
Plan: P106 / Plan 0106

## Outcome

Task authority is now issued by the browser broker rather than trusted as a
caller-computed envelope. An approved request names the exact retained target,
caller, issuer, approval reference, short expiry, and bounded plan steps. The
broker derives unique allowed actions and origins plus exact action/evidence
budgets, pins a read-only consequence ceiling, and persists the immutable
record privately and atomically only after fresh target-bound confirmation.

HTTP and MCP provide issue, status, and revoke operations. Status reads issuer,
approval, plan, expiry, usage, remaining allowance, and revocation without
launching or changing the browser. Revoke is exact-target confirmation-gated,
idempotent for matching evidence, and remains available after same-target
navigation. Required mode accepts only a byte-equivalent active broker record;
fabricated, changed, revoked, expired, wrong-target, wrong-origin, unplanned,
and over-budget authority fails before dispatch.

## Verification

- 1,834 serial Rust tests passed, 57 ignored, zero failed;
- focused broker, HTTP, MCP, service contract, generated-client, caller-label,
  profile-lease, action-allowlist, usage, and revocation regressions passed;
- strict production Clippy, Rust formatting, client types/examples, API/MCP
  parity, docs/dashboard builds, release build, validation selection,
  JavaScript syntax, targeted MDX lint, and diff hygiene passed;
- debug and installed public-only live proofs passed with exact cleanup and no
  login, prompt, composer work, file transfer, or page mutation;
- installed/runtime/reference SHA-256:
  `86380304f45f9d8c6affc3c9caaffe85bbae26aaa18dbf260eae0d8b05cf7868`.

The retained ChatGPT lane remained browser PID `1046742`, CDP port `39377`,
target `B0EC77F279E5434E33FEA97AB1742B1A`, canonical conversation URL, and title
`Architecture Review Boundaries`. No prompt was sent.

## Remaining Environment Boundary

Install doctor reports a ready matching live dashboard and zero stale runtime
processes. Its overall status remains partial for older workstation-payload
binary provenance and the separate privileged remote-view helper, group, and
sudoers setup. Those require operator action and interactive sudo and were not
silently applied.
