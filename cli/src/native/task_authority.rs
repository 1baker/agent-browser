use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

use super::policy::ActionConsequence;

const DEFAULT_EVIDENCE_RESERVATION_BYTES: u64 = 4096;
const LARGE_EVIDENCE_RESERVATION_BYTES: u64 = 65_536;
const MAX_ISSUED_AUTHORITY_TTL_SECONDS: u64 = 3_600;
const MAX_ISSUED_AUTHORITY_STEPS: usize = 100;
const TASK_AUTHORITY_CONFIRMATION_SCHEMA: &str = "agent-browser.task-authority-confirmation.v1";
pub const DEFAULT_CONFIRMATION_RECEIPT_RETENTION_COUNT: usize = 1_000;
pub const DEFAULT_CONFIRMATION_RECEIPT_MIN_AGE_SECONDS: u64 = 30 * 24 * 60 * 60;
const CONFIRMATION_TOMBSTONE_SEGMENT_CAPACITY: usize = 256;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityEnvelope {
    pub id: String,
    pub task_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub allowed_origins: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<TaskAuthorityLineage>,
    pub target_binding: TaskAuthorityTargetBinding,
    pub evidence_budget: TaskAuthorityEvidenceBudget,
    pub consequence_ceiling: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityLineage {
    pub reconciliation_id: String,
    pub predecessor_authority_id: String,
    pub predecessor_step_id: String,
    pub predecessor_step_index: u64,
    pub predecessor_command_id: String,
    pub predecessor_outcome_state: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityTargetBinding {
    pub target_id: String,
    pub initial_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityEvidenceBudget {
    pub max_actions: u64,
    pub max_evidence_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityLedger {
    schema: String,
    authority_id: String,
    envelope_sha256: String,
    session_id: String,
    admitted_actions: u64,
    reserved_evidence_bytes: u64,
    #[serde(default)]
    next_step_index: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    step_receipts: Vec<TaskAuthorityStepReceipt>,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityStepReceipt {
    step_id: String,
    step_index: u64,
    command_id: String,
    action: String,
    target_id: String,
    current_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_url: Option<String>,
    reserved_evidence_bytes: u64,
    admitted_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcome: Option<TaskAuthorityStepOutcome>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityStepOutcome {
    state: String,
    success: bool,
    response_sha256: String,
    response_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    finalized_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityIssuer {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityConfirmationTargetBinding {
    pub target_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityConfirmationRecord {
    schema: String,
    pub confirmation_id: String,
    pub session_id: String,
    pub action: String,
    pub consequence_class: String,
    pub target_binding: TaskAuthorityConfirmationTargetBinding,
    pub request_sha256: String,
    pub requested_by: TaskAuthorityIssuer,
    pub requested_at: String,
    pub expires_at: String,
    pub state: String,
    pub execution_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<TaskAuthorityIssuer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalidation_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_success: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    command: Value,
}

impl TaskAuthorityConfirmationRecord {
    pub fn command(&self) -> &Value {
        &self.command
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyTaskAuthorityConfirmationTombstones {
    #[serde(default = "legacy_confirmation_tombstone_schema")]
    schema: String,
    #[serde(default)]
    confirmation_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

fn legacy_confirmation_tombstone_schema() -> String {
    "agent-browser.task-authority-confirmation-tombstones.v1".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityConfirmationTombstoneManifest {
    schema: String,
    segment_capacity: usize,
    segment_count: u64,
    confirmation_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    head_sha256: Option<String>,
    #[serde(default)]
    active_confirmation_ids: BTreeSet<String>,
    active_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

impl Default for TaskAuthorityConfirmationTombstoneManifest {
    fn default() -> Self {
        Self {
            schema: confirmation_tombstone_manifest_schema(),
            segment_capacity: CONFIRMATION_TOMBSTONE_SEGMENT_CAPACITY,
            segment_count: 0,
            confirmation_count: 0,
            head_sha256: None,
            active_confirmation_ids: BTreeSet::new(),
            active_sha256: format!("{:x}", Sha256::digest(b"[]")),
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityConfirmationTombstoneSegment {
    schema: String,
    index: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_sha256: Option<String>,
    confirmation_ids: Vec<String>,
    created_at: String,
}

#[derive(Debug, Clone)]
struct LoadedTaskAuthorityConfirmationTombstones {
    manifest: TaskAuthorityConfirmationTombstoneManifest,
    confirmation_ids: BTreeSet<String>,
    legacy: bool,
}

fn confirmation_tombstone_manifest_schema() -> String {
    "agent-browser.task-authority-confirmation-tombstone-manifest.v2".to_string()
}

fn confirmation_tombstone_segment_schema() -> String {
    "agent-browser.task-authority-confirmation-tombstone-segment.v1".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityPlanStep {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityIssueRequest {
    pub task_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub expected_target_id: String,
    pub expected_url: String,
    pub issuer: TaskAuthorityIssuer,
    pub approval_reference: String,
    pub expires_in_seconds: u64,
    pub steps: Vec<TaskAuthorityPlanStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskAuthorityReconcileRequest {
    pub reconciliation_id: String,
    pub unresolved_step_id: String,
    pub task_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub expected_target_id: String,
    pub expected_url: String,
    pub issuer: TaskAuthorityIssuer,
    pub approval_reference: String,
    pub expires_in_seconds: u64,
    pub steps: Vec<TaskAuthorityPlanStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityApprovedPlan {
    actions: Vec<String>,
    origins: Vec<String>,
    action_count: u64,
    evidence_bytes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    steps: Vec<TaskAuthorityApprovedStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plan_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityApprovedStep {
    step_id: String,
    index: u64,
    action: String,
    expected_current_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_url: Option<String>,
    evidence_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityRevocation {
    revoked_at: String,
    revoked_by: String,
    reason: String,
    #[serde(default)]
    target_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reconciliation: Option<TaskAuthorityReconciliation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityReconciliation {
    reconciliation_id: String,
    request_sha256: String,
    predecessor_step_id: String,
    predecessor_step_index: u64,
    predecessor_command_id: String,
    predecessor_outcome_state: String,
    replacement_authority_id: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskAuthorityIssuanceRecord {
    schema: String,
    session_id: String,
    envelope: TaskAuthorityEnvelope,
    envelope_sha256: String,
    issuer: TaskAuthorityIssuer,
    approval_reference: String,
    approved_plan: TaskAuthorityApprovedPlan,
    issued_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    revocation: Option<TaskAuthorityRevocation>,
}

#[derive(Debug, Clone)]
pub struct TaskAuthorityContext<'a> {
    pub session_id: &'a str,
    pub target_id: Option<&'a str>,
    pub url: Option<&'a str>,
    pub confirmed_authority_id: Option<&'a str>,
    pub require_authority: bool,
    pub ledger_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAuthorityAdmission {
    pub authority_id: String,
    pub envelope_sha256: String,
    pub consequence_ceiling: ActionConsequence,
    pub consequence: ActionConsequence,
    pub admitted_actions: u64,
    pub reserved_evidence_bytes: u64,
    pub expires_at: String,
    pub step_id: Option<String>,
    pub step_index: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAuthorityDecision {
    NotPresent,
    Admitted(TaskAuthorityAdmission),
    RequiresConfirmation(TaskAuthorityAdmission),
}

pub fn task_authority_required_from_env() -> bool {
    matches!(
        env::var("AGENT_BROWSER_REQUIRE_TASK_AUTHORITY").as_deref(),
        Ok("1" | "true" | "yes")
    )
}

pub fn task_authority_ledger_root() -> PathBuf {
    env::var_os("AGENT_BROWSER_TASK_AUTHORITY_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".agent-browser/task-authority")))
        .unwrap_or_else(|| PathBuf::from(".agent-browser/task-authority"))
}

fn safe_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}

fn ledger_path(root: &Path, session_id: &str, authority_id: &str) -> PathBuf {
    root.join(safe_component(session_id))
        .join(format!("{}.json", safe_component(authority_id)))
}

fn issuance_path(root: &Path, session_id: &str, authority_id: &str) -> PathBuf {
    root.join(safe_component(session_id))
        .join(format!("{}.issuance.json", safe_component(authority_id)))
}

fn normalized_origin(value: &str) -> Result<String, String> {
    let parsed = Url::parse(value).map_err(|error| format!("Invalid authority URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "Task authority permits only http or https origins, got '{}'",
            parsed.scheme()
        ));
    }
    let host = parsed
        .host_str()
        .ok_or("Task authority URL has no host")?
        .to_ascii_lowercase();
    let mut origin = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    Ok(origin)
}

fn envelope_sha256(envelope: &TaskAuthorityEnvelope) -> Result<String, String> {
    let bytes = serde_json::to_vec(envelope)
        .map_err(|error| format!("Failed to serialize task authority: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn approved_plan_sha256(steps: &[TaskAuthorityApprovedStep]) -> Result<String, String> {
    let bytes = serde_json::to_vec(steps)
        .map_err(|error| format!("Failed to serialize task authority plan: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalized_url(value: &str) -> Result<String, String> {
    let mut parsed =
        Url::parse(value).map_err(|error| format!("Invalid authority URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "Task authority permits only http or https URLs, got '{}'",
            parsed.scheme()
        ));
    }
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn explicit_requested_url(cmd: &Value) -> Option<&str> {
    cmd.get("url").and_then(Value::as_str).or_else(|| {
        cmd.get("params")
            .and_then(|params| params.get("url"))
            .and_then(Value::as_str)
    })
}

fn requested_url<'a>(cmd: &'a Value, fallback: Option<&'a str>) -> Option<&'a str> {
    cmd.get("url")
        .and_then(Value::as_str)
        .or_else(|| {
            cmd.get("params")
                .and_then(|params| params.get("url"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            cmd.get("serviceTabHandle")
                .and_then(|handle| handle.get("url"))
                .and_then(Value::as_str)
        })
        .or(fallback)
}

fn default_evidence_reservation(action: &str) -> u64 {
    if matches!(
        action,
        "snapshot" | "content" | "read_page" | "screenshot" | "pdf"
    ) {
        LARGE_EVIDENCE_RESERVATION_BYTES
    } else {
        DEFAULT_EVIDENCE_RESERVATION_BYTES
    }
}

fn evidence_reservation(cmd: &Value, action: &str) -> u64 {
    if let Some(explicit) = cmd.get("taskEvidenceBytes").and_then(Value::as_u64) {
        return explicit;
    }
    for field in ["maxReturnBytes", "maxTextBytes", "maxBodyBytes"] {
        if let Some(value) = cmd.get(field).and_then(Value::as_u64) {
            return value;
        }
    }
    default_evidence_reservation(action)
}

fn read_ledger(path: &Path) -> Result<Option<TaskAuthorityLedger>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read task authority ledger: {error}"))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| format!("Invalid task authority ledger: {error}"))
}

fn write_ledger(path: &Path, ledger: &TaskAuthorityLedger) -> Result<(), String> {
    write_private_json(path, ledger, "task authority ledger")
}

fn validate_ordered_ledger(
    ledger: &TaskAuthorityLedger,
    record: &TaskAuthorityIssuanceRecord,
) -> Result<(), String> {
    if ledger.schema != "agent-browser.task-authority-ledger.v2"
        || ledger.next_step_index
            != u64::try_from(ledger.step_receipts.len())
                .map_err(|_| "Task authority receipt count overflow")?
        || ledger.admitted_actions != ledger.next_step_index
    {
        return Err("Task authority ordered ledger cursor is invalid".to_string());
    }
    let mut evidence_bytes = 0u64;
    let mut command_ids = BTreeSet::new();
    for (index, receipt) in ledger.step_receipts.iter().enumerate() {
        let step = record
            .approved_plan
            .steps
            .get(index)
            .ok_or("Task authority ordered ledger exceeds the approved plan")?;
        if receipt.step_id != step.step_id
            || receipt.step_index != step.index
            || receipt.action != step.action
            || receipt.target_id != record.envelope.target_binding.target_id
            || receipt.current_url != step.expected_current_url
            || receipt.requested_url != step.requested_url
            || receipt.reserved_evidence_bytes != step.evidence_bytes
            || receipt.command_id.is_empty()
            || !command_ids.insert(receipt.command_id.clone())
        {
            return Err("Task authority ordered ledger receipt is invalid".to_string());
        }
        if let Some(outcome) = receipt.outcome.as_ref() {
            if !matches!(outcome.state.as_str(), "completed" | "failed")
                || outcome.success != (outcome.state == "completed")
                || outcome.response_bytes == 0
                || outcome.response_sha256.len() != 64
                || !outcome
                    .response_sha256
                    .chars()
                    .all(|value| value.is_ascii_hexdigit())
                || DateTime::parse_from_rfc3339(&outcome.finalized_at).is_err()
                || outcome
                    .target_id
                    .as_deref()
                    .is_some_and(|value| value != receipt.target_id)
                || outcome.url.as_deref().map(normalized_url).transpose()? != outcome.url
            {
                return Err("Task authority ordered ledger outcome is invalid".to_string());
            }
        }
        evidence_bytes = evidence_bytes
            .checked_add(receipt.reserved_evidence_bytes)
            .ok_or("Task authority ordered ledger evidence overflow")?;
    }
    if ledger.reserved_evidence_bytes != evidence_bytes {
        return Err("Task authority ordered ledger evidence total is invalid".to_string());
    }
    Ok(())
}

fn write_private_json<T: Serialize>(path: &Path, value: &T, label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Task authority ledger path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create {label} directory: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Failed to secure {label} directory: {error}"))?;
    }
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("authority"),
        uuid::Uuid::new_v4()
    ));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Failed to serialize {label}: {error}"))?;
    fs::write(&temporary, bytes).map_err(|error| format!("Failed to stage {label}: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Failed to secure staged {label}: {error}"))?;
    }
    fs::rename(&temporary, path).map_err(|error| format!("Failed to publish {label}: {error}"))
}

fn confirmation_root(root: &Path, session_id: &str) -> PathBuf {
    root.join(safe_component(session_id)).join("confirmations")
}

fn pending_confirmation_path(root: &Path, session_id: &str) -> PathBuf {
    confirmation_root(root, session_id).join("pending.json")
}

fn terminal_confirmation_path(root: &Path, session_id: &str, confirmation_id: &str) -> PathBuf {
    confirmation_root(root, session_id)
        .join("receipts")
        .join(format!("{}.json", safe_component(confirmation_id)))
}

fn confirmation_tombstones_path(root: &Path, session_id: &str) -> PathBuf {
    confirmation_root(root, session_id).join("retired-ids.json")
}

fn confirmation_tombstone_segment_path(root: &Path, session_id: &str, index: u64) -> PathBuf {
    confirmation_root(root, session_id)
        .join("retired-id-segments")
        .join(format!("{index:020}.json"))
}

fn confirmation_tombstone_segment_sha256(
    segment: &TaskAuthorityConfirmationTombstoneSegment,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(segment)
        .map_err(|error| format!("Failed to serialize confirmation tombstone segment: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn confirmation_tombstone_active_sha256(
    confirmation_ids: &BTreeSet<String>,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(confirmation_ids)
        .map_err(|error| format!("Failed to serialize active confirmation tombstones: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn load_confirmation_tombstones(
    root: &Path,
    session_id: &str,
) -> Result<LoadedTaskAuthorityConfirmationTombstones, String> {
    let path = confirmation_tombstones_path(root, session_id);
    if !path.exists() {
        return Ok(LoadedTaskAuthorityConfirmationTombstones {
            manifest: TaskAuthorityConfirmationTombstoneManifest::default(),
            confirmation_ids: BTreeSet::new(),
            legacy: false,
        });
    }
    let text = fs::read_to_string(&path).map_err(|error| {
        format!("Failed to read task authority confirmation tombstones: {error}")
    })?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid task authority confirmation tombstones: {error}"))?;
    let schema = value.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema == legacy_confirmation_tombstone_schema() {
        let legacy: LegacyTaskAuthorityConfirmationTombstones = serde_json::from_value(value)
            .map_err(|error| format!("Invalid legacy confirmation tombstones: {error}"))?;
        return Ok(LoadedTaskAuthorityConfirmationTombstones {
            manifest: TaskAuthorityConfirmationTombstoneManifest::default(),
            confirmation_ids: legacy.confirmation_ids,
            legacy: true,
        });
    }
    let manifest: TaskAuthorityConfirmationTombstoneManifest = serde_json::from_value(value)
        .map_err(|error| format!("Invalid confirmation tombstone manifest: {error}"))?;
    if manifest.schema != confirmation_tombstone_manifest_schema()
        || manifest.segment_capacity != CONFIRMATION_TOMBSTONE_SEGMENT_CAPACITY
        || manifest.active_confirmation_ids.len() > manifest.segment_capacity
        || manifest.active_sha256
            != confirmation_tombstone_active_sha256(&manifest.active_confirmation_ids)?
    {
        return Err("Task authority confirmation tombstone ledger is invalid".to_string());
    }
    let mut confirmation_ids = BTreeSet::new();
    let mut previous_sha256 = None;
    for index in 0..manifest.segment_count {
        let segment_path = confirmation_tombstone_segment_path(root, session_id, index);
        let segment_text = fs::read_to_string(&segment_path).map_err(|error| {
            format!("Failed to read confirmation tombstone segment {index}: {error}")
        })?;
        let segment: TaskAuthorityConfirmationTombstoneSegment =
            serde_json::from_str(&segment_text).map_err(|error| {
                format!("Invalid confirmation tombstone segment {index}: {error}")
            })?;
        if segment.schema != confirmation_tombstone_segment_schema()
            || segment.index != index
            || segment.previous_sha256 != previous_sha256
            || segment.confirmation_ids.is_empty()
            || segment.confirmation_ids.len() > manifest.segment_capacity
        {
            return Err(format!(
                "Task authority confirmation tombstone segment {index} failed closed"
            ));
        }
        for confirmation_id in &segment.confirmation_ids {
            if confirmation_id.trim().is_empty()
                || !confirmation_ids.insert(confirmation_id.clone())
            {
                return Err(format!(
                    "Task authority confirmation tombstone segment {index} contains invalid or duplicate IDs"
                ));
            }
        }
        previous_sha256 = Some(confirmation_tombstone_segment_sha256(&segment)?);
    }
    if manifest.head_sha256 != previous_sha256 {
        return Err("Task authority confirmation tombstone head digest is invalid".to_string());
    }
    for confirmation_id in &manifest.active_confirmation_ids {
        if confirmation_id.trim().is_empty() || !confirmation_ids.insert(confirmation_id.clone()) {
            return Err(
                "Task authority confirmation tombstone manifest contains invalid or duplicate IDs"
                    .to_string(),
            );
        }
    }
    if u64::try_from(confirmation_ids.len()).ok() != Some(manifest.confirmation_count) {
        return Err("Task authority confirmation tombstone count is invalid".to_string());
    }
    Ok(LoadedTaskAuthorityConfirmationTombstones {
        manifest,
        confirmation_ids,
        legacy: false,
    })
}

fn confirmation_tombstone_evidence(
    tombstones: &LoadedTaskAuthorityConfirmationTombstones,
) -> Result<Value, String> {
    let active_confirmation_ids = if tombstones.legacy {
        &tombstones.confirmation_ids
    } else {
        &tombstones.manifest.active_confirmation_ids
    };
    let active_sha256 = confirmation_tombstone_active_sha256(active_confirmation_ids)?;
    Ok(json!({
        "schema": if tombstones.legacy { legacy_confirmation_tombstone_schema() } else { tombstones.manifest.schema.clone() },
        "integrityState": "verified",
        "legacyPendingMigration": tombstones.legacy,
        "segmentCapacity": tombstones.manifest.segment_capacity,
        "segmentCount": tombstones.manifest.segment_count,
        "activeCount": active_confirmation_ids.len(),
        "activeSha256": active_sha256,
        "confirmationCount": tombstones.confirmation_ids.len(),
        "headSha256": tombstones.manifest.head_sha256,
    }))
}

fn persist_confirmation_tombstones(
    root: &Path,
    session_id: &str,
    loaded: LoadedTaskAuthorityConfirmationTombstones,
    new_confirmation_ids: impl IntoIterator<Item = String>,
) -> Result<LoadedTaskAuthorityConfirmationTombstones, String> {
    let mut manifest = if loaded.legacy {
        TaskAuthorityConfirmationTombstoneManifest {
            active_confirmation_ids: loaded.confirmation_ids.clone(),
            ..TaskAuthorityConfirmationTombstoneManifest::default()
        }
    } else {
        loaded.manifest
    };
    let mut confirmation_ids = loaded.confirmation_ids;
    for confirmation_id in new_confirmation_ids {
        if confirmation_id.trim().is_empty() || !confirmation_ids.insert(confirmation_id.clone()) {
            return Err(
                "Task authority confirmation tombstone ID is invalid or repeated".to_string(),
            );
        }
        manifest.active_confirmation_ids.insert(confirmation_id);
    }
    while manifest.active_confirmation_ids.len() > manifest.segment_capacity {
        let segment_ids: Vec<String> = manifest
            .active_confirmation_ids
            .iter()
            .take(manifest.segment_capacity)
            .cloned()
            .collect();
        for confirmation_id in &segment_ids {
            manifest.active_confirmation_ids.remove(confirmation_id);
        }
        let segment = TaskAuthorityConfirmationTombstoneSegment {
            schema: confirmation_tombstone_segment_schema(),
            index: manifest.segment_count,
            previous_sha256: manifest.head_sha256.clone(),
            confirmation_ids: segment_ids,
            created_at: Utc::now().to_rfc3339(),
        };
        let segment_sha256 = confirmation_tombstone_segment_sha256(&segment)?;
        write_private_json(
            &confirmation_tombstone_segment_path(root, session_id, segment.index),
            &segment,
            "task authority confirmation tombstone segment",
        )?;
        manifest.segment_count = manifest
            .segment_count
            .checked_add(1)
            .ok_or("Task authority confirmation tombstone segment count overflow")?;
        manifest.head_sha256 = Some(segment_sha256);
    }
    manifest.confirmation_count = u64::try_from(confirmation_ids.len())
        .map_err(|_| "Task authority confirmation tombstone count overflow")?;
    manifest.active_sha256 =
        confirmation_tombstone_active_sha256(&manifest.active_confirmation_ids)?;
    manifest.updated_at = Some(Utc::now().to_rfc3339());
    write_private_json(
        &confirmation_tombstones_path(root, session_id),
        &manifest,
        "task authority confirmation tombstone manifest",
    )?;
    load_confirmation_tombstones(root, session_id)
}

fn confirmation_request_sha256(command: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(command)
        .map_err(|error| format!("Failed to serialize task authority confirmation: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn task_authority_confirmation_requester(
    action: &str,
    command: &Value,
) -> Result<TaskAuthorityIssuer, String> {
    if action == "task_authority_revoke" {
        let id = command
            .get("revokedBy")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or("Task authority revoke confirmation requires revokedBy")?;
        return Ok(TaskAuthorityIssuer {
            kind: "operator".to_string(),
            id: id.to_string(),
        });
    }
    serde_json::from_value(
        command
            .get("request")
            .and_then(|request| request.get("issuer"))
            .cloned()
            .ok_or("Task authority confirmation request requires issuer")?,
    )
    .map_err(|error| format!("Invalid task authority confirmation issuer: {error}"))
}

fn validate_confirmation_record(
    record: &TaskAuthorityConfirmationRecord,
    session_id: &str,
) -> Result<(), String> {
    if record.schema != TASK_AUTHORITY_CONFIRMATION_SCHEMA
        || record.session_id != session_id
        || !matches!(
            record.action.as_str(),
            "task_authority_issue" | "task_authority_reconcile" | "task_authority_revoke"
        )
        || record.command.get("action").and_then(Value::as_str) != Some(record.action.as_str())
        || record.request_sha256.len() != 64
        || !record
            .request_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || confirmation_request_sha256(&record.command)? != record.request_sha256
        || record.confirmation_id.trim().is_empty()
        || record.target_binding.target_id.trim().is_empty()
        || record.target_binding.url.trim().is_empty()
        || DateTime::parse_from_rfc3339(&record.requested_at).is_err()
        || DateTime::parse_from_rfc3339(&record.expires_at).is_err()
    {
        return Err("Task authority confirmation record is invalid".to_string());
    }
    Ok(())
}

fn read_confirmation_record(
    path: &Path,
    session_id: &str,
) -> Result<Option<TaskAuthorityConfirmationRecord>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read task authority confirmation: {error}"))?;
    let record: TaskAuthorityConfirmationRecord = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid task authority confirmation: {error}"))?;
    validate_confirmation_record(&record, session_id)?;
    Ok(Some(record))
}

fn archive_confirmation(
    root: &Path,
    record: &TaskAuthorityConfirmationRecord,
) -> Result<(), String> {
    let terminal = terminal_confirmation_path(root, &record.session_id, &record.confirmation_id);
    if let Some(existing) = read_confirmation_record(&terminal, &record.session_id)? {
        if existing == *record {
            return Ok(());
        }
        return Err(
            "Task authority confirmation already has a different decision receipt".to_string(),
        );
    }
    write_private_json(&terminal, record, "task authority confirmation receipt")
}

fn remove_pending_confirmation(root: &Path, session_id: &str) -> Result<(), String> {
    let path = pending_confirmation_path(root, session_id);
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Failed to consume task authority confirmation: {error}"))?;
    }
    Ok(())
}

fn expire_pending_confirmation(
    root: &Path,
    mut record: TaskAuthorityConfirmationRecord,
) -> Result<(), String> {
    record.state = "expired".to_string();
    record.execution_state = "not_dispatched".to_string();
    record.invalidation_reason = Some("confirmation expired before decision".to_string());
    archive_confirmation(root, &record)?;
    remove_pending_confirmation(root, &record.session_id)
}

/// Persist one private, exact-session task-authority control confirmation.
/// Arbitrary browser actions are intentionally excluded from this ledger.
pub struct StageTaskAuthorityConfirmation<'a> {
    pub root: &'a Path,
    pub session_id: &'a str,
    pub confirmation_id: &'a str,
    pub action: &'a str,
    pub consequence_class: &'a str,
    pub command: &'a Value,
    pub target_id: &'a str,
    pub url: &'a str,
    pub ttl_seconds: u64,
}

pub fn stage_task_authority_confirmation(
    input: StageTaskAuthorityConfirmation<'_>,
) -> Result<TaskAuthorityConfirmationRecord, String> {
    let StageTaskAuthorityConfirmation {
        root,
        session_id,
        confirmation_id,
        action,
        consequence_class,
        command,
        target_id,
        url,
        ttl_seconds,
    } = input;
    if !matches!(
        action,
        "task_authority_issue" | "task_authority_reconcile" | "task_authority_revoke"
    ) || ttl_seconds == 0
    {
        return Err("Only bounded task authority control confirmations are durable".to_string());
    }
    if terminal_confirmation_path(root, session_id, confirmation_id).exists()
        || load_confirmation_tombstones(root, session_id)?
            .confirmation_ids
            .contains(confirmation_id)
    {
        return Err("Task authority confirmation ID already has a terminal receipt".to_string());
    }
    if let Some(existing) =
        read_confirmation_record(&pending_confirmation_path(root, session_id), session_id)?
    {
        let expires_at = DateTime::parse_from_rfc3339(&existing.expires_at)
            .map_err(|error| format!("Invalid task authority confirmation expiry: {error}"))?
            .with_timezone(&Utc);
        if Utc::now() < expires_at {
            return Err(format!(
                "Confirmation '{}' for action '{}' is still pending",
                existing.confirmation_id, existing.action
            ));
        }
        expire_pending_confirmation(root, existing)?;
    }
    let requested_at = Utc::now();
    let expires_at = requested_at
        + chrono::Duration::seconds(
            i64::try_from(ttl_seconds).map_err(|_| "Confirmation TTL overflow")?,
        );
    let record = TaskAuthorityConfirmationRecord {
        schema: TASK_AUTHORITY_CONFIRMATION_SCHEMA.to_string(),
        confirmation_id: confirmation_id.to_string(),
        session_id: session_id.to_string(),
        action: action.to_string(),
        consequence_class: consequence_class.to_string(),
        target_binding: TaskAuthorityConfirmationTargetBinding {
            target_id: target_id.to_string(),
            url: url.to_string(),
        },
        request_sha256: confirmation_request_sha256(command)?,
        requested_by: task_authority_confirmation_requester(action, command)?,
        requested_at: requested_at.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
        state: "pending".to_string(),
        execution_state: "not_dispatched".to_string(),
        decided_by: None,
        decided_at: None,
        decision: None,
        invalidation_reason: None,
        result_sha256: None,
        result_success: None,
        completed_at: None,
        command: command.clone(),
    };
    validate_confirmation_record(&record, session_id)?;
    write_private_json(
        &pending_confirmation_path(root, session_id),
        &record,
        "task authority pending confirmation",
    )?;
    Ok(record)
}

pub fn load_task_authority_pending_confirmation(
    root: &Path,
    session_id: &str,
) -> Result<Option<TaskAuthorityConfirmationRecord>, String> {
    let record =
        read_confirmation_record(&pending_confirmation_path(root, session_id), session_id)?;
    let Some(record) = record else {
        return Ok(None);
    };
    let terminal = terminal_confirmation_path(root, session_id, &record.confirmation_id);
    if terminal.exists() {
        remove_pending_confirmation(root, session_id)?;
        return Ok(None);
    }
    Ok(Some(record))
}

pub fn active_task_authority_pending_confirmation(
    root: &Path,
    session_id: &str,
) -> Result<Option<TaskAuthorityConfirmationRecord>, String> {
    let Some(record) = load_task_authority_pending_confirmation(root, session_id)? else {
        return Ok(None);
    };
    let expires_at = DateTime::parse_from_rfc3339(&record.expires_at)
        .map_err(|error| format!("Invalid task authority confirmation expiry: {error}"))?
        .with_timezone(&Utc);
    if Utc::now() >= expires_at {
        expire_pending_confirmation(root, record)?;
        return Ok(None);
    }
    Ok(Some(record))
}

/// Commit a single-use decision before dispatch. Every mismatch consumes and
/// archives the pending record so callers cannot probe or replay it.
pub struct DecideTaskAuthorityConfirmation<'a> {
    pub root: &'a Path,
    pub session_id: &'a str,
    pub confirmation_id: &'a str,
    pub expected_action: &'a str,
    pub decision: &'a str,
    pub decided_by: TaskAuthorityIssuer,
    pub target_id: Option<&'a str>,
    pub url: Option<&'a str>,
}

pub fn decide_task_authority_confirmation(
    input: DecideTaskAuthorityConfirmation<'_>,
) -> Result<TaskAuthorityConfirmationRecord, String> {
    let DecideTaskAuthorityConfirmation {
        root,
        session_id,
        confirmation_id,
        expected_action,
        decision,
        decided_by,
        target_id,
        url,
    } = input;
    if !matches!(decision, "confirm" | "deny")
        || decided_by.id.trim().is_empty()
        || !matches!(decided_by.kind.as_str(), "operator" | "service")
    {
        return Err("Task authority confirmation requires a valid decision actor".to_string());
    }
    let mut record = load_task_authority_pending_confirmation(root, session_id)?
        .ok_or("No pending task authority confirmation")?;
    let expires_at = DateTime::parse_from_rfc3339(&record.expires_at)
        .map_err(|error| format!("Invalid task authority confirmation expiry: {error}"))?
        .with_timezone(&Utc);
    let mismatch = if Utc::now() >= expires_at {
        Some("confirmation expired before decision".to_string())
    } else if record.confirmation_id != confirmation_id {
        Some("confirmation ID mismatch".to_string())
    } else if record.action != expected_action {
        Some("confirmation action mismatch".to_string())
    } else if record.requested_by != decided_by {
        Some("confirmation decision actor mismatch".to_string())
    } else if decision == "confirm" && record.target_binding.target_id != target_id.unwrap_or("") {
        Some("confirmation target changed before decision".to_string())
    } else if decision == "confirm" && record.target_binding.url != url.unwrap_or("") {
        Some("confirmation URL changed before decision".to_string())
    } else if confirmation_request_sha256(&record.command)? != record.request_sha256 {
        Some("confirmation request digest changed before decision".to_string())
    } else {
        None
    };
    if let Some(reason) = mismatch {
        record.state = if reason.starts_with("confirmation expired") {
            "expired".to_string()
        } else {
            "invalidated".to_string()
        };
        record.invalidation_reason = Some(reason.clone());
        archive_confirmation(root, &record)?;
        remove_pending_confirmation(root, session_id)?;
        return Err(format!(
            "Task authority confirmation failed closed: {reason}"
        ));
    }
    record.decision = Some(decision.to_string());
    record.decided_by = Some(decided_by);
    record.decided_at = Some(Utc::now().to_rfc3339());
    record.state = if decision == "confirm" {
        "confirmed".to_string()
    } else {
        "denied".to_string()
    };
    record.execution_state = if decision == "confirm" {
        "dispatched".to_string()
    } else {
        "not_dispatched".to_string()
    };
    archive_confirmation(root, &record)?;
    remove_pending_confirmation(root, session_id)?;
    Ok(record)
}

pub fn finalize_task_authority_confirmation(
    root: &Path,
    record: &TaskAuthorityConfirmationRecord,
    result: &Value,
) -> Result<(), String> {
    let path = terminal_confirmation_path(root, &record.session_id, &record.confirmation_id);
    let existing = read_confirmation_record(&path, &record.session_id)?
        .ok_or("Task authority confirmation decision receipt disappeared before finalization")?;
    if existing != *record {
        return Err(
            "Task authority confirmation decision receipt changed before finalization".to_string(),
        );
    }
    let mut completed = record.clone();
    let bytes = serde_json::to_vec(result)
        .map_err(|error| format!("Failed to serialize confirmation result: {error}"))?;
    completed.result_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
    completed.result_success = result.get("success").and_then(Value::as_bool);
    completed.execution_state = if completed.result_success == Some(true) {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    completed.completed_at = Some(Utc::now().to_rfc3339());
    write_private_json(&path, &completed, "task authority confirmation receipt")
}

fn confirmation_public_receipt(record: &TaskAuthorityConfirmationRecord) -> Value {
    let effectively_expired = record.state == "pending"
        && DateTime::parse_from_rfc3339(&record.expires_at)
            .map(|expires_at| Utc::now() >= expires_at.with_timezone(&Utc))
            .unwrap_or(true);
    let state = if effectively_expired {
        "expired"
    } else {
        record.state.as_str()
    };
    json!({
        "schema": record.schema,
        "confirmationId": record.confirmation_id,
        "sessionId": record.session_id,
        "action": record.action,
        "consequenceClass": record.consequence_class,
        "targetBinding": record.target_binding,
        "requestSha256": record.request_sha256,
        "requestedBy": record.requested_by,
        "requestedAt": record.requested_at,
        "expiresAt": record.expires_at,
        "state": state,
        "executionState": if record.state == "confirmed" && record.execution_state == "dispatched" {
            "indeterminate"
        } else {
            record.execution_state.as_str()
        },
        "decidedBy": record.decided_by,
        "decidedAt": record.decided_at,
        "decision": record.decision,
        "invalidationReason": record.invalidation_reason,
        "resultSha256": record.result_sha256,
        "resultSuccess": record.result_success,
        "completedAt": record.completed_at,
    })
}

pub struct CleanupTaskAuthorityConfirmations<'a> {
    pub root: &'a Path,
    pub session_id: &'a str,
    pub retain_count: usize,
    pub min_age_seconds: u64,
    pub requested_by: TaskAuthorityIssuer,
    pub apply: bool,
    pub review_sha256: Option<&'a str>,
}

fn confirmation_receipt_is_indeterminate(record: &TaskAuthorityConfirmationRecord) -> bool {
    record.state == "confirmed" && record.execution_state == "dispatched"
}

fn confirmation_receipt_age_anchor(
    record: &TaskAuthorityConfirmationRecord,
) -> Result<DateTime<Utc>, String> {
    let value = record
        .completed_at
        .as_deref()
        .or(record.decided_at.as_deref())
        .unwrap_or(&record.requested_at);
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| format!("Invalid task authority confirmation timestamp: {error}"))
}

pub fn cleanup_task_authority_confirmations(
    input: CleanupTaskAuthorityConfirmations<'_>,
) -> Result<Value, String> {
    let CleanupTaskAuthorityConfirmations {
        root,
        session_id,
        retain_count,
        min_age_seconds,
        requested_by,
        apply,
        review_sha256,
    } = input;
    if requested_by.id.trim().is_empty()
        || !matches!(requested_by.kind.as_str(), "operator" | "service")
    {
        return Err(
            "Task authority confirmation cleanup requires a transport principal".to_string(),
        );
    }
    let cutoff = Utc::now()
        - chrono::Duration::seconds(
            i64::try_from(min_age_seconds).map_err(|_| "Confirmation retention age overflow")?,
        );
    let receipts_dir = confirmation_root(root, session_id).join("receipts");
    let mut safe_records = Vec::new();
    let mut indeterminate_count = 0usize;
    if receipts_dir.exists() {
        for entry in fs::read_dir(&receipts_dir)
            .map_err(|error| format!("Failed to list task authority confirmations: {error}"))?
        {
            let entry = entry.map_err(|error| {
                format!("Failed to inspect task authority confirmation: {error}")
            })?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(record) = read_confirmation_record(&entry.path(), session_id)? else {
                continue;
            };
            if confirmation_receipt_is_indeterminate(&record) {
                indeterminate_count += 1;
                continue;
            }
            if record.state != "pending" {
                safe_records.push((
                    confirmation_receipt_age_anchor(&record)?,
                    entry.path(),
                    record,
                ));
            }
        }
    }
    safe_records.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.2.confirmation_id.cmp(&right.2.confirmation_id))
    });
    let removable_count = safe_records.len().saturating_sub(retain_count);
    let candidates: Vec<_> = safe_records
        .iter()
        .take(removable_count)
        .filter(|(anchor, _, _)| *anchor <= cutoff)
        .map(|(_, path, record)| {
            let receipt_bytes = serde_json::to_vec(record)
                .map_err(|error| format!("Failed to serialize confirmation receipt: {error}"))?;
            Ok((
                path.clone(),
                record.confirmation_id.clone(),
                json!({
                    "confirmationId": record.confirmation_id,
                    "state": record.state,
                    "executionState": record.execution_state,
                    "requestedAt": record.requested_at,
                    "receiptSha256": format!("{:x}", Sha256::digest(receipt_bytes)),
                }),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let candidate_evidence: Vec<Value> = candidates
        .iter()
        .map(|(_, _, evidence)| evidence.clone())
        .collect();
    let current_tombstones = load_confirmation_tombstones(root, session_id)?;
    let current_tombstone_evidence = confirmation_tombstone_evidence(&current_tombstones)?;
    let review_payload = json!({
        "schema": "agent-browser.task-authority-confirmation-cleanup-review.v1",
        "sessionId": session_id,
        "retainCount": retain_count,
        "minAgeSeconds": min_age_seconds,
        "requestedBy": requested_by,
        "tombstoneLedger": current_tombstone_evidence,
        "candidates": candidate_evidence,
    });
    let review_bytes = serde_json::to_vec(&review_payload)
        .map_err(|error| format!("Failed to serialize confirmation cleanup review: {error}"))?;
    let calculated_review_sha256 = format!("{:x}", Sha256::digest(review_bytes));
    if apply && review_sha256 != Some(calculated_review_sha256.as_str()) {
        return Err("Task authority confirmation cleanup review digest mismatch".to_string());
    }
    let mut removed = Vec::new();
    let mut final_tombstones = current_tombstones;
    if apply && (!candidates.is_empty() || final_tombstones.legacy) {
        final_tombstones = persist_confirmation_tombstones(
            root,
            session_id,
            final_tombstones,
            candidates
                .iter()
                .map(|(_, confirmation_id, _)| confirmation_id.clone()),
        )?;
        for (path, confirmation_id, _) in &candidates {
            if path.exists() {
                fs::remove_file(path).map_err(|error| {
                    format!("Failed to remove retired confirmation receipt: {error}")
                })?;
            }
            removed.push(confirmation_id.clone());
        }
    }
    let final_tombstone_evidence = confirmation_tombstone_evidence(&final_tombstones)?;
    Ok(json!({
        "schema": "agent-browser.task-authority-confirmation-cleanup.v1",
        "sessionId": session_id,
        "apply": apply,
        "reviewSha256": calculated_review_sha256,
        "policy": {"retainCount": retain_count, "minAgeSeconds": min_age_seconds},
        "requestedBy": requested_by,
        "tombstoneLedger": final_tombstone_evidence,
        "pendingPreserved": pending_confirmation_path(root, session_id).exists(),
        "indeterminatePreservedCount": indeterminate_count,
        "terminalReceiptCount": safe_records.len() + indeterminate_count,
        "candidateCount": candidates.len(),
        "candidates": candidate_evidence,
        "removedCount": removed.len(),
        "removedConfirmationIds": removed,
    }))
}

pub fn task_authority_confirmation_status(root: &Path, session_id: &str) -> Result<Value, String> {
    let mut records = Vec::new();
    if let Some(pending) = load_task_authority_pending_confirmation(root, session_id)? {
        records.push(confirmation_public_receipt(&pending));
    }
    let receipts = confirmation_root(root, session_id).join("receipts");
    if receipts.exists() {
        for entry in fs::read_dir(receipts)
            .map_err(|error| format!("Failed to list task authority confirmations: {error}"))?
        {
            let entry = entry.map_err(|error| {
                format!("Failed to inspect task authority confirmation: {error}")
            })?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Some(record) = read_confirmation_record(&entry.path(), session_id)? {
                records.push(confirmation_public_receipt(&record));
            }
        }
    }
    records.sort_by(|left, right| {
        left.get("requestedAt")
            .and_then(Value::as_str)
            .cmp(&right.get("requestedAt").and_then(Value::as_str))
            .then_with(|| {
                left.get("confirmationId")
                    .and_then(Value::as_str)
                    .cmp(&right.get("confirmationId").and_then(Value::as_str))
            })
    });
    Ok(json!({
        "schema": "agent-browser.task-authority-confirmation-collection.v1",
        "sessionId": session_id,
        "count": records.len(),
        "confirmations": records,
    }))
}

fn require_label(cmd: &Value, field: &str, expected: &str) -> Result<(), String> {
    let actual = cmd
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Task authority requires top-level {field}"))?;
    if actual != expected {
        return Err(format!(
            "Task authority {field} mismatch: expected '{expected}', got '{actual}'"
        ));
    }
    Ok(())
}

fn read_issuance(path: &Path) -> Result<Option<TaskAuthorityIssuanceRecord>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read task authority issuance: {error}"))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| format!("Invalid task authority issuance: {error}"))
}

fn authority_status_receipt(
    record: &TaskAuthorityIssuanceRecord,
    ledger: Option<&TaskAuthorityLedger>,
) -> Result<Value, String> {
    if envelope_sha256(&record.envelope)? != record.envelope_sha256 {
        return Err("Task authority issuance envelope hash is invalid".to_string());
    }
    if record.schema == "agent-browser.task-authority-issuance.v2" {
        if record.approved_plan.steps.is_empty() {
            return Err("Task authority ordered plan is empty".to_string());
        }
        let plan_sha256 = approved_plan_sha256(&record.approved_plan.steps)?;
        if record.approved_plan.plan_sha256.as_deref() != Some(plan_sha256.as_str())
            || record.envelope.plan_sha256.as_deref() != Some(plan_sha256.as_str())
        {
            return Err("Task authority issuance plan hash is invalid".to_string());
        }
        if let Some(ledger) = ledger {
            validate_ordered_ledger(ledger, record)?;
        }
    }
    let admitted_actions = ledger.map(|value| value.admitted_actions).unwrap_or(0);
    let reserved_evidence_bytes = ledger
        .map(|value| value.reserved_evidence_bytes)
        .unwrap_or(0);
    let max_actions = record.envelope.evidence_budget.max_actions;
    let max_evidence_bytes = record.envelope.evidence_budget.max_evidence_bytes;
    let expires_at = DateTime::parse_from_rfc3339(&record.envelope.expires_at)
        .map_err(|error| format!("Invalid issued task authority expiry: {error}"))?;
    let expired = expires_at.with_timezone(&Utc) <= Utc::now();
    let exhausted =
        admitted_actions >= max_actions || reserved_evidence_bytes >= max_evidence_bytes;
    let state = if record.revocation.is_some() {
        "revoked"
    } else if expired {
        "expired"
    } else if exhausted {
        "exhausted"
    } else {
        "active"
    };
    let next_step_index = ledger.map(|value| value.next_step_index).unwrap_or(0);
    let next_step = record
        .approved_plan
        .steps
        .get(usize::try_from(next_step_index).unwrap_or(usize::MAX));
    let step_receipts = ledger
        .map(|value| value.step_receipts.clone())
        .unwrap_or_default();
    let step_outcomes = step_receipts
        .iter()
        .map(|receipt| {
            let mut value = serde_json::to_value(receipt)
                .expect("task authority step receipt serialization must succeed");
            if receipt.outcome.is_none() {
                value["outcome"] = json!({
                    "state": "indeterminate",
                    "success": Value::Null,
                    "reason": "admitted_without_durable_terminal_outcome"
                });
            }
            value
        })
        .collect::<Vec<_>>();
    let completed_steps = step_outcomes
        .iter()
        .filter(|value| value["outcome"]["state"] == "completed")
        .cloned()
        .collect::<Vec<_>>();
    let failed_steps = step_outcomes
        .iter()
        .filter(|value| value["outcome"]["state"] == "failed")
        .cloned()
        .collect::<Vec<_>>();
    let indeterminate_steps = step_outcomes
        .iter()
        .filter(|value| value["outcome"]["state"] == "indeterminate")
        .cloned()
        .collect::<Vec<_>>();
    let completed_count = completed_steps.len();
    let failed_count = failed_steps.len();
    let indeterminate_count = indeterminate_steps.len();
    Ok(json!({
        "schema": "agent-browser.task-authority-status.v1",
        "id": record.envelope.id,
        "sessionId": record.session_id,
        "state": state,
        "usable": state == "active",
        "envelope": record.envelope,
        "envelopeSha256": record.envelope_sha256,
        "issuer": record.issuer,
        "approvalReference": record.approval_reference,
        "approvedPlan": record.approved_plan,
        "issuedAt": record.issued_at,
        "revocation": record.revocation,
        "usage": {
            "admittedActions": admitted_actions,
            "maxActions": max_actions,
            "remainingActions": max_actions.saturating_sub(admitted_actions),
            "reservedEvidenceBytes": reserved_evidence_bytes,
            "maxEvidenceBytes": max_evidence_bytes,
            "remainingEvidenceBytes": max_evidence_bytes.saturating_sub(reserved_evidence_bytes),
            "ledgerUpdatedAt": ledger.map(|value| value.updated_at.clone()),
            "nextStepIndex": next_step_index,
            "nextStep": next_step,
            "admittedSteps": step_receipts,
            "stepOutcomes": step_outcomes,
            "completedSteps": completed_steps,
            "failedSteps": failed_steps,
            "indeterminateSteps": indeterminate_steps,
            "outcomeSummary": {
                "completed": completed_count,
                "failed": failed_count,
                "indeterminate": indeterminate_count,
            },
            "remainingSteps": record.approved_plan.steps.len().saturating_sub(next_step_index as usize),
        },
    }))
}

/// Durably classify one already-admitted ordered step before its response is
/// published. An admitted receipt without this terminal outcome is projected
/// as indeterminate after restart and remains consumed; callers must replan
/// rather than replay it.
pub fn finalize_task_authority_step(
    cmd: &Value,
    context: &TaskAuthorityContext<'_>,
    response: &Value,
) -> Result<(), String> {
    let Some(raw_authority) = cmd.get("taskAuthority") else {
        return Ok(());
    };
    let authority: TaskAuthorityEnvelope = serde_json::from_value(raw_authority.clone())
        .map_err(|error| format!("Invalid taskAuthority envelope: {error}"))?;
    let step_id = cmd
        .get("taskStepId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("Task authority outcome requires top-level taskStepId")?;
    let command_id = cmd
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("Task authority outcome requires a top-level command id")?;
    let issuance = read_issuance(&issuance_path(
        &context.ledger_root,
        context.session_id,
        &authority.id,
    ))?
    .ok_or_else(|| {
        format!(
            "Task authority '{}' was not issued by this broker",
            authority.id
        )
    })?;
    if issuance.schema != "agent-browser.task-authority-issuance.v2"
        || issuance.session_id != context.session_id
        || issuance.envelope != authority
        || issuance.envelope_sha256 != envelope_sha256(&authority)?
        || issuance.revocation.is_some()
    {
        return Err("Task authority outcome issuance identity is invalid".to_string());
    }
    let path = ledger_path(&context.ledger_root, context.session_id, &authority.id);
    let mut ledger =
        read_ledger(&path)?.ok_or("Task authority outcome has no durable admission ledger")?;
    validate_ordered_ledger(&ledger, &issuance)?;
    let receipt = ledger
        .step_receipts
        .iter_mut()
        .find(|receipt| receipt.step_id == step_id && receipt.command_id == command_id)
        .ok_or_else(|| {
            format!(
                "Task authority outcome does not match admitted step '{}' and command '{}'",
                step_id, command_id
            )
        })?;
    let response_bytes = serde_json::to_vec(response)
        .map_err(|error| format!("Failed to serialize task authority outcome: {error}"))?;
    let success = response
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let outcome = TaskAuthorityStepOutcome {
        state: if success { "completed" } else { "failed" }.to_string(),
        success,
        response_sha256: format!("{:x}", Sha256::digest(&response_bytes)),
        response_bytes: u64::try_from(response_bytes.len())
            .map_err(|_| "Task authority outcome response size overflow")?,
        target_id: context.target_id.map(str::to_string),
        url: context.url.map(normalized_url).transpose()?,
        finalized_at: Utc::now().to_rfc3339(),
    };
    if let Some(existing) = receipt.outcome.as_ref() {
        let same_terminal_evidence = existing.state == outcome.state
            && existing.success == outcome.success
            && existing.response_sha256 == outcome.response_sha256
            && existing.response_bytes == outcome.response_bytes
            && existing.target_id == outcome.target_id
            && existing.url == outcome.url;
        if same_terminal_evidence {
            return Ok(());
        }
        return Err(format!(
            "Task authority step '{}' already has a conflicting terminal outcome",
            step_id
        ));
    }
    receipt.outcome = Some(outcome);
    ledger.updated_at = Utc::now().to_rfc3339();
    write_ledger(&path, &ledger)
}

/// Mint an immutable authority from bounded plan steps and the exact active
/// target observed by the broker. This initial issuer intentionally permits
/// only read and navigation actions; navigation remains above the read-only
/// consequence ceiling and therefore requires a fresh target confirmation.
pub fn issue_task_authority(
    raw_request: &Value,
    session_id: &str,
    target_id: &str,
    current_url: &str,
    root: &Path,
) -> Result<Value, String> {
    issue_task_authority_with_identity(
        raw_request,
        session_id,
        target_id,
        current_url,
        root,
        None,
        None,
    )
}

fn issue_task_authority_with_identity(
    raw_request: &Value,
    session_id: &str,
    target_id: &str,
    current_url: &str,
    root: &Path,
    authority_id: Option<String>,
    lineage: Option<TaskAuthorityLineage>,
) -> Result<Value, String> {
    let request: TaskAuthorityIssueRequest = serde_json::from_value(raw_request.clone())
        .map_err(|error| format!("Invalid task authority issue request: {error}"))?;
    if request.task_name.trim().is_empty()
        || request.expected_target_id.trim().is_empty()
        || request.expected_url.trim().is_empty()
    {
        return Err(
            "Task authority issue requires taskName, expectedTargetId, and expectedUrl".to_string(),
        );
    }
    if request.expected_target_id != target_id || request.expected_url != current_url {
        return Err(format!(
            "Task authority issue target changed: expected '{}' at '{}', got '{}' at '{}'",
            request.expected_target_id, request.expected_url, target_id, current_url
        ));
    }
    if !matches!(request.issuer.kind.as_str(), "operator" | "service")
        || request.issuer.id.trim().is_empty()
        || request.approval_reference.trim().is_empty()
    {
        return Err(
            "Task authority issue requires operator or service issuer id and approvalReference"
                .to_string(),
        );
    }
    if request.expires_in_seconds == 0
        || request.expires_in_seconds > MAX_ISSUED_AUTHORITY_TTL_SECONDS
    {
        return Err(format!(
            "Task authority expiresInSeconds must be between 1 and {MAX_ISSUED_AUTHORITY_TTL_SECONDS}"
        ));
    }
    if request.steps.is_empty() || request.steps.len() > MAX_ISSUED_AUTHORITY_STEPS {
        return Err(format!(
            "Task authority steps must contain between 1 and {MAX_ISSUED_AUTHORITY_STEPS} entries"
        ));
    }

    let authority_id =
        authority_id.unwrap_or_else(|| format!("authority-{}", uuid::Uuid::new_v4()));
    let mut actions = BTreeSet::new();
    let mut origins = BTreeSet::from([normalized_origin(current_url)?]);
    let mut evidence_bytes = 0u64;
    let mut approved_steps = Vec::with_capacity(request.steps.len());
    let mut expected_current_url = normalized_url(current_url)?;
    for (index, step) in request.steps.iter().enumerate() {
        let action = step.action.trim();
        if action.is_empty() {
            return Err("Task authority plan action must be non-empty".to_string());
        }
        let consequence = super::policy::action_consequence(action);
        if consequence.authority_rank() > ActionConsequence::Navigation.authority_rank() {
            return Err(format!(
                "Task authority issuer does not yet permit '{}' ({}) in an approved plan",
                action,
                consequence.as_str()
            ));
        }
        actions.insert(action.to_string());
        let requested_url = step.url.as_deref().map(normalized_url).transpose()?;
        if consequence == ActionConsequence::Navigation && requested_url.is_none() {
            return Err(format!(
                "Task authority navigation step '{}' requires an exact url",
                action
            ));
        }
        if let Some(url) = requested_url.as_deref() {
            origins.insert(normalized_origin(url)?);
        }
        let reservation = step
            .evidence_bytes
            .unwrap_or_else(|| default_evidence_reservation(action));
        if reservation == 0 {
            return Err("Task authority plan evidenceBytes must be positive".to_string());
        }
        evidence_bytes = evidence_bytes
            .checked_add(reservation)
            .ok_or("Task authority plan evidence budget overflow")?;
        let step_index = u64::try_from(index).map_err(|_| "Task authority step index overflow")?;
        approved_steps.push(TaskAuthorityApprovedStep {
            step_id: format!("{authority_id}:step-{step_index}"),
            index: step_index,
            action: action.to_string(),
            expected_current_url: expected_current_url.clone(),
            requested_url: requested_url.clone(),
            evidence_bytes: reservation,
        });
        if consequence == ActionConsequence::Navigation {
            expected_current_url =
                requested_url.ok_or("Task authority navigation step lost its requested URL")?;
        }
    }

    let issued_at = Utc::now();
    let expires_at = issued_at
        + chrono::Duration::seconds(
            i64::try_from(request.expires_in_seconds)
                .map_err(|_| "Task authority expiry is too large")?,
        );
    let plan_sha256 = approved_plan_sha256(&approved_steps)?;
    let allowed_actions = actions.into_iter().collect::<Vec<_>>();
    let allowed_origins = origins.into_iter().collect::<Vec<_>>();
    let envelope = TaskAuthorityEnvelope {
        id: authority_id.clone(),
        task_name: request.task_name,
        service_name: request.service_name,
        agent_name: request.agent_name,
        allowed_origins: allowed_origins.clone(),
        allowed_actions: allowed_actions.clone(),
        plan_sha256: Some(plan_sha256.clone()),
        lineage,
        target_binding: TaskAuthorityTargetBinding {
            target_id: target_id.to_string(),
            initial_url: current_url.to_string(),
        },
        evidence_budget: TaskAuthorityEvidenceBudget {
            max_actions: u64::try_from(request.steps.len())
                .map_err(|_| "Task authority action count overflow")?,
            max_evidence_bytes: evidence_bytes,
        },
        consequence_ceiling: ActionConsequence::ReadOnly.as_str().to_string(),
        expires_at: expires_at.to_rfc3339(),
    };
    let hash = envelope_sha256(&envelope)?;
    let record = TaskAuthorityIssuanceRecord {
        schema: "agent-browser.task-authority-issuance.v2".to_string(),
        session_id: session_id.to_string(),
        envelope,
        envelope_sha256: hash,
        issuer: request.issuer,
        approval_reference: request.approval_reference,
        approved_plan: TaskAuthorityApprovedPlan {
            actions: allowed_actions,
            origins: allowed_origins,
            action_count: u64::try_from(request.steps.len())
                .map_err(|_| "Task authority action count overflow")?,
            evidence_bytes,
            steps: approved_steps,
            plan_sha256: Some(plan_sha256),
        },
        issued_at: issued_at.to_rfc3339(),
        revocation: None,
    };
    let path = issuance_path(root, session_id, &authority_id);
    if path.exists() {
        let existing = read_issuance(&path)?
            .ok_or("Task authority issuance disappeared during idempotency check")?;
        if existing == record {
            let ledger = read_ledger(&ledger_path(root, session_id, &authority_id))?;
            let mut receipt = authority_status_receipt(&existing, ledger.as_ref())?;
            receipt["idempotent"] = json!(true);
            return Ok(receipt);
        }
        return Err(
            "Generated task authority id already exists with different evidence".to_string(),
        );
    }
    write_private_json(&path, &record, "task authority issuance")?;
    let mut receipt = authority_status_receipt(&record, None)?;
    receipt["idempotent"] = json!(false);
    Ok(receipt)
}

/// Return durable broker issuance, revocation, and usage evidence without
/// requiring or changing browser state.
pub fn task_authority_status(
    root: &Path,
    session_id: &str,
    authority_id: Option<&str>,
) -> Result<Value, String> {
    let session_root = root.join(safe_component(session_id));
    let mut records = Vec::new();
    if let Some(authority_id) = authority_id {
        let record =
            read_issuance(&issuance_path(root, session_id, authority_id))?.ok_or_else(|| {
                format!("Task authority '{authority_id}' was not issued by this broker")
            })?;
        let ledger = read_ledger(&ledger_path(root, session_id, authority_id))?;
        return authority_status_receipt(&record, ledger.as_ref());
    }
    if session_root.exists() {
        for entry in fs::read_dir(&session_root)
            .map_err(|error| format!("Failed to list task authority issuances: {error}"))?
        {
            let entry = entry
                .map_err(|error| format!("Failed to inspect task authority issuance: {error}"))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".issuance.json") {
                continue;
            }
            let record = read_issuance(&entry.path())?
                .ok_or("Task authority issuance disappeared during status read")?;
            let ledger = read_ledger(&ledger_path(root, session_id, &record.envelope.id))?;
            records.push(authority_status_receipt(&record, ledger.as_ref())?);
        }
    }
    records.sort_by(|left, right| {
        left.get("issuedAt")
            .and_then(Value::as_str)
            .cmp(&right.get("issuedAt").and_then(Value::as_str))
            .then_with(|| {
                left.get("id")
                    .and_then(Value::as_str)
                    .cmp(&right.get("id").and_then(Value::as_str))
            })
    });
    Ok(json!({
        "schema": "agent-browser.task-authority-collection.v1",
        "sessionId": session_id,
        "count": records.len(),
        "authorities": records,
    }))
}

/// Revoke one broker-issued authority. Repeating the same revocation is
/// idempotent; conflicting revocation evidence fails closed.
pub fn revoke_task_authority(
    root: &Path,
    session_id: &str,
    authority_id: &str,
    revoked_by: &str,
    reason: &str,
    target_id: &str,
    current_url: &str,
) -> Result<Value, String> {
    if revoked_by.trim().is_empty() || reason.trim().is_empty() {
        return Err("Task authority revoke requires revokedBy and reason".to_string());
    }
    let path = issuance_path(root, session_id, authority_id);
    let mut record = read_issuance(&path)?
        .ok_or_else(|| format!("Task authority '{authority_id}' was not issued by this broker"))?;
    if record.envelope.target_binding.target_id != target_id {
        return Err("Task authority revoke target no longer matches its issued target".to_string());
    }
    if let Some(existing) = record.revocation.as_ref() {
        if existing.revoked_by != revoked_by || existing.reason != reason {
            return Err("Task authority already has different revocation evidence".to_string());
        }
        let ledger = read_ledger(&ledger_path(root, session_id, authority_id))?;
        let mut receipt = authority_status_receipt(&record, ledger.as_ref())?;
        receipt["idempotent"] = json!(true);
        return Ok(receipt);
    }
    record.revocation = Some(TaskAuthorityRevocation {
        revoked_at: Utc::now().to_rfc3339(),
        revoked_by: revoked_by.to_string(),
        reason: reason.to_string(),
        target_url: current_url.to_string(),
        reconciliation: None,
    });
    write_private_json(&path, &record, "task authority issuance")?;
    let ledger = read_ledger(&ledger_path(root, session_id, authority_id))?;
    let mut receipt = authority_status_receipt(&record, ledger.as_ref())?;
    receipt["idempotent"] = json!(false);
    Ok(receipt)
}

fn reconciliation_request_sha256(
    session_id: &str,
    predecessor_authority_id: &str,
    request: &TaskAuthorityReconcileRequest,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(&json!({
        "sessionId": session_id,
        "predecessorAuthorityId": predecessor_authority_id,
        "request": request,
    }))
    .map_err(|error| format!("Failed to serialize task authority reconciliation: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Reconcile exactly one crash-stranded ordered step. The predecessor is
/// durably revoked before a lineage-bound replacement is created. A retry with
/// identical evidence resumes the same deterministic replacement; any changed
/// evidence fails closed.
pub fn reconcile_task_authority(
    root: &Path,
    session_id: &str,
    predecessor_authority_id: &str,
    raw_request: &Value,
    target_id: &str,
    current_url: &str,
) -> Result<Value, String> {
    reconcile_task_authority_inner(
        root,
        session_id,
        predecessor_authority_id,
        raw_request,
        target_id,
        current_url,
        false,
    )
}

fn reconcile_task_authority_inner(
    root: &Path,
    session_id: &str,
    predecessor_authority_id: &str,
    raw_request: &Value,
    target_id: &str,
    current_url: &str,
    stop_after_predecessor_revocation: bool,
) -> Result<Value, String> {
    let request: TaskAuthorityReconcileRequest = serde_json::from_value(raw_request.clone())
        .map_err(|error| format!("Invalid task authority reconcile request: {error}"))?;
    if request.reconciliation_id.trim().is_empty()
        || request.unresolved_step_id.trim().is_empty()
        || request.task_name.trim().is_empty()
        || request.expected_target_id.trim().is_empty()
        || request.expected_url.trim().is_empty()
    {
        return Err("Task authority reconcile requires reconciliationId, unresolvedStepId, taskName, expectedTargetId, and expectedUrl".to_string());
    }
    if !matches!(request.issuer.kind.as_str(), "operator" | "service")
        || request.issuer.id.trim().is_empty()
        || request.approval_reference.trim().is_empty()
    {
        return Err(
            "Task authority reconcile requires operator or service issuer id and approvalReference"
                .to_string(),
        );
    }
    if request.expires_in_seconds == 0
        || request.expires_in_seconds > MAX_ISSUED_AUTHORITY_TTL_SECONDS
        || request.steps.is_empty()
        || request.steps.len() > MAX_ISSUED_AUTHORITY_STEPS
    {
        return Err(
            "Task authority reconcile replacement lifetime or step count is invalid".to_string(),
        );
    }
    let mut replacement_evidence_bytes = 0u64;
    for step in &request.steps {
        let action = step.action.trim();
        let consequence = super::policy::action_consequence(action);
        if action.is_empty()
            || consequence.authority_rank() > ActionConsequence::Navigation.authority_rank()
            || (consequence == ActionConsequence::Navigation && step.url.is_none())
        {
            return Err(
                "Task authority reconcile replacement contains an invalid action".to_string(),
            );
        }
        if let Some(url) = step.url.as_deref() {
            normalized_url(url)?;
        }
        let reservation = step
            .evidence_bytes
            .unwrap_or_else(|| default_evidence_reservation(action));
        if reservation == 0 {
            return Err(
                "Task authority reconcile replacement evidenceBytes must be positive".to_string(),
            );
        }
        replacement_evidence_bytes = replacement_evidence_bytes
            .checked_add(reservation)
            .ok_or("Task authority reconcile replacement evidence budget overflow")?;
    }
    if request.expected_target_id != target_id || request.expected_url != current_url {
        return Err(format!(
            "Task authority reconcile target changed: expected '{}' at '{}', got '{}' at '{}'",
            request.expected_target_id, request.expected_url, target_id, current_url
        ));
    }
    let session_root = root.join(safe_component(session_id));
    if session_root.exists() {
        for entry in fs::read_dir(&session_root).map_err(|error| {
            format!("Failed to inspect task authority reconciliation IDs: {error}")
        })? {
            let entry = entry.map_err(|error| {
                format!("Failed to inspect task authority reconciliation record: {error}")
            })?;
            if !entry
                .file_name()
                .to_string_lossy()
                .ends_with(".issuance.json")
            {
                continue;
            }
            let Some(existing_record) = read_issuance(&entry.path())? else {
                continue;
            };
            let reused_elsewhere = existing_record.envelope.id != predecessor_authority_id
                && (existing_record
                    .revocation
                    .as_ref()
                    .and_then(|value| value.reconciliation.as_ref())
                    .is_some_and(|value| value.reconciliation_id == request.reconciliation_id)
                    || existing_record
                        .envelope
                        .lineage
                        .as_ref()
                        .is_some_and(|value| {
                            value.reconciliation_id == request.reconciliation_id
                                && value.predecessor_authority_id != predecessor_authority_id
                        }));
            if reused_elsewhere {
                return Err(format!(
                    "Task authority reconciliationId '{}' is already bound to another authority",
                    request.reconciliation_id
                ));
            }
        }
    }

    let predecessor_path = issuance_path(root, session_id, predecessor_authority_id);
    let mut predecessor = read_issuance(&predecessor_path)?.ok_or_else(|| {
        format!("Task authority '{predecessor_authority_id}' was not issued by this broker")
    })?;
    if predecessor.schema != "agent-browser.task-authority-issuance.v2"
        || predecessor.session_id != session_id
        || predecessor.envelope.target_binding.target_id != target_id
        || predecessor.envelope.task_name != request.task_name
        || predecessor.envelope.service_name != request.service_name
        || predecessor.envelope.agent_name != request.agent_name
    {
        return Err("Task authority reconcile predecessor identity is invalid".to_string());
    }
    let ledger = read_ledger(&ledger_path(root, session_id, predecessor_authority_id))?
        .ok_or("Task authority reconcile predecessor has no durable admission ledger")?;
    validate_ordered_ledger(&ledger, &predecessor)?;
    let indeterminate = ledger
        .step_receipts
        .iter()
        .filter(|receipt| receipt.outcome.is_none())
        .collect::<Vec<_>>();
    if indeterminate.len() != 1 {
        return Err(format!(
            "Task authority reconcile requires exactly one indeterminate predecessor step, found {}",
            indeterminate.len()
        ));
    }
    let stranded = indeterminate[0];
    if stranded.step_id != request.unresolved_step_id
        || stranded.target_id != target_id
        || ledger.next_step_index <= stranded.step_index
    {
        return Err("Task authority reconcile unresolved step does not match the consumed predecessor receipt".to_string());
    }

    let request_sha256 =
        reconciliation_request_sha256(session_id, predecessor_authority_id, &request)?;
    let replacement_authority_id = format!("authority-reconcile-{}", &request_sha256[..32]);
    let lineage = TaskAuthorityLineage {
        reconciliation_id: request.reconciliation_id.clone(),
        predecessor_authority_id: predecessor_authority_id.to_string(),
        predecessor_step_id: stranded.step_id.clone(),
        predecessor_step_index: stranded.step_index,
        predecessor_command_id: stranded.command_id.clone(),
        predecessor_outcome_state: "indeterminate".to_string(),
    };
    let expected_reconciliation = TaskAuthorityReconciliation {
        reconciliation_id: request.reconciliation_id.clone(),
        request_sha256: request_sha256.clone(),
        predecessor_step_id: stranded.step_id.clone(),
        predecessor_step_index: stranded.step_index,
        predecessor_command_id: stranded.command_id.clone(),
        predecessor_outcome_state: "indeterminate".to_string(),
        replacement_authority_id: replacement_authority_id.clone(),
        state: "pending".to_string(),
        completed_at: None,
    };

    if let Some(revocation) = predecessor.revocation.as_ref() {
        let Some(existing) = revocation.reconciliation.as_ref() else {
            return Err(
                "Task authority reconcile predecessor was already revoked outside reconciliation"
                    .to_string(),
            );
        };
        let mut comparable = existing.clone();
        comparable.state = "pending".to_string();
        comparable.completed_at = None;
        if comparable != expected_reconciliation {
            return Err(
                "Task authority reconciliation already exists with different evidence".to_string(),
            );
        }
    } else {
        predecessor.revocation = Some(TaskAuthorityRevocation {
            revoked_at: Utc::now().to_rfc3339(),
            revoked_by: request.issuer.id.clone(),
            reason: format!("reconcile indeterminate step {}", stranded.step_id),
            target_url: current_url.to_string(),
            reconciliation: Some(expected_reconciliation.clone()),
        });
        write_private_json(
            &predecessor_path,
            &predecessor,
            "task authority reconciliation predecessor",
        )?;
        if stop_after_predecessor_revocation {
            return Err("Injected stop after predecessor revocation".to_string());
        }
    }

    let replacement_path = issuance_path(root, session_id, &replacement_authority_id);
    let replacement = if replacement_path.exists() {
        let existing = read_issuance(&replacement_path)?
            .ok_or("Task authority reconciliation replacement disappeared")?;
        if existing.schema != "agent-browser.task-authority-issuance.v2"
            || existing.session_id != session_id
            || existing.envelope.id != replacement_authority_id
            || existing.envelope.lineage.as_ref() != Some(&lineage)
            || existing.envelope_sha256 != envelope_sha256(&existing.envelope)?
        {
            return Err(
                "Task authority reconciliation replacement identity is invalid".to_string(),
            );
        }
        authority_status_receipt(
            &existing,
            read_ledger(&ledger_path(root, session_id, &replacement_authority_id))?.as_ref(),
        )?
    } else {
        let issue_request = serde_json::to_value(TaskAuthorityIssueRequest {
            task_name: request.task_name.clone(),
            service_name: request.service_name.clone(),
            agent_name: request.agent_name.clone(),
            expected_target_id: request.expected_target_id.clone(),
            expected_url: request.expected_url.clone(),
            issuer: request.issuer.clone(),
            approval_reference: request.approval_reference.clone(),
            expires_in_seconds: request.expires_in_seconds,
            steps: request.steps.clone(),
        })
        .map_err(|error| format!("Failed to prepare replacement authority: {error}"))?;
        issue_task_authority_with_identity(
            &issue_request,
            session_id,
            target_id,
            current_url,
            root,
            Some(replacement_authority_id.clone()),
            Some(lineage.clone()),
        )?
    };

    let reconciliation = predecessor
        .revocation
        .as_mut()
        .and_then(|revocation| revocation.reconciliation.as_mut())
        .ok_or("Task authority reconciliation predecessor evidence disappeared")?;
    let was_completed = reconciliation.state == "completed";
    if !was_completed {
        reconciliation.state = "completed".to_string();
        reconciliation.completed_at = Some(Utc::now().to_rfc3339());
        write_private_json(
            &predecessor_path,
            &predecessor,
            "task authority reconciliation predecessor",
        )?;
    }
    let predecessor_status = authority_status_receipt(&predecessor, Some(&ledger))?;
    Ok(json!({
        "schema": "agent-browser.task-authority-reconciliation.v1",
        "idempotent": was_completed,
        "reconciliationId": request.reconciliation_id,
        "requestSha256": request_sha256,
        "predecessor": predecessor_status,
        "replacement": replacement,
        "lineage": lineage,
    }))
}

pub fn admit_task_authority(
    cmd: &Value,
    action: &str,
    consequence: ActionConsequence,
    context: &TaskAuthorityContext<'_>,
    reserve_budget: bool,
) -> Result<TaskAuthorityDecision, String> {
    let Some(raw) = cmd.get("taskAuthority") else {
        if context.require_authority {
            return Err("Task authority is required for this agentic browser command".to_string());
        }
        return Ok(TaskAuthorityDecision::NotPresent);
    };
    let envelope: TaskAuthorityEnvelope = serde_json::from_value(raw.clone())
        .map_err(|error| format!("Invalid taskAuthority envelope: {error}"))?;
    if envelope.id.trim().is_empty() || envelope.task_name.trim().is_empty() {
        return Err("Task authority id and taskName must be non-empty".to_string());
    }
    if envelope.allowed_origins.is_empty() {
        return Err("Task authority allowedOrigins must be non-empty".to_string());
    }
    if !envelope.allowed_actions.is_empty()
        && !envelope
            .allowed_actions
            .iter()
            .any(|allowed| allowed == action)
    {
        return Err(format!(
            "Task authority '{}' does not allow action '{}'",
            envelope.id, action
        ));
    }
    if envelope.target_binding.target_id.trim().is_empty()
        || envelope.target_binding.initial_url.trim().is_empty()
    {
        return Err("Task authority targetBinding requires targetId and initialUrl".to_string());
    }
    if envelope.evidence_budget.max_actions == 0 || envelope.evidence_budget.max_evidence_bytes == 0
    {
        return Err("Task authority evidenceBudget values must be positive".to_string());
    }
    require_label(cmd, "taskName", &envelope.task_name)?;
    if let Some(service_name) = envelope.service_name.as_deref() {
        require_label(cmd, "serviceName", service_name)?;
    }
    if let Some(agent_name) = envelope.agent_name.as_deref() {
        require_label(cmd, "agentName", agent_name)?;
    }
    let expires_at = DateTime::parse_from_rfc3339(&envelope.expires_at)
        .map_err(|error| format!("Invalid task authority expiresAt: {error}"))?;
    if expires_at.with_timezone(&Utc) <= Utc::now() {
        return Err(format!("Task authority '{}' expired", envelope.id));
    }
    let ceiling = ActionConsequence::parse(&envelope.consequence_ceiling).ok_or_else(|| {
        format!(
            "Invalid task authority consequenceCeiling '{}'",
            envelope.consequence_ceiling
        )
    })?;
    let target_id = context
        .target_id
        .ok_or("Task authority requires a live retained target")?;
    if target_id != envelope.target_binding.target_id {
        return Err(format!(
            "Task authority target mismatch: expected '{}', got '{}'",
            envelope.target_binding.target_id, target_id
        ));
    }

    let allowed_origins = envelope
        .allowed_origins
        .iter()
        .map(|origin| normalized_origin(origin))
        .collect::<Result<Vec<_>, _>>()?;
    let current_url = context
        .url
        .ok_or("Task authority requires the retained target URL")?;
    let current_origin = normalized_origin(current_url)?;
    if !allowed_origins.contains(&current_origin) {
        return Err(format!(
            "Task authority origin '{}' is not allowed",
            current_origin
        ));
    }
    if let Some(url) = requested_url(cmd, context.url) {
        let origin = normalized_origin(url)?;
        if !allowed_origins.contains(&origin) {
            return Err(format!("Task authority origin '{}' is not allowed", origin));
        }
    }

    let hash = envelope_sha256(&envelope)?;
    let issued = read_issuance(&issuance_path(
        &context.ledger_root,
        context.session_id,
        &envelope.id,
    ))?;
    if context.require_authority && issued.is_none() {
        return Err(format!(
            "Task authority '{}' was not issued by this broker",
            envelope.id
        ));
    }
    if let Some(record) = issued.as_ref() {
        if !matches!(
            record.schema.as_str(),
            "agent-browser.task-authority-issuance.v1" | "agent-browser.task-authority-issuance.v2"
        ) || record.session_id != context.session_id
            || record.envelope.id != envelope.id
            || record.envelope_sha256 != hash
            || record.envelope != envelope
        {
            return Err("Task authority broker issuance does not match the envelope".to_string());
        }
        if context.require_authority && record.schema != "agent-browser.task-authority-issuance.v2"
        {
            return Err(format!(
                "Task authority '{}' uses legacy broker issuance; required mode accepts only ordered v2 authority",
                envelope.id
            ));
        }
        if record.revocation.is_some() {
            return Err(format!("Task authority '{}' was revoked", envelope.id));
        }
        if record.schema == "agent-browser.task-authority-issuance.v2" {
            let plan_sha256 = approved_plan_sha256(&record.approved_plan.steps)?;
            if record.approved_plan.steps.is_empty()
                || record.approved_plan.plan_sha256.as_deref() != Some(plan_sha256.as_str())
                || envelope.plan_sha256.as_deref() != Some(plan_sha256.as_str())
            {
                return Err("Task authority broker plan binding is invalid".to_string());
            }
        }
    }
    let path = ledger_path(&context.ledger_root, context.session_id, &envelope.id);
    let existing = read_ledger(&path)?;
    if let Some(ledger) = existing.as_ref() {
        if ledger.envelope_sha256 != hash {
            return Err(format!(
                "Task authority '{}' changed after first use",
                envelope.id
            ));
        }
        if ledger.session_id != context.session_id || ledger.authority_id != envelope.id {
            return Err("Task authority ledger identity mismatch".to_string());
        }
    } else if current_url != envelope.target_binding.initial_url {
        return Err(format!(
            "Task authority initial URL mismatch: expected '{}', got '{}'",
            envelope.target_binding.initial_url, current_url
        ));
    }

    let admitted_actions = existing
        .as_ref()
        .map(|ledger| ledger.admitted_actions)
        .unwrap_or(0);
    let reserved_evidence_bytes = existing
        .as_ref()
        .map(|ledger| ledger.reserved_evidence_bytes)
        .unwrap_or(0);
    let ordered_record = issued
        .as_ref()
        .filter(|record| record.schema == "agent-browser.task-authority-issuance.v2");
    if let (Some(record), Some(ledger)) = (ordered_record, existing.as_ref()) {
        validate_ordered_ledger(ledger, record)?;
    }
    let next_step_index = existing
        .as_ref()
        .map(|ledger| ledger.next_step_index)
        .unwrap_or(0);
    let planned_step = ordered_record
        .map(|record| {
            let supplied_step_id = cmd
                .get("taskStepId")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or("Broker-issued task authority requires top-level taskStepId")?;
            let step = record
                .approved_plan
                .steps
                .get(
                    usize::try_from(next_step_index)
                        .map_err(|_| "Task authority cursor overflow")?,
                )
                .ok_or_else(|| {
                    format!("Task authority '{}' ordered plan is exhausted", envelope.id)
                })?;
            if supplied_step_id != step.step_id {
                return Err(format!(
                    "Task authority expected step '{}' at index {}, got '{}'",
                    step.step_id, step.index, supplied_step_id
                ));
            }
            if step.action != action {
                return Err(format!(
                    "Task authority step '{}' permits action '{}', not '{}'",
                    step.step_id, step.action, action
                ));
            }
            let normalized_current_url = normalized_url(current_url)?;
            if normalized_current_url != step.expected_current_url {
                return Err(format!(
                    "Task authority step '{}' expected current URL '{}', got '{}'",
                    step.step_id, step.expected_current_url, normalized_current_url
                ));
            }
            let explicit_url = explicit_requested_url(cmd)
                .map(normalized_url)
                .transpose()?;
            if explicit_url != step.requested_url {
                return Err(format!(
                    "Task authority step '{}' requested URL mismatch: expected {:?}, got {:?}",
                    step.step_id, step.requested_url, explicit_url
                ));
            }
            let supplied = cmd
                .get("taskEvidenceBytes")
                .and_then(Value::as_u64)
                .ok_or("Broker-issued task authority requires positive taskEvidenceBytes")?;
            if supplied != step.evidence_bytes {
                return Err(format!(
                    "Task authority step '{}' evidence reservation must equal {} bytes",
                    step.step_id, step.evidence_bytes
                ));
            }
            let command_id = cmd
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or("Broker-issued task authority requires a top-level command id")?;
            if existing.as_ref().is_some_and(|ledger| {
                ledger.step_receipts.iter().any(|receipt| {
                    receipt.step_id == step.step_id || receipt.command_id == command_id
                })
            }) {
                return Err(format!(
                    "Task authority step '{}' or command '{}' was already admitted",
                    step.step_id, command_id
                ));
            }
            Ok::<_, String>((step, command_id.to_string()))
        })
        .transpose()?;
    let reservation = planned_step
        .as_ref()
        .map(|(step, _)| step.evidence_bytes)
        .unwrap_or_else(|| evidence_reservation(cmd, action));
    if reservation == 0 {
        return Err("taskEvidenceBytes must be positive".to_string());
    }
    if admitted_actions >= envelope.evidence_budget.max_actions {
        return Err(format!(
            "Task authority '{}' action budget exhausted",
            envelope.id
        ));
    }
    if reservation
        > envelope
            .evidence_budget
            .max_evidence_bytes
            .saturating_sub(reserved_evidence_bytes)
    {
        return Err(format!(
            "Task authority '{}' evidence budget exhausted",
            envelope.id
        ));
    }

    let admission = TaskAuthorityAdmission {
        authority_id: envelope.id.clone(),
        envelope_sha256: hash.clone(),
        consequence_ceiling: ceiling,
        consequence,
        admitted_actions: admitted_actions + 1,
        reserved_evidence_bytes: reserved_evidence_bytes + reservation,
        expires_at: envelope.expires_at.clone(),
        step_id: planned_step.as_ref().map(|(step, _)| step.step_id.clone()),
        step_index: planned_step.as_ref().map(|(step, _)| step.index),
    };
    if consequence.authority_rank() > ceiling.authority_rank()
        && context.confirmed_authority_id != Some(envelope.id.as_str())
    {
        return Ok(TaskAuthorityDecision::RequiresConfirmation(admission));
    }

    if reserve_budget {
        let admitted_at = Utc::now().to_rfc3339();
        let mut step_receipts = existing
            .as_ref()
            .map(|ledger| ledger.step_receipts.clone())
            .unwrap_or_default();
        if let Some((step, command_id)) = planned_step.as_ref() {
            step_receipts.push(TaskAuthorityStepReceipt {
                step_id: step.step_id.clone(),
                step_index: step.index,
                command_id: command_id.clone(),
                action: action.to_string(),
                target_id: target_id.to_string(),
                current_url: normalized_url(current_url)?,
                requested_url: step.requested_url.clone(),
                reserved_evidence_bytes: reservation,
                admitted_at: admitted_at.clone(),
                outcome: None,
            });
        }
        write_ledger(
            &path,
            &TaskAuthorityLedger {
                schema: if ordered_record.is_some() {
                    "agent-browser.task-authority-ledger.v2".to_string()
                } else {
                    "agent-browser.task-authority-ledger.v1".to_string()
                },
                authority_id: envelope.id,
                envelope_sha256: hash,
                session_id: context.session_id.to_string(),
                admitted_actions: admission.admitted_actions,
                reserved_evidence_bytes: admission.reserved_evidence_bytes,
                next_step_index: if ordered_record.is_some() {
                    next_step_index + 1
                } else {
                    0
                },
                step_receipts,
                updated_at: admitted_at,
            },
        )?;
    }
    Ok(TaskAuthorityDecision::Admitted(admission))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn authority(expires_at: DateTime<chrono::FixedOffset>) -> Value {
        json!({
            "id": "authority-1",
            "taskName": "research-task",
            "serviceName": "research-service",
            "agentName": "codex",
            "allowedOrigins": ["https://example.com", "https://example.org"],
            "targetBinding": {
                "targetId": "target-1",
                "initialUrl": "https://example.com/start"
            },
            "evidenceBudget": {
                "maxActions": 2,
                "maxEvidenceBytes": 8192
            },
            "consequenceCeiling": "read_only",
            "expiresAt": expires_at.to_rfc3339()
        })
    }

    fn command(authority: Value) -> Value {
        json!({
            "action": "title",
            "taskName": "research-task",
            "serviceName": "research-service",
            "agentName": "codex",
            "taskAuthority": authority,
            "taskEvidenceBytes": 4096
        })
    }

    fn context<'a>(root: PathBuf, url: &'a str) -> TaskAuthorityContext<'a> {
        TaskAuthorityContext {
            session_id: "session-1",
            target_id: Some("target-1"),
            url: Some(url),
            confirmed_authority_id: None,
            require_authority: false,
            ledger_root: root,
        }
    }

    #[test]
    fn durable_ledger_rejects_drift_and_exhausted_budget_after_fresh_context() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let expires = (Utc::now() + chrono::Duration::minutes(5)).fixed_offset();
        let raw = authority(expires);
        let cmd = command(raw.clone());
        let first = admit_task_authority(
            &cmd,
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.com/start"),
            true,
        )
        .unwrap();
        assert!(matches!(first, TaskAuthorityDecision::Admitted(_)));
        let second = admit_task_authority(
            &cmd,
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.org/next"),
            true,
        )
        .unwrap();
        assert!(matches!(second, TaskAuthorityDecision::Admitted(_)));
        let exhausted = admit_task_authority(
            &cmd,
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.org/next"),
            true,
        )
        .unwrap_err();
        assert!(exhausted.contains("action budget exhausted"));

        let mut changed = raw;
        changed["allowedOrigins"] = json!(["https://example.com"]);
        let drift = admit_task_authority(
            &command(changed),
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.com/start"),
            true,
        )
        .unwrap_err();
        assert!(drift.contains("changed after first use"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn target_origin_task_and_expiry_fail_closed() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let expires = (Utc::now() + chrono::Duration::minutes(5)).fixed_offset();
        let raw = authority(expires);
        let mut wrong_task = command(raw.clone());
        wrong_task["taskName"] = json!("other");
        assert!(admit_task_authority(
            &wrong_task,
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.com/start"),
            true,
        )
        .unwrap_err()
        .contains("taskName mismatch"));

        let mut wrong_target = context(root.clone(), "https://example.com/start");
        wrong_target.target_id = Some("other-target");
        assert!(admit_task_authority(
            &command(raw.clone()),
            "title",
            ActionConsequence::ReadOnly,
            &wrong_target,
            true,
        )
        .unwrap_err()
        .contains("target mismatch"));
        assert!(admit_task_authority(
            &command(raw),
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://not-allowed.example/"),
            true,
        )
        .unwrap_err()
        .contains("is not allowed"));

        let expired = authority((Utc::now() - chrono::Duration::seconds(1)).fixed_offset());
        assert!(admit_task_authority(
            &command(expired),
            "title",
            ActionConsequence::ReadOnly,
            &context(root.clone(), "https://example.com/start"),
            true,
        )
        .unwrap_err()
        .contains("expired"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn consequence_above_ceiling_requires_matching_confirmation() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let expires = (Utc::now() + chrono::Duration::minutes(5)).fixed_offset();
        let raw = authority(expires);
        let mut cmd = command(raw);
        cmd["action"] = json!("click");
        let decision = admit_task_authority(
            &cmd,
            "click",
            ActionConsequence::ExternalMutation,
            &context(root.clone(), "https://example.com/start"),
            false,
        )
        .unwrap();
        assert!(matches!(
            decision,
            TaskAuthorityDecision::RequiresConfirmation(_)
        ));

        let mut confirmed = context(root.clone(), "https://example.com/start");
        confirmed.confirmed_authority_id = Some("authority-1");
        let admitted = admit_task_authority(
            &cmd,
            "click",
            ActionConsequence::ExternalMutation,
            &confirmed,
            true,
        )
        .unwrap();
        assert!(matches!(admitted, TaskAuthorityDecision::Admitted(_)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn required_mode_rejects_missing_authority() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let mut required = context(root, "https://example.com/start");
        required.require_authority = true;
        let error = admit_task_authority(
            &json!({
                "action": "title",
                "taskName": "research-task"
            }),
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap_err();
        assert!(error.contains("required"));
    }

    fn issue_request(steps: Value) -> Value {
        json!({
            "taskName": "research-task",
            "serviceName": "research-service",
            "agentName": "codex",
            "expectedTargetId": "target-1",
            "expectedUrl": "https://example.com/start",
            "issuer": {
                "kind": "operator",
                "id": "operator-1"
            },
            "approvalReference": "approval-1",
            "expiresInSeconds": 300,
            "steps": steps
        })
    }

    #[test]
    fn broker_issue_derives_minimal_scope_and_revocation_blocks_admission() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([
                {"action": "title", "evidenceBytes": 1024},
                {"action": "navigate", "url": "https://example.org/next", "evidenceBytes": 2048}
            ])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        assert_eq!(issued["state"], "active");
        assert_eq!(
            issued["envelope"]["allowedActions"],
            json!(["navigate", "title"])
        );
        assert_eq!(
            issued["envelope"]["allowedOrigins"],
            json!(["https://example.com", "https://example.org"])
        );
        assert_eq!(issued["envelope"]["evidenceBudget"]["maxActions"], 2);
        assert_eq!(
            issued["envelope"]["evidenceBudget"]["maxEvidenceBytes"],
            3072
        );
        assert_eq!(issued["envelope"]["consequenceCeiling"], "read_only");

        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("command-1");
        cmd["taskStepId"] = issued["approvedPlan"]["steps"][0]["stepId"].clone();
        cmd["taskEvidenceBytes"] = json!(1024);
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        let admitted =
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true)
                .unwrap();
        assert!(matches!(admitted, TaskAuthorityDecision::Admitted(_)));

        let authority_id = issued["id"].as_str().unwrap();
        let status = task_authority_status(&root, "session-1", Some(authority_id)).unwrap();
        assert_eq!(status["usage"]["admittedActions"], 1);
        assert_eq!(status["usage"]["remainingActions"], 1);
        let revoked = revoke_task_authority(
            &root,
            "session-1",
            authority_id,
            "operator-1",
            "task complete",
            "target-1",
            "https://example.org/next",
        )
        .unwrap();
        assert_eq!(revoked["state"], "revoked");
        assert_eq!(
            revoked["revocation"]["targetUrl"],
            "https://example.org/next"
        );
        assert!(
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true,)
                .unwrap_err()
                .contains("revoked")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn broker_issue_rejects_target_drift_and_mutating_plan() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let target_error = issue_task_authority(
            &issue_request(json!([{"action": "title"}])),
            "session-1",
            "other-target",
            "https://example.com/start",
            &root,
        )
        .unwrap_err();
        assert!(target_error.contains("target changed"));
        let mutation_error = issue_task_authority(
            &issue_request(json!([{"action": "click"}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap_err();
        assert!(mutation_error.contains("does not yet permit"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn broker_issued_action_allowlist_rejects_unplanned_read() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title"}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        let error = admit_task_authority(
            &command(issued["envelope"].clone()),
            "snapshot",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap_err();
        assert!(error.contains("does not allow action 'snapshot'"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn required_mode_rejects_legacy_broker_issuance() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title", "evidenceBytes": 1024}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let authority_id = issued["id"].as_str().unwrap();
        let path = issuance_path(&root, "session-1", authority_id);
        let mut record = read_issuance(&path).unwrap().unwrap();
        record.schema = "agent-browser.task-authority-issuance.v1".to_string();
        write_private_json(&path, &record, "task authority issuance").unwrap();

        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("legacy-command");
        cmd["taskStepId"] = issued["approvedPlan"]["steps"][0]["stepId"].clone();
        cmd["taskEvidenceBytes"] = json!(1024);
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        assert!(
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true,)
                .unwrap_err()
                .contains("required mode accepts only ordered v2")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn broker_plan_cursor_enforces_order_multiplicity_receipts_and_restart() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([
                {"action": "title", "evidenceBytes": 1024},
                {"action": "navigate", "url": "https://example.org/next", "evidenceBytes": 2048},
                {"action": "title", "evidenceBytes": 1024}
            ])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let steps = issued["approvedPlan"]["steps"].as_array().unwrap();
        assert_ne!(steps[0]["stepId"], steps[2]["stepId"]);
        assert_eq!(steps[1]["expectedCurrentUrl"], "https://example.com/start");
        assert_eq!(steps[2]["expectedCurrentUrl"], "https://example.org/next");

        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        let mut missing_step = command(issued["envelope"].clone());
        missing_step["id"] = json!("ordered-command-missing-step");
        missing_step["taskEvidenceBytes"] = json!(1024);
        assert!(admit_task_authority(
            &missing_step,
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap_err()
        .contains("requires top-level taskStepId"));
        let mut first = command(issued["envelope"].clone());
        first["id"] = json!("ordered-command-1");
        first["taskStepId"] = steps[0]["stepId"].clone();
        first["taskEvidenceBytes"] = json!(1024);
        assert!(matches!(
            admit_task_authority(
                &first,
                "title",
                ActionConsequence::ReadOnly,
                &required,
                true
            )
            .unwrap(),
            TaskAuthorityDecision::Admitted(_)
        ));

        let mut repeated_title = command(issued["envelope"].clone());
        repeated_title["id"] = json!("ordered-command-2-wrong");
        repeated_title["taskStepId"] = steps[2]["stepId"].clone();
        repeated_title["taskEvidenceBytes"] = json!(1024);
        assert!(admit_task_authority(
            &repeated_title,
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap_err()
        .contains("expected step"));

        let replay = admit_task_authority(
            &first,
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap_err();
        assert!(replay.contains("expected step") || replay.contains("already admitted"));

        let mut wrong_evidence = command(issued["envelope"].clone());
        wrong_evidence["id"] = json!("ordered-command-2-wrong-evidence");
        wrong_evidence["action"] = json!("navigate");
        wrong_evidence["url"] = json!("https://example.org/next");
        wrong_evidence["taskStepId"] = steps[1]["stepId"].clone();
        wrong_evidence["taskEvidenceBytes"] = json!(1024);
        assert!(admit_task_authority(
            &wrong_evidence,
            "navigate",
            ActionConsequence::Navigation,
            &required,
            true,
        )
        .unwrap_err()
        .contains("evidence reservation must equal"));

        let mut wrong_url = wrong_evidence.clone();
        wrong_url["id"] = json!("ordered-command-2-wrong-url");
        wrong_url["url"] = json!("https://example.org/other");
        wrong_url["taskEvidenceBytes"] = json!(2048);
        assert!(admit_task_authority(
            &wrong_url,
            "navigate",
            ActionConsequence::Navigation,
            &required,
            true,
        )
        .unwrap_err()
        .contains("requested URL mismatch"));

        let mut navigation = command(issued["envelope"].clone());
        navigation["id"] = json!("ordered-command-2");
        navigation["action"] = json!("navigate");
        navigation["url"] = json!("https://example.org/next");
        navigation["taskStepId"] = steps[1]["stepId"].clone();
        navigation["taskEvidenceBytes"] = json!(2048);
        let mut confirmed = context(root.clone(), "https://example.com/start");
        confirmed.require_authority = true;
        confirmed.confirmed_authority_id = issued["id"].as_str();
        assert!(matches!(
            admit_task_authority(
                &navigation,
                "navigate",
                ActionConsequence::Navigation,
                &confirmed,
                true,
            )
            .unwrap(),
            TaskAuthorityDecision::Admitted(_)
        ));

        let mut final_read = command(issued["envelope"].clone());
        final_read["id"] = json!("ordered-command-3");
        final_read["taskStepId"] = steps[2]["stepId"].clone();
        final_read["taskEvidenceBytes"] = json!(1024);
        let mut after_navigation = context(root.clone(), "https://example.org/next");
        after_navigation.require_authority = true;
        assert!(matches!(
            admit_task_authority(
                &final_read,
                "title",
                ActionConsequence::ReadOnly,
                &after_navigation,
                true,
            )
            .unwrap(),
            TaskAuthorityDecision::Admitted(_)
        ));

        let status = task_authority_status(&root, "session-1", issued["id"].as_str()).unwrap();
        assert_eq!(status["state"], "exhausted");
        assert_eq!(status["usage"]["nextStepIndex"], 3);
        assert_eq!(status["usage"]["remainingSteps"], 0);
        assert_eq!(
            status["usage"]["indeterminateSteps"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            status["usage"]["indeterminateSteps"][0]["commandId"],
            "ordered-command-1"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ordered_step_terminal_outcomes_are_durable_idempotent_and_fail_closed() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([
                {"action": "title", "evidenceBytes": 1024},
                {"action": "title", "evidenceBytes": 1024}
            ])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let steps = issued["approvedPlan"]["steps"].as_array().unwrap();
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;

        let mut completed = command(issued["envelope"].clone());
        completed["id"] = json!("outcome-command-1");
        completed["taskStepId"] = steps[0]["stepId"].clone();
        completed["taskEvidenceBytes"] = json!(1024);
        admit_task_authority(
            &completed,
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap();
        let completed_response = json!({
            "id": "outcome-command-1",
            "success": true,
            "data": {"title": "Example"}
        });
        finalize_task_authority_step(&completed, &required, &completed_response).unwrap();
        finalize_task_authority_step(&completed, &required, &completed_response).unwrap();
        assert!(finalize_task_authority_step(
            &completed,
            &required,
            &json!({"id": "outcome-command-1", "success": false, "error": "changed"}),
        )
        .unwrap_err()
        .contains("conflicting terminal outcome"));

        let mut failed = command(issued["envelope"].clone());
        failed["id"] = json!("outcome-command-2");
        failed["taskStepId"] = steps[1]["stepId"].clone();
        failed["taskEvidenceBytes"] = json!(1024);
        admit_task_authority(
            &failed,
            "title",
            ActionConsequence::ReadOnly,
            &required,
            true,
        )
        .unwrap();
        finalize_task_authority_step(
            &failed,
            &required,
            &json!({"id": "outcome-command-2", "success": false, "error": "read failed"}),
        )
        .unwrap();

        let status = task_authority_status(&root, "session-1", issued["id"].as_str()).unwrap();
        assert_eq!(status["usage"]["outcomeSummary"]["completed"], 1);
        assert_eq!(status["usage"]["outcomeSummary"]["failed"], 1);
        assert_eq!(status["usage"]["outcomeSummary"]["indeterminate"], 0);
        assert_eq!(
            status["usage"]["completedSteps"][0]["outcome"]["state"],
            "completed"
        );
        assert_eq!(
            status["usage"]["failedSteps"][0]["outcome"]["state"],
            "failed"
        );
        assert_eq!(
            status["usage"]["completedSteps"][0]["outcome"]["targetId"],
            "target-1"
        );
        assert_eq!(
            status["usage"]["completedSteps"][0]["outcome"]["url"],
            "https://example.com/start"
        );
        assert_eq!(
            status["usage"]["completedSteps"][0]["outcome"]["responseSha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn admitted_without_terminal_outcome_is_indeterminate_and_never_replayable() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title", "evidenceBytes": 1024}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("crash-after-admission");
        cmd["taskStepId"] = issued["approvedPlan"]["steps"][0]["stepId"].clone();
        cmd["taskEvidenceBytes"] = json!(1024);
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true).unwrap();

        let restarted = context(root.clone(), "https://example.com/start");
        let status = task_authority_status(&root, "session-1", issued["id"].as_str()).unwrap();
        assert_eq!(status["usage"]["outcomeSummary"]["indeterminate"], 1);
        assert_eq!(
            status["usage"]["indeterminateSteps"][0]["outcome"]["reason"],
            "admitted_without_durable_terminal_outcome"
        );
        assert!(
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &restarted, true,)
                .unwrap_err()
                .contains("exhausted")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn reconcile_request(unresolved_step_id: &str) -> Value {
        json!({
            "reconciliationId": "reconcile-crash-1",
            "unresolvedStepId": unresolved_step_id,
            "taskName": "research-task",
            "serviceName": "research-service",
            "agentName": "codex",
            "expectedTargetId": "target-1",
            "expectedUrl": "https://example.com/start",
            "issuer": { "kind": "operator", "id": "operator-1" },
            "approvalReference": "approval-reconcile-1",
            "expiresInSeconds": 300,
            "steps": [
                {"action": "title", "evidenceBytes": 2048}
            ]
        })
    }

    #[test]
    fn indeterminate_reconciliation_resumes_after_revocation_and_binds_lineage() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title", "evidenceBytes": 1024}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let authority_id = issued["id"].as_str().unwrap();
        let step_id = issued["approvedPlan"]["steps"][0]["stepId"]
            .as_str()
            .unwrap();
        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("stranded-command-1");
        cmd["taskStepId"] = json!(step_id);
        cmd["taskEvidenceBytes"] = json!(1024);
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true).unwrap();

        let request = reconcile_request(step_id);
        assert!(reconcile_task_authority_inner(
            &root,
            "session-1",
            authority_id,
            &request,
            "target-1",
            "https://example.com/start",
            true,
        )
        .unwrap_err()
        .contains("Injected stop"));
        let pending = task_authority_status(&root, "session-1", Some(authority_id)).unwrap();
        assert_eq!(pending["state"], "revoked");
        assert_eq!(pending["revocation"]["reconciliation"]["state"], "pending");

        let reconciled = reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &request,
            "target-1",
            "https://example.com/start",
        )
        .unwrap();
        assert_eq!(reconciled["idempotent"], false);
        assert_eq!(
            reconciled["lineage"]["predecessorCommandId"],
            "stranded-command-1"
        );
        assert_eq!(
            reconciled["replacement"]["envelope"]["lineage"],
            reconciled["lineage"]
        );
        assert_eq!(
            reconciled["predecessor"]["revocation"]["reconciliation"]["state"],
            "completed"
        );
        let replacement_id = reconciled["replacement"]["id"].as_str().unwrap();
        assert_ne!(replacement_id, authority_id);
        assert_ne!(
            reconciled["replacement"]["approvedPlan"]["steps"][0]["stepId"],
            json!(step_id)
        );

        let retry = reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &request,
            "target-1",
            "https://example.com/start",
        )
        .unwrap();
        assert_eq!(retry["idempotent"], true);
        assert_eq!(retry["replacement"]["id"], replacement_id);
        assert!(
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true,)
                .unwrap_err()
                .contains("revoked")
        );

        let mut conflicting = request.clone();
        conflicting["approvalReference"] = json!("different-approval");
        assert!(reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &conflicting,
            "target-1",
            "https://example.com/start",
        )
        .unwrap_err()
        .contains("different evidence"));
        assert!(reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &request,
            "other-target",
            "https://example.com/start",
        )
        .unwrap_err()
        .contains("target changed"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reconciliation_rejects_ambiguous_predecessors() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([
                {"action": "title", "evidenceBytes": 1024},
                {"action": "title", "evidenceBytes": 1024}
            ])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let authority_id = issued["id"].as_str().unwrap();
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        for index in 0..2 {
            let mut cmd = command(issued["envelope"].clone());
            cmd["id"] = json!(format!("ambiguous-command-{index}"));
            cmd["taskStepId"] = issued["approvedPlan"]["steps"][index]["stepId"].clone();
            cmd["taskEvidenceBytes"] = json!(1024);
            admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true)
                .unwrap();
        }
        let step_id = issued["approvedPlan"]["steps"][0]["stepId"]
            .as_str()
            .unwrap();
        assert!(reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &reconcile_request(step_id),
            "target-1",
            "https://example.com/start",
        )
        .unwrap_err()
        .contains("exactly one indeterminate"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reconciliation_rejects_terminal_predecessors() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title", "evidenceBytes": 1024}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let authority_id = issued["id"].as_str().unwrap();
        let step_id = issued["approvedPlan"]["steps"][0]["stepId"]
            .as_str()
            .unwrap();
        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("terminal-command-1");
        cmd["taskStepId"] = json!(step_id);
        cmd["taskEvidenceBytes"] = json!(1024);
        let mut required = context(root.clone(), "https://example.com/start");
        required.require_authority = true;
        admit_task_authority(&cmd, "title", ActionConsequence::ReadOnly, &required, true).unwrap();
        finalize_task_authority_step(
            &cmd,
            &required,
            &json!({"id": "terminal-command-1", "success": true, "data": {"title": "Example"}}),
        )
        .unwrap();

        assert!(reconcile_task_authority(
            &root,
            "session-1",
            authority_id,
            &reconcile_request(step_id),
            "target-1",
            "https://example.com/start",
        )
        .unwrap_err()
        .contains("exactly one indeterminate"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn terminal_outcome_requires_the_durable_admission_receipt() {
        let root =
            std::env::temp_dir().join(format!("agent-browser-authority-{}", uuid::Uuid::new_v4()));
        let issued = issue_task_authority(
            &issue_request(json!([{"action": "title", "evidenceBytes": 1024}])),
            "session-1",
            "target-1",
            "https://example.com/start",
            &root,
        )
        .unwrap();
        let mut cmd = command(issued["envelope"].clone());
        cmd["id"] = json!("missing-admission-outcome");
        cmd["taskStepId"] = issued["approvedPlan"]["steps"][0]["stepId"].clone();
        cmd["taskEvidenceBytes"] = json!(1024);
        let required = context(root.clone(), "https://example.com/start");
        let error = finalize_task_authority_step(
            &cmd,
            &required,
            &json!({"id": "missing-admission-outcome", "success": true, "data": {}}),
        )
        .unwrap_err();
        assert!(error.contains("no durable admission ledger"));
        fs::remove_dir_all(root).unwrap();
    }

    fn confirmation_command(id: &str) -> Value {
        json!({
            "id": id,
            "action": "task_authority_issue",
            "taskName": "research-task",
            "request": {
                "taskName": "research-task",
                "expectedTargetId": "target-1",
                "expectedUrl": "https://example.com/",
                "issuer": {"kind": "operator", "id": "requester-1"},
                "approvalReference": "reviewed-plan",
                "expiresInSeconds": 300,
                "steps": [{"action": "title"}],
                "privateMarker": "must-not-appear-in-status"
            }
        })
    }

    fn migrated_confirmation_tombstone_test_ledger(
        count: usize,
    ) -> (PathBuf, TaskAuthorityConfirmationTombstoneManifest) {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-confirmation-tombstone-integrity-{}",
            uuid::Uuid::new_v4()
        ));
        let confirmation_ids: BTreeSet<String> = (0..count)
            .map(|index| format!("retired-{index:04}"))
            .collect();
        write_private_json(
            &confirmation_tombstones_path(&root, "session-1"),
            &LegacyTaskAuthorityConfirmationTombstones {
                schema: legacy_confirmation_tombstone_schema(),
                confirmation_ids,
                updated_at: Some(Utc::now().to_rfc3339()),
            },
            "legacy test confirmation tombstones",
        )
        .unwrap();
        let loaded = load_confirmation_tombstones(&root, "session-1").unwrap();
        let migrated = persist_confirmation_tombstones(
            &root,
            "session-1",
            loaded,
            std::iter::empty::<String>(),
        )
        .unwrap();
        (root, migrated.manifest)
    }

    #[test]
    fn durable_confirmation_survives_restart_and_is_single_use_after_decision_commit() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-confirmation-{}",
            uuid::Uuid::new_v4()
        ));
        let command = confirmation_command("confirmation-restart-1");
        let staged = stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
            root: &root,
            session_id: "session-1",
            confirmation_id: "confirmation-restart-1",
            action: "task_authority_issue",
            consequence_class: "control_plane",
            command: &command,
            target_id: "target-1",
            url: "https://example.com/",
            ttl_seconds: 60,
        })
        .unwrap();
        assert_eq!(staged.state, "pending");

        let restarted = load_task_authority_pending_confirmation(&root, "session-1")
            .unwrap()
            .unwrap();
        assert_eq!(restarted.command(), &command);
        let committed = decide_task_authority_confirmation(DecideTaskAuthorityConfirmation {
            root: &root,
            session_id: "session-1",
            confirmation_id: "confirmation-restart-1",
            expected_action: "task_authority_issue",
            decision: "confirm",
            decided_by: TaskAuthorityIssuer {
                kind: "operator".to_string(),
                id: "requester-1".to_string(),
            },
            target_id: Some("target-1"),
            url: Some("https://example.com/"),
        })
        .unwrap();
        assert_eq!(committed.execution_state, "dispatched");

        let after_crash = task_authority_confirmation_status(&root, "session-1").unwrap();
        assert_eq!(
            after_crash["confirmations"][0]["executionState"],
            "indeterminate"
        );
        assert!(!after_crash
            .to_string()
            .contains("must-not-appear-in-status"));
        assert!(load_task_authority_pending_confirmation(&root, "session-1")
            .unwrap()
            .is_none());
        assert!(
            decide_task_authority_confirmation(DecideTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "confirmation-restart-1",
                expected_action: "task_authority_issue",
                decision: "confirm",
                decided_by: TaskAuthorityIssuer {
                    kind: "operator".to_string(),
                    id: "requester-1".to_string(),
                },
                target_id: Some("target-1"),
                url: Some("https://example.com/"),
            })
            .unwrap_err()
            .contains("No pending")
        );

        finalize_task_authority_confirmation(
            &root,
            &committed,
            &json!({"success": true, "data": {"id": "authority-1"}}),
        )
        .unwrap();
        let completed = task_authority_confirmation_status(&root, "session-1").unwrap();
        assert_eq!(completed["confirmations"][0]["executionState"], "completed");
        assert_eq!(
            completed["confirmations"][0]["decidedBy"]["id"],
            "requester-1"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn durable_confirmation_mismatch_consumes_fail_closed() {
        for (label, confirmation_id, expected_action, actor_id, target_id, url) in [
            (
                "id",
                "wrong",
                "task_authority_issue",
                "requester-1",
                "target-1",
                "https://example.com/",
            ),
            (
                "action",
                "confirmation-1",
                "task_authority_revoke",
                "requester-1",
                "target-1",
                "https://example.com/",
            ),
            (
                "actor",
                "confirmation-1",
                "task_authority_issue",
                "different-operator",
                "target-1",
                "https://example.com/",
            ),
            (
                "target",
                "confirmation-1",
                "task_authority_issue",
                "requester-1",
                "target-2",
                "https://example.com/",
            ),
            (
                "url",
                "confirmation-1",
                "task_authority_issue",
                "requester-1",
                "target-1",
                "https://example.org/",
            ),
        ] {
            let root = std::env::temp_dir().join(format!(
                "agent-browser-confirmation-{label}-{}",
                uuid::Uuid::new_v4()
            ));
            let command = confirmation_command("confirmation-1");
            stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "confirmation-1",
                action: "task_authority_issue",
                consequence_class: "control_plane",
                command: &command,
                target_id: "target-1",
                url: "https://example.com/",
                ttl_seconds: 60,
            })
            .unwrap();
            assert!(
                decide_task_authority_confirmation(DecideTaskAuthorityConfirmation {
                    root: &root,
                    session_id: "session-1",
                    confirmation_id,
                    expected_action,
                    decision: "confirm",
                    decided_by: TaskAuthorityIssuer {
                        kind: "operator".to_string(),
                        id: actor_id.to_string(),
                    },
                    target_id: Some(target_id),
                    url: Some(url),
                })
                .unwrap_err()
                .contains("failed closed")
            );
            assert!(load_task_authority_pending_confirmation(&root, "session-1")
                .unwrap()
                .is_none());
            let status = task_authority_confirmation_status(&root, "session-1").unwrap();
            assert_eq!(status["confirmations"][0]["state"], "invalidated");
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn durable_confirmation_expiry_digest_and_session_checks_fail_closed() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-confirmation-{}",
            uuid::Uuid::new_v4()
        ));
        let expired_command = confirmation_command("confirmation-expired");
        stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
            root: &root,
            session_id: "session-1",
            confirmation_id: "confirmation-expired",
            action: "task_authority_issue",
            consequence_class: "control_plane",
            command: &expired_command,
            target_id: "target-1",
            url: "https://example.com/",
            ttl_seconds: 60,
        })
        .unwrap();
        assert!(load_task_authority_pending_confirmation(&root, "session-2")
            .unwrap()
            .is_none());
        let mut expired = load_task_authority_pending_confirmation(&root, "session-1")
            .unwrap()
            .unwrap();
        expired.expires_at = (Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
        write_private_json(
            &pending_confirmation_path(&root, "session-1"),
            &expired,
            "test expired confirmation",
        )
        .unwrap();
        let projected = task_authority_confirmation_status(&root, "session-1").unwrap();
        assert_eq!(projected["confirmations"][0]["state"], "expired");
        assert!(
            decide_task_authority_confirmation(DecideTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "confirmation-expired",
                expected_action: "task_authority_issue",
                decision: "confirm",
                decided_by: TaskAuthorityIssuer {
                    kind: "operator".to_string(),
                    id: "requester-1".to_string(),
                },
                target_id: Some("target-1"),
                url: Some("https://example.com/"),
            })
            .unwrap_err()
            .contains("expired")
        );

        let tampered_command = confirmation_command("confirmation-tampered");
        stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
            root: &root,
            session_id: "session-1",
            confirmation_id: "confirmation-tampered",
            action: "task_authority_issue",
            consequence_class: "control_plane",
            command: &tampered_command,
            target_id: "target-1",
            url: "https://example.com/",
            ttl_seconds: 60,
        })
        .unwrap();
        let mut tampered = load_task_authority_pending_confirmation(&root, "session-1")
            .unwrap()
            .unwrap();
        tampered.command["taskName"] = json!("changed-after-stage");
        write_private_json(
            &pending_confirmation_path(&root, "session-1"),
            &tampered,
            "test tampered confirmation",
        )
        .unwrap();
        assert!(load_task_authority_pending_confirmation(&root, "session-1")
            .unwrap_err()
            .contains("invalid"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn confirmation_cleanup_is_review_bound_and_preserves_pending_and_indeterminate() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-confirmation-cleanup-{}",
            uuid::Uuid::new_v4()
        ));
        let actor = || TaskAuthorityIssuer {
            kind: "operator".to_string(),
            id: "requester-1".to_string(),
        };
        for (id, decision, finalize) in [
            ("denied-1", "deny", false),
            ("completed-1", "confirm", true),
            ("indeterminate-1", "confirm", false),
        ] {
            let command = confirmation_command(id);
            stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: id,
                action: "task_authority_issue",
                consequence_class: "control_plane",
                command: &command,
                target_id: "target-1",
                url: "https://example.com/",
                ttl_seconds: 60,
            })
            .unwrap();
            let decided = decide_task_authority_confirmation(DecideTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: id,
                expected_action: "task_authority_issue",
                decision,
                decided_by: actor(),
                target_id: Some("target-1"),
                url: Some("https://example.com/"),
            })
            .unwrap();
            if finalize {
                finalize_task_authority_confirmation(&root, &decided, &json!({"success": true}))
                    .unwrap();
            }
        }
        let pending_command = confirmation_command("pending-1");
        stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
            root: &root,
            session_id: "session-1",
            confirmation_id: "pending-1",
            action: "task_authority_issue",
            consequence_class: "control_plane",
            command: &pending_command,
            target_id: "target-1",
            url: "https://example.com/",
            ttl_seconds: 60,
        })
        .unwrap();

        let preview = cleanup_task_authority_confirmations(CleanupTaskAuthorityConfirmations {
            root: &root,
            session_id: "session-1",
            retain_count: 0,
            min_age_seconds: 0,
            requested_by: actor(),
            apply: false,
            review_sha256: None,
        })
        .unwrap();
        assert_eq!(preview["candidateCount"], 2);
        assert_eq!(preview["pendingPreserved"], true);
        assert_eq!(preview["indeterminatePreservedCount"], 1);
        assert_eq!(preview["tombstoneLedger"]["integrityState"], "verified");
        assert_eq!(preview["tombstoneLedger"]["confirmationCount"], 0);
        assert_eq!(
            task_authority_confirmation_status(&root, "session-1").unwrap()["count"],
            4
        );

        assert!(
            cleanup_task_authority_confirmations(CleanupTaskAuthorityConfirmations {
                root: &root,
                session_id: "session-1",
                retain_count: 0,
                min_age_seconds: 0,
                requested_by: actor(),
                apply: true,
                review_sha256: Some(&"0".repeat(64)),
            })
            .unwrap_err()
            .contains("digest mismatch")
        );
        let review_sha256 = preview["reviewSha256"].as_str().unwrap();
        let applied = cleanup_task_authority_confirmations(CleanupTaskAuthorityConfirmations {
            root: &root,
            session_id: "session-1",
            retain_count: 0,
            min_age_seconds: 0,
            requested_by: actor(),
            apply: true,
            review_sha256: Some(review_sha256),
        })
        .unwrap();
        assert_eq!(applied["removedCount"], 2);
        assert_eq!(applied["tombstoneLedger"]["confirmationCount"], 2);
        assert_eq!(applied["tombstoneLedger"]["activeCount"], 2);
        let status = task_authority_confirmation_status(&root, "session-1").unwrap();
        assert_eq!(status["count"], 2);
        assert!(status["confirmations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["executionState"] == "indeterminate"));
        assert!(
            stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "completed-1",
                action: "task_authority_issue",
                consequence_class: "control_plane",
                command: &confirmation_command("completed-1"),
                target_id: "target-1",
                url: "https://example.com/",
                ttl_seconds: 60,
            })
            .unwrap_err()
            .contains("terminal receipt")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn confirmation_tombstone_segments_are_bounded_hash_linked_and_fail_closed() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-confirmation-tombstone-segments-{}",
            uuid::Uuid::new_v4()
        ));
        let legacy_ids: BTreeSet<String> = (0..300)
            .map(|index| format!("retired-{index:04}"))
            .collect();
        write_private_json(
            &confirmation_tombstones_path(&root, "session-1"),
            &LegacyTaskAuthorityConfirmationTombstones {
                schema: legacy_confirmation_tombstone_schema(),
                confirmation_ids: legacy_ids.clone(),
                updated_at: Some(Utc::now().to_rfc3339()),
            },
            "legacy test confirmation tombstones",
        )
        .unwrap();

        let legacy = load_confirmation_tombstones(&root, "session-1").unwrap();
        assert!(legacy.legacy);
        let migrated = persist_confirmation_tombstones(
            &root,
            "session-1",
            legacy,
            ["retired-new".to_string()],
        )
        .unwrap();
        assert!(!migrated.legacy);
        assert_eq!(migrated.manifest.segment_count, 1);
        assert_eq!(migrated.manifest.active_confirmation_ids.len(), 45);
        assert_eq!(migrated.confirmation_ids.len(), 301);
        let segment_path = confirmation_tombstone_segment_path(&root, "session-1", 0);
        let mut segment: TaskAuthorityConfirmationTombstoneSegment =
            serde_json::from_str(&fs::read_to_string(&segment_path).unwrap()).unwrap();
        assert_eq!(
            segment.confirmation_ids.len(),
            CONFIRMATION_TOMBSTONE_SEGMENT_CAPACITY
        );
        assert!(
            stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "retired-0000",
                action: "task_authority_issue",
                consequence_class: "control_plane",
                command: &confirmation_command("retired-0000"),
                target_id: "target-1",
                url: "https://example.com/",
                ttl_seconds: 60,
            })
            .unwrap_err()
            .contains("terminal receipt")
        );

        let manifest_path = confirmation_tombstones_path(&root, "session-1");
        let manifest: TaskAuthorityConfirmationTombstoneManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let mut active_tampered = manifest.clone();
        active_tampered
            .active_confirmation_ids
            .insert("active-tampered".to_string());
        write_private_json(
            &manifest_path,
            &active_tampered,
            "tampered test confirmation tombstone manifest",
        )
        .unwrap();
        assert!(load_confirmation_tombstones(&root, "session-1")
            .unwrap_err()
            .contains("ledger is invalid"));
        write_private_json(
            &manifest_path,
            &manifest,
            "restored test confirmation tombstone manifest",
        )
        .unwrap();

        segment.confirmation_ids[0] = "tampered".to_string();
        write_private_json(
            &segment_path,
            &segment,
            "tampered test confirmation tombstone segment",
        )
        .unwrap();
        assert!(load_confirmation_tombstones(&root, "session-1")
            .unwrap_err()
            .contains("head digest"));
        assert!(
            stage_task_authority_confirmation(StageTaskAuthorityConfirmation {
                root: &root,
                session_id: "session-1",
                confirmation_id: "fresh-after-tamper",
                action: "task_authority_issue",
                consequence_class: "control_plane",
                command: &confirmation_command("fresh-after-tamper"),
                target_id: "target-1",
                url: "https://example.com/",
                ttl_seconds: 60,
            })
            .unwrap_err()
            .contains("head digest")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn confirmation_tombstone_integrity_variants_fail_closed() {
        let (missing_root, _) = migrated_confirmation_tombstone_test_ledger(600);
        fs::remove_file(confirmation_tombstone_segment_path(
            &missing_root,
            "session-1",
            0,
        ))
        .unwrap();
        assert!(load_confirmation_tombstones(&missing_root, "session-1").is_err());
        fs::remove_dir_all(missing_root).unwrap();

        let (reordered_root, _) = migrated_confirmation_tombstone_test_ledger(600);
        let first_path = confirmation_tombstone_segment_path(&reordered_root, "session-1", 0);
        let second_path = confirmation_tombstone_segment_path(&reordered_root, "session-1", 1);
        let first: TaskAuthorityConfirmationTombstoneSegment =
            serde_json::from_str(&fs::read_to_string(&first_path).unwrap()).unwrap();
        let second: TaskAuthorityConfirmationTombstoneSegment =
            serde_json::from_str(&fs::read_to_string(&second_path).unwrap()).unwrap();
        write_private_json(&first_path, &second, "reordered tombstone segment").unwrap();
        write_private_json(&second_path, &first, "reordered tombstone segment").unwrap();
        assert!(load_confirmation_tombstones(&reordered_root, "session-1").is_err());
        fs::remove_dir_all(reordered_root).unwrap();

        let (duplicate_root, _) = migrated_confirmation_tombstone_test_ledger(600);
        let segment_path = confirmation_tombstone_segment_path(&duplicate_root, "session-1", 0);
        let mut segment: TaskAuthorityConfirmationTombstoneSegment =
            serde_json::from_str(&fs::read_to_string(&segment_path).unwrap()).unwrap();
        segment.confirmation_ids[1] = segment.confirmation_ids[0].clone();
        write_private_json(&segment_path, &segment, "duplicate tombstone segment").unwrap();
        assert!(load_confirmation_tombstones(&duplicate_root, "session-1")
            .unwrap_err()
            .contains("duplicate IDs"));
        fs::remove_dir_all(duplicate_root).unwrap();

        for (label, mutate) in [
            (
                "count",
                (|manifest: &mut TaskAuthorityConfirmationTombstoneManifest| {
                    manifest.confirmation_count += 1;
                }) as fn(&mut TaskAuthorityConfirmationTombstoneManifest),
            ),
            (
                "head",
                (|manifest: &mut TaskAuthorityConfirmationTombstoneManifest| {
                    manifest.head_sha256 = Some("0".repeat(64));
                }) as fn(&mut TaskAuthorityConfirmationTombstoneManifest),
            ),
        ] {
            let (root, mut manifest) = migrated_confirmation_tombstone_test_ledger(600);
            mutate(&mut manifest);
            write_private_json(
                &confirmation_tombstones_path(&root, "session-1"),
                &manifest,
                &format!("{label} mismatch tombstone manifest"),
            )
            .unwrap();
            assert!(load_confirmation_tombstones(&root, "session-1").is_err());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
