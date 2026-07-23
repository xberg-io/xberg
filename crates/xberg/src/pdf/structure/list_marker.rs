//! Shared parsing for ordered-list marker syntax.

const MAX_NUMERIC_MARKER_DIGITS: usize = 3;
const MAX_ROMAN_MARKER_CHARS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrderedListMarker {
    pub(super) content_start: usize,
    pub(super) has_content: bool,
    pub(super) has_separator: bool,
}

pub(super) fn parse_ordered_list_marker(text: &str) -> Option<OrderedListMarker> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let leading_whitespace = text.len() - trimmed.len();
    let marker_len = parse_bracketed_numeric_marker(trimmed)
        .or_else(|| parse_parenthesized_marker(trimmed))
        .or_else(|| parse_suffixed_marker(trimmed))?;
    finish_marker(text, leading_whitespace + marker_len)
}

fn parse_bracketed_numeric_marker(text: &str) -> Option<usize> {
    let inner = text.strip_prefix('[')?;
    let closing = inner.find(']')?;
    let marker = &inner[..closing];
    is_numeric_marker(marker).then_some(closing + 2)
}

fn parse_parenthesized_marker(text: &str) -> Option<usize> {
    let inner = text.strip_prefix('(')?;
    let closing = inner.find(')')?;
    let marker = &inner[..closing];
    (!marker.is_empty() && marker.chars().all(char::is_alphanumeric)).then_some(closing + 2)
}

fn parse_suffixed_marker(text: &str) -> Option<usize> {
    let (delimiter_index, delimiter) = text
        .char_indices()
        .find(|(_, character)| matches!(character, '.' | ')'))?;
    let marker = &text[..delimiter_index];
    let valid = is_numeric_marker(marker)
        || marker.chars().count() == 1 && marker.chars().all(char::is_alphanumeric)
        || is_roman_marker(marker);
    valid.then_some(delimiter_index + delimiter.len_utf8())
}

fn finish_marker(text: &str, marker_end: usize) -> Option<OrderedListMarker> {
    let remainder = text.get(marker_end..)?;
    if remainder.is_empty() {
        return Some(OrderedListMarker {
            content_start: marker_end,
            has_content: false,
            has_separator: false,
        });
    }
    if remainder.chars().next()?.is_whitespace() {
        let content = remainder.trim_start();
        return Some(OrderedListMarker {
            content_start: text.len() - content.len(),
            has_content: !content.is_empty(),
            has_separator: true,
        });
    }
    Some(OrderedListMarker {
        content_start: marker_end,
        has_content: true,
        has_separator: false,
    })
}

fn is_numeric_marker(marker: &str) -> bool {
    let length = marker.chars().count();
    (1..=MAX_NUMERIC_MARKER_DIGITS).contains(&length) && marker.chars().all(|character| character.is_ascii_digit())
}

fn is_roman_marker(marker: &str) -> bool {
    let length = marker.chars().count();
    (1..=MAX_ROMAN_MARKER_CHARS).contains(&length)
        && marker
            .chars()
            .all(|character| matches!(character.to_ascii_lowercase(), 'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm'))
}

#[cfg(test)]
mod tests {
    use super::parse_ordered_list_marker;

    #[test]
    fn parses_supported_marker_families_and_content_offsets() {
        for (source, expected_content) in [
            ("a. alpha", "alpha"),
            ("I. Roman", "Roman"),
            ("(1) parenthesized", "parenthesized"),
            ("[1] bracketed", "bracketed"),
            ("  12) numeric", "numeric"),
        ] {
            let marker = parse_ordered_list_marker(source).expect("marker should parse");
            assert!(marker.has_content, "source: {source}");
            assert!(marker.has_separator, "source: {source}");
            assert_eq!(&source[marker.content_start..], expected_content, "source: {source}");
        }
    }

    #[test]
    fn parses_bare_markers_for_split_marker_and_body_runs() {
        for source in ["a.", "I.", "(1)", "[1]"] {
            let marker = parse_ordered_list_marker(source).expect("bare marker should parse");
            assert!(!marker.has_content, "source: {source}");
            assert!(!marker.has_separator, "source: {source}");
            assert_eq!(marker.content_start, source.len(), "source: {source}");
        }
    }

    #[test]
    fn rejects_malformed_or_unsupported_markers() {
        for source in [
            "",
            "1",
            "1: body",
            "1000. body",
            "word. body",
            "[a] body",
            "[1) body",
            "(1] body",
        ] {
            assert!(parse_ordered_list_marker(source).is_none(), "source: {source}");
        }
    }

    #[test]
    fn compact_content_is_available_to_assembly_but_not_detection() {
        let marker = parse_ordered_list_marker("I.Split body").expect("marker should parse");
        assert!(marker.has_content);
        assert!(!marker.has_separator);
        assert_eq!(marker.content_start, 2);
    }
}
