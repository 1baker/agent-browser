# Cold restart rollback core, staged

## Status and authority

The operator approved a backup-protected browser restart and independent local
security review for the browser bootstrap repair only. The later request,
`ok go, when is agent browser fixed?`, authorized adding tested installer
rollback. This is not permission to discard browser sessions, bypass retention,
invent custody receipts, or perform publisher account/licensing actions.

The live repair is **not complete**. This slice stages the rollback engine and
supporting primitives. It exposes no cold-restart CLI flag and supplies no live
runtime adapter. Programmatic opt-in without that adapter fails before staging,
maintenance changes, or shutdown. Existing normal publication remains available
through its unchanged command surface.

## Implemented boundaries

- `scripts/lib/local-dashboard-cold-restart.js` implements write-ahead phases,
  positive proof requirements, explicit recovery, candidate re-fencing before
  rollback, and fail-closed handling of ambiguous close/launch outcomes. Recovery
  never retries a candidate launch. An admitted source launch must be reused by
  exact intent or refused by the adapter, never duplicated.
- `scripts/lib/local-dashboard-cold-publication.js` integrates with the existing
  publication journal lock and maintenance custody, digest-bound prebuilt
  staging, binary backup, workstation provenance, paired replacement/restoration,
  dashboard readiness and final doctor. The normal orchestrator routes only an
  explicit programmatic cold request or an existing nonterminal cold journal.
- `scripts/lib/local-dashboard-cold-profile.js` captures private cold snapshots,
  verifies file bytes/names/modes/symlink text, rejects unsafe path ancestry, and
  restores through a receipted move while retaining displaced profiles. It does
  not establish profile idleness itself; the caller must hold admission control
  and provide that proof.
- `scripts/lib/local-dashboard-cold-process.py` binds Linux processes to boot,
  start time, effective UID and executable inode/digest. Browser evidence also
  binds canonical profile identity, exact argv, DevToolsActivePort and an owned
  kernel listening socket. Shutdown is pidfd-bound SIGTERM only, with a bounded
  wait and no escalation or PID fallback. Observations alone do not authorize
  shutdown. No stop CLI is exposed.

No retained requirement is removed or weakened. Historical service state,
leases, PID files and custody receipts are not restored as current authority.
Unknown process occupancy or failed identity verification must preserve state
and leave recovery nonterminal.

## Verification scope

Main independently ran:

- `node scripts/test-local-dashboard-cold-profile.js`: 15 isolated filesystem
  fixtures, including tampering, interrupted move and concurrent drift.
- `node scripts/test-local-dashboard-cold-restart.js`: in-memory forward,
  rollback, malformed-admission, missing-proof, lost-response and revision-conflict
  fixtures. These simulate runtime state; they do not prove actual custody.
- `node scripts/test-local-dashboard-cold-publication.js`: twelve on-disk
  journal/profile/binary-and-manifest fixtures. Runtime behavior is simulated.
  Failures after binary replacement, manifest replacement, lost launch response,
  qualification and dashboard readiness restore the matching source pair/profile.
  Missing maintenance custody and recovery-only requests without an active cold
  transaction are refused before runtime effects. Unchanged-source rollback
  preserves the canonical retained verification in the real journal.
- `python3 -B scripts/test-local-dashboard-cold-process.py`: 13 fixtures. Actual
  pidfd tests use only owned disposable subprocesses; browser observations use
  mocked procfs snapshots. No operator browser was signalled.
- `pnpm test:local-dashboard-publisher-orchestration`: existing publisher fixture
  passed after adding the cold branch.

These results supplement, not replace, the earlier recovery-candidate Rust and
real-Chrome handoff tests. No cold restart of a real browser, installed rollback,
authenticated session restoration or complete live attestation is proved here.

## Remaining integration and release gate

Implement and independently review a concrete runtime adapter before exposing a
CLI operation. It must bind immutable restart intent to exact current lane and
process evidence, fence new jobs/viewers/launches, retain a safe inverse for
pre-close admission failure, preserve the original profile and conversation,
prove stopped processes and released profile locks, and relaunch or adopt only
the exact transaction-owned generation. Qualification must use real service
handles and ownership evidence. Pin rotation must use the existing verified,
digest-bound rotation helper after the prior authority is genuinely absent.

Prove forward success and failure rollback with an isolated real Chrome lane,
then rerun independent review and live preflight. Only afterward may the approved
live restart proceed. Final acceptance requires installed binary/manifest
agreement, original-session restoration, real ownership attestation and input
verification. There is no reliable completion ETA yet.

## Delegation receipt

`bootstrap_transition` implemented the two cold-profile files and then the two
exact-process Python files under disjoint write scopes. `bootstrap_security`
implemented protocol fault fixtures and independent integration review. Both
returned terminal results. Main inspected source, reran tests and owns final
disposition. No delegated worker received live browser, secret or installation
mutation authority.

The integration reviewer identified three accepted blockers: optional maintenance
custody, recovery-only falling through into fresh publication, and missing
canonical retained final proof after unchanged-source rollback. Main corrected
all three and added regression fixtures. Independent closed-world re-review found
no remaining findings in that scope. Lint, diff checks and the existing publisher
orchestration fixture passed. CodeGraph was synced after the source edits.

Verified source SHA-256 receipts:

- cold-publication.js: `c356605c691de4b7784d006044ff1719ee95f5d0af7586937cb70a574b0b2484`
- cold-restart.js: `ed8780d38e8f598a75e94be296c05fafd49bcc6e9cc73ccce38d015f9ccb0d44`
- cold-profile.js: `82755fbae64700f5dd91a89831696137e17683d1a3cb0e17ff7725533592e935`
- cold-process.py: `723efed2c9d149aabb5a58846b01a700ba0548355a86758772e0d103ded4218d`

The installed executable was rehashed and remains
`90e5dffdbc360f8fe3f1dfef34ffe0f4c5370ac4f11b1a7d602c68d075529272`.
No live browser, installation record, profile or retention pin was modified.

## Real-browser compatibility and admission slice

The next continuation added `test-local-dashboard-cold-browser-live.js` and a
small opt-in installed-command override to the existing MCP smoke helper. Main
ran the isolated real-Chrome test twice successfully after fixing its handle
placement and cleanup lint. The installed old binary handed one disposable
browser and its exact tab to the candidate, which handed the same browser and
tab back to the old binary. Both actual descriptors were schema 1. Candidate
diagnostics correctly did not claim complete custody for this inherited legacy
browser. This proves warm compatibility, not a cold restart or installed rollback.

The process helper now exposes `require_profile_idle`: current effective-UID
process inventory, exact profile arguments, and conservative Chrome singleton
lock/socket inspection. Ambiguous occupancy fails closed; no locks are removed.
Main independently ran all 21 process fixtures. This is an instantaneous proof
and requires a separate admission fence around any subsequent profile mutation.

Recovery source now includes `native/publication_admission.rs`, registered as a
module but deliberately not connected to runtime entry points yet. Its private,
fixed-home receipt checks owner kernel identity, exact session and capability
digest, rejects unsafe files/ancestry, and never confers browser custody. Main
ran eight isolated unit tests successfully. Independent review identified an
absent-path symlink bypass; main fixed it, added regression coverage, and the
reviewer accepted closure. No live admission receipt was created.

Remaining integration is substantive: enforce admission before startup and
daemon creation, at ingress and queued execution, and across private/display and
background producers; prevent capability persistence; prove idle/drained state.
Then implement and test the concrete cold adapter. Old executables do not obey
the new fence. A warm bridge alone does not solve rollback from a fresh schema-2
candidate to the original schema-1 binary: source/candidate/bridge artifacts and
their paired installation records need explicit verified transaction roles.
No production restart or deployment is authorized by a simulated fence proof.

The safe proposed close order is authenticated handoff, actual source-daemon
identity exit, revalidation of held browser pidfd/profile/listener evidence,
then SIGTERM of that browser alone and positive profile-idle proof. Do not use a
STOP/browser-close/CONT sequence: queued daemon commands can race or relaunch.
The installed executable digest was rechecked unchanged. A fresh no-launch
access plan still recommends default/chatgpt-pro reuse; that is not proof of
current authentication or complete control-plane attestation.
