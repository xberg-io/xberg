//! Text conventions of a construction drawing set.
//!
//! The three parsers here need nothing but text: they read the title block, the
//! dimension strings and the scale notes that a drawing set already carries.
//! They are deliberately narrow, following US architectural notation only:
//!
//! - Dimensions use feet and inches: `12'-0 5/8"`, `1'-6"`, `6"`, or a bare
//!   decimal/fractional inch value such as `144.625`.
//! - Scales use either a ratio (`1:100`) or an architectural equivalent
//!   (`3/8" = 1'-0"`, `1" = 10'`).
//! - Sheet numbers look like `A5.01`, `A-101` or `E102`.
//!
//! Every parser returns `None` rather than guessing. A title is reported only
//! when the text labels it or a plain-language line sits next to the sheet
//! number; the parser never fabricates one from unrelated tokens.

use crate::types::VectorPoint;

/// Metadata read from a drawing set's title block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SheetMetadata {
    /// Sheet number such as `A5.01`. `None` when the text carries no match.
    pub sheet_number: Option<String>,
    /// Sheet title, taken from a labelled line or a plain-language line next to
    /// the sheet number. `None` when the text does not identify one.
    pub title: Option<String>,
    /// Revision code such as `C` from `REV C` or `REVISION: C`.
    pub revision: Option<String>,
}

/// A dimension string paired with the page position of its text.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDimension {
    /// The source dimension string.
    pub text: String,
    /// The dimension in inches (`12'-0 5/8"` is `144.625`).
    pub inches: f64,
    /// Position of the dimension text in page coordinates, copied from the
    /// caller so a dimension can be paired with the dimension line it labels.
    pub position: VectorPoint,
}

/// A parsed scale note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParsedScale {
    /// Paper length in inches when both sides of the note name a length.
    /// `None` for a pure ratio such as `1:100`.
    pub drawing_inches: Option<f64>,
    /// Real-world length in inches when both sides name a length. `None` for a
    /// pure ratio.
    pub real_inches: Option<f64>,
    /// Real-world units per paper unit: `3/8" = 1'-0"` is `32.0` (1:32) and
    /// `1:100` is `100.0`.
    pub ratio: f64,
}

/// The drawing-set text parsers.
///
/// Keeping them on one type gives every caller a single entry point and keeps
/// the accepted/rejected grammar in one place.
pub struct DrawingSetText;

impl DrawingSetText {
    /// Parse a dimension string to inches.
    ///
    /// Accepts feet-and-inches (`12'-0 5/8"`), inches alone (`6"`), feet alone
    /// (`12'`) and a bare decimal or fractional inch value (`144.625`, `5/8`).
    /// Rejects scale notes, ratios, sheet numbers, alphabetic text and anything
    /// with trailing junk.
    pub fn parse_dimension(text: &str) -> Option<f64> {
        let normalized = normalize_typography(text);
        let trimmed = normalized.trim();
        if trimmed.is_empty() || trimmed.contains('=') || trimmed.contains(':') {
            return None;
        }
        if trimmed.chars().any(|character| character.is_ascii_alphabetic()) {
            return None;
        }

        let mut total = 0.0;
        let mut rest = trimmed;

        if let Some(feet_end) = rest.find('\'') {
            let feet_text = rest[..feet_end].trim();
            if feet_text.is_empty() {
                return None;
            }
            total += parse_component(feet_text)? * 12.0;
            rest = rest[feet_end + 1..].trim();
            if let Some(after_dash) = rest.strip_prefix('-') {
                rest = after_dash.trim();
            }
        }

        let inch_text = rest.trim().trim_end_matches('"').trim();
        if inch_text.is_empty() {
            return (total > 0.0).then_some(total);
        }
        total += parse_component(inch_text)?;
        Some(total)
    }

    /// Parse the dimension strings of a page, keeping each one's text position.
    ///
    /// Entries that are not dimensions are dropped; the order of the survivors
    /// follows the caller's order.
    #[cfg_attr(feature = "alef-meta", alef(skip))]
    pub fn parse_dimensions<'a>(items: impl IntoIterator<Item = (&'a str, VectorPoint)>) -> Vec<ParsedDimension> {
        items
            .into_iter()
            .filter_map(|(text, position)| {
                Self::parse_dimension(text).map(|inches| ParsedDimension {
                    text: text.to_string(),
                    inches,
                    position,
                })
            })
            .collect()
    }

    /// Parse a scale note to a real-world-to-paper ratio.
    ///
    /// `3/8" = 1'-0"` gives `32.0`, `1" = 10'` gives `120.0` and `1:100` gives
    /// `100.0`. Both sides of an `=` form must name a length; a bare number
    /// without a unit is rejected rather than assumed.
    pub fn parse_scale(text: &str) -> Option<ParsedScale> {
        let normalized = normalize_typography(text);
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some((drawing, real)) = trimmed.split_once(':') {
            let drawing = parse_positive(drawing.trim())?;
            let real = parse_positive(real.trim())?;
            return Some(ParsedScale {
                drawing_inches: None,
                real_inches: None,
                ratio: real / drawing,
            });
        }

        let (drawing, real) = trimmed.split_once('=')?;
        let drawing = parse_length_inches(drawing.trim())?;
        let real = parse_length_inches(real.trim())?;
        if drawing <= 0.0 || real <= 0.0 {
            return None;
        }
        Some(ParsedScale {
            drawing_inches: Some(drawing),
            real_inches: Some(real),
            ratio: real / drawing,
        })
    }

    /// Read a sheet number such as `A5.01`, `A-101` or `E102`.
    ///
    /// A bare two-character token such as `A5` is not accepted: it is far more
    /// likely a paper size or a room label than a sheet number.
    pub fn parse_sheet_number(text: &str) -> Option<String> {
        let normalized = normalize_typography(text);
        for raw_token in normalized.split_whitespace() {
            let token = raw_token
                .trim_matches(|character: char| character.is_ascii_punctuation() && !matches!(character, '.' | '-'));
            let token = token.trim_end_matches('.');
            if is_sheet_number(token) {
                return Some(token.to_string());
            }
        }
        None
    }

    /// Read a revision code from a line that labels it.
    ///
    /// Only an explicit `REV`/`REVISION` label counts; a lone letter elsewhere in
    /// the title block is not treated as a revision.
    pub fn parse_revision(text: &str) -> Option<String> {
        for line in normalize_typography(text).lines() {
            let Some(position) = find_case_insensitive(line, "revision").or_else(|| find_case_insensitive(line, "rev"))
            else {
                continue;
            };
            let tail = &line[position..];
            let tail = tail
                .trim_start_matches(|character: char| character.is_ascii_alphabetic())
                .trim_start_matches(|character: char| {
                    character.is_whitespace() || matches!(character, ':' | '#' | '.' | '-')
                });
            let code: String = tail
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric())
                .collect();
            if (1..=4).contains(&code.len()) && code.chars().any(|character| character.is_ascii_alphanumeric()) {
                return Some(code);
            }
        }
        None
    }

    /// Read a drawing set's title block.
    ///
    /// Each field is independent: a title block that carries only a sheet
    /// number returns a sheet number and `None` for the rest.
    pub fn parse_title_block(text: &str) -> SheetMetadata {
        let normalized = normalize_typography(text);
        let lines: Vec<&str> = normalized
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        let sheet_number = Self::parse_sheet_number(&normalized);
        let revision = Self::parse_revision(&normalized);
        let title = labelled_title(&lines).or_else(|| unlabelled_title(&lines, sheet_number.as_deref()));

        SheetMetadata {
            sheet_number,
            title,
            revision,
        }
    }
}

/// Typographic variants a PDF text extractor may emit for feet, inches, dashes
/// and fractions.
fn normalize_typography(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\u{2032}' => '\'',
            '\u{2033}' => '"',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
            '\u{2044}' => '/',
            other => other,
        })
        .collect()
}

fn parse_positive(text: &str) -> Option<f64> {
    let value: f64 = text.parse().ok()?;
    (value.is_finite() && value > 0.0).then_some(value)
}

/// A length note (`3/8"`, `1'-0"`, `10'`) in inches.
fn parse_length_inches(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if !trimmed.contains('\'') && !trimmed.contains('"') {
        return None;
    }
    DrawingSetText::parse_dimension(trimmed)
}

/// A whole number or fraction (`6`, `5/8`, `6 1/2`) as a decimal.
fn parse_component(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut pieces = text.split_whitespace();
    let first = pieces.next()?;
    let mut value = parse_single_number(first)?;
    for piece in pieces {
        value += parse_single_number(piece)?;
    }
    Some(value)
}

fn parse_single_number(text: &str) -> Option<f64> {
    if let Some((numerator, denominator)) = text.split_once('/') {
        if numerator.contains('/') || denominator.contains('/') {
            return None;
        }
        let numerator: f64 = numerator.trim().parse().ok()?;
        let denominator: f64 = denominator.trim().parse().ok()?;
        if !numerator.is_finite() || !denominator.is_finite() || numerator < 0.0 || denominator == 0.0 {
            return None;
        }
        return Some(numerator / denominator);
    }
    let value: f64 = text.parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

/// `A5.01`, `A-101`, `E102`: one or two letters, then a separated group or a
/// three-digit group, optionally with a dotted subdivision.
fn is_sheet_number(token: &str) -> bool {
    let mut characters = token.chars();
    let mut prefix_length = 0;
    while prefix_length < 2
        && characters
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
    {
        prefix_length += 1;
    }
    if prefix_length == 0 {
        return false;
    }
    let rest = &token[prefix_length..];
    if rest.is_empty() || rest.len() > 8 {
        return false;
    }
    if let Some(without_separator) = rest.strip_prefix('-') {
        if without_separator.is_empty() {
            return false;
        }
        return valid_sheet_digits(without_separator);
    }
    if let Some((group, subdivision)) = rest.split_once('.') {
        return !group.is_empty()
            && valid_sheet_digits(group)
            && !subdivision.is_empty()
            && subdivision.len() <= 3
            && subdivision.chars().all(|character| character.is_ascii_digit());
    }
    rest.len() >= 3 && valid_sheet_digits(rest)
}

fn valid_sheet_digits(text: &str) -> bool {
    !text.is_empty() && text.len() <= 4 && text.chars().all(|character| character.is_ascii_digit())
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let haystack = haystack.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    haystack.find(&needle)
}

fn labelled_title(lines: &[&str]) -> Option<String> {
    for line in lines {
        let lower = line.to_ascii_lowercase();
        for label in ["sheet title", "drawing title", "title"] {
            if let Some(position) = lower.find(label) {
                let tail = &line[position + label.len()..];
                let tail = tail.trim_start_matches(|character: char| {
                    character.is_whitespace() || matches!(character, ':' | '.' | '-')
                });
                let title = tail.trim();
                if !title.is_empty() && title.chars().any(|character| character.is_ascii_alphabetic()) {
                    return Some(title.to_string());
                }
            }
        }
    }
    None
}

/// A plain-language line within three lines of the sheet number, when one looks
/// like a title rather than a number, date, scale or revision.
fn unlabelled_title(lines: &[&str], sheet_number: Option<&str>) -> Option<String> {
    let sheet_number = sheet_number?;
    let index = lines.iter().position(|line| line.contains(sheet_number))?;
    let start = index.saturating_sub(3);
    let end = (index + 4).min(lines.len());
    let mut candidates = Vec::new();
    for (offset, line) in lines[start..end].iter().enumerate() {
        let absolute = start + offset;
        if absolute == index || !looks_like_title(line) {
            continue;
        }
        candidates.push((absolute.abs_diff(index), *line));
    }
    candidates.sort_by_key(|(distance, _)| *distance);
    candidates.into_iter().next().map(|(_, line)| line.to_string())
}

fn looks_like_title(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 || words.len() > 12 {
        return false;
    }
    if line.contains('=')
        || line.contains(':')
        || line.contains('\'')
        || line.contains('"')
        || is_sheet_number(line.trim())
        || DrawingSetText::parse_scale(line).is_some()
        || DrawingSetText::parse_dimension(line).is_some()
        || DrawingSetText::parse_revision(line).is_some()
    {
        return false;
    }
    line.chars().filter(|character| character.is_ascii_alphabetic()).count() >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_parser_accepts_architectural_notation() {
        for (text, expected) in [
            ("12'-0 5/8\"", 144.625),
            ("12'-0\"", 144.0),
            ("1'-6\"", 18.0),
            ("0'-6\"", 6.0),
            ("6\"", 6.0),
            ("12'", 144.0),
            ("144.625", 144.625),
            ("5/8", 0.625),
            ("12'-6 1/2\"", 150.5),
            ("12\u{2032}-0 5/8\u{2033}", 144.625),
            ("3' - 0 3/4\"", 36.75),
        ] {
            assert_eq!(
                DrawingSetText::parse_dimension(text),
                Some(expected),
                "dimension {text:?} must parse to {expected}"
            );
        }
    }

    #[test]
    fn dimension_parser_rejects_what_is_not_a_dimension() {
        for text in [
            "",
            "A5.01",
            "3/8\" = 1'-0\"",
            "1:100",
            "12'-x\"",
            "12'-0 5/8\" extra",
            "FLOOR PLAN",
            "-6\"",
            "1/0",
        ] {
            assert_eq!(
                DrawingSetText::parse_dimension(text),
                None,
                "dimension {text:?} must be rejected"
            );
        }
    }

    #[test]
    fn dimension_list_keeps_positions_and_drops_non_dimensions() {
        let items = [
            ("12'-0 5/8\"", VectorPoint { x: 100.0, y: 200.0 }),
            ("TITLE", VectorPoint { x: 0.0, y: 0.0 }),
            ("1'-6\"", VectorPoint { x: 3.0, y: 4.0 }),
        ];
        let parsed = DrawingSetText::parse_dimensions(items);
        assert_eq!(
            parsed,
            vec![
                ParsedDimension {
                    text: "12'-0 5/8\"".to_string(),
                    inches: 144.625,
                    position: VectorPoint { x: 100.0, y: 200.0 },
                },
                ParsedDimension {
                    text: "1'-6\"".to_string(),
                    inches: 18.0,
                    position: VectorPoint { x: 3.0, y: 4.0 },
                },
            ]
        );
    }

    #[test]
    fn scale_parser_accepts_architectural_and_ratio_forms() {
        for (text, ratio) in [
            ("3/8\" = 1'-0\"", 32.0),
            ("3/8\"=1'-0\"", 32.0),
            ("1/4\" = 1'-0\"", 48.0),
            ("1\" = 10'", 120.0),
            ("1/2\" = 1'-0\"", 24.0),
            ("1:100", 100.0),
            ("1 : 50", 50.0),
        ] {
            let parsed = DrawingSetText::parse_scale(text).unwrap_or_else(|| panic!("scale {text:?} must parse"));
            assert!(
                (parsed.ratio - ratio).abs() < 1e-9,
                "scale {text:?} must give ratio {ratio}, got {}",
                parsed.ratio
            );
        }
        let architectural = DrawingSetText::parse_scale("3/8\" = 1'-0\"").expect("the scale parses");
        assert_eq!(architectural.drawing_inches, Some(0.375));
        assert_eq!(architectural.real_inches, Some(12.0));
        let ratio = DrawingSetText::parse_scale("1:100").expect("the ratio parses");
        assert_eq!(ratio.drawing_inches, None);
        assert_eq!(ratio.real_inches, None);
    }

    #[test]
    fn scale_parser_rejects_ambiguous_or_invalid_notes() {
        for text in [
            "",
            "1 = 10'",
            "1\" = 10",
            "0:100",
            "1:0",
            "3/8\" = 1'-0\" extra",
            "SCALE",
            "FLOOR PLAN",
        ] {
            assert_eq!(
                DrawingSetText::parse_scale(text),
                None,
                "scale {text:?} must be rejected"
            );
        }
    }

    #[test]
    fn sheet_number_parser_accepts_the_drawing_set_forms() {
        for (text, expected) in [
            ("A5.01", "A5.01"),
            ("SHEET A5.01", "A5.01"),
            ("A-101", "A-101"),
            ("E102", "E102"),
            ("A5.01 FLOOR PLAN", "A5.01"),
        ] {
            assert_eq!(
                DrawingSetText::parse_sheet_number(text).as_deref(),
                Some(expected),
                "sheet number {text:?}"
            );
        }
    }

    #[test]
    fn sheet_number_parser_rejects_non_sheet_tokens() {
        for text in ["A5", "12'-0\"", "1:100", "FLOOR PLAN", "", "A5.", "ABC5.01"] {
            assert_eq!(
                DrawingSetText::parse_sheet_number(text),
                None,
                "token {text:?} must not be a sheet number"
            );
        }
    }

    #[test]
    fn revision_parser_requires_a_label() {
        assert_eq!(DrawingSetText::parse_revision("REV C").as_deref(), Some("C"));
        assert_eq!(DrawingSetText::parse_revision("Revision: B12").as_deref(), Some("B12"));
        assert_eq!(DrawingSetText::parse_revision("REV. 3").as_deref(), Some("3"));
        assert_eq!(DrawingSetText::parse_revision("FLOOR PLAN C"), None);
        assert_eq!(DrawingSetText::parse_revision("A5.01"), None);
    }

    #[test]
    fn title_block_reads_each_field_without_inventing_missing_ones() {
        let text = "PROJECT: CIVIC CENTER\nFLOOR PLAN\nA5.01\nREV C\n3/8\" = 1'-0\"";
        let metadata = DrawingSetText::parse_title_block(text);
        assert_eq!(metadata.sheet_number.as_deref(), Some("A5.01"));
        assert_eq!(metadata.title.as_deref(), Some("FLOOR PLAN"));
        assert_eq!(metadata.revision.as_deref(), Some("C"));

        let only_number = DrawingSetText::parse_title_block("A5.01");
        assert_eq!(only_number.sheet_number.as_deref(), Some("A5.01"));
        assert_eq!(only_number.title, None);
        assert_eq!(only_number.revision, None);

        let labelled = DrawingSetText::parse_title_block("SHEET TITLE: ROOF PLAN\nA2.03");
        assert_eq!(labelled.sheet_number.as_deref(), Some("A2.03"));
        assert_eq!(labelled.title.as_deref(), Some("ROOF PLAN"));
    }
}
