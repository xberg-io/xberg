# API Document Store — Durable Rehydration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ephemeral (24h TTL, in-memory, lost-on-restart) rehydration-map store behind `POST /v1/process` and `POST /v1/documents/{id}/rehydrate` with a trait-based, tenant-scoped store that has a durable SQLite backend, without changing the wire contract of either endpoint.

**Architecture:** A new isolated crate `xberg-doc-store` defines an object-safe `RehydrationStore` trait plus two backends — `InMemoryRehydrationStore` (the existing moka logic, ported verbatim) and `SqliteRehydrationStore` (new, WAL-mode `rusqlite`, tenant + id primary key). `xberg`'s `ApiState.rehydration_store` field changes from a concrete struct to `Arc<dyn xberg_doc_store::RehydrationStore>`; `process_handler`/`rehydrate_handler` call the trait instead of inherent methods. Backend selection happens once at router construction via `XBERG_REHYDRATION_DB_PATH`.

**Tech Stack:** Rust 2024, `rusqlite` 0.40 (bundled SQLite, WAL mode), `moka` 0.12 (sync cache), `async-trait`, `tokio::task::spawn_blocking` for the SQLite backend, `xberg-rag`'s `DocumentId` type (reused, not redefined).

**Scope decision (read before starting):** The design spec (`docs/superpowers/specs/2026-06-30-api-document-store-design.md`) bundles `DocumentStore`, `RehydrationStore`, a durable `JobStore`, and `AuditSink` into "Phase 1." This plan implements **only `RehydrationStore`** — the piece with a real route consumer today (`/v1/process`, `/v1/documents/{id}/rehydrate`). `DocumentStore`/`JobStore`/`AuditSink` have zero route consumers yet (`GET /v1/documents`, `DELETE /v1/documents/{id}`, `GET /v1/audit`, durable job polling are all unbuilt), so adding their traits now would be speculative surface with nothing exercising it — a violation of `avoid-duplication`/YAGNI. Each is a follow-up plan once its route lands. This plan alone converts the spec's biggest concrete risk (map loss on restart) into a shipped fix.

**Trait deviation from the spec (evidence-based, applied here):** Spec §4.2 shows `put_map(ctx, id: &DocumentId, blob) -> StoreResult<()>` — a caller-supplied id. The shipped handler (`crates/xberg/src/api/handlers.rs:1283`) generates the id server-side (`state.rehydration_store.store(encrypted) -> String`) and returns it to the caller as `rehydration_key`. This plan's trait matches the shipped, tested behavior instead: `put_map(ctx, blob) -> StoreResult<DocumentId>`. This is also the more correct reading of `xberg-rag`'s own `DocumentId` doc comment — *"Opaque identifier for a document, **assigned by the backend**"* (`crates/xberg-rag/src/types.rs:10`) — so this is a correction, not a regression.

**Course correction discovered during execution (Task 7):** Tasks 1 and 3, as originally written and already reviewed/committed, made `xberg-doc-store` depend on `xberg-rag` (`vector-store` feature) solely to re-export `xberg_rag::types::DocumentId`, so the plan's "no second ID space" goal from the design spec would hold. Task 7 (wiring `xberg` → `xberg-doc-store`) discovered this is impossible: `xberg-rag` carries a pre-existing, unrelated optional dependency on `xberg` (for its `pipeline` feature), and Cargo's cyclic-package-dependency check operates on the **manifest graph**, not the feature-resolved graph — an unactivated optional dependency edge still counts. The resulting cycle (`xberg → xberg-doc-store → xberg-rag → xberg`) makes `cargo check -p xberg` fail at the resolution stage, before any code is even compiled. Verified directly: `cargo check -p xberg --features api` reproduced `error: cyclic package dependency`. **Fix (applied as an amendment to Tasks 1 and 3, re-reviewed):** `xberg-doc-store` drops its dependency on `xberg-rag` entirely and defines its own `DocumentId(pub String)` newtype, structurally identical to `xberg-rag`'s. This is acceptable because no code in this plan's scope needs the two IDs to be the same type — the corpus/`DocumentStore` integration that would have required it is already deferred out of scope (see the Scope decision above). A future plan that wires the corpus in can decide then how to reconcile identity, informed by whatever `xberg-rag`'s dependency shape looks like at that point. The task bodies below still show the original (superseded) code for historical accuracy; the amendment is applied as fix commits on top, called out in Tasks 1/3/4/5's status.

## Global Constraints

- Rust 2024 edition, `cargo fmt` + `clippy -D warnings`, zero warnings policy.
- `Result<T, E>` with `thiserror` — never `.unwrap()`/`panic!` in library code (tests may use `.expect()`).
- Every `unsafe` block needs a `// SAFETY:` comment. This plan introduces no new `unsafe` code.
- New workspace member crate name: `xberg-doc-store`. Crate lib name: `xberg_doc_store`.
- `rusqlite = "0.40"` (bundled feature — already workspace-pinned, do not bump).
- The `RehydrationStore` trait must stay object-safe (no generics, no associated types) — it is held behind `Arc<dyn RehydrationStore>`, exactly like `xberg-rag`'s `VectorStore`.
- Every trait method takes `&TenantCtx` first — tenancy is in every signature from day one per the spec's resolved decision, even though this plan ships only the single-tenant (`"default"`) path.
- The rehydration map store persists **ciphertext only** — the scrypt-derived key and passphrase never reach this crate (`pii-pipeline` rule). This plan does not touch `crates/xberg/src/text/redaction/rehydration.rs` (the crypto).
- Zero wire-format change: `POST /v1/process`'s `rehydration_key` field and `POST /v1/documents/{rehydration_key}/rehydrate`'s path/body shape are unchanged. Only the storage backing changes.
- `crates/xberg` (core) still must not gain a direct dependency on `xberg-rag` — it depends only on `xberg-doc-store`. **Superseded by the course correction above:** `xberg-doc-store` does NOT depend on `xberg-rag` at all (a real Cargo cycle, not a style preference) — it defines its own `DocumentId(pub String)` newtype instead of re-exporting `xberg-rag`'s.

---

## File Structure

New crate `crates/xberg-doc-store/`:

| File | Responsibility |
|---|---|
| `Cargo.toml` | Crate manifest: `in-memory` (default) + `sqlite` feature layers. |
| `src/lib.rs` | Module wiring, public re-exports, `rehydration_store_from_env()` factory. |
| `src/error.rs` | `StoreError` / `StoreResult`. |
| `src/tenant.rs` | `TenantId`, `ActorId`, `TenantCtx`. |
| `src/rehydration.rs` | The `RehydrationStore` trait. |
| `src/backends/mod.rs` | Feature-gated backend module wiring. |
| `src/backends/memory.rs` | `InMemoryRehydrationStore` (moka, ported from `xberg::api::rehydration_store`). |
| `src/backends/sqlite.rs` | `SqliteRehydrationStore` (durable, WAL-mode). |

Modified in `crates/xberg`:

| File | Change |
|---|---|
| `Cargo.toml` (workspace root) | New member, `workspace.dependencies` entry, `[patch.crates-io]` entry. |
| `crates/xberg/Cargo.toml` | New optional dep `xberg-doc-store`; `api` feature gains `dep:xberg-doc-store`; new `doc-store-sqlite` feature. |
| `crates/xberg/src/api/mod.rs` | Remove the `rehydration_store` module declaration. |
| `crates/xberg/src/api/rehydration_store.rs` | **Deleted** — superseded by `xberg-doc-store`. |
| `crates/xberg/src/api/types.rs` | `ApiState.rehydration_store` field type → `Arc<dyn xberg_doc_store::RehydrationStore>`. |
| `crates/xberg/src/api/router.rs` | Construct the store via `xberg_doc_store::rehydration_store_from_env()`. |
| `crates/xberg/src/api/handlers.rs` | `process_handler`/`rehydrate_handler` call the trait; test helper + 5 existing tests updated. |
| `crates/xberg/tests/rehydration_durability.rs` | **New** — proves a map survives store-drop-and-reopen via the SQLite backend. |
| `CHANGELOG.md` | New `[Unreleased]` entry. |

---

### Task 1: Scaffold the `xberg-doc-store` crate + workspace wiring

**Files:**
- Create: `crates/xberg-doc-store/Cargo.toml`
- Create: `crates/xberg-doc-store/src/lib.rs`
- Create: `crates/xberg-doc-store/src/error.rs`
- Modify: `Cargo.toml` (workspace root)

**Interfaces:**
- Produces: `xberg_doc_store::{StoreError, StoreResult}` — consumed by every later task in this crate.

- [ ] **Step 1: Add the workspace member + dependency entries**

In `Cargo.toml`, add to `members` (after `"crates/xberg-cli",`, before `"crates/xberg-ffi",` — keep alphabetical-ish grouping with the other `xberg-*` crates):

```toml
    "crates/xberg-doc-store",
```

Add to `[workspace.dependencies]` (alongside the existing `xberg-rag` line):

```toml
xberg-doc-store = { path = "./crates/xberg-doc-store", version = "1.0.0-rc.1", default-features = false }
```

Add to `[patch.crates-io]`:

```toml
xberg-doc-store = { path = "crates/xberg-doc-store" }
```

- [ ] **Step 2: Write the crate manifest**

Create `crates/xberg-doc-store/Cargo.toml`:

```toml
[package]
name = "xberg-doc-store"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Tenant-scoped sidecar persistence for the Xberg HTTP API: encrypted rehydration maps today, durable jobs and an audit log in future phases. Never vectors or full text — that stays in xberg-rag."

[features]
default = ["in-memory"]
in-memory = ["dep:moka"]
sqlite = ["dep:rusqlite", "dep:tokio"]

[dependencies]
async-trait = { workspace = true }
moka = { version = "0.12", features = ["sync"], optional = true }
rusqlite = { workspace = true, optional = true }
thiserror = { workspace = true }
tokio = { workspace = true, optional = true }
tracing = { workspace = true }
uuid = { version = "1", features = ["v4"] }
xberg-rag = { workspace = true, default-features = false, features = ["vector-store"] }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["rt", "rt-multi-thread", "macros"] }

[lints]
workspace = true
```

- [ ] **Step 3: Write the error type**

Create `crates/xberg-doc-store/src/error.rs`:

```rust
//! Error model for `xberg-doc-store`.

use thiserror::Error;

/// Result type used throughout `xberg-doc-store`.
pub type StoreResult<T> = Result<T, StoreError>;

/// Errors raised by sidecar-store operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StoreError {
    /// A backend-specific error (SQLite, I/O, join failure, …).
    #[error("doc-store backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_error_displays_source_message() {
        let inner = std::io::Error::other("disk full");
        let err = StoreError::Backend(Box::new(inner));
        assert!(err.to_string().contains("disk full"));
    }
}
```

- [ ] **Step 4: Write the crate root**

Create `crates/xberg-doc-store/src/lib.rs`:

```rust
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

pub mod error;

pub use error::{StoreError, StoreResult};
```

- [ ] **Step 5: Verify the crate compiles**

Run: `cargo check -p xberg-doc-store`
Expected: compiles with zero warnings (empty crate besides the error module).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/xberg-doc-store/Cargo.toml crates/xberg-doc-store/src/lib.rs crates/xberg-doc-store/src/error.rs
git commit -m "feat(doc-store): scaffold xberg-doc-store crate"
```

---

### Task 2: Tenant context types

**Files:**
- Create: `crates/xberg-doc-store/src/tenant.rs`
- Modify: `crates/xberg-doc-store/src/lib.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `xberg_doc_store::{TenantId, ActorId, TenantCtx}` — the first argument of every trait method added in Task 3+.

- [ ] **Step 1: Write the failing test**

Create `crates/xberg-doc-store/src/tenant.rs` with the test module only first — actually for a plain data-type task, write test + implementation together since there is no behavior to fail against; instead verify the constructor contract directly. Create the full file:

```rust
//! Tenant/actor identity threaded through every sidecar-store call.

/// Newtype over a tenant identifier. Opaque to this crate — a store instance
/// (SQLite table, pgvector schema, …) partitions all queries by this value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenantId(pub String);

/// Newtype over an actor identifier, used for audit attribution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActorId(pub String);

/// The tenant/actor pair every sidecar-store call is scoped to.
///
/// Carried in every trait method signature from day one (per the design
/// spec's resolved tenancy decision) even when the caller only ever
/// constructs [`TenantCtx::default_tenant`] — so enabling multi-tenancy later
/// is a backend swap, not a breaking API change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantCtx {
    /// The trust domain this call is scoped to.
    pub tenant: TenantId,
    /// The identity performing the call (for audit attribution).
    pub actor: ActorId,
}

impl TenantCtx {
    /// Construct an explicit tenant/actor context.
    pub fn new(tenant: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            tenant: TenantId(tenant.into()),
            actor: ActorId(actor.into()),
        }
    }

    /// The single-tenant context used until a `TenantResolver` (auth layer,
    /// out of scope for this crate) is wired into the API.
    pub fn default_tenant() -> Self {
        Self::new("default", "anonymous")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tenant_uses_default_tenant_id() {
        let ctx = TenantCtx::default_tenant();
        assert_eq!(ctx.tenant, TenantId("default".to_string()));
        assert_eq!(ctx.actor, ActorId("anonymous".to_string()));
    }

    #[test]
    fn new_wraps_supplied_values() {
        let ctx = TenantCtx::new("acme", "user-42");
        assert_eq!(ctx.tenant.0, "acme");
        assert_eq!(ctx.actor.0, "user-42");
    }
}
```

- [ ] **Step 2: Wire the module into `lib.rs`**

In `crates/xberg-doc-store/src/lib.rs`, add:

```rust
pub mod error;
pub mod tenant;

pub use error::{StoreError, StoreResult};
pub use tenant::{ActorId, TenantCtx, TenantId};
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p xberg-doc-store tenant::`
Expected: `default_tenant_uses_default_tenant_id` and `new_wraps_supplied_values` PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-doc-store/src/tenant.rs crates/xberg-doc-store/src/lib.rs
git commit -m "feat(doc-store): add TenantCtx"
```

---

### Task 3: The `RehydrationStore` trait

**Files:**
- Create: `crates/xberg-doc-store/src/rehydration.rs`
- Modify: `crates/xberg-doc-store/src/lib.rs`

**Interfaces:**
- Consumes: `TenantCtx` (Task 2), `xberg_rag::types::DocumentId`.
- Produces: `xberg_doc_store::{RehydrationStore, DocumentId}` — implemented by Task 4 (in-memory) and Task 5 (SQLite); consumed by `crates/xberg`'s handlers in Task 9/10.

- [ ] **Step 1: Write the trait + an object-safety test**

Create `crates/xberg-doc-store/src/rehydration.rs`:

```rust
//! The [`RehydrationStore`] contract: encrypted PII-map blobs, addressed by
//! a backend-assigned [`DocumentId`].

use async_trait::async_trait;
use xberg_rag::types::DocumentId;

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
```

- [ ] **Step 2: Wire the module and re-export `DocumentId`**

In `crates/xberg-doc-store/src/lib.rs`, add:

```rust
pub mod error;
pub mod rehydration;
pub mod tenant;

pub use error::{StoreError, StoreResult};
pub use rehydration::RehydrationStore;
pub use tenant::{ActorId, TenantCtx, TenantId};
pub use xberg_rag::types::DocumentId;
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p xberg-doc-store rehydration::`
Expected: `trait_object_round_trips_through_arc_dyn` PASSES, proving `Arc<dyn RehydrationStore>` works.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-doc-store/src/rehydration.rs crates/xberg-doc-store/src/lib.rs
git commit -m "feat(doc-store): add RehydrationStore trait"
```

---

### Task 4: `InMemoryRehydrationStore` — the default backend

**Files:**
- Create: `crates/xberg-doc-store/src/backends/mod.rs`
- Create: `crates/xberg-doc-store/src/backends/memory.rs`
- Modify: `crates/xberg-doc-store/src/lib.rs`

**Interfaces:**
- Consumes: `RehydrationStore` (Task 3), `TenantCtx` (Task 2).
- Produces: `xberg_doc_store::backends::memory::InMemoryRehydrationStore` — the fallback used by `crates/xberg` in Task 8, and the moka logic that used to live in `xberg::api::rehydration_store`.

- [ ] **Step 1: Write the failing tests**

Create `crates/xberg-doc-store/src/backends/memory.rs`:

```rust
//! In-memory [`RehydrationStore`]: moka cache, 24h TTL.
//!
//! This is a straight port of the logic that shipped in
//! `xberg::api::rehydration_store` — same TTL, same capacity, same key
//! prefix — now implementing the trait instead of exposing inherent methods,
//! and namespaced by tenant.

use std::time::Duration;

use async_trait::async_trait;
use moka::sync::Cache;

use crate::error::StoreResult;
use crate::rehydration::RehydrationStore;
use crate::tenant::TenantCtx;
use xberg_rag::types::DocumentId;

const REHYDRATION_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_CAPACITY: u64 = 10_000;

/// Ephemeral rehydration map cache. Entries are lost on process restart —
/// this is the default backend when `XBERG_REHYDRATION_DB_PATH` is unset.
#[derive(Clone)]
pub struct InMemoryRehydrationStore {
    blobs: Cache<String, Vec<u8>>,
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

    fn namespaced_key(ctx: &TenantCtx, id: &str) -> String {
        format!("{}/{}", ctx.tenant.0, id)
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
```

Note `DocumentId` needs `PartialEq` for `assert_ne!`/`assert_eq!` — it already derives `PartialEq, Eq` in `xberg-rag/src/types.rs:11`, so no change needed there.

- [ ] **Step 2: Create the backends module + wire into `lib.rs`**

Create `crates/xberg-doc-store/src/backends/mod.rs`:

```rust
//! Backend implementations of the sidecar-store traits.

#[cfg(feature = "in-memory")]
pub mod memory;
#[cfg(feature = "sqlite")]
pub mod sqlite;
```

In `crates/xberg-doc-store/src/lib.rs`, add:

```rust
pub mod backends;
pub mod error;
pub mod rehydration;
pub mod tenant;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p xberg-doc-store backends::memory::`
Expected: all 5 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-doc-store/src/backends/mod.rs crates/xberg-doc-store/src/backends/memory.rs crates/xberg-doc-store/src/lib.rs
git commit -m "feat(doc-store): add InMemoryRehydrationStore backend"
```

---

### Task 5: `SqliteRehydrationStore` — the durable backend

**Files:**
- Create: `crates/xberg-doc-store/src/backends/sqlite.rs`

**Interfaces:**
- Consumes: `RehydrationStore` (Task 3), `TenantCtx` (Task 2), `StoreError::Backend` (Task 1).
- Produces: `xberg_doc_store::backends::sqlite::SqliteRehydrationStore` — used by `rehydration_store_from_env()` in Task 6.

- [ ] **Step 1: Write the failing tests**

Create `crates/xberg-doc-store/src/backends/sqlite.rs`:

```rust
//! Durable, WAL-mode SQLite [`RehydrationStore`].
//!
//! One table, `(tenant, id)` primary key. Blocking `rusqlite` calls are
//! routed through `tokio::task::spawn_blocking`, mirroring the pattern
//! `xberg-rag`'s SQLite backend uses: the connection is `Send` but not
//! `Sync`, so it is wrapped in `Arc<Mutex<Connection>>`, and the mutex is
//! locked only inside the blocking closure — never held across an `.await`.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rusqlite::{Connection, params};
use xberg_rag::types::DocumentId;

use crate::error::{StoreError, StoreResult};
use crate::rehydration::RehydrationStore;
use crate::tenant::TenantCtx;

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn backend_err(e: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Backend(Box::new(e))
}

/// Durable rehydration map store backed by an on-disk SQLite database in WAL
/// mode. Survives process restart — this is the backend selected when
/// `XBERG_REHYDRATION_DB_PATH` is set.
#[derive(Clone)]
pub struct SqliteRehydrationStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteRehydrationStore {
    /// Open (or create) the database at `path` and ensure the schema exists.
    pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let conn = Connection::open(path.as_ref()).map_err(backend_err)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    /// Open an in-memory SQLite database. Test-only: not durable across drop
    /// (SQLite's `:memory:` databases are per-connection).
    #[cfg(test)]
    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory().map_err(backend_err)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    fn init(conn: &Connection) -> StoreResult<()> {
        conn.pragma_update(None, "journal_mode", "WAL").map_err(backend_err)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS rehydration_maps (
                tenant     TEXT NOT NULL,
                id         TEXT NOT NULL,
                blob       BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (tenant, id)
            );",
        )
        .map_err(backend_err)?;
        Ok(())
    }
}

#[async_trait]
impl RehydrationStore for SqliteRehydrationStore {
    async fn put_map(&self, ctx: &TenantCtx, blob: Vec<u8>) -> StoreResult<DocumentId> {
        let id = format!("reh_{}", uuid::Uuid::new_v4());
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.clone();
        let created_at = now_unix();
        tokio::task::spawn_blocking(move || -> StoreResult<()> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            conn.execute(
                "INSERT INTO rehydration_maps (tenant, id, blob, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(tenant, id) DO UPDATE SET blob = excluded.blob, created_at = excluded.created_at",
                params![tenant, row_id, blob, created_at],
            )
            .map_err(backend_err)?;
            Ok(())
        })
        .await
        .map_err(backend_err)??;
        Ok(DocumentId(id))
    }

    async fn get_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>> {
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.0.clone();
        tokio::task::spawn_blocking(move || -> StoreResult<Option<Vec<u8>>> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            let mut stmt = conn
                .prepare("SELECT blob FROM rehydration_maps WHERE tenant = ?1 AND id = ?2")
                .map_err(backend_err)?;
            let mut rows = stmt.query(params![tenant, row_id]).map_err(backend_err)?;
            match rows.next().map_err(backend_err)? {
                Some(row) => Ok(Some(row.get::<_, Vec<u8>>(0).map_err(backend_err)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(backend_err)?
    }

    async fn delete_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool> {
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.0.clone();
        tokio::task::spawn_blocking(move || -> StoreResult<bool> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            let changed = conn
                .execute(
                    "DELETE FROM rehydration_maps WHERE tenant = ?1 AND id = ?2",
                    params![tenant, row_id],
                )
                .map_err(backend_err)?;
            Ok(changed > 0)
        })
        .await
        .map_err(backend_err)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![1, 2, 3]).await.expect("put");
        assert!(id.0.starts_with("reh_"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get"), Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn get_missing_id_returns_none() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let missing = DocumentId("reh_nonexistent".to_string());
        assert_eq!(store.get_map(&ctx, &missing).await.expect("get"), None);
    }

    #[tokio::test]
    async fn different_tenants_do_not_see_each_others_maps() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let tenant_a = TenantCtx::new("tenant-a", "user");
        let tenant_b = TenantCtx::new("tenant-b", "user");
        let id = store.put_map(&tenant_a, vec![9, 9, 9]).await.expect("put");
        assert_eq!(store.get_map(&tenant_a, &id).await.expect("get a"), Some(vec![9, 9, 9]));
        assert_eq!(store.get_map(&tenant_b, &id).await.expect("get b"), None);
    }

    #[tokio::test]
    async fn delete_removes_the_map() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![7]).await.expect("put");
        assert!(store.delete_map(&ctx, &id).await.expect("delete"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get after delete"), None);
    }

    /// The money test: a map written by one store instance is still readable
    /// after that instance is dropped and a fresh instance opens the same
    /// on-disk file — i.e. it actually survives a process restart, unlike
    /// the moka in-memory backend.
    #[tokio::test]
    async fn map_survives_store_drop_and_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("rehydration.sqlite3");
        let ctx = TenantCtx::default_tenant();

        let id = {
            let store = SqliteRehydrationStore::open(&db_path).expect("open");
            store.put_map(&ctx, vec![4, 2, 4, 2]).await.expect("put")
        }; // `store` dropped here — simulates process restart.

        let reopened = SqliteRehydrationStore::open(&db_path).expect("reopen");
        assert_eq!(
            reopened.get_map(&ctx, &id).await.expect("get after reopen"),
            Some(vec![4, 2, 4, 2]),
            "map must survive a store drop + reopen against the same file"
        );
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p xberg-doc-store --features sqlite backends::sqlite::`
Expected: all 5 tests PASS, including `map_survives_store_drop_and_reopen`.

- [ ] **Step 3: Run clippy on the new feature combination**

Run: `cargo clippy -p xberg-doc-store --features sqlite --all-targets -- -D warnings`
Expected: zero warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-doc-store/src/backends/sqlite.rs
git commit -m "feat(doc-store): add durable SqliteRehydrationStore backend"
```

---

### Task 6: `rehydration_store_from_env()` factory

**Files:**
- Modify: `crates/xberg-doc-store/src/lib.rs`

**Interfaces:**
- Consumes: `InMemoryRehydrationStore` (Task 4), `SqliteRehydrationStore` (Task 5).
- Produces: `xberg_doc_store::rehydration_store_from_env() -> StoreResult<Arc<dyn RehydrationStore>>` — called once by `crates/xberg`'s `router.rs` in Task 8. Mirrors the existing `crate::cache_dir::resolve_cache_base()` env-driven-path idiom already used in `crates/xberg/src/api/handlers.rs:1024`.

- [ ] **Step 1: Write the failing test**

In `crates/xberg-doc-store/src/lib.rs`, add the function and its test module at the end of the file:

```rust
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
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p xberg-doc-store --features sqlite factory_tests::`
Expected: `env_var_unset_selects_in_memory_backend` PASSES.

- [ ] **Step 3: Run the full crate test suite across both feature combinations**

Run: `cargo test -p xberg-doc-store` (default features: in-memory only)
Expected: all tests PASS, `backends::sqlite` module absent from the build.

Run: `cargo test -p xberg-doc-store --features sqlite`
Expected: all tests PASS, including the SQLite backend tests.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-doc-store/src/lib.rs
git commit -m "feat(doc-store): add rehydration_store_from_env factory"
```

---

### Task 7: Wire `xberg-doc-store` into `crates/xberg`'s feature graph

**Files:**
- Modify: `crates/xberg/Cargo.toml`
- Modify: `crates/xberg/src/api/mod.rs`
- Delete: `crates/xberg/src/api/rehydration_store.rs`

**Interfaces:**
- Consumes: `xberg_doc_store` crate (Tasks 1–6).
- Produces: `xberg` crate features `api` (now pulling in `xberg-doc-store` with the `in-memory` backend) and `doc-store-sqlite` (adds the durable backend) — consumed by `ApiState`/`router.rs` in Task 8.

- [ ] **Step 1: Add the optional dependency**

In `crates/xberg/Cargo.toml`, find the `moka` dependency line (around line 663):

```toml
moka = { version = "0.12", features = ["sync"], optional = true }
```

Add immediately after it:

```toml
xberg-doc-store = { workspace = true, optional = true, features = ["in-memory"] }
```

- [ ] **Step 2: Update the `api` feature and add `doc-store-sqlite`**

Find the `api` feature block (around line 366):

```toml
api = [
    "api-types",
    "tower-service",
    "dep:axum",
    "dep:chrono",
    "dep:moka",
    "dep:tower-http",
    "dep:utoipa",
    "dep:uuid",
    "tokio-runtime",
    # SIGTERM/SIGINT graceful shutdown for the api + mcp-http servers (#1147).
    "tokio?/signal",
    "chunking",
]
```

Add `"dep:xberg-doc-store",` to the list (after `"dep:uuid",`), and add a new feature immediately after the `api` block:

```toml
api = [
    "api-types",
    "tower-service",
    "dep:axum",
    "dep:chrono",
    "dep:moka",
    "dep:tower-http",
    "dep:utoipa",
    "dep:uuid",
    "dep:xberg-doc-store",
    "tokio-runtime",
    # SIGTERM/SIGINT graceful shutdown for the api + mcp-http servers (#1147).
    "tokio?/signal",
    "chunking",
]
# Durable (SQLite) rehydration-map storage, selected at runtime via
# XBERG_REHYDRATION_DB_PATH. Additive to `api` — without it the ephemeral
# in-memory backend (24h TTL) is used.
doc-store-sqlite = ["api", "xberg-doc-store/sqlite"]
```

- [ ] **Step 3: Remove the old module declaration**

In `crates/xberg/src/api/mod.rs`, remove:

```rust
#[cfg(feature = "api")]
pub(crate) mod rehydration_store;
```

- [ ] **Step 4: Delete the superseded file**

```bash
git rm crates/xberg/src/api/rehydration_store.rs
```

- [ ] **Step 5: Verify the crate no longer compiles cleanly (expected — downstream references are updated in Tasks 8–11)**

Run: `cargo check -p xberg --features api 2>&1 | head -40`
Expected: FAILS — `crate::api::rehydration_store` and `ApiState.rehydration_store` no longer resolve. This is expected; Tasks 8–11 fix every call site.

- [ ] **Step 6: Commit**

```bash
git add crates/xberg/Cargo.toml crates/xberg/src/api/mod.rs
git commit -m "feat(doc-store): wire xberg-doc-store into xberg's api feature"
```

---

### Task 8: `ApiState` + router construction

**Files:**
- Modify: `crates/xberg/src/api/types.rs`
- Modify: `crates/xberg/src/api/router.rs`

**Interfaces:**
- Consumes: `xberg_doc_store::{RehydrationStore, rehydration_store_from_env}` (Task 6).
- Produces: `ApiState.rehydration_store: Arc<dyn xberg_doc_store::RehydrationStore>` — consumed by `handlers.rs` in Tasks 9–11.

- [ ] **Step 1: Change the `ApiState` field type**

In `crates/xberg/src/api/types.rs`, replace:

```rust
    /// In-memory store for encrypted rehydration map blobs.
    #[cfg(feature = "api")]
    pub rehydration_store: Arc<super::rehydration_store::RehydrationStore>,
```

with:

```rust
    /// Tenant-scoped store for encrypted rehydration map blobs.
    ///
    /// Backend selected at startup by `xberg_doc_store::rehydration_store_from_env`:
    /// durable SQLite when `XBERG_REHYDRATION_DB_PATH` is set (requires the
    /// `doc-store-sqlite` feature), ephemeral in-memory (24h TTL) otherwise.
    #[cfg(feature = "api")]
    pub rehydration_store: Arc<dyn xberg_doc_store::RehydrationStore>,
```

- [ ] **Step 2: Construct it via the factory in `router.rs`**

In `crates/xberg/src/api/router.rs`, find:

```rust
    let state = ApiState {
        default_config: Arc::new(config),
        extraction_service: Arc::new(std::sync::Mutex::new(extraction_service)),
        #[cfg(feature = "api")]
        job_store: Arc::new(super::jobs::JobStore::new()),
    };
```

Replace with:

```rust
    let state = ApiState {
        default_config: Arc::new(config),
        extraction_service: Arc::new(std::sync::Mutex::new(extraction_service)),
        #[cfg(feature = "api")]
        job_store: Arc::new(super::jobs::JobStore::new()),
        #[cfg(feature = "api")]
        rehydration_store: xberg_doc_store::rehydration_store_from_env()
            .expect("rehydration store must initialize (check XBERG_REHYDRATION_DB_PATH)"),
    };
```

- [ ] **Step 3: Verify compilation (still expected to fail in `handlers.rs` — fixed next task)**

Run: `cargo check -p xberg --features api 2>&1 | head -40`
Expected: `router.rs` and `types.rs` now compile; remaining errors are confined to `handlers.rs` call sites (`.store(...)`, `.get(...)`) and the test module's `make_api_state()`.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg/src/api/types.rs crates/xberg/src/api/router.rs
git commit -m "feat(doc-store): construct ApiState.rehydration_store from xberg-doc-store"
```

---

### Task 9: Update `process_handler`

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`

**Interfaces:**
- Consumes: `xberg_doc_store::TenantCtx` (Task 2), `ApiState.rehydration_store` (Task 8).
- Produces: unchanged `ProcessResponse.rehydration_key: Option<String>` wire shape.

- [ ] **Step 1: Update the call site**

In `crates/xberg/src/api/handlers.rs`, inside `process_handler`, find:

```rust
            let encrypted = crate::text::redaction::encrypt_map(&map, passphrase).map_err(ApiError::from)?;
            (document, Some(state.rehydration_store.store(encrypted)))
```

Replace with:

```rust
            let encrypted = crate::text::redaction::encrypt_map(&map, passphrase).map_err(ApiError::from)?;
            let doc_id = state
                .rehydration_store
                .put_map(&xberg_doc_store::TenantCtx::default_tenant(), encrypted)
                .await
                .map_err(|e| ApiError::internal(crate::error::XbergError::Other(e.to_string())))?;
            (document, Some(doc_id.0))
```

- [ ] **Step 2: Update the two `process_handler` tests that touch `rehydration_store`**

`process_handler_redacts_email_with_mask_strategy` and `process_handler_requires_passphrase_when_rehydrate_is_true` don't call `rehydration_store` directly — they go through the handler, so they need no code change, only recompilation. Leave them as-is; they are fixed by Tasks 8+9 together and re-verified in Task 11.

- [ ] **Step 3: Verify compilation of `process_handler` (rehydrate_handler still pending — Task 10)**

Run: `cargo check -p xberg --features "api redaction-rehydrate" 2>&1 | grep -A3 "rehydrate_handler\|make_api_state"`
Expected: remaining errors confined to `rehydrate_handler` and `make_api_state()` (fixed in Tasks 10–11).

- [ ] **Step 4: Commit**

```bash
git add crates/xberg/src/api/handlers.rs
git commit -m "feat(doc-store): use RehydrationStore trait in process_handler"
```

---

### Task 10: Update `rehydrate_handler`

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`

**Interfaces:**
- Consumes: `xberg_doc_store::{TenantCtx, DocumentId}` (Tasks 2–3), `ApiState.rehydration_store` (Task 8).
- Produces: unchanged 404/403 semantics on `POST /v1/documents/{rehydration_key}/rehydrate`.

- [ ] **Step 1: Update the call site**

In `crates/xberg/src/api/handlers.rs`, inside `rehydrate_handler`, find:

```rust
    let encrypted = state.rehydration_store.get(&rehydration_key).ok_or_else(|| ApiError {
        status: axum::http::StatusCode::NOT_FOUND,
        body: super::types::ErrorResponse {
            error_type: "NotFoundError".to_string(),
            message: format!("Rehydration key '{rehydration_key}' not found or expired"),
            traceback: None,
            status_code: axum::http::StatusCode::NOT_FOUND.as_u16(),
        },
    })?;
```

Replace with:

```rust
    let ctx = xberg_doc_store::TenantCtx::default_tenant();
    let doc_id = xberg_doc_store::DocumentId(rehydration_key.clone());
    let encrypted = state
        .rehydration_store
        .get_map(&ctx, &doc_id)
        .await
        .map_err(|e| ApiError::internal(crate::error::XbergError::Other(e.to_string())))?
        .ok_or_else(|| ApiError {
            status: axum::http::StatusCode::NOT_FOUND,
            body: super::types::ErrorResponse {
                error_type: "NotFoundError".to_string(),
                message: format!("Rehydration key '{rehydration_key}' not found or expired"),
                traceback: None,
                status_code: axum::http::StatusCode::NOT_FOUND.as_u16(),
            },
        })?;
```

- [ ] **Step 2: Verify compilation of the handler bodies**

Run: `cargo check -p xberg --features "api redaction-rehydrate" 2>&1 | head -40`
Expected: remaining errors confined to the `#[cfg(test)]` module (`make_api_state`, and the two tests that call `state.rehydration_store.store(...)` directly) — fixed in Task 11.

- [ ] **Step 3: Commit**

```bash
git add crates/xberg/src/api/handlers.rs
git commit -m "feat(doc-store): use RehydrationStore trait in rehydrate_handler"
```

---

### Task 11: Fix the existing handler tests

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`

**Interfaces:**
- Consumes: `xberg_doc_store::backends::memory::InMemoryRehydrationStore` (Task 4).
- Produces: passing `cargo test -p xberg --features "api redaction-rehydrate"`.

- [ ] **Step 1a: Update `test_router()` (the `#[cfg(test)] mod tests` block near the top of the file, ~line 1335)**

Find:

```rust
    fn test_router() -> Router {
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        let state = ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
            #[cfg(feature = "api")]
            job_store: std::sync::Arc::new(crate::api::jobs::JobStore::new()),
            #[cfg(feature = "api")]
            rehydration_store: std::sync::Arc::new(crate::api::rehydration_store::RehydrationStore::new()),
        };
```

Replace with:

```rust
    fn test_router() -> Router {
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        let state = ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
            #[cfg(feature = "api")]
            job_store: std::sync::Arc::new(crate::api::jobs::JobStore::new()),
            #[cfg(feature = "api")]
            rehydration_store: std::sync::Arc::new(
                xberg_doc_store::backends::memory::InMemoryRehydrationStore::new(),
            ),
        };
```

- [ ] **Step 1b: Update `make_api_state()` (the handler-level helper, ~line 1685 — its enclosing function is already `#[cfg(feature = "api")]`-gated, so its fields carry no per-field `#[cfg]`)**

Find:

```rust
    #[cfg(feature = "api")]
    fn make_api_state() -> ApiState {
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
            job_store: std::sync::Arc::new(crate::api::jobs::JobStore::new()),
            rehydration_store: std::sync::Arc::new(crate::api::rehydration_store::RehydrationStore::new()),
        }
    }
```

Replace with:

```rust
    #[cfg(feature = "api")]
    fn make_api_state() -> ApiState {
        let extraction_service = crate::service::ExtractionServiceBuilder::new().build();
        ApiState {
            default_config: std::sync::Arc::new(crate::ExtractionConfig::default()),
            extraction_service: std::sync::Arc::new(std::sync::Mutex::new(extraction_service)),
            job_store: std::sync::Arc::new(crate::api::jobs::JobStore::new()),
            rehydration_store: std::sync::Arc::new(
                xberg_doc_store::backends::memory::InMemoryRehydrationStore::new(),
            ),
        }
    }
```

- [ ] **Step 2: Update `rehydrate_handler_round_trips_a_stored_map`**

Find:

```rust
        let key = state.rehydration_store.store(encrypted);
        let response = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path(key),
```

Replace with:

```rust
        let doc_id = state
            .rehydration_store
            .put_map(&xberg_doc_store::TenantCtx::default_tenant(), encrypted)
            .await
            .expect("put_map");
        let response = rehydrate_handler(
            axum::extract::State(state),
            axum::extract::Path(doc_id.0),
```

- [ ] **Step 3: Update `rehydrate_handler_rejects_wrong_passphrase`**

Apply the identical replacement shown in Step 2 (same `state.rehydration_store.store(encrypted)` call site pattern) in this second test.

- [ ] **Step 4: Run the full handler test suite**

Run: `cargo test -p xberg --features "api redaction-rehydrate" api::handlers::`
Expected: all tests PASS, including:
- `process_handler_redacts_email_with_mask_strategy`
- `process_handler_requires_passphrase_when_rehydrate_is_true`
- `process_handler_rejects_both_text_and_url`
- `rehydrate_handler_returns_404_for_unknown_key`
- `rehydrate_handler_round_trips_a_stored_map`
- `rehydrate_handler_rejects_wrong_passphrase`

- [ ] **Step 5: Run clippy across the whole feature set touched by this plan**

Run: `cargo clippy -p xberg --features "doc-store-sqlite redaction-rehydrate" --all-targets -- -D warnings`
Expected: zero warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/xberg/src/api/handlers.rs
git commit -m "test(doc-store): update process/rehydrate handler tests for RehydrationStore trait"
```

---

### Task 12: End-to-end durability test

**Files:**
- Create: `crates/xberg/tests/rehydration_durability.rs`

**Interfaces:**
- Consumes: `process_handler`, `rehydrate_handler` (via the public router — `xberg::api::create_router`), `xberg_doc_store::backends::sqlite::SqliteRehydrationStore`.
- Produces: proof that the full `/v1/process` → restart → `/v1/documents/{id}/rehydrate` flow survives a process-equivalent restart, which is the entire point of this plan.

- [ ] **Step 1: Write the failing test**

Create `crates/xberg/tests/rehydration_durability.rs`:

```rust
//! Proves the durable rehydration path end-to-end: a map written through
//! `POST /v1/process` is still rehydratable through `POST
//! /v1/documents/{id}/rehydrate` after the backing store is dropped and
//! reopened against the same file — the scenario the in-memory backend
//! (24h TTL, lost on restart) could never satisfy.

#![cfg(all(feature = "api", feature = "redaction-rehydrate", feature = "doc-store-sqlite"))]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[allow(unsafe_code)]
fn build_router_with_sqlite_store(db_path: &std::path::Path) -> axum::Router {
    // SAFETY: `env::set_var`/`remove_var` are `unsafe` as of the 2024 edition
    // because concurrent env mutation across threads is a data race. This is
    // sound here because each file under `crates/xberg/tests/` compiles to
    // its own test binary/process (cargo's integration-test model), and this
    // file contains exactly one `#[tokio::test]` function, so no other test
    // — in this process or any other — observes or mutates this env var
    // concurrently.
    unsafe {
        std::env::set_var("XBERG_REHYDRATION_DB_PATH", db_path);
    }
    let router = xberg::api::create_router(xberg::ExtractionConfig::default());
    unsafe {
        std::env::remove_var("XBERG_REHYDRATION_DB_PATH");
    }
    router
}

#[tokio::test]
async fn rehydration_map_survives_router_rebuild_against_same_db_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("rehydration.sqlite3");

    // First "process" — build a router backed by the durable store, redact
    // with rehydrate=true, and capture the returned rehydration key.
    let app = build_router_with_sqlite_store(&db_path);
    let process_body = serde_json::json!({
        "text": "Contact Alice at alice@example.com.",
        "operations": {
            "redact": {
                "strategy": "token_replace",
                "rehydrate": true,
                "passphrase": "durability-test-passphrase"
            }
        }
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/process")
                .header("content-type", "application/json")
                .body(Body::from(process_body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("handler responded");
    assert_eq!(response.status(), StatusCode::OK, "expected /v1/process to succeed");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
    let process_json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    let rehydration_key = process_json["rehydration_key"]
        .as_str()
        .expect("rehydration_key must be present when rehydrate=true")
        .to_string();

    // `app` (and the SqliteRehydrationStore it owns) is dropped here —
    // simulating a process restart. A brand-new router is built against the
    // same on-disk database file.
    let app_after_restart = build_router_with_sqlite_store(&db_path);

    let rehydrate_body = serde_json::json!({ "passphrase": "durability-test-passphrase" });
    let response = app_after_restart
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/documents/{rehydration_key}/rehydrate"))
                .header("content-type", "application/json")
                .body(Body::from(rehydrate_body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("handler responded");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "rehydration must succeed after a simulated restart against the same DB file"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
    let rehydrate_json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        rehydrate_json["restored"]["[EMAIL_1]"].as_str(),
        Some("alice@example.com"),
        "restored map must contain the original PII value after reopening the durable store"
    );
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test -p xberg --features "api redaction-rehydrate doc-store-sqlite" --test rehydration_durability`
Expected: `rehydration_map_survives_router_rebuild_against_same_db_file` PASSES.

- [ ] **Step 3: Verify the test is skipped (not failing) without the `doc-store-sqlite` feature**

Run: `cargo test -p xberg --features "api redaction-rehydrate" --test rehydration_durability`
Expected: `0 tests run` (the `#![cfg(...)]` gate excludes the whole file), not a failure.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg/tests/rehydration_durability.rs
git commit -m "test(doc-store): add end-to-end rehydration durability test"
```

---

### Task 13: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: nothing.
- Produces: a documented public-API-relevant change, per the `api-compatibility` rule ("Document all public API changes in CHANGELOG.md").

- [ ] **Step 1: Add an `[Unreleased]` section**

In `CHANGELOG.md`, immediately after the `---` that follows the intro (before `## [1.0.0-rc.1] - 2026-06-26`), insert:

```markdown
## [Unreleased]

### Added

- **Durable rehydration-map storage.** `POST /v1/process` (with
  `operations.redact.rehydrate=true`) and `POST
  /v1/documents/{id}/rehydrate` now persist encrypted PII rehydration maps
  through a new `xberg-doc-store` crate. The default backend is unchanged
  (in-memory, 24h TTL, lost on restart); setting `XBERG_REHYDRATION_DB_PATH`
  and building with the `doc-store-sqlite` feature switches to a durable,
  WAL-mode SQLite backend that survives process restarts. No wire-format
  change to either endpoint.

---
```

- [ ] **Step 2: Verify the file is still well-formed markdown**

Run: `task docs:lint:prose` (if configured) or visually confirm the new section renders correctly relative to the existing `## [1.0.0-rc.1]` heading.

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): note durable rehydration-map storage"
```

---

## Final Verification

After all 13 tasks:

- [ ] Run: `cargo fmt --check`
  Expected: no diff.
- [ ] Run: `cargo clippy -p xberg-doc-store --features sqlite --all-targets -- -D warnings`
  Expected: zero warnings.
- [ ] Run: `cargo clippy -p xberg --features "doc-store-sqlite redaction-rehydrate" --all-targets -- -D warnings`
  Expected: zero warnings.
- [ ] Run: `cargo test -p xberg-doc-store --features sqlite`
  Expected: all tests pass.
- [ ] Run: `cargo test -p xberg --features "api redaction-rehydrate doc-store-sqlite"`
  Expected: all tests pass, including `rehydration_durability::rehydration_map_survives_router_rebuild_against_same_db_file`.
- [ ] Run: `cargo test -p xberg --features "api redaction-rehydrate"` (without `doc-store-sqlite`, proving the default in-memory path still works standalone)
  Expected: all tests pass.
- [ ] Run: `prek run --all-files`
  Expected: all hooks pass (re-stage any files hooks reformat, per `commit-procedure`).
