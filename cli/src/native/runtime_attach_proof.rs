//! Existing-process provenance, independent of launch preferences. Never launches,
//! navigates, takes ownership, or treats a requested executable as observed fact.

use super::service_lifecycle::ServiceLaunchMetadata;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Read;

fn installed_chrome_for_process(
    pid: u32,
    proc_root: &std::path::Path,
) -> Result<std::path::PathBuf, &'static str> {
    let actual = std::fs::read_link(proc_root.join(pid.to_string()).join("exe"))
        .map_err(|_| "process_executable_unreadable")?;
    let entries = std::fs::read_dir(crate::install::get_browsers_dir())
        .map_err(|_| "installed_chrome_missing")?;
    let mut installed_found = false;
    for entry in entries.filter_map(Result::ok).filter(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("chrome-"))
    }) {
        for relative in [
            "chrome",
            "chrome.exe",
            "Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ] {
            let candidate = entry.path().join(relative);
            let Ok(executable) = candidate.canonicalize() else {
                continue;
            };
            if !executable.is_file() {
                continue;
            }
            installed_found = true;
            if executable == actual {
                return Ok(candidate);
            }
        }
    }
    Err(if installed_found {
        "process_executable_mismatch"
    } else {
        "installed_chrome_missing"
    })
}

/// Re-observe build evidence after a metadata-preserving handoff. The caller
/// holds the service repository mutation lock; no ownership fields are changed.
pub(super) fn refresh_retained_attach_proof(
    state: &mut super::service_model::ServiceState,
    session_id: &str,
    runtime_profile: &str,
    pid: Option<u32>,
    endpoint: Option<&str>,
) {
    if cfg!(target_os = "linux") {
        refresh_retained_with_proc_root(
            state,
            session_id,
            runtime_profile,
            pid,
            endpoint,
            std::path::Path::new("/proc"),
        );
    }
}

fn refresh_retained_with_proc_root(
    state: &mut super::service_model::ServiceState,
    session_id: &str,
    runtime_profile: &str,
    pid: Option<u32>,
    endpoint: Option<&str>,
    proc_root: &std::path::Path,
) {
    let Some(session) = state.sessions.get(session_id) else {
        return;
    };
    if session.profile_id.as_deref() != Some(runtime_profile) || session.browser_ids.len() != 1 {
        return;
    }
    let browser_id = session.browser_ids[0].clone();
    let Some(browser) = state.browsers.get(&browser_id) else {
        return;
    };
    if pid.is_none()
        || endpoint.is_none()
        || browser.pid != pid
        || browser.cdp_endpoint.as_deref() != endpoint
        || browser.profile_id.as_deref() != Some(runtime_profile)
        || !browser.active_session_ids.iter().any(|id| id == session_id)
    {
        return;
    }
    let mut metadata = ServiceLaunchMetadata {
        profile_id: session.profile_id.clone(),
        user_data_dir: state
            .profiles
            .get(runtime_profile)
            .and_then(|profile| profile.user_data_dir.clone()),
        browser_capability_launch: session.browser_capability_launch.clone(),
        ..Default::default()
    };
    apply_with_proc_root(&mut metadata, runtime_profile, pid, endpoint, proc_root);
    let Some(proof) = metadata.browser_capability_launch else {
        return;
    };
    if proof["browserBuild"] != "stock_chrome"
        || !matches!(
            proof["reason"].as_str(),
            Some("verified_installed_chrome_runtime_attach" | "managed_runtime_attach_unverified")
        )
    {
        return;
    }
    // Only the existing verifier may mint evidence. Failed re-observation
    // clears stale build projections without rewriting profile or tab custody.
    let proven = proof["applied"] == true;
    let browser = state.browsers.get_mut(&browser_id).unwrap();
    browser.browser_build = proven
        .then(|| serde_json::from_value(proof["browserBuild"].clone()).ok())
        .flatten();
    browser.executable_path = proven
        .then(|| proof["executablePath"].as_str().map(str::to_owned))
        .flatten();
    browser.browser_build_proof = Some(proof.clone());
    state
        .sessions
        .get_mut(session_id)
        .unwrap()
        .browser_capability_launch = Some(proof);
}

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
    // An external BYOP browser has its own registry and live-process proof.
    // A later managed-runtime refresh must not replace it with a stock Chrome
    // runtime-state check, because BYOP has no managed runtime-state file.
    if matches!(
        proof["reason"].as_str(),
        Some("verified_external_byop_process" | "verified_registered_runtime_attach_process")
    ) {
        return;
    }
    // Keep registry refusals and non-stock builds closed. An applied preference
    // is only a launch selection; it must also be verified for an attachment.
    let eligible = proof["browserBuild"] == "stock_chrome"
        && (proof["applied"] == true
            || matches!(
                proof["reason"].as_str(),
                Some(
                    "no_matching_preference_binding"
                        | "explicit_executable_path"
                        | "managed_runtime_attach_unverified"
                )
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
        let process_pid = pid.ok_or("pid_missing")?;
        let installed = installed_chrome_for_process(process_pid, proc_root)?;
        verify(
            &runtime,
            runtime_profile,
            process_pid,
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

/// Verify the physical process behind an already-authorized retained service
/// identity. The caller must establish ownership separately; this helper only
/// observes the exact installed executable, profile and loopback CDP listener.
pub(super) fn verify_retained_service_process(
    runtime: &crate::runtime_profile::RuntimeState,
    runtime_profile: &str,
    pid: u32,
    endpoint: &str,
    profile: &str,
) -> Result<u64, &'static str> {
    let installed = installed_chrome_for_process(pid, std::path::Path::new("/proc"))?;
    verify(
        runtime,
        runtime_profile,
        pid,
        endpoint,
        profile,
        &installed,
        std::path::Path::new("/proc"),
    )
    .map(|process| process.start_ticks)
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

/// Physical evidence for an externally owned Linux browser. The registry and
/// its build label are checked by the caller; this only binds the live process
/// to the exact executable, profile, and loopback DevTools listener.
#[cfg(target_os = "linux")]
pub(super) fn verify_external_byop_process(
    pid: u32,
    endpoint: &str,
    profile: &str,
    executable: &std::path::Path,
) -> Result<u64, &'static str> {
    let port = endpoint
        .strip_prefix("ws://127.0.0.1:")
        .and_then(|rest| rest.split_once("/devtools/browser/"))
        .and_then(|(port, id)| {
            (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
                .then(|| port.parse::<u16>().ok())
                .flatten()
        })
        .ok_or("endpoint_mismatch")?;
    verify_process(
        pid,
        endpoint,
        profile,
        executable,
        std::path::Path::new("/proc"),
        port,
        false,
    )
    .map(|verified| verified.start_ticks)
}

/// Revalidate a retained service-browser proof without depending on the
/// runtime-profile state file. This is the narrow recovery path used when the
/// browser survived its daemon: it observes the exact process, executable,
/// profile, process-start token, DevTools endpoint, and listener again.
pub(super) fn verify_persisted_retained_process(
    browser: &super::service_model::BrowserProcess,
    profile: &super::service_model::BrowserProfile,
    expected_pid: u32,
    expected_endpoint: &str,
) -> Result<u64, &'static str> {
    let proof = browser
        .browser_build_proof
        .as_ref()
        .ok_or("build_proof_missing")?;
    let profile_path = profile
        .user_data_dir
        .as_deref()
        .ok_or("profile_path_missing")?;
    let executable = browser
        .executable_path
        .as_deref()
        .ok_or("executable_path_missing")?;
    let expected_start_ticks = proof
        .get("processStartTicks")
        .and_then(serde_json::Value::as_u64)
        .ok_or("process_start_missing")?;
    let browser_build = browser.browser_build.ok_or("browser_build_missing")?;
    let proof_build = proof
        .get("browserBuild")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let accepted_reason = matches!(
        proof.get("reason").and_then(serde_json::Value::as_str),
        Some(
            "verified_external_byop_process"
                | "verified_registered_runtime_attach_process"
                | "verified_installed_chrome_runtime_attach"
        )
    );
    let expected_sha256 = proof
        .get("executableSha256")
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64);
    if browser_build != super::service_model::BrowserBuild::StockChrome && expected_sha256.is_none()
    {
        return Err("executable_digest_missing");
    }

    if proof.get("applied").and_then(serde_json::Value::as_bool) != Some(true)
        || !accepted_reason
        || proof_build != Some(browser_build)
        || profile
            .browser_build
            .is_some_and(|profile_build| profile_build != browser_build)
        || !matches!(
            browser.host,
            super::service_model::BrowserHost::LocalHeaded
                | super::service_model::BrowserHost::RemoteHeaded
                | super::service_model::BrowserHost::AttachedExisting
        )
        || browser.pid != Some(expected_pid)
        || browser.cdp_endpoint.as_deref() != Some(expected_endpoint)
        || browser.profile_id.as_deref() != Some(profile.id.as_str())
        || proof.get("profileId").and_then(serde_json::Value::as_str) != Some(profile.id.as_str())
        || proof.get("browserPid").and_then(serde_json::Value::as_u64)
            != Some(u64::from(expected_pid))
        || proof.get("cdpEndpoint").and_then(serde_json::Value::as_str) != Some(expected_endpoint)
        || proof.get("userDataDir").and_then(serde_json::Value::as_str) != Some(profile_path)
        || proof
            .get("executablePath")
            .and_then(serde_json::Value::as_str)
            != Some(executable)
    {
        return Err("persisted_proof_mismatch");
    }

    if let Some(expected_sha256) = expected_sha256 {
        let mut file = std::fs::File::open(executable).map_err(|_| "executable_unreadable")?;
        let mut hasher = Sha256::new();
        let mut chunk = [0_u8; 64 * 1024];
        loop {
            let size = file.read(&mut chunk).map_err(|_| "executable_unreadable")?;
            if size == 0 {
                break;
            }
            hasher.update(&chunk[..size]);
        }
        if format!("{:x}", hasher.finalize()) != expected_sha256 {
            return Err("executable_digest_mismatch");
        }
    }

    let observed_start_ticks = verify_external_byop_process(
        expected_pid,
        expected_endpoint,
        profile_path,
        std::path::Path::new(executable),
    )?;
    if observed_start_ticks != expected_start_ticks {
        return Err("process_start_mismatch");
    }
    Ok(observed_start_ticks)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn verify_external_byop_process(
    _pid: u32,
    _endpoint: &str,
    _profile: &str,
    _executable: &std::path::Path,
) -> Result<u64, &'static str> {
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
    if std::path::Path::new(&runtime.user_data_dir)
        .canonicalize()
        .ok()
        != std::path::Path::new(profile).canonicalize().ok()
    {
        return Err("profile_path_mismatch");
    }
    verify_process(pid, endpoint, profile, installed, proc_root, port, true)
}

#[cfg(target_os = "linux")]
fn verify_process(
    pid: u32,
    endpoint: &str,
    profile: &str,
    installed: &std::path::Path,
    proc_root: &std::path::Path,
    port: u16,
    require_active_port: bool,
) -> Result<VerifiedProcess, &'static str> {
    use std::{fs, os::unix::fs::MetadataExt, path::Path};
    let profile = Path::new(profile)
        .canonicalize()
        .map_err(|_| "profile_unreadable")?;
    if require_active_port {
        let active_port = fs::read_to_string(profile.join("DevToolsActivePort"))
            .map_err(|_| "active_port_unreadable")?;
        let mut lines = active_port.lines();
        if lines.next().and_then(|p| p.parse::<u16>().ok()) != Some(port)
            || lines.next() != endpoint.strip_prefix(&format!("ws://127.0.0.1:{port}"))
        {
            return Err("active_port_mismatch");
        }
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
    let mut arguments = cmdline
        .split(|b| *b == 0)
        .filter(|arg| !arg.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "process_arguments_unreadable")?;
    // Linux Chrome may rewrite argv into one space-joined process title.
    // Accept only the unquoted, unambiguous form headed by the independently
    // verified executable. Never shell-parse a title or guess spaced paths.
    if arguments.len() == 1 {
        // Joined text cannot recover original argv boundaries. Require an
        // independent profile lock naming this same live process as well.
        let singleton =
            fs::read_link(profile.join("SingletonLock")).map_err(|_| "profile_lock_unreadable")?;
        if singleton
            .to_str()
            .and_then(|s| s.rsplit_once('-'))
            .and_then(|(_, value)| value.parse::<u32>().ok())
            != Some(pid)
        {
            return Err("profile_lock_mismatch");
        }
        let title = arguments[0];
        let executable_text = executable.to_str().ok_or("process_arguments_unreadable")?;
        let profile_text = profile.to_str().ok_or("process_arguments_unreadable")?;
        if executable_text.chars().any(char::is_whitespace)
            || profile_text.chars().any(char::is_whitespace)
            || title
                .chars()
                .any(|c| matches!(c, '\'' | '"' | '\\') || (c.is_whitespace() && c != ' '))
            || !title.starts_with(&format!("{executable_text} "))
        {
            return Err("process_arguments_ambiguous");
        }
        arguments = title.split(' ').filter(|arg| !arg.is_empty()).collect();
        // Chrome may include one launch URL among its switches. Keep that
        // positional argument unambiguous in a rewritten title; in particular,
        // never accept a truncated spaced profile path.
        let positional = arguments
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, arg)| !arg.starts_with("--"))
            .collect::<Vec<_>>();
        let valid_launch_url = positional.len() == 1
            && url::Url::parse(positional[0].1).is_ok_and(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
            });
        if !positional.is_empty() && !valid_launch_url {
            return Err("process_arguments_ambiguous");
        }
    }
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
    if !require_active_port {
        let expected = format!("--remote-debugging-port={port}");
        let fixed = arguments.iter().filter(|arg| **arg == expected).count() == 1;
        let ephemeral = arguments
            .iter()
            .filter(|arg| **arg == "--remote-debugging-port=0")
            .count()
            == 1;
        if fixed == ephemeral {
            return Err("process_debug_port_mismatch");
        }
        if ephemeral {
            let active_port = fs::read_to_string(profile.join("DevToolsActivePort"))
                .map_err(|_| "active_port_unreadable")?;
            let mut lines = active_port.lines();
            if lines.next().and_then(|p| p.parse::<u16>().ok()) != Some(port)
                || lines.next() != endpoint.strip_prefix(&format!("ws://127.0.0.1:{port}"))
            {
                return Err("active_port_mismatch");
            }
        }
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
    fn managed_refresh_preserves_verified_external_byop_proof() {
        let fixture = Fixture::new();
        let proof = json!({
            "applied": true,
            "reason": "verified_external_byop_process",
            "browserBuild": "stock_chrome",
            "browserPid": 123,
            "cdpEndpoint": fixture.runtime.ws_url,
        });
        let mut metadata = ServiceLaunchMetadata {
            profile_id: Some("qa".into()),
            user_data_dir: Some(fixture.runtime.user_data_dir.clone()),
            browser_capability_launch: Some(proof.clone()),
            ..Default::default()
        };
        apply_with_proc_root(
            &mut metadata,
            "qa",
            Some(123),
            fixture.runtime.ws_url.as_deref(),
            &fixture.root.join("proc"),
        );
        assert_eq!(metadata.browser_capability_launch, Some(proof));
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
    fn metadata_preserving_handoff_refreshes_only_exact_retained_build_proof() {
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
        let newer_cache = fixture
            .root
            .join(".agent-browser/browsers/chrome-999.0.0.0");
        let newer_installed = fixture.root.join("newer-chrome");
        fs::create_dir_all(&newer_cache).unwrap();
        fs::write(&newer_installed, "newer installed fixture").unwrap();
        symlink(&newer_installed, newer_cache.join("chrome")).unwrap();
        assert_eq!(
            crate::install::find_installed_chrome()
                .unwrap()
                .canonicalize()
                .unwrap(),
            newer_installed
        );
        crate::runtime_profile::write_runtime_state(&fixture.runtime).unwrap();
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(
            fixture.root.join("service.json"),
        ));
        let metadata = ServiceLaunchMetadata {
            profile_id: Some("qa".into()),
            user_data_dir: Some(fixture.runtime.user_data_dir.clone()),
            browser_capability_launch: Some(json!({
                "applied": false, "browserBuild": "stock_chrome",
                "profileId": "qa", "reason": "no_matching_preference_binding",
            })),
            ..Default::default()
        };
        for metadata in [Some(metadata), None] {
            persist_service_browser_record_in_repository(
                &repository,
                "qa-session",
                BrowserHost::AttachedExisting,
                BrowserHealth::Ready,
                Some(123),
                fixture.runtime.ws_url.clone(),
                None,
                metadata,
            )
            .unwrap();
        }
        let original = repository.load_snapshot().unwrap();
        assert_eq!(original.browsers["session:qa-session"].browser_build, None);
        let refresh = |state: &mut super::super::service_model::ServiceState| {
            refresh_retained_with_proc_root(
                state,
                "qa-session",
                "qa",
                Some(123),
                fixture.runtime.ws_url.as_deref(),
                &fixture.root.join("proc"),
            );
        };
        for mismatch in [
            "profile",
            "pid",
            "endpoint",
            "session",
            "ambiguous",
            "nonstock",
            "registry",
        ] {
            let mut state = original.clone();
            let browser = state.browsers.get_mut("session:qa-session").unwrap();
            match mismatch {
                "profile" => browser.profile_id = Some("foreign".into()),
                "pid" => browser.pid = Some(124),
                "endpoint" => {
                    browser.cdp_endpoint = Some("ws://127.0.0.1:9222/devtools/browser/other".into())
                }
                "session" => browser.active_session_ids.clear(),
                "ambiguous" => state
                    .sessions
                    .get_mut("qa-session")
                    .unwrap()
                    .browser_ids
                    .push("foreign".into()),
                "nonstock" => {
                    state
                        .sessions
                        .get_mut("qa-session")
                        .unwrap()
                        .browser_capability_launch =
                        Some(json!({"applied":true,"browserBuild":"stealthcdp_chromium"}))
                }
                "registry" => {
                    state
                        .sessions
                        .get_mut("qa-session")
                        .unwrap()
                        .browser_capability_launch = Some(
                        json!({"applied":false,"browserBuild":"stock_chrome","reason":"registry_denied"}),
                    )
                }
                _ => unreachable!(),
            }
            let before = state.clone();
            refresh(&mut state);
            assert_eq!(state, before, "{mismatch}");
        }
        let persist_refresh = || {
            crate::native::service_health::persist_service_browser_record_with_refresh(
                &repository,
                "qa-session",
                BrowserHost::AttachedExisting,
                BrowserHealth::Ready,
                Some(123),
                fixture.runtime.ws_url.clone(),
                None,
                None,
                refresh,
            )
            .unwrap();
            repository.load_snapshot().unwrap()
        };
        let mut state = persist_refresh();
        assert_eq!(
            state.browsers["session:qa-session"].browser_build,
            Some(BrowserBuild::StockChrome)
        );
        let verified = state.clone();
        state = persist_refresh();
        assert_eq!(state, verified, "repeat refresh is idempotent");
        fs::remove_file(fixture.root.join("proc/123/fd/10")).unwrap();
        state = persist_refresh();
        let browser = state.browsers.get_mut("session:qa-session").unwrap();
        assert_eq!(browser.browser_build, None);
        assert_eq!(browser.executable_path, None);
        assert_eq!(
            browser.browser_build_proof.as_ref().unwrap()["applied"],
            false
        );
        browser.browser_build_proof = original.browsers["session:qa-session"]
            .browser_build_proof
            .clone();
        state
            .sessions
            .get_mut("qa-session")
            .unwrap()
            .browser_capability_launch = original.sessions["qa-session"]
            .browser_capability_launch
            .clone();
        assert_eq!(state, original, "all non-proof state is preserved");
        // A failed observation is retryable, never an authorization shortcut:
        // all process evidence must pass again before restoring the projection.
        symlink("socket:[456]", fixture.root.join("proc/123/fd/10")).unwrap();
        let recovered = persist_refresh();
        assert_eq!(
            recovered.browsers["session:qa-session"].browser_build,
            Some(BrowserBuild::StockChrome)
        );
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
    fn joined_chrome_process_title_requires_unambiguous_identity() {
        let fixture = Fixture::new();
        symlink(
            "fixture-host-123",
            fixture.root.join("profile/SingletonLock"),
        )
        .unwrap();
        let title = format!(
            "{} --user-data-dir={} --remote-debugging-port=0",
            fixture.installed.display(),
            fixture.runtime.user_data_dir
        );
        let cmdline = fixture.root.join("proc/123/cmdline");
        fs::write(&cmdline, format!("{title}\0")).unwrap();
        assert!(fixture.verify().is_ok());
        for valid in [
            format!("{title} https://chatgpt.com/"),
            format!("{title} https://chatgpt.com/ --no-sandbox"),
        ] {
            fs::write(&cmdline, format!("{valid}\0")).unwrap();
            assert!(fixture.verify().is_ok(), "{valid}");
        }
        for invalid in [
            title.replace(
                " --remote-debugging-port",
                " suffix --remote-debugging-port",
            ),
            format!("{title} https://chatgpt.com/ https://example.com/"),
            format!("{title} not-a-url"),
            format!("{title} https://user:pass@chatgpt.com/"),
            format!("{title} https://chatgpt.com/a b"),
            format!("{title} https://chatgpt.com/ --bad='quoted value'"),
            format!("{title} --type=renderer"),
            format!("{title} --user-data-dir={}", fixture.runtime.user_data_dir),
            title.replace("--user-data-dir=", "--user-data-dir-prefix="),
            title.replace(&fixture.runtime.user_data_dir, "/wrong"),
            format!("{title} --label='ambiguous value'"),
            title.replace(' ', "\t"),
            format!("not-chrome {title}"),
        ] {
            fs::write(&cmdline, format!("{invalid}\0")).unwrap();
            assert!(fixture.verify().is_err());
        }
        fs::write(&cmdline, format!("{title}\0")).unwrap();
        fs::remove_file(fixture.root.join("profile/SingletonLock")).unwrap();
        assert_eq!(fixture.verify().err(), Some("profile_lock_unreadable"));
        symlink(
            "fixture-host-124",
            fixture.root.join("profile/SingletonLock"),
        )
        .unwrap();
        assert_eq!(fixture.verify().err(), Some("profile_lock_mismatch"));
    }

    #[test]
    fn joined_launch_url_preserves_external_browser_proof() {
        let fixture = Fixture::new();
        symlink(
            "fixture-host-123",
            fixture.root.join("profile/SingletonLock"),
        )
        .unwrap();
        let title = format!(
            "{} --remote-debugging-port=0 --user-data-dir={} https://chatgpt.com/ --no-sandbox",
            fixture.installed.display(),
            fixture.runtime.user_data_dir
        );
        let cmdline = fixture.root.join("proc/123/cmdline");
        fs::write(&cmdline, format!("{title}\0")).unwrap();
        let verify_external = || {
            verify_process(
                123,
                fixture.runtime.ws_url.as_deref().unwrap(),
                &fixture.runtime.user_data_dir,
                &fixture.installed,
                &fixture.root.join("proc"),
                9222,
                false,
            )
        };
        assert!(verify_external().is_ok());
        fs::write(
            &cmdline,
            format!(
                "{}\0",
                title.replace("--remote-debugging-port=0", "--remote-debugging-port=9223")
            ),
        )
        .unwrap();
        assert_eq!(verify_external().err(), Some("process_debug_port_mismatch"));
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
