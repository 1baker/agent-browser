//! Isolated private-queue regressions. No real browser or public job submission.

use super::*;
use crate::native::private_handoff::{receive, PrivateHandoffAuthority, PrivateStagedHandoff};
use crate::native::private_secret_store::SecretStore;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::SystemTime;
use tokio::{io::AsyncWriteExt, net::UnixStream};

fn isolated() {
    assert_eq!(
        std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref(),
        Ok("1"),
        "run only through the isolated private test runner"
    );
}

struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn staged(ttl: Duration) -> (Fixture, Arc<SecretStore>, PrivateStagedHandoff) {
    let root =
        std::env::temp_dir().join(format!("ab-private-worker-test-{}", uuid::Uuid::new_v4()));
    let store = Arc::new(SecretStore::open(&root).unwrap());
    let values = [
        json!({"kind":"login","email_selector":"#email","password_selector":"#password","submit_selector":"#submit",
            "email":"user@example.test","password":"WORKER_PRIVATE_SENTINEL"}),
        json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit","account_selector":"#account",
            "account_value":"user@example.test","code":"SYNTHETIC-CODE","source_scope":"synthetic:1"}),
        json!({"kind":"renew","submit_selector":"#renew","account_selector":"#account","account_value":"user@example.test"}),
        json!({"kind":"read_key","selector":"#key","account_selector":"#account","account_value":"user@example.test"}),
    ].into_iter().enumerate().map(|(index, operation)| {
        let origin=if index<2 {"https://secure.login.gov"} else {"https://sam.gov"};
        json!({"schema":"agent-browser.private-operation.v1","service_tab_handle":{
            "profileId":"sam","browserId":"session:default","sessionName":"default","tabId":"tab","targetId":"retained",
            "url":format!("{origin}/"),"leaseId":"default","leaseState":"exclusive","leaseHeartbeatExpected":true,
            "ownerSessionId":"default","valid":true,"staleReason":null},"expected_origin":origin,"expected_url":format!("{origin}/"),
            "consent_sha256":"a".repeat(64),"account_scope":"login.gov:user@example.test","operation":operation})
    }).collect::<Vec<_>>();
    let key = [21u8; 32];
    let authority = PrivateHandoffAuthority::new(
        "a".repeat(64),
        "login.gov:user@example.test".into(),
        values[0]["service_tab_handle"].clone(),
        values
            .iter()
            .map(|value| {
                (
                    value["expected_origin"].as_str().unwrap().into(),
                    value["expected_url"].as_str().unwrap().into(),
                )
            })
            .collect(),
        key,
        SystemTime::now() + ttl,
    )
    .unwrap();
    let body = serde_json::to_vec(&values).unwrap();
    let mut wire = b"ABPH1\0".to_vec();
    wire.extend_from_slice(&(body.len() as u32).to_be_bytes());
    wire.extend_from_slice(&body);
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    mac.update(&wire);
    wire.extend_from_slice(&mac.finalize().into_bytes());
    let (mut writer, reader) = UnixStream::pair().unwrap();
    let sender = tokio::spawn(async move {
        writer.write_all(&wire).await.unwrap();
        writer.shutdown().await.unwrap();
    });
    let handoff = receive(reader, authority, Arc::clone(&store))
        .await
        .unwrap();
    sender.await.unwrap();
    (Fixture(root), store, handoff)
}

fn channel(capacity: usize) -> (ControlPlaneHandle, mpsc::Receiver<WorkerMessage>) {
    let (tx, rx) = mpsc::channel(capacity);
    (
        ControlPlaneHandle {
            tx,
            status: Arc::new(ControlPlaneStatus::new()),
            service_job_timeout_ms: None,
            service_monitor_interval_ms: None,
            running_cancellations: Arc::new(Mutex::new(HashMap::new())),
        },
        rx,
    )
}

fn jobs() -> Value {
    let repository = LockedServiceStateRepository::default_json().unwrap();
    serde_json::to_value(repository.load_snapshot().unwrap().jobs).unwrap()
}

async fn until_queue(handle: &ControlPlaneHandle, expected: usize) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while handle.queue_depth() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn start_existing(
    handle: &ControlPlaneHandle,
    rx: mpsc::Receiver<WorkerMessage>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_worker(
        DaemonState::new(),
        handle.tx.clone(),
        rx,
        Arc::clone(&handle.status),
        WorkerRuntimeOptions {
            service_reconcile_interval_ms: None,
            service_job_timeout_ms: None,
            service_monitor_interval_ms: None,
            running_cancellations: Arc::clone(&handle.running_cancellations),
        },
    ))
}

#[tokio::test]
#[ignore = "requires isolated runtime root: AGENT_BROWSER_TEST_ISOLATED=1"]
async fn private_full_and_closed_queue_use_fixed_errors_without_public_jobs() {
    isolated();
    let before = jobs();
    for full in [true, false] {
        let (fixture, _store, handoff) = staged(Duration::from_secs(30)).await;
        let files = std::fs::read_dir(&fixture.0).unwrap().count();
        let (handle, rx) = channel(1);
        if full {
            let (done, _) = oneshot::channel();
            handle
                .tx
                .try_send(WorkerMessage::Shutdown(done))
                .ok()
                .unwrap();
            assert!(matches!(
                handle.submit_private(handoff).await,
                Err("private_worker_unavailable")
            ));
            assert_eq!(rx.len(), 1);
        } else {
            drop(rx);
            assert!(matches!(
                handle.submit_private(handoff).await,
                Err("private_worker_unavailable")
            ));
        }
        assert_eq!(handle.queue_depth(), 0);
        assert_eq!(handle.status.worker_state(), WorkerState::Starting);
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), files);
    }
    assert_eq!(jobs(), before);
}

#[tokio::test]
#[ignore = "requires isolated runtime root: AGENT_BROWSER_TEST_ISOLATED=1"]
async fn private_expired_queued_handoff_never_enters_execution_or_public_jobs() {
    isolated();
    let before = jobs();
    let (fixture, _store, handoff) = staged(Duration::from_millis(500)).await;
    let expiry = handoff.expires_at();
    let files = std::fs::read_dir(&fixture.0).unwrap().count();
    let (handle, rx) = channel(1);
    let caller_handle = handle.clone();
    let caller = tokio::spawn(async move { caller_handle.submit_private(handoff).await });
    until_queue(&handle, 1).await;
    tokio::time::sleep(
        expiry.duration_since(SystemTime::now()).unwrap_or_default() + Duration::from_millis(20),
    )
    .await;
    let worker = start_existing(&handle, rx);
    assert!(matches!(
        caller.await.unwrap(),
        Err("private_handoff_expired")
    ));
    assert_eq!(handle.status.worker_state(), WorkerState::Ready);
    assert_eq!(handle.queue_depth(), 0);
    worker.abort();
    let _ = worker.await;
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), files);
    assert_eq!(jobs(), before);
}

#[tokio::test]
#[ignore = "requires isolated runtime root: AGENT_BROWSER_TEST_ISOLATED=1"]
async fn private_caller_cancelled_before_dequeue_never_enters_execution_or_public_jobs() {
    isolated();
    let before = jobs();
    let (fixture, _store, handoff) = staged(Duration::from_secs(30)).await;
    let files = std::fs::read_dir(&fixture.0).unwrap().count();
    let (handle, rx) = channel(1);
    let caller_handle = handle.clone();
    let caller = tokio::spawn(async move { caller_handle.submit_private(handoff).await });
    until_queue(&handle, 1).await;
    caller.abort();
    assert!(matches!(caller.await, Err(error) if error.is_cancelled()));
    let worker = start_existing(&handle, rx);
    until_queue(&handle, 0).await;
    assert_eq!(handle.status.worker_state(), WorkerState::Ready);
    worker.abort();
    let _ = worker.await;
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), files);
    assert_eq!(jobs(), before);
}
