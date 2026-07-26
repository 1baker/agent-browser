# Registered Session Retained Browser Reconnect

Date: 2026-07-25

## Failure

The `last30days` X and LinkedIn adapters correctly selected the registered
`last30days-facebook` profile and its healthy retained browser, but their next
ordinary client command did not reconnect to that browser.

The failing sequence was:

1. `service access-plan` selected profile `last30days-facebook`.
2. Workspace acquisition returned browser
   `session:last30days-facebook` and session `last30days-facebook`.
3. The client sent `tab list` through that registered session.
4. The disconnected session daemon attempted to auto-launch the unrelated
   default runtime profile.
5. The launch collided with the live `auracall-corel` owner and returned
   `agent_browser_error`.

This was not an X authentication failure and not a Guacamole/RDP failure.
Service status still reported:

- browser `session:last30days-facebook`;
- profile `last30days-facebook`;
- browser health `ready`;
- host `remote_headed`;
- view provider `rdp_gateway`;
- display isolation `shared_display`;
- Guacamole route `guacamole:4`.

A diagnostic client attached explicitly to the retained CDP endpoint and
listed the existing `https://x.com/home` tab.

## Contract Gap

Shared-profile auto-attachment already covered acquisition actions such as
`open`, `navigate`, and `tab_new`. It did not cover a disconnected registered
session whose first ordinary browser command was a read such as `tab_list`.

That distinction broke the documented multi-client model. A service session
record and viable retained browser were present, but the named client session
could not resume control without an explicit low-level CDP attachment.

## Repair

Before ordinary-command auto-launch, the daemon now checks whether its own
registered service session already owns a live retained browser with a CDP
endpoint. When it does, the daemon reconnects to that endpoint and uses
detach-on-close semantics.

The resolver:

- accepts only browsers owned by the current registered session;
- honors an exact browser route hint when present;
- rejects cross-session route hints;
- requires live browser health, a profile identity, and a CDP endpoint;
- does not replace, close, or relaunch the retained browser;
- leaves acquisition actions on their existing fresh-tab path.

## Validation

Source validation:

- focused registered-session resolver tests passed;
- existing shared-profile resolver and acquisition-evidence tests passed;
- Rust format and clippy passed;
- CDP stream derivation tests passed;
- route-confusion no-launch gates passed;
- live service CDP tab-streaming smoke passed.

Installed validation:

- installed and built executable SHA:
  `dce721157bf414189587785b8bede7e2630fc979630a9299e7c88d39c1eb2a23`;
- the retained X browser PID and CDP endpoint remained unchanged;
- `install doctor` reported no issues and zero stale runtimes;
- the repaired registered session acquired
  `session:last30days-facebook` and evaluated the existing X tab without
  navigation;
- the auth readback returned `authenticated=true`, `login_form=false`,
  `checkpoint=false`, and `restricted=false` at `https://x.com/home`.

The local runtime publisher reported an unrelated handoff failure for
`auracall-corel` after its prepared handoff file disappeared. The installed
binary and dashboard restart completed, the X browser remained healthy, and a
subsequent doctor readback was successful. The duplicate
`auracall-corel` listener inventory remains separate operational follow-up; it
was not repaired in this slice.

