use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

use super::types::{CdpCommand, CdpEvent, CdpMessage};
use crate::native::privacy_gate::{PrivacyGate, PrivacyObserver, PrivatePermit, PublicLease};
use crate::native::private_identity::{ValidatedPrivateJourney, ValidatedPrivateTarget};

type PendingMap = Arc<StdMutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>;

/// Private transport output must never enter ordinary command serialization.
/// Only a trusted in-process coordinator may consume the underlying value.
pub(crate) struct PrivateCdpResponse(Value);

/// Proof of one client transport's termination, not page cleanup or broker
/// detach. Never serializable or sufficient to reopen public observation.
pub(crate) struct PrivateTransportClosed {
    transport: WsTx,
    gate: Arc<PrivacyGate>,
    epoch: String,
    identity_digest: String,
}

impl PrivateTransportClosed {
    pub(crate) fn matches(&self, client: &CdpClient, permit: &PrivatePermit) -> bool {
        Arc::ptr_eq(&self.transport, &client.ws_tx)
            && permit.authorizes(&self.gate)
            && permit.epoch_id() == self.epoch
            && permit.identity_digest() == Some(self.identity_digest.as_str())
    }
}

impl PrivateCdpResponse {
    pub(crate) fn into_value(self) -> Value {
        self.0
    }
}

fn lock_pending(
    pending: &PendingMap,
) -> std::sync::MutexGuard<'_, HashMap<u64, oneshot::Sender<CdpMessage>>> {
    pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct PendingCommandRegistration {
    id: u64,
    pending: PendingMap,
    active: bool,
}

impl PendingCommandRegistration {
    fn insert(id: u64, sender: oneshot::Sender<CdpMessage>, pending: PendingMap) -> Self {
        lock_pending(&pending).insert(id, sender);
        Self {
            id,
            pending,
            active: true,
        }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for PendingCommandRegistration {
    fn drop(&mut self) {
        if self.active {
            lock_pending(&self.pending).remove(&self.id);
        }
    }
}

const DEFAULT_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Stable command-lifecycle failures for callers that need more than the
/// compatibility string returned by `send_command`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdpCommandError {
    Privacy {
        reason: &'static str,
    },
    Serialization {
        method: String,
        message: String,
    },
    Transport {
        method: String,
        message: String,
    },
    ResponseChannelClosed {
        method: String,
    },
    Timeout {
        method: String,
        timeout: std::time::Duration,
    },
    Protocol {
        method: String,
        message: String,
    },
}

impl std::fmt::Display for CdpCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Privacy { reason } => formatter.write_str(reason),
            Self::Serialization { message, .. } => {
                write!(formatter, "Failed to serialize CDP command: {message}")
            }
            Self::Transport { message, .. } => {
                write!(formatter, "Failed to send CDP command: {message}")
            }
            Self::ResponseChannelClosed { .. } => {
                write!(formatter, "CDP response channel closed")
            }
            Self::Timeout { method, .. } => {
                write!(formatter, "CDP command timed out: {method}")
            }
            Self::Protocol { method, message } => {
                write!(formatter, "CDP error ({method}): {message}")
            }
        }
    }
}

impl std::error::Error for CdpCommandError {}

/// Interval between WebSocket ping frames sent to keep the connection alive
/// through intermediate proxies (reverse proxies, load balancers, service meshes).
const WS_KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Raw incoming CDP message (text) broadcast to all subscribers.
/// Used by the inspect proxy to forward responses and events to DevTools.
#[derive(Debug, Clone)]
pub struct RawCdpMessage {
    pub text: String,
    pub session_id: Option<String>,
}

pub struct CdpClient {
    privacy_gate: Option<Arc<PrivacyGate>>,
    privacy_observer: Option<PrivacyObserver>,
    ws_tx: WsTx,
    transport_sealed: Arc<AtomicBool>,
    next_id: AtomicU64,
    pending: PendingMap,
    event_tx: broadcast::Sender<CdpEvent>,
    raw_tx: broadcast::Sender<RawCdpMessage>,
    shutdown: Mutex<TransportShutdown>,
}

struct TransportShutdown {
    reader: Option<tokio::task::JoinHandle<()>>,
    keepalive: Option<tokio::task::JoinHandle<()>>,
    binding: Option<(String, String)>,
    failed: bool,
}

impl CdpClient {
    pub async fn connect(url: &str) -> Result<Self, String> {
        Self::connect_with_headers(url, None).await
    }

    pub async fn connect_with_headers(
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<Self, String> {
        Self::connect_inner(url, headers, None).await
    }

    /// Internal recovery bootstrap; ordinary callers cannot manufacture a
    /// bound permit. Observers remain gated before the reader starts.
    pub(crate) async fn connect_private_recovery(
        url: &str,
        permit: &PrivatePermit,
    ) -> Result<Self, String> {
        Self::connect_inner(url, None, Some(permit)).await
    }

    async fn connect_inner(
        url: &str,
        headers: Option<Vec<(String, String)>>,
        recovery: Option<&PrivatePermit>,
    ) -> Result<Self, String> {
        // Unsupported platforms keep ordinary CDP support, but cannot admit a
        // private interval. On supported platforms all same-endpoint clients
        // share the persistent gate before any reader or observer starts.
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let privacy_gate = Some(
            match recovery {
                Some(permit) => permit.recovery_gate(url),
                None => PrivacyGate::for_endpoint(url),
            }
            .map_err(str::to_string)?,
        );
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let privacy_gate: Option<Arc<PrivacyGate>> = None;
        Self::connect_with_gate_inner(url, headers, recovery, privacy_gate).await
    }

    async fn connect_with_gate_inner(
        url: &str,
        headers: Option<Vec<(String, String)>>,
        recovery: Option<&PrivatePermit>,
        privacy_gate: Option<Arc<PrivacyGate>>,
    ) -> Result<Self, String> {
        let _connection_lease = if let Some(permit) = recovery {
            if !privacy_gate
                .as_ref()
                .is_some_and(|gate| permit.authorizes(gate))
            {
                return Err("private_permit_mismatch".into());
            }
            None
        } else {
            privacy_gate
                .as_ref()
                .map(|gate| gate.public_lease())
                .transpose()
                .map_err(str::to_string)?
        };
        // Capture before opening the socket while the connection lease excludes
        // private admission. Recovery sockets never become public observers.
        let privacy_observer = privacy_gate
            .as_ref()
            .map(|gate| {
                if recovery.is_some() {
                    Ok(gate.private_observer())
                } else {
                    gate.public_observer()
                }
            })
            .transpose()
            .map_err(str::to_string)?;
        let mut request = url
            .into_client_request()
            .map_err(|e| format!("Invalid WebSocket URL: {}", e))?;

        if let Some(hdrs) = headers {
            let req_headers = request.headers_mut();
            for (key, value) in hdrs {
                if let (Ok(name), Ok(val)) = (
                    key.parse::<tokio_tungstenite::tungstenite::http::header::HeaderName>(),
                    value.parse::<tokio_tungstenite::tungstenite::http::header::HeaderValue>(),
                ) {
                    req_headers.insert(name, val);
                }
            }
        }

        let ws_config = WebSocketConfig {
            max_message_size: None,
            max_frame_size: None,
            ..Default::default()
        };

        let (ws_stream, _) =
            tokio_tungstenite::connect_async_with_config(request, Some(ws_config), false)
                .await
                .map_err(|e| format!("CDP WebSocket connect failed: {}", e))?;

        enable_tcp_keepalive(ws_stream.get_ref());

        let (ws_tx, mut ws_rx) = ws_stream.split();
        let ws_tx = Arc::new(Mutex::new(Some(ws_tx)));
        let transport_sealed = Arc::new(AtomicBool::new(false));

        let pending: PendingMap = Arc::new(StdMutex::new(HashMap::new()));
        let (event_tx, _) = broadcast::channel(4096);
        let (raw_tx, _) = broadcast::channel(4096);

        let pending_clone = pending.clone();
        let event_tx_clone = event_tx.clone();
        let raw_tx_clone = raw_tx.clone();
        let reader_privacy_observer = privacy_observer.clone();

        // Notify used to stop the keepalive task when the reader loop exits.
        let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);

        let reader_handle = tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                // Accept both Text and Binary frames — remote CDP proxies
                // (e.g. Browserless) may send responses as Binary frames.
                let msg = match msg {
                    Ok(Message::Text(text)) => text,
                    Ok(Message::Binary(data)) => match String::from_utf8(data) {
                        Ok(text) => text,
                        Err(_) => continue,
                    },
                    Ok(Message::Close(frame)) => {
                        let debug_lease = reader_privacy_observer
                            .as_ref()
                            .map(|gate| gate.public_lease())
                            .transpose();
                        if std::env::var("AGENT_BROWSER_DEBUG").is_ok() && debug_lease.is_ok() {
                            let reason = frame
                                .as_ref()
                                .map(|f| format!("code={}, reason={}", f.code, f.reason))
                                .unwrap_or_else(|| "no frame".to_string());
                            let _ =
                                writeln!(std::io::stderr(), "[cdp] WebSocket Close: {}", reason);
                        }
                        break;
                    }
                    Ok(Message::Pong(_)) => continue,
                    Ok(_) => continue,
                    Err(e) => {
                        let debug_lease = reader_privacy_observer
                            .as_ref()
                            .map(|gate| gate.public_lease())
                            .transpose();
                        if std::env::var("AGENT_BROWSER_DEBUG").is_ok() && debug_lease.is_ok() {
                            let _ = writeln!(std::io::stderr(), "[cdp] WebSocket Error: {}", e);
                        }
                        break;
                    }
                };

                // Broadcast raw message for inspect proxy subscribers before typed parse,
                // so messages with negative IDs (used by the inspect proxy) are still delivered.
                let observation_lease = reader_privacy_observer
                    .as_ref()
                    .map(|gate| gate.public_lease())
                    .transpose();
                let observable = observation_lease.is_ok();
                if observable && raw_tx_clone.receiver_count() > 0 {
                    let session_id = serde_json::from_str::<serde_json::Value>(&msg)
                        .ok()
                        .and_then(|v| v.get("sessionId")?.as_str().map(String::from));
                    let _ = raw_tx_clone.send(RawCdpMessage {
                        text: msg.clone(),
                        session_id,
                    });
                }

                let parsed: CdpMessage = match serde_json::from_str(&msg) {
                    Ok(m) => m,
                    // Expected for inspect proxy messages with negative IDs
                    // (CdpMessage.id is u64); handled via raw broadcast above.
                    Err(_) => continue,
                };

                if let Some(id) = parsed.id {
                    // Response to a command
                    let mut pending = lock_pending(&pending_clone);
                    if let Some(tx) = pending.remove(&id) {
                        let _ = tx.send(parsed);
                    }
                } else if let Some(ref method) = parsed.method {
                    if !observable {
                        continue;
                    }
                    // Event
                    let event = CdpEvent {
                        method: method.clone(),
                        params: parsed.params.clone().unwrap_or(Value::Null),
                        session_id: parsed.session_id.clone(),
                    };
                    let _ = event_tx_clone.send(event);
                }
            }

            // Reader loop exited (connection closed or error). Drop all pending
            // command senders so callers get an immediate channel-closed error
            // instead of waiting for the 30-second timeout.
            lock_pending(&pending_clone).clear();

            // Stop the keepalive task — the connection is gone.
            let _ = cancel_tx.send(true);
        });

        // Spawn a keepalive task that sends WebSocket Ping frames at a regular
        // interval. This prevents intermediate proxies (Envoy, nginx, OpenResty,
        // cloud load balancers) from closing idle WebSocket connections. If the
        // send fails, the connection is dead and we stop pinging.
        let keepalive_tx = ws_tx.clone();
        let keepalive_sealed = Arc::clone(&transport_sealed);
        let keepalive_handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(WS_KEEPALIVE_INTERVAL_SECS);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = cancel_rx.changed() => break,
                }
                let mut tx = keepalive_tx.lock().await;
                if keepalive_sealed.load(Ordering::SeqCst) {
                    break;
                }
                let Some(tx) = tx.as_mut() else {
                    break;
                };
                if tx.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            privacy_gate,
            privacy_observer,
            ws_tx,
            transport_sealed,
            next_id: AtomicU64::new(1),
            pending,
            event_tx,
            raw_tx,
            shutdown: Mutex::new(TransportShutdown {
                reader: Some(reader_handle),
                keepalive: Some(keepalive_handle),
                binding: None,
                failed: false,
            }),
        })
    }

    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command_with_timeout(method, params, session_id, DEFAULT_COMMAND_TIMEOUT)
            .await
            .map_err(|error| error.to_string())
    }

    /// Sends one CDP command with a caller-selected transport deadline.
    ///
    /// The pending response registration is removed on response, timeout,
    /// transport failure, channel closure, or external future cancellation.
    pub async fn send_command_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<Value, CdpCommandError> {
        let _privacy_lease = self
            .public_lease()
            .map_err(|reason| CdpCommandError::Privacy { reason })?;
        // Write before dispatch, not in Drop: process death must not forget a
        // command that Chrome may still execute. Cancellation leaves it pending.
        let mut command_lease = self
            .privacy_gate
            .as_ref()
            .map(|gate| gate.command_lease())
            .transpose()
            .map_err(|reason| CdpCommandError::Privacy { reason })?;
        let result = self
            .send_command_inner(method, params, session_id, timeout)
            .await;
        if matches!(&result, Ok(_) | Err(CdpCommandError::Protocol { .. })) {
            if let Some(lease) = &mut command_lease {
                lease
                    .complete()
                    .map_err(|reason| CdpCommandError::Privacy { reason })?;
            }
        }
        result
    }

    /// Internal private transport only. No browser command, MCP request or
    /// ordinary evaluate surface can supply a permit. The coordinator must
    /// validate live target identity and durable operation admission separately.
    pub(crate) fn private_permit_matches(&self, permit: &PrivatePermit) -> bool {
        self.privacy_gate
            .as_ref()
            .is_some_and(|gate| permit.authorizes(gate))
            && permit.target_id().is_some()
            && permit.identity_digest().is_some()
            && !self.transport_sealed.load(Ordering::SeqCst)
    }

    pub(crate) async fn send_private_command(
        &self,
        permit: &PrivatePermit,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<PrivateCdpResponse, &'static str> {
        let gate = self
            .privacy_gate
            .as_ref()
            .ok_or("private_gate_unavailable")?;
        if !permit.authorizes(gate) {
            return Err("private_permit_mismatch");
        }
        let target_id = permit.target_id().ok_or("private_scope_missing")?;
        if permit.identity_digest().is_none() {
            return Err("private_scope_missing");
        }
        if !matches!(
            method,
            "Target.getTargetInfo"
                | "Target.attachToTarget"
                | "Target.detachFromTarget"
                | "Page.getFrameTree"
                | "Page.createIsolatedWorld"
                | "Page.navigate"
                | "Page.stopLoading"
                | "Runtime.evaluate"
        ) || timeout.is_zero()
            || timeout > std::time::Duration::from_secs(30)
        {
            return Err("private_cdp_operation_invalid");
        }
        // A scoped permit cannot authorize a sibling target at this endpoint.
        // CDP's session target echo is checked before every mutating command;
        // getTargetInfo must omit targetId here so it cannot mask a wrong session.
        let scoped = async {
            let probe_session = match method {
                "Target.attachToTarget" => {
                    if session_id.is_some()
                        || params
                            .as_ref()
                            .and_then(|v| v.get("targetId"))
                            .and_then(Value::as_str)
                            != Some(target_id)
                        || params
                            .as_ref()
                            .and_then(|v| v.get("flatten"))
                            .and_then(Value::as_bool)
                            != Some(true)
                    {
                        return Err("private_scope_mismatch");
                    }
                    None
                }
                "Target.getTargetInfo" => {
                    if params
                        .as_ref()
                        .and_then(|v| v.get("targetId"))
                        .is_some_and(|v| v.as_str() != Some(target_id))
                        || (session_id.is_none()
                            && params
                                .as_ref()
                                .and_then(|v| v.get("targetId"))
                                .and_then(Value::as_str)
                                != Some(target_id))
                    {
                        return Err("private_scope_mismatch");
                    }
                    None
                }
                "Target.detachFromTarget" => {
                    if session_id.is_some()
                        || params.as_ref().and_then(|v| v.get("targetId")).is_some()
                    {
                        return Err("private_scope_mismatch");
                    }
                    Some(
                        params
                            .as_ref()
                            .and_then(|v| v.get("sessionId"))
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .ok_or("private_scope_mismatch")?,
                    )
                }
                _ => Some(
                    session_id
                        .filter(|s| !s.is_empty())
                        .ok_or("private_scope_mismatch")?,
                ),
            };
            if let Some(session) = probe_session {
                let info = self
                    .send_command_inner("Target.getTargetInfo", None, Some(session), timeout)
                    .await
                    .map_err(|_| "private_cdp_operation_failed")?;
                if info.pointer("/targetInfo/targetId").and_then(Value::as_str) != Some(target_id)
                    || info.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
                {
                    return Err("private_scope_mismatch");
                }
            }
            let result = self
                .send_command_inner(method, params, session_id, timeout)
                .await
                .map_err(|_| "private_cdp_operation_failed")?;
            if method == "Target.getTargetInfo"
                && (result
                    .pointer("/targetInfo/targetId")
                    .and_then(Value::as_str)
                    != Some(target_id)
                    || result.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page"))
            {
                return Err("private_scope_mismatch");
            }
            Ok(PrivateCdpResponse(result))
        };
        tokio::time::timeout(timeout, scoped)
            .await
            .map_err(|_| "private_cdp_operation_failed")?
    }

    // Caller owns either a public command lease or a bound private permit.
    async fn send_command_inner(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<Value, CdpCommandError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let method_name = method.to_string();

        let cmd = CdpCommand {
            id,
            method: method_name.clone(),
            params,
            session_id: session_id.filter(|s| !s.is_empty()).map(|s| s.to_string()),
        };

        let json = serde_json::to_string(&cmd).map_err(|error| CdpCommandError::Serialization {
            method: method_name.clone(),
            message: error.to_string(),
        })?;

        let (tx, rx) = oneshot::channel();

        let mut registration = {
            let mut ws_tx = self.ws_tx.lock().await;
            if self.transport_sealed.load(Ordering::SeqCst) {
                return Err(CdpCommandError::Privacy {
                    reason: "cdp_transport_closed",
                });
            }
            let ws_tx = ws_tx.as_mut().ok_or(CdpCommandError::Privacy {
                reason: "cdp_transport_closed",
            })?;
            // Sealing and registration share this lock. No pending entry can
            // appear after shutdown has removed the sink and cleared the map.
            let registration = PendingCommandRegistration::insert(id, tx, self.pending.clone());
            if let Err(error) = ws_tx.send(Message::Text(json)).await {
                return Err(CdpCommandError::Transport {
                    method: method_name,
                    message: error.to_string(),
                });
            }
            registration
        };

        let response = match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(resp)) => {
                registration.disarm();
                resp
            }
            Ok(Err(_)) => {
                return Err(CdpCommandError::ResponseChannelClosed {
                    method: method_name,
                })
            }
            Err(_) => {
                return Err(CdpCommandError::Timeout {
                    method: method_name,
                    timeout,
                });
            }
        };

        if let Some(error) = response.error {
            return Err(CdpCommandError::Protocol {
                method: method_name,
                message: error.to_string(),
            });
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    /// Held through observer output or a full ordinary command. Admission cannot
    /// race a caller holding this lease, including callers in another process.
    pub fn public_lease(&self) -> Result<Option<PublicLease>, &'static str> {
        self.privacy_observer
            .as_ref()
            .map(|gate| gate.public_lease())
            .transpose()
    }

    /// Internal fail-closed admission primitive, not a public browser command.
    /// There is deliberately no unlock or secret-submit API until verified page
    /// sanitization, exact identity and private result transport are integrated.
    pub(crate) fn begin_private_interval(
        &self,
        target: &ValidatedPrivateTarget,
    ) -> Result<PrivatePermit, &'static str> {
        self.privacy_gate
            .as_ref()
            .ok_or("private_gate_unavailable")?
            .begin_private_scoped(target.target_id(), target.scope_digest())
    }

    pub(crate) fn begin_private_journey(
        &self,
        journey: &ValidatedPrivateJourney,
    ) -> Result<PrivatePermit, &'static str> {
        self.privacy_gate
            .as_ref()
            .ok_or("private_gate_unavailable")?
            .begin_private_journey(journey)
    }

    /// Permanently close this transport under its private permit. The sink is
    /// removed from shared state, including retained inspect handles, before
    /// tasks are aborted and joined. Cancellation retains join handles in self;
    /// repeating this cleanup can finish joins but can never reopen the socket.
    /// This neither detaches broker authority nor releases the privacy epoch.
    pub(crate) async fn quiesce_private_transport(
        &self,
        permit: &PrivatePermit,
    ) -> Result<PrivateTransportClosed, &'static str> {
        let gate = self
            .privacy_gate
            .as_ref()
            .ok_or("private_gate_unavailable")?;
        if !permit.authorizes(gate) || permit.target_id().is_none() {
            return Err("private_permit_mismatch");
        }
        let digest = permit.identity_digest().ok_or("private_scope_missing")?;
        let binding = (permit.epoch_id().to_owned(), digest.to_owned());
        let close = async {
            let mut shutdown = self.shutdown.lock().await;
            if shutdown.binding.as_ref().is_some_and(|old| old != &binding) {
                return Err("private_shutdown_scope_mismatch");
            }
            if shutdown.failed {
                return Err("private_transport_join_failed");
            }
            shutdown.binding = Some(binding.clone());
            self.transport_sealed.store(true, Ordering::SeqCst);
            // Lock contention is bounded by the outer timeout. Until this
            // succeeds no closure evidence is returned and privacy stays closed.
            self.ws_tx.lock().await.take();
            if let Some(handle) = &shutdown.reader {
                handle.abort();
            }
            if let Some(handle) = &shutdown.keepalive {
                handle.abort();
            }
            let TransportShutdown {
                reader,
                keepalive,
                failed,
                ..
            } = &mut *shutdown;
            for slot in [reader, keepalive] {
                if let Some(handle) = slot.as_mut() {
                    let joined = handle.await;
                    // Keep ownership until join resolves; an interrupted await
                    // must not detach the worker and masquerade as completion.
                    *slot = None;
                    if joined.is_err_and(|error| !error.is_cancelled()) {
                        *failed = true;
                        return Err("private_transport_join_failed");
                    }
                }
            }
            lock_pending(&self.pending).clear();
            Ok(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(3), close)
            .await
            .map_err(|_| "private_transport_shutdown_timeout")??;
        Ok(PrivateTransportClosed {
            transport: Arc::clone(&self.ws_tx),
            gate: Arc::clone(gate),
            epoch: binding.0,
            identity_digest: binding.1,
        })
    }

    /// Verify a new empty top-level document without releasing observation.
    /// This is only page cleanup, not transport-queue or mutation-scope proof.
    /// A future coordinator must reconcile both before finishing the epoch.
    pub(crate) async fn sanitize_private_page(
        &self,
        permit: &mut PrivatePermit,
        target_id: &str,
        session_id: &str,
    ) -> Result<(), &'static str> {
        if target_id.is_empty() || session_id.is_empty() || permit.target_id() != Some(target_id) {
            return Err("private_cleanup_identity_mismatch");
        }
        let verify = |value: &Value, blank: bool| {
            value
                .pointer("/targetInfo/targetId")
                .and_then(Value::as_str)
                == Some(target_id)
                && value.pointer("/targetInfo/type").and_then(Value::as_str) == Some("page")
                && (!blank
                    || value.pointer("/targetInfo/url").and_then(Value::as_str)
                        == Some("about:blank"))
        };
        let short = std::time::Duration::from_secs(1);
        let cleanup = async {
            // Omitting targetId binds this query to the supplied CDP session.
            // Never trust a caller's session string without this browser echo.
            let info = self
                .send_private_command(
                    permit,
                    "Target.getTargetInfo",
                    None,
                    Some(session_id),
                    short,
                )
                .await?
                .into_value();
            if !verify(&info, false) {
                return Err("private_cleanup_identity_mismatch");
            }
            self.send_private_command(permit, "Page.stopLoading", None, Some(session_id), short)
                .await?;
            let navigated = self
                .send_private_command(
                    permit,
                    "Page.navigate",
                    Some(serde_json::json!({"url":"about:blank"})),
                    Some(session_id),
                    short,
                )
                .await?
                .into_value();
            if navigated.get("errorText").is_some() {
                return Err("private_cleanup_failed");
            }
            for _ in 0..12 {
                let tree = self
                    .send_private_command(
                        permit,
                        "Page.getFrameTree",
                        None,
                        Some(session_id),
                        short,
                    )
                    .await?
                    .into_value();
                let frame = tree
                    .pointer("/frameTree/frame")
                    .ok_or("private_cleanup_failed")?;
                if frame.get("url").and_then(Value::as_str) != Some("about:blank") {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
                if frame.get("parentId").is_some()
                    || tree
                        .pointer("/frameTree/childFrames")
                        .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                {
                    return Err("private_cleanup_failed");
                }
                let frame_id = frame
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or("private_cleanup_failed")?;
                let world = self.send_private_command(permit, "Page.createIsolatedWorld", Some(serde_json::json!({"frameId":frame_id,"worldName":"agent-browser-private-cleanup"})), Some(session_id), short).await?.into_value();
                let context = world
                    .get("executionContextId")
                    .and_then(Value::as_i64)
                    .ok_or("private_cleanup_failed")?;
                let proof = self.send_private_command(permit, "Runtime.evaluate", Some(serde_json::json!({
                    "contextId":context,"returnByValue":true,
                    "expression":"globalThis === globalThis.top && location.href === 'about:blank' && document.readyState === 'complete' && document.body !== null && document.body.childElementCount === 0 && document.body.textContent === ''"
                })), Some(session_id), short).await?.into_value();
                if proof.get("exceptionDetails").is_some()
                    || proof.pointer("/result/value").and_then(Value::as_bool) != Some(true)
                {
                    return Err("private_cleanup_failed");
                }
                let info = self
                    .send_private_command(
                        permit,
                        "Target.getTargetInfo",
                        None,
                        Some(session_id),
                        short,
                    )
                    .await?
                    .into_value();
                if !verify(&info, true) {
                    return Err("private_cleanup_identity_mismatch");
                }
                return Ok(());
            }
            Err("private_cleanup_failed")
        };
        tokio::time::timeout(std::time::Duration::from_secs(15), cleanup)
            .await
            .map_err(|_| "private_cleanup_failed")??;
        Ok(())
    }

    #[cfg(test)]
    async fn pending_command_count(&self) -> usize {
        lock_pending(&self.pending).len()
    }

    /// Subscribe to all raw incoming CDP messages (responses + events).
    /// Used by the inspect proxy to forward traffic to the DevTools frontend.
    pub fn subscribe_raw(&self) -> broadcast::Receiver<RawCdpMessage> {
        self.raw_tx.subscribe()
    }

    /// Create a lightweight handle for the inspect WebSocket proxy.
    /// Contains only what's needed to forward messages bidirectionally.
    pub fn inspect_handle(&self) -> InspectProxyHandle {
        InspectProxyHandle {
            ws_tx: self.ws_tx.clone(),
            transport_sealed: Arc::clone(&self.transport_sealed),
            raw_tx: self.raw_tx.clone(),
            privacy_gate: self.privacy_gate.clone(),
            privacy_observer: self.privacy_observer.clone(),
        }
    }

    pub async fn send_command_typed<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        session_id: Option<&str>,
    ) -> Result<R, String> {
        let params_value = serde_json::to_value(params)
            .map_err(|e| format!("Failed to serialize params: {}", e))?;
        let result = self
            .send_command(method, Some(params_value), session_id)
            .await?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to deserialize CDP response for {}: {}", method, e))
    }

    pub async fn send_command_no_params(
        &self,
        method: &str,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command(method, None, session_id).await
    }

    /// Send raw JSON through the WebSocket without tracking a response.
    /// Used by the inspect proxy to forward DevTools frontend messages.
    pub async fn send_raw(&self, json: String) -> Result<(), String> {
        let _privacy_lease = self.public_lease().map_err(str::to_string)?;
        if let Some(gate) = &self.privacy_gate {
            // Raw protocol traffic has no bounded completion receipt.
            gate.mark_uncertain().map_err(str::to_string)?;
        }
        let mut ws_tx = self.ws_tx.lock().await;
        if self.transport_sealed.load(Ordering::SeqCst) {
            return Err("cdp_transport_closed".into());
        }
        let ws_tx = ws_tx.as_mut().ok_or("cdp_transport_closed")?;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send raw CDP message: {}", e))
    }
}

impl Drop for CdpClient {
    fn drop(&mut self) {
        // JoinHandle::drop detaches a task. Abort both background tasks so a
        // short-lived client also releases its WebSocket and keepalive state.
        let shutdown = self.shutdown.get_mut();
        if let Some(handle) = &shutdown.reader {
            handle.abort();
        }
        if let Some(handle) = &shutdown.keepalive {
            handle.abort();
        }
    }
}

type WsTx = Arc<
    Mutex<
        Option<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
>;

/// Lightweight handle for the inspect WebSocket proxy, holding only
/// the cloneable parts of CdpClient needed for bidirectional message forwarding.
pub struct InspectProxyHandle {
    ws_tx: WsTx,
    transport_sealed: Arc<AtomicBool>,
    raw_tx: broadcast::Sender<RawCdpMessage>,
    privacy_gate: Option<Arc<PrivacyGate>>,
    privacy_observer: Option<PrivacyObserver>,
}

impl InspectProxyHandle {
    pub async fn send_raw(&self, json: String) -> Result<(), String> {
        let _privacy_lease = self.public_lease().map_err(str::to_string)?;
        if let Some(gate) = &self.privacy_gate {
            gate.mark_uncertain().map_err(str::to_string)?;
        }
        let mut ws_tx = self.ws_tx.lock().await;
        if self.transport_sealed.load(Ordering::SeqCst) {
            return Err("cdp_transport_closed".into());
        }
        let ws_tx = ws_tx.as_mut().ok_or("cdp_transport_closed")?;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send raw CDP message: {}", e))
    }

    pub fn subscribe_raw(&self) -> broadcast::Receiver<RawCdpMessage> {
        self.raw_tx.subscribe()
    }

    pub fn public_lease(&self) -> Result<Option<PublicLease>, &'static str> {
        self.privacy_observer
            .as_ref()
            .map(|gate| gate.public_lease())
            .transpose()
    }
}

/// Enable TCP SO_KEEPALIVE on the underlying socket of a WebSocket connection.
/// This is best-effort: failures are silently ignored since the WebSocket-level
/// Ping keepalive provides the primary connection liveness mechanism.
fn enable_tcp_keepalive(stream: &tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>) {
    let tcp_stream = match stream {
        tokio_tungstenite::MaybeTlsStream::Plain(s) => s,
        tokio_tungstenite::MaybeTlsStream::Rustls(s) => s.get_ref().0,
        _ => return,
    };

    // SockRef borrows the fd without taking ownership.
    let sock = socket2::SockRef::from(tcp_stream);
    let keepalive = socket2::TcpKeepalive::new().with_time(std::time::Duration::from_secs(30));

    // with_interval sets TCP_KEEPINTVL — the time between probes after the
    // first keepalive probe goes unanswered. Available on most platforms
    // (Linux, macOS, Windows, FreeBSD, etc.) but not OpenBSD or Haiku.
    #[cfg(not(any(target_os = "openbsd", target_os = "haiku")))]
    let keepalive = keepalive.with_interval(std::time::Duration::from_secs(10));

    let _ = sock.set_tcp_keepalive(&keepalive);
}

/// Isolated synthetic endpoint. Keep this owner alive until its clients,
/// permits, and peer tasks have ended. Sibling connections share its real gate;
/// distinct fixtures never share durable state, even if the OS reuses a port.
#[cfg(test)]
pub(crate) struct TestCdpEndpoint {
    endpoint: String,
    root: std::path::PathBuf,
    gate: Option<Arc<PrivacyGate>>,
}

#[cfg(test)]
impl TestCdpEndpoint {
    pub(crate) fn new(endpoint: &str) -> Result<Self, String> {
        let root = std::env::temp_dir().join(format!("ab-cdp-gate-{}", uuid::Uuid::new_v4()));
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let gate = Some(PrivacyGate::open_test_endpoint(&root, endpoint).map_err(str::to_string)?);
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let gate = None;
        Ok(Self {
            endpoint: endpoint.to_owned(),
            root,
            gate,
        })
    }

    pub(crate) async fn connect(&self) -> Result<CdpClient, String> {
        CdpClient::connect_with_gate_inner(&self.endpoint, None, None, self.gate.clone()).await
    }

    #[cfg(unix)]
    pub(crate) fn gate(&self) -> &Arc<PrivacyGate> {
        self.gate.as_ref().expect("private gate supported")
    }
}

#[cfg(test)]
impl Drop for TestCdpEndpoint {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    use super::{CdpClient, CdpCommandError, TestCdpEndpoint};

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn synthetic_endpoint_gates_isolate_reused_urls_but_share_sibling_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let first_fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let isolated_fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let mut sockets = Vec::new();
            for _ in 0..3 {
                let (socket, _) = listener.accept().await.unwrap();
                sockets.push(tokio_tungstenite::accept_async(socket).await.unwrap());
            }
            done_rx.await.unwrap();
            drop(sockets);
        });
        let first = first_fixture.connect().await.unwrap();
        let sibling = first_fixture.connect().await.unwrap();
        let permit = first.begin_private_interval(&approved_target()).unwrap();
        assert!(sibling.public_lease().is_err());
        drop(permit);
        assert!(
            matches!(first_fixture.connect().await, Err(reason) if reason == "privacy_gate_locked")
        );
        let isolated = isolated_fixture.connect().await.unwrap();
        assert!(isolated.public_lease().is_ok());
        let isolated_permit = isolated.begin_private_interval(&approved_target()).unwrap();
        drop(isolated_permit);
        assert!(
            matches!(isolated_fixture.connect().await, Err(reason) if reason == "privacy_gate_locked")
        );
        drop(first);
        drop(sibling);
        drop(isolated);
        done_tx.send(()).unwrap();
        peer.await.unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn idle_private_peer() -> (TestCdpEndpoint, CdpClient, tokio::task::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let mut commands = 0;
            while let Some(Ok(message)) = ws.next().await {
                if message.is_text() {
                    commands += 1;
                }
            }
            commands
        });
        let client = fixture.connect().await.unwrap();
        (fixture, client, peer)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_closes_socket_despite_retained_inspector_and_is_idempotent() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let inspect = client.inspect_handle();
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        let (pending_tx, pending_rx) = tokio::sync::oneshot::channel();
        super::lock_pending(&client.pending).insert(42, pending_tx);
        let proof = client.quiesce_private_transport(&permit).await.unwrap();
        assert!(proof.matches(&client, &permit));
        assert!(pending_rx.await.is_err());
        assert_eq!(client.pending_command_count().await, 0);
        {
            let shutdown = client.shutdown.lock().await;
            assert!(shutdown.reader.is_none() && shutdown.keepalive.is_none());
        }
        assert!(inspect.ws_tx.lock().await.is_none());
        assert!(inspect.send_raw("{}".into()).await.is_err());
        assert!(client
            .quiesce_private_transport(&permit)
            .await
            .unwrap()
            .matches(&client, &permit));
        assert!(
            client.public_lease().is_err(),
            "transport closure never unlocks privacy"
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            0
        );
        assert!(client
            .send_command_inner("Runtime.evaluate", None, None, Duration::from_millis(10))
            .await
            .is_err());
        assert_eq!(client.pending_command_count().await, 0);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_cancellation_seals_queued_writer_and_can_finish_cleanup() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        let mut inspect = client.inspect_handle();
        // Isolate transport sealing from the independent observer-epoch guard.
        // This weakening exists only in the synthetic fixture.
        inspect.privacy_gate = None;
        inspect.privacy_observer = None;
        let sink = Arc::clone(&client.ws_tx);
        let held = sink.lock().await;
        let queued =
            tokio::spawn(
                async move { inspect.send_raw("SYNTHETIC_PRIVATE_SENTINEL".into()).await },
            );
        tokio::task::yield_now().await;
        assert!(tokio::time::timeout(
            Duration::from_millis(20),
            client.quiesce_private_transport(&permit)
        )
        .await
        .is_err());
        assert!(client
            .transport_sealed
            .load(std::sync::atomic::Ordering::SeqCst));
        assert!(client.public_lease().is_err());
        drop(held);
        assert_eq!(queued.await.unwrap(), Err("cdp_transport_closed".into()));
        assert!(client
            .quiesce_private_transport(&permit)
            .await
            .unwrap()
            .matches(&client, &permit));
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_wrong_permit_cannot_seal_another_client() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let (_other_fixture, other, other_peer) = idle_private_peer().await;
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        assert!(matches!(
            other.quiesce_private_transport(&permit).await,
            Err("private_permit_mismatch")
        ));
        assert!(!other
            .transport_sealed
            .load(std::sync::atomic::Ordering::SeqCst));
        assert!(other.ws_tx.lock().await.is_some());
        let proof = client.quiesce_private_transport(&permit).await.unwrap();
        assert!(!proof.matches(&other, &permit));
        let other_permit = other.begin_private_interval(&approved_target()).unwrap();
        other
            .quiesce_private_transport(&other_permit)
            .await
            .unwrap();
        for peer in [peer, other_peer] {
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(1), peer)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_old_proof_cannot_authorize_new_epoch() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let mut permit = client.begin_private_interval(&approved_target()).unwrap();
        let proof = client.quiesce_private_transport(&permit).await.unwrap();
        // Storage-only test transition, not a production cleanup authorization.
        client
            .privacy_gate
            .as_ref()
            .unwrap()
            .finish_private(&mut permit)
            .unwrap();
        let next = client.begin_private_interval(&approved_target()).unwrap();
        assert!(!proof.matches(&client, &next));
        assert!(matches!(
            client.quiesce_private_transport(&next).await,
            Err("private_shutdown_scope_mismatch")
        ));
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_cancellation_during_join_retains_completion_evidence() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        let old = client.shutdown.lock().await.keepalive.take().unwrap();
        old.abort();
        let _ = old.await;
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        // A started blocking worker cannot be aborted. This fixture forces an
        // observable join wait without blocking the async runtime thread.
        client.shutdown.lock().await.keepalive = Some(tokio::task::spawn_blocking(move || {
            let _ = started_tx.send(());
            let _ = release_rx.recv_timeout(Duration::from_secs(2));
        }));
        started_rx.await.unwrap();
        assert!(tokio::time::timeout(
            Duration::from_millis(20),
            client.quiesce_private_transport(&permit)
        )
        .await
        .is_err());
        assert!(client.shutdown.lock().await.keepalive.is_some());
        assert!(client.ws_tx.lock().await.is_none());
        assert!(client.public_lease().is_err());
        release_tx.send(()).unwrap();
        assert!(client
            .quiesce_private_transport(&permit)
            .await
            .unwrap()
            .matches(&client, &permit));
        assert!(client.shutdown.lock().await.keepalive.is_none());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_worker_panic_never_becomes_success_on_retry() {
        let (_fixture, client, peer) = idle_private_peer().await;
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        let old = client.shutdown.lock().await.keepalive.take().unwrap();
        old.abort();
        let _ = old.await;
        client.shutdown.lock().await.keepalive = Some(tokio::spawn(async {
            panic!("synthetic worker failure");
        }));
        while !client
            .shutdown
            .lock()
            .await
            .keepalive
            .as_ref()
            .unwrap()
            .is_finished()
        {
            tokio::task::yield_now().await;
        }
        for _ in 0..2 {
            assert!(matches!(
                client.quiesce_private_transport(&permit).await,
                Err("private_transport_join_failed")
            ));
        }
        assert!(client.public_lease().is_err());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_quiescence_cancels_live_pending_reply_without_publication() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let (seen_tx, seen_rx) = tokio::sync::oneshot::channel();
        let (late_tx, late_rx) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let message = ws.next().await.unwrap().unwrap();
            let command: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(command["method"], "Target.getTargetInfo");
            seen_tx.send(()).unwrap();
            late_rx.await.unwrap();
            let _ = ws
                .send(Message::Text(
                    json!({"id":command["id"],"result":{
                "targetInfo":{"targetId":"retained","type":"page"},
                "sentinel":"SYNTHETIC_PRIVATE_DELAYED"}})
                    .to_string(),
                ))
                .await;
        });
        let client = Arc::new(fixture.connect().await.unwrap());
        let permit = Arc::new(client.begin_private_interval(&approved_target()).unwrap());
        let mut raw = client.subscribe_raw();
        let waiting = {
            let client = Arc::clone(&client);
            let permit = Arc::clone(&permit);
            tokio::spawn(async move {
                client
                    .send_private_command(
                        &permit,
                        "Target.getTargetInfo",
                        Some(json!({"targetId":"retained"})),
                        None,
                        Duration::from_secs(3),
                    )
                    .await
            })
        };
        seen_rx.await.unwrap();
        assert_eq!(client.pending_command_count().await, 1);
        let (first, second) = tokio::join!(
            client.quiesce_private_transport(&permit),
            client.quiesce_private_transport(&permit)
        );
        assert!(first.unwrap().matches(&client, &permit));
        assert!(second.unwrap().matches(&client, &permit));
        assert!(matches!(
            waiting.await.unwrap(),
            Err("private_cdp_operation_failed")
        ));
        late_tx.send(()).unwrap();
        peer.await.unwrap();
        assert!(raw.try_recv().is_err());
        assert_eq!(client.pending_command_count().await, 0);
        assert!(client.public_lease().is_err());
    }

    fn approved_target() -> crate::native::private_identity::ValidatedPrivateTarget {
        use crate::native::private_identity::{validate_private_target, LivePrivateIdentity};
        let handle = json!({"profileId":"sam","browserId":"browser","sessionName":"default","targetId":"retained","tabId":"tab","url":"https://secure.login.gov/","leaseId":"default","leaseState":"exclusive","leaseHeartbeatExpected":true,"ownerSessionId":"default","valid":true,"staleReason":null});
        validate_private_target(
            &handle,
            &handle,
            &LivePrivateIdentity {
                profile_id: "sam",
                browser_id: "browser",
                session_name: "default",
                target_id: "retained",
                url: "https://secure.login.gov/",
                ready: true,
            },
            "https://secure.login.gov",
            "https://secure.login.gov/",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn short_deadline_times_out_without_blocking_the_next_command() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let fixture = TestCdpEndpoint::new(&format!("ws://{address}")).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let first = websocket.next().await.unwrap().unwrap();
            let first: serde_json::Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
            assert_eq!(first["method"], "Runtime.evaluate");

            let second = websocket.next().await.unwrap().unwrap();
            let second: serde_json::Value =
                serde_json::from_str(second.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": second["id"],
                        "result": { "ready": true }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = fixture.connect().await.unwrap();
        let error = client
            .send_command_with_timeout(
                "Runtime.evaluate",
                Some(json!({ "expression": "new Promise(() => {})" })),
                Some("session-1"),
                Duration::from_millis(25),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            CdpCommandError::Timeout {
                ref method,
                timeout
            } if method == "Runtime.evaluate" && timeout == Duration::from_millis(25)
        ));

        let result = client
            .send_command_with_timeout(
                "Runtime.evaluate",
                Some(json!({ "expression": "true" })),
                Some("session-1"),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert_eq!(result, json!({ "ready": true }));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn externally_cancelled_command_removes_its_pending_registration() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let fixture = TestCdpEndpoint::new(&format!("ws://{address}")).unwrap();
        let (received_tx, received_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let command = websocket.next().await.unwrap().unwrap();
            let command: serde_json::Value =
                serde_json::from_str(command.to_text().unwrap()).unwrap();
            received_tx.send(command["id"].as_u64().unwrap()).unwrap();
            futures_util::future::pending::<()>().await;
        });

        let client = Arc::new(fixture.connect().await.unwrap());
        let command_client = client.clone();
        let command = tokio::spawn(async move {
            command_client
                .send_command_with_timeout(
                    "Runtime.evaluate",
                    Some(json!({ "expression": "new Promise(() => {})" })),
                    Some("session-1"),
                    Duration::from_secs(60),
                )
                .await
        });

        received_rx.await.unwrap();
        assert_eq!(client.pending_command_count().await, 1);
        command.abort();
        let _ = command.await;

        tokio::task::yield_now().await;
        assert_eq!(client.pending_command_count().await, 0);

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(
            client.begin_private_interval(&approved_target()).is_err(),
            "cancelled Chrome work must block private admission"
        );

        server.abort();
    }

    #[tokio::test]
    async fn dropping_client_closes_background_websocket_tasks() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let fixture = TestCdpEndpoint::new(&format!("ws://{address}")).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            websocket.next().await
        });

        let client = fixture.connect().await.unwrap();
        drop(client);

        let closed = tokio::time::timeout(Duration::from_millis(250), server).await;
        assert!(
            closed.is_ok(),
            "dropping a CDP client must stop its reader and keepalive tasks"
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_scope_rejects_sibling_target_before_mutation() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let wire = ws.next().await.unwrap().unwrap();
            let probe: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
            assert_eq!(probe["method"], "Target.getTargetInfo");
            assert_eq!(probe["sessionId"], "foreign-session");
            assert!(probe.get("params").is_none_or(Value::is_null));
            ws.send(Message::Text(json!({"id":probe["id"],"result":{"targetInfo":{"targetId":"foreign","type":"page"}}}).to_string())).await.unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(100), ws.next())
                    .await
                    .is_err(),
                "no mutation or second target attach may reach Chrome"
            );
        });
        let client = fixture.connect().await.unwrap();
        let mut permit = client.begin_private_interval(&approved_target()).unwrap();
        assert_eq!(
            client
                .send_private_command(
                    &permit,
                    "Runtime.evaluate",
                    Some(json!({"expression":"PRIVATE_SENTINEL"})),
                    Some("foreign-session"),
                    Duration::from_secs(1)
                )
                .await
                .err(),
            Some("private_scope_mismatch")
        );
        assert_eq!(
            client
                .send_private_command(
                    &permit,
                    "Target.attachToTarget",
                    Some(json!({"targetId":"foreign","flatten":true})),
                    None,
                    Duration::from_secs(1)
                )
                .await
                .err(),
            Some("private_scope_mismatch")
        );
        assert_eq!(
            client
                .sanitize_private_page(&mut permit, "foreign", "foreign-session")
                .await
                .err(),
            Some("private_cleanup_identity_mismatch")
        );
        assert!(client.public_lease().is_err());
        server.await.unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn timed_out_private_reply_never_becomes_public_after_cleanup() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let (late_tx, late_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let wire = ws.next().await.unwrap().unwrap();
            let probe: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
            assert_eq!(probe["method"], "Target.getTargetInfo");
            ws.send(Message::Text(json!({"id":probe["id"],"result":{"targetInfo":{"targetId":"retained","type":"page"}}}).to_string())).await.unwrap();
            let wire = ws.next().await.unwrap().unwrap();
            let command: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
            assert_eq!(command["method"], "Runtime.evaluate");
            late_rx.await.unwrap();
            ws.send(Message::Text(
                json!({"id":command["id"],"result":{"value":"LATE_PRIVATE_SENTINEL"}}).to_string(),
            ))
            .await
            .unwrap();
        });
        let client = fixture.connect().await.unwrap();
        let mut raw = client.subscribe_raw();
        let mut permit = client.begin_private_interval(&approved_target()).unwrap();
        assert_eq!(
            client
                .send_private_command(
                    &permit,
                    "Runtime.evaluate",
                    None,
                    Some("page"),
                    Duration::from_millis(100)
                )
                .await
                .err(),
            Some("private_cdp_operation_failed")
        );
        assert_eq!(client.pending_command_count().await, 0);
        // Test the observer defense even if a future coordinator unlocks.
        client
            .privacy_gate
            .as_ref()
            .unwrap()
            .finish_private(&mut permit)
            .unwrap();
        late_tx.send(()).unwrap();
        server.await.unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(30), raw.recv())
            .await
            .is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn old_and_private_sockets_stay_quarantined_after_epoch_cleanup() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let (emit_tx, emit_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let mut sockets = Vec::new();
            for _ in 0..3 {
                let (stream, _) = listener.accept().await.unwrap();
                sockets.push(tokio_tungstenite::accept_async(stream).await.unwrap());
            }
            emit_rx.await.unwrap();
            // These bytes were never observed during the private interval.
            // Receipt-time lock checks alone used to make them public now.
            for ws in &mut sockets {
                for message in [
                    json!({"id":1,"result":{"value":"DELAYED_PRIVATE_SENTINEL"}}),
                    json!({"method":"Runtime.consoleAPICalled","params":{"value":"DELAYED_PRIVATE_SENTINEL"}}),
                    json!({"method":"Page.screencastFrame","params":{"data":"DELAYED_PRIVATE_SENTINEL"}}),
                ] {
                    ws.send(Message::Text(message.to_string())).await.unwrap();
                }
            }
            let (stream, _) = listener.accept().await.unwrap();
            let mut fresh = tokio_tungstenite::accept_async(stream).await.unwrap();
            let wire = fresh.next().await.unwrap().unwrap();
            let cmd: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
            fresh
                .send(Message::Text(
                    json!({"id":cmd["id"],"result":{"public":true}}).to_string(),
                ))
                .await
                .unwrap();
        });
        let first = fixture.connect().await.unwrap();
        let second = fixture.connect().await.unwrap();
        let inspect = second.inspect_handle();
        let mut raw = first.subscribe_raw();
        let mut events = second.subscribe();
        let mut permit = first.begin_private_interval(&approved_target()).unwrap();
        let recovery = CdpClient::connect_private_recovery(&endpoint, &permit)
            .await
            .unwrap();
        let mut private_raw = recovery.subscribe_raw();
        // Storage-only test transition, not production cleanup authorization.
        first
            .privacy_gate
            .as_ref()
            .unwrap()
            .finish_private(&mut permit)
            .unwrap();
        emit_tx.send(()).unwrap();
        for old in [&first, &second, &recovery] {
            assert!(old.public_lease().is_err());
            assert!(old
                .send_command_no_params("Runtime.evaluate", None)
                .await
                .is_err());
        }
        assert!(inspect.send_raw("{}".into()).await.is_err());
        let fresh = fixture.connect().await.unwrap();
        assert_eq!(
            fresh
                .send_command_no_params("Target.getTargets", None)
                .await
                .unwrap(),
            json!({"public":true})
        );
        server.await.unwrap();
        tokio::task::yield_now().await;
        assert!(raw.try_recv().is_err());
        assert!(events.try_recv().is_err());
        assert!(private_raw.try_recv().is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_recovery_reconnects_only_to_bound_endpoint_without_public_output() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let first = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (stream, _) = listener.accept().await.unwrap();
            let mut second = tokio_tungstenite::accept_async(stream).await.unwrap();
            for failed in [false, true] {
                let probe = second.next().await.unwrap().unwrap();
                let probe: Value = serde_json::from_str(probe.to_text().unwrap()).unwrap();
                assert_eq!(probe["method"], "Target.getTargetInfo");
                assert!(probe.get("params").is_none_or(Value::is_null));
                second.send(Message::Text(json!({"id":probe["id"],"result":{"targetInfo":{"targetId":"retained","type":"page"}}}).to_string())).await.unwrap();
                let wire = second.next().await.unwrap().unwrap();
                let command: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
                assert_eq!(command["method"], "Runtime.evaluate");
                second.send(Message::Text(json!({"method":"Runtime.consoleAPICalled","params":{"value":"PRIVATE_SENTINEL"}}).to_string())).await.unwrap();
                let response = if failed {
                    json!({"id":command["id"],"error":{"code":-1,"message":"PRIVATE_SENTINEL"}})
                } else {
                    json!({"id":command["id"],"result":{"value":"PRIVATE_SENTINEL"}})
                };
                second
                    .send(Message::Text(response.to_string()))
                    .await
                    .unwrap();
            }
            drop(first);
        });
        let client = fixture.connect().await.unwrap();
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        assert!(
            CdpClient::connect_private_recovery("ws://127.0.0.1:1", &permit)
                .await
                .is_err()
        );
        let recovered = CdpClient::connect_private_recovery(&endpoint, &permit)
            .await
            .unwrap();
        let mut raw = recovered.subscribe_raw();
        let mut events = recovered.subscribe();
        let value = recovered
            .send_private_command(
                &permit,
                "Runtime.evaluate",
                None,
                Some("page"),
                Duration::from_secs(1),
            )
            .await
            .unwrap()
            .into_value();
        assert_eq!(value["value"], "PRIVATE_SENTINEL");
        let error = recovered
            .send_private_command(
                &permit,
                "Runtime.evaluate",
                None,
                Some("page"),
                Duration::from_secs(1),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(error, "private_cdp_operation_failed");
        server.await.unwrap();
        assert!(raw.try_recv().is_err());
        assert!(events.try_recv().is_err());
        assert!(recovered.public_lease().is_err());
        drop(permit);
        assert!(fixture.connect().await.is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_cleanup_requires_exact_target_and_verified_empty_document() {
        for failure in ["none", "target", "dom", "final_url"] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("ws://{}", listener.local_addr().unwrap());
            let fixture = TestCdpEndpoint::new(&endpoint).unwrap();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let steps = [
                    (
                        "Target.getTargetInfo",
                        json!({"targetInfo":{"targetId":if failure == "target" {"wrong"} else {"retained"},"type":"page","url":"https://secure.login.gov/"}}),
                    ),
                    ("Page.stopLoading", json!({})),
                    ("Page.navigate", json!({"frameId":"main"})),
                    (
                        "Page.getFrameTree",
                        json!({"frameTree":{"frame":{"id":"main","url":"about:blank"}}}),
                    ),
                    ("Page.createIsolatedWorld", json!({"executionContextId":71})),
                    (
                        "Runtime.evaluate",
                        json!({"result":{"type":"boolean","value":failure != "dom"}}),
                    ),
                    (
                        "Target.getTargetInfo",
                        json!({"targetInfo":{"targetId":"retained","type":"page","url":if failure == "final_url" {"https://secure.login.gov/"} else {"about:blank"}}}),
                    ),
                ];
                for (method, result) in steps {
                    if method != "Target.getTargetInfo" {
                        let probe = ws.next().await.unwrap().unwrap();
                        let probe: Value = serde_json::from_str(probe.to_text().unwrap()).unwrap();
                        assert_eq!(probe["method"], "Target.getTargetInfo");
                        assert_eq!(probe["sessionId"], "retained-session");
                        assert!(probe.get("params").is_none_or(Value::is_null));
                        ws.send(Message::Text(json!({"id":probe["id"],"result":{"targetInfo":{"targetId":"retained","type":"page"}}}).to_string())).await.unwrap();
                    }
                    let wire = ws.next().await.unwrap().unwrap();
                    let cmd: Value = serde_json::from_str(wire.to_text().unwrap()).unwrap();
                    assert_eq!(cmd["method"], method);
                    assert_eq!(cmd["sessionId"], "retained-session");
                    if method == "Page.navigate" {
                        assert_eq!(cmd["params"]["url"], "about:blank");
                    }
                    if method == "Runtime.evaluate" {
                        assert_eq!(cmd["params"]["contextId"], 71);
                    }
                    ws.send(Message::Text(
                        json!({"id":cmd["id"],"result":result}).to_string(),
                    ))
                    .await
                    .unwrap();
                    if failure == "target" || (failure == "dom" && method == "Runtime.evaluate") {
                        break;
                    }
                }
            });
            let client = fixture.connect().await.unwrap();
            let mut permit = client.begin_private_interval(&approved_target()).unwrap();
            let result = client
                .sanitize_private_page(&mut permit, "retained", "retained-session")
                .await;
            assert_eq!(result.is_ok(), failure == "none", "{failure}");
            assert!(
                client.public_lease().is_err(),
                "page cleanup alone must not unlock: {failure}"
            );
            if result.is_ok() {
                let proof = client.quiesce_private_transport(&permit).await.unwrap();
                assert!(proof.matches(&client, &permit));
                assert!(
                    client.public_lease().is_err(),
                    "sanitation plus closure is not broker detach"
                );
            } else {
                assert!(!client
                    .transport_sealed
                    .load(std::sync::atomic::Ordering::SeqCst));
            }
            server.await.unwrap();
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn private_interval_suppresses_raw_events_and_refuses_commands_and_reconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let fixture = TestCdpEndpoint::new(&format!("ws://{address}")).unwrap();
        let (go_tx, go_rx) = tokio::sync::oneshot::channel();
        let (sent_tx, sent_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            go_rx.await.unwrap();
            for message in [
                json!({"method":"Runtime.consoleAPICalled","params":{"sentinel":"SYNTHETIC_PRIVATE_SENTINEL"}}),
                json!({"method":"Page.screencastFrame","params":{"data":"SYNTHETIC_PRIVATE_SENTINEL"}}),
                json!({"id":987654,"result":{"sentinel":"SYNTHETIC_PRIVATE_SENTINEL"}}),
            ] {
                websocket
                    .send(Message::Text(message.to_string()))
                    .await
                    .unwrap();
            }
            sent_tx.send(()).unwrap();
            // No public command may reach the peer while the private lock exists.
            assert!(
                tokio::time::timeout(Duration::from_millis(100), websocket.next())
                    .await
                    .is_err()
            );
        });
        let client = fixture.connect().await.unwrap();
        let mut raw = client.subscribe_raw();
        let mut events = client.subscribe();
        let permit = client.begin_private_interval(&approved_target()).unwrap();
        go_tx.send(()).unwrap();
        sent_rx.await.unwrap();
        assert!(client
            .send_command_no_params("Runtime.evaluate", None)
            .await
            .is_err());
        assert!(client
            .send_raw("SYNTHETIC_PRIVATE_SENTINEL".into())
            .await
            .is_err());
        assert!(client
            .inspect_handle()
            .send_raw("SYNTHETIC_PRIVATE_SENTINEL".into())
            .await
            .is_err());
        assert!(fixture.connect().await.is_err());
        assert!(tokio::time::timeout(Duration::from_millis(20), raw.recv())
            .await
            .is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), events.recv())
                .await
                .is_err()
        );
        drop(permit);
        assert!(client.public_lease().is_err());
        assert!(fixture.connect().await.is_err());
        server.await.unwrap();
    }
}
