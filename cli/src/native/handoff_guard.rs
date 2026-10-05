//! Executable-handoff downgrade guard.
//!
//! A client whose executable differs from the running daemon's asks the daemon to
//! `runtime_handoff_prepare`, then spawns a replacement daemon from its *own*
//! executable. After a guarded install, a long-lived client still running an older
//! build could therefore replace the freshly installed daemon with the old build.
//!
//! The guard requires explicit authorization for every `runtime_handoff_prepare`,
//! including when a rename has superseded the running daemon. The publisher
//! prepares before it installs and carries the explicit
//! JSON boolean `allowExecutableSidegrade: true`, which clients send only when
//! `AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE` is exactly `1`. Older clients never
//! send it.
//!
//! Limitation: while no daemon is running, a legacy client can cold-start its
//! own old daemon. Quiescence during publication remains a required gate.

use serde_json::Value;

/// Stable refusal codes (prefixes of the returned error).
pub(crate) const REFUSED_CURRENT_INSTALLED: &str =
    "runtime_handoff_refused_current_installed_executable";
pub(crate) const REFUSED_IDENTITY_UNAVAILABLE: &str =
    "runtime_handoff_refused_executable_identity_unavailable";
pub(crate) const REFUSED_SIDEGRADE_NOT_AUTHORIZED: &str =
    "runtime_handoff_refused_sidegrade_not_authorized";
/// Command field carrying an explicit sidegrade authorization (JSON `true` only).
pub(crate) const SIDEGRADE_FIELD: &str = "allowExecutableSidegrade";
/// Client-side opt-in; only the exact value `1` enables the field.
pub(crate) const SIDEGRADE_ENV: &str = "AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutableIdentity {
    /// The running daemon is still the file installed at its path.
    CurrentInstalled,
    /// The install path now holds a different file, or none.
    Superseded,
}

/// Inspect this process's executable. Non-Linux keeps the previous behavior.
#[cfg(target_os = "linux")]
pub(crate) fn inspect_running_executable() -> Result<ExecutableIdentity, String> {
    use std::os::unix::fs::MetadataExt;
    let proc_exe = std::path::Path::new("/proc/self/exe");
    // stat on /proc/self/exe resolves to the running inode even if it was unlinked.
    let running = std::fs::metadata(proc_exe)
        .map_err(|error| format!("cannot inspect the running executable: {error}"))?;
    let link = std::fs::read_link(proc_exe)
        .map_err(|error| format!("cannot resolve the running executable path: {error}"))?;
    classify(
        (running.dev(), running.ino()),
        &link.to_string_lossy(),
        |path| std::fs::metadata(path).map(|meta| (meta.dev(), meta.ino())),
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn inspect_running_executable() -> Result<ExecutableIdentity, String> {
    Ok(ExecutableIdentity::Superseded)
}

/// Classify from the running inode identity and the kernel's link text. On Linux,
/// an unlinked executable's link text ends with " (deleted)"; that suffix is removed
/// to recover the install path. Only `NotFound` counts as superseded; any other
/// inspection error fails closed.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn classify(
    running: (u64, u64),
    link_text: &str,
    stat: impl Fn(&std::path::Path) -> std::io::Result<(u64, u64)>,
) -> Result<ExecutableIdentity, String> {
    let install_path = link_text.strip_suffix(" (deleted)").unwrap_or(link_text);
    match stat(std::path::Path::new(install_path)) {
        Ok(installed) if installed == running => Ok(ExecutableIdentity::CurrentInstalled),
        Ok(_) => Ok(ExecutableIdentity::Superseded),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ExecutableIdentity::Superseded)
        }
        Err(error) => Err(format!(
            "cannot inspect the installed executable {install_path}: {error}"
        )),
    }
}

/// Decide whether `runtime_handoff_prepare` may proceed. Must run before any
/// preparation side effect. Inspection failure refuses even with authorization.
pub(crate) fn authorize_prepare(
    identity: Result<ExecutableIdentity, String>,
    cmd: &Value,
) -> Result<(), String> {
    match identity {
        Err(error) => Err(format!("{REFUSED_IDENTITY_UNAVAILABLE}: {error}")),
        Ok(ExecutableIdentity::Superseded | ExecutableIdentity::CurrentInstalled)
            if cmd.get(SIDEGRADE_FIELD) == Some(&Value::Bool(true)) =>
        {
            Ok(())
        }
        Ok(ExecutableIdentity::CurrentInstalled) => Err(format!(
            "{REFUSED_CURRENT_INSTALLED}: this daemon is still the installed executable, so a \
             handoff would replace it with the requesting client's different executable \
             (possible downgrade). Use the publisher or converge, or set \
             {SIDEGRADE_ENV}=1 for one intentional sidegrade."
        )),
        Ok(ExecutableIdentity::Superseded) => Err(format!(
            "{REFUSED_SIDEGRADE_NOT_AUTHORIZED}: the running daemon was superseded, but \
             this client did not authorize a handoff. Use the publisher or converge, \
             or set {SIDEGRADE_ENV}=1 for one intentional sidegrade."
        )),
    }
}

/// Client side: the exact env value `1` requests an explicit sidegrade.
pub(crate) fn sidegrade_requested() -> bool {
    sidegrade_from(std::env::var(SIDEGRADE_ENV).ok().as_deref())
}

fn sidegrade_from(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Build the prepare command; the field is present only when requested.
pub(crate) fn prepare_command(id: Value, allow_sidegrade: bool) -> Value {
    let mut command = serde_json::json!({ "id": id, "action": "runtime_handoff_prepare" });
    if allow_sidegrade {
        command[SIDEGRADE_FIELD] = Value::Bool(true);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[cfg(target_os = "linux")]
    use std::io::{Error, ErrorKind};

    #[cfg(target_os = "linux")]
    fn running_of(path: &std::path::Path) -> (u64, u64) {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path).unwrap();
        (meta.dev(), meta.ino())
    }

    #[cfg(target_os = "linux")]
    fn stat_real(path: &std::path::Path) -> std::io::Result<(u64, u64)> {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path).map(|meta| (meta.dev(), meta.ino()))
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn handoff_guard_classifies_current_superseded_deleted_and_errors() {
        let dir = std::env::temp_dir().join(format!("ab-handoff-guard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let installed = dir.join("agent-browser");
        std::fs::write(&installed, b"old").unwrap();
        let running = running_of(&installed);
        let text = installed.to_string_lossy().to_string();
        assert_eq!(
            classify(running, &text, stat_real).unwrap(),
            ExecutableIdentity::CurrentInstalled
        );
        // Publisher-style replacement: stage then rename gives a new inode at the same path.
        let staged = dir.join(".staged");
        std::fs::write(&staged, b"new").unwrap();
        std::fs::rename(&staged, &installed).unwrap();
        assert_eq!(
            classify(running, &text, stat_real).unwrap(),
            ExecutableIdentity::Superseded
        );
        // Unlinked running executable: kernel link text ends with " (deleted)".
        std::fs::remove_file(&installed).unwrap();
        assert_eq!(
            classify(running, &format!("{text} (deleted)"), stat_real).unwrap(),
            ExecutableIdentity::Superseded
        );
        // Any inspection error other than NotFound fails closed.
        let denied = classify(running, &text, |_| {
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert!(denied
            .unwrap_err()
            .contains("cannot inspect the installed executable"));
    }

    #[test]
    fn handoff_guard_authorization_is_exact_and_fails_closed() {
        let current = || Ok(ExecutableIdentity::CurrentInstalled);
        let refused = authorize_prepare(current(), &json!({"action": "runtime_handoff_prepare"}));
        assert!(refused.unwrap_err().starts_with(REFUSED_CURRENT_INSTALLED));
        for value in [
            json!(false),
            json!("true"),
            json!(1),
            json!(null),
            json!({"x": true}),
        ] {
            let cmd = json!({ "action": "runtime_handoff_prepare", SIDEGRADE_FIELD: value });
            assert!(authorize_prepare(current(), &cmd).is_err(), "{cmd}");
        }
        let authorized = json!({ "action": "runtime_handoff_prepare", SIDEGRADE_FIELD: true });
        assert!(authorize_prepare(current(), &authorized).is_ok());
        assert!(authorize_prepare(Ok(ExecutableIdentity::Superseded), &authorized).is_ok());
        assert!(
            authorize_prepare(Ok(ExecutableIdentity::Superseded), &json!({}))
                .unwrap_err()
                .starts_with(REFUSED_SIDEGRADE_NOT_AUTHORIZED)
        );
        // Inspection failure refuses even when authorization is present.
        let failed = authorize_prepare(Err("boom".into()), &authorized);
        assert!(failed
            .unwrap_err()
            .starts_with(REFUSED_IDENTITY_UNAVAILABLE));
    }

    #[test]
    fn handoff_guard_env_value_must_be_exactly_one() {
        assert!(sidegrade_from(Some("1")));
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("yes"),
            Some(" 1"),
            Some("1 "),
            Some("01"),
        ] {
            assert!(!sidegrade_from(value), "{value:?}");
        }
    }

    #[test]
    fn handoff_guard_prepare_command_adds_field_only_when_requested() {
        let plain = prepare_command(json!("a"), false);
        assert_eq!(plain["action"], "runtime_handoff_prepare");
        assert!(plain.get(SIDEGRADE_FIELD).is_none());
        let authorized = prepare_command(json!("a"), true);
        assert_eq!(authorized[SIDEGRADE_FIELD], Value::Bool(true));
    }
}
