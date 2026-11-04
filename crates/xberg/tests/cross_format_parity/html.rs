/// Strip HTML tags, retaining image alternatives and decoding entities once.
pub(super) fn strip_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find('<') {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        if remaining.starts_with("<!--") {
            let Some(end) = remaining.find("-->") else { break };
            remaining = &remaining[end + 3..];
            continue;
        }
        let Some(end) = html_tag_end(remaining) else { break };
        if let Some(alt) = html_image_alt(&remaining[1..end]) {
            result.push(' ');
            result.push_str(alt);
        }
        result.push(' ');
        remaining = &remaining[end + 1..];
    }
    result.push_str(remaining);
    decode_html_entities(&result)
}

fn html_tag_end(text: &str) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in text.char_indices() {
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch == '>' {
            return Some(index);
        }
    }
    None
}

fn html_image_alt(tag: &str) -> Option<&str> {
    let name_end = tag
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '/')
        .unwrap_or(tag.len());
    if !tag[..name_end].eq_ignore_ascii_case("img") {
        return None;
    }
    let mut attributes = &tag[name_end..];
    while let Some((name, value)) = next_html_attribute(&mut attributes) {
        if name.eq_ignore_ascii_case("alt") {
            return Some(value);
        }
    }
    None
}

fn next_html_attribute<'a>(remaining: &mut &'a str) -> Option<(&'a str, &'a str)> {
    let text = remaining.trim_start();
    if text.is_empty() || text.starts_with('/') {
        return None;
    }
    let name_end = text
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '=')
        .unwrap_or(text.len());
    let name = &text[..name_end];
    let rest = text[name_end..].trim_start();
    let Some(value) = rest.strip_prefix('=') else {
        *remaining = rest;
        return Some((name, ""));
    };
    let value = value.trim_start();
    if let Some(quote @ ('\'' | '"')) = value.chars().next() {
        let end = value[1..].find(quote)? + 1;
        *remaining = &value[end + 1..];
        Some((name, &value[1..end]))
    } else {
        let end = value.find(char::is_whitespace).unwrap_or(value.len());
        *remaining = &value[end..];
        Some((name, &value[..end]))
    }
}

/// Decode isolated references with the existing CommonMark entity table.
fn decode_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find('&') {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let reference_length = remaining[1..]
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '#')
            .map(|end| end + 1);
        if let Some(end) = reference_length.filter(|&end| remaining.as_bytes()[end] == b';') {
            let reference = &remaining[..=end];
            for event in pulldown_cmark::Parser::new(reference) {
                if let pulldown_cmark::Event::Text(decoded) = event {
                    result.push_str(&decoded);
                }
            }
            remaining = &remaining[end + 1..];
        } else {
            result.push('&');
            remaining = &remaining[1..];
        }
    }
    result.push_str(remaining);
    result
}
