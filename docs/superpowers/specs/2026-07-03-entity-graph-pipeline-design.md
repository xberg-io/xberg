# Entity-Graph-Pipeline Design

**Date:** 2026-07-03
**Status:** Approved
**Scope:** xberg-rag + xberg-rag-node + MCP server

---

## 1. Goal

Integrate GLiNER2 entity extraction with the SQLite-backed graph store to enable entity-centric discovery and relationship traversal in RAG retrieval. Replace the current disconnected `null`-entities pattern with a full entity-graph-pipeline that builds a graph during ingestion and queries it during retrieval.

## 2. Constraints

- Both `SqliteVectorStore` and graph operations use `rusqlite::Connection` but have separate ownership — must share via `Arc<Mutex<Connection>>`
- `GraphStore::new(conn: Connection)` takes ownership; cannot share with `SqliteVectorStore`'s `Arc<Mutex<Connection>>`
- GLiNER2 extracts flat entity spans `(start, end, text, category, confidence)` — no relationships
- `Entity.category` is `EntityCategory` enum with `Custom(String)` variant for arbitrary labels
- `Entity.confidence` is `Option<f32>` — not always present
- `IngestRequest.entities` is `serde_json::Value` passthrough — never processed into graph
- `RetrieveMode::Graph` currently returns `UnsupportedMode` in `SqliteVectorStore`
- MCP server calls NER through `@xberg-io/xberg` `extract()`, not through xberg-rag-node
- `ingest_folder` already has extensive NER parameters — `ingest_document` should follow the same pattern

## 3. Architecture

### 3.1 Connection Sharing Model

`SqliteVectorStore` owns `Arc<Mutex<Connection>>`. Graph operations are implemented as standalone functions taking `&Connection`, called inside `SqliteVectorStore::with_conn()`:

```rust
// crates/xberg-rag/src/backends/entity_graph.rs

fn graph_init_schema(conn: &Connection) -> RagResult<()>;
fn graph_create_node(conn: &Connection, id: &str, labels: &[&str], properties: &Value) -> RagResult<()>;
fn graph_create_edge(conn: &Connection, id: &str, source: &str, target: &str, label: &str, properties: &Value) -> RagResult<()>;
fn graph_traverse_bfs(conn: &Connection, seeds: &[String], depth: u32, edge_labels: &[&str]) -> RagResult<Vec<String>>;
fn graph_pagerank(conn: &Connection, damping: f64, max_iterations: u32) -> RagResult<Vec<(String, f64)>>;
fn graph_delete_node(conn: &Connection, id: &str) -> RagResult<u64>;
fn graph_get_nodes_by_label(conn: &Connection, label: &str) -> RagResult<Vec<String>>;
fn graph_get_node_count(conn: &Connection) -> RagResult<u64>;
fn graph_get_edge_count(conn: &Connection) -> RagResult<u64>;
```

These functions are called inside `with_conn()` closures — same mutex, same `spawn_blocking`, no new ownership. The existing `GraphStore` struct stays unchanged for standalone use.

`SqliteVectorStore` gains a `graph_enabled: bool` field set at construction time based on the `sqlite-graph` feature flag.

### 3.2 Entity Normalization

`normalize_entity()` produces a canonical key for deduplication:

```rust
fn normalize_entity(text: &str, category: &EntityCategory) -> String {
    let cat_str = match category {
        EntityCategory::Custom(s) => format!("custom:{s}"),
        other => format!("{other:?}").to_lowercase(),
    };
    let lower = text.trim().to_lowercase();
    format!("ent:{cat_str}:{lower}")
}
```

Examples:
- `("Alice Smith", Person)` → `"ent:person:alice smith"`
- `("alice smith", Person)` → `"ent:person:alice smith"` (dedup)
- `("acme corp", Custom("company"))` → `"ent:custom:company:acme corp"`

### 3.3 Graph Schema

```sql
CREATE TABLE IF NOT EXISTS _graph_nodes (
    id TEXT PRIMARY KEY,
    labels TEXT NOT NULL DEFAULT '[]',
    properties TEXT NOT NULL DEFAULT '{}'
) STRICT;

CREATE TABLE IF NOT EXISTS _graph_edges (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL REFERENCES _graph_nodes(id) ON DELETE CASCADE,
    target TEXT NOT NULL REFERENCES _graph_nodes(id) ON DELETE CASCADE,
    label TEXT NOT NULL DEFAULT '',
    properties TEXT NOT NULL DEFAULT '{}',
    UNIQUE(source, target, label)
) STRICT;

CREATE TABLE IF NOT EXISTS _graph_properties (
    node_id TEXT NOT NULL REFERENCES _graph_nodes(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (node_id, key)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_edges_source ON _graph_edges(source);
CREATE INDEX IF NOT EXISTS idx_edges_target ON _graph_edges(target);
CREATE INDEX IF NOT EXISTS idx_edges_label ON _graph_edges(label);
```

**CASCADE delete:** When a document node is deleted, SQLite automatically removes all connected edges and properties. One query, zero orphan checks.

Schema initialization happens in `SqliteVectorStore::open()` when `sqlite-graph` is enabled — calls `graph_init_schema()` on the connection.

### 3.4 Graph Node Structures

**Document node ID format:** `{collection_name}-doc-{rowid}` — matches the existing document ID format in `SqliteVectorStore::insert_new_document()` (`crates/xberg-rag/src/backends/sqlite.rs:1009`). The graph uses the same ID as the `documents` table.

**Document node:**
```json
{
  "id": "default-doc-42",
  "labels": ["Document"],
  "properties": {
    "title": "Q4 Report",
    "collection": "default"
  }
}
```

**Entity node:**
```json
{
  "id": "ent:person:alice smith",
  "labels": ["Entity", "Person"],
  "properties": {
    "text": "Alice Smith",
    "category": "Person",
    "confidence": 0.92,
    "tier": "high"
  }
}
```

### 3.5 Edge Types

- **`CONTAINS`** — Document → Entity (one per entity found in a document)
- **`COOCCURS`** — Entity → Entity (chunk-level pairs within same chunk)

**CONTAINS edge properties:**
```json
{ "confidence": 0.92, "tier": "high" }
```

**COOCCURS edge properties:**
```json
{ "chunk_id": "default-doc-42:0", "confidence_min": 0.85, "confidence_avg": 0.88 }
```

### 3.6 Tiered Confidence Filtering

| Tier | Confidence Range | Behavior |
|------|-----------------|----------|
| `high` | ≥ 0.8 | Always included, weight 1.0 |
| `medium` | 0.5 - 0.8 | Included, flagged, weight 0.7 |
| `low` | < 0.5 | **Discarded**, never enters graph |
| `None` | confidence absent | Treated as `medium` |

```rust
fn confidence_tier(confidence: Option<f32>) -> (&'static str, f64) {
    match confidence {
        Some(c) if c >= 0.8 => ("high", 1.0),
        Some(c) if c >= 0.5 => ("medium", 0.7),
        Some(_) => ("low", 0.0),       // discarded
        None => ("medium", 0.7),       // unknown → include as medium
    }
}
```

## 4. Ingestion Flow

### 4.1 Chunk-Level Co-occurrence

**Two NER paths exist:**

**Path 1 — NER via `@xberg-io/xberg` extract() (MCP server path):**
- NER runs at document level inside `extract()`, returning `Vec<Entity>` with byte-offset positions
- Entities are serialized as JSON and passed to `upsertDocument()` via `IngestRequest.entities: serde_json::Value`
- **Entity JSON schema** (matches `xberg::types::Entity` serialization):
  ```json
  [
    {
      "category": "person",
      "text": "Alice Smith",
      "start": 42,
      "end": 54,
      "confidence": 0.92
    }
  ]
  ```
  Fields: `category` (snake_case string), `text` (string), `start` (u32 byte offset inclusive), `end` (u32 byte offset exclusive), `confidence` (f32 or null).
- `build_entity_graph()` deserializes this JSON into `Vec<Entity>` using serde, then maps entities to chunks by **position overlap**: each entity's `(start, end)` byte range is compared to each chunk's character offset range. An entity belongs to a chunk if its byte range overlaps the chunk's text range.
- No NER is re-run — entities come from the caller

**Path 2 — NER via `pipeline-ner-candle` (Rust pipeline path):**
- `build_entity_graph()` runs NER **per-chunk** using the chunk's text
- Each chunk gets its own `Vec<Entity>` with fresh positions relative to the chunk
- No position mapping needed — entities are already chunk-scoped

**Shared graph construction (both paths):**

```
For each chunk:
  1. Get entities for this chunk (from Path 1 position mapping or Path 2 per-chunk NER)
  2. Filter: discard confidence < 0.5 (low tier)
  3. For each entity above threshold:
     a. Normalize to canonical key: "ent:{category}:{lowercase_text}"
     b. UPSERT entity node (INSERT OR REPLACE into _graph_nodes)
     c. CREATE edge: document_id → entity_id (label="CONTAINS")
  4. For each pair of entities in this chunk (i < j):
     a. CREATE edge: entity_i → entity_j (label="COOCCURS")
```

**Complexity:** A 512-token chunk has ~5-15 entities. Pairs = ~10-105 edges per chunk. Very manageable.

**Dedup:** `UNIQUE(source, target, label)` on `_graph_edges` handles re-ingestion — re-ingesting a document replaces existing edges. If the same entity pair appears in multiple chunks, the edge is replaced (not duplicated). An `occurrence_count` property could be added later but is not in v1 scope.

**When NER is disabled and no entities provided:** Graph construction is skipped entirely. `IngestRequest.entities` is used as-is (free-form JSON). Callers who bring their own NER can populate `entities` and we skip graph building.

### 4.2 NER Backend Selection

The MCP server's `ingest_document` tool accepts NER configuration matching `ingest_folder`'s existing parameters:

```typescript
// ingest_document gains:
use_ner: z.boolean().optional().default(false),
ner_backend: z.enum(["onnx", "llm", "candle"]).optional().default("candle"),
ner_model: z.string().optional(),           // ONNX model alias
ner_hf_repo: z.string().optional(),         // Custom HF repo
ner_hf_model_file: z.string().optional(),
ner_hf_tokenizer_file: z.string().optional(),
ner_hf_architecture: z.enum(["gliner1", "gliner2"]).optional(),
ner_llm_model: z.string().optional(),       // LLM model string
ner_categories: z.array(z.string()).optional(),
```

NER runs inside `@xberg-io/xberg` `extract()` — the MCP server configures it, receives entities in the result, passes them to `upsertDocument()`. The Rust side builds the graph from those entities.

### 4.3 Feature Flags

```toml
# In crates/xberg-rag/Cargo.toml

# Graph CRUD + traversal (pure rusqlite, no heavy deps)
sqlite-graph = ["sqlite", "dep:serde_json"]

# GLiNER2 NER backend for auto-extraction during ingestion
pipeline-ner-candle = ["pipeline", "xberg/ner-candle"]
```

**What each enables:**

| Feature | Enables | Pulls in |
|---------|---------|----------|
| `sqlite-graph` | Graph tables, graph CRUD functions, `RetrieveMode::Graph` path, graph cleanup on delete | `rusqlite` (already), `serde_json` (already) |
| `pipeline-ner-candle` | `CandleBackend` NER integration, `build_entity_graph()` function | `xberg/ner-candle` (GLiNER2 safetensors) |

**Caller scenarios:**

| Caller | NER source | Flags needed | What they get |
|--------|-----------|-------------|---------------|
| MCP server | `@xberg-io/xberg` `extract()` (external) | `sqlite-graph` only | Graph construction from extract() entities + graph retrieval |
| Rust pipeline (with Candle NER) | `pipeline-ner-candle` (internal, **defined in this spec**) | `sqlite-graph` + `pipeline-ner-candle` | Auto-NER per-chunk + graph construction + graph retrieval |
| Rust pipeline (with ONNX/LLM NER) | `pipeline-ner-onnx` or `pipeline-ner-llm` (**existing features**, not in scope) | `sqlite-graph` + existing NER feature | Auto-NER + graph construction + graph retrieval |
| External NER user | Caller provides entities | `sqlite-graph` only | Graph construction from provided entities + graph retrieval |
| Simple RAG (no graph) | none | neither | Vector + FTS + hybrid only (current behavior) |

**Key distinction:** `pipeline-ner-candle` is only needed when NER runs inside the xberg-rag pipeline (Rust-side). The MCP server runs NER through `@xberg-io/xberg` `extract()` — a separate NAPI-RS binding — so it only needs `sqlite-graph`. `pipeline-ner-onnx` and `pipeline-ner-llm` are existing features in `xberg-rag/Cargo.toml` (lines 25-26) — they already work for graph construction via the same `build_entity_graph()` path.

**Note:** `IngestRequest.entities: serde_json::Value` already exists (pipeline.rs:61). No struct changes needed — the field is already a free-form JSON passthrough.

## 5. Retrieval Flow

### 5.1 Graph Mode Retrieval

**Two entry paths:**

**Path A — Explicit seeds:**
```
query.graph_seed_ids = ["ent:organization:acme"]
query.mode = RetrieveMode::Graph
query.graph_depth = 2
```

**Path B — Auto-seed from vector/FTS:**
```
query.query_text = "What projects is Acme involved in?"
query.mode = RetrieveMode::Graph
query.graph_depth = 2
```
→ First runs vector/FTS search to find seed documents, then BFS from those documents.

**Algorithm:**

```
1. Resolve seeds:
   a. If graph_seed_ids provided → use them directly
   b. If no seeds but query_text/query_vector → run vector/FTS search
      to get top-K candidate document IDs → use those as seeds

2. BFS from seeds (depth = graph_depth, default 2):
   a. Start from seed node IDs
   b. Traverse CONTAINS and COOCCURS edges (configurable via graph_edge_labels)
   c. Filter edges: skip edges where the target entity's confidence < graph_min_confidence
      (read confidence from entity node properties; skip edges to entities below threshold)
   d. Collect all reachable node IDs

3. Filter to document nodes:
   a. From BFS result, keep only nodes with label "Document"
   b. These are the documents connected to the seed entities

4. PageRank on the subgraph:
   a. Build subgraph from BFS results: fetch all edges where both source and target are in the BFS result set (single SQL query: `SELECT source, target FROM _graph_edges WHERE source IN (...) AND target IN (...)`)
   b. Run PageRank in Rust on the adjacency list (existing `graph_pagerank()` function operates on full graph; for subgraph, filter to BFS-result nodes first)
   c. Score each document node

5. Return results:
   a. Load chunks for scored documents
   b. Score = PageRank score (or combined with vector/FTS score for auto-seed)
   c. Sort by score descending
   d. Return top_k chunks
```

### 5.2 PrimaryScore Variants

```rust
pub enum PrimaryScore {
    Vector(f32),
    FullText(f32),
    Hybrid { vector: f32, full_text: f32, rrf: f32 },
    // NEW
    Graph { pagerank: f64, depth_from_seed: u32 },
    HybridGraph { vector: f32, pagerank: f64, combined: f32 },
}
```

`HybridGraph` is used for auto-seed mode where vector similarity and PageRank are combined:
```
combined = alpha * vector_score + (1 - alpha) * pagerank_score
```
Where `alpha` defaults to 0.7 (vector-heavy, graph as boost).

### 5.3 RetrieveQuery Additions

```rust
pub struct RetrieveQuery {
    // ... existing fields ...
    pub graph_depth: Option<u32>,          // ALREADY EXISTS (line 72)
    
    // NEW fields:
    pub graph_seed_ids: Option<Vec<String>>,
    pub graph_edge_labels: Option<Vec<String>>,
    pub graph_min_confidence: Option<f32>,
}
```

Defaults:
- `graph_seed_ids`: `None` (auto-seed from vector/FTS)
- `graph_edge_labels`: `None` (all edge types)
- `graph_depth`: `Some(2)` (already exists, BFS depth 2)
- `graph_min_confidence`: `Some(0.5)` (match tier threshold)

### 5.4 Capabilities

```rust
pub struct Capabilities {
    pub full_text: bool,
    pub hybrid: bool,
    pub filtering: bool,
    pub index_methods: Vec<IndexMethod>,
    // NEW
    pub graph: bool,
}
```

When `sqlite-graph` is enabled: `capabilities()` returns `graph: true`.

## 6. NAPI-RS Bridge

### 6.1 Graph Traversal/Scoring Methods

Graph construction is internal (happens inside `upsertDocument`). Traversal and scoring are exposed:

```rust
// crates/xberg-rag-node/src/lib.rs

#[napi]
impl RagStore {
    #[napi]
    pub async fn traverse_bfs(&self, start_ids_json: String, depth: u32, edge_labels_json: String) -> napi::Result<String>

    #[napi]
    pub async fn pagerank(&self, damping: f64, max_iterations: u32) -> napi::Result<String>

    #[napi]
    pub async fn get_nodes_by_label(&self, label: String) -> napi::Result<String>

    #[napi]
    pub async fn get_node(&self, id: String) -> napi::Result<Option<String>>

    #[napi]
    pub async fn delete_node(&self, id: String) -> napi::Result<f64>

    #[napi]
    pub async fn graph_node_count(&self) -> napi::Result<f64>

    #[napi]
    pub async fn graph_edge_count(&self) -> napi::Result<f64>
}
```

All parameters are JSON strings — matches existing NAPI pattern.

## 7. MCP Server Changes

### 7.1 `ingest_document` Tool

```typescript
server.tool(
  "ingest_document",
  "Chunk, embed, and store a pre-extracted document in a RAG collection.",
  {
    collection: z.string(),
    full_text: z.string(),
    title: z.string().optional(),
    mime: z.string().optional(),
    source_uri: z.string().optional(),
    external_id: z.string().optional(),
    keywords: z.array(z.string()).optional(),
    metadata: z.record(z.unknown()).optional(),
    // NEW: NER parameters (matching ingest_folder pattern)
    use_ner: z.boolean().optional().default(false),
    ner_backend: z.enum(["onnx", "llm", "candle"]).optional().default("candle"),
    ner_model: z.string().optional(),
    ner_hf_repo: z.string().optional(),
    ner_hf_model_file: z.string().optional(),
    ner_hf_tokenizer_file: z.string().optional(),
    ner_hf_architecture: z.enum(["gliner1", "gliner2"]).optional(),
    ner_llm_model: z.string().optional(),
    ner_categories: z.array(z.string()).optional(),
  },
  async (params) => {
    // If use_ner=true, build ExtractionConfig with NER, call extract()
    // Get entities from extraction result
    // Pass entities to upsertDocument
    // Graph construction happens on the Rust side
  }
);
```

### 7.2 `query_corpus` Tool

```typescript
// Gains graph-specific parameters:
{
  // ... existing params ...
  graph_depth: z.number().int().min(1).max(5).optional().default(2),
  graph_seed_ids: z.array(z.string()).optional(),
  graph_edge_labels: z.array(z.string()).optional(),
  graph_min_confidence: z.number().min(0).max(1).optional(),
}
```

### 7.3 Entity Flow

```
MCP ingest_document (use_ner=true)
  │
  ├── @xberg-io/xberg extract() with NER config
  │     └── Returns doc.entities: Vec<Entity> (with byte-offset positions)
  │
  ├── Pass entities + chunks to upsertDocument()
  │     └── Rust side: build_entity_graph(doc_entities, chunks)
  │           ├── Map doc entities to chunks by position overlap
  │           ├── For each chunk: filter low-confidence, UPSERT entity nodes
  │           ├── For each chunk: CREATE CONTAINS edges (doc → entity)
  │           ├── For each chunk: CREATE COOCCURS edges (entity pairs)
  │           └── Graph is ready for retrieval
  │
  └── Store document + chunks in SQLite
```

## 8. File Change Summary

| File | Changes |
|------|---------|
| `crates/xberg-rag/Cargo.toml` | Add `sqlite-graph` and `pipeline-ner-candle` features |
| `crates/xberg-rag/src/backends/entity_graph.rs` | **NEW** — standalone graph functions taking `&Connection` |
| `crates/xberg-rag/src/backends/sqlite.rs` | Add `graph_enabled: bool` field; call `graph_init_schema()` in `open()`; call `build_entity_graph()` inside `upsert_document()` after chunks are inserted; implement `RetrieveMode::Graph` in `retrieve()` |
| `crates/xberg-rag/src/backends/mod.rs` | Feature-gate `entity_graph` module |
| `crates/xberg-rag/src/types.rs` | Add `PrimaryScore::Graph` and `PrimaryScore::HybridGraph` variants |
| `crates/xberg-rag/src/query.rs` | Add `graph_seed_ids`, `graph_edge_labels`, `graph_min_confidence` fields |
| `crates/xberg-rag/src/capability.rs` | Add `graph: bool` field |
| `crates/xberg-rag/src/pipeline.rs` | Add `build_entity_graph()` function (accepts `&Connection`, `document_id`, `&[ChunkRecord]`, `&serde_json::Value`, optional NER backend) |
| `crates/xberg-rag/src/error.rs` | (no changes needed) |
| `crates/xberg-rag-node/src/lib.rs` | Add graph traversal/scoring NAPI methods |
| `mcp-server/src/tools/ingest.ts` | Add NER params to `ingest_document`, wire NER extraction |
| `mcp-server/src/tools/query.ts` | Add graph query params to `query_corpus` |

**Ingestion call chain:**
```
MCP ingest_document()
  → @xberg-io/xberg extract() [NER runs here]
  → NAPI-RS store.upsertDocument(collection, documentJson, chunksJson)
    → SqliteVectorStore::upsert_document() [sqlite.rs]
      → insert chunks into chunks/vec/fts tables
      → if graph_enabled:
          → build_entity_graph(conn, doc_id, &chunks, &entities, ner_backend)
            → for each chunk: map entities by position overlap
            → for each chunk: UPSERT entity nodes, CREATE CONTAINS edges
            → for each chunk: CREATE COOCCURS edges
```

## 9. Testing Strategy

- Unit tests for `entity_graph.rs` functions (standalone, in-memory SQLite)
- Unit tests for `normalize_entity()` with all EntityCategory variants including Custom
- Unit tests for `confidence_tier()` with Some/None and boundary values
- Integration test: ingest document with NER → verify graph nodes and edges created
- Integration test: graph retrieval with explicit seeds → verify correct documents returned
- Integration test: graph retrieval with auto-seed → verify vector/FTS → graph pipeline
- Integration test: document deletion → verify CASCADE cleanup
- Integration test: re-ingestion → verify idempotent graph updates
- E2E test: MCP server ingest_document with use_ner=true → query_corpus with mode=graph

## 10. Future Extensions

- **Coreference resolution:** Merge medium-tier entities with high-tier using embedding similarity
- **Entity embeddings:** Generate embeddings for entity nodes, enable semantic entity search
- **Relationship extraction:** Use LLM to extract entity-to-entity relationships from text (beyond co-occurrence)
- **Community detection:** Use Louvain on the entity graph to find entity clusters
- **Temporal graph:** Track entity appearance over time, detect emerging topics
