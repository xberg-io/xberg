use ort::session::Session;

use crate::Result;
use crate::session::validate_schema_names;

/// GLiNER2 tensor input names expected by schema-prompt ONNX exports.
pub const INPUT_NAMES_V2: [&str; 5] = [
    "input_ids",
    "attention_mask",
    "text_positions",
    "schema_positions",
    "span_idx",
];

/// GLiNER2 tensor output names expected by schema-prompt ONNX exports.
pub const OUTPUT_NAMES_V2: [&str; 1] = ["span_scores"];

pub(crate) const TENSOR_V2_INPUT_IDS: &str = "input_ids";
pub(crate) const TENSOR_V2_ATTENTION_MASK: &str = "attention_mask";
pub(crate) const TENSOR_V2_TEXT_POSITIONS: &str = "text_positions";
pub(crate) const TENSOR_V2_SCHEMA_POSITIONS: &str = "schema_positions";
pub(crate) const TENSOR_V2_SPAN_IDX: &str = "span_idx";
pub(crate) const TENSOR_V2_SPAN_SCORES: &str = "span_scores";

pub(crate) fn validate_session_schema_v2(session: &Session) -> Result<()> {
    let inputs = session
        .inputs()
        .iter()
        .map(|input| input.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("input", &INPUT_NAMES_V2, &inputs)?;

    let outputs = session
        .outputs()
        .iter()
        .map(|output| output.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("output", &OUTPUT_NAMES_V2, &outputs)
}
