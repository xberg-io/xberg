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

/// `/Domain` and `/Range` pairs come from the document and feed `clamp`, which panics on reversed
/// bounds.
#[test]
fn sampled_function_with_reversed_domain_or_range_is_rejected() {
    for key in ["Domain", "Range"] {
        let (obj, mut dict) = all_ones_type0_function(1);
        dict.insert(
            key.to_string(),
            Object::Array(vec![Object::Integer(1), Object::Integer(0)]),
        );
        assert_eq!(evaluate_type0_multi(&obj, &dict, &[0.5]), None, "{key}");
    }
}

/// `/Bounds` must hold one entry fewer than `/Functions`; indexing it by the selected interval
/// otherwise runs off the end.
#[test]
fn stitching_function_with_short_bounds_is_rejected() {
    let doc = tests::minimal_pdf_doc();
    let sub = Object::Dictionary(std::collections::HashMap::from([
        ("FunctionType".to_string(), Object::Integer(2)),
        ("N".to_string(), Object::Integer(1)),
    ]));
    let stitching = Object::Dictionary(std::collections::HashMap::from([
        ("FunctionType".to_string(), Object::Integer(3)),
        (
            "Domain".to_string(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        ),
        (
            "Functions".to_string(),
            Object::Array(vec![sub.clone(), sub.clone(), sub]),
        ),
        ("Bounds".to_string(), Object::Array(vec![])),
    ]));
    assert_eq!(evaluate_pdf_function_at(&doc, &stitching, 0.9, 0), None);
}
