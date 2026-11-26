//! Shared parsing for ordered-list marker syntax, and the shared unordered-bullet glyph set.

/// Glyphs a PDF producer uses to mark an unordered list item.
///
/// This exists because the set was enumerated independently at a dozen sites across list
/// detection, list normalisation, paragraph splitting and the table guards, and those copies
/// drifted: `\u{27A4}` appeared in exactly one of them and `\u{27A2}` in none, which is GH#1790 --
/// an arrow-bullet list folded into the paragraph above it. Adding a glyph here reaches every
/// consumer at once. The contents are the union of what the list-detection sites already
/// accepted, plus `\u{27A2}`, so routing them here changes no other glyph's behaviour.
///
/// Dashes (`-`, en/em dash and friends) are deliberately NOT here: they are ambiguous with
/// ordinary punctuation and each site gates them differently, usually on a trailing space. ~keep
pub(super) const BULLET_GLYPHS: &[char] = &[
    '\u{2022}', // bullet
    '\u{00B7}', // middle dot
    '\u{25E6}', // white bullet
    '\u{25AA}', // black small square
    '\u{2023}', // triangular bullet
    '\u{27A2}', // three-d top-lighted rightwards arrowhead (GH#1790)
    '\u{27A4}', // black rightwards arrowhead
    '\u{25BA}', // black right-pointing pointer
    '\u{25B6}', // black right-pointing triangle
    '\u{25CB}', // white circle
    '\u{25CF}', // black circle
];

/// True when `candidate` is one of [`BULLET_GLYPHS`]. ~keep
pub(crate) fn is_bullet_glyph(candidate: char) -> bool {
    BULLET_GLYPHS.contains(&candidate)
}

const MAX_NUMERIC_MARKER_DIGITS: usize = 3;
const MAX_ROMAN_MARKER_CHARS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrderedListMarker {
    pub(super) content_start: usize,
    pub(super) has_content: bool,
    pub(super) has_separator: bool,
    pub(super) numeric_value: Option<u16>,
}

pub(super) fn parse_ordered_list_marker(text: &str) -> Option<OrderedListMarker> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let leading_whitespace = text.len() - trimmed.len();
    let (marker_len, numeric_value) = parse_bracketed_numeric_marker(trimmed)
        .or_else(|| parse_parenthesized_marker(trimmed))
        .or_else(|| parse_suffixed_marker(trimmed))?;
    finish_marker(text, leading_whitespace + marker_len, numeric_value)
}

fn parse_bracketed_numeric_marker(text: &str) -> Option<(usize, Option<u16>)> {
    let inner = text.strip_prefix('[')?;
    let closing = inner.find(']')?;
    let marker = &inner[..closing];
    numeric_marker_value(marker).map(|value| (closing + 2, Some(value)))
}

fn parse_parenthesized_marker(text: &str) -> Option<(usize, Option<u16>)> {
    let inner = text.strip_prefix('(')?;
    let closing = inner.find(')')?;
    let marker = &inner[..closing];
    is_parenthesized_enumerator(marker).then(|| (closing + 2, numeric_marker_value(marker)))
}

/// Whether the text between parentheses enumerates: a number of up to three digits, a
/// letter, a roman numeral, or a doubled letter (`(aa)`, as statutes continue past `(z)`).
/// An acronym or a year in parentheses opens a wrapped line of prose as often as an item:
/// 2026-14578 turned `(SEFSC) performed an analysis`, `(IHA) is provided` and `(1981)
/// documented` into list items, each splitting its sentence in two. ~keep
fn is_parenthesized_enumerator(marker: &str) -> bool {
    let mut chars = marker.chars();
    let (Some(first), second) = (chars.next(), chars.next()) else {
        return false;
    };
    numeric_marker_value(marker).is_some()
        || (first.is_alphabetic() && second.is_none())
        || is_roman_marker(marker)
        || (first.is_alphabetic() && second == Some(first) && chars.next().is_none())
}

fn parse_suffixed_marker(text: &str) -> Option<(usize, Option<u16>)> {
    let (delimiter_index, delimiter) = text
        .char_indices()
        .find(|(_, character)| matches!(character, '.' | ')'))?;
    let marker = &text[..delimiter_index];
    let numeric_value = numeric_marker_value(marker);
    let valid = numeric_value.is_some()
        || marker.chars().count() == 1 && marker.chars().all(char::is_alphanumeric)
        || is_roman_marker(marker);
    valid.then(|| (delimiter_index + delimiter.len_utf8(), numeric_value))
}

fn finish_marker(text: &str, marker_end: usize, numeric_value: Option<u16>) -> Option<OrderedListMarker> {
    let remainder = text.get(marker_end..)?;
    if remainder.is_empty() {
        return Some(OrderedListMarker {
            content_start: marker_end,
            has_content: false,
            has_separator: false,
            numeric_value,
        });
    }
    if remainder.chars().next()?.is_whitespace() {
        let content = remainder.trim_start();
        return Some(OrderedListMarker {
            content_start: text.len() - content.len(),
            has_content: !content.is_empty(),
            has_separator: true,
            numeric_value,
        });
    }
    Some(OrderedListMarker {
        content_start: marker_end,
        has_content: true,
        has_separator: false,
        numeric_value,
    })
}

fn numeric_marker_value(marker: &str) -> Option<u16> {
    let length = marker.chars().count();
    ((1..=MAX_NUMERIC_MARKER_DIGITS).contains(&length) && marker.chars().all(|character| character.is_ascii_digit()))
        .then(|| marker.parse().ok())
        .flatten()
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
    fn exposes_numeric_values_only_for_numeric_markers() {
        for (source, expected) in [
            ("1. first", Some(1)),
            ("  12) twelfth", Some(12)),
            ("(7) seventh", Some(7)),
            ("[42] answer", Some(42)),
            ("a. alpha", None),
            ("I. Roman", None),
        ] {
            let marker = parse_ordered_list_marker(source).expect("marker should parse");
            assert_eq!(marker.numeric_value, expected, "source: {source}");
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

    /// 2026-14578: a wrapped line of prose opening with an acronym or a year in
    /// parentheses was read as an enumerated item.
    #[test]
    fn rejects_parenthesized_acronyms_and_years_but_keeps_enumerators() {
        for prose in [
            "(SEFSC) performed an analysis",
            "(IHA) is provided",
            "(1981) documented",
        ] {
            assert!(parse_ordered_list_marker(prose).is_none(), "source: {prose}");
        }
        for item in [
            "(a) Enforcement",
            "(D) Definitions",
            "(iv) Fourth",
            "(12) Twelfth",
            "(aa) Continued",
        ] {
            assert!(parse_ordered_list_marker(item).is_some(), "source: {item}");
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
