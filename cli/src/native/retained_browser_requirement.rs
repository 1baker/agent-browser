use crate::connection::get_socket_dir;
use crate::runtime_profile::pid_is_running;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

pub const LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA: &str =
    "agent-browser.local-dashboard-retained-browser-requirement.v1";
pub const LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA: &str =
    "agent-browser.local-dashboard-retained-browser-enforcement.v1";
pub const LOCAL_DASHBOARD_RETAINED_BROWSER_ROTATION_SCHEMA: &str =
    "agent-browser.local-dashboard-retained-browser-rotation.v1";

const MAX_REQUIREMENT_BYTES: u64 = 16 * 1024;
const MAX_DAEMON_PID_BYTES: u64 = 64;
const MAX_SERVICE_STATE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CDP_TARGET_BYTES: usize = 4 * 1024 * 1024;
const CDP_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedBrowserRequirementStatus {
    pub schema_version: &'static str,
    pub configured: bool,
    pub verified: bool,
    pub state: &'static str,
    pub requirement_path: String,
    pub enforcement_configured: bool,
    pub enforcement_path: String,
    pub checked_without_launch: bool,
    pub rotation_configured: bool,
    pub rotation_path: String,
    pub rotation_phase: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RequirementRecord {
    schema_version: String,
    created_at: String,
    expectation: RetainedBrowserExpectation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct EnforcementRecord {
    schema_version: String,
    created_at: String,
    requirement_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RetainedBrowserExpectation {
    session_name: String,
    profile_id: String,
    target_id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RotationRecord {
    schema_version: String,
    operation_id: String,
    created_at: String,
    expected_old_sha256: String,
    next_requirement_sha256: String,
    stale_reason: String,
    phase: String,
    old_expectation: RetainedBrowserExpectation,
    next_requirement: RequirementRecord,
}

pub fn verify_retained_browser_requirement_for_root(
    root: &Path,
) -> Result<RetainedBrowserRequirementStatus, String> {
    let requirement_path = retained_browser_requirement_path_for_root(root)?;
    verify_retained_browser_requirement_for_path(root, &requirement_path)
}

pub fn retained_browser_requirement_path_for_root(root: &Path) -> Result<PathBuf, String> {
    let Some(configured) = std::env::var_os("AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT") else {
        return Ok(root.join(".agent-browser/publications/local-dashboard-retained-browser.json"));
    };
    if configured.is_empty() {
        return Ok(root.join(".agent-browser/publications/local-dashboard-retained-browser.json"));
    }
    let configured = PathBuf::from(configured);
    if configured.is_absolute() {
        return Ok(configured);
    }
    std::env::current_dir()
        .map(|current| current.join(configured))
        .map_err(|_| failure("retained_browser_requirement_path_invalid"))
}

pub fn retained_browser_enforcement_path(requirement_path: &Path) -> PathBuf {
    let mut path = requirement_path.as_os_str().to_os_string();
    path.push(".required");
    PathBuf::from(path)
}

pub fn retained_browser_rotation_path(requirement_path: &Path) -> PathBuf {
    let mut path = requirement_path.as_os_str().to_os_string();
    path.push(".rotation.json");
    PathBuf::from(path)
}

fn verify_retained_browser_requirement_for_path(
    root: &Path,
    requirement_path: &Path,
) -> Result<RetainedBrowserRequirementStatus, String> {
    verify_retained_browser_requirement_at_paths(root, requirement_path, &get_socket_dir())
}

fn verify_retained_browser_requirement_at_paths(
    root: &Path,
    requirement_path: &Path,
    socket_dir: &Path,
) -> Result<RetainedBrowserRequirementStatus, String> {
    let enforcement_path = retained_browser_enforcement_path(requirement_path);
    let rotation_path = retained_browser_rotation_path(requirement_path);
    let enforcement = read_enforcement(&enforcement_path)?;
    let rotation = read_rotation(&rotation_path)?;
    let enforcement_configured = enforcement.is_some();
    match fs::symlink_metadata(requirement_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if rotation.is_some() {
                return Err(failure("retained_browser_rotation_requirement_missing"));
            }
            if enforcement_configured {
                return Err(failure("retained_browser_requirement_missing"));
            }
            return Ok(RetainedBrowserRequirementStatus {
                schema_version: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
                configured: false,
                verified: false,
                state: "not_configured",
                requirement_path: requirement_path.display().to_string(),
                enforcement_configured,
                enforcement_path: enforcement_path.display().to_string(),
                checked_without_launch: true,
                rotation_configured: false,
                rotation_path: rotation_path.display().to_string(),
                rotation_phase: None,
            });
        }
        Err(_) => return Err(failure("retained_browser_requirement_invalid")),
    }

    let requirement_bytes = read_bounded_regular_file(
        requirement_path,
        MAX_REQUIREMENT_BYTES,
        true,
        "retained_browser_requirement_invalid",
        "retained_browser_requirement_permissions_invalid",
    )?;
    let record: RequirementRecord = serde_json::from_slice(&requirement_bytes)
        .map_err(|_| failure("retained_browser_requirement_invalid"))?;
    validate_requirement(&record)?;
    let requirement_sha256 = format!("{:x}", Sha256::digest(&requirement_bytes));
    if let Some(rotation) = rotation.as_ref() {
        let enforcement = enforcement
            .as_ref()
            .ok_or_else(|| failure("retained_browser_rotation_enforcement_missing"))?;
        validate_rotation(rotation)?;
        validate_rotation_pair(
            &requirement_sha256,
            &enforcement.requirement_sha256,
            rotation,
        )?;
        return Ok(RetainedBrowserRequirementStatus {
            schema_version: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
            configured: true,
            verified: false,
            state: "rotation_pending",
            requirement_path: requirement_path.display().to_string(),
            enforcement_configured,
            enforcement_path: enforcement_path.display().to_string(),
            checked_without_launch: true,
            rotation_configured: true,
            rotation_path: rotation_path.display().to_string(),
            rotation_phase: Some(rotation.phase.clone()),
        });
    }
    if let Some(enforcement) = enforcement.as_ref() {
        if requirement_sha256 != enforcement.requirement_sha256 {
            return Err(failure("retained_browser_enforcement_digest_mismatch"));
        }
    }

    verify_retained_daemon(socket_dir, &record.expectation.session_name)?;

    let service_state_path = root.join(".agent-browser/service/state.json");
    let state_bytes = read_bounded_regular_file(
        &service_state_path,
        MAX_SERVICE_STATE_BYTES,
        false,
        "retained_browser_service_state_missing_or_invalid",
        "retained_browser_service_state_missing_or_invalid",
    )?;
    let state: Value = serde_json::from_slice(&state_bytes)
        .map_err(|_| failure("retained_browser_service_state_missing_or_invalid"))?;
    verify_persisted_browser(&state, &record.expectation)?;

    Ok(RetainedBrowserRequirementStatus {
        schema_version: LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
        configured: true,
        verified: true,
        state: "verified",
        requirement_path: requirement_path.display().to_string(),
        enforcement_configured,
        enforcement_path: enforcement_path.display().to_string(),
        checked_without_launch: true,
        rotation_configured: false,
        rotation_path: rotation_path.display().to_string(),
        rotation_phase: None,
    })
}

fn verify_retained_daemon(socket_dir: &Path, session_name: &str) -> Result<(), String> {
    let pid_path = socket_dir.join(format!("{session_name}.pid"));
    let pid_bytes = read_bounded_regular_file(
        &pid_path,
        MAX_DAEMON_PID_BYTES,
        false,
        "retained_daemon_missing",
        "retained_daemon_missing",
    )?;
    let pid = std::str::from_utf8(&pid_bytes)
        .ok()
        .map(str::trim)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| failure("retained_daemon_missing"))?;
    if !pid_is_running(pid) {
        return Err(failure("retained_daemon_missing"));
    }
    Ok(())
}

fn read_enforcement(path: &Path) -> Result<Option<EnforcementRecord>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("retained_browser_enforcement_invalid")),
    }
    let bytes = read_bounded_regular_file(
        path,
        MAX_REQUIREMENT_BYTES,
        true,
        "retained_browser_enforcement_invalid",
        "retained_browser_enforcement_permissions_invalid",
    )?;
    let record: EnforcementRecord = serde_json::from_slice(&bytes)
        .map_err(|_| failure("retained_browser_enforcement_invalid"))?;
    if record.schema_version != LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA {
        return Err(failure("retained_browser_enforcement_schema_unsupported"));
    }
    DateTime::parse_from_rfc3339(&record.created_at)
        .map_err(|_| failure("retained_browser_enforcement_created_at_invalid"))?;
    if record.requirement_sha256.len() != 64
        || !record
            .requirement_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(failure(
            "retained_browser_enforcement_requirement_sha256_invalid",
        ));
    }
    Ok(Some(record))
}

fn read_rotation(path: &Path) -> Result<Option<RotationRecord>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("retained_browser_rotation_invalid")),
    }
    let bytes = read_bounded_regular_file(
        path,
        MAX_REQUIREMENT_BYTES,
        true,
        "retained_browser_rotation_invalid",
        "retained_browser_rotation_permissions_invalid",
    )?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| failure("retained_browser_rotation_invalid"))
}

fn validate_rotation(record: &RotationRecord) -> Result<(), String> {
    if record.schema_version != LOCAL_DASHBOARD_RETAINED_BROWSER_ROTATION_SCHEMA {
        return Err(failure("retained_browser_rotation_schema_unsupported"));
    }
    DateTime::parse_from_rfc3339(&record.created_at)
        .map_err(|_| failure("retained_browser_rotation_created_at_invalid"))?;
    validate_sha256(
        &record.expected_old_sha256,
        "retained_browser_rotation_old_digest_invalid",
    )?;
    validate_sha256(
        &record.next_requirement_sha256,
        "retained_browser_rotation_next_digest_invalid",
    )?;
    let expected_operation_id = format!(
        "{}-{}",
        &record.expected_old_sha256[..12],
        &record.next_requirement_sha256[..12]
    );
    if record.operation_id != expected_operation_id {
        return Err(failure("retained_browser_rotation_operation_id_invalid"));
    }
    if !matches!(
        record.stale_reason.as_str(),
        "retained_daemon_missing" | "retained_browser_missing"
    ) {
        return Err(failure("retained_browser_rotation_stale_reason_invalid"));
    }
    if !matches!(
        record.phase.as_str(),
        "prepared" | "requirement_replaced" | "enforcement_replaced" | "committed"
    ) {
        return Err(failure("retained_browser_rotation_phase_invalid"));
    }
    validate_expectation(&record.old_expectation)?;
    validate_requirement(&record.next_requirement)?;
    let mut serialized = serde_json::to_vec_pretty(&record.next_requirement)
        .map_err(|_| failure("retained_browser_rotation_next_requirement_invalid"))?;
    serialized.push(b'\n');
    let next_sha256 = format!("{:x}", Sha256::digest(serialized));
    if next_sha256 != record.next_requirement_sha256 {
        return Err(failure(
            "retained_browser_rotation_next_requirement_digest_mismatch",
        ));
    }
    Ok(())
}

fn validate_rotation_pair(
    requirement_sha256: &str,
    enforcement_sha256: &str,
    rotation: &RotationRecord,
) -> Result<(), String> {
    let old = rotation.expected_old_sha256.as_str();
    let next = rotation.next_requirement_sha256.as_str();
    if !matches!(requirement_sha256, value if value == old || value == next)
        || !matches!(enforcement_sha256, value if value == old || value == next)
    {
        return Err(failure("retained_browser_rotation_digest_conflict"));
    }
    if requirement_sha256 == old && enforcement_sha256 == next {
        return Err(failure("retained_browser_rotation_order_invalid"));
    }
    let phase_matches = match rotation.phase.as_str() {
        "prepared" => {
            (requirement_sha256 == old || requirement_sha256 == next) && enforcement_sha256 == old
        }
        "requirement_replaced" => {
            requirement_sha256 == next && (enforcement_sha256 == old || enforcement_sha256 == next)
        }
        "enforcement_replaced" | "committed" => {
            requirement_sha256 == next && enforcement_sha256 == next
        }
        _ => false,
    };
    if !phase_matches {
        return Err(failure("retained_browser_rotation_phase_conflict"));
    }
    Ok(())
}

fn validate_sha256(value: &str, code: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(failure(code))
    }
}

fn validate_requirement(record: &RequirementRecord) -> Result<(), String> {
    if record.schema_version != LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA {
        return Err(failure("retained_browser_requirement_schema_unsupported"));
    }
    DateTime::parse_from_rfc3339(&record.created_at)
        .map_err(|_| failure("retained_browser_requirement_created_at_invalid"))?;
    validate_expectation(&record.expectation)
}

fn validate_expectation(expectation: &RetainedBrowserExpectation) -> Result<(), String> {
    if !valid_session_name(&expectation.session_name) {
        return Err(failure("retained_browser_session_invalid"));
    }
    if expectation.profile_id.is_empty() || expectation.target_id.is_empty() {
        return Err(failure("retained_browser_identity_incomplete"));
    }
    let expected_url =
        Url::parse(&expectation.url).map_err(|_| failure("retained_target_url_invalid"))?;
    if !matches!(expected_url.scheme(), "http" | "https") {
        return Err(failure("retained_target_url_invalid"));
    }
    Ok(())
}

fn verify_persisted_browser(
    state: &Value,
    expectation: &RetainedBrowserExpectation,
) -> Result<(), String> {
    let browser_id = format!("session:{}", expectation.session_name);
    let browsers = state
        .get("browsers")
        .and_then(Value::as_object)
        .ok_or_else(|| failure("retained_browser_service_state_missing_or_invalid"))?;
    let browser = browsers
        .get(&browser_id)
        .and_then(Value::as_object)
        .ok_or_else(|| failure("retained_browser_missing"))?;

    if browser.get("id").and_then(Value::as_str) != Some(browser_id.as_str()) {
        return Err(failure("retained_browser_id_changed"));
    }
    if browser.get("profileId").and_then(Value::as_str) != Some(expectation.profile_id.as_str()) {
        return Err(failure("retained_browser_profile_changed"));
    }
    if browser.get("health").and_then(Value::as_str) != Some("ready") {
        return Err(failure("retained_browser_not_ready"));
    }
    let pid = browser
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| failure("retained_browser_pid_missing"))?;
    if !pid_is_running(pid) {
        return Err(failure("retained_browser_process_exited"));
    }

    let active_session_matches = browser
        .get("activeSessionIds")
        .and_then(Value::as_array)
        .map(|sessions| {
            sessions
                .iter()
                .filter(|value| value.as_str() == Some(expectation.session_name.as_str()))
                .count()
        })
        .unwrap_or(0);
    if active_session_matches != 1 {
        return Err(failure("retained_browser_session_changed"));
    }

    let matching_handles = browser
        .get("tabHandles")
        .and_then(Value::as_array)
        .map(|handles| {
            handles
                .iter()
                .filter(|handle| {
                    handle.get("browserId").and_then(Value::as_str) == Some(browser_id.as_str())
                        && handle.get("sessionName").and_then(Value::as_str)
                            == Some(expectation.session_name.as_str())
                        && handle.get("targetId").and_then(Value::as_str)
                            == Some(expectation.target_id.as_str())
                        && handle.get("profileId").and_then(Value::as_str)
                            == Some(expectation.profile_id.as_str())
                        && handle.get("url").and_then(Value::as_str)
                            == Some(expectation.url.as_str())
                        && handle.get("valid").and_then(Value::as_bool) == Some(true)
                })
                .count()
        })
        .unwrap_or(0);
    if matching_handles != 1 {
        return Err(failure(if matching_handles == 0 {
            "retained_service_tab_handle_missing_or_changed"
        } else {
            "retained_service_tab_handle_ambiguous"
        }));
    }

    let cdp_endpoint = browser
        .get("cdpEndpoint")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| failure("retained_browser_cdp_missing"))?;
    let targets = fetch_loopback_cdp_targets(cdp_endpoint)?;
    let matches: Vec<&Value> = targets
        .iter()
        .filter(|target| {
            target.get("id").and_then(Value::as_str) == Some(expectation.target_id.as_str())
        })
        .collect();
    if matches.len() != 1 {
        return Err(failure(if matches.is_empty() {
            "retained_target_missing"
        } else {
            "retained_target_ambiguous"
        }));
    }
    if matches[0].get("url").and_then(Value::as_str) != Some(expectation.url.as_str()) {
        return Err(failure("retained_target_url_changed"));
    }
    Ok(())
}

fn fetch_loopback_cdp_targets(cdp_endpoint: &str) -> Result<Vec<Value>, String> {
    let mut endpoint =
        Url::parse(cdp_endpoint).map_err(|_| failure("retained_browser_cdp_invalid"))?;
    if endpoint.scheme() != "ws" || endpoint.port().is_none() {
        return Err(failure("retained_browser_cdp_not_loopback"));
    }
    let host = endpoint
        .host_str()
        .ok_or_else(|| failure("retained_browser_cdp_not_loopback"))?;
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(failure("retained_browser_cdp_not_loopback"));
    }
    endpoint
        .set_scheme("http")
        .map_err(|_| failure("retained_browser_cdp_invalid"))?;
    endpoint.set_path("/json/list");
    endpoint.set_query(None);
    endpoint.set_fragment(None);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| failure("retained_browser_cdp_unreachable"))?;
    runtime.block_on(async move {
        let client = reqwest::Client::builder()
            .connect_timeout(CDP_REQUEST_TIMEOUT)
            .timeout(CDP_REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| failure("retained_browser_cdp_unreachable"))?;
        let mut response = client
            .get(endpoint)
            .send()
            .await
            .map_err(|_| failure("retained_browser_cdp_unreachable"))?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(failure("retained_browser_cdp_unreachable"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CDP_TARGET_BYTES as u64)
        {
            return Err(failure("retained_browser_cdp_response_too_large"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failure("retained_browser_cdp_unreachable"))?
        {
            if body.len().saturating_add(chunk.len()) > MAX_CDP_TARGET_BYTES {
                return Err(failure("retained_browser_cdp_response_too_large"));
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice::<Vec<Value>>(&body)
            .map_err(|_| failure("retained_browser_cdp_response_invalid"))
    })
}

fn read_bounded_regular_file(
    path: &Path,
    max_bytes: u64,
    require_private_mode: bool,
    error_code: &'static str,
    permission_error_code: &'static str,
) -> Result<Vec<u8>, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let mut file = options.open(path).map_err(|_| failure(error_code))?;
    let metadata = file.metadata().map_err(|_| failure(error_code))?;
    if !metadata.file_type().is_file() || metadata.len() > max_bytes {
        return Err(failure(error_code));
    }
    #[cfg(unix)]
    {
        if metadata.uid() != unsafe { libc::geteuid() } as u32 {
            return Err(failure(error_code));
        }
        if require_private_mode && metadata.mode() & 0o077 != 0 {
            return Err(failure(permission_error_code));
        }
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|_| failure(error_code))?;
    if bytes.len() as u64 > max_bytes {
        return Err(failure(error_code));
    }
    Ok(bytes)
}

fn valid_session_name(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn failure(code: &str) -> String {
    format!("Retained browser requirement verification failed: {code}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-retained-requirement-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn requirement_path(root: &Path) -> PathBuf {
        root.join(".agent-browser/publications/local-dashboard-retained-browser.json")
    }

    fn verify_fixture(root: &Path) -> Result<RetainedBrowserRequirementStatus, String> {
        verify_retained_browser_requirement_at_paths(
            root,
            &requirement_path(root),
            &root.join("runtime"),
        )
    }

    fn write_private_json(path: &Path, value: &Value) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut bytes = serde_json::to_vec_pretty(value).unwrap();
        bytes.push(b'\n');
        fs::write(path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn write_requirement_record(path: &Path, value: &Value) {
        let record: RequirementRecord = serde_json::from_value(value.clone()).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut bytes = serde_json::to_vec_pretty(&record).unwrap();
        bytes.push(b'\n');
        fs::write(path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn write_enforcement(requirement: &Path) {
        let requirement_sha256 = fs::read(requirement)
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
            .unwrap_or_else(|_| "0".repeat(64));
        write_private_json(
            &retained_browser_enforcement_path(requirement),
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
                "createdAt": "2026-08-15T11:59:59.000Z",
                "requirementSha256": requirement_sha256
            }),
        );
    }

    fn write_rotation(requirement: &Path, next_requirement: &Value, phase: &str) -> String {
        let old_sha256 = format!("{:x}", Sha256::digest(fs::read(requirement).unwrap()));
        let next_record: RequirementRecord =
            serde_json::from_value(next_requirement.clone()).unwrap();
        let mut next_bytes = serde_json::to_vec_pretty(&next_record).unwrap();
        next_bytes.push(b'\n');
        let next_sha256 = format!("{:x}", Sha256::digest(next_bytes));
        write_private_json(
            &retained_browser_rotation_path(requirement),
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_ROTATION_SCHEMA,
                "operationId": format!("{}-{}", &old_sha256[..12], &next_sha256[..12]),
                "createdAt": "2026-08-16T12:00:00.000Z",
                "expectedOldSha256": old_sha256,
                "nextRequirementSha256": next_sha256,
                "staleReason": "retained_daemon_missing",
                "phase": phase,
                "oldExpectation": {
                    "sessionName": "workshop-retained",
                    "profileId": "chatgpt-pro",
                    "targetId": "target-exact",
                    "url": "https://chatgpt.test/c/exact"
                },
                "nextRequirement": next_requirement
            }),
        );
        next_sha256
    }

    fn start_cdp(targets: Value) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let count = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /json/list "));
            let body = serde_json::to_vec(&targets).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        (
            format!("ws://127.0.0.1:{}/devtools/browser/fixture", address.port()),
            handle,
        )
    }

    fn seed_fixture(root: &Path, cdp_endpoint: &str, target_url: &str) {
        fs::create_dir_all(root.join("runtime")).unwrap();
        fs::write(
            root.join("runtime/workshop-retained.pid"),
            std::process::id().to_string(),
        )
        .unwrap();
        write_private_json(
            &requirement_path(root),
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
                "createdAt": "2026-08-15T12:00:00.000Z",
                "expectation": {
                    "sessionName": "workshop-retained",
                    "profileId": "chatgpt-pro",
                    "targetId": "target-exact",
                    "url": "https://chatgpt.test/c/exact"
                }
            }),
        );
        write_private_json(
            &root.join(".agent-browser/service/state.json"),
            &json!({
                "browsers": {
                    "session:workshop-retained": {
                        "id": "session:workshop-retained",
                        "profileId": "chatgpt-pro",
                        "health": "ready",
                        "pid": std::process::id(),
                        "cdpEndpoint": cdp_endpoint,
                        "activeSessionIds": ["workshop-retained"],
                        "tabHandles": [{
                            "browserId": "session:workshop-retained",
                            "sessionName": "workshop-retained",
                            "targetId": "target-exact",
                            "profileId": "chatgpt-pro",
                            "url": target_url,
                            "valid": true
                        }]
                    }
                }
            }),
        );
    }

    #[test]
    fn stale_service_state_without_live_daemon_fails_before_cdp_access() {
        let root = fixture_root("missing-daemon");
        seed_fixture(
            &root,
            "ws://127.0.0.1:9/devtools/browser/not-contacted",
            "https://chatgpt.test/c/exact",
        );
        fs::remove_file(root.join("runtime/workshop-retained.pid")).unwrap();

        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_daemon_missing"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn absent_requirement_is_read_only_and_not_configured() {
        let root = fixture_root("absent");
        let publications = root.join(".agent-browser/publications");
        let status = verify_fixture(&root).unwrap();
        assert!(!status.configured);
        assert!(!status.enforcement_configured);
        assert_eq!(status.state, "not_configured");
        assert!(!publications.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_private_requirement_verifies_without_launch() {
        let root = fixture_root("exact");
        let (cdp_endpoint, server) = start_cdp(json!([{
            "id": "target-exact",
            "type": "page",
            "url": "https://chatgpt.test/c/exact"
        }]));
        seed_fixture(&root, &cdp_endpoint, "https://chatgpt.test/c/exact");
        write_enforcement(&requirement_path(&root));
        let status = verify_fixture(&root).unwrap();
        assert!(status.configured);
        assert!(status.verified);
        assert!(status.checked_without_launch);
        assert!(status.enforcement_configured);
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enforcement_digest_rejects_replaced_requirement_before_state_access() {
        let root = fixture_root("enforcement-digest-mismatch");
        let requirement = requirement_path(&root);
        write_private_json(
            &requirement,
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
                "createdAt": "2026-08-15T12:00:00.000Z",
                "expectation": {
                    "sessionName": "workshop-retained",
                    "profileId": "chatgpt-pro",
                    "targetId": "target-original",
                    "url": "https://chatgpt.test/c/exact"
                }
            }),
        );
        write_enforcement(&requirement);
        let mut replacement: Value =
            serde_json::from_slice(&fs::read(&requirement).unwrap()).unwrap();
        replacement["expectation"]["targetId"] = json!("target-replaced");
        write_private_json(&requirement, &replacement);

        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_browser_enforcement_digest_mismatch"));
        assert!(!root.join(".agent-browser/service").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wrong_service_handle_url_fails_before_cdp_access() {
        let root = fixture_root("wrong-handle-url");
        seed_fixture(
            &root,
            "ws://127.0.0.1:9/devtools/browser/not-contacted",
            "https://chatgpt.test/c/wrong",
        );
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_service_tab_handle_missing_or_changed"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_handle_with_wrong_rendered_url_fails_closed() {
        let root = fixture_root("wrong-rendered-url");
        let (cdp_endpoint, server) = start_cdp(json!([{
            "id": "target-exact",
            "type": "page",
            "url": "https://chatgpt.test/c/wrong"
        }]));
        seed_fixture(&root, &cdp_endpoint, "https://chatgpt.test/c/exact");
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_target_url_changed"));
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_target_identity_fails_closed() {
        let root = fixture_root("duplicate-target");
        let (cdp_endpoint, server) = start_cdp(json!([
            {"id": "target-exact", "url": "https://chatgpt.test/c/exact"},
            {"id": "target-exact", "url": "https://chatgpt.test/c/exact"}
        ]));
        seed_fixture(&root, &cdp_endpoint, "https://chatgpt.test/c/exact");
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_target_ambiguous"));
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_target_identity_fails_closed() {
        let root = fixture_root("missing-target");
        let (cdp_endpoint, server) = start_cdp(json!([{
            "id": "different-target",
            "url": "https://chatgpt.test/c/exact"
        }]));
        seed_fixture(&root, &cdp_endpoint, "https://chatgpt.test/c/exact");
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_target_missing"));
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changed_browser_session_or_handle_target_fails_before_cdp_access() {
        for (label, pointer, value, expected) in [
            (
                "browser-id",
                "/browsers/session:workshop-retained/id",
                json!("session:other"),
                "retained_browser_id_changed",
            ),
            (
                "active-session",
                "/browsers/session:workshop-retained/activeSessionIds",
                json!(["other-session"]),
                "retained_browser_session_changed",
            ),
            (
                "handle-target",
                "/browsers/session:workshop-retained/tabHandles/0/targetId",
                json!("different-target"),
                "retained_service_tab_handle_missing_or_changed",
            ),
        ] {
            let root = fixture_root(label);
            seed_fixture(
                &root,
                "ws://127.0.0.1:9/devtools/browser/not-contacted",
                "https://chatgpt.test/c/exact",
            );
            let state_path = root.join(".agent-browser/service/state.json");
            let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
            *state.pointer_mut(pointer).unwrap() = value;
            write_private_json(&state_path, &state);
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains(expected));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn changed_profile_or_nonready_health_fails_before_cdp_access() {
        for (label, field, value, expected) in [
            (
                "profile",
                "profileId",
                json!("other-profile"),
                "retained_browser_profile_changed",
            ),
            (
                "health",
                "health",
                json!("reconnecting"),
                "retained_browser_not_ready",
            ),
        ] {
            let root = fixture_root(label);
            seed_fixture(
                &root,
                "ws://127.0.0.1:9/devtools/browser/not-contacted",
                "https://chatgpt.test/c/exact",
            );
            let state_path = root.join(".agent-browser/service/state.json");
            let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
            state["browsers"]["session:workshop-retained"][field] = value;
            write_private_json(&state_path, &state);
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains(expected));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn public_or_symlinked_requirement_is_rejected() {
        let root = fixture_root("permissions");
        let path = requirement_path(&root);
        write_private_json(&path, &json!({}));
        #[cfg(unix)]
        {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains("retained_browser_requirement_permissions_invalid"));
        }
        fs::remove_dir_all(root).unwrap();

        #[cfg(unix)]
        {
            let root = fixture_root("symlink");
            let path = requirement_path(&root);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let target = root.join("target.json");
            write_private_json(&target, &json!({}));
            std::os::unix::fs::symlink(&target, &path).unwrap();
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains("retained_browser_requirement_invalid"));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn configured_requirement_path_is_authoritative_and_no_launch() {
        let env = EnvGuard::new(&[
            "AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT",
            "AGENT_BROWSER_SOCKET_DIR",
        ]);
        let root = fixture_root("configured-path");
        let configured = root.join("private/retained.json");
        let (cdp_endpoint, server) = start_cdp(json!([{
            "id": "target-exact",
            "type": "page",
            "url": "https://chatgpt.test/c/exact"
        }]));
        seed_fixture(&root, &cdp_endpoint, "https://chatgpt.test/c/exact");
        fs::create_dir_all(configured.parent().unwrap()).unwrap();
        fs::rename(requirement_path(&root), &configured).unwrap();
        write_enforcement(&configured);
        env.set(
            "AGENT_BROWSER_DASHBOARD_RETAINED_REQUIREMENT",
            configured.to_str().unwrap(),
        );
        env.set(
            "AGENT_BROWSER_SOCKET_DIR",
            root.join("runtime").to_str().unwrap(),
        );

        let status = verify_retained_browser_requirement_for_root(&root).unwrap();
        assert!(status.configured);
        assert!(status.verified);
        assert!(status.enforcement_configured);
        assert_eq!(status.requirement_path, configured.display().to_string());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotation_crash_matrix_is_distinct_and_fail_closed_without_launch() {
        let root = fixture_root("rotation-crash-matrix");
        let requirement = requirement_path(&root);
        seed_fixture(
            &root,
            "ws://127.0.0.1:9/devtools/browser/not-contacted",
            "https://chatgpt.test/c/exact",
        );
        write_enforcement(&requirement);
        let next_requirement = json!({
            "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
            "createdAt": "2026-08-16T12:00:00.000Z",
            "expectation": {
                "sessionName": "replacement-retained",
                "profileId": "chatgpt-pro",
                "targetId": "replacement-target",
                "url": "https://chatgpt.test/c/replacement"
            }
        });
        let next_sha256 = write_rotation(&requirement, &next_requirement, "prepared");

        let prepared = verify_fixture(&root).unwrap();
        assert_eq!(prepared.state, "rotation_pending");
        assert!(!prepared.verified);
        assert!(prepared.rotation_configured);
        assert_eq!(prepared.rotation_phase.as_deref(), Some("prepared"));

        write_requirement_record(&requirement, &next_requirement);
        let requirement_replaced_before_phase = verify_fixture(&root).unwrap();
        assert_eq!(requirement_replaced_before_phase.state, "rotation_pending");
        assert_eq!(
            requirement_replaced_before_phase.rotation_phase.as_deref(),
            Some("prepared")
        );

        let rotation_path = retained_browser_rotation_path(&requirement);
        let mut rotation: Value =
            serde_json::from_slice(&fs::read(&rotation_path).unwrap()).unwrap();
        rotation["phase"] = json!("requirement_replaced");
        write_private_json(&rotation_path, &rotation);
        let requirement_replaced = verify_fixture(&root).unwrap();
        assert_eq!(requirement_replaced.state, "rotation_pending");
        assert_eq!(
            requirement_replaced.rotation_phase.as_deref(),
            Some("requirement_replaced")
        );

        write_private_json(
            &retained_browser_enforcement_path(&requirement),
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
                "createdAt": "2026-08-16T12:00:00.000Z",
                "requirementSha256": next_sha256
            }),
        );
        let enforcement_replaced_before_phase = verify_fixture(&root).unwrap();
        assert_eq!(enforcement_replaced_before_phase.state, "rotation_pending");

        for phase in ["enforcement_replaced", "committed"] {
            rotation["phase"] = json!(phase);
            write_private_json(&rotation_path, &rotation);
            let pending = verify_fixture(&root).unwrap();
            assert_eq!(pending.state, "rotation_pending");
            assert_eq!(pending.rotation_phase.as_deref(), Some(phase));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotation_rejects_impossible_old_requirement_new_enforcement_order() {
        let root = fixture_root("rotation-invalid-order");
        let requirement = requirement_path(&root);
        seed_fixture(
            &root,
            "ws://127.0.0.1:9/devtools/browser/not-contacted",
            "https://chatgpt.test/c/exact",
        );
        write_enforcement(&requirement);
        let next_requirement = json!({
            "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_REQUIREMENT_SCHEMA,
            "createdAt": "2026-08-16T12:00:00.000Z",
            "expectation": {
                "sessionName": "replacement-retained",
                "profileId": "chatgpt-pro",
                "targetId": "replacement-target",
                "url": "https://chatgpt.test/c/replacement"
            }
        });
        let next_sha256 = write_rotation(&requirement, &next_requirement, "prepared");
        write_private_json(
            &retained_browser_enforcement_path(&requirement),
            &json!({
                "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
                "createdAt": "2026-08-16T12:00:00.000Z",
                "requirementSha256": next_sha256
            }),
        );
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_browser_rotation_order_invalid"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enforcement_without_requirement_fails_closed_without_state_creation() {
        let root = fixture_root("enforced-missing");
        let requirement = requirement_path(&root);
        write_enforcement(&requirement);
        let before = fs::read(retained_browser_enforcement_path(&requirement)).unwrap();
        let error = verify_fixture(&root).unwrap_err();
        assert!(error.contains("retained_browser_requirement_missing"));
        assert!(!root.join(".agent-browser/service").exists());
        assert_eq!(
            fs::read(retained_browser_enforcement_path(&requirement)).unwrap(),
            before
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsafe_enforcement_file_fails_closed() {
        let root = fixture_root("enforcement-permissions");
        let requirement = requirement_path(&root);
        write_enforcement(&requirement);
        let enforcement = retained_browser_enforcement_path(&requirement);
        #[cfg(unix)]
        {
            fs::set_permissions(&enforcement, fs::Permissions::from_mode(0o644)).unwrap();
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains("retained_browser_enforcement_permissions_invalid"));
        }
        fs::remove_dir_all(root).unwrap();

        #[cfg(unix)]
        {
            let root = fixture_root("enforcement-symlink");
            let requirement = requirement_path(&root);
            let target = root.join("enforcement.json");
            write_private_json(
                &target,
                &json!({
                    "schemaVersion": LOCAL_DASHBOARD_RETAINED_BROWSER_ENFORCEMENT_SCHEMA,
                    "createdAt": "2026-08-15T11:59:59.000Z",
                    "requirementSha256": "0".repeat(64)
                }),
            );
            fs::create_dir_all(requirement.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&target, retained_browser_enforcement_path(&requirement))
                .unwrap();
            let error = verify_fixture(&root).unwrap_err();
            assert!(error.contains("retained_browser_enforcement_invalid"));
            fs::remove_dir_all(root).unwrap();
        }
    }
}
