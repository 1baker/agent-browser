# Profile Discovery And Manual Browser Launch UX

Date: 2026-07-25

## Summary

Agent-browser must make registered browser identities easy to find and launch.
An operator must be able to search for a website, login, account, or profile,
select a registered profile, and open a browser from the dashboard.

A live X authentication handoff exposed three gaps:

- A detached manual browser was visible through Guacamole but absent from the
  dashboard left rail.
- The dashboard did not offer a profile-first action to launch the registered
  `last30days-facebook` profile.
- An agent had to inspect configuration, runtime state, service state, and
  process state to answer which profile contained the X login.

These are agent-browser product shortcomings. Callers must not need to
reconstruct profile selection or browser ownership from several low-level
surfaces.

## Live Evidence

The operator authenticated X in the shared registered profile:

```text
profileId=last30days-facebook
name=Last30days social authenticated profile
userDataDir=~/.agent-browser/runtime-profiles/last30days-facebook/user-data
authenticatedServiceIds=facebook,linkedin,x
```

The manual authentication browser remained alive:

```text
pid=592914
launchMode=manual
headed=true
targets=none
```

Guacamole displayed the Chromium window on the assigned RDP desktop. The
operator refreshed a stalled X loading page and confirmed that X was logged in.

Service status contained the registered profile and its X readiness evidence.
It contained no browser, session, or tab record for PID `592914`. Resource
discovery classified that process as `observed`, but it did not correlate the
process with a profile, browser, session, display, or Chrome DevTools Protocol
(CDP) endpoint.

The left rail therefore showed neither:

- a service-owned browser row; nor
- a detected non-owned browser row for the manual process.

The browser was operator-visible but absent from the dashboard inventory.

## Shortcoming 1: Manual Browsers Disappear From Workspace Inventory

Detached `runtime login` intentionally starts without CDP for authentication
flows that reject DevTools. That security and compatibility posture must not
make the browser invisible to the operator console.

The dashboard inventory needs an explicit manual-browser class. Agent-browser
can derive this class from runtime state, process discovery, profile directory,
display, and seeding-handoff state without attaching CDP.

The row must state what is known and what is unavailable:

- runtime profile and profile path;
- process ID and browser family;
- display or remote-view route when known;
- launch mode `manual`;
- target website from the launch record when known;
- control through the remote desktop when available;
- automation unavailable until the browser closes or is relaunched attachably;
- the next safe action.

An operator-visible manual browser must not disappear merely because it has no
CDP target.

## Shortcoming 2: Registered Profiles Are Not Directly Launchable

The dashboard must provide a profile-first launch workflow. An operator must be
able to find `last30days-facebook`, inspect its supported websites and login
readiness, and open it without first creating a browser or service request
elsewhere.

The Profiles UX must provide these actions when policy permits:

- **Open browser**: Launch the selected profile with the recommended browser
  build, host, display, stream, and input posture.
- **Open website**: Launch or reuse the profile and navigate to a selected
  registered website.
- **Add tab**: When a compatible retained browser already owns the profile,
  open a service-owned tab in that browser.
- **Seed login**: Start a detached manual browser when the selected website
  needs authentication.
- **View or control**: Open the profile's live remote-view route when a browser
  already exists.
- **Inspect holder**: Show the browser or session that currently owns an
  incompatible or exclusively leased profile.

The UI must route these actions through the access-plan and service request
contracts. It must preserve the one-process-per-profile invariant.

The profile page or launcher must not require an operator to understand daemon
sessions, profile locks, CDP ports, or route hints before opening a browser.

## Shortcoming 3: Profile And Login Discovery Requires Too Much Investigation

Agent-browser must expose one deterministic, searchable query for profile and
login discovery. A question such as “which profile can use X?” must return the
best profile and supporting evidence in one request.

The query must accept any useful combination of:

- website hostname or URL;
- target service ID;
- login ID;
- account ID or safe account label;
- profile ID or profile name;
- service name;
- browser family or build;
- authentication state;
- freshness state;
- tag;
- free-text search over safe profile metadata.

The response must include:

- selected profile and ranked alternatives;
- the field and value that matched each profile;
- authenticated, target-only, shared-service, and account-match distinctions;
- target-specific freshness and manual-seeding state;
- current browser or session holder;
- whether the next action is launch, add tab, view, seed, wait, or inspect;
- a copyable service request or dashboard URL for the recommended action;
- source provenance for config, persisted state, and runtime observation.

The CLI, HTTP API, Model Context Protocol (MCP), generated client, and dashboard
must expose the same query semantics. Agents must not search environment files,
runtime directories, retained state, and process tables to answer routine
profile questions.

## Search And Catalog Requirements

Profiles need human-useful catalog metadata. The current IDs and target arrays
are necessary, but they are not sufficient for fast discovery.

Each registered profile must support:

- a human-readable name and description;
- website origins and target service IDs;
- login IDs and safe account labels;
- authenticated, configured, stale, blocked, and unknown readiness states;
- browser family, build compatibility, and preferred launch posture;
- tags and aliases;
- last verified time per website or login;
- current holder and route availability;
- provenance and last observed time.

Website and login support also needs a searchable catalog. Each site record
must describe:

- canonical name, aliases, and origins;
- login IDs and supported account labels;
- recommended profile and browser posture;
- authentication and challenge constraints;
- manual-login requirements;
- compatible probe and freshness contracts;
- bounded troubleshooting guidance;
- adapter or service clients that use the login.

The dashboard search must match both profile and website records. Searching for
`X`, `x.com`, `Twitter`, `Facebook`, `LinkedIn`, or `last30days` must surface
the relevant profile, login readiness, and safe launch action.

## Deterministic Selection Contract

Search and launch must remain separate operations.

Profile search ranks candidates without launching a browser. Selection returns
an evidence-backed recommendation. Launch submits the selected recommendation
through the service queue.

The selector must use a documented precedence order. Exact authenticated target
and account matches must outrank aliases, tags, target-only records, and
free-text matches. Every result must state the winning match reason.

If no profile matches, the query must return a structured `not_found` result
with safe next actions. It must not silently substitute the default profile.

## Dashboard Acceptance Criteria

The UX meets this requirement when an operator can complete this flow:

1. Open the dashboard.
2. Search for `X` or `last30days`.
3. Find `Last30days social authenticated profile`.
4. See that the profile supports Facebook, LinkedIn, and X.
5. See target-specific authentication freshness and any current browser holder.
6. Select **Open browser** or **Open X**.
7. Receive either a visible browser or a service-owned explanation of the
   blocker.
8. Find the launched manual, detected, or service-owned browser in the left
   rail with an accurate ownership label.

The agent-facing contract meets this requirement when one query identifies the
same profile, match reason, readiness, holder, and recommended action without
reading configuration or process files.

## Regression Scenarios

The implementation must cover these cases:

- A detached no-CDP `runtime login` browser appears as a manual observed browser
  with its known profile and display.
- A registered profile with no live browser exposes **Open browser**.
- A compatible retained browser exposes **Add tab** instead of launching a
  duplicate profile process.
- An incompatible holder exposes **Inspect holder** with the conflict reason.
- Search by hostname, alias, target service, login, account label, profile name,
  and tag returns deterministic ranked results.
- Exact X lookup selects `last30days-facebook` when its
  `authenticatedServiceIds` contains `x`.
- A missing match returns `not_found` instead of selecting the default profile.
- A launched browser appears in the left rail after the service records the
  browser, session, tab, and stream.
- A manual browser without CDP remains visible but does not expose unsupported
  automation actions.

## Relationship To Existing Work

This note deepens, rather than replaces, existing authorities:

- `docs/dev/notes/2026-05-23-left-pane-workspace-navigator-campaign.md`
  already requires a guided browser and profile launcher.
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
  defines safe retained-browser reuse and one process per profile.
- `docs/dev/plans/0072-2026-07-09-workspace-inventory-plan.md` separates
  inventory placement from workspace-node construction.
- `docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md` records the
  earlier failure to honor an explicit `last30days-facebook` selection.

The next bounded plan must connect these authorities. It must not treat profile
search, profile launch, manual-browser discovery, and left-rail placement as
unrelated UI patches.

## Next Step

Create one implementation plan with four coordinated slices:

- searchable profile and website/login catalog contracts;
- CLI, HTTP, MCP, and generated-client lookup parity;
- profile-first dashboard launch actions;
- manual no-CDP browser discovery and workspace inventory projection.

The plan must include no-launch contract tests, deterministic ranking fixtures,
dashboard interaction tests, and a live RDP/Guacamole proof using a disposable
profile before release.
