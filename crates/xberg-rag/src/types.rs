//! Neutral types for vector-store operations.
//!
//! De-tenanted from the enterprise `vectorstore` crate: `ProjectId` is gone (a
//! store instance is one trust domain), and SaaS-only fields (`quota_max_documents`,
//! `embedding_version`/`enrichment_version` migration columns) are dropped. IDs are
//! opaque strings so backends are free to use UUIDs, row ids, or anything else.

use serde::{Deserialize, Serialize};

/// Opaque identifier for a document, assigned by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocumentId(pub String);

/// Opaque identifier for a chunk, assigned by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkId(pub String);

/// Vector distance metric for similarity computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DistanceMetric {
    /// Cosine similarity (default).
    #[default]
    Cosine,
    /// Euclidean (L2) distance.
    L2,
    /// Inner product.
    InnerProduct,
}

/// Vector index method. A *hint* — backends report what they actually support
/// via [`crate::Capabilities`] and may fall back. `Flat` (exact brute-force) is
/// the universal default; `Hnsw`/`Diskann` are approximate-NN hints honored only
/// by backends that implement them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndexMethod {
    /// Exact brute-force scan. Always available.
    #[default]
    Flat,
    /// Hierarchical Navigable Small World approximate index.
    Hnsw,
    /// StreamingDiskANN approximate index (e.g. pgvectorscale).
    Diskann,
}

/// Collection configuration specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionSpec {
    /// Collection name (the addressing key for all operations).
    pub name: String,
    /// Embedding dimension; every inserted vector must match.
    pub embedding_dim: u32,
    /// Distance metric for similarity search.
    #[serde(default)]
    pub distance_metric: DistanceMetric,
    /// Requested index method (hint; see [`IndexMethod`]).
    #[serde(default)]
    pub index_method: IndexMethod,
}

impl CollectionSpec {
    /// Convenience constructor with cosine distance and a flat index.
    pub fn new(name: impl Into<String>, embedding_dim: u32) -> Self {
        Self {
            name: name.into(),
            embedding_dim,
            distance_metric: DistanceMetric::Cosine,
            index_method: IndexMethod::Flat,
        }
    }
}

/// Document record for upsertion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentRecord {
    /// Caller-supplied external reference (used for idempotent upserts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Human-readable title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// MIME type of the source document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// Source URI (path, URL, object key — backend-opaque).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// Full extracted text.
    pub full_text: String,
    /// Extracted keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Named entities (free-form JSON).
    #[serde(default)]
    pub entities: serde_json::Value,
    /// Labels (free-form JSON).
    #[serde(default)]
    pub labels: serde_json::Value,
    /// Document-level metadata (free-form JSON).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Chunk record for upsertion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkRecord {
    /// Caller-supplied external reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Ordinal position within the parent document.
    pub ordinal: u32,
    /// Chunk text content.
    pub content: String,
    /// Dense embedding vector.
    pub embedding: Vec<f32>,
    /// Chunk-level metadata (free-form JSON).
    #[serde(default)]
    pub chunk_metadata: serde_json::Value,
}

/// Document summary attached to retrieval results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSummary {
    /// Document id.
    pub id: DocumentId,
    /// External reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// Keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Labels.
    #[serde(default)]
    pub labels: serde_json::Value,
    /// Entities.
    #[serde(default)]
    pub entities: serde_json::Value,
    /// Metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Ingestion time as Unix seconds, if the backend tracks it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingested_at: Option<i64>,
}

/// How a retrieved chunk was primarily scored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PrimaryScore {
    /// Vector similarity.
    ///
    /// A struct variant (not a newtype-over-scalar) so it survives the
    /// internally-tagged serde representation: `#[serde(tag = "kind")]` can only
    /// flatten a variant's fields alongside the tag key, and a bare `f32` has no
    /// fields to flatten. Serialized shape: `{ "kind": "vector", "score": 0.9 }`.
    Vector {
        /// Vector similarity score.
        score: f32,
    },
    /// Full-text relevance. Struct variant for the same reason as [`Self::Vector`].
    /// Serialized shape: `{ "kind": "full_text", "score": 0.9 }`.
    FullText {
        /// Full-text relevance score.
        score: f32,
    },
    /// Hybrid (vector + full-text fused via reciprocal rank fusion).
    Hybrid {
        /// Vector similarity component.
        vector: f32,
        /// Full-text component.
        full_text: f32,
        /// Reciprocal-rank-fusion combined score.
        rrf: f32,
    },
}

/// A chunk returned from a retrieval query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievedChunk {
    /// Chunk id.
    pub id: ChunkId,
    /// Parent document id.
    pub document_id: DocumentId,
    /// Position within the document.
    pub ordinal: u32,
    /// External reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Chunk content (present when `include_content` was requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Final score used for ordering.
    pub score: f32,
    /// How the score was produced.
    pub primary_score: PrimaryScore,
    /// Chunk-level metadata.
    #[serde(default)]
    pub chunk_metadata: serde_json::Value,
    /// Parent document summary (present when `include_document` was requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentSummary>,
}

/// Aggregate statistics for a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CollectionStats {
    /// Number of documents.
    pub documents: u64,
    /// Number of chunks.
    pub chunks: u64,
    /// Most recent ingestion time as Unix seconds, if tracked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_ingested_at: Option<i64>,
}
