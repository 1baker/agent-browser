# Plan 0101 | Bounded Non-Runtime Page Reader

State: CLOSED
Roadmap: P101
Plan version: 1
Date: 2026-08-14

## Objective

Add a generic, bounded page-source reader that does not execute JavaScript or
depend on a responsive renderer, then use it to close the remaining WHATWG
mission without weakening canonical URL or evidence checks.

## Scope

- expose `get page` with explicit URL, timeout, byte cap, and optional
  credential inclusion;
- load through Chromium `Network.loadNetworkResource` and bounded `IO.read`;
- enforce domain policy and hard timeout and output caps;
- return source, status, MIME type, byte counts, truncation, and target identity;
- add parser, CDP, action, output, and live benchmark coverage;
- install and safely verify the retained browser remains unchanged.

## Non-Goals

- no JavaScript evaluation, DOM traversal, or action replay;
- no automatic authenticated credential use;
- no bypass of configured domain policy;
- no ChatGPT prompt, login, form submission, or other browser mutation;
- no GitHub write or publication in this slice.

## Acceptance Criteria

1. `get page --url <url> --max-bytes <n> --timeout <ms>` rejects missing or
   invalid bounds before network activity.
2. The reader uses no `Runtime.*` command and closes its stream on success,
   truncation, timeout, and read failure when a stream exists.
3. Credentials are excluded by default and included only by explicit flag.
4. Output binds requested URL, active target URL, HTTP status, MIME type, source,
   body bytes, returned bytes, and truncation state.
5. The frozen ten-mission run passes 10/10 with zero navigation or action retry.
6. Exact disposable cleanup and retained authenticated-browser preservation are
   verified after installed-runtime validation.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the existing `content` path calls `Runtime.evaluate`; response-body
  inspection requires a previously tracked request and renderer-bound session.
  Chromium `Network.loadNetworkResource` provides a browser-network stream that
  can be consumed with bounded `IO.read` without page JavaScript.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation and the parser, CDP stream, output, and live harness are one
  coupled contract.
- authority_classification: explicit user direction to proceed with the next
  priority.
- review_disposition_summary: implement a generic browser-network reader before
  adapting the benchmark.
- next_action_or_stop_reason: add focused regressions and test the reader against
  WHATWG in a disposable installed-browser lane.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- implementation: `get page` now validates HTTP(S), domain policy, byte and
  time bounds, defaults credentials off, creates one temporary background
  target, reads `Network.loadNetworkResource` through bounded `IO.read`, and
  verifies stream plus exact target cleanup before returning. The active target
  is neither navigated nor replaced and no `Runtime.*` command is emitted.
- focused_evidence: three parser tests and three mock-CDP tests prove defaults,
  explicit credential opt-in, bound rejection, base64 truncation, no Runtime
  command, exact stream close on success, read failure, and timeout, exact
  temporary target close, and active-target preservation.
- live_evidence: the isolated WHATWG run passed once through
  `network-resource`; the frozen ten-mission run passed 10/10 in 48.468 seconds
  with zero retries and exact disposable cleanup. WHATWG recovered bounded
  evidence after its navigation timeout and every later mission passed.
- widened_evidence: the serial Rust suite passed 1,816 tests with 57 ignored;
  strict Clippy, formatting, parser/CDP focused tests, docs TypeScript and build,
  route-confusion gates, validation selection, JavaScript syntax, and diff
  hygiene passed. Targeted MDX lint reported only that MDX has no matching
  ESLint configuration. No package plan-audit script exists in this checkout.
  The generic CDP-tab-streaming live smoke remains blocked before navigation by
  its isolated WSL Chrome launch (`Chrome exited early`); exporting the Windows
  executable did not alter that fixture-owned launch path, so it did not
  exercise `get page`.
- install_evidence: runtime handoff retained browser PID 184301, the exact CDP
  endpoint, profile `chatgpt-pro`, and the `Architecture Review Boundaries`
  ChatGPT target. Installed, debug, and reference binaries share SHA-256
  `739201f613f5b5cf987347618908528cd05a57ca8fe2c064b1699d2728f7552b`.
  The dashboard runtime smoke passed after restarting only its standalone
  listener; dashboard SHA-256 is
  `c0ed91d5e5da51c75600ff390e5864635c3205298805fe4b253c7d49e34c6c5b`.
- retained_lane_evidence: installed `get page` fetched Example Domain with
  credentials disabled and 559 returned bytes; the seven browser-level targets
  and exact ChatGPT target identity were unchanged before and after. No prompt,
  form, login, or composer action was sent.
- subagent_status: not_spawned; active system policy prohibited proactive
  delegation.
- next_action_or_stop_reason: P99-F1 is closed; define the next agentic-browser
  priority from the remaining authenticated-work safety gates.
