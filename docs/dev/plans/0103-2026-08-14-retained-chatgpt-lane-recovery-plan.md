# Plan 0103 | Retained ChatGPT Lane Recovery

State: CLOSED
Roadmap: P103
Plan version: 1
Date: 2026-08-14

## Objective

Restore the exact `chatgpt-pro` browser profile and AuraCall session after its
browser process exited, without creating a duplicate profile lane or sending a
ChatGPT prompt.

## Scope

- reconcile the two daemons advertising the same session name;
- keep the WSL-hosted authenticated profile on its compatible Linux Chrome
  executable;
- relaunch the exact retained conversation URL once;
- verify PID, CDP endpoint, profile lock, canonical URL, rendered title, login
  posture, service state, and AuraCall broker health;
- leave the browser retained and prompt-free.

## Non-Goals

- no profile migration from WSL storage into a Windows Chromium profile;
- no ChatGPT prompt, composer action, login, or conversation creation;
- no GitHub write or publication;
- no unrelated daemon or profile cleanup.

## Acceptance Criteria

1. Exactly one current agent-browser daemon owns session
   `auracall-chatgpt-broker-v7` in the authoritative socket directory.
2. Exactly one Chrome main process owns the configured `chatgpt-pro` user-data
   directory and exposes one reachable loopback DevTools browser endpoint.
3. The retained target resolves to the configured canonical conversation URL
   and renders without a login redirect.
4. AuraCall remains bridge-required and can acquire the existing broker lane
   without launching another browser or sending a prompt.
5. The repaired lane survives a bounded health recheck and no profile lock or
   disposable test artifact remains ambiguous.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: the authenticated browser exited at `2026-08-14T20:48:18Z`. An
  old deleted-binary daemon still listens in `/run/user/1000`, while a current
  daemon with Windows Chromium launch authority listens under
  `~/.agent-browser`. The retained profile is WSL-hosted and explicitly bound
  by AuraCall to Linux Chrome, so the current daemon cannot safely recover it
  through its Windows executable default.
- subagent_status: not_spawned; active system policy prohibits proactive
  delegation.
- authority_classification: explicit user direction to fix the retained lane.
- next_action_or_stop_reason: retire only the stale no-browser daemon, hand the
  authoritative session to the configured Linux executable, and relaunch the
  exact retained target once.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: verified_completion
- evidence: the stale deleted-binary daemon was retired, the authenticated
  profile was relaunched once with its configured Linux Chrome, and exact
  handoff moved the current daemon to the authoritative home socket without
  replacing browser PID `1046742` or target
  `B0EC77F279E5434E33FEA97AB1742B1A`.
- integration: AuraCall now discovers the retained home-directory stream. One
  `cdp_attach` and one verified `cdp_detach` reused the exact handle; its API
  service restart preserved browser PID, target, canonical URL, and ready
  health.
- safety: no ChatGPT prompt, composer action, login mutation, duplicate browser,
  temporary profile, or GitHub write occurred.
- material_blockers: none.
- next_action_or_stop_reason: stop; the retained lane and broker discovery are
  both verified.
