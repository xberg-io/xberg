//! Type conversions and utilities for WASM bindings
//!
//! This module provides type conversions between Rust and JavaScript/TypeScript types
//! for seamless interoperability. Includes helpers for configuration and result handling.

use kreuzberg::{
    ExtractionConfig, ExtractionResult,
    utils::{camel_to_snake, snake_to_camel},
};
use wasm_bindgen::prelude::*;

/// Parse extraction configuration from JsValue using serde-wasm-bindgen.
///
/// Converts a JavaScript object to a Rust ExtractionConfig structure.
/// If config is None, returns the default ExtractionConfig.
///
/// # Arguments
///
/// * `config` - JavaScript object with extraction configuration (optional)
///
/// # Returns
///
/// Result containing the parsed ExtractionConfig or a JsValue error
pub fn parse_config(config: Option<JsValue>) -> Result<ExtractionConfig, JsValue> {
    match config {
        Some(js_config) => {
            // JS consumers send camelCase keys (e.g. includeDocumentStructure).
            // serde expects snake_case field names, so convert first:
            // JsValue → serde_json::Value → camel_to_snake → deserialize.
            let json_value: serde_json::Value = serde_wasm_bindgen::from_value(js_config)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {e}")))?;
            let snake_value = camel_to_snake(json_value);
            serde_json::from_value(snake_value).map_err(|e| JsValue::from_str(&format!("Failed to parse config: {e}")))
        }
        None => Ok(ExtractionConfig::default()),
    }
}

/// Convert extraction result to JsValue for JavaScript consumption.
///
/// Serializes to a JSON string via `serde_json` and then parses it with
/// `js_sys::JSON::parse()`. This works around a `serde_wasm_bindgen` v0.6
/// limitation: it doesn't properly handle `#[serde(flatten)]` combined with
/// internally-tagged enums (`#[serde(tag = "...")]`). The `Metadata.format`
/// field uses both, causing `format_type` and all format-specific fields to
/// be silently dropped when serializing directly with `serde_wasm_bindgen`.
///
/// By going through `serde_json` → JSON string → `JSON.parse()`, we bypass
/// `serde_wasm_bindgen` entirely for output and preserve all metadata fields.
pub fn result_to_js_value(result: &ExtractionResult) -> Result<JsValue, JsValue> {
    let json_value =
        serde_json::to_value(result).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {e}")))?;
    // Transform snake_case keys to camelCase for JavaScript/TypeScript consumers
    let camel_value = snake_to_camel(json_value);
    let json_string = serde_json::to_string(&camel_value)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {e}")))?;
    js_sys::JSON::parse(&json_string).map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {e:?}")))
}

/// Convert a vector of results to JsValue.
///
/// Uses the same `serde_json` → `JSON.parse()` approach as `result_to_js_value`
/// to preserve flattened metadata fields.
pub fn results_to_js_value(results: &[ExtractionResult]) -> Result<JsValue, JsValue> {
    let json_value =
        serde_json::to_value(results).map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {e}")))?;
    // Transform snake_case keys to camelCase for JavaScript/TypeScript consumers
    let camel_value = snake_to_camel(json_value);
    let json_string = serde_json::to_string(&camel_value)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {e}")))?;
    js_sys::JSON::parse(&json_string).map_err(|e| JsValue::from_str(&format!("Failed to parse results JSON: {e:?}")))
}
