//! Handoff-only broker authority backed by the original held kernel lease.
//! No defaults mint task authority or inherit an earlier confirmation. The
//! worker must supply every config field explicitly. The task-authority module
//! coordinates ledger writers across attachments, the worker, and processes;
//! this module's lock protects only this attachment's configuration and lease.

use super::actions::{
    service_profile_lease_gate, HandoffBrokerCustody, HandoffBrokerPermit, ServiceProfileLeaseGate,
};
use super::broker_attachment::{
    BrokerAdmissionFuture, BrokerAuthority, BrokerBinding, BrokerCommandPermit,
    BrokerCommandRequest, BrokerEventRequest, BrokerPublicationFuture,
};
use super::policy::{action_consequence, ActionPolicy, ConfirmActions, PolicyResult};
use super::task_authority::{
    admit_task_authority, authorize_task_authority_publication, TaskAuthorityContext,
    TaskAuthorityDecision,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Worker-captured settings; None explicitly means no configured policy or
/// confirmation categories, never an implicit grant of task authority.
pub(crate) struct HandoffAuthorityConfig {
    pub policy: Option<ActionPolicy>,
    pub confirm_actions: Option<ConfirmActions>,
    pub require_task_authority: bool,
    pub ledger_root: PathBuf,
}

struct State {
    lease: Option<Arc<HandoffBrokerPermit>>,
    detached: bool,
    config: HandoffAuthorityConfig,
}

struct Inner {
    binding: BrokerBinding,
    expected_url: String,
    capability: HandoffBrokerCustody,
    state: Mutex<State>,
}

pub(crate) struct HandoffBrokerAuthority {
    inner: Arc<Inner>,
}

impl HandoffBrokerAuthority {
    pub(crate) async fn new(
        capability: HandoffBrokerCustody,
        binding: BrokerBinding,
        expected_url: String,
        config: HandoffAuthorityConfig,
    ) -> Result<Self, String> {
        let url = url::Url::parse(&expected_url).map_err(|_| "broker_authority_url_invalid")?;
        if !matches!(url.scheme(), "https" | "http")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("broker_authority_url_invalid".into());
        }
        if !config.ledger_root.is_absolute() {
            return Err("broker_authority_ledger_root_invalid".into());
        }
        let lease = Arc::new(capability.acquire().await?);
        lease.verify_binding(&binding)?;
        Ok(Self {
            inner: Arc::new(Inner {
                binding,
                expected_url,
                capability,
                state: Mutex::new(State {
                    lease: Some(lease),
                    detached: false,
                    config,
                }),
            }),
        })
    }
}

impl Inner {
    fn live_target(&self, target: &Value) -> Result<(), String> {
        if target
            .pointer("/targetInfo/targetId")
            .and_then(Value::as_str)
            != Some(self.binding.target_id.as_str())
            || target.pointer("/targetInfo/type").and_then(Value::as_str) != Some("page")
            || target.pointer("/targetInfo/url").and_then(Value::as_str)
                != Some(self.expected_url.as_str())
        {
            return Err("broker_authority_live_target_mismatch".into());
        }
        Ok(())
    }

    fn active(&self) -> Result<(), String> {
        if self.capability.is_revoked() {
            return Err("broker_authority_custody_revoked".into());
        }
        Ok(())
    }
}

impl BrokerAuthority for HandoffBrokerAuthority {
    fn admit(&self, binding: &BrokerBinding, operation: &str) -> Result<Box<dyn Send>, String> {
        if binding != &self.inner.binding {
            return Err("broker_authority_binding_mismatch".into());
        }
        let cleanup = matches!(operation, "detach" | "publish_detach");
        if !cleanup {
            self.inner.active()?;
        }
        // The legacy event hook has no task envelope or evidence reservation.
        // Held custody alone cannot grant public read/extraction authority.
        if matches!(operation, "events" | "publish_events") {
            return Err("broker_authority_event_packet_required".into());
        }
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "broker_authority_lock_poisoned")?;
        if state.detached {
            return if cleanup {
                Ok(Box::new(()))
            } else {
                Err("broker_authority_detached".into())
            };
        }
        if !matches!(
            operation,
            "attach"
                | "publish_attach"
                | "command_preflight"
                | "event_preflight"
                | "events"
                | "publish_events"
                | "detach"
                | "publish_detach"
        ) {
            return Err("broker_authority_operation_unsupported".into());
        }
        let lease = state
            .lease
            .as_ref()
            .ok_or("broker_authority_lease_missing")?
            .clone();
        lease.verify_binding(binding)?;
        if operation == "publish_detach" {
            // Called only after the core verifies the exact CDP detach ack.
            state.detached = true;
            state.lease.take();
        }
        Ok(Box::new(lease))
    }

    fn admit_command<'a>(
        &'a self,
        request: &'a BrokerCommandRequest,
        live_target: &'a Value,
    ) -> BrokerAdmissionFuture<'a> {
        Box::pin(async move {
            if request.binding != self.inner.binding {
                return Err("broker_authority_binding_mismatch".into());
            }
            self.inner.active()?;
            self.inner.live_target(live_target)?;
            let (command, action) = native_command(request, &self.inner.expected_url)?;
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "broker_authority_lock_poisoned")?;
            if state.detached {
                return Err("broker_authority_detached".into());
            }
            let lease = state
                .lease
                .as_ref()
                .ok_or("broker_authority_lease_missing")?
                .clone();
            lease.verify_binding(&request.binding)?;
            check_policy(&mut state.config, action)?;
            check_profile(&command, &request.binding.session_name)?;
            let context = task_context(&self.inner, &state.config);
            match admit_task_authority(
                &command,
                action,
                action_consequence(action),
                &context,
                false,
            )? {
                TaskAuthorityDecision::Admitted(_) | TaskAuthorityDecision::NotPresent => {}
                TaskAuthorityDecision::RequiresConfirmation(_) => {
                    return Err("broker_authority_confirmation_required".into())
                }
            }
            let ordered = match admit_task_authority(
                &command,
                action,
                action_consequence(action),
                &context,
                true,
            )? {
                TaskAuthorityDecision::Admitted(admission) => admission.step_id.is_some(),
                TaskAuthorityDecision::NotPresent => false,
                _ => return Err("broker_authority_reservation_failed".into()),
            };
            Ok(Box::new(CommandPermit {
                inner: self.inner.clone(),
                lease,
                command,
                action,
                ordered,
                publication_guard: None,
                evidence_limit: None,
            }) as Box<dyn BrokerCommandPermit>)
        })
    }

    fn admit_events<'a>(
        &'a self,
        request: &'a BrokerEventRequest,
        live_target: &'a Value,
    ) -> BrokerAdmissionFuture<'a> {
        Box::pin(async move {
            if request.binding != self.inner.binding {
                return Err("broker_authority_binding_mismatch".into());
            }
            self.inner.active()?;
            self.inner.live_target(live_target)?;
            let (command, evidence_limit) = native_events(request)?;
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "broker_authority_lock_poisoned")?;
            if state.detached {
                return Err("broker_authority_detached".into());
            }
            let lease = state
                .lease
                .as_ref()
                .ok_or("broker_authority_lease_missing")?
                .clone();
            lease.verify_binding(&request.binding)?;
            check_policy(&mut state.config, "broker_events")?;
            check_profile(&command, &request.binding.session_name)?;
            let context = task_context(&self.inner, &state.config);
            match admit_task_authority(
                &command,
                "broker_events",
                action_consequence("broker_events"),
                &context,
                false,
            )? {
                TaskAuthorityDecision::NotPresent | TaskAuthorityDecision::Admitted(_) => {}
                TaskAuthorityDecision::RequiresConfirmation(_) => {
                    return Err("broker_authority_confirmation_required".into())
                }
            }
            let ordered = match admit_task_authority(
                &command,
                "broker_events",
                action_consequence("broker_events"),
                &context,
                true,
            )? {
                TaskAuthorityDecision::Admitted(admission) => admission.step_id.is_some(),
                TaskAuthorityDecision::NotPresent => false,
                _ => return Err("broker_authority_reservation_failed".into()),
            };
            Ok(Box::new(CommandPermit {
                inner: self.inner.clone(),
                lease,
                command,
                action: "broker_events",
                ordered,
                publication_guard: None,
                evidence_limit: Some(evidence_limit),
            }) as Box<dyn BrokerCommandPermit>)
        })
    }
}

struct CommandPermit {
    inner: Arc<Inner>,
    lease: Arc<HandoffBrokerPermit>,
    command: Value,
    action: &'static str,
    ordered: bool,
    publication_guard: Option<std::fs::File>,
    evidence_limit: Option<u64>,
}

impl BrokerCommandPermit for CommandPermit {
    fn wire_limit(&self) -> Option<u64> {
        self.evidence_limit
    }

    fn publish<'a>(
        &'a mut self,
        result: &'a Value,
        live_target: &'a Value,
    ) -> BrokerPublicationFuture<'a> {
        Box::pin(async move {
            self.inner.active()?;
            self.inner.live_target(live_target)?;
            self.lease.verify_binding(&self.inner.binding)?;
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "broker_authority_lock_poisoned")?;
            check_policy(&mut state.config, self.action)?;
            check_profile(&self.command, &self.inner.binding.session_name)?;
            if let Some(limit) = self.evidence_limit {
                check_event_result_budget(result, limit)?;
            }
            let response = json!({"id":self.command["id"],"success":true,"data":result});
            self.publication_guard = authorize_task_authority_publication(
                &self.command,
                &task_context(&self.inner, &state.config),
                &response,
                self.ordered,
            )?;
            Ok(())
        })
    }
}

fn task_context<'a>(inner: &'a Inner, config: &HandoffAuthorityConfig) -> TaskAuthorityContext<'a> {
    TaskAuthorityContext {
        session_id: &inner.binding.session_name,
        target_id: Some(&inner.binding.target_id),
        url: Some(&inner.expected_url),
        confirmed_authority_id: None,
        require_authority: config.require_task_authority,
        ledger_root: config.ledger_root.clone(),
    }
}

fn check_policy(config: &mut HandoffAuthorityConfig, action: &str) -> Result<(), String> {
    if let Some(policy) = &mut config.policy {
        policy
            .reload()
            .map_err(|_| "broker_authority_policy_reload_failed")?;
        match policy.check(action) {
            PolicyResult::Allow => {}
            PolicyResult::Deny(_) => return Err("broker_authority_policy_denied".into()),
            PolicyResult::RequiresConfirmation => {
                return Err("broker_authority_confirmation_required".into())
            }
        }
    }
    if config
        .confirm_actions
        .as_ref()
        .is_some_and(|value| value.requires_confirmation(action))
    {
        return Err("broker_authority_confirmation_required".into());
    }
    Ok(())
}

fn check_profile(command: &Value, session: &str) -> Result<(), String> {
    match service_profile_lease_gate(command, session, None)? {
        ServiceProfileLeaseGate::Ready => Ok(()),
        ServiceProfileLeaseGate::Wait { .. } => {
            Err("broker_authority_wait_for_profile_lease".into())
        }
        ServiceProfileLeaseGate::Reject { .. } => {
            Err("broker_authority_profile_lease_rejected".into())
        }
    }
}

fn native_action(method: &str) -> Result<&'static str, String> {
    Ok(match method {
        "Runtime.evaluate" | "Runtime.callFunctionOn" => "evaluate",
        "Page.navigate" => "navigate",
        "Page.reload" => "reload",
        "Page.bringToFront" => "focus",
        "Page.handleJavaScriptDialog" => "dialog",
        "Page.captureScreenshot" => "screenshot",
        "DOM.setFileInputFiles" => "upload",
        "Input.insertText" => "type",
        "Input.dispatchKeyEvent" => "press",
        "Input.dispatchMouseEvent" => "click",
        "Network.getResponseBody" => "request_detail",
        "Runtime.getProperties"
        | "DOM.getDocument"
        | "DOM.querySelector"
        | "DOM.querySelectorAll"
        | "DOM.describeNode"
        | "DOM.resolveNode"
        | "Page.getFrameTree" => "snapshot",
        "Runtime.enable"
        | "Runtime.disable"
        | "Page.enable"
        | "Page.disable"
        | "DOM.enable"
        | "DOM.disable"
        | "Network.enable"
        | "Network.disable"
        | "Runtime.releaseObject"
        | "Runtime.releaseObjectGroup" => "diagnostics",
        _ => return Err("broker_authority_method_unsupported".into()),
    })
}

fn native_command(
    request: &BrokerCommandRequest,
    expected_url: &str,
) -> Result<(Value, &'static str), String> {
    let action = native_action(&request.method)?;
    let mut command = native_metadata(
        &request.binding,
        &request.request_id,
        &request.task_context,
        action,
    )?;
    if !request.params.is_object() {
        return Err("broker_authority_params_invalid".into());
    }
    // CDP reload has an optional script injection surface. Navigation approval
    // cannot authorize it, even when the script is empty or malformed.
    if request.method == "Page.reload" && request.params.get("scriptToEvaluateOnLoad").is_some() {
        return Err("broker_authority_reload_script_forbidden".into());
    }
    command["cdpMethod"] = json!(request.method);
    command["cdpParams"] = request.params.clone();
    if action == "navigate" {
        let url = request
            .params
            .get("url")
            .and_then(Value::as_str)
            .ok_or("broker_authority_navigation_url_missing")?;
        if url != expected_url {
            return Err("broker_authority_navigation_outside_binding".into());
        }
        command["url"] = json!(url);
    }
    if action == "reload" {
        command["url"] = json!(expected_url);
    }
    if let Some(expression) = request.params.get("expression") {
        command["script"] = expression.clone();
    }
    if let Some(function) = request.params.get("functionDeclaration") {
        command["script"] = function.clone();
    }
    if let Some(text) = request.params.get("text") {
        command["text"] = text.clone();
    }
    if let Some(files) = request.params.get("files") {
        command["files"] = files.clone();
    }
    Ok((command, action))
}

fn native_metadata(
    binding: &BrokerBinding,
    request_id: &str,
    task_context: &Value,
    action: &str,
) -> Result<Value, String> {
    if request_id.is_empty() || request_id.len() > 128 || request_id.chars().any(char::is_control) {
        return Err("broker_authority_request_id_invalid".into());
    }
    let metadata = task_context
        .as_object()
        .ok_or("broker_authority_task_context_invalid")?;
    const ALLOWED: &[&str] = &[
        "taskAuthority",
        "taskStepId",
        "taskEvidenceBytes",
        "taskName",
        "agentName",
        "serviceName",
    ];
    if metadata.keys().any(|key| !ALLOWED.contains(&key.as_str())) {
        return Err("broker_authority_metadata_override_forbidden".into());
    }
    for name in ["serviceName"] {
        if metadata
            .get(name)
            .and_then(Value::as_str)
            .is_none_or(|v| v.trim().is_empty())
        {
            return Err("broker_authority_task_packet_incomplete".into());
        }
    }
    if let Some(authority) = metadata.get("taskAuthority") {
        if !authority.is_object() {
            return Err("broker_authority_task_packet_incomplete".into());
        }
    }
    let mut command = Value::Object(metadata.clone());
    command["id"] = json!(request_id);
    command["action"] = json!(action);
    command["browserId"] = json!(binding.browser_id);
    command["sessionName"] = json!(binding.session_name);
    command["profileId"] = json!(binding.profile_id);
    command["profileLeasePolicy"] = json!("reject");
    Ok(command)
}

fn native_events(request: &BrokerEventRequest) -> Result<(Value, u64), String> {
    let mut command = native_metadata(
        &request.binding,
        &request.request_id,
        &request.task_context,
        "broker_events",
    )?;
    let limit = match command.get("taskEvidenceBytes") {
        None => 4096,
        Some(value) => value
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("broker_event_evidence_budget_invalid")?,
    };
    command["taskEvidenceBytes"] = json!(limit);
    command["cursor"] = json!(request.cursor);
    Ok((command, limit))
}

fn check_event_result_budget(result: &Value, limit: u64) -> Result<(), String> {
    let bytes = serde_json::to_vec(result).map_err(|_| "broker_event_result_invalid")?;
    if u64::try_from(bytes.len()).map_err(|_| "broker_event_result_invalid")? > limit {
        return Err("broker_event_evidence_budget_exceeded".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event_request() -> BrokerEventRequest {
        let command = request("Runtime.enable", json!({}));
        BrokerEventRequest {
            binding: command.binding,
            request_id: "event-poll".into(),
            cursor: 7,
            task_context: json!({"serviceName":"service"}),
        }
    }

    #[test]
    fn events_have_explicit_native_action_and_default_budget() {
        let event = event_request();
        let (command, limit) = native_events(&event).unwrap();
        assert_eq!(command["action"], "broker_events");
        assert_eq!(command["id"], "event-poll");
        assert_eq!(command["cursor"], 7);
        assert_eq!(command["profileId"], event.binding.profile_id);
        assert!(command.get("cdpMethod").is_none());
        assert_eq!(limit, 4096);
        assert_eq!(command["taskEvidenceBytes"], 4096);
    }

    #[test]
    fn events_reject_metadata_overrides_and_invalid_budgets() {
        for value in [json!(0), json!(-1), json!(1.5), json!("4096"), Value::Null] {
            let mut event = event_request();
            event.task_context["taskEvidenceBytes"] = value;
            assert!(native_events(&event).is_err());
        }
        for key in ["action", "profileId", "confirmedAuthorityId", "cursor"] {
            let mut event = event_request();
            event.task_context[key] = json!("override");
            assert!(native_events(&event).is_err());
        }
        let mut event = event_request();
        event.task_context = json!({});
        assert!(native_events(&event).is_err());
    }

    #[test]
    fn event_budget_measures_serialized_result_including_empty_batch() {
        let result = json!({"cursor":0,"events":[]});
        let bytes = serde_json::to_vec(&result).unwrap().len() as u64;
        assert!(check_event_result_budget(&result, bytes).is_ok());
        assert_eq!(
            check_event_result_budget(&result, bytes - 1).unwrap_err(),
            "broker_event_evidence_budget_exceeded"
        );
        let mut event = event_request();
        event.task_context["taskEvidenceBytes"] = json!(bytes);
        assert_eq!(native_events(&event).unwrap().1, bytes);
    }

    fn request(method: &str, params: Value) -> BrokerCommandRequest {
        BrokerCommandRequest {
            binding: BrokerBinding {
                attachment_id: "a".into(),
                browser_id: "b".into(),
                profile_id: "p".into(),
                session_name: "s".into(),
                target_id: "t".into(),
                generation: "g".into(),
            },
            request_id: "id".into(),
            method: method.into(),
            params,
            task_context: json!({"taskAuthority":{},"taskStepId":"step","taskEvidenceBytes":1,"taskName":"task","agentName":"agent","serviceName":"service"}),
        }
    }
    #[test]
    fn reload_cannot_smuggle_script_under_navigation_authority() {
        for script in [
            json!("fetch('/mutate', {method:'POST'})"),
            json!(""),
            Value::Null,
        ] {
            let request = request("Page.reload", json!({"scriptToEvaluateOnLoad":script}));
            assert_eq!(
                native_command(&request, "https://example.org/").unwrap_err(),
                "broker_authority_reload_script_forbidden"
            );
        }
        assert!(native_command(
            &request("Page.reload", json!({"ignoreCache":true})),
            "https://example.org/"
        )
        .is_ok());
    }

    #[test]
    fn mutation_mappings_are_not_reads() {
        for (method, action) in [
            ("Runtime.evaluate", "evaluate"),
            ("Runtime.callFunctionOn", "evaluate"),
            ("Input.dispatchMouseEvent", "click"),
            ("DOM.setFileInputFiles", "upload"),
        ] {
            assert_eq!(native_action(method).unwrap(), action);
            assert_ne!(
                action_consequence(action),
                super::super::policy::ActionConsequence::ReadOnly
            );
        }
        assert!(native_action("Browser.close").is_err());
    }
    #[test]
    fn params_preserved_and_metadata_override_rejected() {
        let mut request = request(
            "Runtime.evaluate",
            json!({"expression":"1","awaitPromise":true}),
        );
        let (command, _) = native_command(&request, "https://example.org/").unwrap();
        assert_eq!(command["cdpParams"], request.params);
        assert_eq!(command["script"], "1");
        request.task_context["confirmedAuthorityId"] = json!("borrowed");
        assert!(native_command(&request, "https://example.org/").is_err());
    }
    #[test]
    fn navigation_outside_exact_url_and_missing_packet_fail_closed() {
        let request = request("Page.navigate", json!({"url":"https://other.example/"}));
        assert!(native_command(&request, "https://example.org/").is_err());
        let mut request = request;
        request.task_context = json!({});
        assert!(native_command(&request, "https://example.org/").is_err());
    }
}
