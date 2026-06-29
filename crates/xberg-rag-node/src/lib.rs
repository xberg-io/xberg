use std::sync::Arc;
use napi_derive::napi;

#[napi]
pub struct RagStore {
    inner: Arc<dyn xberg_rag::VectorStore>,
}

#[napi]
impl RagStore {
    #[napi(factory)]
    pub async fn open_sqlite(name: String, db_path: String) -> napi::Result<Self> {
        let store = xberg_rag::backends::sqlite::SqliteVectorStore::open(name, db_path)
            .await
            .map_err(|e| napi::Error::from_reason(format!("Failed to open store: {e}")))?;
        Ok(Self {
            inner: Arc::new(store),
        })
    }

    #[napi]
    pub async fn ensure_collection(&self, spec_json: String) -> napi::Result<()> {
        let spec: xberg_rag::CollectionSpec = serde_json::from_str(&spec_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid spec: {e}")))?;
        self.inner
            .ensure_collection(&spec)
            .await
            .map_err(|e| napi::Error::from_reason(format!("ensure_collection failed: {e}")))
    }

    #[napi]
    pub async fn drop_collection(&self, collection: String) -> napi::Result<()> {
        self.inner
            .drop_collection(&collection)
            .await
            .map_err(|e| napi::Error::from_reason(format!("drop_collection failed: {e}")))
    }

    #[napi]
    pub async fn get_collection(&self, collection: String) -> napi::Result<Option<String>> {
        let spec = self
            .inner
            .get_collection(&collection)
            .await
            .map_err(|e| napi::Error::from_reason(format!("get_collection failed: {e}")))?;
        Ok(spec.map(|s| serde_json::to_string(&s).unwrap_or_default()))
    }

    #[napi]
    pub async fn upsert_document(
        &self,
        collection: String,
        document_json: String,
        chunks_json: String,
    ) -> napi::Result<String> {
        let doc: xberg_rag::DocumentRecord = serde_json::from_str(&document_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid document: {e}")))?;
        let chunks: Vec<xberg_rag::ChunkRecord> = serde_json::from_str(&chunks_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid chunks: {e}")))?;
        let doc_id = self
            .inner
            .upsert_document(&collection, &doc, &chunks)
            .await
            .map_err(|e| napi::Error::from_reason(format!("upsert_document failed: {e}")))?;
        Ok(doc_id.0)
    }

    #[napi]
    pub async fn retrieve(&self, collection: String, query_json: String) -> napi::Result<String> {
        let query: xberg_rag::RetrieveQuery = serde_json::from_str(&query_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid query: {e}")))?;
        let output = self
            .inner
            .retrieve(&collection, &query)
            .await
            .map_err(|e| napi::Error::from_reason(format!("retrieve failed: {e}")))?;
        serde_json::to_string(&output)
            .map_err(|e| napi::Error::from_reason(format!("Serialization failed: {e}")))
    }

    #[napi]
    pub async fn delete_documents(&self, collection: String, ids_json: String) -> napi::Result<f64> {
        let ids: Vec<String> = serde_json::from_str(&ids_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid ids: {e}")))?;
        let doc_ids: Vec<xberg_rag::DocumentId> =
            ids.into_iter().map(xberg_rag::DocumentId).collect();
        self.inner
            .delete_documents(&collection, &doc_ids)
            .await
            .map(|n| n as f64)
            .map_err(|e| napi::Error::from_reason(format!("delete_documents failed: {e}")))
    }

    #[napi]
    pub async fn delete_by_filter(&self, collection: String, filter_json: String) -> napi::Result<f64> {
        let filter: xberg_rag::Filter = serde_json::from_str(&filter_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid filter: {e}")))?;
        self.inner
            .delete_by_filter(&collection, &filter)
            .await
            .map(|n| n as f64)
            .map_err(|e| napi::Error::from_reason(format!("delete_by_filter failed: {e}")))
    }

    #[napi]
    pub async fn collection_stats(&self, collection: String) -> napi::Result<String> {
        let stats = self
            .inner
            .collection_stats(&collection)
            .await
            .map_err(|e| napi::Error::from_reason(format!("collection_stats failed: {e}")))?;
        serde_json::to_string(&stats)
            .map_err(|e| napi::Error::from_reason(format!("Serialization failed: {e}")))
    }
}

#[napi]
pub async fn embed_texts(texts_json: String, config_json: String) -> napi::Result<String> {
    let texts: Vec<String> = serde_json::from_str(&texts_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid texts: {e}")))?;
    let config: xberg::core::config::EmbeddingConfig = serde_json::from_str(&config_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid config: {e}")))?;

    let embeddings = xberg::embed_texts_async(texts, &config)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Embedding failed: {e}")))?;

    serde_json::to_string(&embeddings)
        .map_err(|e| napi::Error::from_reason(format!("Serialization failed: {e}")))
}

#[napi]
pub async fn rerank(
    query: String,
    documents_json: String,
    config_json: String,
) -> napi::Result<String> {
    let documents: Vec<String> = serde_json::from_str(&documents_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid documents: {e}")))?;
    let config: xberg::core::config::RerankerConfig = serde_json::from_str(&config_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid config: {e}")))?;

    let ranked = xberg::rerank_async(query, documents, &config)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Reranking failed: {e}")))?;

    serde_json::to_string(&ranked)
        .map_err(|e| napi::Error::from_reason(format!("Serialization failed: {e}")))
}
