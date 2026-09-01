# WSL Windows CDP Streaming Fixture

Date: 2026-08-14
Plan: P102

## Finding

The CDP tab-streaming live smoke copied the caller environment, then replaced
an explicit `AGENT_BROWSER_EXECUTABLE_PATH` with `/usr/bin/google-chrome`
whenever that fallback existed. The caller therefore could not select the
installed Windows stealth Chromium build. Windows Chromium also cannot use the
fixture's ordinary WSL `/tmp` profile because the WSL DevTools relay requires
an exact Windows-mounted profile identity.

## Repair

The fixture now preserves explicit executable selection, classifies WSL
Windows executable paths, creates the disposable profile under Windows Temp,
and binds the initial queued navigation to that exact top-level profile path.
Linux fallback behavior stays isolated under the fixture's temporary home.

A focused no-browser test guards executable precedence and profile-root
selection. The live smoke passed three times through the Windows build,
including two runs with the exact caller shape that previously failed. All
runs opened only local data URLs, exercised stream frame changes across two
tabs, and removed their temporary sessions and profiles.

## Retained Lane Observation

The previously recorded ChatGPT browser PID 184301 was already absent. Service
evidence records its unexpected exit at `2026-08-14T20:48:18Z`; the first P102
implementation file changed at `2026-08-14T20:59:17Z`. P102 did not relaunch
that profile or send a prompt. Restoring the retained profile is separate work
that requires explicit operator authority.
