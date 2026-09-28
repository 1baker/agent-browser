//! Installer admission exclusion, never browser ownership or attestation.
//! A dead owner does not release exclusion: explicit installer recovery is needed.
#![cfg(target_os = "linux")]

use super::handoff_custody::ProcessIdentity;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const FILE_NAME: &str = "runtime-admission.json";
const MAX_BYTES: u64 = 16 * 1024;
const REFUSED: &str = "publication_admission_refused";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    schema_version: String,
    id: String,
    owner_process: ProcessIdentity,
    allowed_session_names: Vec<String>,
    capability_sha256: String,
}

/// Startup authorization is supplied only through the private launch environment.
pub(crate) fn verify_startup(session: &str) -> Result<(), String> {
    let capability = std::env::var("AGENT_BROWSER_PUBLICATION_CAPABILITY").ok();
    verify_at(&admission_directory()?, session, capability.as_deref())
}

/// Each worker command needs its own capability; daemon startup authority must
/// not silently authorize other clients. Call before tracing or command effects,
/// and remove the private field before persisting/logging/forwarding the command.
pub(crate) fn verify_command(session: &str, command: &Value) -> Result<(), String> {
    verify_at(
        &admission_directory()?,
        session,
        command
            .get("_publicationCapability")
            .and_then(Value::as_str),
    )
}

fn admission_directory() -> Result<std::path::PathBuf, String> {
    // This is deliberately independent of AGENT_BROWSER_SOCKET_DIR and XDG:
    // choosing another socket namespace must not bypass publication admission.
    dirs::home_dir()
        .map(|home| home.join(".agent-browser/publications"))
        .ok_or_else(|| REFUSED.into())
}

fn refused<T>() -> Result<T, String> {
    Err(REFUSED.into())
}

fn same_inode(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

fn private_file(metadata: &Metadata, uid: u32) -> bool {
    metadata.is_file()
        && metadata.uid() == uid
        && metadata.mode() & 0o7777 == 0o600
        && metadata.nlink() == 1
        && metadata.len() > 0
        && metadata.len() <= MAX_BYTES
}

fn trusted_directory(path: &Path, uid: u32) -> Result<File, String> {
    if !path.is_absolute() {
        return refused();
    }
    // Ancestors must not redirect the fixed fence or permit unrelated users to
    // replace it. Root-owned sticky /tmp is safe for a privately owned child.
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| REFUSED.to_string())?;
        if !metadata.is_dir()
            || (metadata.uid() != uid && metadata.uid() != 0)
            || (metadata.mode() & 0o022 != 0
                && !(metadata.uid() == 0 && metadata.mode() & 0o1000 != 0))
        {
            return refused();
        }
    }
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| REFUSED.to_string())?;
    let metadata = directory.metadata().map_err(|_| REFUSED.to_string())?;
    if metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return refused();
    }
    Ok(directory)
}

fn verify_at(directory_path: &Path, session: &str, capability: Option<&str>) -> Result<(), String> {
    let path = directory_path.join(FILE_NAME);
    let named = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return verify_absent_path(directory_path);
        }
        Err(_) => return refused(),
    };
    // SAFETY: geteuid has no preconditions.
    let uid = unsafe { libc::geteuid() };
    if !private_file(&named, uid) {
        return refused();
    }
    let directory = trusted_directory(directory_path, uid)?;
    // SAFETY: the directory descriptor and literal NUL-terminated name are live.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"runtime-admission.json".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return refused();
    }
    // SAFETY: openat returned a new descriptor exclusively owned by this File.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let before = file.metadata().map_err(|_| REFUSED.to_string())?;
    if !private_file(&before, uid) || !same_inode(&before, &named) {
        return refused();
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| REFUSED.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return refused();
    }
    let receipt: Receipt = serde_json::from_slice(&bytes).map_err(|_| REFUSED.to_string())?;
    if receipt.schema_version != "agent-browser.publication-admission.v1"
        || receipt.id.len() != 36
        || uuid::Uuid::parse_str(&receipt.id).is_err()
        || receipt.owner_process.uid != uid
        || receipt.allowed_session_names.is_empty()
        || receipt.allowed_session_names.len() > 64
    {
        return refused();
    }
    let mut sessions = HashSet::new();
    for allowed in &receipt.allowed_session_names {
        if allowed.is_empty()
            || allowed.len() > 128
            || !allowed.as_bytes()[0].is_ascii_alphanumeric()
            || !allowed
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            || !sessions.insert(allowed)
        {
            return refused();
        }
    }
    if !receipt
        .allowed_session_names
        .iter()
        .any(|allowed| allowed == session)
    {
        return refused();
    }
    let Some(capability) = capability.filter(|value| !value.is_empty() && value.len() <= 4096)
    else {
        return refused();
    };
    let expected = receipt.capability_sha256.as_bytes();
    if expected.len() != 64 || !expected.iter().all(u8::is_ascii_hexdigit) {
        return refused();
    }
    let actual = format!("{:x}", Sha256::digest(capability.as_bytes()));
    // Compare the fixed-length digest without a prefix-dependent early return.
    let different = actual
        .bytes()
        .zip(expected)
        .fold(0_u8, |diff, (a, b)| diff | (a ^ b.to_ascii_lowercase()));
    if different != 0 {
        return refused();
    }
    receipt
        .owner_process
        .verify_current()
        .map_err(|_| REFUSED.to_string())?;
    let after = file.metadata().map_err(|_| REFUSED.to_string())?;
    let current = fs::symlink_metadata(&path).map_err(|_| REFUSED.to_string())?;
    let current_directory =
        fs::symlink_metadata(directory_path).map_err(|_| REFUSED.to_string())?;
    if !private_file(&after, uid)
        || !private_file(&current, uid)
        || !same_inode(&before, &after)
        || !same_inode(&after, &current)
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || !current_directory.is_dir()
        || !same_inode(
            &directory.metadata().map_err(|_| REFUSED.to_string())?,
            &current_directory,
        )
    {
        return refused();
    }
    Ok(())
}

// Clean installations may lack the publications directory. Missing is allowed
// only through trustworthy existing ancestors, never through a directory link.
fn verify_absent_path(directory_path: &Path) -> Result<(), String> {
    if !directory_path.is_absolute() {
        return refused();
    }
    let uid = unsafe { libc::geteuid() };
    for ancestor in directory_path.ancestors() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return refused(),
        };
        if !metadata.is_dir()
            || (metadata.uid() != uid && metadata.uid() != 0)
            || (metadata.mode() & 0o022 != 0
                && !(metadata.uid() == 0 && metadata.mode() & 0o1000 != 0))
        {
            return refused();
        }
    }
    // Bracket absence validation: a newly present receipt requires a new check,
    // never an authorization based on the earlier missing-path observation.
    match fs::symlink_metadata(directory_path.join(FILE_NAME)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        _ => refused(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "publication-admission-test-{}",
                uuid::Uuid::new_v4()
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn receipt(&self) -> Value {
            json!({"schemaVersion":"agent-browser.publication-admission.v1",
                "id":uuid::Uuid::new_v4().to_string(),
                "ownerProcess":ProcessIdentity::capture(std::process::id()).unwrap(),
                "allowedSessionNames":["default"],
                "capabilitySha256":format!("{:x}",Sha256::digest(b"fixture-only-capability"))})
        }
        fn write(&self, value: &Value) {
            let path = self.0.join(FILE_NAME);
            fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        fn check(&self, session: &str, capability: Option<&str>) -> Result<(), String> {
            verify_at(&self.0, session, capability)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn absent_fence_allows_without_capability() {
        let fixture = Fixture::new();
        assert!(fixture.check("default", None).is_ok());
        assert!(verify_at(&fixture.0.join("missing/publications"), "default", None).is_ok());
    }

    #[test]
    fn absent_fence_through_symlink_or_unsafe_ancestor_is_refused() {
        let fixture = Fixture::new();
        let empty = fixture.0.join("empty");
        fs::create_dir(&empty).unwrap();
        for target in [&empty, &fixture.0.join("missing")] {
            let alias = fixture.0.join("alias");
            symlink(target, &alias).unwrap();
            assert!(verify_at(&alias, "unapproved", None).is_err());
            assert!(verify_at(&alias.join("publications"), "unapproved", None).is_err());
            fs::remove_file(&alias).unwrap();
        }
        fs::set_permissions(&empty, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(verify_at(&empty.join("missing"), "unapproved", None).is_err());
    }

    #[test]
    fn exact_live_owner_session_and_capability_are_required() {
        let fixture = Fixture::new();
        fixture.write(&fixture.receipt());
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_ok());
        for capability in [None, Some(""), Some("wrong")] {
            assert_eq!(fixture.check("default", capability), Err(REFUSED.into()));
        }
        assert!(fixture
            .check("other", Some("fixture-only-capability"))
            .is_err());
    }

    #[test]
    fn stale_owner_and_unknown_fields_fail_closed() {
        let fixture = Fixture::new();
        for field in ["pid", "startTicks", "executableInode", "uid"] {
            let mut value = fixture.receipt();
            value["ownerProcess"][field] = json!(0);
            if field == "uid" {
                value["ownerProcess"][field] = json!(u32::MAX);
            }
            fixture.write(&value);
            assert!(
                fixture
                    .check("default", Some("fixture-only-capability"))
                    .is_err(),
                "{field}"
            );
        }
        let mut value = fixture.receipt();
        value["override"] = json!(true);
        fixture.write(&value);
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());
    }

    #[test]
    fn malformed_and_oversized_receipts_fail_closed() {
        let fixture = Fixture::new();
        for bytes in [
            b"not json".to_vec(),
            Vec::new(),
            vec![b' '; MAX_BYTES as usize + 1],
        ] {
            let path = fixture.0.join(FILE_NAME);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
            assert!(fixture
                .check("default", Some("fixture-only-capability"))
                .is_err());
        }
        for (key, bad) in [
            ("schemaVersion", json!("unknown")),
            ("id", json!("invalid")),
            ("capabilitySha256", json!("zz".repeat(32))),
            ("allowedSessionNames", json!(["default", "default"])),
            ("allowedSessionNames", json!([])),
            ("allowedSessionNames", json!(["../default"])),
        ] {
            let mut value = fixture.receipt();
            value[key] = bad;
            fixture.write(&value);
            assert!(
                fixture
                    .check("default", Some("fixture-only-capability"))
                    .is_err(),
                "{key}"
            );
        }
    }

    #[test]
    fn symlinks_hardlinks_and_public_permissions_fail_closed() {
        let fixture = Fixture::new();
        let path = fixture.0.join(FILE_NAME);
        symlink("missing", &path).unwrap();
        assert!(fixture.check("default", None).is_err());
        fs::remove_file(&path).unwrap();
        fixture.write(&fixture.receipt());
        fs::hard_link(&path, fixture.0.join("alias")).unwrap();
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());
        fs::remove_file(fixture.0.join("alias")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());
    }

    #[test]
    fn nonregular_fence_and_symlink_directory_fail_closed() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join(FILE_NAME)).unwrap();
        assert!(fixture.check("default", None).is_err());
        fs::remove_dir(fixture.0.join(FILE_NAME)).unwrap();
        fixture.write(&fixture.receipt());
        let alias = fixture.0.join("directory-alias");
        symlink(&fixture.0, &alias).unwrap();
        assert!(verify_at(&alias, "default", Some("fixture-only-capability")).is_err());
    }

    #[test]
    fn duplicate_fields_and_invalid_owner_shape_fail_closed() {
        let fixture = Fixture::new();
        let mut value = fixture.receipt();
        value["ownerProcess"]["unexpected"] = json!(true);
        fixture.write(&value);
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());

        let serialized = serde_json::to_string(&fixture.receipt()).unwrap();
        let duplicate = format!("{{\"id\":\"{}\",{}", uuid::Uuid::new_v4(), &serialized[1..]);
        let path = fixture.0.join(FILE_NAME);
        fs::write(&path, duplicate).unwrap();
        assert!(fixture
            .check("default", Some("fixture-only-capability"))
            .is_err());
    }
}
