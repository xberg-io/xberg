//! In-memory [`RehydrationStore`]: moka cache, 24h TTL.
//!
//! This is a straight port of the logic that shipped in
//! `xberg::api::rehydration_store` — same TTL, same capacity, same key
//! prefix — now implementing the trait instead of exposing inherent methods,
//! and namespaced by tenant.

use std::time::Duration;

use async_trait::async_trait;
use moka::sync::Cache;

use crate::DocumentId;
use crate::error::StoreResult;
use crate::rehydration::RehydrationStore;
use crate::tenant::TenantCtx;

const REHYDRATION_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_CAPACITY: u64 = 10_000;

/// Ephemeral rehydration map cache. Entries are lost on process restart —
/// this is the default backend when `XBERG_REHYDRATION_DB_PATH` is unset.
#[derive(Clone)]
pub struct InMemoryRehydrationStore {
    blobs: Cache<(String, String), Vec<u8>>,
}

impl Default for InMemoryRehydrationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRehydrationStore {
    /// Create a new empty store with the default TTL and capacity.
    pub fn new() -> Self {
        let blobs = Cache::builder()
            .max_capacity(MAX_CAPACITY)
            .time_to_live(REHYDRATION_TTL)
            .build();
        Self { blobs }
    }

    fn namespaced_key(ctx: &TenantCtx, id: &str) -> (String, String) {
        (ctx.tenant.0.clone(), id.to_string())
    }
}

#[async_trait]
impl RehydrationStore for InMemoryRehydrationStore {
    async fn put_map(&self, ctx: &TenantCtx, blob: Vec<u8>) -> StoreResult<DocumentId> {
        let id = format!("reh_{}", uuid::Uuid::new_v4());
        self.blobs.insert(Self::namespaced_key(ctx, &id), blob);
        Ok(DocumentId(id))
    }

    async fn get_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.blobs.get(&Self::namespaced_key(ctx, &id.0)))
    }

    async fn delete_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool> {
        let key = Self::namespaced_key(ctx, &id.0);
        let existed = self.blobs.contains_key(&key);
        self.blobs.invalidate(&key);
        Ok(existed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let store = InMemoryRehydrationStore::new();
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![1, 2, 3]).await.expect("put");
        assert!(id.0.starts_with("reh_"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get"), Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn get_missing_id_returns_none() {
        let store = InMemoryRehydrationStore::new();
        let ctx = TenantCtx::default_tenant();
        let missing = DocumentId("reh_nonexistent".to_string());
        assert_eq!(store.get_map(&ctx, &missing).await.expect("get"), None);
    }

    #[tokio::test]
    async fn each_put_call_gets_a_distinct_id() {
        let store = InMemoryRehydrationStore::new();
        let ctx = TenantCtx::default_tenant();
        let a = store.put_map(&ctx, vec![1]).await.expect("put a");
        let b = store.put_map(&ctx, vec![1]).await.expect("put b");
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn different_tenants_do_not_see_each_others_maps() {
        let store = InMemoryRehydrationStore::new();
        let tenant_a = TenantCtx::new("tenant-a", "user");
        let tenant_b = TenantCtx::new("tenant-b", "user");
        let id = store.put_map(&tenant_a, vec![9, 9, 9]).await.expect("put");
        assert_eq!(store.get_map(&tenant_a, &id).await.expect("get a"), Some(vec![9, 9, 9]));
        assert_eq!(store.get_map(&tenant_b, &id).await.expect("get b"), None);
    }

    #[tokio::test]
    async fn delete_returns_false_for_unknown_id() {
        let store = InMemoryRehydrationStore::new();
        let ctx = TenantCtx::default_tenant();
        let missing = DocumentId("reh_nonexistent".to_string());
        assert!(!store.delete_map(&ctx, &missing).await.expect("delete"));
    }

    #[tokio::test]
    async fn delete_removes_the_map() {
        let store = InMemoryRehydrationStore::new();
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![7]).await.expect("put");
        assert!(store.delete_map(&ctx, &id).await.expect("delete"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get after delete"), None);
    }
}
