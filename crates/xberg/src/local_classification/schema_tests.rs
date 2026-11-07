use super::schema::{TensorSpec, validate_schema};
use ort::value::TensorElementType::{Float32, Int64};

fn inputs() -> Vec<TensorSpec> {
    ["input_ids", "attention_mask", "token_type_ids"]
        .map(|name| TensorSpec {
            name: name.to_owned(),
            element_type: Int64,
            dimensions: vec![1, -1],
        })
        .into()
}

fn outputs() -> Vec<TensorSpec> {
    vec![TensorSpec {
        name: "logits".to_owned(),
        element_type: Float32,
        dimensions: vec![1, 3],
    }]
}

#[test]
fn a_dynamic_sequence_three_way_head_is_supported_with_optional_type_ids() {
    validate_schema(&inputs(), &outputs(), "logits").unwrap();
    validate_schema(&inputs()[..2], &outputs(), "logits").unwrap();
}

#[test]
fn fixed_taxonomy_output_dimensions_are_rejected() {
    let mut output = outputs();
    output[0].dimensions = vec![1, 4];
    assert!(validate_schema(&inputs(), &output, "logits").is_err());
}

#[test]
fn input_names_and_required_attention_mask_are_checked() {
    assert!(validate_schema(&inputs()[..1], &outputs(), "logits").is_err());
    let mut input = inputs();
    input[1].name = "unexpected_mask".to_owned();
    assert!(validate_schema(&input, &outputs(), "logits").is_err());
}

#[test]
fn non_integer_or_non_rank_two_inputs_are_rejected() {
    let mut input = inputs();
    input[0].element_type = Float32;
    assert!(validate_schema(&input, &outputs(), "logits").is_err());
    input = inputs();
    input[0].dimensions = vec![-1];
    assert!(validate_schema(&input, &outputs(), "logits").is_err());
}

#[test]
fn fixed_sequence_lengths_and_multiple_batches_are_not_silently_padded_or_split() {
    let mut input = inputs();
    input[0].dimensions = vec![1, 128];
    assert!(validate_schema(&input, &outputs(), "logits").is_err());
    input[0].dimensions = vec![2, -1];
    assert!(validate_schema(&input, &outputs(), "logits").is_err());
}

#[test]
fn unselected_extra_or_non_float_logits_outputs_are_rejected() {
    assert!(validate_schema(&inputs(), &outputs(), "scores").is_err());
    let mut output = outputs();
    output[0].element_type = Int64;
    assert!(validate_schema(&inputs(), &output, "logits").is_err());
    output = outputs();
    output.push(TensorSpec {
        name: "hidden_state".to_owned(),
        element_type: Float32,
        dimensions: vec![1, -1],
    });
    assert!(validate_schema(&inputs(), &output, "logits").is_err());
}
