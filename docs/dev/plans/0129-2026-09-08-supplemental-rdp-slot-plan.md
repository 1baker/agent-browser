# Supplemental RDP slot

Status: supplemental C provisioned and manual browser controllable in service inventory; operator Terms of Service and Google authentication pending. No site-authentication completion claim.

Original request: "you can make a new slot the google drive slot temporarily to test because sam.gov is being used at the moment"; approved implementation: "ok go".

Scope: add one supplemental route with an isolated account and Guacamole connection. Preserve canonical A/B, active desktops, browser profiles, tabs, ingress and account authentication. Normal bounded engineering path: additive local controller, no architecture migration, release, new network exposure or browser authority replacement.

Acceptance: dry-run performs no writes; reserved A/B cannot be targeted; duplicate or interrupted identities fail closed; protected credentials never appear in output; backup before apply; no service restart; exact new desktop and managed browser must be independently ready before handing off. User completes Google authentication; fresh bounded probe is required afterward.

Non-goals: generalize the workstation installer, publish a release, rotate credentials of existing users, park routes, move browser processes, or automate Google credentials.

Implementation choice: keep the canonical installer unchanged. It requires exactly A/B and quiesces shared units. The service route-pool refresh merges entries without removing absent supplemental entries. Add only a new entry after its own readiness gate; never reproject A/B for this operation.

Validation: pure SQL/input tests plus isolated process fixtures, read-only doctor, additive dry-run, one reviewed live apply. Stop on permission failure, backup failure, existing identity or failed isolation. Partial provisioning retains its private journal for explicit recovery; no automatic deletion or overwriting. Rollback is a separately reviewed removal of only the recorded connection/account after proving no active users; never restore the whole database over current work.

Delegation: spawned `/root/slot_test_audit` for independent read-only discovery/review. Primary owns all implementation and live actions. Existing dirty edits in README, output, workstation installer, skill, dashboard docs, retained-browser library/tests are preserved.

Execution: added a checkout-only additive provisioner and explicit C-Z selectors for opener/inspector/readiness. Default installer and readiness remain A/B. Live apply created only C, connection 3, isolated display :12. The installed helper granted only the current operator access to that display. Only readiness-verified C was projected. A service checkout bound the live manual runtime browser to C; service projection reports remoteControlAvailable true. A/B browser PIDs, route IDs, allocations, displays and readiness matched the pre-apply baseline after checkout. No services were restarted. The database backup gate passed before provisioning.

Human boundary: Chrome first-run Terms of Service screen is visible on C. The temporary Google runtime was launched by agent-browser without DevTools and without disabling its sandbox. No Terms were accepted, credentials entered, authentication state exported, or private documents read. After the operator's login, closing and relaunching the same profile through the broker is necessary for bounded automated verification.

Review ledger: TEMP-01 (blocking) found pre-backup absence checks could go stale. Added a post-backup recheck under the provisioner lock and a late-created-user fixture; tests pass. The existing helper has no atomic create-only operation, so unrelated privileged provisioning during the narrow check/helper interval remains a documented limitation. There was no competing C provisioning during this controlled apply. No private journal or credential is included in this note. Partial failures remain fail-closed and require explicit recovery rather than retries.

Primary validation: SQL/input tests and process fixtures; reserved labels, collision failures, late user, backup failure, secret redaction, interrupted journal and singleton-C selection passed. Existing display-selection, route-xsession and route-specific-user-sync tests passed. ESLint for changed scripts, cargo fmt check, cargo clippy with warnings denied, and git diff check passed. Prettier is not installed; no dependencies were added. Source changes are not a released or replaced installed binary. The live supplemental helpers ran from this reviewed checkout, while the existing installer payload was preserved.

Follow-up: operator should select google-drive-temporary-c under Detected in the protected dashboard, review Terms, and sign in. Phone/remote Control on this new slot and post-login agent verification remain operator-dependent. Temporary removal, automatic notification handling and full runtime packaging remain outside this completed provisioning slice.

Closed-world review: TEMP-02 found that C metadata could contain a stale A viewer URL. Added decoded Guacamole client identity validation for every supplied URL role and required a loopback local viewer. No-launch mismatch fixtures now reject those inputs. The actual live C URL encoded connection 3 throughout; no A/B reassignment occurred. Both reviewer rounds completed under `/root/slot_test_audit`; the primary reproduced and fixed both accepted findings. Supplemental route selectors and explicit operator permission checks retain the default A/B contract.
