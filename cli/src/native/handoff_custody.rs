//! Linux prospective handoff evidence. These observations are not historical
//! ownership proof. Callers must authenticate the source descriptor, fence old
//! effects, and verify the exact CDP target before committing a receipt.
#![cfg(target_os = "linux")]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProcessIdentity {
    pub pid: u32,
    pub boot_id: String,
    pub start_ticks: u64,
    pub executable_device: u64,
    pub executable_inode: u64,
    pub uid: u32,
}

fn failure(error: impl std::fmt::Display) -> String {
    format!("handoff_custody_observation_failed:{error}")
}

fn start_ticks(pid: u32) -> Result<u64, String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).map_err(failure)?;
    stat.rsplit_once(')')
        .and_then(|(_, tail)| tail.split_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "handoff_custody_process_start_unreadable".to_string())
}

impl ProcessIdentity {
    pub(crate) fn capture(pid: u32) -> Result<Self, String> {
        if pid == 0 {
            return Err("handoff_custody_invalid_pid".into());
        }
        let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map_err(failure)?
            .trim()
            .to_string();
        if boot_id.is_empty() {
            return Err("handoff_custody_boot_id_missing".into());
        }
        let before = start_ticks(pid)?;
        let executable = fs::metadata(format!("/proc/{pid}/exe")).map_err(failure)?;
        let status = fs::read_to_string(format!("/proc/{pid}/status")).map_err(failure)?;
        let uid = status
            .lines()
            .find_map(|line| line.strip_prefix("Uid:"))
            .and_then(|value| value.split_whitespace().nth(1))
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| "handoff_custody_process_uid_missing".to_string())?;
        let after_executable = fs::metadata(format!("/proc/{pid}/exe")).map_err(failure)?;
        if before != start_ticks(pid)?
            || executable.dev() != after_executable.dev()
            || executable.ino() != after_executable.ino()
        {
            return Err("handoff_custody_process_changed".into());
        }
        Ok(Self {
            pid,
            boot_id,
            start_ticks: before,
            executable_device: executable.dev(),
            executable_inode: executable.ino(),
            uid,
        })
    }

    pub(crate) fn verify_current(&self) -> Result<(), String> {
        if Self::capture(self.pid)? == *self {
            Ok(())
        } else {
            Err("handoff_custody_process_identity_mismatch".into())
        }
    }

    /// PID reuse proves the recorded source is gone; exec or UID changes within
    /// the same process do not. Unreadable observations always fail closed.
    pub(crate) fn require_gone(&self) -> Result<(), String> {
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(failure)?;
        if boot.trim().is_empty() {
            return Err("handoff_custody_boot_id_missing".into());
        }
        if boot.trim() != self.boot_id {
            return Ok(());
        }
        match fs::metadata(format!("/proc/{}", self.pid)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(failure(error)),
            Ok(_) => {}
        }
        if start_ticks(self.pid)? != self.start_ticks {
            Ok(())
        } else {
            Err("handoff_custody_source_still_alive".into())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserIdentity {
    pub process: ProcessIdentity,
    pub canonical_profile: PathBuf,
    pub profile_device: u64,
    pub profile_inode: u64,
    /// Endpoint binding supplied by the authenticated source, not a CDP probe.
    pub cdp_endpoint: String,
}

fn process_profile(pid: u32) -> Result<PathBuf, String> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline")).map_err(failure)?;
    let arguments = bytes
        .split(|byte| *byte == 0)
        .filter(|argument| !argument.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)?;
    let mut profiles = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if argument.starts_with("--type=") || *argument == "--type" {
            return Err("handoff_custody_not_browser_process".into());
        }
        if let Some(value) = argument.strip_prefix("--user-data-dir=") {
            profiles.push(value);
        } else if *argument == "--user-data-dir" {
            profiles.push(arguments.get(index + 1).copied().unwrap_or_default());
        }
    }
    let [profile] = profiles.as_slice() else {
        return Err("handoff_custody_profile_argument_not_unique".into());
    };
    if !Path::new(profile).is_absolute() {
        return Err("handoff_custody_profile_argument_not_absolute".into());
    }
    fs::canonicalize(profile).map_err(failure)
}

impl BrowserIdentity {
    pub(crate) fn capture(pid: u32, profile: &Path, cdp_endpoint: &str) -> Result<Self, String> {
        let process = ProcessIdentity::capture(pid)?;
        let profile_before = fs::metadata(profile).map_err(failure)?;
        let canonical_profile = profile.canonicalize().map_err(failure)?;
        let profile_canonical = fs::metadata(&canonical_profile).map_err(failure)?;
        if !profile_before.is_dir()
            || !profile_canonical.is_dir()
            || profile_before.dev() != profile_canonical.dev()
            || profile_before.ino() != profile_canonical.ino()
            || process_profile(pid)? != canonical_profile
        {
            return Err("handoff_custody_browser_profile_mismatch".into());
        }
        let endpoint = url::Url::parse(cdp_endpoint).map_err(failure)?;
        if endpoint.scheme() != "ws"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.port().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint
                .path()
                .strip_prefix("/devtools/browser/")
                .is_none_or(str::is_empty)
        {
            return Err("handoff_custody_endpoint_not_local".into());
        }
        let port = endpoint
            .port()
            .ok_or("handoff_custody_endpoint_port_missing")?;
        let active =
            fs::read_to_string(canonical_profile.join("DevToolsActivePort")).map_err(failure)?;
        let mut lines = active.lines();
        if lines.next().and_then(|value| value.parse::<u16>().ok()) != Some(port)
            || lines.next() != Some(endpoint.path())
        {
            return Err("handoff_custody_active_port_mismatch".into());
        }
        let tcp = fs::read_to_string(format!("/proc/{pid}/net/tcp")).map_err(failure)?;
        let address = format!("0100007F:{port:04X}");
        let listeners = tcp
            .lines()
            .filter_map(|line| {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                (fields.get(1) == Some(&address.as_str()) && fields.get(3) == Some(&"0A"))
                    .then(|| fields.get(9).copied())
                    .flatten()
            })
            .collect::<Vec<_>>();
        let [inode] = listeners.as_slice() else {
            return Err("handoff_custody_listener_not_unique".into());
        };
        let socket = format!("socket:[{inode}]");
        let owns_listener = fs::read_dir(format!("/proc/{pid}/fd"))
            .map_err(failure)?
            .filter_map(Result::ok)
            .any(|entry| fs::read_link(entry.path()).ok().as_deref() == Some(Path::new(&socket)));
        if !owns_listener {
            return Err("handoff_custody_listener_not_owned".into());
        }
        if process_profile(pid)? != canonical_profile {
            return Err("handoff_custody_browser_profile_changed".into());
        }
        process.verify_current()?;
        let profile_after = fs::metadata(&canonical_profile).map_err(failure)?;
        if profile_before.dev() != profile_after.dev()
            || profile_before.ino() != profile_after.ino()
        {
            return Err("handoff_custody_profile_directory_changed".into());
        }
        Ok(Self {
            process,
            canonical_profile,
            profile_device: profile_before.dev(),
            profile_inode: profile_before.ino(),
            cdp_endpoint: cdp_endpoint.to_string(),
        })
    }

    pub(crate) fn verify_current(&self) -> Result<(), String> {
        let observed = Self::capture(
            self.process.pid,
            &self.canonical_profile,
            &self.cdp_endpoint,
        )?;
        if observed == *self {
            Ok(())
        } else {
            Err("handoff_custody_browser_identity_mismatch".into())
        }
    }
}

fn private_directory(path: &Path) -> Result<(), String> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(failure(error)),
    }
    let metadata = fs::symlink_metadata(path).map_err(failure)?;
    // SAFETY: geteuid has no preconditions or memory arguments.
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("handoff_custody_private_directory_required".into());
    }
    Ok(())
}

/// Keep this value alive for the entire destination authority lifetime. Never
/// unlink its lock file: replacing the inode would admit a competing owner.
/// The lock lives in the canonical profile, so different daemon socket roots
/// cannot create independent ownership lanes for the same physical profile.
#[derive(Debug)]
pub(crate) struct DestinationLease {
    file: File,
    lock_path: PathBuf,
    canonical_profile: PathBuf,
    holder: ProcessIdentity,
}

impl DestinationLease {
    pub(crate) fn acquire(_socket_dir: &Path, browser: &BrowserIdentity) -> Result<Self, String> {
        browser.verify_current()?;
        let directory = browser.canonical_profile.join(".agent-browser-custody");
        private_directory(&directory)?;
        let digest = format!(
            "{:x}",
            Sha256::digest(browser.canonical_profile.as_os_str().as_encoded_bytes())
        );
        let lock_path = directory.join(format!("{digest}.lock"));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&lock_path)
            .map_err(failure)?;
        let metadata = file.metadata().map_err(failure)?;
        // SAFETY: geteuid has no preconditions; flock borrows an open file fd.
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err("handoff_custody_private_lock_required".into());
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("handoff_custody_profile_lease_unavailable".into());
        }
        Ok(Self {
            file,
            lock_path,
            canonical_profile: browser.canonical_profile.clone(),
            holder: ProcessIdentity::capture(std::process::id())?,
        })
    }

    pub(crate) fn verify_receipt(&self, receipt: &CustodyReceipt) -> Result<(), String> {
        if receipt.schema_version != 2
            || receipt.browser.canonical_profile != self.canonical_profile
            || receipt.destination != self.holder
            || receipt.target_id.trim().is_empty()
            || receipt.descriptor_sha256.len() != 64
            || !receipt
                .descriptor_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.holder.pid != std::process::id()
        {
            return Err("handoff_custody_receipt_binding_mismatch".into());
        }
        let held = self.file.metadata().map_err(failure)?;
        let named = fs::symlink_metadata(&self.lock_path).map_err(failure)?;
        if !named.is_file()
            || held.nlink() != 1
            || held.dev() != named.dev()
            || held.ino() != named.ino()
        {
            return Err("handoff_custody_lease_inode_changed".into());
        }
        receipt.source.require_gone()?;
        receipt.destination.verify_current()?;
        receipt.browser.verify_current()?;
        Ok(())
    }

    pub(crate) fn persist_receipt(
        &self,
        path: &Path,
        receipt: &CustodyReceipt,
    ) -> Result<(), String> {
        self.verify_receipt(receipt)?;
        if path.extension().is_none_or(|extension| extension != "json") {
            return Err("handoff_custody_receipt_path_invalid".into());
        }
        let parent = path
            .parent()
            .ok_or("handoff_custody_receipt_parent_missing")?;
        private_directory(parent)?;
        match fs::read(path) {
            Ok(bytes) => {
                let existing: CustodyReceipt = serde_json::from_slice(&bytes).map_err(failure)?;
                let mut comparable = existing.clone();
                comparable.phase = receipt.phase.clone();
                if comparable != *receipt
                    || (existing.phase == CustodyPhase::Committed
                        && receipt.phase != CustodyPhase::Committed)
                {
                    return Err("handoff_custody_receipt_transition_mismatch".into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(failure(error)),
        }
        let staged = parent.join(format!(".custody-receipt-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&staged)
                .map_err(failure)?;
            let bytes = serde_json::to_vec_pretty(receipt).map_err(failure)?;
            file.write_all(&bytes).map_err(failure)?;
            file.sync_all().map_err(failure)?;
            fs::rename(&staged, path).map_err(failure)?;
            File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(failure)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&staged);
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CustodyPhase {
    Claimed,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CustodyReceipt {
    pub schema_version: u8,
    pub phase: CustodyPhase,
    pub source: ProcessIdentity,
    pub destination: ProcessIdentity,
    pub browser: BrowserIdentity,
    pub target_id: String,
    pub descriptor_sha256: String,
}

#[cfg(test)]
pub(crate) use tests::{stopped_source, TestBrowserFixture};

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};
    use tokio_tungstenite::tungstenite::Message;

    pub(crate) struct TestBrowserFixture {
        pub(crate) root: PathBuf,
        child: Child,
        pub(crate) browser: BrowserIdentity,
    }
    impl TestBrowserFixture {
        pub(crate) fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("handoff-custody-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            // Synthetic Rust TCP fixture, not Chrome. argv[0] supplies the
            // profile argument without confusing the test harness parser.
            let child = Command::new(std::env::current_exe().unwrap())
                .arg0(format!("--user-data-dir={}", root.display()))
                .arg("native::handoff_custody::tests::custody_browser_fixture_child")
                .arg("--exact")
                .env("HANDOFF_CUSTODY_FIXTURE_PROFILE", &root)
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let active = loop {
                if let Ok(active) = fs::read_to_string(root.join("DevToolsActivePort")) {
                    if active.lines().count() == 2 {
                        break active;
                    }
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "fixture listener not ready"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            let endpoint = format!(
                "ws://127.0.0.1:{}/devtools/browser/test",
                active.lines().next().unwrap()
            );
            let browser = BrowserIdentity::capture(child.id(), &root, &endpoint).unwrap();
            Self {
                root,
                child,
                browser,
            }
        }

        pub(crate) fn methods(&self) -> Vec<String> {
            fs::read_to_string(self.root.join("cdp-methods.log"))
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect()
        }
    }

    /// A prospective source identity whose exact synthetic process has exited.
    pub(crate) fn stopped_source() -> ProcessIdentity {
        let mut source = Command::new("/bin/sh")
            .arg("-c")
            .arg("read value")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::capture(source.id()).unwrap();
        drop(source.stdin.take());
        source.wait().unwrap();
        identity.require_gone().unwrap();
        identity
    }

    fn synthetic_cdp_result(command: &Value) -> Value {
        match command["method"].as_str().unwrap_or_default() {
            "Target.getTargets" => json!({"targetInfos": [{
                "targetId": "exact-target", "type": "page", "title": "Synthetic custody page",
                "url": "https://example.test/custody", "attached": false, "canAccessOpener": false
            }]}),
            "Target.attachToTarget" => json!({"sessionId": "synthetic-session"}),
            "Runtime.evaluate" => {
                let expression = command["params"]["expression"].as_str().unwrap_or_default();
                let value = if expression.contains("document.title") {
                    "Synthetic custody page"
                } else {
                    "https://example.test/custody"
                };
                json!({"result": {"type": "string", "value": value}})
            }
            _ => json!({}),
        }
    }

    async fn serve_synthetic_cdp(stream: tokio::net::TcpStream, method_log: PathBuf) {
        let Ok(mut websocket) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(message)) = websocket.next().await {
            let command = match message {
                Message::Text(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(command) => command,
                    Err(_) => break,
                },
                Message::Ping(bytes) => {
                    if websocket.send(Message::Pong(bytes)).await.is_err() {
                        break;
                    }
                    continue;
                }
                Message::Close(_) => break,
                _ => continue,
            };
            if let Some(method) = command["method"].as_str() {
                let mut log = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&method_log)
                    .unwrap();
                writeln!(log, "{method}").unwrap();
            }
            let mut response =
                json!({"id": command["id"], "result": synthetic_cdp_result(&command)});
            if let Some(session) = command.get("sessionId") {
                response["sessionId"] = session.clone();
            }
            if websocket
                .send(Message::Text(response.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    }

    #[test]
    fn custody_browser_fixture_child() {
        let Some(root) = std::env::var_os("HANDOFF_CUSTODY_FIXTURE_PROFILE") else {
            return;
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let method_log = Path::new(&root).join("cdp-methods.log");
        let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
        let server = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    loop {
                        tokio::select! {
                            _ = &mut stopped => break,
                            accepted = listener.accept() => {
                                let Ok((stream, _)) = accepted else { break; };
                                tokio::spawn(serve_synthetic_cdp(stream, method_log.clone()));
                            }
                        }
                    }
                });
        });
        fs::write(
            Path::new(&root).join("DevToolsActivePort"),
            format!("{}\n/devtools/browser/test\n", port),
        )
        .unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        let _ = stop.send(());
        server.join().unwrap();
    }
    impl Drop for TestBrowserFixture {
        fn drop(&mut self) {
            drop(self.child.stdin.take());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                match self.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    _ => {
                        // Last-resort cleanup targets only this test-owned Child.
                        let _ = self.child.kill();
                        break;
                    }
                }
            }
            let _ = self.child.wait();
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn process_identity_mismatch_and_live_source_are_denied() {
        let identity = ProcessIdentity::capture(std::process::id()).unwrap();
        identity.verify_current().unwrap();
        assert_eq!(
            identity.require_gone().unwrap_err(),
            "handoff_custody_source_still_alive"
        );
        let mut mismatch = identity.clone();
        mismatch.executable_inode ^= 1;
        assert!(mismatch.verify_current().is_err());
        // Executable mismatch cannot be mistaken for source exit.
        assert!(mismatch.require_gone().is_err());
    }

    #[test]
    fn profile_lease_is_exclusive_and_receipt_is_private_and_durable() {
        let fixture = TestBrowserFixture::new();
        let mut replaced_profile = fixture.browser.clone();
        replaced_profile.profile_inode ^= 1;
        assert!(replaced_profile.verify_current().is_err());
        let lease = DestinationLease::acquire(&fixture.root, &fixture.browser).unwrap();
        assert!(DestinationLease::acquire(&fixture.root, &fixture.browser).is_err());
        let other_socket_root = fixture.root.join("different-daemon-sockets");
        fs::create_dir(&other_socket_root).unwrap();
        assert_eq!(
            DestinationLease::acquire(&other_socket_root, &fixture.browser).unwrap_err(),
            "handoff_custody_profile_lease_unavailable"
        );
        let mut source = Command::new("/bin/sh")
            .arg("-c")
            .arg("read value")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let source_identity = ProcessIdentity::capture(source.id()).unwrap();
        let mut receipt = CustodyReceipt {
            schema_version: 2,
            phase: CustodyPhase::Claimed,
            source: source_identity,
            destination: ProcessIdentity::capture(std::process::id()).unwrap(),
            browser: fixture.browser.clone(),
            target_id: "exact-target".into(),
            descriptor_sha256: "a".repeat(64),
        };
        let path = fixture.root.join("handoff-custody/receipt.json");
        assert!(lease.persist_receipt(&path, &receipt).is_err());
        assert!(!path.exists());
        drop(source.stdin.take());
        source.wait().unwrap();
        lease.persist_receipt(&path, &receipt).unwrap();
        receipt.phase = CustodyPhase::Committed;
        lease.persist_receipt(&path, &receipt).unwrap();
        assert_eq!(
            serde_json::from_slice::<CustodyReceipt>(&fs::read(&path).unwrap()).unwrap(),
            receipt
        );
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        drop(lease);
        assert!(DestinationLease::acquire(&other_socket_root, &fixture.browser).is_ok());
    }
}
