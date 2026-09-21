//! Internal ordered execution, not a public command or consent authority.
//! A completed sequence still holds privacy closed and is not a publishable
//! renewal receipt. Trusted ingress, cleanup and result delivery remain gated.

use super::actions::DaemonState;
use super::cdp::client::PrivateTransportClosed;
use super::privacy_gate::PrivatePermit;
use super::private_attachment::PrivateAttachment;
use super::private_broker::{
    advance_private_target, prepare_private_target, revalidate_private_target,
};
use super::private_controller::RecoveryController;
use super::private_execution::{execute_private_operation, PrivateStepOutcome};
use super::private_identity::{
    validate_private_target, LivePrivateIdentity, ValidatedPrivateJourney,
};
use super::private_operation::{Operation, PrivateOperation};
use super::private_secret_store::SecretStore;
use serde_json::Value;
use std::sync::Arc;

const INVALID: &str = "private_journey_invalid";

/// Deliberately no Debug/Serialize or public result accessor. Possession is not
/// cleanup proof. Dropping this value keeps the durable private epoch closed.
pub(crate) struct PrivateJourneyPendingCleanup {
    permit: PrivatePermit,
    result_reference: String,
    attachment: PrivateAttachment,
}

/// The original document is empty and this client's transport has terminated.
/// The dedicated private attachment was detached before transport closure.
/// Daemon/broker reconciliation and result delivery are still required. This
/// deliberately cannot unlock privacy, install a key or emit a public receipt.
pub(crate) struct PrivateJourneyPendingReconciliation {
    permit: PrivatePermit,
    result_reference: String,
    transport_closed: PrivateTransportClosed,
    attachment: PrivateAttachment,
}

/// Local completion only: no privacy release, public receipt or key delivery.
/// Retains the original closed authority for a future guarded completion path.
pub(crate) struct ReconciledPrivateJourney {
    pending: PrivateJourneyPendingReconciliation,
    reconciliation_reference: String,
    #[cfg(unix)]
    delivery: Option<(
        Arc<SecretStore>,
        super::private_bound_execution::BoundExecution,
    )>,
}

#[cfg(unix)]
impl ReconciledPrivateJourney {
    pub(crate) fn bind_delivery(
        mut self,
        store: Arc<SecretStore>,
        binding: super::private_bound_execution::BoundExecution,
    ) -> Self {
        self.delivery = Some((store, binding));
        self
    }

    /// Only this typed, reconciled success can produce a private delivery.
    /// No key accessor, ordinary job result, retry or privacy release exists.
    pub(crate) async fn deliver(
        self,
        stream: tokio::net::UnixStream,
        authentication_key: [u8; 32],
    ) -> Result<(), &'static str> {
        let failed = "private_delivery_failed_closed";
        let (store, binding) = self.delivery.ok_or(failed)?;
        let operation = PrivateOperation::parse(&store.load(&self.pending.result_reference)?)?;
        if store.load_result(&self.reconciliation_reference)?
            != b"{\"reconciled\":true,\"privacyReleased\":false}"
        {
            return Err(failed);
        }
        let intent: Value = serde_json::from_slice(&store.load(&self.reconciliation_reference)?)
            .map_err(|_| failed)?;
        if intent["resultReference"].as_str() != Some(self.pending.result_reference.as_str())
            || intent["epoch"].as_str() != Some(self.pending.permit.epoch_id())
        {
            return Err(failed);
        }
        let mut metadata =
            binding.delivery_metadata(&operation, intent["endpoint"].as_str().ok_or(failed)?)?;
        metadata["resultReference"] = Value::String(self.pending.result_reference.clone());
        metadata["reconciliationReference"] = Value::String(self.reconciliation_reference);
        super::private_delivery::send(
            stream,
            authentication_key,
            &store,
            &self.pending.result_reference,
            metadata,
        )
        .await
    }
}

impl PrivateJourneyPendingReconciliation {
    /// Consume the success path once. A durable intent precedes broker changes;
    /// any error leaves that intent and the private gate closed for recovery.
    /// Never reacquire CDP, replay a credential, or detach a second time here.
    pub(crate) fn reconcile(
        self,
        state: &mut DaemonState,
        store: &SecretStore,
        controller: &RecoveryController,
    ) -> Result<ReconciledPrivateJourney, &'static str> {
        self.reconcile_with_checkpoints(state, store, controller, |_| Ok(()))
    }

    fn reconcile_with_checkpoints(
        self,
        state: &mut DaemonState,
        store: &SecretStore,
        controller: &RecoveryController,
        checkpoint: impl Fn(&str) -> Result<(), &'static str>,
    ) -> Result<ReconciledPrivateJourney, &'static str> {
        use super::private_broker::validate_broker_snapshot;
        use super::service_model::{BrowserHealth, TabLifecycle};
        use super::service_store::LockedServiceStateRepository;
        let failed = "private_reconciliation_failed_closed";
        let operation = PrivateOperation::parse(&store.load(&self.result_reference)?)?;
        if !matches!(operation.operation(), Operation::ReadKey { .. })
            || self.permit.journey_stage() != Some(3)
        {
            return Err(failed);
        }
        // Existence of the encrypted result is necessary, never public output.
        store.load_result(&self.result_reference)?;
        let browser = state.browser.as_ref().ok_or(failed)?;
        self.attachment.verify_closed_detach(
            &browser.client,
            &self.permit,
            &self.transport_closed,
            store,
        )?;
        let profile = state.private_runtime_profile().ok_or(failed)?.to_owned();
        let target = browser.active_target_id().map_err(|_| failed)?.to_owned();
        let browser_id = format!("session:{}", state.session_id);
        let endpoint = browser.get_cdp_url().to_owned();
        controller.validate_scope(store, &endpoint, &state.session_id)?;
        let tab_id = operation
            .service_tab_handle()
            .get("tabId")
            .and_then(Value::as_str)
            .ok_or(failed)?
            .to_owned();
        let intent = serde_json::to_vec(&serde_json::json!({
            "schema":"private-reconciliation.v2", "epoch":self.permit.epoch_id(),
            "scopeDigest":self.permit.identity_digest(), "profileId":profile,
            "browserId":browser_id,"sessionName":state.session_id,"targetId":target,
            "tabId":tab_id,"endpoint":endpoint,"resultReference":self.result_reference,
            "priorHandle":operation.service_tab_handle(),"sanitizedUrl":"about:blank",
            "detachReference":self.attachment.detach_reference()
        }))
        .map_err(|_| failed)?;
        let reference = store.stage(&intent)?;
        store.bind_reconciliation(self.permit.epoch_id(), &reference)?;
        checkpoint("intent_prepared")?;
        // Capture exact producer bytes before admission, never by rehashing
        // the examined operation intent into authority during restart.
        use sha2::{Digest, Sha256};
        controller.retain(
            store,
            self.permit.epoch_id(),
            &reference,
            &hex::encode(Sha256::digest(&intent)),
        )?;
        checkpoint("authority_retained")?;
        store.admit(&reference)?;
        checkpoint("intent_admitted")?;
        LockedServiceStateRepository::default_json()
            .map_err(|_| failed)?
            .try_mutate_durable(|persisted| {
                // Historical scope comparison, NOT a fabricated live probe.
                // The consumed pending object binds already verified sanitation
                // and transport closure to this exact original authority.
                let binding = LivePrivateIdentity {
                    profile_id: &profile,
                    browser_id: &browser_id,
                    session_name: &state.session_id,
                    target_id: &target,
                    url: operation.expected_url(),
                    ready: true,
                };
                let prior = validate_broker_snapshot(
                    persisted,
                    operation.service_tab_handle(),
                    &binding,
                    &endpoint,
                    operation.expected_origin(),
                    operation.expected_url(),
                )
                .map_err(str::to_owned)?;
                if self.permit.target_id() != Some(prior.target_id())
                    || self.permit.identity_digest() != Some(prior.scope_digest())
                {
                    return Err(failed.to_owned());
                }
                let tab = persisted.tabs.get_mut(&tab_id).ok_or(failed)?;
                tab.url = Some("about:blank".into());
                tab.title = None;
                tab.lifecycle = TabLifecycle::Unknown;
                tab.service_tab_handle = None;
                tab.latest_snapshot_id = None;
                tab.latest_screenshot_id = None;
                persisted
                    .browsers
                    .get_mut(&browser_id)
                    .ok_or(failed)?
                    .health = BrowserHealth::CdpDisconnected;
                Ok(())
            })
            .map_err(|_| failed)?;
        checkpoint("broker_committed")?;
        // Keep the sealed manager and process ownership; dropping/replacing it
        // could lose retained-browser authority. Cached page data is no longer
        // the private login page, and the transport cannot be used again.
        state
            .browser
            .as_mut()
            .ok_or(failed)?
            .set_active_page_metadata(Some("about:blank"), Some(""));
        state.ref_map.clear();
        checkpoint("daemon_updated")?;
        store.store_result(
            &reference,
            b"{\"reconciled\":true,\"privacyReleased\":false}",
        )?;
        checkpoint("completion_committed")?;
        Ok(ReconciledPrivateJourney {
            pending: self,
            reconciliation_reference: reference,
            #[cfg(unix)]
            delivery: None,
        })
    }
}

impl PrivateJourneyPendingCleanup {
    /// Consume the completed sequence into its next internal cleanup stage.
    /// Any uncertainty leaves the durable privacy interval closed, including
    /// cancellation after blanking the page but before the transport closes.
    pub(crate) async fn sanitize_and_close(
        mut self,
        state: &mut DaemonState,
        store: &SecretStore,
    ) -> Result<PrivateJourneyPendingReconciliation, &'static str> {
        let final_operation = PrivateOperation::parse(&store.load(&self.result_reference)?)?;
        if !matches!(final_operation.operation(), Operation::ReadKey { .. }) {
            return Err(INVALID);
        }
        revalidate_private_target(
            state,
            final_operation.service_tab_handle(),
            final_operation.expected_origin(),
            final_operation.expected_url(),
            &self.permit,
        )
        .await?;
        let browser = state
            .browser
            .as_mut()
            .ok_or("private_browser_unavailable")?;
        let target = browser
            .active_target_id()
            .map_err(|_| "private_target_unavailable")?
            .to_owned();
        let session = self
            .attachment
            .session_id(&browser.client, &self.permit)?
            .to_owned();
        browser
            .client
            .sanitize_private_page(&mut self.permit, &target, &session)
            .await?;
        self.attachment.detach(store, &self.permit).await?;
        let transport_closed = browser
            .client
            .quiesce_private_transport(&self.permit)
            .await?;
        Ok(PrivateJourneyPendingReconciliation {
            permit: self.permit,
            result_reference: self.result_reference,
            transport_closed,
            attachment: self.attachment,
        })
    }
}

fn load_sequence(
    store: &SecretStore,
    references: &[String],
) -> Result<Vec<PrivateOperation>, &'static str> {
    if references.len() != 4
        || references
            .iter()
            .enumerate()
            .any(|(index, reference)| references[..index].contains(reference))
    {
        return Err(INVALID);
    }
    let operations: Vec<_> = references
        .iter()
        .map(|reference| PrivateOperation::parse(&store.load(reference)?))
        .collect::<Result<_, _>>()?;
    validate_sequence(&operations)?;
    Ok(operations)
}

/// Structural validation of every stage before the first secret write. These
/// prospective observations are NOT live authority; execution obtains that
/// independently from the daemon-owned broker at admission and each stage.
pub(crate) fn validate_sequence(operations: &[PrivateOperation]) -> Result<(), &'static str> {
    if operations.len() != 4
        || !matches!(operations[0].operation(), Operation::Login { .. })
        || !matches!(operations[1].operation(), Operation::BackupCode { .. })
        || !matches!(operations[2].operation(), Operation::Renew { .. })
        || !matches!(operations[3].operation(), Operation::ReadKey { .. })
    {
        return Err(INVALID);
    }
    let first = &operations[0];
    let handle = first.service_tab_handle();
    let field = |name| handle.get(name).and_then(Value::as_str).ok_or(INVALID);
    for operation in operations {
        if operation.consent_sha256() != first.consent_sha256()
            || operation.account_scope() != first.account_scope()
        {
            return Err(INVALID);
        }
        let mut expected_handle = handle.clone();
        expected_handle["url"] = Value::String(operation.expected_url().to_owned());
        let prospective = LivePrivateIdentity {
            profile_id: field("profileId")?,
            browser_id: field("browserId")?,
            session_name: field("sessionName")?,
            target_id: field("targetId")?,
            url: operation.expected_url(),
            ready: true,
        };
        validate_private_target(
            operation.service_tab_handle(),
            &expected_handle,
            &prospective,
            operation.expected_origin(),
            operation.expected_url(),
        )
        .map_err(|_| INVALID)?;
    }
    Ok(())
}

/// Run only an already trusted, encrypted four-stage SAM sequence. Never retry
/// a dispatch or infer a new URL. A navigation not yet at its approved exact
/// destination fails closed, just like drift, cancellation or an uncertain CDP
/// response. This intentionally cannot resume a partially executed sequence.
pub(crate) async fn execute_private_journey(
    state: &DaemonState,
    store: &SecretStore,
    references: &[String],
) -> Result<PrivateJourneyPendingCleanup, &'static str> {
    let operations = load_sequence(store, references)?;
    let first = &operations[0];
    let initial = prepare_private_target(
        state,
        first.service_tab_handle(),
        first.expected_origin(),
        first.expected_url(),
    )
    .await?;
    let destinations = operations
        .iter()
        .map(|op| {
            (
                op.expected_origin().to_owned(),
                op.expected_url().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let journey = ValidatedPrivateJourney::new(&initial, &destinations)?;
    let mut permit = state
        .browser
        .as_ref()
        .ok_or("private_browser_unavailable")?
        .client
        .begin_private_journey(&journey)?;
    store.reserve_journey(first.consent_sha256(), references)?;
    revalidate_private_target(
        state,
        first.service_tab_handle(),
        first.expected_origin(),
        first.expected_url(),
        &permit,
    )
    .await?;
    let attachment = PrivateAttachment::acquire(
        Arc::clone(
            &state
                .browser
                .as_ref()
                .ok_or("private_browser_unavailable")?
                .client,
        ),
        store,
        &permit,
    )
    .await?;
    for (index, reference) in references.iter().enumerate() {
        if index > 0 {
            advance_private_target(
                state,
                &operations[index - 1],
                &operations[index],
                index - 1,
                &mut permit,
                &attachment,
            )
            .await?;
        }
        // Revalidates actual broker/page authority under the private lock and
        // commits this reference's durable admission before any DOM mutation.
        let outcome =
            execute_private_operation(state, store, reference, &permit, &attachment).await?;
        if !matches!(
            (index, outcome),
            (0..=2, PrivateStepOutcome::DispatchAttempted) | (3, PrivateStepOutcome::ResultStored)
        ) {
            return Err("private_journey_outcome_mismatch");
        }
    }
    Ok(PrivateJourneyPendingCleanup {
        permit,
        result_reference: references[3].clone(),
        attachment,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    #[ignore = "requires repository isolated Rust runner; synthetic CDP only"]
    async fn dedicated_attachment_runs_full_sequence_and_detaches_before_closure() {
        run_full_sequence(false, false, "").await;
    }

    #[tokio::test]
    #[ignore = "requires repository isolated Rust runner; synthetic socket and CDP only"]
    async fn authenticated_handoff_runs_through_private_worker_without_publication() {
        run_full_sequence(true, false, "").await;
    }

    #[tokio::test]
    #[ignore = "requires repository isolated Rust runner; synthetic socket and CDP only"]
    async fn private_worker_cancellation_after_login_stops_remaining_dispatch() {
        run_full_sequence(true, true, "").await;
    }

    #[tokio::test]
    #[ignore = "requires repository isolated Rust runner; synthetic CDP only"]
    async fn reconciliation_rejects_changed_authority_without_replay() {
        for fault in ["profile", "session", "target", "url", "lease"] {
            run_full_sequence(false, false, fault).await;
        }
    }

    #[tokio::test]
    #[ignore = "requires repository isolated Rust runner; synthetic CDP only"]
    async fn reconciliation_interrupted_commits_keep_privacy_closed() {
        for fault in ["intent_admitted", "broker_committed", "daemon_updated"] {
            run_full_sequence(false, false, fault).await;
        }
    }

    #[tokio::test]
    #[ignore = "child entrypoint for isolated process-crash test only"]
    async fn reconciliation_crash_child() {
        assert_eq!(
            std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref(),
            Ok("1")
        );
        let phase = std::env::var("AB_PRIVATE_CRASH_PHASE").expect("parent supplies crash phase");
        assert!(matches!(
            phase.as_str(),
            "intent_prepared"
                | "authority_retained"
                | "intent_admitted"
                | "broker_committed"
                | "daemon_updated"
                | "completion_committed"
        ));
        run_full_sequence(false, false, &format!("crash:{phase}")).await;
        panic!("child must die at checkpoint");
    }

    #[test]
    #[ignore = "requires repository isolated runner; kills only its own synthetic child"]
    fn reconciliation_recovers_after_actual_process_death_without_dispatch() {
        use super::super::privacy_gate::PrivacyGate;
        use super::super::private_recovery::{recover_reconciliation, RecoveryAuthority};
        use super::super::service_model::BrowserHealth;
        use super::super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref(),
            Ok("1")
        );
        let repository = LockedServiceStateRepository::default_json().unwrap();
        let previous = repository.load_snapshot().unwrap();
        for phase in [
            "intent_prepared",
            "authority_retained",
            "intent_admitted",
            "broker_committed",
            "daemon_updated",
            "completion_committed",
        ] {
            let root =
                std::env::temp_dir().join(format!("ab-reconcile-crash-{}", uuid::Uuid::new_v4()));
            SecretStore::ensure_private_directory(&root).unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native::private_journey::tests::reconciliation_crash_child",
                    "--ignored",
                    "--test-threads=1",
                ])
                .env("AB_PRIVATE_CRASH_PHASE", phase)
                .env("AB_PRIVATE_CRASH_ROOT", &root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("synthetic child timed out");
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            };
            assert_eq!(status.signal(), Some(libc::SIGKILL), "phase {phase}");
            let bookmark: Value =
                serde_json::from_slice(&std::fs::read(root.join("handoff.json")).unwrap()).unwrap();
            let field = |name| bookmark[name].as_str().unwrap().to_owned();
            let store = SecretStore::open(&root.join("store")).unwrap();
            let journal = Arc::new(SecretStore::open(&root.join("controller")).unwrap());
            let controller =
                RecoveryController::new(Arc::clone(&journal), field("endpoint"), "default".into())
                    .unwrap();
            let effects = std::fs::read(root.join("effects.json")).unwrap();
            assert_eq!(
                serde_json::from_slice::<Vec<String>>(&effects).unwrap(),
                ["attach", "login", "backup", "renew", "read", "sanitize", "detach"]
            );
            let reference = store.reconciliation_reference(&field("epoch")).unwrap();
            let gate = PrivacyGate::for_endpoint(&field("endpoint")).unwrap();
            assert!(gate.public_lease().is_err());
            if matches!(phase, "intent_prepared" | "authority_retained") {
                assert!(controller.recover(&store, &field("epoch")).is_err());
                assert_eq!(store.status(&reference).unwrap(), "staged");
                assert_eq!(
                    repository.load_snapshot().unwrap().browsers["session:default"].health,
                    BrowserHealth::Ready
                );
                assert!(gate.public_lease().is_err());
                assert_eq!(std::fs::read(root.join("effects.json")).unwrap(), effects);
                repository
                    .mutate(|state| {
                        *state = previous.clone();
                        Ok(())
                    })
                    .unwrap();
                drop(controller);
                drop(journal);
                drop(store);
                drop(gate);
                std::fs::remove_dir_all(root).unwrap();
                continue;
            }
            // Direct low-level rejection coverage remains separate from the
            // successful controller path, which never derives its own digest.
            use sha2::{Digest, Sha256};
            let digest = hex::encode(Sha256::digest(store.load(&reference).unwrap()));
            let authority = || {
                RecoveryAuthority::new(
                    digest.clone(),
                    field("endpoint"),
                    field("epoch"),
                    "default".into(),
                )
            };
            for wrong in ["digest", "epoch", "session", "endpoint"] {
                let bad = RecoveryAuthority::new(
                    if wrong == "digest" {
                        "0".repeat(64)
                    } else {
                        digest.clone()
                    },
                    if wrong == "endpoint" {
                        "ws://127.0.0.1:1".into()
                    } else {
                        field("endpoint")
                    },
                    if wrong == "epoch" {
                        "0".repeat(64)
                    } else {
                        field("epoch")
                    },
                    if wrong == "session" {
                        "foreign".into()
                    } else {
                        "default".into()
                    },
                );
                assert!(recover_reconciliation(&store, &reference, bad).is_err());
                assert!(gate.public_lease().is_err());
            }
            let alternate = store.stage(&store.load(&reference).unwrap()).unwrap();
            assert!(recover_reconciliation(&store, &alternate, authority()).is_err());
            let first = controller.recover(&store, &field("epoch")).unwrap();
            assert!(gate.public_lease().is_err());
            drop(first);
            let second = controller.recover(&store, &field("epoch")).unwrap();
            assert!(gate.public_lease().is_err());
            drop(second);
            let snapshot = repository.load_snapshot().unwrap();
            assert_eq!(
                snapshot.browsers["session:default"].health,
                BrowserHealth::CdpDisconnected
            );
            assert_eq!(
                snapshot.tabs["tab"].target_id.as_deref(),
                Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
            );
            assert_eq!(snapshot.tabs["tab"].url.as_deref(), Some("about:blank"));
            assert_eq!(store.status(&reference).unwrap(), "result_ready");
            // Recovery has no CDP transport and cannot execute browser actions.
            // Drift must fail even after a successful historical reconciliation.
            for kind in ["profile", "session", "target", "url", "lease"] {
                repository
                    .mutate(|state| {
                        *state = snapshot.clone();
                        match kind {
                            "profile" => {
                                state
                                    .browsers
                                    .get_mut("session:default")
                                    .unwrap()
                                    .profile_id = Some("foreign".into())
                            }
                            "session" => {
                                state.sessions.get_mut("default").unwrap().profile_id =
                                    Some("foreign".into())
                            }
                            "target" => {
                                state.tabs.get_mut("tab").unwrap().target_id =
                                    Some("foreign".into())
                            }
                            "url" => {
                                state.tabs.get_mut("tab").unwrap().url =
                                    Some("https://sam.gov/foreign".into())
                            }
                            "lease" => {
                                state.sessions.get_mut("default").unwrap().expires_at =
                                    Some("2000-01-01T00:00:00Z".into())
                            }
                            _ => unreachable!(),
                        }
                        Ok(())
                    })
                    .unwrap();
                assert!(controller.recover(&store, &field("epoch")).is_err());
                assert!(gate.public_lease().is_err());
            }
            assert_eq!(std::fs::read(root.join("effects.json")).unwrap(), effects);
            repository
                .mutate(|state| {
                    *state = previous.clone();
                    Ok(())
                })
                .unwrap();
            drop(store);
            drop(gate);
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires repository isolated runner; synthetic peer only"]
    async fn private_worker_rejects_wrong_controller_before_any_cdp_command() {
        for fault in [
            "controller_endpoint",
            "controller_session",
            "controller_alias",
        ] {
            run_full_sequence(true, false, fault).await;
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore = "requires isolated runner; authenticated synthetic socket and CDP peer only"]
    async fn stored_binding_reaches_private_worker_once_after_authenticated_ingress() {
        run_full_sequence(true, false, "bound").await;
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    #[ignore = "requires isolated runner; synthetic coordinator socket and CDP only"]
    async fn coordinator_socket_reaches_bound_worker_cleanup_and_delivery_once() {
        run_full_sequence(true, false, "coordinator").await;
    }

    async fn run_full_sequence(through_worker: bool, cancel_after_login: bool, fault: &str) {
        use super::super::browser::{BrowserManager, PageInfo};
        use super::super::cdp::client::CdpClient;
        use super::super::service_model::{
            BrowserHealth, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, LeaseState,
            ServiceState, TabLifecycle,
        };
        use super::super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
        use futures_util::{SinkExt, StreamExt};
        use tokio::net::TcpListener;
        use tokio_tungstenite::tungstenite::Message;
        assert_eq!(
            std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref(),
            Ok("1")
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let login_seen = Arc::new(tokio::sync::Notify::new());
        let resume_peer = Arc::new(tokio::sync::Notify::new());
        let peer_login = Arc::clone(&login_seen);
        let peer_resume = Arc::clone(&resume_peer);
        let commands = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peer_commands = Arc::clone(&commands);
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let mut url = "https://secure.login.gov/";
            let mut effects = Vec::new();
            let mut attached = false;
            while let Some(Ok(message)) = ws.next().await {
                if !message.is_text() {
                    continue;
                }
                let command: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                peer_commands.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let method = command["method"].as_str().unwrap();
                let result = match method {
                    "Target.getTargetInfo" => {
                        if command["sessionId"] == "private-session" {
                            assert!(attached);
                        } else {
                            assert_eq!(command["sessionId"], "main-session");
                        }
                        json!({"targetInfo":{"targetId":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","type":"page","url":url}})
                    }
                    "Target.attachToTarget" => {
                        assert!(!attached);
                        assert_eq!(
                            command["params"],
                            json!({"targetId":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","flatten":true})
                        );
                        attached = true;
                        effects.push("attach");
                        json!({"sessionId":"private-session"})
                    }
                    "Target.detachFromTarget" => {
                        assert!(attached);
                        assert_eq!(command["params"], json!({"sessionId":"private-session"}));
                        assert_eq!(url, "about:blank");
                        attached = false;
                        effects.push("detach");
                        json!({})
                    }
                    "Page.getFrameTree" => json!({"frameTree":{"frame":{"id":"top","url":url}}}),
                    "Page.createIsolatedWorld" => json!({"executionContextId":7}),
                    "Page.stopLoading" => json!({}),
                    "Page.navigate" => {
                        assert_eq!(command["params"]["url"], "about:blank");
                        url = "about:blank";
                        effects.push("sanitize");
                        json!({"frameId":"top"})
                    }
                    "Runtime.evaluate" => {
                        assert_eq!(command["sessionId"], "private-session");
                        let expression = command["params"]["expression"].as_str().unwrap();
                        if expression.contains("\"kind\":\"login\"") {
                            effects.push("login");
                            if cancel_after_login {
                                peer_login.notify_one();
                                peer_resume.notified().await;
                            }
                            json!({"result":{"value":{"dispatch_attempted":true}}})
                        } else if expression.contains("\"kind\":\"backup\"") {
                            effects.push("backup");
                            url = "https://sam.gov/";
                            json!({"result":{"value":{"dispatch_attempted":true}}})
                        } else if expression.contains("\"kind\":\"renew\"") {
                            effects.push("renew");
                            json!({"result":{"value":{"dispatch_attempted":true}}})
                        } else if expression.contains("\"kind\":\"read\"") {
                            effects.push("read");
                            json!({"result":{"value":{"key":"SYNTHETIC_PRIVATE_KEY_123456789"}}})
                        } else {
                            assert_eq!(url, "about:blank");
                            assert!(expression.contains("document.body.childElementCount === 0"));
                            json!({"result":{"value":true}})
                        }
                    }
                    _ => panic!("unexpected synthetic protocol method"),
                };
                if method.starts_with("Page.") {
                    assert_eq!(command["sessionId"], "private-session");
                }
                if let Ok(root) = std::env::var("AB_PRIVATE_CRASH_ROOT") {
                    // Fixed synthetic action labels only, never CDP payloads.
                    std::fs::write(
                        std::path::PathBuf::from(root).join("effects.json"),
                        serde_json::to_vec(&effects).unwrap(),
                    )
                    .unwrap();
                }
                ws.send(Message::Text(
                    json!({"id":command["id"],"result":result}).to_string(),
                ))
                .await
                .unwrap();
                if cancel_after_login && effects.last() == Some(&"login") {
                    assert!(
                        tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
                            .await
                            .is_err()
                    );
                    return effects;
                }
            }
            assert!(!attached);
            effects
        });
        let client = Arc::new(CdpClient::connect(&endpoint).await.unwrap());
        let mut raw = client.subscribe_raw();
        let mut broker = ServiceState::default();
        broker.profiles.insert(
            "sam".into(),
            BrowserProfile {
                id: "sam".into(),
                ..Default::default()
            },
        );
        broker.browsers.insert(
            "session:default".into(),
            BrowserProcess {
                id: "session:default".into(),
                profile_id: Some("sam".into()),
                health: BrowserHealth::Ready,
                cdp_endpoint: Some(endpoint.clone()),
                active_session_ids: vec!["default".into()],
                ..Default::default()
            },
        );
        broker.sessions.insert(
            "default".into(),
            BrowserSession {
                id: "default".into(),
                profile_id: Some("sam".into()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["session:default".into()],
                tab_ids: vec!["tab".into()],
                ..Default::default()
            },
        );
        broker.tabs.insert(
            "tab".into(),
            BrowserTab {
                id: "tab".into(),
                browser_id: "session:default".into(),
                target_id: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into()),
                lifecycle: TabLifecycle::Ready,
                url: Some("https://secure.login.gov/".into()),
                owner_session_id: Some("default".into()),
                ..Default::default()
            },
        );
        let mut values = payloads();
        for value in &mut values {
            let mut handle =
                serde_json::to_value(broker.service_tab_handle("tab").unwrap()).unwrap();
            handle["url"] = value["expected_url"].clone();
            value["service_tab_handle"] = handle;
        }
        let repository = LockedServiceStateRepository::default_json().unwrap();
        let previous = repository.load_snapshot().unwrap();
        repository
            .mutate(|state| {
                *state = broker;
                Ok(())
            })
            .unwrap();
        let mut state = DaemonState::new();
        state.session_id = "default".into();
        state.browser = Some(BrowserManager::private_journey_test_fixture(
            Arc::clone(&client),
            endpoint.clone(),
            PageInfo {
                target_id: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
                session_id: "main-session".into(),
                url: "https://secure.login.gov/".into(),
                title: "Synthetic".into(),
                target_type: "page".into(),
            },
        ));
        state.set_private_journey_test_profile("sam");
        let root = if fault.starts_with("crash:") {
            std::path::PathBuf::from(std::env::var("AB_PRIVATE_CRASH_ROOT").unwrap())
        } else {
            std::env::temp_dir().join(format!("ab-private-full-{}", uuid::Uuid::new_v4()))
        };
        SecretStore::ensure_private_directory(&root).unwrap();
        let store = Arc::new(
            SecretStore::open(&root.join(if fault == "coordinator" {
                "operations"
            } else {
                "store"
            }))
            .unwrap(),
        );
        let controller = Arc::new(
            RecoveryController::new(
                if fault == "controller_alias" {
                    Arc::clone(&store)
                } else {
                    Arc::new(SecretStore::open(&root.join("controller")).unwrap())
                },
                if fault == "controller_endpoint" {
                    "ws://127.0.0.1:1".into()
                } else {
                    endpoint.clone()
                },
                if fault == "controller_session" {
                    "foreign".into()
                } else {
                    "default".into()
                },
            )
            .unwrap(),
        );
        let mut direct_state = Some(state);
        let closed = if through_worker {
            use super::super::private_handoff::{receive, PrivateHandoffAuthority};
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            use tokio::io::AsyncWriteExt;
            let key = [42u8; 32];
            let authority = PrivateHandoffAuthority::new(
                "a".repeat(64),
                values[0]["account_scope"].as_str().unwrap().into(),
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
                key,
                std::time::SystemTime::now() + std::time::Duration::from_secs(30),
            )
            .unwrap();
            let body = serde_json::to_vec(&values).unwrap();
            let mut frame = b"ABPH1\0".to_vec();
            frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
            frame.extend_from_slice(&body);
            let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
            mac.update(&frame);
            frame.extend_from_slice(&mac.finalize().into_bytes());
            let (mut sender, receiver) = tokio::net::UnixStream::pair().unwrap();
            sender.write_all(&frame).await.unwrap();
            sender.shutdown().await.unwrap();
            let staged = receive(receiver, authority, Arc::clone(&store))
                .await
                .unwrap();
            let worker = super::super::control_plane::ControlPlaneWorker::start(
                direct_state.take().unwrap(),
            );
            if fault.starts_with("controller_") {
                let files = std::fs::read_dir(root.join("store")).unwrap().count();
                assert!(worker.submit_private(staged, controller).await.is_err());
                assert_eq!(commands.load(std::sync::atomic::Ordering::SeqCst), 0);
                assert!(client.public_lease().is_ok());
                assert_eq!(
                    std::fs::read_dir(root.join("store")).unwrap().count(),
                    files
                );
                assert_eq!(
                    repository.load_snapshot().unwrap().tabs["tab"]
                        .url
                        .as_deref(),
                    Some("https://secure.login.gov/")
                );
                peer.abort();
                let _ = peer.await;
                repository
                    .mutate(|snapshot| {
                        *snapshot = previous;
                        Ok(())
                    })
                    .unwrap();
                drop(store);
                std::fs::remove_dir_all(root).unwrap();
                return;
            }
            if cancel_after_login {
                let submitted_worker = worker.clone();
                let submission = tokio::spawn(async move {
                    submitted_worker.submit_private(staged, controller).await
                });
                tokio::time::timeout(std::time::Duration::from_secs(2), login_seen.notified())
                    .await
                    .unwrap();
                submission.abort();
                assert!(matches!(submission.await, Err(error) if error.is_cancelled()));
                tokio::time::timeout(std::time::Duration::from_secs(2), async {
                    while worker.status_response("test")["data"]["worker_state"] != "Faulted" {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                resume_peer.notify_one();
                assert_eq!(peer.await.unwrap(), ["attach", "login"]);
                assert!(client.public_lease().is_err());
                let restaged = stage(&store, &values);
                assert_eq!(
                    store.reserve_journey(&"a".repeat(64), &restaged),
                    Err("private_journey_consent_consumed")
                );
                repository
                    .mutate(|state| {
                        *state = previous;
                        Ok(())
                    })
                    .unwrap();
                drop(store);
                std::fs::remove_dir_all(root).unwrap();
                return;
            }
            #[cfg(target_os = "linux")]
            if matches!(fault, "bound" | "coordinator") {
                use sha2::Digest;
                use std::io::{Read, Write};
                let bindings = Arc::new(SecretStore::open(&root.join("bindings")).unwrap());
                let server_store = Arc::clone(&bindings);
                let (server, mut sender) = std::os::unix::net::UnixStream::pair().unwrap();
                let server_thread = std::thread::spawn(move || {
                    super::super::private_controller_socket::accept_test_binding(
                        server,
                        [9; 32],
                        &server_store,
                    )
                });
                sender
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut challenge = [0; 32];
                sender.read_exact(&mut challenge).unwrap();
                let payload = serde_json::to_vec(&json!({
                    "operation":"bind","planId":"b".repeat(32),"manifestSha256":"a".repeat(64),
                    "profileId":"sam","browserId":"session:default","sessionName":"default",
                    "targetId":"A".repeat(32),"endpoint":endpoint,
                    "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339()
                }))
                .unwrap();
                let mut mac = Hmac::<Sha256>::new_from_slice(&[9; 32]).unwrap();
                mac.update(b"ABPC1-request\0");
                mac.update(&challenge);
                mac.update(&payload);
                sender
                    .write_all(&(payload.len() as u32).to_be_bytes())
                    .unwrap();
                sender.write_all(&payload).unwrap();
                sender.write_all(&mac.finalize().into_bytes()).unwrap();
                sender.shutdown(std::net::Shutdown::Write).unwrap();
                let mut length = [0; 4];
                sender.read_exact(&mut length).unwrap();
                let mut response = vec![0; u32::from_be_bytes(length) as usize];
                sender.read_exact(&mut response).unwrap();
                let mut tag = [0; 32];
                sender.read_exact(&mut tag).unwrap();
                let mut mac = Hmac::<Sha256>::new_from_slice(&[9; 32]).unwrap();
                mac.update(b"ABPC1-response\0");
                mac.update(&challenge);
                mac.update(&response);
                mac.verify_slice(&tag).unwrap();
                assert_eq!(
                    serde_json::from_slice::<Value>(&response).unwrap()["success"],
                    true
                );
                server_thread.join().unwrap().unwrap();
                if fault == "coordinator" {
                    let bound: Value = serde_json::from_slice(&payload).unwrap();
                    super::super::private_coordinator::test_exchange(
                        &worker,
                        &root,
                        &values,
                        &endpoint,
                        bound["expiresAt"].as_str().unwrap(),
                    )
                    .await;
                    let effects = tokio::time::timeout(std::time::Duration::from_secs(2), peer)
                        .await
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        effects,
                        ["attach", "login", "backup", "renew", "read", "sanitize", "detach"]
                    );
                    assert!(client.public_lease().is_err());
                    let snapshot = repository.load_snapshot().unwrap();
                    assert_eq!(snapshot.tabs["tab"].url.as_deref(), Some("about:blank"));
                    assert_eq!(
                        snapshot.tabs["tab"].target_id.as_deref(),
                        Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
                    );
                    assert!(super::super::private_bound_execution::claim(
                        Arc::clone(&bindings),
                        &"b".repeat(32),
                        &"a".repeat(64),
                        &staged,
                        &endpoint,
                        "default"
                    )
                    .is_err());
                    while let Ok(message) = raw.try_recv() {
                        assert!(!message.text.contains("SYNTHETIC_PRIVATE_KEY"));
                    }
                    repository
                        .mutate(|snapshot| {
                            *snapshot = previous;
                            Ok(())
                        })
                        .unwrap();
                    drop(store);
                    std::fs::remove_dir_all(root).unwrap();
                    return;
                }
                let binding = super::super::private_bound_execution::claim(
                    Arc::clone(&bindings),
                    &"b".repeat(32),
                    &"a".repeat(64),
                    &staged,
                    &endpoint,
                    "default",
                )
                .unwrap();
                assert!(super::super::private_bound_execution::claim(
                    Arc::clone(&bindings),
                    &"b".repeat(32),
                    &"a".repeat(64),
                    &staged,
                    &endpoint,
                    "default",
                )
                .is_err());
                let reference = bindings
                    .reconciliation_reference(&hex::encode(Sha256::digest("b".repeat(32))))
                    .unwrap();
                assert_eq!(bindings.status(&reference).unwrap(), "admitted");
                worker
                    .submit_bound_private(staged, Arc::clone(&controller), binding)
                    .await
                    .unwrap()
            } else {
                worker
                    .submit_private(staged, Arc::clone(&controller))
                    .await
                    .unwrap()
            }
            #[cfg(not(target_os = "linux"))]
            {
                worker
                    .submit_private(staged, Arc::clone(&controller))
                    .await
                    .unwrap()
            }
        } else {
            let state = direct_state.as_mut().unwrap();
            let references = stage(&store, &values);
            let pending = execute_private_journey(state, &store, &references)
                .await
                .unwrap();
            let pending = pending.sanitize_and_close(state, &store).await.unwrap();
            if !fault.is_empty() {
                repository
                    .mutate(|snapshot| {
                        match fault {
                            "profile" => {
                                snapshot
                                    .browsers
                                    .get_mut("session:default")
                                    .unwrap()
                                    .profile_id = Some("other".into())
                            }
                            "session" => {
                                snapshot.sessions.get_mut("default").unwrap().profile_id =
                                    Some("other".into())
                            }
                            "target" => {
                                snapshot.tabs.get_mut("tab").unwrap().target_id =
                                    Some("other".into())
                            }
                            "url" => {
                                snapshot.tabs.get_mut("tab").unwrap().url =
                                    Some("https://sam.gov/wrong".into())
                            }
                            "lease" => {
                                snapshot.sessions.get_mut("default").unwrap().lease =
                                    LeaseState::Shared
                            }
                            _ => {}
                        }
                        Ok(())
                    })
                    .unwrap();
                let epoch = pending.permit.epoch_id().to_owned();
                let endpoint = state.browser.as_ref().unwrap().get_cdp_url().to_owned();
                let result =
                    pending.reconcile_with_checkpoints(state, &store, &controller, |boundary| {
                        if fault.strip_prefix("crash:") == Some(boundary) {
                            use std::io::Write;
                            use std::os::unix::fs::OpenOptionsExt;
                            let handoff =
                                serde_json::to_vec(&json!({"epoch":epoch,"endpoint":endpoint}))
                                    .unwrap();
                            let path = std::path::PathBuf::from(
                                std::env::var("AB_PRIVATE_CRASH_ROOT").unwrap(),
                            )
                            .join("handoff.json");
                            let mut file = std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .mode(0o600)
                                .open(path)
                                .unwrap();
                            file.write_all(&handoff).unwrap();
                            file.sync_all().unwrap();
                            unsafe {
                                libc::kill(libc::getpid(), libc::SIGKILL);
                            }
                            std::process::exit(98);
                        }
                        if boundary == fault {
                            Err("synthetic_interruption")
                        } else {
                            Ok(())
                        }
                    });
                assert!(result.is_err(), "fault must not return completion: {fault}");
                assert!(client.public_lease().is_err());
                let effects = tokio::time::timeout(std::time::Duration::from_secs(2), peer)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    effects,
                    ["attach", "login", "backup", "renew", "read", "sanitize", "detach"]
                );
                let restaged = stage(&store, &values);
                assert_eq!(
                    store.reserve_journey(&"a".repeat(64), &restaged),
                    Err("private_journey_consent_consumed")
                );
                let snapshot = repository.load_snapshot().unwrap();
                if matches!(fault, "broker_committed" | "daemon_updated") {
                    assert_eq!(
                        snapshot.browsers["session:default"].health,
                        BrowserHealth::CdpDisconnected
                    );
                    assert_eq!(snapshot.tabs["tab"].url.as_deref(), Some("about:blank"));
                }
                repository
                    .mutate(|snapshot| {
                        *snapshot = previous;
                        Ok(())
                    })
                    .unwrap();
                drop(store);
                std::fs::remove_dir_all(root).unwrap();
                return;
            }
            pending.reconcile(state, &store, &controller).unwrap()
        };
        assert!(closed
            .pending
            .transport_closed
            .matches(&client, &closed.pending.permit));
        assert_eq!(
            store.load_result(&closed.reconciliation_reference).unwrap(),
            b"{\"reconciled\":true,\"privacyReleased\":false}"
        );
        let reconciled = repository.load_snapshot().unwrap();
        assert_eq!(
            reconciled.browsers["session:default"].health,
            BrowserHealth::CdpDisconnected
        );
        assert_eq!(reconciled.tabs["tab"].url.as_deref(), Some("about:blank"));
        assert_ne!(reconciled.tabs["tab"].lifecycle, TabLifecycle::Ready);
        assert!(!reconciled.service_tab_handle("tab").unwrap().valid);
        assert!(client.public_lease().is_err());
        let effects = tokio::time::timeout(std::time::Duration::from_secs(2), peer)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            effects,
            ["attach", "login", "backup", "renew", "read", "sanitize", "detach"]
        );
        // Only public preflight metadata may precede privacy admission.
        while let Ok(message) = raw.try_recv() {
            assert!(!message.text.contains("SYNTHETIC_PRIVATE_KEY"));
        }
        let restaged = stage(&store, &values);
        assert_eq!(
            store.reserve_journey(&"a".repeat(64), &restaged),
            Err("private_journey_consent_consumed")
        );
        if let Some(state) = direct_state.as_ref() {
            assert_eq!(
                state.browser.as_ref().unwrap().active_page_url(),
                Some("about:blank")
            );
            assert_eq!(
                state.browser.as_ref().unwrap().active_target_id().unwrap(),
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            );
        }
        assert_eq!(
            repository.load_snapshot().unwrap().tabs["tab"]
                .target_id
                .as_deref(),
            Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
        );
        repository
            .mutate(|state| {
                *state = previous;
                Ok(())
            })
            .unwrap();
        if fault == "bound" {
            let reference = closed.pending.result_reference.clone();
            let (producer, consumer) = tokio::net::UnixStream::pair().unwrap();
            let receiver = tokio::spawn(super::super::private_delivery::tests::receiver(
                consumer,
                false,
                "SYNTHETIC_PRIVATE_KEY_123456789",
            ));
            closed.deliver(producer, [7; 32]).await.unwrap();
            receiver.await.unwrap();
            assert!(store.admit_delivery(&reference).is_err());
            assert!(client.public_lease().is_err());
        } else {
            drop(closed);
        }
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn payloads() -> Vec<Value> {
        [
            json!({"kind":"login","email_selector":"#email","password_selector":"#password",
                "submit_selector":"#submit","email":"user@example.test","password":"SYNTHETIC_SECRET"}),
            json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit",
                "account_selector":"#account","account_value":"user@example.test",
                "code":"SYNTHETIC_CODE","source_scope":"synthetic-source:1"}),
            json!({"kind":"renew","submit_selector":"#renew","account_selector":"#account",
                "account_value":"user@example.test"}),
            json!({"kind":"read_key","selector":"#key","account_selector":"#account",
                "account_value":"user@example.test"}),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, mut operation)| {
            if index == 1 { operation["code"] = json!("SYNTHETIC-CODE"); }
            let origin = if index < 2 { "https://secure.login.gov" } else { "https://sam.gov" };
            json!({"schema":"agent-browser.private-operation.v1",
                "service_tab_handle":{"profileId":"sam","browserId":"session:default",
                    "sessionName":"default","tabId":"tab","targetId":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                    "url":format!("{origin}/"),"leaseId":"default","leaseState":"exclusive",
                    "leaseHeartbeatExpected":true,"ownerSessionId":"default","valid":true,"staleReason":null},
                "expected_origin":origin,"expected_url":format!("{origin}/"),
                "consent_sha256":"a".repeat(64),"account_scope":"login.gov:user@example.test",
                "operation":operation})
        }).collect()
    }

    fn parsed(values: &[Value]) -> Vec<PrivateOperation> {
        values
            .iter()
            .map(|value| PrivateOperation::parse(&serde_json::to_vec(value).unwrap()).unwrap())
            .collect()
    }

    fn stage(store: &SecretStore, values: &[Value]) -> Vec<String> {
        values
            .iter()
            .map(|value| store.stage(&serde_json::to_vec(value).unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn sequence_accepts_same_url_stages_but_rejects_substitution_and_scope_drift() {
        let values = payloads();
        assert!(validate_sequence(&parsed(&values)).is_ok());
        for field in [
            "profileId",
            "browserId",
            "sessionName",
            "targetId",
            "tabId",
            "leaseId",
            "ownerSessionId",
            "url",
        ] {
            let mut changed = values.clone();
            changed[3]["service_tab_handle"][field] = json!("wrong");
            assert_eq!(validate_sequence(&parsed(&changed)), Err(INVALID));
        }
        let mut changed = values.clone();
        changed[3]["consent_sha256"] = json!("b".repeat(64));
        assert_eq!(validate_sequence(&parsed(&changed)), Err(INVALID));
        changed = values.clone();
        changed[3]["account_scope"] = json!("login.gov:other@example.test");
        changed[3]["operation"]["account_value"] = json!("other@example.test");
        assert_eq!(validate_sequence(&parsed(&changed)), Err(INVALID));
        changed = values.clone();
        changed.swap(2, 3);
        assert_eq!(validate_sequence(&parsed(&changed)), Err(INVALID));
        changed = values.clone();
        changed[3] = changed[2].clone();
        assert_eq!(validate_sequence(&parsed(&changed)), Err(INVALID));
        assert_eq!(validate_sequence(&[]), Err(INVALID));
    }

    #[tokio::test]
    async fn invalid_final_stage_fails_before_browser_admission() {
        let root =
            std::env::temp_dir().join(format!("ab-private-journey-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        let mut values = payloads();
        values[3]["service_tab_handle"]["valid"] = json!(false);
        let references = stage(&store, &values);
        let state = DaemonState::new();
        assert!(matches!(
            execute_private_journey(&state, &store, &references).await,
            Err(INVALID)
        ));
        let mut duplicate = references.clone();
        duplicate[3] = duplicate[2].clone();
        assert!(matches!(load_sequence(&store, &duplicate), Err(INVALID)));
        // No reservation occurred on invalid input, even though the last stage
        // was the only bad one. No browser is configured in this fixture.
        assert!(store.reserve_journey(&"a".repeat(64), &references).is_ok());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journey_claim_survives_reopen_and_fresh_staged_references() {
        let root =
            std::env::temp_dir().join(format!("ab-private-journey-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        let references = stage(&store, &payloads());
        store.reserve_journey(&"a".repeat(64), &references).unwrap();
        drop(store);
        let reopened = SecretStore::open(&root).unwrap();
        let restaged = stage(&reopened, &payloads());
        assert_ne!(references, restaged);
        assert_eq!(
            reopened.reserve_journey(&"a".repeat(64), &restaged),
            Err("private_journey_consent_consumed")
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journey_claim_crash_helper() {
        let Some(root) = std::env::var_os("AB_PRIVATE_JOURNEY_CRASH_FIXTURE") else {
            return;
        };
        let store = SecretStore::open(std::path::Path::new(&root)).unwrap();
        let references = stage(&store, &payloads());
        store.reserve_journey(&"a".repeat(64), &references).unwrap();
        // Exit without destructors immediately after the durable claim.
        std::process::exit(29);
    }

    #[test]
    fn process_death_after_claim_cannot_replay_with_new_references() {
        let root =
            std::env::temp_dir().join(format!("ab-private-journey-{}", uuid::Uuid::new_v4()));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::private_journey::tests::journey_claim_crash_helper",
            ])
            .env("AB_PRIVATE_JOURNEY_CRASH_FIXTURE", &root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(29));
        let store = SecretStore::open(&root).unwrap();
        let references = stage(&store, &payloads());
        assert_eq!(
            store.reserve_journey(&"a".repeat(64), &references),
            Err("private_journey_consent_consumed")
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_same_consent_claim_has_exactly_one_winner() {
        let root =
            std::env::temp_dir().join(format!("ab-private-journey-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        let references = stage(&store, &payloads());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        // Opening a store also takes a nonblocking lock. Prepare independent
        // handles before racing claims so setup cannot panic before the barrier.
        let contenders = (0..2)
            .map(|_| SecretStore::open(&root).unwrap())
            .collect::<Vec<_>>();
        let workers = contenders
            .into_iter()
            .map(|store| {
                let references = references.clone();
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    store.reserve_journey(&"a".repeat(64), &references)
                })
            })
            .collect::<Vec<_>>();
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        // The store intentionally uses nonblocking locks, so the loser may
        // reject contention before it can read the committed consent receipt.
        assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);
        assert!(outcomes.iter().all(|result| matches!(
            result,
            Ok(()) | Err("private_store_busy" | "private_journey_consent_consumed")
        )));
        assert_eq!(
            store.reserve_journey(&"a".repeat(64), &references),
            Err("private_journey_consent_consumed")
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
