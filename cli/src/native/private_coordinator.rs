//! Opt-in daemon-owned coordinator for one independently approved private plan.
//! The execution socket cannot provision authority, launch a browser, choose a
//! recipe, or recover/replay an uncertain dispatch. Ordinary commands never carry
//! credential material. Missing configuration leaves this surface disabled.

use super::control_plane::ControlPlaneHandle;
use super::private_bound_execution;
use super::private_controller::RecoveryController;
use super::private_controller_socket::coordinator_files;
use super::private_handoff::{receive_connected, PrivateHandoffAuthority};
use super::private_journey::validate_sequence;
use super::private_operation::PrivateOperation;
use super::private_secret_store::SecretStore;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

const FAILED: &str = "private_coordinator_failed_closed";
const PASSWORD_SLOT: &str = "PRIVATE_PASSWORD_SLOT";
const CODE_SLOT: &str = "PRIVATE-CODE-SLOT";
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Authority {
    schema: String,
    plan_id: String,
    manifest_sha256: String,
    recipe_sha256: String,
    endpoint: String,
    session_name: String,
    expires_at: String,
    operations: Vec<Value>,
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Authority {
    fn parse(bytes: &[u8], session: &str) -> Result<Self> {
        // Validate nested field uniqueness before Value would normalize it.
        #[derive(Deserialize)]
        struct Nested {
            operations: Vec<PrivateOperation>,
        }
        let nested: Nested = serde_json::from_slice(bytes).map_err(|_| FAILED)?;
        let authority: Self = serde_json::from_slice(bytes).map_err(|_| FAILED)?;
        if authority.schema != "agent-browser.private-coordinator.v1"
            || !hex(&authority.plan_id, 32)
            || !hex(&authority.manifest_sha256, 64)
            || !hex(&authority.recipe_sha256, 64)
            || authority.session_name != session
            || authority.operations.len() != 4
            || nested.operations.len() != 4
            || authority.operations[0]["operation"]["password"] != PASSWORD_SLOT
            || authority.operations[1]["operation"]["code"] != CODE_SLOT
        {
            return Err(FAILED);
        }
        let canonical = serde_json::to_vec(&authority.operations).map_err(|_| FAILED)?;
        if hex::encode(Sha256::digest(canonical)) != authority.recipe_sha256 {
            return Err(FAILED);
        }
        let parsed = authority.parsed_operations()?;
        validate_sequence(&parsed).map_err(|_| FAILED)?;
        let first = &parsed[0];
        for op in &parsed {
            if op.consent_sha256() != authority.manifest_sha256
                || op.account_scope() != first.account_scope()
                || op.service_tab_handle()["sessionName"].as_str() != Some(session)
                || op.service_tab_handle()["browserId"].as_str()
                    != Some(format!("session:{session}").as_str())
            {
                return Err(FAILED);
            }
        }
        let url = url::Url::parse(&authority.endpoint).map_err(|_| FAILED)?;
        if !matches!(url.scheme(), "ws" | "wss")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || authority
                .endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(FAILED);
        }
        authority.expiry()?;
        Ok(authority)
    }

    fn parsed_operations(&self) -> Result<Vec<PrivateOperation>> {
        self.operations
            .iter()
            .map(|op| {
                PrivateOperation::parse(&serde_json::to_vec(op).map_err(|_| FAILED)?)
                    .map_err(|_| FAILED)
            })
            .collect()
    }

    fn expiry(&self) -> Result<SystemTime> {
        let expires = chrono::DateTime::parse_from_rfc3339(&self.expires_at).map_err(|_| FAILED)?;
        if expires <= chrono::Utc::now() {
            return Err(FAILED);
        }
        Ok(expires.with_timezone(&chrono::Utc).into())
    }

    fn tag(&self, key: &[u8; 32], domain: &[u8], nonce: &[u8; 32]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("fixed key length");
        mac.update(domain);
        mac.update(nonce);
        mac.update(self.plan_id.as_bytes());
        mac.update(self.manifest_sha256.as_bytes());
        mac.update(self.recipe_sha256.as_bytes());
        mac.finalize().into_bytes().into()
    }

    fn validate_received(&self, store: &SecretStore, references: &[String]) -> Result<()> {
        let mut received = Vec::new();
        for reference in references {
            received.push(
                serde_json::from_slice::<Value>(&store.load(reference).map_err(|_| FAILED)?)
                    .map_err(|_| FAILED)?,
            );
        }
        if received.len() != 4 {
            return Err(FAILED);
        }
        received[0]["operation"]["password"] = Value::String(PASSWORD_SLOT.into());
        received[1]["operation"]["code"] = Value::String(CODE_SLOT.into());
        if received != self.operations {
            return Err(FAILED);
        }
        self.expiry()?;
        Ok(())
    }
}

/// All authority is loaded from a separate private provisioned record, never
/// from the network payload. The record is rechecked before dispatch admission.
async fn execute(
    mut stream: UnixStream,
    control_plane: &ControlPlaneHandle,
    root: &Path,
    session: &str,
) -> Result<()> {
    if stream.peer_cred().map_err(|_| FAILED)?.uid() != unsafe { libc::geteuid() } {
        return Err(FAILED);
    }
    let (_directory, key, bytes) = coordinator_files(root).map_err(|_| FAILED)?;
    let authority = Authority::parse(&bytes, session)?;
    let parsed = authority.parsed_operations()?;
    control_plane
        .private_preflight(
            parsed[0].service_tab_handle().clone(),
            parsed[0].expected_origin().to_owned(),
            parsed[0].expected_url().to_owned(),
            authority.endpoint.clone(),
        )
        .await?;
    let mut nonce = [0; 32];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut nonce))
        .await
        .map_err(|_| FAILED)?
        .map_err(|_| FAILED)?;
    let proof = authority.tag(&key, b"ABPX1-server\0", &nonce);
    stream.write_all(&proof).await.map_err(|_| FAILED)?;
    let operations = Arc::new(SecretStore::open(&root.join("operations")).map_err(|_| FAILED)?);
    let handoff = PrivateHandoffAuthority::new(
        authority.manifest_sha256.clone(),
        parsed[0].account_scope().to_owned(),
        parsed[0].service_tab_handle().clone(),
        parsed
            .iter()
            .map(|op| {
                (
                    op.expected_origin().to_owned(),
                    op.expected_url().to_owned(),
                )
            })
            .collect(),
        key,
        authority.expiry()?,
    )
    .map_err(|_| FAILED)?;
    let staged = receive_connected(&mut stream, handoff, Arc::clone(&operations))
        .await
        .map_err(|_| FAILED)?;
    // Compare exact reviewed selectors/actions/source scope in addition to the
    // ingress account, origin, target and sequence checks, before any CDP work.
    authority.validate_received(&operations, staged.references())?;
    let (_, current_key, current_bytes) = coordinator_files(root).map_err(|_| FAILED)?;
    if bytes != current_bytes || key != current_key {
        return Err(FAILED);
    }
    let bindings = Arc::new(SecretStore::open(&root.join("bindings")).map_err(|_| FAILED)?);
    let binding = private_bound_execution::claim(
        bindings,
        &authority.plan_id,
        &authority.manifest_sha256,
        &staged,
        &authority.endpoint,
        session,
    )
    .map_err(|_| FAILED)?;
    let controller = Arc::new(
        RecoveryController::new(
            Arc::new(SecretStore::open(&root.join("recovery")).map_err(|_| FAILED)?),
            authority.endpoint.clone(),
            session.to_owned(),
        )
        .map_err(|_| FAILED)?,
    );
    let reconciled = control_plane
        .submit_bound_private(staged, controller, binding)
        .await
        .map_err(|_| FAILED)?;
    authority.expiry()?;
    let completion = authority.tag(&key, b"ABPX1-complete\0", &nonce);
    stream.write_all(&completion).await.map_err(|_| FAILED)?;
    reconciled.deliver(stream, key).await.map_err(|_| FAILED)
}

struct SocketOwner {
    _directory: File,
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl Drop for SocketOwner {
    fn drop(&mut self) {
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path) {
            if metadata.file_type().is_socket()
                && metadata.dev() == self.dev
                && metadata.ino() == self.ino
            {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}

pub(crate) struct CoordinatorTask(tokio::task::JoinHandle<()>);
impl Drop for CoordinatorTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) fn start_configured(
    control_plane: &ControlPlaneHandle,
    session: &str,
) -> Result<Option<CoordinatorTask>> {
    let Some(root) = std::env::var_os("AGENT_BROWSER_PRIVATE_EXECUTOR_ROOT") else {
        return Ok(None);
    };
    start(control_plane, session, PathBuf::from(root)).map(Some)
}

fn start(
    control_plane: &ControlPlaneHandle,
    session: &str,
    root: PathBuf,
) -> Result<CoordinatorTask> {
    let (directory, _, bytes) = coordinator_files(&root).map_err(|_| FAILED)?;
    Authority::parse(&bytes, session)?;
    let path = PathBuf::from(format!(
        "/proc/self/fd/{}/executor.sock",
        directory.as_raw_fd()
    ));
    // Existing entries are never deleted, reused or treated as stale implicitly.
    let listener = UnixListener::bind(&path).map_err(|_| FAILED)?;
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| FAILED)?;
    let owner = SocketOwner {
        _directory: directory,
        path: path.clone(),
        dev: metadata.dev(),
        ino: metadata.ino(),
    };
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|_| FAILED)?;
    let control_plane = control_plane.clone();
    let session = session.to_owned();
    Ok(CoordinatorTask(tokio::spawn(async move {
        let _owner = owner;
        while let Ok((stream, _)) = listener.accept().await {
            // One private client at a time, bounded including stalled handshakes.
            // Errors never reach ordinary logs and never trigger another attempt.
            let _ = tokio::time::timeout(
                Duration::from_secs(240),
                execute(stream, &control_plane, &root, &session),
            )
            .await;
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        let steps = [
            json!({"kind":"login","email_selector":"#email","password_selector":"#password","submit_selector":"#submit","email":"user@example.test","password":PASSWORD_SLOT}),
            json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit","account_selector":"#account","account_value":"user@example.test","code":CODE_SLOT,"source_scope":"C123:123.000001"}),
            json!({"kind":"renew","submit_selector":"#renew","account_selector":"#account","account_value":"user@example.test"}),
            json!({"kind":"read_key","selector":"#key","account_selector":"#account","account_value":"user@example.test"}),
        ];
        let operations: Vec<Value> = steps.into_iter().enumerate().map(|(i, operation)| {
            let origin = if i < 2 {"https://secure.login.gov"} else {"https://sam.gov"};
            json!({"schema":"agent-browser.private-operation.v1", "service_tab_handle":{
                "profileId":"sam", "browserId":"session:default", "sessionName":"default", "targetId":"A".repeat(32),
                "tabId":"tab", "url":format!("{origin}/"), "leaseId":"default", "leaseState":"exclusive",
                "leaseHeartbeatExpected":true,"ownerSessionId":"default","valid":true,"staleReason":null},
                "expected_origin":origin,"expected_url":format!("{origin}/"),"consent_sha256":"a".repeat(64),
                "account_scope":"login.gov:user@example.test", "operation":operation})
        }).collect();
        json!({"schema":"agent-browser.private-coordinator.v1","planId":"b".repeat(32),"manifestSha256":"a".repeat(64),
            "recipeSha256":hex::encode(Sha256::digest(serde_json::to_vec(&operations).unwrap())),"endpoint":"ws://127.0.0.1:9222/devtools/browser/synthetic",
            "sessionName":"default", "expiresAt":"2030-01-01T00:00:00Z","operations":operations})
    }

    #[test]
    fn independent_recipe_schema_scope_and_digest_are_required() {
        let value = fixture();
        assert!(Authority::parse(&serde_json::to_vec(&value).unwrap(), "default").is_ok());
        for field in [
            "schema",
            "planId",
            "manifestSha256",
            "recipeSha256",
            "endpoint",
            "sessionName",
            "expiresAt",
        ] {
            let mut changed = value.clone();
            changed[field] = json!("invalid");
            assert!(Authority::parse(&serde_json::to_vec(&changed).unwrap(), "default").is_err());
        }
        let mut changed = value;
        changed["operations"][2]["operation"]["submit_selector"] = json!("#unreviewed");
        assert!(Authority::parse(&serde_json::to_vec(&changed).unwrap(), "default").is_err());
    }

    #[test]
    fn incoming_secrets_cannot_change_reviewed_recipe() {
        let authority =
            Authority::parse(&serde_json::to_vec(&fixture()).unwrap(), "default").unwrap();
        let root = std::env::temp_dir().join(format!("ab-coordinator-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        let mut values = authority.operations.clone();
        values[0]["operation"]["password"] = json!("synthetic-password");
        values[1]["operation"]["code"] = json!("SYNTHETIC-CODE");
        let stage = |ops: &[Value]| {
            ops.iter()
                .map(|v| store.stage(&serde_json::to_vec(v).unwrap()).unwrap())
                .collect::<Vec<_>>()
        };
        assert!(authority.validate_received(&store, &stage(&values)).is_ok());
        for (field, value) in [
            ("submit_selector", "#other"),
            ("source_scope", "other:123.000001"),
            ("account_value", "other@example.test"),
        ] {
            let mut changed = values.clone();
            changed[1]["operation"][field] = json!(value);
            assert!(authority
                .validate_received(&store, &stage(&changed))
                .is_err());
        }
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn missing_retained_browser_never_emits_readiness_or_stages_secrets() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let root =
            std::env::temp_dir().join(format!("ab-coordinator-preflight-{}", uuid::Uuid::new_v4()));
        SecretStore::ensure_private_directory(&root).unwrap();
        for (name, bytes) in [
            ("execution.json", serde_json::to_vec(&fixture()).unwrap()),
            ("authentication.key", vec![7; 32]),
        ] {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(root.join(name))
                .unwrap();
            file.write_all(&bytes).unwrap();
            file.sync_all().unwrap();
        }
        let worker = super::super::control_plane::ControlPlaneWorker::start(
            super::super::actions::DaemonState::new(),
        );
        let task = start(&worker, "default", root.clone()).unwrap();
        assert!(start(&worker, "default", root.clone()).is_err());
        let mut peer = UnixStream::connect(root.join("executor.sock"))
            .await
            .unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), peer.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert!(bytes.is_empty());
        assert!(!root.join("operations").exists());
        assert!(!root.join("bindings").exists());
        drop(task);
        worker.shutdown().await;
        tokio::task::yield_now().await;
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Full socket/worker fixture called by the existing synthetic retained-CDP test.
#[cfg(test)]
pub(crate) async fn test_exchange(
    control_plane: &ControlPlaneHandle,
    root: &Path,
    values: &[Value],
    endpoint: &str,
    expires: &str,
) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut template = values.to_vec();
    template[0]["operation"]["password"] = Value::String(PASSWORD_SLOT.into());
    template[1]["operation"]["code"] = Value::String(CODE_SLOT.into());
    let record = serde_json::json!({"schema":"agent-browser.private-coordinator.v1", "planId":"b".repeat(32),
        "manifestSha256":"a".repeat(64), "recipeSha256":hex::encode(Sha256::digest(serde_json::to_vec(&template).unwrap())),
        "sessionName":"default", "endpoint":endpoint, "expiresAt":expires, "operations":template});
    for (name, bytes) in [
        ("execution.json", serde_json::to_vec(&record).unwrap()),
        ("authentication.key", vec![7; 32]),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(name))
            .unwrap();
        file.write_all(&bytes).unwrap();
        file.sync_all().unwrap();
    }
    let task = start(control_plane, "default", root.to_owned()).unwrap();
    let mut consumer = UnixStream::connect(root.join("executor.sock"))
        .await
        .unwrap();
    let nonce = [4; 32];
    consumer.write_all(&nonce).await.unwrap();
    let mut proof = [0; 32];
    consumer.read_exact(&mut proof).await.unwrap();
    let authority = Authority::parse(&serde_json::to_vec(&record).unwrap(), "default").unwrap();
    assert_eq!(proof, authority.tag(&[7; 32], b"ABPX1-server\0", &nonce));
    // Regression: source extraction taking longer than the old five-second
    // ingress window still reaches one dispatch, without any real provider.
    tokio::time::sleep(Duration::from_millis(5100)).await;
    let body = serde_json::to_vec(values).unwrap();
    let mut wire = b"ABPH1\0".to_vec();
    wire.extend_from_slice(&(body.len() as u32).to_be_bytes());
    wire.extend_from_slice(&body);
    let mut mac = Hmac::<Sha256>::new_from_slice(&[7; 32]).unwrap();
    mac.update(&wire);
    wire.extend_from_slice(&mac.finalize().into_bytes());
    consumer.write_all(&wire).await.unwrap();
    consumer.read_exact(&mut proof).await.unwrap();
    assert_eq!(proof, authority.tag(&[7; 32], b"ABPX1-complete\0", &nonce));
    super::private_delivery::tests::receiver(consumer, false, "SYNTHETIC_PRIVATE_KEY_123456789")
        .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if std::fs::read_dir(root.join("operations"))
                .unwrap()
                .any(|e| {
                    e.unwrap()
                        .path()
                        .extension()
                        .is_some_and(|e| e == "delivery")
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    drop(task);
    tokio::time::timeout(Duration::from_secs(2), async {
        while root.join("executor.sock").exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
