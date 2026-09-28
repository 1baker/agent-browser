# Plan 0098 | WSL Windows DevTools Loopback Relay

State: CLOSED
Roadmap: P98
Plan version: 1
Date: 2026-08-14

## Objective

Keep WSL in NAT networking mode while allowing an agent-browser-owned Windows
Chromium process to expose DevTools only through a private WSL loopback relay.
The relay must not require SSH, a firewall rule, mirrored networking, a
persistent shell, or a Byobu tab.

## Architecture

- Work access remains work client to Tailscale to the home-network Bastion to
  the desktop through the existing SSH jump route.
- Browser control remains local to the desktop: WSL launches the reviewed
  Windows Chromium build and starts one WSL loopback relay for its exact
  DevTools endpoint.
- The relay connects each accepted WSL loopback client to Windows loopback
  through WSL interoperability and never listens on LAN or Tailscale addresses.
- The relay watches the exact Windows browser PID, survives daemon executable
  handoff, and exits when that browser exits. Agent-browser close stops the
  exact Windows browser and the relay.

## Scope

- detect the exact Windows browser main process from the translated profile;
- start a dedicated loopback relay child and publish its local endpoint;
- make process health and shutdown use the relay plus exact Windows identity;
- fail closed on missing or ambiguous browser identity, relay startup failure,
  or endpoint mismatch;
- add focused unit and live disposable-profile coverage;
- update required help and operator documentation surfaces.

## Non-Goals

- no `.wslconfig`, Windows firewall, Hyper-V firewall, SSH server, Tailscale,
  Bastion, router, Authelia, or public ingress mutation;
- no VPS route and no use of retired host `srv1635328`;
- no persistent terminal, tmux, or Byobu helper tab;
- no prompt submission or authenticated-site mutation during validation;
- no GitHub write or publication.

## Acceptance Criteria

1. A WSL NAT launch of Windows Chromium returns a reachable WSL-loopback CDP
   endpoint while Windows Chromium remains bound to Windows loopback.
2. The relay binds only `127.0.0.1`, targets only Windows `127.0.0.1`, and is
   tied to one exact browser PID and profile identity.
3. Missing or multiple exact main-process matches fail before browser control.
4. Browser close terminates the exact owned Windows browser and relay without
   broad Chrome process cleanup.
5. Daemon executable handoff does not terminate the relay or retained browser.
6. Focused tests, format, strict Clippy, selected wider tests, docs build, plan
   audit, and diff checks pass.
7. Installed-runtime proof uses a disposable profile first, followed by a
   no-prompt readiness check against the retained exact LitScout target.

## Execution Receipt C01

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_started
- evidence: Windows Chromium advertised DevTools on Windows
  `127.0.0.1:64888`, while WSL NAT could not reach that loopback listener;
  Windows SSH was not available on a suitable local interface.
- subagent_status: none; the dirty Chrome lifecycle slice is tightly coupled
  and parallel edits would create more coordination risk than useful speed.
- authority_classification: explicit user authorization for the recommended
  next engineering step.
- review_disposition_summary: implement an agent-browser-owned WSL loopback
  relay instead of changing WSL networking or creating an SSH tunnel.
- next_action_or_stop_reason: add the fail-closed relay and focused tests.

## Execution Receipt C02

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: implementation_and_live_validation_complete
- evidence: a disposable headed Windows Chromium profile was reachable through
  WSL loopback and closed exactly. Runtime executable handoff preserved relay
  PID 255174, browser identity, CDP endpoint `127.0.0.1:37943`, target, and
  title across daemon replacement, then exact close removed the browser and
  relay.
- subagent_status: none; implementation remained in the tightly coupled dirty
  Chrome lifecycle slice.
- authority_classification: inherited explicit implementation authority.
- review_disposition_summary: relay transport was changed from buffered async
  copying to two synchronous binary pumps after the first disposable smoke
  exposed a transport stall; the two disposable profiles were cleaned by exact
  PID/profile identity.
- next_action_or_stop_reason: complete widened validation, publish locally, and
  perform the bounded retained-target no-prompt readiness check.

## Execution Receipt C03

- plan_version: 1
- state_transition: OPEN -> CLOSED
- progress_classification: outcome_complete_with_separate_broker_readiness_limit
- evidence: 1,866 Rust tests, focused WSL path coverage, disposable live smoke,
  Windows executable-handoff smoke, formatting, strict Clippy, dashboard
  build/typecheck, docs build, release build, and patch checks passed. Local
  publication converged executable, reference, and live dashboard manifest to
  SHA-256 `0d1b1a771d57e2a084d10bcf5e74a47194dcf206af3b7f2c6dc8c428475f66bc`.
- installed_readiness: retained LitScout browser PID 184301 and target
  `F203CCD6CC8B212B3B55D16368150AEB` remained healthy at exact URL
  `https://chatgpt.com/c/6a7e0704-214c-83ea-8e85-79b3750ae6c5`.
  A read-only broker attach attempt returned HTTP 502 before admission; the
  task audit found zero matching `cdp_attach` jobs, so no detach was due. It
  was not retried and no prompt was sent.
- doctor_disposition: launch configuration and live dashboard parity are ready.
  The overall doctor remains non-success only for a stale source-free
  workstation payload hash and optional privileged RDP-helper prerequisites;
  neither controls this local relay.
- plan_audit: this checkout does not define a `plans:audit` package script;
  roadmap, runbook, plan, validation note, and patch checks were audited
  directly.
- subagent_status: none.
- authority_classification: inherited explicit implementation authority.
- review_disposition_summary: all relay acceptance criteria passed. The broker
  502 is retained as a separate fail-closed service-readiness issue rather than
  weakening or replaying exact-target attach.
- next_action_or_stop_reason: stop; diagnose the pre-admission broker 502 under
  separate authority before another retained-target attach.

## Done Definition

- all acceptance criteria have current evidence;
- dirty user work remains preserved;
- installed binary matches the validated candidate;
- no browser prompt was sent and no retired VPS dependency remains.
