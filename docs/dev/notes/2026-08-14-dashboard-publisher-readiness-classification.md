# Dashboard Publisher Readiness Classification

Date: 2026-08-14
Plan: P114 / Plan 0114

## Outcome

Local dashboard publication no longer treats one combined smoke as the
authority for both installation and rendered browser QA. Required readiness
first proves served HTML, requested asset markers, the runtime-manifest
contract, dashboard bundle digest, executable path, and executable digest.
Disposable-browser rendering is then reported independently.

A known Chrome launch failure before renderer acquisition is
`browser_launch_unavailable` and advisory by default. Operators can require it
with `--require-browser-smoke`. HTTP, marker, manifest, authentication, DOM,
workspace, and other post-launch failures remain fatal.

The live attempt exposed a second real gap: without a user systemd bus, an old
standalone listener could survive binary replacement. The publisher now reads
the dashboard PID record and requires matching user, command, and dashboard-mode
evidence before signaling that exact PID. It never uses a broad process match.

## Verification

- focused policy regressions and existing local-convergence fixtures passed;
- script syntax, API/MCP parity, production strict clippy, formatting,
  dashboard release build, docs build, skill parity, and diff hygiene passed;
- live publication replaced exact dashboard PID 2373445 with 2448046;
- installed and release binary SHA-256 is
  `8afeb3a270ce54c85cc25a14292e75e6299eee4c5dcc087c9aaf2342e992929e`;
- the live runtime manifest matches that executable and reports dashboard
  SHA-256 `ae1565768a00643425b703ae38c9fd8a992eb1e02534c5c78e76a7635de14027`;
- the required `Receipt retention review` marker was served;
- the observed pre-render Chrome exit returned successful publication with a
  structured advisory browser diagnostic;
- retained browser PID, CDP endpoint, URL, and title remained exact through the
  executable handoff.

The next increment should select a disposable browser from installed host
capabilities, including the supported WSL-to-Windows patched Chromium lane, so
rendered-page QA can succeed without weakening managed Chrome sandbox policy.
