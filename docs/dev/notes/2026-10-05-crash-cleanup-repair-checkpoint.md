# Owned-browser crash cleanup repair, 2026-10-05

Status: **source repair verified; live installation blocked**. This checkpoint
does not replace the installed runtime or certify retained-provider workflows.
No retained browser, profile, tab, shared service, or provider message was
changed by this repair.

## Source and build identity

- Repair commit: `c31f78ac56a30d2e1873eed69071d73b3ff5a696`.
- Parent checkpoint: `e4cdbb99953712510a4ee63c797deab41680b3fa`.
- Frozen `cli/src/native/control_plane.rs` SHA-256:
  `b2a5e70bcd6fdfd4e4533a701969cac66da145955a11d23fe13a7ededf9bbe9d`.
- Frozen `cli/src/native/e2e_tests.rs` SHA-256:
  `8c34835ffcfc8ff9d1f76e68849d0b02ef4d29f5d0452df79c7ebb631fee12e0`.
- External debug binary SHA-256:
  `71aa6d283fa61b6ee812364e81b193ba9609d48569a365de25fd092f7df2d62a`.
  It was built for isolated tests, not installed or designated a release candidate.

The historical [full E2E checkpoint](2026-10-05-full-e2e-install-checkpoint.md)
is preserved, with an explicit correction: its saved disposable crash fixture
contained two launches, a ready replacement, and no process-exit event. The
earlier claim that the crash event had already been reached was incorrect.

## Diagnosis and bounded repair

Both health consumers checked child exit before awaiting a CDP liveness probe.
Chrome could exit during that wait. A failed transport then took the immediate
relaunch path, discarding the child-exit evidence before crash persistence and
operational cleanup.

The consumers now poll the owned child after the probe. Observed process exit
takes precedence even when the transport result was successful. A failed probe
receives at most 100 ms of exit-observation grace for an owned child, with a
final poll at the deadline. The existing process-exit persistence removes
terminated browser, session, and tab records while retaining the crash event.
It does not eagerly launch a replacement. A later browser command can recover
under the existing policy. External attached browsers acquire no lifecycle
authority, and no provider request is replayed.

Three unit tests check the health truth table. One Linux temporal test covers
both actual production consumers across failed-probe, successful-probe, and
delayed-exit cases: six combinations. Its loopback peer waits for the real
probe before terminating only its captured fixture-owned Chrome PID. It uses
non-reaping `waitid` with `WNOWAIT` for precise waitability evidence rather than
equating a zombie process state with a reapable child. Assertions require exact
PID and signal evidence, durable crash recording, removed operational records,
and no replacement launch. The original crash-and-recovery E2E assertions remain
intact; the fixture additionally checks that its exact owned kill succeeded.

## Final validation

The accepted frozen-source runs completed on 2026-10-05. Tests used disposable
home, profile, runtime, PID, mount, IPC, and network namespaces; host loopback
and the WSL gateway were verified unreachable before each sandbox command.
Only a curated Chrome executable cache was exposed read-only to browser tests.
Production profiles, credentials, runtime sockets, and signed-in chats were not
available to the fixtures. Build output remained in a separate task cache.

| Check | Accepted result |
| --- | --- |
| Repository Rust partition runner, optimized CI profile | 2,199 passed, zero failed |
| Serial ignored native E2E suite, optimized CI profile | Harness reports 59 passed, zero failed, 48.41 seconds |
| Actual browser coverage within that E2E result | 55 executed native Chrome tests plus one six-case temporal test; three optional-engine tests returned early and are not certified |
| Original crash detection, cleanup, and next-command recovery | Passed without relaxing assertions |
| Required Rust format and default-target clippy with warnings denied | Passed |
| Root JavaScript lint, service API/MCP parity, docs production build | Passed |
| Actual CI no-launch scripts | All ten passed against the rebuilt debug binary |
| Source-free workstation installer fixture | Passed against the rebuilt debug binary; not a live installation |
| VM harness, Guacamole assets, PostgreSQL durability, route-specific user sync | All four fixture groups passed |
| Diff whitespace check | Passed |

JavaScript, parity, and docs checks ran during this slice; the final frozen
changes thereafter were limited to Rust race evidence, fixture waitability,
and Rust formatting. Rust validation, no-launch checks, and installer fixtures
used the final frozen Rust source. No custom stealth engine, Lightpanda,
guarded LitScout interoperability, host GUI, or retained-provider Send is
certified by these results.

Earlier failed attempts remain in the private QA receipts. They include the
baseline crash failure, an insufficient immediate second poll, a zombie-state
fixture gate that did not prove waitability, and an intermediate optimized
run with two failures: that temporal fixture and the cursor-many-elements
snapshot test. An unoptimized full run aborted with stack overflow in the
stale-reference click test after its temporal test had passed. The final
optimized run passed both click/snapshot cases; this is not a claim that the
debug-stack issue was fixed. Two superseded task-owned namespace runs were
cancelled while freezing source or correcting a missing browser mount, not
reported as test passes. No non-task process was signalled.

Previously recorded broader all-targets clippy findings, the extra non-CI
service-status smoke failure, and restricted host-provision AppArmor package
availability remain separate limitations. They were not fixed or waived by
this slice. The validation selector also names an installed skill path from
another workstation that does not exist here; no shared skill was overwritten.

## Review and publication gates

A fresh independent Codex source reviewer accepted CRASH-1 and CRASH-2 after
inspecting the exact frozen source, existing recovery guards, test ownership,
and non-reaping evidence. The owner inspected the full patch and final test
receipts before accepting the repair. The reviewer did not run the tests or
provide live installation proof.

The required stateful Claude plan exchange halted at its tool-turn ceiling;
the bounded replan then hit a provider session limit. No limit was bypassed,
no peer editing baton was granted, and no Claude plan or final acceptance is
claimed. CodeGraph is not initialized in the successor checkout; direct source
search and diff review supplied the evidence instead. Graphiti retrieval was
advisory, not runtime authority.

Every dirty source path was reviewed. Full staged blobs are scanned for
high-confidence credential patterns before each commit. The repair scan found
only the unchanged synthetic proxy help example, verified against the parent;
there were no unresolved findings. Runtime logs, profiles, key files, generated
test state, private process identities, and binary artifacts are excluded from
publication. Publication is to personal `1baker`, not an upstream release.

## Installation blocker: W1

Retained legacy clients lack trusted source-to-artifact proof of their complete
state-write and cold-daemon-launch behavior. Static artifact review supports
reader/dispatcher roles for inspected MCP paths but cannot certify all indirect
paths. An old digest alone is not proof of an incompatible writer. The current
runtime writers also require one guarded migration so older read-modify-write
generations cannot discard the new inventory observation revision.

No supported exact-client MCP replacement route was established without
interrupting shared managed owners. Installation therefore remains blocked,
despite the now-green native crash gate. Preserve those owners until either
trusted non-writer compatibility proof or a supported owner-coordinated idle
client migration is available. Then build an exact-source candidate with its
embedded assets, obtain fresh complete writer/custody/quiescence and rollback
proof, and use the guarded publisher. Installed diagnostics, negative custody
controls, and refresh/inventory parity must pass after publication before live
consumer acceptance. Source publication is not that acceptance.
