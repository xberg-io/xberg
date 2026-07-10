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

// ─── Embedder ────────────────────────────────────────────────────────────────

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
#[derive(Debug, Clone, Default, serde::Deserialize)]
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Like [`ingest_document`] but alias for the same codepath on non-wasm32;
/// delegates directly to [`ingest_document`] since both use `spawn_blocking`.
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

/// Like [`ingest_document`] but chunks inline (no `tokio::task::spawn_blocking`),
/// so it compiles and runs on `wasm32` where the multi-thread runtime is absent.
#[cfg(target_arch = "wasm32")]
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

    struct BadEmbedder;

    #[async_trait]
    impl Embedder for BadEmbedder {
        async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
            // Returns one more vector than requested — always a count mismatch.
            Ok(vec![vec![0.0; 4]; texts.len() + 1])
        }
    }

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
}
