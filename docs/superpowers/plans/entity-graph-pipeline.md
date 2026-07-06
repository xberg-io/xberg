# Entity-Graph-Pipeline Implementation Plan

**Spec:** `docs/superpowers/specs/2026-07-03-entity-graph-pipeline-design.md`
**Date:** 2026-07-03
**Status:** Draft

---

## Task 1: Feature flags + graph schema init

**Depends on:** nothing
**Files:**
- `crates/xberg-rag/Cargo.toml` — add `sqlite-graph = ["sqlite", "dep:serde_json"]` and `pipeline-ner-candle = ["pipeline", "xberg/ner-candle"]`
- `crates/xberg-rag/src/backends/mod.rs` — add `#[cfg(feature = "sqlite-graph")] pub mod entity_graph;`
- `crates/xberg-rag/src/backends/entity_graph.rs` — **NEW**: `pub fn graph_init_schema(conn: &Connection) -> RagResult<()>` (CREATE TABLE IF NOT EXISTS for `_graph_nodes`, `_graph_edges`, `_graph_properties` with CASCADE and indexes)
- `crates/xberg-rag/src/backends/sqlite.rs` — add `graph_enabled: bool` field to `SqliteVectorStore`; in `open_conn()` after `setup_schema()`, call `graph_init_schema()` when feature is enabled; in `open()` and `open_in_memory()` pass the flag

**Tests:** `graph_init_schema` creates tables on clean in-memory SQLite (verify `_graph_nodes`, `_graph_edges`, `_graph_properties` exist)

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 2: Entity normalization + confidence tiering

**Depends on:** nothing
**Files:**
- `crates/xberg-rag/src/backends/entity_graph.rs` — add `pub fn normalize_entity(text: &str, category: &EntityCategory) -> String` and `pub fn confidence_tier(confidence: Option<f32>) -> (&'static str, f64)`

**Tests:** Unit tests for all `EntityCategory` variants including `Custom("foo")`, boundary values (0.0, 0.5, 0.8, 1.0), and `None`

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 3: Graph CRUD functions

**Depends on:** Task 1
**Files:**
- `crates/xberg-rag/src/backends/entity_graph.rs` — add standalone functions:
  - `graph_create_node(conn, id, labels, properties)`
  - `graph_create_edge(conn, id, source, target, label, properties)`
  - `graph_delete_node(conn, id) -> RagResult<u64>`
  - `graph_get_nodes_by_label(conn, label) -> RagResult<Vec<String>>`
  - `graph_get_node_count(conn) -> RagResult<u64>`
  - `graph_get_edge_count(conn) -> RagResult<u64>`

**Tests:** CRUD round-trip: create nodes → create edges → verify counts → delete node → verify CASCADE cleanup → verify label query

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 4: Graph traversal + PageRank

**Depends on:** Task 3
**Files:**
- `crates/xberg-rag/src/backends/entity_graph.rs` — add:
  - `graph_traverse_bfs(conn, seeds, depth, edge_labels) -> RagResult<Vec<String>>`
  - `graph_pagerank(conn, damping, max_iterations) -> RagResult<Vec<(String, f64)>>`

**Tests:** BFS from seed with depth 1 and 2, verify reachable nodes; PageRank on small graph verify scores sum to ~1.0 and most-connected node scores highest

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 5: RetrieveQuery + PrimaryScore additions

**Depends on:** nothing
**Files:**
- `crates/xberg-rag/src/types.rs` — add `PrimaryScore::Graph { pagerank: f64, depth_from_seed: u32 }` and `PrimaryScore::HybridGraph { vector: f32, pagerank: f64, combined: f32 }`
- `crates/xberg-rag/src/query.rs` — add `graph_seed_ids: Option<Vec<String>>`, `graph_edge_labels: Option<Vec<String>>`, `graph_min_confidence: Option<f32>` to `RetrieveQuery`; update `vector()` constructor defaults; update `validate()` for graph mode to accept EITHER `query_text`/`query_vector` (for auto-seed) OR `graph_seed_ids` (for explicit seeds) — currently graph mode rejects without query_text/query_vector (query.rs:147-153), change to: `if self.query_text.is_none() && self.query_vector.is_none() && self.graph_seed_ids.is_none() { return Err(...) }`
- `crates/xberg-rag/src/capability.rs` — add `pub graph: bool` to `Capabilities`; update `vector_only()` to default `graph: false`

**Tests:** Query validation accepts graph mode with graph_seed_ids; PrimaryScore serialization round-trip

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 6: Graph construction in pipeline

**Depends on:** Tasks 1, 2, 3
**Files:**
- `crates/xberg-rag/src/pipeline.rs` — add:
  ```rust
  #[cfg(feature = "sqlite-graph")]
  pub fn build_entity_graph(
      conn: &Connection,
      document_id: &str,
      chunks: &[ChunkRecord],
      entities: &serde_json::Value,
      full_text: &str,
  ) -> RagResult<()>
  ```
  - Deserialize entities from JSON into `Vec<Entity>` (using xberg types if available, or a local struct)
  - For each chunk: compute byte-offset range (sum of content lengths up to that chunk)
  - Map doc-level entities to chunks by position overlap
  - For each chunk's entities: filter low-confidence (< 0.5), normalize, UPSERT entity node, CREATE CONTAINS edge
  - For each chunk: CREATE COOCCURS edges for entity pairs
- `crates/xberg-rag/src/backends/sqlite.rs` — in `upsert_document()` after inserting chunks, if `graph_enabled`, call `build_entity_graph(conn, &doc_id.0, &chunks, &document.entities, &document.full_text)`

**Byte-offset mapping (Path 1, doc-level entities):** `build_entity_graph()` receives the document's `full_text` and `chunks`. For each chunk, compute its byte range by scanning `full_text` for the chunk content (or using `chunk_metadata.byte_start`/`byte_end` if present from xberg chunking). An entity's `(start, end)` byte range overlaps a chunk if `entity_start < chunk_end && entity_end > chunk_start`. Entities outside all chunks are attached to the last chunk.

**Tests:** Ingest document with known entities → verify graph nodes and edges created; ingest with empty entities → no graph nodes; ingest with confidence < 0.5 entities → those discarded; entity-to-chunk mapping verifies correct chunk association

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 7: Graph retrieval (RetrieveMode::Graph)

**Depends on:** Tasks 4, 5, 6
**Files:**
- `crates/xberg-rag/src/backends/sqlite.rs` — replace the `RetrieveMode::Graph` error arm with:
  - Auto-seed: if `graph_seed_ids` is None, reuse existing `retrieve_vector()`/`retrieve_fts()`/`retrieve_hybrid()` to find top-K candidate chunks (reuses search logic, avoids drift), extract their `document_id`, convert to graph node IDs (`{store_name}-doc-{rowid}`)
  - BFS: call `graph_traverse_bfs(conn, seeds, depth, edge_labels)`
  - Confidence filtering: post-BFS, for each entity node in BFS result, read `properties.confidence` from `_graph_nodes`. If `graph_min_confidence` is set and confidence < threshold, remove that entity and its edges from the result set. This avoids modifying the BFS function signature.
  - Filter to Document nodes from BFS result (keep only nodes with label "Document")
  - Subgraph PageRank: build `IN` clause from BFS result set (batch into chunks of 500 if >500 nodes), run SQL `SELECT source, target FROM _graph_edges WHERE source IN (...) AND target IN (...)`, build adjacency list in Rust, run PageRank in Rust (reuse existing `graph_pagerank()` logic but on subgraph only)
  - Load chunks for scored documents, assign `PrimaryScore::Graph` or `PrimaryScore::HybridGraph`
  - Return top_k chunks sorted by score
- `crates/xberg-rag/src/backends/mod.rs` — update `capabilities()` to return `graph: true` when `sqlite-graph` feature enabled

**Tests:** Ingest 3 documents with shared entities → graph retrieval with explicit seeds returns connected documents; auto-seed from text query works; depth=1 returns only direct neighbors; document deletion cleans up graph

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 8: Graph cleanup on document deletion

**Depends on:** Task 1 (CASCADE)
**Files:**
- `crates/xberg-rag/src/backends/sqlite.rs` — in `delete_documents()`, after deleting from `documents` table, graph cleanup is automatic via CASCADE (no code change needed). Add test verification only.

**Tests:** Ingest document → verify graph nodes exist → delete document → verify graph nodes and edges are gone; re-ingest same document with same entities → verify idempotent (same node count, same edge count — UNIQUE constraint replaces edges)

**Verify:** `cargo test -p xberg-rag --features sqlite-graph`

---

## Task 9: NAPI-RS graph methods

**Depends on:** Tasks 3, 4
**Files:**
- `crates/xberg-rag-node/src/lib.rs` — add a new `RagGraphStore` NAPI struct that holds `Arc<Mutex<Connection>>` directly (graph ops are SQLite-specific, not backend-agnostic). Add `open_graph(db_path)` factory method. Add methods:
  - `traverse_bfs(start_ids_json, depth, edge_labels_json) -> String`
  - `pagerank(damping, max_iterations) -> String`
  - `get_nodes_by_label(label) -> String`
  - `get_node(id) -> Option<String>`
  - `delete_node(id) -> f64`
  - `graph_node_count() -> f64`
  - `graph_edge_count() -> f64`
  - `graph_node_exists(id) -> bool` (convenience for checking if a node exists before traversal)
- `crates/xberg-rag-node/Cargo.toml` — add `features = ["sqlite-graph"]` to xberg-rag dependency

**Why a separate struct:** `RagStore` holds `Arc<dyn VectorStore>` which is backend-agnostic. Graph standalone functions need `&Connection`. Adding graph methods to `VectorStore` would leak SQLite-specific concerns into the trait. A separate `RagGraphStore` NAPI struct wraps the connection directly and calls the standalone functions. The `open_sqlite` factory creates both a `RagStore` (vector ops) and a `RagGraphStore` (graph ops) from the same DB path.

**Tests:** Build NAPI module → verify methods are callable from a test harness

**Verify:** `cargo build -p xberg-rag-node`

---

## Task 10: MCP server — ingest_document NER params

**Depends on:** Task 9
**Files:**
- `mcp-server/src/tools/ingest.ts` — add NER parameters to `ingest_document` schema:
  - `use_ner`, `ner_backend`, `ner_model`, `ner_hf_repo`, `ner_hf_model_file`, `ner_hf_tokenizer_file`, `ner_hf_architecture`, `ner_llm_model`, `ner_categories`
  - When `use_ner=true`: the `full_text` parameter is already extracted text (the tool description says "pre-extracted document"). Run NER on `full_text` by calling `extract()` with a synthetic document containing the text and NER config. The NER backend processes the text and returns `doc.entities`. Serialize entities as JSON array `[{category, text, start, end, confidence}]` and pass to `upsertDocument()` via the `entities` field. The Rust side builds the graph from those entities.
  - When `use_ner=false` (default): pass `entities: null` as current behavior

**Tests:** MCP tool schema validation with/without NER params; mock extract() call with entities

**Verify:** `task mcp:build`

---

## Task 11: MCP server — query_corpus graph params

**Depends on:** Task 9
**Files:**
- `mcp-server/src/tools/query.ts` — add graph params to `query_corpus`:
  - `graph_seed_ids: z.array(z.string()).optional()`
  - `graph_edge_labels: z.array(z.string()).optional()`
  - `graph_min_confidence: z.number().min(0).max(1).optional()`
  - Wire into `retrieveQuery` object

**Tests:** MCP tool schema validation with graph params; query with mode=graph passes params correctly

**Verify:** `task mcp:build`

---

## Task 12: End-to-end verification

**Depends on:** all above
**Files:** none (verification only)

**Steps:**
1. `task build:bindings` — ensure all bindings compile
2. `cargo test -p xberg-rag --features sqlite-graph` — all unit + integration tests pass
3. `cargo test -p xberg-rag --features "sqlite-graph,pipeline-ner-candle"` — NER pipeline tests pass
4. `task mcp:build` — MCP server compiles
5. `task mcp:test` — MCP server tests pass
6. `prek run --all-files` — pre-commit hooks pass

**Verify:** All tests green, no warnings, no clippy issues
