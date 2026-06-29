//! Embedded SQLite vector store backed by `rusqlite` + `sqlite-vec`.
//!
//! # Architecture
//!
//! `rusqlite::Connection` is `Send` but not `Sync`, and its blocking I/O is
//! incompatible with async runtimes. This backend wraps the connection in
//! `Arc<std::sync::Mutex<Connection>>` and routes every operation through
//! `tokio::task::spawn_blocking`. The `Arc` clone is moved into each blocking
//! task; the `Mutex` is locked inside the task (synchronous context — no lock
//! held across `.await`). This satisfies the `Send + Sync + 'static` bound
//! required by `VectorStore`.
//!
//! # Schema
//!
//! Three persistent tables (`collections`, `documents`, `chunks`) are created
//! once at connection open time. Two per-collection virtual tables —
//! `vec_c{rowid}` (sqlite-vec `vec0`) and `fts_c{rowid}` (FTS5) — are
//! created at `ensure_collection` time. The rowid suffix avoids any collection
//! name character escaping.
//!
//! # sqlite-vec extension
//!
//! sqlite-vec is registered globally once via `sqlite3_auto_extension` so
//! every subsequently opened connection has access to the `vec0` virtual-table
//! module and `vec_*` helper functions.

use crate::capability::Capabilities;
use crate::error::{RagError, RagResult};
use crate::filter::{Filter, FilterField, FilterNamespace};
use crate::query::{RetrieveMode, RetrieveOutput, RetrieveQuery};
use crate::store::VectorStore;
use crate::types::{
    ChunkId, ChunkRecord, CollectionSpec, CollectionStats, DistanceMetric, DocumentId, DocumentRecord, DocumentSummary,
    IndexMethod, PrimaryScore, RetrievedChunk,
};
use async_trait::async_trait;
use rusqlite::{Connection, params, params_from_iter};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::instrument;

// ── sqlite-vec registration ──────────────────────────────────────────────────

/// Register the sqlite-vec extension with SQLite's auto-extension mechanism.
///
/// Called once before the first `Connection::open*`. Every subsequent connection
/// opened in this process will have the `vec0` virtual-table module and
/// `vec_*` scalar functions available automatically.
#[allow(unsafe_code)]
fn register_sqlite_vec_once() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // SAFETY: `sqlite3_vec_init` is the canonical SQLite extension entry point
        // from the statically-linked sqlite-vec library. Its actual C signature is
        //
        //     int sqlite3_vec_init(sqlite3*, char**, const sqlite3_api_routines*)
        //
        // which matches the auto-extension callback contract. The sqlite-vec Rust
        // crate declares the symbol as `fn()` (no args, no return) for linker
        // purposes only; we recover the correct function pointer type by casting
        // through a raw `*const ()`, identical to the pattern in sqlite-vec's own
        // test suite. SQLite's runtime then calls the function with the three
        // correct arguments. `Once` guarantees exactly one registration per
        // process, preventing duplicate-registration errors.
        unsafe {
            // The target type is the auto-extension callback:
            //   unsafe extern "C" fn(*mut sqlite3, *mut *mut c_char, *const sqlite3_api_routines) -> c_int
            type AutoExtFn = unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *mut std::ffi::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> std::ffi::c_int;
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<*const (), AutoExtFn>(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

// ── Error helpers ────────────────────────────────────────────────────────────

fn be(e: rusqlite::Error) -> RagError {
    RagError::Backend(Box::new(e))
}

fn io_be(msg: &str) -> RagError {
    RagError::Backend(Box::new(std::io::Error::other(msg.to_string())))
}

// ── Encoding / decoding ──────────────────────────────────────────────────────

/// Encode a `Vec<f32>` as little-endian bytes for sqlite-vec MATCH parameters.
fn encode_vec(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Map `DistanceMetric` to the sqlite-vec column option string.
///
/// sqlite-vec supports `l2`, `l1`, and `cosine`. `InnerProduct` is not natively
/// supported; we fall back to `cosine`, which is equivalent for unit-normalised
/// vectors and is the typical use case for inner-product similarity.
fn metric_str(m: DistanceMetric) -> &'static str {
    match m {
        DistanceMetric::Cosine | DistanceMetric::InnerProduct => "cosine",
        DistanceMetric::L2 => "l2",
    }
}

fn parse_metric(s: &str) -> DistanceMetric {
    match s {
        "l2" => DistanceMetric::L2,
        _ => DistanceMetric::Cosine,
    }
}

fn parse_index_method(s: &str) -> IndexMethod {
    match s {
        "hnsw" => IndexMethod::Hnsw,
        _ => IndexMethod::Flat,
    }
}

fn index_method_str(m: IndexMethod) -> &'static str {
    match m {
        IndexMethod::Hnsw => "hnsw",
        IndexMethod::Flat | IndexMethod::Diskann => "flat",
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Per-collection virtual-table name helpers ────────────────────────────────

fn vec_table(rowid: i64) -> String {
    format!("vec_c{rowid}")
}

fn fts_table(rowid: i64) -> String {
    format!("fts_c{rowid}")
}

// ── Schema setup ─────────────────────────────────────────────────────────────

const BASE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS collections (
    name           TEXT PRIMARY KEY,
    embedding_dim  INTEGER NOT NULL,
    distance_metric TEXT NOT NULL DEFAULT 'cosine',
    index_method   TEXT NOT NULL DEFAULT 'flat'
) STRICT;

CREATE TABLE IF NOT EXISTS documents (
    id          TEXT PRIMARY KEY,
    collection  TEXT NOT NULL,
    external_id TEXT,
    title       TEXT,
    mime        TEXT,
    source_uri  TEXT,
    full_text   TEXT NOT NULL DEFAULT '',
    keywords    TEXT NOT NULL DEFAULT '[]',
    entities    TEXT NOT NULL DEFAULT 'null',
    labels      TEXT NOT NULL DEFAULT 'null',
    metadata    TEXT NOT NULL DEFAULT 'null',
    ingested_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_docs_ext
    ON documents(collection, external_id)
    WHERE external_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_docs_coll ON documents(collection);

CREATE TABLE IF NOT EXISTS chunks (
    id             TEXT PRIMARY KEY,
    document_id    TEXT NOT NULL,
    collection     TEXT NOT NULL,
    ordinal        INTEGER NOT NULL,
    external_id    TEXT,
    content        TEXT NOT NULL,
    embedding      BLOB NOT NULL,
    chunk_metadata TEXT NOT NULL DEFAULT 'null'
) STRICT;

CREATE INDEX IF NOT EXISTS idx_chunks_doc  ON chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_chunks_coll ON chunks(collection);
";

fn setup_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(BASE_SCHEMA)
}

fn setup_pragmas(conn: &Connection, in_memory: bool) -> rusqlite::Result<()> {
    // WAL mode requires a real file; in-memory databases only support DELETE mode.
    if in_memory {
        conn.execute_batch("PRAGMA foreign_keys = ON;")
    } else {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous   = NORMAL;
             PRAGMA foreign_keys  = ON;",
        )
    }
}

// ── Collection row helper ────────────────────────────────────────────────────

struct CollectionRow {
    rowid: i64,
    spec: CollectionSpec,
}

fn get_collection_row(conn: &Connection, name: &str) -> rusqlite::Result<Option<CollectionRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT rowid, name, embedding_dim, distance_metric, index_method
         FROM collections WHERE name = ?1",
    )?;
    match stmt.query_row(params![name], |row| {
        Ok(CollectionRow {
            rowid: row.get(0)?,
            spec: CollectionSpec {
                name: row.get(1)?,
                embedding_dim: row.get::<_, u32>(2)?,
                distance_metric: parse_metric(&row.get::<_, String>(3)?),
                index_method: parse_index_method(&row.get::<_, String>(4)?),
            },
        })
    }) {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ── Row structs for internal retrieval ──────────────────────────────────────

struct ChunkRow {
    id: String,
    document_id: String,
    ordinal: u32,
    external_id: Option<String>,
    content: String,
    chunk_metadata: serde_json::Value,
}

fn load_chunks_by_ids(conn: &Connection, ids: &[String]) -> rusqlite::Result<Vec<ChunkRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", ids.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, document_id, ordinal, external_id, content, chunk_metadata
         FROM chunks WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_map(params_from_iter(ids.iter()), |row| {
        let meta_str: String = row.get(5)?;
        Ok(ChunkRow {
            id: row.get(0)?,
            document_id: row.get(1)?,
            ordinal: row.get::<_, u32>(2)?,
            external_id: row.get(3)?,
            content: row.get(4)?,
            chunk_metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Null),
        })
    })?
    .collect()
}

fn load_document_record(conn: &Connection, document_id: &str) -> rusqlite::Result<Option<(DocumentRecord, i64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT external_id, title, mime, source_uri, full_text,
                keywords, entities, labels, metadata, ingested_at
         FROM documents WHERE id = ?1",
    )?;
    match stmt.query_row(params![document_id], |row| {
        let kw_str: String = row.get(5)?;
        let ent_str: String = row.get(6)?;
        let lbl_str: String = row.get(7)?;
        let meta_str: String = row.get(8)?;
        let ingested_at: i64 = row.get(9)?;
        Ok((
            DocumentRecord {
                external_id: row.get(0)?,
                title: row.get(1)?,
                mime: row.get(2)?,
                source_uri: row.get(3)?,
                full_text: row.get(4)?,
                keywords: serde_json::from_str(&kw_str).unwrap_or_default(),
                entities: serde_json::from_str(&ent_str).unwrap_or(serde_json::Value::Null),
                labels: serde_json::from_str(&lbl_str).unwrap_or(serde_json::Value::Null),
                metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Null),
            },
            ingested_at,
        ))
    }) {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

fn doc_to_summary(id: &DocumentId, rec: &DocumentRecord, ingested_at: i64) -> DocumentSummary {
    DocumentSummary {
        id: id.clone(),
        external_id: rec.external_id.clone(),
        title: rec.title.clone(),
        mime: rec.mime.clone(),
        keywords: rec.keywords.clone(),
        labels: rec.labels.clone(),
        entities: rec.entities.clone(),
        metadata: rec.metadata.clone(),
        ingested_at: Some(ingested_at).filter(|&t| t > 0),
    }
}

// ── In-process filter evaluation ─────────────────────────────────────────────
//
// All filter predicates are evaluated in Rust after SQL retrieval. This avoids
// translating the Filter IR into SQL (which would require careful quoting of
// arbitrary field paths) and is safe for v1 collection sizes.

fn resolve_field_value(field: &FilterField, doc: &DocumentRecord, chunk: &ChunkRecord) -> Option<serde_json::Value> {
    let parsed = field.parse().ok()?;
    match parsed.namespace {
        FilterNamespace::Doc => match parsed.path.as_str() {
            "full_text" => Some(serde_json::Value::String(doc.full_text.clone())),
            "title" => doc.title.clone().map(serde_json::Value::String),
            "mime" => doc.mime.clone().map(serde_json::Value::String),
            "external_id" => doc.external_id.clone().map(serde_json::Value::String),
            "source_uri" => doc.source_uri.clone().map(serde_json::Value::String),
            "keywords" => serde_json::to_value(&doc.keywords).ok(),
            "labels" => Some(doc.labels.clone()),
            "entities" => Some(doc.entities.clone()),
            // ingested_at is not in DocumentRecord; callers can filter by it
            // if the backend stored it — for Rust evaluation we can't see it here
            "ingested_at" => None,
            path if path.starts_with("metadata.") => json_pointer(&doc.metadata, &path["metadata.".len()..]),
            _ => None,
        },
        FilterNamespace::Chunk => match parsed.path.as_str() {
            "content" => Some(serde_json::Value::String(chunk.content.clone())),
            "ordinal" => Some(serde_json::Value::from(chunk.ordinal)),
            "external_id" => chunk.external_id.clone().map(serde_json::Value::String),
            path if path.starts_with("chunk_metadata.") => {
                json_pointer(&chunk.chunk_metadata, &path["chunk_metadata.".len()..])
            }
            _ => None,
        },
    }
}

fn json_pointer(value: &serde_json::Value, dotted: &str) -> Option<serde_json::Value> {
    let mut cur = value;
    for seg in dotted.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur.clone())
}

fn json_cmp(a: &serde_json::Value, b: &serde_json::Value) -> Option<std::cmp::Ordering> {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x.partial_cmp(&y),
        _ => match (a.as_str(), b.as_str()) {
            (Some(x), Some(y)) => Some(x.cmp(y)),
            _ => None,
        },
    }
}

fn eval_filter(filter: &Filter, doc: &DocumentRecord, chunk: &ChunkRecord) -> bool {
    match filter {
        Filter::Eq { field, value } => resolve_field_value(field, doc, chunk).as_ref() == Some(value),
        Filter::In { field, values } => {
            resolve_field_value(field, doc, chunk).is_some_and(|v| values.iter().any(|c| c == &v))
        }
        Filter::ArrayContains { field, value } => resolve_field_value(field, doc, chunk)
            .and_then(|v| v.as_array().cloned())
            .is_some_and(|arr| arr.iter().any(|item| item == value)),
        Filter::Range {
            field,
            gte,
            gt,
            lte,
            lt,
        } => {
            let Some(v) = resolve_field_value(field, doc, chunk) else {
                return false;
            };
            use std::cmp::Ordering;
            let pass = |bound: &Option<serde_json::Value>, want: &[Ordering]| {
                bound
                    .as_ref()
                    .is_none_or(|b| json_cmp(&v, b).is_some_and(|ord| want.contains(&ord)))
            };
            pass(gte, &[Ordering::Greater, Ordering::Equal])
                && pass(gt, &[Ordering::Greater])
                && pass(lte, &[Ordering::Less, Ordering::Equal])
                && pass(lt, &[Ordering::Less])
        }
        Filter::TextMatch { field, query } => resolve_field_value(field, doc, chunk)
            .and_then(|v| v.as_str().map(str::to_lowercase))
            .is_some_and(|s| s.contains(&query.to_lowercase())),
        Filter::And { filters } => filters.iter().all(|f| eval_filter(f, doc, chunk)),
        Filter::Or { filters } => filters.iter().any(|f| eval_filter(f, doc, chunk)),
        Filter::Not { filter } => !eval_filter(filter, doc, chunk),
    }
}

// ── Transaction helper ────────────────────────────────────────────────────────

/// Run `f` inside a `BEGIN IMMEDIATE … COMMIT` transaction.
///
/// On error, rolls back and returns the error. Uses `&Connection` (not
/// `&mut Connection`) so it can be called inside the read-locked `with_conn`
/// closure without requiring exclusive access.
fn with_tx<F, T>(conn: &Connection, f: F) -> RagResult<T>
where
    F: FnOnce() -> RagResult<T>,
{
    conn.execute_batch("BEGIN IMMEDIATE;").map_err(be)?;
    match f() {
        Ok(v) => {
            conn.execute_batch("COMMIT;").map_err(be)?;
            Ok(v)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

// ── The store struct ──────────────────────────────────────────────────────────

/// An embedded vector store backed by SQLite (`rusqlite`) + the `sqlite-vec`
/// vector-search extension.
///
/// Supports vector KNN (via `vec0`), full-text BM25 (via FTS5), and hybrid
/// retrieval with reciprocal rank fusion. All filtering is evaluated in Rust
/// after SQL retrieval.
///
/// # Thread safety
///
/// The underlying `rusqlite::Connection` is wrapped in
/// `Arc<std::sync::Mutex<Connection>>`. Every trait method routes through
/// `tokio::task::spawn_blocking`, locking the mutex inside the blocking task so
/// no lock is ever held across an `.await` point.
pub struct SqliteVectorStore {
    name: String,
    conn: Arc<Mutex<Connection>>,
}

impl SqliteVectorStore {
    fn open_conn(path: Option<&str>) -> RagResult<Connection> {
        let in_memory = path.is_none();
        let conn = match path {
            None => Connection::open_in_memory().map_err(be)?,
            Some(p) => Connection::open(p).map_err(be)?,
        };
        setup_pragmas(&conn, in_memory).map_err(be)?;
        setup_schema(&conn).map_err(be)?;
        Ok(conn)
    }

    /// Open an in-memory store (contents are lost when dropped).
    pub async fn open_in_memory(name: impl Into<String> + Send + 'static) -> RagResult<Self> {
        register_sqlite_vec_once();
        tokio::task::spawn_blocking(move || {
            let conn = Self::open_conn(None)?;
            Ok(Self {
                name: name.into(),
                conn: Arc::new(Mutex::new(conn)),
            })
        })
        .await
        .map_err(|e| io_be(&e.to_string()))?
    }

    /// Open a file-backed store at `path` (created if absent).
    pub async fn open(
        name: impl Into<String> + Send + 'static,
        path: impl Into<String> + Send + 'static,
    ) -> RagResult<Self> {
        register_sqlite_vec_once();
        let path = path.into();
        tokio::task::spawn_blocking(move || {
            let conn = Self::open_conn(Some(&path))?;
            Ok(Self {
                name: name.into(),
                conn: Arc::new(Mutex::new(conn)),
            })
        })
        .await
        .map_err(|e| io_be(&e.to_string()))?
    }

    async fn with_conn<F, T>(&self, f: F) -> RagResult<T>
    where
        F: FnOnce(&Connection) -> RagResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().map_err(|_| io_be("SQLite connection mutex poisoned"))?;
            f(&guard)
        })
        .await
        .map_err(|e| io_be(&e.to_string()))?
    }
}

// ── VectorStore implementation ────────────────────────────────────────────────

#[async_trait]
impl VectorStore for SqliteVectorStore {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            full_text: true,
            hybrid: true,
            filtering: true,
            // sqlite-vec vec0 performs exact brute-force scan (Flat).
            // HNSW is not implemented by sqlite-vec 0.1.x.
            index_methods: vec![IndexMethod::Flat],
        }
    }

    #[instrument(skip(self, spec), fields(store = %self.name, collection = %spec.name))]
    async fn ensure_collection(&self, spec: &CollectionSpec) -> RagResult<()> {
        let spec = spec.clone();
        self.with_conn(move |conn| {
            let existing = get_collection_row(conn, &spec.name).map_err(be)?;
            match existing {
                Some(row) if row.spec.embedding_dim != spec.embedding_dim => {
                    Err(RagError::CollectionAlreadyExists(spec.name))
                }
                Some(_) => Ok(()), // Compatible; idempotent.
                None => {
                    with_tx(conn, || {
                        let dim = spec.embedding_dim;
                        let m = metric_str(spec.distance_metric);
                        let idx = index_method_str(spec.index_method);
                        conn.execute(
                            "INSERT INTO collections (name, embedding_dim, distance_metric, index_method)
                             VALUES (?1, ?2, ?3, ?4)",
                            params![spec.name, dim, m, idx],
                        )
                        .map_err(be)?;
                        let rowid = conn.last_insert_rowid();
                        let vt = vec_table(rowid);
                        let ft = fts_table(rowid);
                        // DDL inside a transaction is allowed in SQLite.
                        conn.execute_batch(&format!(
                            "CREATE VIRTUAL TABLE IF NOT EXISTS {vt} USING vec0(
                                 chunk_id TEXT PRIMARY KEY,
                                 embedding float[{dim}] distance_metric={m}
                             );
                             CREATE VIRTUAL TABLE IF NOT EXISTS {ft} USING fts5(
                                 chunk_id UNINDEXED,
                                 content
                             );"
                        ))
                        .map_err(be)?;
                        Ok(())
                    })
                }
            }
        })
        .await
    }

    #[instrument(skip(self), fields(store = %self.name, collection = %collection))]
    async fn drop_collection(&self, collection: &str) -> RagResult<()> {
        let collection = collection.to_string();
        self.with_conn(move |conn| {
            let row = get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;

            with_tx(conn, || {
                let vt = vec_table(row.rowid);
                let ft = fts_table(row.rowid);
                conn.execute_batch(&format!(
                    "DROP TABLE IF EXISTS {vt};
                     DROP TABLE IF EXISTS {ft};"
                ))
                .map_err(be)?;
                conn.execute("DELETE FROM chunks    WHERE collection = ?1", params![collection])
                    .map_err(be)?;
                conn.execute("DELETE FROM documents WHERE collection = ?1", params![collection])
                    .map_err(be)?;
                conn.execute("DELETE FROM collections WHERE name = ?1", params![collection])
                    .map_err(be)?;
                Ok(())
            })
        })
        .await
    }

    #[instrument(skip(self), fields(store = %self.name, collection = %collection))]
    async fn get_collection(&self, collection: &str) -> RagResult<Option<CollectionSpec>> {
        let collection = collection.to_string();
        self.with_conn(move |conn| Ok(get_collection_row(conn, &collection).map_err(be)?.map(|r| r.spec)))
            .await
    }

    #[instrument(skip(self, document, chunks), fields(store = %self.name, collection = %collection))]
    async fn upsert_document(
        &self,
        collection: &str,
        document: &DocumentRecord,
        chunks: &[ChunkRecord],
    ) -> RagResult<DocumentId> {
        let collection = collection.to_string();
        let document = document.clone();
        let chunks = chunks.to_vec();
        let store_name = self.name.clone();

        self.with_conn(move |conn| {
            let row = get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;
            let dim = row.spec.embedding_dim;

            // Dimension validation before any writes.
            for chunk in &chunks {
                if chunk.embedding.len() as u32 != dim {
                    return Err(RagError::EmbeddingDimMismatch {
                        expected: dim,
                        got: chunk.embedding.len() as u32,
                    });
                }
            }

            let vt = vec_table(row.rowid);
            let ft = fts_table(row.rowid);

            with_tx(conn, || {
                // Resolve identity: reuse existing id for known external_id.
                let doc_id: DocumentId = match document.external_id.as_deref() {
                    Some(ext) => {
                        let mut stmt = conn
                            .prepare_cached("SELECT id FROM documents WHERE collection = ?1 AND external_id = ?2")
                            .map_err(be)?;
                        match stmt.query_row(params![collection, ext], |r| r.get::<_, String>(0)) {
                            Ok(id) => {
                                // Replace: delete old chunks from all tables.
                                delete_chunks_for_doc(conn, &id, &vt, &ft)?;
                                DocumentId(id)
                            }
                            Err(rusqlite::Error::QueryReturnedNoRows) => {
                                insert_new_document(conn, &collection, &document, &store_name)?
                            }
                            Err(e) => return Err(be(e)),
                        }
                    }
                    None => insert_new_document(conn, &collection, &document, &store_name)?,
                };

                // Update document row if it already existed (upsert metadata).
                conn.execute(
                    "UPDATE documents SET
                         external_id = ?2, title = ?3, mime = ?4, source_uri = ?5,
                         full_text = ?6, keywords = ?7, entities = ?8, labels = ?9,
                         metadata = ?10, ingested_at = ?11
                     WHERE id = ?1",
                    params![
                        doc_id.0,
                        document.external_id,
                        document.title,
                        document.mime,
                        document.source_uri,
                        document.full_text,
                        serde_json::to_string(&document.keywords).unwrap_or_default(),
                        serde_json::to_string(&document.entities).unwrap_or_default(),
                        serde_json::to_string(&document.labels).unwrap_or_default(),
                        serde_json::to_string(&document.metadata).unwrap_or_default(),
                        now_unix(),
                    ],
                )
                .map_err(be)?;

                // Insert chunks.
                for chunk in &chunks {
                    let chunk_id = ChunkId(format!("{}:{}", doc_id.0, chunk.ordinal));
                    let blob = encode_vec(&chunk.embedding);
                    conn.execute(
                        "INSERT INTO chunks
                             (id, document_id, collection, ordinal, external_id, content, embedding, chunk_metadata)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            chunk_id.0,
                            doc_id.0,
                            collection,
                            chunk.ordinal,
                            chunk.external_id,
                            chunk.content,
                            blob,
                            serde_json::to_string(&chunk.chunk_metadata).unwrap_or_default(),
                        ],
                    )
                    .map_err(be)?;

                    // Insert into vec0 virtual table.
                    conn.execute(
                        &format!("INSERT INTO {vt} (chunk_id, embedding) VALUES (?1, ?2)"),
                        params![chunk_id.0, blob],
                    )
                    .map_err(be)?;

                    // Insert into FTS5 virtual table.
                    conn.execute(
                        &format!("INSERT INTO {ft} (chunk_id, content) VALUES (?1, ?2)"),
                        params![chunk_id.0, chunk.content],
                    )
                    .map_err(be)?;
                }

                Ok(doc_id)
            })
        })
        .await
    }

    #[instrument(skip(self, ids), fields(store = %self.name, collection = %collection))]
    async fn delete_documents(&self, collection: &str, ids: &[DocumentId]) -> RagResult<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let collection = collection.to_string();
        let ids = ids.to_vec();

        self.with_conn(move |conn| {
            let row = get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;
            let vt = vec_table(row.rowid);
            let ft = fts_table(row.rowid);

            with_tx(conn, || {
                let mut removed = 0u64;
                for id in &ids {
                    let n = delete_chunks_for_doc(conn, &id.0, &vt, &ft)?;
                    let del = conn
                        .execute("DELETE FROM documents WHERE id = ?1", params![id.0])
                        .map_err(be)?;
                    if del > 0 || n > 0 {
                        removed += 1;
                    }
                }
                Ok(removed)
            })
        })
        .await
    }

    #[instrument(skip(self, filter), fields(store = %self.name, collection = %collection))]
    async fn delete_by_filter(&self, collection: &str, filter: &Filter) -> RagResult<u64> {
        filter.validate()?;
        let collection = collection.to_string();
        let filter = filter.clone();

        self.with_conn(move |conn| {
            let row = get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;
            let vt = vec_table(row.rowid);
            let ft = fts_table(row.rowid);

            // Load all (doc_id, DocumentRecord) pairs in the collection.
            let doc_ids: Vec<String> = {
                let mut stmt = conn
                    .prepare_cached("SELECT id FROM documents WHERE collection = ?1")
                    .map_err(be)?;
                stmt.query_map(params![collection], |r| r.get(0))
                    .map_err(be)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(be)?
            };

            let mut to_delete: Vec<String> = Vec::new();
            for doc_id in &doc_ids {
                let Some((doc_rec, _)) = load_document_record(conn, doc_id).map_err(be)? else {
                    continue;
                };
                // Load chunks for this document.
                let chunk_ids: Vec<String> = {
                    let mut stmt = conn
                        .prepare_cached(
                            "SELECT id, ordinal, external_id, content, embedding, chunk_metadata
                             FROM chunks WHERE document_id = ?1",
                        )
                        .map_err(be)?;
                    stmt.query_map(params![doc_id], |r| r.get::<_, String>(0))
                        .map_err(be)?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(be)?
                };
                // A document matches if any chunk satisfies the filter.
                let matches = chunk_ids.iter().any(|cid| {
                    // Load full chunk for filter evaluation.
                    conn.query_row(
                        "SELECT ordinal, external_id, content, embedding, chunk_metadata
                         FROM chunks WHERE id = ?1",
                        params![cid],
                        |r| {
                            let meta_str: String = r.get(4)?;
                            Ok(ChunkRecord {
                                external_id: r.get(1)?,
                                ordinal: r.get::<_, u32>(0)?,
                                content: r.get(2)?,
                                embedding: Vec::new(), // not needed for filter
                                chunk_metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Null),
                            })
                        },
                    )
                    .map(|chunk| eval_filter(&filter, &doc_rec, &chunk))
                    .unwrap_or(false)
                });
                if matches {
                    to_delete.push(doc_id.clone());
                }
            }

            if to_delete.is_empty() {
                return Ok(0);
            }

            with_tx(conn, || {
                let mut removed = 0u64;
                for doc_id in &to_delete {
                    delete_chunks_for_doc(conn, doc_id, &vt, &ft)?;
                    let n = conn
                        .execute("DELETE FROM documents WHERE id = ?1", params![doc_id])
                        .map_err(be)?;
                    removed += n as u64;
                }
                Ok(removed)
            })
        })
        .await
    }

    #[instrument(skip(self, query), fields(store = %self.name, collection = %collection, mode = ?query.mode))]
    async fn retrieve(&self, collection: &str, query: &RetrieveQuery) -> RagResult<RetrieveOutput> {
        let collection = collection.to_string();
        let query = query.clone();
        let backend_name = self.name.clone();

        self.with_conn(move |conn| {
            let row = get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;
            let spec = &row.spec;

            query.validate(spec)?;

            let vt = vec_table(row.rowid);
            let ft = fts_table(row.rowid);
            let candidate_k = query.top_k as i64 * query.candidate_multiplier.unwrap_or(4).max(1) as i64;

            let t0 = std::time::Instant::now();
            let mode = query.mode;

            let mut chunks: Vec<RetrievedChunk> = match mode {
                RetrieveMode::Vector => {
                    let qv = query.query_vector.as_ref().ok_or_else(|| {
                        RagError::InvalidQuery("SQLite backend cannot embed text; supply query_vector".to_string())
                    })?;
                    retrieve_vector(conn, &vt, qv, candidate_k, &query)?
                }
                RetrieveMode::FullText => {
                    let qt = query
                        .query_text
                        .as_ref()
                        .ok_or_else(|| RagError::InvalidQuery("full_text mode requires query_text".to_string()))?;
                    retrieve_fts(conn, &ft, qt, candidate_k, &query)?
                }
                RetrieveMode::Hybrid => {
                    let qt = query
                        .query_text
                        .as_ref()
                        .ok_or_else(|| RagError::InvalidQuery("hybrid mode requires query_text".to_string()))?;
                    let qv = query.query_vector.as_deref();
                    retrieve_hybrid(conn, &vt, &ft, qt, qv, candidate_k, &query)?
                }
                RetrieveMode::Graph => {
                    return Err(RagError::UnsupportedMode {
                        backend: backend_name.clone(),
                        mode: mode.as_str().to_string(),
                    })
                }
            };

            let primary_latency_ms = t0.elapsed().as_millis() as u64;

            // Apply Rust-side filter.
            if let Some(ref filter) = query.filter {
                chunks = filter_chunks(conn, chunks, filter)?;
            }

            // Attach document summaries if requested.
            if query.include_document {
                for chunk in &mut chunks {
                    if let Some((rec, iat)) = load_document_record(conn, &chunk.document_id.0).map_err(be)? {
                        chunk.document = Some(doc_to_summary(&chunk.document_id, &rec, iat));
                    }
                }
            }

            // Deduplicate per document (keep best score per document).
            if query.group_by_document {
                let mut seen: std::collections::HashMap<DocumentId, ()> = std::collections::HashMap::new();
                chunks.retain(|c| seen.insert(c.document_id.clone(), ()).is_none());
            }

            chunks.truncate(query.top_k as usize);

            Ok(RetrieveOutput {
                mode,
                chunks,
                primary_latency_ms,
            })
        })
        .await
    }

    #[instrument(skip(self), fields(store = %self.name, collection = %collection))]
    async fn collection_stats(&self, collection: &str) -> RagResult<CollectionStats> {
        let collection = collection.to_string();
        self.with_conn(move |conn| {
            get_collection_row(conn, &collection)
                .map_err(be)?
                .ok_or_else(|| RagError::CollectionNotFound(collection.clone()))?;

            let documents: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM documents WHERE collection = ?1",
                    params![collection],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(be)? as u64;

            let chunks: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM chunks WHERE collection = ?1",
                    params![collection],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(be)? as u64;

            let last_ingested_at: Option<i64> = conn
                .query_row(
                    "SELECT MAX(ingested_at) FROM documents WHERE collection = ?1",
                    params![collection],
                    |r| r.get(0),
                )
                .map_err(be)?;

            Ok(CollectionStats {
                documents,
                chunks,
                last_ingested_at: last_ingested_at.filter(|&t| t > 0),
            })
        })
        .await
    }
}

// ── Private helper functions ─────────────────────────────────────────────────

/// Insert a new document row with a rowid-derived id and return the id.
fn insert_new_document(
    conn: &Connection,
    collection: &str,
    document: &DocumentRecord,
    store_name: &str,
) -> RagResult<DocumentId> {
    // Insert a placeholder to obtain a stable rowid.
    conn.execute(
        "INSERT INTO documents
             (id, collection, external_id, title, mime, source_uri,
              full_text, keywords, entities, labels, metadata, ingested_at)
         VALUES ('_tmp', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            collection,
            document.external_id,
            document.title,
            document.mime,
            document.source_uri,
            document.full_text,
            serde_json::to_string(&document.keywords).unwrap_or_default(),
            serde_json::to_string(&document.entities).unwrap_or_default(),
            serde_json::to_string(&document.labels).unwrap_or_default(),
            serde_json::to_string(&document.metadata).unwrap_or_default(),
            now_unix(),
        ],
    )
    .map_err(be)?;
    let rowid = conn.last_insert_rowid();
    let doc_id = DocumentId(format!("{store_name}-doc-{rowid}"));
    conn.execute(
        "UPDATE documents SET id = ?1 WHERE rowid = ?2",
        params![doc_id.0, rowid],
    )
    .map_err(be)?;
    Ok(doc_id)
}

/// Delete all chunks for `doc_id` from `chunks`, `vec0`, and `fts5` tables.
/// Returns the number of rows removed from `chunks`.
fn delete_chunks_for_doc(conn: &Connection, doc_id: &str, vec_tbl: &str, fts_tbl: &str) -> RagResult<usize> {
    // Collect chunk ids first.
    let chunk_ids: Vec<String> = {
        let mut stmt = conn
            .prepare_cached("SELECT id FROM chunks WHERE document_id = ?1")
            .map_err(be)?;
        stmt.query_map(params![doc_id], |r| r.get(0))
            .map_err(be)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(be)?
    };
    for cid in &chunk_ids {
        conn.execute(&format!("DELETE FROM {vec_tbl} WHERE chunk_id = ?1"), params![cid])
            .map_err(be)?;
        conn.execute(&format!("DELETE FROM {fts_tbl} WHERE chunk_id = ?1"), params![cid])
            .map_err(be)?;
    }
    let n = conn
        .execute("DELETE FROM chunks WHERE document_id = ?1", params![doc_id])
        .map_err(be)?;
    Ok(n)
}

/// Apply a filter to a list of `RetrievedChunk`s, loading the document and
/// chunk records from the database as needed for evaluation.
fn filter_chunks(conn: &Connection, chunks: Vec<RetrievedChunk>, filter: &Filter) -> RagResult<Vec<RetrievedChunk>> {
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        // Load the document record for filter evaluation.
        let Some((doc_rec, _)) = load_document_record(conn, &chunk.document_id.0).map_err(be)? else {
            continue;
        };
        // Load the chunk record (content + metadata).
        let Some(chunk_rec) = conn
            .query_row(
                "SELECT ordinal, external_id, content, embedding, chunk_metadata
                 FROM chunks WHERE id = ?1",
                params![chunk.id.0],
                |r| {
                    let meta_str: String = r.get(4)?;
                    Ok(ChunkRecord {
                        ordinal: r.get::<_, u32>(0)?,
                        external_id: r.get(1)?,
                        content: r.get(2)?,
                        embedding: Vec::new(), // not needed for filter evaluation
                        chunk_metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Null),
                    })
                },
            )
            .ok()
        else {
            continue;
        };
        if eval_filter(filter, &doc_rec, &chunk_rec) {
            out.push(chunk);
        }
    }
    Ok(out)
}

// ── Retrieval sub-functions ──────────────────────────────────────────────────

fn retrieve_vector(
    conn: &Connection,
    vec_tbl: &str,
    query_vector: &[f32],
    candidate_k: i64,
    query: &RetrieveQuery,
) -> RagResult<Vec<RetrievedChunk>> {
    let blob = encode_vec(query_vector);
    let sql = format!(
        "SELECT chunk_id, distance FROM {vec_tbl}
         WHERE embedding MATCH ?1 AND k = ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(be)?;
    let knn: Vec<(String, f32)> = stmt
        .query_map(params![blob, candidate_k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)? as f32))
        })
        .map_err(be)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(be)?;

    let ids: Vec<String> = knn.iter().map(|(id, _)| id.clone()).collect();
    let rows = load_chunks_by_ids(conn, &ids).map_err(be)?;

    // sqlite-vec returns ascending distance (lower = more similar).
    // Convert to a 0-1 similarity: similarity = 1 / (1 + distance).
    knn.iter()
        .filter_map(|(cid, dist)| {
            let row = rows.iter().find(|r| &r.id == cid)?;
            let score = 1.0 / (1.0 + dist); // monotone transform, preserves rank
            Some(Ok(RetrievedChunk {
                id: ChunkId(cid.clone()),
                document_id: DocumentId(row.document_id.clone()),
                ordinal: row.ordinal,
                external_id: row.external_id.clone(),
                content: query.include_content.then(|| row.content.clone()),
                score,
                primary_score: PrimaryScore::Vector(score),
                chunk_metadata: row.chunk_metadata.clone(),
                document: None,
            }))
        })
        .collect::<RagResult<Vec<_>>>()
}

fn retrieve_fts(
    conn: &Connection,
    fts_tbl: &str,
    query_text: &str,
    candidate_k: i64,
    query: &RetrieveQuery,
) -> RagResult<Vec<RetrievedChunk>> {
    // FTS5: rank is BM25 (negative; more-negative = more relevant).
    let sql = format!(
        "SELECT chunk_id, -rank AS score FROM {fts_tbl}
         WHERE content MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(be)?;
    let hits: Vec<(String, f32)> = stmt
        .query_map(params![query_text, candidate_k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)? as f32))
        })
        .map_err(be)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(be)?;

    let ids: Vec<String> = hits.iter().map(|(id, _)| id.clone()).collect();
    let rows = load_chunks_by_ids(conn, &ids).map_err(be)?;

    hits.iter()
        .filter_map(|(cid, score)| {
            let row = rows.iter().find(|r| &r.id == cid)?;
            Some(Ok(RetrievedChunk {
                id: ChunkId(cid.clone()),
                document_id: DocumentId(row.document_id.clone()),
                ordinal: row.ordinal,
                external_id: row.external_id.clone(),
                content: query.include_content.then(|| row.content.clone()),
                score: *score,
                primary_score: PrimaryScore::FullText(*score),
                chunk_metadata: row.chunk_metadata.clone(),
                document: None,
            }))
        })
        .collect::<RagResult<Vec<_>>>()
}

fn retrieve_hybrid(
    conn: &Connection,
    vec_tbl: &str,
    fts_tbl: &str,
    query_text: &str,
    query_vector: Option<&[f32]>,
    candidate_k: i64,
    query: &RetrieveQuery,
) -> RagResult<Vec<RetrievedChunk>> {
    // Run vector arm (if a query vector was provided).
    let vec_hits: Vec<(String, f32)> = if let Some(qv) = query_vector {
        let blob = encode_vec(qv);
        let sql = format!(
            "SELECT chunk_id, distance FROM {vec_tbl}
             WHERE embedding MATCH ?1 AND k = ?2"
        );
        let mut stmt = conn.prepare(&sql).map_err(be)?;
        stmt.query_map(params![blob, candidate_k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)? as f32))
        })
        .map_err(be)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(be)?
    } else {
        Vec::new()
    };

    // Run FTS arm.
    let fts_hits: Vec<(String, f32)> = {
        let sql = format!(
            "SELECT chunk_id, -rank AS score FROM {fts_tbl}
             WHERE content MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql).map_err(be)?;
        stmt.query_map(params![query_text, candidate_k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)? as f32))
        })
        .map_err(be)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(be)?
    };

    // Reciprocal Rank Fusion (k_rrf = 60 is the canonical constant).
    const K_RRF: f32 = 60.0;

    let mut rrf_map: std::collections::HashMap<String, (f32, f32, f32, f32)> = std::collections::HashMap::new();
    // (rrf_score, vec_score, fts_score, raw_vec_dist)

    for (rank, (cid, dist)) in vec_hits.iter().enumerate() {
        let vec_score = 1.0 / (1.0 + dist);
        let rrf = 1.0 / (K_RRF + rank as f32 + 1.0);
        let entry = rrf_map.entry(cid.clone()).or_insert((0.0, 0.0, 0.0, *dist));
        entry.0 += rrf;
        entry.1 = vec_score;
    }
    for (rank, (cid, fts_score)) in fts_hits.iter().enumerate() {
        let rrf = 1.0 / (K_RRF + rank as f32 + 1.0);
        let entry = rrf_map.entry(cid.clone()).or_insert((0.0, 0.0, 0.0, 0.0));
        entry.0 += rrf;
        entry.2 = *fts_score;
    }

    // Sort by RRF score descending.
    let mut ranked: Vec<(String, f32, f32, f32)> = rrf_map
        .into_iter()
        .map(|(id, (rrf, vec, fts, _))| (id, rrf, vec, fts))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(candidate_k as usize);

    let ids: Vec<String> = ranked.iter().map(|(id, _, _, _)| id.clone()).collect();
    let rows = load_chunks_by_ids(conn, &ids).map_err(be)?;

    ranked
        .iter()
        .filter_map(|(cid, rrf, vec, fts)| {
            let row = rows.iter().find(|r| &r.id == cid)?;
            Some(Ok(RetrievedChunk {
                id: ChunkId(cid.clone()),
                document_id: DocumentId(row.document_id.clone()),
                ordinal: row.ordinal,
                external_id: row.external_id.clone(),
                content: query.include_content.then(|| row.content.clone()),
                score: *rrf,
                primary_score: PrimaryScore::Hybrid {
                    vector: *vec,
                    full_text: *fts,
                    rrf: *rrf,
                },
                chunk_metadata: row.chunk_metadata.clone(),
                document: None,
            }))
        })
        .collect::<RagResult<Vec<_>>>()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterField;
    use crate::query::RetrieveMode;

    fn mk_chunk(ordinal: u32, content: &str, embedding: Vec<f32>) -> ChunkRecord {
        ChunkRecord {
            external_id: None,
            ordinal,
            content: content.to_string(),
            embedding,
            chunk_metadata: serde_json::Value::Null,
        }
    }

    fn mk_doc(full_text: &str) -> DocumentRecord {
        DocumentRecord {
            full_text: full_text.to_string(),
            ..Default::default()
        }
    }

    async fn empty_store() -> SqliteVectorStore {
        SqliteVectorStore::open_in_memory("test").await.unwrap()
    }

    async fn store_with_col(dim: u32) -> SqliteVectorStore {
        let store = empty_store().await;
        store
            .ensure_collection(&CollectionSpec::new("docs", dim))
            .await
            .unwrap();
        store
    }

    // ── ensure_collection ────────────────────────────────────────────────────

    #[tokio::test]
    async fn ensure_collection_creates_and_is_idempotent() {
        let store = empty_store().await;
        let spec = CollectionSpec::new("col", 4);
        store.ensure_collection(&spec).await.unwrap();
        store.ensure_collection(&spec).await.unwrap(); // idempotent
        let got = store.get_collection("col").await.unwrap().unwrap();
        assert_eq!(got.embedding_dim, 4);
    }

    #[tokio::test]
    async fn ensure_collection_rejects_dim_mismatch() {
        let store = empty_store().await;
        store.ensure_collection(&CollectionSpec::new("col", 4)).await.unwrap();
        let err = store
            .ensure_collection(&CollectionSpec::new("col", 8))
            .await
            .unwrap_err();
        assert!(matches!(err, RagError::CollectionAlreadyExists(_)));
    }

    // ── upsert_document ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_then_retrieve_vector_roundtrip() {
        let store = store_with_col(3).await;
        let doc = mk_doc("hello world");
        let chunks = vec![
            mk_chunk(0, "near", vec![1.0, 0.0, 0.0]),
            mk_chunk(1, "far", vec![0.0, 1.0, 0.0]),
        ];
        let doc_id = store.upsert_document("docs", &doc, &chunks).await.unwrap();
        assert!(doc_id.0.starts_with("test-doc-"));

        let q = RetrieveQuery {
            query_vector: Some(vec![1.0, 0.0, 0.0]),
            include_content: true,
            ..RetrieveQuery::vector(10)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert_eq!(out.mode, RetrieveMode::Vector);
        assert_eq!(out.chunks.len(), 2);
        // First result should be the "near" chunk (cosine distance closest to [1,0,0]).
        assert_eq!(out.chunks[0].content.as_deref(), Some("near"));
    }

    #[tokio::test]
    async fn dimension_mismatch_rejected_at_upsert() {
        let store = store_with_col(3).await;
        let bad = vec![mk_chunk(0, "x", vec![1.0, 0.0])]; // dim=2 vs spec=3
        let err = store.upsert_document("docs", &mk_doc(""), &bad).await.unwrap_err();
        assert!(matches!(err, RagError::EmbeddingDimMismatch { expected: 3, got: 2 }));
    }

    #[tokio::test]
    async fn external_id_upsert_replaces_chunks() {
        let store = store_with_col(2).await;
        let doc = DocumentRecord {
            external_id: Some("ext-1".to_string()),
            ..Default::default()
        };
        store
            .upsert_document("docs", &doc, &[mk_chunk(0, "v1", vec![1.0, 0.0])])
            .await
            .unwrap();
        store
            .upsert_document("docs", &doc, &[mk_chunk(0, "v2", vec![0.0, 1.0])])
            .await
            .unwrap();
        let stats = store.collection_stats("docs").await.unwrap();
        assert_eq!(stats.documents, 1, "should still be one document");
        assert_eq!(stats.chunks, 1, "old chunk replaced by new chunk");

        // Check content is updated.
        let q = RetrieveQuery {
            query_vector: Some(vec![0.0, 1.0]),
            include_content: true,
            ..RetrieveQuery::vector(10)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert_eq!(out.chunks[0].content.as_deref(), Some("v2"));
    }

    // ── full-text retrieval ──────────────────────────────────────────────────

    #[tokio::test]
    async fn full_text_retrieve_roundtrip() {
        let store = store_with_col(2).await;
        let doc_a = mk_doc("document about rust programming");
        let doc_b = mk_doc("document about python scripting");
        store
            .upsert_document("docs", &doc_a, &[mk_chunk(0, "Rust is fast and safe", vec![1.0, 0.0])])
            .await
            .unwrap();
        store
            .upsert_document(
                "docs",
                &doc_b,
                &[mk_chunk(0, "Python is easy to learn", vec![0.0, 1.0])],
            )
            .await
            .unwrap();

        let q = RetrieveQuery {
            mode: RetrieveMode::FullText,
            query_text: Some("Rust".to_string()),
            include_content: true,
            ..RetrieveQuery::vector(5)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert!(!out.chunks.is_empty());
        assert!(
            out.chunks[0].content.as_deref().unwrap_or("").contains("Rust"),
            "top result should mention Rust"
        );
    }

    // ── hybrid retrieval ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn hybrid_retrieve_returns_results() {
        let store = store_with_col(2).await;
        let doc = mk_doc("hybrid search document");
        store
            .upsert_document("docs", &doc, &[mk_chunk(0, "hybrid search content", vec![1.0, 0.0])])
            .await
            .unwrap();

        let q = RetrieveQuery {
            mode: RetrieveMode::Hybrid,
            query_text: Some("hybrid".to_string()),
            query_vector: Some(vec![1.0, 0.0]),
            include_content: true,
            ..RetrieveQuery::vector(5)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert!(!out.chunks.is_empty());
        let ps = out.chunks[0].primary_score;
        assert!(matches!(ps, PrimaryScore::Hybrid { .. }));
    }

    // ── delete_by_filter ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_by_filter_removes_matching_documents() {
        let store = store_with_col(2).await;
        let keep = DocumentRecord {
            title: Some("keep".to_string()),
            ..Default::default()
        };
        let drop = DocumentRecord {
            title: Some("drop".to_string()),
            ..Default::default()
        };
        store
            .upsert_document("docs", &keep, &[mk_chunk(0, "keep", vec![1.0, 0.0])])
            .await
            .unwrap();
        store
            .upsert_document("docs", &drop, &[mk_chunk(0, "drop", vec![0.0, 1.0])])
            .await
            .unwrap();

        let filter = Filter::Eq {
            field: FilterField("doc.title".to_string()),
            value: serde_json::json!("drop"),
        };
        let removed = store.delete_by_filter("docs", &filter).await.unwrap();
        assert_eq!(removed, 1);

        let stats = store.collection_stats("docs").await.unwrap();
        assert_eq!(stats.documents, 1);
        assert_eq!(stats.chunks, 1);
    }

    // ── drop_collection ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn drop_collection_removes_all_data() {
        let store = store_with_col(2).await;
        store
            .upsert_document("docs", &mk_doc("x"), &[mk_chunk(0, "x", vec![1.0, 0.0])])
            .await
            .unwrap();
        store.drop_collection("docs").await.unwrap();
        let spec = store.get_collection("docs").await.unwrap();
        assert!(spec.is_none());
    }

    #[tokio::test]
    async fn drop_nonexistent_collection_errors() {
        let store = empty_store().await;
        let err = store.drop_collection("ghost").await.unwrap_err();
        assert!(matches!(err, RagError::CollectionNotFound(_)));
    }

    // ── collection_stats ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn collection_stats_tracks_documents_and_chunks() {
        let store = store_with_col(2).await;
        store
            .upsert_document(
                "docs",
                &mk_doc("a"),
                &[mk_chunk(0, "c0", vec![1.0, 0.0]), mk_chunk(1, "c1", vec![0.0, 1.0])],
            )
            .await
            .unwrap();
        let stats = store.collection_stats("docs").await.unwrap();
        assert_eq!(stats.documents, 1);
        assert_eq!(stats.chunks, 2);
        assert!(stats.last_ingested_at.is_some());
    }
}
