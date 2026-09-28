# Capability-Resolved Dashboard Browser Smoke

Date: 2026-08-14
Plan: P115

## Finding

The dashboard publisher correctly separated required HTTP readiness from an
optional browser diagnostic, but its disposable browser used the ambient launch
default and profile. Under WSL that selected an incompatible local Chrome path
and exited before DevTools, even though the installed service already reported a
ready patched Windows Chromium capability.

## Resolution

Rendered-page QA now reads the installed no-launch `launchConfig`, verifies the
selected patched Chromium manifest, executable, artifact smoke, and WSL profile
support, and passes the verified build through agent-browser. It generates a
profile in the matching Windows user's temp root, closes only the unique smoke
session, and removes only that generated profile. The resolver emits an empty
`unsafeLaunchArgs` list and the smoke source contains no `--no-sandbox` flag.

Explicit browser-build and profile inputs remain available, but a requested
patched build still has to match ready installed evidence. Stale, missing,
failed, or mismatched evidence stops before browser launch.

The first guarded publish also found a listener lifecycle gap: after exact
verification and quiescence of an existing standalone dashboard, the restart
path returned early because no systemd unit was installed. The publisher now
distinguishes resuming the exact listener it stopped from starting a previously
absent dashboard; only the latter needs `--start-if-missing`.

## Live Evidence

- build: `stealthcdp_chromium`
- executable: `/mnt/c/Users/Baker/AppData/Local/chromium-stealthcdp/current/chrome.exe`
- profile source: `generated_disposable`
- rendered URL: `http://127.0.0.1:4848/`
- rendered markers: Agent Browser app chrome and Workspaces pane
- cleanup: exact smoke session closed; generated profile absent
- retained lane: PID 1046742 and CDP endpoint unchanged; read-only URL and title
  remain the Workshop conversation and `Architecture Review Boundaries`
- required publisher: `rendered_page_verified`, exact standalone restart, and
  installed/runtime-manifest SHA agreement
- installed/release/reference SHA-256:
  `07f2b9c0a85d4d30e1b4cb5bb9a077ca58539a4cfc863d15f77e495a7fe013e4`
- live dashboard SHA-256:
  `867429d803a010647225f314540aeddc82ca19efcd57862df37849e8eac5a630`

No prompt, typing, click, retained-page navigation, credential operation,
external write, or GitHub write occurred.
