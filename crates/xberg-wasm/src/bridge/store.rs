use std::error::Error;
use std::fmt;

use async_trait::async_trait;
use js_sys::{Object, Promise, Reflect};
use wasm_bindgen::prelude::*;
use xberg_rag::capability::Capabilities;
use xberg_rag::error::{RagError, RagResult};
use xberg_rag::filter::Filter;
use xberg_rag::query::{RetrieveOutput, RetrieveQuery};
use xberg_rag::store::VectorStore;
use xberg_rag::types::{ChunkRecord, CollectionSpec, CollectionStats, DocumentId, DocumentRecord};

#[derive(Debug)]
struct JsStoreError(String);

impl fmt::Display for JsStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JS store error: {}", self.0)
    }
}

impl Error for JsStoreError {}

pub struct JsVectorStore {
    name: String,
    inner: Object,
    timeout_ms: u32,
}

impl JsVectorStore {
    pub fn new(name: String, inner: Object) -> Self {
        Self {
            name,
            inner,
            timeout_ms: crate::bridge::BRIDGE_TIMEOUT_MS,
        }
    }

    pub fn with_timeout(name: String, inner: Object, timeout_ms: u32) -> Self {
        Self {
            name,
            inner,
            timeout_ms,
        }
    }

    async fn call_method(&self, method: &str, args: &[JsValue]) -> RagResult<JsValue> {
        let func_val = Reflect::get(&self.inner, &JsValue::from_str(method)).map_err(js_to_rag)?;
        let func: js_sys::Function = func_val.dyn_into().map_err(|_| {
            RagError::Backend(Box::new(JsStoreError(format!(
                "method '{}' not found on JS store",
                method
            ))))
        })?;
        let js_args = js_sys::Array::new();
        for a in args {
            js_args.push(a);
        }
        let result = func.apply(&self.inner, &js_args).map_err(js_to_rag)?;
        let promise = Promise::from(result);
        crate::bridge::timed_js_future_with_timeout(promise, self.timeout_ms)
            .await
            .map_err(js_to_rag)
    }
}

#[async_trait(?Send)]
impl VectorStore for JsVectorStore {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        // Try to read capabilities from the injected JS store, falling back
        // to a default that advertises full-text + hybrid + filtering.
        let fallback = || Capabilities {
            full_text: true,
            hybrid: true,
            filtering: true,
            index_methods: vec![],
        };
        let caps_val = match Reflect::get(&self.inner, &JsValue::from_str("capabilities")) {
            Ok(v) if !v.is_undefined() && !v.is_null() => v,
            _ => return fallback(),
        };
        serde_wasm_bindgen::from_value::<Capabilities>(caps_val).unwrap_or_else(|_| fallback())
    }

    async fn ensure_collection(&self, spec: &CollectionSpec) -> RagResult<()> {
        let val = serde_wasm_bindgen::to_value(spec).map_err(js_to_rag)?;
        let result = self.call_method("ensureCollection", &[val]).await?;
        if let Some(err) = result.as_string() {
            return Err(RagError::Backend(Box::new(JsStoreError(err))));
        }
        Ok(())
    }

    async fn drop_collection(&self, collection: &str) -> RagResult<()> {
        let val = JsValue::from_str(collection);
        let result = self.call_method("dropCollection", &[val]).await?;
        if let Some(err) = result.as_string() {
            return Err(RagError::Backend(Box::new(JsStoreError(err))));
        }
        Ok(())
    }

    async fn get_collection(&self, collection: &str) -> RagResult<Option<CollectionSpec>> {
        let val = JsValue::from_str(collection);
        let result = self.call_method("getCollection", &[val]).await?;
        if result.is_null() || result.is_undefined() {
            return Ok(None);
        }
        let spec: CollectionSpec = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(Some(spec))
    }

    async fn upsert_document(
        &self,
        collection: &str,
        document: &DocumentRecord,
        chunks: &[ChunkRecord],
    ) -> RagResult<DocumentId> {
        let coll_val = JsValue::from_str(collection);
        let doc_val = serde_wasm_bindgen::to_value(document).map_err(js_to_rag)?;
        let chunks_val = serde_wasm_bindgen::to_value(chunks).map_err(js_to_rag)?;
        let result = self
            .call_method("upsertDocument", &[coll_val, doc_val, chunks_val])
            .await?;
        let id: DocumentId = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(id)
    }

    async fn delete_documents(&self, collection: &str, ids: &[DocumentId]) -> RagResult<u64> {
        let coll_val = JsValue::from_str(collection);
        let ids_val = serde_wasm_bindgen::to_value(ids).map_err(js_to_rag)?;
        let result = self.call_method("deleteDocuments", &[coll_val, ids_val]).await?;
        let count: u64 = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(count)
    }

    async fn delete_by_filter(&self, collection: &str, filter: &Filter) -> RagResult<u64> {
        let coll_val = JsValue::from_str(collection);
        let filter_val = serde_wasm_bindgen::to_value(filter).map_err(js_to_rag)?;
        let result = self.call_method("deleteByFilter", &[coll_val, filter_val]).await?;
        let count: u64 = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(count)
    }

    async fn retrieve(&self, collection: &str, query: &RetrieveQuery) -> RagResult<RetrieveOutput> {
        let coll_val = JsValue::from_str(collection);
        let query_val = serde_wasm_bindgen::to_value(query).map_err(js_to_rag)?;
        let result = self.call_method("retrieve", &[coll_val, query_val]).await?;
        let output: RetrieveOutput = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(output)
    }

    async fn collection_stats(&self, collection: &str) -> RagResult<CollectionStats> {
        let val = JsValue::from_str(collection);
        let result = self.call_method("collectionStats", &[val]).await?;
        let stats: CollectionStats = serde_wasm_bindgen::from_value(result).map_err(js_to_rag)?;
        Ok(stats)
    }
}

fn js_to_rag(v: impl Into<JsValue>) -> RagError {
    let v = v.into();
    RagError::Backend(Box::new(JsStoreError(format!("{v:?}"))))
}
