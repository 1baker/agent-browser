//! Dedicated same-user authenticated transport for immutable approved-plan bindings.
//! Key possession authenticates a caller, not browser authority. This module never
//! launches a browser, ingests credentials, or enables live renewal.
use std::path::Path;

#[cfg(target_os = "linux")]
pub(crate) fn coordinator_files(
    root: &Path,
) -> Result<(std::fs::File, [u8; 32], Vec<u8>), &'static str> {
    linux::coordinator_files(root)
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn accept_test_binding(
    stream: std::os::unix::net::UnixStream,
    key: [u8; 32],
    store: &crate::native::private_secret_store::SecretStore,
) -> Result<(), &'static str> {
    linux::accept_test_binding(stream, key, store)
}

pub fn run(root: &Path, mode: &str) -> Result<(), &'static str> {
    #[cfg(target_os = "linux")]
    {
        linux::run(root, mode)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, mode);
        Err("private_controller_unavailable")
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use crate::native::private_secret_store::SecretStore;
    use hmac::{Hmac, Mac};
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};
    use std::ffi::CString;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Component, PathBuf};
    use std::time::{Duration, Instant};

    const ERROR: &str = "private_controller_unavailable";
    const SUCCESS: &[u8] = br#"{"success":true,"renewalEnabled":false}"#;
    const REJECTED: &[u8] =
        br#"{"success":false,"renewalEnabled":false,"error":"private_controller_rejected"}"#;
    const MAX_FRAME: usize = 16_384;
    type Result<T> = std::result::Result<T, &'static str>;
    type HmacSha256 = Hmac<Sha256>;

    struct Key([u8; 32]);
    impl Drop for Key {
        fn drop(&mut self) {
            for byte in &mut self.0 {
                // Best effort: no promise of locked memory or zeroized crypto internals.
                unsafe { std::ptr::write_volatile(byte, 0) };
            }
        }
    }

    fn owned(fd: libc::c_int) -> Result<File> {
        if fd < 0 {
            return Err(ERROR);
        }
        // SAFETY: open/openat returned a fresh, owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    // Reopen every ancestor without symlink traversal, retaining the final FD.
    fn root_directory(root: &Path) -> Result<File> {
        SecretStore::ensure_private_directory(root).map_err(|_| ERROR)?;
        let mut directory = owned(unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        })?;
        let parts: Vec<_> = root.components().collect();
        if !root.is_absolute() || parts.len() < 2 {
            return Err(ERROR);
        }
        for (index, part) in parts.iter().enumerate().skip(1) {
            if !matches!(part, Component::Normal(_)) {
                return Err(ERROR);
            }
            let name = CString::new(part.as_os_str().as_bytes()).map_err(|_| ERROR)?;
            directory = owned(unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            })?;
            let metadata = directory.metadata().map_err(|_| ERROR)?;
            let uid = unsafe { libc::geteuid() };
            if !metadata.is_dir() || metadata.nlink() == 0 {
                return Err(ERROR);
            }
            if index == parts.len() - 1 {
                if metadata.uid() != uid || metadata.mode() & 0o7777 != 0o700 {
                    return Err(ERROR);
                }
            } else if (metadata.uid() != uid && metadata.uid() != 0)
                || (metadata.mode() & 0o022 != 0
                    && !(metadata.uid() == 0 && metadata.mode() & 0o1000 != 0))
            {
                return Err(ERROR);
            }
        }
        Ok(directory)
    }

    #[must_use = "retain the controller lock guard for the entire protected operation"]
    struct DirectoryLock<'a> {
        directory: &'a File,
        owner_pid: u32,
    }

    impl Drop for DirectoryLock<'_> {
        fn drop(&mut self) {
            // A forked child's destructor cannot release its parent's lock.
            if self.owner_pid == std::process::id() {
                // SAFETY: the borrow keeps the descriptor open through Drop.
                // No key, socket or durable authority record is removed here.
                unsafe { libc::flock(self.directory.as_raw_fd(), libc::LOCK_UN) };
            }
        }
    }

    fn lock(directory: &File) -> Result<DirectoryLock<'_>> {
        // Explicit owner lifetime prevents inherited descriptors extending the
        // lock after return. Never unlink a supposed stale lock to acquire it.
        if unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(ERROR);
        }
        Ok(DirectoryLock {
            directory,
            owner_pid: std::process::id(),
        })
    }

    fn key(directory: &File, setup: bool) -> Result<Key> {
        if setup {
            let anchored = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
            match std::fs::symlink_metadata(anchored.join("authentication.key")) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // An absent key in a used root is not permission to rotate identity.
                    if std::fs::read_dir(&anchored)
                        .map_err(|_| ERROR)?
                        .next()
                        .is_some()
                    {
                        return Err(ERROR);
                    }
                }
                Err(_) => return Err(ERROR),
            }
        }
        let mut file = if setup {
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    c"authentication.key".as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd >= 0 {
                let mut file = owned(fd)?;
                let mut generated = Key([0; 32]);
                getrandom::getrandom(&mut generated.0).map_err(|_| ERROR)?;
                file.write_all(&generated.0).map_err(|_| ERROR)?;
                file.sync_all().map_err(|_| ERROR)?;
                directory.sync_all().map_err(|_| ERROR)?;
                drop(file);
            } else if std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ERROR);
            }
            owned(unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    c"authentication.key".as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            })?
        } else {
            owned(unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    c"authentication.key".as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            })?
        };
        let metadata = file.metadata().map_err(|_| ERROR)?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.nlink() != 1
            || metadata.mode() & 0o7777 != 0o600
            || metadata.len() != 32
        {
            return Err(ERROR);
        }
        let mut value = Key([0; 32]);
        file.read_exact(&mut value.0).map_err(|_| ERROR)?;
        Ok(value)
    }

    /// Read independently provisioned authority without creating or rotating it.
    /// Keep the descriptor alive so executor.sock is bound in this exact root.
    pub(super) fn coordinator_files(root: &Path) -> Result<(File, [u8; 32], Vec<u8>)> {
        std::fs::symlink_metadata(root).map_err(|_| ERROR)?;
        let directory = root_directory(root)?;
        let key = key(&directory, false)?;
        let mut file = owned(unsafe {
            libc::openat(
                directory.as_raw_fd(),
                c"execution.json".as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        })?;
        let metadata = file.metadata().map_err(|_| ERROR)?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.nlink() != 1
            || metadata.mode() & 0o7777 != 0o600
            || metadata.len() == 0
            || metadata.len() > 262_144
        {
            return Err(ERROR);
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(262_145)
            .read_to_end(&mut bytes)
            .map_err(|_| ERROR)?;
        let after = file.metadata().map_err(|_| ERROR)?;
        if bytes.len() as u64 != metadata.len()
            || after.len() != metadata.len()
            || after.mtime_nsec() != metadata.mtime_nsec()
            || after.mtime() != metadata.mtime()
            || after.nlink() != 1
            || after.mode() != metadata.mode()
        {
            return Err(ERROR);
        }
        Ok((directory, key.0, bytes))
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Probe {
        operation: ProbeOperation,
    }
    #[derive(Deserialize)]
    enum ProbeOperation {
        #[serde(rename = "probe")]
        Probe,
    }
    #[derive(Deserialize, Serialize)]
    enum BindOperation {
        #[serde(rename = "bind")]
        Bind,
    }
    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    struct Binding {
        operation: BindOperation,
        plan_id: String,
        manifest_sha256: String,
        profile_id: String,
        browser_id: String,
        session_name: String,
        target_id: String,
        endpoint: String,
        expires_at: String,
    }

    fn token(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }
    fn lower_hex(value: &str, length: usize) -> bool {
        value.len() == length
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
    impl Binding {
        fn validate(&self) -> Result<()> {
            if !lower_hex(&self.plan_id, 32)
                || !lower_hex(&self.manifest_sha256, 64)
                || !token(&self.profile_id)
                || !token(&self.session_name)
                || self.browser_id != format!("session:{}", self.session_name)
                || self.target_id.len() != 32
                || !self
                    .target_id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
                || self.endpoint.len() > 2048
                || self
                    .endpoint
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control())
            {
                return Err(ERROR);
            }
            let endpoint = url::Url::parse(&self.endpoint).map_err(|_| ERROR)?;
            if !matches!(endpoint.scheme(), "ws" | "wss")
                || endpoint.host_str().is_none()
                || !endpoint.username().is_empty()
                || endpoint.password().is_some()
                || endpoint.query().is_some()
                || endpoint.fragment().is_some()
                || self
                    .endpoint
                    .split('/')
                    .nth(2)
                    .is_some_and(|authority| authority.contains('@'))
            {
                return Err(ERROR);
            }
            if chrono::DateTime::parse_from_rfc3339(&self.expires_at).map_err(|_| ERROR)?
                <= chrono::Utc::now()
            {
                return Err(ERROR);
            }
            Ok(())
        }
    }

    fn request(body: &[u8], store: &SecretStore) -> Result<()> {
        // Struct deserialization rejects duplicate fields as well as unknown ones.
        if let Ok(probe) = serde_json::from_slice::<Probe>(body) {
            let _ = probe.operation;
            return Ok(());
        }
        let binding: Binding = serde_json::from_slice(body).map_err(|_| ERROR)?;
        binding.validate()?;
        let plan_key = hex::encode(Sha256::digest(binding.plan_id.as_bytes()));
        if store.reconciliation_reference(&plan_key).is_ok() {
            return Err(ERROR);
        }
        let serialized = serde_json::to_vec(&binding).map_err(|_| ERROR)?;
        let reference = store.stage(&serialized).map_err(|_| ERROR)?;
        // create-new bookmark is the final immutable barrier, even if lookup failed.
        store
            .bind_reconciliation(&plan_key, &reference)
            .map_err(|_| ERROR)
    }

    fn remaining(deadline: Instant) -> Result<Duration> {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(ERROR)
    }
    fn read(stream: &mut UnixStream, mut bytes: &mut [u8], deadline: Instant) -> Result<()> {
        while !bytes.is_empty() {
            stream
                .set_read_timeout(Some(remaining(deadline)?))
                .map_err(|_| ERROR)?;
            let count = stream.read(bytes).map_err(|_| ERROR)?;
            if count == 0 {
                return Err(ERROR);
            }
            bytes = &mut bytes[count..];
        }
        Ok(())
    }
    fn write(stream: &mut UnixStream, mut bytes: &[u8], deadline: Instant) -> Result<()> {
        while !bytes.is_empty() {
            stream
                .set_write_timeout(Some(remaining(deadline)?))
                .map_err(|_| ERROR)?;
            let count = stream.write(bytes).map_err(|_| ERROR)?;
            if count == 0 {
                return Err(ERROR);
            }
            bytes = &bytes[count..];
        }
        Ok(())
    }
    fn mac(key: &Key, domain: &[u8], challenge: &[u8; 32], body: &[u8]) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&key.0).expect("fixed HMAC key length");
        mac.update(domain);
        mac.update(challenge);
        mac.update(body);
        mac
    }
    fn peer(stream: &UnixStream) -> Result<()> {
        let mut credential: libc::ucred = unsafe { std::mem::zeroed() };
        let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: writable credential storage and matching length are supplied.
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credential as *mut libc::ucred).cast(),
                &mut size,
            )
        } != 0
            || size as usize != std::mem::size_of::<libc::ucred>()
            || credential.uid != unsafe { libc::geteuid() }
        {
            return Err(ERROR);
        }
        Ok(())
    }
    fn client(mut stream: UnixStream, key: &Key, store: &SecretStore) -> Result<()> {
        peer(&stream)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut challenge = [0; 32];
        getrandom::getrandom(&mut challenge).map_err(|_| ERROR)?;
        write(&mut stream, &challenge, deadline)?;
        let accepted = (|| {
            let mut length = [0; 4];
            read(&mut stream, &mut length, deadline)?;
            let length = u32::from_be_bytes(length) as usize;
            if length == 0 || length > MAX_FRAME {
                return Err(ERROR);
            }
            let mut body = vec![0; length];
            read(&mut stream, &mut body, deadline)?;
            let mut tag = [0; 32];
            read(&mut stream, &mut tag, deadline)?;
            stream
                .set_read_timeout(Some(remaining(deadline)?))
                .map_err(|_| ERROR)?;
            if stream.read(&mut [0; 1]).map_err(|_| ERROR)? != 0 {
                return Err(ERROR);
            }
            mac(key, b"ABPC1-request\0", &challenge, &body)
                .verify_slice(&tag)
                .map_err(|_| ERROR)?;
            request(&body, store)
        })();
        let response = if accepted.is_ok() { SUCCESS } else { REJECTED };
        let tag = mac(key, b"ABPC1-response\0", &challenge, response)
            .finalize()
            .into_bytes();
        write(
            &mut stream,
            &(response.len() as u32).to_be_bytes(),
            deadline,
        )?;
        write(&mut stream, response, deadline)?;
        write(&mut stream, &tag, deadline)
    }

    #[cfg(test)]
    pub(super) fn accept_test_binding(
        stream: UnixStream,
        key: [u8; 32],
        store: &SecretStore,
    ) -> Result<()> {
        client(stream, &Key(key), store)
    }

    struct SocketCleanup {
        path: PathBuf,
        dev: u64,
        ino: u64,
    }
    impl Drop for SocketCleanup {
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

    pub(super) fn run(root: &Path, mode: &str) -> Result<()> {
        if !matches!(mode, "setup" | "serve") {
            return Err(ERROR);
        }
        let directory = root_directory(root)?;
        let _lock = lock(&directory)?;
        let key = key(&directory, mode == "setup")?;
        if mode == "setup" {
            return Ok(());
        }
        let store = SecretStore::open(&root.join("bindings")).map_err(|_| ERROR)?;
        let socket = PathBuf::from(format!(
            "/proc/self/fd/{}/controller.sock",
            directory.as_raw_fd()
        ));
        // Bind fails closed for every existing entry. Never guess that a socket is stale.
        let listener = UnixListener::bind(&socket).map_err(|_| ERROR)?;
        let metadata = std::fs::symlink_metadata(&socket).map_err(|_| ERROR)?;
        let _cleanup = SocketCleanup {
            path: socket.clone(),
            dev: metadata.dev(),
            ino: metadata.ino(),
        };
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| ERROR)?;
        for connection in listener.incoming() {
            if let Ok(stream) = connection {
                let _ = client(stream, &key, &store);
            } else {
                return Err(ERROR);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::Shutdown;

        struct Fixture(PathBuf);
        impl Fixture {
            fn new() -> Self {
                Self(
                    std::env::temp_dir()
                        .join(format!("ab-controller-socket-{}", uuid::Uuid::new_v4())),
                )
            }
            fn store(&self) -> SecretStore {
                SecretStore::ensure_private_directory(&self.0).unwrap();
                SecretStore::open(&self.0.join("bindings")).unwrap()
            }
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        fn binding() -> Vec<u8> {
            serde_json::to_vec(&serde_json::json!({
                "operation":"bind", "planId":"a".repeat(32), "manifestSha256":"b".repeat(64),
                "profileId":"reviewed-profile", "browserId":"session:reviewed-session", "sessionName":"reviewed-session",
                "targetId":"C".repeat(32), "endpoint":"ws://127.0.0.1:9222/devtools/browser/synthetic",
                "expiresAt": (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339()
            })).unwrap()
        }
        #[test]
        fn setup_preserves_key_and_rejects_unsafe_or_missing_identity() {
            let fixture = Fixture::new();
            run(&fixture.0, "setup").unwrap();
            let path = fixture.0.join("authentication.key");
            let original = std::fs::read(&path).unwrap();
            assert_eq!(original.len(), 32);
            assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
            run(&fixture.0, "setup").unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), original);
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(run(&fixture.0, "setup").is_err());
            std::fs::remove_file(&path).unwrap();
            let _store = fixture.store();
            assert!(run(&fixture.0, "setup").is_err());
            assert!(!path.exists());
        }
        #[test]
        fn strict_binding_and_immutable_plan() {
            let fixture = Fixture::new();
            let store = fixture.store();
            assert!(request(br#"{"operation":"probe","operation":"probe"}"#, &store).is_err());
            assert!(request(br#"{"operation":"probe","extra":true}"#, &store).is_err());
            assert!(request(br#"{"operation":"renew"}"#, &store).is_err());
            for (field, invalid) in [
                ("planId", "A".repeat(32)),
                ("manifestSha256", "B".repeat(64)),
                ("targetId", "c".repeat(32)),
                ("sessionName", "../escape".into()),
                ("browserId", "session:other".into()),
                ("endpoint", "ws://user@localhost/path".into()),
                ("endpoint", "ws://localhost/path?secret=x".into()),
                ("endpoint", "https://localhost/path".into()),
                ("expiresAt", "2020-01-01T00:00:00Z".into()),
            ] {
                let mut value: serde_json::Value = serde_json::from_slice(&binding()).unwrap();
                value[field] = serde_json::Value::String(invalid);
                assert!(request(&serde_json::to_vec(&value).unwrap(), &store).is_err());
            }
            let body = binding();
            request(&body, &store).unwrap();
            assert!(request(&body, &store).is_err());
            let mut changed: serde_json::Value = serde_json::from_slice(&body).unwrap();
            changed["manifestSha256"] = serde_json::Value::String("d".repeat(64));
            assert!(request(&serde_json::to_vec(&changed).unwrap(), &store).is_err());
            drop(store);
            assert!(request(&body, &fixture.store()).is_err());
        }
        fn exchange(body: &[u8], valid_mac: bool, extra: bool) -> Vec<u8> {
            let fixture = Fixture::new();
            let store = fixture.store();
            let (server, mut caller) = UnixStream::pair().unwrap();
            let worker = std::thread::spawn(move || client(server, &Key([7; 32]), &store));
            caller
                .set_read_timeout(Some(Duration::from_secs(6)))
                .unwrap();
            let mut challenge = [0; 32];
            caller.read_exact(&mut challenge).unwrap();
            let mut tag = mac(&Key([7; 32]), b"ABPC1-request\0", &challenge, body)
                .finalize()
                .into_bytes();
            if !valid_mac {
                tag[0] ^= 1;
            }
            caller
                .write_all(&(body.len() as u32).to_be_bytes())
                .unwrap();
            caller.write_all(body).unwrap();
            caller.write_all(&tag).unwrap();
            if extra {
                caller.write_all(b"x").unwrap();
            }
            caller.shutdown(Shutdown::Write).unwrap();
            let mut length = [0; 4];
            caller.read_exact(&mut length).unwrap();
            let mut response = vec![0; u32::from_be_bytes(length) as usize];
            caller.read_exact(&mut response).unwrap();
            let mut response_tag = [0; 32];
            caller.read_exact(&mut response_tag).unwrap();
            mac(&Key([7; 32]), b"ABPC1-response\0", &challenge, &response)
                .verify_slice(&response_tag)
                .unwrap();
            worker.join().unwrap().unwrap();
            response
        }
        #[test]
        fn challenge_protocol_authentication_and_eof() {
            let probe = br#"{"operation":"probe"}"#;
            assert_eq!(exchange(probe, true, false), SUCCESS);
            assert_eq!(exchange(probe, false, false), REJECTED);
            assert_eq!(exchange(probe, true, true), REJECTED);
            assert_eq!(exchange(b"\xff", true, false), REJECTED);
        }
        #[test]
        fn controller_lock_drop_releases_inherited_descriptor() {
            let fixture = Fixture::new();
            let inherited = {
                let directory = root_directory(&fixture.0).unwrap();
                let _lease = lock(&directory).unwrap();
                directory.try_clone().unwrap()
            };
            let directory = root_directory(&fixture.0).unwrap();
            let _lease = lock(&directory).unwrap();
            drop(inherited);
            let competitor = root_directory(&fixture.0).unwrap();
            assert!(lock(&competitor).is_err());
        }

        #[test]
        fn controller_lock_child_drop_preserves_parent() {
            let fixture = Fixture::new();
            let directory = root_directory(&fixture.0).unwrap();
            let lease = lock(&directory).unwrap();
            let inherited = directory.try_clone().unwrap();
            let child = DirectoryLock {
                directory: &inherited,
                owner_pid: std::process::id().wrapping_add(1),
            };
            drop(child);
            let competitor = root_directory(&fixture.0).unwrap();
            assert!(lock(&competitor).is_err());
            drop(lease);
            assert!(lock(&competitor).is_ok());
        }

        #[test]
        fn duplicate_lock_and_existing_socket_fail_closed() {
            let fixture = Fixture::new();
            run(&fixture.0, "setup").unwrap();
            let directory = root_directory(&fixture.0).unwrap();
            let lease = lock(&directory).unwrap();
            assert!(run(&fixture.0, "serve").is_err());
            drop(lease);
            drop(directory);
            let path = fixture.0.join("controller.sock");
            let existing = UnixListener::bind(&path).unwrap();
            let inode = std::fs::symlink_metadata(&path).unwrap().ino();
            assert!(run(&fixture.0, "serve").is_err());
            assert_eq!(std::fs::symlink_metadata(&path).unwrap().ino(), inode);
            drop(existing);
        }
    }
}
