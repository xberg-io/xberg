---
priority: high
---

# RAG Store Rules

- `RagStore` trait must remain object-safe: no generic methods, no associated types with bounds
- Every backend must implement `upsert`, `query`, `delete`, `list_collections`, `drop_collection`
- `upsert` is idempotent on `(collectionId, sourceId, chunkIndex)` — re-ingesting the same document replaces existing chunks
- `query` returns `Vec<RetrievedChunk>` sorted by score descending; caller slices to top-k
- `drop_collection` must also remove all vectors and metadata for that collection — no orphaned rows
- SQLite WAL mode required: `PRAGMA journal_mode=WAL` on open to allow concurrent readers
- `sqlite-vec` extension must be loaded before any vector operations: `conn.load_extension("sqlite_vec")`
- Graphqlite backend: only enable when `RetrieveMode::Graph`; fall back to SQLite vector for other modes
- In-memory backend is test-only — never expose it as a public API surface or production default
- Error messages from store operations must include the collection name and operation name
