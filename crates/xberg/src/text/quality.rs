use ahash::AHashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

use memchr::{memchr, memmem};

// ============================================================================
// ============================================================================

const OCR_PENALTY_WEIGHT: f64 = 0.3;
const SCRIPT_PENALTY_WEIGHT: f64 = 0.2;
const NAV_PENALTY_WEIGHT: f64 = 0.1;
const STRUCTURE_BONUS_WEIGHT: f64 = 0.2;
const METADATA_BONUS_WEIGHT: f64 = 0.1;

const MIN_TEXT_LENGTH: usize = 10;
const MIN_SENTENCE_WORDS: f64 = 10.0;
const MAX_SENTENCE_WORDS: f64 = 30.0;
const MIN_PARAGRAPH_WORDS: f64 = 50.0;
const MAX_PARAGRAPH_WORDS: f64 = 300.0;

static DASH_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[-]{3,}").expect("Dash pattern regex is valid and should compile"));

static JS_FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)function\s+\w+\s*\([^)]*\)\s*\{[^}]*\}")
        .expect("JavaScript function regex pattern is valid and should compile")
});
static CSS_RULES_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\.[a-zA-Z][\w-]*\s*\{[^}]*\}").expect("CSS rules regex pattern is valid and should compile")
});

static BREADCRUMB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:Home\s*[>»]\s*|[>»]\s*){2,}").expect("Breadcrumb regex pattern is valid and should compile")
});
static SENTENCE_DETECT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[.!?]\s+[A-Z]").expect("Sentence detection regex pattern is valid and should compile"));
static PUNCTUATION_DETECT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[.!?]").expect("Punctuation detection regex pattern is valid and should compile"));

#[cfg(test)]
static NEWLINE_CLEANUP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n+").expect("Newline cleanup regex pattern is valid and should compile"));

static WORD_CHAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\A\w\z").expect("word-char regex is valid"));
static DECIMAL_DIGIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\A\d\z").expect("digit regex is valid"));

/// Equivalent to the regex crate's Unicode `\w`. ASCII, Latin letters and whitespace are
/// answered directly; every other code point is delegated to the regex, so the answer is exact.
fn is_word_char(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphanumeric() || c == '_';
    }
    // Latin-1 Supplement and Latin Extended-A/B letters are \w in every Unicode version. Other
    // alphabetic code points go to the regex: std and regex-syntax can carry different Unicode
    // tables (U+0558 is alphabetic in std but not \w in regex-syntax).
    if matches!(c as u32, 0xC0..=0x24F) && !matches!(c, '\u{D7}' | '\u{F7}') {
        return true;
    }
    if c.is_whitespace() {
        return false;
    }
    WORD_CHAR.is_match(c.encode_utf8(&mut [0u8; 4]))
}

/// Equivalent to the regex crate's Unicode `\d` (general category Nd).
fn is_decimal_digit(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit();
    }
    c.is_numeric() && DECIMAL_DIGIT.is_match(c.encode_utf8(&mut [0u8; 4]))
}

fn char_at(text: &str, i: usize) -> Option<char> {
    text[i..].chars().next()
}

fn word_before(text: &str, i: usize) -> bool {
    text[..i].chars().next_back().is_some_and(is_word_char)
}

fn word_at(text: &str, i: usize) -> bool {
    char_at(text, i).is_some_and(is_word_char)
}

/// End of the whitespace run starting at `i` and its length in chars.
fn whitespace_run(text: &str, i: usize) -> (usize, usize) {
    let mut end = i;
    let mut count = 0;
    for c in text[i..].chars() {
        if !c.is_whitespace() {
            break;
        }
        end += c.len_utf8();
        count += 1;
    }
    (end, count)
}

fn ascii_letter_at(text: &str, i: usize) -> bool {
    text.as_bytes().get(i).is_some_and(u8::is_ascii_alphabetic)
}

/// `\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b` anchored at `i`.
fn scattered_chars_end(text: &str, i: usize) -> Option<usize> {
    let (j, n) = whitespace_run(text, i + 1);
    if n < 2 || !ascii_letter_at(text, j) {
        return None;
    }
    let (k, n) = whitespace_run(text, j + 1);
    if n < 2 || !ascii_letter_at(text, k) {
        return None;
    }
    let end = k + 1;
    (!word_at(text, end)).then_some(end)
}

/// `\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b` anchored at `i`. Any shorter end would be
/// followed by an ASCII alphanumeric, so only the maximal run can satisfy the final `\b`.
fn malformed_word_end(text: &str, i: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let run_end = i + bytes[i..].iter().take_while(|b| b.is_ascii_alphanumeric()).count();
    let mut p = i;
    while p < run_end && bytes[p].is_ascii_alphabetic() {
        p += 1;
    }
    let mut q = p;
    while q < run_end && bytes[q].is_ascii_digit() {
        q += 1;
    }
    (p > i && q > p && q < run_end && !word_at(text, run_end)).then_some(run_end)
}

fn byte_run_end(text: &str, i: usize, byte: u8) -> usize {
    i + text.as_bytes()[i..].iter().take_while(|&&b| b == byte).count()
}

/// Spans of the OCR-artifact alternation: scattered single letters, runs of `.` or `_`
/// (3+), an isolated punctuation mark, a digit-embedded word, or 3+ whitespace chars,
/// with leftmost-first alternative priority.
fn scan_ocr_artifacts(text: &str, mut emit: impl FnMut(usize, usize)) {
    let mut i = 0;
    while i < text.len() {
        let Some(c) = char_at(text, i) else { break };
        let end = if c.is_ascii_alphabetic() {
            if word_before(text, i) {
                None
            } else {
                scattered_chars_end(text, i).or_else(|| malformed_word_end(text, i))
            }
        } else if c == '.' || c == '_' {
            let end = byte_run_end(text, i, c as u8);
            (end - i >= 3).then_some(end)
        } else if c.is_whitespace() {
            let after = i + c.len_utf8();
            let isolated = char_at(text, after)
                .filter(|p| matches!(p, '.' | ',' | ';' | ':' | '!' | '?'))
                .and_then(|_| {
                    let last = char_at(text, after + 1)?;
                    last.is_whitespace().then(|| after + 1 + last.len_utf8())
                });
            isolated.or_else(|| {
                let (end, n) = whitespace_run(text, i);
                (n >= 3).then_some(end)
            })
        } else {
            None
        };
        match end {
            Some(end) => {
                emit(i, end);
                i = end;
            }
            None => i += c.len_utf8(),
        }
    }
}

/// Folds the only non-ASCII characters that Unicode simple case folding maps onto
/// ASCII letters in the phrases below (U+017F and the Kelvin sign), then lowercases.
fn fold(c: char) -> char {
    match c {
        '\u{17F}' => 's',
        '\u{212A}' => 'k',
        c => c.to_ascii_lowercase(),
    }
}

/// End of `lower` matched case-insensitively at `i`.
fn match_ci(text: &str, i: usize, lower: &str) -> Option<usize> {
    let mut end = i;
    let mut chars = text[i..].chars();
    for want in lower.chars() {
        let c = chars.next()?;
        if fold(c) != want {
            return None;
        }
        end += c.len_utf8();
    }
    Some(end)
}

/// End of one or more Unicode decimal digits at `i`.
fn digits_end(text: &str, i: usize) -> Option<usize> {
    let mut end = i;
    for c in text[i..].chars() {
        if !is_decimal_digit(c) {
            break;
        }
        end += c.len_utf8();
    }
    (end > i).then_some(end)
}

/// `\bPAGE \d+ OF \d+` (ASCII case-insensitive) anchored at `i`, without the trailing `\b`.
fn page_x_of_y_end(text: &str, i: usize) -> Option<usize> {
    let j = digits_end(text, match_ci(text, i, "page ")?)?;
    digits_end(text, match_ci(text, j, " of ")?)
}

/// Scans for `\b(?:alt1|alt2|...)\b` where each alternative is a literal phrase
/// matched case-insensitively; spans are reported with leftmost-first semantics.
fn scan_phrases(text: &str, phrases: &[&str], mut emit: impl FnMut(usize, usize)) {
    scan_with(
        text,
        |i| {
            phrases
                .iter()
                .filter_map(|p| match_ci(text, i, p))
                .find(|&end| !word_at(text, end))
        },
        &mut emit,
    );
}

/// Shared driver: at each char boundary whose preceding char is not a word char, ask
/// `end_at` for a match end that also satisfies the trailing boundary.
fn scan_with(text: &str, mut end_at: impl FnMut(usize) -> Option<usize>, emit: &mut impl FnMut(usize, usize)) {
    let mut i = 0;
    while i < text.len() {
        let Some(c) = char_at(text, i) else { break };
        let end = if word_before(text, i) { None } else { end_at(i) };
        match end {
            Some(end) => {
                emit(i, end);
                i = end;
            }
            None => i += c.len_utf8(),
        }
    }
}

const NAV_PHRASES: [&str; 4] = [
    "skip to main content",
    "back to top",
    "main navigation",
    "site navigation",
];

fn scan_nav_words(text: &str, emit: impl FnMut(usize, usize)) {
    scan_phrases(text, &NAV_PHRASES, emit);
}

/// Pagination markers: `Page N of M`, `First/Last/Previous/Next page`, and a whole text
/// that is exactly `N of M`.
fn scan_pagination(text: &str, mut emit: impl FnMut(usize, usize)) {
    const LITERALS: [&str; 4] = ["first page", "last page", "previous page", "next page"];
    scan_with(
        text,
        |i| {
            let ends = page_x_of_y_end(text, i)
                .into_iter()
                .chain(LITERALS.iter().filter_map(|p| match_ci(text, i, p)))
                .chain(
                    (i == 0)
                        .then(|| {
                            let j = digits_end(text, 0)?;
                            digits_end(text, match_ci(text, j, " of ")?).filter(|&e| e == text.len())
                        })
                        .flatten(),
                );
            ends.into_iter().find(|&end| !word_at(text, end))
        },
        &mut emit,
    );
}

fn sum_scanned(text: &str, scan: impl FnOnce(&str, &mut dyn FnMut(usize, usize))) -> usize {
    let mut total = 0;
    scan(text, &mut |s, e| total += e - s);
    total
}

#[inline]
fn sum_match_lengths(text: &str, pattern: &Regex) -> usize {
    pattern.find_iter(text).map(|m| m.len()).sum()
}

/// Count the total byte span of all `<open_tag>...</close_tag>` pairs using `memmem` scanners.
///
/// The text is lowercased once before scanning so the needles can be lowercase-only, matching
/// `<script>`, `<SCRIPT>`, `<Script>`, etc. The byte lengths are measured on the lowercased
/// copy but equal those of the original because the tags are pure ASCII.
///
/// This replaces `(?is)<script[^>]*>.*?</script>` and the equivalent style regex.  Those
/// patterns trigger the `regex_automata` `BoundedBacktracker` on large inputs, causing
/// multi-MiB transient allocations (observed in staging heap profiles at ~1.6 MiB combined).
fn count_tag_bytes(text: &str, open_needle: &[u8], close_needle: &[u8]) -> usize {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    let open_finder = memmem::Finder::new(open_needle);
    let close_finder = memmem::Finder::new(close_needle);

    let mut total = 0usize;
    let mut pos = 0usize;

    while pos < bytes.len() {
        let Some(open_start) = open_finder.find(&bytes[pos..]) else {
            break;
        };
        let open_start = pos + open_start;

        let tag_body_start = match memchr(b'>', &bytes[open_start..]) {
            Some(rel) => open_start + rel + 1,
            None => break,
        };

        let Some(close_rel) = close_finder.find(&bytes[tag_body_start..]) else {
            break;
        };
        let close_end = tag_body_start + close_rel + close_needle.len();

        total += close_end - open_start;
        pos = close_end;
    }

    total
}

/// Score the cleanliness and readability of extracted text on `[0.0, 1.0]`.
///
/// `1.0` is the neutral score for clean prose; penalties (OCR artifacts, embedded
/// script/style noise, navigation chrome) subtract, structural cues (headings,
/// punctuation) add. The result is clamped to `[0.0, 1.0]`. It measures only the
/// text supplied to this function and therefore cannot measure extraction
/// completeness or recall. Callers must inspect extraction warnings separately
/// for known omissions or degraded processing.
///
/// Pass `metadata` as `None` when the caller has no extraction metadata available;
/// the metadata bonus simply isn't applied in that case. Texts shorter than
/// `MIN_TEXT_LENGTH` short-circuit to `0.1` regardless of metadata.
#[cfg_attr(alef, alef(skip))]
pub fn calculate_quality_score(text: &str, metadata: Option<&AHashMap<Cow<'static, str>, serde_json::Value>>) -> f64 {
    if text.is_empty() || text.trim().is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    if text.len() < MIN_TEXT_LENGTH {
        return 0.1;
    }

    // Every penalty/bonus applies regardless of `text.len()`. This used to
    // short-circuit script and navigation penalties for texts at or below
    // `LARGE_TEXT_LENGTH`, so a short HTML-derived fragment full of nav chrome
    // (breadcrumbs, "Skip to main content", pagination) scored as clean prose
    // purely because it was short (#267). ~keep
    let mut score = 1.0;

    let ocr_penalty = calculate_ocr_penalty(text, total_chars);
    let script_penalty = calculate_script_penalty(text, total_chars);
    let nav_penalty = calculate_navigation_penalty(text, total_chars);
    let structure_bonus = calculate_structure_bonus(text);

    score -= ocr_penalty * OCR_PENALTY_WEIGHT;
    score -= script_penalty * SCRIPT_PENALTY_WEIGHT;
    score -= nav_penalty * NAV_PENALTY_WEIGHT;
    score += structure_bonus * STRUCTURE_BONUS_WEIGHT;

    if let Some(metadata) = metadata {
        score += calculate_metadata_bonus(metadata) * METADATA_BONUS_WEIGHT;
    }

    score.clamp(0.0, 1.0)
}

#[inline]
fn calculate_ocr_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    if memmem::find(text.as_bytes(), b"  ").is_none() && memmem::find(text.as_bytes(), b"...").is_none() {
        return 0.0;
    }

    let artifact_chars = sum_scanned(text, |t, f| scan_ocr_artifacts(t, f)) + count_non_table_dash_artifacts(text);

    (artifact_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn count_non_table_dash_artifacts(text: &str) -> usize {
    let mut artifact_count = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        let is_table_separator = trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && trimmed
                .chars()
                .all(|c| c == '|' || c == '-' || c.is_whitespace() || c == ':');

        if !is_table_separator {
            for m in DASH_PATTERN.find_iter(line) {
                artifact_count += m.len();
            }
        }
    }

    artifact_count
}

/// Cap applied to `text` before running `JS_FUNCTION_PATTERN` and `CSS_RULES_PATTERN`.
///
/// Both patterns use bounded character-class repetition (`[^)]*`, `[^}]*`) that still triggers
/// the `regex_automata` `BoundedBacktracker` when the input slice is large. Because these are
/// noise-detection heuristics (not structural parsers), capping at 64 KiB does not meaningfully
/// change scores for real documents.
const JS_CSS_PATTERN_INPUT_CAP: usize = 64 * 1024;

#[inline]
fn calculate_script_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    let bytes = text.as_bytes();
    if memmem::find(bytes, b"function").is_none()
        && memmem::find(bytes, b"<script").is_none()
        && memmem::find(bytes, b"<style").is_none()
        && memmem::find(bytes, b"FUNCTION").is_none()
        && memmem::find(bytes, b"<SCRIPT").is_none()
        && memmem::find(bytes, b"<STYLE").is_none()
    {
        return 0.0;
    }

    let truncated = if text.len() > JS_CSS_PATTERN_INPUT_CAP {
        let mut end = JS_CSS_PATTERN_INPUT_CAP;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    } else {
        text
    };

    let script_chars = sum_match_lengths(truncated, &JS_FUNCTION_PATTERN)
        + sum_match_lengths(truncated, &CSS_RULES_PATTERN)
        + count_tag_bytes(text, b"<script", b"</script>")
        + count_tag_bytes(text, b"<style", b"</style>");

    (script_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_navigation_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    let breadcrumb_chars =
        if memchr(b'>', text.as_bytes()).is_some() || memmem::find(text.as_bytes(), "»".as_bytes()).is_some() {
            sum_match_lengths(text, &BREADCRUMB_PATTERN)
        } else {
            0
        };
    let nav_chars = sum_scanned(text, |t, f| scan_nav_words(t, f))
        + breadcrumb_chars
        + sum_scanned(text, |t, f| scan_pagination(t, f));

    (nav_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_structure_bonus(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let sentence_count = SENTENCE_DETECT.find_iter(text).count() as f64;
    let paragraph_count = memmem::find_iter(text.as_bytes(), b"\n\n").count() as f64 + 1.0;
    let words = text.split_whitespace().count() as f64;

    if words == 0.0 {
        return 0.0;
    }

    let avg_words_per_sentence = words / sentence_count.max(1.0);
    let avg_words_per_paragraph = words / paragraph_count;

    let mut structure_score: f64 = 0.0;

    if (MIN_SENTENCE_WORDS..=MAX_SENTENCE_WORDS).contains(&avg_words_per_sentence) {
        structure_score += 0.3;
    }

    if (MIN_PARAGRAPH_WORDS..=MAX_PARAGRAPH_WORDS).contains(&avg_words_per_paragraph) {
        structure_score += 0.3;
    }

    if paragraph_count > 1.0 {
        structure_score += 0.2;
    }

    if PUNCTUATION_DETECT.is_match(text) {
        structure_score += 0.2;
    }

    structure_score.min(1.0)
}

#[inline]
fn calculate_metadata_bonus(metadata: &AHashMap<Cow<'static, str>, serde_json::Value>) -> f64 {
    const IMPORTANT_FIELDS: &[&str] = &["title", "author", "subject", "description", "keywords"];

    let present_fields = IMPORTANT_FIELDS
        .iter()
        .filter(|&&field| metadata.contains_key(field))
        .count();

    present_fields as f64 / IMPORTANT_FIELDS.len() as f64
}

#[cfg(test)]
static WHITESPACE_NORMALIZE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[ \t\f\v\r\xa0\u{2000}-\u{200b}\u{2028}\u{2029}\u{3000}]+")
        .expect("Whitespace normalization regex pattern is valid and should compile")
});

#[cfg(test)]
pub(crate) fn normalize_spaces(text: &str) -> String {
    if text.is_empty() || text.trim().is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(text.len());

    let mut first = true;
    for paragraph in text.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !first {
            result.push_str("\n\n");
        }
        first = false;

        let cleaned = WHITESPACE_NORMALIZE.replace_all(paragraph, " ");
        let cleaned = NEWLINE_CLEANUP.replace_all(&cleaned, "\n");

        let mut first_line = true;
        for line in cleaned.split('\n') {
            let line = line.trim();
            if !line.is_empty() {
                if !first_line {
                    result.push('\n');
                }
                result.push_str(line);
                first_line = false;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_quality_score_empty_text() {
        assert_eq!(calculate_quality_score("", None), 0.0);
        assert_eq!(calculate_quality_score("   ", None), 0.0);
        assert_eq!(calculate_quality_score("\n\n\n", None), 0.0);
    }

    #[test]
    fn test_calculate_quality_score_short_text() {
        let text = "Hello";
        let score = calculate_quality_score(text, None);
        assert_eq!(score, 0.1);
    }

    #[test]
    fn test_calculate_quality_score_normal_text() {
        let text =
            "This is a normal sentence with proper punctuation. It has multiple sentences. And proper structure.";
        let score = calculate_quality_score(text, None);
        assert!(score > 0.5);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_normalize_spaces_empty() {
        assert_eq!(normalize_spaces(""), "");
        assert_eq!(normalize_spaces("   "), "");
    }

    #[test]
    fn test_normalize_spaces_single_paragraph() {
        let text = "This  is   a   test";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "This is a test");
    }

    #[test]
    fn test_calculate_quality_score_with_metadata() {
        let text = "This is a normal text with proper structure.";
        let mut metadata: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        metadata.insert(Cow::Borrowed("title"), serde_json::json!("Test Title"));
        metadata.insert(Cow::Borrowed("author"), serde_json::json!("Test Author"));

        let score = calculate_quality_score(text, Some(&metadata));
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_ocr_penalty_clean_text() {
        let text = "This is clean text without artifacts";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_ocr_penalty_with_artifacts() {
        let text = "Text with  excessive   spaces and ....... dots";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
        assert!(penalty <= 1.0);
    }

    #[test]
    fn test_calculate_script_penalty_clean_text() {
        let text = "This is clean text without scripts";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_script_penalty_with_js() {
        let text = "function test() { return 42; }";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_calculate_navigation_penalty_clean_text() {
        let text = "This is clean text without navigation";
        let penalty = calculate_navigation_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_navigation_penalty_with_nav() {
        let text = "Skip to main content and Back to top links everywhere";
        let penalty = calculate_navigation_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_calculate_structure_bonus_empty() {
        assert_eq!(calculate_structure_bonus(""), 0.0);
    }

    #[test]
    fn test_calculate_structure_bonus_well_structured() {
        let text = "This is a sentence. This is another sentence.\n\nNew paragraph here. More content.";
        let bonus = calculate_structure_bonus(text);
        assert!(bonus > 0.0);
        assert!(bonus <= 1.0);
    }

    #[test]
    fn test_calculate_metadata_bonus_empty() {
        let metadata: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        let bonus = calculate_metadata_bonus(&metadata);
        assert_eq!(bonus, 0.0);
    }

    #[test]
    fn test_calculate_metadata_bonus_full() {
        let mut metadata: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        metadata.insert(Cow::Borrowed("title"), serde_json::json!("Title"));
        metadata.insert(Cow::Borrowed("author"), serde_json::json!("Author"));
        metadata.insert(Cow::Borrowed("subject"), serde_json::json!("Subject"));
        metadata.insert(Cow::Borrowed("description"), serde_json::json!("Description"));
        metadata.insert(Cow::Borrowed("keywords"), serde_json::json!("Keywords"));

        let bonus = calculate_metadata_bonus(&metadata);
        assert_eq!(bonus, 1.0);
    }

    #[test]
    fn test_normalize_spaces_multiple_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let normalized = normalize_spaces(text);
        assert!(normalized.contains("\n\n"));
    }

    #[test]
    fn test_normalize_spaces_preserves_paragraphs() {
        let text = "Para 1\n\n\n\nPara 2";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "Para 1\n\nPara 2");
    }

    #[test]
    fn test_count_non_table_dash_artifacts() {
        let text = "Some text --- with dashes";
        let count = count_non_table_dash_artifacts(text);
        assert!(count > 0);
    }

    #[test]
    fn test_count_non_table_dash_artifacts_preserves_tables() {
        let text = "| Header |\n|--------|\n| Data   |";
        let count = count_non_table_dash_artifacts(text);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_quality_score_large_text_with_ocr_issues() {
        let text = "a".repeat(2000) + "   " + &"b".repeat(2000);
        let score = calculate_quality_score(&text, None);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_quality_score_clamped_to_range() {
        let perfect_text = "This is perfect text. ".repeat(100);
        let score = calculate_quality_score(&perfect_text, None);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_quality_constants() {
        assert_eq!(MIN_TEXT_LENGTH, 10);
        assert_eq!(OCR_PENALTY_WEIGHT, 0.3);
    }

    #[test]
    fn should_apply_navigation_penalty_to_short_text() {
        // Below the removed `LARGE_TEXT_LENGTH` threshold, navigation chrome
        // used to be scored as if it were clean prose (#267). This text is
        // short (well under 1000 bytes) and dominated by nav chrome, so the
        // navigation penalty must pull the score down from the neutral 1.0
        // baseline instead of leaving it unpenalized.
        let text = "Skip to main content. Back to top. Home > Products > Widgets. Page 1 of 12.";
        assert!(text.len() < 1000, "fixture must exercise the short-text path");

        let score = calculate_quality_score(text, None);
        let nav_penalty = calculate_navigation_penalty(text, text.len() as f64);

        assert!(
            nav_penalty > 0.0,
            "fixture must actually trigger the navigation pattern"
        );
        assert!(
            score < 1.0,
            "navigation-heavy short text must be penalized, got score {score}"
        );
    }

    #[test]
    fn should_apply_script_penalty_to_short_text() {
        // Same defect as above but for the script/style penalty (#267).
        let text = "function init() { doStuff(); } Welcome to our site!";
        assert!(text.len() < 1000, "fixture must exercise the short-text path");

        let score = calculate_quality_score(text, None);
        let script_penalty = calculate_script_penalty(text, text.len() as f64);

        assert!(script_penalty > 0.0, "fixture must actually trigger the script pattern");
        assert!(
            score < 1.0,
            "script-heavy short text must be penalized, got score {score}"
        );
    }
}

#[cfg(test)]
mod scanner_equivalence_tests {
    use super::*;

    static OLD_OCR: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
        \b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b |
        [.]{3,}|[_]{3,} |
        \s[.,;:!?]\s |
        \b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b |
        \s{3,}
    ",
        )
        .unwrap()
    });
    static OLD_NAV: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:Skip to main content|Back to top|Main navigation|Site navigation)\b").unwrap()
    });
    static OLD_PAGINATION: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:Page \d+ of \d+|First page|Last page|Previous page|Next page|^\d+ of \d+$)\b").unwrap()
    });
    static WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\A\s\z").unwrap());

    fn old_spans(re: &Regex, text: &str) -> Vec<(usize, usize)> {
        re.find_iter(text).map(|m| (m.start(), m.end())).collect()
    }

    fn new_spans(scan: impl FnOnce(&str, &mut dyn FnMut(usize, usize)), text: &str) -> Vec<(usize, usize)> {
        let mut spans = Vec::new();
        scan(text, &mut |s, e| spans.push((s, e)));
        spans
    }

    fn assert_equivalent(text: &str) {
        assert_eq!(
            new_spans(|t, f| scan_ocr_artifacts(t, f), text),
            old_spans(&OLD_OCR, text),
            "ocr artifacts diverge on {text:?}"
        );
        assert_eq!(
            new_spans(|t, f| scan_nav_words(t, f), text),
            old_spans(&OLD_NAV, text),
            "nav words diverge on {text:?}"
        );
        assert_eq!(
            new_spans(|t, f| scan_pagination(t, f), text),
            old_spans(&OLD_PAGINATION, text),
            "pagination diverges on {text:?}"
        );
    }

    #[test]
    fn word_and_digit_and_space_classifiers_match_regex_for_every_char() {
        let word = Regex::new(r"\A\w\z").unwrap();
        let digit = Regex::new(r"\A\d\z").unwrap();
        let mut buf = [0u8; 4];
        for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
            let s = c.encode_utf8(&mut buf);
            assert_eq!(is_word_char(c), word.is_match(s), "\\w differs for U+{:04X}", c as u32);
            assert_eq!(
                is_decimal_digit(c),
                digit.is_match(s),
                "\\d differs for U+{:04X}",
                c as u32
            );
            assert_eq!(
                c.is_whitespace(),
                WHITESPACE.is_match(s),
                "\\s differs for U+{:04X}",
                c as u32
            );
        }
    }

    #[test]
    fn handpicked_strings_match_the_regexes() {
        let cases = [
            "",
            "a  b  c",
            "a  b  c é",
            "éa  b  c",
            "a\t\tb  c\u{a0}\u{a0}d",
            "é1",
            "a1b é1b a1bé a1b1 ab12cd9 x1y_ x1y-",
            "中a1b中 a1b中 中a1b",
            "...  ___ .. __ ....",
            "a . b ; c ,d  !  ?",
            "x\u{a0}.\u{a0}y \u{3000}.\u{3000}",
            "§ 12 — Page 3 of 14 §",
            "page 3 of 14é Page ٣ of ١٤ page 3 of",
            "12 of 34",
            "12 of 34\n",
            " 12 of 34",
            "٣ of ٤",
            "First page. Last Page Previous PAGE next page éNext page Next pageé",
            "skip to main content, BACK TO TOP, ſkip to main content, \u{212a}ip to main content",
            "Skip to main content_ Site navigation1 xSite navigation Main  navigation",
            "ſite navigation Site navigation",
            "tab\t\t\tseparated \n\n\n lines   with    gaps",
            "Home > Docs » API",
            "ǅ1a a1ǅ ᾳ1b",
            "a  b  cé a  b  c_ a  b  c1",
        ];
        for text in cases {
            assert_equivalent(text);
        }
    }

    #[test]
    fn generated_strings_match_the_regexes() {
        const TOKENS: &[&str] = &[
            "a",
            "b",
            "Z",
            "x",
            "1",
            "9",
            "٣",
            " ",
            "  ",
            "\t",
            "\n",
            "\u{a0}",
            "\u{3000}",
            ".",
            "...",
            ",",
            ";",
            "!",
            "_",
            "-",
            "é",
            "中",
            "§",
            "—",
            "ſ",
            "\u{212a}",
            "k",
            "s",
            "»",
            ">",
            "Page ",
            "page ",
            " of ",
            "of",
            "First ",
            "Last ",
            "Next ",
            "Previous ",
            "Skip to main content",
            "Back to top",
            "Main navigation",
            "Site navigation",
            "skip",
            "site",
            " page",
            "Page 3 of 4",
            "12 of 34",
        ];
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as usize
        };
        for _ in 0..30_000 {
            let len = 1 + next() % 12;
            let text: String = (0..len).map(|_| TOKENS[next() % TOKENS.len()]).collect();
            assert_equivalent(&text);
        }
    }

    #[test]
    fn breadcrumb_prefilter_never_hides_a_match() {
        for text in ["Home > Docs", "a » b » c", "plain text with no markers"] {
            let has_marker = text.contains('>') || text.contains('»');
            assert!(has_marker || BREADCRUMB_PATTERN.find(text).is_none());
        }
    }
}
