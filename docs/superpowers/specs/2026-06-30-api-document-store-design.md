# API Document Store — Design

**Date:** 2026-06-30
**Status:** Approved for planning
**Relates to:** `docs/superpowers/specs/2026-06-30-lora-privacy-api-design.md` (resolves its deferred §8 "where are rehydration maps persisted" decision), `docs/superpowers/plans/2026-06-29-xberg-privacy-api.md` Phase 4 (`/v1/search`)

---

## 1. Motivation & Context

The xberg HTTP API (`crates/xberg/src/api/`) was **stateless** until the privacy
pipeline landed. Extraction routes (`/extract`, `/detect`, `/formats`, OpenWebUI
compat) are still pure transforms, and async jobs use an in-memory `JobStore`
with a 5-minute TTL whose own docs note *"Server restarts clear all active
jobs."* — a request cache, not durable persistence.

As of `HEAD` (commits `9d506d9f23`, `680b341ecf`, `f64e31cc24`) the privacy
endpoints shipped **with an ephemeral persistence stopgap** — exactly the
tier this spec is meant to replace with a durable, tenant-scoped one:

- `POST /v1/process` (extract → NER → redact) stores the encrypted rehydration
  map **only when `operations.redact.rehydrate=true`**, into
  `ApiState.rehydration_store`.
- `POST /v1/documents/{id}/rehydrate` looks that map up by the returned key and
  decrypts it with the caller-supplied passphrase.
- `ApiState.rehydration_store`
  ([rehydration_store.rs](../../../crates/xberg/src/api/rehydration_store.rs)) is
  a **`moka` in-memory cache, 24h TTL** — lost on restart, no tenant scoping, no
  erasure, no audit. `POST /v1/search` and any corpus `[embed → store]` step are
  not yet built.

So the crypto, the endpoint shapes, and the `ApiState` injection pattern are
proven; what is missing is durability, tenancy, an ID-keyed document record,
audit, and erasure. The moment the API hands out an ID and lets a later call
rehydrate or search by it, it stops being stateless — an enterprise
document-intelligence platform needs those five properties at the API layer.
This spec defines them, folding the shipped moka store in as the default backend
rather than replacing it wholesale.

### What this design does NOT redo

- **Vector storage / retrieval** — reuse `xberg-rag::VectorStore` verbatim
  (SQLite default, pgvector for Enterprise). No new vector store.
- **PII detection & AES-256-GCM rehydration crypto** — reuse
  `crates/xberg/src/text/redaction/` and the `XPII\x01` map format. This spec
  only decides *where the encrypted map bytes live and how they are addressed*.
- **Extraction** — unchanged; the store sits downstream of it.

---

## 2. Verified Facts (de-risking)

Confirmed by reading source before committing to the design:

1. **The API is genuinely stateless today.** `create_router_*`
   ([router.rs:125](../../../crates/xberg/src/api/router.rs)) builds `ApiState`
   with `default_config`, `extraction_service`, and the in-memory `job_store`
   only. No `xberg-rag`, no DB handle.
2. **`ApiState` is the single injection point.**
   [types.rs:180](../../../crates/xberg/src/api/types.rs) — `ApiState` is
   `Clone + Send + Sync`, held by axum `State`. New store handles are added as
   `Arc<dyn …>` fields here, feature-gated like `job_store`.
3. **`VectorStore` is deliberately single-tenant + object-safe.**
   [store.rs:1-38](../../../crates/xberg-rag/src/store.rs) — *"one instance is
   one trust domain … multi-tenancy is layered on top by the caller."* No
   generic methods, no associated types — safe behind `Arc<dyn VectorStore>`.
4. **`DocumentId` is an opaque backend-assigned string.**
   [types.rs:10-12](../../../crates/xberg-rag/src/types.rs). The same ID type
   keys both the corpus document and its rehydration map — no second ID space.
5. **`DocumentRecord` already carries `entities`, `labels`, `metadata` as
   free-form JSON** ([types.rs:74-103](../../../crates/xberg-rag/src/types.rs)),
   enough to attach redaction/NER provenance without a schema change.

---

## 3. Architecture Overview

Two new object-safe traits sit beside `VectorStore`, all injected through
`ApiState`. The corpus (vectors + document text) stays in `VectorStore`; the new
traits own **ID-keyed sidecar state** the corpus doesn't model: encrypted
rehydration maps, durable jobs, and the audit log.

```text
┌────────────────────────────────────────────────────────────────────┐
│ HTTP API  (crates/xberg/src/api)                                    │
│   POST /v1/process            → extract → ner → pii → redact → store │
│   GET  /v1/documents/{id}                                           │
│   POST /v1/documents/{id}/rehydrate                                 │
│   POST /v1/search             → VectorStore::retrieve               │
│   GET  /v1/jobs/{id}          (durable)                             │
│   GET  /v1/audit              (tenant-scoped)                       │
├────────────────────────────────────────────────────────────────────┤
│ ApiState  (Clone + Send + Sync)                                     │
│   tenant: TenantResolver                                            │
│   corpus: Arc<dyn VectorStore>          (xberg-rag, reused)         │
│   docs:   Arc<dyn DocumentStore>        (NEW — sidecar metadata)    │
│   maps:   Arc<dyn RehydrationStore>     (NEW — encrypted PII maps)  │
│   jobs:   Arc<dyn JobStore>             (durable trait; in-mem impl)│
│   audit:  Arc<dyn AuditSink>            (NEW)                       │
├────────────────────────────────────────────────────────────────────┤
│ Backends (one persistence story, selected by config)               │
│   default     → SQLite (WAL) : corpus via sqlite-vec, sidecars in   │
│                 sibling tables of the same DB file                  │
│   enterprise  → pgvector : tenant-scoped schema, RLS context        │
└────────────────────────────────────────────────────────────────────┘
```

Design rule: **the new traits never duplicate vectors or full text** — those are
`VectorStore`'s job. They store the encrypted-map bytes, job results, and audit
rows the corpus has no place for, all addressed by the same `DocumentId`.

---

## 4. Trait Design

All three new traits are object-safe (no generics, no associated types) so they
compose behind `Arc<dyn …>` exactly like `VectorStore`. Every method takes an
explicit `&TenantCtx` first argument (see §6) — tenancy is in the signatures
from day one, never implicit.

### 4.1 `DocumentStore` — ID-keyed document metadata

```rust
#[async_trait]
pub trait DocumentStore: Send + Sync + 'static {
    /// Persist (or replace) the API-level record for a document already
    /// upserted into the corpus. Idempotent on `(tenant, id)`.
    async fn put(&self, ctx: &TenantCtx, id: &DocumentId, rec: &ApiDocument) -> StoreResult<()>;

    /// Fetch the API-level record, or `None` if absent / not visible to tenant.
    async fn get(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<ApiDocument>>;

    /// Hard-delete the record AND request corpus + map deletion (GDPR Art. 17).
    /// Returns whether anything existed.
    async fn purge(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool>;

    /// List documents for a tenant, newest first, cursor-paginated.
    async fn list(&self, ctx: &TenantCtx, page: &Page) -> StoreResult<Listing<ApiDocumentSummary>>;
}
```

`ApiDocument` is a thin record: `id`, `external_id`, `title`, `mime`,
`source_uri`, `created_at`, `pipeline` provenance (which NER backend/adapter,
which redaction strategy, PII category counts — **counts only, never values**,
per the `pii-pipeline` rule), `has_rehydration_map: bool`, `retention`, and
free-form `metadata`. It does **not** hold full text — that lives in the corpus.

### 4.2 `RehydrationStore` — encrypted PII maps

```rust
#[async_trait]
pub trait RehydrationStore: Send + Sync + 'static {
    /// Store the opaque encrypted map blob (`XPII\x01` framed) for a document.
    /// Overwrites any prior map for `(tenant, id)`.
    async fn put_map(&self, ctx: &TenantCtx, id: &DocumentId, blob: &[u8]) -> StoreResult<()>;

    /// Fetch the encrypted blob, or `None`. Decryption happens in the handler
    /// with the caller-supplied passphrase — the store never sees plaintext or
    /// the key.
    async fn get_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>>;

    /// Delete the map (cascade target of `DocumentStore::purge`).
    async fn delete_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool>;
}
```

Critical invariants (carried from the `pii-pipeline` rules):

- The store persists **ciphertext only**. The scrypt-derived key and passphrase
  never touch the store; decryption is request-scoped in the handler and the key
  is dropped before the response is sent.
- Maps are addressed by `DocumentId`, the same ID the corpus and `DocumentStore`
  use — no separate map-ID namespace, no plaintext PII in any row.
- For the SQLite backend the map column lives in a sibling table of the same DB;
  for Enterprise it is a tenant-scoped table. It is **never** embedded in a
  corpus row.

**Migrating the shipped store.** The concrete
`api::rehydration_store::RehydrationStore` (moka, 24h TTL) already implements this
contract minus tenancy — `store(blob) -> key` ≈ `put_map`, `get(key)` ≈
`get_map`. It becomes the **default `InMemoryRehydrationStore` impl** behind the
trait: rename it, add the ignored `TenantCtx` arg, keep the moka cache verbatim.
The handler changes from calling the struct directly to holding
`Arc<dyn RehydrationStore>`, so swapping in the durable SQLite/pgvector impl is a
one-line `ApiState` change with zero handler edits and no behavior change to the
existing 24h-TTL default.

### 4.3 `JobStore` — durable async jobs

Promote the existing concrete `JobStore` to a trait with the current in-memory
impl as the default and a backend-backed impl for durability:

```rust
#[async_trait]
pub trait JobStore: Send + Sync + 'static {
    async fn create(&self, ctx: &TenantCtx) -> StoreResult<JobId>;
    async fn set_running(&self, ctx: &TenantCtx, id: &JobId) -> StoreResult<()>;
    async fn complete(&self, ctx: &TenantCtx, id: &JobId, result: serde_json::Value) -> StoreResult<()>;
    async fn fail(&self, ctx: &TenantCtx, id: &JobId, error: String) -> StoreResult<()>;
    async fn get(&self, ctx: &TenantCtx, id: &JobId) -> StoreResult<Option<JobStatus>>;
}
```

`JobState`/`JobStatus` are reused unchanged from `api::types`. Default deploy
keeps the in-memory impl (zero behavior change); Enterprise selects the durable
impl so results survive restart and are tenant-isolated. `MAX_ACTIVE_JOBS` /
TTL semantics move into the impl.

### 4.4 `AuditSink` — append-only event log

```rust
#[async_trait]
pub trait AuditSink: Send + Sync + 'static {
    async fn record(&self, ctx: &TenantCtx, event: &AuditEvent) -> StoreResult<()>;
    async fn query(&self, ctx: &TenantCtx, filter: &AuditFilter) -> StoreResult<Vec<AuditEvent>>;
}
```

`AuditEvent` = `{ ts, tenant, actor, action, document_id?, pii_category_counts?,
outcome }`. Logged for every mutating/PII action: `process`, `rehydrate`,
`purge`, `redact`. **Counts only — never PII values** (the `pii-pipeline` and
`monitoring-observability` rules). Append-only; no update/delete in the trait.

---

## 5. API Surface

New routes mounted under `/v1` and feature-gated behind a `doc-store` cargo
feature (additive; the stateless routes stay available when it's off).

| Method & path | Store interaction | Notes |
|---|---|---|
| `POST /v1/process` | `maps.put_map` (already shipped, when `redact.rehydrate=true`) + `docs.put` + `audit.record`; **`corpus.upsert_document` only when request `store=true`** | Extends the shipped handler. The `rehydrate` flag already gates map storage; the new `store: bool` (default `false`) gates the not-yet-built `[embed → corpus]` step so the pure-transform mode survives. Returns `{ document_id, redacted, pii_summary }`. |
| `GET /v1/documents/{id}` | `docs.get` | 404 if absent or not visible to tenant. |
| `GET /v1/documents` | `docs.list` | Cursor-paginated, tenant-scoped. |
| `POST /v1/documents/{id}/rehydrate` | `maps.get_map` → decrypt with body passphrase → reverse tokens | Resolves the privacy spec's deferred decision. 422 if no map / wrong strategy. |
| `DELETE /v1/documents/{id}` | `docs.purge` (cascades corpus + map) + `audit.record` | GDPR Art. 17 erasure. |
| `POST /v1/search` | `corpus.retrieve` | Thin wrapper over `xberg-rag`; the Phase-4 endpoint, now homed. |
| `GET /v1/jobs/{id}` | `jobs.get` | Durable variant of the existing route. |
| `GET /v1/audit` | `audit.query` | Tenant-scoped; admin scope required. |

Request/response types are new and live in `api/types.rs`; they reuse `xberg-rag`
DTOs (`DocumentId`, `RetrieveQuery`, `RetrieveOutput`) and redaction types
(`RedactionStrategy`, `PiiCategory`) rather than redefining them.

---

## 6. Tenancy Model (day-one)

`VectorStore` is single-tenant by design — *"one instance is one trust domain."*
We honor that and put tenancy in the **API layer**, never inside the corpus
backend's signatures:

```rust
pub struct TenantCtx {
    pub tenant: TenantId,   // newtype over String; "default" in single-tenant deploys
    pub actor: ActorId,     // from auth layer; for audit attribution
}
```

- A `TenantResolver` (from the auth/middleware layer) produces a `TenantCtx` per
  request; handlers thread it into every store call.
- **SQLite / default:** single trust domain → `tenant = "default"`; `TenantCtx`
  is still required and audited, so enabling multi-tenancy later is not a
  breaking change.
- **Enterprise / pgvector:** the backend impl sets a row-level-security session
  variable from `ctx.tenant` before delegating (the decorator path the
  `store.rs` doc anticipates), and the corpus is a per-tenant scoped
  `VectorStore` instance or RLS-scoped schema.

Because tenancy lives in the trait *signatures* now (not in a v2 refactor),
shipping single-tenant first costs nothing and the Enterprise path is purely an
impl swap behind the same `Arc<dyn …>`.

---

## 7. Data Model (SQLite default backend)

All sidecar state lives in the **same DB file** as the sqlite-vec corpus, in
sibling tables, so a deploy is one file and one WAL:

```sql
-- api_documents: API-level record (corpus text/vectors stay in xberg-rag tables)
CREATE TABLE api_documents (
  tenant        TEXT NOT NULL,
  id            TEXT NOT NULL,
  external_id   TEXT,
  title         TEXT,
  mime          TEXT,
  source_uri    TEXT,
  created_at    INTEGER NOT NULL,         -- unix seconds
  pipeline_json TEXT NOT NULL,            -- NER backend/adapter, strategy, pii counts
  has_map       INTEGER NOT NULL DEFAULT 0,
  retain_until  INTEGER,                  -- NULL = keep until purged
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (tenant, id)
);

-- rehydration_maps: ciphertext only (XPII\x01 framed), addressed by document id
CREATE TABLE rehydration_maps (
  tenant     TEXT NOT NULL,
  id         TEXT NOT NULL,
  blob       BLOB NOT NULL,               -- AES-256-GCM; no plaintext, no key
  created_at INTEGER NOT NULL,
  PRIMARY KEY (tenant, id)
);

-- jobs: durable async job state
CREATE TABLE jobs (
  tenant     TEXT NOT NULL,
  id         TEXT NOT NULL,
  state      TEXT NOT NULL,               -- pending|running|completed|failed
  result_json TEXT,
  error      TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (tenant, id)
);

-- audit_log: append-only; counts only, never PII values
CREATE TABLE audit_log (
  tenant      TEXT NOT NULL,
  ts          INTEGER NOT NULL,
  actor       TEXT NOT NULL,
  action      TEXT NOT NULL,
  document_id TEXT,
  detail_json TEXT NOT NULL DEFAULT '{}', -- pii_category_counts, outcome
  rowid_seq   INTEGER PRIMARY KEY AUTOINCREMENT
);
```

WAL mode (`PRAGMA journal_mode=WAL`) is already required by the `rag-store` rule.
Retention sweeps delete `api_documents` rows past `retain_until` and cascade to
`rehydration_maps` + corpus. Enterprise maps these tables to a tenant-scoped
pgvector schema with RLS; the trait surface is identical.

---

## 8. Security & GDPR

- **Ciphertext-at-rest only** for PII maps; passphrase/key are request-scoped and
  dropped before response (mirrors `rehydrate_tokens`'s "never cache passphrase
  beyond the call" rule).
- **Right to erasure:** `DELETE /v1/documents/{id}` cascades corpus + map +
  metadata in one transaction; the audit log retains the *event* (counts only),
  not the data.
- **No PII in logs or audit rows** — category counts exclusively
  (`pii-pipeline`, `monitoring-observability`).
- **Tenant isolation** enforced at every store call via `TenantCtx`; Enterprise
  adds RLS so a query can't cross trust domains even on a bug.
- **Fail-open redaction stays fail-open**, but storing an *unredacted* document
  when redaction failed must set `pipeline.redaction=skipped` and audit a WARN,
  so the gap is visible.
- **Retention policy** is per-document (`retain_until`) with a tenant default;
  background sweep is idempotent.

---

## 9. Phasing

1. **Traits + SQLite backend** (`DocumentStore`, `RehydrationStore`, durable
   `JobStore`, `AuditSink`) in a new `xberg-doc-store` module/crate; wire into
   `ApiState` behind the `doc-store` feature. No route changes yet.
2. **Rehydration slice** — `maps.put_map` in `/v1/process` + new
   `/v1/documents/{id}/rehydrate`. Unblocks the privacy API's deferred decision.
3. **Document + search surface** — `GET /v1/documents{,/{id}}`,
   `POST /v1/search` over `xberg-rag`, `DELETE` erasure cascade.
4. **Durable jobs + audit** — swap in durable `JobStore`, mount `/v1/audit`.
5. **Enterprise pgvector backend** — same traits, RLS-scoped impl; no API change.

Each phase is independently shippable; Phase 2 alone closes the concrete blocker.

---

## 10. Out of Scope

- New vector/embedding tech — reuse `xberg-rag` unchanged.
- New PII detection or crypto — reuse `text/redaction/` + `XPII` format.
- Auth/authn mechanism — assumed upstream of `TenantResolver`; this spec only
  consumes the resolved `TenantCtx`.
- Cross-tenant analytics / billing aggregation.
- Object-storage offload for very large extracted text (corpus concern, not
  sidecar).

---

## 11. Resolved Decisions

1. **New crate `xberg-doc-store`** (not a core-crate module). The core crate
   `crates/xberg` has **no dependency on `xberg-rag` today**, and CLAUDE.md marks
   it upstream-tracked / "never modify." A module would pull `rusqlite`/pgvector/
   scrypt deps into the upstream-tracked crate and widen rebase conflict surface.
   A new crate mirrors the proven fork-local, zero-conflict isolation of
   `xberg-rag` / `xberg-rag-node`; the `api` handlers consume
   `Arc<dyn DocumentStore>` and never see the backend.

2. **Passphrase via request body for v1; per-tenant KMS handle for Enterprise.**
   The shipped rehydration already derives the key from a passphrase passed *at
   call time* (`scryptSync(passphrase, salt)` in
   `mcp-server/src/redaction/rehydration.ts`; the `rehydrate_document` MCP tool
   takes `passphrase` as a parameter). A JSON body field is the identical
   semantics — HTTPS only, never in query string/URL, never logged. Enterprise
   swaps the key source to a per-tenant KMS handle behind the same
   `RehydrationStore` trait so the passphrase never transits.

3. **`/v1/process` storing is per-request, default off, AND requires the
   `doc-store` feature.** The shipped handler already proves the pattern:
   encrypted-map storage is gated by `operations.redact.rehydrate` (no map is
   written unless the caller opts in). The new `store: bool` (default `false`)
   applies the same idiom to the not-yet-built `[embed → corpus]` step, and the
   `doc-store` feature must additionally be compiled. Feature-gate alone is too
   coarse — it would force a corpus write on callers wanting the pure transform.
   Default off keeps "do not persist my PII" the safe default even in an
   enterprise build.
