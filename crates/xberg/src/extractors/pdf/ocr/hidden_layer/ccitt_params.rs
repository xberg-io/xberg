//! The Group 4 image parameters that the hidden-layer preview decodes exactly.

use super::Object;

/// The `/BlackIs1` value of a plain Group 4 image whose parameters the probe decodes exactly.
/// `/Rows` and `/BlackIs1` may be absent, as the specification defaults them.
pub(super) fn supported_group4_params(
    params: &std::collections::HashMap<String, Object>,
    width: u32,
    height: u32,
) -> Option<bool> {
    let known = params
        .keys()
        .all(|key| matches!(key.as_str(), "K" | "Columns" | "Rows" | "BlackIs1"));
    let rows_match = params
        .get("Rows")
        .is_none_or(|rows| rows.as_integer() == Some(i64::from(height)));
    let black_is_1 = match params.get("BlackIs1") {
        None => false,
        Some(Object::Boolean(value)) => *value,
        Some(_) => return None,
    };
    (width > 0
        && u16::try_from(width).is_ok()
        && height > 0
        && known
        && rows_match
        && params.get("K").and_then(Object::as_integer) == Some(-1)
        && params.get("Columns").and_then(Object::as_integer) == Some(i64::from(width)))
    .then_some(black_is_1)
}

/// Whether a one-component `/Decode` array reverses the sample range. Only the identity and the
/// inverted range are valid for a bilevel image.
pub(super) fn decode_is_inverted(decode: Option<&Object>) -> Option<bool> {
    let Some(decode) = decode else {
        return Some(false);
    };
    let number = |object: &Object| match object {
        Object::Integer(value) => Some(*value as f64),
        Object::Real(value) => Some(*value),
        _ => None,
    };
    match decode.as_array()?.as_slice() {
        [low, high] => match (number(low)?, number(high)?) {
            (0.0, 1.0) => Some(false),
            (1.0, 0.0) => Some(true),
            _ => None,
        },
        _ => None,
    }
}
