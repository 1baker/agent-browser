//! Deterministic, site-scoped page-gate classification and exact-tab state.
//! A classifier result is evidence, not proof of login or task completion.

use super::service_model::{
    BrowserTab, Challenge, ChallengeKind, ChallengeState, PageGateObserverPolicy, ServiceState,
    ServiceTabHandle, TabLifecycle,
};

pub fn waiting_page_gate_tab<'a>(
    state: &'a ServiceState,
    session_name: &str,
    target_id: &str,
) -> Option<&'a BrowserTab> {
    state.tabs.values().find(|tab| {
        tab.target_id.as_deref() == Some(target_id)
            && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                handle.valid
                    && handle.session_name.as_deref() == Some(session_name)
                    && handle.target_id.as_deref() == Some(target_id)
                    && handle.browser_id == tab.browser_id
                    && handle.tab_id == tab.id
            })
            && tab.challenge_id.as_ref().is_some_and(|id| {
                id.starts_with("page-gate:")
                    && state
                        .challenges
                        .get(id)
                        .is_some_and(|challenge| challenge.state == ChallengeState::WaitingForHuman)
            })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageGateDisposition {
    Clear,
    Challenge,
    SignInRequired,
    Unknown,
}

impl PageGateDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Challenge => "challenge",
            Self::SignInRequired => "signin_required",
            Self::Unknown => "unknown",
        }
    }
}

pub fn validate_policy(policy: &PageGateObserverPolicy) -> Result<(), String> {
    for (name, values, max_len) in [
        (
            "challengeTitleContains",
            &policy.challenge_title_contains,
            120,
        ),
        ("challengeSelectors", &policy.challenge_selectors, 256),
        ("signInUrlPrefixes", &policy.sign_in_url_prefixes, 512),
        ("readySelectors", &policy.ready_selectors, 256),
    ] {
        if values.len() > 5
            || values
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > max_len)
        {
            return Err(format!("page gate policy {name} exceeds bounded limits"));
        }
    }
    if policy
        .sign_in_url_prefixes
        .iter()
        .any(|prefix| !prefix.starts_with("https://"))
    {
        return Err("page gate signInUrlPrefixes must use HTTPS".to_string());
    }
    Ok(())
}

pub fn classify_page_gate(
    policy: &PageGateObserverPolicy,
    url: &str,
    title: &str,
    challenge_selector_visible: bool,
    ready_selector_visible: bool,
) -> PageGateDisposition {
    let title_matches = !policy.challenge_title_contains.is_empty()
        && policy
            .challenge_title_contains
            .iter()
            .any(|fragment| title.to_lowercase().contains(&fragment.to_lowercase()));
    let challenge_matches = (title_matches || policy.challenge_title_contains.is_empty())
        && (challenge_selector_visible || policy.challenge_selectors.is_empty())
        && (!policy.challenge_title_contains.is_empty() || !policy.challenge_selectors.is_empty());
    if challenge_matches {
        return PageGateDisposition::Challenge;
    }
    if policy
        .sign_in_url_prefixes
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        return PageGateDisposition::SignInRequired;
    }
    if ready_selector_visible && !policy.ready_selectors.is_empty() && !challenge_selector_visible {
        return PageGateDisposition::Clear;
    }
    PageGateDisposition::Unknown
}

/// Persist only a positive gate or positive ready-page recheck for the exact
/// live tab. Unknown observations never resolve or replace a human handoff.
pub fn apply_page_gate_observation(
    state: &mut ServiceState,
    handle: &ServiceTabHandle,
    disposition: PageGateDisposition,
    observed_at: &str,
) -> Result<Option<String>, String> {
    let tab = state
        .tabs
        .get_mut(&handle.tab_id)
        .ok_or("page gate tab not retained")?;
    let matches_live_target = matches!(
        tab.lifecycle,
        TabLifecycle::Opening | TabLifecycle::Loading | TabLifecycle::Ready
    ) && handle.valid
        && tab.browser_id == handle.browser_id
        && tab.target_id == handle.target_id
        && tab.service_tab_handle.as_ref().is_some_and(|retained| {
            retained.valid
                && retained.tab_id == handle.tab_id
                && retained.browser_id == handle.browser_id
                && retained.target_id == handle.target_id
                && retained.profile_id == handle.profile_id
                && retained.session_name == handle.session_name
        });
    if !matches_live_target {
        return Err("page gate observation refused stale or changed exact tab".to_string());
    }
    let challenge_id = format!("page-gate:{}", handle.tab_id);
    if tab
        .challenge_id
        .as_deref()
        .is_some_and(|id| id != challenge_id)
    {
        return Err("page gate observation refuses another challenge owner".to_string());
    }
    match disposition {
        PageGateDisposition::Unknown => Ok(None),
        PageGateDisposition::Clear => {
            if let Some(challenge) = state.challenges.get_mut(&challenge_id) {
                if challenge.state == ChallengeState::WaitingForHuman {
                    challenge.state = ChallengeState::Resolved;
                    challenge.result = Some("ready_page_observed_after_human_handoff".to_string());
                    return Ok(Some(challenge_id));
                }
            }
            Ok(None)
        }
        PageGateDisposition::Challenge | PageGateDisposition::SignInRequired => {
            state.challenges.insert(
                challenge_id.clone(),
                Challenge {
                    id: challenge_id.clone(),
                    tab_id: Some(handle.tab_id.clone()),
                    kind: if disposition == PageGateDisposition::Challenge {
                        ChallengeKind::BlockedFlow
                    } else {
                        ChallengeKind::Unknown
                    },
                    state: ChallengeState::WaitingForHuman,
                    detected_at: Some(observed_at.to_string()),
                    provider_id: None,
                    policy_decision: Some(disposition.as_str().to_string()),
                    human_approved: false,
                    result: None,
                },
            );
            tab.challenge_id = Some(challenge_id.clone());
            Ok(Some(challenge_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::service_model::BrowserTab;
    use super::*;

    #[test]
    fn title_requires_configured_selector_and_ready_requires_positive_proof() {
        let policy = PageGateObserverPolicy {
            challenge_title_contains: vec!["Just a moment".to_string()],
            challenge_selectors: vec!["#challenge".to_string()],
            ready_selectors: vec!["article h1".to_string()],
            ..Default::default()
        };
        assert_eq!(
            classify_page_gate(
                &policy,
                "https://example.org/a",
                "Just a moment in science",
                false,
                false
            ),
            PageGateDisposition::Unknown
        );
        assert_eq!(
            classify_page_gate(
                &policy,
                "https://example.org/a",
                "Just a moment",
                true,
                true
            ),
            PageGateDisposition::Challenge
        );
        assert_eq!(
            classify_page_gate(&policy, "https://example.org/a", "Article", false, true),
            PageGateDisposition::Clear
        );
        assert_eq!(
            classify_page_gate(&policy, "https://example.org/a", "Article", true, true),
            PageGateDisposition::Unknown
        );
        assert_eq!(
            classify_page_gate(&policy, "https://example.org/a", "Article", false, false),
            PageGateDisposition::Unknown
        );
    }

    #[test]
    fn observer_policy_rejects_unbounded_or_insecure_sign_in_rules() {
        let mut policy = PageGateObserverPolicy {
            sign_in_url_prefixes: vec!["http://example.org/login".to_string()],
            ..Default::default()
        };
        assert!(validate_policy(&policy).is_err());
        policy.sign_in_url_prefixes = vec!["https://example.org/login".to_string()];
        policy.challenge_selectors = vec!["x".repeat(257)];
        assert!(validate_policy(&policy).is_err());
        policy.challenge_selectors.clear();
        assert!(validate_policy(&policy).is_ok());
        assert_eq!(
            classify_page_gate(
                &policy,
                "https://example.org/login/start",
                "Account",
                false,
                false
            ),
            PageGateDisposition::SignInRequired
        );
    }

    #[test]
    fn exact_tab_gate_survives_state_round_trip_and_unknown_does_not_resolve() {
        let handle = ServiceTabHandle {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            tab_id: "tab-1".to_string(),
            target_id: Some("target-1".to_string()),
            valid: true,
            ..Default::default()
        };
        let mut state = ServiceState::default();
        state.tabs.insert(
            "tab-1".to_string(),
            BrowserTab {
                id: "tab-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("target-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                service_tab_handle: Some(handle.clone()),
                ..Default::default()
            },
        );
        let id = apply_page_gate_observation(
            &mut state,
            &handle,
            PageGateDisposition::Challenge,
            "2026-10-02T00:00:00Z",
        )
        .unwrap()
        .unwrap();
        let mut retained: ServiceState =
            serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
        apply_page_gate_observation(
            &mut retained,
            &handle,
            PageGateDisposition::Unknown,
            "later",
        )
        .unwrap();
        assert_eq!(
            retained.challenges[&id].state,
            ChallengeState::WaitingForHuman
        );
        assert_eq!(
            waiting_page_gate_tab(&retained, "session-1", "target-1").map(|tab| tab.id.as_str()),
            Some("tab-1")
        );
        assert!(waiting_page_gate_tab(&retained, "other-session", "target-1").is_none());
        assert!(waiting_page_gate_tab(&retained, "session-1", "other-target").is_none());
        let mut stale = handle.clone();
        stale.target_id = Some("other".to_string());
        assert!(apply_page_gate_observation(
            &mut retained,
            &stale,
            PageGateDisposition::Clear,
            "later"
        )
        .is_err());
        apply_page_gate_observation(&mut retained, &handle, PageGateDisposition::Clear, "later")
            .unwrap();
        assert_eq!(retained.challenges[&id].state, ChallengeState::Resolved);
        assert!(waiting_page_gate_tab(&retained, "session-1", "target-1").is_none());
    }
}
