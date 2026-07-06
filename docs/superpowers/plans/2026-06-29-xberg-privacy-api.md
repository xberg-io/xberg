# xberg Privacy API v1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade xberg into a Privacy/GDPR-focused document intelligence API by porting GLiNER2Fastino, adding PII rehydration, and creating a unified `/v1/process` endpoint.

**Architecture:** Extend existing xberg REST API (axum + utoipa) with new endpoints. GLiNER2Fastino (from anno) replaces basic xberg-gliner. PII rehydration uses AES-256-GCM encrypted maps. RAG search uses xberg-rag.

**Tech Stack:** Rust (axum), ONNX Runtime, sqlite-vec for RAG

---

## Phase 1: GLiNER2 Upgrade for xberg-gliner

Port `anno::gliner2_fastino` (11 files, 8-session ONNX pipeline, IoBinding) into `xberg-gliner` to replace the basic implementation.

### Task 1: Scaffold new gliner2 module in xberg-gliner

**Files:**
- Create: `crates/xberg-gliner/src/gliner2/mod.rs`
- Create: `crates/xberg-gliner/src/gliner2/config.rs`
- Create: `crates/xberg-gliner/src/gliner2/tokenizer.rs`
- Modify: `crates/xberg-gliner/src/lib.rs`

- [ ] **Step 1: Create `gliner2/mod.rs`**

```rust
//! GLiNER2 Fastino backend for xberg-gliner.
//! Ported from anno::gliner2_fastino.

pub mod config;
pub mod tokenizer;

use tokenizers::Tokenizer;

pub struct Gliner2Engine {
    tokenizer: Tokenizer,
    // ONNX sessions will be added in Task 2
}

impl Gliner2Engine {
    pub fn from_pretrained(model_id: &str) -> Result<Self, crate::Error> {
        let tokenizer = Tokenizer::from_pretrained(model_id, None)
            .map_err(|e| crate::Error::Tokenizer(e.to_string()))?;
        Ok(Self { tokenizer })
    }
}
```

- [ ] **Step 2: Add `gliner2` module to `lib.rs`**

In `crates/xberg-gliner/src/lib.rs`, add:

```rust
pub mod gliner2;
```

- [ ] **Step 3: Test it compiles**

Run: `cargo check -p xberg-gliner`
Expected: PASS (no linker errors)

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/gliner2/ crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): scaffold gliner2 module"
```

---

### Task 2: Port tokenizer and config from anno

**Files:**
- Modify: `crates/xberg-gliner/src/gliner2/tokenizer.rs`
- Modify: `crates/xberg-gliner/src/gliner2/config.rs`
- Modify: `crates/xberg-gliner/src/gliner2/mod.rs`

- [ ] **Step 1: Copy tokenizer logic from `anno`**

Adapt `anno::gliner2_fastino::processor.rs` tokenizer functions into `crates/xberg-gliner/src/gliner2/tokenizer.rs`.

Key functions to port:
- `SchemaTransformer::new()`
- `SchemaTransformer::transform()`
- `SpecialTokenIds::resolve()`

- [ ] **Step 2: Copy config from `anno`**

Adapt `anno::gliner2_fastino::config.rs` into `crates/xberg-gliner/src/gliner2/config.rs`.

Key structs:
- `FastinoConfig`
- `GLiNER2FastinoConfig`
- `ExecutionMode`

- [ ] **Step 3: Integrate into Gliner2Engine**

Update `crates/xberg-gliner/src/gliner2/mod.rs`:

```rust
use config::FastinoConfig;
use tokenizer::{SchemaTransformer, SpecialTokenIds};

pub struct Gliner2Engine {
    tokenizer: Tokenizer,
    transformer: SchemaTransformer,
    special: SpecialTokenIds,
    config: FastinoConfig,
}
```

- [ ] **Step 4: Test**

Run: `cargo test -p xberg-gliner gliner2 --no-run`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(gliner): port gliner2 tokenizer and config from anno"
```

---

### Task 3: Port 8-session ONNX pipeline from anno

**Files:**
- Create: `crates/xberg-gliner/src/gliner2/sessions.rs`
- Create: `crates/xberg-gliner/src/gliner2/pipeline.rs`
- Modify: `crates/xberg-gliner/src/gliner2/mod.rs`

- [ ] **Step 1: Copy sessions logic**

Port `anno::gliner2_fastino::sessions.rs` into `crates/xberg-gliner/src/gliner2/sessions.rs`.

Key: Use `ort` crate (same as xberg already does) instead of `ort` from anno.

- [ ] **Step 2: Copy pipeline logic**

Port `anno::gliner2_fastino::pipeline.rs` into `crates/xberg-gliner/src/gliner2/pipeline.rs`.

Key functions:
- `run_pipeline()`
- `decode_entities()`
- `decode_structure()`

- [ ] **Step 3: Add IoBinding pipeline option**

Port `anno::gliner2_fastino::pipeline_iobinding.rs` into `crates/xberg-gliner/src/gliner2/pipeline_iobinding.rs`.

- [ ] **Step 4: Integrate into Gliner2Engine**

Update `mod.rs` to load sessions:

```rust
use sessions::Sessions;

pub struct Gliner2Engine {
    tokenizer: Tokenizer,
    transformer: SchemaTransformer,
    special: SpecialTokenIds,
    config: FastinoConfig,
    sessions: Sessions,
}

impl Gliner2Engine {
    pub fn from_local(model_dir: &std::path::Path) -> Result<Self, crate::Error> {
        // Load tokenizer, sessions, config from model_dir
        todo!("implement model loading")
    }
}
```

- [ ] **Step 5: Test**

Run: `cargo test -p xberg-gliner --no-run`
Expected: Compiles

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(gliner): port gliner2 8-session onnx pipeline from anno"
```

---

### Task 4: Wire Gliner2Engine into NerConfig and NER trait

**Files:**
- Modify: `crates/xberg-gliner/src/lib.rs`
- Modify: `crates/xberg/src/core/config/ner.rs`
- Modify: `crates/xberg/src/text/ner.rs` (or wherever NER trait is defined)

- [ ] **Step 1: Add `Gliner2` variant to NerBackendKind**

In `crates/xberg/src/core/config/ner.rs`:

```rust
pub enum NerBackendKind {
    Onnx,  // existing xberg-gliner
    Llm,   // existing liter-llm
    Gliner2,  // NEW
}
```

- [ ] **Step 2: Add Gliner2 model config**

Add to `NerConfig`:

```rust
pub struct NerConfig {
    pub backend: NerBackendKind,
    pub categories: Vec<EntityCategory>,
    pub model: Option<String>,
    pub llm: Option<LlmConfig>,
    pub custom_labels: Vec<String>,
    // NEW:
    pub gliner2_model: Option<String>,  // e.g. "fastino/gliner2-multi-v1"
}
```

- [ ] **Step 3: Implement NER trait for Gliner2Engine**

In `crates/xberg-gliner/src/lib.rs`:

```rust
impl xberg_text::NERBackend for Gliner2Engine {
    fn extract_entities(&self, text: &str, types: &[&str], threshold: f32) -> Result<Vec<Entity>, Error> {
        self.extract_ner(text, types, threshold)
    }
}
```

(Adapt trait name to actual xberg trait)

- [ ] **Step 4: Test**

Run: `cargo test -p xberg-gliner --no-run`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(gliner): wire gliner2engine into ner config and trait"
```

---

### Task 5: Add structure extraction (schema capability)

**Files:**
- Create: `crates/xberg-gliner/src/gliner2/schema.rs`
- Modify: `crates/xberg-gliner/src/gliner2/mod.rs`

- [ ] **Step 1: Port schema types from anno**

Port `anno::gliner2_fastino::schema::{TaskSchema, StructureTask, FieldType, ExtractedStructure}` into `crates/xberg-gliner/src/gliner2/schema.rs`.

- [ ] **Step 2: Add `extract_structure` method to Gliner2Engine**

In `mod.rs`:

```rust
use schema::{TaskSchema, ExtractedStructure};

impl Gliner2Engine {
    pub fn extract_structure(&self, text: &str, schema: &TaskSchema, threshold: f32) -> Result<Vec<ExtractedStructure>, Error> {
        // Use pipeline::decode_structure
        todo!()
    }
}
```

- [ ] **Step 3: Test**

Run: `cargo test -p xberg-gliner --no-run`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(gliner): add schema/structure extraction from anno"
```

---

## Phase 2: PII Pipeline as First-Class Feature

### Task 6: Integrate PII detection from anno

**Files:**
- Create: `crates/xberg/src/pii/detector.rs`
- Modify: `crates/xberg/src/pii/mod.rs`

- [ ] **Step 1: Port PII detection logic**

Port `anno::pii` PII detection (11 GDPR categories) into `crates/xberg/src/pii/detector.rs`.

Use `Gliner2Engine` for zero-shot PII entity detection.

Key function:

```rust
pub fn detect_pii(text: &str, engine: &Gliner2Engine) -> Vec<PiiFinding> {
    let labels = vec![
        "person", "email", "phone", "ssn", "credit_card",
        "iban", "address", "date_of_birth", "passport", "driver_license"
    ];
    engine.extract_ner(text, &labels, 0.5)
        .into_iter()
        .map(|e| PiiFinding::from(e))
        .collect()
}
```

- [ ] **Step 2: Add to xberg PII module**

In `crates/xberg/src/pii/mod.rs`:

```rust
pub mod detector;
```

- [ ] **Step 3: Test**

Run: `cargo test -p xberg pii --no-run`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(pii): integrate gliner2 pii detection from anno"
```

---

### Task 7: Add PII rehydration with AES-256-GCM

**Files:**
- Create: `crates/xberg/src/pii/rehydration.rs`
- Modify: `crates/xberg/src/pii/mod.rs`

- [ ] **Step 1: Implement rehydration map**

```rust
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use scrypt::{scrypt, Params};

pub struct RehydrationMap {
    pub encrypted_data: Vec<u8>,
    pub nonce: [u8; 12],
    pub salt: [u8; 32],
}

impl RehydrationMap {
    pub fn new(findings: &[PiiFinding], passphrase: &str) -> Self {
        // Derive key using scrypt
        // Encrypt with AES-256-GCM
        // Return encrypted map
    }

    pub fn rehydrate(&self, passphrase: &str) -> Vec<PiiFinding> {
        // Decrypt with AES-256-GCM
        // Return original findings
    }
}
```

- [ ] **Step 2: Implement strategies**

```rust
pub enum RehydrationStrategy {
    ServerEncrypted,  // xberg manages key
    CustomerKey { key: String },  // customer provides key
    AuditLogged,  // requires approval + logging
}
```

- [ ] **Step 3: Test**

Run: `cargo test -p xberg pii::rehydration`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(pii): add aes-256-gcm rehydration with 3 strategies"
```

---

## Phase 3: Unified `/v1/process` Endpoint

### Task 8: Create ProcessRequest and ProcessResponse types

**Files:**
- Create: `crates/xberg/src/api/process.rs`
- Modify: `crates/xberg/src/api/types.rs`

- [ ] **Step 1: Add ProcessRequest type**

In `crates/xberg/src/api/types.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessRequest {
    pub input: ProcessInput,
    pub operations: OperationsConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub enum ProcessInput {
    File { data: Vec<u8>, filename: String },
    Url { url: String },
    Text { text: String },
}

#[derive(Debug, Clone, Deserialize escapedDeserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OperationsConfig {
    pub extract: Option<ExtractConfig>,
    pub redact: Option<RedactConfig>,
    pub ner: Option<NerConfig>,
    pub classify: Option<ClassifyConfig>,
    pub chunk: Option<ChunkConfig>,
    pub embed: Option<EmbedConfig>,
}
```

- [ ] **Step 2: Add ProcessResponse type**

```rust
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessResponse {
    pub task_id: String,
    pub status: String,
    pub document: DocumentResult,
}
```

- [ ] **Step 3: Test**

Run: `cargo check -p xberg --features api`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(api): add process request/response types"
```

---

### Task 9: Implement `/v1/process` handler

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`
- Modify: `crates/xberg/src/api/router.rs`

- [ ] **Step 1: Add process_handler function**

In `crates/xberg/src/api/handlers.rs`:

```rust
pub async fn process_handler(
    State(state): State<ApiState>,
    request: ProcessRequest,
) -> Result<Json<ProcessResponse>, ApiError> {
    // 1. Normalize input (file, url, text) -> bytes
    // 2. Auto-route based on MIME type
    // 3. Execute operations pipeline
    // 4. Return ProcessResponse

    let document = process_document(request.input, request.operations).await?;

    Ok(Json(ProcessResponse {
        task_id: generate_task_id(),
        status: "completed".to_string(),
        document,
    }))
}
```

- [ ] **Step 2: Add route to router**

In `crates/xberg/src/api/router.rs`:

```rust
.route("/v1/process", post(process_handler))
```

- [ ] **Step 3: Test**

```bash
curl -X POST http://localhost:3000/v1/process \
  -H "Content-Type: application/json" \
  -d '{"input": {"type": "text", "text": "hello"}, "operations": {"extract": null}}'
```

Expected: Returns task_id and document structure

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(api): implement /v1/process endpoint"
```

---

## Phase 4: Search & RAG

### Task 10: Implement `/v1/search` and `/v1/collections`

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`
- Modify: `crates/xberg/src/api/router.rs`
- Modify: `crates/xberg/src/api/types.rs`

- [ ] **Step 1: Add SearchRequest and SearchResponse**

In `crates/xberg/src/api/types.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub collections: Vec<String>,
    pub mode: SearchMode,
    pub top_k: usize,
    pub rerank: bool,
    pub filters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_results: usize,
    pub query_time_ms: u64,
}
```

- [ ] **Step 2: Add search_handler**

```rust
pub async fn search_handler(
    State(state): State<ApiState>,
    request: SearchRequest,
) -> Result<Json<SearchResponse>, ApiError> {
    // Use xberg-rag for hybrid search
    let results = state.rag_store.query(
        &request.query,
        &request.collections,
        request.mode,
        request.top_k,
    ).await?;

    Ok(Json(SearchResponse {
        results,
        total_results: results.len(),
        query_time_ms: 0,
    }))
}
```

- [ ] **Step 3: Add routes**

```rust
.route("/v1/search", post(search_handler))
.route("/v1/collections", get(list_collections).post(create_collection))
```

- [ ] **Step 4: Test**

```bash
curl -X POST http://localhost:3000/v1/search \
  -H "Content-Type: application/json" \
  -d '{"query": "contract terms", "collections": ["legal"], "top_k": 5}'
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(api): add search and collections endpoints"
```

---

## Phase 5: PII Rehydration Endpoint

### Task 11: Implement `/v1/documents/{id}/rehydrate`

**Files:**
- Modify: `crates/xberg/src/api/handlers.rs`
- Modify: `crates/xberg/src/api/router.rs`
- Modify: `crates/xberg/src/api/types.rs`

- [ ] **Step 1: Add RehydrateRequest**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RehydrateRequest {
    pub rehydration_key: String,
    pub passphrase: String,
    pub scope: String,  // "all" or specific fields
}
```

- [ ] **Step 2: Add rehydrate handler**

```rust
pub async fn rehydrate_handler(
    State(state): State<ApiState>,
    Path(document_id): Path<String>,
    request: RehydrateRequest,
) -> Result<Json<RehydrateResponse>, ApiError> {
    // Load encrypted rehydration map
    // Decrypt with passphrase
    // Return original PII
}
```

- [ ] **Step 3: Add route**

```rust
.route("/v1/documents/:id/rehydrate", post(rehydrate_handler))
```

- [ ] **Step 4: Test**

```bash
curl -X POST http://localhost:3000/v1/documents/doc_123/rehydrate \
  -H "Content-Type: application/json" \
  -d '{"rehydration_key": "reh_e4f8...", "passphrase": "secret"}'
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(api): add pii rehydration endpoint"
```

---

## Task Summary

| Phase | Task | Description | Est. Time |
|-------|------|-------------|-----------|
| Phase 1 | 1-5 | Port GLiNER2 from anno to xberg-gliner | 5 days |
| Phase 2 | 6-7 | Add PII detection + rehydration | 3 days |
| Phase 3 | 8-9 | Create unified `/v1/process` endpoint | 4 days |
| Phase 4 | 10 | Add search + RAG collections | 3 days |
| Phase 5 | 11 | Add PII rehydration API | 2 days |
| **Total** | 11 tasks | | **17 days** |

---

## Dependencies

- `xberg-gliner` crate (being upgraded)
- `xberg-rag` crate (already exists)
- `ort` for ONNX (already in xberg)
- `aes-gcm` + `scrypt` for rehydration encryption
- `utoipa` for OpenAPI (already in xberg)
