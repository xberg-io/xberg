//! Tenant-scoped sidecar persistence for the Xberg HTTP API.
//!
//! This crate never stores vectors or full document text — that stays in
//! `xberg-rag`. It owns the ID-keyed sidecar state the corpus has no place
//! for: encrypted rehydration maps today; durable jobs and an audit log in
//! future phases. See
//! `docs/superpowers/specs/2026-06-30-api-document-store-design.md`.
//!
//! ## Feature layers
//!
//! - `in-memory` (default): ephemeral [`backends::memory::InMemoryRehydrationStore`]
//!   (moka, 24h TTL). Matches the behavior shipped in `xberg::api` prior to
//!   this crate — entries are lost on process restart.
//! - `sqlite`: durable [`backends::sqlite::SqliteRehydrationStore`] (WAL-mode
//!   `rusqlite`, tenant + id primary key).

pub mod backends;
pub mod error;
pub mod rehydration;
pub mod tenant;

pub use error::{StoreError, StoreResult};
pub use rehydration::RehydrationStore;
pub use tenant::{ActorId, TenantCtx, TenantId};

/// Opaque identifier for a document, assigned by the backend.
///
/// Structurally identical to `xberg_rag::types::DocumentId`, but defined
/// locally to avoid a manifest-level dependency on `xberg-rag` (which would
/// otherwise create a `xberg -> xberg-doc-store -> xberg-rag -> xberg` cycle
/// once `xberg` depends on this crate). Nothing in this crate's current
/// scope requires the two `DocumentId` types to be identical; a future plan
/// will reconcile identity if the corpus / `DocumentStore` integration ever
/// needs it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct DocumentId(pub String);

/// Build the process-wide [`RehydrationStore`] from environment configuration.
///
/// If `XBERG_REHYDRATION_DB_PATH` is set and the `sqlite` feature is
/// compiled in, opens a durable [`backends::sqlite::SqliteRehydrationStore`]
/// at that path. Otherwise falls back to the ephemeral
/// [`backends::memory::InMemoryRehydrationStore`] (24h TTL, lost on restart).
///
/// # Errors
///
/// Returns [`StoreError::Backend`] if `XBERG_REHYDRATION_DB_PATH` is set but
/// the database cannot be opened (bad path, permissions, corrupt file).
#[cfg(feature = "in-memory")]
pub fn rehydration_store_from_env() -> StoreResult<std::sync::Arc<dyn RehydrationStore>> {
    #[cfg(feature = "sqlite")]
    if let Ok(path) = std::env::var("XBERG_REHYDRATION_DB_PATH") {
        let store = backends::sqlite::SqliteRehydrationStore::open(&path)?;
        tracing::info!(path = %path, "rehydration store: durable SQLite backend");
        return Ok(std::sync::Arc::new(store));
    }
    #[cfg(not(feature = "sqlite"))]
    if std::env::var("XBERG_REHYDRATION_DB_PATH").is_ok() {
        tracing::warn!(
            "XBERG_REHYDRATION_DB_PATH is set but this binary was built without the \
             `sqlite` feature; rehydration maps will NOT be durable — rebuild with \
             `doc-store-sqlite` enabled for the configured path to take effect"
        );
    }
    tracing::warn!(
        "rehydration store: ephemeral in-memory backend (24h TTL, lost on restart); \
         set XBERG_REHYDRATION_DB_PATH for durability"
    );
    Ok(std::sync::Arc::new(backends::memory::InMemoryRehydrationStore::new()))
}

#[cfg(all(test, feature = "sqlite"))]
mod factory_tests {
    use super::*;

    #[test]
    fn env_var_unset_selects_in_memory_backend() {
        // No other test in this crate's test binary sets XBERG_REHYDRATION_DB_PATH
        // (the only test that does — crates/xberg's rehydration_durability.rs —
        // lives in a different crate and compiles to a separate test process),
        // so the var is unconditionally absent here: this assertion always runs.
        assert!(
            std::env::var("XBERG_REHYDRATION_DB_PATH").is_err(),
            "no test in this crate sets XBERG_REHYDRATION_DB_PATH; if this fails, \
             a new test introduced env pollution and must be fixed, not worked around"
        );
        let store = rehydration_store_from_env().expect("factory must succeed");
        // Type-erased behind Arc<dyn RehydrationStore>; a round trip proves
        // *a* working backend was selected without needing downcasting.
        let ctx = TenantCtx::default_tenant();
        let handle = tokio::runtime::Runtime::new().expect("rt");
        handle.block_on(async {
            let id = store.put_map(&ctx, vec![1]).await.expect("put");
            assert_eq!(store.get_map(&ctx, &id).await.expect("get"), Some(vec![1]));
        });
    }
}
