//! Bibliography entries set as author-year paragraphs.
//!
//! A numbered bibliography already reaches assembly as a list (`[1]`, `1.` markers), but an
//! author-year bibliography (`Avramov, D., Cheng, S., ... 2022.`) carries no marker, so its
//! entries were emitted as loose paragraphs. Each entry is one item of the bibliography list,
//! set apart by a hanging indent rather than a glyph; this pass restores that structure.

use super::classify::effective_text as paragraph_text;
use super::types::PdfParagraph;

/// Section titles that open a bibliography.
const BIBLIOGRAPHY_TITLES: &[&str] = &[
    "references",
    "bibliography",
    "literature cited",
    "works cited",
    "reference list",
    "literature",
];

/// Entries a section must open before its paragraphs are read as a bibliography: one or two
/// author-shaped lines under a `References` heading can be prose.
const MIN_BIBLIOGRAPHY_ENTRIES: usize = 3;

/// Longest run of words before an entry's first comma: a surname, possibly compound
/// (`Ben-Porath`, `De Gregorio`, `van der Vaart`).
const MAX_SURNAME_WORDS: usize = 3;

/// Longest bracketed citation key (`[Behrmann et al., 2022]`).
const MAX_CITATION_KEY_CHARS: usize = 80;

/// A paragraph that continues an entry across a paragraph break is a wrapped line or two,
/// not a block of prose.
const MAX_CONTINUATION_WORDS: usize = 60;

/// Mark the entries of an author-year bibliography as unordered list items, folding a
/// paragraph that continues an entry on the same page into that entry.
///
/// An entry takes one continuation of any shape; a further one must open lowercase, mid
/// sentence. Otherwise every short paragraph up to the next heading (a note, a closing
/// remark) would fold into the last entry.
pub(super) fn mark_author_year_bibliography_entries(all_pages: &mut [Vec<PdfParagraph>]) {
    let sections = bibliography_sections(all_pages);
    for (page_index, page) in all_pages.iter_mut().enumerate() {
        let old = std::mem::take(page);
        let mut open_entry: Option<EntryContinuation> = None;
        for (index, mut paragraph) in old.into_iter().enumerate() {
            let in_bibliography = sections[page_index][index];
            if !in_bibliography || !is_candidate(&paragraph) {
                open_entry = None;
                page.push(paragraph);
                continue;
            }
            let text = paragraph_text(&paragraph);
            let text = text.trim();
            if opens_author_year_entry(text) {
                paragraph.is_list_item = true;
                open_entry = Some(EntryContinuation::None);
                page.push(paragraph);
            } else if let Some(state) = open_entry
                && (state == EntryContinuation::None || text.chars().next().is_some_and(char::is_lowercase))
                && paragraph.word_count <= MAX_CONTINUATION_WORDS
                && let Some(entry) = page.last_mut()
            {
                append_continuation(entry, paragraph);
                open_entry = Some(EntryContinuation::Folded);
            } else {
                open_entry = None;
                page.push(paragraph);
            }
        }
    }
}

/// Whether the entry being built has already absorbed a continuation paragraph.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryContinuation {
    None,
    Folded,
}

/// For every paragraph, whether it sits in a bibliography section that opens at least
/// [`MIN_BIBLIOGRAPHY_ENTRIES`] author-year entries. A section runs from its title to the
/// next heading, across pages.
fn bibliography_sections(all_pages: &[Vec<PdfParagraph>]) -> Vec<Vec<bool>> {
    let mut section_of: Vec<Vec<Option<usize>>> = Vec::with_capacity(all_pages.len());
    let mut entry_counts: Vec<usize> = Vec::new();
    let mut current: Option<usize> = None;
    for page in all_pages {
        let mut page_sections = Vec::with_capacity(page.len());
        for paragraph in page {
            if paragraph.heading_level.is_some() {
                current = is_bibliography_title(&paragraph_text(paragraph)).then(|| {
                    entry_counts.push(0);
                    entry_counts.len() - 1
                });
                page_sections.push(None);
                continue;
            }
            if let Some(section) = current
                && is_candidate(paragraph)
                && opens_author_year_entry(paragraph_text(paragraph).trim())
            {
                entry_counts[section] += 1;
            }
            page_sections.push(current);
        }
        section_of.push(page_sections);
    }
    section_of
        .into_iter()
        .map(|page| {
            page.into_iter()
                .map(|section| section.is_some_and(|section| entry_counts[section] >= MIN_BIBLIOGRAPHY_ENTRIES))
                .collect()
        })
        .collect()
}

fn is_candidate(paragraph: &PdfParagraph) -> bool {
    paragraph.heading_level.is_none()
        && !paragraph.is_list_item
        && !paragraph.is_code_block
        && !paragraph.is_formula
        && !paragraph.is_page_furniture
        && paragraph.word_count > 0
}

fn is_bibliography_title(text: &str) -> bool {
    let title = text
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, '.' | ' ' | 'I' | 'V' | 'X'))
        .trim_end_matches([':', '.'])
        .trim()
        .to_lowercase();
    BIBLIOGRAPHY_TITLES.contains(&title.as_str())
}

/// Whether `text` opens an author-year entry: a bracketed author-year key
/// (`[Behrmann et al., 2022]`), or a surname, a comma, and a given name or initial
/// (`Avramov, D.,`, `Alaei, Saeed,`) in an entry that also carries a year or further
/// initials.
fn opens_author_year_entry(text: &str) -> bool {
    opens_with_citation_key(text) || opens_with_surname_and_given_name(text)
}

fn opens_with_citation_key(text: &str) -> bool {
    let Some(inner) = text.strip_prefix('[') else {
        return false;
    };
    let Some(close) = inner.find(']') else {
        return false;
    };
    let key = &inner[..close];
    key.chars().count() <= MAX_CITATION_KEY_CHARS && key.chars().any(char::is_alphabetic) && has_year(key)
}

fn opens_with_surname_and_given_name(text: &str) -> bool {
    let Some((surname, rest)) = text.split_once(',') else {
        return false;
    };
    let surname_words: Vec<&str> = surname.split_whitespace().collect();
    let surname_ok = (1..=MAX_SURNAME_WORDS).contains(&surname_words.len())
        && surname_words
            .last()
            .and_then(|word| word.chars().next())
            .is_some_and(char::is_uppercase)
        && surname_words.iter().all(|word| is_name_word(word));
    let given_ok = rest.starts_with(' ') && rest.trim_start().chars().next().is_some_and(char::is_uppercase);
    surname_ok && given_ok && (has_year(text) || initial_count(text) >= 2)
}

fn is_name_word(word: &str) -> bool {
    word.chars().any(char::is_alphabetic)
        && word
            .chars()
            .all(|c| c.is_alphabetic() || matches!(c, '-' | '\'' | '\u{2019}' | '`' | '\\' | '.'))
}

/// Whether `text` holds a four-digit year from 1800 to 2099.
fn has_year(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(4).enumerate().any(|(start, window)| {
        window.iter().all(u8::is_ascii_digit)
            && matches!(&window[..2], b"18" | b"19" | b"20")
            && (start == 0 || !bytes[start - 1].is_ascii_digit())
            && bytes.get(start + 4).is_none_or(|next| !next.is_ascii_digit())
    })
}

/// Initials such as the `D.` of `Avramov, D.`: one capital followed by a full stop.
fn initial_count(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len().saturating_sub(1))
        .filter(|&index| {
            chars[index].is_uppercase() && chars[index + 1] == '.' && (index == 0 || !chars[index - 1].is_alphabetic())
        })
        .count()
}

fn append_continuation(entry: &mut PdfParagraph, continuation: PdfParagraph) {
    entry.text.clear();
    entry.block_bbox = super::geometry::union_bbox(entry.block_bbox, continuation.block_bbox);
    entry.lines.extend(continuation.lines);
    entry.word_count = PdfParagraph::compute_word_count("", &entry.lines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::PdfLine;

    fn paragraph(text: &str) -> PdfParagraph {
        let segments = vec![SegmentData {
            text: text.to_string(),
            x: 72.0,
            y: 700.0,
            width: 400.0,
            height: 10.0,
            font_size: 10.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
            ocr_heading_spans: None,
        }];
        let lines = vec![PdfLine {
            native_scripts: Vec::new(),
            segments,
            baseline_y: Some(700.0),
            dominant_font_size: 10.0,
            is_bold: false,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 10.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            ocr_source_spans: Vec::new(),
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    fn heading(text: &str) -> PdfParagraph {
        let mut heading = paragraph(text);
        heading.heading_level = Some(2);
        heading
    }

    fn summary(pages: &[Vec<PdfParagraph>]) -> Vec<(bool, String)> {
        pages
            .iter()
            .flatten()
            .map(|paragraph| (paragraph.is_list_item, paragraph_text(paragraph)))
            .collect()
    }

    /// 2310.02163, 1602.03238 and 1710.02091 set their bibliographies author-year with a
    /// hanging indent and no marker, so every entry was emitted as a loose paragraph.
    #[test]
    fn marks_author_year_entries_and_folds_their_continuations() {
        let mut pages = vec![vec![
            paragraph("We thank the referees, Smith, J., for comments in 2021."),
            heading("References"),
            paragraph("Alaei, Saeed, Hu Fu, and Azarakhsh Malekian (2019):"),
            paragraph("\u{201c}Efficient computation of optimal auctions via reduced forms,\u{201d}"),
            paragraph("Avramov, D., Cheng, S., Lioui, A., Tarelli, A., 2022. Sustainable investing."),
            paragraph("[Behrmann et al., 2022] Nadine Behrmann and Mehdi Noroozi. Unified supervision."),
            heading("Appendix A"),
            paragraph("Proof, Lemma 1 holds for every n in 2020 cases."),
        ]];
        mark_author_year_bibliography_entries(&mut pages);
        assert_eq!(
            summary(&pages),
            vec![
                (
                    false,
                    "We thank the referees, Smith, J., for comments in 2021.".to_string()
                ),
                (false, "References".to_string()),
                (
                    true,
                    "Alaei, Saeed, Hu Fu, and Azarakhsh Malekian (2019): \u{201c}Efficient computation of optimal \
                     auctions via reduced forms,\u{201d}"
                        .to_string()
                ),
                (
                    true,
                    "Avramov, D., Cheng, S., Lioui, A., Tarelli, A., 2022. Sustainable investing.".to_string()
                ),
                (
                    true,
                    "[Behrmann et al., 2022] Nadine Behrmann and Mehdi Noroozi. Unified supervision.".to_string()
                ),
                (false, "Appendix A".to_string()),
                (false, "Proof, Lemma 1 holds for every n in 2020 cases.".to_string()),
            ],
            "entries under the bibliography title become items; text outside it is untouched"
        );
    }

    #[test]
    fn folds_one_continuation_per_entry_and_further_ones_only_mid_sentence() {
        let mut pages = vec![vec![
            heading("References"),
            paragraph("Alaei, Saeed, Hu Fu, and Azarakhsh Malekian (2019):"),
            paragraph("\u{201c}Efficient computation of optimal auctions via reduced forms,\u{201d}"),
            paragraph("and its online appendix."),
            paragraph("Avramov, D., Cheng, S., Lioui, A., Tarelli, A., 2022. Sustainable investing."),
            paragraph("Avramov, D., Cheng, S., 2023. Green investing."),
            paragraph("Entries marked with an asterisk are included in the meta-analysis."),
            paragraph("Further reading is listed on the project website."),
        ]];
        mark_author_year_bibliography_entries(&mut pages);
        assert_eq!(
            summary(&pages),
            vec![
                (false, "References".to_string()),
                (
                    true,
                    "Alaei, Saeed, Hu Fu, and Azarakhsh Malekian (2019): \u{201c}Efficient computation of optimal \
                     auctions via reduced forms,\u{201d} and its online appendix."
                        .to_string()
                ),
                (
                    true,
                    "Avramov, D., Cheng, S., Lioui, A., Tarelli, A., 2022. Sustainable investing.".to_string()
                ),
                (
                    true,
                    "Avramov, D., Cheng, S., 2023. Green investing. Entries marked with an asterisk are included \
                     in the meta-analysis."
                        .to_string()
                ),
                (false, "Further reading is listed on the project website.".to_string()),
            ]
        );
    }

    #[test]
    fn leaves_a_section_with_too_few_entries_alone() {
        let mut pages = vec![vec![
            heading("References"),
            paragraph("Avramov, D., Cheng, S., 2022. Sustainable investing."),
            paragraph("Further reading is listed on the project website."),
        ]];
        mark_author_year_bibliography_entries(&mut pages);
        assert!(
            pages[0].iter().all(|paragraph| !paragraph.is_list_item),
            "one author-shaped line is not a bibliography"
        );
    }

    #[test]
    fn reads_numbered_and_roman_bibliography_titles() {
        assert!(is_bibliography_title("7 References"));
        assert!(is_bibliography_title("VII. REFERENCES"));
        assert!(is_bibliography_title("Bibliography:"));
        assert!(!is_bibliography_title("Related Work"));
        assert!(!is_bibliography_title("Cross References to Rules"));
    }
}
