---
description: RAG (Retrieval-Augmented Generation) pipeline architecture — fork-local crates xberg-rag and xberg-rag-node
---

- `RagStore` trait is the single public abstraction over all backends (SQLite, graphqlite, in-memory)
- Backend selection is at construction time — callers never reference backend types directly
- `RetrieveMode` variants: `Vector` (ANN search), `FullText` (BM25), `Hybrid` (RRF fusion), `Graph` (graphqlite)
- Embedding pipeline: text → `embedTexts` (NAPI-RS or Rust direct) → `Vec<f32>` stored alongside chunk text
- SQLite backend uses `sqlite-vec` extension for vector similarity; graphqlite backend for entity-graph traversal
- `openSqlite(name, path)` is the canonical constructor — returns `RagStore` interface, not a concrete type
- All store operations are async; NAPI-RS boundary uses `#[napi]` on `async fn` (no blocking)
- Chunk metadata required fields: `chunkIndex`, `sourceId`, `collectionId`; extra fields stored as JSON blob
- Collections are named namespaces within a single SQLite DB file — no cross-collection queries
- Fork-local: these crates are not in upstream `jamon8888/xberg`; conflict risk is zero on upstream pull
