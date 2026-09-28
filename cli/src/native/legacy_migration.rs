//! Explicit operator-local legacy migration. Enrollment records observations
//! prospectively; it never upgrades or rewrites a legacy source descriptor.
use super::actions::{handoff_display_proof, verify_handoff_snapshot};
use super::handoff_custody::{BrowserIdentity, CustodyPhase, CustodyReceipt, ProcessIdentity};
use super::service_model::{LeaseState, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::connection::{get_socket_dir, MigrationConnection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MigrationPlan {
    pub schema: String,
    pub source: ProcessIdentity,
    pub browser: BrowserIdentity,
    pub session: String,
    pub profile: String,
    pub target: String,
    pub url: String,
    pub display: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Enrollment {
    pub plan: MigrationPlan,
    pub descriptor_sha256: String,
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| e.to_string())
}

fn directory(session: &str) -> Result<PathBuf, String> {
    if session.is_empty()
        || session.len() > 128
        || !session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err("migration_session_invalid".into());
    }
    Ok(get_socket_dir().join(format!("{session}.legacy-migration")))
}

pub(crate) fn exists(session: &str) -> bool {
    directory(session).is_ok_and(|path| !matches!(fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound))
}

fn private_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("migration_private_directory_required".into());
    }
    Ok(())
}

pub(crate) fn read_private(path: &Path) -> Result<Vec<u8>, String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 1_048_576
    {
        return Err("migration_private_record_required".into());
    }
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("migration_record_too_large".into());
    }
    Ok(bytes)
}

fn create_record(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("migration_record_parent_missing")?;
    private_directory(parent)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}

fn canonical_http_url(value: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(value).map_err(|_| "migration_url_invalid")?;
    if !matches!(parsed.scheme(), "https" | "http")
        || parsed.as_str() != value
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err("migration_canonical_url_required".into());
    }
    Ok(parsed)
}

fn verify_plan_identity(
    plan: &MigrationPlan,
    snapshot: &ServiceState,
) -> Result<(url::Url, String), String> {
    if plan.schema != "agent-browser.prospective-legacy-migration.v1" {
        return Err("migration_schema_invalid".into());
    }
    let initial_url = canonical_http_url(&plan.url)?;
    plan.browser.verify_current()?;
    let expected_browser = format!("session:{}", plan.session);
    if snapshot.browsers.values().any(|browser| {
        browser.id != expected_browser
            && (browser.pid == Some(plan.browser.process.pid)
                || browser.cdp_endpoint.as_deref() == Some(plan.browser.cdp_endpoint.as_str()))
    }) {
        return Err("migration_ambiguous_browser_owner".into());
    }
    for session in snapshot.sessions.values().filter(|session| {
        session.id != plan.session
            && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
    }) {
        if let Some(path) = session
            .profile_id
            .as_ref()
            .and_then(|id| snapshot.profiles.get(id))
            .and_then(|profile| profile.user_data_dir.as_ref())
        {
            let canonical =
                fs::canonicalize(path).map_err(|_| "migration_competing_profile_unreadable")?;
            if canonical == plan.browser.canonical_profile {
                return Err("migration_ambiguous_profile_owner".into());
            }
        }
    }
    verify_handoff_snapshot(snapshot, &plan.session, &plan.browser, &plan.target)?;
    let current_url = snapshot
        .tabs
        .get(&format!("target:{}", plan.target))
        .and_then(|tab| tab.url.as_deref())
        .ok_or("migration_target_url_missing")?
        .to_owned();
    if snapshot
        .sessions
        .get(&plan.session)
        .and_then(|session| session.profile_id.as_deref())
        != Some(plan.profile.as_str())
        || handoff_display_proof(snapshot, &plan.session)? != plan.display
    {
        return Err("migration_profile_or_display_changed".into());
    }
    Ok((initial_url, current_url))
}

pub(crate) fn verify_plan(plan: &MigrationPlan, snapshot: &ServiceState) -> Result<(), String> {
    let (_, current_url) = verify_plan_identity(plan, snapshot)?;
    if current_url != plan.url {
        return Err("migration_profile_target_url_display_changed".into());
    }
    Ok(())
}

/// Revalidates a committed migration without freezing its target at the
/// enrollment URL. Browser, process, profile, session, target, and display
/// identity remain exact; only a canonical same-origin URL evolution is
/// accepted after custody has committed.
pub(crate) fn verify_committed_plan(
    plan: &MigrationPlan,
    snapshot: &ServiceState,
) -> Result<(), String> {
    let (initial_url, current_url) = verify_plan_identity(plan, snapshot)?;
    let current_url = canonical_http_url(&current_url)?;
    if current_url.origin() != initial_url.origin() {
        return Err("migration_target_origin_changed".into());
    }
    Ok(())
}

/// Revalidate a committed prospective migration after its exact enrolled tab
/// has closed and custody is being recovered onto another ready tab in the
/// same retained browser. The browser, profile, session, display and origin
/// stay fixed; only the target identity may advance.
pub(crate) fn verify_recovered_committed_plan(
    plan: &MigrationPlan,
    recovered_target: &str,
    snapshot: &ServiceState,
) -> Result<(), String> {
    if recovered_target.is_empty() || recovered_target == plan.target {
        return Err("migration_recovery_target_must_advance".into());
    }
    if snapshot
        .tabs
        .get(&format!("target:{}", plan.target))
        .is_some_and(|tab| tab.lifecycle == super::service_model::TabLifecycle::Ready)
    {
        return Err("migration_recovery_stale_target_still_ready".into());
    }
    verify_handoff_snapshot(snapshot, &plan.session, &plan.browser, recovered_target)?;
    let session = snapshot
        .sessions
        .get(&plan.session)
        .ok_or("migration_session_missing")?;
    if session.profile_id.as_deref() != Some(plan.profile.as_str())
        || handoff_display_proof(snapshot, &plan.session)? != plan.display
    {
        return Err("migration_recovery_profile_or_display_changed".into());
    }
    let initial_url = canonical_http_url(&plan.url)?;
    let current_url = snapshot
        .tabs
        .get(&format!("target:{recovered_target}"))
        .and_then(|tab| tab.url.as_deref())
        .ok_or("migration_recovery_target_url_missing")?;
    if canonical_http_url(current_url)?.origin() != initial_url.origin() {
        return Err("migration_recovery_target_origin_changed".into());
    }
    plan.source.require_gone()?;
    Ok(())
}

/// Validate the old committed receipt used to construct a schema-v3 recovery
/// descriptor. This consumes no authority and performs no mutation.
pub(crate) fn stale_snapshot_receipt(
    snapshot: &ServiceState,
    session: &str,
    recovered_target: &str,
    source: &ProcessIdentity,
    browser: &BrowserIdentity,
) -> Result<CustodyReceipt, String> {
    let receipt: CustodyReceipt = serde_json::from_value(
        snapshot
            .runtime_custody_receipts
            .get(session)
            .cloned()
            .ok_or("migration_recovery_prior_receipt_missing")?,
    )
    .map_err(|_| "migration_recovery_prior_receipt_invalid")?;
    let plan = receipt
        .prospective_migration
        .as_ref()
        .ok_or("migration_recovery_prospective_plan_missing")?;
    if receipt.schema_version != 2
        || receipt.phase != CustodyPhase::Committed
        || receipt.destination != *source
        || receipt.browser != *browser
        || receipt.target_id != plan.target
        || plan.browser != *browser
        || plan.session != session
    {
        return Err("migration_recovery_prior_receipt_binding_mismatch".into());
    }
    source.require_gone()?;
    browser.verify_current()?;
    verify_recovered_committed_plan(plan, recovered_target, snapshot)?;
    Ok(receipt)
}

fn command(action: &str) -> Value {
    json!({"id":uuid::Uuid::new_v4().to_string(), "action":action})
}

fn verify_source_target(
    connection: &mut MigrationConnection,
    plan: &MigrationPlan,
) -> Result<(), String> {
    let mut request = command("tab_list");
    request["verbose"] = json!(true);
    let tabs = connection.request_once(request)?;
    let active = tabs["tabs"]
        .as_array()
        .ok_or("migration_tabs_missing")?
        .iter()
        .filter(|tab| tab["active"] == true)
        .collect::<Vec<_>>();
    if active.len() != 1 || active[0]["targetId"] != plan.target || active[0]["url"] != plan.url {
        return Err("migration_active_target_changed".into());
    }
    let mut request = command("evaluate");
    // Fixed read-only expression, never request-controlled JavaScript.
    request["script"] = json!("location.href");
    let rendered = connection.request_once(request)?;
    if rendered["result"] != plan.url {
        return Err("migration_rendered_url_changed".into());
    }
    Ok(())
}

pub(crate) fn coordinate(
    session: &str,
    target: &str,
    url: &str,
    approved_digest: Option<&str>,
) -> Result<Value, String> {
    if exists(session) {
        return Err("migration_existing_enrollment_requires_reconciliation".into());
    }
    let mut connection = MigrationConnection::open(session)?;
    let snapshot = LockedServiceStateRepository::default_json()?.load_snapshot()?;
    let browser = snapshot
        .browsers
        .get(&format!("session:{session}"))
        .ok_or("migration_browser_missing")?;
    let profile = snapshot
        .sessions
        .get(session)
        .and_then(|s| s.profile_id.as_deref())
        .ok_or("migration_profile_missing")?;
    let profile_path = snapshot
        .profiles
        .get(profile)
        .and_then(|p| p.user_data_dir.as_deref())
        .ok_or("migration_profile_path_missing")?;
    let plan = MigrationPlan {
        schema: "agent-browser.prospective-legacy-migration.v1".into(),
        source: connection.peer.clone(),
        browser: BrowserIdentity::capture(
            browser.pid.ok_or("migration_browser_pid_missing")?,
            Path::new(profile_path),
            browser
                .cdp_endpoint
                .as_deref()
                .ok_or("migration_endpoint_missing")?,
        )?,
        session: session.into(),
        profile: profile.into(),
        target: target.into(),
        url: url.into(),
        display: handoff_display_proof(&snapshot, session)?,
    };
    verify_plan(&plan, &snapshot)?;
    verify_source_target(&mut connection, &plan)?;
    let plan_bytes = encoded(&plan)?;
    let plan_digest = digest(&plan_bytes);
    let Some(approved_digest) = approved_digest else {
        return Ok(json!({"ready":true,"mutated":false,"planSha256":plan_digest,"plan":plan}));
    };
    if approved_digest != plan_digest {
        return Err("migration_approved_plan_changed".into());
    }
    let descriptor_path = get_socket_dir().join(format!("{session}.handoff.json"));
    if !matches!(fs::symlink_metadata(&descriptor_path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    {
        return Err("migration_existing_descriptor_requires_reconciliation".into());
    }
    verify_plan(
        &plan,
        &LockedServiceStateRepository::default_json()?.load_snapshot()?,
    )?;
    let root = directory(session)?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .map_err(|e| e.to_string())?;
    File::open(get_socket_dir())
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    // Durable intent before any possibly delivered prepare. Never overwrite or
    // automatically replay it, even when dispatch/acknowledgement is uncertain.
    create_record(&root.join("pending.json"), &plan_bytes)?;
    let result = connection.request_once(command("runtime_handoff_prepare"))?;
    if result["prepared"] != true || result["sessionName"] != session {
        return Err("migration_prepare_not_acknowledged".into());
    }
    let bytes = read_private(&descriptor_path)?;
    validate_descriptor(&plan, &bytes)?;
    create_record(&root.join("legacy-descriptor.json"), &bytes)?;
    let enrollment = Enrollment {
        plan,
        descriptor_sha256: digest(&bytes),
    };
    create_record(&root.join("acknowledged.json"), &encoded(&enrollment)?)?;
    Ok(
        json!({"prepared":true,"enrollmentPath":root,"descriptorSha256":enrollment.descriptor_sha256,"sourceExitVerified":false}),
    )
}

pub(crate) fn validate_descriptor(plan: &MigrationPlan, bytes: &[u8]) -> Result<(), String> {
    let descriptor: Value =
        serde_json::from_slice(bytes).map_err(|_| "migration_descriptor_invalid")?;
    if descriptor["schemaVersion"] != 1
        || descriptor.get("custody").is_some_and(|v| !v.is_null())
        || descriptor["sessionName"] != plan.session
        || descriptor["browserPid"] != plan.browser.process.pid
        || descriptor["cdpUrl"] != plan.browser.cdp_endpoint
        || descriptor["runtimeProfile"] != plan.profile
        || descriptor["activeTargetId"] != plan.target
        || descriptor["host"] == "attached_existing"
    {
        return Err("migration_legacy_descriptor_changed".into());
    }
    Ok(())
}

pub(crate) fn load_for_resume(session: &str, descriptor: &[u8]) -> Result<Enrollment, String> {
    let root = directory(session)?;
    private_directory(&root)?;
    let plan: MigrationPlan = serde_json::from_slice(&read_private(&root.join("pending.json"))?)
        .map_err(|_| "migration_plan_invalid")?;
    let enrollment: Enrollment =
        serde_json::from_slice(&read_private(&root.join("acknowledged.json"))?)
            .map_err(|_| "migration_prepare_indeterminate")?;
    if enrollment.plan != plan
        || enrollment.plan.session != session
        || enrollment.descriptor_sha256 != digest(descriptor)
        || read_private(&root.join("legacy-descriptor.json"))? != descriptor
    {
        return Err("migration_enrollment_descriptor_changed".into());
    }
    validate_descriptor(&plan, descriptor)?;
    plan.source.require_gone()?;
    let snapshot = LockedServiceStateRepository::default_json()?.load_snapshot()?;
    let current_url = snapshot
        .tabs
        .get(&format!("target:{}", plan.target))
        .and_then(|tab| tab.url.as_deref())
        .ok_or("migration_target_url_missing")?;
    if current_url == plan.url {
        verify_plan(&plan, &snapshot)?;
    } else {
        let prior: CustodyReceipt = serde_json::from_value(
            snapshot
                .runtime_custody_receipts
                .get(session)
                .cloned()
                .ok_or("migration_prior_committed_custody_required")?,
        )
        .map_err(|_| "migration_prior_committed_custody_invalid")?;
        if prior.schema_version != 2
            || prior.phase != CustodyPhase::Committed
            || prior.source != plan.source
            || prior.browser != plan.browser
            || prior.target_id != plan.target
            || prior.descriptor_sha256 != enrollment.descriptor_sha256
            || prior.prospective_migration.as_ref() != Some(&plan)
        {
            return Err("migration_prior_committed_custody_mismatch".into());
        }
        prior.destination.require_gone()?;
        verify_committed_plan(&plan, &snapshot)?;
    }
    Ok(enrollment)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    fn migration_source_fixture_child() {
        let Ok(config_path) = std::env::var("LEGACY_MIGRATION_TEST_CONFIG") else {
            return;
        };
        let config: Value = serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
        let listener = UnixListener::bind(get_socket_dir().join("migration-test.sock")).unwrap();
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let mut reader = BufReader::new(stream);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    break;
                }
                let request: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(request["_agentBrowserAuthToken"], "fixture-token");
                let action = request["action"].as_str().unwrap();
                let data = match action {
                    "tab_list" => {
                        json!({"tabs":[{"active":true,"targetId":"exact-target","url":"https://example.test/custody"}]})
                    }
                    "evaluate" => {
                        assert_eq!(request["script"], "location.href");
                        json!({"result":"https://example.test/custody"})
                    }
                    "runtime_handoff_prepare" => {
                        let bytes = encoded(&config["descriptor"]).unwrap();
                        create_record(
                            &get_socket_dir().join("migration-test.handoff.json"),
                            &bytes,
                        )
                        .unwrap();
                        if config["dropAcknowledgement"] == true {
                            return;
                        }
                        json!({"prepared":true,"sessionName":"migration-test"})
                    }
                    _ => panic!("unexpected fixture command"),
                };
                let mut response =
                    encoded(&json!({"id":request["id"],"success":true,"data":data})).unwrap();
                response.push(b'\n');
                reader.get_mut().write_all(&response).unwrap();
                if action == "runtime_handoff_prepare" {
                    return;
                }
            }
        }
    }

    pub(crate) struct SourceChild(pub(crate) Child);
    impl Drop for SourceChild {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    pub(crate) fn setup(
        drop_acknowledgement: bool,
    ) -> (
        EnvGuard<'static>,
        super::super::handoff_custody::TestBrowserFixture,
        SourceChild,
    ) {
        let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_SOCKET_DIR", "AGENT_BROWSER_SESSION"]);
        let fixture = super::super::handoff_custody::TestBrowserFixture::new();
        fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o700)).unwrap();
        let sockets = fixture.root.join("sockets");
        fs::DirBuilder::new().mode(0o700).create(&sockets).unwrap();
        guard.set("HOME", fixture.root.to_str().unwrap());
        guard.set("AGENT_BROWSER_SOCKET_DIR", sockets.to_str().unwrap());
        guard.set("AGENT_BROWSER_SESSION", "migration-test");
        create_record(&sockets.join("migration-test.token"), b"fixture-token").unwrap();
        let snapshot: ServiceState = serde_json::from_value(json!({
            "profiles":{"migration-test":{"id":"migration-test","userDataDir":fixture.root}},
            "sessions":{"migration-test":{"id":"migration-test","profileId":"migration-test","lease":"exclusive","browserIds":["session:migration-test"],"tabIds":["target:exact-target"]}},
            "browsers":{"session:migration-test":{"id":"session:migration-test","profileId":"migration-test","pid":fixture.browser.process.pid,"cdpEndpoint":fixture.browser.cdp_endpoint,"host":"local_headless","health":"ready","activeSessionIds":["migration-test"]}},
            "tabs":{"target:exact-target":{"id":"target:exact-target","browserId":"session:migration-test","targetId":"exact-target","ownerSessionId":"migration-test","url":"https://example.test/custody","lifecycle":"ready"}}
        })).unwrap();
        LockedServiceStateRepository::default_json()
            .unwrap()
            .mutate(|state| {
                *state = snapshot;
                Ok(())
            })
            .unwrap();
        let config = json!({"dropAcknowledgement":drop_acknowledgement,"descriptor":{
            "schemaVersion":1,"sessionName":"migration-test","browserPid":fixture.browser.process.pid,
            "cdpUrl":fixture.browser.cdp_endpoint,"runtimeProfile":"migration-test","activeTargetId":"exact-target",
            "host":"local_headless","engine":"chrome","closeBrowserOnClose":false,"preparedAt":"synthetic-test"
        }});
        let config_path = fixture.root.join("source.json");
        create_record(&config_path, &encoded(&config).unwrap()).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "native::legacy_migration::tests::migration_source_fixture_child",
                "--exact",
            ])
            .env("LEGACY_MIGRATION_TEST_CONFIG", config_path)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !sockets.join("migration-test.sock").exists() {
            assert!(Instant::now() < deadline, "fixture source missing");
            std::thread::sleep(Duration::from_millis(10));
        }
        (guard, fixture, SourceChild(child))
    }

    #[test]
    fn migration_prepare_is_witnessed_and_descriptor_is_preserved() {
        let (_guard, _fixture, mut source) = setup(false);
        let plan = coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            None,
        )
        .unwrap();
        let observed: MigrationPlan = serde_json::from_value(plan["plan"].clone()).unwrap();
        assert_eq!(observed.source.pid, source.0.id());
        assert_eq!(
            observed.source.require_gone().unwrap_err(),
            "handoff_custody_source_still_alive"
        );
        coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            plan["planSha256"].as_str(),
        )
        .unwrap();
        source.0.wait().unwrap();
        let original = read_private(&get_socket_dir().join("migration-test.handoff.json")).unwrap();
        let enrollment = load_for_resume("migration-test", &original).unwrap();
        assert_eq!(enrollment.plan, observed);
        assert_eq!(
            serde_json::from_slice::<Value>(&original).unwrap()["schemaVersion"],
            1
        );
        let prior = CustodyReceipt {
            schema_version: 2,
            phase: CustodyPhase::Committed,
            source: observed.source.clone(),
            destination: super::super::handoff_custody::stopped_source(),
            browser: observed.browser.clone(),
            target_id: observed.target.clone(),
            descriptor_sha256: digest(&original),
            prospective_migration: Some(observed.clone()),
        };
        LockedServiceStateRepository::default_json()
            .unwrap()
            .mutate(|snapshot| {
                snapshot.tabs.get_mut("target:exact-target").unwrap().url =
                    Some("https://example.test/c/recovered".into());
                snapshot.runtime_custody_receipts.insert(
                    "migration-test".into(),
                    serde_json::to_value(&prior).unwrap(),
                );
                Ok(())
            })
            .unwrap();
        assert_eq!(
            load_for_resume("migration-test", &original).unwrap().plan,
            observed
        );
        LockedServiceStateRepository::default_json()
            .unwrap()
            .mutate(|snapshot| {
                snapshot.tabs.get_mut("target:exact-target").unwrap().url =
                    Some("https://foreign.example/c/recovered".into());
                Ok(())
            })
            .unwrap();
        assert_eq!(
            load_for_resume("migration-test", &original).unwrap_err(),
            "migration_target_origin_changed"
        );
        assert!(coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            plan["planSha256"].as_str()
        )
        .is_err());
        assert_eq!(
            read_private(&get_socket_dir().join("migration-test.handoff.json")).unwrap(),
            original
        );
    }

    #[test]
    fn migration_uncertain_prepare_is_not_replayed() {
        let (_guard, _fixture, mut source) = setup(true);
        let plan = coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            None,
        )
        .unwrap();
        assert!(coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            plan["planSha256"].as_str()
        )
        .unwrap_err()
        .starts_with("migration_dispatch_indeterminate"));
        source.0.wait().unwrap();
        let bytes = read_private(&get_socket_dir().join("migration-test.handoff.json")).unwrap();
        assert!(load_for_resume("migration-test", &bytes).is_err());
        assert_eq!(
            coordinate(
                "migration-test",
                "exact-target",
                "https://example.test/custody",
                plan["planSha256"].as_str()
            )
            .unwrap_err(),
            "migration_existing_enrollment_requires_reconciliation"
        );
    }

    #[test]
    fn migration_plan_and_descriptor_drift_fail_closed() {
        let (_guard, _fixture, _source) = setup(false);
        let result = coordinate(
            "migration-test",
            "exact-target",
            "https://example.test/custody",
            None,
        )
        .unwrap();
        let plan: MigrationPlan = serde_json::from_value(result["plan"].clone()).unwrap();
        let snapshot = LockedServiceStateRepository::default_json()
            .unwrap()
            .load_snapshot()
            .unwrap();
        let mut duplicate = snapshot.clone();
        let mut browser = duplicate.browsers["session:migration-test"].clone();
        browser.id = "session:foreign".into();
        duplicate.browsers.insert(browser.id.clone(), browser);
        assert_eq!(
            verify_plan(&plan, &duplicate).unwrap_err(),
            "migration_ambiguous_browser_owner"
        );
        let mut duplicate = snapshot.clone();
        let mut profile = duplicate.profiles["migration-test"].clone();
        profile.id = "alias-profile".into();
        duplicate.profiles.insert(profile.id.clone(), profile);
        let mut session = duplicate.sessions["migration-test"].clone();
        session.id = "foreign".into();
        session.profile_id = Some("alias-profile".into());
        duplicate.sessions.insert(session.id.clone(), session);
        assert_eq!(
            verify_plan(&plan, &duplicate).unwrap_err(),
            "migration_ambiguous_profile_owner"
        );
        for field in ["profile", "session", "target", "url"] {
            let mut changed = serde_json::to_value(&plan).unwrap();
            changed[field] = json!("changed");
            assert!(verify_plan(&serde_json::from_value(changed).unwrap(), &snapshot).is_err());
        }
        assert_eq!(
            coordinate(
                "migration-test",
                "exact-target",
                "https://example.test/custody",
                Some("wrong-digest")
            )
            .unwrap_err(),
            "migration_approved_plan_changed"
        );
        assert!(!exists("migration-test"));
        let config: Value =
            serde_json::from_slice(&read_private(&_fixture.root.join("source.json")).unwrap())
                .unwrap();
        for field in [
            "sessionName",
            "browserPid",
            "cdpUrl",
            "runtimeProfile",
            "activeTargetId",
        ] {
            let mut changed = config["descriptor"].clone();
            changed[field] = json!("changed");
            assert!(validate_descriptor(&plan, &encoded(&changed).unwrap()).is_err());
        }
    }
}
