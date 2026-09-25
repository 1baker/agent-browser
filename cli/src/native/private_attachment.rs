//! Dedicated private page-session ownership. No public command, gate release,
//! browser close, automatic retry, or caller-selected detach session is exposed.

use super::cdp::client::{CdpClient, PrivateTransportClosed};
use super::privacy_gate::PrivatePermit;
use super::private_secret_store::SecretStore;
use serde_json::json;
use std::{sync::Arc, time::Duration};

const INVALID: &str = "private_attachment_invalid";
const FAILED: &str = "private_attachment_failed";

enum DetachState {
    Attached,
    Uncertain,
    Completed,
}

/// Only this object owns the new flattened page session. Neither its identity
/// nor its encrypted receipts are ordinary output. Drop never retries detach.
pub(crate) struct PrivateAttachment {
    client: Arc<CdpClient>,
    epoch: String,
    target: String,
    session: String,
    detach_reference: String,
    detach_payload: Vec<u8>,
    state: DetachState,
}

impl PrivateAttachment {
    /// Commit admission before attaching; persist the returned identity before
    /// exposing the session to private execution. Any uncertainty is terminal.
    pub(crate) async fn acquire(
        client: Arc<CdpClient>,
        store: &SecretStore,
        permit: &PrivatePermit,
    ) -> Result<Self, &'static str> {
        if !client.private_permit_matches(permit) {
            return Err(INVALID);
        }
        let target = permit.target_id().ok_or(INVALID)?.to_owned();
        let epoch = permit.epoch_id().to_owned();
        let admission = serde_json::to_vec(&json!({
            "schema":"private-attachment-admission.v1", "epoch":epoch, "targetId":target
        }))
        .map_err(|_| INVALID)?;
        let reference = store.stage(&admission)?;
        store.admit(&reference)?;
        let attached = client
            .send_private_command(
                permit,
                "Target.attachToTarget",
                Some(json!({"targetId":target,"flatten":true})),
                None,
                Duration::from_secs(3),
            )
            .await?
            .into_value();
        let session = attached
            .get("sessionId")
            .and_then(|value| value.as_str())
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 4096
                    && value.trim() == *value
                    && !value.chars().any(char::is_control)
            })
            .ok_or(FAILED)?
            .to_owned();
        client
            .send_private_command(
                permit,
                "Target.getTargetInfo",
                None,
                Some(&session),
                Duration::from_secs(3),
            )
            .await?;
        let identity = serde_json::to_vec(&json!({
            "schema":"private-attachment-identity.v1", "epoch":epoch,
            "targetId":target,"sessionId":session
        }))
        .map_err(|_| FAILED)?;
        store.store_result(&reference, &identity)?;
        let detach_payload = serde_json::to_vec(&json!({
            "schema":"private-attachment-detach.v1", "attachmentReference":reference,
            "epoch":epoch,"targetId":target,"sessionId":session
        }))
        .map_err(|_| FAILED)?;
        let detach_reference = store.stage(&detach_payload)?;
        Ok(Self {
            client,
            epoch,
            target,
            session,
            detach_reference,
            detach_payload,
            state: DetachState::Attached,
        })
    }

    /// Bind to this client and active epoch/target, not a mutable journey URL
    /// digest: advancing an already authorized journey keeps session ownership.
    pub(crate) fn session_id<'a>(
        &'a self,
        client: &Arc<CdpClient>,
        permit: &PrivatePermit,
    ) -> Result<&'a str, &'static str> {
        self.validate(client, permit)?;
        if !matches!(self.state, DetachState::Attached) {
            return Err("private_attachment_not_attached");
        }
        Ok(&self.session)
    }

    fn validate(
        &self,
        client: &Arc<CdpClient>,
        permit: &PrivatePermit,
    ) -> Result<(), &'static str> {
        if !Arc::ptr_eq(&self.client, client)
            || !client.private_permit_matches(permit)
            || permit.epoch_id() != self.epoch
            || permit.target_id() != Some(self.target.as_str())
        {
            return Err(INVALID);
        }
        Ok(())
    }

    /// Verify the durable acknowledgment after sealing the transport. This is
    /// read-only evidence: it neither retries detach nor releases observation.
    pub(crate) fn verify_closed_detach(
        &self,
        client: &Arc<CdpClient>,
        permit: &PrivatePermit,
        closed: &PrivateTransportClosed,
        store: &SecretStore,
    ) -> Result<(), &'static str> {
        if !Arc::ptr_eq(&self.client, client)
            || permit.epoch_id() != self.epoch
            || permit.target_id() != Some(self.target.as_str())
            || !closed.matches(client, permit)
            || !matches!(self.state, DetachState::Completed)
        {
            return Err(INVALID);
        }
        if store.load(&self.detach_reference)? != self.detach_payload
            || store.load_result(&self.detach_reference)? != b"{\"detached\":true}"
        {
            return Err(INVALID);
        }
        Ok(())
    }

    /// Opaque durable reference; caller must verify closed detach before recording it.
    pub(crate) fn detach_reference(&self) -> &str {
        &self.detach_reference
    }

    /// Admit once before dispatch. Repetition is harmless only after both the
    /// exact empty Chrome acknowledgment and encrypted completion are committed.
    pub(crate) async fn detach(
        &mut self,
        store: &SecretStore,
        permit: &PrivatePermit,
    ) -> Result<(), &'static str> {
        self.validate(&self.client, permit)?;
        match self.state {
            DetachState::Completed => return Ok(()),
            DetachState::Uncertain => return Err("private_detach_uncertain"),
            DetachState::Attached => {}
        }
        if store.load(&self.detach_reference)? != self.detach_payload {
            return Err(INVALID);
        }
        self.state = DetachState::Uncertain;
        store.admit(&self.detach_reference)?;
        let response = self
            .client
            .send_private_command(
                permit,
                "Target.detachFromTarget",
                Some(json!({"sessionId":self.session})),
                None,
                Duration::from_secs(3),
            )
            .await?
            .into_value();
        if response != json!({}) {
            return Err("private_detach_ack_invalid");
        }
        store.store_result(&self.detach_reference, b"{\"detached\":true}")?;
        self.state = DetachState::Completed;
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::native::private_identity::{validate_private_target, LivePrivateIdentity};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    fn target() -> crate::native::private_identity::ValidatedPrivateTarget {
        let handle = json!({"profileId":"sam","browserId":"session:default",
            "sessionName":"default","tabId":"tab","targetId":"retained",
            "url":"https://secure.login.gov/","leaseId":"default","leaseState":"exclusive",
            "leaseHeartbeatExpected":true,"ownerSessionId":"default","valid":true,"staleReason":null});
        validate_private_target(
            &handle,
            &handle,
            &LivePrivateIdentity {
                profile_id: "sam",
                browser_id: "session:default",
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

    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn store() -> (Fixture, SecretStore) {
        let root =
            std::env::temp_dir().join(format!("ab-private-attachment-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        (Fixture(root), store)
    }

    async fn peer(
        detach_mode: u8,
    ) -> (
        crate::native::cdp::client::TestCdpEndpoint,
        Arc<CdpClient>,
        tokio::task::JoinHandle<Vec<String>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let mut methods = Vec::new();
            while let Some(Ok(Message::Text(text))) = ws.next().await {
                let command: Value = serde_json::from_str(&text).unwrap();
                let method = command["method"].as_str().unwrap();
                methods.push(method.to_owned());
                let result = match method {
                    "Target.attachToTarget" => {
                        assert_eq!(
                            command["params"],
                            json!({"targetId":"retained","flatten":true})
                        );
                        json!({"sessionId":"private-owned"})
                    }
                    "Target.getTargetInfo" => {
                        assert_eq!(command["sessionId"], "private-owned");
                        json!({"targetInfo":{"targetId":"retained","type":"page"}})
                    }
                    "Target.detachFromTarget" => {
                        assert_eq!(command["params"], json!({"sessionId":"private-owned"}));
                        assert!(command.get("sessionId").is_none());
                        if detach_mode == 2 {
                            continue;
                        }
                        if detach_mode == 1 {
                            json!({"unexpected":true})
                        } else {
                            json!({})
                        }
                    }
                    _ => panic!("unexpected synthetic method"),
                };
                ws.send(Message::Text(
                    json!({"id":command["id"],"result":result}).to_string(),
                ))
                .await
                .unwrap();
            }
            methods
        });
        let fixture = crate::native::cdp::client::TestCdpEndpoint::new(&endpoint).unwrap();
        let client = Arc::new(fixture.connect().await.unwrap());
        (fixture, client, task)
    }

    #[tokio::test]
    async fn exact_attachment_detaches_once_and_preserves_private_gate() {
        let (_fixture, store) = store();
        let (_endpoint_fixture, client, peer) = peer(0).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let mut attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        assert_eq!(
            attachment.session_id(&client, &permit).unwrap(),
            "private-owned"
        );
        attachment.detach(&store, &permit).await.unwrap();
        attachment.detach(&store, &permit).await.unwrap();
        assert!(attachment.session_id(&client, &permit).is_err());
        assert_eq!(
            store.status(&attachment.detach_reference).unwrap(),
            "result_ready"
        );
        assert!(client.public_lease().is_err());
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        assert!(!client.private_permit_matches(&permit));
        attachment
            .verify_closed_detach(&client, &permit, &closed, &store)
            .unwrap();
        attachment
            .verify_closed_detach(&client, &permit, &closed, &store)
            .unwrap();
        assert!(client.public_lease().is_err());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap(),
            [
                "Target.attachToTarget",
                "Target.getTargetInfo",
                "Target.getTargetInfo",
                "Target.detachFromTarget"
            ]
        );
    }

    #[tokio::test]
    async fn malformed_detach_ack_is_consumed_and_never_replayed() {
        let (_fixture, store) = store();
        let (_endpoint_fixture, client, peer) = peer(1).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let mut attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        assert_eq!(
            attachment.detach(&store, &permit).await,
            Err("private_detach_ack_invalid")
        );
        assert_eq!(
            attachment.detach(&store, &permit).await,
            Err("private_detach_uncertain")
        );
        assert_eq!(
            store.status(&attachment.detach_reference).unwrap(),
            "admitted"
        );
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap()
                .len(),
            4
        );
    }

    #[tokio::test]
    async fn foreign_client_and_permit_never_borrow_or_detach_session() {
        let (_fixture, store) = store();
        let (_endpoint_fixture, client, peer) = peer(0).await;
        let (_other_endpoint_fixture, other, other_peer) = self::peer(0).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let other_permit = other.begin_private_interval(&target()).unwrap();
        let mut attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        assert!(attachment.session_id(&other, &permit).is_err());
        assert!(attachment.session_id(&client, &other_permit).is_err());
        assert_eq!(attachment.detach(&store, &other_permit).await, Err(INVALID));
        assert_eq!(
            store.status(&attachment.detach_reference).unwrap(),
            "staged"
        );
        attachment.detach(&store, &permit).await.unwrap();
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        let other_closed = other
            .quiesce_private_transport(&other_permit)
            .await
            .unwrap();
        assert_eq!(
            attachment.verify_closed_detach(&other, &permit, &closed, &store),
            Err(INVALID)
        );
        assert_eq!(
            attachment.verify_closed_detach(&client, &other_permit, &closed, &store),
            Err(INVALID)
        );
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &other_closed, &store),
            Err(INVALID)
        );
        attachment
            .verify_closed_detach(&client, &permit, &closed, &store)
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap()
                .len(),
            4
        );
        assert!(tokio::time::timeout(Duration::from_secs(1), other_peer)
            .await
            .unwrap()
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn closed_detach_requires_exact_identity_and_durable_receipts() {
        let (_fixture, store) = store();
        let (_other_fixture, other_store) = self::store();
        let (_endpoint_fixture, client, peer) = peer(0).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let mut attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        attachment.detach(&store, &permit).await.unwrap();
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        assert!(attachment
            .verify_closed_detach(&client, &permit, &closed, &other_store)
            .is_err());

        attachment.target.push_str("-wrong");
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        attachment.target = "retained".to_owned();
        attachment.epoch.push_str("-wrong");
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        attachment.epoch = permit.epoch_id().to_owned();
        attachment.detach_payload.push(b' ');
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        attachment.detach_payload.pop();

        let exact_reference = attachment.detach_reference.clone();
        let invalid_result_reference = store.stage(&attachment.detach_payload).unwrap();
        store.admit(&invalid_result_reference).unwrap();
        attachment.detach_reference = invalid_result_reference;
        assert!(attachment
            .verify_closed_detach(&client, &permit, &closed, &store)
            .is_err());
        store
            .store_result(&attachment.detach_reference, b"{\"detached\":false}")
            .unwrap();
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        attachment.detach_reference = exact_reference;
        attachment
            .verify_closed_detach(&client, &permit, &closed, &store)
            .unwrap();
        assert!(client.public_lease().is_err());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "Target.detachFromTarget")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn closed_transport_without_detach_is_not_completion() {
        let (_fixture, store) = store();
        let (_endpoint_fixture, client, peer) = peer(0).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        assert!(client.public_lease().is_err());
        assert!(!tokio::time::timeout(Duration::from_secs(1), peer)
            .await
            .unwrap()
            .unwrap()
            .iter()
            .any(|method| method == "Target.detachFromTarget"));
    }

    #[tokio::test]
    async fn cancelled_detach_retains_consumption_and_never_replays() {
        let (_fixture, store) = store();
        let (_endpoint_fixture, client, peer) = peer(2).await;
        let permit = client.begin_private_interval(&target()).unwrap();
        let mut attachment = PrivateAttachment::acquire(Arc::clone(&client), &store, &permit)
            .await
            .unwrap();
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            attachment.detach(&store, &permit)
        )
        .await
        .is_err());
        assert_eq!(
            attachment.detach(&store, &permit).await,
            Err("private_detach_uncertain")
        );
        assert_eq!(
            store.status(&attachment.detach_reference).unwrap(),
            "admitted"
        );
        assert!(client.public_lease().is_err());
        let closed = client.quiesce_private_transport(&permit).await.unwrap();
        assert_eq!(
            attachment.verify_closed_detach(&client, &permit, &closed, &store),
            Err(INVALID)
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), peer)
                .await
                .unwrap()
                .unwrap()
                .len(),
            4
        );
    }
}
