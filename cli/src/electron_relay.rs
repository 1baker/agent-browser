//! Managed, loopback-only WSL relays for operator-owned Windows Electron apps.
//!
//! The lifecycle never launches the Windows application. It discovers one
//! exact main process and its exact loopback listener, then runs the existing
//! PID-bound WSL relay until that process exits.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(target_os = "linux")]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

#[cfg(target_os = "linux")]
use crate::native::cdp::chrome::run_wsl_windows_cdp_relay;

const SCHEMA: &str = "agent-browser.electron-relay.v1";
const MANAGED_MARKER: &str = "Managed by agent-browser electron relay";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RelayConfig {
    schema_version: String,
    name: String,
    process_name: String,
    local_port: u16,
    remote_port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct WindowsDiscovery {
    process_count: usize,
    pids: Vec<u32>,
    listener_count: usize,
    owners: Vec<u32>,
    addresses: Vec<String>,
}

#[derive(Clone, Debug)]
enum DiscoveryState {
    NotRunning,
    Ready(u32),
}

#[derive(Clone, Debug)]
struct ManagedPaths {
    config: PathBuf,
    service: PathBuf,
    timer: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemdState {
    load_state: String,
    active_state: String,
    sub_state: String,
    main_pid: u32,
}

pub fn run_command(args: &[String], json_mode: bool) -> ExitCode {
    match run_command_inner(args) {
        Ok(value) => {
            if json_mode {
                println!("{}", serde_json::to_string(&value).unwrap_or_default());
            } else {
                print_human(&value);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if json_mode {
                println!("{}", json!({"success": false, "error": error}));
            } else {
                eprintln!("Electron relay error: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run_command_inner(args: &[String]) -> Result<Value, String> {
    if !cfg!(target_os = "linux") {
        return Err("electron relay is supported only from Linux under WSL".to_string());
    }
    if args.get(1).map(String::as_str) != Some("relay") {
        return Err(
            "Usage: agent-browser electron relay <install|doctor|run|uninstall>".to_string(),
        );
    }
    let operation = args.get(2).map(String::as_str).ok_or_else(|| {
        "Usage: agent-browser electron relay <install|doctor|run|uninstall>".to_string()
    })?;
    let name = required_arg(args, "--name")?;
    validate_name(&name)?;
    match operation {
        "install" => install(args, &name),
        "doctor" => doctor(&name),
        "run" => run(&name),
        "uninstall" => uninstall(args, &name),
        other => Err(format!("Unknown electron relay operation: {other}")),
    }
}

fn install(args: &[String], name: &str) -> Result<Value, String> {
    let apply = args.iter().any(|arg| arg == "--apply");
    if apply && args.iter().any(|arg| arg == "--dry-run") {
        return Err("Choose only one of --apply or --dry-run".to_string());
    }
    let process_name = required_arg(args, "--process-name")?;
    validate_process_name(&process_name)?;
    let local_port = port_arg(args, "--local-port")?;
    let remote_port = port_arg(args, "--remote-port")?;
    if local_port == remote_port {
        return Err("local and remote ports must differ".to_string());
    }
    let config = RelayConfig {
        schema_version: SCHEMA.to_string(),
        name: name.to_string(),
        process_name,
        local_port,
        remote_port,
    };
    let paths = managed_paths(name)?;
    let executable = std::env::current_exe()
        .map_err(|err| format!("Unable to resolve agent-browser executable: {err}"))?;
    let artifacts = rendered_artifacts(&config, &paths, &executable)?;
    let states = artifact_states(&artifacts)?;
    let already_exact = states.iter().all(|state| *state == "exact");
    if states.contains(&"drifted") {
        return Err("Refusing to replace drifted electron relay artifacts".to_string());
    }
    if apply {
        if !already_exact {
            for (path, content) in &artifacts {
                atomic_write(path, content)?;
            }
        }
        systemctl(&["daemon-reload"])?;
        systemctl(&["enable", "--now", &timer_name(name)])?;
    }
    Ok(json!({
        "success": true,
        "schemaVersion": SCHEMA,
        "operation": "install",
        "mode": if apply { "apply" } else { "dry-run" },
        "mutated": apply && !already_exact,
        "idempotent": already_exact,
        "config": config,
        "paths": path_report(&paths),
        "artifactStates": states,
        "nextAction": if apply { format!("agent-browser electron relay doctor --name {name} --json") } else { format!("rerun with --apply to install relay {name}") },
    }))
}

fn doctor(name: &str) -> Result<Value, String> {
    let (config, paths) = load_config(name)?;
    let executable = std::env::current_exe()
        .map_err(|err| format!("Unable to resolve agent-browser executable: {err}"))?;
    let artifacts = rendered_artifacts(&config, &paths, &executable)?;
    let artifact_states = artifact_states(&artifacts)?;
    let managed_exact = artifact_states.iter().all(|state| *state == "exact");
    let discovery = discover_windows(&config)?;
    let systemd = systemd_state(name)?;
    let (state, windows_pid) = match discovery {
        DiscoveryState::NotRunning => ("not_running", None),
        DiscoveryState::Ready(pid) => {
            if local_cdp_ready(config.local_port) {
                if systemd.active_state != "active" || systemd.main_pid == 0 {
                    return Err(format!(
                        "WSL CDP endpoint on port {} is not owned by the active managed relay service",
                        config.local_port
                    ));
                }
                ("ready", Some(pid))
            } else if TcpListener::bind(("127.0.0.1", config.local_port)).is_err() {
                return Err(format!(
                    "WSL loopback port {} is occupied by a non-CDP listener",
                    config.local_port
                ));
            } else {
                ("relay_not_running", Some(pid))
            }
        }
    };
    Ok(json!({
        "success": managed_exact,
        "schemaVersion": SCHEMA,
        "operation": "doctor",
        "ready": state == "ready" && managed_exact,
        "state": state,
        "windowsPid": windows_pid,
        "windowsEndpoint": format!("http://127.0.0.1:{}", config.remote_port),
        "wslEndpoint": format!("http://127.0.0.1:{}", config.local_port),
        "loopbackOnly": true,
        "managedArtifactsExact": managed_exact,
        "service": systemd,
        "artifactStates": artifact_states,
        "paths": path_report(&paths),
    }))
}

fn run(name: &str) -> Result<Value, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = name;
        return Err("electron relay is supported only from Linux under WSL".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        let (config, _) = load_config(name)?;
        match discover_windows(&config)? {
            DiscoveryState::NotRunning => Ok(json!({
                "success": true,
                "schemaVersion": SCHEMA,
                "operation": "run",
                "state": "not_running",
                "started": false,
            })),
            DiscoveryState::Ready(pid) => {
                run_wsl_windows_cdp_relay(config.local_port, config.remote_port, pid, None)?;
                Ok(json!({
                    "success": true,
                    "schemaVersion": SCHEMA,
                    "operation": "run",
                    "state": "application_exited",
                    "started": true,
                    "windowsPid": pid,
                }))
            }
        }
    }
}

fn uninstall(args: &[String], name: &str) -> Result<Value, String> {
    let apply = args.iter().any(|arg| arg == "--apply");
    if apply && args.iter().any(|arg| arg == "--dry-run") {
        return Err("Choose only one of --apply or --dry-run".to_string());
    }
    let (config, paths) = load_config(name)?;
    let executable = std::env::current_exe()
        .map_err(|err| format!("Unable to resolve agent-browser executable: {err}"))?;
    let artifacts = rendered_artifacts(&config, &paths, &executable)?;
    let states = artifact_states(&artifacts)?;
    if states.iter().any(|state| *state != "exact") {
        return Err("Refusing to remove missing or drifted electron relay artifacts".to_string());
    }
    if apply {
        systemctl(&["disable", "--now", &timer_name(name)])?;
        systemctl(&["stop", &service_name(name)])?;
        for (path, _) in artifacts.iter().rev() {
            fs::remove_file(path)
                .map_err(|err| format!("Failed to remove {}: {err}", path.display()))?;
        }
        systemctl(&["daemon-reload"])?;
    }
    Ok(json!({
        "success": true,
        "schemaVersion": SCHEMA,
        "operation": "uninstall",
        "mode": if apply { "apply" } else { "dry-run" },
        "mutated": apply,
        "paths": path_report(&paths),
    }))
}

fn discover_windows(config: &RelayConfig) -> Result<DiscoveryState, String> {
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
$processes = @(Get-CimInstance Win32_Process | Where-Object {{ $_.Name -eq '{process}' -and $_.CommandLine -match '--remote-debugging-port={port}(?:\s|$)' -and $_.CommandLine -notmatch '--type=' }})
$listeners = @(Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue)
[pscustomobject]@{{ ProcessCount = $processes.Count; Pids = @($processes.ProcessId); ListenerCount = $listeners.Count; Owners = @($listeners.OwningProcess); Addresses = @($listeners.LocalAddress) }} | ConvertTo-Json -Compress"#,
        process = config.process_name,
        port = config.remote_port,
    );
    let output = powershell_output(&script)?;
    let discovery: WindowsDiscovery = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("Invalid Windows relay discovery output: {err}"))?;
    if discovery.process_count == 0 && discovery.listener_count == 0 {
        return Ok(DiscoveryState::NotRunning);
    }
    if discovery.process_count != 1
        || discovery.listener_count != 1
        || discovery.pids.len() != 1
        || discovery.owners.len() != 1
        || discovery.addresses.as_slice() != ["127.0.0.1"]
    {
        return Err(format!(
            "Expected one {} main process and one Windows loopback listener on port {}",
            config.process_name, config.remote_port
        ));
    }
    if discovery.pids[0] != discovery.owners[0] {
        return Err(
            "Windows listener owner does not match the exact Electron main process".to_string(),
        );
    }
    Ok(DiscoveryState::Ready(discovery.pids[0]))
}

fn powershell_output(script: &str) -> Result<std::process::Output, String> {
    let executable = std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_POWERSHELL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
        });
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut command = Command::new(&executable);
    if std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_SKIP_INTEROP_CHECK").is_none() {
        command.env("WSL_INTEROP", latest_interop_socket()?);
    }
    let output = command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .output()
        .map_err(|err| format!("Failed to run Windows PowerShell: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "Windows relay discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

#[cfg(target_os = "linux")]
fn latest_interop_socket() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("WSL_INTEROP").map(PathBuf::from) {
        if is_socket(&path) {
            return Ok(path);
        }
    }
    let mut entries = fs::read_dir("/run/WSL")
        .map_err(|err| format!("No live WSL interoperability socket is available: {err}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            (name.ends_with("_interop") && metadata.file_type().is_socket())
                .then_some((modified, path))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    entries
        .into_iter()
        .map(|(_, path)| path)
        .next()
        .ok_or_else(|| "No live WSL interoperability socket is available".to_string())
}

#[cfg(not(target_os = "linux"))]
fn latest_interop_socket() -> Result<PathBuf, String> {
    Err("electron relay is supported only from Linux under WSL".to_string())
}

#[cfg(target_os = "linux")]
fn is_socket(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

fn local_cdp_ready(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(500),
    ) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream
        .write_all(b"GET /json/version HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                response.extend_from_slice(&chunk[..count]);
                if response
                    .windows(b"webSocketDebuggerUrl".len())
                    .any(|window| window == b"webSocketDebuggerUrl")
                {
                    break;
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => return false,
        }
    }
    let response = String::from_utf8_lossy(&response);
    response.starts_with("HTTP/1.1 200") && response.contains("webSocketDebuggerUrl")
}

fn rendered_artifacts(
    config: &RelayConfig,
    paths: &ManagedPaths,
    executable: &Path,
) -> Result<Vec<(PathBuf, String)>, String> {
    let config_text = format!(
        "{}\n",
        serde_json::to_string_pretty(config)
            .map_err(|err| format!("Failed to serialize relay config: {err}"))?
    );
    let digest = hex::encode(Sha256::digest(config_text.as_bytes()));
    let executable = executable.display();
    let service = format!(
        "# {MANAGED_MARKER}\n# Config-SHA256={digest}\n[Unit]\nDescription=Private PID-bound Windows Electron DevTools relay ({name})\nAfter=default.target\n\n[Service]\nType=simple\nExecStart=\"{executable}\" electron relay run --name {name}\nKillMode=control-group\nTimeoutStopSec=5\nNoNewPrivileges=true\nPrivateTmp=true\nProtectSystem=strict\nProtectHome=read-only\n",
        name = config.name,
    );
    let timer = format!(
        "# {MANAGED_MARKER}\n# Config-SHA256={digest}\n[Unit]\nDescription=Restore private Windows Electron DevTools relay ({name})\n\n[Timer]\nOnBootSec=15s\nOnUnitInactiveSec=15s\nAccuracySec=1s\nUnit={service}\n\n[Install]\nWantedBy=timers.target\n",
        name = config.name,
        service = service_name(&config.name),
    );
    Ok(vec![
        (paths.config.clone(), config_text),
        (paths.service.clone(), service),
        (paths.timer.clone(), timer),
    ])
}

fn managed_paths(name: &str) -> Result<ManagedPaths, String> {
    let config_root = std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_CONFIG_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        })
        .ok_or_else(|| "Unable to resolve the user configuration directory".to_string())?;
    Ok(ManagedPaths {
        config: config_root
            .join("agent-browser/electron-relays")
            .join(format!("{name}.json")),
        service: config_root.join("systemd/user").join(service_name(name)),
        timer: config_root.join("systemd/user").join(timer_name(name)),
    })
}

fn load_config(name: &str) -> Result<(RelayConfig, ManagedPaths), String> {
    let paths = managed_paths(name)?;
    let raw = fs::read_to_string(&paths.config)
        .map_err(|err| format!("Failed to read {}: {err}", paths.config.display()))?;
    let config: RelayConfig = serde_json::from_str(&raw)
        .map_err(|err| format!("Invalid relay config {}: {err}", paths.config.display()))?;
    if config.schema_version != SCHEMA || config.name != name {
        return Err("Relay config schema or name does not match the request".to_string());
    }
    validate_name(&config.name)?;
    validate_process_name(&config.process_name)?;
    Ok((config, paths))
}

fn artifact_states(artifacts: &[(PathBuf, String)]) -> Result<Vec<&'static str>, String> {
    artifacts
        .iter()
        .map(|(path, expected)| match fs::read_to_string(path) {
            Ok(actual) if actual == *expected => Ok("exact"),
            Ok(_) => Ok("drifted"),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("missing"),
            Err(err) => Err(format!("Failed to inspect {}: {err}", path.display())),
        })
        .collect()
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Invalid managed path {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;
    let temporary = parent.join(format!(
        ".{}.tmp",
        path.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&temporary, content)
        .map_err(|err| format!("Failed to stage {}: {err}", path.display()))?;
    fs::rename(&temporary, path)
        .map_err(|err| format!("Failed to commit {}: {err}", path.display()))
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    if std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_SKIP_SYSTEMCTL").is_some() {
        return Ok(());
    }
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .map_err(|err| format!("Failed to run systemctl --user: {err}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("systemctl --user {} failed", args.join(" ")))
}

fn systemd_state(name: &str) -> Result<SystemdState, String> {
    if std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_SKIP_SYSTEMCTL").is_some() {
        let active =
            std::env::var_os("AGENT_BROWSER_ELECTRON_RELAY_FIXTURE_SERVICE_ACTIVE").is_some();
        return Ok(SystemdState {
            load_state: "fixture".to_string(),
            active_state: if active { "active" } else { "inactive" }.to_string(),
            sub_state: if active { "running" } else { "dead" }.to_string(),
            main_pid: u32::from(active),
        });
    }
    let output = Command::new("systemctl")
        .args([
            "--user",
            "show",
            &service_name(name),
            "--property=LoadState",
            "--property=ActiveState",
            "--property=SubState",
            "--property=MainPID",
            "--no-pager",
        ])
        .output()
        .map_err(|err| format!("Failed to inspect managed relay service: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to inspect managed relay service: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut values = std::collections::HashMap::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key, value);
        }
    }
    Ok(SystemdState {
        load_state: values
            .get("LoadState")
            .copied()
            .unwrap_or("unknown")
            .to_string(),
        active_state: values
            .get("ActiveState")
            .copied()
            .unwrap_or("unknown")
            .to_string(),
        sub_state: values
            .get("SubState")
            .copied()
            .unwrap_or("unknown")
            .to_string(),
        main_pid: values
            .get("MainPID")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
    })
}

fn required_arg(args: &[String], flag: &str) -> Result<String, String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .filter(|value| !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| format!("Missing required {flag}"))
}

fn port_arg(args: &[String], flag: &str) -> Result<u16, String> {
    required_arg(args, flag)?
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("{flag} must be a port from 1 to 65535"))
}

fn validate_name(value: &str) -> Result<(), String> {
    let valid = !value.is_empty()
        && value.len() <= 63
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || (byte == b'-' && index > 0)
        });
    if valid && !value.ends_with('-') {
        Ok(())
    } else {
        Err(
            "relay name must use 1 to 63 lowercase letters, digits, or interior hyphens"
                .to_string(),
        )
    }
}

fn validate_process_name(value: &str) -> Result<(), String> {
    let valid = value.len() > 4
        && value.len() <= 96
        && value.ends_with(".exe")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    valid
        .then_some(())
        .ok_or_else(|| "process name must be one simple Windows .exe basename".to_string())
}

fn service_name(name: &str) -> String {
    format!("agent-browser-electron-relay-{name}.service")
}

fn timer_name(name: &str) -> String {
    format!("agent-browser-electron-relay-{name}.timer")
}

fn path_report(paths: &ManagedPaths) -> Value {
    json!({
        "config": paths.config,
        "service": paths.service,
        "timer": paths.timer,
    })
}

fn print_human(value: &Value) {
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("relay");
    let state = value
        .get("state")
        .or_else(|| value.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("complete");
    println!("Electron relay {operation}: {state}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_managed_identifiers() {
        assert!(validate_name("termius").is_ok());
        assert!(validate_name("bad/name").is_err());
        assert!(validate_process_name("Termius.exe").is_ok());
        assert!(validate_process_name("Termius.exe';Remove-Item").is_err());
    }

    #[test]
    fn rendered_units_are_loopback_relay_only_and_timer_bounded() {
        let config = RelayConfig {
            schema_version: SCHEMA.to_string(),
            name: "termius".to_string(),
            process_name: "Termius.exe".to_string(),
            local_port: 19222,
            remote_port: 9222,
        };
        let paths = ManagedPaths {
            config: PathBuf::from("/tmp/config.json"),
            service: PathBuf::from("/tmp/service"),
            timer: PathBuf::from("/tmp/timer"),
        };
        let rendered = rendered_artifacts(&config, &paths, Path::new("/opt/agent-browser"))
            .expect("render artifacts");
        assert!(rendered[1].1.contains("electron relay run --name termius"));
        assert!(!rendered[1].1.contains("Restart="));
        assert!(rendered[2].1.contains("OnUnitInactiveSec=15s"));
        assert!(!rendered.iter().any(|(_, text)| text.contains("0.0.0.0")));
    }

    #[test]
    fn discovery_rejects_wrong_listener_owner() {
        let value = WindowsDiscovery {
            process_count: 1,
            pids: vec![42],
            listener_count: 1,
            owners: vec![43],
            addresses: vec!["127.0.0.1".to_string()],
        };
        assert_ne!(value.pids[0], value.owners[0]);
    }
}
