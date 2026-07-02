//! The [`RehydrationStore`] contract: encrypted PII-map blobs, addressed by
//! a backend-assigned [`DocumentId`].

use async_trait::async_trait;

use crate::DocumentId;
use crate::error::StoreResult;
use crate::tenant::TenantCtx;

/// Tenant-scoped storage for encrypted rehydration map blobs.
///
/// # Invariants
///
/// - The store persists **ciphertext only**. Callers pass already-encrypted
///   bytes (the `XPII\x01`-framed AES-256-GCM blob from
///   `xberg::text::redaction::encrypt_map`); this trait never sees a
///   passphrase or a derived key.
/// - `id` is backend-assigned: [`put_map`](RehydrationStore::put_map) chooses
///   it and returns it. There is no caller-supplied-id overload — this
///   matches [`DocumentId`]'s own contract ("assigned by the backend").
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn RehydrationStore>`; they may be called concurrently.
#[async_trait]
pub trait RehydrationStore: Send + Sync + 'static {
    /// Store an encrypted map blob under a freshly assigned [`DocumentId`].
    async fn put_map(&self, ctx: &TenantCtx, blob: Vec<u8>) -> StoreResult<DocumentId>;

    /// Fetch the encrypted blob for `id`, or `None` if absent / expired /
    /// not visible to `ctx.tenant`.
    async fn get_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>>;

    /// Delete the map for `id`. Returns whether anything existed.
    async fn delete_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A minimal in-test double proving the trait is object-safe and usable
    /// behind `Arc<dyn RehydrationStore>` — the exact shape `ApiState` needs.
    struct DummyStore {
        blobs: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl RehydrationStore for DummyStore {
        async fn put_map(&self, _ctx: &TenantCtx, blob: Vec<u8>) -> StoreResult<DocumentId> {
            let id = "dummy-1".to_string();
            self.blobs.lock().expect("lock").insert(id.clone(), blob);
            Ok(DocumentId(id))
        }

        async fn get_map(&self, _ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>> {
            Ok(self.blobs.lock().expect("lock").get(&id.0).cloned())
        }

        async fn delete_map(&self, _ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool> {
            Ok(self.blobs.lock().expect("lock").remove(&id.0).is_some())
        }
    }

    #[tokio::test]
    async fn trait_object_round_trips_through_arc_dyn() {
        let store: std::sync::Arc<dyn RehydrationStore> = std::sync::Arc::new(DummyStore {
            blobs: Mutex::new(HashMap::new()),
        });
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![1, 2, 3]).await.expect("put");
        let got = store.get_map(&ctx, &id).await.expect("get");
        assert_eq!(got, Some(vec![1, 2, 3]));
        assert!(store.delete_map(&ctx, &id).await.expect("delete"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get after delete"), None);
    }
}
