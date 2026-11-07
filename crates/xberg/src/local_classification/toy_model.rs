fn varint(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value >= 0x80 {
        bytes.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
    bytes
}

fn scalar(field: u64, value: u64) -> Vec<u8> {
    [varint(field << 3), varint(value)].concat()
}

fn message(field: u64, payload: &[u8]) -> Vec<u8> {
    [varint((field << 3) | 2), varint(payload.len() as u64), payload.to_vec()].concat()
}

fn value_info(name: &str) -> Vec<u8> {
    let shape = message(1, &scalar(1, 1));
    let tensor = [scalar(1, 1), message(2, &shape)].concat();
    [message(1, name.as_bytes()), message(2, &message(1, &tensor))].concat()
}

/// Generate the repository-owned float `[1]` graph `Y = X + X`; no external model artifact is used.
/// The encoder uses ONNX `onnx.proto3` field numbers, IR version 8 and standard-domain opset 13.
pub(super) fn add_model_bytes() -> Vec<u8> {
    let node = [message(1, b"X"), message(1, b"X"), message(2, b"Y"), message(4, b"Add")].concat();
    let graph = [
        message(1, &node),
        message(2, b"xberg-local-classifier-termination"),
        message(11, &value_info("X")),
        message(12, &value_info("Y")),
    ]
    .concat();
    [
        scalar(1, 8),
        message(2, b"xberg-owned-toy-fixture"),
        message(7, &graph),
        message(8, &scalar(2, 13)),
    ]
    .concat()
}

fn tensor_info(name: &str, element_type: u64, dimensions: &[Option<u64>]) -> Vec<u8> {
    let shape = dimensions
        .iter()
        .flat_map(|dimension| {
            message(
                1,
                &dimension.map_or_else(|| message(2, b"sequence"), |value| scalar(1, value)),
            )
        })
        .collect::<Vec<_>>();
    let tensor = [scalar(1, element_type), message(2, &shape)].concat();
    [message(1, name.as_bytes()), message(2, &message(1, &tensor))].concat()
}

fn initializer(name: &str, element_type: u64, dimensions: &[u64], bytes: &[u8]) -> Vec<u8> {
    [
        dimensions.iter().flat_map(|value| scalar(1, *value)).collect(),
        scalar(2, element_type),
        message(8, name.as_bytes()),
        message(9, bytes),
    ]
    .concat()
}

fn integer_attribute(name: &str, value: u64) -> Vec<u8> {
    [message(1, name.as_bytes()), scalar(3, value), scalar(20, 2)].concat()
}

fn node(operator: &str, inputs: &[&str], output: &str, attributes: &[Vec<u8>]) -> Vec<u8> {
    [
        inputs.iter().flat_map(|input| message(1, input.as_bytes())).collect(),
        message(2, output.as_bytes()),
        message(4, operator.as_bytes()),
        attributes.iter().flat_map(|attribute| message(5, attribute)).collect(),
    ]
    .concat()
}

/// Repository-owned operational graph: entailment is one tenth of the input token-ID sum.
/// This tests pair-dependent execution and never represents a qualified NLI model.
pub(super) fn token_sum_nli_model_bytes(classes: u64) -> Vec<u8> {
    token_sum_graph(classes, initializer("scale", 1, &[1], &0.1f32.to_le_bytes()))
}

/// Put the operational graph's scale in an ONNX external-data tensor at the selected location.
pub(super) fn external_token_sum_nli_model_bytes(location: &str) -> Vec<u8> {
    let entries = [("location", location), ("offset", "0"), ("length", "4")];
    let external_data = entries
        .iter()
        .flat_map(|(key, value)| message(13, &[message(1, key.as_bytes()), message(2, value.as_bytes())].concat()))
        .collect::<Vec<_>>();
    let scale = [
        scalar(1, 1),
        scalar(2, 1),
        message(8, b"scale"),
        external_data,
        scalar(14, 1),
    ]
    .concat();
    token_sum_graph(3, scale)
}

fn token_sum_graph(classes: u64, scale: Vec<u8>) -> Vec<u8> {
    let nodes = [
        node("Cast", &["input_ids"], "float_ids", &[integer_attribute("to", 1)]),
        node(
            "ReduceSum",
            &["float_ids", "axes"],
            "sum",
            &[integer_attribute("keepdims", 1)],
        ),
        node("Mul", &["sum", "scale"], "entailment", &[]),
        node(
            "Concat",
            &["zero", "zero", "entailment"],
            "logits",
            &[integer_attribute("axis", 1)],
        ),
    ];
    let initializers = [
        initializer("axes", 7, &[1], &1i64.to_le_bytes()),
        scale,
        initializer("zero", 1, &[1, 1], &0.0f32.to_le_bytes()),
    ];
    let inputs = ["input_ids", "attention_mask", "token_type_ids"]
        .map(|name| message(11, &tensor_info(name, 7, &[Some(1), None])))
        .concat();
    let graph = [
        nodes.iter().flat_map(|node| message(1, node)).collect(),
        message(2, b"xberg-owned-token-pair-control"),
        initializers.iter().flat_map(|tensor| message(5, tensor)).collect(),
        inputs,
        message(12, &tensor_info("logits", 1, &[Some(1), Some(classes)])),
    ]
    .concat();
    [
        scalar(1, 8),
        message(2, b"xberg-owned-toy-fixture"),
        message(7, &graph),
        message(8, &scalar(2, 13)),
    ]
    .concat()
}
