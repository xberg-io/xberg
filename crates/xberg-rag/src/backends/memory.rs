//! Pure-Rust, brute-force in-memory vector store.
//!
//! The reference backend: exact vector search over `Vec<f32>`, a compact filter
//! evaluator, and document/chunk bookkeeping. No external dependencies, WASM-safe.
//! It does not implement full-text or hybrid retrieval (it has no text index) and
//! reports that via [`Capabilities`].

use crate::capability::Capabilities;
use crate::error::{RagError, RagResult};
use crate::filter::{Filter, FilterField};
use crate::query::{RetrieveMode, RetrieveOutput, RetrieveQuery};
use crate::store::VectorStore;
use crate::types::{
    ChunkId, ChunkRecord, CollectionSpec, CollectionStats, DistanceMetric, DocumentId, DocumentRecord, DocumentSummary,
    PrimaryScore, RetrievedChunk,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

struct StoredChunk {
    id: ChunkId,
    document_id: DocumentId,
    record: ChunkRecord,
}

struct StoredDocument {
    record: DocumentRecord,
}

#[derive(Default)]
struct Collection {
    spec: Option<CollectionSpec>,
    documents: HashMap<DocumentId, StoredDocument>,
    external_index: HashMap<String, DocumentId>,
    chunks: Vec<StoredChunk>,
}

/// An in-memory [`VectorStore`] backed by brute-force scan.
pub struct InMemoryVectorStore {
    name: String,
    collections: RwLock<HashMap<String, Collection>>,
    doc_counter: AtomicU64,
}

impl InMemoryVectorStore {
    /// Create a new, empty in-memory store with the given registry name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            collections: RwLock::new(HashMap::new()),
            doc_counter: AtomicU64::new(0),
        }
    }

    fn next_doc_id(&self) -> String {
        let n = self.doc_counter.fetch_add(1, Ordering::Relaxed);
        format!("{}-doc-{n}", self.name)
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new("in-memory")
    }
}

fn score(metric: DistanceMetric, a: &[f32], b: &[f32]) -> f32 {
    match metric {
        DistanceMetric::Cosine => {
            let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
            let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
        }
        DistanceMetric::InnerProduct => a.iter().zip(b).map(|(x, y)| x * y).sum(),
        // Higher score == more relevant, so negate the L2 distance.
        DistanceMetric::L2 => {
            let d2: f32 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
            -d2.sqrt()
        }
    }
}

/// Resolve a filter field to a JSON value within a (document, chunk) context.
fn resolve_field(field: &FilterField, doc: &DocumentRecord, chunk: &ChunkRecord) -> Option<serde_json::Value> {
    let parsed = field.parse().ok()?;
    use crate::filter::FilterNamespace::*;
    match parsed.namespace {
        Doc => match parsed.path.as_str() {
            "full_text" => Some(serde_json::Value::String(doc.full_text.clone())),
            "title" => doc.title.clone().map(serde_json::Value::String),
            "mime" => doc.mime.clone().map(serde_json::Value::String),
            "external_id" => doc.external_id.clone().map(serde_json::Value::String),
            "source_uri" => doc.source_uri.clone().map(serde_json::Value::String),
            "keywords" => serde_json::to_value(&doc.keywords).ok(),
            "labels" => Some(doc.labels.clone()),
            "entities" => Some(doc.entities.clone()),
            path if path.starts_with("metadata.") => json_pointer(&doc.metadata, &path["metadata.".len()..]),
            _ => None,
        },
        Chunk => match parsed.path.as_str() {
            "content" => Some(serde_json::Value::String(chunk.content.clone())),
            "ordinal" => Some(serde_json::Value::from(chunk.ordinal)),
            "external_id" => chunk.external_id.clone().map(serde_json::Value::String),
            path if path.starts_with("chunk_metadata.") => {
                json_pointer(&chunk.chunk_metadata, &path["chunk_metadata.".len()..])
            }
            _ => None,
        },
    }
}

fn json_pointer(value: &serde_json::Value, dotted: &str) -> Option<serde_json::Value> {
    let mut cur = value;
    for segment in dotted.split('.') {
        cur = cur.get(segment)?;
    }
    Some(cur.clone())
}

fn json_cmp(a: &serde_json::Value, b: &serde_json::Value) -> Option<std::cmp::Ordering> {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x.partial_cmp(&y),
        _ => match (a.as_str(), b.as_str()) {
            (Some(x), Some(y)) => Some(x.cmp(y)),
            _ => None,
        },
    }
}

fn eval_filter(filter: &Filter, doc: &DocumentRecord, chunk: &ChunkRecord) -> bool {
    match filter {
        Filter::Eq { field, value } => resolve_field(field, doc, chunk).as_ref() == Some(value),
        Filter::In { field, values } => {
            resolve_field(field, doc, chunk).is_some_and(|v| values.iter().any(|candidate| candidate == &v))
        }
        Filter::ArrayContains { field, value } => resolve_field(field, doc, chunk)
            .and_then(|v| v.as_array().cloned())
            .is_some_and(|arr| arr.iter().any(|item| item == value)),
        Filter::Range {
            field,
            gte,
            gt,
            lte,
            lt,
        } => {
            let Some(v) = resolve_field(field, doc, chunk) else {
                return false;
            };
            use std::cmp::Ordering;
            let pass = |bound: &Option<serde_json::Value>, want: &[Ordering]| {
                bound
                    .as_ref()
                    .is_none_or(|b| json_cmp(&v, b).is_some_and(|ord| want.contains(&ord)))
            };
            pass(gte, &[Ordering::Greater, Ordering::Equal])
                && pass(gt, &[Ordering::Greater])
                && pass(lte, &[Ordering::Less, Ordering::Equal])
                && pass(lt, &[Ordering::Less])
        }
        Filter::TextMatch { field, query } => resolve_field(field, doc, chunk)
            .and_then(|v| v.as_str().map(str::to_string))
            .is_some_and(|s| s.to_lowercase().contains(&query.to_lowercase())),
        Filter::And { filters } => filters.iter().all(|f| eval_filter(f, doc, chunk)),
        Filter::Or { filters } => filters.iter().any(|f| eval_filter(f, doc, chunk)),
        Filter::Not { filter } => !eval_filter(filter, doc, chunk),
    }
}

fn summarize(id: &DocumentId, doc: &DocumentRecord) -> DocumentSummary {
    DocumentSummary {
        id: id.clone(),
        external_id: doc.external_id.clone(),
        title: doc.title.clone(),
        mime: doc.mime.clone(),
        keywords: doc.keywords.clone(),
        labels: doc.labels.clone(),
        entities: doc.entities.clone(),
        metadata: doc.metadata.clone(),
        ingested_at: None,
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl VectorStore for InMemoryVectorStore {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            full_text: false,
            hybrid: false,
            filtering: true,
            index_methods: vec![crate::types::IndexMethod::Flat],
        }
    }

    async fn ensure_collection(&self, spec: &CollectionSpec) -> RagResult<()> {
        let mut collections = self.collections.write().expect("poisoned");
        let entry = collections.entry(spec.name.clone()).or_default();
        match &entry.spec {
            Some(existing) if existing.embedding_dim != spec.embedding_dim => {
                Err(RagError::CollectionAlreadyExists(spec.name.clone()))
            }
            _ => {
                entry.spec = Some(spec.clone());
                Ok(())
            }
        }
    }

    async fn drop_collection(&self, collection: &str) -> RagResult<()> {
        let mut collections = self.collections.write().expect("poisoned");
        collections
            .remove(collection)
            .map(|_| ())
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))
    }

    async fn get_collection(&self, collection: &str) -> RagResult<Option<CollectionSpec>> {
        let collections = self.collections.read().expect("poisoned");
        Ok(collections.get(collection).and_then(|c| c.spec.clone()))
    }

    async fn upsert_document(
        &self,
        collection: &str,
        document: &DocumentRecord,
        chunks: &[ChunkRecord],
    ) -> RagResult<DocumentId> {
        let mut collections = self.collections.write().expect("poisoned");
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        let dim = coll.spec.as_ref().map(|s| s.embedding_dim).unwrap_or(0);
        for chunk in chunks {
            if chunk.embedding.len() as u32 != dim {
                return Err(RagError::EmbeddingDimMismatch {
                    expected: dim,
                    got: chunk.embedding.len() as u32,
                });
            }
        }

        // Resolve identity: reuse the id for an existing external_id, else mint one.
        let doc_id = match document
            .external_id
            .as_ref()
            .and_then(|ext| coll.external_index.get(ext).cloned())
        {
            Some(existing) => {
                coll.chunks.retain(|c| c.document_id != existing);
                existing
            }
            None => DocumentId(self.next_doc_id()),
        };

        if let Some(ext) = &document.external_id {
            coll.external_index.insert(ext.clone(), doc_id.clone());
        }
        coll.documents.insert(
            doc_id.clone(),
            StoredDocument {
                record: document.clone(),
            },
        );
        for chunk in chunks {
            coll.chunks.push(StoredChunk {
                id: ChunkId(format!("{}:{}", doc_id.0, chunk.ordinal)),
                document_id: doc_id.clone(),
                record: chunk.clone(),
            });
        }
        Ok(doc_id)
    }

    async fn delete_documents(&self, collection: &str, ids: &[DocumentId]) -> RagResult<u64> {
        let mut collections = self.collections.write().expect("poisoned");
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        let mut removed = 0u64;
        for id in ids {
            if let Some(doc) = coll.documents.remove(id) {
                removed += 1;
                if let Some(ext) = doc.record.external_id {
                    coll.external_index.remove(&ext);
                }
            }
        }
        coll.chunks.retain(|c| !ids.contains(&c.document_id));
        Ok(removed)
    }

    async fn delete_by_filter(&self, collection: &str, filter: &Filter) -> RagResult<u64> {
        filter.validate()?;
        let mut collections = self.collections.write().expect("poisoned");
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        // A document matches if any of its chunks satisfies the filter.
        let mut to_remove: Vec<DocumentId> = Vec::new();
        for (id, doc) in &coll.documents {
            let matches = coll
                .chunks
                .iter()
                .filter(|c| &c.document_id == id)
                .any(|c| eval_filter(filter, &doc.record, &c.record));
            if matches {
                to_remove.push(id.clone());
            }
        }
        let mut removed = 0u64;
        for id in &to_remove {
            if let Some(doc) = coll.documents.remove(id) {
                removed += 1;
                if let Some(ext) = doc.record.external_id {
                    coll.external_index.remove(&ext);
                }
            }
        }
        coll.chunks.retain(|c| !to_remove.contains(&c.document_id));
        Ok(removed)
    }

    async fn retrieve(&self, collection: &str, query: &RetrieveQuery) -> RagResult<RetrieveOutput> {
        if query.mode != RetrieveMode::Vector {
            return Err(RagError::UnsupportedMode {
                backend: self.name.clone(),
                mode: query.mode.as_str().to_string(),
            });
        }
        let collections = self.collections.read().expect("poisoned");
        let coll = collections
            .get(collection)
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        let spec = coll
            .spec
            .as_ref()
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        query.validate(spec)?;
        let query_vector = query.query_vector.as_ref().ok_or_else(|| {
            RagError::InvalidQuery("in-memory backend cannot embed text; supply query_vector".to_string())
        })?;

        let mut scored: Vec<RetrievedChunk> = coll
            .chunks
            .iter()
            .filter(|c| {
                query.filter.as_ref().is_none_or(|f| {
                    coll.documents
                        .get(&c.document_id)
                        .is_some_and(|d| eval_filter(f, &d.record, &c.record))
                })
            })
            .map(|c| {
                let s = score(spec.distance_metric, query_vector, &c.record.embedding);
                RetrievedChunk {
                    id: c.id.clone(),
                    document_id: c.document_id.clone(),
                    ordinal: c.record.ordinal,
                    external_id: c.record.external_id.clone(),
                    content: query.include_content.then(|| c.record.content.clone()),
                    score: s,
                    primary_score: PrimaryScore::Vector { score: s },
                    chunk_metadata: c.record.chunk_metadata.clone(),
                    document: query.include_document.then(|| {
                        coll.documents
                            .get(&c.document_id)
                            .map(|d| summarize(&c.document_id, &d.record))
                            .unwrap_or_else(|| summarize(&c.document_id, &DocumentRecord::default()))
                    }),
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        if query.group_by_document {
            let mut seen: HashMap<DocumentId, ()> = HashMap::new();
            scored.retain(|c| seen.insert(c.document_id.clone(), ()).is_none());
        }
        scored.truncate(query.top_k as usize);

        Ok(RetrieveOutput {
            mode: RetrieveMode::Vector,
            chunks: scored,
            primary_latency_ms: 0,
        })
    }

    async fn collection_stats(&self, collection: &str) -> RagResult<CollectionStats> {
        let collections = self.collections.read().expect("poisoned");
        let coll = collections
            .get(collection)
            .ok_or_else(|| RagError::CollectionNotFound(collection.to_string()))?;
        Ok(CollectionStats {
            documents: coll.documents.len() as u64,
            chunks: coll.chunks.len() as u64,
            last_ingested_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store_with_collection(dim: u32) -> InMemoryVectorStore {
        let store = InMemoryVectorStore::new("test");
        store
            .ensure_collection(&CollectionSpec::new("docs", dim))
            .await
            .unwrap();
        store
    }

    fn chunk(ordinal: u32, content: &str, embedding: Vec<f32>) -> ChunkRecord {
        ChunkRecord {
            external_id: None,
            ordinal,
            content: content.to_string(),
            embedding,
            chunk_metadata: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn upsert_then_retrieve_orders_by_similarity() {
        let store = store_with_collection(3).await;
        let doc = DocumentRecord {
            full_text: "hello world".to_string(),
            ..Default::default()
        };
        let chunks = vec![
            chunk(0, "near", vec![1.0, 0.0, 0.0]),
            chunk(1, "far", vec![0.0, 1.0, 0.0]),
        ];
        store.upsert_document("docs", &doc, &chunks).await.unwrap();

        let q = RetrieveQuery {
            query_vector: Some(vec![1.0, 0.0, 0.0]),
            ..RetrieveQuery::vector(10)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert_eq!(out.chunks.len(), 2);
        assert_eq!(out.chunks[0].content.as_deref(), Some("near"));
    }

    #[tokio::test]
    async fn rejects_dimension_mismatch() {
        let store = store_with_collection(3).await;
        let doc = DocumentRecord::default();
        let bad = vec![chunk(0, "x", vec![1.0, 0.0])];
        let err = store.upsert_document("docs", &doc, &bad).await.unwrap_err();
        assert!(matches!(err, RagError::EmbeddingDimMismatch { expected: 3, got: 2 }));
    }

    #[tokio::test]
    async fn rejects_unsupported_full_text_mode() {
        let store = store_with_collection(3).await;
        let mut q = RetrieveQuery::vector(5);
        q.mode = RetrieveMode::FullText;
        q.query_text = Some("hi".to_string());
        let err = store.retrieve("docs", &q).await.unwrap_err();
        assert!(matches!(err, RagError::UnsupportedMode { .. }));
    }

    #[tokio::test]
    async fn filter_constrains_results() {
        let store = store_with_collection(2).await;
        let a = DocumentRecord {
            title: Some("keep".to_string()),
            full_text: "a".to_string(),
            ..Default::default()
        };
        let b = DocumentRecord {
            title: Some("drop".to_string()),
            full_text: "b".to_string(),
            ..Default::default()
        };
        store
            .upsert_document("docs", &a, &[chunk(0, "a", vec![1.0, 0.0])])
            .await
            .unwrap();
        store
            .upsert_document("docs", &b, &[chunk(0, "b", vec![1.0, 0.0])])
            .await
            .unwrap();

        let q = RetrieveQuery {
            query_vector: Some(vec![1.0, 0.0]),
            filter: Some(Filter::Eq {
                field: FilterField("doc.title".to_string()),
                value: serde_json::json!("keep"),
            }),
            ..RetrieveQuery::vector(10)
        };
        let out = store.retrieve("docs", &q).await.unwrap();
        assert_eq!(out.chunks.len(), 1);
    }

    #[tokio::test]
    async fn external_id_upsert_replaces_chunks() {
        let store = store_with_collection(2).await;
        let doc = DocumentRecord {
            external_id: Some("ext-1".to_string()),
            ..Default::default()
        };
        store
            .upsert_document("docs", &doc, &[chunk(0, "v1", vec![1.0, 0.0])])
            .await
            .unwrap();
        store
            .upsert_document("docs", &doc, &[chunk(0, "v2", vec![0.0, 1.0])])
            .await
            .unwrap();
        let stats = store.collection_stats("docs").await.unwrap();
        assert_eq!(stats.documents, 1);
        assert_eq!(stats.chunks, 1);
    }
}
