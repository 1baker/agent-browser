//! Same-user private staging for the credential broker. No plaintext, paths, or
//! operating-system error details are returned through diagnostic surfaces.
//!
//! This is not protection against the account owner, root, memory inspection,
//! filesystem rollback, or copying the complete store together with its key.
//! Callers must keep returned plaintext out of logs and ordinary responses.
use std::path::Path;

pub type StoreResult<T> = Result<T, &'static str>;
pub const MAX_SECRET_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CleanupCommit {
    Durable,
    ReconciledDurabilityUnknown,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivateScope {
    target_id: String,
    identity_digest: String,
    #[serde(default)]
    journey: Vec<String>,
    #[serde(default)]
    stage: usize,
}

impl PrivateScope {
    fn validate(&self) -> StoreResult<()> {
        if self.journey.len() > 16
            || (self.journey.is_empty() && self.stage != 0)
            || (!self.journey.is_empty()
                && (self.journey.get(self.stage) != Some(&self.identity_digest)
                    || self.journey.iter().any(|digest| {
                        digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit())
                    })))
        {
            return Err("privacy_gate_invalid_scope");
        }
        if self.target_id.is_empty()
            || self.target_id.len() > 128
            || !self
                .target_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            || self.identity_digest.len() != 64
            || !self
                .identity_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("privacy_gate_invalid_scope");
        }
        Ok(())
    }
    pub(crate) fn target_id(&self) -> &str {
        &self.target_id
    }
    pub(crate) fn identity_digest(&self) -> &str {
        &self.identity_digest
    }
    pub(crate) fn journey_stage(&self) -> Option<usize> {
        (!self.journey.is_empty()).then_some(self.stage)
    }
}
const INVALID: &str = "private_store_invalid";
const UNAVAILABLE: &str = "private_store_unavailable";

#[cfg(unix)]
mod platform {
    use super::*;
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes256Gcm, Nonce,
    };
    use std::{
        ffi::{CStr, CString},
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        },
        path::Component,
        sync::{Mutex, MutexGuard},
    };

    const MAX_FILES: usize = 4096;
    const KEY: &str = "store.key";
    const MAGIC: &[u8] = b"ABPS1";
    const OVERHEAD: usize = 5 + 12 + 16;
    const STATE_REF: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const STATE_ROLE: &str = "gate-state";

    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GateState {
        epoch: String,
        clean: bool,
        device: u64,
        inode: u64,
        anchor_sha256: String,
        #[serde(default)]
        scope: Option<PrivateScope>,
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn directory_entry(dir: *mut libc::DIR) -> StoreResult<*mut libc::dirent> {
        // These are the Unix targets supported by the installed broker. Other
        // Unix targets fail unavailable until their errno ABI is verified.
        #[cfg(target_os = "linux")]
        let error = unsafe { libc::__errno_location() };
        #[cfg(target_os = "macos")]
        let error = unsafe { libc::__error() };
        // SAFETY: errno is thread-local; dir is live and used under the store lock.
        unsafe {
            *error = 0;
            let entry = libc::readdir(dir);
            if entry.is_null() && *error != 0 {
                Err(UNAVAILABLE)
            } else {
                Ok(entry)
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn directory_entry(_: *mut libc::DIR) -> StoreResult<*mut libc::dirent> {
        Err(UNAVAILABLE)
    }

    /// Intentionally does not implement Debug or Serialize.
    pub struct SecretStore {
        directory: File,
        key: [u8; 32],
        mutex: Mutex<()>,
    }

    struct Locked<'a> {
        directory: &'a File,
        _guard: MutexGuard<'a, ()>,
    }

    impl Drop for Locked<'_> {
        fn drop(&mut self) {
            // SAFETY: the borrowed descriptor remains open for this guard.
            unsafe { libc::flock(self.directory.as_raw_fd(), libc::LOCK_UN) };
        }
    }

    impl Drop for SecretStore {
        fn drop(&mut self) {
            for byte in &mut self.key {
                // Best effort only: AES internals and caller buffers are not locked memory.
                unsafe { std::ptr::write_volatile(byte, 0) };
            }
        }
    }

    fn name(value: &str) -> StoreResult<CString> {
        CString::new(value).map_err(|_| INVALID)
    }

    fn owned_file(fd: libc::c_int) -> StoreResult<File> {
        if fd < 0 {
            return Err(UNAVAILABLE);
        }
        // SAFETY: successful open/openat returned a newly owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    fn valid_directory(file: &File, private: bool) -> StoreResult<()> {
        let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
        // SAFETY: geteuid has no arguments or memory safety preconditions.
        let uid = unsafe { libc::geteuid() };
        if !metadata.is_dir() || metadata.nlink() == 0 {
            return Err(INVALID);
        }
        if private {
            if metadata.uid() != uid || metadata.mode() & 0o7777 != 0o700 {
                return Err(INVALID);
            }
        } else if (metadata.uid() != uid && metadata.uid() != 0)
            || (metadata.mode() & 0o022 != 0
                && !(metadata.uid() == 0 && metadata.mode() & 0o1000 != 0))
        {
            return Err(INVALID);
        }
        Ok(())
    }

    fn directory(root: &Path) -> StoreResult<File> {
        directory_with_mode(root, true)
    }

    fn directory_with_mode(root: &Path, private: bool) -> StoreResult<File> {
        if !root.is_absolute() || root.as_os_str().as_bytes().len() > 4096 {
            return Err(INVALID);
        }
        let components: Vec<_> = root.components().collect();
        if components.len() < 2
            || components.len() > 128
            || components[1..]
                .iter()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(INVALID);
        }
        let slash = name("/")?;
        // SAFETY: all C strings are terminated and remain live during syscalls.
        let mut current = owned_file(unsafe {
            libc::open(
                slash.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        })?;
        valid_directory(&current, false)?;
        for (index, component) in components[1..].iter().enumerate() {
            let part = CString::new(component.as_os_str().as_bytes()).map_err(|_| INVALID)?;
            let final_component = index == components.len() - 2;
            if final_component {
                let result = unsafe { libc::mkdirat(current.as_raw_fd(), part.as_ptr(), 0o700) };
                if result != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(UNAVAILABLE);
                }
                if result == 0 {
                    current.sync_all().map_err(|_| UNAVAILABLE)?;
                }
            }
            let next = owned_file(unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    part.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            })?;
            valid_directory(&next, final_component && private)?;
            current = next;
        }
        Ok(current)
    }

    impl SecretStore {
        /// Existing runtime homes may be readable (0755), but must be owned by
        /// this user and never writable by another user. Never chmod or repair.
        pub(crate) fn ensure_gate_base(root: &Path) -> StoreResult<()> {
            let directory = directory_with_mode(root, false)?;
            let metadata = directory.metadata().map_err(|_| UNAVAILABLE)?;
            if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
                return Err(INVALID);
            }
            Ok(())
        }
        /// Initialize only the final private directory component, with anchored
        /// ancestor validation. Used for the dedicated gate hierarchy.
        pub(crate) fn ensure_private_directory(root: &Path) -> StoreResult<()> {
            directory(root).map(|_| ())
        }

        /// Existing gates can be opened while another process holds a public
        /// lease. New key initialization still requires the exclusive store lock.
        pub(crate) fn open_gate(root: &Path) -> StoreResult<Self> {
            if !cfg!(any(target_os = "linux", target_os = "macos")) {
                return Err(UNAVAILABLE);
            }
            let mut store = Self {
                directory: directory(root)?,
                key: [0; 32],
                mutex: Mutex::new(()),
            };
            let lease = store.independent_lock(false)?;
            if let Some(bytes) = store.read_optional(KEY, 32)? {
                store.key = bytes.try_into().map_err(|_| INVALID)?;
                drop(lease);
                return Ok(store);
            }
            drop(lease);
            Self::open(root)
        }

        /// A fresh open file description is essential: dup/try_clone would
        /// share flock state and allow one guard to release another's lock.
        fn independent_lock(&self, exclusive: bool) -> StoreResult<File> {
            valid_directory(&self.directory, true)?;
            let dot = name(".")?;
            let file = owned_file(unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    dot.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            })?;
            valid_directory(&file, true)?;
            let mode = if exclusive {
                libc::LOCK_EX
            } else {
                libc::LOCK_SH
            };
            if unsafe { libc::flock(file.as_raw_fd(), mode | libc::LOCK_NB) } != 0 {
                return Err("private_store_busy");
            }
            Ok(file)
        }

        fn gate_key(&self) -> StoreResult<()> {
            let current = self.read_optional(KEY, 32)?.ok_or(INVALID)?;
            if current.as_slice() != self.key {
                return Err(INVALID);
            }
            Ok(())
        }

        pub(crate) fn gate_public_lease(&self) -> StoreResult<File> {
            let lease = self.independent_lock(false)?;
            self.gate_key()?;
            if self.read_optional("privacy.lock", 32)?.is_some()
                && !self.gate_state()?.is_some_and(|state| state.clean)
            {
                return Err("privacy_gate_locked");
            }
            // A state record without its immutable anchor is corrupt, not clean.
            self.gate_state()?;
            Ok(lease)
        }

        /// Caller must hold an owned public lease while capturing or comparing
        /// this epoch, so a private transition cannot race the observation.
        pub(crate) fn gate_public_epoch(&self) -> StoreResult<Option<String>> {
            self.gate_key()?;
            match self.gate_state()? {
                Some(state) if state.clean => Ok(Some(state.epoch)),
                Some(_) => Err("privacy_gate_locked"),
                None if self.read_optional("privacy.lock", 32)?.is_some() => {
                    Err("privacy_gate_locked")
                }
                None => Ok(None),
            }
        }

        pub(crate) fn gate_begin_private(&self) -> StoreResult<(File, String)> {
            self.gate_begin_private_with_scope(None)
        }

        pub(crate) fn gate_begin_private_scoped(
            &self,
            target_id: &str,
            identity_digest: &str,
        ) -> StoreResult<(File, String)> {
            let scope = PrivateScope {
                target_id: target_id.to_owned(),
                identity_digest: identity_digest.to_owned(),
                journey: Vec::new(),
                stage: 0,
            };
            scope.validate()?;
            self.gate_begin_private_with_scope(Some(scope))
        }

        pub(crate) fn gate_begin_private_journey(
            &self,
            target_id: &str,
            digests: &[String],
        ) -> StoreResult<(File, String)> {
            let scope = PrivateScope {
                target_id: target_id.to_owned(),
                identity_digest: digests.first().ok_or(INVALID)?.clone(),
                journey: digests.to_vec(),
                stage: 0,
            };
            scope.validate()?;
            self.gate_begin_private_with_scope(Some(scope))
        }

        pub(crate) fn gate_advance_private(
            &self,
            permit: &File,
            epoch: &str,
            previous: &str,
            expected_stage: usize,
            target: &str,
            next: &str,
        ) -> StoreResult<PrivateScope> {
            self.gate_permit_authorizes(permit, epoch)?;
            let mut state = self.gate_state()?.ok_or(INVALID)?;
            let scope = state.scope.as_mut().ok_or(INVALID)?;
            scope.validate()?;
            if scope.target_id != target
                || scope.identity_digest != previous
                || scope.journey_stage() != Some(expected_stage)
                || scope.journey.get(scope.stage + 1).map(String::as_str) != Some(next)
            {
                return Err("private_journey_transition_rejected");
            }
            scope.stage += 1;
            scope.identity_digest = next.to_owned();
            let committed = scope.clone();
            if self.write_gate_state(&state)? != CleanupCommit::Durable {
                return Err("privacy_gate_commit_unknown");
            }
            Ok(committed)
        }

        fn gate_begin_private_with_scope(
            &self,
            scope: Option<PrivateScope>,
        ) -> StoreResult<(File, String)> {
            let lease = self.independent_lock(true)?;
            self.gate_key()?;
            let anchor = self.read_optional("privacy.lock", 32)?;
            let state = self.gate_state()?;
            if anchor.is_some() && !state.as_ref().is_some_and(|state| state.clean) {
                return Err("privacy_gate_locked");
            }
            if self.read_optional("uncertain.lock", 32)?.is_some() {
                return Err("privacy_gate_uncertain");
            }
            // A clean gate contains only its key. Pending command receipts or
            // any unrecognized entry deny admission, including partial writes.
            if self.file_count()? != if state.is_some() { 3 } else { 1 } {
                return Err("privacy_gate_pending");
            }
            if anchor.is_none() {
                self.create("privacy.lock", b"private-v1")?;
            }
            let state = self.new_gate_state(scope)?;
            let epoch = state.epoch.clone();
            if self.write_gate_state(&state)? != CleanupCommit::Durable {
                return Err("privacy_gate_commit_unknown");
            }
            Ok((lease, epoch))
        }

        /// Recovery reattaches only to an existing private interval. Pending or
        /// uncertain receipts are retained, not interpreted as reconciled.
        pub(crate) fn gate_resume_private(&self) -> StoreResult<(File, String)> {
            let lease = self.independent_lock(true)?;
            self.gate_key()?;
            if self.read_optional("privacy.lock", 32)?.is_none() {
                return Err("privacy_gate_not_locked");
            }
            let state = match self.gate_state()? {
                Some(state) if state.clean => return Err("privacy_gate_already_clean"),
                Some(state) => state,
                None => {
                    let state = self.new_gate_state(None)?;
                    if self.write_gate_state(&state)? != CleanupCommit::Durable {
                        return Err("privacy_gate_commit_unknown");
                    }
                    state
                }
            };
            Ok((lease, state.epoch))
        }

        fn new_gate_state(&self, scope: Option<PrivateScope>) -> StoreResult<GateState> {
            use sha2::{Digest, Sha256};
            let mut random = [0; 32];
            getrandom::getrandom(&mut random).map_err(|_| UNAVAILABLE)?;
            let metadata = self.directory.metadata().map_err(|_| UNAVAILABLE)?;
            let anchor = self.read_optional("privacy.lock", 32)?.ok_or(INVALID)?;
            Ok(GateState {
                epoch: hex::encode(random),
                clean: false,
                device: metadata.dev(),
                inode: metadata.ino(),
                anchor_sha256: hex::encode(Sha256::digest(anchor)),
                scope,
            })
        }

        fn gate_state(&self) -> StoreResult<Option<GateState>> {
            use sha2::{Digest, Sha256};
            let filename = format!("{STATE_REF}.{STATE_ROLE}");
            if self.read_optional(&filename, 2048)?.is_none() {
                return Ok(None);
            }
            let bytes = self.decrypt(STATE_REF, STATE_ROLE)?;
            let state: GateState = serde_json::from_slice(&bytes).map_err(|_| INVALID)?;
            if let Some(scope) = &state.scope {
                scope.validate()?;
            }
            Self::reference(&state.epoch)?;
            let metadata = self.directory.metadata().map_err(|_| UNAVAILABLE)?;
            let anchor = self.read_optional("privacy.lock", 32)?.ok_or(INVALID)?;
            if state.device != metadata.dev()
                || state.inode != metadata.ino()
                || state.anchor_sha256 != hex::encode(Sha256::digest(anchor))
            {
                return Err(INVALID);
            }
            Ok(Some(state))
        }

        /// Readback is used only while the returned private descriptor holds EX.
        pub(crate) fn gate_private_scope(&self, epoch: &str) -> StoreResult<Option<PrivateScope>> {
            self.gate_key()?;
            let state = self.gate_state()?.ok_or(INVALID)?;
            if state.epoch != epoch || state.clean {
                return Err("privacy_gate_wrong_epoch");
            }
            Ok(state.scope)
        }

        /// Atomic authenticated epoch transition. A successful rename after
        /// verified cleanup is safe even when durability remains uncertain.
        fn write_gate_state(&self, state: &GateState) -> StoreResult<CleanupCommit> {
            let bytes = serde_json::to_vec(state).map_err(|_| INVALID)?;
            let envelope = self.encrypt(STATE_REF, STATE_ROLE, &bytes)?;
            let mut random = [0; 32];
            getrandom::getrandom(&mut random).map_err(|_| UNAVAILABLE)?;
            let temporary = format!("{}.state-tmp", hex::encode(random));
            self.create(&temporary, &envelope)?;
            let temporary = name(&temporary)?;
            let target = name(&format!("{STATE_REF}.{STATE_ROLE}"))?;
            if unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    temporary.as_ptr(),
                    self.directory.as_raw_fd(),
                    target.as_ptr(),
                )
            } != 0
            {
                return Err(UNAVAILABLE);
            }
            Ok(if self.directory.sync_all().is_ok() {
                CleanupCommit::Durable
            } else {
                CleanupCommit::ReconciledDurabilityUnknown
            })
        }

        /// Only the trusted coordinator may request this transition, after its
        /// actual cleanup proof. Never accept a caller-supplied 'clean' boolean.
        pub(crate) fn gate_finish_private(
            &self,
            permit: &File,
            epoch: &str,
        ) -> StoreResult<CleanupCommit> {
            self.gate_permit_matches(permit)?;
            let mut state = self.gate_state()?.ok_or(INVALID)?;
            if state.epoch != epoch {
                return Err("privacy_gate_wrong_epoch");
            }
            if state.clean {
                return Err("privacy_gate_already_clean");
            }
            state.clean = true;
            self.write_gate_state(&state)
        }

        pub(crate) fn gate_permit_matches(&self, permit: &File) -> StoreResult<()> {
            valid_directory(&self.directory, true)?;
            valid_directory(permit, true)?;
            self.gate_key()?;
            let expected = self.directory.metadata().map_err(|_| UNAVAILABLE)?;
            let actual = permit.metadata().map_err(|_| UNAVAILABLE)?;
            if expected.dev() != actual.dev() || expected.ino() != actual.ino() {
                return Err("privacy_gate_wrong_permit");
            }
            Ok(())
        }

        pub(crate) fn gate_same_store(&self, other: &Self) -> StoreResult<()> {
            self.gate_permit_matches(&other.directory)
        }

        pub(crate) fn gate_permit_authorizes(&self, permit: &File, epoch: &str) -> StoreResult<()> {
            self.gate_permit_matches(permit)?;
            let state = self.gate_state()?.ok_or(INVALID)?;
            if state.clean || state.epoch != epoch {
                return Err("privacy_gate_wrong_epoch");
            }
            Ok(())
        }

        pub(crate) fn gate_command_lease(&self) -> StoreResult<(File, String)> {
            let lease = self.gate_public_lease()?;
            let mut random = [0; 32];
            getrandom::getrandom(&mut random).map_err(|_| UNAVAILABLE)?;
            let reference = hex::encode(random);
            self.create(&format!("{reference}.pending"), b"pending-v1")?;
            Ok((lease, reference))
        }

        /// Only called by an owned command lease after a matching response.
        /// The lease's shared flock excludes private admission until completion.
        pub(crate) fn gate_complete_command(&self, reference: &str) -> StoreResult<()> {
            valid_directory(&self.directory, true)?;
            self.gate_key()?;
            Self::reference(reference)?;
            let filename = format!("{reference}.pending");
            if self.read_optional(&filename, 32)?.as_deref() != Some(b"pending-v1") {
                return Err(INVALID);
            }
            let filename = name(&filename)?;
            if unsafe { libc::unlinkat(self.directory.as_raw_fd(), filename.as_ptr(), 0) } != 0 {
                return Err(UNAVAILABLE);
            }
            self.directory.sync_all().map_err(|_| UNAVAILABLE)
        }

        /// May run with other shared leases held. The create-new file itself is
        /// the fail-closed marker, even if its write/fsync is interrupted.
        pub(crate) fn gate_mark_uncertain(&self) -> StoreResult<()> {
            let _lease = self.independent_lock(false)?;
            self.gate_key()?;
            match self.create("uncertain.lock", b"uncertain-v1") {
                Ok(()) => Ok(()),
                Err("private_store_already_exists") => {
                    self.read_optional("uncertain.lock", 32)?.ok_or(INVALID)?;
                    self.directory.sync_all().map_err(|_| UNAVAILABLE)
                }
                Err(error) => Err(error),
            }
        }

        /// Opens an absolute private root, creating only its final component.
        /// Missing keys in a nonempty store and malformed existing keys are never
        /// replaced. An empty directory is treated as a new store.
        pub fn open(root: &Path) -> StoreResult<Self> {
            if !cfg!(any(target_os = "linux", target_os = "macos")) {
                return Err(UNAVAILABLE);
            }
            let mut store = Self {
                directory: directory(root)?,
                key: [0; 32],
                mutex: Mutex::new(()),
            };
            let key = {
                let _lock = store.lock()?;
                match store.read_optional(KEY, 32)? {
                    Some(bytes) => bytes.try_into().map_err(|_| INVALID)?,
                    None => {
                        if store.file_count()? != 0 {
                            return Err(INVALID);
                        }
                        let mut key = [0; 32];
                        getrandom::getrandom(&mut key).map_err(|_| UNAVAILABLE)?;
                        store.create(KEY, &key)?;
                        key
                    }
                }
            };
            store.key = key;
            Ok(store)
        }

        fn lock(&self) -> StoreResult<Locked<'_>> {
            let guard = self.mutex.lock().map_err(|_| UNAVAILABLE)?;
            valid_directory(&self.directory, true)?;
            // Nonblocking: another process cannot hang a broker command indefinitely.
            if unsafe { libc::flock(self.directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }
                != 0
            {
                return Err("private_store_busy");
            }
            Ok(Locked {
                directory: &self.directory,
                _guard: guard,
            })
        }

        fn checked_lock(&self) -> StoreResult<Locked<'_>> {
            let lock = self.lock()?;
            let current = self.read_optional(KEY, 32)?.ok_or(INVALID)?;
            if current.as_slice() != self.key {
                return Err(INVALID);
            }
            Ok(lock)
        }

        fn file_count(&self) -> StoreResult<usize> {
            let dot = name(".")?;
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    dot.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
            let file = owned_file(fd)?;
            // fdopendir owns a duplicate, leaving the original RAII-owned fd untouched.
            let duplicate = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
            if duplicate < 0 {
                return Err(UNAVAILABLE);
            }
            let dir = unsafe { libc::fdopendir(duplicate) };
            if dir.is_null() {
                unsafe { libc::close(duplicate) };
                return Err(UNAVAILABLE);
            }
            let mut count = 0;
            let mut read_failed = false;
            loop {
                let entry = match directory_entry(dir) {
                    Ok(entry) => entry,
                    Err(_) => {
                        read_failed = true;
                        break;
                    }
                };
                if entry.is_null() {
                    break;
                }
                let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if bytes != b"." && bytes != b".." {
                    count += 1;
                }
                if count > MAX_FILES {
                    break;
                }
            }
            unsafe { libc::closedir(dir) };
            if read_failed {
                return Err(UNAVAILABLE);
            }
            if count > MAX_FILES {
                return Err("private_store_full");
            }
            Ok(count)
        }

        fn validate_file(file: &File, max: usize) -> StoreResult<()> {
            let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
            if !metadata.is_file()
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.nlink() != 1
                || metadata.mode() & 0o7777 != 0o600
                || metadata.len() > max as u64
            {
                return Err(INVALID);
            }
            Ok(())
        }

        fn read_optional(&self, filename: &str, max: usize) -> StoreResult<Option<Vec<u8>>> {
            let filename = name(filename)?;
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    filename.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            };
            if fd < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            let mut file = owned_file(fd)?;
            Self::validate_file(&file, max)?;
            let mut bytes = Vec::new();
            (&mut file)
                .take(max as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| UNAVAILABLE)?;
            Self::validate_file(&file, max)?;
            if bytes.len() > max {
                return Err(INVALID);
            }
            Ok(Some(bytes))
        }

        /// Create-new, fsync file, fsync directory. Partial writes stay present and
        /// fail closed. Neither interrupted receipts nor key files are removed.
        fn create(&self, filename: &str, bytes: &[u8]) -> StoreResult<()> {
            if self.file_count()? >= MAX_FILES {
                return Err("private_store_full");
            }
            let filename = name(filename)?;
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    filename.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists
            {
                return Err("private_store_already_exists");
            }
            let mut file = owned_file(fd)?;
            Self::validate_file(&file, bytes.len())?;
            file.write_all(bytes).map_err(|_| UNAVAILABLE)?;
            file.sync_all().map_err(|_| UNAVAILABLE)?;
            self.directory.sync_all().map_err(|_| UNAVAILABLE)?;
            Ok(())
        }

        fn reference(reference: &str) -> StoreResult<()> {
            if reference.len() != 64
                || !reference
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(INVALID);
            }
            Ok(())
        }

        fn encrypt(&self, reference: &str, role: &str, bytes: &[u8]) -> StoreResult<Vec<u8>> {
            if bytes.len() > MAX_SECRET_BYTES {
                return Err(INVALID);
            }
            let mut nonce = [0; 12];
            getrandom::getrandom(&mut nonce).map_err(|_| UNAVAILABLE)?;
            let aad = format!("agent-browser-private-v1:{role}:{reference}");
            let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| INVALID)?;
            let encrypted = cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: bytes,
                        aad: aad.as_bytes(),
                    },
                )
                .map_err(|_| INVALID)?;
            let mut envelope = Vec::with_capacity(OVERHEAD + bytes.len());
            envelope.extend_from_slice(MAGIC);
            envelope.extend_from_slice(&nonce);
            envelope.extend_from_slice(&encrypted);
            Ok(envelope)
        }

        fn decrypt(&self, reference: &str, role: &str) -> StoreResult<Vec<u8>> {
            Self::reference(reference)?;
            let envelope = self
                .read_optional(&format!("{reference}.{role}"), MAX_SECRET_BYTES + OVERHEAD)?
                .ok_or(INVALID)?;
            if envelope.len() < OVERHEAD || !envelope.starts_with(MAGIC) {
                return Err(INVALID);
            }
            let aad = format!("agent-browser-private-v1:{role}:{reference}");
            Aes256Gcm::new_from_slice(&self.key)
                .map_err(|_| INVALID)?
                .decrypt(
                    Nonce::from_slice(&envelope[5..17]),
                    Payload {
                        msg: &envelope[17..],
                        aad: aad.as_bytes(),
                    },
                )
                .map_err(|_| INVALID)
        }

        pub fn stage(&self, payload: &[u8]) -> StoreResult<String> {
            let _lock = self.checked_lock()?;
            if self.file_count()? + 3 > MAX_FILES {
                return Err("private_store_full");
            }
            let mut random = [0; 32];
            getrandom::getrandom(&mut random).map_err(|_| UNAVAILABLE)?;
            let reference = hex::encode(random);
            let encrypted = self.encrypt(&reference, "payload", payload)?;
            self.create(&format!("{reference}.payload"), &encrypted)?;
            Ok(reference)
        }

        pub fn load(&self, reference: &str) -> StoreResult<Vec<u8>> {
            let _lock = self.checked_lock()?;
            self.decrypt(reference, "payload")
        }

        /// Must succeed before browser mutation. Any existing receipt is consumed,
        /// including empty/interrupted receipts. No retry/reset operation exists.
        pub fn admit(&self, reference: &str) -> StoreResult<()> {
            let _lock = self.checked_lock()?;
            self.decrypt(reference, "payload")?;
            self.create(&format!("{reference}.admitted"), b"admitted-v1")
        }

        fn reservation_fingerprint(&self, domain: &[u8], parts: &[&[u8]]) -> StoreResult<String> {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            // Derive a domain-specific key instead of concatenating key bytes
            // with low-entropy codes or publishing a brute-forceable plain hash.
            let mut derive =
                <Hmac<Sha256> as Mac>::new_from_slice(&self.key).map_err(|_| INVALID)?;
            derive.update(b"agent-browser-backup-reservation-key-v1");
            let derived = derive.finalize().into_bytes();
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&derived).map_err(|_| INVALID)?;
            mac.update(&(domain.len() as u64).to_be_bytes());
            mac.update(domain);
            for part in parts {
                mac.update(&(part.len() as u64).to_be_bytes());
                mac.update(part);
            }
            Ok(hex::encode(mac.finalize().into_bytes()))
        }

        /// Permanently reserve an authenticated staged operation's backup code
        /// before any browser dispatch. This is additional to, not a substitute
        /// for, `admit` and exact consent/identity/transport validation.
        ///
        /// The trusted caller must derive ALL arguments from its reviewed staged
        /// operation. Account scope must be a stable canonical provider/account
        /// identity, excluding mutable consent IDs and opaque staging references.
        /// Source scope must identify the exact reviewed message/code slot, not a
        /// whole channel; use the same canonical slot name across new consents.
        /// This store cannot verify those external identities or parse the payload.
        ///
        /// Account/code and account/source keys deliberately exclude consent and
        /// reference. A new consent or restaged opaque reference cannot reset them.
        /// Code fingerprinting ignores ASCII spaces/hyphens and letter case; this
        /// conservative normalization may over-reject but never alters browser input.
        /// Filenames and receipt bodies contain only opaque keyed fingerprints.
        ///
        /// Three create-new receipts are fsynced under the exclusive store lock.
        /// Partial failure remains consumed; no rollback, release, or automatic
        /// retry exists. A failed reservation must never dispatch to the browser.
        pub(crate) fn reserve_backup_code(
            &self,
            reference: &str,
            consent_sha256: &str,
            account_scope: &str,
            source_scope: &str,
            code: &[u8],
        ) -> StoreResult<()> {
            Self::reference(reference)?;
            Self::reference(consent_sha256)?;
            if account_scope.is_empty()
                || account_scope.len() > 512
                || source_scope.is_empty()
                || source_scope.len() > 2048
                || !account_scope
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte))
                || !source_scope
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte))
                || account_scope.trim() != account_scope
                || source_scope.trim() != source_scope
                || code.is_empty()
                || code.len() > 256
            {
                return Err(INVALID);
            }
            let mut normalized = Vec::with_capacity(code.len());
            for byte in code {
                if byte.is_ascii_alphanumeric() {
                    normalized.push(byte.to_ascii_uppercase());
                } else if !matches!(byte, b' ' | b'-') {
                    return Err(INVALID);
                }
            }
            if normalized.is_empty() {
                return Err(INVALID);
            }
            let _lock = self.checked_lock()?;
            self.decrypt(reference, "payload")?;
            let code_key = self.reservation_fingerprint(
                b"account-code",
                &[account_scope.as_bytes(), &normalized],
            )?;
            let source_key = self.reservation_fingerprint(
                b"account-source",
                &[account_scope.as_bytes(), source_scope.as_bytes()],
            )?;
            let reference_key =
                self.reservation_fingerprint(b"staged-reference", &[reference.as_bytes()])?;
            let receipt = self.reservation_fingerprint(
                b"consent-binding",
                &[
                    consent_sha256.as_bytes(),
                    reference.as_bytes(),
                    account_scope.as_bytes(),
                    source_scope.as_bytes(),
                    &normalized,
                ],
            )?;
            let reservations = [
                (
                    format!("{reference_key}.backup-reference"),
                    "private_backup_reference_reserved",
                ),
                (
                    format!("{source_key}.backup-source"),
                    "private_backup_source_reserved",
                ),
                (
                    format!("{code_key}.backup-code"),
                    "private_backup_code_reserved",
                ),
            ];
            for (filename, error) in &reservations {
                if self.read_optional(filename, 64)?.is_some() {
                    return Err(error);
                }
            }
            if self.file_count()? + reservations.len() > MAX_FILES {
                return Err("private_store_full");
            }
            for (filename, _) in reservations {
                self.create(&filename, receipt.as_bytes())?;
            }
            Ok(())
        }

        /// Consume the entire approved journey independently of its staged
        /// reference names. Restaging payloads cannot replay a consumed consent.
        /// The trusted coordinator validates semantics before this reservation.
        pub(crate) fn reserve_journey(
            &self,
            consent_sha256: &str,
            references: &[String],
        ) -> StoreResult<()> {
            Self::reference(consent_sha256)?;
            if references.len() != 4 {
                return Err(INVALID);
            }
            let _lock = self.checked_lock()?;
            for (index, reference) in references.iter().enumerate() {
                Self::reference(reference)?;
                if references[..index].contains(reference) {
                    return Err(INVALID);
                }
                self.decrypt(reference, "payload")?;
            }
            let key = self.reservation_fingerprint(
                b"private-journey-consent",
                &[consent_sha256.as_bytes()],
            )?;
            let filename = format!("{key}.journey-consent");
            if self.read_optional(&filename, 64)?.is_some() {
                return Err("private_journey_consent_consumed");
            }
            if self.file_count()? + 1 > MAX_FILES {
                return Err("private_store_full");
            }
            let fields = references
                .iter()
                .map(|value| value.as_bytes())
                .collect::<Vec<_>>();
            let receipt = self.reservation_fingerprint(b"private-journey-references", &fields)?;
            self.create(&filename, receipt.as_bytes())
        }

        /// Capacity or I/O failure can occur after admission. The caller MUST keep
        /// the sensitive browser lock on failure; a missing result is not permission
        /// to retry or expose the still-sensitive page. Stage does not reserve quota.
        pub fn store_result(&self, reference: &str, bytes: &[u8]) -> StoreResult<()> {
            let _lock = self.checked_lock()?;
            Self::reference(reference)?;
            if self
                .read_optional(&format!("{reference}.admitted"), 32)?
                .is_none()
            {
                return Err(INVALID);
            }
            let encrypted = self.encrypt(reference, "result", bytes)?;
            self.create(&format!("{reference}.result"), &encrypted)
        }

        pub fn load_result(&self, reference: &str) -> StoreResult<Vec<u8>> {
            let _lock = self.checked_lock()?;
            self.decrypt(reference, "result")
        }

        /// Fixed labels only. "admitted" includes an indeterminate interrupted run.
        pub fn status(&self, reference: &str) -> StoreResult<&'static str> {
            let _lock = self.checked_lock()?;
            self.decrypt(reference, "payload")?;
            if self
                .read_optional(&format!("{reference}.admitted"), 32)?
                .is_some()
            {
                if self
                    .read_optional(&format!("{reference}.result"), MAX_SECRET_BYTES + OVERHEAD)?
                    .is_some()
                {
                    self.decrypt(reference, "result")?;
                    return Ok("result_ready");
                }
                return Ok("admitted");
            }
            Ok("staged")
        }
    }
}

#[cfg(unix)]
pub type SecretStore = platform::SecretStore;

#[cfg(not(unix))]
pub struct SecretStore;

#[cfg(not(unix))]
impl SecretStore {
    pub(crate) fn gate_begin_private_journey(
        &self,
        _: &str,
        _: &[String],
    ) -> StoreResult<(std::fs::File, String)> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_advance_private(
        &self,
        _: &std::fs::File,
        _: &str,
        _: &str,
        _: usize,
        _: &str,
        _: &str,
    ) -> StoreResult<PrivateScope> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn ensure_gate_base(_: &Path) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn ensure_private_directory(_: &Path) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn open_gate(_: &Path) -> StoreResult<Self> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_public_lease(&self) -> StoreResult<std::fs::File> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_public_epoch(&self) -> StoreResult<Option<String>> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_begin_private(&self) -> StoreResult<(std::fs::File, String)> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_begin_private_scoped(
        &self,
        _: &str,
        _: &str,
    ) -> StoreResult<(std::fs::File, String)> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_private_scope(&self, _: &str) -> StoreResult<Option<PrivateScope>> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_resume_private(&self) -> StoreResult<(std::fs::File, String)> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_permit_matches(&self, _: &std::fs::File) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_same_store(&self, _: &Self) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_permit_authorizes(&self, _: &std::fs::File, _: &str) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_finish_private(
        &self,
        _: &std::fs::File,
        _: &str,
    ) -> StoreResult<CleanupCommit> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_mark_uncertain(&self) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_command_lease(&self) -> StoreResult<(std::fs::File, String)> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn gate_complete_command(&self, _: &str) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub fn open(_: &Path) -> StoreResult<Self> {
        Err(UNAVAILABLE)
    }
    pub fn stage(&self, _: &[u8]) -> StoreResult<String> {
        Err(UNAVAILABLE)
    }
    pub fn load(&self, _: &str) -> StoreResult<Vec<u8>> {
        Err(UNAVAILABLE)
    }
    pub fn admit(&self, _: &str) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn reserve_backup_code(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &[u8],
    ) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub fn store_result(&self, _: &str, _: &[u8]) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub(crate) fn reserve_journey(&self, _: &str, _: &[String]) -> StoreResult<()> {
        Err(UNAVAILABLE)
    }
    pub fn load_result(&self, _: &str) -> StoreResult<Vec<u8>> {
        Err(UNAVAILABLE)
    }
    pub fn status(&self, _: &str) -> StoreResult<&'static str> {
        Err(UNAVAILABLE)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
        path::PathBuf,
        sync::Arc,
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("ab-private-store-{}", uuid::Uuid::new_v4()));
            Self(root)
        }
        fn store(&self) -> SecretStore {
            SecretStore::open(&self.0).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn private_journey_persists_order_and_rejects_replay_after_restart() {
        let fixture = Fixture::new();
        let store = SecretStore::open_gate(&fixture.0).unwrap();
        let first = "a".repeat(64);
        let second = "b".repeat(64);
        let (lease, epoch) = store
            .gate_begin_private_journey(
                "retained",
                &[first.clone(), second.clone(), second.clone()],
            )
            .unwrap();
        assert!(store
            .gate_advance_private(&lease, &epoch, &first, 0, "wrong", &second)
            .is_err());
        assert!(store
            .gate_advance_private(&lease, &epoch, &first, 0, "retained", &"c".repeat(64))
            .is_err());
        assert!(store
            .gate_advance_private(&lease, &epoch, &first, 1, "retained", &second)
            .is_err());
        let scope = store
            .gate_advance_private(&lease, &epoch, &first, 0, "retained", &second)
            .unwrap();
        assert_eq!(scope.journey_stage(), Some(1));
        drop(lease);
        drop(store);
        let store = SecretStore::open_gate(&fixture.0).unwrap();
        assert!(store.gate_public_lease().is_err());
        let (lease, restored_epoch) = store.gate_resume_private().unwrap();
        assert_eq!(restored_epoch, epoch);
        assert_eq!(
            store
                .gate_private_scope(&epoch)
                .unwrap()
                .unwrap()
                .journey_stage(),
            Some(1)
        );
        assert!(store
            .gate_advance_private(&lease, &epoch, &second, 0, "retained", &second)
            .is_err());
        assert_eq!(
            store
                .gate_advance_private(&lease, &epoch, &second, 1, "retained", &second)
                .unwrap()
                .journey_stage(),
            Some(2)
        );
        assert!(store
            .gate_advance_private(&lease, &epoch, &second, 1, "retained", &second)
            .is_err());
        assert!(store
            .gate_advance_private(&lease, &epoch, &second, 2, "retained", &second)
            .is_err());
    }

    #[test]
    fn private_journey_crash_helper() {
        let Ok(root) = std::env::var("AB_PRIVATE_JOURNEY_TEST_ROOT") else {
            return;
        };
        let store = SecretStore::open_gate(Path::new(&root)).unwrap();
        let first = "a".repeat(64);
        let second = "b".repeat(64);
        let (lease, epoch) = store
            .gate_begin_private_journey("retained", &[first.clone(), second.clone()])
            .unwrap();
        store
            .gate_advance_private(&lease, &epoch, &first, 0, "retained", &second)
            .unwrap();
        std::process::exit(37);
    }

    #[test]
    fn private_journey_crash_after_commit_stays_private_and_consumed() {
        let fixture = Fixture::new();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::private_secret_store::tests::private_journey_crash_helper",
            ])
            .env("AB_PRIVATE_JOURNEY_TEST_ROOT", &fixture.0)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(37));
        let store = SecretStore::open_gate(&fixture.0).unwrap();
        assert!(store.gate_public_lease().is_err());
        let (lease, epoch) = store.gate_resume_private().unwrap();
        let scope = store.gate_private_scope(&epoch).unwrap().unwrap();
        assert_eq!(scope.journey_stage(), Some(1));
        assert_eq!(scope.identity_digest(), "b".repeat(64));
        assert!(store
            .gate_advance_private(
                &lease,
                &epoch,
                &"a".repeat(64),
                0,
                "retained",
                &"b".repeat(64)
            )
            .is_err());
    }

    #[test]
    fn ciphertext_and_fixed_status_roundtrip() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let sentinel = b"SYNTHETIC_PRIVATE_SENTINEL_not_a_credential";
        let reference = store.stage(sentinel).unwrap();
        assert_eq!(store.status(&reference).unwrap(), "staged");
        assert_eq!(store.load(&reference).unwrap(), sentinel);
        assert!(store.store_result(&reference, sentinel).is_err());
        store.admit(&reference).unwrap();
        assert_eq!(store.status(&reference).unwrap(), "admitted");
        store.store_result(&reference, sentinel).unwrap();
        assert_eq!(store.load_result(&reference).unwrap(), sentinel);
        assert_eq!(store.status(&reference).unwrap(), "result_ready");
        assert!(store.store_result(&reference, b"replacement").is_err());
        for entry in fs::read_dir(&fixture.0).unwrap() {
            let bytes = fs::read(entry.unwrap().path()).unwrap();
            assert!(!bytes
                .windows(sentinel.len())
                .any(|window| window == sentinel));
        }
    }

    #[test]
    fn concurrent_admission_and_restart_replay() {
        let fixture = Fixture::new();
        let store = Arc::new(fixture.store());
        let reference = store.stage(b"synthetic").unwrap();
        let workers: Vec<_> = (0..12)
            .map(|_| {
                let store = Arc::clone(&store);
                let reference = reference.clone();
                std::thread::spawn(move || store.admit(&reference).is_ok())
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .filter_map(|worker| worker.join().ok())
                .filter(|won| *won)
                .count(),
            1
        );
        drop(store);
        assert!(fixture.store().admit(&reference).is_err());
        fs::write(fixture.0.join(format!("{reference}.admitted")), b"").unwrap();
        assert!(fixture.store().admit(&reference).is_err());
    }

    #[test]
    fn tamper_reference_swap_and_role_swap_fail_closed() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let first = store.stage(b"first").unwrap();
        let second = store.stage(b"second").unwrap();
        let first_path = fixture.0.join(format!("{first}.payload"));
        let second_path = fixture.0.join(format!("{second}.payload"));
        fs::copy(&first_path, second_path).unwrap();
        assert!(store.load(&second).is_err());
        store.admit(&first).unwrap();
        fs::copy(&first_path, fixture.0.join(format!("{first}.result"))).unwrap();
        assert!(store.load_result(&first).is_err());
        let mut bytes = fs::read(&first_path).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(first_path, bytes).unwrap();
        assert!(store.load(&first).is_err());
        assert!(store.load("../../escape").is_err());
        assert!(store.stage(&vec![0; MAX_SECRET_BYTES + 1]).is_err());
    }

    #[test]
    fn filesystem_guards_and_missing_key() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic").unwrap();
        let payload = fixture.0.join(format!("{reference}.payload"));
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.load(&reference).is_err());
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o600)).unwrap();
        let linked = fixture.0.join("unexpected-link");
        fs::hard_link(&payload, &linked).unwrap();
        assert!(store.load(&reference).is_err());
        fs::remove_file(linked).unwrap();
        fs::remove_file(&payload).unwrap();
        symlink("store.key", &payload).unwrap();
        assert!(store.load(&reference).is_err());
        fs::remove_file(fixture.0.join("store.key")).unwrap();
        assert!(SecretStore::open(&fixture.0).is_err());
        assert!(!fixture.0.join("store.key").exists());
        assert!(store.stage(b"synthetic").is_err());
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(SecretStore::open(&fixture.0).is_err());
    }

    #[test]
    fn root_symlink_and_invalid_existing_key_are_never_repaired() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let alias = fixture.0.join("alias");
        symlink(&fixture.0, &alias).unwrap();
        assert!(SecretStore::open(&alias).is_err());
        fs::write(fixture.0.join("store.key"), b"invalid").unwrap();
        assert!(SecretStore::open(&fixture.0).is_err());
        assert!(store.stage(b"synthetic").is_err());
        assert_eq!(fs::read(fixture.0.join("store.key")).unwrap(), b"invalid");
    }

    #[test]
    fn independent_openers_admit_at_most_once() {
        let fixture = Fixture::new();
        let first = fixture.store();
        let second = fixture.store();
        let reference = first.stage(b"synthetic").unwrap();
        let other_reference = reference.clone();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let other_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            other_barrier.wait();
            second.admit(&other_reference).is_ok()
        });
        barrier.wait();
        let won = first.admit(&reference).is_ok();
        assert_eq!(usize::from(won) + usize::from(worker.join().unwrap()), 1);
        assert!(fixture.store().admit(&reference).is_err());
    }

    #[test]
    fn oversized_and_interrupted_files_fail_closed() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic").unwrap();
        let payload = fixture.0.join(format!("{reference}.payload"));
        fs::OpenOptions::new()
            .write(true)
            .open(&payload)
            .unwrap()
            .set_len((MAX_SECRET_BYTES + 128) as u64)
            .unwrap();
        assert!(store.load(&reference).is_err());
        assert!(store.admit(&reference).is_err());
        fs::write(fixture.0.join("store.key"), b"").unwrap();
        assert!(SecretStore::open(&fixture.0).is_err());
        assert_eq!(fs::metadata(fixture.0.join("store.key")).unwrap().len(), 0);
    }

    #[test]
    fn subprocess_admit_helper() {
        let Some(root) = std::env::var_os("AB_PRIVATE_STORE_TEST_ROOT") else {
            return;
        };
        let reference = std::env::var("AB_PRIVATE_STORE_TEST_REFERENCE").unwrap();
        let store = SecretStore::open(Path::new(&root)).unwrap();
        store.admit(&reference).unwrap();
        // Exit immediately, without running Rust destructors or any browser work.
        std::process::exit(37);
    }

    #[test]
    fn subprocess_death_after_admission_remains_consumed() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::private_secret_store::tests::subprocess_admit_helper",
            ])
            .env("AB_PRIVATE_STORE_TEST_ROOT", &fixture.0)
            .env("AB_PRIVATE_STORE_TEST_REFERENCE", &reference)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(37));
        assert!(store.admit(&reference).is_err());
        assert!(fixture.store().admit(&reference).is_err());
        assert_eq!(store.status(&reference).unwrap(), "admitted");
    }

    #[test]
    fn backup_code_restaging_and_new_consent_cannot_reset_reservations() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let first = store.stage(b"synthetic operation one").unwrap();
        let second = store.stage(b"synthetic operation two").unwrap();
        store
            .reserve_backup_code(
                &first,
                &"a".repeat(64),
                "login.gov:canonical-account",
                "slack:reviewed-message:slot-0",
                b"abcd-1234-EFGH-5678",
            )
            .unwrap();
        // New consent, source slot and opaque ref still match the same account/code.
        assert_eq!(
            store.reserve_backup_code(
                &second,
                &"b".repeat(64),
                "login.gov:canonical-account",
                "slack:different-message:slot-1",
                b"ABCD 1234 efgh 5678"
            ),
            Err("private_backup_code_reserved")
        );
        // A different code does not permit reusing the original reviewed source slot.
        assert_eq!(
            store.reserve_backup_code(
                &second,
                &"b".repeat(64),
                "login.gov:canonical-account",
                "slack:reviewed-message:slot-0",
                b"different-code-9999"
            ),
            Err("private_backup_source_reserved")
        );
        assert_eq!(
            store.reserve_backup_code(
                &first,
                &"c".repeat(64),
                "login.gov:canonical-account",
                "slack:third-message:slot-2",
                b"different-code-9999"
            ),
            Err("private_backup_reference_reserved")
        );
        // Reservation is separate from the existing operation-admission contract.
        assert_eq!(store.status(&first).unwrap(), "staged");
        store.admit(&first).unwrap();
        assert_eq!(store.status(&first).unwrap(), "admitted");
    }

    #[test]
    fn backup_reservation_is_account_scoped_and_contains_no_raw_inputs() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let account = "SYNTHETIC_ACCOUNT_IDENTITY_MARKER";
        let source = "SYNTHETIC_SLACK_SOURCE_SLOT_MARKER";
        let code = b"SYNTHETICBACKUPCODE1234567890";
        for name in [account, "distinct-canonical-account"] {
            let reference = store.stage(b"synthetic reviewed operation").unwrap();
            store
                .reserve_backup_code(&reference, &"a".repeat(64), name, source, code)
                .unwrap();
        }
        for entry in fs::read_dir(&fixture.0).unwrap() {
            let entry = entry.unwrap();
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            let contents = fs::read(entry.path()).unwrap();
            for marker in [account.as_bytes(), source.as_bytes(), code.as_slice()] {
                assert!(!filename
                    .as_bytes()
                    .windows(marker.len())
                    .any(|window| window == marker));
                assert!(!contents
                    .windows(marker.len())
                    .any(|window| window == marker));
            }
        }
    }

    #[test]
    fn backup_reservations_survive_restart_and_empty_interrupted_receipts() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic operation").unwrap();
        store
            .reserve_backup_code(
                &reference,
                &"a".repeat(64),
                "account",
                "source-slot",
                b"backup1234",
            )
            .unwrap();
        drop(store);
        let store = fixture.store();
        let reference = store.stage(b"restaged synthetic operation").unwrap();
        assert_eq!(
            store.reserve_backup_code(
                &reference,
                &"b".repeat(64),
                "account",
                "other-slot",
                b"backup1234"
            ),
            Err("private_backup_code_reserved")
        );
        for entry in fs::read_dir(&fixture.0).unwrap() {
            let entry = entry.unwrap();
            if entry
                .file_name()
                .to_string_lossy()
                .ends_with(".backup-source")
            {
                fs::write(entry.path(), b"").unwrap();
            }
        }
        assert_eq!(
            store.reserve_backup_code(
                &reference,
                &"b".repeat(64),
                "account",
                "source-slot",
                b"other1234"
            ),
            Err("private_backup_source_reserved")
        );
    }

    #[test]
    fn invalid_backup_inputs_do_not_create_reservations() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic").unwrap();
        for code in [b"".as_slice(), b"---   ", b"code\n", b"code/part", &[0xff]] {
            assert!(store
                .reserve_backup_code(&reference, &"a".repeat(64), "account", "slot", code)
                .is_err());
        }
        assert!(store
            .reserve_backup_code(&reference, "invalid", "account", "slot", b"code1234")
            .is_err());
        assert!(store
            .reserve_backup_code(&reference, &"a".repeat(64), " account", "slot", b"code1234")
            .is_err());
        assert!(store
            .reserve_backup_code(&reference, &"a".repeat(64), "account", "", b"code1234")
            .is_err());
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
    }

    #[test]
    fn concurrent_restaged_backup_code_has_one_winner() {
        let fixture = Fixture::new();
        let store = Arc::new(fixture.store());
        let references: Vec<_> = (0..8).map(|_| store.stage(b"synthetic").unwrap()).collect();
        let workers: Vec<_> = references
            .into_iter()
            .enumerate()
            .map(|(slot, reference)| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    store
                        .reserve_backup_code(
                            &reference,
                            &"a".repeat(64),
                            "account",
                            &format!("source-slot-{slot}"),
                            b"samebackup1234",
                        )
                        .is_ok()
                })
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .filter_map(|worker| worker.join().ok())
                .filter(|won| *won)
                .count(),
            1
        );
    }

    #[test]
    fn independent_store_handles_reserve_backup_code_at_most_once() {
        let fixture = Fixture::new();
        let first = fixture.store();
        let second = fixture.store();
        let first_ref = first.stage(b"synthetic first").unwrap();
        let second_ref = second.stage(b"synthetic second").unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let other_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            other_barrier.wait();
            second
                .reserve_backup_code(
                    &second_ref,
                    &"b".repeat(64),
                    "account",
                    "source-second",
                    b"samebackup1234",
                )
                .is_ok()
        });
        barrier.wait();
        let first_won = first
            .reserve_backup_code(
                &first_ref,
                &"a".repeat(64),
                "account",
                "source-first",
                b"samebackup1234",
            )
            .is_ok();
        assert_eq!(
            usize::from(first_won) + usize::from(worker.join().unwrap()),
            1
        );
        let reopened = fixture.store();
        let fresh_ref = reopened.stage(b"synthetic restart").unwrap();
        assert_eq!(
            reopened.reserve_backup_code(
                &fresh_ref,
                &"c".repeat(64),
                "account",
                "source-third",
                b"samebackup1234"
            ),
            Err("private_backup_code_reserved")
        );
    }

    #[test]
    fn subprocess_backup_reservation_helper() {
        let Some(root) = std::env::var_os("AB_PRIVATE_BACKUP_TEST_ROOT") else {
            return;
        };
        let reference = std::env::var("AB_PRIVATE_BACKUP_TEST_REFERENCE").unwrap();
        let store = SecretStore::open(Path::new(&root)).unwrap();
        store
            .reserve_backup_code(
                &reference,
                &"a".repeat(64),
                "synthetic-account",
                "synthetic-source-slot",
                b"syntheticbackup1234",
            )
            .unwrap();
        std::process::exit(41);
    }

    #[test]
    fn subprocess_exit_after_backup_reservation_prevents_restaging() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let reference = store.stage(b"synthetic").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::private_secret_store::tests::subprocess_backup_reservation_helper",
            ])
            .env("AB_PRIVATE_BACKUP_TEST_ROOT", &fixture.0)
            .env("AB_PRIVATE_BACKUP_TEST_REFERENCE", &reference)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(41));
        let reference = store.stage(b"synthetic restaged").unwrap();
        assert_eq!(
            store.reserve_backup_code(
                &reference,
                &"b".repeat(64),
                "synthetic-account",
                "different-source-slot",
                b"syntheticbackup1234"
            ),
            Err("private_backup_code_reserved")
        );
    }
}
