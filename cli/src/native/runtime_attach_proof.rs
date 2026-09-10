//! Existing-process provenance, independent of launch preferences. Never launches,
//! navigates, takes ownership, or treats a requested executable as observed fact.

use super::service_lifecycle::ServiceLaunchMetadata;
use serde_json::json;

pub(super) fn apply_managed_runtime_attach_proof(
    metadata: &mut ServiceLaunchMetadata,
    runtime_profile: &str,
    pid: Option<u32>,
    endpoint: Option<&str>,
) {
    // This repair supplies Linux process evidence only. Preserve existing
    // platform-specific authority on other hosts; never synthesize a fallback.
    if !cfg!(target_os = "linux") {
        return;
    }
    apply_with_proc_root(
        metadata,
        runtime_profile,
        pid,
        endpoint,
        std::path::Path::new("/proc"),
    );
}

fn apply_with_proc_root(
    metadata: &mut ServiceLaunchMetadata,
    runtime_profile: &str,
    pid: Option<u32>,
    endpoint: Option<&str>,
    proc_root: &std::path::Path,
) {
    let Some(proof) = metadata.browser_capability_launch.as_mut() else {
        return;
    };
    // Keep registry refusals and non-stock builds closed. An applied preference
    // is only a launch selection; it must also be verified for an attachment.
    let eligible = proof["browserBuild"] == "stock_chrome"
        && (proof["applied"] == true
            || matches!(
                proof["reason"].as_str(),
                Some("no_matching_preference_binding" | "explicit_executable_path")
            ));
    let selected_path = proof["executablePath"].as_str().map(str::to_owned);
    if !eligible {
        return;
    }
    if proof["applied"] == true {
        proof["applied"] = json!(false);
        proof["reason"] = json!("managed_runtime_attach_unverified");
    }
    let result = (|| {
        if metadata.profile_id.as_deref() != Some(runtime_profile) {
            return Err("profile_mismatch");
        }
        let runtime = crate::runtime_profile::read_runtime_state(runtime_profile)
            .map_err(|_| "runtime_state_unreadable")?
            .ok_or("runtime_state_missing")?;
        let installed =
            crate::install::find_installed_chrome().ok_or("installed_chrome_missing")?;
        verify(
            &runtime,
            runtime_profile,
            pid.ok_or("pid_missing")?,
            endpoint.ok_or("endpoint_missing")?,
            metadata
                .user_data_dir
                .as_deref()
                .ok_or("profile_path_missing")?,
            &installed,
            proc_root,
        )
        .and_then(|verified| {
            if let Some(selected) = selected_path.as_deref() {
                if std::path::Path::new(selected).canonicalize().ok().as_ref()
                    != Some(&verified.executable)
                {
                    return Err("selected_executable_mismatch");
                }
            }
            // Reject state replacement while observing the process.
            if crate::runtime_profile::read_runtime_state(runtime_profile)
                .ok()
                .flatten()
                .as_ref()
                != Some(&runtime)
            {
                return Err("runtime_state_changed");
            }
            Ok(verified)
        })
    })();
    match result {
        Ok(verified) => {
            *proof = json!({
                "applied": true,
                "reason": "verified_installed_chrome_runtime_attach",
                "browserBuild": "stock_chrome",
                "profileId": runtime_profile,
                "executablePath": verified.executable,
                "browserPid": pid,
                "processStartTicks": verified.start_ticks,
                "cdpEndpoint": endpoint,
                "userDataDir": verified.profile,
            });
        }
        Err(reason) => {
            proof["applied"] = json!(false);
            proof["reason"] = json!("managed_runtime_attach_unverified");
            proof["verificationReason"] = json!(reason);
        }
    }
}

struct VerifiedProcess {
    executable: std::path::PathBuf,
    profile: std::path::PathBuf,
    start_ticks: u64,
}

#[cfg(not(target_os = "linux"))]
fn verify(
    _runtime: &crate::runtime_profile::RuntimeState,
    _runtime_profile: &str,
    _pid: u32,
    _endpoint: &str,
    _profile: &str,
    _installed: &std::path::Path,
    _proc_root: &std::path::Path,
) -> Result<VerifiedProcess, &'static str> {
    Err("process_proof_unsupported")
}

#[cfg(target_os = "linux")]
fn verify(
    runtime: &crate::runtime_profile::RuntimeState,
    runtime_profile: &str,
    pid: u32,
    endpoint: &str,
    profile: &str,
    installed: &std::path::Path,
    proc_root: &std::path::Path,
) -> Result<VerifiedProcess, &'static str> {
    use std::{fs, os::unix::fs::MetadataExt, path::Path};
    if pid == 0 || runtime.browser_pid != pid || runtime.runtime_profile != runtime_profile {
        return Err("runtime_identity_mismatch");
    }
    let port = runtime.devtools_port.ok_or("runtime_port_missing")?;
    let prefix = format!("ws://127.0.0.1:{port}/devtools/browser/");
    if runtime.ws_url.as_deref() != Some(endpoint)
        || !endpoint.strip_prefix(&prefix).is_some_and(|id| {
            !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("endpoint_mismatch");
    }
    let profile = Path::new(profile)
        .canonicalize()
        .map_err(|_| "profile_unreadable")?;
    if Path::new(&runtime.user_data_dir)
        .canonicalize()
        .ok()
        .as_ref()
        != Some(&profile)
    {
        return Err("profile_path_mismatch");
    }
    let active_port = fs::read_to_string(profile.join("DevToolsActivePort"))
        .map_err(|_| "active_port_unreadable")?;
    let mut lines = active_port.lines();
    if lines.next().and_then(|p| p.parse::<u16>().ok()) != Some(port)
        || lines.next() != endpoint.strip_prefix(&format!("ws://127.0.0.1:{port}"))
    {
        return Err("active_port_mismatch");
    }
    let process = proc_root.join(pid.to_string());
    let start_ticks = process_start_ticks(&process)?;
    let executable = installed
        .canonicalize()
        .map_err(|_| "installed_chrome_unreadable")?;
    let actual = fs::read_link(process.join("exe")).map_err(|_| "process_executable_unreadable")?;
    if actual != executable || !executable.is_file() {
        return Err("process_executable_mismatch");
    }
    // A pathname alone cannot detect replacement of the on-disk executable.
    let actual_stat =
        fs::metadata(process.join("exe")).map_err(|_| "process_executable_unreadable")?;
    let installed_stat = fs::metadata(&executable).map_err(|_| "installed_chrome_unreadable")?;
    if (actual_stat.dev(), actual_stat.ino()) != (installed_stat.dev(), installed_stat.ino()) {
        return Err("process_executable_replaced");
    }
    let cmdline = fs::read(process.join("cmdline")).map_err(|_| "process_arguments_unreadable")?;
    let arguments = cmdline
        .split(|b| *b == 0)
        .filter(|arg| !arg.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "process_arguments_unreadable")?;
    let mut directories = Vec::new();
    for (index, arg) in arguments.iter().enumerate() {
        if arg.starts_with("--type=") || *arg == "--type" {
            return Err("not_browser_process");
        }
        if let Some(value) = arg.strip_prefix("--user-data-dir=") {
            directories.push(value);
        } else if *arg == "--user-data-dir" {
            directories.push(*arguments.get(index + 1).ok_or("profile_argument_missing")?);
        }
    }
    if directories.len() != 1
        || Path::new(directories[0]).canonicalize().ok().as_ref() != Some(&profile)
    {
        return Err("process_profile_mismatch");
    }
    // /proc/PID/net/tcp is namespace-wide. Require the listener inode to also
    // be present in this exact process's descriptors, not merely in its namespace.
    let tcp = fs::read_to_string(process.join("net/tcp")).map_err(|_| "listener_unreadable")?;
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
    if listeners.len() != 1 {
        return Err("listener_mismatch");
    }
    let socket = format!("socket:[{}]", listeners[0]);
    let owns_listener = fs::read_dir(process.join("fd"))
        .map_err(|_| "process_descriptors_unreadable")?
        .filter_map(Result::ok)
        .any(|entry| fs::read_link(entry.path()).ok().as_deref() == Some(Path::new(&socket)));
    if !owns_listener {
        return Err("listener_not_owned_by_process");
    }
    if process_start_ticks(&process)? != start_ticks
        || fs::read_link(process.join("exe")).ok().as_ref() != Some(&actual)
        || fs::read(process.join("cmdline")).ok().as_ref() != Some(&cmdline)
    {
        return Err("process_changed");
    }
    Ok(VerifiedProcess {
        executable,
        profile,
        start_ticks,
    })
}

#[cfg(target_os = "linux")]
fn process_start_ticks(process: &std::path::Path) -> Result<u64, &'static str> {
    let stat =
        std::fs::read_to_string(process.join("stat")).map_err(|_| "process_stat_unreadable")?;
    let (_, fields) = stat.rsplit_once(") ").ok_or("process_stat_invalid")?;
    let fields = fields.split_whitespace().collect::<Vec<_>>();
    if matches!(fields.first(), Some(&"Z" | &"X")) {
        return Err("process_exited");
    }
    fields
        .get(19)
        .and_then(|s| s.parse().ok())
        .ok_or("process_stat_invalid")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::runtime_profile::RuntimeState;
    use std::{fs, os::unix::fs::symlink, path::PathBuf};

    struct Fixture {
        root: PathBuf,
        runtime: RuntimeState,
        installed: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("runtime-proof-{}", uuid::Uuid::new_v4()));
            let profile = root.join("profile");
            let process = root.join("proc/123");
            fs::create_dir_all(&profile).unwrap();
            fs::create_dir_all(process.join("fd")).unwrap();
            fs::create_dir_all(process.join("net")).unwrap();
            let installed = root.join("chrome");
            fs::write(&installed, "installed fixture").unwrap();
            symlink(&installed, process.join("exe")).unwrap();
            fs::write(
                process.join("stat"),
                format!("123 (chrome) S {} 42", vec!["0"; 18].join(" ")),
            )
            .unwrap();
            fs::write(
                process.join("cmdline"),
                format!("chrome\0--user-data-dir={}\0", profile.display()),
            )
            .unwrap();
            fs::write(
                profile.join("DevToolsActivePort"),
                "9222\n/devtools/browser/abc-123\n",
            )
            .unwrap();
            fs::write(
                process.join("net/tcp"),
                "0: 0100007F:2406 00000000:0000 0A 0 0 0 1000 0 456\n",
            )
            .unwrap();
            symlink("socket:[456]", process.join("fd/10")).unwrap();
            let runtime = RuntimeState {
                runtime_profile: "qa".into(),
                user_data_dir: profile.to_string_lossy().into(),
                browser_pid: 123,
                headed: true,
                launch_mode: "automation".into(),
                devtools_port: Some(9222),
                ws_url: Some("ws://127.0.0.1:9222/devtools/browser/abc-123".into()),
                launch_record: None,
            };
            Self {
                root,
                runtime,
                installed,
            }
        }

        fn verify(&self) -> Result<VerifiedProcess, &'static str> {
            verify(
                &self.runtime,
                "qa",
                123,
                "ws://127.0.0.1:9222/devtools/browser/abc-123",
                self.root.join("profile").to_str().unwrap(),
                &self.installed,
                &self.root.join("proc"),
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn exact_process_identity_is_verified_without_launch() {
        let fixture = Fixture::new();
        let verified = fixture.verify().unwrap();
        assert_eq!(verified.executable, fixture.installed);
        assert_eq!(verified.profile, fixture.root.join("profile"));
        assert_eq!(verified.start_ticks, 42);
    }

    #[test]
    fn verified_attach_persists_and_failed_reverification_clears_build() {
        use crate::native::service_health::persist_service_browser_record_in_repository;
        use crate::native::service_model::{BrowserBuild, BrowserHealth, BrowserHost};
        use crate::native::service_store::{
            JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
        };
        use crate::test_utils::EnvGuard;
        let fixture = Fixture::new();
        let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_HOME"]);
        guard.set("HOME", fixture.root.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            fixture.root.join(".agent-browser").to_str().unwrap(),
        );
        let cache = fixture
            .root
            .join(".agent-browser/browsers/chrome-152.0.0.0");
        fs::create_dir_all(&cache).unwrap();
        symlink(&fixture.installed, cache.join("chrome")).unwrap();
        crate::runtime_profile::write_runtime_state(&fixture.runtime).unwrap();
        let mut metadata = ServiceLaunchMetadata {
            profile_id: Some("qa".into()),
            user_data_dir: Some(fixture.runtime.user_data_dir.clone()),
            browser_capability_launch: Some(
                json!({"applied": false, "browserBuild": "stock_chrome", "profileId": "qa", "reason": "no_matching_preference_binding"}),
            ),
            ..Default::default()
        };
        let original = metadata.clone();
        for selected in [&fixture.installed, &fixture.root.join("unmatched-chrome")] {
            let mut selected_metadata = original.clone();
            selected_metadata.browser_capability_launch = Some(json!({
                "applied": true, "browserBuild": "stock_chrome",
                "reason": "validated_binding_applied", "executablePath": selected,
            }));
            apply_with_proc_root(
                &mut selected_metadata,
                "qa",
                Some(123),
                fixture.runtime.ws_url.as_deref(),
                &fixture.root.join("proc"),
            );
            let proof = selected_metadata.browser_capability_launch.unwrap();
            assert_eq!(proof["applied"], selected == &fixture.installed);
            if selected != &fixture.installed {
                assert_eq!(proof["verificationReason"], "selected_executable_mismatch");
            }
        }
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(
            fixture.root.join("service.json"),
        ));
        for expected in [true, false] {
            metadata = original.clone();
            if !expected {
                fs::remove_file(fixture.root.join("proc/123/fd/10")).unwrap();
            }
            apply_with_proc_root(
                &mut metadata,
                "qa",
                Some(123),
                fixture.runtime.ws_url.as_deref(),
                &fixture.root.join("proc"),
            );
            assert_eq!(
                metadata.browser_capability_launch.as_ref().unwrap()["applied"],
                expected
            );
            persist_service_browser_record_in_repository(
                &repository,
                "qa-session",
                BrowserHost::AttachedExisting,
                BrowserHealth::Ready,
                Some(123),
                fixture.runtime.ws_url.clone(),
                None,
                Some(metadata.clone()),
            )
            .unwrap();
            let state = repository.load_snapshot().unwrap();
            let browser = &state.browsers["session:qa-session"];
            assert_eq!(browser.pid, Some(123));
            assert_eq!(
                browser.browser_build,
                expected.then_some(BrowserBuild::StockChrome)
            );
            assert_eq!(
                browser.executable_path.as_deref(),
                expected.then(|| fixture.installed.to_str().unwrap())
            );
            assert_eq!(
                state.sessions["qa-session"].browser_capability_launch,
                metadata.browser_capability_launch
            );
        }
    }

    #[test]
    fn runtime_identity_drift_fails_closed() {
        for field in [
            "pid",
            "profile",
            "directory",
            "port",
            "websocket",
            "remote",
            "missing",
        ] {
            let mut fixture = Fixture::new();
            match field {
                "pid" => fixture.runtime.browser_pid = 124,
                "profile" => fixture.runtime.runtime_profile = "other".into(),
                "directory" => {
                    fixture.runtime.user_data_dir = fixture.root.to_string_lossy().into()
                }
                "port" => fixture.runtime.devtools_port = Some(9223),
                "websocket" => {
                    fixture.runtime.ws_url =
                        Some("ws://127.0.0.1:9222/devtools/browser/other".into())
                }
                "remote" => {
                    fixture.runtime.ws_url =
                        Some("ws://example.com:9222/devtools/browser/abc-123".into())
                }
                "missing" => fixture.runtime.ws_url = None,
                _ => unreachable!(),
            }
            assert!(fixture.verify().is_err(), "{field}");
        }
    }

    #[test]
    fn proc_evidence_drift_fails_closed() {
        for (path, contents) in [
            ("proc/123/cmdline", "chrome\0--user-data-dir=/wrong\0"),
            (
                "proc/123/cmdline",
                "chrome\0--user-data-dir-prefix=/wrong\0",
            ),
            ("proc/123/cmdline", "chrome\0--type=renderer\0"),
            ("proc/123/stat", "malformed"),
            ("proc/123/net/tcp", "malformed"),
            (
                "proc/123/net/tcp",
                "0: 0100007F:2406 00000000:0000 01 0 0 0 1000 0 456",
            ),
            (
                "proc/123/net/tcp",
                "0: 0100007F:2406 00000000:0000 0A 0 0 0 1000 0 999",
            ),
            (
                "profile/DevToolsActivePort",
                "9222\n/devtools/browser/other\n",
            ),
        ] {
            let fixture = Fixture::new();
            fs::write(fixture.root.join(path), contents).unwrap();
            assert!(fixture.verify().is_err(), "{path}: {contents}");
        }
        for path in [
            "proc/123/exe",
            "proc/123/stat",
            "proc/123/cmdline",
            "proc/123/net/tcp",
            "proc/123/fd/10",
            "profile/DevToolsActivePort",
            "chrome",
        ] {
            let fixture = Fixture::new();
            fs::remove_file(fixture.root.join(path)).unwrap();
            assert!(fixture.verify().is_err(), "missing {path}");
        }
    }

    #[test]
    fn process_executable_and_unambiguous_profile_arguments_required() {
        let fixture = Fixture::new();
        let cmdline = fixture.root.join("proc/123/cmdline");
        fs::write(
            &cmdline,
            format!(
                "chrome\0--user-data-dir\0{}\0",
                fixture.runtime.user_data_dir
            ),
        )
        .unwrap();
        assert!(fixture.verify().is_ok());
        fs::write(
            &cmdline,
            format!(
                "chrome\0--user-data-dir={}\0--user-data-dir={}\0",
                fixture.runtime.user_data_dir, fixture.runtime.user_data_dir
            ),
        )
        .unwrap();
        assert_eq!(fixture.verify().err(), Some("process_profile_mismatch"));
        let custom = fixture.root.join("custom");
        fs::write(&custom, "custom executable").unwrap();
        fs::remove_file(fixture.root.join("proc/123/exe")).unwrap();
        symlink(custom, fixture.root.join("proc/123/exe")).unwrap();
        assert_eq!(fixture.verify().err(), Some("process_executable_mismatch"));
    }

    #[test]
    fn registry_refusals_and_stealth_do_not_gain_installer_proof() {
        for (build, reason) in [
            ("stock_chrome", "profile_incompatible"),
            ("stock_chrome", "service_state_unavailable"),
            ("stealthcdp_chromium", "no_matching_preference_binding"),
        ] {
            let original = json!({"applied": false, "browserBuild": build, "reason": reason});
            let mut metadata = ServiceLaunchMetadata {
                browser_capability_launch: Some(original.clone()),
                ..Default::default()
            };
            apply_managed_runtime_attach_proof(
                &mut metadata,
                "qa",
                Some(123),
                Some("ws://127.0.0.1:9222/devtools/browser/a"),
            );
            assert_eq!(metadata.browser_capability_launch, Some(original));
        }
    }

    #[test]
    fn applied_selection_is_not_observed_attachment_evidence() {
        let mut metadata = ServiceLaunchMetadata {
            browser_capability_launch: Some(
                json!({"applied": true, "browserBuild": "stock_chrome", "reason": "validated_binding_applied", "executablePath": "/requested/chrome"}),
            ),
            ..Default::default()
        };
        apply_managed_runtime_attach_proof(&mut metadata, "qa", Some(123), None);
        assert_eq!(
            metadata.browser_capability_launch.as_ref().unwrap()["applied"],
            false
        );
    }
}
