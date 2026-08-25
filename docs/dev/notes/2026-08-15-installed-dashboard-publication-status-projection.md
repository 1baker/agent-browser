# Installed Dashboard Publication Status Projection

Date: 2026-08-15
Plan: P121

## Finding

P120 separated read-only journal status from recovery authorization, but that
authority boundary lived in a repository Node command. An installed operator
or dashboard client could not see an interrupted publication without shell
access to the checkout. Install doctor could also report runtime drift without
showing the durable transaction that explained it.

## Resolution

The installed Rust runtime now reads the same journal, exact PID lock, bounded
transaction metadata, and SHA-256-bound installed artifact. It validates the
journal schema, bounds journal and artifact sizes, rejects non-regular files,
and projects only handoff counts plus bounded failure classification.

The result is available through install doctor, authenticated dashboard/service
HTTP, an MCP resource, the generated observability client, and the Service
dashboard. The dashboard displays status and the exact reviewed command when
recovery is safe. It exposes no recovery POST, MCP tool, or button. Reading
status therefore cannot become authority to build, install, restart, or
recover.

## Verification Status

The projection is implemented and installed. Focused and widened tests,
typecheck, builds, contracts, generated clients, isolated no-launch reads, and
publication fixtures pass. Repository status, install doctor, authenticated
HTTP, and MCP agree on terminal phase `ready`, a verified replacement, no lock,
and no recovery action. The installed and release binaries have SHA-256
`2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

P121 remains open because retained-browser verification failed. The prior
Workshop Chrome PID and CDP endpoint are gone, and the current service state has
no matching browser, session, or target row. No replacement browser was started
and no prompt, navigation, typing, or click occurred. Closure requires an
isolated diagnosis followed by exact retained-target readback without a prompt.

## Retention Guard Follow-up

The browser exit predates the publication transaction by about eight minutes,
so the publisher did not terminate it. The remaining safety gap was that an
expected retained lane could disappear first and a later publication could
still succeed by classifying its surviving daemon as idle.

Publication now accepts exact retained session, profile, target, URL, and CDP
expectations. It checks live CDP inventory before backup or quiescence, pins the
observed PID and endpoint, and rechecks the same identity after handoff, at final
readiness, and during recovery. A separate read-only preflight performs that
check without a lock, build, daemon launch, or browser launch. Local full Rust
validation now runs through an isolated serial wrapper instead of sharing the
operator home.

The exact Workshop preflight currently fails `retained_daemon_missing` without
changing service state, journal bytes, session inventory, or lock state. The
release candidate is built but intentionally not installed until a retained
lane exists and can satisfy the new guard.

## Durable Critical-Lane Follow-up

Exact flags are no longer the only authority path. A verified live lane can be
pinned into a private, bounded, mode-0600 requirement containing only stable
session, profile, target, and canonical URL identity. Normal publication loads
that requirement automatically and verifies it before build. Explicit flags
may strengthen the requirement but cannot conflict with it.

The read-only check treats an absent requirement as `not_configured` and fails
closed on invalid permissions, symlinks, schema drift, unsupported fields, or
live identity drift. The source-checkout user-service interlock now runs that
check before convergence mutation, providing a recurring watchdog without
launching or touching a browser. The currently missing Workshop lane prevents
the requirement from being safely created and keeps P121 acceptance criterion
8 open.

The widened slice passes all 1,861 non-ignored Rust tests with 57 ignored,
strict Clippy, formatting, client typecheck, dashboard and documentation
production builds, workflow and schema parsing, skill parity, and diff hygiene.
The release candidate SHA-256 is
`68303e1b9c89aa7e8a1f81cf7e646ac0f570208cf5c2559635cb186f95f40381`;
the intentionally retained installed binary remains
`2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.

## Native Source-Free Interlock Follow-up

The source-checkout preflight did not protect the binary-owned workstation
timer because that timer invokes `agent-browser install workstation reconcile`
without Node or pnpm. Native apply and reconcile now read the same durable
requirement without launching a daemon or browser. They require the exact ready
browser, active session, valid service-tab handle, profile, live PID, loopback
DevTools target, and canonical URL. Apply checks before its lock, sudo, payload
staging, or service quiescence; reconcile checks again immediately before its
first mutation so identity drift cannot pass on an earlier observation.

Nine focused Rust regressions and the isolated source-free installer fixture
prove exact success and fail-closed handling for changed or ambiguous identity,
unsafe files, and invalid configured state before mutation. The widened result
is 1,870 passing non-ignored Rust tests with 57 ignored, plus strict Clippy,
formatting, workstation fixtures, service-client checks, dashboard and docs
builds, installed skill parity, and diff hygiene.

The release candidate SHA-256 is
`b8eaac13578f3380aca701e4ea127d2653b4d2a22fdbb73a1172e8738fc474dc`.
It remains uninstalled because the durable requirement is absent. The installed
binary remains
`2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`.
No browser action or GitHub write occurred.

## Native Status and Configuration-Parity Follow-up

The installed controller can now run `agent-browser install workstation
retained-browser-status --json` before any apply or reconcile attempt. The
command uses the same verifier but acquires no workstation lock, invokes no
daemon, launches no browser, and creates no state. Its result is identity
redacted.

Native status, apply, and reconcile now honor
`AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT`. The normal agent-browser dotenv
loader admits that key from `AGENT_BROWSER_ENV_FILE` or
`~/.agent-browser/.env`, closing the prior mismatch where repository tooling
could use an override that the source-free installed watchdog ignored.

The isolated fixture covers default, direct environment, and dotenv-loaded
paths and proves absent status does not create the configured parent. All 1,871
non-ignored Rust tests pass with 57 ignored, plus strict Clippy, formatting,
source-free workstation, dashboard, docs, skill-parity, and diff gates.

The optimized command was run safely against the real configuration. It
reported `not_configured`; service-state SHA-256
`b19cfc6746ff921ffd54358749cf660af2723b222e02a12e220a3721c2eb0a3e` and
publication-journal SHA-256
`a3efa8ef2e6a0e3bb1aec6247a8328fdfb9793b65b5bbdad87c5118c45a79fa7`
were unchanged before and after. The release candidate SHA-256 is
`ce5a783994a8744de27e6f59c0be145ee64de78a4379248c50868b3dcddd7835`;
the installed binary remains the prior build. No browser or GitHub action
occurred.

## Marker-First Enforcement Follow-up

The durable requirement previously remained vulnerable to a single-file loss:
deleting it made every reader report `not_configured`. Pinning now commits a
separate private `.required` enforcement record first. If the process crashes
before the identity requirement is committed, retry is idempotent; until then,
repository publication and native status/apply/reconcile fail closed on the
missing requirement.

Both implementations bound the marker read, require an owner-only regular
non-symlink file, validate the v1 schema and timestamp, and project no retained
identity. Focused tests cover crash-after-marker, retry, requirement deletion,
permissions, symlinks, and pre-mutation watchdog behavior. The full isolated
Rust result is 1,873 passed with 57 ignored. Production-binary strict Clippy,
formatting, publisher fixtures, source-free workstation coverage, dashboard
and docs builds, schema parsing, skill parity, and diff hygiene pass. Full
all-target strict Clippy is separately blocked by 12 warnings in unrelated
existing test code.

The live optimized no-launch status remains `not_configured` because neither
authority file exists. Service-state SHA-256
`fa8067d2756dba729fadce2fce174290fd5b06c749cc84b8569dfd890f664ecc`
and publication-journal SHA-256
`a3efa8ef2e6a0e3bb1aec6247a8328fdfb9793b65b5bbdad87c5118c45a79fa7`
were unchanged. The release candidate SHA-256 is
`ab15748e93330c2c4d61d1bc6b0ff2de18b86be3852869a2f83b657424b13165`;
the installed binary remains unchanged. No browser or GitHub action occurred.

This interlock detects loss of only the requirement. Deliberately removing both
private authority files remains an administrative removal of enforcement.

## Digest-Bound Enrollment Follow-up

Marker-first ordering prevented a single missing requirement from disabling
enforcement, but the two valid JSON files were not bound together. A stale
marker could be paired with different valid requirement bytes. The enforcement
record now precommits `requirementSha256`, the lowercase SHA-256 of the exact
requirement serialization allowed to appear.

The writer calculates the final bytes before the first commit. On a crash, it
reuses the marker timestamp and digest: identical verified evidence completes
the original enrollment, while changed evidence conflicts and leaves the
requirement absent. Node and native readers compare the digest before accessing
service state. A legacy markerless requirement stays readable and is bound when
the next idempotent pin retrofits enforcement. No prior marker-only candidate
was installed, and live state contains neither authority file.

Focused Node tests prove crash retry, changed-evidence refusal, byte replacement
refusal, and watchdog behavior. Thirteen native tests include digest mismatch
before service-state access, and the source-free installer fixture passes
against the rebuilt release candidate. All 1,874 non-ignored Rust tests pass
with 57 ignored; formatting, production-binary strict Clippy, publisher
fixtures, dashboard and docs builds, schema parsing, skill parity, and diff
hygiene pass.

The optimized live no-launch status remains `not_configured`. Service-state
SHA-256 `c7b92949339e831e65e6f317bace89dc0796bb0065a93a743d095e78d4353c7a`
and publication-journal SHA-256
`a3efa8ef2e6a0e3bb1aec6247a8328fdfb9793b65b5bbdad87c5118c45a79fa7`
were unchanged. The release candidate SHA-256 is
`f129a45f9e7b70506808723637432038c6b9e94ccf9327ab24cb8c858f6b88b3`;
the installed binary remains unchanged. No browser or GitHub action occurred.

## Workstation Remote-View Readiness Follow-up

After the required WSL relogin, the source-free controller finished installing
the Guacamole stack and canonical route users. Its first route-display repair
exposed an executable-selection mismatch: a fresh viewer daemon inherited the
workstation's Windows StealthCDP default, which cannot render on the Linux XRDP
display. Reconciliation now resolves the installed Linux Chrome after its
browser-install step and passes that exact executable to both disposable route
viewer daemons.

The rendered live gate also had a deterministic fixture deadlock. It created
the HTTP fixture in the main Node thread and then called `spawnSync`, preventing
that same thread from serving Chrome's navigation. The fixture now runs in a
worker, and a focused regression proves a synchronous child can fetch it while
the parent is blocked. Exact CDP URL and title checks retain full fixture
identity; OCR uses the shorter unique `REMOTE VIEW OPEN` prefix because
Tesseract can render the final word as `FIXT!` in the narrow XRDP tab strip.

Install doctor and remote-view doctor now report ready with no issues, routes A
and B use distinct displays `:10` and `:11`, and the dashboard plus both
maintenance timers are active. The optimized fixture smoke passed with exact
URL/title, X11 PID, target deduplication, route-bound visible-window proof, OCR,
and cleanup evidence. Twenty workstation Rust tests, the source-free fixture,
the worker-liveness regression, syntax checks, formatting, strict Clippy, and
diff hygiene pass.

The release candidate SHA-256 is
`a886febac4a3794bd0b20387e603406b319e9419c2047cd2b1b15cd404450eec`.
The installed binary remains
`2e855070b644a28370a07ec0b4a45d26ad277d25a19b92452989c558832646c1`
until the exact Workshop lane can be established, pinned, and preserved through
the guarded install. Only disposable local fixture browsers were used; no
ChatGPT prompt or GitHub write occurred.

## Exact Retained-Target Discovery Follow-up

The original enrollment command required an operator or agent to copy the
session, profile, target, and canonical URL independently. The publication
tooling now accepts one reviewed origin and path prefix for requirement writes,
enumerates current live daemon lanes, reads each ready lane's CDP inventory, and
derives the exact stable identity only when one page matches.

Discovery is read-only until the existing marker-first requirement commit. It
fails closed when no page matches, multiple pages match, a live session or CDP
inventory cannot be read, the service browser does not match its session, the
browser is not ready, PID or profile identity is incomplete, or the candidate
origin and path are outside the reviewed prefix. The stored authority retains
the exact discovered URL, not the prefix. The repository Node guard now also
requires `health=ready`, matching the native verifier.

Current no-launch service state has no live browser, active session, or ChatGPT
tab, so discovery correctly remains a no-match and no authority file was
created. The operator still needs to establish the intended Workshop page, but
does not need to transfer its internal target identity.

During validation, live artifact reconciliation found a separate terminal
publisher transaction at 13:58. It installed the then-current release artifact
without a retained-browser requirement because no retained lane existed. The
installed SHA-256 is now
`758c9f4d4e89941799dbe053357e50597e13b633e3e6013f8a5e7259d39b2984`.
The rebuilt source candidate, including unique-prefix discovery and loopback
DevTools enforcement, has SHA-256
`0e2186376c0795d6df684797f779ce94400f99fb8d582c68b03741918f306acf`
and remains uninstalled pending exact Workshop enrollment.

Widened convergence validation initially found that its temporary-home fixture
still inherited the operator's `XDG_RUNTIME_DIR`. It consequently classified a
live unrelated `etf-paper-dashboard` token as fixture state and attempted to
close it through the fake binary. The fixture now sets its own explicit
`AGENT_BROWSER_SOCKET_DIR`; the rerun passes and cannot inspect or act on live
session metadata.
