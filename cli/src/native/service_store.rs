//! Persistent service-state storage.
//!
//! The first service-mode store is JSON-backed and intentionally small. It gives
//! later lifecycle work a durable contract without forcing a database choice yet.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use super::service_model::{RemoteViewHandoff, ServiceState};

const SERVICE_DIR: &str = "service";
const SERVICE_STATE_FILENAME: &str = "state.json";
const REMOTE_VIEW_HANDOFFS_FILENAME: &str = "remote-view-handoffs.json";
const REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION: &str = "agent-browser.remote-view-handoffs.v1";
static SERVICE_STATE_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoteViewHandoffRegistry {
    schema_version: String,
    handoffs: BTreeMap<String, RemoteViewHandoff>,
}

pub trait ServiceStateStore {
    fn load(&self) -> Result<ServiceState, String>;
    fn save(&self, state: &ServiceState) -> Result<(), String>;

    fn state_path(&self) -> Option<&Path> {
        None
    }
}

pub trait ServiceStateRepository {
    fn load_snapshot(&self) -> Result<ServiceState, String>;
    fn mutate<R>(
        &self,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String>;
}

#[derive(Debug, Clone)]
pub struct JsonServiceStateStore {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LockedServiceStateRepository<S> {
    store: S,
}

impl JsonServiceStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> Result<PathBuf, String> {
        default_service_state_path()
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }
}

impl<S> LockedServiceStateRepository<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl LockedServiceStateRepository<JsonServiceStateStore> {
    pub fn default_json() -> Result<Self, String> {
        Ok(Self::new(JsonServiceStateStore::new(
            default_service_state_path()?,
        )))
    }
}

impl ServiceStateStore for JsonServiceStateStore {
    fn load(&self) -> Result<ServiceState, String> {
        let mut state_file_missing = false;
        let mut state = match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str(&raw).map_err(|err| {
                format!(
                    "Invalid service state JSON {}: {}",
                    self.path.display(),
                    err
                )
            })?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                state_file_missing = true;
                ServiceState::default()
            }
            Err(err) => {
                return Err(format!(
                    "Failed to read service state {}: {}",
                    self.path.display(),
                    err
                ))
            }
        };

        let handoff_registry = load_remote_view_handoff_registry(&self.path)?;
        if state_file_missing && handoff_registry.handoffs.is_empty() {
            return Ok(ServiceState::default());
        }
        state.remote_view_handoffs.extend(handoff_registry.handoffs);
        state.mark_persisted_entity_sources();
        state.refresh_derived_views();
        Ok(state)
    }

    fn save(&self, state: &ServiceState) -> Result<(), String> {
        let mut normalized = state.clone();
        normalized.refresh_derived_views();
        normalized.remove_builtin_entity_defaults_for_persistence();
        save_remote_view_handoff_registry(&self.path, &normalized.remote_view_handoffs)?;
        let serialized = serde_json::to_string_pretty(&normalized)
            .map_err(|err| format!("Failed to serialize service state: {}", err))?;
        let payload = format!("{}\n", serialized);
        let temp_path = temp_state_path(&self.path);

        for attempt in 0..2 {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "Failed to create service state directory {}: {}",
                        parent.display(),
                        err
                    )
                })?;
            }

            fs::write(&temp_path, &payload).map_err(|err| {
                format!(
                    "Failed to write temporary service state {}: {}",
                    temp_path.display(),
                    err
                )
            })?;

            match fs::rename(&temp_path, &self.path) {
                Ok(()) => return Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound && attempt == 0 => {
                    let _ = fs::remove_file(&temp_path);
                    continue;
                }
                Err(err) => {
                    let _ = fs::remove_file(&temp_path);
                    return Err(format!(
                        "Failed to replace service state {}: {}",
                        self.path.display(),
                        err
                    ));
                }
            }
        }

        Ok(())
    }

    fn state_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

impl<S> ServiceStateRepository for LockedServiceStateRepository<S>
where
    S: ServiceStateStore,
{
    fn load_snapshot(&self) -> Result<ServiceState, String> {
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock
            .lock()
            .map_err(|_| "Service state mutation lock was poisoned".to_string())?;
        let _file_guard = self
            .store
            .state_path()
            .map(|path| acquire_service_state_file_lock(path, ServiceStateFileLockMode::Shared))
            .transpose()?;
        self.store.load()
    }

    fn mutate<R>(
        &self,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String> {
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock
            .lock()
            .map_err(|_| "Service state mutation lock was poisoned".to_string())?;
        let _file_guard = self
            .store
            .state_path()
            .map(|path| acquire_service_state_file_lock(path, ServiceStateFileLockMode::Exclusive))
            .transpose()?;
        let mut state = self.store.load()?;
        let result = mutator(&mut state)?;
        self.store.save(&state)?;
        Ok(result)
    }
}

pub fn default_service_state_path() -> Result<PathBuf, String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Could not determine home directory for service state".to_string());
    };
    Ok(home
        .join(".agent-browser")
        .join(SERVICE_DIR)
        .join(SERVICE_STATE_FILENAME))
}

/// Load a stable point-in-time snapshot of the default JSON service state.
///
/// Readers take the same mutex as mutators so they do not observe a snapshot
/// while a serialized read-modify-write operation is in progress. This does
/// not make the snapshot live after it is returned; callers that later write
/// must still use merge-aware mutation helpers.
pub fn load_default_service_state_snapshot() -> Result<ServiceState, String> {
    LockedServiceStateRepository::default_json()?.load_snapshot()
}

/// Serialize read-modify-write operations against the default JSON service state.
///
/// The JSON store is intentionally simple, but callers that mutate state must
/// not race independent load/save cycles. This helper provides the narrow
/// service-state control point used by queued service mutations and job audit
/// updates until a dedicated service database exists.
pub fn mutate_default_service_state<R>(
    mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
) -> Result<R, String> {
    LockedServiceStateRepository::default_json()?.mutate(mutator)
}

fn temp_state_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    path.with_file_name(format!("{}.tmp.{}", file_name, std::process::id()))
}

fn remote_view_handoff_registry_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(REMOTE_VIEW_HANDOFFS_FILENAME)
}

fn load_remote_view_handoff_registry(
    state_path: &Path,
) -> Result<RemoteViewHandoffRegistry, String> {
    let path = remote_view_handoff_registry_path(state_path);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RemoteViewHandoffRegistry::default())
        }
        Err(err) => {
            return Err(format!(
                "Failed to read remote-view handoff registry {}: {}",
                path.display(),
                err
            ))
        }
    };
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid remote-view handoff registry JSON {}: {}",
            path.display(),
            err
        )
    })
}

fn save_remote_view_handoff_registry(
    state_path: &Path,
    handoffs: &BTreeMap<String, RemoteViewHandoff>,
) -> Result<(), String> {
    let path = remote_view_handoff_registry_path(state_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create remote-view handoff registry directory {}: {}",
                parent.display(),
                err
            )
        })?;
    }
    let registry = RemoteViewHandoffRegistry {
        schema_version: REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION.to_string(),
        handoffs: handoffs.clone(),
    };
    let payload = format!(
        "{}\n",
        serde_json::to_string_pretty(&registry)
            .map_err(|err| format!("Failed to serialize remote-view handoff registry: {err}"))?
    );
    let temp_path = temp_state_path(&path);
    fs::write(&temp_path, payload).map_err(|err| {
        format!(
            "Failed to write temporary remote-view handoff registry {}: {}",
            temp_path.display(),
            err
        )
    })?;
    fs::rename(&temp_path, &path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!(
            "Failed to replace remote-view handoff registry {}: {}",
            path.display(),
            err
        )
    })
}

#[derive(Debug, Clone, Copy)]
enum ServiceStateFileLockMode {
    Shared,
    Exclusive,
}

fn service_state_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    path.with_file_name(format!("{file_name}.lock"))
}

fn acquire_service_state_file_lock(
    state_path: &Path,
    mode: ServiceStateFileLockMode,
) -> Result<File, String> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create service state directory {}: {}",
                parent.display(),
                err
            )
        })?;
    }
    let lock_path = service_state_lock_path(state_path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|err| {
            format!(
                "Failed to open service state lock {}: {}",
                lock_path.display(),
                err
            )
        })?;
    let result = match mode {
        ServiceStateFileLockMode::Shared => file.lock_shared(),
        ServiceStateFileLockMode::Exclusive => file.lock(),
    };
    result.map_err(|err| {
        format!(
            "Failed to acquire service state lock {}: {}",
            lock_path.display(),
            err
        )
    })?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserHost, BrowserProcess, RemoteViewHandoff, SitePolicy,
    };
    use std::collections::BTreeMap;

    fn unique_state_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("state.json")
    }

    #[test]
    fn missing_state_file_loads_default_state() {
        let store = JsonServiceStateStore::new(unique_state_path("missing-service-state"));

        let state = store.load().expect("missing state should load default");

        assert_eq!(state, ServiceState::default());
    }

    #[test]
    fn save_and_load_round_trips_service_state() {
        let path = unique_state_path("round-trip-service-state");
        let store = JsonServiceStateStore::new(&path);
        let state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    host: BrowserHost::DockerHeaded,
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "https://accounts.google.com".to_string(),
                    manual_login_preferred: true,
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        store.save(&state).expect("state should save");
        let loaded = store.load().expect("state should load");

        assert_eq!(loaded.browsers, state.browsers);
        assert_eq!(
            loaded.site_policies["google"].origin_pattern,
            "https://accounts.google.com"
        );
        assert_eq!(
            loaded.site_policy_source("google"),
            Some(super::super::service_model::ServiceEntitySource::PersistedState)
        );
        assert_eq!(
            loaded.site_policy_source("microsoft"),
            Some(super::super::service_model::ServiceEntitySource::Builtin)
        );
        assert_eq!(store.path(), path.as_path());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn locked_repository_mutates_and_loads_snapshot() {
        let path = unique_state_path("repository-service-state");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));

        let result = repository
            .mutate(|state| {
                state.browsers.insert(
                    "browser-1".to_string(),
                    BrowserProcess {
                        id: "browser-1".to_string(),
                        host: BrowserHost::LocalHeaded,
                        health: BrowserHealth::Ready,
                        ..BrowserProcess::default()
                    },
                );
                Ok("mutated")
            })
            .expect("repository mutation should save");

        let state = repository
            .load_snapshot()
            .expect("repository snapshot should load");

        assert_eq!(result, "mutated");
        assert_eq!(state.browsers["browser-1"].health, BrowserHealth::Ready);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn service_state_file_lock_excludes_an_independent_writer() {
        let path = unique_state_path("cross-process-service-state-lock");
        let first = acquire_service_state_file_lock(&path, ServiceStateFileLockMode::Exclusive)
            .expect("first writer should acquire the service-state lock");
        let lock_path = service_state_lock_path(&path);
        let second = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("second writer should open the stable lock file");

        let error = second
            .try_lock()
            .expect_err("an independent writer must not enter while the lock is held");

        assert!(matches!(error, std::fs::TryLockError::WouldBlock));
        drop(first);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn durable_remote_view_handoffs_survive_a_legacy_state_writer() {
        let path = unique_state_path("legacy-writer-remote-view-handoff");
        let store = JsonServiceStateStore::new(&path);
        let handoff = RemoteViewHandoff {
            id: "handoff-a".to_string(),
            state: "ready".to_string(),
            browser_id: Some("browser-a".to_string()),
            session_name: Some("session-a".to_string()),
            view_stream_provider: Some(
                crate::native::service_model::ViewStreamProvider::RdpGateway,
            ),
            ..RemoteViewHandoff::default()
        };
        store
            .save(&ServiceState {
                remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
                ..ServiceState::default()
            })
            .expect("handoff should save");

        let mut legacy_state: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("saved state should be readable"),
        )
        .expect("saved state should be JSON");
        legacy_state
            .as_object_mut()
            .expect("service state should be an object")
            .remove("remoteViewHandoffs");
        fs::write(
            &path,
            serde_json::to_string_pretty(&legacy_state).expect("legacy state should serialize"),
        )
        .expect("legacy writer should replace the primary state file");

        let loaded = store
            .load()
            .expect("state should load after legacy rewrite");

        assert_eq!(loaded.remote_view_handoffs["handoff-a"].id, "handoff-a");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn invalid_state_file_returns_error() {
        let path = unique_state_path("bad-service-state");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{not-json").unwrap();
        let store = JsonServiceStateStore::new(&path);

        let err = store.load().expect_err("invalid state should fail");

        assert!(err.contains("Invalid service state JSON"));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_backfills_derived_incidents() {
        let path = unique_state_path("service-state-incidents");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{
  "events": [
    {
      "id": "event-1",
      "timestamp": "2026-04-22T00:00:00Z",
      "kind": "reconciliation_error",
      "message": "Failed to reconcile service state"
    }
  ]
}"#,
        )
        .unwrap();
        let store = JsonServiceStateStore::new(&path);

        let state = store.load().expect("state should load");

        assert_eq!(
            state.incidents,
            vec![crate::native::service_model::ServiceIncident {
                id: "service".to_string(),
                label: "Service incidents".to_string(),
                state: crate::native::service_model::ServiceIncidentState::Service,
                severity: crate::native::service_model::ServiceIncidentSeverity::Error,
                escalation: crate::native::service_model::ServiceIncidentEscalation::ServiceTriage,
                recommended_action: "Inspect service logs, reconciliation state, and recent jobs."
                    .to_string(),
                latest_timestamp: "2026-04-22T00:00:00Z".to_string(),
                latest_message: "Failed to reconcile service state".to_string(),
                latest_kind: "reconciliation_error".to_string(),
                event_ids: vec!["event-1".to_string()],
                ..crate::native::service_model::ServiceIncident::default()
            }]
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
