//! Source-free Linux workstation installation.
//!
//! The workstation installer materializes the release binary and versioned
//! support assets without relying on a repository checkout or package manager
//! command at runtime. Host provisioning and runtime reconciliation are added
//! as explicit phases so every failure can stop before service activation.

use serde::Serialize;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

const INSTALL_SCHEMA_VERSION: &str = "agent-browser.workstation-install.v1";
const DEFAULT_DASHBOARD_PORT: u16 = 4848;
const DEFAULT_GUACAMOLE_PORT: u16 = 8092;
const GUACAMOLE_COMPOSE: &str = include_str!("../assets/workstation/guacamole/compose.yml");
const GUACAMOLE_ENVIRONMENT_EXAMPLE: &str =
    include_str!("../assets/workstation/guacamole/environment.example");
const GUACAMOLE_SCHEMA_GENERATOR: &str =
    include_str!("../assets/workstation/guacamole/generate-initdb.sh");
const GUACAMOLE_BUNDLE_MANIFEST: &str =
    include_str!("../assets/workstation/guacamole/manifest.json");
const GUACAMOLE_INITDB: &str = include_str!("../assets/workstation/guacamole/init/001-initdb.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkstationInstallArgs {
    mode: InstallMode,
    json: bool,
    dashboard_port: u16,
    guacamole_port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkstationPaths {
    root: String,
    binary: String,
    support_dir: String,
    unit_dir: String,
    guacamole_state_dir: String,
    guacamole_secret_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkstationInstallReport {
    schema_version: &'static str,
    success: bool,
    mode: &'static str,
    mutated: bool,
    ready: bool,
    version: &'static str,
    dashboard_port: u16,
    guacamole_port: u16,
    paths: WorkstationPaths,
    phases: Vec<&'static str>,
    next_action: &'static str,
}

pub fn run_workstation_install(args: &[String]) {
    let parsed = match parse_workstation_install_args(args) {
        Ok(parsed) => parsed,
        Err(error) => fail(&error, args.iter().any(|arg| arg == "--json")),
    };
    if !cfg!(target_os = "linux") {
        fail(
            "agent-browser install workstation is only supported on Linux",
            parsed.json,
        );
    }

    let root = match workstation_root() {
        Ok(root) => root,
        Err(error) => fail(&error, parsed.json),
    };
    let paths = install_paths(&root);
    let mut phases = vec!["plan-validated"];

    let mutated = if parsed.mode == InstallMode::Apply {
        match materialize_payload(&paths, &parsed) {
            Ok(()) => {
                phases.extend(["payload-staged", "units-staged", "payload-committed"]);
                true
            }
            Err(error) => fail(&error, parsed.json),
        }
    } else {
        false
    };

    let report = WorkstationInstallReport {
        schema_version: INSTALL_SCHEMA_VERSION,
        success: true,
        mode: match parsed.mode {
            InstallMode::DryRun => "dry-run",
            InstallMode::Apply => "apply",
        },
        mutated,
        ready: false,
        version: env!("CARGO_PKG_VERSION"),
        dashboard_port: parsed.dashboard_port,
        guacamole_port: parsed.guacamole_port,
        paths: WorkstationPaths {
            root: root.display().to_string(),
            binary: paths.binary.display().to_string(),
            support_dir: paths.support_dir.display().to_string(),
            unit_dir: paths.unit_dir.display().to_string(),
            guacamole_state_dir: paths.guacamole_state_dir.display().to_string(),
            guacamole_secret_file: paths.guacamole_secret_file.display().to_string(),
        },
        phases,
        next_action: "workstation substrate provisioning is required before service activation",
    };

    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        );
    } else {
        println!(
            "Workstation install {} complete for agent-browser {}.",
            report.mode, report.version
        );
        println!("  Binary: {}", report.paths.binary);
        println!("  Support: {}", report.paths.support_dir);
        println!("  Units: {}", report.paths.unit_dir);
        println!("  Ready: no");
        println!("  Next: {}", report.next_action);
    }
}

fn parse_workstation_install_args(args: &[String]) -> Result<WorkstationInstallArgs, String> {
    let mut mode = None;
    let mut json = false;
    let mut dashboard_port = DEFAULT_DASHBOARD_PORT;
    let mut guacamole_port = DEFAULT_GUACAMOLE_PORT;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "install" | "workstation" => {}
            "--dry-run" => set_mode(&mut mode, InstallMode::DryRun)?,
            "--apply" => set_mode(&mut mode, InstallMode::Apply)?,
            "--json" => json = true,
            "--dashboard-port" => {
                index += 1;
                dashboard_port = parse_port(args.get(index), "--dashboard-port")?;
            }
            "--guacamole-port" => {
                index += 1;
                guacamole_port = parse_port(args.get(index), "--guacamole-port")?;
            }
            "--help" | "-h" => {
                return Err(workstation_usage().to_string());
            }
            unknown => return Err(format!("Unknown workstation install argument: {unknown}")),
        }
        index += 1;
    }

    let mode = mode.ok_or_else(|| {
        "Choose exactly one of --dry-run or --apply for workstation installation".to_string()
    })?;
    Ok(WorkstationInstallArgs {
        mode,
        json,
        dashboard_port,
        guacamole_port,
    })
}

fn set_mode(mode: &mut Option<InstallMode>, requested: InstallMode) -> Result<(), String> {
    if let Some(current) = mode {
        if *current != requested {
            return Err("--dry-run and --apply are mutually exclusive".to_string());
        }
        return Err("The workstation install mode may be specified only once".to_string());
    }
    *mode = Some(requested);
    Ok(())
}

fn parse_port(value: Option<&String>, flag: &str) -> Result<u16, String> {
    value
        .ok_or_else(|| format!("{flag} requires a port"))?
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("{flag} must be an integer from 1 through 65535"))
}

fn workstation_usage() -> &'static str {
    "Usage: agent-browser install workstation <--dry-run|--apply> [--json] [--dashboard-port <port>] [--guacamole-port <port>]"
}

#[derive(Debug)]
struct InstallPaths {
    root: PathBuf,
    binary: PathBuf,
    support_dir: PathBuf,
    unit_dir: PathBuf,
    guacamole_state_dir: PathBuf,
    guacamole_secret_file: PathBuf,
}

fn workstation_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("AGENT_BROWSER_WORKSTATION_ROOT") {
        let path = PathBuf::from(root);
        if !path.is_absolute() {
            return Err("AGENT_BROWSER_WORKSTATION_ROOT must be an absolute path".to_string());
        }
        return Ok(path);
    }
    dirs::home_dir().ok_or_else(|| "Unable to resolve the current home directory".to_string())
}

fn install_paths(root: &Path) -> InstallPaths {
    InstallPaths {
        root: root.to_path_buf(),
        binary: root.join(".local/bin/agent-browser"),
        support_dir: root
            .join(".local/lib/agent-browser")
            .join(env!("CARGO_PKG_VERSION")),
        unit_dir: root.join(".config/systemd/user"),
        guacamole_state_dir: root.join(".agent-browser/guacamole"),
        guacamole_secret_file: root.join(".agent-browser/guacamole/secrets/guacamole.env"),
    }
}

fn materialize_payload(paths: &InstallPaths, args: &WorkstationInstallArgs) -> Result<(), String> {
    let staging = paths
        .root
        .join(".agent-browser/install-staging")
        .join(env!("CARGO_PKG_VERSION"));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(display_io("clear install staging", &staging))?;
    }

    let result = (|| {
        let staged_binary = staging.join("bin/agent-browser");
        let staged_support = staging.join("support");
        let staged_units = staging.join("units");
        fs::create_dir_all(&staged_support)
            .map_err(display_io("create support staging", &staged_support))?;
        fs::create_dir_all(&staged_units)
            .map_err(display_io("create unit staging", &staged_units))?;
        if let Some(parent) = staged_binary.parent() {
            fs::create_dir_all(parent).map_err(display_io("create binary staging", parent))?;
        }

        let current_exe = env::current_exe()
            .map_err(|error| format!("Unable to resolve current executable: {error}"))?;
        fs::copy(&current_exe, &staged_binary)
            .map_err(display_io("stage agent-browser executable", &staged_binary))?;
        set_executable(&staged_binary)?;

        let manifest = render_manifest(args);
        fs::write(staged_support.join("manifest.json"), manifest)
            .map_err(display_io("stage workstation manifest", &staged_support))?;
        fs::write(
            staged_support.join("README.txt"),
            "Versioned agent-browser workstation support assets.\n",
        )
        .map_err(display_io("stage support readme", &staged_support))?;
        materialize_guacamole_assets(&staged_support)?;

        let final_binary = paths.binary.display().to_string();
        for (name, content) in render_units(&final_binary, args.dashboard_port) {
            fs::write(staged_units.join(name), content)
                .map_err(display_io("stage systemd user unit", &staged_units))?;
        }

        inject_failure("units-staged")?;

        commit_directory(&staged_support, &paths.support_dir)?;
        if let Some(parent) = paths.binary.parent() {
            fs::create_dir_all(parent).map_err(display_io("create binary directory", parent))?;
        }
        replace_file(&staged_binary, &paths.binary)?;
        fs::create_dir_all(&paths.unit_dir)
            .map_err(display_io("create systemd user directory", &paths.unit_dir))?;
        for entry in
            fs::read_dir(&staged_units).map_err(display_io("read staged units", &staged_units))?
        {
            let entry = entry.map_err(|error| format!("Unable to read staged unit: {error}"))?;
            replace_file(&entry.path(), &paths.unit_dir.join(entry.file_name()))?;
        }
        ensure_workstation_state(paths, args)?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

fn materialize_guacamole_assets(staged_support: &Path) -> Result<(), String> {
    let guacamole_dir = staged_support.join("guacamole");
    let init_dir = guacamole_dir.join("init");
    fs::create_dir_all(&init_dir)
        .map_err(display_io("create Guacamole asset staging", &init_dir))?;
    let assets = [
        ("compose.yml", GUACAMOLE_COMPOSE, false),
        ("environment.example", GUACAMOLE_ENVIRONMENT_EXAMPLE, false),
        ("generate-initdb.sh", GUACAMOLE_SCHEMA_GENERATOR, true),
        ("manifest.json", GUACAMOLE_BUNDLE_MANIFEST, false),
        ("init/001-initdb.sql", GUACAMOLE_INITDB, false),
    ];
    for (relative, content, executable) in assets {
        let destination = guacamole_dir.join(relative);
        fs::write(&destination, content)
            .map_err(display_io("stage Guacamole support asset", &destination))?;
        if executable {
            set_executable(&destination)?;
        }
    }
    Ok(())
}

fn ensure_workstation_state(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
) -> Result<(), String> {
    let secrets_dir = paths.guacamole_state_dir.join("secrets");
    for directory in [
        &paths.guacamole_state_dir,
        &secrets_dir,
        &paths.guacamole_state_dir.join("state"),
        &paths.guacamole_state_dir.join("backups"),
    ] {
        fs::create_dir_all(directory)
            .map_err(display_io("create Guacamole state directory", directory))?;
        set_private_directory(directory)?;
    }

    let environment_file = paths.guacamole_state_dir.join(".env");
    fs::write(
        &environment_file,
        format!(
            "AGENT_BROWSER_GUACAMOLE_HTTP_PORT={}\n",
            args.guacamole_port
        ),
    )
    .map_err(display_io(
        "write Guacamole listener environment",
        &environment_file,
    ))?;

    if !paths.guacamole_secret_file.exists() {
        let secret = format!(
            "POSTGRES_PASSWORD={}{}\n",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        fs::write(&paths.guacamole_secret_file, secret).map_err(display_io(
            "write protected Guacamole secrets",
            &paths.guacamole_secret_file,
        ))?;
    }
    set_private_file(&paths.guacamole_secret_file)?;
    Ok(())
}

fn render_manifest(args: &WorkstationInstallArgs) -> String {
    let guacamole_bundle: serde_json::Value = serde_json::from_str(GUACAMOLE_BUNDLE_MANIFEST)
        .expect("embedded Guacamole bundle manifest must be valid JSON");
    serde_json::to_string_pretty(&serde_json::json!({
        "schemaVersion": "agent-browser.workstation-payload.v1",
        "version": env!("CARGO_PKG_VERSION"),
        "dashboardPort": args.dashboard_port,
        "guacamolePort": args.guacamole_port,
        "runtimeController": "installed-binary",
        "sourceCheckoutRequired": false,
        "guacamoleBundle": guacamole_bundle
    }))
    .expect("static workstation manifest must serialize")
}

fn render_units(binary: &str, dashboard_port: u16) -> Vec<(&'static str, String)> {
    vec![
        (
            "agent-browser-dashboard.service",
            format!(
                "[Unit]\nDescription=agent-browser dashboard\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nEnvironment=AGENT_BROWSER_DASHBOARD=1\nEnvironment=AGENT_BROWSER_DASHBOARD_PORT={dashboard_port}\nExecStart={binary}\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
            ),
        ),
        (
            "agent-browser-runtime-interlock.service",
            format!(
                "[Unit]\nDescription=agent-browser runtime health interlock\nAfter=agent-browser-dashboard.service network-online.target\nWants=agent-browser-dashboard.service network-online.target\n\n[Service]\nType=oneshot\nExecStart={binary} install workstation reconcile --json\nTimeoutStartSec=5min\n"
            ),
        ),
        (
            "agent-browser-runtime-interlock.timer",
            "[Unit]\nDescription=Periodically reconcile agent-browser runtime health\n\n[Timer]\nOnBootSec=20s\nOnUnitInactiveSec=5min\nAccuracySec=5s\nPersistent=true\nUnit=agent-browser-runtime-interlock.service\n\n[Install]\nWantedBy=timers.target\n".to_string(),
        ),
        (
            "agent-browser-guacamole-postgres-backup.service",
            format!(
                "[Unit]\nDescription=Back up agent-browser Guacamole PostgreSQL\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={binary} install workstation backup --json\nTimeoutStartSec=10min\n"
            ),
        ),
        (
            "agent-browser-guacamole-postgres-backup.timer",
            "[Unit]\nDescription=Daily agent-browser Guacamole PostgreSQL backup\n\n[Timer]\nOnCalendar=daily\nRandomizedDelaySec=15min\nPersistent=true\nUnit=agent-browser-guacamole-postgres-backup.service\n\n[Install]\nWantedBy=timers.target\n".to_string(),
        ),
    ]
}

fn inject_failure(phase: &str) -> Result<(), String> {
    if env::var("AGENT_BROWSER_WORKSTATION_FAIL_AFTER").as_deref() == Ok(phase) {
        return Err(format!(
            "Injected workstation install failure after {phase}"
        ));
    }
    Ok(())
}

fn commit_directory(staged: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(display_io(
            "replace installed support directory",
            destination,
        ))?;
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(display_io("create installed support parent", parent))?;
    }
    fs::rename(staged, destination).map_err(display_io(
        "commit installed support directory",
        destination,
    ))
}

fn replace_file(staged: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        fs::remove_file(destination).map_err(display_io(
            "replace installed workstation file",
            destination,
        ))?;
    }
    fs::rename(staged, destination)
        .map_err(display_io("commit installed workstation file", destination))
}

fn display_io<'a>(action: &'static str, path: &'a Path) -> impl FnOnce(io::Error) -> String + 'a {
    move |error| format!("Unable to {action} {}: {error}", path.display())
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(display_io("set executable permissions on", path))?;
    }
    Ok(())
}

fn set_private_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(display_io("set private directory permissions on", path))?;
    }
    Ok(())
}

fn set_private_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(display_io("set private file permissions on", path))?;
    }
    Ok(())
}

fn fail(message: &str, json: bool) -> ! {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": message,
            }))
            .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        );
    } else {
        eprintln!("{message}");
    }
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_dry_run() {
        let args = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--json".to_string(),
            "--dashboard-port".to_string(),
            "4949".to_string(),
        ];
        let parsed = parse_workstation_install_args(&args).unwrap();
        assert_eq!(parsed.mode, InstallMode::DryRun);
        assert!(parsed.json);
        assert_eq!(parsed.dashboard_port, 4949);
        assert_eq!(parsed.guacamole_port, DEFAULT_GUACAMOLE_PORT);
    }

    #[test]
    fn requires_exactly_one_mode() {
        let missing = vec!["install".to_string(), "workstation".to_string()];
        assert!(parse_workstation_install_args(&missing)
            .unwrap_err()
            .contains("Choose exactly one"));

        let conflicting = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--apply".to_string(),
        ];
        assert!(parse_workstation_install_args(&conflicting)
            .unwrap_err()
            .contains("mutually exclusive"));
    }

    #[test]
    fn installed_units_are_source_free() {
        let units = render_units("/home/test/.local/bin/agent-browser", 4848);
        for (_, body) in units {
            assert!(!body.contains("pnpm"));
            assert!(!body.contains("WorkingDirectory="));
            assert!(!body.contains("workspace.local"));
        }
    }

    #[test]
    fn manifest_records_binary_owned_runtime() {
        let manifest = render_manifest(&WorkstationInstallArgs {
            mode: InstallMode::Apply,
            json: true,
            dashboard_port: 4848,
            guacamole_port: 8092,
        });
        assert!(manifest.contains(r#""runtimeController": "installed-binary""#));
        assert!(manifest.contains(r#""sourceCheckoutRequired": false"#));
    }
}
