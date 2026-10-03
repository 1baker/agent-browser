//! Read-only task workbench projection over retained service evidence.
//!
//! This is intentionally a recent-work view. Jobs and events are capped, so a
//! successful browser command cannot be promoted to a durable task outcome.

use serde_json::{json, Value};

use super::service_model::{
    ChallengeState, JobState, JobTarget, ProfileSeedingHandoffState, ServiceState, TabLifecycle,
};

pub fn service_runs_response(state: &ServiceState) -> Value {
    let mut runs = Vec::<Value>::new();

    for job in state.jobs.values() {
        let Some(task_name) = job.task_name.as_deref() else {
            continue;
        };
        let (run_state, next_action) = match job.state {
            JobState::Queued | JobState::WaitingProfileLease => ("queued", "wait_for_dispatch"),
            JobState::Running => ("working", "wait_for_result"),
            JobState::Succeeded => ("activity_succeeded", "verify_task_outcome"),
            JobState::Failed | JobState::Cancelled | JobState::TimedOut => {
                ("attention", "inspect_failure")
            }
        };
        let (browser_id, tab_id, profile_id) = match &job.target {
            JobTarget::Browser(id) => (Some(id.as_str()), None, None),
            JobTarget::Tab(id) => (None, Some(id.as_str()), None),
            JobTarget::Profile(id) => (None, None, Some(id.as_str())),
            _ => (None, None, None),
        };
        let tab = tab_id.and_then(|id| state.tabs.get(id));
        let live_tab = tab.filter(|tab| {
            matches!(
                tab.lifecycle,
                TabLifecycle::Opening | TabLifecycle::Loading | TabLifecycle::Ready
            ) && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                handle.valid
                    && handle.browser_id == tab.browser_id
                    && handle.tab_id == tab.id
                    && handle.target_id == tab.target_id
            })
        });
        let handle = live_tab.and_then(|tab| tab.service_tab_handle.as_ref());
        runs.push(json!({
            "id": format!("job:{}", job.id),
            "kind": "recent_job",
            "state": run_state,
            "nextAction": next_action,
            "taskOutcomeVerified": false,
            "evidenceScope": "bounded_job_log",
            "taskName": task_name,
            "serviceName": job.service_name,
            "agentName": job.agent_name,
            "jobId": job.id,
            "action": job.action,
            "browserId": live_tab.map(|tab| tab.browser_id.as_str()).or(browser_id),
            "tabId": tab_id,
            "profileId": handle.and_then(|handle| handle.profile_id.as_deref()).or(profile_id),
            "targetId": live_tab.and_then(|tab| tab.target_id.as_deref()),
            "sessionName": handle.and_then(|handle| handle.session_name.as_deref()),
            "serviceTabHandle": handle,
            "url": tab.and_then(|tab| tab.url.as_deref()),
            "updatedAt": job.completed_at.as_ref().or(job.started_at.as_ref()).or(job.submitted_at.as_ref()),
        }));
    }

    for challenge in state.challenges.values() {
        if !matches!(challenge.state, ChallengeState::WaitingForHuman) {
            continue;
        }
        let tab = challenge
            .tab_id
            .as_ref()
            .and_then(|tab_id| state.tabs.get(tab_id));
        let live_tab = tab.filter(|tab| {
            matches!(
                tab.lifecycle,
                TabLifecycle::Opening | TabLifecycle::Loading | TabLifecycle::Ready
            ) && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                handle.valid
                    && handle.browser_id == tab.browser_id
                    && handle.tab_id == tab.id
                    && handle.target_id == tab.target_id
            })
        });
        let handle = live_tab.and_then(|tab| tab.service_tab_handle.as_ref());
        runs.push(json!({
            "id": format!("challenge:{}", challenge.id),
            "kind": "human_challenge",
            "state": "needs_human",
            "nextAction": if live_tab.is_some() { "open_exact_tab_and_recheck" } else { "inspect_stale_handoff" },
            "taskOutcomeVerified": false,
            "evidenceScope": "retained_challenge",
            "challengeId": challenge.id,
            "tabId": challenge.tab_id,
            "browserId": tab.map(|tab| &tab.browser_id),
            "profileId": handle.and_then(|handle| handle.profile_id.as_ref()),
            "targetId": live_tab.and_then(|tab| tab.target_id.as_ref()),
            "sessionName": handle.and_then(|handle| handle.session_name.as_ref()),
            "serviceTabHandle": handle,
            "gatePolicyDecision": challenge.policy_decision,
            "url": tab.and_then(|tab| tab.url.as_ref()),
            "updatedAt": challenge.detected_at,
        }));
    }

    for handoff in state.profile_seeding_handoffs.values() {
        if matches!(
            handoff.state,
            ProfileSeedingHandoffState::Fresh | ProfileSeedingHandoffState::NotRequired
        ) {
            continue;
        }
        let next_action = match handoff.state {
            ProfileSeedingHandoffState::NeedsManualSeeding => "start_manual_seeding",
            ProfileSeedingHandoffState::SeedingLaunchedDetached
            | ProfileSeedingHandoffState::SeedingWaitingForClose
            | ProfileSeedingHandoffState::CompletionDeclaredWaitingForClose => {
                "finish_sign_in_and_close_browser"
            }
            ProfileSeedingHandoffState::SeedingClosedUnverified
            | ProfileSeedingHandoffState::VerificationPending => "verify_login_readiness",
            ProfileSeedingHandoffState::Failed | ProfileSeedingHandoffState::Abandoned => {
                "inspect_handoff_failure"
            }
            ProfileSeedingHandoffState::Fresh | ProfileSeedingHandoffState::NotRequired => {
                unreachable!()
            }
        };
        runs.push(json!({
            "id": format!("seeding:{}", handoff.id),
            "kind": "manual_sign_in",
            "state": if matches!(handoff.state, ProfileSeedingHandoffState::Failed | ProfileSeedingHandoffState::Abandoned) { "attention" } else { "needs_human" },
            "nextAction": next_action,
            "taskOutcomeVerified": false,
            "evidenceScope": "profile_seeding_handoff",
            "profileId": handoff.profile_id,
            "targetServiceId": handoff.target_service_id,
            "handoffId": handoff.id,
            "updatedAt": handoff.updated_at,
        }));
    }

    runs.sort_by(|left, right| {
        right["updatedAt"]
            .as_str()
            .unwrap_or_default()
            .cmp(left["updatedAt"].as_str().unwrap_or_default())
            .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
    });
    json!({
        "runs": runs,
        "count": runs.len(),
        "coverage": "recent_jobs_and_retained_human_gates",
        "durableTaskHistory": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserTab, Challenge, ProfileSeedingHandoffRecord, ServiceJob, ServiceTabHandle,
    };
    use std::collections::BTreeMap;

    #[test]
    fn successful_browser_command_does_not_claim_task_completion() {
        let state = ServiceState {
            jobs: BTreeMap::from([(
                "job-1".to_string(),
                ServiceJob {
                    id: "job-1".to_string(),
                    action: "navigate".to_string(),
                    task_name: Some("find article".to_string()),
                    state: JobState::Succeeded,
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };
        let response = service_runs_response(&state);
        assert_eq!(response["count"], 1);
        assert_eq!(response["runs"][0]["state"], "activity_succeeded");
        assert_eq!(response["runs"][0]["taskOutcomeVerified"], false);
    }

    #[test]
    fn human_gate_keeps_exact_tab_and_profile_identity() {
        let state = ServiceState {
            challenges: BTreeMap::from([(
                "check-1".to_string(),
                Challenge {
                    id: "check-1".to_string(),
                    tab_id: Some("tab-1".to_string()),
                    state: ChallengeState::WaitingForHuman,
                    ..Challenge::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    target_id: Some("target-1".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    service_tab_handle: Some(ServiceTabHandle {
                        browser_id: "browser-1".to_string(),
                        tab_id: "tab-1".to_string(),
                        profile_id: Some("profile-1".to_string()),
                        target_id: Some("target-1".to_string()),
                        valid: true,
                        ..ServiceTabHandle::default()
                    }),
                    ..BrowserTab::default()
                },
            )]),
            profile_seeding_handoffs: BTreeMap::from([(
                "seed-1".to_string(),
                ProfileSeedingHandoffRecord {
                    id: "seed-1".to_string(),
                    profile_id: "profile-2".to_string(),
                    target_service_id: "site-2".to_string(),
                    state: ProfileSeedingHandoffState::VerificationPending,
                    ..ProfileSeedingHandoffRecord::default()
                },
            )]),
            ..ServiceState::default()
        };
        let response = service_runs_response(&state);
        assert_eq!(response["count"], 2);
        assert_eq!(response["runs"][0]["targetId"], "target-1");
        assert_eq!(response["runs"][0]["profileId"], "profile-1");
        assert_eq!(response["runs"][1]["nextAction"], "verify_login_readiness");
    }

    #[test]
    fn stale_tab_handle_cannot_be_opened_from_recent_job() {
        let state = ServiceState {
            jobs: BTreeMap::from([(
                "job-1".to_string(),
                ServiceJob {
                    id: "job-1".to_string(),
                    task_name: Some("review page".to_string()),
                    target: JobTarget::Tab("tab-1".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    target_id: Some("new-target".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    service_tab_handle: Some(ServiceTabHandle {
                        browser_id: "browser-1".to_string(),
                        tab_id: "tab-1".to_string(),
                        target_id: Some("old-target".to_string()),
                        valid: true,
                        ..ServiceTabHandle::default()
                    }),
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        };
        let response = service_runs_response(&state);
        assert_eq!(response["runs"][0]["tabId"], "tab-1");
        assert!(response["runs"][0]["targetId"].is_null());
        assert!(response["runs"][0]["browserId"].is_null());
    }
}
