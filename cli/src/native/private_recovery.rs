//! Offline recovery of a durably admitted, already detached reconciliation.
//! This never connects to Chrome, replays operations, delivers a key, or opens
//! observation. The independent controller must retain the exact intent digest.

use super::privacy_gate::{PrivacyGate, PrivatePermit};
use super::private_broker::validate_broker_snapshot;
use super::private_identity::{validate_private_target, LivePrivateIdentity};
use super::private_operation::{Operation, PrivateOperation};
use super::private_secret_store::SecretStore;
use super::service_model::{BrowserHealth, TabLifecycle};
use super::service_store::LockedServiceStateRepository;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const FAILED: &str = "private_recovery_failed_closed";
const COMPLETED: &[u8] = b"{\"reconciled\":true,\"privacyReleased\":false}";

/// Trusted independent controller input, never deserialized from a job or
/// inferred from the intent under examination. Possession does not release a gate.
pub(crate) struct RecoveryAuthority {
    intent_sha256: String,
    endpoint: String,
    epoch: String,
    session_name: String,
}

impl RecoveryAuthority {
    pub(crate) fn new(
        intent_sha256: String,
        endpoint: String,
        epoch: String,
        session_name: String,
    ) -> Self {
        Self {
            intent_sha256,
            endpoint,
            epoch,
            session_name,
        }
    }
}

/// No serialization, cloning, key accessor, or privacy-release API. Dropping
/// this proof preserves the durable closed epoch for another explicit recovery.
pub(crate) struct RecoveredReconciliation {
    permit: PrivatePermit,
    reconciliation_reference: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Intent {
    schema: String,
    epoch: String,
    scope_digest: String,
    profile_id: String,
    browser_id: String,
    session_name: String,
    target_id: String,
    tab_id: String,
    endpoint: String,
    result_reference: String,
    prior_handle: Value,
    sanitized_url: String,
    detach_reference: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Detach {
    schema: String,
    attachment_reference: String,
    epoch: String,
    target_id: String,
    session_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttachmentAdmission {
    schema: String,
    epoch: String,
    target_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttachmentIdentity {
    schema: String,
    epoch: String,
    target_id: String,
    session_id: String,
}

fn verify_detach(store: &SecretStore, intent: &Intent) -> Result<(), &'static str> {
    if store.status(&intent.detach_reference)? != "result_ready"
        || store.load_result(&intent.detach_reference)? != b"{\"detached\":true}"
    {
        return Err(FAILED);
    }
    let detach: Detach =
        serde_json::from_slice(&store.load(&intent.detach_reference)?).map_err(|_| FAILED)?;
    if detach.schema != "private-attachment-detach.v1"
        || detach.epoch != intent.epoch
        || detach.target_id != intent.target_id
        || detach.session_id.is_empty()
        || detach.session_id.len() > 4096
        || detach.session_id.trim() != detach.session_id
        || detach.session_id.chars().any(char::is_control)
        || store.status(&detach.attachment_reference)? != "result_ready"
    {
        return Err(FAILED);
    }
    let admission: AttachmentAdmission =
        serde_json::from_slice(&store.load(&detach.attachment_reference)?).map_err(|_| FAILED)?;
    let identity: AttachmentIdentity =
        serde_json::from_slice(&store.load_result(&detach.attachment_reference)?)
            .map_err(|_| FAILED)?;
    if admission.schema != "private-attachment-admission.v1"
        || identity.schema != "private-attachment-identity.v1"
        || admission.epoch != intent.epoch
        || identity.epoch != intent.epoch
        || admission.target_id != intent.target_id
        || identity.target_id != intent.target_id
        || identity.session_id != detach.session_id
    {
        return Err(FAILED);
    }
    Ok(())
}

/// Reconcile only the broker metadata described by an admitted v2 intent.
/// The private gate is locked before broker access and retained on success;
/// every error is a fixed label and leaves observation closed. No CDP exists
/// on this path, so even process-crash recovery cannot replay a browser action.
pub(crate) fn recover_reconciliation(
    store: &SecretStore,
    reference: &str,
    authority: RecoveryAuthority,
) -> Result<RecoveredReconciliation, &'static str> {
    recover(store, reference, authority).map_err(|_| FAILED)
}

fn recover(
    store: &SecretStore,
    reference: &str,
    authority: RecoveryAuthority,
) -> Result<RecoveredReconciliation, &'static str> {
    let bytes = store.load(reference)?;
    if hex::encode(Sha256::digest(&bytes)) != authority.intent_sha256 {
        return Err(FAILED);
    }
    let intent: Intent = serde_json::from_slice(&bytes).map_err(|_| FAILED)?;
    if intent.schema != "private-reconciliation.v2"
        || intent.endpoint != authority.endpoint
        || intent.epoch != authority.epoch
        || intent.session_name != authority.session_name
        || intent.browser_id != format!("session:{}", authority.session_name)
        || intent.sanitized_url != "about:blank"
        || store.reconciliation_reference(&authority.epoch)? != reference
    {
        return Err(FAILED);
    }
    let status = store.status(reference)?;
    match status {
        "admitted" => {}
        "result_ready" if store.load_result(reference)? == COMPLETED => {}
        _ => return Err(FAILED),
    }
    let operation = PrivateOperation::parse(&store.load(&intent.result_reference)?)?;
    if !matches!(operation.operation(), Operation::ReadKey { .. })
        || operation.service_tab_handle() != &intent.prior_handle
        || intent.prior_handle.get("tabId").and_then(Value::as_str) != Some(intent.tab_id.as_str())
        || store.status(&intent.result_reference)? != "result_ready"
    {
        return Err(FAILED);
    }
    // Require the encrypted result to exist but never return its plaintext.
    store.load_result(&intent.result_reference)?;
    let binding = LivePrivateIdentity {
        profile_id: &intent.profile_id,
        browser_id: &intent.browser_id,
        session_name: &intent.session_name,
        target_id: &intent.target_id,
        url: operation.expected_url(),
        ready: true,
    };
    // Historical binding only, not a replacement for a live browser probe.
    let expected = validate_private_target(
        &intent.prior_handle,
        &intent.prior_handle,
        &binding,
        operation.expected_origin(),
        operation.expected_url(),
    )?;
    if expected.scope_digest() != intent.scope_digest {
        return Err(FAILED);
    }
    verify_detach(store, &intent)?;
    let permit = PrivacyGate::for_endpoint(&authority.endpoint)?.resume_private()?;
    if permit.epoch_id() != intent.epoch
        || permit.target_id() != Some(intent.target_id.as_str())
        || permit.identity_digest() != Some(intent.scope_digest.as_str())
        || permit.journey_stage() != Some(3)
    {
        return Err(FAILED);
    }
    LockedServiceStateRepository::default_json()
        .map_err(|_| FAILED)?
        .try_mutate_durable(|persisted| {
            let browser = persisted.browsers.get(&intent.browser_id).ok_or(FAILED)?;
            let tab = persisted.tabs.get(&intent.tab_id).ok_or(FAILED)?;
            let mut historical = persisted.clone();
            match browser.health {
                BrowserHealth::Ready if tab.lifecycle == TabLifecycle::Ready => {
                    if status == "result_ready" {
                        return Err(FAILED.to_owned());
                    }
                }
                BrowserHealth::CdpDisconnected
                    if tab.lifecycle == TabLifecycle::Unknown
                        && tab.url.as_deref() == Some("about:blank")
                        && tab.title.is_none()
                        // Normal repository loading rebuilds this derived
                        // handle. It must remain unusable, not necessarily absent.
                        && tab.service_tab_handle.as_ref().is_none_or(|handle| !handle.valid)
                        && tab.latest_snapshot_id.is_none()
                        && tab.latest_screenshot_id.is_none() =>
                {
                    historical
                        .browsers
                        .get_mut(&intent.browser_id)
                        .ok_or(FAILED)?
                        .health = BrowserHealth::Ready;
                    let prior = historical.tabs.get_mut(&intent.tab_id).ok_or(FAILED)?;
                    prior.url = Some(operation.expected_url().to_owned());
                    prior.lifecycle = TabLifecycle::Ready;
                }
                _ => return Err(FAILED.to_owned()),
            }
            let validated = validate_broker_snapshot(
                &historical,
                &intent.prior_handle,
                &binding,
                &intent.endpoint,
                operation.expected_origin(),
                operation.expected_url(),
            )
            .map_err(str::to_owned)?;
            if validated.scope_digest() != intent.scope_digest {
                return Err(FAILED.to_owned());
            }
            let tab = persisted.tabs.get_mut(&intent.tab_id).ok_or(FAILED)?;
            tab.url = Some("about:blank".into());
            tab.title = None;
            tab.lifecycle = TabLifecycle::Unknown;
            tab.service_tab_handle = None;
            tab.latest_snapshot_id = None;
            tab.latest_screenshot_id = None;
            persisted
                .browsers
                .get_mut(&intent.browser_id)
                .ok_or(FAILED)?
                .health = BrowserHealth::CdpDisconnected;
            Ok(())
        })
        .map_err(|_| FAILED)?;
    if status == "admitted" {
        store.store_result(reference, COMPLETED)?;
    }
    Ok(RecoveredReconciliation {
        permit,
        reconciliation_reference: reference.to_owned(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fixture(std::path::PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("ab-private-recovery-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn intent(detach_reference: String) -> Intent {
        Intent {
            schema: "private-reconciliation.v2".into(),
            epoch: "epoch".into(),
            scope_digest: "scope".into(),
            profile_id: "profile".into(),
            browser_id: "session:fixture".into(),
            session_name: "fixture".into(),
            target_id: "target".into(),
            tab_id: "tab".into(),
            endpoint: "ws://127.0.0.1:9222/devtools/browser/fixture".into(),
            result_reference: "unused".into(),
            prior_handle: Value::Null,
            sanitized_url: "about:blank".into(),
            detach_reference,
        }
    }

    fn detach_fixture(store: &SecretStore, identity_target: &str, complete: bool) -> Intent {
        let admission = store
            .stage(
                &serde_json::to_vec(&json!({
                    "schema":"private-attachment-admission.v1",
                    "epoch":"epoch", "targetId":"target"
                }))
                .unwrap(),
            )
            .unwrap();
        store.admit(&admission).unwrap();
        store
            .store_result(
                &admission,
                &serde_json::to_vec(&json!({
                    "schema":"private-attachment-identity.v1", "epoch":"epoch",
                    "targetId":identity_target,"sessionId":"private-session"
                }))
                .unwrap(),
            )
            .unwrap();
        let reference = store
            .stage(
                &serde_json::to_vec(&json!({
                    "schema":"private-attachment-detach.v1", "attachmentReference":admission,
                    "epoch":"epoch","targetId":"target","sessionId":"private-session"
                }))
                .unwrap(),
            )
            .unwrap();
        store.admit(&reference).unwrap();
        if complete {
            store
                .store_result(&reference, b"{\"detached\":true}")
                .unwrap();
        }
        intent(reference)
    }

    #[test]
    fn detach_chain_requires_exact_owned_identity_and_completed_ack() {
        let dir = Fixture::new();
        let store = SecretStore::open(&dir.0.join("store")).unwrap();
        assert!(verify_detach(&store, &detach_fixture(&store, "target", true)).is_ok());
        assert!(verify_detach(&store, &detach_fixture(&store, "other-target", true)).is_err());
        assert!(verify_detach(&store, &detach_fixture(&store, "target", false)).is_err());
    }

    #[test]
    fn recovery_refuses_wrong_digest_before_parsing_or_gate_access() {
        let dir = Fixture::new();
        let store = SecretStore::open(&dir.0.join("store")).unwrap();
        let reference = store.stage(b"not an intent").unwrap();
        let authority = RecoveryAuthority::new(
            "wrong".into(),
            "not an endpoint".into(),
            "epoch".into(),
            "fixture".into(),
        );
        assert!(matches!(
            recover_reconciliation(&store, &reference, authority),
            Err(FAILED)
        ));
        assert_eq!(store.status(&reference).unwrap(), "staged");
    }

    #[test]
    fn intent_parser_rejects_missing_detach_and_unknown_fields() {
        let mut value = json!({
            "schema":"private-reconciliation.v2","epoch":"epoch","scopeDigest":"scope",
            "profileId":"profile","browserId":"session:fixture","sessionName":"fixture",
            "targetId":"target","tabId":"tab","endpoint":"ws://127.0.0.1:9222/",
            "resultReference":"result","priorHandle":{},"sanitizedUrl":"about:blank"
        });
        assert!(serde_json::from_value::<Intent>(value.clone()).is_err());
        value["detachReference"] = json!("detach");
        assert!(serde_json::from_value::<Intent>(value.clone()).is_ok());
        value["releasePrivacy"] = json!(true);
        assert!(serde_json::from_value::<Intent>(value).is_err());
    }
}
