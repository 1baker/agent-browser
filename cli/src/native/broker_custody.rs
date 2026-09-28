//! Shared ownership of the original custody payload, with sticky revocation.
//! This is a lifetime barrier, not a substitute for verifying the held lease.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::{OwnedRwLockReadGuard, RwLock};

const REVOKED: &str = "broker_custody_revoked";
const DRAIN_TIMEOUT: &str = "broker_custody_drain_timeout";

struct Shared<T> {
    payload: T,
    revoked: AtomicBool,
    barrier: Arc<RwLock<()>>,
}

/// Sole revocation owner. Moving this value transfers ownership; cloning it is
/// deliberately unsupported. Dropping it seals all existing handles.
pub(crate) struct SharedCustodyOwner<T> {
    shared: Arc<Shared<T>>,
}

pub(crate) struct SharedCustodyHandle<T> {
    shared: Weak<Shared<T>>,
}

impl<T> Clone for SharedCustodyHandle<T> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

/// Holds a read barrier and the original payload, never a cloned proof.
pub(crate) struct SharedCustodyPermit<T> {
    // Release the barrier before releasing our payload reference.
    _guard: OwnedRwLockReadGuard<()>,
    shared: Arc<Shared<T>>,
}

impl<T> SharedCustodyOwner<T> {
    pub(crate) fn new(payload: T) -> Self {
        Self {
            shared: Arc::new(Shared {
                payload,
                revoked: AtomicBool::new(false),
                barrier: Arc::new(RwLock::new(())),
            }),
        }
    }

    pub(crate) fn payload(&self) -> &T {
        &self.shared.payload
    }

    pub(crate) fn handle(&self) -> SharedCustodyHandle<T> {
        SharedCustodyHandle {
            shared: Arc::downgrade(&self.shared),
        }
    }

    pub(crate) fn is_revoked(&self) -> bool {
        self.shared.revoked.load(Ordering::SeqCst)
    }

    /// Seals before waiting. Cancellation and timeout retain both the seal and
    /// the owner's original payload. Success means all admitted readers drained.
    pub(crate) async fn revoke(&self, timeout: Duration) -> Result<(), String> {
        self.shared.revoked.store(true, Ordering::SeqCst);
        let guard = tokio::time::timeout(timeout, self.shared.barrier.clone().write_owned())
            .await
            .map_err(|_| DRAIN_TIMEOUT.to_string())?;
        drop(guard);
        Ok(())
    }
}

impl<T> Drop for SharedCustodyOwner<T> {
    fn drop(&mut self) {
        self.shared.revoked.store(true, Ordering::SeqCst);
    }
}

impl<T> SharedCustodyHandle<T> {
    pub(crate) fn is_revoked(&self) -> bool {
        self.shared
            .upgrade()
            .is_none_or(|shared| shared.revoked.load(Ordering::SeqCst))
    }

    pub(crate) async fn acquire(&self) -> Result<SharedCustodyPermit<T>, String> {
        let shared = self.shared.upgrade().ok_or_else(|| REVOKED.to_string())?;
        if shared.revoked.load(Ordering::SeqCst) {
            return Err(REVOKED.into());
        }
        let guard = shared.barrier.clone().read_owned().await;
        if shared.revoked.load(Ordering::SeqCst) {
            return Err(REVOKED.into());
        }
        Ok(SharedCustodyPermit {
            _guard: guard,
            shared,
        })
    }
}

impl<T> SharedCustodyPermit<T> {
    pub(crate) fn payload(&self) -> &T {
        &self.shared.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Payload(Arc<AtomicUsize>);
    impl Drop for Payload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn concurrent_permits_reference_original_and_block_revocation() {
        let owner = SharedCustodyOwner::new(String::from("original"));
        let handle = owner.handle();
        let other = handle.clone();
        let (first, second) = tokio::join!(handle.acquire(), other.acquire());
        let first = first.unwrap();
        let second = second.unwrap();
        assert!(std::ptr::eq(owner.payload(), first.payload()));
        assert!(std::ptr::eq(first.payload(), second.payload()));
        assert_eq!(
            owner.revoke(Duration::from_millis(1)).await,
            Err(DRAIN_TIMEOUT.into())
        );
        assert!(owner.is_revoked() && handle.is_revoked());
        assert!(handle.acquire().await.is_err());
        drop(first);
        assert_eq!(
            owner.revoke(Duration::from_millis(1)).await,
            Err(DRAIN_TIMEOUT.into())
        );
        drop(second);
        owner.revoke(Duration::from_secs(1)).await.unwrap();
        assert!(handle.acquire().await.is_err());
    }

    #[tokio::test]
    async fn owner_drop_retains_original_until_last_permit() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = SharedCustodyOwner::new(Payload(drops.clone()));
        let handle = owner.handle();
        let first = handle.acquire().await.unwrap();
        let second = handle.acquire().await.unwrap();
        drop(owner);
        assert!(handle.acquire().await.is_err());
        drop(handle);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(first);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(second);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancelled_revocation_is_sticky_and_preserves_payload() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = SharedCustodyOwner::new(Payload(drops.clone()));
        let handle = owner.handle();
        let permit = handle.acquire().await.unwrap();
        // Outer timeout drops the pending revocation future before its deadline.
        assert!(tokio::time::timeout(
            Duration::from_millis(1),
            owner.revoke(Duration::from_secs(60))
        )
        .await
        .is_err());
        assert!(handle.acquire().await.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(permit);
        owner.revoke(Duration::from_secs(1)).await.unwrap();
        assert!(handle.acquire().await.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(handle);
        drop(owner);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn handle_alone_cannot_authorize_after_owner_drop() {
        let owner = SharedCustodyOwner::new(42);
        let handle = owner.handle();
        drop(owner);
        assert!(handle.is_revoked());
        assert_eq!(handle.acquire().await.err(), Some(REVOKED.into()));
        assert_eq!(handle.clone().acquire().await.err(), Some(REVOKED.into()));
    }

    #[tokio::test]
    async fn stale_handle_does_not_retain_original_custody_payload() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = SharedCustodyOwner::new(Payload(drops.clone()));
        let handle = owner.handle();
        let permit = handle.acquire().await.unwrap();
        drop(owner);
        assert!(handle.is_revoked());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert!(handle.acquire().await.is_err());
        drop(permit);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(handle.is_revoked());
        assert!(handle.clone().acquire().await.is_err());
        drop(handle);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}
