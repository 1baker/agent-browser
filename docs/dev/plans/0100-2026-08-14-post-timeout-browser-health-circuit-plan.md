# Plan 0100 | Post-Timeout Browser Health Circuit

State: CLOSED
Roadmap: P100
Plan version: 1
Date: 2026-08-14

## Objective

Prevent a renderer or service-job timeout from silently poisoning later work in
the same browser session. Preserve the original timeout result, never replay the
timed-out action, quarantine only an affected owned tab when browser-level CDP
remains healthy, and recover the exact owned browser lane when CDP is lost.

## Scope

- classify renderer-facing commands that require a post-timeout circuit;
- probe browser-level CDP independently from the timed-out renderer;
- replace one affected ordinary owned target with a ready blank target;
- preserve service-tab handles and externally attached retained browsers;
- recover a locally owned browser without replaying the timed-out command;
- record focused unit and live ten-mission regression evidence.

## Non-Goals

- no automatic retry or replay of the timed-out action;
- no closing or replacement of service-owned retained tabs;
- no authenticated browser mutation or ChatGPT prompt;
- no change to the caller-visible timeout classification;
- no GitHub write or publication in this slice.

## Acceptance Criteria

1. A renderer-facing service-job timeout is returned once and remains recorded
   as `timed_out`.
2. Browser-level CDP health is probed without using the timed-out renderer.
3. A healthy locally owned browser gets one new blank target and closes only the
   timed-out target; the browser process is preserved.
4. A lost locally owned browser connection receives one bounded recovery pass
   without replaying the timed-out action.
5. Service-tab handles and external retained browsers fail closed without tab
   replacement or browser shutdown.
6. The frozen ten-mission concierge sequence completes without a delayed CDP
   cascade, and exact cleanup plus retained-browser preservation are verified.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started_from_p99_blockers
- evidence: P99 proved that a WHATWG renderer timeout can leave browser-level
  CDP initially healthy while a later mission receives a refused CDP recovery;
  the isolated Node.js mission passed by itself.
- subagent_status: not_spawned; the active system instruction forbids proactive
  subagent spawning, and timeout state, target ownership, and recovery are one
  tightly coupled Rust write surface.
- authority_classification: explicit user direction to continue with the next
  recommended engineering priority.
- review_disposition_summary: implement a no-replay health circuit before
  expanding agentic browsing scope.
- next_action_or_stop_reason: add focused regressions, implement bounded target
  quarantine and owned-browser recovery, then rerun the frozen live sequence.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: live_cascade_repair
- evidence: the first full rerun reached Node.js successfully but the immediate
  Python mission waited 58 seconds because disconnected-browser recovery still
  attempted polite close through the dead CDP channel. Exact-process cleanup
  replaced polite close only for locally owned disconnected browsers.
- subagent_status: not_spawned; system policy prohibited proactive delegation.
- authority_classification: inherited implementation authority.
- review_disposition_summary: keep healthy-browser target quarantine and add a
  force-close fallback after replacement failure; never mutate external lanes.
- next_action_or_stop_reason: rerun the critical cascade and full frozen suite.

## Execution Receipt C03

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: verified_complete
- evidence: the critical WHATWG/Python/Rust sequence passed both post-timeout
  missions, and the full frozen run scored 9/10. WHATWG remained the one
  bounded failure at `about:blank`; Python, Rust, Git, and Node passed afterward
  in the same session. The installed binary repeated WHATWG then Python with
  Python passing in 1.663 seconds and exact cleanup.
- validation: 1,811 Rust tests passed with 57 ignored; formatting, strict
  Clippy, focused timeout tests, docs build and TypeScript, route-confusion
  gates, release build, installed dashboard runtime smoke, validation selection,
  and diff checks passed. The CDP-tab-streaming live smoke remains blocked by
  its disposable Linux Chrome launch configuration under WSL and fails before
  navigation; the P100 Windows Chromium harness covers the changed path.
- installed_runtime: installed, workspace, and reference binaries share SHA-256
  `6c6bcd338465639a3937b6a9c4c6f4a787b7e2847396cde451965603174968a8`.
- retained_browser: executable handoff preserved browser PID 184301, runtime
  profile `chatgpt-pro`, the same CDP endpoint, and one target. Read-only checks
  found the current retained page titled `Architecture Review Boundaries` at
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`; no prompt was
  sent.
- plan_audit: no `plans:audit` script exists; plan, roadmap, runbook, note,
  behavior docs, inline comments, tests, installed skill, and patch scope were
  checked directly.
- subagent_status: not_spawned; system policy prohibited proactive delegation.
- authority_classification: inherited implementation and safe install authority.
- review_disposition_summary: P99-F2 is closed. P99-F1 remains a separate
  precise-section or non-Runtime extraction improvement.
- next_action_or_stop_reason: proceed to a bounded non-Runtime reader for
  monolithic standards pages without weakening the frozen benchmark.
