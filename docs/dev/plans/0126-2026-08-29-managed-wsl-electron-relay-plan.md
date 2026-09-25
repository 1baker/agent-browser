# Plan 0126 | Managed WSL Electron Relay

State: READY_FOR_DISRUPTIVE_DRILL
Roadmap: P126
Plan version: 1
Date: 2026-08-29

## Objective

Productize the verified private WSL-to-Windows Termius DevTools relay as a
supported, generic agent-browser lifecycle with dry-run-first installation,
read-only doctor evidence, drift-guarded removal, and deterministic recovery.

## Immutable Intake

Implement a supported agent-browser install, doctor, run, and uninstall
contract for one exact Windows Electron main process and its loopback-only CDP
listener. Preserve the currently retained Termius process and target until an
isolated candidate passes. Then prove normal app close/reopen and user-service
recovery without launching a duplicate app, opening a persistent shell,
changing SSH or firewall state, or exposing CDP beyond loopback. Audit the
obsolete Tailscale port 2223 mapping separately and remove it only after exact
rollback evidence. Report whether a coordinated reboot drill is ready; do not
interrupt the active Codex session without a separate execution gate.

## Frozen Acceptance Criteria

1. The public CLI supports a generic named Windows Electron relay with
   dry-run-first install, explicit apply, read-only doctor, foreground run, and
   drift-guarded uninstall operations.
2. Readiness requires exactly one matching Windows main process, exactly one
   loopback listener on the configured remote port, and listener ownership by
   that exact process. Absent, ambiguous, non-loopback, and wrong-owner states
   fail closed.
3. The WSL listener binds only `127.0.0.1`; no firewall, SSH, mirrored-network,
   app-launch, persistent-shell, or Byobu changes are made.
4. A user-scoped service dynamically reacquires the current WSL interop socket,
   exits when the Windows app exits, and is retried by a bounded timer.
5. Install is idempotent. Uninstall removes only exact managed artifacts and
   refuses unit or configuration drift.
6. Fresh-install fixtures cover apply, idempotence, doctor, absent and
   ambiguous processes, wrong listener ownership, local-port conflict, exact
   removal, and drift preservation.
7. The retained Termius PID, endpoint, and target survive candidate staging.
   Live migration occurs only after focused and widened validation passes.
8. After migration, a normal Termius close/reopen yields one new matching
   process and listener, automatic relay recovery, and preserved application
   state without a duplicate app launch.
9. Required README, CLI help, docs app, skill, inline documentation, plan,
   runbook, and roadmap surfaces remain synchronized.
10. Focused tests, Rust formatting and strict Clippy, TypeScript typecheck,
    targeted lint, production build, plan audit, validation selection,
    CodeGraph synchronization, and diff hygiene pass.
11. The bilateral workflow binds this intake, collects and approves both
    four-lobe packets, performs exactly one synthesis transit, and reaches a
    passing ChatGPT Pro review before closeout.
12. No ChatGPT prompt is sent before Codex approves the complete initiating
    packet. No browser page receives an application prompt during relay tests.

## Execution Graph

1. Freeze intake and collect independent lifecycle/security and operator/UX
   evidence into the bilateral packet workflow.
2. Implement a small generic relay lifecycle module around the existing
   private WSL loopback relay primitive.
3. Add isolated fixtures and synchronize every public documentation surface.
4. Run focused and widened validation, then complete the ordered bilateral Pro
   synthesis and review.
5. Stage and verify the installed candidate while preserving the current
   retained Termius authority.
6. Migrate the local Termius relay, prove close/reopen and service recovery,
   then audit the obsolete Tailscale mapping and reboot-drill readiness.

## Delegation Receipt

- State: `not_spawned`
- Reason: the available runtime does not authorize sub-agent spawning for this
  turn, and the relay core, CLI surface, fixtures, and migration share one
  tightly coupled lifecycle boundary.
- Runtime handle: none

## Evidence Log

- 2026-08-29: the current disposable relay is healthy on
  `127.0.0.1:19222`, maps to the exact Termius main process listener on Windows
  `127.0.0.1:9222`, and retains target
  `7624870B4329A4585A9AFF031B897B3C`.
- 2026-08-29: CodeGraph is current at 470 files, 16,047 nodes, and 53,589
  edges. Version 1.6 is available, but the indexed 1.5 service remains in use
  for this plan.
- 2026-08-29: Plan 0098 supplies the already-verified loopback-only relay
  invariants; its Tailscale and Bastion non-goals remain separate from the
  productization slice.
- 2026-08-29: the installed bilateral controller exposes immutable begin,
  per-lobe collection, assembly, Codex approval, ordered Pro submission, and
  single-transit state gates.
- 2026-08-29: the generic `electron relay install|doctor|run|uninstall`
  lifecycle passed its isolated fixture, including drift refusal, exact process
  and listener ownership, loopback binding, and persistent-CDP HTTP response
  handling.
- 2026-08-29: handoff-safe local publication completed with eight retained
  sessions resumed and a matching installed/runtime-manifest executable hash.
  The exact Workshop URL and retained Termius targets remained unchanged.
- 2026-08-29: the managed Termius relay replaced the disposable units, passed
  doctor, and survived a service stop/start with the same three target IDs:
  `22546E9696501A4628E527A075FCD94F`,
  `441BB575EAF2BF72073DC180901ADE21`, and
  `7624870B4329A4585A9AFF031B897B3C`.
- 2026-08-29: the obsolete tailnet TCP `2223` listener was removed while TCP
  `22 -> 127.0.0.1:2223` and every HTTPS/web handler remained unchanged. The
  post-change configuration is retained at
  `/tmp/agent-browser-tailscale-serve-after-p126.json`; the exact rollback is
  `tailscale serve --tcp=2223 127.0.0.1:2223`.
- 2026-08-29: the legacy wrapper and units were disabled and moved to
  `/home/bak3r/.agent-browser/quarantine/p126-legacy-termius-relay-20260829`.
  They were not deleted.
- 2026-08-29: the focused Rust relay tests, lifecycle and widened runtime
  fixtures, JavaScript lint, service-client typecheck, docs and dashboard
  production builds, production-target strict Clippy, direct plan audit,
  CodeGraph sync, repository/installed skill parity, and diff hygiene pass.
  All-target Clippy remains blocked by 12 pre-existing test-only warnings
  outside the P126 slice; no unrelated test cleanup was taken.
- 2026-08-29: the required bilateral DOCX/PDF submission failed closed before
  Send with `agent-browser retained service browser
  session:auracall-chatgpt-bridge-v3 is not live`. Read-only CDP and service
  inventory immediately proved the retained browser healthy at port 38605,
  exact target `7FE266BDF4EBD22B5709975451FBAE24`, and the pinned Workshop
  URL. No replacement browser or document prompt was allowed; artifact release
  remains blocked on exact-handle broker reacquisition.

## Closeout

Implementation, publication, managed migration, service recovery, and the
Tailscale cleanup audit are complete. The normal Termius application
close/reopen and coordinated reboot drill remain intentionally gated because
they terminate active operator SSH terminals. Run them only after the operator
confirms those sessions may be interrupted; until then acceptance criterion 8
and final closeout remain open. Bilateral DOCX/PDF release is separately
blocked by the fail-closed retained-browser liveness mismatch recorded above.
