//! Audit the exact source paint before treating OCR as a supplement to native text.

use super::{
    MAX_CONTENT_BYTES, MAX_OPERATORS, Object, Operator, PdfDocument, decode_probe_stream, positive_axis_matrix,
    simple_font_resources, stream_filters, visit_complete_content_stream,
};

pub(super) fn audited_image_name(
    doc: &PdfDocument,
    page: usize,
    content: &Object,
    byte_limit: usize,
) -> Option<String> {
    let (bytes, split) = if let Some(streams) = content.as_array() {
        (audit_split_contents(doc, streams, byte_limit)?, true)
    } else {
        (
            decode_content_stream(content, MAX_CONTENT_BYTES.min(byte_limit / 4), byte_limit)?,
            false,
        )
    };
    let fonts = simple_font_resources(doc, page)?;
    let source = audit_hidden_text_image(&bytes, split, |font| fonts.contains_key(font))?;
    (doc.get_page_content_data(page).ok()? == bytes).then_some(source)
}

fn decode_content_stream(content: &Object, output_limit: usize, byte_limit: usize) -> Option<Vec<u8>> {
    let Object::Stream { dict, data } = content else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
    decode_probe_stream(data, stream_filters(dict.get("Filter"))?, output_limit, byte_limit)
}

const SPLIT_PAINT_OPERATORS: usize = 4;

pub(super) fn audit_split_contents(doc: &PdfDocument, streams: &[Object], byte_limit: usize) -> Option<Vec<u8>> {
    let [paint, text] = streams else {
        return None;
    };
    let paint = doc.resolve_object(paint).ok()?;
    let text = doc.resolve_object(text).ok()?;
    let mut encoded = 0usize;
    for content in [&paint, &text] {
        let Object::Stream { dict, data } = content else {
            return None;
        };
        if dict.keys().any(|key| !matches!(key.as_str(), "Length" | "Filter")) {
            return None;
        }
        encoded = encoded.checked_add(data.len())?;
    }
    // Bound encoded bytes, both decoded streams, the joined audit and Native's comparison. ~keep
    if encoded > byte_limit / 4 {
        return None;
    }
    let decoded_limit = MAX_CONTENT_BYTES.min(byte_limit / 8);
    let paint = decode_content_stream(&paint, decoded_limit.checked_sub(2)?, byte_limit / 4)?;
    let text = decode_content_stream(&text, decoded_limit.checked_sub(paint.len() + 2)?, byte_limit / 4)?;
    let mut count = 0;
    let complete = visit_complete_content_stream(&paint, SPLIT_PAINT_OPERATORS, |_| {
        count += 1;
        true
    })
    .ok()?;
    if !complete || count != SPLIT_PAINT_OPERATORS {
        return None;
    }
    let mut bytes = Vec::with_capacity(paint.len() + text.len() + 2);
    bytes.extend_from_slice(&paint);
    bytes.push(b'\n');
    bytes.extend_from_slice(&text);
    bytes.push(b'\n');
    Some(bytes)
}

#[cfg(test)]
pub(super) fn hidden_text_image_name(content: &[u8], font_is_known: impl Fn(&str) -> bool) -> Option<String> {
    audit_hidden_text_image(content, false, font_is_known)
}

pub(super) fn audit_hidden_text_image(
    content: &[u8],
    split: bool,
    font_is_known: impl Fn(&str) -> bool,
) -> Option<String> {
    let mut mode = 0;
    let mut known_font = false;
    let mut inside_text = false;
    let mut states = Vec::new();
    let mut image = None;
    let mut hidden = false;
    let mut index = 0;
    let complete = visit_complete_content_stream(content, MAX_OPERATORS, |op| {
        if split && !split_operator_in_sequence(index, inside_text, &op) {
            return false;
        }
        index += 1;
        match op {
            Operator::SaveState if !inside_text => states.push((mode, known_font)),
            Operator::RestoreState if !inside_text => {
                let Some((previous_mode, previous_font)) = states.pop() else {
                    return false;
                };
                mode = previous_mode;
                known_font = previous_font;
            }
            Operator::Cm { a, b, c, d, e, f } if !inside_text && positive_axis_matrix(a, b, c, d, e, f) => {}
            Operator::Do { name } if !inside_text && image.is_none() => image = Some(name),
            Operator::BeginText if !inside_text => inside_text = true,
            Operator::EndText if inside_text => inside_text = false,
            Operator::Tr { render } if inside_text => mode = render,
            Operator::Tf { font, size } if inside_text && size.is_finite() && size > 0.0 && font_is_known(&font) => {
                known_font = true;
            }
            Operator::Tj { .. } | Operator::TJ { .. } | Operator::Quote { .. } | Operator::DoubleQuote { .. }
                if inside_text && mode == 3 && known_font =>
            {
                hidden = true
            }
            Operator::Td { .. }
            | Operator::TD { .. }
            | Operator::Tm { .. }
            | Operator::TStar
            | Operator::Tz { .. }
            | Operator::TL { .. }
            | Operator::Tc { .. }
            | Operator::Tw { .. }
            | Operator::Ts { .. }
                if inside_text => {}
            _ => return false,
        }
        true
    })
    .ok()?;
    if !complete || inside_text || !states.is_empty() || !hidden {
        return None;
    }
    image
}

fn split_operator_in_sequence(index: usize, inside_text: bool, op: &Operator) -> bool {
    match index {
        0 => matches!(op, Operator::SaveState),
        1 => matches!(op, Operator::Cm { .. }),
        2 => matches!(op, Operator::Do { .. }),
        3 => matches!(op, Operator::RestoreState),
        SPLIT_PAINT_OPERATORS => matches!(op, Operator::BeginText),
        _ => inside_text,
    }
}
