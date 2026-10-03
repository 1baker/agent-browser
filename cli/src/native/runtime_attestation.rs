//! Linux-only proof for an exact retained-browser daemon handoff.
//!
//! A legacy handoff is deliberately not proof. The first install may consume
//! schema v1; only a subsequent verified v4 handoff can mint a receipt.

use serde_json::{json, Map, Value};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use super::runtime_handoff_v2::{current_process_identity, require_process_gone, verify_browser};
use super::service_model::{BrowserHost, LeaseState, ServiceState};

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

fn binding(
    state: &ServiceState,
    session_name: &str,
    profile_id: &str,
    browser_id: &str,
    target_id: &str,
    pid: u32,
    endpoint: &str,
) -> Result<(), String> {
    let session = state
        .sessions
        .get(session_name)
        .ok_or("attestation_session_missing")?;
    let tab_id = format!("target:{target_id}");
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or("attestation_browser_missing")?;
    let tab = state.tabs.get(&tab_id).ok_or("attestation_tab_missing")?;
    if session.lease != LeaseState::Exclusive
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
        || !session.tab_ids.iter().any(|id| id == &tab_id)
        || browser.id != browser_id
        || browser.profile_id.as_deref() != Some(profile_id)
        || browser.pid != Some(pid)
        || browser.cdp_endpoint.as_deref() != Some(endpoint)
        || browser.host == BrowserHost::RemoteHeaded
        || tab.browser_id != browser_id
        || tab.target_id.as_deref() != Some(target_id)
        || tab.owner_session_id.as_deref() != Some(session_name)
    {
        return Err("attestation_service_binding_mismatch".into());
    }
    Ok(())
}

fn verify_custody(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    target_id: &str,
) -> Result<(), String> {
    let profile_id = field(custody, "profileId")?;
    let browser_id = field(custody, "browserId")?;
    if field(custody, "targetId")? != target_id || field(custody, "cdpEndpoint")? != endpoint {
        return Err("attestation_target_or_endpoint_mismatch".into());
    }
    binding(
        state,
        session_name,
        profile_id,
        browser_id,
        target_id,
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
    let generation = if let Some(previous) = state.runtime_custody_receipts.get(session_name) {
        let prior_target = field(previous, "targetId")?;
        verify_committed(state, session_name, previous, pid, endpoint, prior_target)?;
        previous["ownerGeneration"]
            .as_u64()
            .ok_or("attestation_generation_missing")?
            + 1
    } else {
        1
    };
    Ok(Some((
        pid,
        json!({
            "source": current_process_identity(std::process::id())?,
            "browser": browser,
            "browserId": browser_id,
            "profileId": profile_id,
            "targetId": target_id,
            "cdpEndpoint": endpoint,
            "ownerGeneration": generation,
        }),
    )))
}

pub(super) fn verify_resume(
    state: &ServiceState,
    session_name: &str,
    custody: &Value,
    pid: u32,
    endpoint: &str,
    target_id: &str,
) -> Result<(), String> {
    let source = custody.get("source").ok_or("attestation_source_missing")?;
    wait_for_source_exit(source)?;
    verify_custody(state, session_name, custody, pid, endpoint, target_id)?;
    if let Some(previous) = state.runtime_custody_receipts.get(session_name) {
        if previous["schemaVersion"] != 4
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
    } else if custody["ownerGeneration"] != 1 {
        return Err("attestation_missing_prior_receipt".into());
    }
    Ok(())
}

pub(super) fn committed(custody: &Value) -> Result<Value, String> {
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
) -> Result<(), String> {
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
    verify_custody(state, session_name, receipt, pid, endpoint, target_id)
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
        verify_committed(state, session_name, receipt, pid, endpoint, handoff_target)?;
        Ok(json!({
            "complete": true,
            "missingProofs": [],
            "ownerCustody": {"verified": true, "basis": "schema_v4_committed_handoff", "ownerGeneration": receipt["ownerGeneration"]},
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
        let handle = serde_json::from_value::<Map<String, Value>>(json!({
            "browserId": "session:proposal", "tabId": "target:tab2",
            "targetId": "tab2", "profileId": "retained",
            "leaseState": "shared", "profileOrigin": "external_byop"
        }))
        .unwrap();
        let result = diagnostics(Some(&state), &handle, "proposal", Some("tab2"));
        assert_eq!(result["complete"], false);
        assert_eq!(result["missingProofs"][0], "attestation_receipt_missing");
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
                "tab1"
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
                "tab1"
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
}
