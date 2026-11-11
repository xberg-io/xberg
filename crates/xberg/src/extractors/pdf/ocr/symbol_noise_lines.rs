//! Removal of symbol-noise lines from a page OCR already judged to be recognition noise (#2312).
//!
//! Tesseract reads a photograph as a long run of one- and two-letter fragments and stray
//! symbols, one per line. [`super::scoring::is_ocr_recognition_noise`] flags such a page, but
//! with `discard_suspected_ocr_noise` off the text is retained whole. This module prunes only
//! the lines made of symbols and fragments, and runs after the page verdict, so every
//! backend, table and quality decision still sees the unpruned text. A line is pruned only
//! when it is dominated by tokens that are not plausible text; any line that reads as words,
//! a list label or an equation is kept. Two 3-letter lowercase tokens together count as
//! plausible (`mal cad` is kept); a lone 3-letter token needs a capital with a closing mark or
//! a letter outside ASCII, or an alphabet run (`Yes.`, `\u{c9}t\u{e9}`, `ABC`). A bare
//! capitalized one such as `Tee` is pruned, but it does not count toward the page decision. The
//! page decision is made once by
//! [`page_is_dominated_by_noise_lines`]; the pruners below apply it without re-deciding, so a
//! page's text and its paragraphs always lose the same lines.

use crate::pdf::structure::types::PdfParagraph;

/// Fewest non-blank lines a page needs before its line mix says anything.
const MIN_LINES: usize = 8;
/// A Latin token with at least this many letters is word-shaped.
const MIN_WORD_LETTERS: usize = 3;
/// A line made of one token needs one more letter: nothing around it vouches for a bare trigram.
const MIN_LONE_WORD_LETTERS: usize = 4;
/// Short function words of English and the other Latin-script languages Tesseract commonly reads,
/// which let a short line such as `I am`, `Go to` or `du Louvre` count as text.
const FUNCTION_WORDS: [&str; 43] = [
    "a", "am", "an", "as", "at", "be", "by", "do", "go", "he", "i", "if", "in", "is", "it", "me", "my", "no", "of",
    "on", "or", "so", "to", "up", "us", "we", "de", "la", "le", "les", "du", "des", "un", "une", "et", "en", "au",
    "aux", "el", "los", "las", "der", "die",
];
/// Operators that, between two operands, make a line an equation or a sum. `-`, `*` and `/` are
/// left out: Tesseract draws them from rules and specks, giving `fragment - fragment` noise.
const OPERATORS: [char; 9] = [
    '=', '+', '<', '>', '\u{2264}', '\u{2265}', '\u{d7}', '\u{f7}', '\u{2260}',
];
/// Single letters that look like a vertical stroke or a ring and are what texture reads as, so
/// they do not count as an equation variable (`i - l`).
const STROKE_LETTERS: [char; 6] = ['i', 'l', 'j', 'o', 'I', 'O'];
/// Contraction endings (`I'm`, `it's`, `don't`, `we'll`, `they're`, `I've`, `he'd`).
const CONTRACTION_ENDINGS: [&str; 7] = ["m", "s", "t", "d", "ll", "re", "ve"];

/// Scripts written without spaces between words, where one character can be a whole word.
fn is_unspaced_script(c: char) -> bool {
    matches!(u32::from(c),
        0x0E00..=0x0EFF
        | 0x1000..=0x109F
        | 0x1100..=0x11FF
        | 0x1780..=0x17FF
        | 0x2E80..=0x2FDF
        | 0x3040..=0x30FF
        | 0x3130..=0x318F
        | 0x31F0..=0x31FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xAC00..=0xD7AF
        | 0xF900..=0xFAFF
        | 0x20000..=0x2FA1F)
}

/// Alphabetic characters outside the Latin script, which Tesseract does not hallucinate from texture.
fn is_non_latin_letter(c: char) -> bool {
    let code = u32::from(c);
    c.is_alphabetic() && code >= 0x0250 && !(0x1E00..=0x1EFF).contains(&code)
}

/// The token without surrounding punctuation (`"Go`, `to.`, `(am)`).
fn core(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_alphanumeric())
}

fn letter_count(token: &str) -> usize {
    token.chars().filter(|c| c.is_alphabetic()).count()
}

fn is_function_word(token: &str) -> bool {
    FUNCTION_WORDS.iter().any(|word| word.eq_ignore_ascii_case(token))
}

fn is_contraction(token: &str) -> bool {
    let Some((head, tail)) = token.split_once(['\'', '\u{2019}']) else {
        return false;
    };
    !head.is_empty()
        && head.chars().all(char::is_alphabetic)
        && CONTRACTION_ENDINGS
            .iter()
            .any(|ending| ending.eq_ignore_ascii_case(tail))
}

fn is_word_like(token: &str, min_letters: usize) -> bool {
    let token = core(token);
    token.chars().any(|c| is_unspaced_script(c) || is_non_latin_letter(c))
        || letter_count(token) >= min_letters
        || is_contraction(token)
}

/// A token of letters only, allowing an interior apostrophe, hyphen or dot (`don't`, `e-mail`, `U.S.A`).
fn is_alphabetic_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_alphabetic() || matches!(c, '\'' | '\u{2019}' | '-' | '.'))
}

fn is_capitalized(token: &str) -> bool {
    token.chars().next().is_some_and(char::is_uppercase)
}

/// A run of consecutive alphabet letters (`ABC`, `pqr`), which texture noise does not produce.
fn is_alphabet_run(token: &str) -> bool {
    let letters: Vec<u32> = token.chars().map(|c| u32::from(c.to_ascii_lowercase())).collect();
    token.is_ascii()
        && letters.len() >= MIN_WORD_LETTERS
        && token.chars().all(|c| c.is_ascii_alphabetic())
        && letters.windows(2).all(|pair| pair[1] == pair[0] + 1)
}

/// A list label such as `b)` or `c.`: one letter followed by a closing mark.
fn is_list_label(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(char::is_alphabetic) && matches!(chars.as_str(), ")" | "." | ":")
}

/// A token that reads as text inside a multi-token line: a word of 3 or more letters, a function
/// word, a capitalized initial or abbreviation of 1 or 2 letters (`X`, `Mr`, `Al`), a list label,
/// a contraction, or a letter of a script Tesseract does not hallucinate.
fn is_plausible_token(token: &str) -> bool {
    if is_list_label(token) {
        return true;
    }
    let stem = core(token);
    stem.chars().any(|c| is_unspaced_script(c) || is_non_latin_letter(c))
        || is_contraction(stem)
        || (is_alphabetic_token(stem)
            && (letter_count(stem) >= MIN_WORD_LETTERS
                || is_function_word(stem)
                || (is_capitalized(stem) && letter_count(stem) <= 2)))
}

/// A lone token has nothing around it to vouch for it, so it needs 4 letters, an alphabet run at 3
/// (`ABC`, `pqr`), a capital and a letter outside ASCII at 3 (`\u{c9}t\u{e9}`), or a capital and a
/// closing mark at 2 or 3 (`No.`, `Yes.`). A bare `Tee` or `CAS` has the shape of texture noise.
fn is_plausible_lone_token(token: &str) -> bool {
    let stem = core(token);
    let marked = is_capitalized(stem) && token.ends_with(['.', '!', '?']);
    is_word_like(token, MIN_LONE_WORD_LETTERS)
        || (is_alphabetic_token(stem)
            && match letter_count(stem) {
                MIN_WORD_LETTERS => marked || is_alphabet_run(stem) || (is_capitalized(stem) && !stem.is_ascii()),
                2 => marked,
                _ => false,
            })
}

/// Whether a line reads as text: a lone plausible token; a line of which at least half the tokens
/// are words of 3 or more letters (`Annual report q z`, `were ee`); or a line of which at least two
/// thirds of the tokens are plausible and which holds a word of 3 or more letters or two function
/// words, one of them of 2 or more letters (`a a` and `we SY` are not text).
fn is_plausible_text(tokens: &[&str]) -> bool {
    if let [token] = tokens {
        return is_plausible_lone_token(token);
    }
    let words = tokens
        .iter()
        .filter(|token| is_word_like(token, MIN_WORD_LETTERS))
        .count();
    if words * 2 >= tokens.len() {
        return true;
    }
    let plausible = tokens.iter().filter(|token| is_plausible_token(token)).count();
    let function_words: Vec<&str> = tokens
        .iter()
        .map(|token| core(token))
        .filter(|stem| is_function_word(stem))
        .collect();
    plausible * 3 >= tokens.len() * 2
        && (tokens.iter().any(|token| is_word_like(token, MIN_WORD_LETTERS))
            || (function_words.len() >= 2 && function_words.iter().any(|word| word.chars().count() >= 2)))
}

/// A token with a digit and a stray quotation mark or a trailing section mark (`\u{201c}9`,
/// `\u{ab}5%`, `5\u{a7}`), which is what a photograph's texture reads as. A quoted number closes
/// its quote (`\u{201c}9\u{201d}`, `\u{2018}9\u{2019}`) and `\u{a7}12` is a section reference.
fn is_junk_digit_token(token: &str) -> bool {
    let balance = |openers: &[char], closers: &[char]| {
        token.chars().filter(|c| openers.contains(c)).count() != token.chars().filter(|c| closers.contains(c)).count()
    };
    token.chars().any(|c| c.is_numeric())
        && (token.ends_with('\u{a7}')
            || balance(&['\u{201c}', '\u{ab}', '\u{2018}'], &['\u{201d}', '\u{bb}', '\u{2019}'])
            || token.matches('"').count() % 2 == 1)
}

/// A line with a digit is noise only when every token that carries a digit is junk-shaped and no
/// other token is plausible text, so `"5 items`, `Siehe \u{a7}12` and `\u{201c}3 days\u{201d}` stay.
fn is_junk_digit_line(tokens: &[&str]) -> bool {
    tokens.iter().all(|token| {
        if token.chars().any(|c| c.is_numeric()) {
            is_junk_digit_token(token)
        } else {
            !is_plausible_token(token)
        }
    })
}

/// An operand of an equation: a word of 3 or more letters, one letter that is not a stroke-like
/// glyph (`x`, `y`, `a`, `b`), or a call such as `f(x)`.
fn is_operand(segment: &str) -> bool {
    if let Some((name, rest)) = segment.split_once('(')
        && let Some(argument) = rest.strip_suffix(')')
    {
        return !name.is_empty()
            && name.chars().all(char::is_alphabetic)
            && !argument.is_empty()
            && argument.chars().all(char::is_alphanumeric);
    }
    let token = core(segment);
    token.chars().all(char::is_alphabetic)
        && (letter_count(token) >= MIN_WORD_LETTERS
            || (token.chars().count() == 1 && token.chars().all(|c| !STROKE_LETTERS.contains(&c))))
}

/// An equation or sum, spaced or not (`x = y`, `x=y`, `a=`, `f(x) = x`): an operator with at
/// least one operand beside it and nothing but operands on either side.
fn is_equation(line: &str) -> bool {
    if !line.contains(OPERATORS) {
        return false;
    }
    let mut operands = line
        .split(OPERATORS)
        .map(str::trim)
        .filter(|segment| !segment.is_empty());
    let first = operands.next();
    first.is_some_and(is_operand) && operands.all(is_operand)
}

/// A line is symbol noise when it holds no character of an unspaced script, no equation
/// (`x = y`) and its tokens are not plausible text, or when it has a digit and every digit
/// token is a digit with a stray quotation or section mark and no other token is plausible
/// text. Any other line with a digit is kept.
fn is_symbol_noise_line(line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty() || line.chars().any(is_unspaced_script) {
        return false;
    }
    if line.chars().any(|c| c.is_numeric()) {
        return is_junk_digit_line(&tokens);
    }
    !(is_equation(line) || is_plausible_text(&tokens))
}

/// Whether a line is one capitalized 3-letter token, such as `Tee` or `CAS`. A real short word has
/// the same shape, so the page decision does not count it as noise.
fn is_lone_capitalized_trigram(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    let stem = tokens.next().map(core).unwrap_or_default();
    tokens.next().is_none()
        && is_alphabetic_token(stem)
        && letter_count(stem) == MIN_WORD_LETTERS
        && is_capitalized(stem)
}

/// Whether symbol-noise lines are at least half of a page's non-blank lines, on a page with at
/// least [`MIN_LINES`] of them. A lone capitalized trigram is pruned from a noise page but does not
/// count toward this decision. This is the one page decision; callers apply it to every rendering
/// of the page.
pub(super) fn page_is_dominated_by_noise_lines<'a>(lines: impl Iterator<Item = &'a str>) -> bool {
    let (mut total, mut noise) = (0usize, 0usize);
    for line in lines.filter(|line| !line.trim().is_empty()) {
        total += 1;
        noise += usize::from(is_symbol_noise_line(line) && !is_lone_capitalized_trigram(line));
    }
    total >= MIN_LINES && noise * 2 >= total
}

fn collapse_blank_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0usize;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out.trim().to_string()
}

/// The text without its symbol-noise lines and the number of lines removed. Does not decide
/// whether the page qualifies.
pub(super) fn prune_text(text: &str) -> (String, usize) {
    let removed = text.lines().filter(|line| is_symbol_noise_line(line)).count();
    let kept = text
        .lines()
        .filter(|line| !is_symbol_noise_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    (collapse_blank_runs(&kept), removed)
}

fn segment_line_text(line: &crate::pdf::structure::types::PdfLine) -> String {
    line.segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Removes the same lines from a page's structured paragraphs. Does not decide whether the page
/// qualifies. A paragraph holding its text keeps the lines of that text; otherwise its own
/// `lines` are filtered one for one.
pub(super) fn prune_paragraphs(paragraphs: &mut Vec<PdfParagraph>) {
    paragraphs.retain_mut(|paragraph| {
        if !paragraph.text.is_empty() {
            let (text, removed) = prune_text(&paragraph.text);
            if removed == 0 {
                return true;
            }
            paragraph.word_count = text.split_whitespace().count();
            paragraph.text = text;
            return !paragraph.text.trim().is_empty();
        }
        let before = paragraph.lines.len();
        paragraph
            .lines
            .retain(|line| !is_symbol_noise_line(&segment_line_text(line)));
        before == paragraph.lines.len() || !paragraph.lines.is_empty()
    });
}

/// Rebuilds an element's text without its noise lines and moves each annotation that sits
/// inside one surviving line to that line's new offset. An annotation that crosses a line
/// boundary is dropped, even between two surviving lines: its byte range has no single new
/// offset once the lines between or around it are gone.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn prune_element_text(element: &mut crate::types::internal::InternalElement) -> bool {
    let mut kept = String::with_capacity(element.text.len());
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    let (mut offset, mut removed) = (0usize, false);
    for line in element.text.split('\n') {
        let end = offset + line.len();
        if is_symbol_noise_line(line) {
            removed = true;
        } else {
            if !kept.is_empty() {
                kept.push('\n');
            }
            spans.push((offset, end, kept.len()));
            kept.push_str(line);
        }
        offset = end + 1;
    }
    if !removed {
        return true;
    }
    for annotation in &mut element.annotations {
        let (start, end) = (annotation.start as usize, annotation.end as usize);
        if let Some(&(from, _, to)) = spans
            .iter()
            .find(|(from, line_end, _)| *from <= start && end <= *line_end)
        {
            annotation.start = (start - from + to) as u32;
            annotation.end = (end - from + to) as u32;
        } else {
            annotation.end = annotation.start;
        }
    }
    element
        .annotations
        .retain(|annotation| annotation.start < annotation.end);
    element.text = kept;
    !element.text.trim().is_empty()
}

/// Prunes a standalone image's OCR text and its hOCR element tree together when the page text
/// is both recognition noise and dominated by symbol-noise lines. Returns the number of lines
/// removed. Pages holding a table are left alone.
#[cfg(all(feature = "ocr", feature = "pdf"))]
pub(crate) fn prune_standalone_image_noise(
    content: &mut String,
    document: Option<&mut crate::types::internal::InternalDocument>,
    has_tables: bool,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> usize {
    if !thresholds.prune_symbol_noise_lines
        || has_tables
        || !super::scoring::is_ocr_recognition_noise(content, thresholds)
        || !page_is_dominated_by_noise_lines(content.lines())
    {
        return 0;
    }
    let (pruned, removed) = prune_text(content);
    *content = pruned;
    if let Some(document) = document {
        prune_document_elements(document);
    }
    removed
}

/// Whether a document's text elements can be pruned: relationships refer to elements by
/// position, so removing one would leave them pointing at the wrong element.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn elements_are_prunable(document: &crate::types::internal::InternalDocument) -> bool {
    document.relationships.is_empty()
}

/// Prunes the text elements of a document, unless [`elements_are_prunable`] is false.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn prune_document_elements(document: &mut crate::types::internal::InternalDocument) {
    use crate::types::internal::ElementKind;

    if !elements_are_prunable(document) {
        return;
    }
    document.elements.retain_mut(|element| {
        !matches!(
            element.kind,
            ElementKind::OcrText { .. }
                | ElementKind::Title
                | ElementKind::Heading { .. }
                | ElementKind::Paragraph
                | ElementKind::ListItem { .. }
        ) || prune_element_text(element)
    });
}

/// The warning that names a pruned page and how many lines it lost.
pub(crate) fn pruned_lines_warning(page_number: usize, removed: usize) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: format!(
            "Page {page_number}: removed {removed} symbol-noise line(s) from suspected OCR recognition noise; \
             set prune_symbol_noise_lines to false to keep them."
        )
        .into(),
    }
}

#[cfg(test)]
#[path = "symbol_noise_lines_tests.rs"]
mod tests;
