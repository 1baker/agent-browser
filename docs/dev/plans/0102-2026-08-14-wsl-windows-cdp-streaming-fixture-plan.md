# Plan 0102 | WSL Windows CDP Streaming Fixture

State: CLOSED
Roadmap: P102
Plan version: 2
Date: 2026-08-14

## Objective

Make the isolated CDP tab-streaming live smoke honor an operator-selected
browser executable and launch Windows Chromium from WSL with a disposable
Windows-mounted profile.

## Scope

- preserve explicit executable selection ahead of the Linux Chrome fallback;
- create the disposable profile on a Windows-mounted temporary root when the
  selected executable is a Windows browser;
- bind the first service request to that exact profile;
- clean up only the disposable session and profile;
- add a no-browser regression for executable precedence and profile placement;
- rerun the live streaming smoke without touching the retained authenticated
  browser lane.

## Non-Goals

- no change to retained browser or broker identity;
- no ChatGPT prompt, login, form submission, or authenticated navigation;
- no GitHub write or publication;
- no weakening of Windows browser process and profile ownership checks.

## Acceptance Criteria

1. An explicit `AGENT_BROWSER_EXECUTABLE_PATH` is never replaced by the
   `/usr/bin/google-chrome` fixture fallback.
2. A `/mnt/<drive>/.../*.exe` browser receives a disposable profile below a
   Windows-mounted temporary root.
3. The live smoke launches through the selected Windows browser, advertises a
   ready controllable `cdp_screencast` stream, switches between two disposable
   tabs, and closes its exact session and profile.
4. Non-Windows fixture behavior remains isolated and the focused regression
   runs without launching a browser.
5. The fixture never addresses or relaunches the retained ChatGPT lane. If that
   lane is already unavailable, preserve its state and report the timestamped
   failure evidence instead of manufacturing a preservation claim.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the live smoke copied the caller environment and then overwrote an
  explicit executable with `/usr/bin/google-chrome` whenever that fallback was
  installed. Windows Chromium also requires a Windows-mounted profile for its
  exact process identity and DevTools relay checks.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: explicit user direction to fix the recorded gap.
- next_action_or_stop_reason: repair fixture executable precedence and profile
  placement, add focused coverage, then rerun the exact live smoke.

## Execution Receipt C02

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- implementation: explicit executable selection now wins over the installed
  Linux fallback. The live fixture detects WSL-mounted Windows executables,
  creates its profile under Windows Temp, passes that exact path as the
  top-level service-request profile, and removes only that profile after exact
  session close.
- focused_evidence: `test:service-cdp-tab-streaming-fixture` proves configured
  executable precedence, absent-fallback behavior, WSL Windows executable
  classification, and Windows versus Linux disposable-profile placement
  without launching a browser.
- live_evidence: `test:service-cdp-tab-streaming-live` passed through the debug
  executable and twice through the exact previously failing caller shape with
  only `AGENT_BROWSER_EXECUTABLE_PATH` configured. All runs advertised a ready
  controllable CDP stream, switched between two data-URL tabs, and removed
  their temporary sessions and profiles.
- retained_lane_evidence: the former retained PID 184301 had already exited at
  `2026-08-14T20:48:18Z`, eleven minutes before the first implementation file
  changed at `2026-08-14T20:59:17Z`. The fixture did not address or relaunch
  that profile. A later intended read-only CLI probe observed the stale session
  and unexpectedly entered auto-launch, which failed before exposing DevTools;
  it did not relaunch the retained profile and no ChatGPT prompt followed.
- subagent_status: not_spawned; active system policy prohibited proactive
  delegation.
- next_action_or_stop_reason: P102 is closed. Diagnose or explicitly relaunch
  the retained ChatGPT profile only under a separate operator-authorized task.
