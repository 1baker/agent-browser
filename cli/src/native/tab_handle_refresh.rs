//! Exact-target URL projection. No browser IO or default-runtime access here.

use serde_json::{Map, Value};

use super::service_model::{
    BrowserTab, LeaseState, ProfileOrigin, ServiceEvent, ServiceState, ServiceTabHandle,
    SessionCleanupPolicy,
};
use super::service_store::ServiceStateRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactTabRefreshFence {
    tab: BrowserTab,
    browser_pid: Option<u32>,
    cdp_endpoint: Option<String>,
    executable_path: Option<String>,
    browser_build_proof: Option<Value>,
    lease: LeaseState,
    cleanup: SessionCleanupPolicy,
    session_created_at: Option<String>,
    profile_origin: ProfileOrigin,
    custody_receipt: Option<Value>,
}

/// Derived handles can change when jobs are recorded; they are not tab authority.
pub(crate) fn tab_projection_source(tab: &BrowserTab) -> BrowserTab {
    let mut source = tab.clone();
    source.service_tab_handle = None;
    source
}

pub(crate) fn capture_exact_tab_refresh_fence(
    state: &ServiceState,
    handle: &Map<String, Value>,
    session_id: &str,
) -> Result<ExactTabRefreshFence, String> {
    state
        .validate_diagnostics_handle(handle, session_id)
        .map_err(|reason| reason.replacen("attestation_", "tab_handle_refresh_", 1))?;
    // The validator above requires these fields and their persisted relationships.
    let tab_id = handle["tabId"]
        .as_str()
        .ok_or("tab_handle_refresh_tab_missing")?;
    let current = state
        .service_tab_handle(tab_id)
        .ok_or("tab_handle_refresh_tab_missing")?;
    if !current.valid {
        return Err(format!(
            "tab_handle_refresh_{}",
            current.stale_reason.as_deref().unwrap_or("stale_target")
        ));
    }
    let tab = &state.tabs[tab_id];
    let browser = &state.browsers[&tab.browser_id];
    let session = &state.sessions[session_id];
    let profile_id = current
        .profile_id
        .as_deref()
        .ok_or("tab_handle_refresh_profile_missing")?;
    Ok(ExactTabRefreshFence {
        tab: tab_projection_source(tab),
        browser_pid: browser.pid,
        cdp_endpoint: browser.cdp_endpoint.clone(),
        executable_path: browser.executable_path.clone(),
        browser_build_proof: browser.browser_build_proof.clone(),
        lease: session.lease,
        cleanup: session.cleanup,
        session_created_at: session.created_at.clone(),
        profile_origin: state.profiles[profile_id].profile_origin,
        custody_receipt: state.runtime_custody_receipts.get(session_id).cloned(),
    })
}

pub(crate) fn refresh_rejection_decision(reason: &str) -> &'static str {
    match reason {
        "tab_handle_refresh_tab_missing"
        | "tab_handle_refresh_handle_tab_missing"
        | "tab_handle_refresh_handle_browser_missing"
        | "tab_handle_refresh_tab_closed"
        | "tab_handle_refresh_tab_crashed"
        | "tab_handle_refresh_target_missing" => "rejected_stale_or_missing_target",
        "tab_handle_refresh_concurrent_target_advance" => "rejected_concurrent_target_advance",
        "tab_handle_refresh_observation_failed" => "rejected_observation_failed",
        _ => "rejected_persisted_custody_mismatch",
    }
}

pub(crate) fn commit_exact_tab_refresh(
    repository: &impl ServiceStateRepository,
    handle: &Map<String, Value>,
    session_id: &str,
    fence: &ExactTabRefreshFence,
    observation: Result<(&str, &str), String>,
    event: ServiceEvent,
) -> Result<ServiceTabHandle, String> {
    let (url, title) = observation.map_err(|_| "tab_handle_refresh_observation_failed")?;
    if url.trim().is_empty() {
        return Err("tab_handle_refresh_observation_failed".into());
    }
    repository.mutate(|state| {
        let current = capture_exact_tab_refresh_fence(state, handle, session_id)?;
        if current != *fence {
            // Err aborts save; Ok would still normalize/write the repository.
            return Err("tab_handle_refresh_concurrent_target_advance".into());
        }
        let tab = state
            .tabs
            .get_mut(&fence.tab.id)
            .ok_or("tab_handle_refresh_tab_missing")?;
        tab.url = Some(url.to_string());
        tab.title = Some(title.to_string());
        // Every observation advances the fence, including same-value/ABA refresh.
        // This marker is deliberately separate from runtime custody receipts.
        tab.observation_revision = Some(uuid::Uuid::new_v4().to_string());
        state.events.push(event);
        if state.events.len() > 100 {
            state.events.drain(0..state.events.len() - 100);
        }
        state.refresh_service_tab_handles();
        state
            .service_tab_handle(&fence.tab.id)
            .ok_or_else(|| "tab_handle_refresh_tab_missing".into())
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::service_health::merge_reconciled_service_state;
    use super::super::service_model::{
        BrowserHealth, BrowserProcess, BrowserProfile, BrowserSession, TabLifecycle,
    };
    use super::super::service_store::{JsonServiceStateStore, LockedServiceStateRepository};
    use super::*;
    use serde_json::json;

    pub(crate) struct TestDirectory(std::path::PathBuf);
    impl TestDirectory {
        pub(crate) fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("ab-exact-refresh-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        pub(crate) fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn fixture() -> ServiceState {
        let mut state = ServiceState::default();
        state.profiles.insert(
            "profile".into(),
            BrowserProfile {
                id: "profile".into(),
                ..BrowserProfile::default()
            },
        );
        state.browsers.insert(
            "session:fixture".into(),
            BrowserProcess {
                id: "session:fixture".into(),
                profile_id: Some("profile".into()),
                health: BrowserHealth::Ready,
                pid: Some(42),
                cdp_endpoint: Some("http://fixture.invalid".into()),
                active_session_ids: vec!["fixture".into()],
                ..BrowserProcess::default()
            },
        );
        state.sessions.insert(
            "fixture".into(),
            BrowserSession {
                id: "fixture".into(),
                profile_id: Some("profile".into()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["session:fixture".into()],
                tab_ids: vec!["tab".into(), "peer".into()],
                ..BrowserSession::default()
            },
        );
        for id in ["tab", "peer"] {
            state.tabs.insert(
                id.into(),
                BrowserTab {
                    id: id.into(),
                    browser_id: "session:fixture".into(),
                    target_id: Some(id.into()),
                    owner_session_id: Some("fixture".into()),
                    lifecycle: TabLifecycle::Ready,
                    url: Some(format!("https://example.invalid/{id}")),
                    title: Some(id.into()),
                    ..BrowserTab::default()
                },
            );
        }
        state.refresh_service_tab_handles();
        state
    }

    fn repo() -> (
        TestDirectory,
        LockedServiceStateRepository<JsonServiceStateStore>,
    ) {
        let dir = TestDirectory::new();
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(
            dir.path().join("state.json"),
        ));
        repository
            .mutate(|state| {
                *state = fixture();
                Ok(())
            })
            .unwrap();
        (dir, repository)
    }
    fn handle(state: &ServiceState) -> Map<String, Value> {
        serde_json::to_value(state.service_tab_handle("tab").unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    }
    fn event() -> ServiceEvent {
        ServiceEvent {
            id: "exact-refresh".into(),
            ..ServiceEvent::default()
        }
    }
    fn commit(
        repository: &impl ServiceStateRepository,
        handle: &Map<String, Value>,
        fence: &ExactTabRefreshFence,
    ) -> Result<ServiceTabHandle, String> {
        commit_exact_tab_refresh(
            repository,
            handle,
            "fixture",
            fence,
            Ok(("https://example.invalid/new", "new")),
            event(),
        )
    }

    #[test]
    fn exact_refresh_projection_commits_url_and_preserves_peers_and_custody() {
        let (_dir, repository) = repo();
        let before = repository.load_snapshot().unwrap();
        let handle = handle(&before);
        let fence = capture_exact_tab_refresh_fence(&before, &handle, "fixture").unwrap();
        let refreshed = commit(&repository, &handle, &fence).unwrap();
        let after = repository.load_snapshot().unwrap();
        assert_eq!(after.service_tab_handle("tab").unwrap(), refreshed);
        assert_eq!(
            after.browsers["session:fixture"]
                .tab_handles
                .iter()
                .find(|h| h.tab_id == "tab")
                .unwrap()
                .url
                .as_deref(),
            Some("https://example.invalid/new")
        );
        assert_eq!(after.tabs["peer"], before.tabs["peer"]);
        assert_eq!(after.sessions, before.sessions);
        assert_eq!(after.profiles, before.profiles);
        assert_eq!(
            after.runtime_custody_receipts,
            before.runtime_custody_receipts
        );
        let mut expected = before.clone();
        expected.tabs.get_mut("tab").unwrap().url = refreshed.url;
        expected.tabs.get_mut("tab").unwrap().title = refreshed.title;
        let revision = after.tabs["tab"].observation_revision.as_deref().unwrap();
        uuid::Uuid::parse_str(revision).unwrap();
        expected.tabs.get_mut("tab").unwrap().observation_revision = Some(revision.to_string());
        expected.events.push(event());
        expected.refresh_derived_views();
        assert_eq!(after, expected);
    }

    #[test]
    fn exact_refresh_projection_rejects_foreign_released_and_missing_metadata() {
        let state = fixture();
        let original = handle(&state);
        for (key, value) in [
            ("ownerSessionId", json!("foreign")),
            ("leaseId", json!("foreign")),
            ("sessionName", json!("foreign")),
            ("leaseState", json!("released")),
            ("profileId", json!("foreign")),
            ("targetId", json!("missing")),
            ("valid", json!(false)),
        ] {
            let mut bad = original.clone();
            bad.insert(key.into(), value);
            assert!(
                capture_exact_tab_refresh_fence(&state, &bad, "fixture").is_err(),
                "{key}"
            );
        }
        let mut bad = original.clone();
        bad.remove("ownerSessionId");
        assert!(capture_exact_tab_refresh_fence(&state, &bad, "fixture").is_err());
    }

    #[test]
    fn exact_refresh_projection_rejects_persisted_stale_custody() {
        let state = fixture();
        let handle = handle(&state);
        for edit in 0..6 {
            let mut stale = state.clone();
            match edit {
                0 => stale.sessions.get_mut("fixture").unwrap().lease = LeaseState::Released,
                1 => stale.tabs.get_mut("tab").unwrap().lifecycle = TabLifecycle::Closed,
                2 => {
                    stale.tabs.remove("tab");
                }
                3 => {
                    stale.browsers.clear();
                }
                4 => stale.tabs.get_mut("tab").unwrap().owner_session_id = Some("foreign".into()),
                _ => stale.sessions.get_mut("fixture").unwrap().profile_id = Some("foreign".into()),
            }
            let reason = capture_exact_tab_refresh_fence(&stale, &handle, "fixture").unwrap_err();
            if matches!(edit, 1..=3) {
                assert_eq!(
                    refresh_rejection_decision(&reason),
                    "rejected_stale_or_missing_target"
                );
            }
        }
    }

    #[test]
    fn exact_refresh_projection_concurrent_advancement_aborts_without_save() {
        for edit in 0..5 {
            let (dir, repository) = repo();
            let before = repository.load_snapshot().unwrap();
            let handle = handle(&before);
            let fence = capture_exact_tab_refresh_fence(&before, &handle, "fixture").unwrap();
            repository
                .mutate(|state| {
                    match edit {
                        0 => {
                            state.tabs.get_mut("tab").unwrap().url =
                                Some("https://example.invalid/later".into())
                        }
                        1 => state.tabs.get_mut("tab").unwrap().title = Some("later".into()),
                        2 => state.browsers.get_mut("session:fixture").unwrap().pid = Some(43),
                        3 => {
                            state
                                .runtime_custody_receipts
                                .insert("fixture".into(), json!({"generation":2}));
                        }
                        _ => {
                            state.tabs.get_mut("tab").unwrap().owner_session_id =
                                Some("foreign".into())
                        }
                    }
                    Ok(())
                })
                .unwrap();
            let bytes = std::fs::read(dir.path().join("state.json")).unwrap();
            assert!(commit(&repository, &handle, &fence).is_err());
            assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), bytes);
        }
    }

    #[test]
    fn exact_refresh_projection_failed_observation_does_not_save() {
        let (dir, repository) = repo();
        let state = repository.load_snapshot().unwrap();
        let handle = handle(&state);
        let fence = capture_exact_tab_refresh_fence(&state, &handle, "fixture").unwrap();
        let bytes = std::fs::read(dir.path().join("state.json")).unwrap();
        for observation in [Ok(("", "title")), Err("observation unavailable".into())] {
            assert_eq!(
                commit_exact_tab_refresh(
                    &repository,
                    &handle,
                    "fixture",
                    &fence,
                    observation,
                    event()
                )
                .unwrap_err(),
                "tab_handle_refresh_observation_failed"
            );
            assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), bytes);
        }
    }

    struct FailedRepository;
    impl ServiceStateRepository for FailedRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(fixture())
        }
        fn mutate<R>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            let _ = mutator(&mut fixture())?;
            Err("synthetic_storage_failed".into())
        }
    }
    #[test]
    fn exact_refresh_projection_storage_failure_never_returns_success_handle() {
        let state = fixture();
        let handle = handle(&state);
        let fence = capture_exact_tab_refresh_fence(&state, &handle, "fixture").unwrap();
        assert_eq!(
            commit(&FailedRepository, &handle, &fence).unwrap_err(),
            "synthetic_storage_failed"
        );
    }

    #[test]
    fn exact_refresh_projection_same_url_observation_fences_delayed_reconciliation() {
        let (_dir, repository) = repo();
        let before = repository.load_snapshot().unwrap();
        // A legacy record without the optional marker loads successfully.
        assert!(before.tabs["tab"].observation_revision.is_none());
        assert!(serde_json::to_value(&before.tabs["tab"])
            .unwrap()
            .get("observationRevision")
            .is_none());
        let handle = handle(&before);
        let fence = capture_exact_tab_refresh_fence(&before, &handle, "fixture").unwrap();
        let returned = commit_exact_tab_refresh(
            &repository,
            &handle,
            "fixture",
            &fence,
            Ok((
                before.tabs["tab"].url.as_deref().unwrap(),
                before.tabs["tab"].title.as_deref().unwrap(),
            )),
            event(),
        )
        .unwrap();
        // Observation marker is not exposed as authority in the derived handle.
        assert_eq!(returned, before.service_tab_handle("tab").unwrap());
        assert!(serde_json::to_value(&returned)
            .unwrap()
            .get("observationRevision")
            .is_none());
        let committed = repository.load_snapshot().unwrap();
        let revision = committed.tabs["tab"].observation_revision.clone().unwrap();
        uuid::Uuid::parse_str(&revision).unwrap();
        let mut delayed = before.clone();
        delayed.tabs.get_mut("tab").unwrap().url = Some("https://example.invalid/older".into());
        repository
            .mutate(|state| {
                merge_reconciled_service_state(state, &before, &delayed);
                Ok(())
            })
            .unwrap();
        let after = repository.load_snapshot().unwrap();
        assert_eq!(after.tabs["tab"], committed.tabs["tab"]);
        assert_eq!(after.tabs["peer"], committed.tabs["peer"]);
    }

    #[test]
    fn exact_refresh_projection_second_refresh_cannot_use_old_same_url_fence() {
        let (dir, repository) = repo();
        let before = repository.load_snapshot().unwrap();
        let handle = handle(&before);
        let fence = capture_exact_tab_refresh_fence(&before, &handle, "fixture").unwrap();
        let observation = Ok((
            before.tabs["tab"].url.as_deref().unwrap(),
            before.tabs["tab"].title.as_deref().unwrap(),
        ));
        commit_exact_tab_refresh(
            &repository,
            &handle,
            "fixture",
            &fence,
            observation.clone(),
            event(),
        )
        .unwrap();
        let bytes = std::fs::read(dir.path().join("state.json")).unwrap();
        assert_eq!(
            commit_exact_tab_refresh(
                &repository,
                &handle,
                "fixture",
                &fence,
                observation,
                event()
            )
            .unwrap_err(),
            "tab_handle_refresh_concurrent_target_advance"
        );
        assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), bytes);
    }

    #[test]
    fn exact_refresh_projection_delayed_reconcile_preserves_new_url_and_reconciles_peer() {
        let (_dir, repository) = repo();
        let before = repository.load_snapshot().unwrap();
        let handle = handle(&before);
        let fence = capture_exact_tab_refresh_fence(&before, &handle, "fixture").unwrap();
        commit(&repository, &handle, &fence).unwrap();
        let mut reconciled = before.clone();
        reconciled.tabs.get_mut("tab").unwrap().url =
            Some("https://example.invalid/old-observation".into());
        reconciled.tabs.get_mut("peer").unwrap().url =
            Some("https://example.invalid/peer-current".into());
        // Rebuilding a derived handle alone must not inhibit reconciliation.
        repository
            .mutate(|state| {
                state.tabs.get_mut("peer").unwrap().service_tab_handle = None;
                merge_reconciled_service_state(state, &before, &reconciled);
                Ok(())
            })
            .unwrap();
        let after = repository.load_snapshot().unwrap();
        assert_eq!(
            after.tabs["tab"].url.as_deref(),
            Some("https://example.invalid/new")
        );
        assert_eq!(
            after.tabs["peer"].url.as_deref(),
            Some("https://example.invalid/peer-current")
        );
    }

    #[test]
    fn exact_refresh_projection_delayed_reconcile_cannot_resurrect_or_reassign_tab() {
        for remove in [true, false] {
            let before = fixture();
            let mut current = before.clone();
            if remove {
                current.tabs.remove("tab");
            } else {
                current.tabs.get_mut("tab").unwrap().owner_session_id = Some("foreign".into());
            }
            merge_reconciled_service_state(&mut current, &before, &before);
            if remove {
                assert!(!current.tabs.contains_key("tab"));
            } else {
                assert_eq!(
                    current.tabs["tab"].owner_session_id.as_deref(),
                    Some("foreign")
                );
            }
        }
    }
}
