//! Isolated private-queue regressions. No real browser or public job submission.

use super::*;
use crate::native::private_controller::RecoveryController;
use crate::native::private_handoff::{receive, PrivateHandoffAuthority, PrivateStagedHandoff};
use crate::native::private_secret_store::SecretStore;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
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

async fn staged(
    ttl: Duration,
) -> (
    Fixture,
    Arc<SecretStore>,
    PrivateStagedHandoff,
    Arc<RecoveryController>,
) {
    let root =
        std::env::temp_dir().join(format!("ab-private-worker-test-{}", uuid::Uuid::new_v4()));
    let store = Arc::new(SecretStore::open(&root).unwrap());
    let controller = Arc::new(
        RecoveryController::new(
            Arc::new(SecretStore::open(&root.join("controller")).unwrap()),
            "ws://127.0.0.1:1".into(),
            "default".into(),
        )
        .unwrap(),
    );
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
            "profileId":"sam","browserId":"session:default","sessionName":"default","tabId":"tab","targetId":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
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
    (Fixture(root), store, handoff, controller)
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

fn bound(
    fixture: &Fixture,
    staged: &PrivateStagedHandoff,
) -> super::super::private_bound_execution::BoundExecution {
    let store = Arc::new(SecretStore::open(&fixture.0.join("bindings")).unwrap());
    let plan = "b".repeat(32);
    let digest = "a".repeat(64);
    let bytes = serde_json::to_vec(&json!({
        "operation":"bind","planId":plan,"manifestSha256":digest,
        "profileId":"sam","browserId":"session:default","sessionName":"default",
        "targetId":"A".repeat(32),"endpoint":"ws://127.0.0.1:1",
        "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339()
    }))
    .unwrap();
    let reference = store.stage(&bytes).unwrap();
    store
        .bind_reconciliation(&hex::encode(Sha256::digest(plan.as_bytes())), &reference)
        .unwrap();
    let result = super::super::private_bound_execution::claim(
        Arc::clone(&store),
        &plan,
        &digest,
        staged,
        "ws://127.0.0.1:1",
        "default",
    )
    .unwrap();
    let reopened = Arc::new(SecretStore::open(&fixture.0.join("bindings")).unwrap());
    assert!(super::super::private_bound_execution::claim(
        reopened,
        &plan,
        &digest,
        staged,
        "ws://127.0.0.1:1",
        "default",
    )
    .is_err());
    assert_eq!(store.status(&reference).unwrap(), "admitted");
    result
}

fn jobs() -> Value {
    let repository = LockedServiceStateRepository::default_json().unwrap();
    serde_json::to_value(repository.load_snapshot().unwrap().jobs).unwrap()
}

fn binding_remains_consumed(fixture: &Fixture) {
    let store = SecretStore::open(&fixture.0.join("bindings")).unwrap();
    let reference = store
        .reconciliation_reference(&hex::encode(Sha256::digest("b".repeat(32))))
        .unwrap();
    assert_eq!(store.status(&reference).unwrap(), "admitted");
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
        let (fixture, _store, handoff, controller) = staged(Duration::from_secs(30)).await;
        let binding = bound(&fixture, &handoff);
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
                handle
                    .submit_bound_private(handoff, controller, binding)
                    .await,
                Err("private_worker_unavailable")
            ));
            assert_eq!(rx.len(), 1);
        } else {
            drop(rx);
            assert!(matches!(
                handle
                    .submit_bound_private(handoff, controller, binding)
                    .await,
                Err("private_worker_unavailable")
            ));
        }
        assert_eq!(handle.queue_depth(), 0);
        assert_eq!(handle.status.worker_state(), WorkerState::Starting);
        binding_remains_consumed(&fixture);
        assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), files);
    }
    assert_eq!(jobs(), before);
}

#[tokio::test]
#[ignore = "requires isolated runtime root: AGENT_BROWSER_TEST_ISOLATED=1"]
async fn private_expired_queued_handoff_never_enters_execution_or_public_jobs() {
    isolated();
    let before = jobs();
    let (fixture, _store, handoff, controller) = staged(Duration::from_millis(500)).await;
    let binding = bound(&fixture, &handoff);
    let expiry = handoff.expires_at();
    let files = std::fs::read_dir(&fixture.0).unwrap().count();
    let (handle, rx) = channel(1);
    let caller_handle = handle.clone();
    let caller = tokio::spawn(async move {
        caller_handle
            .submit_bound_private(handoff, controller, binding)
            .await
    });
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
    binding_remains_consumed(&fixture);
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
    let (fixture, _store, handoff, controller) = staged(Duration::from_secs(30)).await;
    let binding = bound(&fixture, &handoff);
    let files = std::fs::read_dir(&fixture.0).unwrap().count();
    let (handle, rx) = channel(1);
    let caller_handle = handle.clone();
    let caller = tokio::spawn(async move {
        caller_handle
            .submit_bound_private(handoff, controller, binding)
            .await
    });
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

#[tokio::test]
#[ignore = "requires isolated runtime root: synthetic binding drift only"]
async fn private_binding_drift_is_rejected_before_consumption_or_queue() {
    isolated();
    for (field, replacement) in [
        ("profileId", "other"),
        ("targetId", "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"),
        (
            "manifestSha256",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
        ("endpoint", "ws://127.0.0.1:2"),
        ("sessionName", "other"),
        ("expiresAt", "2000-01-01T00:00:00Z"),
        ("expiresAt", "ceiling"),
        ("store", "alias"),
    ] {
        let (fixture, _store, staged, _controller) = staged(Duration::from_secs(30)).await;
        let bindings = if field == "store" {
            Arc::clone(&_store)
        } else {
            Arc::new(SecretStore::open(&fixture.0.join("bindings")).unwrap())
        };
        let mut value = json!({
            "operation":"bind","planId":"b".repeat(32),"manifestSha256":"a".repeat(64),
            "profileId":"sam","browserId":"session:default","sessionName":"default",
            "targetId":"A".repeat(32),"endpoint":"ws://127.0.0.1:1",
            "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339()
        });
        if replacement == "ceiling" {
            value[field] = json!((chrono::Utc::now() + chrono::Duration::seconds(1)).to_rfc3339());
        } else if field != "store" {
            value[field] = json!(replacement);
        }
        let reference = bindings
            .stage(&serde_json::to_vec(&value).unwrap())
            .unwrap();
        bindings
            .bind_reconciliation(&hex::encode(Sha256::digest("b".repeat(32))), &reference)
            .unwrap();
        assert!(super::super::private_bound_execution::claim(
            Arc::clone(&bindings),
            &"b".repeat(32),
            &"a".repeat(64),
            &staged,
            "ws://127.0.0.1:1",
            "default",
        )
        .is_err());
        assert_eq!(bindings.status(&reference).unwrap(), "staged");
    }
}
