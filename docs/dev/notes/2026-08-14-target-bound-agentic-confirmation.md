# Target-Bound Agentic Confirmation

Date: 2026-08-14
Plan: P104 / Plan 0104

## Trigger

P101 recommended a deterministic retained-target task with explicit authority,
identity pinning, consequence classification, and a human boundary before
external mutation. The existing confirmation path stored only an action and raw
command, ignored the caller's confirmation ID, had no enforced expiry, could be
silently overwritten, and did not prove that approval still addressed the same
page.

## Contract

- every action has one stable consequence class;
- confirmation configuration accepts exact actions, consequence categories, or
  the `mutation` umbrella;
- a pending request includes an exact confirmation ID, consequence description,
  active target ID, active URL, and 60-second lifetime;
- confirm and deny require the exact ID and consume the request on mismatch or
  expiry;
- a second request cannot overwrite an unexpired approval;
- confirmation fails closed when the active target ID or URL differs;
- matching authority executes the pending command once, while read-only actions
  remain usable without confirmation when policy allows them.

## Verification

- five focused asynchronous regressions cover classification and denial, wrong
  ID consumption, expiry, target changes, overwrite prevention, and one-time
  matching execution;
- thirteen policy tests cover stable consequence mapping plus exact-action,
  category, and mutation-umbrella configuration;
- the serial Rust suite passed 1,824 tests with 57 ignored and zero failures;
- formatting, strict Clippy, docs TypeScript and production build, JavaScript
  syntax, release build, validation selection, and diff hygiene passed;
- the disposable public live smoke reads Example Domain, stops its outbound link
  as `external_mutation`, proves denial preserves the page, then changes to
  Example.org and proves the old approval cannot execute;
- rebuilt debug and installed runtime proofs both returned success with exact
  cleanup, no authenticated profile, no mutation, and no prompt.

Two disposable pre-build smoke attempts exposed the stale standalone debug
binary and followed the Example Domain link. They did not use authenticated
state and cleaned up their exact profiles and sessions. Rebuilding the debug
CLI corrected the test subject; both final proofs stopped before mutation.

## Installed Runtime

The optimized publisher installed and synchronized SHA-256
`0f57a4b060d68473d13b07155cd6fc502393124244d18def9340c5e7f083b468`.
Its handoff retained browser PID `1046742`, the same CDP endpoint, and one target
at `https://chatgpt.com/c/6a7f6bfc-61a8-83ea-82fc-59504c5f1bf2`, titled
`Architecture Review Boundaries`.

The initial publisher final check observed the old dashboard manifest because
the WSL user systemd bus was unavailable. The exact port-4848 listener was a
deleted-executable process, so only that listener was terminated and restarted.
The live manifest then reported the installed executable hash and dashboard
asset hash `3486d8ef88653784addecb62c07785c959a19fdf750ea36210d450ae5e8afdad`.
No ChatGPT prompt, composer action, login, form submission, or GitHub write
occurred.

The final install doctor confirms the current executable and workspace binary
share the new hash, the live dashboard is ready, and there are zero stale
runtimes. Its overall state remains partial for the separate source-free
workstation payload provenance and missing remote-view privileged helper. The
doctor marks that host-level repair as requiring interactive sudo; it was not
silently applied as part of this confirmation-policy slice.
