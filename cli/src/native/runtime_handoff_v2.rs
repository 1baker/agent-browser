//! Compatibility verifier for custody-aware schema-v2 runtime handoffs.
//!
//! Some retained daemons may come from a generation that committed exact
//! process and browser custody receipts. A later writer must not downgrade
//! that descriptor to v1 or discard its receipt. This verifier admits the
//! prepared descriptor only after the former daemon is gone and the receipt,
//! browser, profile, CDP endpoint, service lease, and target still agree.

use serde_json::{json, Value};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use super::service_model::{LeaseState, ServiceState};

fn value_u32(value: &Value, key: &str) -> Result<u32, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| format!("runtime_handoff_v2_{key}_missing"))
}

fn start_ticks(pid: u32) -> Result<u64, String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| format!("runtime_handoff_v2_process_unreadable:{error}"))?;
    stat.rsplit_once(')')
        .and_then(|(_, tail)| tail.split_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "runtime_handoff_v2_process_start_unreadable".to_string())
}

fn current_process_identity(pid: u32) -> Result<Value, String> {
    let executable = fs::metadata(format!("/proc/{pid}/exe"))
        .map_err(|error| format!("runtime_handoff_v2_executable_unreadable:{error}"))?;
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|error| format!("runtime_handoff_v2_status_unreadable:{error}"))?;
    let uid = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|value| value.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| "runtime_handoff_v2_uid_missing".to_string())?;
    Ok(json!({
        "pid": pid,
        "bootId": fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map_err(|error| format!("runtime_handoff_v2_boot_id_unreadable:{error}"))?
            .trim(),
        "startTicks": start_ticks(pid)?,
        "executableDevice": executable.dev(),
        "executableInode": executable.ino(),
        "uid": uid,
    }))
}

fn require_process_gone(identity: &Value) -> Result<(), String> {
    let pid = value_u32(identity, "pid")?;
    let expected_boot = identity
        .get("bootId")
        .and_then(Value::as_str)
        .ok_or("runtime_handoff_v2_boot_id_missing")?;
    let current_boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(|error| format!("runtime_handoff_v2_boot_id_unreadable:{error}"))?;
    if current_boot.trim() != expected_boot {
        return Ok(());
    }
    match fs::metadata(format!("/proc/{pid}")) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("runtime_handoff_v2_process_unreadable:{error}")),
        Ok(_) if start_ticks(pid)? != identity["startTicks"].as_u64().unwrap_or_default() => Ok(()),
        Ok(_) => Err("runtime_handoff_v2_source_still_alive".into()),
    }
}

fn process_profile_argument(bytes: &[u8], executable: &std::path::Path) -> Result<String, String> {
    let mut arguments = bytes
        .split(|byte| *byte == 0)
        .filter(|argument| !argument.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("runtime_handoff_v2_cmdline_invalid:{error}"))?;
    // Chrome may replace its argv vector with one process-title string after
    // launch. Accept that only when it remains bound to the exact live exe.
    if arguments.len() == 1
        && bytes.ends_with(&[0])
        && !bytes[..bytes.len().saturating_sub(1)].contains(&0)
    {
        let title = arguments[0];
        let executable = executable
            .to_str()
            .ok_or("runtime_handoff_v2_executable_path_invalid")?;
        let prefix = format!("{executable} ");
        if !title.starts_with(&prefix) {
            return Err("runtime_handoff_v2_process_title_executable_mismatch".into());
        }
        arguments = title.split_whitespace().collect();
    }
    let mut profiles = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(value) = argument.strip_prefix("--user-data-dir=") {
            profiles.push(value);
        } else if *argument == "--user-data-dir" {
            profiles.push(arguments.get(index + 1).copied().unwrap_or_default());
        }
    }
    let [profile] = profiles.as_slice() else {
        return Err("runtime_handoff_v2_profile_argument_not_unique".into());
    };
    Ok(profile.to_string())
}

fn process_profile(pid: u32) -> Result<PathBuf, String> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline"))
        .map_err(|error| format!("runtime_handoff_v2_cmdline_unreadable:{error}"))?;
    let executable = fs::canonicalize(format!("/proc/{pid}/exe"))
        .map_err(|error| format!("runtime_handoff_v2_executable_unreadable:{error}"))?;
    let profile = process_profile_argument(&bytes, &executable)?;
    fs::canonicalize(profile)
        .map_err(|error| format!("runtime_handoff_v2_profile_unreadable:{error}"))
}

fn verify_browser(browser: &Value, pid: u32, cdp_url: &str) -> Result<PathBuf, String> {
    let process = browser
        .get("process")
        .ok_or("runtime_handoff_v2_browser_process_missing")?;
    if current_process_identity(pid)? != *process || browser["cdpEndpoint"] != cdp_url {
        return Err("runtime_handoff_v2_browser_identity_mismatch".into());
    }
    let profile = browser
        .get("canonicalProfile")
        .and_then(Value::as_str)
        .ok_or("runtime_handoff_v2_profile_missing")?;
    let canonical = fs::canonicalize(profile)
        .map_err(|error| format!("runtime_handoff_v2_profile_unreadable:{error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("runtime_handoff_v2_profile_unreadable:{error}"))?;
    if metadata.dev() != browser["profileDevice"].as_u64().unwrap_or_default()
        || metadata.ino() != browser["profileInode"].as_u64().unwrap_or_default()
        || process_profile(pid)? != canonical
    {
        return Err("runtime_handoff_v2_profile_identity_mismatch".into());
    }
    let endpoint = url::Url::parse(cdp_url)
        .map_err(|error| format!("runtime_handoff_v2_cdp_invalid:{error}"))?;
    if endpoint.scheme() != "ws"
        || endpoint.host_str() != Some("127.0.0.1")
        || endpoint.port().is_none()
        || endpoint
            .path()
            .strip_prefix("/devtools/browser/")
            .is_none_or(str::is_empty)
    {
        return Err("runtime_handoff_v2_cdp_not_loopback_browser".into());
    }
    let active = fs::read_to_string(canonical.join("DevToolsActivePort"))
        .map_err(|error| format!("runtime_handoff_v2_active_port_unreadable:{error}"))?;
    let mut lines = active.lines();
    if lines.next().and_then(|value| value.parse::<u16>().ok()) != endpoint.port()
        || lines.next() != Some(endpoint.path())
    {
        return Err("runtime_handoff_v2_active_port_mismatch".into());
    }
    Ok(canonical)
}

pub(super) fn verify(
    state: &ServiceState,
    session_name: &str,
    browser_pid: Option<u32>,
    runtime_profile: Option<&str>,
    cdp_url: &str,
    target_id: Option<&str>,
    custody: Option<&Value>,
) -> Result<(), String> {
    let pid = browser_pid.ok_or("runtime_handoff_v2_browser_pid_missing")?;
    let target_id = target_id.ok_or("runtime_handoff_v2_target_missing")?;
    let custody = custody.ok_or("runtime_handoff_v2_custody_missing")?;
    let source = custody
        .get("source")
        .ok_or("runtime_handoff_v2_source_missing")?;
    let browser = custody
        .get("browser")
        .ok_or("runtime_handoff_v2_browser_missing")?;
    let receipt = state
        .runtime_custody_receipts
        .get(session_name)
        .ok_or("runtime_handoff_v2_receipt_missing")?;
    if receipt["schemaVersion"] != 2
        || receipt["phase"] != "committed"
        || receipt["destination"] != *source
        || receipt["browser"] != *browser
        || receipt["targetId"] != target_id
        || receipt["descriptorSha256"].as_str().is_none_or(|digest| {
            digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("runtime_handoff_v2_receipt_binding_mismatch".into());
    }
    require_process_gone(source)?;
    let profile_path = verify_browser(browser, pid, cdp_url)?;

    let session = state
        .sessions
        .get(session_name)
        .ok_or("runtime_handoff_v2_session_missing")?;
    if session.lease != LeaseState::Exclusive
        || runtime_profile.is_some_and(|profile| session.profile_id.as_deref() != Some(profile))
        || state.sessions.values().any(|other| {
            other.id != session_name
                && other.profile_id == session.profile_id
                && !matches!(other.lease, LeaseState::Released | LeaseState::Expired)
        })
    {
        return Err("runtime_handoff_v2_exclusive_lease_required".into());
    }
    let profile_id = session
        .profile_id
        .as_deref()
        .ok_or("runtime_handoff_v2_profile_id_missing")?;
    let configured_profile = state
        .profiles
        .get(profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .ok_or("runtime_handoff_v2_service_profile_missing")?;
    if fs::canonicalize(configured_profile)
        .map_err(|_| "runtime_handoff_v2_service_profile_unreadable")?
        != profile_path
    {
        return Err("runtime_handoff_v2_service_profile_mismatch".into());
    }
    let browser_id = format!("session:{session_name}");
    let tab_id = format!("target:{target_id}");
    let record = state
        .browsers
        .get(&browser_id)
        .ok_or("runtime_handoff_v2_service_browser_missing")?;
    let tab = state
        .tabs
        .get(&tab_id)
        .ok_or("runtime_handoff_v2_service_target_missing")?;
    if record.pid != Some(pid)
        || record.cdp_endpoint.as_deref() != Some(cdp_url)
        || record.profile_id.as_deref() != Some(profile_id)
        || !session.browser_ids.contains(&browser_id)
        || !session.tab_ids.contains(&tab_id)
        || tab.browser_id != browser_id
        || tab.owner_session_id.as_deref() != Some(session_name)
        || tab.target_id.as_deref() != Some(target_id)
    {
        return Err("runtime_handoff_v2_service_identity_mismatch".into());
    }
    Ok(())
}

pub(super) fn verify_stale_snapshot_recovery(
    state: &ServiceState,
    session_name: &str,
    browser_pid: Option<u32>,
    runtime_profile: Option<&str>,
    cdp_url: &str,
    target_id: Option<&str>,
    custody: Option<&Value>,
) -> Result<(), String> {
    let pid = browser_pid.ok_or("runtime_handoff_v3_browser_pid_missing")?;
    let target_id = target_id.ok_or("runtime_handoff_v3_target_missing")?;
    let custody = custody.ok_or("runtime_handoff_v3_custody_missing")?;
    let source = custody
        .get("source")
        .ok_or("runtime_handoff_v3_source_missing")?;
    let browser = custody
        .get("browser")
        .ok_or("runtime_handoff_v3_browser_missing")?;
    let receipt = state
        .runtime_custody_receipts
        .get(session_name)
        .ok_or("runtime_handoff_v3_stale_receipt_missing")?;
    let stale_target = receipt
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or("runtime_handoff_v3_stale_target_missing")?;
    if receipt["schemaVersion"] != 2
        || receipt["phase"] != "committed"
        || receipt["destination"] != *source
        || receipt["browser"] != *browser
        || stale_target == target_id
        || state
            .tabs
            .get(&format!("target:{stale_target}"))
            .is_some_and(|tab| tab.lifecycle == super::service_model::TabLifecycle::Ready)
    {
        return Err("runtime_handoff_v3_stale_receipt_mismatch".into());
    }
    require_process_gone(source)?;
    let profile_path = verify_browser(browser, pid, cdp_url)?;
    let session = state
        .sessions
        .get(session_name)
        .ok_or("runtime_handoff_v3_session_missing")?;
    let profile_id = session
        .profile_id
        .as_deref()
        .ok_or("runtime_handoff_v3_profile_id_missing")?;
    if session.lease != LeaseState::Exclusive
        || runtime_profile.is_some_and(|profile| profile != profile_id)
        || state.sessions.values().any(|other| {
            other.id != session_name
                && other.profile_id == session.profile_id
                && !matches!(other.lease, LeaseState::Released | LeaseState::Expired)
        })
        || state
            .profiles
            .get(profile_id)
            .and_then(|profile| profile.user_data_dir.as_deref())
            .and_then(|path| fs::canonicalize(path).ok())
            .as_ref()
            != Some(&profile_path)
    {
        return Err("runtime_handoff_v3_exclusive_profile_mismatch".into());
    }
    let browser_id = format!("session:{session_name}");
    let tab_id = format!("target:{target_id}");
    let record = state
        .browsers
        .get(&browser_id)
        .ok_or("runtime_handoff_v3_service_browser_missing")?;
    let tab = state
        .tabs
        .get(&tab_id)
        .ok_or("runtime_handoff_v3_service_target_missing")?;
    if record.pid != Some(pid)
        || record.cdp_endpoint.as_deref() != Some(cdp_url)
        || record.profile_id.as_deref() != Some(profile_id)
        || !session.browser_ids.contains(&browser_id)
        || !session.tab_ids.contains(&tab_id)
        || tab.browser_id != browser_id
        || tab.owner_session_id.as_deref() != Some(session_name)
        || tab.target_id.as_deref() != Some(target_id)
        || tab.lifecycle != super::service_model::TabLifecycle::Ready
    {
        return Err("runtime_handoff_v3_service_identity_mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::process_profile_argument;
    use std::path::Path;

    #[test]
    fn accepts_executable_bound_collapsed_chrome_title() {
        let executable = Path::new("/opt/agent-browser/chrome");
        let bytes =
            b"/opt/agent-browser/chrome --remote-debugging-port=0 --user-data-dir=/tmp/profile\0";
        assert_eq!(
            process_profile_argument(bytes, executable).unwrap(),
            "/tmp/profile"
        );
    }

    #[test]
    fn rejects_collapsed_title_for_another_executable() {
        let executable = Path::new("/opt/agent-browser/chrome");
        let bytes = b"/usr/bin/other --user-data-dir=/tmp/profile\0";
        assert_eq!(
            process_profile_argument(bytes, executable).unwrap_err(),
            "runtime_handoff_v2_process_title_executable_mismatch"
        );
    }
}
