//! Additional persisted identity and single-use constraint for private work.
//! A binding is NOT account, origin, or action authority. Independently supplied
//! `PrivateHandoffAuthority` approval and all executor checks remain mandatory.

#![cfg(unix)]

use super::private_handoff::PrivateStagedHandoff;
use super::private_secret_store::SecretStore;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const REJECTED: &str = "private_bound_execution_rejected";
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
enum BindOperation {
    #[serde(rename = "bind")]
    Bind,
}

// Keep this exact schema aligned with private_controller_socket::Binding.
// Struct deserialization also rejects duplicate fields; never expose serde errors.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Binding {
    operation: BindOperation,
    plan_id: String,
    manifest_sha256: String,
    profile_id: String,
    browser_id: String,
    session_name: String,
    target_id: String,
    endpoint: String,
    expires_at: String,
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

impl Binding {
    fn parse(payload: &[u8]) -> Result<Self> {
        serde_json::from_slice(payload).map_err(|_| REJECTED)
    }

    fn validate(
        &self,
        plan_id: &str,
        digest: &str,
        endpoint: &str,
        session: &str,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Utc>> {
        let _ = self.operation;
        if !lower_hex(&self.plan_id, 32)
            || !lower_hex(&self.manifest_sha256, 64)
            || self.plan_id != plan_id
            || self.manifest_sha256 != digest
            || !token(&self.profile_id)
            || !token(&self.session_name)
            || self.session_name != session
            || self.browser_id != format!("session:{}", self.session_name)
            || self.target_id.len() != 32
            || !self
                .target_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            || self.endpoint != endpoint
            || self.endpoint.len() > 2048
            || self
                .endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(REJECTED);
        }
        let parsed = url::Url::parse(&self.endpoint).map_err(|_| REJECTED)?;
        if !matches!(parsed.scheme(), "ws" | "wss")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || self
                .endpoint
                .split('/')
                .nth(2)
                .is_some_and(|authority| authority.contains('@'))
        {
            return Err(REJECTED);
        }
        let expires = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|_| REJECTED)?
            .with_timezone(&Utc);
        if expires <= now {
            return Err(REJECTED);
        }
        Ok(expires)
    }
}

/// Only the successful durable claim constructs this token. Deliberately no
/// Debug, Clone, Deserialize, or Serialize; it does not construct authority.
pub(crate) struct BoundExecution {
    store: Arc<SecretStore>,
    reference: String,
    plan_id: String,
    digest: String,
}

impl BoundExecution {
    /// Recheck immutable admitted authority against the completed operation and
    /// reconciled endpoint. This grants no browser access and cannot re-claim.
    pub(crate) fn delivery_metadata(
        &self,
        operation: &super::private_operation::PrivateOperation,
        endpoint: &str,
    ) -> Result<serde_json::Value> {
        if self.store.status(&self.reference).map_err(|_| REJECTED)? != "admitted" {
            return Err(REJECTED);
        }
        let plan_key = hex::encode(Sha256::digest(self.plan_id.as_bytes()));
        if self
            .store
            .reconciliation_reference(&plan_key)
            .map_err(|_| REJECTED)?
            != self.reference
        {
            return Err(REJECTED);
        }
        let binding = Binding::parse(&self.store.load(&self.reference).map_err(|_| REJECTED)?)?;
        binding.validate(
            &self.plan_id,
            &self.digest,
            endpoint,
            &binding.session_name,
            Utc::now(),
        )?;
        if operation.consent_sha256() != binding.manifest_sha256
            || !matches!(
                operation.operation(),
                super::private_operation::Operation::ReadKey { .. }
            )
        {
            return Err(REJECTED);
        }
        for (field, expected) in [
            ("profileId", &binding.profile_id),
            ("browserId", &binding.browser_id),
            ("sessionName", &binding.session_name),
            ("targetId", &binding.target_id),
        ] {
            if operation
                .service_tab_handle()
                .get(field)
                .and_then(serde_json::Value::as_str)
                != Some(expected.as_str())
            {
                return Err(REJECTED);
            }
        }
        Ok(serde_json::json!({
            "schema":"agent-browser.private-delivery.v1", "planId":binding.plan_id,
            "manifestSha256":binding.manifest_sha256, "profileId":binding.profile_id,
            "browserId":binding.browser_id, "sessionName":binding.session_name,
            "targetId":binding.target_id, "endpoint":binding.endpoint,
            "accountScope":operation.account_scope(), "expiresAt":binding.expires_at
        }))
    }

    fn check(&self, staged: &PrivateStagedHandoff, endpoint: &str, session: &str) -> Result<()> {
        if self
            .store
            .same_directory(staged.operation_store())
            .map_err(|_| REJECTED)?
            || !staged.is_current()
        {
            return Err(REJECTED);
        }
        let plan_key = hex::encode(Sha256::digest(self.plan_id.as_bytes()));
        if self
            .store
            .reconciliation_reference(&plan_key)
            .map_err(|_| REJECTED)?
            != self.reference
        {
            return Err(REJECTED);
        }
        let binding = Binding::parse(&self.store.load(&self.reference).map_err(|_| REJECTED)?)?;
        let expires =
            binding.validate(&self.plan_id, &self.digest, endpoint, session, Utc::now())?;
        if DateTime::<Utc>::from(staged.expires_at()) > expires {
            return Err(REJECTED);
        }
        let operations = staged.binding_operations().map_err(|_| REJECTED)?;
        if operations.len() != 4 {
            return Err(REJECTED);
        }
        for operation in operations {
            if operation.consent_sha256() != binding.manifest_sha256 {
                return Err(REJECTED);
            }
            let handle = operation.service_tab_handle();
            for (key, expected) in [
                ("profileId", binding.profile_id.as_str()),
                ("browserId", binding.browser_id.as_str()),
                ("sessionName", binding.session_name.as_str()),
                ("targetId", binding.target_id.as_str()),
            ] {
                if handle.get(key).and_then(serde_json::Value::as_str) != Some(expected) {
                    return Err(REJECTED);
                }
            }
        }
        if !staged.is_current() || Utc::now() >= expires {
            return Err(REJECTED);
        }
        Ok(())
    }

    /// Recheck on dequeue without re-admitting or granting new authority.
    pub(crate) fn validate(
        &self,
        staged: &PrivateStagedHandoff,
        endpoint: &str,
        session: &str,
    ) -> Result<()> {
        if self.store.status(&self.reference).map_err(|_| REJECTED)? != "admitted" {
            return Err(REJECTED);
        }
        self.check(staged, endpoint, session)
    }
}

/// Persist the irreversible receipt before returning anything queueable. A
/// queue failure leaves it consumed; competing and reopened claims fail closed.
pub(crate) fn claim(
    binding_store: Arc<SecretStore>,
    plan_id: &str,
    digest: &str,
    staged: &PrivateStagedHandoff,
    endpoint: &str,
    session: &str,
) -> Result<BoundExecution> {
    if !lower_hex(plan_id, 32) || !lower_hex(digest, 64) {
        return Err(REJECTED);
    }
    let plan_key = hex::encode(Sha256::digest(plan_id.as_bytes()));
    let reference = binding_store
        .reconciliation_reference(&plan_key)
        .map_err(|_| REJECTED)?;
    let execution = BoundExecution {
        store: binding_store,
        reference,
        plan_id: plan_id.to_owned(),
        digest: digest.to_owned(),
    };
    execution.check(staged, endpoint, session)?;
    execution
        .store
        .admit(&execution.reference)
        .map_err(|_| REJECTED)?;
    execution.validate(staged, endpoint, session)?;
    Ok(execution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn fixture() -> Value {
        json!({
            "operation": "bind", "planId": "a".repeat(32),
            "manifestSha256": "b".repeat(64), "profileId": "private-profile",
            "browserId": "session:private-session", "sessionName": "private-session",
            "targetId": "C".repeat(32), "endpoint": "ws://127.0.0.1:9222/devtools/browser/test",
            "expiresAt": "2030-01-01T00:00:00Z"
        })
    }

    fn valid(value: &Value) -> Result<DateTime<Utc>> {
        Binding::parse(&serde_json::to_vec(value).unwrap())?.validate(
            &"a".repeat(32),
            &"b".repeat(64),
            value["endpoint"].as_str().unwrap_or_default(),
            "private-session",
            DateTime::parse_from_rfc3339("2029-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        )
    }

    #[test]
    fn exact_schema_and_identity_are_required() {
        assert!(valid(&fixture()).is_ok());
        for (key, value) in [
            ("operation", "execute"),
            ("planId", "c"),
            ("manifestSha256", "secret-canary"),
            ("profileId", "bad/profile"),
            ("browserId", "session:other"),
            ("sessionName", "other"),
            ("targetId", "c"),
            ("expiresAt", "2029-01-01T00:00:00Z"),
            ("expiresAt", "not-a-date"),
            ("unknown", "secret-canary"),
        ] {
            let mut changed = fixture();
            changed[key] = json!(value);
            assert_eq!(valid(&changed), Err(REJECTED));
        }
        let mut missing = fixture();
        missing.as_object_mut().unwrap().remove("profileId");
        assert_eq!(valid(&missing), Err(REJECTED));
        let duplicate = serde_json::to_string(&fixture()).unwrap();
        let duplicate = duplicate.replacen('{', "{\"operation\":\"bind\",", 1);
        assert!(matches!(
            Binding::parse(duplicate.as_bytes()),
            Err(REJECTED)
        ));
    }

    #[test]
    fn unsafe_endpoints_and_caller_drift_are_rejected() {
        for endpoint in [
            "https://localhost/devtools/browser/test",
            "ws://user@localhost/test",
            "ws://@localhost/test",
            "ws://localhost/test?query=secret",
            "ws://localhost/test#fragment",
            "ws://localhost/test\n",
        ] {
            let mut value = fixture();
            value["endpoint"] = json!(endpoint);
            assert_eq!(valid(&value), Err(REJECTED));
        }
        let binding = Binding::parse(&serde_json::to_vec(&fixture()).unwrap()).unwrap();
        let now = DateTime::parse_from_rfc3339("2029-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for (plan, digest, endpoint, session) in [
            (
                "c".repeat(32),
                "b".repeat(64),
                binding.endpoint.as_str(),
                "private-session",
            ),
            (
                "a".repeat(32),
                "c".repeat(64),
                binding.endpoint.as_str(),
                "private-session",
            ),
            (
                "a".repeat(32),
                "b".repeat(64),
                "ws://localhost/other",
                "private-session",
            ),
            (
                "a".repeat(32),
                "b".repeat(64),
                binding.endpoint.as_str(),
                "other",
            ),
        ] {
            assert_eq!(
                binding.validate(&plan, &digest, endpoint, session, now),
                Err(REJECTED)
            );
        }
    }
}
