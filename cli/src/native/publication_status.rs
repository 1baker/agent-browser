use crate::runtime_profile::pid_is_running;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const LOCAL_DASHBOARD_PUBLICATION_SCHEMA: &str = "agent-browser.local-dashboard-publication.v1";
pub const LOCAL_DASHBOARD_PUBLICATION_HTTP_ROUTE: &str =
    "/api/service/publications/local-dashboard";
pub const LOCAL_DASHBOARD_PUBLICATION_MCP_RESOURCE: &str =
    "agent-browser://publications/local-dashboard";

const MAX_JOURNAL_BYTES: u64 = 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_FAILURE_FIELD_BYTES: usize = 512;

pub fn local_dashboard_publication_status() -> Result<Value, String> {
    let journal_path = dirs::home_dir()
        .ok_or_else(|| "Unable to resolve the current home directory".to_string())?
        .join(".agent-browser/publications/local-dashboard-publication.json");
    local_dashboard_publication_status_for_path(&journal_path)
}

pub(crate) fn local_dashboard_publication_status_for_path(
    journal_path: &Path,
) -> Result<Value, String> {
    let lock_path = PathBuf::from(format!("{}.lock", journal_path.display()));
    let lock = publication_lock_status(&lock_path);
    if !journal_path.exists() {
        return Ok(json!({
            "schemaVersion": LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
            "journalPath": journal_path,
            "exists": false,
            "lock": lock,
            "transaction": Value::Null,
            "installedArtifact": Value::Null,
            "recoverable": false,
            "recommendedAction": if lock["live"].as_bool() == Some(true) {
                "wait_for_active_publisher"
            } else {
                "none"
            },
        }));
    }

    let metadata = fs::symlink_metadata(journal_path).map_err(|error| {
        format!("Unable to inspect local dashboard publication journal metadata: {error}")
    })?;
    if !metadata.file_type().is_file() {
        return Err("Local dashboard publication journal must be a regular file".to_string());
    }
    if metadata.len() > MAX_JOURNAL_BYTES {
        return Err(format!(
            "Local dashboard publication journal exceeds {MAX_JOURNAL_BYTES} bytes"
        ));
    }
    let record_bytes = fs::read(journal_path)
        .map_err(|error| format!("Unable to read local dashboard publication journal: {error}"))?;
    let record: Value = serde_json::from_slice(&record_bytes)
        .map_err(|error| format!("Local dashboard publication journal is invalid: {error}"))?;
    validate_publication_record(&record)?;

    let install_bin = record
        .get("installBin")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let installed_artifact = inspect_installed_artifact(&record, install_bin)?;
    let terminal = terminal_publication_phase(record["phase"].as_str().unwrap_or_default());
    let artifact_verified = installed_artifact["verified"].as_bool() == Some(true);
    let lock_live = lock["live"].as_bool() == Some(true);
    let retained_browser_expectation_required = record
        .pointer("/retainedBrowserExpectation/required")
        .and_then(Value::as_bool)
        == Some(true);
    let retained_browser_expectation_verified = retained_browser_expectation_required.then(|| {
        record
            .pointer("/retainedBrowserExpectation/final/verified")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let terminal_retained_browser_unverified = terminal
        && retained_browser_expectation_required
        && retained_browser_expectation_verified != Some(true);
    let recoverable = !terminal && artifact_verified && !lock_live;
    let recommended_action = if lock_live {
        "wait_for_active_publisher"
    } else if terminal_retained_browser_unverified {
        "investigate_retained_browser"
    } else if !terminal && !artifact_verified {
        "investigate_installed_artifact"
    } else if recoverable {
        "recover_only"
    } else {
        "none"
    };

    Ok(json!({
        "schemaVersion": LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
        "journalPath": journal_path,
        "exists": true,
        "lock": lock,
        "transaction": {
            "transactionId": record["transactionId"],
            "revision": record["revision"],
            "phase": record["phase"],
            "terminal": terminal,
            "createdAt": nullable_string(&record, "createdAt"),
            "updatedAt": nullable_string(&record, "updatedAt"),
            "installBin": install_bin,
            "builtBin": nullable_string(&record, "builtBin"),
            "backupPath": nullable_string(&record, "backupPath"),
            "candidateSessionCount": array_len(&record, "candidateSessions"),
            "preparedHandoffCount": array_len(&record, "handoffs"),
            "resumedHandoffCount": array_len(&record, "resumedHandoffs"),
            "retainedBrowserExpectationRequired": retained_browser_expectation_required,
            "retainedBrowserExpectationVerified": retained_browser_expectation_verified,
            "retainedBrowserExpectationStage": record
                .pointer("/retainedBrowserExpectation/final/stage")
                .or_else(|| record.pointer("/retainedBrowserExpectation/afterHandoff/stage"))
                .or_else(|| record.pointer("/retainedBrowserExpectation/before/stage"))
                .and_then(Value::as_str),
            "failure": bounded_failure(record.get("failure").or_else(|| record.get("originalFailure"))),
            "recoveryError": bounded_failure(record.get("recoveryError")),
        },
        "installedArtifact": installed_artifact,
        "recoverable": recoverable,
        "recommendedAction": recommended_action,
    }))
}

fn publication_lock_status(lock_path: &Path) -> Value {
    if !lock_path.exists() {
        return json!({
            "path": lock_path,
            "present": false,
            "ownerPid": Value::Null,
            "live": false,
            "stale": false,
        });
    }
    let owner_pid = fs::read_to_string(lock_path)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0);
    let live = owner_pid.is_some_and(pid_is_running);
    json!({
        "path": lock_path,
        "present": true,
        "ownerPid": owner_pid,
        "live": live,
        "stale": !live,
    })
}

fn inspect_installed_artifact(record: &Value, install_bin: Option<&str>) -> Result<Value, String> {
    let Some(path) = install_bin else {
        return Ok(json!({
            "path": Value::Null,
            "exists": false,
            "sha256": Value::Null,
            "classification": "missing",
            "verified": false,
        }));
    };
    let path = Path::new(path);
    if !path.exists() {
        return Ok(json!({
            "path": path,
            "exists": false,
            "sha256": Value::Null,
            "classification": "missing",
            "verified": false,
        }));
    }
    let sha256 = sha256_file(path)?;
    let evidence = record.get("artifactEvidence").unwrap_or(&Value::Null);
    let classification = classify_installed_artifact(evidence, &sha256);
    Ok(json!({
        "path": path,
        "exists": true,
        "sha256": sha256,
        "classification": classification,
        "verified": classification != "unknown",
    }))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect installed publication artifact: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err("Installed publication artifact must be a regular file".to_string());
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!(
            "Installed publication artifact exceeds {MAX_ARTIFACT_BYTES} bytes"
        ));
    }
    let mut file = File::open(path)
        .map_err(|error| format!("Unable to read installed publication artifact: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Unable to hash installed publication artifact: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn classify_installed_artifact(evidence: &Value, sha256: &str) -> &'static str {
    if evidence
        .pointer("/replacement/verified")
        .and_then(Value::as_bool)
        == Some(true)
        && evidence
            .pointer("/replacement/actualSha256")
            .and_then(Value::as_str)
            == Some(sha256)
    {
        return "replacement";
    }
    if evidence
        .pointer("/backup/verified")
        .and_then(Value::as_bool)
        == Some(true)
        && evidence.pointer("/backup/sha256").and_then(Value::as_str) == Some(sha256)
    {
        return "backup";
    }
    if evidence.pointer("/built/sha256").and_then(Value::as_str) == Some(sha256) {
        return "built_replacement";
    }
    "unknown"
}

fn validate_publication_record(record: &Value) -> Result<(), String> {
    let object = record
        .as_object()
        .ok_or_else(|| "Local dashboard publication journal must be an object".to_string())?;
    if object.get("schemaVersion").and_then(Value::as_str)
        != Some(LOCAL_DASHBOARD_PUBLICATION_SCHEMA)
    {
        return Err("Unsupported local dashboard publication journal schema".to_string());
    }
    if object
        .get("transactionId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err("Local dashboard publication journal transactionId is required".to_string());
    }
    if object.get("revision").and_then(Value::as_u64).unwrap_or(0) < 1 {
        return Err("Local dashboard publication journal revision must be positive".to_string());
    }
    let phase = object
        .get("phase")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Local dashboard publication journal phase is required".to_string())?;
    if object.get("terminal").and_then(Value::as_bool) != Some(terminal_publication_phase(phase)) {
        return Err(
            "Local dashboard publication journal terminal state does not match phase".to_string(),
        );
    }
    Ok(())
}

fn terminal_publication_phase(phase: &str) -> bool {
    matches!(
        phase,
        "ready" | "rolled_back" | "recovered_ready" | "recovered_rolled_back"
    )
}

fn nullable_string(record: &Value, key: &str) -> Value {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null)
}

fn array_len(record: &Value, key: &str) -> usize {
    record
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn bounded_failure(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    if let Some(text) = value.as_str() {
        return Value::String(truncate(text, MAX_FAILURE_FIELD_BYTES));
    }
    let Some(object) = value.as_object() else {
        return Value::String(truncate(&value.to_string(), MAX_FAILURE_FIELD_BYTES));
    };
    let mut bounded = Map::new();
    for key in ["code", "message", "phase", "classification"] {
        if let Some(field) = object.get(key) {
            let rendered = field
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| field.to_string());
            bounded.insert(
                key.to_string(),
                Value::String(truncate(&rendered, MAX_FAILURE_FIELD_BYTES)),
            );
        }
    }
    Value::Object(bounded)
}

fn truncate(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &value[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-publication-status-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_journal(path: &Path, install_bin: &Path, sha256: &str, phase: &str) {
        let terminal = terminal_publication_phase(phase);
        fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "schemaVersion": LOCAL_DASHBOARD_PUBLICATION_SCHEMA,
                "transactionId": "local-dashboard-fixture",
                "revision": 4,
                "phase": phase,
                "terminal": terminal,
                "createdAt": "2026-08-15T00:00:00.000Z",
                "updatedAt": "2026-08-15T00:01:00.000Z",
                "installBin": install_bin,
                "candidateSessions": ["one", "two"],
                "handoffs": [{"secret": "not projected"}],
                "resumedHandoffs": [{"secret": "not projected"}],
                "failure": {"code": "fixture", "message": "bounded", "secret": "hidden"},
                "artifactEvidence": {
                    "replacement": {"verified": true, "actualSha256": sha256}
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn absent_status_is_read_only_and_recommends_none() {
        let root = fixture_root("absent");
        let journal = root.join("missing/publication.json");
        let status = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(status["exists"], false);
        assert_eq!(status["recommendedAction"], "none");
        assert!(!journal.parent().unwrap().exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recoverable_status_hashes_exact_artifact_and_bounds_handoffs() {
        let root = fixture_root("recoverable");
        let journal = root.join("publication.json");
        let installed = root.join("agent-browser");
        fs::write(&installed, b"replacement").unwrap();
        let sha256 = sha256_file(&installed).unwrap();
        write_journal(&journal, &installed, &sha256, "replacement_installed");

        let status = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(status["recoverable"], true);
        assert_eq!(status["recommendedAction"], "recover_only");
        assert_eq!(status["installedArtifact"]["classification"], "replacement");
        assert_eq!(status["transaction"]["candidateSessionCount"], 2);
        assert_eq!(status["transaction"]["preparedHandoffCount"], 1);
        assert_eq!(status["transaction"]["resumedHandoffCount"], 1);
        assert_eq!(
            status["transaction"]["retainedBrowserExpectationRequired"],
            false
        );
        assert_eq!(
            status["transaction"]["retainedBrowserExpectationVerified"],
            Value::Null
        );
        assert!(status.to_string().find("not projected").is_none());
        assert!(status.to_string().find("hidden").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_browser_guard_projects_only_bounded_verification_state() {
        let root = fixture_root("retained-browser-guard");
        let journal = root.join("publication.json");
        let installed = root.join("agent-browser");
        fs::write(&installed, b"replacement").unwrap();
        let sha256 = sha256_file(&installed).unwrap();
        write_journal(&journal, &installed, &sha256, "ready");
        let mut record: Value = serde_json::from_slice(&fs::read(&journal).unwrap()).unwrap();
        record["retainedBrowserExpectation"] = json!({
            "required": true,
            "pinned": {
                "sessionName": "private-session",
                "targetId": "private-target",
                "url": "https://example.test/private"
            },
            "before": {"verified": true, "stage": "pre_mutation"},
            "afterHandoff": {"verified": true, "stage": "post_handoff"},
            "final": {"verified": true, "stage": "final_readiness"}
        });
        fs::write(&journal, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let status = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(
            status["transaction"]["retainedBrowserExpectationRequired"],
            true
        );
        assert_eq!(
            status["transaction"]["retainedBrowserExpectationVerified"],
            true
        );
        assert_eq!(
            status["transaction"]["retainedBrowserExpectationStage"],
            "final_readiness"
        );
        let rendered = status.to_string();
        assert!(!rendered.contains("private-session"));
        assert!(!rendered.contains("private-target"));
        assert!(!rendered.contains("example.test/private"));

        record["retainedBrowserExpectation"]["final"]["verified"] = json!(false);
        fs::write(&journal, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        let unverified = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(
            unverified["recommendedAction"],
            "investigate_retained_browser"
        );
        assert_eq!(unverified["recoverable"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_artifact_fails_closed() {
        let root = fixture_root("unknown");
        let journal = root.join("publication.json");
        let installed = root.join("agent-browser");
        fs::write(&installed, b"changed").unwrap();
        write_journal(
            &journal,
            &installed,
            &"0".repeat(64),
            "replacement_installed",
        );

        let status = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(status["recoverable"], false);
        assert_eq!(
            status["recommendedAction"],
            "investigate_installed_artifact"
        );
        assert_eq!(status["installedArtifact"]["classification"], "unknown");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_lock_requires_wait_without_recovery() {
        let root = fixture_root("lock");
        let journal = root.join("publication.json");
        fs::write(
            format!("{}.lock", journal.display()),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let status = local_dashboard_publication_status_for_path(&journal).unwrap();
        assert_eq!(status["lock"]["live"], true);
        assert_eq!(status["recommendedAction"], "wait_for_active_publisher");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_or_nonregular_journal_is_rejected() {
        let root = fixture_root("invalid");
        let journal = root.join("publication.json");
        fs::write(&journal, b"not-json").unwrap();
        assert!(local_dashboard_publication_status_for_path(&journal)
            .unwrap_err()
            .contains("journal is invalid"));
        fs::remove_file(&journal).unwrap();
        fs::create_dir(&journal).unwrap();
        assert!(local_dashboard_publication_status_for_path(&journal)
            .unwrap_err()
            .contains("regular file"));
        fs::remove_dir_all(root).unwrap();
    }
}
