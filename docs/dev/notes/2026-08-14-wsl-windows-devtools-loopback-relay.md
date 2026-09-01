# WSL Windows DevTools Loopback Relay Validation

Date: 2026-08-14
Plan: P98 / Plan 0098

## Outcome

Agent-browser can retain Windows Chromium DevTools access while WSL remains in
NAT mode. A dedicated child binds only WSL `127.0.0.1` and forwards binary CDP
traffic to Windows `127.0.0.1` through WSL interoperability. It is tied to the
exact Windows main-process PID and translated profile, survives daemon
executable handoff, and exits when that browser exits.

This path requires no mirrored networking, Windows or Hyper-V firewall rule,
SSH tunnel, long-lived login shell, tmux, or Byobu helper tab. Remote shell
access remains the separate stateless path through Tailscale to the
home-network Bastion and then the desktop.

## Source Validation

- `pnpm test:wsl-windows-chromium-profile-paths`: passed.
- `bash scripts/smoke-wsl-windows-chromium-profile.sh`: headed launch, title
  readback, exact profile/process close, and cleanup passed.
- parameterized `scripts/smoke-runtime-executable-handoff.js`: daemon PID
  changed while relay PID 255174, CDP endpoint `127.0.0.1:37943`, target, and
  title remained stable; final exact close passed.
- `cargo fmt --check`: passed.
- `cargo clippy -- -D warnings`: passed.
- full Rust suite: 1,866 passed.
- dashboard build/typecheck, documentation build, release build, and
  `git diff --check`: passed.
- package plan audit command: unavailable because this checkout has no
  `plans:audit` script; planning surfaces were checked directly.

## Installed Runtime

Local publication converged the installed executable, reference, and live
dashboard manifest to SHA-256
`0d1b1a771d57e2a084d10bcf5e74a47194dcf206af3b7f2c6dc8c428475f66bc`.
Dashboard runtime smoke passed after an exact stale-listener cleanup.

Install doctor reports the launch configuration and Windows Chromium manifest
ready. Its aggregate status remains non-success because an older source-free
workstation payload records a prior executable hash and the optional
privileged RDP helper is not installed. Those are outside the loopback relay
and were not mutated.

## Retained Target Safety Check

The retained broker-owned browser remained healthy on PID 184301. The exact
LitScout target was `F203CCD6CC8B212B3B55D16368150AEB`, titled `LitScout
Workflow Review`, at
`https://chatgpt.com/c/6a7e0704-214c-83ea-8e85-79b3750ae6c5`.

A no-prompt read-only broker attach request returned HTTP 502 before operation
admission. The matching task/job audit found zero committed `cdp_attach` jobs,
so no detach obligation existed. The request was not retried. No prompt was
sent, no tab was navigated or closed, and the retained browser was preserved.

## SSH Retirement

The active `srv1635328` SSH block, four exact authorized keys, and host-key
records were removed. Active SSH files contain no remaining reference to that
retired VPS. The `bastion` and `desktop-gdjcaft-via-bastion` Tailscale routes
and unrelated device keys remain intact.
