# Browser workbench end-to-end validation

Date: 2026-10-02 (America/Chicago)
Plan: 0133
Verdict: disposable workbench journey passed; external site completion partial.

## What the live test proved

`pnpm test:workbench-live` runs real Chrome, the native service, and the built
dashboard on isolated local endpoints. It uses generated disposable dashboard
credentials and a server-controlled synthetic gate. It does not solve a real
verification challenge or prove a human completed one.

The final run passed 15 checks:

1. Session inventory excludes foreign browsers.
2. Browser acquisition returns an exact handle and the observer records a gate.
3. An agent click on the waiting target is refused.
4. The rendered dashboard signs in through the actual authentication endpoint.
5. Needs you shows the exact task tab.
6. The rendered recheck button preserves a still-present gate.
7. Unknown page evidence does not clear the gate.
8. Positive ready-page evidence resolves it through the dashboard button.
9. Input resumes on the same exact target and the resulting DOM is checked.
10. Desktop layout at 1280 by 900 has no horizontal overflow.
11. Mobile layout at 390 by 844 has no horizontal overflow.
12. Physical tab release removes the task target and preserves another tab.
13. Reopen restores the URL in the same profile with a different target.
14. Gate state and recent work survive a verified daemon stop and new process.
15. The fixture browser and daemon terminate before the temporary home is removed.

The test checks that successful actions never become verified task outcomes in
the recent-work projection. General durable outcome and cleanup receipts are
still outside that projection's capabilities.

## Defects found and corrected

- Basic page input could act on the active tab even when a different
  serviceTabHandle was supplied. After a task probe switched the active target,
  the dashboard recheck click failed on that task page. Page input and
  inspection now select and validate their handle target before admission and
  fallback launch. The live journey reproduces this cross-tab sequence.
- The temporary Chrome inventory hid the real installed build, causing the
  generic live probe to fail build proof. The fixture now exposes only the
  installed executable through a temporary inventory symlink.
- The source runtime did not implement the external-discovery environment
  switch expected by the fixture. It now omits foreign process/CDP discovery
  when disabled; owned socket sessions remain visible.

Independent review accepted two blockers, both resolved and reviewed again:
WB-R1 removed inherited attachment, provider, profile, config, and authority
overrides from both fixtures. WB-R2 added process-instance shutdown/restart
checks and guaranteed fixture HTTP server shutdown even when process cleanup
fails. A failed termination preserves the temporary home for investigation.
The reviewer performed source review; live results were verified by Codex.

## Real service checks

The installed LitScout CLI retrieved DOI `10.1016/0022-328x(85)88056-6`
from Crossref without saving a literature session. Agent Browser's access plan
and no-launch preflight selected the registered Linux stealth Chromium route.
The matching ScienceDirect page returned title "Just a moment...", heading
"Are you a robot?", and no citation DOI/title metadata. The task's exact tab
was physically closed and its dedicated browser session shut down. This proves
acquisition, bounded observation, and cleanup, not external human completion.

The retained ChatGPT route answered a bounded URL/title probe on its existing
target. Its browser process and target remained retained, and the original
selected tab was restored. No prompt, account change, or fresh login occurred.

## Validation and limits

Passed: workbench live journey, service probe live, CDP tab streaming live,
service client suite, API/MCP parity, dashboard navigator and inspector checks,
route-confusion gates, smoke environment tests, focused handle-target,
page-gate, run-projection, discovery, and CDP-stream Rust checks, formatting,
Clippy with warnings denied, JavaScript lint, documentation build, and diff
checks. Linux with installed Chrome for Testing and a built dashboard is
required for the workbench fixture.

Desktop and mobile screenshots were inspected. The page is readable, but on
mobile the selected detail follows the activity list and can require substantial
scrolling. The activity view remains a command history rather than a durable
task ledger. This is a usability backlog item, not a claim of polished release
readiness.

This checkpoint does not install or replace the retained runtime. It does not
prove OS-level remote keyboard/mouse forwarding, a human-completed real sign-in,
or ScienceDirect article retrieval. Those remain explicit acceptance limits.
