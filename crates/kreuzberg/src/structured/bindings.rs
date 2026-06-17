//! JSON-string-args binding entry points for the structured-extraction engine.
//!
//! These functions are the synchronous JSON-in / JSON-out bridge used by
//! language bindings (FFI, CLI, WASM-adjacent) where async, `Arc<dyn Trait>`,
//! and boxed generics cannot cross the language boundary.
//!
//! # Calling convention
//!
//! Both functions accept:
//!
//! - `bytes` — raw document bytes.
//! - `mime` — MIME type string (e.g. `"text/plain"`, `"application/pdf"`).
//! - `preset_spec_json` — JSON string encoding a [`super::PresetSpec`]:
//!   `{"named":"invoice"}` or `{"inline":{...full Preset JSON...}}`.
//! - `options_json` — JSON string encoding a subset of [`super::StructuredOptions`]
//!   (every field *except* `cache`).  Missing fields default to their
//!   [`super::StructuredOptions`] defaults.
//!
//! Both return `Ok(String)` on success — the JSON-serialised
//! [`super::StructuredOutput`] (or a JSON array thereof for
//! [`split_and_extract_json`]).

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{StructuredError, StructuredOptions, VisionConfig};
use crate::core::config::LlmConfig;
use crate::heuristics::{StructuredCallMode, StructuredThresholds};

// ── Deserializable mirror of StructuredOptions (cache excluded) ───────────────

/// Serialisable mirror of [`super::StructuredOptions`] used only at the
/// JSON-deserialization layer.  The `cache` field is not representable in JSON
/// and is always set to `None` when converting to [`StructuredOptions`].
#[derive(Debug, Deserialize, Default)]
struct StructuredOptionsJson {
    #[serde(default)]
    llm: LlmConfig,
    #[serde(default)]
    thresholds: StructuredThresholds,
    #[serde(default)]
    force_call_mode: Option<StructuredCallMode>,
    #[serde(default)]
    context: BTreeMap<String, String>,
    #[serde(default)]
    max_parallel_calls: Option<usize>,
    #[serde(default)]
    vision: VisionConfig,
    #[serde(default)]
    emit_citations: Option<bool>,
}

impl From<StructuredOptionsJson> for StructuredOptions {
    fn from(j: StructuredOptionsJson) -> Self {
        let defaults = StructuredOptions::default();
        Self {
            llm: j.llm,
            thresholds: j.thresholds,
            force_call_mode: j.force_call_mode,
            context: j.context,
            cache: None,
            max_parallel_calls: j.max_parallel_calls.unwrap_or(defaults.max_parallel_calls),
            vision: j.vision,
            emit_citations: j.emit_citations,
        }
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Extract structured JSON from a document using JSON-encoded preset spec and options.
///
/// This is the synchronous JSON-in / JSON-out entry point suitable for FFI and
/// language-binding call paths.
///
/// # Arguments
///
/// * `bytes` — raw document bytes.
/// * `mime` — MIME type string.
/// * `preset_spec_json` — JSON string: `{"named":"id"}` or `{"inline":{...}}`.
/// * `options_json` — JSON string mirroring [`StructuredOptions`] fields (except
///   `cache`).  Pass `"{}"` to use all defaults.
///
/// # Returns
///
/// JSON-serialised [`super::StructuredOutput`] on success.
///
/// # Errors
///
/// Returns [`StructuredError::InvalidJson`] when either JSON argument is
/// malformed.  All other error variants come from the underlying
/// [`super::extract_structured_sync`] call.
pub fn extract_structured_json(
    bytes: &[u8],
    mime: &str,
    preset_spec_json: &str,
    options_json: &str,
) -> Result<String, StructuredError> {
    let spec = parse_preset_spec(preset_spec_json)?;
    let options = parse_options(options_json)?;
    let output = super::extract_structured_sync(bytes, mime, spec, options)?;
    serde_json::to_string(&output)
        .map_err(|e| StructuredError::InvalidJson(format!("failed to serialise output: {e}")))
}

/// Split a multi-document PDF and extract structured JSON from each segment,
/// returning a JSON array of [`super::StructuredOutput`] objects.
///
/// Non-PDF documents are passed through as a single-element array.
///
/// # Arguments
///
/// Same as [`extract_structured_json`].
///
/// # Returns
///
/// JSON-serialised `Vec<`[`super::StructuredOutput`]`>` (a JSON array) on success.
///
/// # Errors
///
/// Returns [`StructuredError::InvalidJson`] when either JSON argument is
/// malformed.  All other error variants come from the underlying
/// [`super::split_and_extract_sync`] call.
pub fn split_and_extract_json(
    bytes: &[u8],
    mime: &str,
    preset_spec_json: &str,
    options_json: &str,
) -> Result<String, StructuredError> {
    let spec = parse_preset_spec(preset_spec_json)?;
    let options = parse_options(options_json)?;
    let outputs = super::split_and_extract_sync(bytes, mime, spec, options)?;
    serde_json::to_string(&outputs)
        .map_err(|e| StructuredError::InvalidJson(format!("failed to serialise output array: {e}")))
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn parse_preset_spec(json: &str) -> Result<super::PresetSpec, StructuredError> {
    serde_json::from_str(json)
        .map_err(|e| StructuredError::InvalidJson(format!("preset_spec_json: {e}")))
}

fn parse_options(json: &str) -> Result<StructuredOptions, StructuredError> {
    let mirror: StructuredOptionsJson = serde_json::from_str(json)
        .map_err(|e| StructuredError::InvalidJson(format!("options_json: {e}")))
        ?;
    Ok(StructuredOptions::from(mirror))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn two_field_preset_json() -> serde_json::Value {
        json!({
            "id": "test_invoice",
            "version": "v1",
            "schema_name": "test_invoice",
            "description": "Minimal test preset",
            "category": "finance",
            "tags": [],
            "schema": {
                "type": "object",
                "properties": {
                    "invoice_number": {"type": "string"},
                    "vendor": {"type": "string"}
                },
                "required": ["invoice_number", "vendor"]
            },
            "system_prompt": "Extract invoice_number and vendor.",
            "merge_mode": "object_merge",
            "preferred_call_mode": "text_only",
            "emit_citations": false
        })
    }

    fn stub_openai_response(json_str: &str) -> serde_json::Value {
        json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 0,
            "model": "openai/gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": json_str},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 20,
                "total_tokens": 120
            }
        })
    }

    /// Build `options_json` pointing at the given mock server URI.
    fn options_json_for_server(uri: &str) -> String {
        json!({
            "llm": {
                "model": "openai/gpt-4o",
                "api_key": "test-key",
                "base_url": uri
            },
            "thresholds": {
                "docx_text_min_density": 50.0
            },
            "force_call_mode": "text_only"
        })
        .to_string()
    }

    const PLAIN_TEXT_MIME: &str = "text/plain";
    const PLAIN_TEXT_CONTENT: &[u8] = b"Invoice number: INV-001\nVendor: Acme Corp\n\
        Total: $42.00\nThis document has enough text to satisfy the density threshold \
        for text-mode extraction and further content goes here.";

    // ── Test: inline-preset round-trip ────────────────────────────────────────

    /// Happy-path: inline preset, text-only with wiremock stub, result is valid JSON
    /// containing the expected `structured_output_flat` fields.
    ///
    /// The sync wrapper is called from `spawn_blocking` to avoid "runtime within
    /// a runtime" panics when running under `#[tokio::test]`.
    #[tokio::test]
    async fn inline_preset_round_trip_returns_expected_fields() {
        let server = MockServer::start().await;

        let response_body = r#"{"invoice_number":"INV-001","vendor":"Acme Corp"}"#;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(stub_openai_response(response_body)),
            )
            .mount(&server)
            .await;

        let preset_spec_json =
            json!({"inline": two_field_preset_json()}).to_string();
        let options_json = options_json_for_server(&server.uri());
        let bytes = PLAIN_TEXT_CONTENT.to_vec();
        let mime = PLAIN_TEXT_MIME.to_string();

        // extract_structured_json calls block_on internally; run it off the
        // current async thread via spawn_blocking to avoid the nested-runtime
        // panic.
        let json_str = tokio::task::spawn_blocking(move || {
            extract_structured_json(&bytes, &mime, &preset_spec_json, &options_json)
        })
        .await
        .expect("spawn_blocking must not panic")
        .expect("extract_structured_json must succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("result must be valid JSON");

        assert_eq!(
            parsed["structured_output_flat"]["invoice_number"].as_str(),
            Some("INV-001"),
            "invoice_number must match stub response"
        );
        assert_eq!(
            parsed["structured_output_flat"]["vendor"].as_str(),
            Some("Acme Corp"),
            "vendor must match stub response"
        );
        // preset_id and preset_version must be set
        assert_eq!(parsed["preset_id"].as_str(), Some("test_invoice"));
        assert_eq!(parsed["preset_version"].as_str(), Some("v1"));
    }

    // ── Test: malformed preset_spec_json → InvalidJson ────────────────────────

    #[test]
    fn malformed_preset_spec_json_returns_invalid_json_error() {
        let bad_json = "this is not json {{{";
        let options_json = "{}";

        let err = extract_structured_json(b"hello", "text/plain", bad_json, options_json)
            .expect_err("must fail on malformed preset_spec_json");

        assert!(
            matches!(err, StructuredError::InvalidJson(_)),
            "expected InvalidJson, got: {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("preset_spec_json"),
            "error message must identify the argument, got: {msg}"
        );
    }

    // ── Test: malformed options_json → InvalidJson ────────────────────────────

    #[test]
    fn malformed_options_json_returns_invalid_json_error() {
        let preset_spec_json = json!({"named": "generic_document"}).to_string();
        let bad_options = r#"{"llm": not-an-object}"#;

        let err = extract_structured_json(b"hello", "text/plain", &preset_spec_json, bad_options)
            .expect_err("must fail on malformed options_json");

        assert!(
            matches!(err, StructuredError::InvalidJson(_)),
            "expected InvalidJson, got: {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("options_json"),
            "error message must identify the argument, got: {msg}"
        );
    }

    // ── Test: split_and_extract_json on non-PDF returns single-element array ──

    #[tokio::test]
    async fn split_and_extract_json_non_pdf_returns_single_element_array() {
        let server = MockServer::start().await;

        let response_body = r#"{"invoice_number":"INV-002","vendor":"Widget Co"}"#;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(stub_openai_response(response_body)),
            )
            .mount(&server)
            .await;

        let preset_spec_json =
            json!({"inline": two_field_preset_json()}).to_string();
        let options_json = options_json_for_server(&server.uri());
        let bytes = PLAIN_TEXT_CONTENT.to_vec();
        let mime = PLAIN_TEXT_MIME.to_string();

        // split_and_extract_json calls block_on internally; run it off the
        // current async thread via spawn_blocking to avoid the nested-runtime
        // panic.
        let json_str = tokio::task::spawn_blocking(move || {
            split_and_extract_json(&bytes, &mime, &preset_spec_json, &options_json)
        })
        .await
        .expect("spawn_blocking must not panic")
        .expect("split_and_extract_json must succeed for text/plain");

        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("result must be valid JSON");

        let arr = parsed.as_array().expect("result must be a JSON array");
        assert_eq!(
            arr.len(),
            1,
            "non-PDF input must produce exactly one element, got {}",
            arr.len()
        );
        assert_eq!(
            arr[0]["structured_output_flat"]["invoice_number"].as_str(),
            Some("INV-002"),
        );
    }

    // ── Test: default options_json ("{}") is accepted ─────────────────────────

    #[test]
    fn parse_options_accepts_empty_object() {
        let opts = parse_options("{}").expect("empty JSON object must parse");
        // Verify defaults match StructuredOptions::default()
        let defaults = StructuredOptions::default();
        assert_eq!(opts.max_parallel_calls, defaults.max_parallel_calls);
        assert!(opts.cache.is_none());
        assert!(opts.force_call_mode.is_none());
    }

    // ── Test: named PresetSpec round-trips through parse_preset_spec ──────────

    #[test]
    fn parse_preset_spec_named_variant() {
        let json = r#"{"named": "generic_document"}"#;
        let spec = parse_preset_spec(json).expect("named variant must parse");
        assert!(
            matches!(spec, super::super::PresetSpec::Named(ref id) if id == "generic_document"),
            "expected Named(\"generic_document\"), got: {spec:?}"
        );
    }
}
