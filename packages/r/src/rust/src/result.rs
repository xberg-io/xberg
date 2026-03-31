//! ExtractionResult -> R named list conversion and serialization

use crate::helpers::json_to_robj;
use crate::error::to_r_error;
use extendr_api::prelude::*;
use kreuzberg::ExtractionResult;
use serde_json::Value;

/// Fields on ExtractionResult that are Option<Vec<_>> — should default to NULL in R.
const OPTIONAL_FIELDS: &[&str] = &[
    "annotations",
    "children",
    "chunks",
    "detected_languages",
    "djot_content",
    "document",
    "elements",
    "extracted_keywords",
    "images",
    "ocr_elements",
    "pages",
    "quality_score",
    "uris",
];

/// Fields on ExtractionResult that are Vec<_> — should default to empty array in R.
const VEC_FIELDS: &[&str] = &[
    "processing_warnings",
];

/// Ensure all expected fields exist in the serialized JSON object.
///
/// serde's `skip_serializing_if` omits None/empty fields, but R parity tests
/// expect every field to be present (NULL for None, empty list for empty Vec).
fn ensure_all_fields(value: &mut Value) {
    if let Value::Object(map) = value {
        for &field in OPTIONAL_FIELDS {
            map.entry(field.to_owned()).or_insert(Value::Null);
        }
        for &field in VEC_FIELDS {
            map.entry(field.to_owned()).or_insert_with(|| Value::Array(Vec::new()));
        }
    }
}

/// Convert an ExtractionResult to an R named list
pub fn extraction_result_to_list(result: ExtractionResult) -> extendr_api::Result<List> {
    // Serialize to JSON then convert to R objects
    let mut json_value = serde_json::to_value(&result).map_err(to_r_error)?;

    // Ensure all expected fields are present even when serde skips them
    ensure_all_fields(&mut json_value);

    let robj = json_to_robj(&json_value)?;

    // Convert to list and add class attribute
    let list = List::try_from(robj).map_err(to_r_error)?;
    let mut result_robj = list.into_robj();
    result_robj.set_class(&["kreuzberg_result", "list"]).map_err(to_r_error)?;
    List::try_from(result_robj).map_err(to_r_error)
}
