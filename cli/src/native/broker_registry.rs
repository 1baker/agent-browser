//! Authenticated daemon fast-lane registry. Only the worker can insert an
//! already acquired attachment; a request cannot reconstruct one from JSON.
use super::broker_attachment::{BrokerAttachment, BrokerBinding, BrokerOutput};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
pub(crate) struct BrokerRegistry {
    state: Mutex<RegistryState>,
}

#[derive(Default)]
struct RegistryState {
    closing: bool,
    attachments: HashMap<String, Arc<BrokerAttachment>>,
    reservations: HashMap<String, (BrokerBinding, String)>,
    uncertain: HashMap<String, BrokerBinding>,
}

/// Linear, registry-bound capacity reservation. Drop releases an unused slot.
pub(crate) struct BrokerReservation {
    registry: Arc<BrokerRegistry>,
    binding: BrokerBinding,
    token: String,
}

/// Never discard a live attachment merely because registration failed. On a
/// closing race it is also retained, sealed, in the registry for cleanup.
pub(crate) struct BrokerCommitFailure {
    pub error: String,
    pub attachment: Arc<BrokerAttachment>,
}

impl std::fmt::Debug for BrokerCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BrokerCommitFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl BrokerReservation {
    /// Preserve an unknown acquisition's exact binding after reservation drop.
    /// This in-memory seal is irreversible without future reviewed reconciliation.
    pub(crate) fn mark_uncertain(self) {
        let mut state = self
            .registry
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.closing = true;
        state
            .uncertain
            .insert(self.binding.attachment_id.clone(), self.binding.clone());
        for attachment in state.attachments.values() {
            attachment.seal();
        }
    }

    pub(crate) fn commit(
        self,
        attachment: Arc<BrokerAttachment>,
    ) -> Result<(), BrokerCommitFailure> {
        let result = self.commit_inner(&attachment);
        result.map_err(|error| BrokerCommitFailure { error, attachment })
    }

    fn commit_inner(&self, attachment: &Arc<BrokerAttachment>) -> Result<(), String> {
        let mut state = self
            .registry
            .state
            .lock()
            .map_err(|_| "broker_registry_poisoned")?;
        let id = &self.binding.attachment_id;
        if attachment.binding() != &self.binding
            || !state
                .reservations
                .get(id)
                .is_some_and(|(binding, token)| binding == &self.binding && token == &self.token)
            || state.attachments.contains_key(id)
        {
            return Err("broker_reservation_mismatch".into());
        }
        // Even if shutdown began while CDP attach was pending, retain the live
        // result for acknowledged cleanup. Never publish it as a usable handle.
        let closing = state.closing;
        if closing {
            attachment.seal();
        }
        state.attachments.insert(id.clone(), attachment.clone());
        state.reservations.remove(id);
        if closing {
            Err("broker_registry_closing".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for BrokerReservation {
    fn drop(&mut self) {
        // A poisoned registry remains fail closed, but releasing an unused
        // reservation is safe and must not leak capacity during unwinding.
        let mut state = self
            .registry
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let id = &self.binding.attachment_id;
        if state
            .reservations
            .get(id)
            .is_some_and(|(_, token)| token == &self.token)
        {
            state.reservations.remove(id);
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Command {
        binding: BrokerBinding,
        #[serde(rename = "requestId")]
        request_id: String,
        method: String,
        params: Value,
        #[serde(rename = "taskContext")]
        task_context: Value,
    },
    Events {
        binding: BrokerBinding,
        cursor: u64,
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "taskContext")]
        task_context: Value,
    },
    Detach {
        binding: BrokerBinding,
        #[serde(rename = "requestId")]
        request_id: String,
    },
}

impl BrokerRegistry {
    /// Reserve before any CDP attachment creation. The binding is immutable and
    /// must match the acquired attachment in full at commit, not only its ID.
    pub(crate) fn reserve(
        self: &Arc<Self>,
        binding: BrokerBinding,
    ) -> Result<BrokerReservation, String> {
        if [
            &binding.attachment_id,
            &binding.browser_id,
            &binding.profile_id,
            &binding.session_name,
            &binding.target_id,
            &binding.generation,
        ]
        .iter()
        .any(|value| {
            value.is_empty()
                || value.len() > 4096
                || value.trim() != value.as_str()
                || value.chars().any(char::is_control)
        }) {
            return Err("broker_reservation_binding_invalid".into());
        }
        let mut state = self.state.lock().map_err(|_| "broker_registry_poisoned")?;
        let id = &binding.attachment_id;
        if state.closing
            || state.attachments.len() + state.reservations.len() >= 128
            || state.attachments.contains_key(id)
            || state.reservations.contains_key(id)
        {
            return Err("broker_reservation_denied".into());
        }
        let token = uuid::Uuid::new_v4().to_string();
        state
            .reservations
            .insert(id.clone(), (binding.clone(), token.clone()));
        Ok(BrokerReservation {
            registry: self.clone(),
            binding,
            token,
        })
    }

    #[cfg(test)]
    pub(crate) fn is_closing(&self) -> bool {
        self.state.lock().unwrap().closing
    }

    /// The caller retains its Arc on failure and must reconcile that attachment.
    /// Never replace an existing attachment or accept an unbounded registry.
    #[cfg(test)]
    pub(crate) fn register(&self, attachment: Arc<BrokerAttachment>) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "broker_registry_poisoned")?;
        let id = &attachment.binding().attachment_id;
        if state.closing
            || state.attachments.len() + state.reservations.len() >= 128
            || state.attachments.contains_key(id)
            || state.reservations.contains_key(id)
        {
            return Err("broker_registration_denied".into());
        }
        state.attachments.insert(id.clone(), attachment);
        Ok(())
    }

    pub(crate) async fn dispatch(&self, value: Value) -> Result<BrokerOutput, String> {
        if value.to_string().len() > 9_437_184 {
            return Err("broker_request_too_large".into());
        }
        let request: Request =
            serde_json::from_value(value).map_err(|_| "broker_request_invalid")?;
        let (binding, cleanup) = match &request {
            Request::Command { binding, .. } | Request::Events { binding, .. } => (binding, false),
            Request::Detach { binding, .. } => (binding, true),
        };
        let attachment = {
            let state = self.state.lock().map_err(|_| "broker_registry_poisoned")?;
            if state.closing && !cleanup {
                return Err("broker_registry_closing".into());
            }
            let attachment = state
                .attachments
                .get(&binding.attachment_id)
                .ok_or("broker_attachment_unknown")?;
            if attachment.binding() != binding {
                return Err("broker_attachment_identity_mismatch".into());
            }
            attachment.clone()
        };
        // No registry lock is held across a command, event poll, or detach.
        match request {
            Request::Command {
                binding,
                request_id,
                method,
                params,
                task_context,
            } => {
                attachment
                    .command_authorized(&binding, &request_id, &method, params, task_context)
                    .await
            }
            Request::Events {
                binding,
                cursor,
                request_id,
                task_context,
            } => {
                attachment
                    .events_authorized(&binding, &request_id, cursor, task_context)
                    .await
            }
            Request::Detach {
                binding,
                request_id,
            } => attachment.detach(&binding, &request_id).await,
        }
    }

    /// Seals all page dispatch first, then reconciles attachments before the
    /// worker revokes original custody or changes the browser lifecycle.
    pub(crate) async fn close_all(&self) -> Result<(), String> {
        let attachments = {
            let mut state = self.state.lock().map_err(|_| "broker_registry_poisoned")?;
            state.closing = true;
            let attachments: Vec<_> = state.attachments.values().cloned().collect();
            for attachment in &attachments {
                attachment.seal();
            }
            if !state.uncertain.is_empty() {
                return Err("broker_registry_acquisition_uncertain".into());
            }
            if !state.reservations.is_empty() {
                // Existing attachments are sealed, not yet detached. Retry
                // after acquisition settles; unknown acquisition instead needs
                // reviewed reconciliation and never permits lifecycle success.
                return Err("broker_registry_acquisition_pending".into());
            }
            attachments
        };
        tokio::time::timeout(Duration::from_secs(15), async {
            for attachment in attachments {
                attachment
                    .detach(attachment.binding(), "broker-lifecycle-detach")
                    .await?;
            }
            Ok::<(), String>(())
        })
        .await
        .map_err(|_| "broker_registry_cleanup_timeout")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn binding(id: &str) -> BrokerBinding {
        BrokerBinding {
            attachment_id: id.into(),
            browser_id: "browser".into(),
            profile_id: "profile".into(),
            session_name: "session".into(),
            target_id: "target".into(),
            generation: "generation".into(),
        }
    }

    #[test]
    fn reservations_bound_capacity_and_drop_releases_unused_slot() {
        let registry = Arc::new(BrokerRegistry::default());
        let mut reservations = Vec::new();
        for index in 0..128 {
            reservations.push(registry.reserve(binding(&index.to_string())).unwrap());
        }
        assert!(registry.reserve(binding("overflow")).is_err());
        assert!(registry.reserve(binding("0")).is_err());
        drop(reservations.pop());
        let replacement = registry.reserve(binding("replacement")).unwrap();
        assert!(registry.reserve(binding("overflow")).is_err());
        drop(replacement);
        drop(reservations);
        assert!(registry.state.lock().unwrap().reservations.is_empty());
    }

    #[tokio::test]
    async fn pending_acquisition_blocks_shutdown_and_cancellation_releases_slot() {
        let registry = Arc::new(BrokerRegistry::default());
        let reservation = registry.reserve(binding("pending")).unwrap();
        assert_eq!(
            registry.close_all().await.unwrap_err(),
            "broker_registry_acquisition_pending"
        );
        assert!(registry.reserve(binding("new")).is_err());
        drop(reservation);
        registry.close_all().await.unwrap();
        assert!(registry.reserve(binding("new")).is_err());
    }

    #[tokio::test]
    async fn cancelled_acquisition_future_releases_reservation() {
        let registry = Arc::new(BrokerRegistry::default());
        let acquiring = async {
            let _reservation = registry.reserve(binding("pending")).unwrap();
            std::future::pending::<()>().await;
        };
        assert!(tokio::time::timeout(Duration::from_millis(1), acquiring)
            .await
            .is_err());
        assert!(registry.state.lock().unwrap().reservations.is_empty());
        assert!(registry.reserve(binding("pending")).is_ok());
    }

    #[tokio::test]
    async fn uncertain_acquisition_survives_reservation_drop_and_blocks_lifecycle() {
        let registry = Arc::new(BrokerRegistry::default());
        registry
            .reserve(binding("unknown"))
            .unwrap()
            .mark_uncertain();
        {
            let state = registry.state.lock().unwrap();
            assert!(state.attachments.is_empty());
            assert!(state.reservations.is_empty());
            assert_eq!(state.uncertain.get("unknown"), Some(&binding("unknown")));
        }
        assert!(registry.reserve(binding("new")).is_err());
        for _ in 0..2 {
            assert_eq!(
                registry.close_all().await.unwrap_err(),
                "broker_registry_acquisition_uncertain"
            );
        }
    }

    #[tokio::test]
    async fn json_cannot_create_authority_or_select_raw_transport() {
        let registry = BrokerRegistry::default();
        assert!(registry
            .dispatch(json!({"operation":"attach","endpoint":"ws://127.0.0.1:9222"}))
            .await
            .is_err());
        assert!(registry.dispatch(json!({"operation":"events","cursor":0,"binding":{
            "attachmentId":"a","browserId":"b","profileId":"p","sessionName":"s","targetId":"t","generation":"g"
        }})).await.is_err());
        registry.close_all().await.unwrap();
        registry.close_all().await.unwrap();
        assert!(registry.state.lock().unwrap().closing);
    }
}
