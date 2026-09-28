# Post-Timeout Browser Health Circuit

Date: 2026-08-14
Plan: P100 / Plan 0100

## Trigger

The P99 public-web benchmark showed that the WHATWG HTML Standard could exceed
the renderer deadline and leave the session damaged. Several later missions
could pass before a final CDP connection refusal, while the same final mission
passed alone. The original worker follow-up checked only `Browser.getVersion`,
which cannot prove that the active renderer remains usable.

## Contract

- return and persist the original timeout before recovery work;
- never replay the timed-out action;
- use browser-level CDP to separate browser loss from renderer damage;
- prepare a blank target before closing only the affected ordinary owned tab;
- when browser CDP is lost, skip polite close on that dead channel, terminate
  the exact locally owned process, and launch one blank replacement lane;
- preserve service-tab handles and externally attached browsers for explicit
  broker or operator reconciliation.

## Interim Evidence

- focused Rust tests cover exact target replacement ordering, one-target close,
  no circuit for service handles, and action classification;
- the first full live rerun after target quarantine reached the final Node.js
  mission successfully but delayed the immediate Python mission because the
  disconnected-browser path still attempted a polite CDP close;
- after exact-process cleanup replaced polite close, the frozen
  WHATWG/Python/Rust sequence produced the expected WHATWG failure at
  `about:blank`, then passed Python in 818 ms and Rust in 763 ms in the same
  session with exact cleanup.

## Final Evidence

- the full frozen run scored 9/10 in 63.159 seconds; WHATWG was the single
  bounded failure, then Python, Rust, Git, and Node all passed in the same
  session;
- the installed runtime repeated the critical WHATWG/Python pair and Python
  passed in 1.663 seconds after the timeout circuit;
- 1,811 Rust tests passed with 57 ignored, plus formatting, strict Clippy,
  focused timeout tests, docs TypeScript/build, route-confusion gates, release
  build, validation selection, patch checks, and installed dashboard smoke;
- the generic CDP-tab-streaming smoke remains blocked before navigation by its
  disposable Linux Chrome launch configuration under WSL, even with the Windows
  executable exported. It did not exercise the changed timeout path;
- installed, workspace, and reference binaries share SHA-256
  `6c6bcd338465639a3937b6a9c4c6f4a787b7e2847396cde451965603174968a8`;
- runtime handoff preserved retained browser PID 184301, profile `chatgpt-pro`,
  its CDP endpoint, and one target. Read-only verification found the current
  page titled `Architecture Review Boundaries` at
  `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`. No prompt was
  sent.

P99-F2 is closed. P99-F1 remains the next bounded implementation slice: a
precise-section or non-Runtime evidence path for monolithic standards pages.
