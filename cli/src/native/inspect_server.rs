use std::io::Write;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio_tungstenite::tungstenite::Message;

use super::cdp::client::InspectProxyHandle;

/// Counter for unique attach IDs so concurrent connections don't collide.
static ATTACH_ID: AtomicI64 = AtomicI64::new(-1000);

/// Lightweight HTTP + WebSocket server for `agent-browser inspect`.
///
/// Serves two purposes:
/// - `GET /` redirects to Chrome's built-in DevTools frontend with `ws=` pointing to this server
/// - WebSocket connections create a dedicated CDP session via `Target.attachToTarget` and proxy
///   CDP messages through the daemon's existing browser-level connection, injecting/stripping
///   `sessionId` so the DevTools frontend sees a page-level view
pub struct InspectServer {
    port: u16,
    handle: Option<JoinHandle<Result<(), String>>>,
    stop_tx: Option<oneshot::Sender<()>>,
    shutdown_error: Option<String>,
}

impl InspectServer {
    /// Start the inspect proxy server.
    ///
    /// - `proxy_handle`: lightweight handle for sending/receiving raw CDP messages
    /// - `target_id`: the CDP target ID of the page to inspect
    /// - `chrome_host_port`: the Chrome debug server address (e.g. "127.0.0.1:9222")
    pub async fn start(
        proxy_handle: InspectProxyHandle,
        target_id: String,
        chrome_host_port: String,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind inspect server: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?
            .port();

        let proxy = Arc::new(proxy_handle);

        let (stop_tx, stop_rx) = oneshot::channel();
        let handle = tokio::spawn(accept_loop(
            listener,
            proxy,
            target_id,
            chrome_host_port,
            port,
            stop_rx,
        ));

        Ok(Self {
            port,
            handle: Some(handle),
            stop_tx: Some(stop_tx),
            shutdown_error: None,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn shutdown(self) {
        // Drop is best-effort only. Privacy barriers must use shutdown_and_wait.
    }

    /// Stop accepting connections and verify that every proxy task has exited.
    /// The handle remains owned while awaiting, so cancellation permits retry.
    pub async fn shutdown_and_wait(&mut self) -> Result<(), String> {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(handle) = self.handle.as_mut() {
            let result = handle
                .await
                .unwrap_or_else(|_| Err("inspect_shutdown_unverified".to_string()));
            self.shutdown_error = result.err();
            self.handle.take();
        }
        self.shutdown_error.clone().map_or(Ok(()), Err)
    }
}

impl Drop for InspectServer {
    fn drop(&mut self) {
        if let Some(handle) = &self.handle {
            // Aborting the owner drops its JoinSet, aborting all connections.
            handle.abort();
        }
    }
}

async fn accept_loop(
    listener: TcpListener,
    proxy: Arc<InspectProxyHandle>,
    target_id: String,
    chrome_host_port: String,
    proxy_port: u16,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let mut connections = JoinSet::new();
    let mut task_failed = false;
    let (connection_stop, _) = watch::channel(false);
    loop {
        let (stream, _) = tokio::select! {
            biased;
            _ = &mut stop_rx => break,
            joined = connections.join_next(), if !connections.is_empty() => {
                task_failed |= joined.is_some_and(|result| !matches!(result, Ok(Ok(()))));
                continue;
            }
            accepted = listener.accept() => match accepted {
                Ok(s) => s,
                Err(_) => continue,
            },
        };

        let proxy = proxy.clone();
        let tid = target_id.clone();
        let chp = chrome_host_port.clone();
        let stop = connection_stop.subscribe();

        connections.spawn(async move {
            handle_connection(stream, proxy, tid, chp, proxy_port, stop).await
        });
    }
    drop(listener);
    let _ = connection_stop.send(true);
    // Let acquired sessions complete one acknowledged detach. Cancellation of
    // the outer waiter does not cancel this server-owned cleanup task.
    let drained = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        while let Some(result) = connections.join_next().await {
            task_failed |= !matches!(result, Ok(Ok(())));
        }
    })
    .await;
    if drained.is_err() {
        task_failed = true;
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    if task_failed {
        Err("inspect_connection_cleanup_unverified".to_string())
    } else {
        Ok(())
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    proxy: Arc<InspectProxyHandle>,
    target_id: String,
    chrome_host_port: String,
    proxy_port: u16,
    mut stop: watch::Receiver<bool>,
) -> Result<(), String> {
    // Peek at the request line to determine routing WITHOUT consuming bytes.
    // This is critical: tokio_tungstenite::accept_async needs to read the full
    // HTTP upgrade request itself, so we must not consume anything for WS paths.
    let mut peek_buf = [0u8; 32];
    let n = tokio::select! {
        biased;
        _ = wait_for_stop(&mut stop) => return Ok(()),
        result = stream.peek(&mut peek_buf) => result.map_err(|e| e.to_string())?,
    };
    let peek = String::from_utf8_lossy(&peek_buf[..n]);

    if peek.starts_with("GET /ws") {
        return handle_ws_proxy(stream, proxy, target_id, stop).await;
    }

    if peek.starts_with("GET / ") {
        let buf_reader = BufReader::new(stream);
        return tokio::select! {
            biased;
            _ = wait_for_stop(&mut stop) => Ok(()),
            result = handle_http_redirect(buf_reader, chrome_host_port, proxy_port) => result,
        };
    }

    // Unknown request -- consume and respond 404
    let mut stream = stream;
    let mut discard = [0u8; 4096];
    let _ = stream.read(&mut discard).await;
    let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream
        .write_all(resp.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

const MAX_HEADER_BYTES: usize = 8192;

async fn handle_http_redirect(
    buf_reader: BufReader<tokio::net::TcpStream>,
    chrome_host_port: String,
    proxy_port: u16,
) -> Result<(), String> {
    let mut br = buf_reader;
    let mut total_bytes = 0usize;
    loop {
        let mut line = String::new();
        let n = br.read_line(&mut line).await.map_err(|e| e.to_string())?;
        total_bytes += n;
        if line == "\r\n" || line == "\n" || line.is_empty() || total_bytes > MAX_HEADER_BYTES {
            break;
        }
    }

    let location = format!(
        "http://{}/devtools/devtools_app.html?ws=127.0.0.1:{}/ws",
        chrome_host_port, proxy_port
    );
    let body = format!(
        "<html><body>Redirecting to <a href=\"{url}\">{url}</a></body></html>",
        url = location
    );
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        location,
        body.len(),
        body
    );
    let mut stream = br.into_inner();
    stream
        .write_all(resp.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn handle_ws_proxy(
    stream: tokio::net::TcpStream,
    proxy: Arc<InspectProxyHandle>,
    target_id: String,
    mut stop: watch::Receiver<bool>,
) -> Result<(), String> {
    let ws_stream = tokio::select! {
        biased;
        _ = wait_for_stop(&mut stop) => return Ok(()),
        result = tokio_tungstenite::accept_async(stream) =>
            result.map_err(|e| format!("WebSocket handshake failed: {}", e))?,
    };

    // Create a dedicated CDP session for this DevTools connection.
    // Each connection gets its own session so domain enablements (DOM.enable, etc.)
    // always trigger fresh initial state dumps from Chrome.
    let attach_id = ATTACH_ID.fetch_sub(1, Ordering::SeqCst);
    let attach_cmd = format!(
        r#"{{"id":{},"method":"Target.attachToTarget","params":{{"targetId":"{}","flatten":true}}}}"#,
        attach_id, target_id
    );

    // Subscribe BEFORE sending so we don't miss the response (tokio broadcast
    // receivers only deliver messages to receivers that already exist).
    let mut raw_rx = proxy.subscribe_raw();

    proxy
        .send_raw(attach_cmd)
        .await
        .map_err(|e| format!("Failed to send attachToTarget: {}", e))?;

    // Wait for the attachToTarget response to extract the session ID
    let session_id = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Ok(raw_msg) = raw_rx.recv().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_msg.text) {
                if val.get("id").and_then(|v| v.as_i64()) == Some(attach_id) {
                    if let Some(sid) = val
                        .get("result")
                        .and_then(|r| r.get("sessionId"))
                        .and_then(|s| s.as_str())
                    {
                        return Ok(sid.to_string());
                    }
                    return Err("attachToTarget failed".to_string());
                }
            }
        }
        Err("raw message channel closed".to_string())
    })
    .await
    .map_err(|_| "Timed out waiting for attachToTarget response".to_string())?
    .map_err(|e| format!("Failed to create DevTools session: {}", e))?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let mut raw_rx = proxy.subscribe_raw();
    let session_id_clone = session_id.clone();
    let proxy_for_output = proxy.clone();

    // Chrome -> DevTools: forward messages matching our session, strip sessionId
    // Keep forwarding futures inside the tracked connection task. Detached
    // spawn handles would survive cancellation of their parent connection.
    let chrome_to_devtools = async move {
        loop {
            let raw_msg = match raw_rx.recv().await {
                Ok(msg) => msg,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let _ = writeln!(
                        std::io::stderr(),
                        "[inspect] warning: dropped {} CDP messages (channel lag)",
                        n
                    );
                    continue;
                }
                Err(_) => break,
            };

            if raw_msg.session_id.as_deref() != Some(&session_id_clone) {
                continue;
            }

            let Ok(_privacy_lease) = proxy_for_output.public_lease() else {
                break;
            };

            let stripped = strip_session_id(&raw_msg.text);

            if ws_tx.send(Message::Text(stripped)).await.is_err() {
                break;
            }
        }
    };

    // DevTools -> Chrome: inject sessionId and forward
    let proxy_for_send = proxy.clone();
    let session_id_for_send = session_id.clone();
    let devtools_to_chrome = async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            let text = match msg {
                Message::Text(t) => t,
                Message::Close(_) => break,
                _ => continue,
            };

            let injected = inject_session_id(&text, &session_id_for_send);
            if proxy_for_send.send_raw(injected).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        biased;
        _ = wait_for_stop(&mut stop) => {},
        _ = chrome_to_devtools => {},
        _ = devtools_to_chrome => {},
    }

    detach_session(&proxy, &session_id).await
}

async fn wait_for_stop(stop: &mut watch::Receiver<bool>) {
    while !*stop.borrow_and_update() {
        if stop.changed().await.is_err() {
            break;
        }
    }
}

/// Send exactly once and require a matching browser-level acknowledgment.
/// Timeout, channel loss and protocol errors are cleanup uncertainty, not success.
async fn detach_session(proxy: &InspectProxyHandle, session_id: &str) -> Result<(), String> {
    let id = ATTACH_ID.fetch_sub(1, Ordering::SeqCst);
    let mut responses = proxy.subscribe_raw();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        proxy
            .send_raw(
                serde_json::json!({
                    "id": id,
                    "method": "Target.detachFromTarget",
                    "params": {"sessionId": session_id},
                })
                .to_string(),
            )
            .await?;
        loop {
            let raw = responses
                .recv()
                .await
                .map_err(|_| "inspect_detach_channel_lost")?;
            let Ok(response) = serde_json::from_str::<serde_json::Value>(&raw.text) else {
                continue;
            };
            if response.get("id").and_then(serde_json::Value::as_i64) != Some(id) {
                continue;
            }
            if response.get("sessionId").is_some()
                || response.get("error").is_some()
                || !response
                    .get("result")
                    .is_some_and(serde_json::Value::is_object)
            {
                return Err("inspect_detach_acknowledgment_invalid".into());
            }
            return Ok(());
        }
    })
    .await
    .map_err(|_| "inspect_detach_acknowledgment_timeout".to_string())?
}

fn inject_session_id(json: &str, session_id: &str) -> String {
    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(obj) = val.as_object_mut() {
            obj.insert(
                "sessionId".to_string(),
                serde_json::Value::String(session_id.to_string()),
            );
        }
        serde_json::to_string(&val).unwrap_or_else(|_| json.to_string())
    } else {
        json.to_string()
    }
}

fn strip_session_id(json: &str) -> String {
    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(obj) = val.as_object_mut() {
            obj.remove("sessionId");
        }
        serde_json::to_string(&val).unwrap_or_else(|_| json.to_string())
    } else {
        json.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::cdp::client::CdpClient;
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[derive(Clone, Copy, Debug)]
    enum DetachReply {
        Accepted,
        Rejected,
        Missing,
        WrongSession,
    }

    async fn synthetic_proxy() -> (CdpClient, JoinHandle<()>) {
        let (client, peer, _) = synthetic_proxy_with_detach_reply(DetachReply::Accepted).await;
        (client, peer)
    }

    async fn synthetic_proxy_with_detach_reply(
        detach_reply: DetachReply,
    ) -> (CdpClient, JoinHandle<()>, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let detaches = Arc::new(AtomicUsize::new(0));
        let peer_detaches = detaches.clone();
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(Message::Text(text))) = ws.next().await {
                let request: serde_json::Value = serde_json::from_str(&text).unwrap();
                let result = if request["method"] == "Target.attachToTarget" {
                    json!({"sessionId": "synthetic-inspect-session"})
                } else {
                    json!({"method": request["method"]})
                };
                let mut response = json!({"id": request["id"], "result": result});
                if request["method"] == "Target.detachFromTarget" {
                    assert_eq!(request["params"]["sessionId"], "synthetic-inspect-session");
                    peer_detaches.fetch_add(1, Ordering::SeqCst);
                    match detach_reply {
                        DetachReply::Accepted => {}
                        DetachReply::Rejected => {
                            response = json!({"id": request["id"], "error": {"code": -32000, "message": "synthetic rejection"}});
                        }
                        DetachReply::Missing => continue,
                        DetachReply::WrongSession => {
                            response["sessionId"] = json!("foreign-session")
                        }
                    }
                }
                if let Some(session_id) = request.get("sessionId") {
                    response["sessionId"] = session_id.clone();
                }
                if ws.send(Message::Text(response.to_string())).await.is_err() {
                    break;
                }
            }
        });
        let client = CdpClient::connect(&format!("ws://{addr}")).await.unwrap();
        (client, peer, detaches)
    }

    #[tokio::test]
    async fn shutdown_joins_connected_websocket_and_preserves_upstream() {
        let (client, peer, detaches) =
            synthetic_proxy_with_detach_reply(DetachReply::Accepted).await;
        let mut server = InspectServer::start(
            client.inspect_handle(),
            "synthetic-target".into(),
            "127.0.0.1:1".into(),
        )
        .await
        .unwrap();
        let (mut ws, _) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", server.port()))
                .await
                .unwrap();
        ws.send(Message::Text(
            json!({"id": 31, "method": "Runtime.enable"}).to_string(),
        ))
        .await
        .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(response.to_text().unwrap().contains("31"));
        tokio::time::timeout(Duration::from_secs(2), server.shutdown_and_wait())
            .await
            .unwrap()
            .unwrap();
        server.shutdown_and_wait().await.unwrap();
        assert_eq!(detaches.load(Ordering::SeqCst), 1);
        let closed = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap();
        assert!(matches!(
            closed,
            None | Some(Err(_)) | Some(Ok(Message::Close(_)))
        ));
        assert!(tokio::net::TcpStream::connect(("127.0.0.1", server.port()))
            .await
            .is_err());
        client
            .send_command("Browser.getVersion", None, None)
            .await
            .unwrap();
        peer.abort();
        let _ = peer.await;
    }

    #[tokio::test]
    async fn detach_rejection_is_propagated_and_not_replayed_on_shutdown_retry() {
        assert_detach_failure(DetachReply::Rejected).await;
        assert_detach_failure(DetachReply::Missing).await;
        assert_detach_failure(DetachReply::WrongSession).await;
    }

    async fn assert_detach_failure(reply: DetachReply) {
        let (client, peer, detaches) = synthetic_proxy_with_detach_reply(reply).await;
        let mut server = InspectServer::start(
            client.inspect_handle(),
            "synthetic-target".into(),
            "127.0.0.1:1".into(),
        )
        .await
        .unwrap();
        let (mut ws, _) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", server.port()))
                .await
                .unwrap();
        ws.send(Message::Text(
            json!({"id": 51, "method": "Runtime.enable"}).to_string(),
        ))
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        for _ in 0..2 {
            let error = tokio::time::timeout(Duration::from_secs(7), server.shutdown_and_wait())
                .await
                .unwrap()
                .unwrap_err();
            assert_eq!(error, "inspect_connection_cleanup_unverified");
        }
        assert_eq!(detaches.load(Ordering::SeqCst), 1);
        client
            .send_command("Browser.getVersion", None, None)
            .await
            .unwrap();
        peer.abort();
        let _ = peer.await;
    }

    #[tokio::test]
    async fn websocket_disconnect_finishes_forwarding_and_detaches_session() {
        let (client, peer) = synthetic_proxy().await;
        let mut server = InspectServer::start(
            client.inspect_handle(),
            "synthetic-target".into(),
            "127.0.0.1:1".into(),
        )
        .await
        .unwrap();
        let mut raw_rx = client.subscribe_raw();
        let (mut ws, _) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/ws", server.port()))
                .await
                .unwrap();
        ws.close(None).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let raw = raw_rx.recv().await.unwrap();
                let response: serde_json::Value = serde_json::from_str(&raw.text).unwrap();
                if response["result"]["method"] == "Target.detachFromTarget" {
                    break;
                }
            }
        })
        .await
        .unwrap();
        server.shutdown_and_wait().await.unwrap();
        peer.abort();
        let _ = peer.await;
    }

    #[tokio::test]
    async fn shutdown_joins_incomplete_http_connections() {
        let (client, peer) = synthetic_proxy().await;
        let mut server = InspectServer::start(
            client.inspect_handle(),
            "synthetic-target".into(),
            "127.0.0.1:1".into(),
        )
        .await
        .unwrap();
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", server.port()))
            .await
            .unwrap();
        // Get the connection into the header reader without completing headers.
        stream.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
        tokio::task::yield_now().await;
        server.shutdown_and_wait().await.unwrap();
        let mut byte = [0u8; 1];
        let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte))
            .await
            .unwrap();
        assert!(matches!(read, Ok(0) | Err(_)));
        peer.abort();
        let _ = peer.await;
    }

    #[tokio::test]
    async fn shutdown_failure_remains_fail_closed_on_retry() {
        let (stop_tx, _stop_rx) = oneshot::channel();
        let mut server = InspectServer {
            port: 0,
            handle: Some(tokio::spawn(async {
                Err("synthetic_cleanup_failure".into())
            })),
            stop_tx: Some(stop_tx),
            shutdown_error: None,
        };
        assert_eq!(
            server.shutdown_and_wait().await.unwrap_err(),
            "synthetic_cleanup_failure"
        );
        assert_eq!(
            server.shutdown_and_wait().await.unwrap_err(),
            "synthetic_cleanup_failure"
        );
    }

    #[tokio::test]
    async fn cancelled_shutdown_can_be_joined_again() {
        let (stop_tx, stop_rx) = oneshot::channel();
        let (finish_tx, finish_rx) = oneshot::channel();
        let mut server = InspectServer {
            port: 0,
            handle: Some(tokio::spawn(async move {
                let _ = stop_rx.await;
                let _ = finish_rx.await;
                Ok(())
            })),
            stop_tx: Some(stop_tx),
            shutdown_error: None,
        };
        assert!(
            tokio::time::timeout(Duration::from_millis(10), server.shutdown_and_wait())
                .await
                .is_err()
        );
        assert!(server.handle.is_some());
        finish_tx.send(()).unwrap();
        server.shutdown_and_wait().await.unwrap();
        assert!(server.handle.is_none());
    }

    #[test]
    fn test_inject_session_id() {
        let input = r#"{"id":1,"method":"DOM.getDocument"}"#;
        let result = inject_session_id(input, "abc123");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid JSON");
        assert_eq!(parsed["sessionId"], "abc123");
        assert_eq!(parsed["method"], "DOM.getDocument");
        assert_eq!(parsed["id"], 1);
    }

    #[test]
    fn test_inject_session_id_empty_object() {
        let result = inject_session_id("{}", "abc");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid JSON");
        assert_eq!(parsed["sessionId"], "abc");
    }

    #[test]
    fn test_strip_session_id() {
        let input = r#"{"id":1,"result":{},"sessionId":"abc123"}"#;
        let result = strip_session_id(input);
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid JSON");
        assert!(parsed.get("sessionId").is_none());
        assert_eq!(parsed["id"], 1);
    }

    #[test]
    fn test_inject_then_strip_roundtrip() {
        let input = r#"{"id":42,"method":"Runtime.evaluate"}"#;
        let injected = inject_session_id(input, "sess1");
        let stripped = strip_session_id(&injected);
        let original: serde_json::Value = serde_json::from_str(input).unwrap();
        let result: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(original, result);
    }
}
