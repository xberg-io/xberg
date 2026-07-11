# Shared Engine-Driven PII+NER-Protected Ingest Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `ingest_document`/`ingest_document_local` in `xberg-rag`'s pipeline mandatorily detect and redact PII (regex + Candle NER) before chunking, thread that capability through `xberg-wasm`'s `engine.ingest()`, and add a host-agnostic `ingestFolder()` orchestrator to `xberg-wasm-runtime` so MCP and the browser can eventually ingest folders through byte-for-byte identical logic.

**Architecture:** A new `pipeline-redaction` feature in `xberg-rag` requires a caller-supplied `&dyn xberg::text::ner::NerBackend` and calls a new core function `xberg::text::redaction::redact_text_capturing_rehydration_map()` before chunking. `xberg-wasm::engine.ingest()` supplies the already-loaded Candle PII backend (from Task 11's `initCandleNer`/`CANDLE_NER`) and surfaces the resulting rehydration map + PII category counts to JS. `xberg-wasm-runtime` gets a new `ingestFolder()` export that loops `engine.extract()` → `engine.ingest()` per file with zero filesystem access.

**Tech Stack:** Rust (xberg, xberg-rag, xberg-wasm), TypeScript (xberg-wasm-runtime), tokio, wasm-bindgen, vitest.

## Global Constraints

- Redaction strategy is fixed to `TokenReplace` — the only reversible strategy. No other strategy is exposed by this pipeline.
- PII detection is mandatory: `engine.ingest()` errors if `initCandleNer()` has not been called first. There is no opt-out.
- The rehydration map is returned to the caller, never persisted or encrypted by this pipeline itself (`engine.encryptMap()` already exists, separately, for callers that want that).
- Only category counts are ever logged — never the matched PII text itself.
- `crates/xberg-wasm/Cargo.toml` is Alef-generated (header: "DO NOT EDIT"). Its `xberg-rag` feature list is sourced from `alef.toml`'s `[crates.wasm.extra_dependencies]` section (line 566) — edit `alef.toml`, then run `task alef:generate`, never hand-edit the Cargo.toml.
- No changes to `mcp-server/` or any browser UI code — those are separately owned (Sub-project E and D respectively). This plan only builds the shared contract they will consume later.

---

### Task 1: Core `redact_text_capturing_rehydration_map()` in `xberg`

**Files:**
- Modify: `crates/xberg/src/text/redaction/engine.rs`
- Test: same file, `#[cfg(test)] mod tests` at the bottom

**Interfaces:**
- Consumes: existing private helpers in this file — `scan_text` (from `super::patterns`, already imported), `dedupe_overlaps`, `apply_replacements_reverse` (private fns already defined in this file), `TokenCounter`/`apply_strategy` (from `super::strategy`, already imported), `xberg::text::ner::NerBackend` trait (`async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>>`), `xberg::types::entity::{Entity, EntityCategory}`.
- Produces: `pub struct TextRedactionOutcome { pub redacted_text: String, pub rehydration_map: RehydrationMap, pub category_counts: std::collections::HashMap<String, usize> }` and `pub async fn redact_text_capturing_rehydration_map(text: &str, strategy: RedactionStrategy, ner: &dyn crate::text::ner::NerBackend) -> Result<TextRedactionOutcome>`, both `#[cfg(all(feature = "redaction-rehydrate", feature = "ner"))]`. Task 2 calls this directly.

This function exists because the codebase's existing `redact()`/`redact_capturing_rehydration_map()` (same file) operate on `ExtractedDocument` and always construct their own NER backend from a `NerConfig` (filesystem `model_dir` for the Candle path — unusable on wasm32, where the model is loaded via `from_bytes` into a `CANDLE_NER` thread-local instead). This new function takes plain text and an already-constructed backend reference instead, reusing the same regex/merge/strategy machinery.

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `crates/xberg/src/text/redaction/engine.rs`, inside the existing `#[cfg(test)] mod tests { use super::*; ... }` block (append after the existing tests, before the closing `}`):

```rust
    struct StubNerBackend {
        entities: Vec<crate::types::entity::Entity>,
    }

    #[async_trait::async_trait]
    impl crate::text::ner::NerBackend for StubNerBackend {
        async fn detect(
            &self,
            _text: &str,
            _categories: &[crate::types::entity::EntityCategory],
        ) -> Result<Vec<crate::types::entity::Entity>> {
            Ok(self.entities.clone())
        }
    }

    #[tokio::test]
    async fn redact_text_merges_regex_and_ner_matches() {
        let ner = StubNerBackend {
            entities: vec![crate::types::entity::Entity {
                category: crate::types::entity::EntityCategory::Person,
                text: "Alice".to_string(),
                start: 8,
                end: 13,
                confidence: Some(0.99),
            }],
        };

        let outcome = redact_text_capturing_rehydration_map(
            "Contact Alice at alice@example.com for details.",
            RedactionStrategy::TokenReplace,
            &ner,
        )
        .await
        .unwrap();

        assert_eq!(outcome.redacted_text, "Contact [PERSON_1] at [EMAIL_1] for details.");
        assert_eq!(outcome.rehydration_map.get("[PERSON_1]").map(String::as_str), Some("Alice"));
        assert_eq!(
            outcome.rehydration_map.get("[EMAIL_1]").map(String::as_str),
            Some("alice@example.com")
        );
        assert_eq!(outcome.category_counts.get("Person"), Some(&1));
        assert_eq!(outcome.category_counts.get("Email"), Some(&1));
    }

    #[tokio::test]
    async fn redact_text_works_with_no_ner_matches() {
        let ner = StubNerBackend { entities: vec![] };

        let outcome = redact_text_capturing_rehydration_map(
            "Call 555-0100 or email bob@test.io.",
            RedactionStrategy::TokenReplace,
            &ner,
        )
        .await
        .unwrap();

        assert!(outcome.redacted_text.contains("[EMAIL_1]"));
        assert!(!outcome.redacted_text.contains("bob@test.io"));
        assert_eq!(outcome.category_counts.len(), outcome.rehydration_map.len());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p xberg --lib --features "redaction-rehydrate,ner,ner-candle" text::redaction::engine::tests::redact_text -- --nocapture`
Expected: FAIL with "cannot find function `redact_text_capturing_rehydration_map`" (and "cannot find type `TextRedactionOutcome`" / "cannot find type `StubNerBackend`" trait errors).

- [ ] **Step 3: Add the import and implementation**

In `crates/xberg/src/text/redaction/engine.rs`, change the top-level import block from:

```rust
use crate::Result;
use crate::core::config::redaction::RedactionConfig;
use crate::types::ExtractedDocument;
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionReport};

use super::patterns::{PatternMatch, scan_text};
use super::strategy::{TokenCounter, apply_strategy};

#[cfg(feature = "redaction-rehydrate")]
use super::rehydration::RehydrationMap;
```

to:

```rust
use crate::Result;
use crate::core::config::redaction::RedactionConfig;
use crate::types::ExtractedDocument;
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionReport, RedactionStrategy};

use super::patterns::{PatternMatch, scan_text};
use super::strategy::{TokenCounter, apply_strategy};

#[cfg(feature = "redaction-rehydrate")]
use super::rehydration::RehydrationMap;
```

(only the added `RedactionStrategy` import — everything else is unchanged).

Then append this new code after `redact_capturing_rehydration_map` (i.e., right after its closing `}` at line 45, before the `redact_inner` doc comment):

```rust
/// Outcome of [`redact_text_capturing_rehydration_map`]: the redacted text,
/// its rehydration map, and per-category finding counts. Counts only — the
/// matched PII text itself is never included here, per the redaction
/// pipeline's logging rule.
#[cfg(all(feature = "redaction-rehydrate", feature = "ner"))]
#[derive(Debug, Clone, Default)]
pub struct TextRedactionOutcome {
    pub redacted_text: String,
    pub rehydration_map: RehydrationMap,
    pub category_counts: std::collections::HashMap<String, usize>,
}

/// Redact plain text (not an [`ExtractedDocument`]) using regex PII patterns
/// merged with NER-detected Person/Organization/Location entities, capturing
/// a token→original rehydration map.
///
/// Unlike [`redact`]/[`redact_capturing_rehydration_map`], which always
/// construct their own NER backend from a [`RedactionConfig`]'s
/// [`NerConfig`](crate::core::config::ner::NerConfig) (filesystem `model_dir`
/// for the Candle path), this function takes an already-constructed `ner`
/// backend reference — for callers holding a backend loaded via
/// `NerBackend::from_bytes` (e.g. wasm32, no filesystem access) instead of
/// from a local directory.
///
/// Only `RedactionStrategy::TokenReplace` populates `rehydration_map`;
/// `Mask`/`Hash`/`Drop` leave it empty (matching [`apply_strategy`]).
///
/// # Errors
///
/// Propagates errors from `ner.detect(...)`.
#[cfg(all(feature = "redaction-rehydrate", feature = "ner"))]
pub async fn redact_text_capturing_rehydration_map(
    text: &str,
    strategy: RedactionStrategy,
    ner: &dyn crate::text::ner::NerBackend,
) -> Result<TextRedactionOutcome> {
    use crate::types::entity::EntityCategory;

    let mut matches = scan_text(text, &[]);

    let entities = ner
        .detect(
            text,
            &[EntityCategory::Person, EntityCategory::Organization, EntityCategory::Location],
        )
        .await?;
    matches.extend(entities.into_iter().filter_map(|e| {
        let category = match e.category {
            EntityCategory::Person => PiiCategory::Person,
            EntityCategory::Organization => PiiCategory::Organization,
            EntityCategory::Location => PiiCategory::Location,
            _ => return None,
        };
        Some(PatternMatch {
            start: e.start as usize,
            end: e.end as usize,
            category,
            text: e.text,
        })
    }));

    let matches = dedupe_overlaps(matches);

    let mut counter = TokenCounter::new();
    let mut rehydration_map = RehydrationMap::new();
    let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut findings: Vec<RedactionFinding> = Vec::with_capacity(matches.len());
    for m in &matches {
        let replacement = apply_strategy(strategy, &m.text, &m.category, &mut counter);
        if strategy == RedactionStrategy::TokenReplace {
            rehydration_map.entry(replacement.clone()).or_insert_with(|| m.text.clone());
        }
        *category_counts.entry(format!("{:?}", m.category)).or_insert(0) += 1;
        findings.push(RedactionFinding {
            start: m.start as u32,
            end: m.end as u32,
            category: m.category.clone(),
            strategy,
            replacement_token: replacement,
        });
    }

    let redacted_text = apply_replacements_reverse(text, &matches, &findings);
    Ok(TextRedactionOutcome {
        redacted_text,
        rehydration_map,
        category_counts,
    })
}
```

Also export it from the module. In `crates/xberg/src/text/redaction/mod.rs`, change:

```rust
pub use engine::redact;
#[cfg(feature = "redaction-rehydrate")]
pub use engine::redact_capturing_rehydration_map;
```

to:

```rust
pub use engine::redact;
#[cfg(feature = "redaction-rehydrate")]
pub use engine::redact_capturing_rehydration_map;
#[cfg(all(feature = "redaction-rehydrate", feature = "ner"))]
pub use engine::{TextRedactionOutcome, redact_text_capturing_rehydration_map};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p xberg --lib --features "redaction-rehydrate,ner,ner-candle" text::redaction::engine::tests -- --nocapture`
Expected: PASS — all tests in `text::redaction::engine::tests`, including the two new ones and every pre-existing test in this module (`test_dedupe_overlaps_keeps_longer_first`, `test_apply_replacements_reverse`).

- [ ] **Step 5: Commit**

```bash
git add crates/xberg/src/text/redaction/engine.rs crates/xberg/src/text/redaction/mod.rs
git commit -m "feat(xberg): add redact_text_capturing_rehydration_map for plain-text PII+NER redaction"
```

---

### Task 2: `pipeline-redaction` feature in `xberg-rag`

**Files:**
- Modify: `crates/xberg-rag/Cargo.toml`
- Modify: `crates/xberg-rag/src/pipeline.rs`
- Test: same file, `#[cfg(all(test, feature = "in-memory"))] mod tests`

**Interfaces:**
- Consumes: `xberg::text::redaction::redact_text_capturing_rehydration_map` (Task 1), `xberg::text::ner::NerBackend` trait, `xberg::types::redaction::RedactionStrategy`, `xberg::text::redaction::RehydrationMap`.
- Produces: `pub struct IngestOutcome { pub document_id: DocumentId, pub rehydration_map: xberg::text::redaction::RehydrationMap, pub pii_category_counts: HashMap<String, usize> }`. New signatures for `ingest_document`/`ingest_document_local` under `#[cfg(feature = "pipeline-redaction")]`: both gain a 6th parameter `ner: &dyn xberg::text::ner::NerBackend` and return `RagResult<IngestOutcome>` instead of `RagResult<DocumentId>`. Task 3 (`xberg-wasm::engine.rs`) calls these new signatures directly.

- [ ] **Step 1: Add the feature to Cargo.toml**

In `crates/xberg-rag/Cargo.toml`, change:

```toml
pipeline-ner-llm = ["pipeline", "xberg/ner-llm"]
pipeline-ner-onnx = ["pipeline", "xberg/ner-onnx"]
```

to:

```toml
pipeline-ner-llm = ["pipeline", "xberg/ner-llm"]
pipeline-ner-onnx = ["pipeline", "xberg/ner-onnx"]
pipeline-redaction = ["pipeline", "xberg/redaction-rehydrate", "xberg/ner"]
```

And add it to the `full` aggregate — change:

```toml
full = [
    "vector-store",
    "pipeline",
    "pipeline-embeddings",
    "pipeline-reranker",
    "pipeline-keywords",
    "pipeline-ner-llm",
    "streaming",
    "in-memory",
    "sqlite",
]
```

to:

```toml
full = [
    "vector-store",
    "pipeline",
    "pipeline-embeddings",
    "pipeline-reranker",
    "pipeline-keywords",
    "pipeline-ner-llm",
    "pipeline-redaction",
    "streaming",
    "in-memory",
    "sqlite",
]
```

- [ ] **Step 2: Write the failing test**

Append to the `mod tests` block at the bottom of `crates/xberg-rag/src/pipeline.rs` (after `ingest_document_local_delegates_to_ingest_document`, before the closing `}` of `mod tests`):

```rust
    #[cfg(feature = "pipeline-redaction")]
    struct StubNerBackend;

    #[cfg(feature = "pipeline-redaction")]
    #[async_trait]
    impl xberg::text::ner::NerBackend for StubNerBackend {
        async fn detect(
            &self,
            text: &str,
            _categories: &[xberg::types::entity::EntityCategory],
        ) -> xberg::Result<Vec<xberg::types::entity::Entity>> {
            if let Some(pos) = text.find("Alice") {
                Ok(vec![xberg::types::entity::Entity {
                    category: xberg::types::entity::EntityCategory::Person,
                    text: "Alice".to_string(),
                    start: pos as u32,
                    end: (pos + 5) as u32,
                    confidence: Some(0.99),
                }])
            } else {
                Ok(vec![])
            }
        }
    }

    #[cfg(feature = "pipeline-redaction")]
    #[tokio::test]
    async fn ingest_document_redacts_pii_and_returns_rehydration_map() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("pii-test");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };
        let ner = StubNerBackend;

        let request = IngestRequest {
            full_text: "Contact Alice at alice@example.com for details.".to_string(),
            ..Default::default()
        };

        let outcome = ingest_document(Arc::clone(&store), "docs", request, &config, &embedder, &ner)
            .await
            .unwrap();

        assert!(!outcome.document_id.0.is_empty());
        assert_eq!(outcome.pii_category_counts.get("Email"), Some(&1));
        assert_eq!(outcome.pii_category_counts.get("Person"), Some(&1));
        assert_eq!(outcome.rehydration_map.len(), 2);
        assert!(outcome.rehydration_map.values().any(|v| v == "alice@example.com"));
        assert!(outcome.rehydration_map.values().any(|v| v == "Alice"));
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p xberg-rag --features "in-memory,pipeline-redaction" ingest_document_redacts_pii -- --nocapture`
Expected: FAIL with a type error — `ingest_document` called with 6 arguments but the current signature only accepts 5, and returns `DocumentId` not something with a `.document_id` field.

- [ ] **Step 4: Add `IngestOutcome` and wire redaction into both ingest functions**

In `crates/xberg-rag/src/pipeline.rs`, add this new struct right after `IngestRequest` (after its closing `}`, before the `// ─── RagPipelineConfig ───` section header):

```rust
/// Result of ingesting one document when `pipeline-redaction` is enabled.
///
/// The pipeline never persists or encrypts `rehydration_map` itself — the
/// caller decides what to do with it (e.g. `xberg-wasm`'s `engine.ingest()`
/// returns it to JS as-is; a separate, unchanged `encryptMap` method exists
/// for callers that want to persist an encrypted copy).
#[cfg(feature = "pipeline-redaction")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct IngestOutcome {
    /// The [`DocumentId`] assigned by the store.
    pub document_id: DocumentId,
    /// Token → original-text map. This pipeline always uses `TokenReplace`,
    /// so every PII finding produces an entry.
    pub rehydration_map: xberg::text::redaction::RehydrationMap,
    /// Per-category finding counts (e.g. `{"Email": 2, "Person": 1}`) —
    /// counts only, never the matched text itself.
    pub pii_category_counts: std::collections::HashMap<String, usize>,
}

/// Detect and redact PII (regex + NER) in `request.full_text`, returning a
/// copy of `request` with `full_text` replaced by the redacted text, plus the
/// rehydration map and category counts produced along the way. Shared by
/// both [`ingest_document`] and the wasm32 [`ingest_document_local`].
#[cfg(feature = "pipeline-redaction")]
async fn redact_request(
    request: IngestRequest,
    ner: &dyn xberg::text::ner::NerBackend,
) -> RagResult<(
    IngestRequest,
    xberg::text::redaction::RehydrationMap,
    std::collections::HashMap<String, usize>,
)> {
    let outcome = xberg::text::redaction::redact_text_capturing_rehydration_map(
        &request.full_text,
        xberg::types::redaction::RedactionStrategy::TokenReplace,
        ner,
    )
    .await
    .map_err(RagError::Core)?;

    let redacted_request = IngestRequest {
        full_text: outcome.redacted_text,
        ..request
    };
    Ok((redacted_request, outcome.rehydration_map, outcome.category_counts))
}
```

Then change the `ingest_document` signature. Replace:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_document(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
```

with:

```rust
#[cfg(all(not(target_arch = "wasm32"), not(feature = "pipeline-redaction")))]
pub async fn ingest_document(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
```

(the body of this function, up to and including its closing `}`, is otherwise **unchanged** — only the `#[cfg(...)]` line above it changes).

Immediately after that function's closing `}`, add the `pipeline-redaction` variant:

```rust
#[cfg(all(not(target_arch = "wasm32"), feature = "pipeline-redaction"))]
pub async fn ingest_document(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
    ner: &dyn xberg::text::ner::NerBackend,
) -> RagResult<IngestOutcome> {
    let (request, rehydration_map, pii_category_counts) = redact_request(request, ner).await?;

    let text = request.full_text.clone();
    let chunking_config = config.chunking.clone();

    let chunks = tokio::task::spawn_blocking(move || xberg::chunking::chunk_for_rag(&text, &chunking_config))
        .await
        .map_err(|e| RagError::Backend(Box::new(e)))?
        .map_err(RagError::Core)?
        .chunks;

    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = embedder.embed(texts).await?;

    if embeddings.len() != chunks.len() {
        return Err(RagError::EmbeddingCountMismatch {
            expected: chunks.len(),
            got: embeddings.len(),
        });
    }

    let chunk_records: Vec<ChunkRecord> = chunks
        .into_iter()
        .zip(embeddings)
        .enumerate()
        .map(|(i, (chunk, emb))| chunk_to_record(chunk, i as u32, emb))
        .collect();

    let document = DocumentRecord {
        external_id: request.external_id,
        title: request.title,
        mime: request.mime,
        source_uri: request.source_uri,
        full_text: request.full_text,
        keywords: request.keywords,
        entities: request.entities,
        labels: request.labels,
        metadata: request.metadata,
    };

    let document_id = store.upsert_document(collection, &document, &chunk_records).await?;
    Ok(IngestOutcome {
        document_id,
        rehydration_map,
        pii_category_counts,
    })
}
```

Now update `ingest_document_local`. Replace the existing non-wasm32 variant:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
    ingest_document(store, collection, request, config, embedder).await
}
```

with two cfg-split variants:

```rust
#[cfg(all(not(target_arch = "wasm32"), not(feature = "pipeline-redaction")))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
    ingest_document(store, collection, request, config, embedder).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "pipeline-redaction"))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
    ner: &dyn xberg::text::ner::NerBackend,
) -> RagResult<IngestOutcome> {
    ingest_document(store, collection, request, config, embedder, ner).await
}
```

And replace the wasm32 variant:

```rust
#[cfg(target_arch = "wasm32")]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
```

with:

```rust
#[cfg(all(target_arch = "wasm32", not(feature = "pipeline-redaction")))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
```

(body unchanged), then immediately after its closing `}`, add:

```rust
#[cfg(all(target_arch = "wasm32", feature = "pipeline-redaction"))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
    ner: &dyn xberg::text::ner::NerBackend,
) -> RagResult<IngestOutcome> {
    let (request, rehydration_map, pii_category_counts) = redact_request(request, ner).await?;

    let chunks = xberg::chunking::chunk_for_rag(&request.full_text, config.chunking)
        .map_err(RagError::Core)?
        .chunks;

    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = embedder.embed(texts).await?;

    if embeddings.len() != chunks.len() {
        return Err(RagError::EmbeddingCountMismatch {
            expected: chunks.len(),
            got: embeddings.len(),
        });
    }

    let chunk_records: Vec<ChunkRecord> = chunks
        .into_iter()
        .zip(embeddings)
        .enumerate()
        .map(|(i, (chunk, emb))| chunk_to_record(chunk, i as u32, emb))
        .collect();

    let document = DocumentRecord {
        external_id: request.external_id,
        title: request.title,
        mime: request.mime,
        source_uri: request.source_uri,
        full_text: request.full_text,
        keywords: request.keywords,
        entities: request.entities,
        labels: request.labels,
        metadata: request.metadata,
    };

    let document_id = store.upsert_document(collection, &document, &chunk_records).await?;
    Ok(IngestOutcome {
        document_id,
        rehydration_map,
        pii_category_counts,
    })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p xberg-rag --features "in-memory,pipeline-redaction" -- --nocapture`
Expected: PASS — the new `ingest_document_redacts_pii_and_returns_rehydration_map` test, plus every pre-existing test in `pipeline.rs` (`ingest_document_returns_document_id`, `ingest_rejects_embedder_count_mismatch`, `retrieve_embeds_query_when_no_vector_provided`, `chunk_to_record_maps_ordinal_and_content`, `ingest_document_local_delegates_to_ingest_document`).

Also run without the new feature to confirm zero regression for existing callers:

Run: `cargo test -p xberg-rag --features "in-memory" -- --nocapture`
Expected: PASS — same pre-existing tests, `pipeline-redaction` test skipped (feature off).

- [ ] **Step 6: Commit**

```bash
git add crates/xberg-rag/Cargo.toml crates/xberg-rag/src/pipeline.rs
git commit -m "feat(xberg-rag): add pipeline-redaction feature for mandatory PII+NER ingest"
```

---

### Task 3: Thread the Candle PII model through `xberg-wasm::engine.ingest()`

**Files:**
- Modify: `alef.toml:566`
- Regenerate: `crates/xberg-wasm/Cargo.toml` (via `task alef:generate` — do not hand-edit)
- Modify: `crates/xberg-wasm/src/bridge/ner.rs`
- Modify: `crates/xberg-wasm/src/engine.rs`

**Interfaces:**
- Consumes: `xberg_rag::pipeline::{ingest_document_local, IngestOutcome}` (Task 2, `pipeline-redaction` variant), the existing `CANDLE_NER` thread-local in `bridge/ner.rs`.
- Produces: `pub(crate) fn get_candle_ner() -> Option<std::rc::Rc<CandleBackend>>` in `bridge/ner.rs`, consumed by `engine.rs`. `engine.ingest()`'s JS-visible return shape changes from a bare document id string to `{ documentId, rehydrationMap, piiCategoryCounts }` (exact JSON key casing determined by `serde` — `IngestOutcome`'s fields are `document_id`/`rehydration_map`/`pii_category_counts`, no `rename_all`, so the JS object keys are literally snake_case: `document_id`, `rehydration_map`, `pii_category_counts`). Task 4 (`ingestFolder()`) consumes this shape.

- [ ] **Step 1: Edit `alef.toml` to add the new feature**

In `alef.toml`, change line 566 from:

```toml
xberg-rag = { path = "../xberg-rag", default-features = false, features = ["vector-store", "pipeline"] }
```

to:

```toml
xberg-rag = { path = "../xberg-rag", default-features = false, features = ["vector-store", "pipeline", "pipeline-redaction"] }
```

- [ ] **Step 2: Regenerate and verify**

Run: `task alef:generate`
Then: `git diff crates/xberg-wasm/Cargo.toml`
Expected: the diff shows only the `xberg-rag` features array gaining `"pipeline-redaction"`, plus the auto-updated `alef:hash:` comment at the top of the file. No other lines change.

- [ ] **Step 3: Add the Candle backend accessor**

In `crates/xberg-wasm/src/bridge/ner.rs`, add this function immediately after `init_candle_ner` (after its closing `}`, before the `resolve_ner` doc comment):

```rust
/// Return the currently-loaded Candle NER backend, if `initCandleNer` has
/// been called. Used by `engine.rs::ingest()` to thread the already-loaded
/// model into `xberg-rag`'s mandatory PII+NER redaction step.
pub(crate) fn get_candle_ner() -> Option<std::rc::Rc<CandleBackend>> {
    CANDLE_NER.with(|cell| cell.borrow().clone())
}
```

- [ ] **Step 4: Update `engine.rs::ingest()`**

In `crates/xberg-wasm/src/engine.rs`, replace the entire `ingest` method:

```rust
    /// Ingest a single document into the RAG vector store.
    ///
    /// Requires both an `embedder` and a `store` to have been injected.
    /// `config` is an optional object; only `chunking.maxCharacters` and
    /// `chunking.overlap` are currently supported. All other fields are
    /// ignored.
    #[allow(clippy::missing_errors_doc)]
    pub async fn ingest(
        &self,
        doc: JsValue,
        collection: String,
        config: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let ingest_req: xberg_rag::pipeline::IngestRequest =
            serde_wasm_bindgen::from_value(doc)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let embedder = self
            .embedder
            .as_ref()
            .ok_or_else(|| JsValue::from_str("embedder not injected"))?;
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| JsValue::from_str("store not injected"))?;

        let chunking = match config {
            Some(c) if !c.is_undefined() && !c.is_null() => {
                let c_obj: Object = c
                    .dyn_into()
                    .map_err(|_| JsValue::from_str("config must be an object"))?;
                match get_opt_field(&c_obj, "chunking")? {
                    Some(chunking_obj) => {
                        let mut cfg = xberg::ChunkingConfig::default();
                        if let Some(n) = get_opt_number(&chunking_obj, "maxCharacters")? {
                            cfg.max_characters = n as usize;
                        }
                        if let Some(n) = get_opt_number(&chunking_obj, "overlap")? {
                            cfg.overlap = n as usize;
                        }
                        cfg
                    }
                    None => xberg::ChunkingConfig::default(),
                }
            }
            _ => xberg::ChunkingConfig::default(),
        };
        let pipeline_config = xberg_rag::pipeline::RagPipelineConfig { chunking: &chunking };
        let result = xberg_rag::pipeline::ingest_document_local(
            store.clone(),
            &collection,
            ingest_req,
            &pipeline_config,
            embedder.as_ref(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
```

with:

```rust
    /// Ingest a single document into the RAG vector store.
    ///
    /// Requires an `embedder` and a `store` to have been injected, and
    /// requires `initCandleNer` to have been called first — PII+NER
    /// redaction is mandatory and always runs before chunking. `config` is
    /// an optional object; only `chunking.maxCharacters` and
    /// `chunking.overlap` are currently supported. All other fields are
    /// ignored.
    ///
    /// Returns `{ document_id, rehydration_map, pii_category_counts }`. The
    /// caller decides whether/how to persist or encrypt `rehydration_map` —
    /// this method never does so itself (use `encryptMap` separately).
    #[allow(clippy::missing_errors_doc)]
    pub async fn ingest(
        &self,
        doc: JsValue,
        collection: String,
        config: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let ingest_req: xberg_rag::pipeline::IngestRequest =
            serde_wasm_bindgen::from_value(doc)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let embedder = self
            .embedder
            .as_ref()
            .ok_or_else(|| JsValue::from_str("embedder not injected"))?;
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| JsValue::from_str("store not injected"))?;
        let ner_backend = crate::bridge::ner::get_candle_ner().ok_or_else(|| {
            JsValue::from_str("PII detection unavailable: initCandleNer has not been called")
        })?;

        let chunking = match config {
            Some(c) if !c.is_undefined() && !c.is_null() => {
                let c_obj: Object = c
                    .dyn_into()
                    .map_err(|_| JsValue::from_str("config must be an object"))?;
                match get_opt_field(&c_obj, "chunking")? {
                    Some(chunking_obj) => {
                        let mut cfg = xberg::ChunkingConfig::default();
                        if let Some(n) = get_opt_number(&chunking_obj, "maxCharacters")? {
                            cfg.max_characters = n as usize;
                        }
                        if let Some(n) = get_opt_number(&chunking_obj, "overlap")? {
                            cfg.overlap = n as usize;
                        }
                        cfg
                    }
                    None => xberg::ChunkingConfig::default(),
                }
            }
            _ => xberg::ChunkingConfig::default(),
        };
        let pipeline_config = xberg_rag::pipeline::RagPipelineConfig { chunking: &chunking };
        let result = xberg_rag::pipeline::ingest_document_local(
            store.clone(),
            &collection,
            ingest_req,
            &pipeline_config,
            embedder.as_ref(),
            ner_backend.as_ref(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
```

- [ ] **Step 5: Verify it compiles for the wasm32 target**

Run: `cargo check -p xberg-wasm --target wasm32-unknown-unknown --no-default-features --features "wasm-target,url-ingestion"`
Expected: compiles cleanly (this exercises `xberg`'s side; `xberg-rag`'s `pipeline-redaction` is pulled in transitively via the Task 2 Cargo.toml change plus this task's Step 2 regeneration — no separate feature flag needed on the `cargo check` invocation itself since it's baked into `xberg-wasm`'s own `Cargo.toml` dependency declaration).

Run: `cargo test -p xberg-wasm --lib` (native-target compile check only — `xberg-wasm` has `crate-type = ["cdylib"]` with no dev-dependencies today, so this validates compilation, not runtime behavior; runtime behavior is exercised end-to-end in Task 4 via the compiled wasm binary).
Expected: compiles cleanly, no test binary to run (0 tests).

- [ ] **Step 6: Commit**

```bash
git add alef.toml crates/xberg-wasm/Cargo.toml crates/xberg-wasm/src/bridge/ner.rs crates/xberg-wasm/src/engine.rs
git commit -m "feat(xberg-wasm): require PII+NER redaction in engine.ingest()"
```

---

### Task 4: Shared `ingestFolder()` orchestrator in `xberg-wasm-runtime`

**Files:**
- Create: `packages/xberg-wasm-runtime/src/ingest-folder.ts`
- Create: `packages/xberg-wasm-runtime/src/ingest-folder.test.ts`
- Modify: `packages/xberg-wasm-runtime/src/index.ts`

**Interfaces:**
- Consumes: nothing from earlier tasks directly (this package doesn't depend on `@xberg-io/xberg-wasm`) — it defines its own minimal structural interface, `XbergEngineLike`, matching the wire shapes Task 3 produces: `extract()` takes `{ kind: "bytes", bytes: number[], filename?: string }` (matches `xberg::ExtractInput`'s literal snake_case JSON — no `rename_all` on that struct) and `ingest()` takes `{ full_text, title?, mime?, source_uri? }` / returns `{ document_id, rehydration_map, pii_category_counts }` (matches `xberg_rag::pipeline::IngestRequest`/`IngestOutcome`'s literal snake_case JSON — neither struct has `rename_all` either).
- Produces: `ingestFolder(engine: XbergEngineLike, collection: string, files: FolderFileSource[]): Promise<IngestFolderFileResult[]>`, exported from `index.ts`. This is what a future `mcp-server` retarget (Sub-project E) and browser UI (Sub-project D) will both call — not built in this plan.

Note on `extract()`'s result shape: `WasmExtractedDocument`'s exact JSON field name for document text could not be confirmed by static reading alone (the struct's `Serialize` impl, if any, lives somewhere in a very large generated-feeling `crates/xberg-wasm/src/lib.rs` this plan does not otherwise touch). The implementation below reads `content` first (the Rust field's literal name) — verify this against a real compiled `xberg-wasm` build before wiring `ingestFolder()` into a real consumer; the in-memory test in this task uses a stub engine and does not exercise the real wasm binary.

- [ ] **Step 1: Write the failing test**

Create `packages/xberg-wasm-runtime/src/ingest-folder.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { ingestFolder, type XbergEngineLike } from "./ingest-folder.js";

function makeStubEngine(behavior: {
	extractText?: (filename: string) => string | null;
	failIngestFor?: string[];
}): XbergEngineLike {
	return {
		extract: async (input) => {
			const filename = input.filename ?? "";
			const text = behavior.extractText ? behavior.extractText(filename) : "stub content";
			if (text === null) return { results: [] };
			return { results: [{ content: text, mimeType: "text/plain" }] };
		},
		ingest: async (doc, _collection) => {
			if (behavior.failIngestFor?.includes(doc.title ?? "")) {
				throw new Error("simulated ingest failure");
			}
			return {
				document_id: `doc-${doc.title}`,
				rehydration_map: doc.full_text.includes("Alice") ? { "[PERSON_1]": "Alice" } : {},
				pii_category_counts: doc.full_text.includes("Alice") ? { Person: 1 } : {},
			};
		},
	};
}

describe("ingestFolder", () => {
	it("ingests every file and returns per-file results", async () => {
		const engine = makeStubEngine({ extractText: (name) => `Text from ${name}` });
		const files = [
			{ name: "a.txt", path: "/src/a.txt", bytes: new Uint8Array([1, 2, 3]) },
			{ name: "b.txt", path: "/src/b.txt", bytes: new Uint8Array([4, 5, 6]) },
		];

		const results = await ingestFolder(engine, "docs", files);

		expect(results).toHaveLength(2);
		expect(results[0]).toMatchObject({ filename: "a.txt", documentId: "doc-a.txt" });
		expect(results[1]).toMatchObject({ filename: "b.txt", documentId: "doc-b.txt" });
	});

	it("surfaces PII category counts and rehydration map per file", async () => {
		const engine = makeStubEngine({ extractText: (name) => (name === "alice.txt" ? "Hi Alice" : "no pii here") });
		const files = [{ name: "alice.txt", path: "/src/alice.txt", bytes: new Uint8Array([1]) }];

		const results = await ingestFolder(engine, "docs", files);

		expect(results[0]?.piiCategoryCounts).toEqual({ Person: 1 });
		expect(results[0]?.rehydrationMap).toEqual({ "[PERSON_1]": "Alice" });
	});

	it("records a per-file error and continues the batch when one file fails", async () => {
		const engine = makeStubEngine({ extractText: () => "content", failIngestFor: ["bad.txt"] });
		const files = [
			{ name: "bad.txt", path: "/src/bad.txt", bytes: new Uint8Array([1]) },
			{ name: "good.txt", path: "/src/good.txt", bytes: new Uint8Array([2]) },
		];

		const results = await ingestFolder(engine, "docs", files);

		expect(results).toHaveLength(2);
		expect(results[0]).toMatchObject({ filename: "bad.txt", documentId: null, error: "simulated ingest failure" });
		expect(results[1]).toMatchObject({ filename: "good.txt", documentId: "doc-good.txt" });
	});

	it("records an error when extraction produces no document", async () => {
		const engine = makeStubEngine({ extractText: () => null });
		const files = [{ name: "empty.bin", path: "/src/empty.bin", bytes: new Uint8Array([]) }];

		const results = await ingestFolder(engine, "docs", files);

		expect(results[0]).toMatchObject({ filename: "empty.bin", documentId: null });
		expect(results[0]?.error).toMatch(/no document/);
	});
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter xberg-wasm-runtime test:run ingest-folder`
Expected: FAIL — `Cannot find module './ingest-folder.js'`.

- [ ] **Step 3: Implement `ingest-folder.ts`**

Create `packages/xberg-wasm-runtime/src/ingest-folder.ts`:

```typescript
/**
 * Host-agnostic folder ingest orchestrator, shared between MCP and the
 * browser app so both drive the exact same extract -> PII+NER redact ->
 * chunk -> embed -> store sequence via the wasm engine. No filesystem
 * access happens in this file — callers supply file bytes already read
 * into memory, and are responsible for any file-output side effects
 * (writing redacted copies, reports, rehydration-map files) themselves.
 */

export interface ExtractInput {
	kind: "bytes";
	bytes: number[];
	filename?: string;
}

export interface ExtractedDocumentLike {
	content?: string;
	mimeType?: string;
}

export interface IngestDoc {
	full_text: string;
	title?: string;
	mime?: string;
	source_uri?: string;
}

export interface EngineIngestOutcome {
	document_id: string;
	rehydration_map: Record<string, string>;
	pii_category_counts: Record<string, number>;
}

export interface XbergEngineLike {
	extract(input: ExtractInput, config?: unknown): Promise<{ results?: ExtractedDocumentLike[] }>;
	ingest(doc: IngestDoc, collection: string, config?: unknown): Promise<EngineIngestOutcome>;
}

export interface FolderFileSource {
	name: string;
	path: string;
	bytes: Uint8Array;
}

export interface IngestFolderFileResult {
	filename: string;
	documentId: string | null;
	piiCategoryCounts: Record<string, number>;
	rehydrationMap: Record<string, string>;
	error?: string;
}

export async function ingestFolder(
	engine: XbergEngineLike,
	collection: string,
	files: FolderFileSource[],
): Promise<IngestFolderFileResult[]> {
	const results: IngestFolderFileResult[] = [];

	for (const file of files) {
		try {
			const extracted = await engine.extract({
				kind: "bytes",
				bytes: Array.from(file.bytes),
				filename: file.name,
			});
			const doc = extracted.results?.[0];
			if (!doc) {
				results.push({
					filename: file.name,
					documentId: null,
					piiCategoryCounts: {},
					rehydrationMap: {},
					error: "extraction produced no document",
				});
				continue;
			}

			const outcome = await engine.ingest(
				{
					full_text: doc.content ?? "",
					title: file.name,
					mime: doc.mimeType,
					source_uri: file.path,
				},
				collection,
			);

			results.push({
				filename: file.name,
				documentId: outcome.document_id,
				piiCategoryCounts: outcome.pii_category_counts,
				rehydrationMap: outcome.rehydration_map,
			});
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err);
			results.push({
				filename: file.name,
				documentId: null,
				piiCategoryCounts: {},
				rehydrationMap: {},
				error: message,
			});
		}
	}

	return results;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter xberg-wasm-runtime test:run ingest-folder`
Expected: PASS — all 4 tests in `ingest-folder.test.ts`.

- [ ] **Step 5: Export from the package barrel**

In `packages/xberg-wasm-runtime/src/index.ts`, change:

```typescript
export * from "./embedder.js";
export * from "./store.js";
export * from "./ner.js";
export * from "./ocr.js";
export * from "./cache.js";
export * from "./async_shim.js";
export { createXbergRuntimeFactory } from "./factory.js";
```

to:

```typescript
export * from "./embedder.js";
export * from "./store.js";
export * from "./ner.js";
export * from "./ocr.js";
export * from "./cache.js";
export * from "./async_shim.js";
export * from "./ingest-folder.js";
export { createXbergRuntimeFactory } from "./factory.js";
```

- [ ] **Step 6: Run the full package test suite and lint**

Run: `pnpm --filter xberg-wasm-runtime test:run`
Expected: PASS — no regressions in any existing test file.

Run: `pnpm --filter xberg-wasm-runtime lint`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add packages/xberg-wasm-runtime/src/ingest-folder.ts packages/xberg-wasm-runtime/src/ingest-folder.test.ts packages/xberg-wasm-runtime/src/index.ts
git commit -m "feat(wasm-runtime): add shared host-agnostic ingestFolder orchestrator"
```

---

## Self-Review Notes

- **Spec coverage:** Component 1 (core `redact()`) → Task 1 (implemented as `redact_text_capturing_rehydration_map`, reusing the codebase's *existing* `redact`/`redact_capturing_rehydration_map` machinery in the same file rather than duplicating it a third time — a refinement discovered during planning: the spec assumed no canonical redact function existed yet, but `engine.rs`'s `redact()` in `xberg-wasm` was duplicating logic that already existed in `xberg::text::redaction::engine`, just not in a form callable with a pre-loaded NER backend). Component 2 (pipeline-redaction) → Task 2. Component 3 (engine threading) → Task 3. Component 4 (ingestFolder) → Task 4. Non-Goals are respected — no `mcp-server/` or browser UI changes anywhere in this plan.
- **Placeholder scan:** No TBD/TODO markers; every step has complete, runnable code; the one open item (exact `WasmExtractedDocument` JSON field name) is explicitly flagged as a pre-integration verification step, not left as an unstated assumption.
- **Type consistency:** `IngestOutcome`'s Rust fields (`document_id`, `rehydration_map`, `pii_category_counts`) match `ingest-folder.ts`'s `EngineIngestOutcome` interface exactly (snake_case, matching `serde`'s default — neither struct has `rename_all`). `TextRedactionOutcome` (Task 1) and `IngestOutcome` (Task 2) both carry `rehydration_map`/`category_counts` (or `pii_category_counts`) with matching types (`RehydrationMap` = `HashMap<String,String>`, counts = `HashMap<String,usize>` in Rust / `Record<string, number>` in TS).
