//! Endpoint-scoped cooperative privacy barrier. Only the trusted cleanup
//! coordinator may finish a private epoch; dropping a permit never reopens it.
//! DNS aliases, tunnels, and distinct configured homes are not globally proven
//! equivalent. This is not protection from the OS owner or out-of-band clients.
use super::private_identity::{ValidatedPrivateJourney, ValidatedPrivateTarget};
pub(crate) use super::private_secret_store::CleanupCommit;
use super::private_secret_store::{GateLease, PrivateScope, SecretStore};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct PrivacyGate {
    store: Arc<SecretStore>,
    endpoint: Option<String>,
}

/// Owned, Send, non-clone shared OS lease. Hold through response processing and
/// any resulting public output; a point-in-time boolean is insufficient.
pub struct PublicLease {
    _file: GateLease,
}

/// Public sockets/subscriptions are permanently bound to the epoch in which
/// they were created. Cleanup permits new observers, never revives old ones.
#[derive(Clone)]
pub(crate) struct PrivacyObserver {
    gate: Arc<PrivacyGate>,
    captured_epoch: Option<String>,
    private: bool,
}

impl PrivacyObserver {
    pub(crate) fn public_lease(&self) -> Result<PublicLease, &'static str> {
        if self.private {
            return Err("privacy_observer_private");
        }
        let lease = self.gate.public_lease()?;
        if self.gate.store.gate_public_epoch()? != self.captured_epoch {
            return Err("privacy_observer_revoked");
        }
        Ok(lease)
    }
}

/// Non-clone private capability. Its durable lock survives Drop and process exit.
pub struct PrivatePermit {
    file: Option<GateLease>,
    store: Arc<SecretStore>,
    epoch: String,
    completion: Option<CleanupCommit>,
    endpoint: Option<String>,
    scope: Option<PrivateScope>,
}

impl PrivatePermit {
    /// Internal cleanup binding only; this identifier is not an authority token.
    pub(crate) fn epoch_id(&self) -> &str {
        &self.epoch
    }

    /// Unscoped legacy/storage-only permits must never authorize transport.
    pub(crate) fn target_id(&self) -> Option<&str> {
        self.scope.as_ref().map(PrivateScope::target_id)
    }
    pub(crate) fn identity_digest(&self) -> Option<&str> {
        self.scope.as_ref().map(PrivateScope::identity_digest)
    }
    pub(crate) fn journey_stage(&self) -> Option<usize> {
        self.scope.as_ref().and_then(PrivateScope::journey_stage)
    }

    /// Advance only to the next precommitted exact destination, with explicit
    /// stage CAS even when consecutive stages share a URL. Fresh identity must
    /// come from the coordinator's locked broker and private CDP validation.
    pub(crate) fn advance_target(
        &mut self,
        expected_stage: usize,
        fresh: &ValidatedPrivateTarget,
    ) -> Result<(), &'static str> {
        let file = self.file.as_ref().ok_or("privacy_gate_consumed_permit")?;
        let previous = self.identity_digest().ok_or("private_scope_missing")?;
        let result = self.store.gate_advance_private(
            file,
            &self.epoch,
            previous,
            expected_stage,
            fresh.target_id(),
            fresh.scope_digest(),
        );
        match result {
            Ok(scope) => {
                self.scope = Some(scope);
                Ok(())
            }
            Err(error) => {
                // No stale local capability survives failed/uncertain commit.
                // Durable private state remains closed for cleanup recovery.
                self.file.take();
                Err(error)
            }
        }
    }
    /// Reuse the already locked store without trying to acquire a conflicting
    /// shared initialization lock. A different transport authority is rejected.
    pub(crate) fn recovery_gate(&self, endpoint: &str) -> Result<Arc<PrivacyGate>, &'static str> {
        let key = endpoint_key(endpoint)?;
        if self.endpoint.as_deref() != Some(key.as_str()) {
            return Err("private_permit_mismatch");
        }
        let gate = Arc::new(PrivacyGate {
            store: Arc::clone(&self.store),
            endpoint: Some(key),
        });
        if !self.authorizes(&gate) {
            return Err("private_permit_mismatch");
        }
        Ok(gate)
    }

    /// A private capability cannot be borrowed to operate another endpoint's
    /// gate. Independently opened handles to the same store are equivalent.
    pub(crate) fn authorizes(&self, gate: &PrivacyGate) -> bool {
        self.file
            .as_ref()
            .is_some_and(|file| gate.store.gate_permit_authorizes(file, &self.epoch).is_ok())
    }
}

/// A public command's crash-safe outstanding receipt. Drop releases the OS
/// lease but leaves the receipt blocking private admission across restarts.
pub struct CommandLease {
    store: Arc<SecretStore>,
    _file: GateLease,
    pending: Option<String>,
}

impl CommandLease {
    /// Call only after a matching browser response (including protocol errors).
    /// Keep the lease alive while any derived public output is being emitted.
    pub fn complete(&mut self) -> Result<(), &'static str> {
        if let Some(reference) = &self.pending {
            self.store.gate_complete_command(reference)?;
            self.pending = None;
        }
        Ok(())
    }
}

fn endpoint_key(endpoint: &str) -> Result<String, &'static str> {
    let url = url::Url::parse(endpoint).map_err(|_| "privacy_gate_invalid_endpoint")?;
    if !matches!(url.scheme(), "ws" | "wss")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("privacy_gate_invalid_endpoint");
    }
    let host = url
        .host_str()
        .ok_or("privacy_gate_invalid_endpoint")?
        .to_ascii_lowercase();
    let port = url
        .port_or_known_default()
        .ok_or("privacy_gate_invalid_endpoint")?;
    // Do not scope to a browser/target path: every target at this authority shares
    // the same gate. Scheme and effective port distinguish transport authorities.
    Ok(hex::encode(Sha256::digest(format!(
        "{}://{}:{}",
        url.scheme(),
        host,
        port
    ))))
}

impl PrivacyGate {
    /// Synthetic transports use real gate semantics without sharing operator state.
    #[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
    pub(crate) fn open_test_endpoint(
        root: &Path,
        endpoint: &str,
    ) -> Result<Arc<Self>, &'static str> {
        Ok(Arc::new(Self {
            store: Arc::new(SecretStore::open_gate(root)?),
            endpoint: Some(endpoint_key(endpoint)?),
        }))
    }

    pub fn for_endpoint(endpoint: &str) -> Result<Arc<Self>, &'static str> {
        let key = endpoint_key(endpoint)?;
        let base = match std::env::var_os("AGENT_BROWSER_HOME") {
            Some(value) => PathBuf::from(value),
            None => PathBuf::from(std::env::var_os("HOME").ok_or("privacy_gate_unavailable")?)
                .join(".agent-browser"),
        };
        SecretStore::ensure_gate_base(&base)?;
        let root = base.join("private-gates");
        SecretStore::ensure_private_directory(&root)?;
        Ok(Arc::new(Self {
            store: Arc::new(SecretStore::open_gate(&root.join(&key))?),
            endpoint: Some(key),
        }))
    }

    pub fn open(root: &Path) -> Result<Arc<Self>, &'static str> {
        Ok(Arc::new(Self {
            store: Arc::new(SecretStore::open_gate(root)?),
            endpoint: None,
        }))
    }

    pub fn public_lease(&self) -> Result<PublicLease, &'static str> {
        Ok(PublicLease {
            _file: self.store.gate_public_lease()?,
        })
    }

    pub(crate) fn public_observer(self: &Arc<Self>) -> Result<PrivacyObserver, &'static str> {
        let _lease = self.public_lease()?;
        let captured_epoch = self.store.gate_public_epoch()?;
        Ok(PrivacyObserver {
            gate: Arc::clone(self),
            captured_epoch,
            private: false,
        })
    }

    /// A recovery/private socket can never become an ordinary observer, even
    /// after cleanup. A fresh public connection must capture the new epoch.
    pub(crate) fn private_observer(self: &Arc<Self>) -> PrivacyObserver {
        PrivacyObserver {
            gate: Arc::clone(self),
            captured_epoch: None,
            private: true,
        }
    }

    pub fn command_lease(&self) -> Result<CommandLease, &'static str> {
        let (file, reference) = self.store.gate_command_lease()?;
        Ok(CommandLease {
            store: Arc::clone(&self.store),
            _file: file,
            pending: Some(reference),
        })
    }

    pub fn begin_private(&self) -> Result<PrivatePermit, &'static str> {
        let (file, epoch) = self.store.gate_begin_private()?;
        let scope = self.store.gate_private_scope(&epoch)?;
        Ok(PrivatePermit {
            file: Some(file),
            epoch,
            store: Arc::clone(&self.store),
            completion: None,
            endpoint: self.endpoint.clone(),
            scope,
        })
    }

    /// The target and reviewed identity digest are immutable for this epoch.
    /// Admission persists them before a capability can reach a CDP transport.
    pub(crate) fn begin_private_scoped(
        &self,
        target_id: &str,
        identity_digest: &str,
    ) -> Result<PrivatePermit, &'static str> {
        let (file, epoch) = self
            .store
            .gate_begin_private_scoped(target_id, identity_digest)?;
        let scope = self.store.gate_private_scope(&epoch)?;
        Ok(PrivatePermit {
            file: Some(file),
            epoch,
            store: Arc::clone(&self.store),
            completion: None,
            endpoint: self.endpoint.clone(),
            scope,
        })
    }

    pub(crate) fn begin_private_journey(
        &self,
        journey: &ValidatedPrivateJourney,
    ) -> Result<PrivatePermit, &'static str> {
        let (file, epoch) = self
            .store
            .gate_begin_private_journey(journey.target_id(), journey.digests())?;
        let scope = self.store.gate_private_scope(&epoch)?;
        Ok(PrivatePermit {
            file: Some(file),
            epoch,
            store: Arc::clone(&self.store),
            completion: None,
            endpoint: self.endpoint.clone(),
            scope,
        })
    }

    /// Internal recovery capability only; this neither proves cleanup nor
    /// releases the durable private marker or any outstanding command receipt.
    pub(crate) fn resume_private(&self) -> Result<PrivatePermit, &'static str> {
        let (file, epoch) = self.store.gate_resume_private()?;
        let scope = self.store.gate_private_scope(&epoch)?;
        Ok(PrivatePermit {
            file: Some(file),
            epoch,
            store: Arc::clone(&self.store),
            completion: None,
            endpoint: self.endpoint.clone(),
            scope,
        })
    }

    /// Trusted coordinator boundary, not an MCP/HTTP action. Invoke only after
    /// actual private cleanup is verified; no caller boolean is accepted.
    /// Pending and uncertainty evidence is never cleared by this operation.
    pub(crate) fn finish_private(
        &self,
        permit: &mut PrivatePermit,
    ) -> Result<CleanupCommit, &'static str> {
        self.store.gate_same_store(&permit.store)?;
        if let Some(outcome) = permit.completion {
            return Ok(outcome);
        }
        let file = permit.file.as_ref().ok_or("privacy_gate_consumed_permit")?;
        let outcome = self.store.gate_finish_private(file, &permit.epoch)?;
        permit.completion = Some(outcome);
        permit.file.take();
        Ok(outcome)
    }

    pub fn observation_allowed(&self) -> bool {
        self.public_lease().is_ok()
    }

    /// Caller must retain its public lease through this operation. A persistence
    /// error is not reconciliation and must prevent enabling private execution.
    pub fn mark_uncertain(&self) -> Result<(), &'static str> {
        self.store.gate_mark_uncertain()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
        process::{Command, Stdio},
    };

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> (Fixture, PathBuf) {
        let root = std::env::temp_dir().join(format!("ab-privacy-gate-{}", uuid::Uuid::new_v4()));
        (Fixture(root.clone()), root)
    }

    #[test]
    fn authority_key_is_path_independent_and_normalized() {
        assert_eq!(
            endpoint_key("ws://LOCALHOST:80/devtools/a").unwrap(),
            endpoint_key("ws://localhost/devtools/b?q=x").unwrap()
        );
        assert_ne!(
            endpoint_key("ws://localhost:81/a").unwrap(),
            endpoint_key("ws://localhost/a").unwrap()
        );
        assert!(endpoint_key("http://localhost/a").is_err());
        assert!(endpoint_key("ws://user:pass@localhost/a").is_err());
    }

    #[test]
    fn independent_readers_block_admission_and_drop_does_not_unlock_private() {
        fn send<T: Send>() {}
        send::<PublicLease>();
        send::<PrivatePermit>();
        let (_temp, root) = fixture();
        let first = PrivacyGate::open(&root).unwrap();
        let a = first.public_lease().unwrap();
        let second = PrivacyGate::open(&root).unwrap();
        let b = second.public_lease().unwrap();
        assert!(first.begin_private().is_err());
        drop(a);
        assert!(first.begin_private().is_err());
        drop(b);
        let private = first.begin_private().unwrap();
        assert!(!second.observation_allowed());
        drop(private);
        assert!(!first.observation_allowed());
        assert!(!PrivacyGate::open(&root).unwrap().observation_allowed());
    }

    #[test]
    fn uncertainty_survives_restart_but_allows_public_recovery() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let lease = gate.public_lease().unwrap();
        gate.mark_uncertain().unwrap();
        gate.mark_uncertain().unwrap();
        drop(lease);
        drop(gate);
        let gate = PrivacyGate::open(&root).unwrap();
        assert!(gate.observation_allowed());
        assert!(gate.begin_private().is_err());
    }

    #[test]
    fn unsafe_files_and_interrupted_marker_fail_closed() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let marker = root.join("privacy.lock");
        fs::write(&marker, b"").unwrap();
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!gate.observation_allowed());
        fs::remove_file(&marker).unwrap();
        symlink("store.key", &marker).unwrap();
        assert!(!gate.observation_allowed());
        assert!(gate.begin_private().is_err());
    }

    #[test]
    fn readable_owned_home_is_accepted_without_permission_repair() {
        let (_temp, root) = fixture();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        SecretStore::ensure_gate_base(&root).unwrap();
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(PrivacyGate::open(&root).is_err());
        let child = root.join("private-gates");
        SecretStore::ensure_private_directory(&child).unwrap();
        assert_eq!(
            fs::metadata(&child).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(SecretStore::ensure_gate_base(&root).is_err());
    }

    #[test]
    fn changed_or_missing_key_never_reopens_a_gate() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        fs::write(root.join("store.key"), [0; 32]).unwrap();
        assert!(!gate.observation_allowed());
        assert!(gate.begin_private().is_err());
        fs::remove_file(root.join("store.key")).unwrap();
        fs::write(root.join("privacy.lock"), b"private-v1").unwrap();
        assert!(PrivacyGate::open(&root).is_err());
        assert!(!root.join("store.key").exists());
    }

    #[test]
    fn subprocess_helper() {
        let Some(root) = std::env::var_os("AB_PRIVACY_GATE_TEST_ROOT") else {
            return;
        };
        let gate = PrivacyGate::open(Path::new(&root)).unwrap();
        if std::env::var_os("AB_PRIVACY_GATE_PENDING").is_some() {
            let _lease = gate.command_lease().unwrap();
            std::process::exit(39);
        }
        if std::env::var_os("AB_PRIVACY_GATE_EXPECT_BUSY").is_some() {
            assert!(gate.begin_private().is_err());
            std::process::exit(38);
        }
        let _permit = gate.begin_private().unwrap();
        std::process::exit(37);
    }

    #[test]
    fn subprocess_reader_barrier_and_private_death() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let lease = gate.public_lease().unwrap();
        let child = |busy| {
            let mut cmd = Command::new(std::env::current_exe().unwrap());
            cmd.args(["--exact", "native::privacy_gate::tests::subprocess_helper"])
                .env("AB_PRIVACY_GATE_TEST_ROOT", &root)
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if busy {
                cmd.env("AB_PRIVACY_GATE_EXPECT_BUSY", "1");
            }
            cmd.status().unwrap().code()
        };
        assert_eq!(child(true), Some(38));
        drop(lease);
        assert_eq!(child(false), Some(37));
        assert!(!gate.observation_allowed());
        assert!(!PrivacyGate::open(&root).unwrap().observation_allowed());
    }

    #[test]
    fn confirmed_commands_clear_only_their_own_receipt() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let mut first = gate.command_lease().unwrap();
        let second = gate.command_lease().unwrap();
        first.complete().unwrap();
        first.complete().unwrap();
        drop(first);
        drop(second);
        assert!(gate.observation_allowed());
        assert!(gate.begin_private().is_err());
        assert!(PrivacyGate::open(&root).unwrap().begin_private().is_err());
    }

    #[test]
    fn confirmed_command_permits_admission_after_lease_drop() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let mut lease = gate.command_lease().unwrap();
        lease.complete().unwrap();
        assert!(gate.begin_private().is_err());
        drop(lease);
        assert!(gate.begin_private().is_ok());
    }

    #[test]
    fn process_death_before_response_leaves_pending_receipt() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "native::privacy_gate::tests::subprocess_helper"])
            .env("AB_PRIVACY_GATE_TEST_ROOT", &root)
            .env("AB_PRIVACY_GATE_PENDING", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(39));
        assert!(gate.observation_allowed());
        assert!(gate.begin_private().is_err());
        assert!(PrivacyGate::open(&root).unwrap().begin_private().is_err());
    }

    #[test]
    fn private_permits_are_bound_and_recovery_requires_existing_marker() {
        let (_first_temp, first_root) = fixture();
        let (_second_temp, second_root) = fixture();
        let first = PrivacyGate::open(&first_root).unwrap();
        let reopened_first = PrivacyGate::open(&first_root).unwrap();
        let second = PrivacyGate::open(&second_root).unwrap();
        assert!(first.resume_private().is_err());
        let permit = first.begin_private().unwrap();
        assert!(permit.authorizes(&first));
        assert!(permit.authorizes(&reopened_first));
        assert!(!permit.authorizes(&second));
        assert!(first.resume_private().is_err());
        drop(permit);
        drop(first);
        drop(reopened_first);
        let first = PrivacyGate::open(&first_root).unwrap();
        let recovery = first.resume_private().unwrap();
        assert!(recovery.authorizes(&first));
        assert!(!first.observation_allowed());
        drop(recovery);
        assert!(!first.observation_allowed());
    }

    #[test]
    fn recovery_retains_uncertainty_and_rejects_unsafe_marker() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        drop(gate.begin_private().unwrap());
        // Synthetic interrupted-public evidence, never discarded by recovery.
        gate.mark_uncertain().unwrap();
        drop(gate.resume_private().unwrap());
        assert!(root.join("uncertain.lock").exists());
        fs::set_permissions(root.join("privacy.lock"), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(gate.resume_private().is_err());
        assert!(!gate.observation_allowed());
    }

    #[test]
    fn trusted_finish_is_bound_idempotent_and_allows_multiple_epochs() {
        let (_temp, root) = fixture();
        let (_other_temp, other_root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let other = PrivacyGate::open(&other_root).unwrap();
        let mut permit = gate.begin_private().unwrap();
        assert!(other.finish_private(&mut permit).is_err());
        assert!(permit.authorizes(&gate));
        assert_eq!(
            gate.finish_private(&mut permit).unwrap(),
            CleanupCommit::Durable
        );
        assert!(!permit.authorizes(&gate));
        assert!(gate.observation_allowed());
        assert_eq!(
            gate.finish_private(&mut permit).unwrap(),
            CleanupCommit::Durable
        );
        assert!(other.finish_private(&mut permit).is_err());
        let mut second = gate.begin_private().unwrap();
        assert!(!gate.observation_allowed());
        assert!(!permit.authorizes(&gate));
        // Replaying the completed permit must not clear the next private epoch.
        gate.finish_private(&mut permit).unwrap();
        assert!(!gate.observation_allowed());
        gate.finish_private(&mut second).unwrap();
        assert!(PrivacyGate::open(&root).unwrap().observation_allowed());
        assert!(root.join("privacy.lock").exists());
    }

    #[test]
    fn legacy_marker_migrates_only_through_recovery_and_clean_receipt_is_authenticated() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        fs::write(root.join("privacy.lock"), b"").unwrap();
        fs::set_permissions(root.join("privacy.lock"), fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!gate.observation_allowed());
        let mut permit = gate.resume_private().unwrap();
        gate.finish_private(&mut permit).unwrap();
        assert!(gate.observation_allowed());
        let state = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.to_string_lossy().ends_with(".gate-state"))
            .unwrap();
        let mut bytes = fs::read(&state).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(state, bytes).unwrap();
        assert!(!gate.observation_allowed());
        assert!(gate.begin_private().is_err());
        assert!(gate.resume_private().is_err());
    }

    #[test]
    fn failed_cleanup_commit_does_not_release_permit_or_marker() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let mut permit = gate.begin_private().unwrap();
        fs::set_permissions(root.join("privacy.lock"), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(gate.finish_private(&mut permit).is_err());
        assert!(permit.file.is_some());
        assert!(permit.completion.is_none());
        drop(permit);
        assert!(!gate.observation_allowed());
        fs::set_permissions(root.join("privacy.lock"), fs::Permissions::from_mode(0o600)).unwrap();
        let mut permit = gate.resume_private().unwrap();
        gate.finish_private(&mut permit).unwrap();
        assert!(gate.observation_allowed());
    }

    #[test]
    fn old_observers_are_revoked_after_unobserved_private_interval() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let old = gate.public_observer().unwrap();
        let old_clone = old.clone();
        assert!(old.public_lease().is_ok());
        let mut permit = gate.begin_private().unwrap();
        // Neither observer reads during the private interval. Epoch comparison
        // must still deny it once observation becomes available again.
        gate.finish_private(&mut permit).unwrap();
        assert!(gate.observation_allowed());
        assert!(old.public_lease().is_err());
        assert!(old_clone.public_lease().is_err());
        let current = gate.public_observer().unwrap();
        assert!(current.public_lease().is_ok());
        let mut second = gate.begin_private().unwrap();
        gate.finish_private(&mut second).unwrap();
        assert!(current.public_lease().is_err());
        assert!(old.public_lease().is_err());
        assert!(gate.public_observer().unwrap().public_lease().is_ok());
    }

    #[test]
    fn observer_leases_exclude_private_admission_and_private_observers_never_open() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let public = gate.public_observer().unwrap();
        let private = gate.private_observer();
        assert!(private.public_lease().is_err());
        let lease = public.public_lease().unwrap();
        assert!(gate.begin_private().is_err());
        drop(lease);
        let mut permit = gate.begin_private().unwrap();
        assert!(gate.public_observer().is_err());
        assert!(public.public_lease().is_err());
        assert!(private.public_lease().is_err());
        gate.finish_private(&mut permit).unwrap();
        assert!(private.public_lease().is_err());
        assert!(private.clone().public_lease().is_err());
        assert!(gate.public_observer().unwrap().public_lease().is_ok());
    }

    #[test]
    fn scoped_admission_survives_restart_without_rebinding() {
        let (_temp, root) = fixture();
        let digest = "a".repeat(64);
        let gate = PrivacyGate::open(&root).unwrap();
        let permit = gate.begin_private_scoped("target-123", &digest).unwrap();
        assert_eq!(permit.target_id(), Some("target-123"));
        assert_eq!(permit.identity_digest(), Some(digest.as_str()));
        drop(permit);
        drop(gate);
        let gate = PrivacyGate::open(&root).unwrap();
        assert!(gate
            .begin_private_scoped("different-target", &"b".repeat(64))
            .is_err());
        let mut resumed = gate.resume_private().unwrap();
        assert_eq!(resumed.target_id(), Some("target-123"));
        assert_eq!(resumed.identity_digest(), Some(digest.as_str()));
        gate.finish_private(&mut resumed).unwrap();
        assert_eq!(resumed.target_id(), Some("target-123"));
        assert_eq!(resumed.identity_digest(), Some(digest.as_str()));
        let next = gate
            .begin_private_scoped("next-target", &"b".repeat(64))
            .unwrap();
        assert_eq!(next.target_id(), Some("next-target"));
        assert!(!resumed.authorizes(&gate));
    }

    #[test]
    fn invalid_scope_never_creates_private_admission() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        for target in ["", "a b", "../escape", &"a".repeat(129)] {
            assert!(gate.begin_private_scoped(target, &"a".repeat(64)).is_err());
        }
        for digest in ["", "short", &"g".repeat(64), &"a".repeat(65)] {
            assert!(gate.begin_private_scoped("target", digest).is_err());
        }
        assert!(!root.join("privacy.lock").exists());
        assert!(gate.observation_allowed());
    }

    #[test]
    fn scoped_state_tampering_prevents_recovery_and_unscoped_permits_stay_unscoped() {
        let (_temp, root) = fixture();
        let gate = PrivacyGate::open(&root).unwrap();
        let mut unscoped = gate.begin_private().unwrap();
        assert_eq!(unscoped.target_id(), None);
        assert_eq!(unscoped.identity_digest(), None);
        gate.finish_private(&mut unscoped).unwrap();
        drop(
            gate.begin_private_scoped("target", &"a".repeat(64))
                .unwrap(),
        );
        let state = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.to_string_lossy().ends_with(".gate-state"))
            .unwrap();
        let mut bytes = fs::read(&state).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(state, bytes).unwrap();
        assert!(gate.resume_private().is_err());
        assert!(!gate.observation_allowed());
    }
}
