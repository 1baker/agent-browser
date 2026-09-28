//! Private connected-socket ingress only. This module opens no listener and
//! creates no consent authority. The caller supplies independently approved
//! scope and a private authentication key; ordinary commands cannot do so.

#![cfg(unix)]

use super::private_identity::{validate_private_target, LivePrivateIdentity};
use super::private_journey::validate_sequence;
use super::private_operation::PrivateOperation;
use super::private_secret_store::SecretStore;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{io::AsyncReadExt, net::UnixStream};

const MAGIC: &[u8; 6] = b"ABPH1\0";
const MAX_BODY: usize = 262_144;
const FAILED: &str = "private_handoff_rejected";

/// Trusted independent approval binding, never deserialized from wire input.
/// No Debug, Serialize or Clone implementation may expose or duplicate its key.
pub(crate) struct PrivateHandoffAuthority {
    consent_sha256: String,
    account_scope: String,
    initial_handle: Value,
    destinations: Vec<(String, String)>,
    authentication_key: [u8; 32],
    expires_at: SystemTime,
}

impl PrivateHandoffAuthority {
    pub(crate) fn new(
        consent_sha256: String,
        account_scope: String,
        initial_handle: Value,
        destinations: Vec<(String, String)>,
        authentication_key: [u8; 32],
        expires_at: SystemTime,
    ) -> Result<Self, &'static str> {
        if consent_sha256.len() != 64
            || !consent_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !account_scope.starts_with("login.gov:")
            || destinations.len() != 4
            || SystemTime::now() >= expires_at
        {
            return Err(FAILED);
        }
        let authority = Self {
            consent_sha256,
            account_scope,
            initial_handle,
            destinations,
            authentication_key,
            expires_at,
        };
        authority.validate_initial()?;
        Ok(authority)
    }

    fn validate_initial(&self) -> Result<(), &'static str> {
        let field = |name| {
            self.initial_handle
                .get(name)
                .and_then(Value::as_str)
                .ok_or(FAILED)
        };
        let (origin, url) = &self.destinations[0];
        validate_private_target(
            &self.initial_handle,
            &self.initial_handle,
            &LivePrivateIdentity {
                profile_id: field("profileId")?,
                browser_id: field("browserId")?,
                session_name: field("sessionName")?,
                target_id: field("targetId")?,
                url,
                ready: true,
            },
            origin,
            url,
        )
        .map_err(|_| FAILED)?;
        Ok(())
    }
}

/// Only opaque encrypted-store references cross into the daemon worker. These
/// are not proof of live browser authority or permission to skip expiry checks.
pub(crate) struct PrivateStagedHandoff {
    store: Arc<SecretStore>,
    references: Vec<String>,
    expires_at: SystemTime,
}

impl PrivateStagedHandoff {
    pub(crate) fn references(&self) -> &[String] {
        &self.references
    }
    pub(crate) fn operation_store(&self) -> &SecretStore {
        &self.store
    }

    pub(crate) fn binding_operations(&self) -> Result<Vec<PrivateOperation>, &'static str> {
        self.references
            .iter()
            .map(|reference| {
                PrivateOperation::parse(&self.store.load(reference)?).map_err(|_| FAILED)
            })
            .collect()
    }

    pub(crate) fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
    pub(crate) fn is_current(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
    pub(crate) fn into_parts(self) -> (Arc<SecretStore>, Vec<String>, SystemTime) {
        (self.store, self.references, self.expires_at)
    }
}

/// Read exactly one authenticated frame and EOF from a same-user connected
/// peer. MAC verification precedes JSON parsing. The entire receive deadline
/// includes blocking staging, which cannot dispatch anything if canceled.
pub(crate) async fn receive(
    mut stream: UnixStream,
    authority: PrivateHandoffAuthority,
    store: Arc<SecretStore>,
) -> Result<PrivateStagedHandoff, &'static str> {
    receive_frame(&mut stream, authority, store, true).await
}

/// Coordinator-only duplex ingress. A single length/MAC-delimited frame leaves
/// the connection available for completion and authenticated private delivery.
pub(crate) async fn receive_connected(
    stream: &mut UnixStream,
    authority: PrivateHandoffAuthority,
    store: Arc<SecretStore>,
) -> Result<PrivateStagedHandoff, &'static str> {
    receive_frame(stream, authority, store, false).await
}

async fn receive_frame(
    stream: &mut UnixStream,
    authority: PrivateHandoffAuthority,
    store: Arc<SecretStore>,
    require_eof: bool,
) -> Result<PrivateStagedHandoff, &'static str> {
    let receive = async move {
        if stream.peer_cred().map_err(|_| FAILED)?.uid() != unsafe { libc::geteuid() }
            || SystemTime::now() >= authority.expires_at
        {
            return Err(FAILED);
        }
        let mut header = [0u8; 10];
        stream.read_exact(&mut header).await.map_err(|_| FAILED)?;
        if &header[..6] != MAGIC {
            return Err(FAILED);
        }
        let length = u32::from_be_bytes(header[6..].try_into().map_err(|_| FAILED)?) as usize;
        if length == 0 || length > MAX_BODY {
            return Err(FAILED);
        }
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).await.map_err(|_| FAILED)?;
        let mut tag = [0u8; 32];
        stream.read_exact(&mut tag).await.map_err(|_| FAILED)?;
        let mut trailing = [0u8; 1];
        if require_eof && stream.read(&mut trailing).await.map_err(|_| FAILED)? != 0 {
            return Err(FAILED);
        }
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&authority.authentication_key).map_err(|_| FAILED)?;
        mac.update(&header);
        mac.update(&body);
        mac.verify_slice(&tag).map_err(|_| FAILED)?;
        tokio::task::spawn_blocking(move || stage_validated(body, authority, store))
            .await
            .map_err(|_| FAILED)?
    };
    // Duplex coordinator peers authenticate the server before refetching their
    // approved private source, so allow a bounded source-fetch window there.
    tokio::time::timeout(
        Duration::from_secs(if require_eof { 5 } else { 60 }),
        receive,
    )
    .await
    .map_err(|_| FAILED)?
}

fn stage_validated(
    body: Vec<u8>,
    authority: PrivateHandoffAuthority,
    store: Arc<SecretStore>,
) -> Result<PrivateStagedHandoff, &'static str> {
    // Check typed field uniqueness before Value normalization could erase
    // duplicate operation fields. Semantic validation follows below.
    let _: Vec<PrivateOperation> = serde_json::from_slice(&body).map_err(|_| FAILED)?;
    let values: Vec<Value> = serde_json::from_slice(&body).map_err(|_| FAILED)?;
    if values.len() != 4 {
        return Err(FAILED);
    }
    let payloads: Vec<Vec<u8>> = values
        .iter()
        .map(|value| serde_json::to_vec(value).map_err(|_| FAILED))
        .collect::<Result<_, _>>()?;
    let operations = payloads
        .iter()
        .map(|payload| PrivateOperation::parse(payload).map_err(|_| FAILED))
        .collect::<Result<Vec<_>, _>>()?;
    validate_sequence(&operations).map_err(|_| FAILED)?;
    for (operation, (origin, url)) in operations.iter().zip(&authority.destinations) {
        if operation.consent_sha256() != authority.consent_sha256
            || operation.account_scope() != authority.account_scope
            || operation.expected_origin() != origin
            || operation.expected_url() != url
        {
            return Err(FAILED);
        }
    }
    let field = |name| {
        authority
            .initial_handle
            .get(name)
            .and_then(Value::as_str)
            .ok_or(FAILED)
    };
    let first = &operations[0];
    validate_private_target(
        first.service_tab_handle(),
        &authority.initial_handle,
        &LivePrivateIdentity {
            profile_id: field("profileId")?,
            browser_id: field("browserId")?,
            session_name: field("sessionName")?,
            target_id: field("targetId")?,
            url: first.expected_url(),
            ready: true,
        },
        first.expected_origin(),
        first.expected_url(),
    )
    .map_err(|_| FAILED)?;
    let mut references = Vec::new();
    for payload in &payloads {
        if SystemTime::now() >= authority.expires_at {
            return Err(FAILED);
        }
        references.push(store.stage(payload).map_err(|_| FAILED)?);
    }
    if SystemTime::now() >= authority.expires_at {
        return Err(FAILED);
    }
    Ok(PrivateStagedHandoff {
        store,
        references,
        expires_at: authority.expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::AsyncWriteExt;

    const KEY: [u8; 32] = [17; 32];
    fn payloads() -> Vec<Value> {
        [
            json!({"kind":"login","email_selector":"#email","password_selector":"#password",
                "submit_selector":"#submit","email":"user@example.test","password":"PRIVATE_HANDOFF_SENTINEL"}),
            json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit",
                "account_selector":"#account","account_value":"user@example.test","code":"SYNTHETIC-CODE","source_scope":"synthetic:1"}),
            json!({"kind":"renew","submit_selector":"#renew","account_selector":"#account","account_value":"user@example.test"}),
            json!({"kind":"read_key","selector":"#key","account_selector":"#account","account_value":"user@example.test"}),
        ].into_iter().enumerate().map(|(index, operation)| {
            let origin = if index < 2 { "https://secure.login.gov" } else { "https://sam.gov" };
            json!({"schema":"agent-browser.private-operation.v1", "service_tab_handle":{
                "profileId":"sam","browserId":"session:default","sessionName":"default",
                "tabId":"tab","targetId":"retained","url":format!("{origin}/"),
                "leaseId":"default","leaseState":"exclusive","leaseHeartbeatExpected":true,
                "ownerSessionId":"default","valid":true,"staleReason":null},
                "expected_origin":origin,"expected_url":format!("{origin}/"),
                "consent_sha256":"a".repeat(64),"account_scope":"login.gov:user@example.test","operation":operation})
        }).collect()
    }
    fn authority() -> PrivateHandoffAuthority {
        let values = payloads();
        PrivateHandoffAuthority::new(
            "a".repeat(64),
            "login.gov:user@example.test".into(),
            values[0]["service_tab_handle"].clone(),
            values
                .iter()
                .map(|value| {
                    (
                        value["expected_origin"].as_str().unwrap().to_owned(),
                        value["expected_url"].as_str().unwrap().to_owned(),
                    )
                })
                .collect(),
            KEY,
            SystemTime::now() + Duration::from_secs(30),
        )
        .unwrap()
    }
    fn wire(values: &[Value]) -> Vec<u8> {
        let body = serde_json::to_vec(values).unwrap();
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&body);
        let mut mac = Hmac::<Sha256>::new_from_slice(&KEY).unwrap();
        mac.update(&bytes);
        bytes.extend_from_slice(&mac.finalize().into_bytes());
        bytes
    }
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn store() -> (Fixture, Arc<SecretStore>) {
        let root =
            std::env::temp_dir().join(format!("ab-private-handoff-{}", uuid::Uuid::new_v4()));
        let store = Arc::new(SecretStore::open(&root).unwrap());
        (Fixture(root), store)
    }
    async fn submit(
        bytes: Vec<u8>,
        authority: PrivateHandoffAuthority,
        store: Arc<SecretStore>,
    ) -> Result<PrivateStagedHandoff, &'static str> {
        let (mut writer, reader) = UnixStream::pair().unwrap();
        let sender = tokio::spawn(async move {
            let _ = writer.write_all(&bytes).await;
            let _ = writer.shutdown().await;
        });
        let result = receive(reader, authority, store).await;
        sender.await.unwrap();
        result
    }

    #[tokio::test]
    async fn authenticated_handoff_stages_only_ciphertext_and_returns_opaque_references() {
        let (fixture, store) = store();
        let staged = submit(wire(&payloads()), authority(), Arc::clone(&store))
            .await
            .unwrap();
        assert!(staged.is_current());
        assert!(staged.expires_at() > SystemTime::now());
        let (returned, refs, _) = staged.into_parts();
        assert!(Arc::ptr_eq(&returned, &store));
        assert_eq!(refs.len(), 4);
        for reference in refs {
            assert_eq!(reference.len(), 64);
            assert_eq!(store.status(&reference).unwrap(), "staged");
        }
        for entry in std::fs::read_dir(&fixture.0).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert!(!bytes
                .windows(b"PRIVATE_HANDOFF_SENTINEL".len())
                .any(|part| part == b"PRIVATE_HANDOFF_SENTINEL"));
        }
    }

    #[tokio::test]
    async fn rejects_authenticated_cross_scope_without_staging() {
        for kind in ["consent", "target", "url", "account"] {
            let (fixture, store) = store();
            let before = std::fs::read_dir(&fixture.0).unwrap().count();
            let mut values = payloads();
            for value in &mut values {
                match kind {
                    "consent" => value["consent_sha256"] = json!("b".repeat(64)),
                    "target" => value["service_tab_handle"]["targetId"] = json!("other"),
                    "url" => {
                        let url = format!("{}/wrong", value["expected_origin"].as_str().unwrap());
                        value["expected_url"] = json!(url);
                        value["service_tab_handle"]["url"] = json!(url);
                    }
                    "account" => {
                        value["account_scope"] = json!("login.gov:other@example.test");
                        if value["operation"]["kind"] == "login" {
                            value["operation"]["email"] = json!("other@example.test");
                        } else {
                            value["operation"]["account_value"] = json!("other@example.test");
                        }
                    }
                    _ => unreachable!(),
                }
            }
            assert!(matches!(
                submit(wire(&values), authority(), store).await,
                Err(FAILED)
            ));
            assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), before);
        }
    }

    #[tokio::test]
    async fn rejects_bad_mac_expiry_trailing_truncated_and_oversized_frames() {
        for kind in ["mac", "expired", "trailing", "truncated", "oversized"] {
            let (fixture, store) = store();
            let before = std::fs::read_dir(&fixture.0).unwrap().count();
            let mut authority = authority();
            let mut bytes = wire(&payloads());
            match kind {
                "mac" => *bytes.last_mut().unwrap() ^= 1,
                "expired" => authority.expires_at = SystemTime::UNIX_EPOCH,
                "trailing" => bytes.push(0),
                "truncated" => bytes.truncate(bytes.len() - 1),
                "oversized" => bytes[6..10].copy_from_slice(&((MAX_BODY + 1) as u32).to_be_bytes()),
                _ => unreachable!(),
            }
            assert!(matches!(submit(bytes, authority, store).await, Err(FAILED)));
            assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), before);
        }
    }

    #[tokio::test]
    async fn expiry_during_eof_wait_prevents_staging() {
        let (fixture, store) = store();
        let before = std::fs::read_dir(&fixture.0).unwrap().count();
        let mut authority = authority();
        let (mut writer, reader) = UnixStream::pair().unwrap();
        writer.write_all(&wire(&payloads())).await.unwrap();
        reader.readable().await.unwrap();
        authority.expires_at = SystemTime::now() + Duration::from_millis(100);
        let expires_at = authority.expires_at;
        let mut receiving = Box::pin(receive(reader, authority, store));
        // The entire frame is readable, but EOF is withheld. Prove admission
        // has not completed instead of racing a separately scheduled sender.
        assert!(futures_util::poll!(&mut receiving).is_pending());
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), before);

        // Tokio's monotonic sleep is not evidence that SystemTime expired.
        // Observe the same clock as the authority, bounded against clock stalls.
        let wait_started = std::time::Instant::now();
        while SystemTime::now() < expires_at {
            assert!(wait_started.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        writer.shutdown().await.unwrap();
        assert!(matches!(receiving.await, Err(FAILED)));
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), before);
    }

    #[test]
    fn expired_validated_frame_never_stages() {
        let (fixture, store) = store();
        let before = std::fs::read_dir(&fixture.0).unwrap().count();
        let mut authority = authority();
        authority.expires_at = SystemTime::UNIX_EPOCH;
        assert!(matches!(
            stage_validated(serde_json::to_vec(&payloads()).unwrap(), authority, store),
            Err(FAILED)
        ));
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), before);
    }
}
