//! Exact-target, daemon-owned CDP attachment for a transferred browser.
//! It never exposes Chrome's WebSocket endpoint or accepts caller-selected CDP
//! sessions. Every command is mapped to existing task-authority consequences.

use super::actions::DaemonState;
use super::cdp::client::CdpClient;
use super::cdp::types::CdpEvent;
use super::private_broker::validate_broker_snapshot;
use super::private_identity::LivePrivateIdentity;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

const INVALID: &str = "custody_broker_identity_invalid";
const UNKNOWN: &str = "custody_broker_attachment_outcome_unknown";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AttachmentState {
    Uncertain,
    Active,
    Detached,
}

pub(crate) struct BrokerAttachment {
    binding: Value,
    handle: Value,
    client: Arc<CdpClient>,
    page_session: Option<String>,
    events: broadcast::Receiver<CdpEvent>,
    cursor: u64,
    replies: HashMap<String, Value>,
    consumed: HashSet<String>,
    state: AttachmentState,
    journal_path: PathBuf,
    journal_details: Value,
    cleanup_only: bool,
}

fn journal_path(session: &str) -> PathBuf {
    let digest = format!("{:x}", Sha256::digest(session.as_bytes()));
    crate::connection::get_socket_dir().join(format!("custody-broker-{digest}.json"))
}

fn private_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "custody_broker_journal_unavailable")?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("custody_broker_journal_unavailable".into());
    }
    Ok(())
}

fn read_journal(path: &Path) -> Result<Option<Value>, String> {
    let parent = path.parent().ok_or("custody_broker_journal_unavailable")?;
    private_directory(parent)?;
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("custody_broker_journal_unavailable".into()),
    };
    let metadata = file
        .metadata()
        .map_err(|_| "custody_broker_journal_unavailable")?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 65_536
    {
        return Err("custody_broker_journal_unavailable".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "custody_broker_journal_unavailable")?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "custody_broker_journal_unavailable")?;
    if value.get("schema") != Some(&json!("agent-browser.custody-broker.v1"))
        || !value.get("binding").is_some_and(Value::is_object)
        || [
            "attachmentId",
            "browserId",
            "profileId",
            "sessionName",
            "targetId",
            "generation",
        ]
        .iter()
        .any(|key| bounded_str(&value["binding"], key).is_err())
        || !value.get("details").is_some_and(Value::is_object)
        || bounded_str(&value["details"], "url").is_err()
        || value["details"]["browserPid"]
            .as_u64()
            .is_none_or(|pid| pid == 0)
        || bounded_str(&value["details"], "endpointSha256").is_err()
        || value["details"]["endpointSha256"]
            .as_str()
            .is_none_or(|digest| {
                digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        || !matches!(
            value.get("phase").and_then(Value::as_str),
            Some("attaching" | "attached" | "detaching" | "detached")
        )
    {
        return Err("custody_broker_journal_unavailable".into());
    }
    if value["phase"] != "attaching" && bounded_str(&value["details"], "pageSession").is_err() {
        return Err("custody_broker_journal_unavailable".into());
    }
    Ok(Some(value))
}

fn write_journal(path: &Path, binding: &Value, phase: &str, details: &Value) -> Result<(), String> {
    let parent = path.parent().ok_or("custody_broker_journal_unavailable")?;
    private_directory(parent)?;
    let previous = read_journal(path)?;
    if let Some(previous) = previous.as_ref() {
        let old_phase = previous
            .get("phase")
            .and_then(Value::as_str)
            .ok_or("custody_broker_journal_unavailable")?;
        let same = same_binding(
            previous
                .get("binding")
                .ok_or("custody_broker_journal_unavailable")?,
            binding,
        );
        let same_browser = ["url", "browserPid", "endpointSha256"]
            .iter()
            .all(|key| previous["details"][key] == details[key]);
        if !(old_phase == "detached" && phase == "attaching"
            || same
                && same_browser
                && matches!(
                    (old_phase, phase),
                    ("attaching", "attached")
                        | ("attaching", "detaching")
                        | ("attached", "detaching")
                        | ("detaching", "detached")
                ))
        {
            return Err("custody_broker_journal_transition_invalid".into());
        }
    } else if phase != "attaching" {
        return Err("custody_broker_journal_transition_invalid".into());
    }
    let staged = parent.join(format!(".custody-broker-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&staged)
            .map_err(|_| "custody_broker_journal_unavailable")?;
        let bytes = serde_json::to_vec(
            &json!({"schema":"agent-browser.custody-broker.v1", "binding":binding, "phase":phase, "details":details}),
        )
        .map_err(|_| "custody_broker_journal_unavailable")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "custody_broker_journal_unavailable")?;
        fs::rename(&staged, path).map_err(|_| "custody_broker_journal_unavailable")?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "custody_broker_journal_unavailable")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result.map_err(str::to_string)
}

fn bounded_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|part| !part.is_empty() && part.len() <= 4096 && part.trim() == *part)
        .ok_or_else(|| INVALID.to_string())
}

fn same_binding(expected: &Value, actual: &Value) -> bool {
    [
        "attachmentId",
        "browserId",
        "profileId",
        "sessionName",
        "targetId",
        "generation",
    ]
    .into_iter()
    .all(|key| expected.get(key).and_then(Value::as_str) == actual.get(key).and_then(Value::as_str))
}

fn live_target_info(info: &Value, target: &str, url: &str) -> Result<(), String> {
    if info.pointer("/targetInfo/targetId").and_then(Value::as_str) != Some(target)
        || info.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
        || info.pointer("/targetInfo/url").and_then(Value::as_str) != Some(url)
    {
        return Err("custody_broker_rendered_target_changed".into());
    }
    Ok(())
}

fn passive_event_admitted(method: &str) -> bool {
    matches!(
        method,
        "Page.javascriptDialogOpening"
            | "Page.javascriptDialogClosed"
            | "Page.loadEventFired"
            | "Page.domContentEventFired"
            | "Page.frameNavigated"
            | "Page.frameDetached"
            | "Network.responseReceived"
            | "Network.loadingFinished"
            | "Network.loadingFailed"
            | "Network.requestWillBeSent"
            | "Runtime.executionContextCreated"
            | "Runtime.executionContextDestroyed"
            | "Runtime.executionContextsCleared"
            | "Runtime.consoleAPICalled"
            | "Runtime.exceptionThrown"
    )
}

pub(crate) async fn attach(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    if cmd.get("brokerTransport").and_then(Value::as_bool) != Some(true)
        || cmd.get("cdpAttachmentAllowed").and_then(Value::as_bool) != Some(true)
        || !cmd.get("taskAuthority").is_some_and(Value::is_object)
        || cmd
            .get("taskStepId")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || cmd
            .get("taskEvidenceBytes")
            .and_then(Value::as_u64)
            .is_none_or(|bytes| bytes == 0)
        || state
            .custody_broker_attachment
            .as_ref()
            .is_some_and(|attachment| attachment.state != AttachmentState::Detached)
    {
        return Err("custody_broker_attach_not_admitted".into());
    }
    let handle = cmd.get("serviceTabHandle").ok_or(INVALID)?;
    let target = bounded_str(handle, "targetId")?.to_owned();
    state.select_custody_broker_target(&target).await?;
    let (active_target, generation) = state.custody_broker_identity().ok_or(INVALID)?;
    if active_target != target {
        return Err(INVALID.into());
    }
    let profile = state.private_runtime_profile().ok_or(INVALID)?.to_owned();
    let url = bounded_str(cmd, "url")?.to_owned();
    let browser_id = format!("session:{}", state.session_id);
    if bounded_str(handle, "targetId")? != target
        || bounded_str(handle, "browserId")? != browser_id
        || bounded_str(handle, "profileId")? != profile
        || bounded_str(handle, "sessionName")? != state.session_id
        || bounded_str(handle, "url")? != url
        || handle.get("valid").and_then(Value::as_bool) != Some(true)
    {
        return Err(INVALID.into());
    }
    let browser = state.browser.as_ref().ok_or(INVALID)?;
    let browser_pid = state.custody_broker_process_id().ok_or(INVALID)?;
    let endpoint_sha256 = format!("{:x}", Sha256::digest(browser.get_cdp_url().as_bytes()));
    let page_session = browser.page_session_for_target(&target).ok_or(INVALID)?;
    let client = browser.client.clone();
    let info = client
        .send_command_with_timeout(
            "Target.getTargetInfo",
            None,
            Some(page_session),
            Duration::from_secs(3),
        )
        .await
        .map_err(|_| "custody_broker_target_probe_failed".to_string())?;
    live_target_info(&info, &target, &url)?;
    let origin = url::Url::parse(&url)
        .map_err(|_| INVALID.to_string())?
        .origin()
        .ascii_serialization();
    let snapshot = LockedServiceStateRepository::default_json()?.load_snapshot()?;
    validate_broker_snapshot(
        &snapshot,
        handle,
        &LivePrivateIdentity {
            profile_id: &profile,
            browser_id: &browser_id,
            session_name: &state.session_id,
            target_id: &target,
            url: &url,
            ready: true,
        },
        browser.get_cdp_url(),
        &origin,
        &url,
    )
    .map_err(str::to_string)?;

    // A lost attach reply is never retried. Retain an uncertain in-memory fence
    // before asking Chrome to create a distinct flattened page session.
    let binding = json!({
        "attachmentId": uuid::Uuid::new_v4().to_string(),
        "browserId": browser_id,
        "profileId": profile,
        "sessionName": state.session_id,
        "targetId": target,
        "generation": generation,
    });
    let details = json!({"url":url,"browserPid":browser_pid,"endpointSha256":endpoint_sha256});
    let journal_path = journal_path(&state.session_id);
    if read_journal(&journal_path)?.is_some_and(|journal| journal["phase"] != "detached") {
        return Err("custody_broker_prior_attachment_unreconciled".into());
    }
    write_journal(&journal_path, &binding, "attaching", &details)?;
    state.custody_broker_attachment = Some(BrokerAttachment {
        binding: binding.clone(),
        handle: handle.clone(),
        client: client.clone(),
        page_session: None,
        events: client.subscribe(),
        cursor: 0,
        replies: HashMap::new(),
        consumed: HashSet::new(),
        state: AttachmentState::Uncertain,
        journal_path,
        journal_details: details,
        cleanup_only: false,
    });
    let response = client
        .send_command_with_timeout(
            "Target.attachToTarget",
            Some(json!({"targetId": target, "flatten": true})),
            None,
            Duration::from_secs(3),
        )
        .await
        .map_err(|_| UNKNOWN.to_string())?;
    let session = bounded_str(&response, "sessionId")?.to_owned();
    let attachment = state.custody_broker_attachment.as_mut().ok_or(INVALID)?;
    attachment.page_session = Some(session.clone());
    attachment.journal_details["pageSession"] = json!(session);
    let info = client
        .send_command_with_timeout(
            "Target.getTargetInfo",
            None,
            Some(&session),
            Duration::from_secs(3),
        )
        .await
        .map_err(|_| UNKNOWN.to_string())?;
    live_target_info(&info, &target, &url)?;
    let attachment = state.custody_broker_attachment.as_ref().ok_or(INVALID)?;
    write_journal(
        &attachment.journal_path,
        &attachment.binding,
        "attached",
        &attachment.journal_details,
    )?;
    state
        .custody_broker_attachment
        .as_mut()
        .ok_or(INVALID)?
        .state = AttachmentState::Active;
    Ok(json!({
        "attached": true,
        "controlPlaneMode": "broker",
        "transportAction": "__broker_transport",
        "binding": binding,
        "serviceTabHandle": handle,
        "detachRequired": true,
        "browserProcessPreserved": true,
        "closeBrowserOnDetach": false,
    }))
}

pub(crate) fn authority_handle(state: &DaemonState) -> Option<&Value> {
    state
        .custody_broker_attachment
        .as_ref()
        .map(|attachment| &attachment.handle)
}

pub(crate) fn has_unreconciled_attachment(state: &DaemonState) -> Result<bool, String> {
    if state
        .custody_broker_attachment
        .as_ref()
        .is_some_and(|attachment| attachment.state != AttachmentState::Detached)
    {
        return Ok(true);
    }
    Ok(read_journal(&journal_path(&state.session_id))?
        .is_some_and(|journal| journal["phase"] != "detached"))
}

async fn recover_cleanup_only(request: &Value, state: &mut DaemonState) -> Result<(), String> {
    if bounded_str(request, "operation")? != "detach" {
        return Err("custody_broker_prior_attachment_unreconciled".into());
    }
    let path = journal_path(&state.session_id);
    let journal = read_journal(&path)?.ok_or("custody_broker_attachment_missing")?;
    if journal["phase"] != "attached"
        || !same_binding(&journal["binding"], request.get("binding").ok_or(INVALID)?)
    {
        return Err("custody_broker_prior_attachment_unreconciled".into());
    }
    let target = bounded_str(&journal["binding"], "targetId")?.to_owned();
    let profile = bounded_str(&journal["binding"], "profileId")?.to_owned();
    let browser_id = format!("session:{}", state.session_id);
    if state.custody_broker_identity().map(|identity| identity.0) != Some(target.clone())
        || state.private_runtime_profile() != Some(profile.as_str())
        || bounded_str(&journal["binding"], "browserId")? != browser_id
        || bounded_str(&journal["binding"], "sessionName")? != state.session_id
    {
        return Err(INVALID.into());
    }
    let browser = state.browser.as_ref().ok_or(INVALID)?;
    let client = browser.client.clone();
    if state.custody_broker_process_id().map(u64::from) != journal["details"]["browserPid"].as_u64()
        || format!("{:x}", Sha256::digest(browser.get_cdp_url().as_bytes()))
            != bounded_str(&journal["details"], "endpointSha256")?
    {
        return Err(INVALID.into());
    }
    let page_session = bounded_str(&journal["details"], "pageSession")?.to_owned();
    let url = bounded_str(&journal["details"], "url")?.to_owned();
    let info = client
        .send_command_with_timeout(
            "Target.getTargetInfo",
            None,
            Some(&page_session),
            Duration::from_secs(3),
        )
        .await
        .map_err(|_| "custody_broker_recovery_target_probe_failed".to_string())?;
    live_target_info(&info, &target, &url)?;
    let snapshot = LockedServiceStateRepository::default_json()?.load_snapshot()?;
    let tabs = snapshot
        .tabs
        .values()
        .filter(|tab| {
            tab.browser_id == browser_id && tab.target_id.as_deref() == Some(target.as_str())
        })
        .collect::<Vec<_>>();
    let [tab] = tabs.as_slice() else {
        return Err(INVALID.into());
    };
    let handle = serde_json::to_value(snapshot.service_tab_handle(&tab.id).ok_or(INVALID)?)
        .map_err(|_| INVALID.to_string())?;
    let origin = url::Url::parse(&url)
        .map_err(|_| INVALID.to_string())?
        .origin()
        .ascii_serialization();
    validate_broker_snapshot(
        &snapshot,
        &handle,
        &LivePrivateIdentity {
            profile_id: &profile,
            browser_id: &browser_id,
            session_name: &state.session_id,
            target_id: &target,
            url: &url,
            ready: true,
        },
        browser.get_cdp_url(),
        &origin,
        &url,
    )
    .map_err(str::to_string)?;
    state.custody_broker_attachment = Some(BrokerAttachment {
        binding: journal["binding"].clone(),
        handle,
        client: client.clone(),
        page_session: Some(page_session),
        events: client.subscribe(),
        cursor: 0,
        replies: HashMap::new(),
        consumed: HashSet::new(),
        state: AttachmentState::Active,
        journal_path: path,
        journal_details: journal["details"].clone(),
        cleanup_only: true,
    });
    Ok(())
}

pub(crate) fn authority_action(cmd: &Value) -> Result<Option<&'static str>, String> {
    let request = cmd.get("brokerRequest").ok_or(INVALID)?;
    match bounded_str(request, "operation")? {
        "command" => match bounded_str(request, "method")? {
            "Runtime.evaluate"
                if request
                    .pointer("/params/expression")
                    .and_then(Value::as_str)
                    == Some("location.href") =>
            {
                Ok(Some("url"))
            }
            "Runtime.evaluate"
                if request
                    .pointer("/params/expression")
                    .and_then(Value::as_str)
                    == Some("document.title") =>
            {
                Ok(Some("title"))
            }
            "Runtime.evaluate" | "Runtime.callFunctionOn" => Ok(Some("evaluate")),
            "Runtime.enable"
            | "Runtime.disable"
            | "Runtime.getProperties"
            | "Runtime.releaseObject"
            | "Runtime.releaseObjectGroup"
            | "Page.enable"
            | "Page.disable"
            | "Page.getFrameTree"
            | "DOM.enable"
            | "DOM.disable"
            | "DOM.getDocument"
            | "DOM.querySelector"
            | "DOM.querySelectorAll"
            | "DOM.describeNode"
            | "DOM.resolveNode"
            | "Network.enable"
            | "Network.disable"
            | "Network.getResponseBody" => Ok(Some("diagnostics")),
            "Page.captureScreenshot" => Ok(Some("screenshot")),
            "Page.bringToFront"
            | "Page.handleJavaScriptDialog"
            | "Input.dispatchKeyEvent"
            | "Input.dispatchMouseEvent" => Ok(Some("ui_action")),
            "Input.insertText" => Ok(Some("type")),
            "DOM.setFileInputFiles" => Ok(Some("upload")),
            _ => Err("custody_broker_method_not_admitted".into()),
        },
        // Event delivery is passive, exact-session filtered, bounded, and part
        // of the already-authorized attachment. Requiring a freshly issued
        // ordered step for every empty poll would create confirmation pressure
        // without granting any additional browser effect.
        "events" => Ok(None),
        "detach" => Ok(None),
        _ => Err(INVALID.into()),
    }
}

pub(crate) async fn execute(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let request = cmd.get("brokerRequest").ok_or(INVALID)?;
    let operation = bounded_str(request, "operation")?;
    let request_id = bounded_str(request, "requestId")?;
    if request_id.len() > 128 || cmd.get("id").and_then(Value::as_str) != Some(request_id) {
        return Err(INVALID.into());
    }
    if state.custody_broker_attachment.is_none() {
        recover_cleanup_only(request, state).await?;
    }
    let attachment = state.custody_broker_attachment.as_mut().ok_or(INVALID)?;
    if !same_binding(&attachment.binding, request.get("binding").ok_or(INVALID)?) {
        return Err(INVALID.into());
    }
    if let Some(reply) = attachment.replies.get(request_id) {
        return Ok(reply.clone());
    }
    if !attachment.consumed.insert(request_id.to_owned()) {
        return Err("custody_broker_outcome_unknown_no_replay".into());
    }
    let response = match operation {
        "detach" => {
            if attachment.state == AttachmentState::Uncertain && attachment.page_session.is_none() {
                return Err(UNKNOWN.into());
            }
            if attachment.state != AttachmentState::Detached {
                write_journal(
                    &attachment.journal_path,
                    &attachment.binding,
                    "detaching",
                    &attachment.journal_details,
                )?;
                attachment.state = AttachmentState::Uncertain;
                let session = attachment.page_session.as_deref().ok_or(INVALID)?;
                let result = attachment
                    .client
                    .send_command_with_timeout(
                        "Target.detachFromTarget",
                        Some(json!({"sessionId": session})),
                        None,
                        Duration::from_secs(3),
                    )
                    .await
                    .map_err(|_| UNKNOWN.to_string())?;
                if result != json!({}) {
                    return Err(UNKNOWN.into());
                }
                write_journal(
                    &attachment.journal_path,
                    &attachment.binding,
                    "detached",
                    &attachment.journal_details,
                )?;
                attachment.state = AttachmentState::Detached;
            }
            json!({"requestId": request_id, "binding": attachment.binding, "detached": true, "browserPreserved": true})
        }
        "command" => {
            if attachment.state != AttachmentState::Active || attachment.cleanup_only {
                return Err(INVALID.into());
            }
            authority_action(cmd)?;
            let session = attachment.page_session.as_deref().ok_or(INVALID)?;
            let params = request
                .get("params")
                .filter(|value| value.is_object())
                .ok_or(INVALID)?
                .clone();
            if params.get("sessionId").is_some()
                || params.get("targetId").is_some()
                || serde_json::to_vec(&params)
                    .map_err(|_| INVALID.to_string())?
                    .len()
                    > 8_388_608
            {
                return Err("custody_broker_params_not_admitted".into());
            }
            let target = bounded_str(&attachment.binding, "targetId")?;
            let url = bounded_str(&attachment.handle, "url")?;
            let info = attachment
                .client
                .send_command_with_timeout(
                    "Target.getTargetInfo",
                    None,
                    Some(session),
                    Duration::from_secs(3),
                )
                .await
                .map_err(|_| "custody_broker_target_probe_failed".to_string())?;
            live_target_info(&info, target, url)?;
            let method = bounded_str(request, "method")?;
            let result = attachment
                .client
                .send_command_with_timeout(
                    method,
                    Some(params),
                    Some(session),
                    Duration::from_secs(30),
                )
                .await
                .map_err(|_| UNKNOWN.to_string())?;
            json!({"requestId": request_id, "binding": attachment.binding, "result": result})
        }
        "events" => {
            if attachment.state != AttachmentState::Active || attachment.cleanup_only {
                return Err(INVALID.into());
            }
            if request.get("cursor").and_then(Value::as_u64) != Some(attachment.cursor) {
                return Err(INVALID.into());
            }
            let mut events = Vec::new();
            loop {
                match attachment.events.try_recv() {
                    Ok(event) => {
                        if event.session_id.as_deref() != attachment.page_session.as_deref() {
                            continue;
                        }
                        // This passive version never subscribes to provider event domains.
                        // Unexpected events are not forwarded into a new authority context.
                        if !passive_event_admitted(&event.method) {
                            // Chrome emits many domain events incidentally once a
                            // provider enables Runtime/Page/Network. They are not
                            // requested evidence and grant no capability, so keep
                            // them inside daemon custody instead of tearing down
                            // the authorized attachment.
                            continue;
                        }
                        if events.len() == 256 {
                            return Err("custody_broker_event_overflow".into());
                        }
                        attachment.cursor += 1;
                        events.push(json!({"sequence":attachment.cursor,"method":event.method,"params":event.params}));
                    }
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(_) => return Err("custody_broker_event_overflow".into()),
                }
            }
            json!({"requestId":request_id,"binding":attachment.binding,"cursor":attachment.cursor,"overflow":false,"events":events})
        }
        _ => return Err(INVALID.into()),
    };
    if attachment.replies.len() >= 4096 {
        return Err("custody_broker_receipt_capacity_exhausted".into());
    }
    attachment
        .replies
        .insert(request_id.to_owned(), response.clone());
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;

    #[test]
    fn methods_are_mapped_to_existing_authority_consequences() {
        for expression in ["location.href", "document.title"] {
            let command = json!({"brokerRequest":{"operation":"command","method":"Runtime.evaluate","params":{"expression":expression}}});
            assert!(authority_action(&command).unwrap().is_some());
        }
        for expression in [
            "location.href; fetch('/send')",
            "document.querySelector('button').click()",
        ] {
            let command = json!({"brokerRequest":{"operation":"command","method":"Runtime.evaluate","params":{"expression":expression}}});
            assert_eq!(authority_action(&command).unwrap(), Some("evaluate"));
        }
        assert_eq!(authority_action(&json!({"brokerRequest":{"operation":"command","method":"Input.dispatchKeyEvent","params":{}}})).unwrap(), Some("ui_action"));
        assert!(authority_action(
            &json!({"brokerRequest":{"operation":"command","method":"Page.navigate","params":{}}})
        )
        .is_err());
        assert!(authority_action(
            &json!({"brokerRequest":{"operation":"command","method":"Browser.close","params":{}}})
        )
        .is_err());
    }

    #[test]
    fn passive_event_filter_keeps_incidental_chrome_events_private() {
        assert!(passive_event_admitted("Page.loadEventFired"));
        assert!(!passive_event_admitted("Page.frameStartedLoading"));
        assert!(!passive_event_admitted("Runtime.bindingCalled"));
    }

    #[test]
    fn binding_requires_all_six_exact_fields() {
        let expected = json!({"attachmentId":"a","browserId":"b","profileId":"p","sessionName":"s","targetId":"t","generation":"g"});
        assert!(same_binding(&expected, &expected));
        let mut changed = expected.clone();
        changed["targetId"] = json!("other");
        assert!(!same_binding(&expected, &changed));
    }

    #[test]
    fn journal_fences_restart_and_accepts_only_verified_detach_sequence() {
        let root =
            std::env::temp_dir().join(format!("custody-broker-journal-{}", uuid::Uuid::new_v4()));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let path = root.join("receipt.json");
        let binding = json!({"attachmentId":"a","browserId":"b","profileId":"p","sessionName":"s","targetId":"t","generation":"g"});
        let details = json!({"url":"https://example.test/custody","browserPid":12,"endpointSha256":"a".repeat(64)});
        let mut attached_details = details.clone();
        attached_details["pageSession"] = json!("session-one");
        write_journal(&path, &binding, "attaching", &details).unwrap();
        assert_eq!(read_journal(&path).unwrap().unwrap()["phase"], "attaching");
        assert!(write_journal(&path, &binding, "attaching", &details).is_err());
        write_journal(&path, &binding, "attached", &attached_details).unwrap();
        write_journal(&path, &binding, "detaching", &attached_details).unwrap();
        assert!(write_journal(&path, &binding, "attached", &attached_details).is_err());
        write_journal(&path, &binding, "detached", &attached_details).unwrap();
        let mut next = binding.clone();
        next["attachmentId"] = json!("next");
        write_journal(&path, &next, "attaching", &details).unwrap();
        assert_eq!(read_journal(&path).unwrap().unwrap()["binding"], next);
        fs::remove_dir_all(root).unwrap();
    }
}
