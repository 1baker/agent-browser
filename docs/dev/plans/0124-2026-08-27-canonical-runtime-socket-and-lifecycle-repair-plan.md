# Plan 0124 | Canonical Runtime Socket And Lifecycle Repair

State: CLOSED
Roadmap: P124
Plan version: 1
Date: 2026-08-27

## Objective

Make interactive, noninteractive, and user-service Agent Browser clients resolve
the same daemon namespace, preserve service-owned browser ownership during
reattachment and executable migration, and remove false one-time profile
warnings for the exact profile recommended by the access plan.

## Frozen Acceptance Criteria

1. On Linux, an invocation without `XDG_RUNTIME_DIR` uses the current user's
   secure `/run/user/<uid>/agent-browser` directory before the home fallback.
2. Explicit `AGENT_BROWSER_SOCKET_DIR` and `XDG_RUNTIME_DIR` remain
   authoritative, and an unsafe or unavailable inferred runtime directory
   fails back without trusting another user's path.
3. Rust CLI, local runtime convergence, and guarded dashboard publication use
   the same socket-directory resolution contract.
4. A command naming an existing service-owned session reaches its owning daemon
   instead of launching a duplicate daemon or attaching to raw CDP as an
   external browser.
5. The exact deterministic `managed-one-time-*` profile recommended by an
   access plan is registered as `managed_one_time`, reused on retry, and does
   not produce an arbitrary-profile warning.
6. Truly unknown caller-supplied profiles continue to warn and fail closed
   where the existing access-plan and launch contracts require it.
7. Browser close remains routed through `service_browser_close`, which must
   retain its existing polite-close, bounded force-kill, health evidence, and
   process-exit verification behavior.
8. Live convergence uses authenticated runtime handoff and preserves retained
   browser PIDs, targets, profiles, and URLs. It sends no prompt and performs no
   page interaction.
9. A daemon whose control-plane shutdown acknowledgement is stranded after a
   successful handoff prepare exits within the publisher's bounded wait, while
   the durable browser descriptor remains available to the replacement daemon.
10. Migration from an already-installed daemon with the old unbounded shutdown
    verifies the exact durable descriptor and live browser PID before retiring
    only that relinquished daemon, then requires normal exact-PID/CDP resume.
11. A Linux daemon still executing a deleted pre-publication inode uses the
    verified current or rollback binary as its command client instead of trying
    to spawn another daemon from the stale `/proc/<pid>/exe` identity.
12. Publisher admission attempts authenticated `handoff prepare` before using
    service-record absence to classify an old daemon as idle; compatibility
    `close` is allowed only when handoff is unsupported and readback proves no
    active browser.
13. Recovery recognizes an already-resumed broker browser through exactly one
    persisted CDP match plus the still-live prepared browser PID even when its
    attached-existing service record omits PID or uses a bridge alias. PID,
    endpoint, or multiplicity drift still fails closed.

## Non-goals

- automating Google, Gmail, or ChatGPT authentication;
- changing retained conversation URLs or selecting a new ChatGPT target;
- broad retained-record pruning;
- changing public ingress or GitHub state;
- replacing route A and route B topology during this repair.

## Execution Graph

1. Add red regressions for Linux runtime-directory inference and exact
   recommended one-time profile registration.
2. Implement one canonical socket-directory resolver in Rust and reuse the
   same decision in local runtime scripts.
3. Repair the managed one-time profile admission path without weakening
   unknown-profile warnings.
4. Run focused tests, widened service tests, Rust format and Clippy, build,
   documentation checks, plan audit, CodeGraph sync, and diff hygiene.
5. Install the verified candidate, hand stale daemons to the current executable
   without closing browsers, and re-run no-launch doctors plus exact retained
   target checks.

## Delegation Receipt

- State: `not_spawned`
- Reason: the runtime resolver, daemon ownership, close semantics, and profile
  admission changes share one critical Rust and lifecycle-script write surface;
  parallel edits would increase overlap and reconciliation risk.
- Runtime handle: none

## Closeout

Guarded publication and explicit recovery reached terminal `recovered_ready`.
The installed executable, live dashboard manifest, and workspace debug binary
agree at SHA
`96e141a23e3da440d7757e0ff994e6eca0caf672f8b22df3a23c42cd675b4c42`.
The source-free workstation payload is installed and ready. Final compact
install and remote-view doctors report zero issues, remote control ready,
many-to-many ready, five converged runtimes, and zero stale runtimes.

AuraCall browser PID 3246087 and NYSE browser PID 1762940 retained their exact
CDP endpoints through handoff and recovery. Route browser PIDs 1711948 and
1750635 plus default browser PID 2376036 also remain live. The retained Workshop
target `B3128B9AAE6E22E34771378075CD4517` remains on its original canonical URL.
No handoff descriptor remains, the publication journal recommends no action,
and no ChatGPT prompt or page interaction was sent.

## Evidence Log

- 2026-08-27: live diagnosis found service sockets under
  `/run/user/1000/agent-browser` while the noninteractive shell lacked
  `XDG_RUNTIME_DIR` and therefore selected `~/.agent-browser`.
- 2026-08-27: the split namespace reproduced duplicate daemon startup, failed
  automatic service-session reacquisition, false stale-executable findings, and
  a raw-CDP close that released records without owning the Chrome process.
- 2026-08-27: red Rust coverage proved the no-environment client selected the
  home namespace instead of `/run/user/1000`; red native-action coverage proved
  the exact deterministic managed one-time profile was left unregistered.
- 2026-08-27: the canonical Rust and Node resolvers now preserve explicit
  overrides, accept only a secure current-user Linux runtime directory, and
  align convergence, dashboard publication, retained-browser checks, and
  executable-handoff smoke discovery.
- 2026-08-27: focused runtime-directory and remote-view tests pass, including
  exact recommended profile registration and the existing unknown-profile
  warning regression.
- 2026-08-27: first guarded publication failed closed and rolled back before
  executable replacement because `auracall-chatgpt-broker-v7` completed handoff
  prepare but its daemon waited indefinitely for control-plane shutdown. The
  durable descriptor and retained browser remained available. Daemon shutdown
  is now bounded below the publisher timeout, and daemon startup uses the same
  canonical Rust socket resolver as clients.
- 2026-08-27: a fresh old-daemon retry reproduced the same stranded exit,
  proving candidate-only shutdown changes cannot migrate the installed daemon.
  The publisher compatibility path now verifies session, descriptor schema,
  CDP URL, daemon PID, and retained browser PID before bounded TERM/KILL of only
  the relinquished daemon; ordinary unprepared sessions cannot enter this path.
- 2026-08-27: the next retry failed closed before prepare because the daemon
  client inherited a deleted `/proc/<pid>/exe` identity from the earlier
  rollback. Shared publisher and retained-browser discovery now reject deleted
  proc targets and use the already verified fallback binary.
- 2026-08-27: the final guarded retry failed closed because the broker daemon's
  live browser is retained under a bridge-session service record, so exact-name
  service readback misclassified the broker daemon as idle. Publisher ordering
  now uses handoff prepare as the authoritative browser-presence test and keeps
  service-record-based close only for older unsupported idle daemons. No further
  live publication retry was made in this turn.
- 2026-08-27: installed payload reconciliation triggered a guarded interlock
  publication whose recovery found AuraCall already resumed under its bridge
  service record with a null attached-existing PID. Alias-aware recovery now
  accepts exactly one matching CDP record only while the prepared PID remains
  live, and retains strict mismatch and ambiguity failures.
