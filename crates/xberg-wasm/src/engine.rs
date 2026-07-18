//! `XbergEngine` — a stateful handle carrying injected bridges and
//! convenience methods on top of the raw WASM exports.

use js_sys::Object;
use wasm_bindgen::prelude::*;

use crate::bridge::ocr::resolve_ocr_with_timeout;

/// Extract an optional JS object field, returning `None` if the field is
/// missing, `null`, or `undefined`.
fn get_opt_field(obj: &Object, field: &str) -> Result<Option<Object>, JsValue> {
    let val = js_sys::Reflect::get(obj, &JsValue::from_str(field))
        .map_err(|_| JsValue::from_str(&format!("failed to read field '{field}'")))?;
    if val.is_undefined() || val.is_null() {
        return Ok(None);
    }
    val.dyn_into::<Object>()
        .map(Some)
        .map_err(|_| JsValue::from_str(&format!("field '{field}' must be an object")))
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

/// Stateful engine handle exposed to JS.
///
/// Constructed via `XbergEngine.new(config, injection)` where `config` may
/// contain optional settings (e.g. `bridgeTimeoutMs`) and `injection` is a
/// plain object with an optional `ocr` key.
#[wasm_bindgen]
pub struct XbergEngine {
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
    /// - `ocr` — object with `ocr(imageBytes, opts): Promise<{ text: string, lines?: Array<{ text: string, confidence: number, bbox?: { x: number, y: number, w: number, h: number } }> }>`
    ///
    /// Unknown injection keys are ignored, so hosts can pass richer injection
    /// objects shared with other engines.
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue, injection: JsValue) -> Result<XbergEngine, JsValue> {
        let bridge_timeout_ms = if config.is_undefined() || config.is_null() {
            crate::bridge::BRIDGE_TIMEOUT_MS
        } else {
            let config_obj: Object = config
                .dyn_into()
                .map_err(|_| JsValue::from_str("config must be an object"))?;
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

        let ocr = get_opt_field(&obj, "ocr")?;

        Ok(XbergEngine { ocr, bridge_timeout_ms })
    }

    /// Extract content from a single bytes or URI input.
    #[allow(clippy::missing_errors_doc)]
    pub async fn extract(&self, input: JsValue, config: JsValue) -> Result<JsValue, JsValue> {
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

    /// Perform OCR on image bytes, returning extracted text with per-line
    /// confidence and bounding-box geometry (when the backend provides it).
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

        let result = resolve_ocr_with_timeout(self.ocr.clone(), &bytes, &language, self.bridge_timeout_ms)
            .await
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
