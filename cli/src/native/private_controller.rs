//! Source-only durable controller handoff for offline reconciliation recovery.
//! The embedding trusted caller must independently provision this controller's
//! endpoint, session and separate authority journal. This module does not mint
//! consent or establish that caller's trust. No installed producer, listener,
//! browser access, public receipt, consumer delivery or privacy release exists.

use super::private_recovery::{recover_reconciliation, RecoveredReconciliation, RecoveryAuthority};
use super::private_secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const FAILED: &str = "private_controller_failed_closed";
const SCHEMA: &str = "private-controller-authority.v1";

/// Nonserializable trusted capability. A separate journal is necessary but does
/// not make an untrusted constructor caller into an independent authority.
pub(crate) struct RecoveryController {
    journal: Arc<SecretStore>,
    endpoint: String,
    session_name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthorityRecord {
    schema: String,
    epoch: String,
    reference: String,
    intent_sha256: String,
    endpoint: String,
    session_name: String,
}

fn valid_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RecoveryController {
    pub(crate) fn new(
        journal: Arc<SecretStore>,
        endpoint: String,
        session_name: String,
    ) -> Result<Self, &'static str> {
        if endpoint.is_empty()
            || session_name.is_empty()
            || endpoint.chars().any(char::is_control)
            || session_name.chars().any(char::is_control)
        {
            return Err(FAILED);
        }
        Ok(Self {
            journal,
            endpoint,
            session_name,
        })
    }

    /// Verify independently pinned routing before any journey action. The
    /// worker must repeat this check at reconciliation, before retaining proof.
    pub(crate) fn validate_scope(
        &self,
        operation_store: &SecretStore,
        endpoint: &str,
        session_name: &str,
    ) -> Result<(), &'static str> {
        if endpoint != self.endpoint
            || session_name != self.session_name
            || operation_store
                .same_directory(&self.journal)
                .map_err(|_| FAILED)?
        {
            return Err(FAILED);
        }
        Ok(())
    }

    /// Retain an immutable binding before intent admission. The caller provides
    /// the digest of the intent it just prepared and verified; restart never
    /// reconstructs this authority from the operation payload under examination.
    /// Failure can leave encrypted orphan staging, never an admitted operation.
    pub(crate) fn retain(
        &self,
        operation_store: &SecretStore,
        epoch: &str,
        reference: &str,
        intent_sha256: &str,
    ) -> Result<(), &'static str> {
        let retain = || {
            if !valid_hex(epoch)
                || !valid_hex(reference)
                || !valid_hex(intent_sha256)
                || operation_store.same_directory(&self.journal)?
                || operation_store.status(reference)? != "staged"
                || operation_store.reconciliation_reference(epoch)? != reference
                || hex::encode(Sha256::digest(operation_store.load(reference)?)) != intent_sha256
            {
                return Err(FAILED);
            }
            let record = AuthorityRecord {
                schema: SCHEMA.into(),
                epoch: epoch.into(),
                reference: reference.into(),
                intent_sha256: intent_sha256.into(),
                endpoint: self.endpoint.clone(),
                session_name: self.session_name.clone(),
            };
            let bytes = serde_json::to_vec(&record).map_err(|_| FAILED)?;
            let authority_reference = self.journal.stage(&bytes)?;
            // create-new bookmark semantics reject duplicate epoch bindings,
            // including identical retries, without replacing existing authority.
            self.journal
                .bind_reconciliation(epoch, &authority_reference)
        };
        retain().map_err(|_| FAILED)
    }

    fn load_authority(&self, epoch: &str) -> Result<AuthorityRecord, &'static str> {
        if !valid_hex(epoch) {
            return Err(FAILED);
        }
        let reference = self.journal.reconciliation_reference(epoch)?;
        let record: AuthorityRecord =
            serde_json::from_slice(&self.journal.load(&reference)?).map_err(|_| FAILED)?;
        if record.schema != SCHEMA
            || record.epoch != epoch
            || record.endpoint != self.endpoint
            || record.session_name != self.session_name
            || !valid_hex(&record.reference)
            || !valid_hex(&record.intent_sha256)
        {
            return Err(FAILED);
        }
        Ok(record)
    }

    /// Dispatch historical metadata recovery only. No daemon/browser/client is
    /// accepted, and possession of its private result never unlocks observation.
    pub(crate) fn recover(
        &self,
        operation_store: &SecretStore,
        epoch: &str,
    ) -> Result<RecoveredReconciliation, &'static str> {
        let recover = || {
            if operation_store.same_directory(&self.journal)? {
                return Err(FAILED);
            }
            let record = self.load_authority(epoch)?;
            recover_reconciliation(
                operation_store,
                &record.reference,
                RecoveryAuthority::new(
                    record.intent_sha256,
                    record.endpoint,
                    record.epoch,
                    record.session_name,
                ),
            )
        };
        recover().map_err(|_| FAILED)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use std::path::PathBuf;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("ab-private-controller-{}", uuid::Uuid::new_v4()));
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&root)
                .unwrap();
            Self(root)
        }
        fn store(&self, name: &str) -> SecretStore {
            SecretStore::open(&self.0.join(name)).unwrap()
        }
        fn controller(&self) -> RecoveryController {
            RecoveryController::new(
                Arc::new(self.store("journal")),
                "ws://127.0.0.1:1/synthetic".into(),
                "synthetic".into(),
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn staged(store: &SecretStore, epoch: &str) -> (String, String) {
        let payload = b"PRIVATE_CONTROLLER_SYNTHETIC_SENTINEL";
        let reference = store.stage(payload).unwrap();
        store.bind_reconciliation(epoch, &reference).unwrap();
        (reference, hex::encode(Sha256::digest(payload)))
    }

    #[test]
    fn controller_retains_before_admission_and_reopens_immutable_binding() {
        let fixture = Fixture::new();
        let store = fixture.store("operations");
        let epoch = "a".repeat(64);
        let (reference, digest) = staged(&store, &epoch);
        let controller = fixture.controller();
        controller
            .retain(&store, &epoch, &reference, &digest)
            .unwrap();
        assert_eq!(store.status(&reference).unwrap(), "staged");
        assert_eq!(
            controller.retain(&store, &epoch, &reference, &digest),
            Err(FAILED)
        );
        drop(controller);
        let controller = fixture.controller();
        let record = controller.load_authority(&epoch).unwrap();
        assert_eq!(record.reference, reference);
        assert_eq!(record.intent_sha256, digest);
        for name in ["operations", "journal"] {
            for entry in std::fs::read_dir(fixture.0.join(name)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    let bytes = std::fs::read(path).unwrap();
                    for sentinel in [
                        b"PRIVATE_CONTROLLER_SYNTHETIC_SENTINEL".as_slice(),
                        SCHEMA.as_bytes(),
                        controller.endpoint.as_bytes(),
                    ] {
                        assert!(!bytes.windows(sentinel.len()).any(|part| part == sentinel));
                    }
                }
            }
        }
        assert_eq!(
            controller.retain(&store, &epoch, &reference, &digest),
            Err(FAILED)
        );
        // Synthetic bytes cannot become a reconciliation capability.
        assert!(matches!(controller.recover(&store, &epoch), Err(FAILED)));
        store.admit(&reference).unwrap();
        assert!(matches!(controller.recover(&store, &epoch), Err(FAILED)));
    }

    #[test]
    fn controller_rejects_wrong_missing_and_aliased_authority() {
        let fixture = Fixture::new();
        let store = fixture.store("operations");
        let controller = fixture.controller();
        let epoch = "b".repeat(64);
        let (reference, digest) = staged(&store, &epoch);
        assert!(controller
            .validate_scope(&store, &controller.endpoint, &controller.session_name)
            .is_ok());
        assert_eq!(
            controller.validate_scope(&store, "wrong", &controller.session_name),
            Err(FAILED)
        );
        assert_eq!(
            controller.validate_scope(&store, &controller.endpoint, "wrong"),
            Err(FAILED)
        );
        assert!(matches!(controller.recover(&store, &epoch), Err(FAILED)));
        assert_eq!(
            controller.retain(&store, &epoch, &reference, &"0".repeat(64)),
            Err(FAILED)
        );
        assert_eq!(
            controller.retain(&store, &"c".repeat(64), &reference, &digest),
            Err(FAILED)
        );
        let alias = fixture.store("journal");
        assert_eq!(
            controller.validate_scope(&alias, &controller.endpoint, &controller.session_name),
            Err(FAILED)
        );
        let (alias_reference, alias_digest) = staged(&alias, &epoch);
        assert_eq!(
            controller.retain(&alias, &epoch, &alias_reference, &alias_digest),
            Err(FAILED)
        );
        assert!(matches!(controller.recover(&alias, &epoch), Err(FAILED)));
        store.admit(&reference).unwrap();
        assert_eq!(
            controller.retain(&store, &epoch, &reference, &digest),
            Err(FAILED)
        );
    }

    #[test]
    fn controller_rejects_malformed_or_wrong_scope_journal_records() {
        let fixture = Fixture::new();
        let controller = fixture.controller();
        let store = fixture.store("operations");
        for (index, change) in [
            "schema",
            "epoch",
            "endpoint",
            "sessionName",
            "reference",
            "intentSha256",
            "extra",
        ]
        .iter()
        .enumerate()
        {
            let epoch = format!("{index:064x}");
            let mut value = serde_json::json!({
                "schema": SCHEMA, "epoch": epoch, "reference": "d".repeat(64),
                "intentSha256": "e".repeat(64), "endpoint": controller.endpoint,
                "sessionName": controller.session_name
            });
            value[*change] = serde_json::json!("wrong");
            let reference = controller
                .journal
                .stage(&serde_json::to_vec(&value).unwrap())
                .unwrap();
            controller
                .journal
                .bind_reconciliation(&epoch, &reference)
                .unwrap();
            assert!(matches!(controller.recover(&store, &epoch), Err(FAILED)));
        }
    }

    #[test]
    fn controller_rejects_empty_or_control_character_scope() {
        let fixture = Fixture::new();
        for (endpoint, session) in [
            ("", "session"),
            ("endpoint", ""),
            ("endpoint\n", "session"),
            ("endpoint", "session\0"),
        ] {
            assert!(matches!(
                RecoveryController::new(
                    Arc::new(fixture.store("journal")),
                    endpoint.into(),
                    session.into(),
                ),
                Err(FAILED)
            ));
        }
    }
}
