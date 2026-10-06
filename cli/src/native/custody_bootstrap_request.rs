//! Bounded operator-only input. Never echo input contents or filesystem errors.
use serde::Deserialize;
use serde_json::{Map, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) const ACTION: &str = "runtime_custody_bootstrap";
pub(crate) const STARTUP_ENV: &str = "AGENT_BROWSER_CUSTODY_BOOTSTRAP_ONLY";
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OperatorRequest {
    pub session_name: String,
    pub service_tab_handle: Map<String, Value>,
    pub expected_predecessor_sha256: String,
    pub physical_profile: PhysicalProfile,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PhysicalProfile {
    pub canonical_profile: PathBuf,
    pub profile_device: u64,
    pub profile_inode: u64,
}

impl OperatorRequest {
    pub(crate) fn validate(&self, session: &str) -> Result<(), &'static str> {
        let digest = self.expected_predecessor_sha256.strip_prefix("sha256:");
        if !crate::validation::is_valid_session_name(&self.session_name)
            || self.session_name != session
            || self.service_tab_handle.is_empty()
            || !self.physical_profile.canonical_profile.is_absolute()
            || self.physical_profile.profile_inode == 0
            || !digest.is_some_and(|v| {
                v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err("fresh_chain_request_invalid");
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn into_bootstrap(self) -> super::runtime_custody_bootstrap::BootstrapRequest {
        super::runtime_custody_bootstrap::BootstrapRequest {
            session_name: self.session_name,
            handle: self.service_tab_handle,
            expected_predecessor_sha256: self.expected_predecessor_sha256,
            profile: super::runtime_custody_bootstrap::PhysicalProfileExpectation {
                canonical_profile: self.physical_profile.canonical_profile,
                profile_device: self.physical_profile.profile_device,
                profile_inode: self.physical_profile.profile_inode,
            },
        }
    }
}

pub(crate) fn parse(value: Value, session: &str) -> Result<OperatorRequest, &'static str> {
    let request: OperatorRequest =
        serde_json::from_value(value).map_err(|_| "fresh_chain_request_invalid")?;
    request.validate(session)?;
    Ok(request)
}

pub(crate) fn read_file(path: &Path, session: &str) -> Result<Value, &'static str> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| "fresh_chain_request_unreadable")?;
    let metadata = file
        .metadata()
        .map_err(|_| "fresh_chain_request_unreadable")?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("fresh_chain_request_invalid");
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "fresh_chain_request_unreadable")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("fresh_chain_request_invalid");
    }
    // Typed parsing rejects duplicate top-level and physical-profile fields.
    let request: OperatorRequest =
        serde_json::from_slice(&bytes).map_err(|_| "fresh_chain_request_invalid")?;
    request.validate(session)?;
    serde_json::from_slice(&bytes).map_err(|_| "fresh_chain_request_invalid")
}

pub(crate) fn startup_mode() -> bool {
    std::env::var(STARTUP_ENV).as_deref() == Ok("1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({"sessionName":"fixture", "serviceTabHandle":{"targetId":"approved"},
            "expectedPredecessorSha256":format!("sha256:{}", "a".repeat(64)),
            "physicalProfile":{"canonicalProfile":"/tmp/fixture", "profileDevice":1,"profileInode":2}})
    }

    #[test]
    fn custody_bootstrap_request_strict_shape_and_binding() {
        assert!(parse(valid(), "fixture").is_ok());
        assert!(parse(valid(), "foreign").is_err());
        for field in ["unknown", "cookies", "autoConnect"] {
            let mut v = valid();
            v[field] = json!(true);
            assert!(parse(v, "fixture").is_err());
        }
        for (field, value) in [
            ("canonicalProfile", json!("relative")),
            ("profileDevice", json!("1")),
            ("profileInode", json!(0)),
            ("unknown", json!(1)),
        ] {
            let mut v = valid();
            v["physicalProfile"][field] = value;
            assert!(parse(v, "fixture").is_err());
        }
        for value in [
            json!("bad"),
            json!(format!("sha256:{}", "A".repeat(64))),
            Value::Null,
        ] {
            let mut v = valid();
            v["expectedPredecessorSha256"] = value;
            assert!(parse(v, "fixture").is_err());
        }
    }

    #[test]
    fn custody_bootstrap_request_file_is_bounded_regular_and_sanitized() {
        let root =
            std::env::temp_dir().join(format!("ab-bootstrap-input-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("request.json");
        std::fs::write(&path, valid().to_string()).unwrap();
        assert_eq!(read_file(&path, "fixture").unwrap(), valid());
        std::fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        assert_eq!(
            read_file(&path, "fixture"),
            Err("fresh_chain_request_invalid")
        );
        std::fs::write(&path, "PRIVATE INPUT NOT TO ECHO").unwrap();
        assert_eq!(
            read_file(&path, "fixture"),
            Err("fresh_chain_request_invalid")
        );
        assert_eq!(
            read_file(&root, "fixture"),
            Err("fresh_chain_request_invalid")
        );
        #[cfg(unix)]
        {
            let link = root.join("symlink");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert_eq!(
                read_file(&link, "fixture"),
                Err("fresh_chain_request_unreadable")
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
