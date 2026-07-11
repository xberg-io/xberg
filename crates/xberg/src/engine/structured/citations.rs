//! Citation fusion: enrich LLM output with extracted OCR/metadata context.
//!
//! Takes the merged vision-LLM JSON and optionally fuses each value with
//! extracted source data (page numbers, bounding boxes, confidence scores).
//! Produces both a cited envelope and a flattened values-only view.

use serde::{Deserialize, Serialize};

/// Provenance of a cited field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationSource {
    /// Value came from the LLM only.
    Llm,
    /// Value came from extracted source data only.
    Extracted,
    /// Value fused from both LLM output and extracted source data.
    Fused,
    /// No provenance could be attributed.
    None,
}

/// A single field with its citation envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitedField {
    /// The field value.
    pub value: serde_json::Value,
    /// 1-based page number where the value appears, when known.
    pub page: Option<u32>,
    /// Bounding box `[x1, y1, x2, y2]`, when known.
    #[cfg_attr(alef, alef(skip))]
    pub bbox: Option<[f64; 4]>,
    /// Confidence in the citation, when known.
    pub confidence: Option<f64>,
    /// Provenance of the value.
    pub source: CitationSource,
}

/// Both views of the fused output: citation-wrapped and flattened.
pub struct CitationOutput {
    /// Citation-wrapped envelope (or the original `merged` when citations are off).
    pub structured_output: serde_json::Value,
    /// Values only (`CitedField.value` extracted recursively).
    pub structured_output_flat: serde_json::Value,
}

/// Fuse merged vision-LLM output with extracted OCR elements and metadata.
///
/// # Arguments
/// * `merged` - Merged JSON output from the schema module
/// * `ocr_elements` - Array of extracted OCR elements (as `serde_json::Value`)
/// * `element_metadata` - Array of extracted element metadata (as `serde_json::Value`)
/// * `emit_citations` - Whether to produce citation envelopes
/// * `match_threshold` - Minimum text-similarity score (`0.0`–`1.0`) for an OCR
///   element to be accepted as the source of a field value
/// * `fused_confidence` - Confidence recorded on a successfully fused field
///
/// # Returns
/// - `structured_output`: Citation-wrapped envelope if `emit_citations=true`, else original `merged`
/// - `structured_output_flat`: Values only (`CitedField.value` extracted recursively)
///
/// # Implementation notes
/// `ocr_elements` and `element_metadata` are taken as `&[serde_json::Value]` because
/// their concrete types are opaque to this module. The citation logic does fuzzy text
/// matching on the stringified field values against OCR text, so the exact schema of the
/// source data is not needed. `match_threshold` and `fused_confidence` are caller
/// parameters: the mechanism imposes no default, matching the rest of this module.
pub fn fuse(
    merged: serde_json::Value,
    ocr_elements: &[serde_json::Value],
    element_metadata: &[serde_json::Value],
    emit_citations: bool,
    match_threshold: f64,
    fused_confidence: f64,
) -> CitationOutput {
    if !emit_citations {
        return CitationOutput {
            structured_output: merged.clone(),
            structured_output_flat: merged,
        };
    }

    let structured_output = envelope_with_citations(
        &merged,
        ocr_elements,
        element_metadata,
        match_threshold,
        fused_confidence,
    );
    let structured_output_flat = flatten_cited(&structured_output);

    CitationOutput {
        structured_output,
        structured_output_flat,
    }
}

/// Walk the merged JSON and wrap leaf values in CitedField envelopes.
fn envelope_with_citations(
    value: &serde_json::Value,
    ocr_elements: &[serde_json::Value],
    element_metadata: &[serde_json::Value],
    match_threshold: f64,
    fused_confidence: f64,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            let mut result = serde_json::Map::new();
            for (k, v) in obj {
                result.insert(
                    k.clone(),
                    envelope_with_citations(v, ocr_elements, element_metadata, match_threshold, fused_confidence),
                );
            }
            serde_json::Value::Object(result)
        }
        serde_json::Value::Array(arr) => {
            let result: Vec<_> = arr
                .iter()
                .map(|v| envelope_with_citations(v, ocr_elements, element_metadata, match_threshold, fused_confidence))
                .collect();
            serde_json::Value::Array(result)
        }
        // Leaf value: check if it's already a citation envelope, or wrap it.
        leaf => {
            if is_citation_envelope(leaf) {
                leaf.clone()
            } else {
                let cited =
                    try_fuse_with_extracted(leaf, ocr_elements, element_metadata, match_threshold, fused_confidence);
                serde_json::to_value(&cited).unwrap_or(leaf.clone())
            }
        }
    }
}

/// Check if a value looks like a citation envelope: {value, page?, bbox?, confidence?, source?}
fn is_citation_envelope(v: &serde_json::Value) -> bool {
    if let serde_json::Value::Object(obj) = v {
        obj.contains_key("value") && !obj.is_empty() && obj.len() <= 5
    } else {
        false
    }
}

/// Attempt to match a scalar value against extracted OCR text.
/// Returns a CitedField with source set based on match quality.
fn try_fuse_with_extracted(
    value: &serde_json::Value,
    ocr_elements: &[serde_json::Value],
    _element_metadata: &[serde_json::Value],
    match_threshold: f64,
    fused_confidence: f64,
) -> CitedField {
    let value_str = match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let value_lower = value_str.to_lowercase();
    let value_trimmed = value_lower.trim();

    for ocr in ocr_elements {
        if let Some(text) = ocr.get("text").and_then(|t| t.as_str()) {
            let ocr_lower = text.to_lowercase();
            let ocr_text = ocr_lower.trim();

            if text_similarity(value_trimmed, ocr_text) > match_threshold {
                let page = ocr.get("page_number").and_then(|p| p.as_u64()).map(|p| p as u32);
                let bbox = extract_bbox(ocr);
                return CitedField {
                    value: value.clone(),
                    page,
                    bbox,
                    confidence: Some(fused_confidence),
                    source: CitationSource::Fused,
                };
            }
        }
    }

    CitedField {
        value: value.clone(),
        page: None,
        bbox: None,
        confidence: None,
        source: CitationSource::None,
    }
}

/// Simple text similarity: shared character count / max length.
/// Used for fuzzy matching of LLM output against OCR text.
fn text_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let max_len = a.chars().count().max(b.chars().count());
    let matching = a.chars().zip(b.chars()).filter(|(ca, cb)| ca == cb).count();
    matching as f64 / max_len as f64
}

/// Extract bounding box from an OCR element (assumes [x, y, width, height] or similar).
fn extract_bbox(ocr: &serde_json::Value) -> Option<[f64; 4]> {
    ocr.get("bbox").and_then(|b| {
        if let serde_json::Value::Array(arr) = b {
            if arr.len() >= 4 {
                let coords: Option<Vec<f64>> = arr.iter().take(4).map(|v| v.as_f64()).collect();
                coords.map(|c| [c[0], c[1], c[2], c[3]])
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Walk the citation envelope and extract just the .value fields.
fn flatten_cited(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            if is_citation_envelope(value) {
                // This is a CitedField; extract the .value and recurse
                if let Some(inner_value) = obj.get("value") {
                    return flatten_cited(inner_value);
                }
            }
            // Regular object; recurse on each field
            let mut result = serde_json::Map::new();
            for (k, v) in obj {
                result.insert(k.clone(), flatten_cited(v));
            }
            serde_json::Value::Object(result)
        }
        serde_json::Value::Array(arr) => {
            let result: Vec<_> = arr.iter().map(flatten_cited).collect();
            serde_json::Value::Array(result)
        }
        leaf => leaf.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_citations_false_returns_passthrough() {
        let merged = serde_json::json!({"name": "Alice", "age": 30});
        let result = fuse(merged.clone(), &[], &[], false, 0.8, 0.95);

        assert_eq!(result.structured_output, merged);
        assert_eq!(result.structured_output_flat, merged);
    }

    #[test]
    fn llm_envelope_passes_through() {
        let merged = serde_json::json!({
            "name": {
                "value": "Alice",
                "page": 1,
                "bbox": [10.0, 20.0, 100.0, 30.0],
                "source": "llm"
            }
        });

        let result = fuse(merged.clone(), &[], &[], true, 0.8, 0.95);
        assert!(result.structured_output.get("name").is_some());
    }

    #[test]
    fn scalar_value_without_match_sets_source_none() {
        let merged = serde_json::json!({"field": "unknown_value"});
        let result = fuse(merged, &[], &[], true, 0.8, 0.95);

        let field = result.structured_output.get("field").unwrap();
        if let Ok(cited) = serde_json::from_value::<CitedField>(field.clone()) {
            assert_eq!(cited.source, CitationSource::None);
            assert_eq!(cited.value, serde_json::json!("unknown_value"));
        }
    }

    #[test]
    fn scalar_with_matching_ocr_fuses() {
        let merged = serde_json::json!({"field": "Alice"});
        let ocr = serde_json::json!({
            "text": "Alice",
            "page_number": 1,
            "bbox": [10.0, 20.0, 100.0, 30.0]
        });

        let result = fuse(merged, &[ocr], &[], true, 0.8, 0.95);
        let field = result.structured_output.get("field").unwrap();

        if let Ok(cited) = serde_json::from_value::<CitedField>(field.clone()) {
            assert_eq!(cited.source, CitationSource::Fused);
            assert_eq!(cited.page, Some(1));
            assert!(cited.bbox.is_some());
        }
    }

    #[test]
    fn flatten_cited_extracts_values_only() {
        let cited = serde_json::json!({
            "name": {
                "value": "Alice",
                "page": 1,
                "source": "fused"
            },
            "age": {
                "value": 30,
                "source": "none"
            }
        });

        let flattened = flatten_cited(&cited);
        let name = flattened.get("name").unwrap();
        let age = flattened.get("age").unwrap();

        assert_eq!(name.as_str(), Some("Alice"));
        assert_eq!(age.as_u64(), Some(30));
    }

    #[test]
    fn text_similarity_identical_multibyte_strings_score_one() {
        // Regression: byte-length denominator made identical CJK strings score
        // chars/bytes (e.g. 2/6 = 0.33) instead of 1.0. Char-count denominator fixes it.
        assert_eq!(text_similarity("世界", "世界"), 1.0);
        assert_eq!(text_similarity("café", "café"), 1.0);
    }

    #[test]
    fn scalar_with_matching_non_ascii_ocr_fuses() {
        // Regression: before the char-count denominator fix, a non-ASCII value
        // could never clear the > 0.8 similarity gate, so source stayed `None`.
        let merged = serde_json::json!({"field": "世界"});
        let ocr = serde_json::json!({
            "text": "世界",
            "page_number": 2,
            "bbox": [11.0, 22.0, 110.0, 33.0]
        });

        let result = fuse(merged, &[ocr], &[], true, 0.8, 0.95);
        let field = result.structured_output.get("field").unwrap();

        let cited: CitedField = serde_json::from_value(field.clone()).expect("field should deserialize as CitedField");
        assert_eq!(cited.source, CitationSource::Fused);
        assert_eq!(cited.page, Some(2));
        assert_eq!(cited.bbox, Some([11.0, 22.0, 110.0, 33.0]));
        assert_eq!(cited.confidence, Some(0.95));
    }

    #[test]
    fn nested_objects_are_handled_recursively() {
        let merged = serde_json::json!({
            "person": {
                "name": "Bob",
                "contact": {
                    "email": "bob@example.com"
                }
            }
        });

        let result = fuse(merged, &[], &[], true, 0.8, 0.95);
        assert!(result.structured_output.get("person").is_some());
        assert!(
            result
                .structured_output
                .get("person")
                .and_then(|p| p.get("contact"))
                .is_some()
        );
    }
}
