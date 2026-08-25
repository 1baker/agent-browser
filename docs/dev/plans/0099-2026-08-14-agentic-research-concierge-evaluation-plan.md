# Plan 0099 | Agentic Research Concierge Evaluation

State: CLOSED
Roadmap: P99
Plan version: 1
Date: 2026-08-14

## Objective

Add and run a repeatable ten-mission public-web evaluation that measures
whether agent-browser can acquire the intended page, verify canonical identity,
extract bounded evidence, stay within time limits, and clean up exactly before
authenticated or consequential agentic workflows are attempted.

## Scope

- one disposable browser profile and named daemon session;
- ten stable public primary or official documentation sources;
- per-mission canonical URL, title, evidence-term, and duration checks;
- zero retries so transient and deterministic failures remain visible;
- structured JSON scorecard with per-mission evidence and cleanup posture;
- installed agent-browser plus configured Windows Chromium when available.

## Non-Goals

- no ChatGPT, Gmail, Slack, Google Drive, or other authenticated profile;
- no form submission, file transfer, purchase, message, or account mutation;
- no LLM judging, autonomous prompt loop, anti-bot bypass, or CAPTCHA work;
- no retained-target attach against the current AuraCall browser;
- no GitHub write or publication in this slice.

## Acceptance Criteria

1. The harness always uses a unique session and disposable profile.
2. Every mission binds evidence to its requested and canonical final URL.
3. Every mission records title, heading, evidence terms, link count, duration,
   and pass/fail checks in a versioned JSON report.
4. Mission commands have bounded timeouts and no automatic retry.
5. Cleanup closes the exact session and removes only the created profile and
   isolated agent-browser home.
6. The ten-mission live run either passes completely or produces a concrete,
   reproducible improvement backlog without touching authenticated browsers.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: evaluation_harness_implementation_started
- evidence: existing operation-level live smokes cover browser actions,
  service requests, recovery, and handoff, but no compact public-web mission
  scorecard was found.
- subagent_status: not_spawned; implementation, one disposable browser session,
  and failure adjudication are tightly coupled, and parallel browser work would
  add profile and lifecycle contention.
- authority_classification: explicit user acceptance of the recommended
  Browser Research Concierge evaluation.
- review_disposition_summary: measure deterministic public-source acquisition
  before expanding into authenticated or consequential agentic browsing.
- next_action_or_stop_reason: run syntax and live validation, then classify all
  failures without weakening page-identity checks.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: live_evaluation_and_bounded_harness_repair
- evidence: the first browser-reaching run scored 6/10. Example Domain used
  current `documentation examples` text; RFC 3986 and RFC 8259 provided correct
  body evidence with empty document titles; the WHATWG single-page standard
  timed out during navigation and Runtime evidence extraction.
- subagent_status: not_spawned; the same disposable session was required for
  causal failure attribution.
- authority_classification: inherited evaluation authority.
- review_disposition_summary: update source-drift evidence from captured text,
  accept page identity from title, heading, or bounded body evidence, and add
  post-timeout outcome verification without replaying navigation.
- next_action_or_stop_reason: enforce one absolute mission deadline and repeat
  the full frozen mission set.

## Execution Receipt C03

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: evaluation_complete_with_blocking_agentic_findings
- evidence: the final budgeted full run passed eight of ten missions. WHATWG
  failed in 49.6 seconds with a typed 15-second service-job timeout. Python,
  Rust, and Git then passed, but the final Node.js mission received a CDP
  connection refusal and auto-launch failure. The same Node.js mission passed
  alone in 1.3 seconds. Every run completed exact cleanup.
- blocking_findings:
  - `P99-F1`: a monolithic page can keep the renderer unavailable after the
    navigation deadline; bounded Runtime evidence remains unavailable.
  - `P99-F2`: the stalled renderer can cause delayed session or browser CDP
    loss several missions later, and automatic recovery does not restore the
    next mission.
- nonblocking_findings:
  - titleless standards pages require evidence-backed page identity rather than
    a title-only gate;
  - public-source text changes require explicit captured-term refresh rather
    than silent benchmark weakening.
- plan_audit: this checkout has no `plans:audit` script; roadmap, runbook, plan,
  note, and patch consistency were checked directly.
- subagent_status: not_spawned; no independent write or browser lane remained
  after the causal cascade became the acceptance evidence.
- authority_classification: inherited evaluation authority.
- review_disposition_summary: close the measurement slice honestly at 8/10;
  do not replace the difficult source or add blind retries to manufacture a
  passing score.
- next_action_or_stop_reason: implement a post-timeout browser and target health
  circuit breaker, exact affected-tab replacement, exact-browser recovery when
  CDP is lost, and a non-Runtime or precise-section evidence path.
