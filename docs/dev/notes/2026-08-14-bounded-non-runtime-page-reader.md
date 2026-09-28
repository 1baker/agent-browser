# Bounded Non-Runtime Page Reader

Date: 2026-08-14
Plan: P101 / Plan 0101

## Trigger

The public Browser Research Concierge reached 9/10 after P100. The remaining
WHATWG HTML Standard page could exceed the renderer deadline even though the
browser and later missions stayed healthy. Existing `content` extraction used
`Runtime.evaluate`, while response-body inspection required a prior tracked
request and the old page session.

## Contract

- `get page --url <url>` accepts HTTP(S) only and obeys the configured domain
  allowlist;
- default limits are 64,000 bytes and 15 seconds, with hard limits of 1,000,000
  bytes and 60 seconds;
- credentials are excluded unless `--include-credentials` is explicit;
- the reader creates one temporary background target, attaches to its main
  frame, calls `Network.loadNetworkResource`, consumes bounded `IO.read`
  chunks, closes the stream, and closes that exact target;
- the active target is not navigated, replaced, or evaluated, and no action is
  replayed;
- output includes requested URL, active-target identity, status, MIME type,
  source, byte counts, truncation, credential posture, and text.

## Verification

- mock-CDP coverage proves there is no `Runtime.*` command, credentials default
  off, base64 data truncates exactly, stream cleanup occurs after success and
  read failure or timeout, the temporary target closes exactly once, and the
  original target remains active;
- parser coverage proves bounded defaults, explicit overrides, credential
  opt-in, and missing/zero-value rejection;
- the isolated WHATWG run passed with `evidenceSource=network-resource`, no
  retry, exact cleanup, and the expected title and evidence terms;
- the frozen public suite passed 10/10 in 48.468 seconds. WHATWG was the only
  navigation timeout and all four later missions passed in the same session;
- the serial Rust suite passed 1,816 tests with 57 ignored. Formatting, strict
  Clippy, docs TypeScript/build, route-confusion gates, JavaScript syntax,
  validation selection, and diff hygiene also passed;
- the first parallel Rust run exposed two unrelated shared-temporary-state
  collisions; both disappeared in the deterministic serial full run;
- targeted MDX lint returned no error and one warning that the MDX file has no
  matching ESLint configuration. This checkout has no plan-audit package
  command.
- the generic CDP-tab-streaming live smoke remains blocked before navigation by
  its isolated WSL Chrome launch (`Chrome exited early`); an exported Windows
  executable path did not alter that fixture-owned launch, so the reader path
  was not exercised by that unrelated smoke.

## Installed Runtime

Executable handoff preserved retained browser PID 184301, profile
`chatgpt-pro`, CDP endpoint
`ws://127.0.0.1:35685/devtools/browser/29bb473f-7ba6-4b46-aef3-777d62312dd5`,
and the `Architecture Review Boundaries` target at
`https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`.

Installed, debug, and reference binaries share SHA-256
`739201f613f5b5cf987347618908528cd05a57ca8fe2c064b1699d2728f7552b`.
The initial publisher final check saw the old standalone dashboard manifest
because WSL had no user bus. Restarting only the exact dashboard listener fixed
that stale readback; runtime smoke then passed with dashboard SHA-256
`c0ed91d5e5da51c75600ff390e5864635c3205298805fe4b253c7d49e34c6c5b`.

The installed reader fetched `https://example.com/` with credentials disabled,
HTTP 200, 559 bytes, and no truncation. Browser-level target inventory was
identical before and after, including the retained ChatGPT target. No prompt,
composer action, login, or form submission occurred.
