# Guarded Retained Lane Preparation

Date: 2026-08-15
Plan: P122

## Finding

P121 can uniquely discover and pin a selected retained target, but it still
requires an operator to navigate the managed browser first. That manual handoff
is easy to perform in the ambient browser instead of the Guacamole route, which
leaves the managed profile on a blank tab and produces repeated no-match
results.

## Resolution

The new preparation command accepts both an exact canonical URL and a reviewed
origin and path prefix. It opens that URL through route-bound `remote-view
open`, verifies operator-visible readiness and exact URL, profile, target,
browser, and session agreement, then invokes exact-URL discovery and the
existing digest-bound retained requirement writer.

The command does not accept or invoke click, type, fill, evaluate, upload,
send, submit, or prompt actions. Wrong URL, wrong profile, incomplete identity,
zero matches, and multiple matches fail closed. A failed pin intentionally
preserves the managed browser for inspection instead of adding a hidden cleanup
action that could destroy authenticated state.

## Verification Status

Focused discovery, preparation-contract, and isolated command-orchestration
fixtures pass. The command-level fixture records exactly two child operations:
route-bound open and retained requirement pin. Live ChatGPT execution remains
deferred because the active operator boundary forbids automatic ChatGPT
navigation and prompt submission.

Retained guard, requirement, watchdog, publisher operation, publisher
orchestration, service-client typecheck, dashboard and docs production build,
runtime convergence, skill parity, JavaScript syntax, and diff hygiene gates
also pass. Full docs lint remains red on an unrelated existing synchronous
state update in `docs/src/components/theme-toggle.tsx`; the production docs
TypeScript build passes.

The preparation workflow is now source-free. The installed workstation payload
embeds the controller plus exact discovery, guard, live reverification,
preparation, and requirement libraries. The native installed command forwards
the reviewed URL, boundary, and profile to that controller while binding the
installed binary. An isolated fresh-install fixture verifies hashes and runs
both installed-command and controller help without the checkout.

The prior docs lint blocker is also repaired. Full lint and production build
now pass. The optimized candidate SHA-256 is
`954a01d3783912a129a1a23f6dc6c606dc455ce1991d380a64b0a898d2f29389`;
it remains uninstalled until the exact retained lane can be created and guarded.

The first real localhost preparation smoke stopped safely before launch because
both Guacamole routes were occupied by live retained browsers:
`away-auth-handoff` on Route A and `nyse-developer-route` on Route B. The smoke
created no browser or authority file and did not park or displace either route.
It remains as a repeatable gate for the next free route window. Preparation now
preserves the bounded structured child error so this condition is visible to
operators instead of appearing as an opaque failure.

Because that diagnostic is embedded in the source-free support payload, the
optimized candidate was relinked. Its current SHA-256 is
`fc4fd837feeadbe9da3ffa9c2bab903a4af64005ad2504330e4e46498675bf0e`.
The installed binary remains unchanged.

Route A was subsequently parked with the explicit route-switch policy after
verifying that it had no viewer or controller lease. Its retained browser and
target remained alive. Live testing exposed and repaired two deterministic
route issues: canonical route-pool rows now include distinct stable display
allocation ids, and checkout accepts only the exact same-owner acquisition
reservation created by its planning phase. Preparation also binds the daemon
session to the runtime-profile id instead of reusing an ambient default lane.

With those fixes, the disposable localhost preparation completed and cleaned
up successfully. The exact Workshop conversation was then opened through the
`chatgpt-pro` Route A lane, but ChatGPT redirected it to the project landing
page. The exact-URL guard returned `retained_target_url_changed`, wrote neither
authority file, and invoked no prompt action. The live browser is preserved for
inspection. Publication remains blocked until a real Workshop conversation URL
exists. The optimized candidate SHA-256 is
`2bbfe6d53e7553424e88773d67f222f0105d2dc791fe67d5098b2022bd8a3e88`;
the installed binary remains unchanged.

## Live Closure

The operator later authorized exactly one short prompt solely to create a real
Workshop conversation. The retained Chrome process survived an initial daemon
metadata mismatch and was reattached through its existing DevTools endpoint;
no duplicate browser lane was launched. One prompt was typed and submitted
once from the empty Workshop project composer. It produced canonical URL
`https://chatgpt.com/g/g-p-6a7e016622e48191a60c4bc34366b537-codex-chatgpt-workshop/c/6a80e64e-e830-83ea-b21f-9079abf27a1d`.

Direct CDP readback established stable page target
`91DBB20C67DFB0398978722D6B6FA85A`, browser PID `1326080`, CDP port `38405`,
and profile/session `chatgpt-pro`. The default retained requirement already
protected the NYSE developer lane, so it was left unchanged. A separate
Workshop requirement and enforcement record were written mode `0600`; the
requirement SHA-256 is
`80d2f5883c47f7bc7ef7370ec0a5b32a11a468a6a668bd440baaa9df98c326f7`.

Guarded release publication completed with the Workshop-specific requirement.
Transaction `local-dashboard-5a1ca8ea-e8f6-4f91-88f7-2d4ea6b26d21` is terminal
`ready`, retained identity is verified at `final_readiness`, and all four
prepared daemon handoffs resumed. The release and installed binary SHA-256 are
both `7704b89a579e6bb1678d43cbe3d3ea402197a411525cab83e9f6a641228755bb`.
The exact retained target passes the no-build read-only check, runtime
convergence reports five current sessions and zero stale runtimes, the live
dashboard manifest is ready, and remote-view doctor reports ready. Install
doctor separately recommends refreshing the versioned source-free workstation
payload; that follow-up does not block the verified live runtime or P122
closure.
