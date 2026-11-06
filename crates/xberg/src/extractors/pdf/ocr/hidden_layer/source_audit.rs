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
    remaining_work: &mut usize,
) -> Option<String> {
    let (bytes, split) = if let Some(streams) = content.as_array() {
        (audit_split_contents(doc, streams, byte_limit, remaining_work)?, true)
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

pub(super) fn audit_split_contents(
    doc: &PdfDocument,
    streams: &[Object],
    byte_limit: usize,
    remaining_work: &mut usize,
) -> Option<Vec<u8>> {
    let [paint, text] = streams else {
        return None;
    };
    let paint = doc.resolve_object(paint).ok()?;
    let text = doc.resolve_object(text).ok()?;
    let paint = raw_stream(&paint)?;
    let text = raw_stream(&text)?;
    let raw_bytes = paint.len().checked_add(text.len())?;
    let joined_bytes = raw_bytes.checked_add(2)?;
    // The raw inputs, joined audit, and Native's comparison must fit together. ~keep
    if joined_bytes > MAX_CONTENT_BYTES.min(byte_limit / 4) {
        return None;
    }
    *remaining_work = remaining_work.checked_sub(raw_bytes)?;
    // The general parser normalizes malformed operator operands. Validate exact raw syntax first. ~keep
    let text_operators = strict_split_syntax(paint, text)?;
    let joined_operators = SPLIT_PAINT_OPERATORS.checked_add(text_operators)?;
    if joined_operators > MAX_OPERATORS {
        return None;
    }
    *remaining_work = remaining_work.checked_sub(SPLIT_PAINT_OPERATORS.checked_add(joined_operators)?)?;
    let mut count = 0;
    let complete = visit_complete_content_stream(paint, SPLIT_PAINT_OPERATORS, |_| {
        count += 1;
        true
    })
    .ok()?;
    if !complete || count != SPLIT_PAINT_OPERATORS {
        return None;
    }
    let mut bytes = Vec::with_capacity(joined_bytes);
    bytes.extend_from_slice(paint);
    bytes.push(b'\n');
    bytes.extend_from_slice(text);
    bytes.push(b'\n');
    Some(bytes)
}

fn raw_stream(content: &Object) -> Option<&[u8]> {
    let Object::Stream { dict, data } = content else {
        return None;
    };
    dict.keys().all(|key| key == "Length").then_some(data.as_ref())
}

fn strict_split_syntax(paint: &[u8], text: &[u8]) -> Option<usize> {
    if !paint
        .iter()
        .chain(text)
        .all(|byte| matches!(*byte, b' '..=b'~' | b'\t' | b'\r' | b'\n'))
    {
        return None;
    }
    let mut paint = std::str::from_utf8(paint).ok()?.split_ascii_whitespace();
    if paint.next()? != "q" || !axis_matrix(&mut paint, "cm")? {
        return None;
    }
    if !simple_name(paint.next()?) || paint.next()? != "Do" || paint.next()? != "Q" || paint.next().is_some() {
        return None;
    }
    let mut lines = std::str::from_utf8(text).ok()?.trim_ascii().lines();
    let mut header = lines.next()?.split_ascii_whitespace();
    if header.next()? != "BT"
        || !simple_name(header.next()?)
        || pdf_number(header.next()?)? <= 0.0
        || !header.eq(["Tf", "3", "Tr"])
        || lines.next_back()?.trim_ascii() != "ET"
    {
        return None;
    }
    let mut rows = 0usize;
    for line in lines {
        let (matrix, literal) = line.trim_ascii().split_once(" Tm ")?;
        let mut matrix = matrix.split_ascii_whitespace().chain(["Tm"]);
        if !axis_matrix(&mut matrix, "Tm")? || matrix.next().is_some() || !plain_literal(literal) {
            return None;
        }
        rows += 1;
    }
    (rows > 0).then_some(4 + 2 * rows)
}

fn axis_matrix<'a>(words: &mut impl Iterator<Item = &'a str>, operator: &str) -> Option<bool> {
    let (a, b, c, d, e, f) = (
        pdf_number(words.next()?)?,
        pdf_number(words.next()?)?,
        pdf_number(words.next()?)?,
        pdf_number(words.next()?)?,
        pdf_number(words.next()?)?,
        pdf_number(words.next()?)?,
    );
    Some(words.next()? == operator && positive_axis_matrix(a, b, c, d, e, f))
}

fn pdf_number(token: &str) -> Option<f32> {
    let unsigned = token.strip_prefix(['-', '+']).unwrap_or(token);
    if unsigned.is_empty() || !unsigned.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.') {
        return None;
    }
    let number: f32 = token.parse().ok()?;
    number.is_finite().then_some(number)
}

fn simple_name(token: &str) -> bool {
    token
        .strip_prefix('/')
        .is_some_and(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric()))
}

fn plain_literal(token: &str) -> bool {
    token
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(") Tj"))
        .is_some_and(|text| {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte) && !matches!(byte, b'(' | b')' | b'\\'))
        })
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
