//! Removal of symbol-noise lines from a page OCR already judged to be recognition noise (#2312).
//!
//! Tesseract reads a photograph as a long run of one- and two-letter fragments and stray
//! symbols, one per line. [`super::scoring::is_ocr_recognition_noise`] flags such a page, but
//! with `discard_suspected_ocr_noise` off the text is retained whole. This module prunes only
//! the lines that carry no word-shaped token, and runs after the page verdict, so every
//! backend, table and quality decision still sees the unpruned text.

use crate::pdf::structure::types::PdfParagraph;

/// Fewest non-blank lines a page needs before its line mix says anything.
const MIN_LINES: usize = 8;
/// A token with at least this many letters is word-shaped.
const MIN_WORD_LETTERS: usize = 3;
/// A line made of one token needs one more letter: nothing around it vouches for a bare trigram.
const MIN_LONE_WORD_LETTERS: usize = 4;
/// A digit run this long is a number worth keeping; a lone digit is as likely a speck.
const MIN_NUMBER_DIGITS: usize = 2;

fn has_number(line: &str) -> bool {
    let mut run = 0usize;
    for c in line.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= MIN_NUMBER_DIGITS {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// A line is symbol noise when fewer than half its tokens are word-shaped and it holds no number.
fn is_symbol_noise_line(line: &str) -> bool {
    let tokens = line.split_whitespace().count();
    if tokens == 0 || has_number(line) {
        return false;
    }
    let min_letters = if tokens == 1 {
        MIN_LONE_WORD_LETTERS
    } else {
        MIN_WORD_LETTERS
    };
    let words = line
        .split_whitespace()
        .filter(|token| token.chars().filter(|c| c.is_alphabetic()).count() >= min_letters)
        .count();
    words * 2 < tokens
}

/// Whether symbol-noise lines are at least half of a page's non-blank lines.
fn page_is_dominated_by_noise_lines<'a>(lines: impl Iterator<Item = &'a str>) -> bool {
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

/// The page text without its symbol-noise lines, or `None` when the page is not dominated by them.
pub(super) fn prune_symbol_noise_lines(text: &str) -> Option<String> {
    if !page_is_dominated_by_noise_lines(text.lines()) {
        return None;
    }
    let kept = text
        .lines()
        .filter(|line| !is_symbol_noise_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    Some(collapse_blank_runs(&kept))
}

fn paragraph_lines(paragraph: &PdfParagraph) -> Vec<String> {
    if !paragraph.text.is_empty() {
        return paragraph.text.lines().map(str::to_string).collect();
    }
    paragraph
        .lines
        .iter()
        .map(|line| {
            line.segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Prunes the same lines from a page's structured paragraphs. Returns whether anything changed.
pub(super) fn prune_symbol_noise_paragraphs(paragraphs: &mut Vec<PdfParagraph>) -> bool {
    let per_paragraph: Vec<Vec<String>> = paragraphs.iter().map(paragraph_lines).collect();
    if !page_is_dominated_by_noise_lines(per_paragraph.iter().flatten().map(String::as_str)) {
        return false;
    }
    let before = paragraphs.len();
    let mut index = 0usize;
    let mut changed = false;
    paragraphs.retain_mut(|paragraph| {
        let lines = &per_paragraph[index];
        index += 1;
        let keep: Vec<bool> = lines.iter().map(|line| !is_symbol_noise_line(line)).collect();
        if keep.iter().all(|k| *k) {
            return true;
        }
        changed = true;
        if !keep.iter().any(|k| *k) {
            return false;
        }
        if !paragraph.text.is_empty() {
            paragraph.text = lines
                .iter()
                .zip(&keep)
                .filter(|(_, k)| **k)
                .map(|(line, _)| line.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            paragraph.word_count = paragraph.text.split_whitespace().count();
        } else {
            let mut flags = keep.iter();
            paragraph.lines.retain(|_| *flags.next().unwrap_or(&true));
        }
        true
    });
    changed || paragraphs.len() != before
}

/// Prunes a standalone image's OCR text and its hOCR element tree together once the page text
/// is both recognition noise and dominated by symbol-noise lines. Pages holding a table are
/// left alone.
#[cfg(all(feature = "ocr", feature = "pdf"))]
pub(crate) fn prune_standalone_image_noise(
    content: &mut String,
    document: Option<&mut crate::types::internal::InternalDocument>,
    has_tables: bool,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> bool {
    use crate::types::internal::ElementKind;

    if has_tables || !super::scoring::is_ocr_recognition_noise(content, thresholds) {
        return false;
    }
    let Some(pruned) = prune_symbol_noise_lines(content) else {
        return false;
    };
    *content = pruned;
    if let Some(document) = document
        && document.relationships.is_empty()
    {
        document.elements.retain_mut(|element| {
            if !matches!(
                element.kind,
                ElementKind::OcrText { .. }
                    | ElementKind::Title
                    | ElementKind::Heading { .. }
                    | ElementKind::Paragraph
                    | ElementKind::ListItem { .. }
            ) || !element.text.lines().any(is_symbol_noise_line)
            {
                return true;
            }
            element.text = element
                .text
                .lines()
                .filter(|line| !is_symbol_noise_line(line))
                .collect::<Vec<_>>()
                .join("\n");
            element.annotations.clear();
            !element.text.trim().is_empty()
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_word_shaped_lines_numbers_and_titles() {
        for line in [
            "SECTION TITLE",
            "Chapter 1",
            "Page 12",
            "99",
            "NORTH GATE 42",
            "Figure 3 sunset",
        ] {
            assert!(!is_symbol_noise_line(line), "{line}");
        }
    }

    #[test]
    fn flags_fragment_and_symbol_lines() {
        for line in [
            "a",
            "mal",
            "Of 3a",
            "# ae",
            "ef \"ae Car Sa % ey Bo ary \"dl",
            "<4",
            "1a",
            "~",
        ] {
            assert!(is_symbol_noise_line(line), "{line}");
        }
    }

    #[test]
    fn leaves_a_page_without_a_noise_majority_untouched() {
        let prose = "The quick brown fox\njumps over\nthe lazy dog\nand then\na\nb\n";
        assert_eq!(prune_symbol_noise_lines(prose), None);
    }

    #[test]
    fn prunes_a_noise_page_and_keeps_its_real_lines() {
        let mut text = String::from("SECTION TITLE\n\n");
        for fragment in ["a", "oe", "# 4", "Of 3a", "ae", "\"", "ee", "=|", "rs", "~"] {
            text.push_str(fragment);
            text.push_str("\n\n");
        }
        text.push_str("page 12\n");
        assert_eq!(prune_symbol_noise_lines(&text).unwrap(), "SECTION TITLE\n\npage 12");
    }
}
