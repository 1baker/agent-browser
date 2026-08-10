# Facebook Search Target CDP Runtime Stall

Date: 2026-08-09

## Request

Investigate a retained authenticated Chromium page that remains present and
visually rendered after Facebook search navigation while all target-scoped CDP
read operations stop producing responses.

This is an investigation handoff. It does not authorize closing or restarting
the retained browser, changing its profile, clearing storage, interacting with
authentication challenges, or retrying the upstream scraping provider.

## Environment

- agent-browser 0.28.0
- installed executable SHA-256:
  `01965e35f09883522ca281fcd66657a6d8d372dcda8797eca7fe260c6f8b4c9b`
- Chromium artifact: `150.0.7835.0+stealthcdp.3676a7503929`
- Chromium executable SHA-256:
  `aebeac48273efa3a2767763cf0694cfa8f1be52c91b7fbafff0d4698a993ffce`
- retained service browser: `session:last30days-facebook`, PID 63205,
  attached CDP port 37539
- install doctor reports the runtime converged and the browser ready with
  `lastError=null`

The doctor inventory also reports both `last30days-facebook.sock` and
`last30days-facebook--last30days-facebook.sock` as live converged daemon
sessions. Treat that as an investigation lead, not a demonstrated cause.

## Upstream Reproducer

Last30Days service 0.3.39 ran one bounded Facebook-only tick:

- tick: `tick-b80d12af0293fc7bb5d3903c71ac72e1`
- execution attempt: `tick-attempt-0aa716a6d8c85b04bf07e8d2225016c8`
- provider attempt: `provider-attempt-65db1ad398602e6f8c7a259bc47a3e79`
- result digest:
  `sha256:d08753a4e782c82b91bf9036eaa1f78feafae145daa6c0d352aca7f19e7683da`
- limits: one attempt, 50 requests, 120 wall seconds, three items, zero cost,
  and zero model tokens

The exact browser operation ledger was:

```text
service  ok         254 ms
service  ok       1,970 ms
tab      ok       9,142 ms
eval     ok       9,137 ms
open     ok      15,544 ms
eval     failed  10,907 ms
tab      ok       8,893 ms
eval     timed_out 12,023 ms
```

The first inventory and authentication evaluation completed. Navigation then
reached the exact Facebook search URL. The navigation read failed, a fresh
target was opened directly at the same verified search URL, and that target's
bounded evaluation also timed out. The provider ended
`facebook_target_unresponsive` after 104 seconds with zero observed candidates,
items, cost, or model use. No authentication, CAPTCHA, checkpoint, rate-limit,
or quality-rejection signal was emitted.

## Independent Manual Checks

The primary agent performed the following read-only checks after the provider
terminated:

1. Raw `http://127.0.0.1:37539/json/list` target discovery returned promptly
   and included the active Facebook search page.
2. The exact session inventory succeeded in about 8.3 seconds and reported four
   tabs, exactly one Facebook tab, and the Facebook search tab active:

   ```bash
   agent-browser --json --session last30days-facebook tab list
   ```

3. A simple session-scoped evaluation did not return within 25 seconds:

   ```bash
   agent-browser --json --session last30days-facebook eval \
     'JSON.stringify({href:location.href,title:document.title,readyState:document.readyState})'
   ```

4. `agent-browser snapshot` on the same session did not return within 30
   seconds.
5. A raw WebSocket connection to the page target sent
   `Runtime.evaluate` directly and received no command response within 18
   seconds.
6. A separate connection to the browser WebSocket received successful replies
   for `Browser.getVersion`, `Target.getTargets`, and
   `Target.attachToTarget` with `flatten=true`. After the successful attach,
   neither `Page.getFrameTree` nor a trivial `Runtime.evaluate` returned within
   12 seconds. The browser-level socket and target attachment therefore remain
   responsive while commands routed into the attached page session stall.
7. The same direct page-WebSocket `Runtime.evaluate("1+1")` probe returned
   `2` within five seconds on the retained LinkedIn, X, preview, and new-tab
   page targets. Only the Facebook search target timed out. The failure is
   page-target-specific, not a browser-wide CDP outage.

These checks distinguish target inventory reachability from target command
responsiveness. They also rule out the Last30Days JSON parsing layer as the
sole cause.

## Rejected Explanation

An earlier unscoped explicit-CDP CLI experiment encountered a default runtime
profile lock held by the separate `shared-social` session. That is not accepted
as the provider failure cause. The production command uses the exact
`last30days-facebook` session, and its tab inventory consistently reaches the
intended retained browser.

## Investigation Targets

CodeGraph identifies the relevant command path at:

- `cli/src/native/browser.rs:1020`, where `Browser::evaluate()` sends
  `Runtime.evaluate` against `active_session_id()`;
- `cli/src/native/cdp/client.rs`, where pending command registration and the
  30-second default CDP command timeout are implemented;
- the native snapshot path, which exhibits the same target-read stall.

Recommended bounded investigation:

1. Add an isolated live reproducer that navigates a dedicated temporary
   profile to a Facebook search page only when an operator-provided authenticated
   fixture is available. Never depend on or mutate the default runtime profile.
2. Record whether `Target.attachToTarget`, `Runtime.evaluate`,
   `Page.getFrameTree`, and `DOM.getDocument` each receive a response after the
   target is visible in `/json/list`.
   Use another retained page target as a same-browser positive control.
3. Inspect the CDP reader loop and pending map to determine whether responses
   are absent from Chromium, received without the expected session or command
   ID, or received but not delivered to the pending caller.
4. Expose a typed per-target responsiveness result distinct from browser
   process health and `/json/list` reachability. A browser should not be
   reported fully usable for page automation solely because inventory works.
5. Preserve the existing browser and target on failure. Do not automatically
   restart, relaunch, close, or clear the profile while collecting this
   evidence.

## Acceptance Evidence For A Repair

A repair is not proven by a longer outer timeout. It should show that, after
the Facebook search page is visible in target inventory:

- a bounded target command returns a typed success or typed protocol failure;
- `eval` and `snapshot` do not hang until their caller deadline;
- the exact retained session remains mapped to the same browser and profile;
- non-Facebook CDP behavior remains unchanged;
- browser and profile lifecycle are preserved when the target command is
  unresponsive.

The upstream Last30Days provider should be rerun only after those conditions
are installed and independently verified.

## Downstream Adaptive Proofs And Final Handoff

Last30Days subsequently consumed its operator-bounded maximum of three adaptive
attempts. These were distinct installed candidates, not blind retries:

1. Service 0.3.40 opened a blank successor, closed the exact wedged predecessor,
   navigated home, and successfully evaluated authentication before opening
   desktop top search. The search command reached its deadline even though the
   resulting target rendered.
2. Service 0.3.41 changed to desktop posts-only search. Navigation returned,
   but the next Runtime evaluation stalled. A separate raw page-WebSocket
   Runtime probe received no response within 15 seconds.
3. Service 0.3.42 navigated directly to mobile posts search and combined auth,
   page-state, and extraction into one Runtime call. Facebook redirected the
   mobile URL to the rendered desktop posts route, where that sole composite
   call timed out after 30 seconds.

The final provider attempt was
`provider-attempt-556a632cf7e995cfa8c3fbf079200478` and ended
`facebook_target_unresponsive` after 105 seconds with one request and zero
observed, accepted, rejected, cost, or model-use counters. It emitted no auth,
challenge, CAPTCHA, rate-limit, or quality signal.

An additional direct `Page.captureScreenshot` command returned an immediate
CDP internal error on the rendered Facebook target. Browser-level discovery,
target inventory, and attachment remained responsive; direct Runtime controls
continued to return on LinkedIn, X, preview, and new-tab targets in the same
browser.

Downstream route, renderer-replacement, capture-count, and clock adaptations
are exhausted. The remaining work is an upstream agent-browser/Chromium target
response investigation using a disposable authenticated fixture. Do not use,
restart, clear, or close the retained default profile as an investigation
shortcut.

## Upstream Repair Candidate And Independent Proof

The upstream investigation continued without another Last30Days provider
attempt and without changing the retained Facebook browser or profile. The
primary agent used direct CDP, disposable agent-browser sessions, and a stock
Chrome control to isolate four separate behaviors:

1. Disposable stock Chrome and stealthcdp Chromium sessions both returned
   ordinary Runtime results on Facebook public pages. The browser build and CDP
   transport are therefore not generally unable to automate Facebook.
2. On the retained Facebook search target, browser-level discovery and selected
   Page commands still returned while Runtime and Debugger commands did not.
   The same commands returned on other targets in that browser.
3. Chromium accepted a renderer-side `Runtime.evaluate.timeout`. A direct raw
   CDP infinite loop returned a typed protocol error after about one second,
   and a subsequent `1+1` evaluation returned `2` on the same target. This
   proves the renderer can be interrupted without closing the browser.
4. A system-call trace showed that an ordinary debug CLI invocation spent about
   5.75 seconds rereading and hashing the 289 MB executable before sending the
   command. The daemon itself completed a raw authenticated baseline evaluation
   in 377 ms total, the infinite-loop failure in 2.952 seconds total, and the
   recovery evaluation in 242 ms total.

The repair candidate addresses those findings at their owning seams:

- Runtime evaluations carry a Chromium renderer deadline that expires before
  the caller deadline. Cancelling the Rust future no longer leaves JavaScript
  executing indefinitely in the renderer.
- Successful navigation reads URL and title through browser-level
  `Target.getTargetInfo`, so navigation no longer depends on a follow-up Runtime
  evaluation against a busy renderer.
- Evaluation responses use already known target metadata rather than adding a
  second target command to the critical path.
- The control-plane worker no longer performs a health probe before starting
  the caller-bounded action. It sends the terminal response before its
  post-action health probe.
- On Linux, a live daemon that references the same executable inode as the CLI
  uses a constant-time identity fast path. SHA-256 remains the fallback when
  the inode differs or process metadata cannot be read, preserving rebuild and
  upgrade detection.

The final fresh disposable CLI proof used session
`p0037-cli-final-timeout-proof` and closed it after validation:

```text
open                         4,632 ms  success
baseline eval 1+1            2,059 ms  success, result 2
infinite-loop eval           4,487 ms  typed CDP failure
immediate recovery eval 1+1  1,383 ms  success, result 2
close                                    success
```

The infinite loop was guarded by a 3,000 ms worker deadline and a 6,000 ms
outer process deadline. It returned before the outer deadline, and the next
evaluation proved that the renderer and session remained usable. This is an
upstream adversarial proof, not a fourth Facebook provider attempt. The
installed Last30Days provider was not rerun. The validated agent-browser
candidate was installed with SHA-256
`17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b`.
An installed disposable proof improved the baseline to 455 ms, returned the
typed infinite-loop failure in 4.546 seconds, and returned the immediate
recovery result in 1.482 seconds before closing the disposable browser. Install
doctor and remote-view doctor then reported ready with zero issues. Retained
Facebook PID 63205 remained ready and was not restarted or closed. Fresh
operator authority is still required for any downstream provider proof.
