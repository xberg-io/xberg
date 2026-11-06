use ort::value::TensorElementType;

pub(super) struct TensorSpec {
    pub(super) name: String,
    pub(super) element_type: TensorElementType,
    pub(super) dimensions: Vec<i64>,
}

pub(super) fn validate_schema(inputs: &[TensorSpec], outputs: &[TensorSpec], output_name: &str) -> crate::Result<()> {
    use super::policy::invalid;
    let names = inputs
        .iter()
        .map(|input| input.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if !(2..=3).contains(&inputs.len())
        || names.len() != inputs.len()
        || !names.contains("input_ids")
        || !names.contains("attention_mask")
        || names
            .iter()
            .any(|name| !matches!(*name, "input_ids" | "attention_mask" | "token_type_ids"))
    {
        return Err(invalid(
            "Local NLI graph requires input_ids, attention_mask and optional token_type_ids",
        ));
    }
    if inputs.iter().any(|input| {
        input.element_type != TensorElementType::Int64
            || input.dimensions.len() != 2
            || !matches!(input.dimensions[0], -1 | 1)
            || input.dimensions[1] != -1
    }) {
        return Err(invalid(
            "Local NLI inputs must be rank-two i64 tensors with batch one and dynamic sequence",
        ));
    }
    if outputs.len() != 1
        || outputs[0].name != output_name
        || outputs[0].element_type != TensorElementType::Float32
        || outputs[0].dimensions.len() != 2
        || !matches!(outputs[0].dimensions[0], -1 | 1)
        || outputs[0].dimensions[1] != 3
    {
        return Err(invalid(
            "Local NLI graph must expose only the selected three-way float32 logits head",
        ));
    }
    Ok(())
}
