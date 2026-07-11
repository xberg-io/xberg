//! OCR (Optical Character Recognition) bridge with injected-first dispatch.
//!
//! Similar to the NER bridge, the WASM engine prefers an externally-
//! injected JavaScript object that implements an
//! `ocr(imageBytes, options)` async method.  When no injection is
//! present it attempts an in-binary Tesseract fallback under
//! `#[cfg(feature = "ocr-wasm")]`.

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;

/// Resolve the best available OCR backend and return extracted text.
///
/// 1. If `injected` is `Some(obj)`, call
///    `obj.ocr(imageBytes, { language })` — the host returns a
///    promise resolving to `{ text: "..." }`.
/// 2. If `injected` is `None` and `ocr-wasm` is enabled, attempt
///    the in-binary Tesseract backend.
/// 3. Otherwise return an error explaining that OCR is unavailable.
pub async fn resolve_ocr(
    injected: Option<js_sys::Object>,
    image_bytes: &[u8],
    language: &str,
) -> Result<String, JsValue> {
    resolve_ocr_with_timeout(injected, image_bytes, language, crate::bridge::BRIDGE_TIMEOUT_MS).await
}

/// Like [`resolve_ocr`] but with a configurable bridge timeout.
pub async fn resolve_ocr_with_timeout(
    injected: Option<js_sys::Object>,
    image_bytes: &[u8],
    language: &str,
    timeout_ms: u32,
) -> Result<String, JsValue> {
    match injected {
        Some(obj) => call_injected_ocr(obj, image_bytes, language, timeout_ms).await,
        None => fallback_ocr(image_bytes, language).await,
    }
}

/// Call the injected JS `ocr(imageBytes, { language })` method.
async fn call_injected_ocr(
    obj: Object,
    image_bytes: &[u8],
    language: &str,
    timeout_ms: u32,
) -> Result<String, JsValue> {
    let fn_val = Reflect::get(&obj, &JsValue::from_str("ocr"))
        .map_err(|e| js_from_any(format!("failed to read 'ocr' property: {e:?}")))?;
    let func: Function = fn_val
        .dyn_into()
        .map_err(|_| js_from_any("injected OCR object has no 'ocr' function"))?;

    let js_bytes = js_sys::Uint8Array::from(image_bytes);
    let opts = js_sys::Object::new();
    Reflect::set(&opts, &JsValue::from_str("language"), &JsValue::from_str(language))?;

    let args = js_sys::Array::of2(&js_bytes, &opts);
    let result = func.apply(&obj, &args)?;
    let promise = Promise::from(result);
    let js_val = crate::bridge::timed_js_future_with_timeout(promise, timeout_ms).await?;

    let text = Reflect::get(&js_val, &JsValue::from_str("text"))
        .map_err(|e| js_from_any(format!("ocr result missing 'text': {e:?}")))?
        .as_string()
        .ok_or_else(|| js_from_any("ocr result 'text' is not a string"))?;
    Ok(text)
}

/// In-binary OCR fallback via Tesseract WASM backend.
async fn fallback_ocr(image_bytes: &[u8], language: &str) -> Result<String, JsValue> {
    if image_bytes.is_empty() {
        return Err(js_from_any("OCR input image is empty"));
    }

    #[cfg(all(feature = "ocr-wasm", not(feature = "ocr")))]
    {
        // TesseractWasmBackend::new() is pub(crate) in xberg, so we cannot
        // construct it from xberg-wasm.  Return a diagnostic error.
        let _ = language;
        Err(js_from_any(
            "OCR unavailable: no injected backend and ocr-wasm backend constructor is not public; \
             provide an injected JS backend or use the full xberg API directly",
        ))
    }

    #[cfg(not(all(feature = "ocr-wasm", not(feature = "ocr"))))]
    {
        Err(js_from_any(
            "OCR unavailable: no injected backend and ocr-wasm disabled",
        ))
    }
}

/// Convert a Display error into a JsValue suitable for propagation.
fn js_from_any(v: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&v.to_string())
}
