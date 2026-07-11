//! `XbergEngine` — a stateful handle carrying injected bridges and
//! convenience methods on top of the raw WASM exports.

use std::collections::HashMap;
use std::sync::Arc;

use js_sys::Object;
use wasm_bindgen::prelude::*;

use crate::bridge::embedder::JsEmbedder;
use crate::bridge::ner::resolve_ner_with_timeout;
use crate::bridge::ocr::resolve_ocr_with_timeout;
use crate::bridge::store::JsVectorStore;
use xberg_rag::pipeline::Embedder;
use xberg_rag::query::{RetrieveMode, RetrieveQuery};
use xberg_rag::VectorStore;

/// Extract an optional JS object field, returning `None` if the field is
/// missing, `null`, or `undefined`.
fn get_opt_field(obj: &Object, field: &str) -> Result<Option<Object>, JsValue> {
    let val = js_sys::Reflect::get(obj, &JsValue::from_str(field))
        .map_err(|_| JsValue::from_str(&format!("failed to read field '{field}'")))?;
    if val.is_undefined() || val.is_null() {
        return Ok(None);
    }
    val.dyn_into::<Object>().map(Some).map_err(|_| {
        JsValue::from_str(&format!("field '{field}' must be an object"))
    })
}

/// Extract an optional numeric field, returning `None` if the field is
/// missing, `null`, `undefined`, or not a number.
fn get_opt_number(obj: &Object, field: &str) -> Result<Option<f64>, JsValue> {
    let val = js_sys::Reflect::get(obj, &JsValue::from_str(field))
        .map_err(|_| JsValue::from_str(&format!("failed to read field '{field}'")))?;
    if val.is_undefined() || val.is_null() {
        return Ok(None);
    }
    Ok(val.as_f64())
}

/// Rehydration map type (token → original PII text).
type RehydrationMap = HashMap<String, String>;

/// Stateful engine handle exposed to JS.
///
/// Constructed via `XbergEngine.new(config, injection)` where `config` may
/// contain optional settings (e.g. `bridgeTimeoutMs`) and `injection` is a
/// plain object with optional `embedder`, `store`, `ner`, and `ocr` keys.
#[wasm_bindgen]
pub struct XbergEngine {
    embedder: Option<Arc<JsEmbedder>>,
    store: Option<Arc<JsVectorStore>>,
    ner: Option<js_sys::Object>,
    ocr: Option<js_sys::Object>,
    bridge_timeout_ms: u32,
}

#[wasm_bindgen]
impl XbergEngine {
    /// Create a new engine with injected bridges.
    ///
    /// `config` may contain:
    /// - `bridgeTimeoutMs` — timeout in milliseconds for JS bridge calls
    ///   (defaults to 30,000ms if not provided)
    ///
    /// `injection` may contain:
    /// - `embedder` — object with `embed(texts: string[]): Promise<number[][]>`
    /// - `store`    — object implementing the VectorStore JS protocol
    /// - `ner`      — object with `ner(text, categories): Promise<...>`
    /// - `ocr`      — object with `ocr(imageBytes, opts): Promise<string>`
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue, injection: JsValue) -> Result<XbergEngine, JsValue> {
        let bridge_timeout_ms = if config.is_undefined() || config.is_null() {
            crate::bridge::BRIDGE_TIMEOUT_MS
        } else {
            let config_obj: Object = config.dyn_into().map_err(|_| {
                JsValue::from_str("config must be an object")
            })?;
            get_opt_number(&config_obj, "bridgeTimeoutMs")?
                .map(|v| v as u32)
                .unwrap_or(crate::bridge::BRIDGE_TIMEOUT_MS)
        };

        let obj: Object = if injection.is_undefined() || injection.is_null() {
            Object::new()
        } else {
            injection
                .dyn_into::<Object>()
                .map_err(|_| JsValue::from_str("injection must be an object"))?
        };

        let embedder = get_opt_field(&obj, "embedder")?
            .map(|o| Arc::new(JsEmbedder::with_timeout(o, bridge_timeout_ms)));

        let store = get_opt_field(&obj, "store")?
            .map(|o| Arc::new(JsVectorStore::with_timeout("default".to_string(), o, bridge_timeout_ms)));

        let ner = get_opt_field(&obj, "ner")?;
        let ocr = get_opt_field(&obj, "ocr")?;

        Ok(XbergEngine {
            embedder,
            store,
            ner,
            ocr,
            bridge_timeout_ms,
        })
    }

    /// Extract content from a single bytes or URI input.
    #[allow(clippy::missing_errors_doc)]
    pub async fn extract(
        &self,
        input: JsValue,
        config: JsValue,
    ) -> Result<JsValue, JsValue> {
        let input_core: xberg::ExtractInput = if input.is_undefined() {
            xberg::ExtractInput::default()
        } else {
            serde_wasm_bindgen::from_value::<xberg::ExtractInput>(input)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };
        let config_core: xberg::ExtractionConfig = if config.is_undefined() {
            xberg::ExtractionConfig::default()
        } else {
            serde_wasm_bindgen::from_value::<xberg::ExtractionConfig>(config)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };
        let result = xberg::extract(input_core, &config_core)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let wasm_result = crate::WasmExtractionResult::from(result);
        Ok(wasm_result.into())
    }

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

    /// Perform OCR on image bytes, returning extracted text.
    #[allow(clippy::missing_errors_doc)]
    pub async fn ocr(&self, bytes: Vec<u8>, opts: JsValue) -> Result<JsValue, JsValue> {
        let language = if opts.is_undefined() || opts.is_null() {
            "eng".to_string()
        } else {
            js_sys::Reflect::get(&opts, &JsValue::from_str("language"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "eng".to_string())
        };

        let text = resolve_ocr_with_timeout(self.ocr.clone(), &bytes, &language, self.bridge_timeout_ms)
            .await
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        Ok(JsValue::from_str(&text))
    }

    /// Decrypt a rehydration map and substitute tokens in `doc`.
    ///
    /// Returns the dehydrated text with original PII values restored.
    #[cfg(feature = "redaction-rehydrate")]
    #[allow(clippy::missing_errors_doc)]
    pub fn rehydrate(
        &self,
        doc: String,
        map_bytes: Vec<u8>,
        passphrase: String,
    ) -> Result<String, JsValue> {
        let map: RehydrationMap =
            xberg::text::redaction::rehydration::decrypt_map(&map_bytes, &passphrase)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut result = doc;
        for (token, original) in &map {
            result = result.replace(token, original);
        }
        Ok(result)
    }

    /// Perform Named Entity Recognition on `text`.
    ///
    /// Returns entities as a JSON-serializable JsValue array.
    #[allow(clippy::missing_errors_doc)]
    pub async fn ner(&self, text: String, opts: JsValue) -> Result<JsValue, JsValue> {
        // Parse categories from opts if provided, otherwise use empty list.
        let categories: Vec<xberg::types::entity::EntityCategory> =
            if !opts.is_undefined() && !opts.is_null() {
                if let Ok(cats_val) = js_sys::Reflect::get(&opts, &JsValue::from_str("categories"))
                {
                    if let Ok(arr) = cats_val.dyn_into::<js_sys::Array>() {
                        let mut cats = Vec::new();
                        for i in 0..arr.length() {
                            if let Some(s) = arr.get(i).as_string() {
                                if let Ok(cat) = serde_json::from_str::<xberg::types::entity::EntityCategory>(
                                    &format!("\"{}\"", s),
                                ) {
                                    cats.push(cat);
                                }
                            }
                        }
                        cats
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

        let entities = resolve_ner_with_timeout(self.ner.clone(), &text, &categories, self.bridge_timeout_ms)
            .await
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        serde_wasm_bindgen::to_value(&entities).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Query the RAG vector store with `q` in `collection`, returning top `k` results.
    ///
    /// Requires a `store` injection. If an `embedder` is also available, the query
    /// text will be embedded for vector similarity; otherwise full-text mode is used.
    #[allow(clippy::missing_errors_doc)]
    pub async fn query(
        &self,
        q: String,
        collection: String,
        k: u32,
    ) -> Result<JsValue, JsValue> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| JsValue::from_str("store not injected"))?;

        let mode = if self.embedder.is_some() {
            RetrieveMode::Vector
        } else {
            RetrieveMode::FullText
        };

        let query_vector = match self.embedder.as_ref() {
            Some(emb) => {
                let mut vecs = emb
                    .embed(vec![q.clone()])
                    .await
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                vecs.pop()
            }
            None => None,
        };

        let retrieve_query = RetrieveQuery {
            mode,
            query_text: Some(q),
            query_vector,
            top_k: k,
            filter: None,
            candidate_multiplier: None,
            group_by_document: false,
            include_content: true,
            include_document: false,
            graph_depth: None,
        };

        let output = store
            .retrieve(&collection, &retrieve_query)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&output).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Detect PII in `text`. Returns an array of `{ start, end, category, text }`.
    #[allow(clippy::missing_errors_doc)]
    pub fn detect_pii(
        &self,
        text: &str,
        categories: Option<Vec<String>>,
    ) -> Result<JsValue, JsValue> {
        let cats: Vec<xberg::types::redaction::PiiCategory> = categories
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
        let matches = xberg::text::redaction::patterns::scan_text(text, &cats);
        serde_wasm_bindgen::to_value(&matches).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Redact PII from `text` using the given `strategy`.
    ///
    /// Returns `{ redacted: string, rehydrationMap: { token: original } }`.
    ///
    /// NOTE: This method reimplements redaction logic inline rather than delegating
    /// to `xberg::text::redaction::redact`. In a future pass this should be replaced
    /// with a direct call to the core redaction API to avoid duplication.
    #[allow(clippy::missing_errors_doc)]
    pub fn redact(
        &self,
        text: &str,
        strategy: Option<String>,
        categories: Option<Vec<String>>,
    ) -> Result<JsValue, JsValue> {
        let strat: xberg::types::redaction::RedactionStrategy =
            strategy.unwrap_or_else(|| "token_replace".to_string()).into();
        let cats: Vec<xberg::types::redaction::PiiCategory> = categories
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();

        let matches = xberg::text::redaction::patterns::scan_text(text, &cats);

        // Pre-count per category so we can assign deterministic token indices
        // when processing in reverse.
        let mut category_counts: HashMap<String, u32> = HashMap::new();
        for m in &matches {
            let key = format!("{:?}", m.category);
            *category_counts.entry(key).or_insert(0) += 1;
        }

        let mut rehydration_map: RehydrationMap = HashMap::new();
        let mut running: HashMap<String, u32> = HashMap::new();
        let mut result = text.to_string();

        // Process matches in reverse byte order so replacements don't shift offsets.
        for m in matches.iter().rev() {
            let cat_key = format!("{:?}", m.category);
            let total = category_counts[&cat_key];
            let counter = running.entry(cat_key.clone()).or_insert(0);
            *counter += 1;
            // Token index counts from the end: total - (counter - 1)
            let idx = total - (*counter - 1);

            let replacement = match strat {
                xberg::types::redaction::RedactionStrategy::Mask => "[REDACTED]".to_string(),
                xberg::types::redaction::RedactionStrategy::Hash => {
                    use sha2::{Digest, Sha256};
                    let hash = Sha256::digest(m.text.as_bytes());
                    hash[..8].iter().map(|b| format!("{b:02x}")).collect::<String>()
                }
                xberg::types::redaction::RedactionStrategy::TokenReplace => {
                    let token = format!("[{}_{}]", cat_key.to_uppercase(), idx);
                    rehydration_map.insert(token.clone(), m.text.clone());
                    token
                }
                xberg::types::redaction::RedactionStrategy::Drop => String::new(),
            };

            result.replace_range(m.start..m.end, &replacement);
        }

        let out = js_sys::Object::new();
        js_sys::Reflect::set(&out, &"redacted".into(), &result.into())
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        js_sys::Reflect::set(
            &out,
            &"rehydrationMap".into(),
            &serde_wasm_bindgen::to_value(&rehydration_map)
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        )
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        Ok(out.into())
    }

    /// Encrypt a rehydration map with `passphrase`.
    ///
    /// Returns the raw ciphertext bytes (`XPII\x01` wire format).
    #[cfg(feature = "redaction-rehydrate")]
    #[allow(clippy::missing_errors_doc)]
    pub fn encrypt_map(
        &self,
        map: JsValue,
        passphrase: &str,
    ) -> Result<Vec<u8>, JsValue> {
        let inner: RehydrationMap =
            serde_wasm_bindgen::from_value(map).map_err(|e| JsValue::from_str(&e.to_string()))?;
        xberg::text::redaction::rehydration::encrypt_map(&inner, passphrase)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Decrypt an encrypted blob back into a token→original map.
    #[cfg(feature = "redaction-rehydrate")]
    #[allow(clippy::missing_errors_doc)]
    pub fn decrypt_map(
        &self,
        blob: Vec<u8>,
        passphrase: &str,
    ) -> Result<JsValue, JsValue> {
        let inner = xberg::text::redaction::rehydration::decrypt_map(&blob, passphrase)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Search a decrypted rehydration map for `query`, matching either the
    /// token (exact) or the original value (case-insensitive substring).
    ///
    /// Returns an array of `{ token, original, category }`.
    #[cfg(feature = "redaction-rehydrate")]
    #[allow(clippy::missing_errors_doc)]
    pub fn find_subject(&self, map: JsValue, query: &str) -> Result<JsValue, JsValue> {
        let inner: RehydrationMap =
            serde_wasm_bindgen::from_value(map).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let matches = xberg::text::redaction::rehydration::find_subject(&inner, query);
        serde_wasm_bindgen::to_value(&matches).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Remove every mapping in `map` whose token or original value matches
    /// `query`. Mutates a copy and returns
    /// `{ removed: [{ token, original, category }], remaining: { token: original } }` —
    /// the caller re-encrypts `remaining` with [`XbergEngine::encrypt_map`]
    /// and persists it; this method does not touch disk.
    #[cfg(feature = "redaction-rehydrate")]
    #[allow(clippy::missing_errors_doc)]
    pub fn forget_subject(&self, map: JsValue, query: &str) -> Result<JsValue, JsValue> {
        let mut inner: RehydrationMap =
            serde_wasm_bindgen::from_value(map).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let removed = xberg::text::redaction::rehydration::forget_subject(&mut inner, query);

        let out = js_sys::Object::new();
        js_sys::Reflect::set(
            &out,
            &"removed".into(),
            &serde_wasm_bindgen::to_value(&removed).map_err(|e| JsValue::from_str(&e.to_string()))?,
        )
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        js_sys::Reflect::set(
            &out,
            &"remaining".into(),
            &serde_wasm_bindgen::to_value(&inner).map_err(|e| JsValue::from_str(&e.to_string()))?,
        )
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        Ok(out.into())
    }

    /// Return aggregate statistics for the WASM extraction cache.
    #[allow(clippy::missing_errors_doc)]
    pub fn cache_stats(&self) -> Result<JsValue, JsValue> {
        let stats = crate::WasmCacheStats::default();
        Ok(stats.into())
    }

    /// Invalidate all cached extraction results.
    #[allow(clippy::missing_errors_doc)]
    pub fn invalidate_cache(&self) -> Result<(), JsValue> {
        // Cache is in-memory per WASM instance — dropping the cache is a no-op
        // because each engine instance owns its own process.  Return Ok so JS
        // callers can chain without a try/catch.
        Ok(())
    }
}
