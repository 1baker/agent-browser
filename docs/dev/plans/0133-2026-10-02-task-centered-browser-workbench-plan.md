# Plan 0133: Task Centered Browser Workbench

Date: 2026-10-02
State: IN PROGRESS; recent-work view and opt-in page-gate path implemented, live acceptance pending
Review: Codex and Claude Code, session `cc8cd1d7-57e0-4c30-a95f-81425fdcca93`

## Goal

Make Agent Browser a dependable place for an agent and a human to complete
browser work together. A task should have a clear current state, one next
action, an exact browser and tab identity, a reviewable outcome, and a safe
cleanup path. LitScout acquisition and retained ChatGPT work are the first
acceptance journeys.

## Current evidence and boundaries

- The Rust service already owns browser processes, profiles, sessions, tabs,
  leases, jobs, access plans, and handoffs. MCP and HTTP clients use the same
  service request contract. See `cli/src/native/service_contracts.rs`,
  `cli/src/native/service_access.rs`, and `cli/src/mcp.rs`.
- The dashboard has browser, workspace, activity, and Service surfaces, plus a
  large Service panel and a task authority workspace. See
  `packages/dashboard/src/components/app-shell.tsx`, `service-panel.tsx`, and
  `task-authority-workspace.tsx`.
- Park, reopen, exact handle refresh, and transactional retained browser
  recovery exist. See `cli/src/native/actions.rs` and plan 0132. Park and
  reopen preserve the profile and URL lineage, not unsaved page state.
- Challenge types and access plan reasoning exist, but a production path that
  creates a waiting challenge from an observed page has not been established.
  The observed LitScout DOI run reached ScienceDirect and stopped on its
  robot check without article metadata.
- Existing work in `cli/src/native/cdp/chrome.rs`,
  `cli/src/native/remote_view_handoff.rs`, and
  `cli/src/native/service_store.rs` belongs to another unfinished slice.
  Implementation touching those files must be coordinated after reviewing
  that slice; this plan does not modify them.

## Product shape

The main entry point is a task queue with a **Needs you** section and an
**Active runs** section. A run detail shows the intent, current state, exact
browser/profile/tab, what happened, evidence, and one next action. It embeds
the existing remote view for human control. The existing Browser, Service,
and Activity surfaces remain available for operations and diagnosis.

The backend initially computes a run view from existing jobs, task authority,
outcome receipts, tab handles, challenges, and handoffs. It does not introduce
a second browser owner or a new durable run store by default. If a state
transition cannot be reconstructed after restart, add only that missing
durable fact through the existing service authority.

Human gates pause work on the bound tab and present a durable handoff.
Challenge and sign-in detection never imply that Agent Browser solved the
challenge or authenticated the account. A user action to recheck must confirm
the page changed before the run resumes.

## Implementation sequence

1. **Trace and gap matrix.** Follow every production write to challenge
   records, the profile seeding handoff path, task authority and outcome
   receipts, and access plan `manual_action_required`. Record which run states
   can be rebuilt after restart. Output a source cited note before changing
   models or storage.
2. **Read-only run projection.** Add a service-owned run collection over the
   existing records, with HTTP, MCP, and generated client parity. Include
   queued, working, needs human, completed, failed, and unknown states only
   where evidence supports them. Show the exact next action and bounded
   outcome evidence. Test seeded state and restart reconstruction.
3. **Durable transition facts.** Add only the missing facts identified in step
   1, likely human gate opened or resolved and run to tab binding. Prefer the
   existing event and outcome mechanisms. Recheck persistence across daemon
   restart. Coordinate any `service_store.rs` work with its current owner.
4. **Bounded page observer.** Enable site policy detection for known sign-in
   and challenge signals with a time limit, false positive fixtures, and an
   explicit `clear`, `challenge`, `signin_required`, or `unknown` result.
   Without CDP or a matching policy, preserve cheap observed status/title as
   `unknown` evidence; never infer a solved challenge from that evidence.
   A detected gate pauses commands for the exact tab and creates a handoff.
5. **Task first dashboard.** Add Needs you and run detail as one entry path.
   Reuse the current remote viewport and durable handoff. When recovery has
   multiple proven targets, show a target picker and send the chosen exact
   target ID. Keep the other target and browser process unchanged. Coordinate
   `remote_view_handoff.rs` with its current owner.
6. **Park and reopen clarity.** Show that reopening restores the URL in the
   same profile with a new target. Record target lineage. For run-bound
   handles, reject silent replacement by `tab_handle_refresh`; preserve
   existing behavior for other callers.
7. **Live acceptance.** On a disposable profile, LitScout metadata discovery
   acquires the matching DOI page, either captures verified article metadata
   or creates a human gate, resumes only after operator action and recheck,
   records the outcome, and closes exactly its task tab and browser. A
   separate retained ChatGPT check proves reuse of the same process and
   target without sending a prompt. A manual sign-in fixture proves the
   handoff and readiness transition without revealing credentials. Failure
   at a real site remains a typed partial outcome, not a success claim.

## Verification and release gates

- For each source slice: focused Rust tests, service API and MCP parity,
  generated client checks, `cargo fmt --check`, and `clippy -D warnings`.
- For UI slices: rendered interaction tests, dashboard build, desktop and
  narrow viewport inspection, and an isolated dashboard flow.
- For live acceptance: exact profile, process, target, lease, handoff,
  persisted outcome, and cleanup readback. Preserve retained browser owners.
- Update every user facing documentation surface required by `AGENTS.md` when
  implementation changes behavior. Use `pnpm validation:select` to select
  the final relevant checks.

## Decisions

- LitScout remains a client of Agent Browser's service contract; provider
  specific article logic stays outside the browser broker.
- Passive 403/429 or title evidence is useful as `unknown`; it cannot alone
  set a challenge state.
- No challenge bypass, automatic CAPTCHA solving, profile substitution, or
  silent target replacement is part of this plan.
- The three existing dirty Rust files are preserved until their ownership and
  current change are reviewed.

## 2026-10-02 implementation checkpoint

- `a5f739f6` added the read-only recent-work projection, HTTP/MCP/client
  parity, dashboard Browser work route, and initial gap matrix. Browser action
  success remains unverified as a task outcome.
- The next source slice adds bounded site-policy page-gate signals, a
  persisted exact-tab human challenge, a before-admission pause for agent page
  mutations, and a dashboard recheck path. Its unit, contract, and build gates
  have passed; it is not yet a live-accepted or installed runtime behavior.
- The isolated `test:service-probe-live` prerequisite failed before a probe:
  first Chrome exited before exposing DevTools; with an explicit Chrome-for-
  Testing executable, the fixture refused a retained browser whose build was
  `unknown` when `stock_chrome` was required. Both disposable fixture sessions
  cleaned up. Do not interpret this as page-gate acceptance or retry over a
  retained browser.
- Durable general task-run identity, outcome/cleanup receipts, recovery picker,
  Park/Reopen lineage, and LitScout/ChatGPT live journeys remain open.
