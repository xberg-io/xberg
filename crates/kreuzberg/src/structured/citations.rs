//! Fuse OCR bounding boxes onto extracted fields to produce citation envelopes.
//!
//! Walks the merged vision-LLM JSON recursively and wraps each leaf value in a
//! [`super::CitedField`] envelope that records page, bounding box, confidence, and
//! provenance source.  Fuzzy text matching (shared-character ratio > 0.8) is used to
//! correlate field values against [`crate::types::OcrElement`] text.

use serde_json::Value;

use super::{CitationEnvelope, CitationSource, CitedField};

// ── Public entry point ────────────────────────────────────────────────────────

/// Fuse merged vision-LLM output with typed OCR elements to produce citation envelopes.
///
/// When `emit_citations` is `false` the function is a cheap identity: both envelope fields
/// are cloned from `merged` unchanged.
///
/// When `emit_citations` is `true` every leaf value in `merged` is matched against
/// `ocr_elements` via a shared-character ratio (threshold > 0.8).  A match attaches the
/// element's page number, bounding box, and a confidence of 0.95 with
/// [`CitationSource::Fused`].  Unmatched leaves receive [`CitationSource::None`].
///
/// The returned [`CitationEnvelope`] carries:
/// - `structured_output` — the citation-annotated tree (leaves are [`CitedField`] objects).
/// - `flat` — value-only projection (strips the citation envelope, keeping just `.value`).
pub fn fuse(
    merged: Value,
    ocr_elements: &[crate::types::OcrElement],
    emit_citations: bool,
) -> CitationEnvelope {
    if !emit_citations {
        return CitationEnvelope {
            structured_output: merged.clone(),
            flat: merged,
        };
    }

    let ocr_values: Vec<Value> = ocr_elements.iter().map(ocr_element_to_value).collect();
    let structured_output = envelope_with_citations(&merged, &ocr_values);
    let flat = flatten_cited(&structured_output);

    CitationEnvelope { structured_output, flat }
}

// ── OcrElement adapter ────────────────────────────────────────────────────────

/// Convert a typed [`crate::types::OcrElement`] into the flat JSON shape expected by the
/// fuzzy-match logic: `{ "text": …, "page_number": …, "bbox": [x, y, w, h] }`.
///
/// Geometry is normalised to an axis-aligned bounding rect regardless of variant:
/// - `Rectangle { left, top, width, height }` → `[left, top, width, height]` as `f64`.
/// - `Quadrilateral { points }` → minimal enclosing AABB: `[min_x, min_y, w, h]` as `f64`.
fn ocr_element_to_value(el: &crate::types::OcrElement) -> Value {
    use crate::types::OcrBoundingGeometry;

    let (x, y, w, h): (f64, f64, f64, f64) = match &el.geometry {
        OcrBoundingGeometry::Rectangle { left, top, width, height } => {
            (*left as f64, *top as f64, *width as f64, *height as f64)
        }
        OcrBoundingGeometry::Quadrilateral { points } => {
            let min_x = points.iter().map(|(px, _)| *px).min().unwrap_or(0);
            let max_x = points.iter().map(|(px, _)| *px).max().unwrap_or(0);
            let min_y = points.iter().map(|(_, py)| *py).min().unwrap_or(0);
            let max_y = points.iter().map(|(_, py)| *py).max().unwrap_or(0);
            (
                min_x as f64,
                min_y as f64,
                max_x.saturating_sub(min_x) as f64,
                max_y.saturating_sub(min_y) as f64,
            )
        }
    };

    serde_json::json!({
        "text": el.text,
        "page_number": el.page_number,
        "bbox": [x, y, w, h]
    })
}

// ── Citation tree traversal ───────────────────────────────────────────────────

/// Walk `value` and wrap every leaf in a citation envelope.
fn envelope_with_citations(value: &Value, ocr_values: &[Value]) -> Value {
    match value {
        Value::Object(obj) => {
            let mut result = serde_json::Map::new();
            for (k, v) in obj {
                result.insert(k.clone(), envelope_with_citations(v, ocr_values));
            }
            Value::Object(result)
        }
        Value::Array(arr) => {
            let items: Vec<Value> = arr
                .iter()
                .map(|v| envelope_with_citations(v, ocr_values))
                .collect();
            Value::Array(items)
        }
        leaf => {
            if is_citation_envelope(leaf) {
                leaf.clone()
            } else {
                let cited = try_fuse_with_ocr(leaf, ocr_values);
                serde_json::to_value(&cited).unwrap_or_else(|_| leaf.clone())
            }
        }
    }
}

/// Return `true` when `v` already looks like a citation envelope
/// (`{ "value": …, … }` with at most 5 keys).
///
/// This is checked only for scalar leaves (non-Object, non-Array values) in
/// [`envelope_with_citations`]; Objects are always recursed into.
fn is_citation_envelope(v: &Value) -> bool {
    if let Value::Object(obj) = v {
        obj.contains_key("value") && !obj.is_empty() && obj.len() <= 5
    } else {
        false
    }
}

// ── Fuzzy matching (ported verbatim from cloud) ───────────────────────────────

/// Try to fuse a scalar leaf value with an OCR element via fuzzy text matching.
///
/// The match threshold is > 0.8 on a shared-character ratio (see [`text_similarity`]).
fn try_fuse_with_ocr(value: &Value, ocr_values: &[Value]) -> CitedField {
    let value_str = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let value_lower = value_str.to_lowercase();
    let value_trimmed = value_lower.trim();

    for ocr in ocr_values {
        if let Some(text) = ocr.get("text").and_then(|t| t.as_str()) {
            let ocr_lower = text.to_lowercase();
            let ocr_text = ocr_lower.trim();

            if text_similarity(value_trimmed, ocr_text) > 0.8 {
                let page = ocr
                    .get("page_number")
                    .and_then(|p| p.as_u64())
                    .map(|p| p as u32);
                let bbox = extract_bbox(ocr);
                return CitedField {
                    value: value.clone(),
                    page,
                    bbox,
                    confidence: Some(0.95),
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

/// Simple text similarity: shared prefix character count / max length.
///
/// Ported verbatim from the cloud implementation.  Counts how many characters
/// at corresponding positions are equal, then divides by the length of the
/// longer string.
fn text_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let max_len = a.len().max(b.len());
    let matching = a.chars().zip(b.chars()).filter(|(ca, cb)| ca == cb).count();
    matching as f64 / max_len as f64
}

/// Extract a `[x, y, w, h]` bounding box from an OCR value produced by
/// [`ocr_element_to_value`].
fn extract_bbox(ocr: &Value) -> Option<[f64; 4]> {
    ocr.get("bbox").and_then(|b| {
        if let Value::Array(arr) = b {
            if arr.len() >= 4 {
                let coords: Option<Vec<f64>> =
                    arr.iter().take(4).map(|v| v.as_f64()).collect();
                coords.map(|c| [c[0], c[1], c[2], c[3]])
            } else {
                None
            }
        } else {
            None
        }
    })
}

// ── Flat projection ───────────────────────────────────────────────────────────

/// Recursively strip citation envelopes, returning a value-only tree.
fn flatten_cited(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            if is_citation_envelope(value) && let Some(inner) = obj.get("value") {
                return flatten_cited(inner);
            }
            let mut result = serde_json::Map::new();
            for (k, v) in obj {
                result.insert(k.clone(), flatten_cited(v));
            }
            Value::Object(result)
        }
        Value::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(flatten_cited).collect();
            Value::Array(items)
        }
        leaf => leaf.clone(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    fn make_rect_element(text: &str, page: u32, left: u32, top: u32, w: u32, h: u32) -> OcrElement {
        OcrElement {
            text: text.to_owned(),
            geometry: OcrBoundingGeometry::Rectangle { left, top, width: w, height: h },
            confidence: OcrConfidence { recognition: 0.95, detection: None },
            page_number: page,
            ..Default::default()
        }
    }

    // ── emit_citations=false ──────────────────────────────────────────────────

    #[test]
    fn emit_citations_false_passes_through_both_fields() {
        let merged = serde_json::json!({"name": "Alice", "age": 30});
        let envelope = fuse(merged.clone(), &[], false);

        assert_eq!(envelope.structured_output, merged);
        assert_eq!(envelope.flat, merged);
    }

    #[test]
    fn emit_citations_false_with_ocr_elements_ignores_them() {
        let merged = serde_json::json!({"field": "Alice"});
        let el = make_rect_element("Alice", 1, 10, 20, 100, 30);
        let envelope = fuse(merged.clone(), &[el], false);

        assert_eq!(envelope.structured_output, merged);
        assert_eq!(envelope.flat, merged);
    }

    // ── exact OCR match ───────────────────────────────────────────────────────

    #[test]
    fn exact_match_attaches_page_bbox_and_fused_source() {
        let merged = serde_json::json!({"field": "Alice"});
        let el = make_rect_element("Alice", 3, 10, 20, 100, 30);
        let envelope = fuse(merged, &[el], true);

        let cited: CitedField =
            serde_json::from_value(envelope.structured_output["field"].clone())
                .expect("field should be a CitedField");

        assert_eq!(cited.source, CitationSource::Fused);
        assert_eq!(cited.page, Some(3));
        assert_eq!(cited.bbox, Some([10.0, 20.0, 100.0, 30.0]));
        assert_eq!(cited.confidence, Some(0.95));
        assert_eq!(cited.value, serde_json::json!("Alice"));
    }

    #[test]
    fn case_insensitive_match_succeeds() {
        let merged = serde_json::json!({"field": "alice"});
        let el = make_rect_element("ALICE", 1, 0, 0, 50, 10);
        let envelope = fuse(merged, &[el], true);

        let cited: CitedField =
            serde_json::from_value(envelope.structured_output["field"].clone())
                .expect("field should be a CitedField");
        assert_eq!(cited.source, CitationSource::Fused);
    }

    // ── no match → CitationSource::None ──────────────────────────────────────

    #[test]
    fn unmatched_value_gets_source_none() {
        let merged = serde_json::json!({"field": "completely_unknown_value"});
        let el = make_rect_element("Alice", 1, 0, 0, 50, 10);
        let envelope = fuse(merged, &[el], true);

        let cited: CitedField =
            serde_json::from_value(envelope.structured_output["field"].clone())
                .expect("field should be a CitedField");

        assert_eq!(cited.source, CitationSource::None);
        assert!(cited.page.is_none());
        assert!(cited.bbox.is_none());
        assert!(cited.confidence.is_none());
    }

    #[test]
    fn empty_ocr_elements_gives_source_none() {
        let merged = serde_json::json!({"field": "some value"});
        let envelope = fuse(merged, &[], true);

        let cited: CitedField =
            serde_json::from_value(envelope.structured_output["field"].clone())
                .expect("field should be a CitedField");
        assert_eq!(cited.source, CitationSource::None);
    }

    // ── flat projection ───────────────────────────────────────────────────────

    #[test]
    fn flat_field_is_bare_value_not_envelope() {
        let merged = serde_json::json!({"name": "Alice", "age": 30});
        let el = make_rect_element("Alice", 1, 0, 0, 50, 10);
        let envelope = fuse(merged, &[el], true);

        // flat must not contain citation keys
        let flat_name = &envelope.flat["name"];
        assert_eq!(flat_name.as_str(), Some("Alice"));
        let flat_age = &envelope.flat["age"];
        assert_eq!(flat_age.as_u64(), Some(30));
    }

    // ── nested objects ────────────────────────────────────────────────────────

    #[test]
    fn nested_objects_are_fused_recursively() {
        let merged = serde_json::json!({
            "person": {
                "name": "Bob",
                "contact": { "email": "bob@example.com" }
            }
        });
        let envelope = fuse(merged, &[], true);

        let name_field = &envelope.structured_output["person"]["name"];
        let cited: CitedField =
            serde_json::from_value(name_field.clone()).expect("should be CitedField");
        assert_eq!(cited.value, serde_json::json!("Bob"));
        assert_eq!(cited.source, CitationSource::None);
    }

    // ── quadrilateral geometry adapter ───────────────────────────────────────

    #[test]
    fn quadrilateral_geometry_converts_to_aabb() {
        let el = OcrElement {
            text: "quad".to_owned(),
            geometry: OcrBoundingGeometry::Quadrilateral {
                points: [(10, 22), (108, 20), (110, 72), (12, 74)],
            },
            confidence: OcrConfidence { recognition: 0.9, detection: None },
            page_number: 2,
            ..Default::default()
        };
        let v = ocr_element_to_value(&el);
        let bbox = v["bbox"].as_array().expect("bbox must be array");
        assert_eq!(bbox[0].as_f64(), Some(10.0));  // min_x
        assert_eq!(bbox[1].as_f64(), Some(20.0));  // min_y
        assert_eq!(bbox[2].as_f64(), Some(100.0)); // width = 110 - 10
        assert_eq!(bbox[3].as_f64(), Some(54.0));  // height = 74 - 20
    }

    // ── pre-existing envelope: object traversal ──────────────────────────────

    #[test]
    fn object_with_citation_shape_is_traversed_and_leaves_are_wrapped() {
        // An object that looks like a CitedField is still an Object to the
        // envelope_with_citations walker, so each scalar leaf inside it gets
        // wrapped.  The "name" key's value is an Object (not a bare scalar),
        // so the walker recurses into it rather than treating it as a leaf.
        let merged = serde_json::json!({
            "name": {
                "value": "Alice",
                "page": 1,
                "bbox": [10.0, 20.0, 100.0, 30.0],
                "source": "llm"
            }
        });
        let envelope = fuse(merged.clone(), &[], true);
        // The outer "name" key is preserved.
        assert!(envelope.structured_output.get("name").is_some());
        // The inner "value" leaf "Alice" should itself become a CitedField.
        let inner_value = &envelope.structured_output["name"]["value"];
        let cited: CitedField =
            serde_json::from_value(inner_value.clone()).expect("leaf should be CitedField");
        assert_eq!(cited.value, serde_json::json!("Alice"));
        assert_eq!(cited.source, CitationSource::None);
    }

    // ── arrays ────────────────────────────────────────────────────────────────

    #[test]
    fn array_elements_are_each_fused() {
        let merged = serde_json::json!({"tags": ["rust", "ocr"]});
        let el = make_rect_element("rust", 1, 0, 0, 40, 10);
        let envelope = fuse(merged, &[el], true);

        let tags = envelope.structured_output["tags"].as_array().expect("array");
        let first: CitedField =
            serde_json::from_value(tags[0].clone()).expect("CitedField");
        let second: CitedField =
            serde_json::from_value(tags[1].clone()).expect("CitedField");

        assert_eq!(first.source, CitationSource::Fused);
        assert_eq!(second.source, CitationSource::None);
    }

    // ── text_similarity edge cases ────────────────────────────────────────────

    #[test]
    fn text_similarity_both_empty_is_one() {
        assert_eq!(text_similarity("", ""), 1.0);
    }

    #[test]
    fn text_similarity_one_empty_is_zero() {
        assert_eq!(text_similarity("hello", ""), 0.0);
        assert_eq!(text_similarity("", "hello"), 0.0);
    }

    #[test]
    fn text_similarity_identical_strings_is_one() {
        assert_eq!(text_similarity("hello", "hello"), 1.0);
    }

    #[test]
    fn text_similarity_below_threshold() {
        // "abc" vs "xyz" — no positional matches
        assert!(text_similarity("abc", "xyz") <= 0.8);
    }
}
