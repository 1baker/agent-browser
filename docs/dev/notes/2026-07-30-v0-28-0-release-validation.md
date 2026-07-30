# v0.28.0 Release Validation

Date: 2026-07-30
Release: `v0.28.0`
Release URL:
`https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.28.0`

## Commit and CI Binding

- Release PR 7 candidate: `412684f661ef8b8837eb303fecad6be3f1d1d22a`
- PR merge commit: `80f64885419ee072263c7d5f22f57659f812048e`
- Release-workflow repair and published tag commit:
  `4132e78203f00db26b65456bde4bc36355714ae4`
- Candidate fast CI: `30552821524`
- Candidate manually dispatched full CI: `30553477964`
- Release-repair exact-head full CI: `30576313066`

The candidate fast and full CI runs passed. The release-repair commit also
passed the complete exact-head CI packet, including Linux, both macOS
architectures, Windows, native E2E, and global-install coverage.

## Release Workflow

The first dry run, `30575205599`, built all seven platform binaries but stopped
at the exact-asset verifier. The workflow had staged release binaries in the
repository `bin/` directory, where the tracked `agent-browser.js` file made the
inventory eight files instead of the expected seven.

One bounded remediation moved dry-run and publication staging into an isolated
`release-assets/` directory and added a regression guard. No product behavior
changed.

- Corrected dry run: `30578774481`, passed
- Publication run: `30579564702`, passed
- Published inventory: seven supported binaries plus `SHA256SUMS`
- Public tag target:
  `4132e78203f00db26b65456bde4bc36355714ae4`

## Public Asset Proof

The Linux x64 binary and `SHA256SUMS` were downloaded through their public
GitHub release URLs without using repository files or an authenticated GitHub
client.

- Binary: `agent-browser-linux-x64`
- Size: `21641080` bytes
- Version: `agent-browser 0.28.0`
- SHA-256:
  `4af2aba4e3670b2ffcd9601ab0134ad24cd13ec9e8131212f42a5645cb9baa22`
- The checksum matched both `SHA256SUMS` and GitHub's published asset digest.

The public binary was staged into the previously accepted disposable Ubuntu
24.04 clean-overlay VM. It completed an idempotent source-free
`install workstation --apply --json` reconciliation with:

- `success=true`
- `complete=true`
- `state=ready`
- `version=0.28.0`
- `sessionRefreshRequired=false`

The installed binary hash exactly matched the downloaded public binary.
Installed support metadata reported `sourceCheckoutRequired=false` and
`sourceFree=true`.

From a normal login environment:

- `agent-browser install doctor --json` returned success with no issues,
  converged runtime state, ready source-free payload, and version `0.28.0`.
- `agent-browser doctor remote-view --json` returned success and
  `status=ready`; remote control, install state, viewer prerequisites, and
  runtime convergence were ready.
- A route-open dry run selected route `guacamole:1`, connection `1`, and
  display `:10`. Its verification reported
  `browserLaunchRequested=false`, `routeCheckoutRequested=false`, and
  `tabOpenRequested=false`.

The remote-view doctor retained one advisory drift item for the deliberately
route-specific RDP users used by the accepted display-isolation topology. It
did not reduce readiness or require release remediation.

## Independent Judgment and Boundaries

Fresh evaluator `/root/p82_final_go_no_go` returned GO after independently
checking the exact PR head, CI evidence, synchronized version surfaces,
changelog binding, and release hard stops. The primary agent independently
verified the merge, release runs, public tag, checksums, installed hash,
doctors, and no-launch receipt.

The failed initial dry run consumed the single authorized remediation pass.
No open-ended retry loop followed. The operator-owned untracked `--full-page`
file remained excluded and untouched.

No Graphiti episode was written because this session did not include explicit
memory-write authorization. This note, Plan 0082, the roadmap, the runbook,
GitHub Actions, and the public release are the durable closeout authorities.
