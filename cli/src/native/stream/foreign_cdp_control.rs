use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const MAX_BORROW_TTL_SECONDS: u64 = 900;

/// A short-lived, operator-bound authority grant for interactive CDP input.
///
/// A grant never transfers browser lifecycle ownership. It only authorizes the
/// fixed input operations implemented by the foreign-CDP dashboard endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForeignCdpBorrowGrant {
    pub(crate) id: String,
    pub(crate) port: u16,
    pub(crate) target_id: String,
    pub(crate) owner: String,
    pub(crate) reason: String,
    pub(crate) issued_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
}

#[derive(Default)]
pub(crate) struct ForeignCdpBorrowRegistry {
    grants: HashMap<(u16, String), ForeignCdpBorrowGrant>,
}

impl ForeignCdpBorrowRegistry {
    pub(crate) fn borrow(
        &mut self,
        port: u16,
        target_id: &str,
        owner: &str,
        reason: &str,
        ttl_seconds: u64,
        now: DateTime<Utc>,
    ) -> Result<ForeignCdpBorrowGrant, String> {
        let target_id = target_id.trim();
        let owner = owner.trim();
        let reason = reason.trim();
        if port == 0 {
            return Err("Borrow requires a valid CDP port".to_string());
        }
        if target_id.is_empty() {
            return Err("Borrow requires a CDP target ID".to_string());
        }
        if owner.is_empty() {
            return Err("Borrow requires an authenticated operator".to_string());
        }
        if reason.is_empty() {
            return Err("Borrow requires a reason".to_string());
        }
        if ttl_seconds == 0 {
            return Err("Borrow TTL must be greater than zero".to_string());
        }

        let key = (port, target_id.to_string());
        if let Some(current) = self.grants.get(&key) {
            if current.expires_at > now && current.owner != owner {
                return Err(format!(
                    "CDP target is already borrowed by {} until {}",
                    current.owner,
                    current.expires_at.to_rfc3339()
                ));
            }
        }

        let ttl_seconds = ttl_seconds.min(MAX_BORROW_TTL_SECONDS);
        let grant = ForeignCdpBorrowGrant {
            id: Uuid::new_v4().to_string(),
            port,
            target_id: target_id.to_string(),
            owner: owner.to_string(),
            reason: reason.to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(ttl_seconds as i64),
        };
        self.grants.insert(key, grant.clone());
        Ok(grant)
    }

    pub(crate) fn status(
        &mut self,
        port: u16,
        target_id: &str,
        now: DateTime<Utc>,
    ) -> Option<ForeignCdpBorrowGrant> {
        let key = (port, target_id.trim().to_string());
        if self
            .grants
            .get(&key)
            .is_some_and(|grant| grant.expires_at <= now)
        {
            self.grants.remove(&key);
        }
        self.grants.get(&key).cloned()
    }

    pub(crate) fn authorize(
        &mut self,
        port: u16,
        target_id: &str,
        grant_id: &str,
        owner: &str,
        now: DateTime<Utc>,
    ) -> Result<ForeignCdpBorrowGrant, String> {
        let Some(grant) = self.status(port, target_id, now) else {
            return Err("No active Borrow grant exists for this CDP target".to_string());
        };
        if grant.id != grant_id.trim() || grant.owner != owner.trim() {
            return Err("Borrow grant does not authorize this operator and target".to_string());
        }
        Ok(grant)
    }

    pub(crate) fn release(
        &mut self,
        port: u16,
        target_id: &str,
        grant_id: &str,
        owner: &str,
    ) -> Result<ForeignCdpBorrowGrant, String> {
        let key = (port, target_id.trim().to_string());
        let Some(grant) = self.grants.get(&key) else {
            return Err("No active Borrow grant exists for this CDP target".to_string());
        };
        if grant.id != grant_id.trim() || grant.owner != owner.trim() {
            return Err("Borrow grant does not authorize this release".to_string());
        }
        self.grants
            .remove(&key)
            .ok_or_else(|| "Borrow grant disappeared before release".to_string())
    }
}

fn registry() -> &'static Mutex<ForeignCdpBorrowRegistry> {
    static REGISTRY: OnceLock<Mutex<ForeignCdpBorrowRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ForeignCdpBorrowRegistry::default()))
}

pub(crate) fn borrow(
    port: u16,
    target_id: &str,
    owner: &str,
    reason: &str,
    ttl_seconds: u64,
) -> Result<ForeignCdpBorrowGrant, String> {
    registry()
        .lock()
        .map_err(|_| "Foreign CDP Borrow registry lock was poisoned".to_string())?
        .borrow(port, target_id, owner, reason, ttl_seconds, Utc::now())
}

pub(crate) fn status(port: u16, target_id: &str) -> Result<Option<ForeignCdpBorrowGrant>, String> {
    Ok(registry()
        .lock()
        .map_err(|_| "Foreign CDP Borrow registry lock was poisoned".to_string())?
        .status(port, target_id, Utc::now()))
}

pub(crate) fn authorize(
    port: u16,
    target_id: &str,
    grant_id: &str,
    owner: &str,
) -> Result<ForeignCdpBorrowGrant, String> {
    registry()
        .lock()
        .map_err(|_| "Foreign CDP Borrow registry lock was poisoned".to_string())?
        .authorize(port, target_id, grant_id, owner, Utc::now())
}

pub(crate) fn release(
    port: u16,
    target_id: &str,
    grant_id: &str,
    owner: &str,
) -> Result<ForeignCdpBorrowGrant, String> {
    registry()
        .lock()
        .map_err(|_| "Foreign CDP Borrow registry lock was poisoned".to_string())?
        .release(port, target_id, grant_id, owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 5, 12, 0, 0).unwrap()
    }

    #[test]
    fn borrow_is_scoped_to_operator_port_and_target() {
        let mut registry = ForeignCdpBorrowRegistry::default();
        let grant = registry
            .borrow(
                9223,
                "target-a",
                "operator-a",
                "investigate checkout",
                300,
                now(),
            )
            .unwrap();

        assert_eq!(grant.port, 9223);
        assert_eq!(grant.target_id, "target-a");
        assert_eq!(grant.owner, "operator-a");
        assert_eq!(grant.expires_at, now() + Duration::seconds(300));
        assert!(registry
            .authorize(
                9223,
                "target-a",
                &grant.id,
                "operator-a",
                now() + Duration::seconds(1),
            )
            .is_ok());
        assert!(registry
            .authorize(
                9223,
                "target-a",
                &grant.id,
                "operator-b",
                now() + Duration::seconds(1),
            )
            .is_err());
    }

    #[test]
    fn borrow_ttl_is_capped_and_expiry_fails_closed() {
        let mut registry = ForeignCdpBorrowRegistry::default();
        let grant = registry
            .borrow(9223, "target-a", "operator-a", "diagnosis", 86_400, now())
            .unwrap();

        assert_eq!(grant.expires_at, now() + Duration::seconds(900));
        assert!(registry
            .authorize(
                9223,
                "target-a",
                &grant.id,
                "operator-a",
                now() + Duration::seconds(901),
            )
            .is_err());
        assert!(registry
            .status(9223, "target-a", now() + Duration::seconds(901))
            .is_none());
    }

    #[test]
    fn release_invalidates_the_grant() {
        let mut registry = ForeignCdpBorrowRegistry::default();
        let grant = registry
            .borrow(9223, "target-a", "operator-a", "diagnosis", 300, now())
            .unwrap();

        registry
            .release(9223, "target-a", &grant.id, "operator-a")
            .unwrap();
        assert!(registry
            .authorize(
                9223,
                "target-a",
                &grant.id,
                "operator-a",
                now() + Duration::seconds(1),
            )
            .is_err());
    }

    #[test]
    fn empty_reason_and_zero_ttl_are_rejected() {
        let mut registry = ForeignCdpBorrowRegistry::default();
        assert!(registry
            .borrow(9223, "target-a", "operator-a", "", 300, now())
            .is_err());
        assert!(registry
            .borrow(9223, "target-a", "operator-a", "diagnosis", 0, now())
            .is_err());
    }
}
