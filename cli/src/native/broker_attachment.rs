//! Native broker transport core. Experimental worker acquisition supplies real
//! transferred custody and authenticated concurrent ingress; no default client
//! or installed AuraCall path selects it yet.
//! No endpoint export, browser close, raw-send, or implicit replay is provided.

use super::cdp::client::CdpClient;
use super::cdp::types::CdpEvent;
use super::privacy_gate::PublicLease;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, OnceCell};

/// Owns both barriers through socket publication; no unguarded JSON export.
pub(crate) struct BrokerOutput {
    value: Value,
    wire_limit: Option<u64>,
    _authority: Box<dyn Send>,
    _privacy: Option<PublicLease>,
}

impl BrokerOutput {
    pub(crate) async fn write_daemon_response<W: tokio::io::AsyncWrite + Unpin>(
        mut self,
        id: &str,
        writer: &mut W,
    ) -> Result<(), String> {
        self.value = json!({"id":id,"success":true,"data":self.value});
        self.write_json(writer).await
    }
    pub(crate) async fn write_json<W: tokio::io::AsyncWrite + Unpin>(
        self,
        writer: &mut W,
    ) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;
        let mut bytes = serde_json::to_vec(&self.value).map_err(|_| INVALID)?;
        bytes.push(b'\n');
        if self
            .wire_limit
            .is_some_and(|limit| bytes.len() as u64 > limit)
        {
            return Err("broker_event_evidence_budget_exceeded".into());
        }
        writer.write_all(&bytes).await.map_err(|_| UNKNOWN.into())
    }
}

const UNKNOWN: &str = "broker_cdp_outcome_unknown_no_replay";
const INVALID: &str = "broker_attachment_invalid";
const TIMEOUT: Duration = Duration::from_secs(5);

struct SealOnCancellation<'a>(Option<&'a AtomicBool>);
impl Drop for SealOnCancellation<'_> {
    fn drop(&mut self) {
        if let Some(sealed) = self.0 {
            sealed.store(true, Ordering::SeqCst);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrokerBinding {
    pub attachment_id: String,
    pub browser_id: String,
    pub profile_id: String,
    pub session_name: String,
    pub target_id: String,
    pub generation: String,
}

/// The daemon must verify exact live profile/browser/session/target/URL, ready
/// health, held custody generation and action-specific policy, not echoed JSON.
/// The returned guard must hold authority until response delivery finishes.
pub(crate) trait BrokerAuthority: Send + Sync {
    fn admit(&self, binding: &BrokerBinding, operation: &str) -> Result<Box<dyn Send>, String>;

    /// Required, with no method-only fallback. Implementations must validate the
    /// full task envelope, params, live URL and consequence before reserving the
    /// task budget. The returned permit owns that reservation and its identity.
    fn admit_command<'a>(
        &'a self,
        request: &'a BrokerCommandRequest,
        live_target: &'a Value,
    ) -> BrokerAdmissionFuture<'a>;

    /// Event extraction has its own task packet and reservation. A held browser
    /// lease alone does not authorize reading network or console payloads.
    fn admit_events<'a>(
        &'a self,
        request: &'a BrokerEventRequest,
        live_target: &'a Value,
    ) -> BrokerAdmissionFuture<'a>;
}

pub(crate) type BrokerAdmissionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn BrokerCommandPermit>, String>> + Send + 'a>>;
pub(crate) type BrokerPublicationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Request data is evidence for daemon validation, never authority by itself.
/// Own all bytes so admission and publication cannot see different requests.
pub(crate) struct BrokerCommandRequest {
    pub binding: BrokerBinding,
    pub request_id: String,
    pub method: String,
    pub params: Value,
    pub task_context: Value,
}

pub(crate) struct BrokerEventRequest {
    pub binding: BrokerBinding,
    pub request_id: String,
    pub cursor: u64,
    pub task_context: Value,
}

/// Keep the same permit through execution, outcome finalization and socket write.
/// Dropping an unfinished permit must leave its durable reservation uncertain;
/// it must never restore a consumed ordered step or authorize replay.
pub(crate) trait BrokerCommandPermit: Send {
    /// Preserve the admitted byte limit through framing and the final write.
    fn wire_limit(&self) -> Option<u64> {
        None
    }
    fn publish<'a>(
        &'a mut self,
        result: &'a Value,
        live_target: &'a Value,
    ) -> BrokerPublicationFuture<'a>;
}

/// Trusted daemon-selected directory, never a request parameter. Create-new and
/// fsync precede dispatch; a crash or repeated id therefore cannot replay input.
/// Admission stores identity digests; a separate protected receipt retains the
/// owned CDP session for reconciliation. Never store scripts or page responses.
pub(crate) struct BrokerJournal(pub PathBuf);

impl BrokerJournal {
    fn retain_identity(&self, binding: &BrokerBinding, page_session: &str) -> Result<(), String> {
        let name = format!(
            "{}.attachment",
            hex::encode(Sha256::digest(
                serde_json::to_vec(binding).map_err(|_| INVALID)?
            ))
        );
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(self.0.join(name)).map_err(|_| UNKNOWN)?;
        let bytes = serde_json::to_vec(&json!({"binding":binding,"pageSessionId":page_session}))
            .map_err(|_| UNKNOWN)?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| UNKNOWN)?;
        File::open(&self.0)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| UNKNOWN)?;
        Ok(())
    }

    fn admit(&self, binding: &BrokerBinding, request: &str) -> Result<(), String> {
        if request.is_empty() || request.len() > 128 || request.chars().any(char::is_control) {
            return Err(INVALID.into());
        }
        let encoded = serde_json::to_vec(&(binding, request)).map_err(|_| INVALID)?;
        let name = hex::encode(Sha256::digest(encoded));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(self.0.join(name)).map_err(|_| UNKNOWN)?;
        file.write_all(b"admitted-no-replay\n")
            .map_err(|_| UNKNOWN)?;
        file.sync_all().map_err(|_| UNKNOWN)?;
        File::open(&self.0)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| UNKNOWN)?;
        Ok(())
    }
}

pub(crate) struct BrokerAttachment {
    binding: BrokerBinding,
    client: Arc<CdpClient>,
    authority: Arc<dyn BrokerAuthority>,
    journal: BrokerJournal,
    page_session: String,
    events: Mutex<(broadcast::Receiver<CdpEvent>, u64)>,
    sealed: AtomicBool,
    detached: OnceCell<Result<(), String>>,
    detach_started: AtomicBool,
}

impl BrokerAttachment {
    pub(crate) fn binding(&self) -> &BrokerBinding {
        &self.binding
    }
    pub(crate) fn seal(&self) {
        self.sealed.store(true, Ordering::SeqCst);
    }
    pub(crate) async fn acquire(
        binding: BrokerBinding,
        client: Arc<CdpClient>,
        authority: Arc<dyn BrokerAuthority>,
        journal: BrokerJournal,
        request_id: &str,
    ) -> Result<Self, String> {
        for value in [
            &binding.attachment_id,
            &binding.browser_id,
            &binding.profile_id,
            &binding.session_name,
            &binding.target_id,
            &binding.generation,
        ] {
            if value.is_empty()
                || value.len() > 4096
                || value.trim() != value
                || value.chars().any(char::is_control)
            {
                return Err(INVALID.into());
            }
        }
        let _authority = authority.admit(&binding, "attach")?;
        let _privacy = client.public_lease()?;
        journal.admit(&binding, request_id)?;
        let events = client.subscribe();
        let result = client
            .send_broker_command(
                "Target.attachToTarget",
                Some(json!({"targetId":binding.target_id,"flatten":true})),
                None,
                TIMEOUT,
                None,
            )
            .await
            .map_err(|_| UNKNOWN)?;
        let page_session = result
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 4096
                    && value.trim() == *value
                    && !value.chars().any(char::is_control)
            })
            .ok_or(UNKNOWN)?
            .to_owned();
        let attachment = Self {
            binding,
            client,
            authority,
            journal,
            page_session,
            events: Mutex::new((events, 0)),
            sealed: AtomicBool::new(false),
            detached: OnceCell::new(),
            detach_started: AtomicBool::new(false),
        };
        if attachment
            .journal
            .retain_identity(&attachment.binding, &attachment.page_session)
            .is_err()
        {
            attachment
                .detach(&attachment.binding, "failed-receipt-detach")
                .await?;
            return Err(UNKNOWN.into());
        }
        let verified = attachment
            .client
            .send_broker_command(
                "Target.getTargetInfo",
                None,
                Some(&attachment.page_session),
                TIMEOUT,
                None,
            )
            .await;
        let valid = verified.as_ref().is_ok_and(|value| {
            value
                .pointer("/targetInfo/targetId")
                .and_then(Value::as_str)
                == Some(attachment.binding.target_id.as_str())
                && value.pointer("/targetInfo/type").and_then(Value::as_str) == Some("page")
        });
        if !valid
            || attachment
                .authority
                .admit(&attachment.binding, "publish_attach")
                .is_err()
        {
            // Cleanup has separate admission and cannot close the browser.
            attachment
                .detach(&attachment.binding, "failed-acquire-detach")
                .await?;
            return Err(INVALID.into());
        }
        Ok(attachment)
    }

    fn check(&self, binding: &BrokerBinding, operation: &str) -> Result<Box<dyn Send>, String> {
        if binding != &self.binding || self.sealed.load(Ordering::SeqCst) {
            return Err(INVALID.into());
        }
        self.authority.admit(binding, operation)
    }

    /// Worker-only final acquisition check, on this attachment's owned session.
    /// A caller-supplied handle URL or a service snapshot alone is insufficient.
    pub(crate) async fn verify_acquired_url(&self, expected_url: &str) -> Result<(), String> {
        let _guard = self.check(&self.binding, "publish_attach")?;
        let target = self
            .client
            .send_broker_command(
                "Target.getTargetInfo",
                None,
                Some(&self.page_session),
                TIMEOUT,
                Some(&self.sealed),
            )
            .await
            .map_err(|_| "broker_acquisition_observation_failed")?;
        if target
            .pointer("/targetInfo/targetId")
            .and_then(Value::as_str)
            != Some(self.binding.target_id.as_str())
            || target.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
            || target.pointer("/targetInfo/url").and_then(Value::as_str) != Some(expected_url)
        {
            return Err("broker_acquisition_live_url_mismatch".into());
        }
        self.check(&self.binding, "publish_attach")?;
        Ok(())
    }

    pub(crate) async fn command_authorized(
        &self,
        binding: &BrokerBinding,
        request_id: &str,
        method: &str,
        params: Value,
        task_context: Value,
    ) -> Result<BrokerOutput, String> {
        // This check is only attachment/custody liveness. It cannot authorize a
        // CDP command; the required request-aware admission below must succeed.
        let _authority = self.check(binding, "command_preflight")?;
        let _privacy = self.client.public_lease()?;
        if !allowed_method(method)
            || !params.is_object()
            || params.get("sessionId").is_some()
            || params.get("targetId").is_some()
            || params.to_string().len() > 8_388_608
        {
            return Err(INVALID.into());
        }
        if !task_context.is_object() || task_context.to_string().len() > 65_536 {
            return Err(INVALID.into());
        }
        let request = BrokerCommandRequest {
            binding: binding.clone(),
            request_id: request_id.to_owned(),
            method: method.to_owned(),
            params,
            task_context,
        };
        self.journal.admit(binding, request_id)?;
        let mut cancellation = SealOnCancellation(Some(&self.sealed));
        let live_target = self.observe_target().await?;
        let mut permit = self.authority.admit_command(&request, &live_target).await?;
        let result = self
            .client
            .send_broker_command(
                method,
                Some(request.params.clone()),
                Some(&self.page_session),
                Duration::from_secs(120),
                Some(&self.sealed),
            )
            .await;
        match result {
            Ok(result) if !self.sealed.load(Ordering::SeqCst) => {
                let live_target = self.observe_target().await?;
                permit.publish(&result, &live_target).await?;
                cancellation.0 = None;
                Ok(BrokerOutput {
                    value: json!({"binding":binding,"requestId":request_id,"result":result}),
                    wire_limit: permit.wire_limit(),
                    _authority: Box::new(permit),
                    _privacy,
                })
            }
            _ => {
                self.sealed.store(true, Ordering::SeqCst);
                Err(UNKNOWN.into())
            }
        }
    }

    async fn observe_target(&self) -> Result<Value, String> {
        let info = self
            .client
            .send_broker_command(
                "Target.getTargetInfo",
                None,
                Some(&self.page_session),
                TIMEOUT,
                Some(&self.sealed),
            )
            .await
            .map_err(|_| INVALID)?;
        if info.pointer("/targetInfo/targetId").and_then(Value::as_str)
            != Some(self.binding.target_id.as_str())
            || info.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
            || info
                .pointer("/targetInfo/url")
                .and_then(Value::as_str)
                .is_none_or(|url| url.is_empty())
        {
            return Err(INVALID.into());
        }
        Ok(info)
    }

    #[cfg(test)]
    pub(crate) async fn events(
        &self,
        binding: &BrokerBinding,
        cursor: u64,
    ) -> Result<BrokerOutput, String> {
        self.events_authorized(
            binding,
            &uuid::Uuid::new_v4().to_string(),
            cursor,
            json!({"taskName":"fixture"}),
        )
        .await
    }

    /// Independent receiver lock: never waits for a pending command. Admission
    /// and publication both use the same explicit event task permit.
    pub(crate) async fn events_authorized(
        &self,
        binding: &BrokerBinding,
        request_id: &str,
        cursor: u64,
        task_context: Value,
    ) -> Result<BrokerOutput, String> {
        let _authority = self.check(binding, "event_preflight")?;
        let _privacy = self.client.public_lease()?;
        if !task_context.is_object() || task_context.to_string().len() > 65_536 {
            return Err(INVALID.into());
        }
        let mut receiver = self.events.try_lock().map_err(|_| INVALID)?;
        if receiver.1 != cursor {
            return Err(INVALID.into());
        }
        self.journal.admit(binding, request_id)?;
        let mut cancellation = SealOnCancellation(Some(&self.sealed));
        let request = BrokerEventRequest {
            binding: binding.clone(),
            request_id: request_id.into(),
            cursor,
            task_context,
        };
        let live_target = self.observe_target().await?;
        let mut permit = self.authority.admit_events(&request, &live_target).await?;
        let mut batch = Vec::new();
        let mut bytes = 0;
        let mut overflow = false;
        // Bound foreign traffic too; no unbounded drain under continuous events.
        for _ in 0..256 {
            match receiver.0.try_recv() {
                Ok(event)
                    if event.session_id.as_deref() == Some(&self.page_session)
                        && allowed_event(&event.method) =>
                {
                    bytes += event.params.to_string().len() + event.method.len() + 128;
                    if bytes > 1_000_000 {
                        overflow = true;
                        break;
                    }
                    receiver.1 += 1;
                    batch.push(
                        json!({"sequence":receiver.1,"method":event.method,"params":event.params}),
                    );
                }
                Ok(_) => {}
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => {
                    overflow = true;
                    break;
                }
            }
        }
        if self.sealed.load(Ordering::SeqCst) {
            return Err(INVALID.into());
        }
        if overflow {
            batch.clear();
        }
        let value = json!({"binding":binding,"requestId":request_id,"cursor":receiver.1,"overflow":overflow,"events":batch});
        let live_target = self.observe_target().await?;
        permit.publish(&value, &live_target).await?;
        if self.sealed.load(Ordering::SeqCst) {
            return Err(INVALID.into());
        }
        if overflow {
            self.sealed.store(true, Ordering::SeqCst);
        }
        cancellation.0 = None;
        Ok(BrokerOutput {
            value,
            wire_limit: permit.wire_limit(),
            _authority: Box::new(permit),
            _privacy,
        })
    }

    pub(crate) async fn detach(
        &self,
        binding: &BrokerBinding,
        request_id: &str,
    ) -> Result<BrokerOutput, String> {
        if binding != &self.binding {
            return Err(INVALID.into());
        }
        self.sealed.store(true, Ordering::SeqCst);
        let result = self
            .detached
            .get_or_init(|| async {
                // OnceCell retries a cancelled initializer; do not retry CDP.
                if self.detach_started.swap(true, Ordering::SeqCst) {
                    return Err(UNKNOWN.into());
                }
                let _authority = self.authority.admit(binding, "detach")?;
                let _privacy = self.client.public_lease()?;
                self.journal.admit(binding, request_id)?;
                self.client
                    .send_broker_command(
                        "Target.detachFromTarget",
                        Some(json!({"sessionId":self.page_session})),
                        None,
                        TIMEOUT,
                        None,
                    )
                    .await
                    .map_err(|_| UNKNOWN)?;
                Ok(())
            })
            .await;
        result.clone()?;
        let _authority = self.authority.admit(binding, "publish_detach")?;
        let _privacy = self.client.public_lease()?;
        Ok(BrokerOutput {
            value: json!({"binding":binding,"requestId":request_id,"detached":true,"browserPreserved":true}),
            wire_limit: None,
            _authority,
            _privacy,
        })
    }
}

fn allowed_method(method: &str) -> bool {
    matches!(
        method,
        "Runtime.enable"
            | "Runtime.disable"
            | "Runtime.evaluate"
            | "Runtime.callFunctionOn"
            | "Runtime.getProperties"
            | "Runtime.releaseObject"
            | "Runtime.releaseObjectGroup"
            | "Page.enable"
            | "Page.disable"
            | "Page.navigate"
            | "Page.reload"
            | "Page.bringToFront"
            | "Page.captureScreenshot"
            | "Page.handleJavaScriptDialog"
            | "Page.getFrameTree"
            | "DOM.enable"
            | "DOM.disable"
            | "DOM.getDocument"
            | "DOM.querySelector"
            | "DOM.querySelectorAll"
            | "DOM.describeNode"
            | "DOM.resolveNode"
            | "DOM.setFileInputFiles"
            | "Input.insertText"
            | "Input.dispatchKeyEvent"
            | "Input.dispatchMouseEvent"
            | "Network.enable"
            | "Network.disable"
            | "Network.getResponseBody"
    )
}

fn allowed_event(method: &str) -> bool {
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn event_wire_budget_covers_daemon_wrapper_and_newline_before_write() {
        let value = serde_json::json!({"events":[]});
        let framed = serde_json::json!({"id":"outer-id","success":true,"data":value});
        let size = serde_json::to_vec(&framed).unwrap().len() as u64 + 1;
        for (limit, allowed) in [(size - 1, false), (size, true)] {
            let output = super::BrokerOutput {
                value: value.clone(),
                wire_limit: Some(limit),
                _authority: Box::new(()),
                _privacy: None,
            };
            let mut bytes = Vec::new();
            let result = output.write_daemon_response("outer-id", &mut bytes).await;
            assert_eq!(result.is_ok(), allowed);
            if allowed {
                assert_eq!(bytes.len() as u64, size);
            } else {
                assert_eq!(result.unwrap_err(), "broker_event_evidence_budget_exceeded");
                assert!(bytes.is_empty());
            }
        }
    }

    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::sync::Mutex as StdMutex;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    struct Authority(AtomicBool, std::sync::atomic::AtomicUsize);
    impl BrokerAuthority for Authority {
        fn admit(&self, _: &BrokerBinding, operation: &str) -> Result<Box<dyn Send>, String> {
            if self.0.load(Ordering::SeqCst) || operation.contains("detach") {
                Ok(Box::new(()))
            } else {
                Err("custody_revoked".into())
            }
        }

        fn admit_command<'a>(
            &'a self,
            request: &'a BrokerCommandRequest,
            live: &'a Value,
        ) -> BrokerAdmissionFuture<'a> {
            Box::pin(async move {
                self.1.fetch_add(1, Ordering::SeqCst);
                if !self.0.load(Ordering::SeqCst)
                    || request.task_context["taskName"] != "fixture"
                    || request.binding != binding()
                    || request.request_id.is_empty()
                    || !allowed_method(&request.method)
                    || request.params.get("expression").and_then(Value::as_str) == Some("forbidden")
                    || live.pointer("/targetInfo/url").and_then(Value::as_str)
                        != Some("https://example.test/retained")
                {
                    return Err("command_authority_denied".into());
                }
                Ok(Box::new(Permit {
                    reject_outcome: request.task_context["rejectOutcome"] == true,
                }) as Box<dyn BrokerCommandPermit>)
            })
        }

        fn admit_events<'a>(
            &'a self,
            request: &'a BrokerEventRequest,
            live: &'a Value,
        ) -> BrokerAdmissionFuture<'a> {
            Box::pin(async move {
                if !self.0.load(Ordering::SeqCst)
                    || request.task_context["taskName"] != "fixture"
                    || request.binding != binding()
                    || request.request_id.is_empty()
                    || live.pointer("/targetInfo/url").and_then(Value::as_str)
                        != Some("https://example.test/retained")
                {
                    return Err("event_authority_denied".into());
                }
                Ok(Box::new(Permit {
                    reject_outcome: request.task_context["rejectOutcome"] == true,
                }) as Box<dyn BrokerCommandPermit>)
            })
        }
    }

    struct Permit {
        reject_outcome: bool,
    }
    impl BrokerCommandPermit for Permit {
        fn publish<'a>(&'a mut self, _: &'a Value, live: &'a Value) -> BrokerPublicationFuture<'a> {
            Box::pin(async move {
                if self.reject_outcome
                    || live.pointer("/targetInfo/url").and_then(Value::as_str)
                        != Some("https://example.test/retained")
                {
                    Err("command_outcome_denied".into())
                } else {
                    Ok(())
                }
            })
        }
    }

    // Fixtures explicitly supply a task envelope. Production has no overload
    // that drops command context or falls back to method-only authority.
    impl BrokerAttachment {
        async fn command(
            &self,
            binding: &BrokerBinding,
            request_id: &str,
            method: &str,
            params: Value,
        ) -> Result<BrokerOutput, String> {
            self.command_authorized(
                binding,
                request_id,
                method,
                params,
                json!({"taskName":"fixture"}),
            )
            .await
        }
    }

    struct Fixture {
        root: PathBuf,
        client: Arc<CdpClient>,
        authority: Arc<Authority>,
        commands: Arc<StdMutex<Vec<Value>>>,
        peer: tokio::task::JoinHandle<()>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.peer.abort();
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }
    fn binding() -> BrokerBinding {
        BrokerBinding {
            attachment_id: "attachment".into(),
            browser_id: "browser".into(),
            profile_id: "profile".into(),
            session_name: "session".into(),
            target_id: "target".into(),
            generation: "generation".into(),
        }
    }
    impl Fixture {
        async fn new(mode: &'static str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "broker-core-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let commands = Arc::new(StdMutex::new(Vec::new()));
            let observed = commands.clone();
            let peer = tokio::spawn(async move {
                let (socket, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
                let mut pending = None;
                let mut url_drifted = false;
                while let Some(Ok(Message::Text(text))) = ws.next().await {
                    let request: Value = serde_json::from_str(&text).unwrap();
                    observed.lock().unwrap().push(request.clone());
                    let method = request["method"].as_str().unwrap();
                    if mode == "url-drift" && method == "Runtime.evaluate" {
                        url_drifted = true;
                    }
                    if method == "Runtime.enable" && matches!(mode, "events" | "overflow") {
                        for session in ["foreign-session", "owned-session"] {
                            let params = if mode == "overflow" {
                                json!({"data":"x".repeat(1_000_001)})
                            } else {
                                json!({"requestId":"owned"})
                            };
                            ws.send(Message::Text(json!({"sessionId":session,"method":"Network.responseReceived","params":params}).to_string())).await.unwrap();
                        }
                    }
                    if method == "Runtime.evaluate" && mode == "dialog" {
                        pending = Some(request);
                        ws.send(Message::Text(json!({"sessionId":"owned-session", "method":"Page.javascriptDialogOpening", "params":{}}).to_string())).await.unwrap();
                        continue;
                    }
                    if method == "Target.detachFromTarget" && mode == "missing-detach" {
                        continue;
                    }
                    let result = match method {
                        "Target.attachToTarget" => json!({"sessionId":"owned-session"}),
                        "Target.getTargetInfo" => {
                            json!({"targetInfo":{"targetId":"target","type":"page","url":
                                if mode == "wrong-url" || url_drifted { "https://example.test/wrong" } else { "https://example.test/retained" }}})
                        }
                        _ => json!({}),
                    };
                    let mut reply = json!({"id":request["id"],"result":result});
                    if let Some(session) = request.get("sessionId") {
                        reply["sessionId"] = session.clone();
                    }
                    if (mode == "wrong-command" && method == "Runtime.enable")
                        || (mode == "wrong-detach" && method == "Target.detachFromTarget")
                    {
                        reply["sessionId"] = json!("foreign-session");
                    }
                    if ws.send(Message::Text(reply.to_string())).await.is_err() {
                        break;
                    }
                    if method == "Page.handleJavaScriptDialog" {
                        let request = pending.take().unwrap();
                        ws.send(Message::Text(
                            json!({"id":request["id"],"sessionId":"owned-session","result":{}})
                                .to_string(),
                        ))
                        .await
                        .unwrap();
                    }
                }
            });
            Self {
                root,
                client: Arc::new(
                    CdpClient::connect(&format!("ws://{address}"))
                        .await
                        .unwrap(),
                ),
                authority: Arc::new(Authority(
                    AtomicBool::new(true),
                    std::sync::atomic::AtomicUsize::new(0),
                )),
                commands,
                peer,
            }
        }
        async fn attach(&self) -> BrokerAttachment {
            BrokerAttachment::acquire(
                binding(),
                self.client.clone(),
                self.authority.clone(),
                BrokerJournal(self.root.clone()),
                "attach",
            )
            .await
            .unwrap()
        }
        fn count(&self, method: &str) -> usize {
            self.commands
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v["method"] == method)
                .count()
        }
    }

    #[tokio::test]
    async fn event_task_packet_and_publication_are_mandatory() {
        for context in [
            json!({}),
            json!({"taskName":"fixture", "rejectOutcome":true}),
        ] {
            let fixture = Fixture::new("normal").await;
            let attachment = fixture.attach().await;
            assert!(attachment
                .events_authorized(&binding(), "event-request", 0, context)
                .await
                .is_err());
            assert!(attachment.events(&binding(), 0).await.is_err());
            attachment.detach(&binding(), "cleanup").await.unwrap();
            assert_eq!(fixture.count("Target.detachFromTarget"), 1);
        }
    }

    #[tokio::test]
    async fn event_request_replay_cannot_reserve_or_read_twice() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        let output = attachment
            .events_authorized(
                &binding(),
                "event-request",
                0,
                json!({"taskName":"fixture"}),
            )
            .await
            .unwrap();
        output.write_json(&mut tokio::io::sink()).await.unwrap();
        let before = fixture.count("Target.getTargetInfo");
        assert!(attachment
            .events_authorized(
                &binding(),
                "event-request",
                0,
                json!({"taskName":"fixture"})
            )
            .await
            .is_err());
        assert_eq!(fixture.count("Target.getTargetInfo"), before);
        attachment.detach(&binding(), "cleanup").await.unwrap();
    }

    #[tokio::test]
    async fn dedicated_session_command_and_exact_idempotent_detach() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        let output = attachment
            .command(&binding(), "command", "Runtime.enable", json!({}))
            .await
            .unwrap();
        assert_eq!(output.value["binding"], json!(binding()));
        drop(output);
        let identity = binding();
        let (first, second) = tokio::join!(
            attachment.detach(&identity, "detach"),
            attachment.detach(&identity, "detach")
        );
        assert!(first.is_ok() && second.is_ok());
        assert_eq!(fixture.count("Target.attachToTarget"), 1);
        assert_eq!(fixture.count("Target.detachFromTarget"), 1);
        assert_eq!(fixture.count("Browser.close"), 0);
        assert_eq!(
            fixture
                .commands
                .lock()
                .unwrap()
                .iter()
                .find(|v| v["method"] == "Runtime.enable")
                .unwrap()["sessionId"],
            "owned-session"
        );
    }

    #[tokio::test]
    async fn durable_request_admission_survives_journal_reopen() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        attachment
            .command(
                &binding(),
                "user-turn",
                "Runtime.evaluate",
                json!({"expression":"1"}),
            )
            .await
            .unwrap();
        let reopened = BrokerJournal(fixture.root.clone());
        assert_eq!(
            reopened.admit(&binding(), "user-turn").unwrap_err(),
            UNKNOWN
        );
        assert!(attachment
            .command(
                &binding(),
                "user-turn",
                "Runtime.evaluate",
                json!({"expression":"2"})
            )
            .await
            .is_err());
        assert_eq!(fixture.count("Runtime.evaluate"), 1);
        assert_eq!(fixture.authority.1.load(Ordering::SeqCst), 1);
        attachment.detach(&binding(), "detach").await.unwrap();
    }

    #[tokio::test]
    async fn dialog_events_and_dismissal_do_not_wait_for_evaluation() {
        let fixture = Fixture::new("dialog").await;
        let attachment = fixture.attach().await;
        let identity = binding();
        let evaluate = attachment.command(
            &identity,
            "evaluate",
            "Runtime.evaluate",
            json!({"expression":"alert(1)"}),
        );
        let dismiss = async {
            loop {
                let events = attachment.events(&binding(), 0).await.unwrap();
                if !events.value["events"].as_array().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            attachment
                .command(
                    &binding(),
                    "dismiss",
                    "Page.handleJavaScriptDialog",
                    json!({"accept":false}),
                )
                .await
                .unwrap();
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            let (result, ()) = tokio::join!(evaluate, dismiss);
            result.unwrap();
        })
        .await
        .unwrap();
        attachment.detach(&binding(), "detach").await.unwrap();
        assert_eq!(fixture.count("Runtime.evaluate"), 1);
    }

    #[tokio::test]
    async fn concurrent_duplicate_reserves_authority_only_once() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        let identity = binding();
        let (first, second) = tokio::join!(
            attachment.command(&identity, "same-id", "Runtime.enable", json!({})),
            attachment.command(&identity, "same-id", "Runtime.enable", json!({}))
        );
        assert_ne!(first.is_ok(), second.is_ok());
        assert_eq!(fixture.authority.1.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.count("Runtime.enable"), 1);
        attachment.detach(&identity, "detach").await.unwrap();
    }

    #[tokio::test]
    async fn wrong_session_reply_seals_without_replay() {
        let fixture = Fixture::new("wrong-command").await;
        let attachment = fixture.attach().await;
        assert_eq!(
            attachment
                .command(&binding(), "one", "Runtime.enable", json!({}))
                .await
                .err()
                .unwrap(),
            UNKNOWN
        );
        assert!(attachment
            .command(&binding(), "two", "Runtime.enable", json!({}))
            .await
            .is_err());
        assert_eq!(fixture.count("Runtime.enable"), 1);
        attachment.detach(&binding(), "detach").await.unwrap();
    }

    #[tokio::test]
    async fn command_params_and_task_context_are_required_for_admission() {
        for (expression, context) in [
            ("1", json!({})),
            ("forbidden", json!({"taskName":"fixture"})),
        ] {
            let fixture = Fixture::new("normal").await;
            let attachment = fixture.attach().await;
            assert!(attachment
                .command_authorized(
                    &binding(),
                    "no-task",
                    "Runtime.evaluate",
                    json!({"expression":expression}),
                    context
                )
                .await
                .is_err());
            assert_eq!(fixture.count("Runtime.evaluate"), 0);
            assert_eq!(fixture.authority.1.load(Ordering::SeqCst), 1);
            attachment.detach(&binding(), "detach").await.unwrap();
        }
    }

    #[tokio::test]
    async fn failed_outcome_finalization_seals_without_publishing_or_replay() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        assert_eq!(
            attachment
                .command_authorized(
                    &binding(),
                    "one",
                    "Runtime.evaluate",
                    json!({"expression":"1"}),
                    json!({"taskName":"fixture", "rejectOutcome":true})
                )
                .await
                .err()
                .unwrap(),
            "command_outcome_denied"
        );
        assert!(attachment
            .command(
                &binding(),
                "two",
                "Runtime.evaluate",
                json!({"expression":"1"})
            )
            .await
            .is_err());
        assert_eq!(fixture.count("Runtime.evaluate"), 1);
        attachment.detach(&binding(), "detach").await.unwrap();
    }

    #[tokio::test]
    async fn live_url_is_checked_at_admission_and_publication() {
        for mode in ["wrong-url", "url-drift"] {
            let fixture = Fixture::new(mode).await;
            let attachment = fixture.attach().await;
            assert!(attachment
                .command(
                    &binding(),
                    "one",
                    "Runtime.evaluate",
                    json!({"expression":"1"})
                )
                .await
                .is_err());
            assert_eq!(
                fixture.count("Runtime.evaluate"),
                if mode == "wrong-url" { 0 } else { 1 }
            );
            if mode == "url-drift" {
                assert!(attachment
                    .command(
                        &binding(),
                        "two",
                        "Runtime.evaluate",
                        json!({"expression":"1"})
                    )
                    .await
                    .is_err());
                assert_eq!(fixture.count("Runtime.evaluate"), 1);
            }
            attachment.detach(&binding(), "detach").await.unwrap();
        }
    }

    #[tokio::test]
    async fn wrong_detach_acknowledgment_stays_failed() {
        let fixture = Fixture::new("wrong-detach").await;
        let attachment = fixture.attach().await;
        assert!(attachment.detach(&binding(), "one").await.is_err());
        assert!(attachment.detach(&binding(), "two").await.is_err());
        assert_eq!(fixture.count("Target.detachFromTarget"), 1);
    }

    #[tokio::test]
    async fn cancelled_detach_cannot_restart_with_a_new_id() {
        let fixture = Fixture::new("missing-detach").await;
        let attachment = fixture.attach().await;
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            attachment.detach(&binding(), "one")
        )
        .await
        .is_err());
        assert!(attachment.detach(&binding(), "two").await.is_err());
        assert_eq!(fixture.count("Target.detachFromTarget"), 1);
    }

    #[tokio::test]
    async fn cancelled_command_seals_new_ids_and_events() {
        let fixture = Fixture::new("dialog").await;
        let attachment = fixture.attach().await;
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            attachment.command(
                &binding(),
                "one",
                "Runtime.evaluate",
                json!({"expression":"alert(1)"})
            )
        )
        .await
        .is_err());
        assert!(attachment
            .command(&binding(), "two", "Runtime.enable", json!({}))
            .await
            .is_err());
        assert!(attachment.events(&binding(), 0).await.is_err());
        assert_eq!(fixture.count("Runtime.evaluate"), 1);
        assert_eq!(fixture.count("Runtime.enable"), 0);
        attachment.detach(&binding(), "detach").await.unwrap();
    }

    #[tokio::test]
    async fn filters_foreign_events_and_fails_closed_on_overflow() {
        for mode in ["events", "overflow"] {
            let fixture = Fixture::new(mode).await;
            let attachment = fixture.attach().await;
            attachment
                .command(&binding(), "enable", "Runtime.enable", json!({}))
                .await
                .unwrap();
            let output = attachment.events(&binding(), 0).await.unwrap();
            assert_eq!(output.value["overflow"], mode == "overflow");
            assert_eq!(
                output.value["events"].as_array().unwrap().len(),
                if mode == "overflow" { 0 } else { 1 }
            );
            drop(output);
            attachment.detach(&binding(), "detach").await.unwrap();
            fixture
                .client
                .send_command("Runtime.enable", None, None)
                .await
                .unwrap();
            assert_eq!(fixture.count("Browser.close"), 0);
        }
    }

    #[tokio::test]
    async fn rejects_foreign_binding_unsafe_methods_and_revoked_authority() {
        let fixture = Fixture::new("normal").await;
        let attachment = fixture.attach().await;
        let mut foreign = binding();
        foreign.generation = "old".into();
        assert!(attachment
            .command(&foreign, "one", "Runtime.enable", json!({}))
            .await
            .is_err());
        assert!(attachment
            .command(&binding(), "two", "Browser.close", json!({}))
            .await
            .is_err());
        assert!(attachment
            .command(
                &binding(),
                "three",
                "Runtime.enable",
                json!({"sessionId":"foreign"})
            )
            .await
            .is_err());
        fixture.authority.0.store(false, Ordering::SeqCst);
        assert!(attachment
            .command(&binding(), "four", "Runtime.enable", json!({}))
            .await
            .is_err());
        assert!(attachment.events(&binding(), 0).await.is_err());
        assert_eq!(fixture.count("Runtime.enable"), 0);
        attachment.detach(&binding(), "detach").await.unwrap();
    }
}
