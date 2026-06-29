# Xberg MCP RAG GDPR-Compliant Design Spec

## 1. Executive Summary

A Node.js MCP server wrapping xberg's extraction, adding GDPR-compliant PII detection with rehydration, high-precision BGE-M3 embedding (1024-dim, 100+ languages) + reranking, and sqlite-vec + graphqlite RAG corpus search — targeting a 12GB laptop, for use with Claude Desktop as the AI client.

## 2. Constraints & Requirements

| Constraint | Value |
|------------|-------|
| Target RAM | 12GB laptop (~3GB for all models) |
| EU/GDPR | Full compliance (Art. 5, 6, 9, 17, 25, 30, 32) |
| Embedder | BAAI/bge-m3 (1024-dim, ONNX, 100+ languages) |
| Reranker | BAAI/bge-reranker-base (cross-encoder, ONNX) |
| PII Detection | fastino GLiNER2-PII (42 types, 7 languages) |
| RAG Backend | sqlite-vec + graphqlite (embedded, single-file) |
| Transports | stdio (Claude Desktop) + HTTP (Streamable HTTP) |
| Ingest Modes | One-shot + continuous watch |

## 3. Memory Budget (12GB)

| Component | Est. RAM | Notes |
|-----------|----------|-------|
| Tesseract OCR | ~50MB | Thread-local cached, 1 instance per thread |
| BGE-M3 ONNX | ~800MB | 1024-dim, 100+ languages, 8192 context |
| bge-reranker-base | ~280MB | Cross-encoder reranker (BAAI/bge-reranker-base) |
| GLiNER2-PII (okasi Q8) | ~510MB | 42 PII types, 7 languages |
| sqlite-vec DB | ~100MB | Embedded, file-backed |
| graphqlite graph | ~50-100MB | Graph nodes/edges, algorithms |
| Node.js process | ~200MB | MCP server + NAPI-RS bindings |
| **Total** | **~1.94GB** | Leaves 10GB+ for OS + Claude Desktop (ONNX Runtime adds ~300-500MB overhead) |

### 3.1 Model Loading Strategy

- **Lazy load** on first use, not at startup
- **LRU cache** with configurable max (default: all 3 models stay resident after first load)
- **Graceful fallback** if model download fails (offline mode with pattern-only PII)
- **BGE-M3 download**: 2.29 GB ONNX file — downloads once, cached in xberg model cache

### 3.2 First-Run Warmup System

When the MCP server starts for the first time, users must not be surprised by silent downloads. The warmup system provides clear feedback:

#### Startup Flow

```
MCP Server Start
    │
    ├─ Check cache directory for required models
    │
    ├─ IF models missing:
    │   │
    │   ├─ Send MCP notification: "First-time setup: Downloading models..."
    │   │
    │   ├─ Download BGE-M3 (2.29 GB) ──────── Progress: [████████░░] 80%
    │   ├─ Download bge-reranker (280 MB) ─── Progress: [██████████] 100%
    │   ├─ Download GLiNER2-PII (510 MB) ─── Progress: [██████░░░░] 60%
    │   │
    │   └─ Send MCP notification: "Setup complete! Ready to process documents."
    │
    └─ IF models cached:
        └─ Send MCP notification: "Xberg MCP ready. Models loaded from cache."
```

#### MCP Notifications (Sent to Claude Desktop/Codex)

```json
// First tool call triggers lazy init
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "data": {
      "progressToken": "xberg-warmup",
      "message": "First-time setup: Downloading embedding model (BGE-M3, 2.29 GB)..."
    }
  }
}

// Progress updates during download
{
  "method": "notifications/progress",
  "params": {
    "progressToken": "xberg-warmup",
    "progress": 1800,
    "total": 2290,
    "message": "Downloading BGE-M3: 1.8 GB / 2.3 GB"
  }
}

// Completion
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "data": {
      "message": "Setup complete! All models downloaded and cached."
    }
  }
}
```

#### User Experience by Scenario

| Scenario | What User Sees | Time |
|----------|----------------|------|
| **First run (no cache)** | Progress notifications in Claude Desktop | 2-5 min (depends on internet) |
| **Subsequent runs** | "Models loaded from cache" message | <1 sec |
| **Partial cache** | Downloads only missing models | 30 sec - 2 min |
| **Offline (no cache)** | Error: "Models not available offline. Run `cache_warm` when online." | Immediate |
| **Manual warmup** | User calls `cache_warm` tool before first use | 2-5 min |

#### MCP Tool: `cache_warm`

```json
{
  "name": "cache_warm",
  "description": "Download and cache all required models for offline use",
  "inputSchema": {
    "type": "object",
    "properties": {
      "embedding": { "type": "boolean", "default": true, "description": "Download BGE-M3 embedding model" },
      "reranker": { "type": "boolean", "default": true, "description": "Download bge-reranker-base" },
      "ner": { "type": "boolean", "default": true, "description": "Download GLiNER2-PII model" }
    }
  }
}
```

#### Implementation Notes

1. **First-run detection**: Check if `$CACHE_DIR/embeddings/BAAI--bge-m3/model.onnx` exists
2. **Progress reporting**: Use `hf_hub`'s built-in `.with_progress(true)` + MCP progress notifications
3. **Non-blocking startup**: Server starts immediately, warmup happens on first tool call
4. **Graceful degradation**: If download fails, tools return error with instructions to run `cache_warm`
5. **Windows note**: No cross-process lock on Windows (falls back to hf-hub's own lock)

## 4. Architecture

### 4.1 Component Stack

```
┌─────────────────────────────────────────────────┐
│ Claude Desktop (stdio)  │  Web/Remote (HTTP)    │
└─────────┬───────────────┴──────────┬────────────┘
          │                          │
┌─────────▼──────────────────────────▼────────────┐
│            Node.js MCP Server                    │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐ │
│  │ stdio       │  │ HTTP         │  │ Tools  │ │
│  │ transport   │  │ transport    │  │ 22+    │ │
│  └──────┬──────┘  └──────┬───────┘  └───┬────┘ │
│         └────────┬───────┘              │       │
│                  ▼                      ▼       │
│  ┌──────────────────────────────────────────┐   │
│  │         xberg-node NAPI-RS Bindings      │   │
│  │  extract() / extractBatch() / NER / OCR  │   │
│  └──────────────────────────────────────────┘   │
│                  │                               │
│  ┌──────────────────────────────────────────┐   │
│  │      xberg-rag NAPI-RS Extension (NEW)   │   │
│  │  SQLiteVectorStore / IngestPipeline /    │   │
│  │  Embedder / Reranker / Retriever /       │   │
│  │  GraphIngest / GraphRetrieval            │   │
│  └──────────────────────────────────────────┘   │
│                  │                               │
│  ┌──────────────────────────────────────────┐   │
│  │      PII Detection & Rehydration         │   │
│  │  GLiNER2-PII ONNX + Patterns + Maps     │   │
│  └──────────────────────────────────────────┘   │
└──────────────────────────────────────────────────┘
          │                          │
┌─────────▼──────────────────────────▼────────────┐
│  SQLite + sqlite-vec + graphqlite               │
│  (single file per corpus)                       │
│  collections │ documents │ chunks │ vec_c{N}    │
│  nodes │ edges │ properties │ graph_traversal   │
└──────────────────────────────────────────────────┘
```

### 4.2 Data Flow

#### Ingest Flow (One-shot or Watch)
```
Source File → xberg::extract() → PII Detection → Redaction → 
Chunking (semantic/text) → Embedding (BGE-M3 1024d) → 
SQLite-vec upsert (document + chunks + metadata)
Graph upsert (nodes + edges + relationships)
Redacted copy → redacted_folder/ (DOCX/PDF/TXT)
```

#### Query Flow (4 Modes)
```
User Query → Embedding (BGE-M3) → 

Mode "vector":    sqlite-vec vector search → Rerank → Top-K → Response
Mode "full_text": FTS5 keyword search → Rerank → Top-K → Response
Mode "hybrid":    sqlite-vec + FTS5 + RRF → Rerank → Top-K → Response
Mode "graph":     sqlite-vec (seeds) → Cypher traversal → Louvain communities → 
                  Combine + dedup → Rerank → Top-K → Response
```

#### First-Run Flow (Warmup Notifications)
```
MCP Server Start → First Tool Call → Check Cache → 
IF missing: Download with progress → MCP notifications → Ready
IF cached: "Models loaded from cache" → Ready
```

## 5. SQLite-vec RAG Schema

Based on xberg-rag's existing `BASE_SCHEMA` (`sqlite.rs:149-191`):

```sql
-- Core tables (created once at DB open)
CREATE TABLE IF NOT EXISTS collections (
    name           TEXT PRIMARY KEY,
    embedding_dim  INTEGER NOT NULL,
    distance_metric TEXT NOT NULL DEFAULT 'cosine',
    index_method   TEXT NOT NULL DEFAULT 'flat'
) STRICT;

CREATE TABLE IF NOT EXISTS documents (
    id          TEXT PRIMARY KEY,
    collection  TEXT NOT NULL,
    external_id TEXT,                    -- blake3 content hash
    title       TEXT,
    mime        TEXT,
    source_uri  TEXT,                    -- original file path
    full_text   TEXT NOT NULL DEFAULT '', -- redacted text
    keywords    TEXT NOT NULL DEFAULT '[]',
    entities    TEXT NOT NULL DEFAULT 'null',
    labels      TEXT NOT NULL DEFAULT 'null',
    metadata    TEXT NOT NULL DEFAULT 'null', -- format-specific metadata JSON
    ingested_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_docs_ext
    ON documents(collection, external_id)
    WHERE external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS chunks (
    id             TEXT PRIMARY KEY,
    document_id    TEXT NOT NULL,
    collection     TEXT NOT NULL,
    ordinal        INTEGER NOT NULL,
    external_id    TEXT,
    content        TEXT NOT NULL,          -- redacted chunk text
    embedding      BLOB NOT NULL,          -- 1024-dim f32 LE bytes (BGE-M3)
    chunk_metadata TEXT NOT NULL DEFAULT 'null' -- heading_path, page numbers
) STRICT;

-- Per-collection virtual tables (created at ensure_collection)
-- vec_c{rowid}: sqlite-vec vector index
-- fts_c{rowid}: FTS5 full-text index
```

### 5.1 GraphQLite Graph Schema

GraphQLite creates its own internal tables for graph storage. The graph schema (created during ingest):

```cypher
// Node labels
(:Document {id, title, mime, source_uri, ingested_at})
(:Chunk {id, ordinal, content, heading_path})
(:Entity {id, name, type})  // Person, Organization, Location, etc.
(:Concept {id, name})       // Extracted concepts/topics

// Edge types
(:Document)-[:HAS_CHUNK {ordinal}]->(:Chunk)
(:Chunk)-[:MENTIONS]->(:Entity)
(:Document)-[:COOCCURS {chunk_count}]->(:Document)  // Same context
(:Entity)-[:RELATED_TO {strength}]->(:Entity)        // Co-occurrence
(:Document)-[:BELONGS_TO]->(:Concept)                // Topic membership
```

**GraphQLite internal storage:**
- Nodes stored in `_graph_nodes` table with typed properties (text, integer, real, boolean, json)
- Edges stored in `_graph_edges` table with source/target node IDs
- Properties stored in EAV model for flexible schema
- All operations go through Cypher query engine

### 5.2 Document Metadata Storage

The `metadata` JSON column in `documents` stores format-specific metadata as `serde_json::Value`:

```json
{
  "format": "pdf",
  "page_count": 42,
  "title": "Contract Draft",
  "author": "Legal Team",
  "created_at": "2026-01-15T10:30:00Z",
  "language": "en",
  "has_forms": false,
  "has_images": true,
  "tables_detected": 3,
  "ocr_applied": true,
  "pii_entities_detected": 12,
  "pii_tokens_generated": 47
}
```

### 5.3 Graph Ingestion Pipeline

When ingesting a document, graph nodes and edges are created alongside vector ingestion:

```
1. Document extraction (existing)
   └── xberg::extract() → text + metadata + entities

2. Vector ingest (existing)
   └── chunking → embedding → sqlite-vec upsert

3. Graph ingest (NEW, internal)
   ├── Create Document node: (:Document {id, title, mime, source_uri})
   ├── Create Chunk nodes: (:Chunk {id, ordinal, content, heading_path})
   ├── Create HAS_CHUNK edges: (Document)-[:HAS_CHUNK]->(Chunk)
   ├── Create Entity nodes: (:Entity {id, name, type}) from NER results
   ├── Create MENTIONS edges: (Chunk)-[:MENTIONS]->(Entity)
   ├── Create COOCCURS edges: (Document)-[:COOCCURS]-(Document) for same-context docs
   └── Create BELONGS_TO edges: (Document)-[:BELONGS_TO]->(Concept) from topic extraction
```

**Entity extraction source:** NER results from fastino GLiNER2-PII or xberg's built-in NER (Person, Organization, Location, Date, etc.)

**COOCCURS edge computation:** Two documents get a COOCCURS edge if they appear in the same ingestion batch (e.g., same folder, same archive) or share entity nodes.

**Concept extraction:** Optional — can use keyword extraction (xberg's `keywords::extract_keywords`) or topic modeling to create Concept nodes.

### 5.4 Extension Loading Order

Both sqlite-vec and graphqlite must be loaded on the same SQLite connection:

```rust
// 1. Open rusqlite connection
let conn = Connection::open(db_path)?;

// 2. Load sqlite-vec extension (virtual table module)
sqlite_vec::load(&conn)?;

// 3. Wrap with graphqlite Connection (loads graph extension)
let graph_conn = graphqlite::Connection::from_rusqlite(conn)?;

// 4. Both extensions now available on same connection
// - sqlite-vec: vec0 virtual tables for vector search
// - graphqlite: Cypher queries + graph algorithms
```

### 5.5 Chunk Metadata

Each chunk stores:
- `heading_path`: hierarchical heading context (e.g., `["§2", "2.1", "2.1.3"]`)
- `page_numbers`: source page(s) for PDFs
- `byte_range`: `[start, end]` in source document
- `chunk_index`: position within document
- `total_chunks`: total chunks in document
- `image_indices`: referenced image indices (for multimodal)

## 6. PII Detection & GDPR Compliance

### 6.1 Detection Layers

| Layer | Engine | Purpose |
|-------|--------|---------|
| 1. Pattern Engine | 8 pure-Rust regex | Email, phone, SSN, credit card, postal code, IP, IBAN, SWIFT/BIC |
| 2. NER Engine | fastino GLiNER2-PII ONNX | 42 PII types, 7 languages (EN, FR, ES, DE, IT, PT, NL) |
| 3. Custom Rules | User-defined regex | Domain-specific patterns |

### 6.2 PII Types (fastino GLiNER2-PII)

42 categories including:
- **Person**: names, aliases, nicknames
- **Organization**: company names, government bodies
- **Location**: addresses, cities, countries
- **Financial**: credit cards, IBAN, SWIFT, tax IDs
- **Health**: medical record numbers, insurance IDs
- **Contact**: emails, phone numbers, URLs
- **Identity**: SSN, passport numbers, driver licenses
- **Dates/Times**: birth dates, appointment times

### 6.3 Redaction Strategy: TokenReplace (Default)

**Required for GDPR Art. 25 (Data Protection by Design):**

```json
{
  "strategy": "token_replace",
  "token_format": "[{category}_{index}]",
  "stable_ids": true,
  "category_index": {
    "PERSON": 1,
    "EMAIL": 1,
    "PHONE": 1
  }
}
```

**Before redaction:**
> "John Smith sent email to jane.doe@company.com on 2026-01-15 regarding the invoice."

**After redaction:**
> "[PERSON_1] sent email to [EMAIL_1] on 2026-01-15 regarding the invoice."

**Benefits:**
- Stable IDs across chunks (same person → same `[PERSON_1]` everywhere)
- Preserves document structure for embeddings
- Enables rehydration via encrypted map

### 6.4 Rehydration (Option 3: Hybrid)

**Architecture:**
1. **Server-side encrypted map** stored in a separate SQLite table
2. **MCP tools** for authorized access (Claude Desktop can call tools)
3. **Encryption**: age or GPG (user's choice)
4. **Access control**: User must unlock map with passphrase

**Rehydration Table:**
```sql
CREATE TABLE IF NOT EXISTS rehydration_maps (
    document_id TEXT NOT NULL,
    category    TEXT NOT NULL,     -- PERSON, EMAIL, etc.
    token       TEXT NOT NULL,     -- [PERSON_1]
    original    BLOB NOT NULL,     -- encrypted original text
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (document_id, category, token)
) STRICT;
```

**MCP Tools for Rehydration:**
- `rehydrate_document` — decrypt and show full document
- `rehydrate_token` — decrypt single token
- `rehydrate_search` — find documents containing specific PII

### 6.5 GDPR Compliance Mapping

| Article | Requirement | Implementation |
|---------|-------------|----------------|
| Art. 5(1)(a) | Lawfulness, fairness, transparency | PII detection runs only on user-provided documents with explicit opt-in via MCP tool call |
| Art. 5(1)(b) | Purpose limitation | RAG corpus purpose declared at collection creation; documents indexed only for declared purpose |
| Art. 5(1)(f) | Integrity & confidentiality | Encrypted rehydration maps, access-controlled |
| Art. 6(1)(a) | Consent | User explicitly enables PII detection per collection |
| Art. 9 | Special categories | Health/financial PII types detected and redacted |
| Art. 17 | Right to erasure | `delete_by_filter` on VectorStore, rehydration map cleanup |
| Art. 25 | Privacy by design | TokenReplace default, stable IDs, minimal PII exposure |
| Art. 30 | Records of processing | Audit trail in RedactionEngine, ingestion timestamps |
| Art. 32 | Security of processing | age/GPG encryption, passphrase-protected maps |

**⚠️ Art. 17 Performance Note:** `delete_by_filter` loads all documents in the collection and evaluates the filter against each one. For large collections (10k+ documents), this could be slow. Consider SQL-side filter evaluation for common cases as an optimization.

**⚠️ Graph Cleanup:** When a document is deleted via `delete_document` or `delete_by_filter`, associated graph nodes and edges must also be removed:
- Delete `:Document` node and all `:HAS_CHUNK` edges
- Delete orphaned `:Chunk` nodes (no other document references them)
- Delete orphaned `:Entity` nodes (no other chunks mention them)
- Delete `:COOCCURS` edges referencing deleted document

### 6.6 Audit Trail

Every redaction operation generates an audit record:
```json
{
  "timestamp": "2026-01-15T10:30:00Z",
  "document_id": "abc123",
  "source_uri": "/docs/contract.pdf",
  "strategy": "token_replace",
  "entities_detected": 12,
  "tokens_generated": 47,
  "categories": {
    "PERSON": 3,
    "EMAIL": 4,
    "ORGANIZATION": 2,
    "LOCATION": 1,
    "DATE": 2
  }
}
```

## 7. Embedder: BAAI/bge-m3

### 7.1 Why BGE-M3

| Metric | BGE-base-en-v1.5 | BGE-M3 | Winner |
|--------|------------------|--------|--------|
| Dimensions | 768 | 1024 | BGE-M3 (1.3x precision) |
| Languages | English only | 100+ (FR, DE, ES, IT, NL, PT...) | **BGE-M3** |
| Context Length | 512 tokens | 8192 tokens | **BGE-M3** (16x longer) |
| Sparse Retrieval | No | Yes (built-in) | **BGE-M3** |
| ColBERT | No | Yes (built-in) | **BGE-M3** |
| Model Size (ONNX) | ~500 MB | 2.29 GB | BGE-base |
| MTEB (English) | 64.2 | 64.8 | BGE-M3 |
| MIRACL (Multilingual) | N/A | 68.1 | **BGE-M3** |

**Decision:** BGE-M3 selected for:
- French and European language support (EU project requirement)
- 8192 token context for long legal documents
- Built-in sparse retrieval for hybrid search
- Top multilingual performance (surpasses OpenAI embeddings)

### 7.2 Embedding Configuration

```json
{
  "model_type": "custom",
  "model_id": "BAAI/bge-m3",
  "dimensions": 1024,
  "pooling": "cls",
  "normalize": true,
  "batch_size": 16,
  "max_tokens": 8192,
  "model_file": "onnx/model.onnx",
  "tokenizer_file": "onnx/tokenizer.json"
}
```

### 7.3 Embedding Pipeline

```
Chunk Text → Tokenize (8192 max) → ONNX Inference → CLS Pooling → 
L2 Normalize → Store as BLOB (1024 × 4 bytes = 4096 bytes)
```

### 7.4 Concurrency Control

```rust
// From xberg embeddings engine
static EMBED_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(thread_budget())  // min(cpus, 8)
});
```

- Caps concurrent ONNX inference to `thread_budget`
- Prevents OOM on 12GB laptop
- All embedding calls go through semaphore

### 7.5 Multilingual Support

BGE-M3 supports 100+ languages including:
- **French** (fr) — primary requirement
- **German** (de)
- **Spanish** (es)
- **Italian** (it)
- **Dutch** (nl)
- **Portuguese** (pt)
- **English** (en)
- And 93+ more languages

No language detection needed — BGE-M3 handles multilingual input natively.

## 8. Reranker: BAAI/bge-reranker-base

### 8.1 Cross-Encoder Reranking

**Feature flags:**
- `xberg/Cargo.toml`: `reranker` (enables ONNX Runtime for reranking)
- `xberg-rag/Cargo.toml`: `pipeline-reranker` (enables `xberg/reranker`)

### 8.2 Reranking Pipeline

```
Query Embedding + Candidate Chunks → Cross-Encoder Inference → 
Score Each (query, chunk) Pair → Sort by Score → Return Top-K
```

### 8.3 Reranker Configuration

```json
{
  "model": "BAAI/bge-reranker-base",
  "top_k": 10,
  "candidate_multiplier": 5,
  "max_query_length": 256,
  "max_doc_length": 512
}
```

### 8.4 Retrieval Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `vector` | Pure vector similarity | Semantic search |
| `full_text` | FTS5 keyword search | Exact term matching |
| `hybrid` | Vector + FTS5 fused via RRF | Best precision + recall |
| `graph` | Vector seeds + Cypher traversal + Louvain communities + rerank | Multi-hop reasoning, contextual retrieval |

**Default:** `hybrid` mode for highest precision.

**Reranking:** Applied as a post-retrieval step (not a query parameter). The MCP server calls `rerank()` after `retrieve()` when the user requests reranking. The `candidate_multiplier` parameter in `RetrieveQuery` pulls extra candidates before reranking.

### 8.5 Graph Mode Internals

When `mode: "graph"` is selected, the `query_corpus` tool internally executes:

```
STEP 1: Vector Search (sqlite-vec)
    Find top-k similar documents by embedding distance → seed documents

STEP 2: Graph Traversal (Cypher)
    For each seed document, traverse relationships to find connected documents
    MATCH (d:Document)-[:COOCCURS]-(related:Document) WHERE d.id IN $seed_ids

STEP 3: Community Detection (Louvain via Rust API)
    Call g.louvain() via graphqlite Rust binding to find topic clusters
    Filter to communities containing seed documents

STEP 4: Combined Context
    Merge all retrieved documents (vector + graph + community), deduplicate

STEP 5: Reranking (BGE-reranker)
    Cross-encoder precision on combined candidate set

STEP 6: Return Top-K
    Final ranked results
```

**Graph Traversal Query (Cypher):**
```cypher
MATCH (d:Document)-[:HAS_CHUNK]->(c:Chunk)
WHERE d.id IN $seed_ids
MATCH (d)-[:COOCCURS]-(related:Document)
RETURN related.id, related.title, count(*) AS connection_strength
ORDER BY connection_strength DESC
LIMIT $limit
```

**Community Detection (Rust API, not Cypher):**
```rust
// Called via graphqlite Rust binding, not Cypher
let communities = graph.louvain(1.0);  // resolution=1.0
// Filter to communities containing seed document IDs
let relevant_communities = communities.filter_by_seeds(&seed_ids);
```

### 8.6 GraphQLite Algorithms Available

| Algorithm | Purpose | When Used |
|-----------|---------|-----------|
| PageRank | Document importance scoring | Optional post-processing |
| Louvain | Community detection | Graph mode, step 3 |
| Dijkstra | Shortest path between entities | Entity relationship queries |
| BFS/DFS | Graph traversal | Custom traversal patterns |
| Connected Components | Cluster detection | Corpus analysis |
| Betweenness Centrality | Bridge document identification | Importance ranking |

## 9. MCP Tool Specification

### 9.1 Core Tools (27+)

| Tool | Description |
|------|-------------|
| `extract_document` | Extract text + metadata from file |
| `extract_batch` | Extract multiple files in parallel |
| `create_collection` | Create RAG corpus with embedding config |
| `ingest_document` | Chunk, embed, store document |
| `ingest_folder` | Watch/ingest folder, create redacted copies, vectorize |
| `query_corpus` | Search with vector/fulltext/hybrid + rerank |
| `delete_document` | Remove document + chunks (Art. 17) |
| `delete_by_filter` | Bulk delete matching filter |
| `get_document` | Retrieve document + metadata |
| `list_collections` | List all collections |
| `list_documents` | List documents in collection |
| `rehydrate_document` | Decrypt full document (requires passphrase) |
| `rehydrate_token` | Decrypt single PII token |
| `rehydrate_search` | Find documents containing specific PII |
| `get_audit_log` | View PII detection audit trail |
| `detect_pii` | Run PII detection on text |
| `redact_text` | Redact PII from text |
| `cache_warm` | Download and cache all models for offline use |
| `get_extraction_stats` | View extraction metrics |
| `export_collection` | Export collection as JSON/JSONL |
| `import_collection` | Import from JSON/JSONL |
| `update_metadata` | Update document metadata |
| `collection_stats` | View collection statistics (includes graph node/edge counts) |
| `get_ingestion_summary` | Get summary of all redacted files in a collection |
| `get_document_report` | Get PII report for a specific document |
| `explain_reports` | Explain how to use redaction reports and redacted files |

### 9.2 Tool Parameters (Example: `query_corpus`)

```json
{
  "name": "query_corpus",
  "description": "Search a RAG corpus with vector, full-text, hybrid, or graph retrieval + reranking",
  "inputSchema": {
    "type": "object",
    "properties": {
      "collection": { "type": "string", "description": "Collection name" },
      "query": { "type": "string", "description": "Search query" },
      "mode": { "type": "string", "enum": ["vector", "full_text", "hybrid", "graph"], "default": "hybrid" },
      "top_k": { "type": "integer", "default": 10, "maximum": 200 },
      "candidate_multiplier": { "type": "integer", "default": 5, "description": "Over-fetch before rerank" },
      "graph_depth": { "type": "integer", "default": 2, "description": "Graph traversal depth (graph mode only)" },
      "filters": { "type": "object", "description": "Filter by metadata fields" },
      "include_metadata": { "type": "boolean", "default": true }
    },
    "required": ["collection", "query"]
  }
}
```

**Note:** Reranking is applied server-side after retrieval when `candidate_multiplier > 1`. The tool automatically reranks if candidates exceed `top_k`. Graph mode adds `graph_depth` parameter to control traversal depth.

### 9.3 Tool Parameters (Example: `ingest_folder`)

```json
{
  "name": "ingest_folder",
  "description": "Watch/ingest a folder: extract, redact, create redacted copies, vectorize",
  "inputSchema": {
    "type": "object",
    "properties": {
      "source_folder": { "type": "string", "description": "Path to original files" },
      "redacted_folder": { "type": "string", "description": "Path for redacted copies" },
      "collection": { "type": "string", "description": "RAG collection name" },
      "mode": { "type": "string", "enum": ["one-shot", "watch"], "default": "one-shot" },
      "redaction_strategy": { "type": "string", "enum": ["token_replace", "mask", "hash"], "default": "token_replace" },
      "preserve_structure": { "type": "boolean", "default": true, "description": "Keep original formatting in redacted files" }
    },
    "required": ["source_folder", "redacted_folder", "collection"]
  }
}
```

## 10. NAPI-RS Extension Plan

### 10.1 What Needs Extending

xberg-node currently exposes:
- `extract()` / `extractBatch()` — document extraction
- NER types, OCR config, chunking config

**Missing from Node.js bindings:**
- `xberg-rag` SQLiteVectorStore
- `ingest_document()` / `retrieve()` pipeline
- Embedder / Reranker
- Rehydration maps
- GraphQLite graph operations (internal to query pipeline)

### 10.2 NAPI-RS Functions to Add

```rust
// crates/xberg-node/src/rag.rs (NEW)

#[napi]
pub fn create_vector_store(db_path: String) -> RagStore { ... }

#[napi]
pub fn ensure_collection(store: RagStore, spec: CollectionSpecJs) { ... }

#[napi]
pub fn drop_collection(store: RagStore, collection: String) { ... }

#[napi]
pub fn get_collection(store: RagStore, collection: String) -> Option<CollectionSpecJs> { ... }

#[napi]
pub fn ingest_document(store: RagStore, collection: String, request: IngestRequestJs) -> String { ... }

#[napi]
pub fn query_corpus(store: RagStore, collection: String, query: RetrieveQueryJs) -> Vec<RetrievedChunkJs> { ... }

#[napi]
pub fn get_document(store: RagStore, collection: String, doc_id: String) -> Option<DocumentRecordJs> { ... }

#[napi]
pub fn delete_document(store: RagStore, collection: String, doc_id: String) -> u64 { ... }

#[napi]
pub fn delete_by_filter(store: RagStore, collection: String, filter: FilterJs) -> u64 { ... }

#[napi]
pub fn collection_stats(store: RagStore, collection: String) -> CollectionStatsJs { ... }

#[napi]
pub fn rehydrate_token(store: RagStore, document_id: String, token: String, passphrase: String) -> String { ... }

// ─── Internal graph functions (called by query_corpus when mode="graph") ───
// These are NOT exposed as MCP tools — they're internal to the query pipeline

/// Traverse graph from seed document IDs, return connected document IDs
fn graph_traverse_seeds(
    conn: &Connection,           // graphqlite Connection wrapping rusqlite
    seed_ids: &[String],
    depth: u32,
) -> RagResult<Vec<String>>

/// Run Louvain community detection, return community assignments
fn graph_communities(
    conn: &Connection,
) -> RagResult<Vec<Community>>

/// Filter communities to those containing seed document IDs
fn filter_communities_by_seeds(
    communities: &[Community],
    seed_ids: &[String],
) -> RagResult<Vec<Community>>
```

### 10.3 TypeScript Declarations

```typescript
// index.d.ts additions

export interface VectorStore {
  ensureCollection(spec: CollectionSpec): void;
  ingestDocument(collection: string, request: IngestRequest): string;
  queryCorpus(collection: string, query: RetrieveQuery): RetrievedChunk[];
  deleteDocument(collection: string, docId: string): number;
  rehydrateToken(documentId: string, token: string, passphrase: string): string;
}

export interface CollectionSpec {
  name: string;
  embeddingDim: number;  // 1024 for BGE-M3
  distanceMetric?: 'cosine' | 'l2';
  indexMethod?: 'flat' | 'hnsw';
}

export interface IngestRequest {
  fullText: string;
  title?: string;
  mime?: string;
  sourceUri?: string;
  externalId?: string;
  keywords?: string[];
  metadata?: Record<string, unknown>;
}

export interface RetrieveQuery {
  mode?: 'vector' | 'full_text' | 'hybrid' | 'graph';
  queryText?: string;
  queryVector?: number[];
  topK: number;
  filter?: Filter;
  candidateMultiplier?: number;
  graphDepth?: number;  // graph mode only
  includeContent?: boolean;
  includeDocument?: boolean;
}
```

## 11. Fastino GLiNER2-PII Integration

### 11.1 Model Registry

Add to xberg's `KNOWN_MODELS` (`gline.rs`):

```rust
GlinerModelDefinition {
    id: "okasi/gliner2-privacy-filter-pii-multi-onnx",
    aliases: &[],
    upstream_repo: "okasi/gliner2-privacy-filter-pii-multi-onnx",
    mode: "span",
    variant: "pii",
    model_file: "model.onnx",           // single-file ONNX
    tokenizer_file: "tokenizer.json",   // REQUIRED — must verify HF repo has this
}
```

**⚠️ Compatibility Note:** All existing xberg GLiNER models use separate `tokenizer.json` files. The okasi model's compatibility depends on whether it includes a tokenizer file in the repo. If not, may need to use a different model variant or extract tokenizer from the ONNX export.

### 11.2 PII Labels

**⚠️ Verification Required:** The fastino model's actual label set must be verified against the HuggingFace repo's `config.json` or label mappings. The xberg `PiiCategory` enum has 13 categories:

```rust
// From xberg types/redaction.rs
pub enum PiiCategory {
    Email, Phone, Ssn, CreditCard, PostalCode, IpAddress,
    Iban, SwiftBic, DateOfBirth, Person, Organization, Location, Custom
}
```

The GLiNER2-PII model may use different label names (e.g., `person_name` vs `Person`). The MCP server should map between model labels and xberg's `PiiCategory` enum.

### 11.3 Configuration

```json
{
  "ner": {
    "provider": "gliner",
    "model": "okasi/gliner2-privacy-filter-pii-multi-onnx",
    "threshold": 0.5,
    "labels": "pii_all",
    "languages": ["en", "fr", "es", "de", "it", "pt", "nl"]
  }
}
```

## 12. Optimization Strategies

### 12.1 Extraction Optimization

| Technique | Impact | Status |
|-----------|--------|--------|
| Content-hash dedup | Skip re-extraction of unchanged files | Implemented (blake3) |
| Batch extraction | Parallel processing with semaphore | Implemented |
| Streaming PDF parse | Reduce memory for large PDFs | Verify xberg support |
| Archive streaming | ZIP/TAR without full extraction | Verify xberg support |

### 12.2 Embedding Optimization

| Technique | Impact | Status |
|-----------|--------|--------|
| Batch embedding | Process 32 chunks at once | Implemented |
| Semaphore cap | Limit concurrent ONNX inference | Implemented |
| CLS pooling | Faster than mean pooling | Implemented |
| L2 normalize | Pre-normalize for cosine search | Implemented |

### 12.3 Retrieval Optimization

| Technique | Impact | Status |
|-----------|--------|--------|
| FTS5 index | Fast keyword search | Implemented |
| sqlite-vec | Embedded vector search | Implemented |
| Hybrid RRF | Vector + text fusion | Implemented |
| Candidate multiplier | Over-fetch before rerank | Implemented |
| Document grouping | Best chunk per doc | Implemented |

### 12.4 Storage Optimization

| Technique | Impact | Status |
|-----------|--------|--------|
| WAL mode | Concurrent reads | Implemented |
| STRICT tables | Type safety | Implemented |
| Partial indexes | Unique external_id | Implemented |
| BLOB embeddings | 1024×4=4096 bytes/chunk | Implemented |

## 13. Implementation Phases

### Phase 1: NAPI-RS Extension (Week 1-2)
- [ ] Add `rag.rs` to xberg-node
- [ ] Expose SQLiteVectorStore
- [ ] Expose ingest pipeline
- [ ] Expose query pipeline
- [ ] TypeScript declarations
- [ ] Add graphqlite dependency to Cargo.toml

### Phase 2: MCP Server (Week 3-4)
- [ ] Create Node.js MCP server
- [ ] Implement stdio transport
- [ ] Implement HTTP transport
- [ ] Wire all 27+ tools
- [ ] Implement graph mode in query_corpus tool
- [ ] Implement first-run warmup with MCP progress notifications
- [ ] Implement report access tools (get_ingestion_summary, get_document_report, explain_reports)
- [ ] Implement SUMMARY_REPORT.docx generation

### Phase 3: PII & GDPR (Week 5-6)
- [ ] Integrate fastino GLiNER2-PII (verify tokenizer.json in repo)
- [ ] Implement TokenReplace redaction
- [ ] Build rehydration maps (encrypted)
- [ ] MCP tools for rehydration
- [ ] Map GLiNER labels → xberg PiiCategory enum
- [ ] Implement redacted file output (docx + pdf-lib)
- [ ] Implement `ingest_folder` tool with redacted copy creation

### Phase 4: Optimization (Week 7-8)
- [ ] Verify streaming PDF/archive support in xberg
- [ ] Lazy model loading
- [ ] Memory profiling
- [ ] Performance benchmarks
- [ ] SQL-side filter optimization for Art. 17

### Phase 5: Testing & Docs (Week 9-10)
- [ ] Unit tests for all tools
- [ ] GDPR compliance audit
- [ ] Documentation
- [ ] Deployment guide

## 14. File Structure

```
xberg/
├── crates/
│   ├── xberg-node/
│   │   └── src/
│   │       ├── lib.rs          # existing NAPI bindings
│   │       └── rag.rs          # NEW: RAG NAPI bindings
│   ├── xberg-rag/
│   │   └── src/
│   │       ├── backends/sqlite.rs  # sqlite-vec backend
│   │       ├── pipeline.rs         # ingest/retrieve
│   │       ├── graph.rs            # TO CREATE: graphqlite integration
│   │       └── types.rs            # ChunkRecord, etc.
│   └── xberg-gliner/
│       └── src/engine.rs      # GLiNER ONNX inference
├── mcp-server/
│   ├── src/
│   │   ├── index.ts           # MCP server entry
│   │   ├── tools/             # 27+ tool implementations
│   │   │   ├── query_corpus.ts # graph mode implementation
│   │   │   ├── cache_warm.ts  # model download with progress
│   │   │   ├── ingest_folder.ts # folder ingestion + redacted output
│   │   │   ├── get_ingestion_summary.ts # collection summary
│   │   │   ├── get_document_report.ts # per-document report
│   │   │   └── explain_reports.ts # usage guide
│   │   ├── transports/        # stdio + HTTP
│   │   ├── warmup.ts          # first-run detection + MCP notifications
│   │   ├── redaction/         # PII detection + redaction
│   │   │   ├── detect.ts      # GLiNER2-PII + patterns
│   │   │   ├── redact.ts      # TokenReplace, Mask, Hash
│   │   │   └── output/        # redacted file generation
│   │   │       ├── docx.ts    # DOCX redaction via docx npm
│   │   │       ├── pdf.ts     # PDF redaction via pdf-lib
│   │   │       ├── text.ts    # TXT/MD/HTML redaction
│   │   │       └── report.ts  # DOCX report generation
│   │   └── pii/               # PII detection layer
│   ├── package.json
│   └── tsconfig.json
└── docs/
    └── superpowers/specs/
        └── xberg-mcp-rag-gdpr-spec.md  # THIS FILE
```

## 16. Redacted File Output

### 16.1 User Workflow

```
Source Folder (originals untouched)
    ├── contract.pdf
    ├── invoice.docx
    ├── report.xlsx
    └── notes.txt

                    ↓ xberg extract + PII detect + redact

Redacted Folder (ingested + vectorized)
    ├── contract_REDACTED.pdf
    ├── invoice_REDACTED.docx
    ├── report_REDACTED.xlsx
    └── notes_REDACTED.txt
```

**Key principles:**
- Original files **never modified**
- Redacted copies created in separate folder
- Redacted files are what get ingested into RAG corpus
- Rehydration maps link tokens back to originals

### 16.2 Supported Output Formats

| Input Format | Output Format | Library | Notes |
|--------------|---------------|---------|-------|
| .docx | .docx | `docx` (npm) | Preserves structure, tables, images |
| .pdf | .pdf | `pdf-lib` (npm) | Preserves layout, adds redaction annotations |
| .txt | .txt | Native | Simple text replacement |
| .md | .md | Native | Markdown-aware redaction |
| .html | .html | Native | Tag-preserving redaction |
| .xlsx | .xlsx | N/A | Not supported (skip file output, store in DB only) |

### 16.3 DOCX Redaction Implementation

Using `docx` npm package (https://www.npmjs.com/package/docx):

```typescript
import { Document, Packer, Paragraph, TextRun } from 'docx';
import * as fs from 'fs';

async function createRedactedDocx(
  originalPath: string,
  redactedPath: string,
  redactions: RedactionFinding[]
): Promise<void> {
  // 1. Read original DOCX
  const originalBuffer = fs.readFileSync(originalPath);
  
  // 2. Extract text and locate PII spans
  // 3. Replace PII with tokens while preserving formatting
  // 4. Write redacted copy
  const doc = new Document({
    sections: [{
      children: redactedParagraphs  // Paragraphs with [PERSON_1] etc.
    }]
  });
  
  const buffer = await Packer.toBuffer(doc);
  fs.writeFileSync(redactedPath, buffer);
}
```

### 16.4 PDF Redaction Implementation

Using `pdf-lib` npm package (https://www.npmjs.com/package/pdf-lib):

```typescript
import { PDFDocument, StandardFonts, rgb } from 'pdf-lib';
import * as fs from 'fs';

async function createRedactedPdf(
  originalPath: string,
  redactedPath: string,
  redactions: RedactionFinding[]
): Promise<void> {
  const pdfBytes = fs.readFileSync(originalPath);
  const pdfDoc = await PDFDocument.load(pdfBytes);
  
  // Strategy 1: Overlay redaction boxes (visible redaction)
  const pages = pdfDoc.getPages();
  for (const finding of redactions) {
    const page = pages[finding.page];
    page.drawRectangle({
      x: finding.x,
      y: finding.y,
      width: finding.width,
      height: finding.height,
      color: rgb(1, 1, 1),  // White box
    });
    // Add token text on top
    page.drawText(finding.token, {
      x: finding.x + 2,
      y: finding.y + 2,
      size: 10,
      font: await pdfDoc.embedFont(StandardFonts.Helvetica),
      color: rgb(0, 0, 0),
    });
  }
  
  // Strategy 2: Replace text content (cleaner but complex)
  // Requires text stream manipulation
  
  const redactedBytes = await pdfDoc.save();
  fs.writeFileSync(redactedPath, redactedBytes);
}
```

### 16.5 MCP Tool: `ingest_folder`

```json
{
  "name": "ingest_folder",
  "description": "Watch/ingest a folder: extract, redact, create redacted copies, vectorize",
  "inputSchema": {
    "type": "object",
    "properties": {
      "source_folder": { "type": "string", "description": "Path to original files" },
      "redacted_folder": { "type": "string", "description": "Path for redacted copies" },
      "collection": { "type": "string", "description": "RAG collection name" },
      "mode": { "type": "string", "enum": ["one-shot", "watch"], "default": "one-shot" },
      "redaction_strategy": { "type": "string", "enum": ["token_replace", "mask", "hash"], "default": "token_replace" },
      "preserve_structure": { "type": "boolean", "default": true, "description": "Keep original formatting in redacted files" }
    },
    "required": ["source_folder", "redacted_folder", "collection"]
  }
}
```

### 16.6 Implementation Pipeline

```
ingest_folder workflow:
    │
    ├─ 1. Scan source_folder for supported files
    │
    ├─ 2. For each file:
    │   ├─ xberg::extract() → text + metadata + entities
    │   ├─ PII detection (GLiNER2-PII + patterns)
    │   ├─ Redaction (TokenReplace → [PERSON_1], [EMAIL_1], etc.)
    │   ├─ Create redacted copy in redacted_folder/
    │   │   ├─ .docx → docx npm library
    │   │   ├─ .pdf → pdf-lib npm library
    │   │   └─ .txt/.md → native string replacement
    │   ├─ Store rehydration map (encrypted)
    │   └─ Ingest redacted text into RAG corpus
    │
    ├─ 3. Create sqlite-vec vectors for all chunks
    │
    ├─ 4. Create graph nodes/edges (if graphqlite enabled)
    │
    └─ 5. Return summary: files processed, PII found, corpus ready
```

### 16.7 File Naming Convention

```
Original:     /docs/contract.pdf
Redacted:     /docs-redacted/contract_REDACTED.pdf
Rehydration:  /docs-redacted/.rehydration/contract.pdf.map (encrypted)
```

### 16.8 Dependencies to Add

```json
// mcp-server/package.json
{
  "dependencies": {
    "docx": "^9.0.0",        // DOCX creation + reports
    "pdf-lib": "^1.17.1",    // PDF manipulation
    "file-type": "^16.0.0"   // MIME detection
  }
}
```

### 16.9 Redaction Report Generation Pipeline

When `ingest_folder` processes a file, the report is generated as part of the pipeline:

```
File Processing Pipeline:
    │
    ├─ 1. xberg::extract() → text + metadata + entities
    │
    ├─ 2. PII Detection (GLiNER2-PII + patterns)
    │   └── Returns: PiiFinding[] with token, original, category, byteOffset, confidence
    │
    ├─ 3. Redaction (TokenReplace)
    │   └── Returns: redacted text + RehydrationMap
    │
    ├─ 4. Create Redacted Copy (per format)
    │   ├── .docx → docx npm (preserve formatting)
    │   ├── .pdf → pdf-lib (overlay strategy)
    │   └── .txt/.md → native replacement
    │
    ├─ 5. Generate DOCX Report (NEW)
    │   ├── Collect PiiFinding[] from step 2
    │   ├── Build RedactionReport object
    │   ├── createRedactionReport() → Document
    │   ├── Packer.toBuffer() → Buffer
    │   └── Write to {filename}_REPORT.docx
    │
    ├─ 6. Store rehydration map (encrypted)
    │
    └─ 7. Ingest redacted text into RAG corpus
```

### 16.11 Redaction Report Aggregation

When multiple documents are ingested into a collection, a **summary report** is maintained that aggregates all redacted files. This gives the user a clear overview of the entire ingestion job.

#### Summary Report Structure

```
Collection Ingestion Summary: "legal-docs"
├── Overview
│   ├── Total Files Processed: 15
│   ├── Total PII Redacted: 127 entities
│   ├── Collection: "legal-docs"
│   ├── Ingestion Date: 2026-06-29
│   └── Status: ✓ Complete
│
├── Category Breakdown (aggregated across all files)
│   ├── PERSON: 42 entities
│   ├── EMAIL: 28 entities
│   ├── PHONE: 21 entities
│   ├── ORGANIZATION: 18 entities
│   ├── LOCATION: 12 entities
│   └── DATE: 6 entities
│
├── Per-File Details
│   ├── contract.pdf
│   │   ├── Redacted: contract_REDACTED.pdf
│   │   ├── Report: contract_REPORT.docx
│   │   ├── PII Found: 12 entities
│   │   └── Rehydration: contract.pdf.map
│   │
│   ├── invoice.pdf
│   │   ├── Redacted: invoice_REDACTED.pdf
│   │   ├── Report: invoice_REPORT.docx
│   │   ├── PII Found: 8 entities
│   │   └── Rehydration: invoice.pdf.map
│   │
│   └── ... (all files)
│
├── File Output Locations
│   ├── Originals: /docs/
│   ├── Redacted: /docs-redacted/
│   ├── Reports: /docs-redacted/*_REPORT.docx
│   └── Rehydration Maps: /docs-redacted/.rehydration/
│
└── Usage Guide
    ├── How to view reports
    ├── How to cross-reference redacted files
    ├── How to rehydrate (decrypt PII)
    └── How to query the RAG corpus
```

#### Summary Report File

```
/docs-redacted/SUMMARY_REPORT.docx
```

This file is updated after each document is processed and contains:
- Running totals of PII detected
- Per-file breakdown
- Category distribution
- Usage instructions

### 16.12 Report File Naming

```
Original:      /docs/contract.pdf
Redacted:      /docs-redacted/contract_REDACTED.pdf
Report:        /docs-redacted/contract_REPORT.docx
Rehydration:   /docs-redacted/.rehydration/contract.pdf.map

Summary:       /docs-redacted/SUMMARY_REPORT.docx
```

### 16.13 MCP Tools for Report Access

The MCP server provides tools to query reports and guide the user:

```typescript
// Tool 1: Get ingestion summary
{
  "name": "get_ingestion_summary",
  "description": "Get summary of all redacted files in a collection",
  "inputSchema": {
    "type": "object",
    "properties": {
      "collection": { "type": "string", "description": "Collection name" }
    },
    "required": ["collection"]
  }
}

// Tool 2: Get report for specific document
{
  "name": "get_document_report",
  "description": "Get PII report for a specific document",
  "inputSchema": {
    "type": "object",
    "properties": {
      "collection": { "type": "string" },
      "document_id": { "type": "string" }
    },
    "required": ["collection", "document_id"]
  }
}

// Tool 3: Explain how to use reports
{
  "name": "explain_reports",
  "description": "Explain how to use redaction reports and redacted files",
  "inputSchema": {
    "type": "object",
    "properties": {
      "format": { "type": "string", "enum": ["summary", "detailed"], "default": "summary" }
    }
  }
}
```

#### MCP Tool Responses

**`get_ingestion_summary` response:**
```json
{
  "collection": "legal-docs",
  "total_files": 15,
  "total_pii_entities": 127,
  "category_breakdown": {
    "PERSON": 42,
    "EMAIL": 28,
    "PHONE": 21,
    "ORGANIZATION": 18,
    "LOCATION": 12,
    "DATE": 6
  },
  "output_locations": {
    "originals": "/docs/",
    "redacted": "/docs-redacted/",
    "reports": "/docs-redacted/*_REPORT.docx",
    "summary_report": "/docs-redacted/SUMMARY_REPORT.docx",
    "rehydration_maps": "/docs-redacted/.rehydration/"
  },
  "files": [
    {
      "name": "contract.pdf",
      "redacted": "contract_REDACTED.pdf",
      "report": "contract_REPORT.docx",
      "pii_count": 12,
      "rehydration_map": "contract.pdf.map"
    }
  ],
  "usage_hints": [
    "Open *_REPORT.docx files to see colored PII highlights",
    "Open *_REDACTED.pdf files to see the sanitized versions",
    "Use rehydrate_document tool to decrypt PII (requires passphrase)",
    "Query the RAG corpus using query_corpus tool"
  ]
}
```

**`explain_reports` response:**
```json
{
  "summary": "Redaction reports show what PII was detected and redacted from your documents.",
  "how_to_use": {
    "view_reports": "Open any *_REPORT.docx file in Microsoft Word or LibreOffice. Each report shows:\n- Summary table with PII categories (color-coded)\n- Page-by-page preview with highlighted tokens\n- Legend explaining what each token means",
    "view_redacted_files": "Open *_REDACTED.pdf files to see the sanitized versions. PII has been replaced with tokens like [PERSON_1], [EMAIL_1], etc.",
    "cross_reference": "Compare the report with the redacted file to understand exactly what was changed.",
    "rehydrate": "Use the rehydrate_document or rehydrate_token MCP tool to decrypt specific PII tokens back to original values. You'll need your passphrase.",
    "query_corpus": "Use the query_corpus MCP tool to search the ingested documents. The RAG corpus contains the redacted text.",
    "summary_report": "Open SUMMARY_REPORT.docx for an overview of all processed files in this collection."
  },
  "file_locations": {
    "originals": "Your original files are untouched in the source folder.",
    "redacted": "Redacted copies are in the redacted folder.",
    "reports": "Individual reports are named {filename}_REPORT.docx.",
    "summary": "The summary report is SUMMARY_REPORT.docx."
  },
  "gdpr_notes": [
    "Original files are never modified",
    "Rehydration maps are encrypted and require a passphrase",
    "Audit trail is maintained for compliance",
    "Right to erasure (Art. 17) supported via delete tools"
  ]
}
```

### 16.14 MCP Server Guidance During Ingestion

When `ingest_folder` runs, the MCP server sends progress messages that inform the user about reports:

```json
// During ingestion
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "data": {
      "message": "Processing contract.pdf... Found 12 PII entities"
    }
  }
}

// After each file
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "data": {
      "message": "✓ contract.pdf: Redacted copy created, report generated"
    }
  }
}

// After ingestion complete
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "data": {
      "message": "✓ Ingestion complete! 15 files processed, 127 PII entities redacted.\n\nReports available:\n- Individual: /docs-redacted/*_REPORT.docx\n- Summary: /docs-redacted/SUMMARY_REPORT.docx\n\nUse explain_reports tool for usage guide."
    }
  }
}
```

### 16.15 Report Enhancements (Optional)

Future improvements for the report:

| Enhancement | Description | Priority |
|-------------|-------------|----------|
| **Side-by-side view** | Original vs redacted text in two columns | High |
| **Confidence indicators** | Show detection confidence (e.g., "High: 95%") | Medium |
| **Page thumbnails** | PDF page images in the report | Low |
| **Interactive legend** | Click to highlight all tokens of that category | Low |
| **Export options** | PDF version of the report itself | Medium |
| **Statistics dashboard** | Charts showing PII distribution over time | Low |
| **HTML reports** | Web-viewable reports with search/filter | Medium |
| **Report comparison** | Compare PII across document versions | Low |

### 16.9 Redaction Report (DOCX)

A user-friendly DOCX report showing what was redacted, generated alongside the redacted files.

#### Report Structure

```
PII Detection Report
├── Cover Page
│   ├── Title: "PII Detection Report"
│   ├── Document name: "contract.pdf"
│   ├── Date: "2026-06-29"
│   └── Summary: "12 PII entities detected and redacted"
│
├── Summary Page
│   ├── Category Breakdown (colored table)
│   │   ├── PERSON (green): 3 entities
│   │   ├── EMAIL (red): 2 entities
│   │   ├── PHONE (blue): 2 entities
│   │   ├── ORGANIZATION (orange): 1 entity
│   │   ├── LOCATION (purple): 1 entity
│   │   └── DATE (gray): 3 entities
│   │
│   ├── Total Redactions: 12
│   ├── Strategy: TokenReplace
│   └── Rehydration: Enabled (encrypted map)
│
├── Legend Page
│   ├── Token format: [CATEGORY_N]
│   ├── Color coding guide
│   └── Rehydration instructions
│
├── Document Preview (Page-by-page)
│   ├── Page 1: Original text with colored highlights
│   │   ├── [PERSON_1] highlighted in GREEN
│   │   ├── [EMAIL_1] highlighted in RED
│   │   └── [PHONE_1] highlighted in BLUE
│   │
│   ├── Page 2: ...
│   └── Page N: ...
│
└── Appendix
    ├── Full entity list with byte offsets
    ├── Audit trail JSON
    └── Rehydration map reference
```

#### Visual Design

```
┌─────────────────────────────────────────────────────────┐
│  PII Detection Report                                    │
│  ═══════════════════════════════════════════════════════  │
│                                                          │
│  Document: contract.pdf                                  │
│  Date: 2026-06-29                                        │
│  Status: ✓ Redacted & Ingested                          │
│                                                          │
├─────────────────────────────────────────────────────────┤
│  Summary                                                 │
│  ───────                                                 │
│  Total PII Found: 12 entities                            │
│                                                          │
│  ┌──────────────┬───────┬──────────┐                     │
│  │ Category     │ Count │ Color    │                     │
│  ├──────────────┼───────┼──────────┤                     │
│  │ PERSON       │ 3     │ 🟢 Green │                     │
│  │ EMAIL        │ 2     │ 🔴 Red   │                     │
│  │ PHONE        │ 2     │ 🔵 Blue  │                     │
│  │ ORGANIZATION │ 1     │ 🟠 Orange│                     │
│  │ LOCATION     │ 1     │ 🟣 Purple│                     │
│  │ DATE         │ 3     │ ⚪ Gray  │                     │
│  └──────────────┴───────┴──────────┘                     │
│                                                          │
├─────────────────────────────────────────────────────────┤
│  Page 1 Preview                                          │
│  ──────────────                                          │
│                                                          │
│  The [PERSON_1] signed the contract on [DATE_1].         │
│  Contact: [EMAIL_1] or [PHONE_1].                        │
│  Company: [ORGANIZATION_1] located in [LOCATION_1].      │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │ [PERSON_1]  = "John Smith"        (🟢 PERSON)   │   │
│  │ [DATE_1]    = "2026-01-15"        (⚪ DATE)     │   │
│  │ [EMAIL_1]   = "john@acme.com"     (🔴 EMAIL)    │   │
│  │ [PHONE_1]   = "+33 1 23 45 67 89" (🔵 PHONE)   │   │
│  │ [ORGANIZATION_1] = "Acme Corp"    (🟠 ORG)     │   │
│  │ [LOCATION_1] = "Paris, France"    (🟣 LOC)     │   │
│  └──────────────────────────────────────────────────┘   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

#### Color Coding Scheme

| Category | Text Color | Highlight | Token Example |
|----------|------------|-----------|---------------|
| PERSON | #00B050 (green) | Light green | `[PERSON_1]` |
| EMAIL | #FF0000 (red) | Light red | `[EMAIL_1]` |
| PHONE | #0070C0 (blue) | Light blue | `[PHONE_1]` |
| ORGANIZATION | #ED7D31 (orange) | Light orange | `[ORGANIZATION_1]` |
| LOCATION | #7030A0 (purple) | Light purple | `[LOCATION_1]` |
| DATE | #808080 (gray) | Light gray | `[DATE_1]` |
| SSN | #C00000 (dark red) | Pink | `[SSN_1]` |
| CREDIT_CARD | #002060 (navy) | Light navy | `[CREDIT_CARD_1]` |

#### DOCX Generation Code

```typescript
import {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, PageNumber, AlignmentType, WidthType,
  HeadingLevel, PageBreak, BorderStyle, ShadingType
} from "docx";

interface PiiFinding {
  token: string;
  original: string;
  category: string;
  byteOffset: number;
  confidence: number;
}

interface RedactionReport {
  documentName: string;
  timestamp: string;
  totalRedacted: number;
  findings: PiiFinding[];
  pages: { pageNum: number; text: string; highlights: Highlight[] }[];
}

function createRedactionReport(report: RedactionReport): Document {
  const categoryColors: Record<string, { text: string; highlight: string }> = {
    PERSON: { text: "00B050", highlight: "lightGreen" },
    EMAIL: { text: "FF0000", highlight: "red" },
    PHONE: { text: "0070C0", highlight: "lightBlue" },
    ORGANIZATION: { text: "ED7D31", highlight: "orange" },
    LOCATION: { text: "7030A0", highlight: "lightPurple" },
    DATE: { text: "808080", highlight: "lightGray" },
  };

  return new Document({
    sections: [{
      headers: {
        default: new Header({
          children: [new Paragraph({
            children: [new TextRun({ text: "PII Detection Report", bold: true })],
          })],
        }),
      },
      footers: {
        default: new Footer({
          children: [new Paragraph({
            alignment: AlignmentType.CENTER,
            children: [
              new TextRun("Page "),
              new TextRun({ children: [PageNumber.CURRENT] }),
              new TextRun(" of "),
              new TextRun({ children: [PageNumber.TOTAL_PAGES] }),
            ],
          })],
        }),
      },
      children: [
        // Cover page
        new Paragraph({
          heading: HeadingLevel.TITLE,
          children: [new TextRun({ text: "PII Detection Report", bold: true, size: 48 })],
        }),
        new Paragraph({
          children: [new TextRun({ text: `Document: ${report.documentName}`, size: 24 })],
        }),
        new Paragraph({
          children: [new TextRun({ text: `Date: ${report.timestamp}`, size: 24 })],
        }),
        new Paragraph({
          children: [new TextRun({ text: `Total Redactions: ${report.totalRedacted}`, size: 24, bold: true })],
        }),

        // Page break
        new Paragraph({ children: [new PageBreak()] }),

        // Summary table
        new Paragraph({
          heading: HeadingLevel.HEADING_1,
          children: [new TextRun({ text: "Summary", bold: true })],
        }),
        createSummaryTable(report.findings, categoryColors),

        // Page break
        new Paragraph({ children: [new PageBreak()] }),

        // Document preview with highlights
        new Paragraph({
          heading: HeadingLevel.HEADING_1,
          children: [new TextRun({ text: "Document Preview", bold: true })],
        }),
        ...report.pages.flatMap(page => [
          new Paragraph({
            heading: HeadingLevel.HEADING_2,
            children: [new TextRun({ text: `Page ${page.pageNum}` })],
          }),
          ...createHighlightedParagraphs(page, categoryColors),
        ]),
      ],
    }],
  });
}

function createSummaryTable(findings: PiiFinding[], colors: Record<string, any>): Table {
  const categories = [...new Set(findings.map(f => f.category))];
  
  return new Table({
    width: { size: 100, type: WidthType.PERCENTAGE },
    rows: [
      // Header row
      new TableRow({
        children: ["Category", "Count", "Example"].map(h =>
          new TableCell({
            children: [new Paragraph({ children: [new TextRun({ text: h, bold: true })] })],
            shading: { fill: "D9E2F3" },
          })
        ),
      }),
      // Data rows
      ...categories.map(cat => {
        const catFindings = findings.filter(f => f.category === cat);
        const color = colors[cat] || { text: "000000", highlight: "white" };
        return new TableRow({
          children: [
            new TableCell({
              children: [new Paragraph({
                children: [new TextRun({ text: cat, color: color.text, bold: true })],
              })],
            }),
            new TableCell({
              children: [new Paragraph({ children: [new TextRun({ text: catFindings.length.toString() })] })],
            }),
            new TableCell({
              children: [new Paragraph({
                children: [new TextRun({ text: catFindings[0]?.token || "N/A", highlight: color.highlight as any })],
              })],
            }),
          ],
        });
      }),
    ],
  });
}

function createHighlightedParagraphs(
  page: { text: string; highlights: any[] },
  colors: Record<string, any>
): Paragraph[] {
  // Split text by redacted tokens and create colored TextRuns
  const parts = page.text.split(/(\[[A-Z]+_\d+\])/);
  
  return [new Paragraph({
    children: parts.map(part => {
      const match = part.match(/^\[([A-Z]+)_(\d+)\]$/);
      if (match) {
        const [, category] = match;
        const color = colors[category] || { text: "000000", highlight: "white" };
        return new TextRun({
          text: part,
          bold: true,
          color: color.text,
          highlight: color.highlight as any,
        });
      }
      return new TextRun({ text: part });
    }),
  })];
}
```

#### MCP Tool Output

When `ingest_folder` completes, it returns:

```json
{
  "status": "success",
  "files_processed": 5,
  "total_redactions": 47,
  "redacted_files": [
    {
      "original": "/docs/contract.pdf",
      "redacted": "/docs-redacted/contract_REDACTED.pdf",
      "report": "/docs-redacted/contract_REPORT.docx",
      "rehydration_map": "/docs-redacted/.rehydration/contract.pdf.map"
    }
  ],
  "category_breakdown": {
    "PERSON": 12,
    "EMAIL": 8,
    "PHONE": 7,
    "ORGANIZATION": 9,
    "LOCATION": 5,
    "DATE": 6
  },
  "corpus_ready": true,
  "collection": "legal-docs"
}
```

#### User Experience Flow

```
1. User runs: ingest_folder --source /docs --redacted /docs-redacted --collection legal-docs

2. MCP server shows progress:
   "Processing contract.pdf... Found 12 PII entities"
   "Creating redacted copy: contract_REDACTED.pdf"
   "Creating report: contract_REPORT.docx"
   "Ingesting into corpus: legal-docs"
   "✓ Complete: 5 files processed, 47 redactions"

3. User opens /docs-redacted/:
   ├── contract.pdf              (original, untouched)
   ├── contract_REDACTED.pdf     (redacted copy)
   ├── contract_REPORT.docx      (visual report with highlights)
   ├── invoice.pdf
   ├── invoice_REDACTED.pdf
   ├── invoice_REPORT.docx
   └── .rehydration/
       ├── contract.pdf.map      (encrypted)
       └── invoice.pdf.map       (encrypted)

4. User opens contract_REPORT.docx:
   - Sees summary table with colored categories
   - Sees page-by-page preview with highlighted tokens
   - Sees legend explaining what each token means
   - Can cross-reference with redacted PDF
```

| Risk | Impact | Mitigation |
|------|--------|------------|
| Model OOM on 12GB | Crash | Lazy load, LRU eviction, semaphore cap |
| BGE-M3 download large (2.29 GB) | Long first-run wait | Progress notifications, `cache_warm` tool, show MB downloaded |
| GLiNER2-PII tokenizer incompatibility | NER fails | Verify okasi repo has tokenizer.json; fallback to pattern-only PII |
| Rehydration key loss | Permanent data loss | Key escrow option, backup prompts |
| GDPR audit failure | Legal risk | Full audit trail, encrypted maps, Art. 30 compliance |
| sqlite-vec performance | Slow queries | FTS5 + vector hybrid, HNSW index for large collections |
| graphqlite coexistence | Extension conflicts | Both use rusqlite bundled; test在同一 DB file |
| Graph traversal slow | High latency | Limit graph_depth, cache communities, use index |
| `delete_by_filter` slow on large collections | Art. 17 delay | SQL-side filter optimization for common cases |
| Reranker adds latency | Slower queries | Use `candidate_multiplier` to limit candidates before rerank |
| Louvain community detection slow | Graph mode delay | Run periodically, cache results keyed by `(collection, last_ingest_timestamp)`, invalidate on ingest |
| First-run download fails (no internet) | Tools unusable | Graceful error: "Models not available offline. Run `cache_warm` when online." |
| Download interrupted | Partial cache | hf-hub resumes downloads, stale lock cleanup after 30 min |
