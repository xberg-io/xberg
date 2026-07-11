//! NER (Named Entity Recognition) bridge with injected-first dispatch.
//!
//! The WASM engine tries an externally-injected JavaScript object first —
//! `ner(text, categories)`, called positionally to match this file's own
//! `call_injected_ner`. When no injection is provided, it falls back to an
//! in-binary Candle GLiNER2 backend (`crates/xberg-gliner-candle`, via
//! `xberg::text::ner::candle::CandleBackend`), initialized once via
//! `initCandleNer` (JS calls this after downloading the pinned model's
//! three files — see packages/xberg-wasm-runtime's CacheManager). wasm32
//! is single-threaded, so a thread-local cache (not a Mutex-guarded
//! static, unlike the native multi-key CANDLE_BACKEND_CACHE in
//! xberg::text::ner::candle) is sufficient and simpler.

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;

use xberg::text::ner::NerBackend;
use xberg::text::ner::candle::CandleBackend;
use xberg::types::entity::{Entity, EntityCategory};

thread_local! {
    // Rc, not a bare CandleBackend: fallback_ner clones the Rc out of the
    // cell and drops the RefCell borrow *before* awaiting detect() below —
    // holding a RefCell borrow across an .await is a footgun (a re-entrant
    // call while the future is suspended would panic on double-borrow).
    static CANDLE_NER: std::cell::RefCell<Option<std::rc::Rc<CandleBackend>>> = const { std::cell::RefCell::new(None) };
}

/// Initialize the in-binary Candle NER fallback from in-memory model bytes.
/// JS calls this once, after downloading the pinned PII model's
/// `model.safetensors`, `tokenizer.json`, and `encoder_config/config.json`.
/// Calling this more than once replaces the previously-loaded model.
#[allow(clippy::missing_errors_doc)]
#[wasm_bindgen(js_name = "initCandleNer")]
pub fn init_candle_ner(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> Result<(), JsValue> {
    let backend = CandleBackend::from_bytes(safetensors, tokenizer_json, encoder_config_json)
        .map_err(|e| js_from_any(format!("initCandleNer: {e}")))?;
    CANDLE_NER.with(|cell| {
        *cell.borrow_mut() = Some(std::rc::Rc::new(backend));
    });
    Ok(())
}

/// Return the currently-loaded Candle NER backend, if `initCandleNer` has
/// been called. Used by `engine.rs::ingest()` to thread the already-loaded
/// model into `xberg-rag`'s mandatory PII+NER redaction step.
pub(crate) fn get_candle_ner() -> Option<std::rc::Rc<CandleBackend>> {
    CANDLE_NER.with(|cell| cell.borrow().clone())
}

/// Resolve the best available NER backend for the current request.
///
/// 1. If `injected` is `Some(obj)`, call `obj.ner(text, categories)`.
/// 2. If `injected` is `None`, use the in-binary Candle backend if
///    `initCandleNer` has been called.
/// 3. Otherwise return an error explaining that NER is unavailable.
pub async fn resolve_ner(
    injected: Option<js_sys::Object>,
    text: &str,
    categories: &[EntityCategory],
) -> Result<Vec<Entity>, JsValue> {
    resolve_ner_with_timeout(injected, text, categories, crate::bridge::BRIDGE_TIMEOUT_MS).await
}

/// Like [`resolve_ner`] but with a configurable bridge timeout.
pub async fn resolve_ner_with_timeout(
    injected: Option<js_sys::Object>,
    text: &str,
    categories: &[EntityCategory],
    timeout_ms: u32,
) -> Result<Vec<Entity>, JsValue> {
    match injected {
        Some(obj) => call_injected_ner(obj, text, categories, timeout_ms).await,
        None => fallback_ner(text, categories).await,
    }
}

/// Call the injected JS `ner(text, categories)` method and deserialize the
/// returned promise into a Vec<Entity>.
async fn call_injected_ner(
    obj: Object,
    text: &str,
    categories: &[EntityCategory],
    timeout_ms: u32,
) -> Result<Vec<Entity>, JsValue> {
    let fn_val = Reflect::get(&obj, &JsValue::from_str("ner"))
        .map_err(|e| js_from_any(format!("failed to read 'ner' property: {e:?}")))?;
    let func: Function = fn_val
        .dyn_into()
        .map_err(|_| js_from_any("injected NER object has no 'ner' function"))?;

    let js_text = JsValue::from_str(text);
    let js_cats = js_sys::Array::new();
    for c in categories {
        let cat_str = serde_json::to_value(c)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        js_cats.push(&JsValue::from_str(&cat_str));
    }
    let args = js_sys::Array::of2(&js_text, &js_cats);

    let result = func.apply(&obj, &args)?;
    let promise = Promise::from(result);
    let js_val = crate::bridge::timed_js_future_with_timeout(promise, timeout_ms).await?;

    let entities: Vec<Entity> = serde_wasm_bindgen::from_value(js_val)
        .map_err(|e| js_from_any(format!("failed to deserialize NER result: {e}")))?;
    Ok(entities)
}

/// In-binary NER fallback. Uses the Candle backend initialized via
/// `initCandleNer`, if any; otherwise returns a diagnostic error.
///
/// Clones the `Rc<CandleBackend>` out of the thread-local cell and drops the
/// `RefCell` borrow before awaiting `detect()` — `resolve_ner_with_timeout`
/// (the caller) is already `async`, so this just awaits directly; no
/// blocking executor (pollster et al.) is needed or wasm32-safe here.
async fn fallback_ner(text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>, JsValue> {
    let backend = CANDLE_NER.with(|cell| cell.borrow().clone());
    match backend {
        Some(backend) => backend
            .detect(text, categories)
            .await
            .map_err(|e| js_from_any(format!("Candle NER inference: {e}"))),
        None => Err(js_from_any(
            "NER unavailable: no injected backend and initCandleNer has not been called",
        )),
    }
}

/// Adapter that wraps an injected JS NER object as a [`NerBackend`].
///
/// Used by `engine.ingest()` to feed the injected JS NER into
/// `xberg-rag`'s `redact_request` pipeline, which requires a
/// `&dyn NerBackend`. The JS bridge is already async and fits the
/// `NerBackend::detect` contract directly.
pub(crate) struct JsNerBridge {
    obj: Object,
    timeout_ms: u32,
}

impl JsNerBridge {
    /// Wrap an injected JS object that exposes `ner(text, categories)`.
    pub fn new(obj: Object, timeout_ms: u32) -> Self {
        Self { obj, timeout_ms }
    }
}

#[async_trait(?Send)]
impl NerBackend for JsNerBridge {
    async fn detect(
        &self,
        text: &str,
        categories: &[EntityCategory],
    ) -> xberg::Result<Vec<Entity>> {
        call_injected_ner(self.obj.clone(), text, categories, self.timeout_ms)
            .await
            .map_err(|e| xberg::XbergError::Plugin(format!("JS NER bridge: {e:?}")))
    }
}

/// Resolve the best NER backend for `ingest()`, preferring the injected JS
/// bridge when available, falling back to the in-binary Candle backend.
///
/// Returns `Ok(Some(backend))` if NER is available, `Ok(None)` only if
/// both are unavailable (caller should error), or `Err` on init failure.
pub(crate) fn resolve_ingest_ner(
    injected: Option<&js_sys::Object>,
    timeout_ms: u32,
) -> Option<Box<dyn NerBackend>> {
    // Injected JS bridge takes priority — it's always available in browser
    // contexts where Candle may not have been initialized.
    if let Some(obj) = injected {
        return Some(Box::new(JsNerBridge::new(obj.clone(), timeout_ms)));
    }
    // Fall back to the Candle backend if initCandleNer was called.
    None
}

/// Convert a Display error into a JsValue suitable for propagation.
fn js_from_any(v: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&v.to_string())
}
