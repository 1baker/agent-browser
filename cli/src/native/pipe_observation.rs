//! Short-lived service health observations from an already owned anonymous pipe.
//!
//! These records are not authentication, custody attestations, transferable
//! endpoints, or permission to launch, attach, replace, or stop a browser.

use serde_json::Value;

use super::browser::BrowserManager;

/// Query only the manager's existing pipe. Never discover or open a transport.
pub(crate) async fn capture(browser: &BrowserManager, session_id: &str) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        linux::capture(browser, session_id).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (browser, session_id);
        Err("pipe_observation_linux_only".into())
    }
}

/// Validate health freshness and process continuity, then return target records
/// shaped like HTTP discovery records, without any connectable endpoint fields.
pub(crate) fn validate(
    value: &Value,
    browser_id: &str,
    pid: Option<u32>,
    session_ids: &[String],
) -> Result<Vec<Value>, String> {
    #[cfg(target_os = "linux")]
    {
        linux::validate(value, browser_id, pid, session_ids)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (value, browser_id, pid, session_ids);
        Err("pipe_observation_linux_only".into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashSet;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;
    use std::time::Duration;

    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use super::BrowserManager;
    use crate::native::handoff_custody::ProcessIdentity;

    const SCHEMA: &str = "agent-browser.pipe-observation.v1";
    const TTL_MS: u64 = 15_000;
    const MAX_TARGETS: usize = 256;
    const MAX_TEXT: usize = 16_384;
    const MAX_ID: usize = 256;

    #[derive(Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Observation {
        schema: String,
        transport: String,
        session_id: String,
        browser_id: String,
        pipe_identity: String,
        owner: ProcessIdentity,
        browser: ProcessIdentity,
        observed_boot_ms: u64,
        canonical_profile: String,
        canonical_executable: String,
        version: Version,
        targets: Vec<Target>,
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Version {
        product: String,
        protocol_version: String,
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Target {
        id: String,
        #[serde(rename = "type")]
        kind: String,
        title: String,
        url: String,
    }

    fn failure(error: impl std::fmt::Display) -> String {
        format!("pipe_observation_invalid:{error}")
    }

    fn boot_ms() -> Result<u64, String> {
        let mut value = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: a valid writable timespec is provided to the Linux clock API.
        if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) } != 0 {
            return Err(failure(std::io::Error::last_os_error()));
        }
        let seconds = u64::try_from(value.tv_sec).map_err(failure)?;
        let nanos = u64::try_from(value.tv_nsec).map_err(failure)?;
        seconds
            .checked_mul(1000)
            .and_then(|millis| millis.checked_add(nanos / 1_000_000))
            .ok_or_else(|| failure("clock_overflow"))
    }

    fn canonical(path: &Path) -> Result<String, String> {
        fs::canonicalize(path)
            .map_err(failure)?
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| failure("non_utf8_path"))
    }

    fn text(value: &Value, key: &str) -> Result<String, String> {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| failure(format!("missing_string:{key}")))
    }

    fn bounded(value: &str, max: usize, nonempty: bool) -> bool {
        (!nonempty || !value.is_empty()) && value.len() <= max && !value.contains('\0')
    }

    fn validate_shape(
        observation: &Observation,
        browser_id: &str,
        pid: Option<u32>,
        sessions: &[String],
        now: u64,
    ) -> Result<(), String> {
        if observation.schema != SCHEMA || observation.transport != "anonymous_pipe" {
            return Err(failure("schema_or_transport"));
        }
        if sessions.len() != 1
            || sessions[0] != observation.session_id
            || !bounded(&observation.session_id, MAX_ID, true)
            || observation.browser_id != browser_id
            || observation.browser_id != format!("session:{}", observation.session_id)
            || pid != Some(observation.browser.pid)
        {
            return Err(failure("session_or_browser_binding"));
        }
        let opaque_id = observation
            .pipe_identity
            .strip_prefix("pipe:")
            .ok_or_else(|| failure("opaque_pipe_identity"))?;
        if uuid::Uuid::parse_str(opaque_id).is_err() || opaque_id.len() != 36 {
            return Err(failure("opaque_pipe_identity"));
        }
        if now
            .checked_sub(observation.observed_boot_ms)
            .is_none_or(|age| age > TTL_MS)
        {
            return Err(failure("stale_or_future"));
        }
        if observation.owner.boot_id != observation.browser.boot_id
            || observation.owner.uid != observation.browser.uid
            || observation.owner.pid == 0
            || observation.browser.pid == 0
        {
            return Err(failure("process_binding"));
        }
        if !bounded(&observation.canonical_profile, 4096, true)
            || !bounded(&observation.canonical_executable, 4096, true)
            || !Path::new(&observation.canonical_profile).is_absolute()
            || !Path::new(&observation.canonical_executable).is_absolute()
            || !bounded(&observation.version.product, MAX_ID, true)
            || !bounded(&observation.version.protocol_version, MAX_ID, true)
        {
            return Err(failure("profile_executable_or_version"));
        }
        if observation.targets.len() > MAX_TARGETS {
            return Err(failure("too_many_targets"));
        }
        let mut ids = HashSet::new();
        for target in &observation.targets {
            if !bounded(&target.id, MAX_ID, true)
                || !ids.insert(&target.id)
                || !bounded(&target.kind, MAX_ID, true)
                || !bounded(&target.title, MAX_TEXT, false)
                || !bounded(&target.url, MAX_TEXT, false)
            {
                return Err(failure("target_shape"));
            }
        }
        Ok(())
    }

    fn verify_current(observation: &Observation) -> Result<(), String> {
        observation.owner.verify_current().map_err(failure)?;
        observation.browser.verify_current().map_err(failure)?;
        let profile = Path::new(&observation.canonical_profile);
        if !profile.is_dir() || canonical(profile)? != observation.canonical_profile {
            return Err(failure("profile_changed"));
        }
        let executable = Path::new(&observation.canonical_executable);
        let metadata = fs::metadata(executable).map_err(failure)?;
        if canonical(executable)? != observation.canonical_executable
            || metadata.dev() != observation.browser.executable_device
            || metadata.ino() != observation.browser.executable_inode
        {
            return Err(failure("executable_changed"));
        }
        Ok(())
    }

    pub(super) async fn capture(
        manager: &BrowserManager,
        session_id: &str,
    ) -> Result<Value, String> {
        if !manager.uses_pipe_transport() {
            return Err(failure("owned_pipe_required"));
        }
        manager.verify_fresh_pipe_launch()?;
        let observed_boot_ms = boot_ms()?;
        let owner = ProcessIdentity::capture(std::process::id()).map_err(failure)?;
        let browser = ProcessIdentity::capture(
            manager
                .browser_pid()
                .ok_or_else(|| failure("local_browser_required"))?,
        )
        .map_err(failure)?;
        let canonical_profile = canonical(
            manager
                .browser_user_data_dir()
                .ok_or_else(|| failure("profile_required"))?,
        )?;
        let canonical_executable = canonical(
            manager
                .launched_chrome_executable()
                .ok_or_else(|| failure("owned_chrome_required"))?,
        )?;
        let (version, targets) = tokio::time::timeout(Duration::from_secs(2), async {
            let version = manager
                .client
                .send_command_with_timeout("Browser.getVersion", None, None, Duration::from_secs(2))
                .await
                .map_err(failure)?;
            let targets = manager
                .client
                .send_command_with_timeout("Target.getTargets", None, None, Duration::from_secs(2))
                .await
                .map_err(failure)?;
            Ok::<_, String>((version, targets))
        })
        .await
        .map_err(|_| failure("query_timeout"))??;
        let target_values = targets
            .get("targetInfos")
            .and_then(Value::as_array)
            .ok_or_else(|| failure("target_infos_required"))?;
        if target_values.len() > MAX_TARGETS {
            return Err(failure("too_many_targets"));
        }
        let targets = target_values
            .iter()
            .map(|target| {
                Ok(Target {
                    id: text(target, "targetId")?,
                    kind: text(target, "type")?,
                    title: text(target, "title")?,
                    url: text(target, "url")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let observation = Observation {
            schema: SCHEMA.into(),
            transport: "anonymous_pipe".into(),
            session_id: session_id.into(),
            browser_id: format!("session:{session_id}"),
            pipe_identity: manager.get_cdp_url().into(),
            owner,
            browser,
            observed_boot_ms,
            canonical_profile,
            canonical_executable,
            version: Version {
                product: text(&version, "product")?,
                protocol_version: text(&version, "protocolVersion")?,
            },
            targets,
        };
        validate_shape(
            &observation,
            &observation.browser_id,
            Some(observation.browser.pid),
            &[session_id.into()],
            boot_ms()?,
        )?;
        // Re-read both identities after CDP replies to reject process reuse or exec.
        verify_current(&observation)?;
        manager.verify_fresh_pipe_launch()?;
        serde_json::to_value(observation).map_err(failure)
    }

    pub(super) fn validate(
        value: &Value,
        browser_id: &str,
        pid: Option<u32>,
        sessions: &[String],
    ) -> Result<Vec<Value>, String> {
        let observation: Observation = serde_json::from_value(value.clone()).map_err(failure)?;
        validate_shape(&observation, browser_id, pid, sessions, boot_ms()?)?;
        verify_current(&observation)?;
        Ok(observation
            .targets
            .into_iter()
            .map(|target| {
                json!({
                    "id": target.id, "type": target.kind, "title": target.title, "url": target.url,
                })
            })
            .collect())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn fixture() -> Observation {
            let identity = ProcessIdentity::capture(std::process::id()).unwrap();
            Observation {
                schema: SCHEMA.into(),
                transport: "anonymous_pipe".into(),
                session_id: "test".into(),
                browser_id: "session:test".into(),
                pipe_identity: format!("pipe:{}", uuid::Uuid::new_v4()),
                owner: identity.clone(),
                browser: identity,
                observed_boot_ms: 30_000,
                canonical_profile: canonical(Path::new("/tmp")).unwrap(),
                canonical_executable: canonical(Path::new("/proc/self/exe")).unwrap(),
                version: Version {
                    product: "Chrome/test".into(),
                    protocol_version: "1.3".into(),
                },
                targets: vec![Target {
                    id: "one".into(),
                    kind: "page".into(),
                    title: "title".into(),
                    url: "about:blank".into(),
                }],
            }
        }

        fn shape(value: &Observation, now: u64) -> Result<(), String> {
            validate_shape(
                value,
                "session:test",
                Some(std::process::id()),
                &["test".into()],
                now,
            )
        }

        #[test]
        fn freshness_is_monotonic_and_bounded() {
            let value = fixture();
            assert!(shape(&value, 30_000).is_ok());
            assert!(shape(&value, 45_000).is_ok());
            assert!(shape(&value, 45_001).is_err());
            assert!(shape(&value, 29_999).is_err());
        }

        #[test]
        fn exact_session_browser_and_pid_are_required() {
            let value = fixture();
            for sessions in [
                vec![],
                vec!["wrong".into()],
                vec!["test".into(), "test".into()],
            ] {
                assert!(validate_shape(
                    &value,
                    "session:test",
                    Some(value.browser.pid),
                    &sessions,
                    30_000
                )
                .is_err());
            }
            assert!(validate_shape(
                &value,
                "wrong",
                Some(value.browser.pid),
                &["test".into()],
                30_000
            )
            .is_err());
            assert!(
                validate_shape(&value, "session:test", None, &["test".into()], 30_000).is_err()
            );
        }

        #[test]
        fn stale_process_identities_are_rejected() {
            let mut value = fixture();
            assert!(verify_current(&value).is_ok());
            value.owner.start_ticks ^= 1;
            assert!(verify_current(&value).is_err());
            value = fixture();
            value.browser.executable_inode ^= 1;
            assert!(verify_current(&value).is_err());
            value = fixture();
            value.browser.boot_id = "different-boot".into();
            assert!(shape(&value, 30_000).is_err());
        }

        #[test]
        fn targets_are_bounded_and_unique() {
            let mut value = fixture();
            value.targets.push(value.targets[0].clone());
            assert!(shape(&value, 30_000).is_err());
            value = fixture();
            value.targets[0].url = "x".repeat(MAX_TEXT + 1);
            assert!(shape(&value, 30_000).is_err());
            value = fixture();
            value.targets[0].id.clear();
            assert!(shape(&value, 30_000).is_err());
            value = fixture();
            value.targets = vec![value.targets[0].clone(); MAX_TARGETS + 1];
            assert!(shape(&value, 30_000).is_err());
        }

        #[test]
        fn strict_json_rejects_extra_fields_and_wrong_types() {
            let mut value = serde_json::to_value(fixture()).unwrap();
            value["webSocketDebuggerUrl"] = json!("ws://localhost:9222");
            assert!(serde_json::from_value::<Observation>(value).is_err());
            let mut value = serde_json::to_value(fixture()).unwrap();
            value["owner"]["unknown"] = json!(true);
            assert!(serde_json::from_value::<Observation>(value).is_err());
            let mut value = serde_json::to_value(fixture()).unwrap();
            value["observedBootMs"] = json!("30000");
            assert!(serde_json::from_value::<Observation>(value).is_err());
        }

        #[test]
        fn opaque_identity_is_not_a_network_endpoint() {
            let mut value = fixture();
            value.pipe_identity = "ws://localhost:9222".into();
            assert!(shape(&value, 30_000).is_err());
            value.pipe_identity = "pipe:not-a-uuid".into();
            assert!(shape(&value, 30_000).is_err());
        }

        #[test]
        fn valid_observation_returns_no_transport_endpoint() {
            let mut value = fixture();
            value.observed_boot_ms = boot_ms().unwrap();
            let result = validate(
                &serde_json::to_value(value).unwrap(),
                "session:test",
                Some(std::process::id()),
                &["test".into()],
            )
            .unwrap();
            assert_eq!(
                result,
                vec![json!({"id":"one", "type":"page", "title":"title", "url":"about:blank"})]
            );
        }
    }
}
