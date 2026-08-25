# Agentic Research Concierge Evaluation

Date: 2026-08-14
Plan: P99 / Plan 0099

## Outcome

The new `test:agentic-research-concierge-live` harness runs ten official public
documentation missions through installed agent-browser and the configured
Windows Chromium build. It uses one disposable profile and session, allows no
authenticated state or page mutation, performs no automatic navigation retry,
and writes a versioned JSON scorecard when
`AGENT_BROWSER_RESEARCH_REPORT` is set.

Each mission records requested and final URL, title, heading, bounded text
sample, link count, evidence terms, missing terms, duration, timeout recovery,
and independent checks for canonical URL, page identity, evidence, and the
mission deadline. Focused reruns can set
`AGENT_BROWSER_RESEARCH_MISSION_IDS` to comma-separated mission IDs.

## Live Results

The first browser-reaching run scored 6/10 and provided useful evaluator
corrections:

- Example Domain now says `documentation examples`;
- RFC 3986 and RFC 8259 expose correct body identity with empty HTML titles;
- the WHATWG single-page HTML Standard exceeds the renderer deadline.

After evidence and page-identity correction, a full run scored 9/10, with only
WHATWG failing. The final absolute-deadline run scored 8/10:

- eight missions produced correct canonical URL, identity, and evidence;
- WHATWG failed in 49.6 seconds with a typed 15-second service-job timeout;
- Python, Rust, and Git still passed after that timeout;
- the final Node.js mission then failed because the CDP endpoint refused the
  connection and auto-launch did not recover it;
- Node.js passed in 1.3 seconds in a new isolated session, proving delayed
  damage from the earlier renderer stall rather than an independently bad
  source.

All runs reported `cleanupComplete: true`. No ChatGPT prompt, authenticated
profile, form submission, download, upload, message, or account mutation was
used.

## Findings

### P99-F1 | Monolithic renderer stall

The WHATWG single-page standard can finish neither navigation nor a small
Runtime evaluation within the bounded job budget. Incremental text-node
walking does not help because the renderer remains unavailable before the
expression can execute.

Required direction: use precise-section source routing when the task permits
it, and add a bounded non-Runtime evidence reader for cases where the exact
monolithic document is required.

### P99-F2 | Delayed CDP-loss cascade

A timed-out renderer job can leave the retained session apparently usable for
several later missions before the browser CDP endpoint refuses connection.
The following auto-launch path did not restore the mission.

Required direction: after every renderer or service-job timeout, probe browser
CDP and exact target health before accepting another mission. Replace only the
affected tab when browser CDP is healthy. When the exact owned browser endpoint
is lost, perform one evidence-backed browser recovery and prove the failed
action was not committed before replaying anything.

## Reproduction

Run the complete public suite:

```bash
pnpm test:agentic-research-concierge-live
```

Run only the monolithic-page finding:

```bash
AGENT_BROWSER_RESEARCH_MISSION_IDS=whatwg-html-standard \
  pnpm test:agentic-research-concierge-live
```

The command intentionally exits nonzero while either blocking mission remains.
