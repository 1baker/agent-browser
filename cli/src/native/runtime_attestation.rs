//! Linux-only proof for an exact retained-browser daemon handoff.
//!
//! A legacy handoff is deliberately not proof. This module verifies schema-v4
//! continuity; the separate internal fresh-chain module proves independent custody.

use serde_json::{json, Map, Value};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use super::runtime_handoff_v2::{current_process_identity, require_process_gone, verify_browser};
use super::service_model::{BrowserHost, LeaseState, ServiceState, TabLifecycle};

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("attestation_{name}_missing"))
}

fn physical_browser(browser: &Value, pid: u32, endpoint: &str) -> Result<(), String> {
    let path = verify_browser(browser, pid, endpoint)?;
    let lock = fs::read_link(path.join("SingletonLock"))
        .map_err(|_| "attestation_profile_lock_missing")?;
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname")
        .map_err(|_| "attestation_hostname_unreadable")?;
    if lock.to_string_lossy() != format!("{}-{pid}", hostname.trim()) {
        return Err("attestation_profile_lock_mismatch".into());
    }
    Ok(())
}

fn exclusive_browser_binding(
    state: &ServiceState,
    session_name: &str,
    profile_id: &str,
    browser_id: &str,
    pid: u32,
    endpoint: &str,
) -> Result<(), String> {
    let session = state
        .sessions
        .get(session_name)
        .ok_or("attestation_session_missing")?;
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or("attestation_browser_missing")?;
    if session.id != session_name
        || session.lease != LeaseState::Exclusive
        || session.profile_id.as_deref() != Some(profile_id)
        || state.sessions.values().any(|other| {
            other.id != session_name
                && other.profile_id.as_deref() == Some(profile_id)
                && !matches!(other.lease, LeaseState::Released | LeaseState::Expired)
        })
    {
        return Err("attestation_exclusive_profile_lease_missing".into());
    }
    if !session.browser_ids.iter().any(|id| id == browser_id)
        || browser.id != browser_id
        || browser.profile_id.as_deref() != Some(profile_id)
        || browser.pid != Some(pid)
        || browser.cdp_endpoint.as_deref() != Some(endpoint)
        || browser.host == BrowserHost::RemoteHeaded
    {
        return Err("attestation_service_binding_mismatch".into());
    }
    Ok(())
}

fn binding(
    state: &ServiceState,
    session_name: &str,
    profile_id: &str,
    browser_id: &str,
    target_id: &str,
    pid: u32,
    endpoint: &str,
) -> Result<(), String> {
    exclusive_browser_binding(state, session_name, profile_id, browser_id, pid, endpoint)?;
    let tab_id = format!("target:{target_id}");
    let tab = state.tabs.get(&tab_id).ok_or("attestation_tab_missing")?;
    if !state.sessions[session_name].tab_ids.contains(&tab_id)
        || tab.id != tab_id
        || tab.browser_id != browser_id
        || tab.target_id.as_deref() != Some(target_id)
        || tab.owner_session_id.as_deref() != Some(session_name)
        || matches!(
            tab.lifecycle,
            TabLifecycle::Closed | TabLifecycle::Closing | TabLifecycle::Crashed
        )
    {
        return Err("attestation_service_binding_mismatch".into());
    }
    Ok(())
}

/// Historical closure is not a new owner proof. Both the current target and
/// the same exclusive browser binding must still verify independently.
fn anchor_disposition(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    current_target: &str,
) -> Result<&'static str, String> {
    let profile_id = field(custody, "profileId")?;
    let browser_id = field(custody, "browserId")?;
    let anchor = field(custody, "targetId")?;
    binding(
        state,
        session_name,
        profile_id,
        browser_id,
        current_target,
        pid,
        endpoint,
    )?;
    let tab_id = format!("target:{anchor}");
    let tab = state
        .tabs
        .get(&tab_id)
        .ok_or("attestation_historical_anchor_missing")?;
    if browser_id != format!("session:{session_name}")
        || tab.id != tab_id
        || tab.browser_id != browser_id
        || tab.target_id.as_deref() != Some(anchor)
        || tab.owner_session_id.as_deref() != Some(session_name)
    {
        return Err("attestation_historical_anchor_foreign".into());
    }
    if tab.lifecycle == TabLifecycle::Closed {
        if state
            .sessions
            .values()
            .any(|session| session.tab_ids.contains(&tab_id))
        {
            return Err("attestation_historical_anchor_listed".into());
        }
        return Ok("closed_historical");
    }
    binding(
        state,
        session_name,
        profile_id,
        browser_id,
        anchor,
        pid,
        endpoint,
    )
    .map_err(|_| "attestation_historical_anchor_not_closed".to_string())?;
    Ok(if anchor == current_target {
        "active"
    } else {
        "live"
    })
}

fn verify_custody(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    target_id: &str,
) -> Result<(), String> {
    verify_bound_custody(
        state,
        session_name,
        custody,
        pid,
        endpoint,
        target_id,
        target_id,
    )
}

fn verify_bound_custody(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    stored_anchor: &str,
    current_target: &str,
) -> Result<(), String> {
    let profile_id = field(custody, "profileId")?;
    let browser_id = field(custody, "browserId")?;
    if field(custody, "targetId")? != stored_anchor || field(custody, "cdpEndpoint")? != endpoint {
        return Err("attestation_target_or_endpoint_mismatch".into());
    }
    binding(
        state,
        session_name,
        profile_id,
        browser_id,
        current_target,
        pid,
        endpoint,
    )?;
    let browser = custody
        .get("browser")
        .ok_or("attestation_browser_proof_missing")?;
    physical_browser(browser, pid, endpoint)?;
    let physical_profile = field(browser, "canonicalProfile")?;
    let configured = state
        .profiles
        .get(profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .ok_or("attestation_service_profile_missing")?;
    if fs::canonicalize(configured).map_err(|_| "attestation_service_profile_unreadable")?
        != fs::canonicalize(physical_profile)
            .map_err(|_| "attestation_browser_profile_unreadable")?
    {
        return Err("attestation_service_profile_mismatch".into());
    }
    Ok(())
}

fn wait_for_source_exit(source: &Value) -> Result<(), String> {
    if source.get("startTicks").and_then(Value::as_u64).is_none()
        || source.get("uid").and_then(Value::as_u64).is_none()
        || source
            .get("executableDevice")
            .and_then(Value::as_u64)
            .is_none()
        || source
            .get("executableInode")
            .and_then(Value::as_u64)
            .is_none()
    {
        return Err("attestation_source_identity_incomplete".into());
    }
    for _ in 0..100 {
        match require_process_gone(source) {
            Ok(()) => return Ok(()),
            Err(error) if error == "runtime_handoff_v2_source_still_alive" => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
    Err("attestation_source_exit_timeout".into())
}

/// Prepare a custody-bearing descriptor only when the current physical
/// Chrome and persisted exclusive lease agree. Never silently downgrade an
/// already attested session to a legacy descriptor.
pub(super) fn prepare(
    state: &ServiceState,
    session_name: &str,
    target_id: &str,
    endpoint: &str,
) -> Result<Option<(u32, Value)>, String> {
    let browser_id = format!("session:{session_name}");
    let Some(session) = state.sessions.get(session_name) else {
        return Ok(None);
    };
    let Some(profile_id) = session.profile_id.as_deref() else {
        return Ok(None);
    };
    let Some(pid) = state
        .browsers
        .get(&browser_id)
        .and_then(|browser| browser.pid)
    else {
        return Ok(None);
    };
    if let Err(error) = binding(
        state,
        session_name,
        profile_id,
        &browser_id,
        target_id,
        pid,
        endpoint,
    ) {
        return if state.runtime_custody_receipts.contains_key(session_name) {
            Err(error)
        } else {
            Ok(None)
        };
    }
    let configured = state
        .profiles
        .get(profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .ok_or("attestation_service_profile_missing")?;
    let canonical =
        fs::canonicalize(configured).map_err(|_| "attestation_service_profile_unreadable")?;
    let metadata =
        fs::metadata(&canonical).map_err(|_| "attestation_service_profile_unreadable")?;
    let browser = json!({
        "process": current_process_identity(pid)?,
        "canonicalProfile": canonical,
        "profileDevice": metadata.dev(),
        "profileInode": metadata.ino(),
        "cdpEndpoint": endpoint,
    });
    physical_browser(&browser, pid, endpoint)?;
    let (generation, prior_anchor) =
        if let Some(previous) = state.runtime_custody_receipts.get(session_name) {
            let prior_target = field(previous, "targetId")?;
            let disposition = verify_committed(
                state,
                session_name,
                previous,
                pid,
                endpoint,
                prior_target,
                target_id,
            )?;
            let generation = previous["ownerGeneration"]
                .as_u64()
                .ok_or("attestation_generation_missing")?
                .checked_add(1)
                .ok_or("attestation_generation_overflow")?;
            (
                generation,
                Some(json!({"targetId": prior_target, "disposition": disposition})),
            )
        } else {
            (1, None)
        };
    let mut custody = json!({
        "source": current_process_identity(std::process::id())?,
        "browser": browser,
        "browserId": browser_id,
        "profileId": profile_id,
        "targetId": target_id,
        "cdpEndpoint": endpoint,
        "ownerGeneration": generation,
    });
    if let Some(prior_anchor) = prior_anchor {
        custody["priorAnchor"] = prior_anchor;
    }
    Ok(Some((pid, custody)))
}

pub(super) fn verify_resume(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    target_id: &str,
) -> Result<(), String> {
    if custody.get("kind").is_some() {
        return Err("attestation_legacy_receipt_kind_unsupported".into());
    }
    let source = custody.get("source").ok_or("attestation_source_missing")?;
    wait_for_source_exit(source)?;
    verify_custody(state, session_name, custody, pid, endpoint, target_id)?;
    if let Some(previous) = state.runtime_custody_receipts.get(session_name) {
        if previous.get("kind").is_some()
            || previous["schemaVersion"] != 4
            || previous["phase"] != "committed"
            || previous["destination"] != *source
            || previous["browser"] != custody["browser"]
            || previous["profileId"] != custody["profileId"]
            || previous["browserId"] != custody["browserId"]
            || previous["ownerGeneration"]
                .as_u64()
                .and_then(|n| n.checked_add(1))
                != custody["ownerGeneration"].as_u64()
        {
            return Err("attestation_stale_owner".into());
        }
        if let Some(prior_anchor) = custody.get("priorAnchor") {
            let disposition =
                anchor_disposition(state, session_name, previous, pid, endpoint, target_id)?;
            let recorded = field(prior_anchor, "disposition")?;
            if prior_anchor.get("targetId") != previous.get("targetId")
                || !(recorded == disposition
                    || (recorded == "live" && disposition == "closed_historical"))
            {
                return Err("attestation_prior_anchor_mismatch".into());
            }
        }
    } else if custody["ownerGeneration"] != 1 {
        return Err("attestation_missing_prior_receipt".into());
    } else if custody.get("priorAnchor").is_some() {
        return Err("attestation_unexpected_prior_anchor".into());
    }
    Ok(())
}

pub(super) fn committed(custody: &Value) -> Result<Value, String> {
    if custody.get("kind").is_some() {
        return Err("attestation_legacy_receipt_kind_unsupported".into());
    }
    let mut receipt = custody.clone();
    let object = receipt
        .as_object_mut()
        .ok_or("attestation_custody_invalid")?;
    object.insert("schemaVersion".into(), json!(4));
    object.insert("phase".into(), json!("committed"));
    object.insert(
        "destination".into(),
        current_process_identity(std::process::id())?,
    );
    Ok(receipt)
}

fn verify_committed(
    state: &ServiceState,
    session_name: &str,
    receipt: &Value,
    pid: u32,
    endpoint: &str,
    target_id: &str,
    current_target: &str,
) -> Result<&'static str, String> {
    if receipt.get("kind").is_some() {
        return Err("attestation_legacy_receipt_kind_unsupported".into());
    }
    if receipt["schemaVersion"] != 4
        || receipt["phase"] != "committed"
        || receipt["ownerGeneration"]
            .as_u64()
            .is_none_or(|generation| generation == 0)
        || receipt.get("destination") != Some(&current_process_identity(std::process::id())?)
    {
        return Err("attestation_stale_owner".into());
    }
    wait_for_source_exit(receipt.get("source").ok_or("attestation_source_missing")?)?;
    verify_bound_custody(
        state,
        session_name,
        receipt,
        pid,
        endpoint,
        target_id,
        current_target,
    )?;
    anchor_disposition(state, session_name, receipt, pid, endpoint, current_target)
}

/// Crate-private reuse of the unchanged binding proof for the fresh chain.
pub(super) fn fresh_chain_binding(
    state: &ServiceState,
    session_name: &str,
    profile_id: &str,
    browser_id: &str,
    target_id: &str,
    pid: u32,
    endpoint: &str,
) -> Result<(), String> {
    binding(
        state,
        session_name,
        profile_id,
        browser_id,
        target_id,
        pid,
        endpoint,
    )
}

/// Crate-private reuse of the unchanged physical browser/profile-lock proof.
pub(super) fn fresh_chain_physical_browser(
    browser: &Value,
    pid: u32,
    endpoint: &str,
) -> Result<(), String> {
    physical_browser(browser, pid, endpoint)
}

/// Diagnostics never infer owner custody from a browser being reachable.
/// The receipt must bind this daemon generation and the exact current handle.
pub(super) fn diagnostics(
    state: Option<&ServiceState>,
    handle: &Map<String, Value>,
    session_name: &str,
    active_target: Option<&str>,
) -> Value {
    let result = (|| -> Result<Value, String> {
        let state = state.ok_or("attestation_state_unavailable")?;
        state.validate_diagnostics_handle(handle, session_name)?;
        let target = handle
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or("attestation_handle_target_missing")?;
        let tab_id = handle
            .get("tabId")
            .and_then(Value::as_str)
            .ok_or("attestation_handle_tab_missing")?;
        let browser_id = handle
            .get("browserId")
            .and_then(Value::as_str)
            .ok_or("attestation_handle_browser_missing")?;
        let profile_id = handle
            .get("profileId")
            .and_then(Value::as_str)
            .ok_or("attestation_handle_profile_missing")?;
        let profile = state
            .profiles
            .get(profile_id)
            .ok_or("attestation_handle_profile_missing")?;
        if handle.get("profileOrigin")
            != Some(
                &serde_json::to_value(profile.profile_origin)
                    .map_err(|_| "attestation_profile_origin_invalid")?,
            )
        {
            return Err("attestation_handle_profile_origin_mismatch".into());
        }
        if active_target != Some(target)
            || tab_id != format!("target:{target}")
            || browser_id != format!("session:{session_name}")
        {
            return Err("attestation_handle_or_live_target_mismatch".into());
        }
        let receipt = state
            .runtime_custody_receipts
            .get(session_name)
            .ok_or("attestation_receipt_missing")?;
        if receipt["profileId"] != profile_id || receipt["browserId"] != browser_id {
            return Err("attestation_handle_binding_mismatch".into());
        }
        if receipt.get("schemaVersion").and_then(Value::as_u64) == Some(5) {
            return super::runtime_custody_bootstrap::diagnostics(
                state,
                session_name,
                receipt,
                profile_id,
                browser_id,
                target,
            );
        }
        let browser = state
            .browsers
            .get(browser_id)
            .ok_or("attestation_browser_missing")?;
        let pid = browser.pid.ok_or("attestation_browser_pid_missing")?;
        let endpoint = browser
            .cdp_endpoint
            .as_deref()
            .ok_or("attestation_endpoint_missing")?;
        binding(
            state,
            session_name,
            profile_id,
            browser_id,
            target,
            pid,
            endpoint,
        )?;
        let handoff_target = field(receipt, "targetId")?;
        let disposition = verify_committed(
            state,
            session_name,
            receipt,
            pid,
            endpoint,
            handoff_target,
            target,
        )?;
        Ok(json!({
            "complete": true,
            "missingProofs": [],
            "ownerCustody": {"verified": true, "basis": "schema_v4_committed_handoff", "ownerGeneration": receipt["ownerGeneration"],
                "historicalAnchor": {"targetId": handoff_target, "disposition": disposition}},
            "profileLease": {"verified": true, "leaseState": "exclusive"},
            "browserProcess": {"verified": true, "pid": pid, "startTicks": receipt["browser"]["process"]["startTicks"]},
            "tab": {"verified": true, "targetId": target},
        }))
    })();
    result.unwrap_or_else(|reason| {
        json!({
            "complete": false,
            "missingProofs": [reason],
            "ownerCustody": {"verified": false},
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, ProfileOrigin,
    };
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};

    const ENDPOINT: &str = "ws://127.0.0.1:9222/devtools/browser/x";

    #[test]
    fn legacy_unknown_kind_refused_before_handoff_prepare_and_proof_promotion() {
        let mut fixture = physical_fixture();
        fixture
            .state
            .browsers
            .get_mut("session:proposal")
            .unwrap()
            .health = super::super::service_model::BrowserHealth::Ready;
        for tab in fixture.state.tabs.values_mut() {
            tab.lifecycle = TabLifecycle::Ready;
        }
        let handle = serde_json::to_value(fixture.state.service_tab_handle("target:tab2").unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        for kind in [
            json!("unknown"),
            json!("fresh_chain_bootstrap"),
            Value::Null,
        ] {
            fixture
                .state
                .runtime_custody_receipts
                .get_mut("proposal")
                .unwrap()["kind"] = kind;
            assert_eq!(
                prepare(&fixture.state, "proposal", "tab2", ENDPOINT).unwrap_err(),
                "attestation_legacy_receipt_kind_unsupported"
            );
            let proof = diagnostics(Some(&fixture.state), &handle, "proposal", Some("tab2"));
            assert_eq!(proof["complete"], false);
            assert_eq!(
                proof["missingProofs"][0],
                "attestation_legacy_receipt_kind_unsupported"
            );
            assert_eq!(
                committed(&fixture.state.runtime_custody_receipts["proposal"]).unwrap_err(),
                "attestation_legacy_receipt_kind_unsupported"
            );
        }
        fixture
            .state
            .runtime_custody_receipts
            .get_mut("proposal")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("kind");
        prepare(&fixture.state, "proposal", "tab2", ENDPOINT).unwrap();
        assert_eq!(
            diagnostics(Some(&fixture.state), &handle, "proposal", Some("tab2"))["complete"],
            true
        );
    }

    struct FixtureProfile(PathBuf);

    impl FixtureProfile {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("agent-browser-anchor-{}", uuid::Uuid::new_v4()));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            let mut profile = Self(path);
            profile.0 = fs::canonicalize(&profile.0).unwrap();
            profile
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for FixtureProfile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    // These are private process/file fixtures, not Chromium or live E2E tests.
    struct PhysicalFixture {
        child: Child,
        _profile: FixtureProfile,
        state: ServiceState,
        pid: u32,
    }

    impl Drop for PhysicalFixture {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn waiting_child(profile: &std::path::Path) -> Child {
        Command::new("/bin/sh")
            .args(["-c", "read -r fixture", "fixture"])
            .arg(format!("--user-data-dir={}", profile.display()))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn exited_identity(profile: &std::path::Path) -> Value {
        struct ReapedChild(Child);
        impl Drop for ReapedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut child = ReapedChild(waiting_child(profile));
        let identity = current_process_identity(child.0.id());
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        identity.unwrap()
    }

    fn physical_fixture() -> PhysicalFixture {
        let profile = FixtureProfile::new();
        let child = waiting_child(profile.path());
        let pid = child.id();
        // Install the RAII guard before any assertion or filesystem operation.
        let mut fixture = PhysicalFixture {
            child,
            _profile: profile,
            state: fixture(),
            pid,
        };
        let path = fixture._profile.path();
        fs::write(
            path.join("DevToolsActivePort"),
            "9222\n/devtools/browser/x\n",
        )
        .unwrap();
        let hostname = fs::read_to_string("/proc/sys/kernel/hostname").unwrap();
        std::os::unix::fs::symlink(
            format!("{}-{pid}", hostname.trim()),
            path.join("SingletonLock"),
        )
        .unwrap();
        let metadata = fs::metadata(path).unwrap();
        fixture
            .state
            .profiles
            .get_mut("retained")
            .unwrap()
            .user_data_dir = Some(path.to_string_lossy().into());
        fixture
            .state
            .browsers
            .get_mut("session:proposal")
            .unwrap()
            .pid = Some(pid);
        fixture.state.runtime_custody_receipts.insert(
            "proposal".into(),
            json!({
                "schemaVersion": 4, "phase": "committed", "ownerGeneration": 5,
                "source": exited_identity(path),
                "destination": current_process_identity(std::process::id()).unwrap(),
                "browserId": "session:proposal", "profileId": "retained",
                "targetId": "tab1", "cdpEndpoint": ENDPOINT,
                "browser": {"process": current_process_identity(pid).unwrap(),
                    "canonicalProfile": path, "profileDevice": metadata.dev(),
                    "profileInode": metadata.ino(), "cdpEndpoint": ENDPOINT}
            }),
        );
        fixture
    }

    fn close_anchor(state: &mut ServiceState) {
        state.tabs.get_mut("target:tab1").unwrap().lifecycle = TabLifecycle::Closed;
        state
            .sessions
            .get_mut("proposal")
            .unwrap()
            .tab_ids
            .retain(|id| id != "target:tab1");
    }

    fn fixture_disposition(state: &ServiceState) -> Result<&'static str, String> {
        anchor_disposition(
            state,
            "proposal",
            &json!({"profileId":"retained", "browserId":"session:proposal", "targetId":"tab1"}),
            17,
            ENDPOINT,
            "tab2",
        )
    }

    fn fixture() -> ServiceState {
        let mut state = ServiceState::default();
        let session = BrowserSession {
            id: "proposal".into(),
            lease: LeaseState::Exclusive,
            profile_id: Some("retained".into()),
            browser_ids: vec!["session:proposal".into()],
            tab_ids: vec!["target:tab1".into(), "target:tab2".into()],
            ..Default::default()
        };
        state.sessions.insert(session.id.clone(), session);
        let browser = BrowserProcess {
            id: "session:proposal".into(),
            profile_id: Some("retained".into()),
            pid: Some(17),
            cdp_endpoint: Some("ws://127.0.0.1:9222/devtools/browser/x".into()),
            ..Default::default()
        };
        state.browsers.insert(browser.id.clone(), browser);
        state.profiles.insert(
            "retained".into(),
            BrowserProfile {
                id: "retained".into(),
                profile_origin: ProfileOrigin::ExternalByop,
                ..Default::default()
            },
        );
        for target in ["tab1", "tab2"] {
            let tab = BrowserTab {
                id: format!("target:{target}"),
                browser_id: "session:proposal".into(),
                target_id: Some(target.into()),
                owner_session_id: Some("proposal".into()),
                ..Default::default()
            };
            state.tabs.insert(tab.id.clone(), tab);
        }
        state
    }

    #[test]
    fn exclusive_lease_and_both_preserved_tabs_bind() {
        let state = fixture();
        for target in ["tab1", "tab2"] {
            binding(
                &state,
                "proposal",
                "retained",
                "session:proposal",
                target,
                17,
                "ws://127.0.0.1:9222/devtools/browser/x",
            )
            .unwrap();
        }
    }

    #[test]
    fn conflicting_profile_lease_fails_closed() {
        let mut state = fixture();
        state.sessions.insert(
            "other".into(),
            BrowserSession {
                id: "other".into(),
                profile_id: Some("retained".into()),
                lease: LeaseState::Shared,
                ..Default::default()
            },
        );
        assert_eq!(
            binding(
                &state,
                "proposal",
                "retained",
                "session:proposal",
                "tab1",
                17,
                "ws://127.0.0.1:9222/devtools/browser/x"
            )
            .unwrap_err(),
            "attestation_exclusive_profile_lease_missing"
        );
    }

    #[test]
    fn wrong_process_or_target_fails_closed() {
        let state = fixture();
        for (pid, target) in [(18, "tab1"), (17, "wrong")] {
            assert!(binding(
                &state,
                "proposal",
                "retained",
                "session:proposal",
                target,
                pid,
                "ws://127.0.0.1:9222/devtools/browser/x"
            )
            .is_err());
        }
    }

    #[test]
    fn missing_receipt_never_completes_diagnostics() {
        let state = fixture();
        let handle = valid_handle();
        let result = diagnostics(Some(&state), &handle, "proposal", Some("tab2"));
        assert_eq!(result["complete"], false);
        assert_eq!(result["missingProofs"][0], "attestation_receipt_missing");
    }

    fn valid_handle() -> Map<String, Value> {
        serde_json::from_value(json!({
            "browserId": "session:proposal", "tabId": "target:tab2",
            "targetId": "tab2", "profileId": "retained",
            "leaseState": "shared", "profileOrigin": "external_byop",
            "valid": true, "sessionName": "proposal", "leaseId": "proposal",
            "ownerSessionId": "proposal"
        }))
        .unwrap()
    }

    #[test]
    fn diagnostics_rejects_invalid_handle_metadata_before_custody_proof() {
        let state = fixture();
        let mut cases = vec![];
        for lease in [
            "released",
            "expired",
            "foreign",
            "human_takeover",
            "",
            "Exclusive",
        ] {
            cases.push((
                "leaseState",
                json!(lease),
                "attestation_handle_lease_state_invalid".to_string(),
            ));
        }
        for key in ["sessionName", "leaseId", "ownerSessionId"] {
            cases.push((
                key,
                json!("foreign"),
                format!("attestation_handle_{key}_mismatch"),
            ));
            cases.push((key, json!(""), format!("attestation_handle_{key}_mismatch")));
        }
        for key in ["leaseState", "sessionName", "leaseId", "ownerSessionId"] {
            let reason = if key == "leaseState" {
                "attestation_handle_lease_state_invalid".to_string()
            } else {
                format!("attestation_handle_{key}_mismatch")
            };
            for value in [Value::Null, json!(17), json!(true)] {
                cases.push((key, value, reason.clone()));
            }
            let mut handle = valid_handle();
            handle.remove(key);
            let result = diagnostics(Some(&state), &handle, "proposal", Some("tab2"));
            assert_eq!(result["complete"], false);
            assert_eq!(result["missingProofs"][0], reason, "missing {key}");
        }
        cases.push((
            "valid",
            json!(false),
            "attestation_handle_invalid".to_string(),
        ));
        for (key, value, reason) in cases {
            let mut handle = valid_handle();
            handle.insert(key.into(), value.clone());
            let result = diagnostics(Some(&state), &handle, "proposal", Some("tab2"));
            assert_eq!(result["complete"], false, "{key}={value}");
            assert_eq!(result["missingProofs"][0], reason, "{key}={value}");
        }
    }

    #[test]
    fn shared_handle_compatibility_requires_exact_current_ownership() {
        let state = fixture();
        for lease in ["shared", "exclusive"] {
            let mut handle = valid_handle();
            handle.insert("leaseState".into(), json!(lease));
            state
                .validate_diagnostics_handle(&handle, "proposal")
                .unwrap();
        }
        for key in [
            "browserId",
            "tabId",
            "targetId",
            "profileId",
            "profileOrigin",
        ] {
            let mut handle = valid_handle();
            handle.insert(key.into(), json!("foreign"));
            assert!(
                state
                    .validate_diagnostics_handle(&handle, "proposal")
                    .is_err(),
                "{key}"
            );
        }
        for lease in [
            LeaseState::Released,
            LeaseState::Expired,
            LeaseState::HumanTakeover,
        ] {
            let mut state = fixture();
            state.sessions.get_mut("proposal").unwrap().lease = lease;
            assert!(state
                .validate_diagnostics_handle(&valid_handle(), "proposal")
                .is_err());
        }
        let mut state = fixture();
        state.tabs.get_mut("target:tab2").unwrap().owner_session_id = Some("foreign".into());
        assert!(state
            .validate_diagnostics_handle(&valid_handle(), "proposal")
            .is_err());
    }

    #[test]
    fn stale_destination_identity_fails_closed() {
        let state = fixture();
        let receipt = json!({"schemaVersion": 4, "phase": "committed",
            "destination": {"pid": 999999999}});
        assert_eq!(
            verify_committed(
                &state,
                "proposal",
                &receipt,
                17,
                "ws://127.0.0.1:9222/devtools/browser/x",
                "tab1",
                "tab2"
            )
            .unwrap_err(),
            "attestation_stale_owner"
        );
    }

    #[test]
    fn partial_committed_receipt_is_not_owner_proof() {
        let state = fixture();
        let receipt = json!({
            "schemaVersion": 4,
            "phase": "committed",
            "ownerGeneration": 1,
            "destination": current_process_identity(std::process::id()).unwrap(),
        });
        assert_eq!(
            verify_committed(
                &state,
                "proposal",
                &receipt,
                17,
                "ws://127.0.0.1:9222/devtools/browser/x",
                "tab1",
                "tab2"
            )
            .unwrap_err(),
            "attestation_source_missing"
        );
    }

    #[test]
    fn reused_process_id_with_changed_start_ticks_fails_closed() {
        let pid = std::process::id();
        let mut process = current_process_identity(pid).unwrap();
        process["startTicks"] = json!(process["startTicks"].as_u64().unwrap() + 1);
        let browser =
            json!({"process": process, "cdpEndpoint": "ws://127.0.0.1:9222/devtools/browser/x"});
        assert_eq!(
            physical_browser(&browser, pid, "ws://127.0.0.1:9222/devtools/browser/x").unwrap_err(),
            "runtime_handoff_v2_browser_identity_mismatch"
        );
    }

    #[test]
    fn historical_anchor_accepts_only_bound_live_or_owned_closed_record() {
        let mut state = fixture();
        assert_eq!(fixture_disposition(&state).unwrap(), "live");
        let custody =
            json!({"profileId":"retained", "browserId":"session:proposal", "targetId":"tab2"});
        assert_eq!(
            anchor_disposition(&state, "proposal", &custody, 17, ENDPOINT, "tab2").unwrap(),
            "active"
        );
        close_anchor(&mut state);
        assert_eq!(fixture_disposition(&state).unwrap(), "closed_historical");
        assert!(binding(
            &state,
            "proposal",
            "retained",
            "session:proposal",
            "tab1",
            17,
            ENDPOINT
        )
        .is_err());
    }

    #[test]
    fn historical_anchor_missing_or_unlisted_ready_is_not_closure() {
        let mut state = fixture();
        state
            .sessions
            .get_mut("proposal")
            .unwrap()
            .tab_ids
            .retain(|id| id != "target:tab1");
        state.tabs.get_mut("target:tab1").unwrap().lifecycle = TabLifecycle::Ready;
        assert_eq!(
            fixture_disposition(&state).unwrap_err(),
            "attestation_historical_anchor_not_closed"
        );
        state.tabs.remove("target:tab1");
        assert_eq!(
            fixture_disposition(&state).unwrap_err(),
            "attestation_historical_anchor_missing"
        );
    }

    #[test]
    fn closed_anchor_rejects_foreign_record_or_any_session_membership() {
        for field in ["id", "owner", "browser", "target"] {
            let mut state = fixture();
            close_anchor(&mut state);
            let tab = state.tabs.get_mut("target:tab1").unwrap();
            match field {
                "id" => tab.id = "foreign".into(),
                "owner" => tab.owner_session_id = Some("foreign".into()),
                "browser" => tab.browser_id = "foreign".into(),
                _ => tab.target_id = Some("foreign".into()),
            }
            assert_eq!(
                fixture_disposition(&state).unwrap_err(),
                "attestation_historical_anchor_foreign",
                "{field}"
            );
        }
        for owner in ["proposal", "other"] {
            let mut state = fixture();
            close_anchor(&mut state);
            state
                .sessions
                .entry(owner.into())
                .or_default()
                .tab_ids
                .push("target:tab1".into());
            assert_eq!(
                fixture_disposition(&state).unwrap_err(),
                "attestation_historical_anchor_listed"
            );
        }
    }

    #[test]
    fn closed_anchor_never_bypasses_current_target_or_exclusive_lease() {
        for lifecycle in [
            TabLifecycle::Closed,
            TabLifecycle::Closing,
            TabLifecycle::Crashed,
        ] {
            let mut state = fixture();
            close_anchor(&mut state);
            state.tabs.get_mut("target:tab2").unwrap().lifecycle = lifecycle;
            assert!(fixture_disposition(&state).is_err());
        }
        for lease in [
            LeaseState::Released,
            LeaseState::Shared,
            LeaseState::Expired,
            LeaseState::HumanTakeover,
        ] {
            let mut state = fixture();
            close_anchor(&mut state);
            state.sessions.get_mut("proposal").unwrap().lease = lease;
            assert_eq!(
                fixture_disposition(&state).unwrap_err(),
                "attestation_exclusive_profile_lease_missing"
            );
        }
        let mut state = fixture();
        close_anchor(&mut state);
        state.sessions.insert(
            "other".into(),
            BrowserSession {
                id: "other".into(),
                profile_id: Some("retained".into()),
                lease: LeaseState::Shared,
                ..Default::default()
            },
        );
        assert_eq!(
            fixture_disposition(&state).unwrap_err(),
            "attestation_exclusive_profile_lease_missing"
        );
    }

    #[test]
    fn physical_closed_anchor_prepare_and_diagnostics_preserve_real_owner_proof() {
        let mut fixture = physical_fixture();
        close_anchor(&mut fixture.state);
        let before = serde_json::to_vec(&fixture.state).unwrap();
        let (pid, custody) = prepare(&fixture.state, "proposal", "tab2", ENDPOINT)
            .unwrap()
            .unwrap();
        assert_eq!(pid, fixture.pid);
        assert_eq!(custody["ownerGeneration"], 6);
        assert_eq!(
            custody["priorAnchor"],
            json!({"targetId":"tab1", "disposition":"closed_historical"})
        );
        let diagnostic = diagnostics(
            Some(&fixture.state),
            &valid_handle(),
            "proposal",
            Some("tab2"),
        );
        assert_eq!(diagnostic["complete"], true);
        assert_eq!(
            diagnostic["ownerCustody"]["historicalAnchor"],
            custody["priorAnchor"]
        );
        assert_eq!(serde_json::to_vec(&fixture.state).unwrap(), before);
        // Released handles remain invalid even with positive physical fixtures.
        let mut handle = valid_handle();
        handle["leaseState"] = json!("released");
        assert_eq!(
            diagnostics(Some(&fixture.state), &handle, "proposal", Some("tab2"))["complete"],
            false
        );
    }

    #[test]
    fn closed_anchor_physical_and_owner_mismatches_fail_without_downgrade() {
        for field in ["startTicks", "profileInode", "destination", "source"] {
            let mut fixture = physical_fixture();
            close_anchor(&mut fixture.state);
            let receipt = fixture
                .state
                .runtime_custody_receipts
                .get_mut("proposal")
                .unwrap();
            match field {
                "startTicks" => receipt["browser"]["process"]["startTicks"] = json!(0),
                "profileInode" => receipt["browser"]["profileInode"] = json!(0),
                "destination" => receipt["destination"] = exited_identity(fixture._profile.path()),
                _ => {
                    receipt.as_object_mut().unwrap().remove("source");
                }
            }
            assert!(
                prepare(&fixture.state, "proposal", "tab2", ENDPOINT).is_err(),
                "{field}"
            );
            assert_eq!(
                diagnostics(
                    Some(&fixture.state),
                    &valid_handle(),
                    "proposal",
                    Some("tab2")
                )["complete"],
                false
            );
        }
        let mut fixture = physical_fixture();
        close_anchor(&mut fixture.state);
        fixture.state.tabs.remove("target:tab1");
        assert_eq!(
            prepare(&fixture.state, "proposal", "tab2", ENDPOINT).unwrap_err(),
            "attestation_historical_anchor_missing"
        );
    }

    #[test]
    fn generation_overflow_refuses_prepare() {
        let mut fixture = physical_fixture();
        fixture
            .state
            .runtime_custody_receipts
            .get_mut("proposal")
            .unwrap()["ownerGeneration"] = json!(u64::MAX);
        assert_eq!(
            prepare(&fixture.state, "proposal", "tab2", ENDPOINT).unwrap_err(),
            "attestation_generation_overflow"
        );
    }

    #[test]
    fn resume_validates_optional_anchor_transition_and_legacy_compatibility() {
        let mut fixture = physical_fixture();
        let (_, mut custody) = prepare(&fixture.state, "proposal", "tab2", ENDPOINT)
            .unwrap()
            .unwrap();
        // A synthetic prior-source fixture is genuinely exited and reaped.
        let source = exited_identity(fixture._profile.path());
        custody["source"] = source.clone();
        fixture
            .state
            .runtime_custody_receipts
            .get_mut("proposal")
            .unwrap()["destination"] = source;
        verify_resume(
            &fixture.state,
            "proposal",
            &custody,
            fixture.pid,
            ENDPOINT,
            "tab2",
        )
        .unwrap();
        close_anchor(&mut fixture.state);
        verify_resume(
            &fixture.state,
            "proposal",
            &custody,
            fixture.pid,
            ENDPOINT,
            "tab2",
        )
        .unwrap();
        for bad in [
            Value::Null,
            json!("wrong"),
            json!({}),
            json!({"targetId":"wrong", "disposition":"live"}),
            json!({"targetId":"tab1", "disposition":"active"}),
        ] {
            let mut candidate = custody.clone();
            candidate["priorAnchor"] = bad;
            assert!(verify_resume(
                &fixture.state,
                "proposal",
                &candidate,
                fixture.pid,
                ENDPOINT,
                "tab2"
            )
            .is_err());
        }
        custody["priorAnchor"]["disposition"] = json!("closed_historical");
        verify_resume(
            &fixture.state,
            "proposal",
            &custody,
            fixture.pid,
            ENDPOINT,
            "tab2",
        )
        .unwrap();
        fixture.state.tabs.get_mut("target:tab1").unwrap().lifecycle = TabLifecycle::Ready;
        fixture
            .state
            .sessions
            .get_mut("proposal")
            .unwrap()
            .tab_ids
            .push("target:tab1".into());
        assert!(verify_resume(
            &fixture.state,
            "proposal",
            &custody,
            fixture.pid,
            ENDPOINT,
            "tab2"
        )
        .is_err());
        custody.as_object_mut().unwrap().remove("priorAnchor");
        verify_resume(
            &fixture.state,
            "proposal",
            &custody,
            fixture.pid,
            ENDPOINT,
            "tab2",
        )
        .unwrap();
        let receipt = committed(&custody).unwrap();
        assert_eq!(receipt["ownerGeneration"], 6);
        assert_eq!(
            receipt["destination"],
            current_process_identity(std::process::id()).unwrap()
        );
        fixture.state.runtime_custody_receipts.clear();
        custody["ownerGeneration"] = json!(1);
        custody["priorAnchor"] = json!({"targetId":"tab1", "disposition":"live"});
        assert_eq!(
            verify_resume(
                &fixture.state,
                "proposal",
                &custody,
                fixture.pid,
                ENDPOINT,
                "tab2"
            )
            .unwrap_err(),
            "attestation_unexpected_prior_anchor"
        );
    }
}
