# Plan 0123 | Compact Doctor JSON

State: CLOSED
Roadmap: P123
Plan version: 1
Date: 2026-08-24

## Objective

Give agents bounded install and remote-view readiness output without changing
the authoritative probes, exit status, or full diagnostic JSON contract.

## Scope

- add command-specific `--compact` JSON projection to install doctor and
  remote-view doctor;
- preserve readiness, issue counts and codes, bounded issue details, remedies,
  runtime counts, and next action;
- omit executable, route, host, user, URL, hash, and full child-report evidence;
- keep full JSON unchanged and both modes strictly no-launch;
- update help, README, docs, skill guidance, and focused tests.

## Acceptance Criteria

1. Compact mode runs the same doctor probes and preserves success semantics.
2. Each response uses a versioned summary schema and caps issue details at 20.
3. Install output retains remedy commands and confirmation or sudo posture.
4. Remote-view output retains control, many-to-many, runtime, and next action.
5. Focused tests prove private and high-volume evidence is omitted.
6. Installed no-launch verification materially reduces emitted bytes.

## Validation Receipt

- compact install doctor: 8 focused compact tests pass; the release projection
  is approximately 78 percent smaller than the full JSON in the same checkout;
- compact remote-view doctor: focused projection coverage and all 46
  remote-view doctor tests pass; the release projection is approximately 95
  percent smaller than the full JSON in the same checkout;
- retained-browser publication safety: native verification now requires the
  named daemon PID to be present and live before consulting persisted browser
  or CDP state; 14 isolated retained-requirement tests pass, including stale
  service state with no daemon;
- Rust formatting, strict Clippy, docs lint/build, optimized build, CodeGraph
  sync, installed-skill parity, direct plan audit, and diff hygiene pass;
- the implementation-validation phase launched no browser and sent no prompt.

## Live Closeout

The operator authorized restoration of the exact Workshop lane. A new
`chatgpt-pro` daemon opened only the retained canonical conversation URL and
produced one exact valid service-tab handle. The obsolete dead-target
requirement and enforcement marker were moved to a recoverable private archive,
then marker-first pinning selected exactly one live profile, session, target,
and URL.

Guarded release publication preserved the ChatGPT, FigureLabs, NYSE, and Route
B viewer browser PIDs across executable handoff. Initial final readiness found
the dashboard user service still serving the old manifest because the shell
lacked its user-bus environment. Recovery first failed closed on the separate
XDG socket directory, then succeeded idempotently with the user bus and
canonical `~/.agent-browser` daemon socket directory. The journal is terminal
`recovered_ready` and final exact-target readiness passed.

The source-free workstation payload refresh completed without interactive
sudo. Installed and release SHA-256 are
`143fc1f682c9c1ef7d7fd1d51175d1a66330e58eeb146803481721d8be0bb4f1`.
Installed compact install and remote-view doctors both report zero issues,
runtime convergence and remote control are ready, and the exact Workshop URL
remains readable. No ChatGPT prompt or page interaction was sent.
