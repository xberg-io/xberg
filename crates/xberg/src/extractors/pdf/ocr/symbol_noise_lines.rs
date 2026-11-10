//! Removal of symbol-noise lines from a page OCR already judged to be recognition noise (#2312).
//!
//! Tesseract reads a photograph as a long run of one- and two-letter fragments and stray
//! symbols, one per line. [`super::scoring::is_ocr_recognition_noise`] flags such a page, but
//! with `discard_suspected_ocr_noise` off the text is retained whole. This module prunes only
//! the lines that carry no word-shaped token, and runs after the page verdict, so every
//! backend, table and quality decision still sees the unpruned text. The page decision is
//! made once by [`page_is_dominated_by_noise_lines`]; the pruners below apply it without
//! re-deciding, so a page's text and its paragraphs always lose the same lines.

use crate::pdf::structure::types::PdfParagraph;

/// Fewest non-blank lines a page needs before its line mix says anything.
const MIN_LINES: usize = 8;
/// A Latin token with at least this many letters is word-shaped.
const MIN_WORD_LETTERS: usize = 3;
/// A line made of one token needs one more letter: nothing around it vouches for a bare trigram.
const MIN_LONE_WORD_LETTERS: usize = 4;
/// English function words that let a short line such as `I am` or `Go to` count as a phrase.
/// English-only: the same short words of another Latin-script language (`de la`) are pruned.
const FUNCTION_WORDS: [&str; 26] = [
    "a", "am", "an", "as", "at", "be", "by", "do", "go", "he", "i", "if", "in", "is", "it", "me", "my", "no", "of",
    "on", "or", "so", "to", "up", "us", "we",
];
/// Operators that, between two operands, make a line an equation or a sum. `-`, `*` and `/` are
/// left out: Tesseract draws them from rules and specks, giving `fragment - fragment` noise.
const OPERATORS: [&str; 9] = [
    "=", "+", "<", ">", "\u{2264}", "\u{2265}", "\u{d7}", "\u{f7}", "\u{2260}",
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

fn has_vowel(token: &str) -> bool {
    token.chars().any(|c| {
        c.to_lowercase().any(|lower| {
            "aeiouy\u{e0}\u{e1}\u{e2}\u{e3}\u{e4}\u{e5}\u{e6}\u{e8}\u{e9}\u{ea}\u{eb}\u{ec}\u{ed}\u{ee}\u{ef}\u{f2}\u{f3}\u{f4}\u{f5}\u{f6}\u{f8}\u{f9}\u{fa}\u{fb}\u{fc}\u{fd}\u{ff}\u{153}".contains(lower)
        })
    })
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

/// An English phrase: two or more purely alphabetic tokens, each a function word or a word of 3 or
/// more letters with a vowel, and at least one of them a function word of 2 or more letters. The
/// vowel and length conditions keep `fragment word` junk such as `ee so` or `rs or` out.
fn is_english_phrase(tokens: &[&str]) -> bool {
    let cores: Vec<&str> = tokens.iter().map(|token| core(token)).collect();
    cores.len() >= 2
        && cores.iter().all(|c| {
            c.chars().all(char::is_alphabetic)
                && (is_function_word(c) || (letter_count(c) >= MIN_WORD_LETTERS && has_vowel(c)))
        })
        && cores.iter().any(|c| c.chars().count() >= 2 && is_function_word(c))
}

/// An operand of an equation: a digit-bearing token, a word of 3 or more letters, or one letter
/// that is not a stroke-like glyph (`x`, `y`, `a`, `b`).
fn is_operand(token: &str) -> bool {
    let token = core(token);
    let mut letters = token.chars().filter(|c| c.is_alphabetic());
    token.chars().any(|c| c.is_numeric())
        || letter_count(token) >= MIN_WORD_LETTERS
        || (token.chars().count() == 1 && letters.next().is_some_and(|c| !STROKE_LETTERS.contains(&c)))
}

fn has_operator_between_operands(tokens: &[&str]) -> bool {
    tokens
        .windows(3)
        .any(|w| OPERATORS.contains(&w[1]) && is_operand(w[0]) && is_operand(w[2]))
}

/// A line is symbol noise when fewer than half its tokens are word-shaped and it holds no digit,
/// no character of an unspaced script, no equation (`x = y`) and is not an English phrase.
fn is_symbol_noise_line(line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty()
        || line.chars().any(|c| c.is_numeric() || is_unspaced_script(c))
        || has_operator_between_operands(&tokens)
        || is_english_phrase(&tokens)
    {
        return false;
    }
    let min_letters = if tokens.len() == 1 {
        MIN_LONE_WORD_LETTERS
    } else {
        MIN_WORD_LETTERS
    };
    let words = tokens.iter().filter(|token| is_word_like(token, min_letters)).count();
    words * 2 < tokens.len()
}

/// Whether symbol-noise lines are at least half of a page's non-blank lines, on a page with at
/// least [`MIN_LINES`] of them. This is the one page decision; callers apply it to every
/// rendering of the page.
pub(super) fn page_is_dominated_by_noise_lines<'a>(lines: impl Iterator<Item = &'a str>) -> bool {
    let (mut total, mut noise) = (0usize, 0usize);
    for line in lines.filter(|line| !line.trim().is_empty()) {
        total += 1;
        noise += usize::from(is_symbol_noise_line(line));
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
#[cfg(all(feature = "ocr", feature = "pdf"))]
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
    use crate::types::internal::ElementKind;

    if !thresholds.prune_symbol_noise_lines
        || has_tables
        || !super::scoring::is_ocr_recognition_noise(content, thresholds)
        || !page_is_dominated_by_noise_lines(content.lines())
    {
        return 0;
    }
    let (pruned, removed) = prune_text(content);
    *content = pruned;
    if let Some(document) = document
        && document.relationships.is_empty()
    {
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
    removed
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
mod tests {
    use super::*;

    fn noise_filler(count: usize) -> Vec<&'static str> {
        ["a", "oe", "# =", "ae", "\"", "ee", "=|", "rs", "~", "ie"]
            .into_iter()
            .cycle()
            .take(count)
            .collect()
    }

    fn page(real: &[&str], filler: usize) -> String {
        let mut lines: Vec<&str> = real.to_vec();
        lines.extend(noise_filler(filler));
        lines.join("\n\n")
    }

    fn pruned(real: &[&str], filler: usize) -> String {
        let text = page(real, filler);
        assert!(
            page_is_dominated_by_noise_lines(text.lines()),
            "gate must pass: {text:?}"
        );
        prune_text(&text).0
    }

    fn assert_kept_exactly(real: &[&str]) {
        assert_eq!(pruned(real, 12), real.join("\n\n"), "{real:?}");
    }

    #[test]
    fn keeps_word_shaped_lines_numbers_and_titles() {
        for line in [
            "SECTION TITLE",
            "Chapter 1",
            "Page 12",
            "99",
            "NORTH GATE 42",
            "Figure 3 sunset",
            "$5",
            "1.",
        ] {
            assert!(!is_symbol_noise_line(line), "{line}");
        }
    }

    #[test]
    fn flags_fragment_and_symbol_lines() {
        for line in [
            "a",
            "mal",
            "# ae",
            "ef \"ae Car Sa % ey Bo ary \"dl",
            "~",
            "I",
            "ie ee",
            "No.",
        ] {
            assert!(is_symbol_noise_line(line), "{line}");
        }
    }

    #[test]
    fn keeps_cjk_kana_hangul_and_thai_lines_of_two_or_three_characters() {
        assert_kept_exactly(&[
            "\u{5546}\u{54c1}",
            "\u{5408}\u{8a08}\u{984d}",
            "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}",
            "\u{c548}\u{b155}",
            "\u{0e2a}\u{0e27}\u{0e31}\u{0e2a}\u{0e14}\u{0e35}",
        ]);
    }

    #[test]
    fn keeps_arabic_and_cyrillic_lines() {
        assert_kept_exactly(&[
            "\u{645}\u{631}\u{62d}\u{628}\u{627}",
            "\u{641}\u{64a} \u{627}\u{644}",
            "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}",
            "\u{434}\u{430} \u{43d}\u{435}\u{442}",
            "\u{44f}",
        ]);
    }

    #[test]
    fn keeps_short_codes_and_amounts() {
        assert_kept_exactly(&["A1", "B2", "$5", "1.", "7"]);
    }

    #[test]
    fn keeps_equations_between_variables_and_sums() {
        assert_kept_exactly(&["x = y", "a + b", "n < m", "total = cost"]);
    }

    #[test]
    fn accented_latin_vowels_count_as_vowels() {
        for word in ["f\u{ff}r", "Gr\u{fc}n", "\u{c9}t\u{e9}", "p\u{e5}st", "s\u{153}ur"] {
            assert!(has_vowel(word), "{word}");
        }
        assert!(!has_vowel("pst"));
        assert!(
            !is_symbol_noise_line("go f\u{ff}r"),
            "a vowel-bearing accented word makes the phrase"
        );
    }

    #[test]
    fn prunes_a_three_token_non_english_line_on_a_gated_page() {
        assert!(is_symbol_noise_line("de la gare"));
        let text = page(&["Go to."], 12) + "\n\nde la gare";
        assert_eq!(prune_text(&text).0, "Go to.");
    }

    #[test]
    fn prunes_a_single_token_answer_and_an_unspaced_equation() {
        for line in ["No.", "Yes", "Qty", "x=y"] {
            assert!(is_symbol_noise_line(line), "{line}");
        }
    }

    #[test]
    fn prunes_fragment_operator_fragment_lines() {
        for line in ["ae - ee", "i - l", "oe = ie", "a * e", "= -", "ee + oe"] {
            assert!(is_symbol_noise_line(line), "{line}");
        }
        let text = page(&["x = y"], 12) + "\n\nae - ee\n\ni - l\n\noe = ie";
        assert_eq!(prune_text(&text).0, "x = y");
    }

    #[test]
    fn keeps_english_phrases_with_punctuation_case_and_contractions() {
        assert_kept_exactly(&[
            "I am",
            "Go to",
            "I am.",
            "Go to.",
            "I'm",
            "IS IT",
            "We go to",
            "\"Go to\"",
            "it\u{2019}s",
        ]);
    }

    #[test]
    fn prunes_a_function_word_beside_a_fragment() {
        for line in ["ee so", "rs or", "ie it", "oe of", "a a", "de la", "zz to"] {
            assert!(is_symbol_noise_line(line), "{line}");
        }
        let text = page(&["Go to."], 12) + "\n\nee so\n\nrs or";
        assert_eq!(prune_text(&text).0, "Go to.");
    }

    #[test]
    fn a_page_of_seven_lines_is_never_pruned_and_eight_is_judged() {
        let seven: Vec<&str> = noise_filler(7);
        assert!(!page_is_dominated_by_noise_lines(seven.into_iter()));
        let eight: Vec<&str> = noise_filler(8);
        assert!(page_is_dominated_by_noise_lines(eight.into_iter()));
    }

    #[test]
    fn exactly_half_noise_is_pruned_and_one_fewer_is_not() {
        let real = ["alpha beta", "gamma delta", "epsilon zeta", "theta iota"];
        let half = page(&real, 4);
        assert!(page_is_dominated_by_noise_lines(half.lines()));
        let below = page(&real, 3);
        let below_with_extra_real = format!("{below}\n\nkappa lambda");
        assert!(!page_is_dominated_by_noise_lines(below_with_extra_real.lines()));
        assert_eq!(
            prune_text(&half).0,
            "alpha beta\n\ngamma delta\n\nepsilon zeta\n\ntheta iota"
        );
    }

    #[test]
    fn a_prose_page_with_a_few_short_lines_is_not_dominated() {
        let mut lines = vec![
            "The quick brown fox",
            "jumps over",
            "the lazy dog",
            "and then",
            "runs far away",
        ];
        lines.extend(["a", "b", "c"]);
        let text = lines.join("\n");
        assert!(!page_is_dominated_by_noise_lines(text.lines()));
    }

    #[test]
    fn prunes_a_noise_page_and_keeps_its_real_lines() {
        assert_eq!(pruned(&["SECTION TITLE", "page 12"], 10), "SECTION TITLE\n\npage 12");
    }

    #[test]
    fn paragraph_text_loses_the_same_lines_as_the_page_text() {
        let mut paragraph = crate::pdf::structure::types::PdfParagraph {
            text: "oe\nSECTION TITLE\nae".to_string(),
            lines: vec![],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count: 4,
        };
        let mut paragraphs = vec![paragraph.clone()];
        prune_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs[0].text, "SECTION TITLE");
        assert_eq!(paragraphs[0].word_count, 2);
        paragraph.text = "oe\nae".to_string();
        let mut emptied = vec![paragraph];
        prune_paragraphs(&mut emptied);
        assert!(emptied.is_empty());
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn annotations_on_surviving_lines_move_with_their_line() {
        use crate::types::document_structure::{AnnotationKind, TextAnnotation};
        use crate::types::internal::{ElementKind, InternalElement};

        let mut element = InternalElement::text(ElementKind::Paragraph, "oe\nSECTION TITLE\nae\nlast line", 0);
        element.annotations = vec![
            TextAnnotation {
                start: 3,
                end: 10,
                kind: AnnotationKind::Bold,
            },
            TextAnnotation {
                start: 20,
                end: 24,
                kind: AnnotationKind::Bold,
            },
            TextAnnotation {
                start: 0,
                end: 2,
                kind: AnnotationKind::Bold,
            },
        ];
        assert!(prune_element_text(&mut element));
        assert_eq!(element.text, "SECTION TITLE\nlast line");
        let spans: Vec<(u32, u32)> = element.annotations.iter().map(|a| (a.start, a.end)).collect();
        assert_eq!(spans, vec![(0, 7), (14, 18)]);
    }
}
