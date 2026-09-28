//! Repository-backed service job operations.

use chrono::DateTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::service_model::{JobState, ServiceJob, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

pub const MAX_SERVICE_JOBS: usize = 200;
const LEGACY_INTERRUPTED_MIN_AGE_SECONDS: i64 = 300;
const LEGACY_INTERRUPTED_REVIEW_TTL_SECONDS: i64 = 300;
const LEGACY_INTERRUPTED_REVIEW_PREFIX: &str = "abjir1";

pub fn mutate_persisted_service_jobs(mutator: impl FnOnce(&mut ServiceState)) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = mutate_service_jobs_in_repository(&repository, mutator);
    }
}

pub fn mutate_service_jobs_in_repository(
    repository: &impl ServiceStateRepository,
    mutator: impl FnOnce(&mut ServiceState),
) -> Result<(), String> {
    repository.mutate(|state| {
        mutator(state);
        prune_service_jobs(state);
        Ok(())
    })
}

pub fn cancel_persisted_service_job(
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    LockedServiceStateRepository::default_json()
        .and_then(|repository| cancel_service_job_in_repository(&repository, job_id, reason))
        .map_err(cancel_persisted_service_job_response_error)
}

pub fn cancel_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    repository.mutate(|state| {
        let job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| format!("Service job not found: {}", job_id))?;

        match job.state {
            JobState::Queued | JobState::WaitingProfileLease => {
                job.state = JobState::Cancelled;
                job.completed_at = Some(current_timestamp());
                job.error = Some(
                    reason
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("Cancelled by operator")
                        .to_string(),
                );
                job.result = Some(json!({ "success": false, "cancelled": true }));
                Ok(job.clone())
            }
            JobState::Cancelled => Ok(job.clone()),
            JobState::Running => Err(format!(
                "Service job {} is already running and cannot be cancelled safely",
                job_id
            )),
            JobState::Succeeded | JobState::Failed | JobState::TimedOut => Err(format!(
                "Service job {} is already terminal with state {}",
                job_id,
                job_state_name(job.state)
            )),
        }
    })
}

pub fn load_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Option<ServiceJob> {
    repository.load_snapshot().ok()?.jobs.remove(id)
}

pub fn cancel_persisted_service_job_response_error(err: String) -> String {
    if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
        format!("Unable to load service state: {}", err)
    } else {
        err
    }
}

pub fn legacy_interrupted_jobs_preview(
    state: &ServiceState,
    job_ids: &[String],
) -> Result<Value, String> {
    let now = time::OffsetDateTime::now_utc();
    let candidates = legacy_interrupted_candidates(state, job_ids, now)?;
    let issued_at = now.unix_timestamp();
    Ok(json!({
        "apply": false,
        "candidateCount": candidates.len(),
        "candidates": candidates,
        "minimumAgeSeconds": LEGACY_INTERRUPTED_MIN_AGE_SECONDS,
        "reviewTokenTtlSeconds": LEGACY_INTERRUPTED_REVIEW_TTL_SECONDS,
        "reviewToken": legacy_interrupted_review_token(&candidates, issued_at),
        "recommendedNextStep": "Review the exact candidates, then rerun service recover-interrupted with --apply, the same --job-id values, and this reviewToken.",
    }))
}

pub fn recover_legacy_interrupted_jobs(
    state: &mut ServiceState,
    job_ids: &[String],
    review_token: Option<&str>,
) -> Result<Value, String> {
    let now = time::OffsetDateTime::now_utc();
    let candidates = legacy_interrupted_candidates(state, job_ids, now)?;
    let token = review_token.ok_or("legacy_interrupted_review_token_required")?;
    validate_legacy_interrupted_review_token(&candidates, token, now.unix_timestamp())?;
    let completed_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let mut recovered = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let id = candidate["id"]
            .as_str()
            .ok_or("legacy_interrupted_candidate_missing_id")?;
        let job = state
            .jobs
            .get_mut(id)
            .ok_or_else(|| format!("Service job not found: {id}"))?;
        job.state = JobState::Failed;
        job.completed_at = Some(completed_at.clone());
        job.error =
            Some("Legacy service worker exited without durable ownership evidence".to_string());
        job.result = Some(json!({
            "success": false,
            "interrupted": true,
            "reason": "legacy_service_worker_unowned",
        }));
        recovered.push(id.to_string());
    }
    Ok(json!({
        "apply": true,
        "candidateCount": candidates.len(),
        "recoveredCount": recovered.len(),
        "recoveredJobIds": recovered,
        "reason": "legacy_service_worker_unowned",
    }))
}

fn legacy_interrupted_candidates(
    state: &ServiceState,
    job_ids: &[String],
    now: time::OffsetDateTime,
) -> Result<Vec<Value>, String> {
    if job_ids.is_empty() {
        return Err("legacy_interrupted_job_id_required".to_string());
    }
    let mut ids = job_ids.to_vec();
    ids.sort();
    ids.dedup();
    if ids.len() != job_ids.len() {
        return Err("legacy_interrupted_duplicate_job_id".to_string());
    }
    let mut candidates = Vec::with_capacity(ids.len());
    for id in ids {
        let job = state
            .jobs
            .get(&id)
            .ok_or_else(|| format!("Service job not found: {id}"))?;
        if job.state != JobState::Running {
            return Err(format!("legacy_interrupted_job_not_running:{id}"));
        }
        if job.runner_session_id.is_some() || job.runner_instance_id.is_some() {
            return Err(format!("legacy_interrupted_job_has_owner_identity:{id}"));
        }
        if job.completed_at.is_some() || job.result.is_some() || job.error.is_some() {
            return Err(format!("legacy_interrupted_job_has_terminal_evidence:{id}"));
        }
        let submitted_at = job
            .submitted_at
            .as_deref()
            .ok_or_else(|| format!("legacy_interrupted_job_missing_submitted_at:{id}"))?;
        let submitted = DateTime::parse_from_rfc3339(submitted_at)
            .map_err(|_| format!("legacy_interrupted_job_invalid_submitted_at:{id}"))?;
        let age_seconds = now.unix_timestamp() - submitted.timestamp();
        if age_seconds < LEGACY_INTERRUPTED_MIN_AGE_SECONDS {
            return Err(format!("legacy_interrupted_job_too_fresh:{id}"));
        }
        candidates.push(json!({
            "id": job.id,
            "action": job.action,
            "submittedAt": job.submitted_at,
            "startedAt": job.started_at,
            "state": "running",
            "runnerSessionId": job.runner_session_id,
            "runnerInstanceId": job.runner_instance_id,
            "ageSeconds": age_seconds,
        }));
    }
    Ok(candidates)
}

fn legacy_interrupted_review_token(candidates: &[Value], issued_at: i64) -> String {
    let mut bound_candidates = candidates.to_vec();
    for candidate in &mut bound_candidates {
        if let Some(object) = candidate.as_object_mut() {
            object.remove("ageSeconds");
        }
    }
    let payload = serde_json::to_vec(&bound_candidates).unwrap_or_default();
    let digest = format!("{:x}", Sha256::digest(payload));
    format!("{LEGACY_INTERRUPTED_REVIEW_PREFIX}:{issued_at}:{digest}")
}

fn validate_legacy_interrupted_review_token(
    candidates: &[Value],
    token: &str,
    now: i64,
) -> Result<(), String> {
    let mut parts = token.split(':');
    if parts.next() != Some(LEGACY_INTERRUPTED_REVIEW_PREFIX) {
        return Err("legacy_interrupted_review_token_prefix".to_string());
    }
    let issued_at = parts
        .next()
        .ok_or("legacy_interrupted_review_token_timestamp")?
        .parse::<i64>()
        .map_err(|_| "legacy_interrupted_review_token_timestamp".to_string())?;
    let supplied_digest = parts
        .next()
        .ok_or("legacy_interrupted_review_token_digest")?;
    if parts.next().is_some() {
        return Err("legacy_interrupted_review_token_format".to_string());
    }
    if issued_at > now {
        return Err("legacy_interrupted_review_token_from_future".to_string());
    }
    if now - issued_at > LEGACY_INTERRUPTED_REVIEW_TTL_SECONDS {
        return Err("legacy_interrupted_review_token_expired".to_string());
    }
    let expected = legacy_interrupted_review_token(candidates, issued_at);
    let expected_digest = expected.rsplit(':').next().unwrap_or_default();
    if supplied_digest != expected_digest {
        return Err("legacy_interrupted_review_token_candidate_mismatch".to_string());
    }
    Ok(())
}

fn prune_service_jobs(state: &mut ServiceState) {
    if state.jobs.len() <= MAX_SERVICE_JOBS {
        return;
    }
    let mut jobs = state
        .jobs
        .values()
        .filter(|job| {
            matches!(
                job.state,
                JobState::Succeeded | JobState::Failed | JobState::Cancelled | JobState::TimedOut
            )
        })
        .map(|job| (job.submitted_at.clone().unwrap_or_default(), job.id.clone()))
        .collect::<Vec<_>>();
    jobs.sort();
    let excess = state.jobs.len() - MAX_SERVICE_JOBS;
    for (_, id) in jobs.into_iter().take(excess) {
        state.jobs.remove(&id);
    }
}

fn job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::WaitingProfileLease => "waiting_profile_lease",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::TimedOut => "timed_out",
    }
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_running_job(id: &str) -> ServiceJob {
        ServiceJob {
            id: id.to_string(),
            action: "confirm".to_string(),
            state: JobState::Running,
            submitted_at: Some("2020-01-01T00:00:00Z".to_string()),
            started_at: Some("2020-01-01T00:00:01Z".to_string()),
            ..ServiceJob::default()
        }
    }

    #[test]
    fn legacy_interrupted_recovery_requires_matching_preview_token() {
        let mut state = ServiceState::default();
        state
            .jobs
            .insert("legacy-a".to_string(), legacy_running_job("legacy-a"));
        let ids = vec!["legacy-a".to_string()];
        let preview = legacy_interrupted_jobs_preview(&state, &ids).unwrap();
        assert_eq!(preview["candidateCount"], 1);
        let token = preview["reviewToken"].as_str().unwrap();

        let response = recover_legacy_interrupted_jobs(&mut state, &ids, Some(token)).unwrap();
        assert_eq!(response["recoveredCount"], 1);
        let recovered = &state.jobs["legacy-a"];
        assert_eq!(recovered.state, JobState::Failed);
        assert_eq!(
            recovered.result.as_ref().unwrap()["reason"],
            "legacy_service_worker_unowned"
        );
    }

    #[test]
    fn legacy_interrupted_recovery_rejects_owned_or_changed_candidates() {
        let mut state = ServiceState::default();
        let mut owned = legacy_running_job("owned");
        owned.runner_session_id = Some("session-a".to_string());
        owned.runner_instance_id = Some("worker-a".to_string());
        state.jobs.insert("owned".to_string(), owned);
        assert_eq!(
            legacy_interrupted_jobs_preview(&state, &["owned".to_string()]).unwrap_err(),
            "legacy_interrupted_job_has_owner_identity:owned"
        );

        state
            .jobs
            .insert("legacy-a".to_string(), legacy_running_job("legacy-a"));
        let ids = vec!["legacy-a".to_string()];
        let preview = legacy_interrupted_jobs_preview(&state, &ids).unwrap();
        let token = preview["reviewToken"].as_str().unwrap().to_string();
        state.jobs.get_mut("legacy-a").unwrap().action = "navigate".to_string();
        assert_eq!(
            recover_legacy_interrupted_jobs(&mut state, &ids, Some(&token)).unwrap_err(),
            "legacy_interrupted_review_token_candidate_mismatch"
        );
    }

    #[test]
    fn pruning_never_evicts_nonterminal_jobs() {
        let mut state = ServiceState::default();
        for index in 0..MAX_SERVICE_JOBS {
            let id = format!("terminal-{index:03}");
            state.jobs.insert(
                id.clone(),
                ServiceJob {
                    id,
                    action: "title".to_string(),
                    state: JobState::Succeeded,
                    submitted_at: Some(format!("2020-01-01T00:{:02}:00Z", index % 60)),
                    ..ServiceJob::default()
                },
            );
        }
        state.jobs.insert(
            "running-oldest".to_string(),
            ServiceJob {
                id: "running-oldest".to_string(),
                action: "confirm".to_string(),
                state: JobState::Running,
                submitted_at: Some("2019-01-01T00:00:00Z".to_string()),
                ..ServiceJob::default()
            },
        );

        prune_service_jobs(&mut state);

        assert_eq!(state.jobs.len(), MAX_SERVICE_JOBS);
        assert!(state.jobs.contains_key("running-oldest"));
        assert_eq!(
            state
                .jobs
                .values()
                .filter(|job| job.state == JobState::Succeeded)
                .count(),
            MAX_SERVICE_JOBS - 1
        );
    }
}
