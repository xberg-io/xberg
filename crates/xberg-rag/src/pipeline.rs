//! Ingest/retrieve orchestration composing xberg core primitives.
//!
//! The pipeline is the bridge between raw document content and a running
//! [`VectorStore`](crate::store::VectorStore): it chunks text via
//! `xberg::chunking::chunk_for_rag`, embeds the chunks through the caller-supplied
//! [`Embedder`], and upserts the resulting [`ChunkRecord`](crate::types::ChunkRecord)s
//! as one atomic unit.  Retrieval embeds the query text when required, then
//! delegates to the store.
//!
//! Optional convenience wiring is compiled behind narrow feature flags so that
//! callers who bring their own embedder or reranker incur zero pull-in of ORT
//! binaries:
//!
//! - `pipeline-embeddings` — [`CoreEmbedder`] backed by `xberg::embed_texts_async`
//! - `pipeline-reranker`   — [`rerank`] backed by `xberg::rerank_async`
//! - `pipeline-keywords`   — [`extract_keywords`] backed by `xberg::keywords::extract_keywords`

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{RagError, RagResult};
use crate::query::{RetrieveMode, RetrieveQuery};
use crate::store::VectorStore;
use crate::types::{ChunkRecord, DocumentId, DocumentRecord, RetrievedChunk};

#[cfg(feature = "pipeline-redaction")]
use serde_json::Value;
#[cfg(feature = "pipeline-redaction")]
use xberg::text::redaction::TokenCounter;
#[cfg(feature = "pipeline-redaction")]
use xberg::types::redaction::RedactionStrategy;

// ─── Helpers for full-field redaction ──────────────────────────────────────────

/// Keys whose string values are opaque identifiers rather than narrative text.
/// NER is skipped for these leaves to bound inference cost.
#[cfg(feature = "pipeline-redaction")]
const NON_NARRATIVE_KEYS: &[&str] = &[
    "id", "url", "uri", "hash", "sha", "email", "phone", "token", "type", "kind", "lang",
];

/// Heuristic: should a JSON string leaf get the NER+regex treatment (vs
/// regex-only)? Only free-text-shaped, longer leaves — short terms and
/// opaque-keyed values stay regex-only to bound NER cost.
#[cfg(feature = "pipeline-redaction")]
fn is_free_text_leaf(key: &str, text: &str) -> bool {
    if text.len() <= 20 {
        return false;
    }
    if NON_NARRATIVE_KEYS.iter().any(|k| key.eq_ignore_ascii_case(k)) {
        return false;
    }
    true
}

/// Async JSON redactor: applies NER+regex to free-text string leaves and
/// regex-only to opaque/short leaves, sharing the request-wide `TokenCounter`
/// and accumulators. Mirrors the fail-closed discipline of `redact_request`.
#[cfg(feature = "pipeline-redaction")]
fn redact_json_value_async<'a>(
    value: &'a mut Value,
    ner: &'a dyn xberg::text::ner::NerBackend,
    counter: &'a mut TokenCounter,
    rehydration_map: &'a mut xberg::text::redaction::RehydrationMap,
    category_counts: &'a mut std::collections::HashMap<String, usize>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = RagResult<()>> + Send + 'a>> {
    Box::pin(async move {
        match value {
            Value::String(s) => {
                if is_free_text_leaf("", s) {
                    let outcome = xberg::text::redaction::redact_text_capturing_rehydration_map(
                        s,
                        RedactionStrategy::TokenReplace,
                        ner,
                        counter,
                    )
                    .await
                    .map_err(RagError::Core)?;
                    rehydration_map.extend(outcome.rehydration_map);
                    for (category, count) in outcome.category_counts {
                        *category_counts.entry(category).or_insert(0) += count;
                    }
                    *s = outcome.redacted_text;
                } else {
                    *s = redact_string_sync(s, counter, rehydration_map, category_counts)?;
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    redact_json_value_async(v, ner, counter, rehydration_map, category_counts).await?;
                }
            }
            Value::Object(obj) => {
                for (key, v) in obj.iter_mut() {
                    if let Value::String(s) = v {
                        if is_free_text_leaf(key, s) {
                            let outcome = xberg::text::redaction::redact_text_capturing_rehydration_map(
                                s,
                                RedactionStrategy::TokenReplace,
                                ner,
                                counter,
                            )
                            .await
                            .map_err(RagError::Core)?;
                            rehydration_map.extend(outcome.rehydration_map);
                            for (category, count) in outcome.category_counts {
                                *category_counts.entry(category).or_insert(0) += count;
                            }
                            *s = outcome.redacted_text;
                        } else {
                            *s = redact_string_sync(s, counter, rehydration_map, category_counts)?;
                        }
                    } else {
                        redact_json_value_async(v, ner, counter, rehydration_map, category_counts).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })
}

/// Synchronous version for JSON values (regex patterns only, no NER).
#[cfg(feature = "pipeline-redaction")]
fn redact_string_sync(
    text: &str,
    counter: &mut TokenCounter,
    rehydration_map: &mut xberg::text::redaction::RehydrationMap,
    category_counts: &mut std::collections::HashMap<String, usize>,
) -> RagResult<String> {
    let matches = xberg::text::redaction::scan_text(text, &[]);
    let matches = xberg::text::redaction::dedupe_overlaps(matches);

    let mut result = String::new();
    let mut last_end = 0;

    for m in &matches {
        result.push_str(&text[last_end..m.start]);
        let replacement = xberg::text::redaction::apply_strategy(
            xberg::types::redaction::RedactionStrategy::TokenReplace,
            &m.text,
            &m.category,
            counter,
        );
        rehydration_map
            .entry(replacement.clone())
            .or_insert_with(|| m.text.clone());
        *category_counts.entry(format!("{:?}", m.category)).or_insert(0) += 1;
        result.push_str(&replacement);
        last_end = m.end;
    }
    result.push_str(&text[last_end..]);
    Ok(result)
}

// ─── IngestRequest ───────────────────────────────────────────────────────────

/// Embeds a batch of texts into dense float vectors.
///
/// Implementations must be `Send + Sync + 'static` off-wasm so they can be held
/// behind `Arc` and passed across thread and task boundaries. On wasm32, the
/// `?Send` bound allows non-Send futures (e.g., JSPI bridges over async JS).
///
/// # Errors
///
/// Returns [`RagError::Backend`] or [`RagError::Core`] on failure.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Embedder: 'static {
    /// Embed `texts`, returning one vector per input string.
    async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>>;
}

// ─── IngestRequest ───────────────────────────────────────────────────────────

/// Input for a single document ingestion.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IngestRequest {
    /// Full text of the document to chunk and embed.
    pub full_text: String,
    /// Optional human-readable title.
    pub title: Option<String>,
    /// Optional MIME type of the source document.
    pub mime: Option<String>,
    /// Optional source URI (file path, URL, object key — backend-opaque).
    pub source_uri: Option<String>,
    /// Optional caller-supplied external reference for idempotent upserts.
    pub external_id: Option<String>,
    /// Extracted keywords.
    pub keywords: Vec<String>,
    /// Named entities (free-form JSON).
    pub entities: serde_json::Value,
    /// Labels (free-form JSON).
    pub labels: serde_json::Value,
    /// Document-level metadata (free-form JSON).
    pub metadata: serde_json::Value,
}

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

/// Redact one secondary narrative-string field (regex + NER), merging its
/// findings into the request-wide `counter`/`rehydration_map`/`category_counts`
/// accumulators. See [`redact_request`] for why secondary string fields get
/// the same NER-aware treatment as `full_text`.
#[cfg(feature = "pipeline-redaction")]
async fn redact_secondary_string(
    text: &str,
    strategy: xberg::types::redaction::RedactionStrategy,
    ner: &dyn xberg::text::ner::NerBackend,
    counter: &mut TokenCounter,
    rehydration_map: &mut xberg::text::redaction::RehydrationMap,
    category_counts: &mut std::collections::HashMap<String, usize>,
) -> RagResult<String> {
    let outcome = xberg::text::redaction::redact_text_capturing_rehydration_map(text, strategy, ner, counter)
        .await
        .map_err(RagError::Core)?;
    rehydration_map.extend(outcome.rehydration_map);
    for (category, count) in outcome.category_counts {
        *category_counts.entry(category).or_insert(0) += count;
    }
    Ok(outcome.redacted_text)
}

/// Detect and redact PII (regex + NER) in ALL string fields of `request`,
/// returning a copy with all fields redacted, plus the rehydration map and
/// category counts. Shared by both [`ingest_document`] and the wasm32
/// [`ingest_document_local`].
#[cfg(feature = "pipeline-redaction")]
async fn redact_request(
    request: IngestRequest,
    ner: &dyn xberg::text::ner::NerBackend,
) -> RagResult<(
    IngestRequest,
    xberg::text::redaction::RehydrationMap,
    std::collections::HashMap<String, usize>,
)> {
    // One counter shared across full_text and every secondary field, so token
    // numbers stay unique request-wide. Starting a fresh counter per field
    // (the previous approach) let a category collide across fields — e.g.
    // full_text's [EMAIL_1] and title's [EMAIL_1] would both exist, and the
    // second `rehydration_map.entry(...).or_insert_with(...)` would silently
    // discard the title's original value, since the key already existed.
    let mut counter = TokenCounter::new();

    // Redact full_text once with NER + regex, getting the base outcome
    let outcome = xberg::text::redaction::redact_text_capturing_rehydration_map(
        &request.full_text,
        RedactionStrategy::TokenReplace,
        ner,
        &mut counter,
    )
    .await
    .map_err(RagError::Core)?;

    let mut rehydration_map = outcome.rehydration_map;
    let mut category_counts = outcome.category_counts;

    // title/source_uri are single narrative strings — exactly the shape a
    // person's name (NER, not regex-detectable) is likely to appear in (e.g. a
    // filename-derived title "Report for Alice Smith"). Route them through the
    // same NER+regex path as full_text, sharing the one counter. `external_id`
    // is preserved unchanged below as the caller-supplied idempotency key.
    let title = match request.title {
        Some(s) => Some(
            redact_secondary_string(
                &s,
                RedactionStrategy::TokenReplace,
                ner,
                &mut counter,
                &mut rehydration_map,
                &mut category_counts,
            )
            .await?,
        ),
        None => None,
    };
    let source_uri = match request.source_uri {
        Some(s) => Some(
            redact_secondary_string(
                &s,
                RedactionStrategy::TokenReplace,
                ner,
                &mut counter,
                &mut rehydration_map,
                &mut category_counts,
            )
            .await?,
        ),
        None => None,
    };
    // `external_id` is the caller-supplied idempotency key for upserts. Routing
    // it through `redact_secondary_string` would replace PII with an
    // order-dependent token (e.g. `[EMAIL_1]`) that a re-ingest of the *same*
    // id would not reproduce, silently breaking idempotency. Preserve it
    // unchanged — it is an opaque key, not narrative text, and callers must
    // supply a non-PII id. (Rejecting PII-bearing ids is handled at the
    // ingestion boundary if stricter guarantees are required.)
    let external_id = request.external_id;

    // Keywords are already curated, short terms (not arbitrary blobs), so each
    // one always gets full NER+regex treatment regardless of length — a bare
    // name like "Alice" must still be caught. entities/labels/metadata are
    // unbounded JSON leaves; to avoid a NER inference call per array element /
    // JSON leaf (which does not scale with metadata size), those only apply
    // NER to leaves that look like free text (length > 20 chars and a
    // non-opaque key), staying regex-only otherwise. This closes the gap
    // where a person/org/location PII in a long narrative metadata value
    // slipped past regex and persisted raw.
    let mut keywords = Vec::new();
    for kw in request.keywords {
        // Fail closed: a redaction error must not persist the raw keyword.
        let redacted = redact_secondary_string(
            &kw,
            RedactionStrategy::TokenReplace,
            ner,
            &mut counter,
            &mut rehydration_map,
            &mut category_counts,
        )
        .await?;
        keywords.push(redacted);
    }

    // Fail closed: propagate any redaction error instead of falling back to the
    // unredacted value, which could silently persist PII into structured fields.
    let mut entities = request.entities;
    redact_json_value_async(
        &mut entities,
        ner,
        &mut counter,
        &mut rehydration_map,
        &mut category_counts,
    )
    .await?;

    let mut labels = request.labels;
    redact_json_value_async(
        &mut labels,
        ner,
        &mut counter,
        &mut rehydration_map,
        &mut category_counts,
    )
    .await?;

    let mut metadata = request.metadata;
    redact_json_value_async(
        &mut metadata,
        ner,
        &mut counter,
        &mut rehydration_map,
        &mut category_counts,
    )
    .await?;

    let redacted_request = IngestRequest {
        full_text: outcome.redacted_text, // reuse the already-redacted text
        title,
        mime: request.mime,
        source_uri,
        external_id,
        keywords,
        entities,
        labels,
        metadata,
    };

    Ok((redacted_request, rehydration_map, category_counts))
}

// ─── RagPipelineConfig ───────────────────────────────────────────────────────

/// Configuration for the ingest/retrieve pipeline.
pub struct RagPipelineConfig<'a> {
    /// Chunking configuration forwarded to `xberg::chunking::chunk_for_rag`.
    pub chunking: &'a xberg::ChunkingConfig,
}

// ─── chunk_to_record ─────────────────────────────────────────────────────────

/// Convert one [`xberg::Chunk`] into a [`ChunkRecord`] ready for upsertion.
///
/// `ordinal` is the 0-based position within the parent document. `embedding`
/// must match the collection's declared dimension; the caller is responsible
/// for ensuring this.
///
/// `ChunkMetadata` serialisation is infallible for well-formed input; any
/// edge-case error yields a `serde_json::Value::Null` object rather than
/// panicking.
pub fn chunk_to_record(chunk: xberg::Chunk, ordinal: u32, embedding: Vec<f32>) -> ChunkRecord {
    let chunk_metadata = serde_json::to_value(&chunk.metadata).unwrap_or_default();
    ChunkRecord {
        external_id: None,
        ordinal,
        content: chunk.content,
        embedding,
        chunk_metadata,
    }
}

// ─── ingest_document ─────────────────────────────────────────────────────────

/// Chunk, embed, and upsert a document into `collection`.
///
/// Steps:
/// 1. Chunk `request.full_text` via `xberg::chunking::chunk_for_rag`, offloaded
///    to a blocking thread via [`tokio::task::spawn_blocking`].
/// 2. Embed all chunk texts in one batch call to `embedder`.
/// 3. Pair each chunk with its embedding and upsert the document atomically via
///    [`VectorStore::upsert_document`].
///
/// Returns the [`DocumentId`] assigned by the store.
///
/// # Errors
///
/// Propagates chunking, embedding, or store errors wrapped in
/// [`RagError`].
#[cfg(all(not(target_arch = "wasm32"), not(feature = "pipeline-redaction")))]
pub async fn ingest_document(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
    let text = request.full_text.clone();
    let chunking_config = config.chunking.clone();

    let chunks = tokio::task::spawn_blocking(move || xberg::chunking::chunk_for_rag(&text, &chunking_config))
        .await
        .map_err(|e| RagError::Backend(Box::new(e)))?
        .map_err(RagError::Core)?
        .chunks;

    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = embedder.embed(texts).await?;

    // Guard the embedder contract: zip would silently drop chunks on a mismatch.
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

    store.upsert_document(collection, &document, &chunk_records).await
}

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

/// Like [`ingest_document`] but alias for the same codepath on non-wasm32;
/// delegates directly to [`ingest_document`] since both use `spawn_blocking`.
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

/// Like [`ingest_document`] but alias for the same codepath on non-wasm32;
/// delegates directly to [`ingest_document`] since both use `spawn_blocking`.
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

/// Like [`ingest_document`] but chunks inline (no `tokio::task::spawn_blocking`),
/// so it compiles and runs on `wasm32` where the multi-thread runtime is absent.
#[cfg(all(target_arch = "wasm32", not(feature = "pipeline-redaction")))]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
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

    store.upsert_document(collection, &document, &chunk_records).await
}

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

// ─── retrieve ────────────────────────────────────────────────────────────────

/// Retrieve chunks for `query` from `collection`.
///
/// When the query mode requires a vector ([`RetrieveMode::Vector`] or
/// [`RetrieveMode::Hybrid`]) and no pre-computed `query_vector` is set,
/// `embedder` is used to embed `query_text`.  If `embedder` is `None` and no
/// vector was supplied, the store will receive the query as-is and may return
/// an error if it cannot serve the mode without a vector.
///
/// Returns the [`RetrievedChunk`]s in descending relevance order.
///
/// # Errors
///
/// Propagates embedding or store errors.
pub async fn retrieve(
    store: Arc<dyn VectorStore>,
    collection: &str,
    mut query: RetrieveQuery,
    embedder: Option<&dyn Embedder>,
) -> RagResult<Vec<RetrievedChunk>> {
    if query.query_vector.is_none() {
        let needs_embedding = matches!(query.mode, RetrieveMode::Vector | RetrieveMode::Hybrid);
        if needs_embedding && let (Some(embedder), Some(text)) = (embedder, &query.query_text) {
            let mut vecs = embedder.embed(vec![text.clone()]).await?;
            query.query_vector = vecs.pop();
        }
    }
    let output = store.retrieve(collection, &query).await?;
    Ok(output.chunks)
}

// ─── CoreEmbedder ────────────────────────────────────────────────────────────

/// Embedder backed by `xberg::embed_texts_async`.
///
/// Requires the `pipeline-embeddings` feature, which enables ONNX Runtime.
#[cfg(feature = "pipeline-embeddings")]
pub struct CoreEmbedder {
    /// Embedding model configuration.
    pub config: xberg::EmbeddingConfig,
}

#[cfg(feature = "pipeline-embeddings")]
#[async_trait]
impl Embedder for CoreEmbedder {
    async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
        xberg::embed_texts_async(texts, &self.config)
            .await
            .map_err(RagError::Core)
    }
}

// ─── rerank ──────────────────────────────────────────────────────────────────

/// Rerank retrieved chunks using `xberg::rerank_async`.
///
/// Returns `output` reordered by descending reranker score.  Returns the
/// input unchanged when `output` is empty.
///
/// Requires the `pipeline-reranker` feature, which enables ONNX Runtime.
///
/// # Errors
///
/// Propagates reranking errors.
#[cfg(feature = "pipeline-reranker")]
pub async fn rerank(
    query: &str,
    output: Vec<RetrievedChunk>,
    config: &xberg::RerankerConfig,
) -> RagResult<Vec<RetrievedChunk>> {
    if output.is_empty() {
        return Ok(output);
    }

    let docs: Vec<String> = output.iter().map(|c| c.content.clone().unwrap_or_default()).collect();

    let results = xberg::rerank_async(query.to_string(), docs, config)
        .await
        .map_err(RagError::Core)?;

    let mut scored: Vec<(usize, f32)> = results.iter().map(|r| (r.index, r.score)).collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let reranked = scored
        .into_iter()
        .filter_map(|(idx, _)| output.get(idx).cloned())
        .collect();

    Ok(reranked)
}

// ─── extract_keywords ────────────────────────────────────────────────────────

/// Extract keywords from `text` using `xberg::keywords::extract_keywords`.
///
/// Returns keyword strings sorted by descending relevance score.
///
/// Requires the `pipeline-keywords` feature.
///
/// # Errors
///
/// Propagates extraction errors.
#[cfg(feature = "pipeline-keywords")]
pub fn extract_keywords(text: &str, config: &xberg::KeywordConfig) -> RagResult<Vec<String>> {
    xberg::keywords::extract_keywords(text, config)
        .map(|kws| kws.into_iter().map(|k| k.text).collect())
        .map_err(RagError::Core)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "in-memory"))]
mod tests {
    use super::*;
    use crate::backends::memory::InMemoryVectorStore;
    use crate::types::{CollectionSpec, DistanceMetric, IndexMethod};

    struct StubEmbedder {
        dim: usize,
    }

    #[async_trait]
    impl Embedder for StubEmbedder {
        async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.1f32; self.dim]).collect())
        }
    }

    fn make_store(name: &str) -> Arc<InMemoryVectorStore> {
        Arc::new(InMemoryVectorStore::new(name))
    }

    fn make_collection(name: &str, dim: u32) -> CollectionSpec {
        CollectionSpec {
            name: name.to_string(),
            embedding_dim: dim,
            distance_metric: DistanceMetric::Cosine,
            index_method: IndexMethod::Flat,
        }
    }

    // These pre-existing tests call the 5-argument `ingest_document`/
    // `ingest_document_local` signatures, which only exist when
    // `pipeline-redaction` is off (see the cfg-split variants above). They
    // are gated out under `pipeline-redaction`, where the 6-argument
    // `IngestOutcome`-returning variants take over; that codepath is covered
    // by `ingest_document_redacts_pii_and_returns_rehydration_map` below.
    #[cfg(not(feature = "pipeline-redaction"))]
    #[tokio::test]
    async fn ingest_document_returns_document_id() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("test");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };

        let request = IngestRequest {
            full_text: "Hello world. This is a test document.".to_string(),
            title: Some("Test".to_string()),
            ..Default::default()
        };

        let doc_id = ingest_document(Arc::clone(&store), "docs", request, &config, &embedder)
            .await
            .unwrap();

        assert!(!doc_id.0.is_empty());
    }

    #[cfg(not(feature = "pipeline-redaction"))]
    struct BadEmbedder;

    #[cfg(not(feature = "pipeline-redaction"))]
    #[async_trait]
    impl Embedder for BadEmbedder {
        async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
            // Returns one more vector than requested — always a count mismatch.
            Ok(vec![vec![0.0; 4]; texts.len() + 1])
        }
    }

    #[cfg(not(feature = "pipeline-redaction"))]
    #[tokio::test]
    async fn ingest_rejects_embedder_count_mismatch() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("bad-embedder");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };
        let request = IngestRequest {
            full_text: "Sentence one. Sentence two. Sentence three.".to_string(),
            ..Default::default()
        };

        let err = ingest_document(Arc::clone(&store), "docs", request, &config, &BadEmbedder)
            .await
            .unwrap_err();
        assert!(matches!(err, RagError::EmbeddingCountMismatch { .. }));
    }

    #[cfg(not(feature = "pipeline-redaction"))]
    #[tokio::test]
    async fn retrieve_embeds_query_when_no_vector_provided() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("retrieve-test");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };

        ingest_document(
            Arc::clone(&store),
            "docs",
            IngestRequest {
                full_text: "Rust is great for systems programming.".to_string(),
                ..Default::default()
            },
            &config,
            &embedder,
        )
        .await
        .unwrap();

        let query = RetrieveQuery {
            query_text: Some("systems".to_string()),
            include_content: true,
            ..RetrieveQuery::vector(5)
        };

        let chunks = retrieve(Arc::clone(&store), "docs", query, Some(&embedder))
            .await
            .unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn chunk_to_record_maps_ordinal_and_content() {
        let metadata = xberg::ChunkMetadata {
            byte_start: 0,
            byte_end: 5,
            token_count: None,
            chunk_index: 0,
            total_chunks: 1,
            first_page: None,
            last_page: None,
            heading_context: None,
            heading_path: vec![],
            image_indices: vec![],
        };
        let chunk = xberg::Chunk {
            content: "Hello".to_string(),
            chunk_type: xberg::ChunkType::Unknown,
            embedding: None,
            metadata,
        };
        let record = chunk_to_record(chunk, 7, vec![0.1, 0.2, 0.3]);
        assert_eq!(record.ordinal, 7);
        assert_eq!(record.content, "Hello");
        assert_eq!(record.embedding, vec![0.1, 0.2, 0.3]);
        assert!(record.external_id.is_none());
    }

    #[cfg(not(feature = "pipeline-redaction"))]
    #[tokio::test]
    async fn ingest_document_local_delegates_to_ingest_document() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("test-local");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };

        let request = IngestRequest {
            full_text: "hello world. second sentence.".into(),
            ..Default::default()
        };

        let id = ingest_document_local(Arc::clone(&store), "docs", request, &config, &embedder)
            .await
            .unwrap();

        assert!(!id.0.is_empty());
    }

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

        // Verify stored chunks are actually redacted (L739 fix).
        // InMemoryVectorStore only supports RetrieveMode::Vector (it errors
        // with UnsupportedMode for FullText/Hybrid/Graph — see
        // backends::memory::tests). StubEmbedder returns a constant vector
        // for every text, so any non-empty query_vector of the right
        // dimension retrieves everything in the collection.
        let query = RetrieveQuery {
            mode: RetrieveMode::Vector,
            query_vector: Some(vec![0.1; DIM as usize]),
            top_k: 10,
            include_content: true,
            ..Default::default()
        };
        let results = store.retrieve("docs", &query).await.unwrap();
        assert!(!results.chunks.is_empty(), "should retrieve at least one chunk");
        for chunk in &results.chunks {
            let content = chunk.content.as_ref().expect("content should be included");
            assert!(
                content.contains("[EMAIL_1]") || content.contains("[PERSON_1]"),
                "stored chunk should contain redacted tokens, got: {}",
                content
            );
            assert!(
                !content.contains("alice@example.com"),
                "stored chunk should NOT contain raw email"
            );
            assert!(!content.contains("Alice"), "stored chunk should NOT contain raw name");
        }
    }

    #[cfg(feature = "pipeline-redaction")]
    #[tokio::test]
    async fn ingest_document_shares_token_counter_across_fields() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("pii-cross-field-test");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };
        let ner = StubNerBackend;

        // Two distinct emails in two different fields, same category. Before
        // the fix, full_text's own TokenCounter numbered its email [EMAIL_1],
        // then a *fresh* counter for title also started at [EMAIL_1] — the
        // second `rehydration_map.entry("[EMAIL_1]").or_insert_with(...)` was
        // a no-op since the key already existed, silently discarding the
        // title's real email from the map and mislabeling its stored text.
        let request = IngestRequest {
            full_text: "Please review the attached document.".to_string(),
            title: Some("Report for bob@example.com".to_string()),
            source_uri: Some("Sent by carol@example.com".to_string()),
            ..Default::default()
        };

        let outcome = ingest_document(Arc::clone(&store), "docs", request, &config, &embedder, &ner)
            .await
            .unwrap();

        assert_eq!(outcome.pii_category_counts.get("Email"), Some(&2));
        assert_eq!(outcome.rehydration_map.len(), 2);
        assert_eq!(
            outcome.rehydration_map.get("[EMAIL_1]").map(String::as_str),
            Some("bob@example.com")
        );
        assert_eq!(
            outcome.rehydration_map.get("[EMAIL_2]").map(String::as_str),
            Some("carol@example.com")
        );
    }

    #[cfg(feature = "pipeline-redaction")]
    #[tokio::test]
    async fn ingest_redacts_pii_in_structured_keywords_and_metadata() {
        const DIM: u32 = 4;
        let store: Arc<dyn VectorStore> = make_store("structured-pii-test");
        store.ensure_collection(&make_collection("docs", DIM)).await.unwrap();

        let embedder = StubEmbedder { dim: DIM as usize };
        let chunking = xberg::ChunkingConfig::default();
        let config = RagPipelineConfig { chunking: &chunking };
        let ner = StubNerBackend;

        // A keyword carrying a person name (short, but still goes through
        // redact_string_maybe_ner). A metadata string long enough to pass the
        // free-text heuristic (> 20 chars) and carrying "Alice".
        let metadata_json = serde_json::json!({
            "author": "Alice submitted this document on Monday morning",
            "id": "abc-123"
        });

        let request = IngestRequest {
            full_text: "Some neutral content here.".to_string(),
            keywords: vec!["Alice".to_string(), "rust".to_string()],
            metadata: metadata_json,
            ..Default::default()
        };

        let outcome = ingest_document(Arc::clone(&store), "docs", request, &config, &embedder, &ner)
            .await
            .unwrap();

        // Alice appears in keyword AND in metadata value — should be redacted.
        assert!(
            outcome.pii_category_counts.get("Person").copied().unwrap_or(0) >= 2,
            "expected at least 2 Person redactions (keyword + metadata), got {:?}",
            outcome.pii_category_counts
        );

        // Rehydration map must contain the original "Alice" text.
        assert!(
            outcome.rehydration_map.values().any(|v| v == "Alice"),
            "rehydration map should contain 'Alice', got: {:?}",
            outcome.rehydration_map
        );

        // Verify stored chunks: no raw "Alice" should survive.
        let query = RetrieveQuery {
            mode: RetrieveMode::Vector,
            query_vector: Some(vec![0.1; DIM as usize]),
            top_k: 10,
            include_content: true,
            ..Default::default()
        };
        let results = store.retrieve("docs", &query).await.unwrap();
        assert!(!results.chunks.is_empty());
        for chunk in &results.chunks {
            let content = chunk.content.as_ref().expect("content should be included");
            assert!(
                !content.contains("Alice"),
                "stored chunk should NOT contain raw name 'Alice', got: {}",
                content
            );
        }
    }
}
