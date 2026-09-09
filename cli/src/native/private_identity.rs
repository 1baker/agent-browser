//! Fail-closed identity checks for private retained-tab operations.
//!
//! Callers must obtain a fresh authoritative `ServiceState::service_tab_handle`
//! and live CDP identity while holding the privacy barrier. This module neither
//! selects a tab nor establishes authority from a caller-supplied handle alone.

use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

/// Validated authorization snapshot, constructible only through the comparison
/// below. This is not evidence that the supplied observations were fetched from
/// Chrome: the trusted caller must collect them under the privacy barrier.
/// Intentionally not Debug, Serialize, Deserialize, Default or Clone.
pub(crate) struct ValidatedPrivateTarget {
    profile_id: String,
    browser_id: String,
    session_name: String,
    target_id: String,
    tab_id: String,
    lease_id: String,
    lease_state: String,
    owner_session_id: Option<String>,
    origin: String,
    url: String,
    scope_digest: String,
}

impl ValidatedPrivateTarget {
    pub(crate) fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Domain-separated SHA-256 of fixed, canonical authorization fields.
    /// This is an identity binding, not a password hash or bearer credential.
    pub(crate) fn scope_digest(&self) -> &str {
        &self.scope_digest
    }
}

/// Ordered destinations supplied by the trusted consent coordinator before
/// admission. This validates identity, not the caller's consent provenance.
pub(crate) struct ValidatedPrivateJourney {
    target_id: String,
    digests: Vec<String>,
}

impl ValidatedPrivateJourney {
    pub(crate) fn new(
        initial: &ValidatedPrivateTarget,
        destinations: &[(String, String)],
    ) -> Result<Self, &'static str> {
        if destinations.is_empty() || destinations.len() > 16 {
            return Err("private_journey_invalid");
        }
        let mut digests = Vec::new();
        for (origin, url) in destinations {
            let canonical = checked_destination(origin, url)?;
            let bytes = serde_json::to_vec(&serde_json::json!([
                "agent-browser.private-target.v1",
                initial.profile_id,
                initial.browser_id,
                initial.session_name,
                initial.target_id,
                initial.tab_id,
                initial.lease_id,
                initial.lease_state,
                initial.owner_session_id,
                origin,
                canonical,
                true
            ]))
            .map_err(|_| "private_journey_invalid")?;
            let digest = hex::encode(Sha256::digest(bytes));
            digests.push(digest);
        }
        if digests.first().map(String::as_str) != Some(initial.scope_digest()) {
            return Err("private_journey_initial_mismatch");
        }
        Ok(Self {
            target_id: initial.target_id.clone(),
            digests,
        })
    }

    pub(crate) fn target_id(&self) -> &str {
        &self.target_id
    }
    pub(crate) fn digests(&self) -> &[String] {
        &self.digests
    }
}

/// Observations from the currently attached browser, not the staged request.
pub struct LivePrivateIdentity<'a> {
    pub profile_id: &'a str,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub target_id: &'a str,
    pub url: &'a str,
    /// True only for ready service/browser health, not launching/reconnecting.
    pub ready: bool,
}

fn required<'a>(handle: &'a Value, field: &str) -> Result<&'a str, &'static str> {
    handle
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.trim() == *value)
        .ok_or("private_identity_incomplete")
}

fn usable(handle: &Value) -> Result<(), &'static str> {
    if handle.get("valid").and_then(Value::as_bool) != Some(true)
        || handle.get("staleReason").is_none_or(|v| !v.is_null())
    {
        return Err("private_identity_stale");
    }
    // Shared and human-takeover leases cannot authorize secret mutation.
    if required(handle, "leaseState")? != "exclusive"
        || handle
            .get("leaseHeartbeatExpected")
            .and_then(Value::as_bool)
            != Some(true)
        || required(handle, "leaseId")? != required(handle, "sessionName")?
    {
        return Err("private_identity_lease_unavailable");
    }
    Ok(())
}

/// Normalize only HTTPS URLs; refuse userinfo, fragments and ambiguous inputs.
/// Error labels never contain supplied URLs or identity fields.
pub fn canonical_private_url(value: &str) -> Result<String, &'static str> {
    if !value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains(['\\', '#'])
    {
        return Err("private_url_invalid");
    }
    let parsed = Url::parse(value).map_err(|_| "private_url_invalid")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || value
            .split('/')
            .nth(2)
            .is_some_and(|host| host.contains('@'))
    {
        return Err("private_url_invalid");
    }
    Ok(parsed.to_string())
}

fn checked_destination(origin: &str, url: &str) -> Result<String, &'static str> {
    let canonical = canonical_private_url(url)?;
    let parsed = Url::parse(&canonical).map_err(|_| "private_url_invalid")?;
    if parsed.origin().ascii_serialization() != origin {
        return Err("private_origin_mismatch");
    }
    Ok(canonical)
}

/// Compare staged authority with a freshly rebuilt service handle and the exact
/// currently attached target. Never silently switch to a compatible candidate.
pub fn validate_private_identity(
    staged: &Value,
    authoritative: &Value,
    live: &LivePrivateIdentity<'_>,
    expected_origin: &str,
    expected_url: &str,
) -> Result<(), &'static str> {
    usable(staged)?;
    usable(authoritative)?;
    if !live.ready {
        return Err("private_identity_not_ready");
    }
    for field in [
        "profileId",
        "browserId",
        "sessionName",
        "targetId",
        "tabId",
        "leaseId",
        "leaseState",
    ] {
        if required(staged, field)? != required(authoritative, field)? {
            return Err("private_identity_mismatch");
        }
    }
    if staged.get("ownerSessionId") != authoritative.get("ownerSessionId") {
        return Err("private_identity_mismatch");
    }
    for (field, observed) in [
        ("profileId", live.profile_id),
        ("browserId", live.browser_id),
        ("sessionName", live.session_name),
        ("targetId", live.target_id),
    ] {
        if required(authoritative, field)? != observed {
            return Err("private_identity_mismatch");
        }
    }
    let expected = checked_destination(expected_origin, expected_url)?;
    for observed in [
        required(staged, "url")?,
        required(authoritative, "url")?,
        live.url,
    ] {
        if canonical_private_url(observed)? != expected {
            return Err("private_url_mismatch");
        }
    }
    Ok(())
}

/// Produce an owned target capability only after complete identity validation.
/// Persist its digest with the private epoch before any mutation, and require
/// the same binding when recovering or cleaning up that epoch.
pub(crate) fn validate_private_target(
    staged: &Value,
    authoritative: &Value,
    live: &LivePrivateIdentity<'_>,
    expected_origin: &str,
    expected_url: &str,
) -> Result<ValidatedPrivateTarget, &'static str> {
    validate_private_identity(staged, authoritative, live, expected_origin, expected_url)?;
    let owner_session_id = match authoritative.get("ownerSessionId") {
        None | Some(Value::Null) => None,
        Some(Value::String(owner)) if !owner.is_empty() && owner.trim() == owner => {
            Some(owner.clone())
        }
        _ => return Err("private_identity_incomplete"),
    };
    let mut target = ValidatedPrivateTarget {
        profile_id: required(authoritative, "profileId")?.to_owned(),
        browser_id: required(authoritative, "browserId")?.to_owned(),
        session_name: required(authoritative, "sessionName")?.to_owned(),
        target_id: required(authoritative, "targetId")?.to_owned(),
        tab_id: required(authoritative, "tabId")?.to_owned(),
        lease_id: required(authoritative, "leaseId")?.to_owned(),
        lease_state: required(authoritative, "leaseState")?.to_owned(),
        owner_session_id,
        origin: expected_origin.to_owned(),
        url: checked_destination(expected_origin, expected_url)?,
        scope_digest: String::new(),
    };
    // An ordered JSON array provides unambiguous string boundaries and explicit
    // null ownership. Do not hash the raw handle, mutable title or job metadata.
    let canonical = serde_json::to_vec(&serde_json::json!([
        "agent-browser.private-target.v1",
        target.profile_id,
        target.browser_id,
        target.session_name,
        target.target_id,
        target.tab_id,
        target.lease_id,
        target.lease_state,
        target.owner_session_id,
        target.origin,
        target.url,
        true // validated leaseHeartbeatExpected
    ]))
    .map_err(|_| "private_identity_incomplete")?;
    target.scope_digest = hex::encode(Sha256::digest(canonical));
    Ok(target)
}

/// A synchronous guard to prepend inside the same top-level evaluation that
/// accesses a secret. It is not a standalone preflight: callers must not await
/// between this guard and DOM access, nor use it to authorize child frames.
/// Execute in a CDP isolated world so page scripts cannot replace the globals.
/// The returned source contains only destination identity, never secret data.
pub fn top_level_origin_guard(
    expected_origin: &str,
    expected_url: &str,
) -> Result<String, &'static str> {
    let canonical = checked_destination(expected_origin, expected_url)?;
    let origin = serde_json::to_string(expected_origin).map_err(|_| "private_url_invalid")?;
    let url = serde_json::to_string(&canonical).map_err(|_| "private_url_invalid")?;
    Ok(format!(
        "if (globalThis !== globalThis.top || location.origin !== {origin} || location.href !== {url}) {{ throw new Error('private_destination_mismatch'); }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn handle() -> Value {
        json!({
            "profileId": "sam", "browserId": "session:default",
            "sessionName": "default", "tabId": "tab-1", "targetId": "target-1",
            "url": "https://secure.login.gov/", "leaseId": "default",
            "leaseState": "exclusive", "leaseHeartbeatExpected": true,
            "ownerSessionId": "default", "valid": true, "staleReason": null
        })
    }

    fn live() -> LivePrivateIdentity<'static> {
        LivePrivateIdentity {
            profile_id: "sam",
            browser_id: "session:default",
            session_name: "default",
            target_id: "target-1",
            url: "https://secure.login.gov/",
            ready: true,
        }
    }

    fn check(
        staged: &Value,
        current: &Value,
        live: &LivePrivateIdentity<'_>,
    ) -> Result<(), &'static str> {
        validate_private_identity(
            staged,
            current,
            live,
            "https://secure.login.gov",
            "https://secure.login.gov/",
        )
    }

    #[test]
    fn accepts_complete_exact_ready_identity() {
        assert_eq!(check(&handle(), &handle(), &live()), Ok(()));
    }

    #[test]
    fn journey_binds_initial_identity_and_exact_ordered_destinations() {
        let initial = token(&handle(), &handle(), &live()).unwrap();
        let destinations = vec![
            (
                "https://secure.login.gov".into(),
                "https://secure.login.gov/".into(),
            ),
            (
                "https://sam.gov".into(),
                "https://sam.gov/workspace/profile/account-details".into(),
            ),
            (
                "https://sam.gov".into(),
                "https://sam.gov/workspace/profile/account-details".into(),
            ),
        ];
        let journey = ValidatedPrivateJourney::new(&initial, &destinations).unwrap();
        assert_eq!(journey.digests()[0], initial.scope_digest());
        assert_eq!(journey.digests()[1], journey.digests()[2]);
        let mut destination_handle = handle();
        destination_handle["url"] = serde_json::json!(destinations[1].1);
        let mut observed = live();
        observed.url = &destinations[1].1;
        let arrived = validate_private_target(
            &destination_handle,
            &destination_handle,
            &observed,
            &destinations[1].0,
            &destinations[1].1,
        )
        .unwrap();
        assert_eq!(journey.digests()[1], arrived.scope_digest());
        assert!(ValidatedPrivateJourney::new(&initial, &destinations[1..]).is_err());
        assert!(ValidatedPrivateJourney::new(&initial, &[]).is_err());
        let mut wrong = destinations.clone();
        wrong[1].1 = "https://evil.example/".into();
        assert!(ValidatedPrivateJourney::new(&initial, &wrong).is_err());
        let mut changed = handle();
        changed["tabId"] = serde_json::json!("other-tab");
        let other = token(&changed, &changed, &live()).unwrap();
        let other_journey = ValidatedPrivateJourney::new(&other, &destinations).unwrap();
        assert_ne!(journey.digests()[1], other_journey.digests()[1]);
    }

    fn token(
        staged: &Value,
        current: &Value,
        observed: &LivePrivateIdentity<'_>,
    ) -> Result<ValidatedPrivateTarget, &'static str> {
        validate_private_target(
            staged,
            current,
            observed,
            "https://secure.login.gov",
            "https://secure.login.gov/",
        )
    }

    #[test]
    fn capability_rejects_drift_and_missing_authority() {
        for field in [
            "profileId",
            "browserId",
            "sessionName",
            "targetId",
            "tabId",
            "leaseId",
            "url",
        ] {
            let mut changed = handle();
            changed[field] = serde_json::json!("different");
            assert!(token(&handle(), &changed, &live()).is_err());
            changed.as_object_mut().unwrap().remove(field);
            assert!(token(&changed, &changed, &live()).is_err());
        }
        let mut observed = live();
        observed.url = "https://secure.login.gov/wrong-conversation";
        assert!(token(&handle(), &handle(), &observed).is_err());
        observed = live();
        observed.ready = false;
        assert!(token(&handle(), &handle(), &observed).is_err());
        let mut malformed = handle();
        malformed["ownerSessionId"] = serde_json::json!({"untrusted": true});
        assert!(token(&malformed, &malformed, &live()).is_err());
    }

    #[test]
    fn capability_digest_is_canonical_and_ignores_display_metadata() {
        let original = token(&handle(), &handle(), &live()).unwrap();
        assert_eq!(original.target_id(), "target-1");
        assert_eq!(original.scope_digest().len(), 64);
        let mut canonical_equivalent = handle();
        canonical_equivalent["url"] = serde_json::json!("https://SECURE.LOGIN.GOV:443");
        canonical_equivalent["title"] = serde_json::json!("A different display label");
        canonical_equivalent["jobId"] = serde_json::json!("later-job");
        let equivalent = token(&canonical_equivalent, &canonical_equivalent, &live()).unwrap();
        assert_eq!(original.scope_digest(), equivalent.scope_digest());
    }

    #[test]
    fn capability_digest_binds_each_mutable_authorization_identifier() {
        let original = token(&handle(), &handle(), &live()).unwrap();
        for field in [
            "profileId",
            "browserId",
            "targetId",
            "tabId",
            "ownerSessionId",
        ] {
            let mut changed = handle();
            changed[field] = serde_json::json!("different");
            let mut observed = live();
            match field {
                "profileId" => observed.profile_id = "different",
                "browserId" => observed.browser_id = "different",
                "targetId" => observed.target_id = "different",
                _ => {}
            }
            let changed = token(&changed, &changed, &observed).unwrap();
            assert_ne!(original.scope_digest(), changed.scope_digest(), "{field}");
        }
        let mut changed = handle();
        changed["sessionName"] = serde_json::json!("different");
        changed["leaseId"] = serde_json::json!("different");
        let mut observed = live();
        observed.session_name = "different";
        assert_ne!(
            original.scope_digest(),
            token(&changed, &changed, &observed).unwrap().scope_digest()
        );
        changed = handle();
        changed["url"] = serde_json::json!("https://secure.login.gov/new");
        observed = live();
        observed.url = "https://secure.login.gov/new";
        let changed = validate_private_target(
            &changed,
            &changed,
            &observed,
            "https://secure.login.gov",
            "https://secure.login.gov/new",
        )
        .unwrap();
        assert_ne!(original.scope_digest(), changed.scope_digest());
    }

    #[test]
    fn matching_healthy_handles_at_wrong_url_cannot_authorize_requested_destination() {
        let mut wrong = handle();
        wrong["url"] = json!("https://secure.login.gov/other-conversation");
        let mut current = live();
        current.url = "https://secure.login.gov/other-conversation";
        assert_eq!(check(&wrong, &wrong, &current), Err("private_url_mismatch"));
    }

    #[test]
    fn missing_validity_and_owner_drift_fail_closed() {
        for field in ["valid", "staleReason", "leaseHeartbeatExpected"] {
            let mut missing = handle();
            missing.as_object_mut().unwrap().remove(field);
            assert!(check(&missing, &missing, &live()).is_err());
        }
        let mut changed = handle();
        changed["ownerSessionId"] = json!("other");
        assert_eq!(
            check(&handle(), &changed, &live()),
            Err("private_identity_mismatch")
        );
    }

    #[test]
    fn rejects_each_missing_and_changed_binding() {
        for field in [
            "profileId",
            "browserId",
            "sessionName",
            "targetId",
            "tabId",
            "leaseId",
            "leaseState",
            "url",
        ] {
            let mut changed = handle();
            changed[field] = json!("other");
            assert!(check(&handle(), &changed, &live()).is_err(), "{field}");
            let mut missing = handle();
            missing.as_object_mut().unwrap().remove(field);
            assert!(check(&missing, &missing, &live()).is_err(), "{field}");
        }
    }

    #[test]
    fn rejects_same_profile_wrong_live_target_and_other_live_drift() {
        let mut current = live();
        current.target_id = "wrong-target";
        assert_eq!(
            check(&handle(), &handle(), &current),
            Err("private_identity_mismatch")
        );
        current = live();
        current.profile_id = "wrong-profile";
        assert!(check(&handle(), &handle(), &current).is_err());
        current = live();
        current.session_name = "wrong-session";
        assert!(check(&handle(), &handle(), &current).is_err());
        current = live();
        current.browser_id = "session:other";
        assert!(check(&handle(), &handle(), &current).is_err());
        current = live();
        current.url = "https://secure.login.gov/wrong-chat";
        assert_eq!(
            check(&handle(), &handle(), &current),
            Err("private_url_mismatch")
        );
        current = live();
        current.ready = false;
        assert_eq!(
            check(&handle(), &handle(), &current),
            Err("private_identity_not_ready")
        );
    }

    #[test]
    fn rejects_stale_unowned_and_nonexclusive_leases() {
        for (field, value) in [
            ("valid", json!(false)),
            ("staleReason", json!("lease_expired")),
            ("staleReason", json!("")),
            ("leaseState", json!("expired")),
            ("leaseState", json!("shared")),
            ("leaseState", json!("human_takeover")),
            ("leaseHeartbeatExpected", json!(false)),
        ] {
            let mut changed = handle();
            changed[field] = value;
            assert!(check(&changed, &handle(), &live()).is_err());
            assert!(check(&handle(), &changed, &live()).is_err());
        }
    }

    #[test]
    fn rejects_unsafe_urls_and_compares_canonical_https_destination() {
        for url in [
            "http://secure.login.gov/",
            "https:/secure.login.gov/",
            "https://user:pass@secure.login.gov/",
            "https://@secure.login.gov/",
            "https://secure.login.gov/#",
            "https://secure.login.gov/#secret",
            " https://secure.login.gov/",
            "https://secure.login.gov/\n",
            "https://secure.login.gov\\@evil.test/",
        ] {
            assert_eq!(canonical_private_url(url), Err("private_url_invalid"));
        }
        assert_eq!(
            canonical_private_url("https://SECURE.LOGIN.GOV:443"),
            Ok("https://secure.login.gov/".to_owned())
        );
        assert!(checked_destination("https://secure.login.gov", "https://evil.test/").is_err());
        assert!(
            checked_destination("https://secure.login.gov/", "https://secure.login.gov/").is_err()
        );
        assert!(
            checked_destination("https://secure.login.gov", "https://secure.login.gov:444/")
                .is_err()
        );
    }

    #[test]
    fn origin_guard_checks_top_level_and_exact_url_without_payload() {
        let guard = top_level_origin_guard("https://secure.login.gov", "https://secure.login.gov/")
            .unwrap();
        assert!(guard.contains("globalThis !== globalThis.top"));
        assert!(guard.contains("location.origin !== \"https://secure.login.gov\""));
        assert!(guard.contains("location.href !== \"https://secure.login.gov/\""));
        assert!(!guard.contains("await"));
    }
}
