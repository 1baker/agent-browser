# Plan 0187 | Legacy Generation And Custody Recovery

Date: 2026-09-20

State: OPEN

Lane: P187

Product lane: PL-BUGFIX

Branch: `fix/plan-0187-legacy-generation-custody-recovery`

Target: `main`

Integration: merge

Work item: local autonomous recovery goal `01a04493-d237-7483-a944-a0b35a58536f`

Consolidation: required

## Objective

Let a legacy mutable workstation installation enter the immutable-generation
upgrade transaction before candidate staging, while preserving a live
transferred browser whose old custody projection is inconsistent. Recover the
retained ChatGPT browser through the existing exact orphan-adoption path
without launching another browser, closing a tab, replaying a prompt, or
rewriting Service State by hand.

## Current State

The installed `0.28.0` daemon and Chrome process are alive, but its committed
custody receipt names a closed target. Exact task-authority status fails with
`handoff_snapshot_identity_mismatch`. Current source contains a two-phase owner
registry and exact orphan-adoption path, and its dry-run can stage the legacy
Service State migration without protected record removal.

The first production apply stopped before runtime or payload effects. Candidate
staging calls `validate_generation_install_preconditions` before the legacy
mutable payload is imported as an immutable generation, so the later
`migrate_legacy_payload_to_generation` branch is unreachable. A separate local
change correctly classifies `handoff_custody_receipt_snapshot_mismatch` as
preserve-only rather than a stale alias; this lane carries that behavior with
focused regression coverage.

After that ordering defect was repaired, the first effect-free census exposed
four live managed Chrome rows that predate persisted browser process identity.
Their process, registered profile path, `DevToolsActivePort`, exact loopback
WebSocket endpoint, browser family, and target enumeration agree, but PID-only
legacy handling intentionally leaves them ambiguous. This lane therefore adds
a read-only all-axis compatibility proof for transaction classification; it
does not backfill Service State or grant authority from PID alone. The exact
blocked transaction was closed through its compare-and-swap guard with the old
generation preserved before any retry.

## Consolidated Batch

- Import and select the exact legacy payload before staging a candidate when
  no immutable generation selector exists.
- Record the imported generation as the transaction's old generation before
  any candidate activation or runtime transfer.
- Preserve committed custody projection mismatches; never retire that daemon
  as an observation-only or browser-unavailable alias.
- Retain transaction rollback evidence and fail closed on ambiguous census,
  process, profile, endpoint, target-set, or owner evidence.
- Use the existing orphan-adoption path after exact old-daemon revocation; do
  not add another tunnel, browser, profile, or general recovery endpoint.

## Delivery Sequence And Budget

1. Add red focused tests for legacy import ordering and custody mismatch
   classification, then implement the smallest installer correction.
2. Run focused Rust tests, format, Clippy for the changed target, and the
   workstation fixture plus planning audit.
3. Build one candidate per newly exposed pre-effect blocker, run workstation
   dry-run, create the installer backup, and perform one guarded apply for that
   reviewed candidate. The first attempt remained effect-free and is terminal;
   permit one final candidate/apply cycle for the all-axis legacy census proof.
4. If activation stops after an irreversible owner transition, follow only the
   exact transaction's advertised forward recovery. Otherwise retain the old
   runtime and stop the production attempt.

Overall effort ceiling: two source repairs, two candidate builds, two
effect-bounded production applies, and one transaction-bound recovery attempt.
Any further new pre-effect blocker returns this lane to review before another
apply; any irreversible owner transition remains forward-only.

## Verification And Current Blocker

- The legacy-import regression was red on the original source and green after
  moving exact legacy generation import before candidate staging.
- Focused legacy import, custody classification, ownerless bootstrap, and
  all-axis legacy browser probe tests pass. `cargo fmt --check`, `git
  diff --check`, and zero-warning Clippy pass for the CLI target.
- Transaction `upgrade-7023bf06-fd24-42b3-9417-934054b3c97f` reached stable
  census, staged and validated Service State migration, and pre-admission
  requalification. Host preparation then failed before admission drain because
  the installed root-owned remote-view helper lacks the candidate runtime
  contract and non-interactive sudo could not authenticate.
- The transaction rolled back to `failed_preserved_old_generation` at revision
  10 with zero runtime handoffs and zero outstanding owner obligations. The
  four retained Chrome PIDs and the old daemon PID remained unchanged.
- Local Agent Browser and Codex Research/Graphiti MCP calls still respond. The
  no-launch access plan is healthy, but exact task-authority readback remains
  degraded with `handoff_snapshot_identity_mismatch` until the privileged
  helper is upgraded and this guarded install is rerun.

The next action is a local authenticated privilege-helper installation in a
protected terminal, followed by the same dry-run, backup, guarded apply, and
retained-process acceptance checks. Do not place the sudo password in chat or
shell arguments.

## Worker Assignments

- Primary Codex owner: installer sequencing, exact live evidence, validation,
  production transaction, and final acceptance.
- No delegated workers: the critical path is one overlapping installer file
  and one live retained-browser transaction.

## Evidence And Exit

- A focused regression fails before and passes after legacy import occurs
  before candidate staging.
- Custody projection mismatch remains preserve-only in candidate selection.
- Candidate workstation dry-run reports no protected record removals and a
  forward-compatible Service State migration.
- Production readback proves the same Chrome PID, process start identity,
  profile inode, and three preexisting target IDs survived.
- Agent Browser task-authority status succeeds on the exact intended ChatGPT
  target; AuraCall can perform a bounded read round trip without creating or
  replaying a prompt.

## Non-Goals

- No Cloudflare tunnel or new MCP server.
- No new browser/profile lane, tab replacement, prompt replay, or manual state
  editing.
- No broad cleanup of historical profiles, displays, routes, or transactions.
- No proposal-generation replay until browser custody and bounded readback pass.

## Definition Of Done

The source gates pass, the guarded production transaction is terminal and
auditable, the retained Chrome and its original targets survive, exact browser
authority succeeds, and the existing AuraCall-to-Agent-Browser path completes
a non-mutating round trip. Any missing proof leaves this plan open.
