//! Fresh-chain custody bootstrap; reachable only by the cold operator entrypoint.
//!
//! A schema v5 `fresh_chain_bootstrap` receipt starts an independent custody
//! chain (new chainId, generation 1, no `source`). It is proven by a new exact
//! current attachment plus the current browser process, physical profile,
//! exclusive lease and exact handle. It does NOT continue the old lineage and
//! is NOT an authenticated detach ACK: the former owner is only observed
//! absent. The predecessor v4 receipt is embedded verbatim and digest-bound.
//!
//! No service_request/MCP action or install gate invokes this module.
//! `bootstrap` is attachment-only: CDP domains are NOT enabled and it is never
//! handed to ready-runtime consumers. `bootstrap_ready` additionally performs
//! protocol-only initialization (Page/Runtime/Network.enable plus an exact
//! session echo) BEFORE the single commit; that is not proof of a running or
//! rendered page. Its manager is extractable only from the committed token.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use super::browser::{BrowserManager, ProtocolReadyAttachment, StagedExactAttachment};
use super::runtime_attestation::{fresh_chain_binding, fresh_chain_physical_browser};
use super::runtime_handoff_v2::{current_process_identity, require_process_gone};
use super::service_model::ServiceState;
use super::service_store::ServiceStateRepository;
use super::tab_handle_refresh::{capture_exact_tab_refresh_fence, ExactTabRefreshFence};

pub(super) const SCHEMA_VERSION: u64 = 5;
pub(super) const KIND: &str = "fresh_chain_bootstrap";
pub(super) const OWNER_EXIT: &str = "observed_absent_unauthenticated";

/// Pinned physical profile the caller expects the current browser to own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalProfileExpectation {
    pub canonical_profile: PathBuf,
    pub profile_device: u64,
    pub profile_inode: u64,
}

#[derive(Clone)]
pub(super) struct BootstrapRequest {
    pub session_name: String,
    /// The exact current handle Map, validated by the existing diagnostics rules.
    pub handle: Map<String, Value>,
    pub expected_predecessor_sha256: String,
    pub profile: PhysicalProfileExpectation,
}

/// Facts admitted from persisted state and the live process table. Opaque;
/// recomputed under the repository lock and compared before commit.
#[derive(Debug, Clone, PartialEq)]
struct Admitted {
    profile_id: String,
    browser_id: String,
    target_id: String,
    pid: u32,
    endpoint: String,
    browser: Value,
    predecessor: Value,
    predecessor_digest: String,
    fence: ExactTabRefreshFence,
}

/// Committed receipt plus the attachment-only manager and its held public
/// lease. The guard is retained until the caller finishes publication.
pub(super) struct CommittedBootstrap {
    attachment: StagedExactAttachment,
    receipt: Value,
}

impl CommittedBootstrap {
    pub(super) fn receipt(&self) -> &Value {
        &self.receipt
    }

    pub(super) fn attachment(&self) -> &StagedExactAttachment {
        &self.attachment
    }
}

fn canonical_json(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                canonical_json(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical_json(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// SHA-256 over JSON with recursively sorted object keys.
pub(super) fn canonical_digest(value: &Value) -> String {
    let mut text = String::new();
    canonical_json(value, &mut text);
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("fresh_chain_{key}_missing"))
}

fn complete_identity(identity: Option<&Value>) -> Result<(), String> {
    let identity = identity
        .filter(|identity| identity.is_object())
        .ok_or("fresh_chain_identity_missing")?;
    for key in [
        "pid",
        "startTicks",
        "uid",
        "executableDevice",
        "executableInode",
    ] {
        if identity.get(key).and_then(Value::as_u64).is_none() {
            return Err("fresh_chain_identity_incomplete".into());
        }
    }
    if identity["pid"]
        .as_u64()
        .is_none_or(|pid| pid == 0 || pid > u32::MAX as u64)
        || identity["uid"]
            .as_u64()
            .is_none_or(|uid| uid > u32::MAX as u64)
        || identity["startTicks"].as_u64() == Some(0)
        || identity["executableInode"].as_u64() == Some(0)
        || uuid::Uuid::parse_str(text(identity, "bootId")?).is_err()
    {
        return Err("fresh_chain_identity_incomplete".into());
    }
    Ok(())
}

fn validate_predecessor(body: &Value) -> Result<(), String> {
    if body.get("schemaVersion").and_then(Value::as_u64) != Some(4)
        || body.get("phase").and_then(Value::as_str) != Some("committed")
        || body.get("kind").is_some()
        || body
            .get("ownerGeneration")
            .and_then(Value::as_u64)
            .is_none_or(|generation| generation == 0)
    {
        return Err("fresh_chain_predecessor_not_committed_v4".into());
    }
    complete_identity(body.get("source"))?;
    complete_identity(body.get("destination"))?;
    if body.get("source") == body.get("destination") {
        return Err("fresh_chain_predecessor_identity_reused".into());
    }
    for key in ["browserId", "profileId", "targetId", "cdpEndpoint"] {
        text(body, key)?;
    }
    complete_identity(
        body.get("browser")
            .and_then(|browser| browser.get("process")),
    )?;
    Ok(())
}

fn admit(state: &ServiceState, request: &BootstrapRequest) -> Result<Admitted, String> {
    let session = request.session_name.as_str();
    state.validate_diagnostics_handle(&request.handle, session)?;
    let fence = capture_exact_tab_refresh_fence(state, &request.handle, session)?;
    let handle = Value::Object(request.handle.clone());
    let target_id = text(&handle, "targetId")?.to_string();
    let profile_id = text(&handle, "profileId")?.to_string();
    let browser_id = text(&handle, "browserId")?.to_string();
    if text(&handle, "tabId")? != format!("target:{target_id}")
        || browser_id != format!("session:{session}")
    {
        return Err("fresh_chain_handle_identity_mismatch".into());
    }

    let predecessor = state
        .runtime_custody_receipts
        .get(session)
        .ok_or("fresh_chain_predecessor_missing")?
        .clone();
    let predecessor_digest = canonical_digest(&predecessor);
    if predecessor_digest != request.expected_predecessor_sha256 {
        return Err("fresh_chain_predecessor_digest_mismatch".into());
    }
    validate_predecessor(&predecessor)?;
    let former = &predecessor["destination"];
    if *former == current_process_identity(std::process::id())? {
        return Err("fresh_chain_predecessor_is_current_owner".into());
    }
    // Same PID and start ticks alive rejects; a reused PID means former gone.
    require_process_gone(former).map_err(|error| format!("fresh_chain_former_owner:{error}"))?;
    if predecessor.get("profileId").and_then(Value::as_str) != Some(profile_id.as_str())
        || predecessor.get("browserId").and_then(Value::as_str) != Some(browser_id.as_str())
    {
        return Err("fresh_chain_predecessor_binding_mismatch".into());
    }

    let record = state
        .browsers
        .get(&browser_id)
        .ok_or("fresh_chain_browser_missing")?;
    let pid = record.pid.ok_or("fresh_chain_browser_pid_missing")?;
    let endpoint = record
        .cdp_endpoint
        .clone()
        .ok_or("fresh_chain_endpoint_missing")?;
    fresh_chain_binding(
        state,
        session,
        &profile_id,
        &browser_id,
        &target_id,
        pid,
        &endpoint,
    )?;
    let configured = state
        .profiles
        .get(&profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .ok_or("fresh_chain_service_profile_missing")?;
    let canonical =
        fs::canonicalize(configured).map_err(|_| "fresh_chain_service_profile_unreadable")?;
    let metadata =
        fs::metadata(&canonical).map_err(|_| "fresh_chain_service_profile_unreadable")?;
    if canonical != request.profile.canonical_profile
        || metadata.dev() != request.profile.profile_device
        || metadata.ino() != request.profile.profile_inode
    {
        return Err("fresh_chain_physical_profile_mismatch".into());
    }
    let browser = json!({
        "process": current_process_identity(pid)?,
        "canonicalProfile": canonical,
        "profileDevice": metadata.dev(),
        "profileInode": metadata.ino(),
        "cdpEndpoint": endpoint,
    });
    fresh_chain_physical_browser(&browser, pid, &endpoint)?;
    if predecessor.get("browser") != Some(&browser)
        || predecessor.get("cdpEndpoint").and_then(Value::as_str) != Some(endpoint.as_str())
    {
        return Err("fresh_chain_physical_browser_changed".into());
    }
    Ok(Admitted {
        profile_id,
        browser_id,
        target_id,
        pid,
        endpoint,
        browser,
        predecessor,
        predecessor_digest,
        fence,
    })
}

fn fresh_receipt(admitted: &Admitted, destination: Value) -> Value {
    json!({
        "schemaVersion": SCHEMA_VERSION,
        "kind": KIND,
        "chainId": uuid::Uuid::new_v4().to_string(),
        "ownerGeneration": 1,
        "phase": "committed",
        "destination": destination,
        "browser": admitted.browser,
        "browserId": admitted.browser_id,
        "profileId": admitted.profile_id,
        "targetId": admitted.target_id,
        "cdpEndpoint": admitted.endpoint,
        "predecessor": {
            "digest": admitted.predecessor_digest,
            "body": admitted.predecessor,
            "ownerExit": OWNER_EXIT,
        },
    })
}

/// Structural, fail-closed v5 receipt check against the expected destination.
fn verify_receipt_shape(receipt: &Value, destination: &Value) -> Result<(), String> {
    let object = receipt.as_object().ok_or("fresh_chain_receipt_invalid")?;
    if receipt.get("schemaVersion").and_then(Value::as_u64) != Some(SCHEMA_VERSION)
        || receipt.get("kind").and_then(Value::as_str) != Some(KIND)
        || receipt.get("phase").and_then(Value::as_str) != Some("committed")
        || receipt.get("ownerGeneration").and_then(Value::as_u64) != Some(1)
        || object.contains_key("source")
        || object.contains_key("priorAnchor")
    {
        return Err("fresh_chain_receipt_invalid".into());
    }
    let chain_id = uuid::Uuid::parse_str(text(receipt, "chainId")?)
        .map_err(|_| "fresh_chain_chain_id_invalid")?;
    if chain_id.get_version_num() != 4 {
        return Err("fresh_chain_chain_id_invalid".into());
    }
    complete_identity(receipt.get("destination"))?;
    if receipt.get("destination") != Some(destination) {
        return Err("fresh_chain_destination_mismatch".into());
    }
    for key in ["browserId", "profileId", "targetId", "cdpEndpoint"] {
        text(receipt, key)?;
    }
    let predecessor = receipt
        .get("predecessor")
        .and_then(Value::as_object)
        .ok_or("fresh_chain_predecessor_missing")?;
    if predecessor.len() != 3
        || predecessor.get("ownerExit").and_then(Value::as_str) != Some(OWNER_EXIT)
    {
        return Err("fresh_chain_predecessor_invalid".into());
    }
    let body = predecessor
        .get("body")
        .ok_or("fresh_chain_predecessor_missing")?;
    if predecessor.get("digest").and_then(Value::as_str) != Some(canonical_digest(body).as_str()) {
        return Err("fresh_chain_predecessor_digest_mismatch".into());
    }
    validate_predecessor(body)?;
    if body.get("destination") == receipt.get("destination") {
        return Err("fresh_chain_predecessor_is_current_owner".into());
    }
    for key in ["browserId", "profileId", "cdpEndpoint", "browser"] {
        if body.get(key) != receipt.get(key) {
            return Err("fresh_chain_predecessor_binding_mismatch".into());
        }
    }
    Ok(())
}

/// Stage locally, then revalidate every admission fact under the repository
/// lock and replace the receipt Value once. Pre-commit rejection never requests
/// a save. Failed/uncertain persistence publishes no result even if storage may
/// have changed; no rollback or retry is inferred. Any failure drops only the
/// connection. The repository is injected; no default live path is used.
pub(super) async fn bootstrap<R: ServiceStateRepository>(
    repository: &R,
    request: BootstrapRequest,
) -> Result<CommittedBootstrap, String> {
    let admitted = admit(&repository.load_snapshot()?, &request)?;
    let attachment =
        BrowserManager::attach_exact_unowned(&admitted.endpoint, &admitted.target_id).await?;
    if attachment.target_id() != admitted.target_id || attachment.endpoint() != admitted.endpoint {
        return Err("fresh_chain_attachment_mismatch".into());
    }
    let (receipt, _) = commit_receipt(repository, &request, &admitted)?;
    Ok(CommittedBootstrap {
        attachment,
        receipt,
    })
}

/// Bound for each protocol-ready CDP exchange.
pub(super) const READY_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Protocol-ready variant: identical admission and locked commit, but the
/// exact attachment is protocol-initialized on the same connection and
/// session BEFORE the commit. Initialization failure requests zero saves.
/// After a successful save the receipt is read back; a mismatch or read
/// failure is reported as uncertain and publishes nothing (no rollback or
/// retry). Strict reuse of the existing Exclusive lease only.
pub(super) async fn bootstrap_ready<R: ServiceStateRepository>(
    repository: &R,
    request: BootstrapRequest,
) -> Result<CommittedReadyBootstrap, String> {
    bootstrap_ready_with_timeout(repository, request, READY_COMMAND_TIMEOUT).await
}

async fn bootstrap_ready_with_timeout<R: ServiceStateRepository>(
    repository: &R,
    request: BootstrapRequest,
    ready_timeout: std::time::Duration,
) -> Result<CommittedReadyBootstrap, String> {
    bootstrap_ready_observed(repository, request, ready_timeout, &mut false).await
}

/// The operator latches uncertainty before the first possible durable write.
/// Conservative true means never retry or retire the daemon on an error.
pub(super) async fn bootstrap_ready_observed<R: ServiceStateRepository>(
    repository: &R,
    request: BootstrapRequest,
    ready_timeout: std::time::Duration,
    commit_started: &mut bool,
) -> Result<CommittedReadyBootstrap, String> {
    let admitted = admit(&repository.load_snapshot()?, &request)?;
    let staged =
        BrowserManager::attach_exact_unowned(&admitted.endpoint, &admitted.target_id).await?;
    if staged.target_id() != admitted.target_id || staged.endpoint() != admitted.endpoint {
        return Err("fresh_chain_attachment_mismatch".into());
    }
    let session_id = staged.session_id().to_string();
    let mut ready = staged.initialize_protocol_ready(ready_timeout).await?;
    if ready.target_id() != admitted.target_id
        || ready.endpoint() != admitted.endpoint
        || ready.session_id() != session_id
    {
        return Err("fresh_chain_attachment_mismatch".into());
    }
    ready.ensure_not_paused()?;
    *commit_started = true;
    let (receipt, committed_fence) = commit_receipt(repository, &request, &admitted)?;
    let persisted = repository
        .load_snapshot()
        .map_err(|error| format!("fresh_chain_commit_readback_uncertain:{error}"))?;
    if persisted
        .runtime_custody_receipts
        .get(&request.session_name)
        != Some(&receipt)
    {
        return Err("fresh_chain_commit_readback_uncertain".into());
    }
    // Receipt equality alone cannot establish current custody: a lease, tab,
    // process or physical binding may have changed after persistence. Any
    // disagreement is post-commit uncertainty, never rollback or retry.
    let readback_fence =
        capture_exact_tab_refresh_fence(&persisted, &request.handle, &request.session_name)
            .map_err(|error| format!("fresh_chain_commit_readback_uncertain:{error}"))?;
    if readback_fence != committed_fence {
        return Err("fresh_chain_commit_readback_uncertain".into());
    }
    diagnostics(
        &persisted,
        &request.session_name,
        &receipt,
        &admitted.profile_id,
        &admitted.browser_id,
        &admitted.target_id,
    )
    .map_err(|error| format!("fresh_chain_commit_readback_uncertain:{error}"))?;
    ready
        .ensure_not_paused()
        .map_err(|error| format!("fresh_chain_commit_publication_uncertain:{error}"))?;
    Ok(CommittedReadyBootstrap { ready, receipt })
}

/// Committed receipt plus the protocol-ready attachment and held public lease.
/// Not Clone. Constructed only after a verified commit; consumed only by
/// `BrowserManager::adopt_committed_ready`.
pub(super) struct CommittedReadyBootstrap {
    ready: ProtocolReadyAttachment,
    receipt: Value,
}

impl CommittedReadyBootstrap {
    pub(super) fn receipt(&self) -> &Value {
        &self.receipt
    }

    pub(super) fn attachment(&self) -> &ProtocolReadyAttachment {
        &self.ready
    }

    /// Unpacks a COMMITTED token; the manager stays sealed inside
    /// `ProtocolReadyAttachment`, which only the adoption can open.
    pub(super) fn into_committed_parts(self) -> (ProtocolReadyAttachment, Value) {
        (self.ready, self.receipt)
    }
}

/// Builds the fresh receipt, revalidates every admission fact and the
/// destination under the repository lock, and inserts the receipt once.
fn commit_receipt<R: ServiceStateRepository>(
    repository: &R,
    request: &BootstrapRequest,
    admitted: &Admitted,
) -> Result<(Value, ExactTabRefreshFence), String> {
    let destination = current_process_identity(std::process::id())?;
    let receipt = fresh_receipt(admitted, destination.clone());
    verify_receipt_shape(&receipt, &destination)?;
    let committed_fence = repository.mutate(|state| {
        if current_process_identity(std::process::id())? != destination {
            return Err("fresh_chain_destination_changed".into());
        }
        if &admit(state, request)? != admitted {
            return Err("fresh_chain_admission_changed".into());
        }
        state
            .runtime_custody_receipts
            .insert(request.session_name.clone(), receipt.clone());
        capture_exact_tab_refresh_fence(state, &request.handle, &request.session_name)
    })?;
    Ok((receipt, committed_fence))
}

/// Diagnostics for a v5 receipt. Positivity is the independent current
/// binding and physical proof, never old-chain continuity or an ACK.
pub(super) fn diagnostics(
    state: &ServiceState,
    session_name: &str,
    receipt: &Value,
    profile_id: &str,
    browser_id: &str,
    target: &str,
) -> Result<Value, String> {
    let record = state
        .browsers
        .get(browser_id)
        .ok_or("attestation_browser_missing")?;
    let pid = record.pid.ok_or("attestation_browser_pid_missing")?;
    let endpoint = record
        .cdp_endpoint
        .as_deref()
        .ok_or("attestation_endpoint_missing")?;
    fresh_chain_binding(
        state,
        session_name,
        profile_id,
        browser_id,
        target,
        pid,
        endpoint,
    )?;
    verify_receipt_shape(receipt, &current_process_identity(std::process::id())?)?;
    require_process_gone(&receipt["predecessor"]["body"]["destination"])
        .map_err(|error| format!("fresh_chain_former_owner:{error}"))?;
    if receipt.get("profileId").and_then(Value::as_str) != Some(profile_id)
        || receipt.get("browserId").and_then(Value::as_str) != Some(browser_id)
        || receipt.get("targetId").and_then(Value::as_str) != Some(target)
        || receipt.get("cdpEndpoint").and_then(Value::as_str) != Some(endpoint)
    {
        return Err("fresh_chain_target_or_endpoint_mismatch".into());
    }
    let browser = &receipt["browser"];
    fresh_chain_physical_browser(browser, pid, endpoint)?;
    let configured = state
        .profiles
        .get(profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .ok_or("attestation_service_profile_missing")?;
    let physical = text(browser, "canonicalProfile")?;
    if fs::canonicalize(configured).map_err(|_| "attestation_service_profile_unreadable")?
        != fs::canonicalize(physical).map_err(|_| "attestation_browser_profile_unreadable")?
    {
        return Err("attestation_service_profile_mismatch".into());
    }
    Ok(json!({
        "complete": true,
        "missingProofs": [],
        "ownerCustody": {"verified": true, "basis": "fresh_chain_exact_attach", "ownerGeneration": 1,
            "chainId": receipt["chainId"],
            "predecessor": {"digest": receipt["predecessor"]["digest"], "ownerExit": OWNER_EXIT}},
        "profileLease": {"verified": true, "leaseState": "exclusive"},
        "browserProcess": {"verified": true, "pid": pid, "startTicks": browser["process"]["startTicks"]},
        "tab": {"verified": true, "targetId": target},
    }))
}

#[cfg(test)]
mod tests {
    use super::super::browser::{exact_attach_wire, pinned_loopback_browser_ws};
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::os::unix::fs::DirBuilderExt;
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    const ENDPOINT: &str = "ws://127.0.0.1:9222/devtools/browser/x";

    // Shell children only emulate process/profile identity. They never start Chrome.
    struct ChildGuard(Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    struct ProfileGuard(PathBuf);
    impl Drop for ProfileGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct PhysicalFixture {
        browser: ChildGuard,
        _profile: ProfileGuard,
        state: ServiceState,
        request: BootstrapRequest,
    }
    fn waiting_child(path: &std::path::Path) -> ChildGuard {
        ChildGuard(
            Command::new("/bin/sh")
                .args(["-c", "read -r fixture", "fixture"])
                .arg(format!("--user-data-dir={}", path.display()))
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    fn physical_fixture(endpoint: &str) -> PhysicalFixture {
        use super::super::service_model::{
            BrowserHealth, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, LeaseState,
            ProfileOrigin, TabLifecycle,
        };
        let path = std::env::temp_dir().join(format!("ab-fresh-chain-{}", uuid::Uuid::new_v4()));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let profile = ProfileGuard(fs::canonicalize(path).unwrap());
        let browser = waiting_child(&profile.0);
        let pid = browser.0.id();
        let mut old_owner = waiting_child(&profile.0);
        let old_identity = current_process_identity(old_owner.0.id()).unwrap();
        old_owner.0.kill().unwrap();
        old_owner.0.wait().unwrap();
        let parsed = url::Url::parse(endpoint).unwrap();
        fs::write(
            profile.0.join("DevToolsActivePort"),
            format!("{}\n{}\n", parsed.port().unwrap(), parsed.path()),
        )
        .unwrap();
        let hostname = fs::read_to_string("/proc/sys/kernel/hostname").unwrap();
        std::os::unix::fs::symlink(
            format!("{}-{pid}", hostname.trim()),
            profile.0.join("SingletonLock"),
        )
        .unwrap();
        let metadata = fs::metadata(&profile.0).unwrap();
        let mut state = ServiceState::default();
        state.profiles.insert(
            "p".into(),
            BrowserProfile {
                id: "p".into(),
                profile_origin: ProfileOrigin::ExternalByop,
                user_data_dir: Some(profile.0.to_string_lossy().into()),
                ..Default::default()
            },
        );
        state.browsers.insert(
            "session:s".into(),
            BrowserProcess {
                id: "session:s".into(),
                profile_id: Some("p".into()),
                pid: Some(pid),
                health: BrowserHealth::Ready,
                cdp_endpoint: Some(endpoint.into()),
                active_session_ids: vec!["s".into()],
                ..Default::default()
            },
        );
        state.sessions.insert(
            "s".into(),
            BrowserSession {
                id: "s".into(),
                profile_id: Some("p".into()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["session:s".into()],
                tab_ids: vec!["target:t1".into(), "target:peer".into()],
                ..Default::default()
            },
        );
        for target in ["t1", "peer"] {
            state.tabs.insert(
                format!("target:{target}"),
                BrowserTab {
                    id: format!("target:{target}"),
                    browser_id: "session:s".into(),
                    target_id: Some(target.into()),
                    owner_session_id: Some("s".into()),
                    lifecycle: TabLifecycle::Ready,
                    url: Some(format!("https://example.invalid/{target}")),
                    ..Default::default()
                },
            );
        }
        let proof = json!({"process": current_process_identity(pid).unwrap(),
            "canonicalProfile": profile.0, "profileDevice": metadata.dev(),
            "profileInode": metadata.ino(), "cdpEndpoint": endpoint});
        let mut previous = predecessor();
        previous["destination"] = old_identity;
        previous["browser"] = proof;
        previous["cdpEndpoint"] = json!(endpoint);
        state
            .runtime_custody_receipts
            .insert("s".into(), previous.clone());
        state.refresh_service_tab_handles();
        let request = BootstrapRequest {
            session_name: "s".into(),
            handle: serde_json::to_value(state.service_tab_handle("target:t1").unwrap())
                .unwrap()
                .as_object()
                .unwrap()
                .clone(),
            expected_predecessor_sha256: canonical_digest(&previous),
            profile: PhysicalProfileExpectation {
                canonical_profile: profile.0.clone(),
                profile_device: metadata.dev(),
                profile_inode: metadata.ino(),
            },
        };
        PhysicalFixture {
            browser,
            _profile: profile,
            state,
            request,
        }
    }

    struct FakeBrowser {
        endpoint: String,
        commands: Arc<Mutex<Vec<Value>>>,
        server: Option<tokio::task::JoinHandle<()>>,
    }
    impl Drop for FakeBrowser {
        fn drop(&mut self) {
            if let Some(server) = self.server.as_ref() {
                server.abort();
            }
        }
    }
    /// Replies for one command: messages with `method` are sent verbatim as
    /// events; others get the command id/sessionId. Empty means no reply.
    type Responder = Box<dyn FnMut(&Value) -> Vec<Value> + Send>;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Ready {
        Ok,
        AttachPause,
        AttachEventPause,
        DomainError,
        DomainPause,
        LateEventPause,
        WrongEcho,
        Silent,
    }

    const READY_METHODS: [&str; 7] = [
        "Target.getTargets",
        "Target.attachToTarget",
        "Target.getTargetInfo",
        "Page.enable",
        "Runtime.enable",
        "Network.enable",
        "Target.getTargetInfo",
    ];

    impl FakeBrowser {
        async fn new(echo_target: &'static str) -> Self {
            Self::serve(Box::new(move |command: &Value| {
                let result = match command["method"].as_str().unwrap() {
                    "Target.getTargets" => json!({"targetInfos": [
                        {"targetId":"peer", "type":"page"}, {"targetId":"t1", "type":"page"}]}),
                    "Target.attachToTarget" => json!({"sessionId":"exact-session"}),
                    "Target.getTargetInfo" => {
                        json!({"targetInfo":{"targetId":echo_target, "type":"page"}})
                    }
                    other => panic!("unexpected command: {other}"),
                };
                vec![json!({ "result": result })]
            }))
            .await
        }

        /// Ready-mode fixture: the exact 7-command allowlist; anything else panics.
        async fn ready(mode: Ready) -> Self {
            let mut echoes = 0;
            Self::serve(Box::new(move |command: &Value| {
                let mut replies = Vec::new();
                let result = match command["method"].as_str().unwrap() {
                    "Target.getTargets" => json!({"targetInfos": [
                        {"targetId":"peer", "type":"page"}, {"targetId":"t1", "type":"page"}]}),
                    "Target.attachToTarget" => {
                        if mode == Ready::AttachEventPause {
                            replies.push(json!({"method": "Target.attachedToTarget", "params": {
                                "sessionId": "exact-session", "waitingForDebugger": true,
                                "targetInfo": {"targetId": "t1", "type": "page"}}}));
                        }
                        let paused = mode == Ready::AttachPause;
                        json!({"sessionId": "exact-session", "waitingForDebugger": paused})
                    }
                    "Target.getTargetInfo" => {
                        echoes += 1;
                        let target = if mode == Ready::WrongEcho && echoes == 2 {
                            "peer"
                        } else {
                            "t1"
                        };
                        json!({"targetInfo": {"targetId": target, "type": "page"}})
                    }
                    "Page.enable" if mode == Ready::DomainError => {
                        return vec![json!({"error": {"code": -32000, "message": "synthetic"}})];
                    }
                    "Runtime.enable" if mode == Ready::DomainPause => {
                        json!({"waitingForDebugger": true})
                    }
                    "Runtime.enable" if mode == Ready::LateEventPause => {
                        replies.push(json!({"method": "Target.attachedToTarget", "params": {
                            "sessionId": "exact-session", "waitingForDebugger": true,
                            "targetInfo": {"targetId": "t1", "type": "page"}}}));
                        json!({})
                    }
                    "Network.enable" if mode == Ready::Silent => return replies,
                    "Page.enable" | "Runtime.enable" | "Network.enable" => json!({}),
                    other => panic!("unexpected command: {other}"),
                };
                replies.push(json!({ "result": result }));
                replies
            }))
            .await
        }

        async fn serve(mut responder: Responder) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("ws://{}/devtools/browser/x", listener.local_addr().unwrap());
            let commands = Arc::new(Mutex::new(Vec::new()));
            let observed = Arc::clone(&commands);
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
                'messages: while let Some(Ok(message)) = websocket.next().await {
                    let Message::Text(text) = message else {
                        continue;
                    };
                    let command: Value = serde_json::from_str(&text).unwrap();
                    observed.lock().unwrap().push(command.clone());
                    for mut response in responder(&command) {
                        if response.get("method").is_none() {
                            response["id"] = command["id"].clone();
                            if let Some(session) = command.get("sessionId") {
                                response["sessionId"] = session.clone();
                            }
                        }
                        if websocket
                            .send(Message::Text(response.to_string()))
                            .await
                            .is_err()
                        {
                            break 'messages;
                        }
                    }
                }
            });
            Self {
                endpoint,
                commands,
                server: Some(server),
            }
        }
        async fn finish(mut self) -> Vec<Value> {
            let mut server = self.server.take().unwrap();
            if tokio::time::timeout(std::time::Duration::from_secs(5), &mut server)
                .await
                .is_err()
            {
                server.abort();
                let _ = server.await;
                panic!("staged connection did not close");
            }
            self.commands.lock().unwrap().clone()
        }
    }

    struct TestRepository {
        persisted: RefCell<ServiceState>,
        change_before_commit: Option<fn(&mut ServiceState)>,
        fail_save: bool,
        fail_after_save: bool,
        change_after_save: Option<fn(&mut ServiceState)>,
        fail_readback: bool,
        saves: Cell<usize>,
    }
    impl TestRepository {
        fn new(state: ServiceState) -> Self {
            Self {
                persisted: RefCell::new(state),
                change_before_commit: None,
                fail_save: false,
                fail_after_save: false,
                change_after_save: None,
                fail_readback: false,
                saves: Cell::new(0),
            }
        }
    }
    impl ServiceStateRepository for TestRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            if self.fail_readback && self.saves.get() > 0 {
                return Err("synthetic_readback_unavailable".into());
            }
            Ok(self.persisted.borrow().clone())
        }
        fn mutate<R>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            if let Some(change) = self.change_before_commit {
                change(&mut self.persisted.borrow_mut());
            }
            let mut candidate = self.persisted.borrow().clone();
            // The real constructor must hold the required public guard through persistence.
            let endpoint = candidate.browsers["session:s"]
                .cdp_endpoint
                .as_deref()
                .unwrap();
            let gate = crate::native::privacy_gate::PrivacyGate::for_endpoint(endpoint).unwrap();
            assert!(gate.begin_private().is_err());
            let result = mutator(&mut candidate)?;
            self.saves.set(self.saves.get() + 1);
            if self.fail_save {
                return Err("synthetic_save_failed_or_uncertain".into());
            }
            *self.persisted.borrow_mut() = candidate;
            if let Some(change) = self.change_after_save {
                change(&mut self.persisted.borrow_mut());
            }
            if self.fail_after_save {
                return Err("synthetic_commit_uncertain_after_write".into());
            }
            Ok(result)
        }
    }

    impl ServiceStateRepository for &TestRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            TestRepository::load_snapshot(self)
        }
        fn mutate<R>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            TestRepository::mutate(self, mutator)
        }
    }

    fn operator_command(request: &BootstrapRequest) -> Value {
        json!({"id":"bootstrap-fixture","action":super::super::custody_bootstrap_request::ACTION,
            "request":{"sessionName":request.session_name,"serviceTabHandle":request.handle,
                "expectedPredecessorSha256":request.expected_predecessor_sha256,
                "physicalProfile":{"canonicalProfile":request.profile.canonical_profile,
                    "profileDevice":request.profile.profile_device,"profileInode":request.profile.profile_inode}}})
    }

    fn operator_state() -> super::super::actions::DaemonState {
        let mut state = super::super::actions::DaemonState::new();
        state.session_id = "s".into();
        state.session_name = None;
        state.custody_bootstrap_status = super::super::actions::CustodyBootstrapStatus::Pending;
        state
    }

    #[tokio::test]
    async fn custody_bootstrap_operator_adopts_once_without_stream_or_state_rewrite() {
        use super::super::actions::{
            handle_runtime_custody_bootstrap_using, CustodyBootstrapStatus,
        };
        let server = FakeBrowser::ready(Ready::Ok).await;
        let fixture = physical_fixture(&server.endpoint);
        let before = serde_json::to_value(&fixture.state).unwrap();
        let repository = TestRepository::new(fixture.state.clone());
        let command = operator_command(&fixture.request);
        let mut state = operator_state();
        let response =
            handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                .await
                .unwrap();
        assert_eq!(response["attached"], true);
        assert_eq!(response["targetId"], "t1");
        assert_eq!(
            state.custody_bootstrap_status,
            CustodyBootstrapStatus::Ready
        );
        assert!(state.stream_client.is_none() && state.stream_server.is_none());
        assert!(state.session_name.is_none() && !state.auto_dialog);
        state.update_stream_client().await; // Must remain inert.
        let gate = super::super::privacy_gate::PrivacyGate::for_endpoint(&server.endpoint).unwrap();
        assert!(gate.begin_private().is_err());
        assert!(
            handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                .await
                .is_err()
        );
        assert_eq!(repository.saves.get(), 1);
        let after = repository.load_snapshot().unwrap();
        let mut expected = before;
        expected["runtimeCustodyReceipts"]["s"] = after.runtime_custody_receipts["s"].clone();
        assert_eq!(serde_json::to_value(after).unwrap(), expected);
        drop(state);
        assert_eq!(methods(&server.finish().await), READY_METHODS);
        assert!(current_process_identity(fixture.browser.0.id()).is_ok());
    }

    #[tokio::test]
    async fn custody_bootstrap_operator_uncertainty_retains_latch_without_adoption_or_retry() {
        use super::super::actions::{
            handle_runtime_custody_bootstrap_using, CustodyBootstrapStatus,
        };
        for fail_after_save in [false, true] {
            let server = FakeBrowser::ready(Ready::Ok).await;
            let fixture = physical_fixture(&server.endpoint);
            let mut repository = TestRepository::new(fixture.state.clone());
            repository.fail_save = !fail_after_save;
            repository.fail_after_save = fail_after_save;
            let command = operator_command(&fixture.request);
            let mut state = operator_state();
            assert_eq!(
                handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                    .await,
                Err("fresh_chain_commit_uncertain".into())
            );
            assert_eq!(
                state.custody_bootstrap_status,
                CustodyBootstrapStatus::Uncertain
            );
            assert!(state.browser.is_none());
            assert!(
                handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                    .await
                    .is_err()
            );
            assert_eq!(repository.saves.get(), 1);
            assert_eq!(methods(&server.finish().await), READY_METHODS);
            assert!(current_process_identity(fixture.browser.0.id()).is_ok());
        }
    }

    #[tokio::test]
    async fn custody_bootstrap_operator_precommit_rejection_never_touches_repository() {
        use super::super::actions::{
            handle_runtime_custody_bootstrap_using, CustodyBootstrapStatus,
        };
        let fixture = physical_fixture(ENDPOINT);
        for mode in ["foreign", "released", "extra", "policy", "backend"] {
            let repository = TestRepository::new(fixture.state.clone());
            let mut command = operator_command(&fixture.request);
            let mut state = operator_state();
            match mode {
                "foreign" => command["request"]["sessionName"] = json!("other"),
                "released" => {
                    command["request"]["serviceTabHandle"]["leaseState"] = json!("released")
                }
                "extra" => command["autoConnect"] = json!(true),
                "policy" => state.require_task_authority = true,
                "backend" => state.backend_type = super::super::actions::BackendType::WebDriver,
                _ => unreachable!(),
            }
            assert!(
                handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                    .await
                    .is_err()
            );
            assert_eq!(
                state.custody_bootstrap_status,
                CustodyBootstrapStatus::PrecommitFailed
            );
            assert!(state.browser.is_none());
            assert_eq!(repository.saves.get(), 0);
            assert!(
                handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
                    .await
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn custody_bootstrap_occupied_manager_is_preserved_on_refusal() {
        use super::super::actions::{
            handle_runtime_custody_bootstrap_using, CustodyBootstrapStatus,
        };
        let server = FakeBrowser::ready(Ready::Ok).await;
        let fixture = physical_fixture(&server.endpoint);
        let repository = TestRepository::new(fixture.state.clone());
        let command = operator_command(&fixture.request);
        let mut state = operator_state();
        handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository))
            .await
            .unwrap();
        state.custody_bootstrap_status = CustodyBootstrapStatus::Pending;
        assert_eq!(
            handle_runtime_custody_bootstrap_using(&command, &mut state, || Ok(&repository)).await,
            Err("fresh_chain_daemon_occupied".into())
        );
        assert!(state.browser.is_some());
        assert_eq!(
            state.custody_bootstrap_status,
            CustodyBootstrapStatus::Uncertain
        );
        assert_eq!(repository.saves.get(), 1);
        drop(state);
        assert_eq!(methods(&server.finish().await), READY_METHODS);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn custody_bootstrap_cold_cli_daemon_preserves_exact_browser_and_no_stream() {
        use super::super::service_store::{JsonServiceStateStore, ServiceStateStore};
        // This process-level fixture is deliberately unavailable outside isolated QA.
        if std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref() != Ok("1") {
            return;
        }
        let binary = std::env::var("AGENT_BROWSER_FIXTURE_BIN").unwrap();
        assert!(
            std::path::Path::new(&binary).is_file(),
            "build candidate before this test"
        );
        let path = super::super::service_store::default_service_state_path().unwrap();
        let previous = fs::read(&path).ok();
        for ready_mode in [Ready::Ok, Ready::WrongEcho] {
            let server = FakeBrowser::ready(ready_mode).await;
            let fixture = physical_fixture(&server.endpoint);
            let identity = current_process_identity(fixture.browser.0.id()).unwrap();
            let store = JsonServiceStateStore::new(&path);
            store.save(&fixture.state).unwrap();
            let before = store.load().unwrap();
            let root = ProfileGuard(
                std::env::temp_dir().join(format!("ab-bootstrap-cli-{}", uuid::Uuid::new_v4())),
            );
            fs::create_dir(&root.0).unwrap();
            let socket_dir = root.0.join("sockets");
            fs::create_dir(&socket_dir).unwrap();
            let request_path = root.0.join("request.json");
            fs::write(
                &request_path,
                operator_command(&fixture.request)["request"].to_string(),
            )
            .unwrap();
            let output = tokio::process::Command::new(&binary)
                .args([
                    "--json",
                    "--session",
                    "s",
                    "handoff",
                    "bootstrap",
                    "--request-file",
                ])
                .arg(&request_path)
                .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
                .env("AGENT_BROWSER_CDP", "not-an-endpoint")
                .env("AGENT_BROWSER_SESSION_NAME", "must-not-autosave")
                .env("AGENT_BROWSER_AUTO_CONNECT", "1")
                .env("AGENT_BROWSER_STREAM_PORT", "1")
                .env(
                    "AGENT_BROWSER_PRIVATE_EXECUTOR_ROOT",
                    root.0.join("must-not-exist"),
                )
                .env("AGENT_BROWSER_STATE_EXPIRE_DAYS", "1")
                .env("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS", "1")
                .env("AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS", "1")
                .output()
                .await
                .unwrap();
            let result: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(result["success"], ready_mode == Ready::Ok, "{result}");
            assert!(!socket_dir.join("s.stream").exists());
            let after = store.load().unwrap();
            if ready_mode == Ready::Ok {
                assert_eq!(result["data"]["browserPid"], fixture.browser.0.id());
                let mut expected = serde_json::to_value(&before).unwrap();
                expected["runtimeCustodyReceipts"]["s"] =
                    after.runtime_custody_receipts["s"].clone();
                assert_eq!(serde_json::to_value(&after).unwrap(), expected);
                assert_eq!(
                    after.runtime_custody_receipts["s"]["predecessor"]["body"],
                    before.runtime_custody_receipts["s"]
                );
                // A second CLI attempt must refuse occupied metadata, not migrate/retry.
                let denied = tokio::process::Command::new(&binary)
                    .args([
                        "--json",
                        "--session",
                        "s",
                        "handoff",
                        "bootstrap",
                        "--request-file",
                    ])
                    .arg(&request_path)
                    .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
                    .output()
                    .await
                    .unwrap();
                let denied: Value = serde_json::from_slice(&denied.stdout).unwrap();
                assert_eq!(denied["success"], false);
                assert_eq!(denied["error"], "fresh_chain_cold_admission_denied");
                // Retire only our fake-browser daemon using its existing authenticated socket.
                let guard = crate::test_utils::EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR"]);
                guard.set("AGENT_BROWSER_SOCKET_DIR", socket_dir.to_str().unwrap());
                let closed = crate::connection::send_command_once(
                    &json!({"id":"fixture-close","action":"close"}),
                    "s",
                )
                .unwrap();
                assert!(closed.success);
            } else {
                assert_eq!(
                    serde_json::to_value(&after).unwrap(),
                    serde_json::to_value(&before).unwrap()
                );
            }
            for _ in 0..100 {
                if !socket_dir.join("s.sock").exists() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            assert!(
                !socket_dir.join("s.sock").exists(),
                "owned fixture daemon must retire"
            );
            assert_eq!(methods(&server.finish().await), READY_METHODS);
            assert_eq!(
                current_process_identity(fixture.browser.0.id()).unwrap(),
                identity
            );
        }
        match previous {
            Some(bytes) => fs::write(path, bytes).unwrap(),
            None => fs::remove_file(path).unwrap(),
        }
    }

    #[tokio::test]
    async fn custody_bootstrap_worker_fences_pending_publication_and_duplicate_refusal() {
        use super::super::control_plane::ControlPlaneWorker;
        use super::super::service_store::{JsonServiceStateStore, ServiceStateStore};
        if std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref() != Ok("1") {
            return;
        }
        let path = super::super::service_store::default_service_state_path().unwrap();
        let previous = fs::read(&path).ok();
        for delivered in [false, true] {
            let server = FakeBrowser::ready(Ready::Ok).await;
            let fixture = physical_fixture(&server.endpoint);
            let store = JsonServiceStateStore::new(&path);
            store.save(&fixture.state).unwrap();
            let worker =
                ControlPlaneWorker::start_with_options(operator_state(), Some(1), Some(1), Some(1));
            let command = operator_command(&fixture.request);
            let response = worker.custody_bootstrap(command.clone()).await;
            assert_eq!(response["success"], true);
            let committed = fs::read(&path).unwrap();
            assert!(worker.custody_bootstrap_rejects_ordinary());
            assert_eq!(
                worker
                    .submit(json!({"id":"blocked","action":"launch"}))
                    .await["success"],
                false
            );
            let rejected = worker.custody_bootstrap(command.clone()).await;
            assert_eq!(rejected["error"], "fresh_chain_bootstrap_attempt_denied");
            assert!(worker.custody_bootstrap_rejects_ordinary());
            assert_eq!(fs::read(&path).unwrap(), committed);
            if delivered {
                worker.custody_bootstrap_delivery_succeeded().await;
                assert!(!worker.custody_bootstrap_rejects_ordinary());
                assert_eq!(worker.custody_bootstrap(command).await["success"], false);
                assert!(!worker.custody_bootstrap_rejects_ordinary());
            } else {
                worker.custody_bootstrap_delivery_failed().await;
                assert!(worker.custody_bootstrap_rejects_ordinary());
                worker.custody_bootstrap_delivery_succeeded().await;
                assert!(worker.custody_bootstrap_rejects_ordinary());
            }
            assert_eq!(fs::read(&path).unwrap(), committed);
            worker.shutdown().await;
            assert_eq!(methods(&server.finish().await), READY_METHODS);
            assert!(current_process_identity(fixture.browser.0.id()).is_ok());
        }
        match previous {
            Some(bytes) => fs::write(path, bytes).unwrap(),
            None => fs::remove_file(path).unwrap(),
        }
    }

    #[tokio::test]
    async fn bootstrap_commits_once_only_after_exact_attach_and_preserves_browser_and_peers() {
        let server = FakeBrowser::new("t1").await;
        let fixture = physical_fixture(&server.endpoint);
        let identity_before = current_process_identity(fixture.browser.0.id()).unwrap();
        let before = serde_json::to_value(&fixture.state).unwrap();
        let repository = TestRepository::new(fixture.state.clone());
        let committed = bootstrap(&repository, fixture.request.clone())
            .await
            .unwrap();
        assert_eq!(repository.saves.get(), 1);
        assert_eq!(committed.attachment().target_id(), "t1");
        assert_eq!(committed.attachment().session_id(), "exact-session");
        let after = repository.load_snapshot().unwrap();
        assert_eq!(committed.receipt(), &after.runtime_custody_receipts["s"]);
        assert_eq!(
            committed.receipt()["predecessor"]["body"],
            before["runtimeCustodyReceipts"]["s"]
        );
        let mut expected = before;
        expected["runtimeCustodyReceipts"]["s"] = committed.receipt().clone();
        assert_eq!(serde_json::to_value(&after).unwrap(), expected);
        assert_eq!(
            current_process_identity(fixture.browser.0.id()).unwrap(),
            identity_before
        );
        let proof = super::super::runtime_attestation::diagnostics(
            Some(&after),
            &fixture.request.handle,
            "s",
            Some("t1"),
        );
        assert_eq!(proof["complete"], true);
        assert_eq!(proof["ownerCustody"]["basis"], "fresh_chain_exact_attach");
        assert_eq!(
            proof["ownerCustody"]["predecessor"]["ownerExit"],
            OWNER_EXIT
        );
        drop(committed);
        let commands = server.finish().await;
        assert_eq!(
            methods(&commands),
            [
                "Target.getTargets",
                "Target.attachToTarget",
                "Target.getTargetInfo"
            ]
        );
        assert_eq!(commands[2]["sessionId"], "exact-session");
        assert!(commands[2].get("params").is_none());
        assert_eq!(
            current_process_identity(fixture.browser.0.id()).unwrap(),
            identity_before
        );
        assert!(admit(&after, &fixture.request).is_err()); // Superseded v4 cannot be replayed.
    }

    #[tokio::test]
    async fn bootstrap_save_failure_returns_no_publication_and_keeps_old_bytes() {
        let server = FakeBrowser::new("t1").await;
        let fixture = physical_fixture(&server.endpoint);
        let mut repository = TestRepository::new(fixture.state.clone());
        repository.fail_save = true;
        let bytes = serde_json::to_vec(&fixture.state).unwrap();
        assert!(matches!(bootstrap(&repository, fixture.request).await,
            Err(reason) if reason == "synthetic_save_failed_or_uncertain"));
        assert_eq!(repository.saves.get(), 1);
        assert_eq!(
            serde_json::to_vec(&repository.load_snapshot().unwrap()).unwrap(),
            bytes
        );
        server.finish().await;
        assert!(current_process_identity(fixture.browser.0.id()).is_ok());
    }

    #[tokio::test]
    async fn bootstrap_uncertain_save_after_write_never_publishes_or_retries() {
        let server = FakeBrowser::new("t1").await;
        let fixture = physical_fixture(&server.endpoint);
        let mut repository = TestRepository::new(fixture.state.clone());
        repository.fail_after_save = true;
        assert!(matches!(bootstrap(&repository, fixture.request).await,
            Err(reason) if reason == "synthetic_commit_uncertain_after_write"));
        assert_eq!(repository.saves.get(), 1);
        let after = repository.load_snapshot().unwrap();
        assert_eq!(after.runtime_custody_receipts["s"]["schemaVersion"], 5);
        assert_eq!(
            after.runtime_custody_receipts["s"]["predecessor"]["body"],
            fixture.state.runtime_custody_receipts["s"]
        );
        server.finish().await;
        assert!(current_process_identity(fixture.browser.0.id()).is_ok());
    }

    #[tokio::test]
    async fn bootstrap_real_locked_json_repository_preserves_all_other_bytes() {
        use super::super::service_store::{JsonServiceStateStore, LockedServiceStateRepository};
        let server = FakeBrowser::new("t1").await;
        let fixture = physical_fixture(&server.endpoint);
        let dir = super::super::tab_handle_refresh::tests::TestDirectory::new();
        let path = dir.path().join("state.json");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                *state = fixture.state.clone();
                Ok(())
            })
            .unwrap();
        let mut expected: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let old_body = expected["runtimeCustodyReceipts"]["s"].clone();
        let committed = bootstrap(&repository, fixture.request).await.unwrap();
        expected["runtimeCustodyReceipts"]["s"] = committed.receipt().clone();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, expected);
        assert_eq!(
            saved["runtimeCustodyReceipts"]["s"]["predecessor"]["body"],
            old_body
        );
        drop(committed);
        server.finish().await;
        assert!(current_process_identity(fixture.browser.0.id()).is_ok());
    }

    #[tokio::test]
    async fn bootstrap_revalidates_all_authority_before_commit() {
        use super::super::service_model::LeaseState;
        let changes: [fn(&mut ServiceState); 4] = [
            |state| state.sessions.get_mut("s").unwrap().lease = LeaseState::Released,
            |state| {
                state.runtime_custody_receipts.get_mut("s").unwrap()["ownerGeneration"] = json!(7)
            },
            |state| {
                state
                    .tabs
                    .get_mut("target:t1")
                    .unwrap()
                    .observation_revision = Some("concurrent".into())
            },
            |state| {
                state.sessions.get_mut("s").unwrap().created_at =
                    Some("different-generation".into())
            },
        ];
        for change in changes {
            let server = FakeBrowser::new("t1").await;
            let fixture = physical_fixture(&server.endpoint);
            let mut repository = TestRepository::new(fixture.state.clone());
            repository.change_before_commit = Some(change);
            assert!(bootstrap(&repository, fixture.request).await.is_err());
            assert_eq!(repository.saves.get(), 0);
            assert_eq!(
                repository.load_snapshot().unwrap().runtime_custody_receipts["s"]["schemaVersion"],
                4
            );
            server.finish().await;
            assert!(current_process_identity(fixture.browser.0.id()).is_ok());
        }
    }

    #[tokio::test]
    async fn bootstrap_session_substitution_never_mutates() {
        let server = FakeBrowser::new("peer").await;
        let fixture = physical_fixture(&server.endpoint);
        let repository = TestRepository::new(fixture.state.clone());
        assert!(bootstrap(&repository, fixture.request).await.is_err());
        assert_eq!(repository.saves.get(), 0);
        assert_eq!(
            serde_json::to_value(repository.load_snapshot().unwrap()).unwrap(),
            serde_json::to_value(fixture.state).unwrap()
        );
        server.finish().await;
    }

    #[tokio::test]
    async fn ready_bootstrap_initializes_before_one_commit_and_adopts_unowned() {
        let server = FakeBrowser::ready(Ready::Ok).await;
        let fixture = physical_fixture(&server.endpoint);
        let before = serde_json::to_value(&fixture.state).unwrap();
        let repository = TestRepository::new(fixture.state.clone());
        let committed = bootstrap_ready(&repository, fixture.request.clone())
            .await
            .unwrap();
        assert_eq!(repository.saves.get(), 1);
        assert_eq!(committed.attachment().target_id(), "t1");
        assert_eq!(committed.attachment().session_id(), "exact-session");
        let after = repository.load_snapshot().unwrap();
        assert_eq!(committed.receipt(), &after.runtime_custody_receipts["s"]);
        assert_eq!(
            committed.receipt()["predecessor"]["body"],
            before["runtimeCustodyReceipts"]["s"]
        );
        let mut expected = before;
        expected["runtimeCustodyReceipts"]["s"] = committed.receipt().clone();
        assert_eq!(serde_json::to_value(&after).unwrap(), expected);
        let (manager, lease, receipt) = BrowserManager::adopt_committed_ready(committed).unwrap();
        assert!(manager.is_cdp_connection()); // Unowned: no Chrome lifecycle.
        assert_eq!(receipt, after.runtime_custody_receipts["s"]);
        let gate =
            crate::native::privacy_gate::PrivacyGate::for_endpoint(&server.endpoint).unwrap();
        assert!(gate.begin_private().is_err()); // Guard survives adoption.
        drop(manager);
        drop(lease);
        let commands = server.finish().await;
        assert_eq!(methods(&commands), READY_METHODS);
        for command in &commands[2..] {
            assert_eq!(command["sessionId"], "exact-session");
            assert!(command.get("params").is_none());
        }
        assert!(current_process_identity(fixture.browser.0.id()).is_ok());
    }

    #[tokio::test]
    async fn ready_initialization_failures_never_save_or_replace_receipt() {
        for (mode, sent) in [
            (Ready::AttachPause, 3),
            (Ready::AttachEventPause, 3),
            (Ready::DomainError, 4),
            (Ready::DomainPause, 5),
            (Ready::LateEventPause, 5),
            (Ready::Silent, 6),
            (Ready::WrongEcho, 7),
        ] {
            let server = FakeBrowser::ready(mode).await;
            let fixture = physical_fixture(&server.endpoint);
            let repository = TestRepository::new(fixture.state.clone());
            let started = std::time::Instant::now();
            let result = bootstrap_ready_with_timeout(
                &repository,
                fixture.request.clone(),
                std::time::Duration::from_millis(300),
            )
            .await;
            assert!(result.is_err(), "{mode:?}");
            assert!(started.elapsed() < std::time::Duration::from_secs(5));
            assert_eq!(repository.saves.get(), 0, "{mode:?}");
            assert_eq!(
                canonical_digest(
                    &repository.load_snapshot().unwrap().runtime_custody_receipts["s"]
                ),
                fixture.request.expected_predecessor_sha256
            );
            drop(result);
            let commands = server.finish().await;
            assert_eq!(methods(&commands), READY_METHODS[..sent], "{mode:?}");
            assert!(current_process_identity(fixture.browser.0.id()).is_ok());
        }
    }

    #[tokio::test]
    async fn ready_commit_rejection_or_uncertainty_publishes_nothing() {
        use super::super::service_model::LeaseState;
        let release: fn(&mut ServiceState) =
            |state| state.sessions.get_mut("s").unwrap().lease = LeaseState::Released;
        for case in 0..3 {
            let server = FakeBrowser::ready(Ready::Ok).await;
            let fixture = physical_fixture(&server.endpoint);
            let mut repository = TestRepository::new(fixture.state.clone());
            match case {
                0 => repository.change_before_commit = Some(release),
                1 => repository.fail_save = true,
                _ => repository.fail_after_save = true,
            }
            assert!(bootstrap_ready(&repository, fixture.request).await.is_err());
            assert_eq!(repository.saves.get(), usize::from(case > 0));
            let schema =
                &repository.load_snapshot().unwrap().runtime_custody_receipts["s"]["schemaVersion"];
            assert_eq!(schema, &json!(if case == 2 { 5 } else { 4 }));
            assert_eq!(methods(&server.finish().await), READY_METHODS);
        }
    }

    #[tokio::test]
    async fn ready_readback_disagreement_or_unavailability_publishes_nothing() {
        use super::super::service_model::LeaseState;
        let changes: [Option<fn(&mut ServiceState)>; 4] = [
            Some(|state| {
                state.runtime_custody_receipts.get_mut("s").unwrap()["chainId"] =
                    json!(uuid::Uuid::new_v4().to_string());
            }),
            Some(|state| state.sessions.get_mut("s").unwrap().lease = LeaseState::Released),
            Some(|state| {
                state
                    .tabs
                    .get_mut("target:t1")
                    .unwrap()
                    .observation_revision = Some("synthetic-concurrent-observation".into());
            }),
            None,
        ];
        for change in changes {
            let server = FakeBrowser::ready(Ready::Ok).await;
            let fixture = physical_fixture(&server.endpoint);
            let mut repository = TestRepository::new(fixture.state.clone());
            repository.change_after_save = change;
            repository.fail_readback = change.is_none();
            let result = bootstrap_ready(&repository, fixture.request.clone()).await;
            let Err(error) = result else {
                panic!("uncertain readback published a manager");
            };
            assert!(error.starts_with("fresh_chain_commit_readback_uncertain"));
            assert_eq!(repository.saves.get(), 1);
            // The save happened: no rollback or unchanged-receipt claim.
            let persisted = repository.persisted.borrow();
            assert_eq!(persisted.runtime_custody_receipts["s"]["schemaVersion"], 5);
            assert_eq!(
                canonical_digest(&persisted.runtime_custody_receipts["s"]["predecessor"]["body"]),
                fixture.request.expected_predecessor_sha256
            );
            drop(persisted);
            assert_eq!(methods(&server.finish().await), READY_METHODS);
            assert!(current_process_identity(fixture.browser.0.id()).is_ok());
        }
    }

    #[test]
    fn admission_rejects_live_owner_conflicts_physical_changes_and_invalid_handles() {
        use super::super::service_model::{BrowserSession, LeaseState};
        let fixture = physical_fixture(ENDPOINT);
        admit(&fixture.state, &fixture.request).unwrap();
        let mut released = fixture.request.clone();
        released
            .handle
            .insert("leaseState".into(), json!("released"));
        assert!(admit(&fixture.state, &released).is_err());
        for key in ["sessionName", "leaseId", "ownerSessionId"] {
            let mut foreign = fixture.request.clone();
            foreign.handle.insert(key.into(), json!("foreign"));
            assert!(admit(&fixture.state, &foreign).is_err());
        }
        let mut conflict = fixture.state.clone();
        conflict.sessions.insert(
            "other".into(),
            BrowserSession {
                id: "other".into(),
                profile_id: Some("p".into()),
                lease: LeaseState::Shared,
                ..Default::default()
            },
        );
        assert!(admit(&conflict, &fixture.request).is_err());
        let mut live = fixture.state.clone();
        live.runtime_custody_receipts.get_mut("s").unwrap()["destination"] =
            current_process_identity(fixture.browser.0.id()).unwrap();
        let mut request = fixture.request.clone();
        request.expected_predecessor_sha256 = canonical_digest(&live.runtime_custody_receipts["s"]);
        assert!(admit(&live, &request)
            .unwrap_err()
            .contains("source_still_alive"));
        // PID reuse is evidence that the former instance is gone, not an ACK.
        live.runtime_custody_receipts.get_mut("s").unwrap()["destination"]["startTicks"] = json!(
            live.runtime_custody_receipts["s"]["destination"]["startTicks"]
                .as_u64()
                .unwrap()
                + 1
        );
        request.expected_predecessor_sha256 = canonical_digest(&live.runtime_custody_receipts["s"]);
        admit(&live, &request).unwrap();
        request.profile.profile_inode += 1;
        assert!(admit(&live, &request).is_err());
        fs::remove_file(fixture._profile.0.join("SingletonLock")).unwrap();
        assert!(admit(&fixture.state, &fixture.request).is_err());
    }

    #[test]
    fn admission_rejects_missing_malformed_or_changed_browser_receipts() {
        let fixture = physical_fixture(ENDPOINT);
        let mut missing = fixture.state.clone();
        missing.runtime_custody_receipts.clear();
        assert!(admit(&missing, &fixture.request).is_err());
        for (pointer, replacement) in [
            ("/source/pid", json!(0)),
            ("/destination/pid", json!(u64::MAX)),
            ("/destination/startTicks", json!(0)),
            ("/destination/bootId", json!("malformed")),
            ("/browser/process/startTicks", json!(u64::MAX)),
            ("/browser/process/executableInode", json!(u64::MAX)),
            ("/browser/profileInode", json!(u64::MAX)),
        ] {
            let mut state = fixture.state.clone();
            let receipt = state.runtime_custody_receipts.get_mut("s").unwrap();
            *receipt.pointer_mut(pointer).unwrap() = replacement;
            let mut request = fixture.request.clone();
            request.expected_predecessor_sha256 = canonical_digest(receipt);
            assert!(admit(&state, &request).is_err(), "{pointer}");
        }
        fs::write(
            fixture._profile.0.join("DevToolsActivePort"),
            "1\n/devtools/browser/other\n",
        )
        .unwrap();
        assert!(admit(&fixture.state, &fixture.request).is_err());
    }

    #[tokio::test]
    async fn fresh_diagnostics_fail_closed_on_bad_history_destination_and_physical_state() {
        let server = FakeBrowser::new("t1").await;
        let fixture = physical_fixture(&server.endpoint);
        let repository = TestRepository::new(fixture.state.clone());
        let committed = bootstrap(&repository, fixture.request.clone())
            .await
            .unwrap();
        let current = repository.load_snapshot().unwrap();
        for change in [
            ("/kind", json!("handoff")),
            ("/destination/startTicks", json!(0)),
            ("/predecessor/digest", json!("sha256:00")),
            ("/browser/profileInode", json!(0)),
        ] {
            let mut state = current.clone();
            *state
                .runtime_custody_receipts
                .get_mut("s")
                .unwrap()
                .pointer_mut(change.0)
                .unwrap() = change.1;
            let proof = super::super::runtime_attestation::diagnostics(
                Some(&state),
                &fixture.request.handle,
                "s",
                Some("t1"),
            );
            assert_eq!(proof["complete"], false);
        }
        let mut released = fixture.request.handle.clone();
        released.insert("leaseState".into(), json!("released"));
        assert_eq!(
            super::super::runtime_attestation::diagnostics(
                Some(&current),
                &released,
                "s",
                Some("t1")
            )["complete"],
            false
        );
        fs::remove_file(fixture._profile.0.join("SingletonLock")).unwrap();
        assert_eq!(
            super::super::runtime_attestation::diagnostics(
                Some(&current),
                &fixture.request.handle,
                "s",
                Some("t1")
            )["complete"],
            false
        );
        drop(committed);
        server.finish().await;
    }

    fn identity(pid: u64) -> Value {
        json!({"pid": pid, "bootId": "00000000-0000-4000-8000-000000000001", "startTicks": 7, "uid": 1000,
            "executableDevice": 1, "executableInode": 2})
    }

    fn predecessor() -> Value {
        json!({
            "schemaVersion": 4, "phase": "committed", "ownerGeneration": 3,
            "source": identity(40), "destination": identity(41),
            "browser": {"process": identity(9), "canonicalProfile": "/synthetic/profile",
                "profileDevice": 3, "profileInode": 4, "cdpEndpoint": ENDPOINT},
            "browserId": "session:s", "profileId": "p", "targetId": "t0", "cdpEndpoint": ENDPOINT,
            "priorAnchor": {"targetId": "old", "disposition": "closed_historical"},
        })
    }

    fn receipt(body: &Value) -> Value {
        json!({
            "schemaVersion": 5, "kind": KIND, "chainId": uuid::Uuid::new_v4().to_string(),
            "ownerGeneration": 1, "phase": "committed", "destination": identity(42),
            "browser": body["browser"], "browserId": "session:s", "profileId": "p",
            "targetId": "t1", "cdpEndpoint": ENDPOINT,
            "predecessor": {"digest": canonical_digest(body), "body": body, "ownerExit": OWNER_EXIT},
        })
    }

    #[test]
    fn canonical_digest_ignores_key_order_recursively() {
        let left: Value =
            serde_json::from_str(r#"{"b":{"y":1,"x":[{"q":1,"p":2}]},"a":2}"#).unwrap();
        let right: Value =
            serde_json::from_str(r#"{"a":2,"b":{"x":[{"p":2,"q":1}],"y":1}}"#).unwrap();
        assert_eq!(canonical_digest(&left), canonical_digest(&right));
        assert_ne!(canonical_digest(&left), canonical_digest(&json!({"a": 3})));
    }

    #[test]
    fn fresh_receipt_embeds_predecessor_verbatim_with_independent_chain() {
        let body = predecessor();
        let bytes = serde_json::to_vec(&body).unwrap();
        let digest = canonical_digest(&body);
        let receipt = receipt(&body);
        verify_receipt_shape(&receipt, &identity(42)).unwrap();
        assert_eq!(receipt["predecessor"]["body"], body);
        assert_eq!(
            serde_json::to_vec(&receipt["predecessor"]["body"]).unwrap(),
            bytes
        );
        assert_eq!(canonical_digest(&body), digest);
        assert_eq!(receipt["ownerGeneration"], 1);
        assert!(receipt.get("source").is_none());
        assert_ne!(receipt["chainId"], Value::Null);
    }

    #[test]
    fn fresh_receipt_rejects_tampering_fail_closed() {
        let body = predecessor();
        let base = receipt(&body);
        let cases: Vec<fn(&mut Value)> = vec![
            |r| r["schemaVersion"] = json!(4),
            |r| r["kind"] = json!("continuation"),
            |r| r["ownerGeneration"] = json!(2),
            |r| r["phase"] = json!("prepared"),
            |r| r["chainId"] = json!("not-a-uuid"),
            |r| r["source"] = identity(40),
            |r| r["destination"] = identity(43),
            |r| {
                r.as_object_mut().unwrap().remove("predecessor");
            },
            |r| r["predecessor"]["extra"] = json!(true),
            |r| r["predecessor"]["ownerExit"] = json!("acknowledged"),
            |r| r["predecessor"]["body"]["targetId"] = json!("replayed"),
            |r| r["predecessor"]["digest"] = json!("sha256:00"),
            |r| r["predecessor"]["body"] = json!("malformed"),
            |r| r["profileId"] = json!("other"),
            |r| r["browser"]["process"] = identity(10),
        ];
        for mutate in cases {
            let mut tampered = base.clone();
            mutate(&mut tampered);
            assert!(verify_receipt_shape(&tampered, &identity(42)).is_err());
        }
        let mut zero = predecessor();
        zero["ownerGeneration"] = json!(0);
        assert!(verify_receipt_shape(&receipt(&zero), &identity(42)).is_err());
        let mut current_owner = predecessor();
        current_owner["destination"] = identity(42);
        assert!(verify_receipt_shape(&receipt(&current_owner), &identity(42)).is_err());
        let mut incomplete = predecessor();
        incomplete["destination"]
            .as_object_mut()
            .unwrap()
            .remove("startTicks");
        assert!(verify_receipt_shape(&receipt(&incomplete), &identity(42)).is_err());
    }

    #[test]
    fn pinned_endpoint_rejects_non_browser_websocket_input() {
        assert_eq!(pinned_loopback_browser_ws(ENDPOINT).unwrap(), ENDPOINT);
        for input in [
            "9222",
            "http://127.0.0.1:9222",
            "ws://localhost:9222/devtools/browser/x",
            "ws://127.0.0.1/devtools/browser/x",
            "ws://user:pw@127.0.0.1:9222/devtools/browser/x",
            "ws://127.0.0.1:9222/devtools/browser/x?q=1",
            "ws://127.0.0.1:9222/devtools/browser/x#f",
            "ws://127.0.0.1:9222/devtools/page/x",
            "ws://10.0.0.1:9222/devtools/browser/x",
            "wss://127.0.0.1:9222/devtools/browser/x",
        ] {
            assert!(pinned_loopback_browser_ws(input).is_err(), "{input}");
        }
    }

    struct NoAdmissionRepository {
        mutations: Cell<usize>,
    }

    impl ServiceStateRepository for NoAdmissionRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(ServiceState::default())
        }

        fn mutate<R>(
            &self,
            _mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            self.mutations.set(self.mutations.get() + 1);
            Err("unexpected_mutation".into())
        }
    }

    #[tokio::test]
    async fn failed_admission_never_connects_or_mutates() {
        let repository = NoAdmissionRepository {
            mutations: Cell::new(0),
        };
        let request = BootstrapRequest {
            session_name: "s".into(),
            handle: Map::new(),
            expected_predecessor_sha256: "sha256:00".into(),
            profile: PhysicalProfileExpectation {
                canonical_profile: PathBuf::from("/synthetic/profile"),
                profile_device: 3,
                profile_inode: 4,
            },
        };
        assert!(bootstrap(&repository, request).await.is_err());
        assert_eq!(repository.mutations.get(), 0);
    }

    async fn wire(
        targets: Value,
        session: &'static str,
        echo: Value,
    ) -> (Result<String, String>, Vec<Value>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let observed = Arc::new(Mutex::new(Vec::<Value>::new()));
        let recorder = Arc::clone(&observed);
        let mut server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(message)) = websocket.next().await {
                let Message::Text(text) = message else {
                    continue;
                };
                let command: Value = serde_json::from_str(&text).unwrap();
                recorder.lock().unwrap().push(command.clone());
                let result = match command["method"].as_str().unwrap() {
                    "Target.getTargets" => json!({"targetInfos": targets}),
                    "Target.attachToTarget" => json!({"sessionId": session}),
                    "Target.getTargetInfo" => json!({"targetInfo": echo}),
                    _ => json!({}),
                };
                let mut response = json!({"id": command["id"], "result": result});
                if let Some(session_id) = command.get("sessionId") {
                    response["sessionId"] = session_id.clone();
                }
                if websocket
                    .send(Message::Text(response.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let endpoint =
            crate::native::cdp::client::TestCdpEndpoint::new(&format!("ws://{address}")).unwrap();
        let client = endpoint.connect().await.unwrap();
        let result = exact_attach_wire(&client, "t1")
            .await
            .map(|page| format!("{}|{}", page.target_id, page.session_id));
        drop(client);
        if tokio::time::timeout(std::time::Duration::from_secs(5), &mut server)
            .await
            .is_err()
        {
            server.abort();
            let _ = server.await;
            panic!("wire fixture connection did not close");
        }
        let observed = observed.lock().unwrap().clone();
        for command in &observed {
            assert!(command["method"].as_str().unwrap().starts_with("Target."));
        }
        (result, observed)
    }

    fn methods(observed: &[Value]) -> Vec<&str> {
        observed
            .iter()
            .map(|command| command["method"].as_str().unwrap())
            .collect()
    }

    #[tokio::test]
    async fn exact_attach_wire_attaches_only_the_approved_page() {
        let (result, observed) = wire(
            json!([{"targetId": "t2", "type": "page"}, {"targetId": "t1", "type": "page"}]),
            "s1",
            json!({"targetId": "t1", "type": "page"}),
        )
        .await;
        assert_eq!(result.unwrap(), "t1|s1");
        assert_eq!(
            methods(&observed),
            [
                "Target.getTargets",
                "Target.attachToTarget",
                "Target.getTargetInfo"
            ]
        );
        assert_eq!(observed[1]["params"]["targetId"], "t1");
        assert_eq!(observed[1]["params"]["flatten"], true);
        assert_eq!(observed[2]["sessionId"], "s1");
        assert!(observed[2]
            .get("params")
            .and_then(|params| params.get("targetId"))
            .is_none());
    }

    #[tokio::test]
    async fn exact_attach_wire_fails_before_attach_without_one_page() {
        for targets in [
            json!([{"targetId": "t2", "type": "page"}]),
            json!([{"targetId": "t1", "type": "page"}, {"targetId": "t1", "type": "page"}]),
            json!([{"targetId": "t1", "type": "iframe"}]),
        ] {
            let (result, observed) =
                wire(targets, "s1", json!({"targetId": "t1", "type": "page"})).await;
            assert!(result.is_err());
            assert_eq!(methods(&observed), ["Target.getTargets"]);
        }
    }

    #[tokio::test]
    async fn exact_attach_wire_rejects_empty_or_substituted_session() {
        let (result, observed) = wire(
            json!([{"targetId": "t1", "type": "page"}]),
            "",
            json!({"targetId": "t1", "type": "page"}),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            methods(&observed),
            ["Target.getTargets", "Target.attachToTarget"]
        );
        for echo in [
            json!({"targetId": "t2", "type": "page"}),
            json!({"targetId": "t1", "type": "worker"}),
        ] {
            let (result, observed) =
                wire(json!([{"targetId": "t1", "type": "page"}]), "s1", echo).await;
            assert!(result.is_err());
            assert_eq!(observed.len(), 3);
        }
    }
}
