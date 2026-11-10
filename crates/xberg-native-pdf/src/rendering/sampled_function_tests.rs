use super::*;

/// A Type 0 sampled tint transform's input arity comes straight from the
/// document's `/Domain` array, and the evaluator interpolates across
/// `2^arity` grid corners. An all-ones `/Size` collapses the declared
/// sample count to 1, so a one-byte stream satisfies every length guard at
/// any arity whatsoever — which leaves [`MAX_SAMPLED_FUNCTION_DIMS`] as the
/// only thing bounding the corner loop.
fn all_ones_type0_function(dims: usize) -> (Object, std::collections::HashMap<String, Object>) {
    let mut dict = std::collections::HashMap::new();
    dict.insert(
        "Domain".to_string(),
        Object::Array(
            (0..dims)
                .flat_map(|_| [Object::Integer(0), Object::Integer(1)])
                .collect(),
        ),
    );
    dict.insert(
        "Range".to_string(),
        Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
    );
    dict.insert("Size".to_string(), Object::Array(vec![Object::Integer(1); dims]));
    dict.insert("BitsPerSample".to_string(), Object::Integer(8));
    let obj = Object::Stream {
        dict: dict.clone(),
        data: bytes::Bytes::from_static(&[0u8]),
    };
    (obj, dict)
}

/// The cap is a real ceiling, not a rejection of everything: a tint
/// transform sitting exactly on it must still evaluate. Without this the
/// dimension guard could be tightened to zero and every negative test
/// would keep passing.
#[test]
fn sampled_tint_transform_at_the_dimension_cap_still_evaluates() {
    let (obj, dict) = all_ones_type0_function(MAX_SAMPLED_FUNCTION_DIMS);
    let bc = vec![0.5_f32; MAX_SAMPLED_FUNCTION_DIMS];

    assert_eq!(
        evaluate_type0_multi(&obj, &dict, &bc),
        Some(vec![0.0]),
        "a {MAX_SAMPLED_FUNCTION_DIMS}-dimension tint transform is legal and must still evaluate"
    );
}

/// One dimension past the cap must be rejected on arity alone. Against
/// uncapped code this returns `Some` — every other guard is satisfied by
/// the all-ones `/Size` — so the corner count is document-controlled.
#[test]
fn sampled_tint_transform_past_the_dimension_cap_is_rejected() {
    let dims = MAX_SAMPLED_FUNCTION_DIMS + 1;
    let (obj, dict) = all_ones_type0_function(dims);
    let bc = vec![0.5_f32; dims];

    assert_eq!(
        evaluate_type0_multi(&obj, &dict, &bc),
        None,
        "a {dims}-dimension /Domain must be rejected before the 2^{dims} corner loop"
    );
}
