//! Private execution's daemon-owned authority boundary. No caller-provided
//! serviceState, profile fallback, target switching, discovery or browser launch.
//! This preflight is not admission: the coordinator must revalidate while locked
//! and apply the top-level origin guard in the same evaluation as secret access.

use super::actions::DaemonState;
use super::privacy_gate::PrivatePermit;
use super::private_attachment::PrivateAttachment;
use super::private_identity::{
    validate_private_target, LivePrivateIdentity, ValidatedPrivateTarget,
};
use super::private_operation::PrivateOperation;
use super::service_model::{BrowserHealth, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use serde_json::Value;
use std::time::Duration;

/// Rebuild authority from the persisted broker and read the target from the
/// already-owned CDP page session. Error messages never echo caller or page data.
pub(crate) async fn prepare_private_target(
    state: &DaemonState,
    staged: &Value,
    expected_origin: &str,
    expected_url: &str,
) -> Result<ValidatedPrivateTarget, &'static str> {
    observe_private_target(state, staged, expected_origin, expected_url, None).await
}

/// Begin an identity-bound privacy interval and close the preflight/admission
/// race by repeating authority checks under its lock. No secret is dispatched.
/// Cancellation or failed revalidation leaves the durable interval closed for
/// explicit recovery; callers must never retry by launching a replacement.
pub(crate) async fn begin_authorized_private_interval(
    state: &DaemonState,
    staged: &Value,
    expected_origin: &str,
    expected_url: &str,
) -> Result<PrivatePermit, &'static str> {
    let target = prepare_private_target(state, staged, expected_origin, expected_url).await?;
    let permit = state
        .browser
        .as_ref()
        .ok_or("private_browser_unavailable")?
        .client
        .begin_private_interval(&target)?;
    revalidate_private_target(state, staged, expected_origin, expected_url, &permit).await?;
    Ok(permit)
}

/// Repeat broker and live-page validation inside the already-held private
/// interval, including recovery. Uses only private CDP transport: public
/// observation is revoked and cannot be temporarily reopened for validation.
/// This never changes the permit's original scope or unlocks on failure.
pub(crate) async fn revalidate_private_target(
    state: &DaemonState,
    staged: &Value,
    expected_origin: &str,
    expected_url: &str,
    permit: &PrivatePermit,
) -> Result<ValidatedPrivateTarget, &'static str> {
    let target =
        observe_private_target(state, staged, expected_origin, expected_url, Some(permit)).await?;
    validate_permit_scope(permit, &target)?;
    Ok(target)
}

fn validate_permit_scope(
    permit: &PrivatePermit,
    target: &ValidatedPrivateTarget,
) -> Result<(), &'static str> {
    if permit.target_id() != Some(target.target_id())
        || permit.identity_digest() != Some(target.scope_digest())
    {
        return Err("private_revalidation_scope_mismatch");
    }
    Ok(())
}

/// Advance only the next preauthorized destination, without reopening public
/// observations. The private epoch is committed before the broker URL update;
/// a crash between the two leaves recovery closed, never permission to replay.
pub(crate) async fn advance_private_target(
    state: &DaemonState,
    previous: &PrivateOperation,
    next: &PrivateOperation,
    expected_stage: usize,
    permit: &mut PrivatePermit,
    attachment: &PrivateAttachment,
) -> Result<(), &'static str> {
    let browser = state
        .browser
        .as_ref()
        .ok_or("private_browser_unavailable")?;
    let profile = state
        .private_runtime_profile()
        .ok_or("private_profile_unproven")?;
    let target = browser
        .active_target_id()
        .map_err(|_| "private_target_unavailable")?;
    let session = attachment.session_id(&browser.client, permit)?;
    let info = browser
        .client
        .send_private_command(
            permit,
            "Target.getTargetInfo",
            None,
            Some(session),
            Duration::from_secs(3),
        )
        .await?
        .into_value();
    if info.pointer("/targetInfo/targetId").and_then(Value::as_str) != Some(target)
        || info.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
    {
        return Err("private_identity_mismatch");
    }
    let live_url = info
        .pointer("/targetInfo/url")
        .and_then(Value::as_str)
        .ok_or("private_target_probe_failed")?;
    let browser_id = format!("session:{}", state.session_id);
    let live = LivePrivateIdentity {
        profile_id: profile,
        browser_id: &browser_id,
        session_name: &state.session_id,
        target_id: target,
        url: live_url,
        ready: true,
    };
    LockedServiceStateRepository::default_json()
        .map_err(|_| "private_authority_unavailable")?
        .mutate(|persisted| {
            advance_broker_snapshot(
                persisted,
                &live,
                browser.get_cdp_url(),
                previous,
                next,
                expected_stage,
                permit,
            )
            .map_err(str::to_owned)
        })
        .map_err(|_| "private_transition_failed")
}

fn advance_broker_snapshot(
    persisted: &mut ServiceState,
    live: &LivePrivateIdentity<'_>,
    endpoint: &str,
    previous: &PrivateOperation,
    next: &PrivateOperation,
    expected_stage: usize,
    permit: &mut PrivatePermit,
) -> Result<(), &'static str> {
    // This synthetic prior URL validates the already committed broker binding,
    // not the current page. The current page is independently checked below.
    let prior_live = LivePrivateIdentity {
        profile_id: live.profile_id,
        browser_id: live.browser_id,
        session_name: live.session_name,
        target_id: live.target_id,
        url: previous.expected_url(),
        ready: live.ready,
    };
    let prior = validate_broker_snapshot(
        persisted,
        previous.service_tab_handle(),
        &prior_live,
        endpoint,
        previous.expected_origin(),
        previous.expected_url(),
    )?;
    validate_permit_scope(permit, &prior)?;
    if previous.consent_sha256() != next.consent_sha256()
        || previous.account_scope() != next.account_scope()
    {
        return Err("private_transition_scope_mismatch");
    }
    let tab_id = previous
        .service_tab_handle()
        .get("tabId")
        .and_then(Value::as_str)
        .ok_or("private_identity_incomplete")?;
    let mut candidate = persisted.clone();
    candidate
        .tabs
        .get_mut(tab_id)
        .ok_or("private_authority_unavailable")?
        .url = Some(next.expected_url().to_owned());
    let fresh = validate_broker_snapshot(
        &candidate,
        next.service_tab_handle(),
        live,
        endpoint,
        next.expected_origin(),
        next.expected_url(),
    )?;
    permit.advance_target(expected_stage, &fresh)?;
    *persisted = candidate;
    Ok(())
}

async fn observe_private_target(
    state: &DaemonState,
    staged: &Value,
    expected_origin: &str,
    expected_url: &str,
    permit: Option<&PrivatePermit>,
) -> Result<ValidatedPrivateTarget, &'static str> {
    let browser = state
        .browser
        .as_ref()
        .ok_or("private_browser_unavailable")?;
    let profile = state
        .private_runtime_profile()
        .ok_or("private_profile_unproven")?;
    let target = browser
        .active_target_id()
        .map_err(|_| "private_target_unavailable")?;
    let session = browser
        .active_session_id()
        .map_err(|_| "private_target_unavailable")?;
    if staged.get("targetId").and_then(Value::as_str) != Some(target) {
        return Err("private_identity_mismatch");
    }
    let timeout = Duration::from_secs(3);
    let info = if let Some(permit) = permit {
        // Omit targetId so Chrome must identify the actual attached session.
        browser
            .client
            .send_private_command(permit, "Target.getTargetInfo", None, Some(session), timeout)
            .await
            .map_err(|_| "private_target_probe_failed")?
            .into_value()
    } else {
        tokio::time::timeout(
            timeout,
            browser.client.send_command_with_timeout(
                "Target.getTargetInfo",
                None,
                Some(session),
                timeout,
            ),
        )
        .await
        .map_err(|_| "private_target_probe_failed")?
        .map_err(|_| "private_target_probe_failed")?
    };
    let live_target = info
        .pointer("/targetInfo/targetId")
        .and_then(Value::as_str)
        .ok_or("private_target_probe_failed")?;
    let live_url = info
        .pointer("/targetInfo/url")
        .and_then(Value::as_str)
        .ok_or("private_target_probe_failed")?;
    if live_target != target
        || info.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
    {
        return Err("private_identity_mismatch");
    }
    // Deliberately bypass browser_capability_service_state(cmd), which permits
    // a supplied serviceState value for ordinary planning/testing paths.
    let persisted = LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .map_err(|_| "private_authority_unavailable")?;
    let browser_id = format!("session:{}", state.session_id);
    let live = LivePrivateIdentity {
        profile_id: profile,
        browser_id: &browser_id,
        session_name: &state.session_id,
        target_id: live_target,
        url: live_url,
        ready: true,
    };
    validate_broker_snapshot(
        &persisted,
        staged,
        &live,
        browser.get_cdp_url(),
        expected_origin,
        expected_url,
    )
}

pub(crate) fn validate_broker_snapshot(
    persisted: &ServiceState,
    staged: &Value,
    live: &LivePrivateIdentity<'_>,
    connected_endpoint: &str,
    expected_origin: &str,
    expected_url: &str,
) -> Result<ValidatedPrivateTarget, &'static str> {
    let tab_id = staged
        .get("tabId")
        .and_then(Value::as_str)
        .ok_or("private_identity_incomplete")?;
    let browser = persisted
        .browsers
        .get(live.browser_id)
        .ok_or("private_authority_unavailable")?;
    let session = persisted
        .sessions
        .get(live.session_name)
        .ok_or("private_authority_unavailable")?;
    if let Some(expiry) = &session.expires_at {
        let expiry = chrono::DateTime::parse_from_rfc3339(expiry)
            .map_err(|_| "private_lease_expiry_invalid")?;
        if expiry <= chrono::Utc::now() {
            return Err("private_lease_expired");
        }
    }
    let tab = persisted
        .tabs
        .get(tab_id)
        .ok_or("private_authority_unavailable")?;
    // Exact endpoints only. Alias equivalence requires separate trusted proof;
    // URL similarity or matching profile labels do not authorize a transport.
    if browser.health != BrowserHealth::Ready
        || browser.profile_id.as_deref() != Some(live.profile_id)
        || browser.cdp_endpoint.as_deref() != Some(connected_endpoint)
        || !browser
            .active_session_ids
            .iter()
            .any(|id| id == live.session_name)
        || session.profile_id.as_deref() != Some(live.profile_id)
        || !session.browser_ids.iter().any(|id| id == live.browser_id)
        || !session.tab_ids.iter().any(|id| id == tab_id)
        || tab.browser_id != live.browser_id
        || tab.owner_session_id.as_deref() != Some(live.session_name)
        || !persisted.profiles.contains_key(live.profile_id)
    {
        return Err("private_broker_identity_mismatch");
    }
    if persisted
        .tabs
        .values()
        .filter(|candidate| {
            candidate.browser_id == live.browser_id
                && candidate.target_id.as_deref() == Some(live.target_id)
        })
        .count()
        != 1
    {
        return Err("private_target_ambiguous");
    }
    let authoritative = persisted
        .service_tab_handle(tab_id)
        .ok_or("private_authority_unavailable")?;
    let authoritative =
        serde_json::to_value(authoritative).map_err(|_| "private_authority_unavailable")?;
    validate_private_target(staged, &authoritative, live, expected_origin, expected_url)
}

#[cfg(test)]
mod tests {
    use super::super::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, LeaseState, TabLifecycle,
    };
    use super::*;
    use serde_json::json;

    fn fixture() -> (ServiceState, Value) {
        let mut state = ServiceState::default();
        state.profiles.insert(
            "sam".into(),
            BrowserProfile {
                id: "sam".into(),
                ..Default::default()
            },
        );
        state.browsers.insert(
            "session:default".into(),
            BrowserProcess {
                id: "session:default".into(),
                profile_id: Some("sam".into()),
                health: BrowserHealth::Ready,
                cdp_endpoint: Some("ws://127.0.0.1:9222/devtools/browser/original".into()),
                active_session_ids: vec!["default".into()],
                ..Default::default()
            },
        );
        state.sessions.insert(
            "default".into(),
            BrowserSession {
                id: "default".into(),
                profile_id: Some("sam".into()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["session:default".into()],
                tab_ids: vec!["tab".into()],
                ..Default::default()
            },
        );
        state.tabs.insert(
            "tab".into(),
            BrowserTab {
                id: "tab".into(),
                browser_id: "session:default".into(),
                target_id: Some("retained".into()),
                lifecycle: TabLifecycle::Ready,
                url: Some("https://secure.login.gov/?request_id=approved".into()),
                owner_session_id: Some("default".into()),
                ..Default::default()
            },
        );
        let staged = serde_json::to_value(state.service_tab_handle("tab").unwrap()).unwrap();
        (state, staged)
    }

    fn validate(
        state: &ServiceState,
        staged: &Value,
        endpoint: &str,
        url: &str,
    ) -> Result<ValidatedPrivateTarget, &'static str> {
        let live = LivePrivateIdentity {
            profile_id: "sam",
            browser_id: "session:default",
            session_name: "default",
            target_id: "retained",
            url,
            ready: true,
        };
        validate_broker_snapshot(
            state,
            staged,
            &live,
            endpoint,
            "https://secure.login.gov",
            "https://secure.login.gov/?request_id=approved",
        )
    }

    fn operation(staged: &Value, url: &str, backup: bool) -> PrivateOperation {
        let mut handle = staged.clone();
        handle["url"] = json!(url);
        let operation = if backup {
            json!({"kind":"backup_code", "code_selector":"#code",
                "submit_selector":"#submit", "account_selector":"#account",
                "account_value":"fixture@example.test", "code":"synthetic-code",
                "source_scope":"synthetic-source:1"})
        } else {
            json!({"kind":"login", "email_selector":"#email",
                "password_selector":"#password", "submit_selector":"#submit",
                "email":"fixture@example.test", "password":"synthetic-password"})
        };
        PrivateOperation::parse(
            &serde_json::to_vec(&json!({
                "schema":"agent-browser.private-operation.v1", "service_tab_handle":handle,
                "expected_origin":"https://secure.login.gov", "expected_url":url,
                "consent_sha256":"a".repeat(64), "account_scope":"login.gov:fixture@example.test",
                "operation":operation,
            }))
            .unwrap(),
        )
        .unwrap()
    }

    #[cfg(unix)]
    struct JourneyFixture(std::path::PathBuf);

    #[cfg(unix)]
    impl JourneyFixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("ab-broker-journey-{}", uuid::Uuid::new_v4())))
        }
    }

    #[cfg(unix)]
    impl Drop for JourneyFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn journey_advances_exact_broker_url_without_reopening_or_replaying() {
        use super::super::{privacy_gate::PrivacyGate, private_identity::ValidatedPrivateJourney};
        let root = JourneyFixture::new();
        let gate = PrivacyGate::open(&root.0).unwrap();
        let (mut state, staged) = fixture();
        let endpoint = "ws://127.0.0.1:9222/devtools/browser/original";
        let initial_url = "https://secure.login.gov/?request_id=approved";
        let next_url = "https://secure.login.gov/login/two_factor/backup_code";
        let target = validate(&state, &staged, endpoint, initial_url).unwrap();
        let journey = ValidatedPrivateJourney::new(
            &target,
            &[
                ("https://secure.login.gov".into(), initial_url.into()),
                ("https://secure.login.gov".into(), next_url.into()),
            ],
        )
        .unwrap();
        let mut permit = gate.begin_private_journey(&journey).unwrap();
        let previous = operation(&staged, initial_url, false);
        let next = operation(&staged, next_url, true);
        let live = LivePrivateIdentity {
            profile_id: "sam",
            browser_id: "session:default",
            session_name: "default",
            target_id: "retained",
            url: next_url,
            ready: true,
        };
        advance_broker_snapshot(
            &mut state,
            &live,
            endpoint,
            &previous,
            &next,
            0,
            &mut permit,
        )
        .unwrap();
        assert_eq!(state.tabs["tab"].url.as_deref(), Some(next_url));
        assert_eq!(permit.journey_stage(), Some(1));
        assert!(gate.public_lease().is_err());
        let after = serde_json::to_value(&state).unwrap();
        assert!(advance_broker_snapshot(
            &mut state,
            &live,
            endpoint,
            &previous,
            &next,
            0,
            &mut permit
        )
        .is_err());
        assert_eq!(serde_json::to_value(&state).unwrap(), after);
        drop(permit);
        let resumed = gate.resume_private().unwrap();
        assert_eq!(resumed.journey_stage(), Some(1));
        assert!(gate.public_lease().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn journey_refuses_unapproved_destination_and_authority_drift_without_broker_writes() {
        use super::super::{privacy_gate::PrivacyGate, private_identity::ValidatedPrivateJourney};
        for change in [
            "url",
            "target",
            "profile",
            "session",
            "health",
            "owner",
            "destination",
        ] {
            let root = JourneyFixture::new();
            let gate = PrivacyGate::open(&root.0).unwrap();
            let (mut state, staged) = fixture();
            let endpoint = "ws://127.0.0.1:9222/devtools/browser/original";
            let initial_url = "https://secure.login.gov/?request_id=approved";
            let next_url = "https://secure.login.gov/login/two_factor/backup_code";
            let target = validate(&state, &staged, endpoint, initial_url).unwrap();
            let journey = ValidatedPrivateJourney::new(
                &target,
                &[
                    ("https://secure.login.gov".into(), initial_url.into()),
                    ("https://secure.login.gov".into(), next_url.into()),
                ],
            )
            .unwrap();
            let mut permit = gate.begin_private_journey(&journey).unwrap();
            let previous = operation(&staged, initial_url, false);
            let chosen = if change == "destination" {
                "https://secure.login.gov/unapproved"
            } else {
                next_url
            };
            let next = operation(&staged, chosen, true);
            let live = LivePrivateIdentity {
                profile_id: if change == "profile" { "other" } else { "sam" },
                browser_id: "session:default",
                session_name: if change == "session" {
                    "other"
                } else {
                    "default"
                },
                target_id: if change == "target" {
                    "other"
                } else {
                    "retained"
                },
                url: if change == "url" { initial_url } else { chosen },
                ready: change != "health",
            };
            if change == "owner" {
                state.tabs.get_mut("tab").unwrap().owner_session_id = Some("other".into());
            }
            let before = serde_json::to_value(&state).unwrap();
            assert!(
                advance_broker_snapshot(
                    &mut state,
                    &live,
                    endpoint,
                    &previous,
                    &next,
                    0,
                    &mut permit
                )
                .is_err(),
                "{change}"
            );
            assert_eq!(serde_json::to_value(&state).unwrap(), before, "{change}");
            drop(permit);
            let resumed = gate.resume_private().unwrap();
            assert_eq!(resumed.journey_stage(), Some(0), "{change}");
            assert!(gate.public_lease().is_err());
        }
    }

    #[test]
    fn exact_broker_authority_accepts_without_trusting_embedded_handle() {
        let (mut state, staged) = fixture();
        state.tabs.get_mut("tab").unwrap().service_tab_handle = Some(Default::default());
        assert!(validate(
            &state,
            &staged,
            "ws://127.0.0.1:9222/devtools/browser/original",
            "https://secure.login.gov/?request_id=approved"
        )
        .is_ok());
    }

    #[test]
    fn stale_or_fabricated_authority_fails_closed() {
        for change in [
            "profile",
            "session",
            "endpoint",
            "health",
            "duplicate",
            "url",
            "caller_state",
            "expired",
            "invalid_expiry",
        ] {
            let (mut state, mut staged) = fixture();
            let mut url = "https://secure.login.gov/?request_id=approved";
            match change {
                "profile" => {
                    state
                        .browsers
                        .get_mut("session:default")
                        .unwrap()
                        .profile_id = Some("other".into())
                }
                "session" => state
                    .sessions
                    .get_mut("default")
                    .unwrap()
                    .browser_ids
                    .clear(),
                "endpoint" => {
                    state
                        .browsers
                        .get_mut("session:default")
                        .unwrap()
                        .cdp_endpoint = Some("ws://localhost:9222/devtools/browser/original".into())
                }
                "health" => {
                    state.browsers.get_mut("session:default").unwrap().health =
                        BrowserHealth::NotStarted
                }
                "duplicate" => {
                    let mut second = state.tabs["tab"].clone();
                    second.id = "duplicate".into();
                    state.tabs.insert(second.id.clone(), second);
                }
                "url" => url = "https://secure.login.gov/?request_id=other",
                "caller_state" => {
                    staged["serviceState"] = json!(state);
                    state.browsers.clear();
                }
                "expired" => {
                    state.sessions.get_mut("default").unwrap().expires_at =
                        Some("2000-01-01T00:00:00Z".into())
                }
                "invalid_expiry" => {
                    state.sessions.get_mut("default").unwrap().expires_at =
                        Some("not-a-time".into())
                }
                _ => unreachable!(),
            }
            assert!(
                validate(
                    &state,
                    &staged,
                    "ws://127.0.0.1:9222/devtools/browser/original",
                    url
                )
                .is_err(),
                "{change}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn locked_revalidation_cannot_rebind_scope_and_failure_keeps_gate_closed() {
        use super::super::privacy_gate::PrivacyGate;
        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Fixture(
            std::env::temp_dir().join(format!("ab-private-broker-{}", uuid::Uuid::new_v4())),
        );
        let gate = PrivacyGate::open(&root.0).unwrap();
        let (state, staged) = fixture();
        let target = validate(
            &state,
            &staged,
            "ws://127.0.0.1:9222/devtools/browser/original",
            "https://secure.login.gov/?request_id=approved",
        )
        .unwrap();
        let permit = gate
            .begin_private_scoped(target.target_id(), target.scope_digest())
            .unwrap();
        assert!(validate_permit_scope(&permit, &target).is_ok());

        // Even internally consistent new observations cannot replace the URL
        // authorized by the original durable interval.
        let changed_url = "https://secure.login.gov/?request_id=replacement";
        let mut changed = staged.clone();
        changed["url"] = json!(changed_url);
        let live = LivePrivateIdentity {
            profile_id: "sam",
            browser_id: "session:default",
            session_name: "default",
            target_id: "retained",
            url: changed_url,
            ready: true,
        };
        let replacement = validate_private_target(
            &changed,
            &changed,
            &live,
            "https://secure.login.gov",
            changed_url,
        )
        .unwrap();
        assert_eq!(
            validate_permit_scope(&permit, &replacement),
            Err("private_revalidation_scope_mismatch")
        );
        assert!(!gate.observation_allowed());
        drop(permit);
        let restarted = gate.resume_private().unwrap();
        assert!(validate_permit_scope(&restarted, &target).is_ok());
        assert!(validate_permit_scope(&restarted, &replacement).is_err());
        assert!(!gate.observation_allowed());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn revalidation_without_owned_browser_fails_without_unlocking() {
        use super::super::privacy_gate::PrivacyGate;
        let root = std::env::temp_dir().join(format!("ab-private-broker-{}", uuid::Uuid::new_v4()));
        let gate = PrivacyGate::open(&root).unwrap();
        let permit = gate
            .begin_private_scoped("retained", &"a".repeat(64))
            .unwrap();
        let result = revalidate_private_target(
            &DaemonState::new(),
            &json!({}),
            "https://secure.login.gov",
            "https://secure.login.gov/",
            &permit,
        )
        .await;
        assert!(matches!(result, Err("private_browser_unavailable")));
        assert!(!gate.observation_allowed());
        drop(permit);
        drop(gate);
        std::fs::remove_dir_all(root).unwrap();
    }
}
