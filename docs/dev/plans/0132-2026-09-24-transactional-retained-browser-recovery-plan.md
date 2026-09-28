# Plan 0132: Transactional Retained Browser Recovery

Date: 2026-09-24
State: CLOSED — source-validated; installed retained runtime unchanged
Lane: P132
Depends On:
- `docs/dev/plans/0096-2026-08-07-durable-remote-view-handoff-plan.md`
- `docs/dev/plans/0125-2026-08-27-runtime-handoff-browser-identity-regression-plan.md`

## Immutable Intake

The operator accepted the recommendation to make Agent Browser recovery less
clunky by adding a transactional recover-and-take-control path for an existing
retained browser. The initiating prompt was `ok go`, referring to that accepted
recommendation and the observed durable-handoff failure where Chrome remained
live after its daemon/session route disappeared.

## Intake Task Graph

1. Prove the exact durable handoff, browser, session, profile, PID, DevTools
   endpoint, build proof, and target intent before recovery.
2. Start or restore only the exact daemon lane and attach it to the retained
   browser without launching Chrome, opening a replacement tab, or changing the
   profile.
3. Dispatch durable-handoff resolution only after owner preparation succeeds.
4. Fail closed on missing, changed, duplicated, or ambiguous evidence with a
   typed recovery cause and operator recourse.
5. Expose one clear dashboard recovery action and keep the durable handoff URL.
6. Verify source behavior, no-launch invariants, dashboard behavior, contracts,
   documentation, and relevant Rust quality gates without mutating the installed
   production runtime.

## Acceptance Criteria

- A stopped daemon plus one exact retained browser prepares that same daemon and
  reuses the same browser PID, profile, CDP endpoint, and target intent.
- Owner preparation never invokes a browser-launching command or the resolver
  after proof failure.
- Duplicate or conflicting retained identity fails before daemon attachment.
- The dashboard labels the explicit retry as `Recover and take control` and
  preserves authentication and the opaque handoff URL.
- Tests cover successful planning, missing proof, ambiguity, and the no-launch
  dispatch boundary.
- Existing dirty work remains intact and no installed browser, daemon, route,
  profile, or service is replaced during source validation.

## Non-Goals

- No credential, MFA, CAPTCHA, consent, or account automation.
- No browser/profile cleanup, route eviction, installed-runtime replacement, or
  installed dashboard publication.
- No fallback to a different browser, profile, tab, provider, or raw provider
  URL.

## Review Receipt

- status: completed
- optimization: balanced wall-clock and independent review quality
- architecture lane: source and control-flow review, read-only
- security lane: adversarial identity, ambiguity, and no-launch review, read-only
- integration authority: primary agent
- disposition: fixed typed recovery failures, pre-mutation owner proof, and
  isolated live-smoke cleanup before publication

## Validation Receipt

- Rust formatting, check, clippy, and focused recovery/model/health/stream tests
  passed.
- Service API/MCP parity, generated-client contract and type checks, service
  client tests, route-confusion gates, dashboard fixtures/build, and docs build
  passed.
- Publisher rollback/display-custody fixtures and workstation/Guacamole
  fixtures passed.
- A resolver sender-counter test proves preparation failure leaves
  `requestDispatched: false` and makes zero resolver calls.
- The installed dashboard publisher and live CDP/browser smokes were not run:
  both could replace or interfere with the retained browser owner that this
  change is required to preserve.
- No browser, profile, daemon, route, credential store, or retained service was
  started, stopped, replaced, or republished during validation.

## 2026-09-25 Multi-Tab Recovery Addendum

A production-shaped no-launch check found one remaining ambiguity: the exact
retained browser and session were proven, but two ready owned page targets made
the generic reconnect admission refuse recovery. Recovery now accepts an exact
CLI `--target-id` and MCP `expectedTargetId`. The shared selector requires that
target to be ready, owned by the exact session, present in that session's tab
inventory, and still present in the live CDP target inventory. CLI omission is
kept only for the unambiguous single-ready-page case; MCP requires the target.

Validation proved that omitted and nonexistent targets fail before daemon
startup. Supplying the retained ChatGPT page target then restored only the
missing daemon, returned the expected ChatGPT URL and title, preserved browser
PID `3044531`, and left both original page targets live. The source daemon was
retained as the recovered owner. The installed binary and browser process were
not replaced.
