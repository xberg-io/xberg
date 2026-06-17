//! Build system + user prompts for the vision-LLM call.
//!
//! # Design notes
//!
//! **`Preset` vs `ResolvedPreset`**: this module accepts [`crate::presets::Preset`] directly
//! rather than [`crate::presets::resolve::ResolvedPreset`].  `ResolvedPreset` is the output of
//! [`crate::presets::resolve::resolve`], which *already* renders `context_template` into
//! `system_prompt`; accepting it here would silently double-substitute.  `Preset` is the correct
//! input: the prompt builder owns the substitution step so it can apply `max_excerpt_chars`
//! truncation and fence the extracted-text excerpt independently of the resolve step.
//!
//! **Context type**: callers pass `&BTreeMap<String, String>` (the type carried by
//! [`super::StructuredOptions::context`]) rather than the cloud-side
//! `Option<&serde_json::Map<String, serde_json::Value>>`.  The substitution logic is equivalent
//! but simpler: every key maps to a plain `&str`.
//!
//! **Fence nonce**: a small random-looking nonce fences untrusted content (extracted text, prior
//! LLM JSON) inside the prompt.  An attacker planting content in the document cannot predict the
//! nonce, so they cannot escape the fence.  Generated with an atomic counter seeded from
//! `std::time::SystemTime` — no external deps required.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::heuristics::{ExtractionConfidence, StructuredCallMode};
use crate::presets::Preset;

// ---------------------------------------------------------------------------
// Citation instruction
// ---------------------------------------------------------------------------

/// Generic description of the per-field citation envelope.
///
/// This is an intentionally neutral description of the `{value, page, bbox,
/// confidence}` shape.  Preset-specific prompt bodies live in the preset itself;
/// only the structural shape is defined here.
const CITATION_INSTRUCTION: &str = concat!(
    "\n\n---\n\n",
    "When returning field values, wrap each field as an object with the following keys:\n",
    "- value: the extracted field value (any JSON type)\n",
    "- page: the 1-indexed page number where the value appears, or null if unavailable\n",
    "- bbox: the bounding box as [x1, y1, x2, y2] in normalised page coordinates",
    " (0.0-1.0), or null if unavailable\n",
    "- confidence: a score in 0.0-1.0 representing extraction confidence, or null if unavailable\n",
    "\nExample:\n",
    "{\n",
    "  \"invoice_number\": {\n",
    "    \"value\": \"INV-2024-001\",\n",
    "    \"page\": 1,\n",
    "    \"bbox\": [0.05, 0.12, 0.45, 0.18],\n",
    "    \"confidence\": 0.97\n",
    "  }\n",
    "}",
);

// ---------------------------------------------------------------------------
// Nonce generation (no external deps)
// ---------------------------------------------------------------------------

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a small nonce from a combination of the system clock and an
/// atomic counter.  The result is 16 hex characters.
fn fence_nonce() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{ts:08x}{counter:08x}")
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Built prompt components ready to send to the vision model.
#[derive(Debug, Clone)]
pub struct BuiltPrompt {
    /// System instruction primer.
    pub system: String,
    /// User content (text excerpt + rendered context template).
    ///
    /// `None` for [`StructuredCallMode::VisionOnly`] (images-only call) and
    /// [`StructuredCallMode::Skip`].
    pub user_text: Option<String>,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Build system + user prompts from a preset, caller context, and the
/// extracted-text excerpt.
///
/// - The system prompt is `preset.system_prompt` with `{{var}}` substitution
///   applied from `context`.
/// - The citation instruction is appended to the system prompt when
///   `preset.emit_citations` is `true`.
/// - The user text carries the rendered `preset.context_template` (if
///   present) followed by a `---` separator and the excerpt (for text-bearing
///   modes).
/// - `VisionOnly` yields `user_text = None`; the caller attaches page images.
/// - `Skip` yields `user_text = None` (no content to send).
/// - The excerpt is truncated to `max_excerpt_chars` before inclusion.
pub fn build(
    preset: &Preset,
    context: &BTreeMap<String, String>,
    extracted_text_excerpt: &str,
    call_mode: StructuredCallMode,
    max_excerpt_chars: usize,
) -> BuiltPrompt {
    let mut system = substitute_vars(&preset.system_prompt, context);

    if preset.emit_citations {
        system.push_str(CITATION_INSTRUCTION);
    }

    let user_text = match call_mode {
        StructuredCallMode::VisionOnly => None,
        StructuredCallMode::Skip => {
            tracing::warn!("build() called with Skip mode; returning no user content");
            None
        }
        StructuredCallMode::TextOnly
        | StructuredCallMode::TextPlusVision
        | StructuredCallMode::TextOnlyWithVisionFallback => {
            let mut content = String::new();

            if let Some(template) = &preset.context_template {
                let rendered = substitute_vars(template, context);
                content.push_str(&rendered);
            }

            let excerpt = truncate_to_char_boundary(extracted_text_excerpt, max_excerpt_chars);

            if !content.is_empty() && !excerpt.is_empty() {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(excerpt);

            Some(content)
        }
    };

    BuiltPrompt { system, user_text }
}

/// Build a vision-fallback prompt when a text-only extraction pass had low
/// confidence.
///
/// Composes: rendered system prompt → rendered `context_template` (if present) →
/// fenced extracted-text excerpt → fenced prior-JSON output → confidence
/// breakdown → instruction to correct/complete using page images.
///
/// The caller must attach the page images to the request separately; this
/// function produces the text portion only.
pub fn build_vision_fallback(
    preset: &Preset,
    context: &BTreeMap<String, String>,
    extracted_text_excerpt: &str,
    prior_json: &serde_json::Value,
    confidence: &ExtractionConfidence,
    max_excerpt_chars: usize,
) -> BuiltPrompt {
    let mut system = substitute_vars(&preset.system_prompt, context);

    if preset.emit_citations {
        system.push_str(CITATION_INSTRUCTION);
    }

    let mut content = String::new();

    if let Some(template) = &preset.context_template {
        let rendered = substitute_vars(template, context);
        content.push_str(&rendered);
    }

    let excerpt = truncate_to_char_boundary(extracted_text_excerpt, max_excerpt_chars);
    let nonce = fence_nonce();

    if !excerpt.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\n");
        }
        content.push_str(&format!("--- BEGIN EXTRACTED_TEXT_{nonce} ---\n"));
        content.push_str(excerpt);
        content.push_str(&format!("\n--- END EXTRACTED_TEXT_{nonce} ---\n"));
    }

    content.push_str("\nPrior text-only extraction (low confidence):\n");
    content.push_str(&format!("--- BEGIN PRIOR_JSON_{nonce} ---\n"));
    let prior_str = match serde_json::to_string_pretty(prior_json) {
        Ok(pretty) => pretty,
        Err(_) => {
            let fallback = serde_json::json!({"error": "prior extraction unavailable"});
            serde_json::to_string_pretty(&fallback).unwrap_or_default()
        }
    };
    content.push_str(&prior_str);
    content.push_str(&format!("\n--- END PRIOR_JSON_{nonce} ---\n\n"));

    content.push_str("Confidence breakdown of prior extraction:\n");
    content.push_str(&format!(
        "  - Text coverage: {:.2}%\n",
        confidence.text_coverage * 100.0
    ));
    if let Some(ocr) = confidence.ocr_aggregate {
        content.push_str(&format!("  - OCR confidence: {:.2}%\n", ocr * 100.0));
    }
    content.push_str(&format!(
        "  - Schema compliance: {:?}\n",
        confidence.schema_compliance
    ));
    content.push_str(&format!(
        "  - Combined score: {:.2}%\n",
        confidence.combined * 100.0
    ));

    content.push_str(
        "\nPlease review the source page images directly and produce a corrected/completed \
         JSON output matching the schema. Focus on details that the text-only pass may have missed.",
    );

    BuiltPrompt {
        system,
        user_text: Some(content),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Substitute `{{key}}` placeholders from `context`.
///
/// - Known keys are replaced with their string value.
/// - Unknown keys are left as `{{key}}` (no silent drops).
/// - Unterminated `{{` is emitted verbatim and scanning stops.
fn substitute_vars(template: &str, context: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                let key = after_open[..close].trim();
                match context.get(key) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push_str("{{");
                        out.push_str(key);
                        out.push_str("}}");
                    }
                }
                rest = &after_open[close + 2..];
            }
            None => {
                // Unterminated `{{` — emit verbatim and stop scanning.
                out.push_str("{{");
                rest = after_open;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Truncate `s` to at most `max_chars` Unicode scalar values, without
/// splitting a multi-byte character.
fn truncate_to_char_boundary(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    // Walk char boundaries to find the byte offset for `max_chars` chars.
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristics::confidence::SchemaCompliance;
    use crate::heuristics::{score_confidence, ConfidenceSignals, ConfidenceWeights};
    use crate::presets::types::{CallMode, MergeMode, PresetCategory};

    const DEFAULT_MAX_EXCERPT: usize = 200_000;

    fn stub_preset() -> Preset {
        Preset {
            id: "test".to_string(),
            version: "v1".to_string(),
            schema_name: "test_schema".to_string(),
            description: "test".to_string(),
            category: PresetCategory::Other,
            tags: vec![],
            schema: serde_json::json!({"type": "object"}),
            system_prompt: "You are a helpful extractor.".to_string(),
            context_template: None,
            merge_mode: MergeMode::ObjectMerge,
            preferred_call_mode: CallMode::TextOnly,
            emit_citations: false,
            sample: None,
            fingerprint: "test-fingerprint".to_string(),
        }
    }

    fn empty_context() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    // -----------------------------------------------------------------------
    // Ported cloud tests
    // -----------------------------------------------------------------------

    #[test]
    fn text_only_includes_excerpt() {
        let preset = stub_preset();
        let excerpt = "Sample extracted text";
        let prompt = build(
            &preset,
            &empty_context(),
            excerpt,
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert_eq!(prompt.system, "You are a helpful extractor.");
        assert!(prompt.user_text.is_some());
        assert!(prompt.user_text.unwrap().contains("Sample extracted text"));
    }

    #[test]
    fn vision_only_has_no_user_text() {
        let preset = stub_preset();
        let prompt = build(
            &preset,
            &empty_context(),
            "Sample extracted text",
            StructuredCallMode::VisionOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert_eq!(prompt.system, "You are a helpful extractor.");
        assert!(prompt.user_text.is_none());
    }

    #[test]
    fn context_substitution_all_vars_present() {
        let mut preset = stub_preset();
        preset.system_prompt = "Extract {{entity_type}} from the {{document_kind}}.".to_string();
        preset.context_template = Some("Context: {{entity_type}}".to_string());

        let mut context = BTreeMap::new();
        context.insert("entity_type".to_string(), "invoice".to_string());
        context.insert("document_kind".to_string(), "receipt".to_string());

        let prompt = build(
            &preset,
            &context,
            "text",
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(prompt.system.contains("Extract invoice from the receipt."));
        assert!(prompt.user_text.unwrap().contains("Context: invoice"));
    }

    #[test]
    fn context_substitution_missing_var_leaves_placeholder() {
        let mut preset = stub_preset();
        preset.system_prompt = "Extract {{entity_type}} from {{missing_var}}.".to_string();

        let mut context = BTreeMap::new();
        context.insert("entity_type".to_string(), "invoice".to_string());

        let prompt = build(
            &preset,
            &context,
            "text",
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(
            prompt
                .system
                .contains("Extract invoice from {{missing_var}}.")
        );
    }

    #[test]
    fn citation_instruction_appended_when_emit_citations_true() {
        let mut preset = stub_preset();
        preset.emit_citations = true;

        let prompt = build(
            &preset,
            &empty_context(),
            "text",
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(prompt.system.contains(CITATION_INSTRUCTION));
    }

    #[test]
    fn citation_instruction_not_appended_when_emit_citations_false() {
        let preset = stub_preset();
        assert!(!preset.emit_citations);

        let prompt = build(
            &preset,
            &empty_context(),
            "text",
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(!prompt.system.contains("bbox"));
    }

    #[test]
    fn excerpt_truncated_at_max_excerpt_chars() {
        let preset = stub_preset();
        let long_excerpt = "a".repeat(300_000);
        let max_chars = 200_000;

        let prompt = build(
            &preset,
            &empty_context(),
            &long_excerpt,
            StructuredCallMode::TextOnly,
            max_chars,
        );

        let user_text = prompt.user_text.unwrap();
        assert!(user_text.len() <= max_chars);
        assert!(user_text.contains(&"a".repeat(1_000)));
    }

    #[test]
    fn excerpt_not_truncated_when_within_limit() {
        let preset = stub_preset();
        let excerpt = "short text";

        let prompt = build(
            &preset,
            &empty_context(),
            excerpt,
            StructuredCallMode::TextOnly,
            200_000,
        );

        assert_eq!(prompt.user_text.unwrap(), excerpt);
    }

    #[test]
    fn context_template_merged_with_excerpt() {
        let mut preset = stub_preset();
        preset.context_template = Some("Document type: invoice".to_string());

        let excerpt = "Invoice number: 12345";
        let prompt = build(
            &preset,
            &empty_context(),
            excerpt,
            StructuredCallMode::TextOnly,
            DEFAULT_MAX_EXCERPT,
        );

        let user_text = prompt.user_text.unwrap();
        assert!(user_text.contains("Document type: invoice"));
        assert!(user_text.contains("Invoice number: 12345"));
        assert!(user_text.contains("---"));
    }

    #[test]
    fn text_plus_vision_includes_excerpt() {
        let preset = stub_preset();
        let excerpt = "Sample text with images";

        let prompt = build(
            &preset,
            &empty_context(),
            excerpt,
            StructuredCallMode::TextPlusVision,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(prompt.user_text.is_some());
        assert!(prompt.user_text.unwrap().contains("Sample text with images"));
    }

    #[test]
    fn skip_mode_returns_none() {
        let preset = stub_preset();
        let prompt = build(
            &preset,
            &empty_context(),
            "text",
            StructuredCallMode::Skip,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(prompt.user_text.is_none());
        assert_eq!(prompt.system, "You are a helpful extractor.");
    }

    // -----------------------------------------------------------------------
    // build_vision_fallback tests
    // -----------------------------------------------------------------------

    fn stub_confidence(combined: f32) -> ExtractionConfidence {
        let signals = ConfidenceSignals {
            text_coverage: combined,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::PartialValid,
        };
        score_confidence(signals, ConfidenceWeights::default())
    }

    #[test]
    fn build_vision_fallback_embeds_prior_json() {
        let preset = stub_preset();
        let prior = serde_json::json!({"invoice_number": "INV-001"});
        let confidence = stub_confidence(0.4);

        let prompt = build_vision_fallback(
            &preset,
            &empty_context(),
            "some extracted text",
            &prior,
            &confidence,
            DEFAULT_MAX_EXCERPT,
        );

        let user_text = prompt.user_text.unwrap();
        assert!(user_text.contains("INV-001"), "prior JSON value must appear in user_text");
        assert!(
            user_text.contains("Prior text-only extraction"),
            "low-confidence note must appear"
        );
    }

    #[test]
    fn build_vision_fallback_references_combined_score() {
        let preset = stub_preset();
        let prior = serde_json::json!({});
        let confidence = stub_confidence(0.35);

        let prompt = build_vision_fallback(
            &preset,
            &empty_context(),
            "",
            &prior,
            &confidence,
            DEFAULT_MAX_EXCERPT,
        );

        let user_text = prompt.user_text.unwrap();
        // The combined score is rendered as a percentage in the breakdown.
        assert!(
            user_text.contains("Combined score"),
            "combined score must appear in user_text"
        );
    }

    #[test]
    fn build_vision_fallback_instructs_correction_from_images() {
        let preset = stub_preset();
        let prior = serde_json::json!({});
        let confidence = stub_confidence(0.3);

        let prompt = build_vision_fallback(
            &preset,
            &empty_context(),
            "",
            &prior,
            &confidence,
            DEFAULT_MAX_EXCERPT,
        );

        let user_text = prompt.user_text.unwrap();
        assert!(
            user_text.contains("page images"),
            "fallback must instruct model to use page images"
        );
    }

    #[test]
    fn build_vision_fallback_truncates_excerpt() {
        let preset = stub_preset();
        let long_excerpt = "x".repeat(50_000);
        let max_chars = 1_000;
        let prior = serde_json::json!({});
        let confidence = stub_confidence(0.2);

        let prompt = build_vision_fallback(
            &preset,
            &empty_context(),
            &long_excerpt,
            &prior,
            &confidence,
            max_chars,
        );

        let user_text = prompt.user_text.unwrap();
        // The fenced excerpt must not contain more than max_chars 'x' characters.
        let x_run: String = "x".repeat(max_chars + 1);
        assert!(
            !user_text.contains(&x_run),
            "excerpt must be truncated to max_excerpt_chars"
        );
    }

    #[test]
    fn build_vision_fallback_citation_appended_when_emit_citations() {
        let mut preset = stub_preset();
        preset.emit_citations = true;
        let prior = serde_json::json!({});
        let confidence = stub_confidence(0.4);

        let prompt = build_vision_fallback(
            &preset,
            &empty_context(),
            "text",
            &prior,
            &confidence,
            DEFAULT_MAX_EXCERPT,
        );

        assert!(
            prompt.system.contains(CITATION_INSTRUCTION),
            "citation instruction must appear in system prompt"
        );
    }

    // -----------------------------------------------------------------------
    // substitute_vars unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn substitute_vars_replaces_known_key() {
        let mut ctx = BTreeMap::new();
        ctx.insert("name".to_string(), "Alice".to_string());

        let result = substitute_vars("Hello {{name}}!", &ctx);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn substitute_vars_leaves_unknown_key_intact() {
        let ctx = BTreeMap::new();
        let result = substitute_vars("Hello {{unknown}}!", &ctx);
        assert_eq!(result, "Hello {{unknown}}!");
    }

    #[test]
    fn substitute_vars_unterminated_brace_emitted_verbatim() {
        let ctx = BTreeMap::new();
        let result = substitute_vars("Hello {{never_closed", &ctx);
        assert!(result.contains("{{never_closed"));
    }

    #[test]
    fn substitute_vars_empty_context_leaves_all_placeholders() {
        let ctx = BTreeMap::new();
        let template = "{{a}} and {{b}}";
        let result = substitute_vars(template, &ctx);
        assert_eq!(result, "{{a}} and {{b}}");
    }

    #[test]
    fn substitute_vars_multiple_keys() {
        let mut ctx = BTreeMap::new();
        ctx.insert("vendor".to_string(), "Acme".to_string());
        ctx.insert("locale".to_string(), "en-US".to_string());

        let result = substitute_vars("Vendor: {{vendor}}, Locale: {{locale}}", &ctx);
        assert_eq!(result, "Vendor: Acme, Locale: en-US");
    }

    // -----------------------------------------------------------------------
    // truncate_to_char_boundary unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_short_string_unchanged() {
        let s = "hello";
        assert_eq!(truncate_to_char_boundary(s, 100), s);
    }

    #[test]
    fn truncate_at_exact_boundary() {
        let s = "abcde";
        assert_eq!(truncate_to_char_boundary(s, 5), "abcde");
    }

    #[test]
    fn truncate_cuts_at_char_boundary() {
        let s = "abcde";
        assert_eq!(truncate_to_char_boundary(s, 3), "abc");
    }

    #[test]
    fn truncate_multibyte_utf8_does_not_split_char() {
        // "café" is 4 chars but 5 bytes (é = 2 bytes).
        let s = "café";
        let truncated = truncate_to_char_boundary(s, 3);
        // "caf" — 3 valid ASCII chars, 3 bytes.
        assert_eq!(truncated, "caf");
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }
}
