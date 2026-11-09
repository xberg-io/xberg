//! Glyphs without a Unicode value in native PDF text.
//!
//! The native extractor keeps one marker for each Type0 glyph that has no Unicode value (U+FFFD,
//! or U+FFFE for a glyph of a symbol font), so the OCR routing gates can count the loss. A page
//! that OCR does not replace must not show a marker to the caller. This module replaces each run
//! of markers with one space between the characters that survive, so that `A<marker>B` and
//! `1<marker>2` do not fuse into `AB` and `12`. It does this after the routing decision, keeps
//! every byte offset that points into the text correct, and builds the warning for a document
//! that still lacks characters. ~keep

use crate::core::config::ExtractionConfig;
use crate::types::internal::InternalDocument;
use crate::types::{PageBoundary, PageContent, ProcessingWarning, Table};

/// The marker the native extractor leaves for a glyph without a Unicode value.
pub(super) const LOST_GLYPH_MARKER: char = '\u{FFFD}';

/// The marker the native extractor leaves for a glyph without a Unicode value from a symbol font
/// (a checkbox, a bullet): `xberg_native_pdf::LOST_SYMBOL_GLYPH_MARKER`, repeated here because
/// this module also builds without the `pdf` feature. Such a glyph is no letter of the word next
/// to it. The decoder writes this noncharacter for nothing else; U+FFFF it does write for text
/// (a glyph a font cannot name), so U+FFFF is no marker. ~keep
pub(super) const LOST_SYMBOL_GLYPH_MARKER: char = '\u{FFFE}';

/// Whether `c` is a lost-glyph marker of either kind.
pub(super) fn is_lost_glyph(c: char) -> bool {
    c == LOST_GLYPH_MARKER || c == LOST_SYMBOL_GLYPH_MARKER
}

/// A run of consecutive markers in a text, and what replaces it. One lost glyph, or several in a
/// row, stands for characters that exist on the page, so the run leaves a single space between
/// the characters that survive. Next to whitespace, or at an edge of the text, it leaves nothing.
struct Run {
    start: usize,
    end: usize,
    replacement: &'static str,
}

/// The runs of markers in a text, in order. Maps an offset in the text with the markers to the same
/// place in the text without them.
struct MarkerRuns(Vec<Run>);

impl MarkerRuns {
    fn of(text: &str) -> Option<Self> {
        Self::outside(text, &[])
    }

    /// The runs made of markers that lie outside every protected byte range.
    fn outside(text: &str, protected: &[(usize, usize)]) -> Option<Self> {
        let mut runs: Vec<Run> = Vec::new();
        for (at, marker) in text.match_indices(is_lost_glyph) {
            if protected.iter().any(|&(start, end)| (start..end).contains(&at)) {
                continue;
            }
            match runs.last_mut() {
                Some(run) if run.end == at => run.end = at + marker.len(),
                _ => runs.push(Run {
                    start: at,
                    end: at + marker.len(),
                    replacement: "",
                }),
            }
        }
        for run in &mut runs {
            let before = text[..run.start].chars().next_back();
            let after = text[run.end..].chars().next();
            if before.is_some_and(|c| !c.is_whitespace()) && after.is_some_and(|c| !c.is_whitespace()) {
                run.replacement = " ";
            }
        }
        (!runs.is_empty()).then_some(Self(runs))
    }

    fn count(&self) -> usize {
        self.0
            .iter()
            .map(|run| (run.end - run.start) / LOST_GLYPH_MARKER.len_utf8())
            .sum()
    }

    fn map(&self, offset: usize) -> usize {
        let mut removed = 0;
        for run in &self.0 {
            if offset >= run.end {
                removed += run.end - run.start - run.replacement.len();
            } else {
                if offset > run.start {
                    return run.start - removed;
                }
                break;
            }
        }
        offset - removed
    }

    fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut from = 0;
        for run in &self.0 {
            out.push_str(&text[from..run.start]);
            out.push_str(run.replacement);
            from = run.end;
        }
        out.push_str(&text[from..]);
        out
    }
}

fn without_markers(text: &str) -> String {
    MarkerRuns::of(text).map_or_else(|| text.to_string(), |runs| runs.apply(text))
}

/// Clean a selected native fragment before it is combined with independently owned OCR text.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn strip_native_fragment(text: &mut String) -> usize {
    strip_text(text, None, &[])
}

/// The text a quality gate or a comparison reads: the native text with each run of markers
/// stripped by [`strip_native_fragment`], so `caf<marker>e` reads as `caf e` and not as `cafe`.
/// The text itself keeps its markers. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn readable_text(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains(is_lost_glyph) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut readable = text.to_owned();
    strip_native_fragment(&mut readable);
    std::borrow::Cow::Owned(readable)
}

/// Remove the markers that native extraction left in `text`. `boundaries` index `text` by byte and
/// move with it. A character U+FFFD inside a page in `ocr_pages` belongs to the OCR output, so it
/// stays. Returns the number of markers removed.
pub(super) fn strip_text(text: &mut String, boundaries: Option<&mut Vec<PageBoundary>>, ocr_pages: &[u32]) -> usize {
    let protected: Vec<(usize, usize)> = boundaries
        .as_deref()
        .into_iter()
        .flatten()
        .filter(|b| ocr_pages.contains(&b.page_number))
        .map(|b| (b.byte_start, b.byte_end))
        .collect();
    let Some(runs) = MarkerRuns::outside(text, &protected) else {
        return 0;
    };
    for boundary in boundaries.into_iter().flatten() {
        boundary.byte_start = runs.map(boundary.byte_start);
        boundary.byte_end = runs.map(boundary.byte_end);
    }
    *text = runs.apply(text);
    runs.count()
}

/// Remove every marker from the text of the elements of `doc`, with their annotation offsets.
pub(super) fn strip_document(doc: &mut InternalDocument) -> usize {
    let mut removed = 0;
    for element in &mut doc.elements {
        let Some(runs) = MarkerRuns::of(&element.text) else {
            continue;
        };
        for annotation in &mut element.annotations {
            annotation.start = runs.map(annotation.start as usize) as u32;
            annotation.end = runs.map(annotation.end as usize) as u32;
        }
        element.text = runs.apply(&element.text);
        removed += runs.count();
    }
    for table in &mut doc.tables {
        removed += strip_table(table);
    }
    removed
}

/// Remove every marker from a page: its text, its hierarchy blocks and its tables.
/// A page whose text becomes blank is marked blank.
pub(super) fn strip_page(page: &mut PageContent) -> usize {
    let mut removed = page.content.matches(is_lost_glyph).count();
    if removed > 0 {
        page.content = without_markers(&page.content);
        page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
    }
    if let Some(hierarchy) = page.hierarchy.as_mut() {
        for block in &mut hierarchy.blocks {
            removed += block.text.matches(is_lost_glyph).count();
            block.text = without_markers(&block.text);
        }
    }
    for table in &mut page.tables {
        if table.markdown.contains(is_lost_glyph)
            || table.cells.iter().flatten().any(|c| c.contains(is_lost_glyph))
            || table.columns.iter().flatten().any(|c| c.contains(is_lost_glyph))
        {
            removed += strip_table(std::sync::Arc::make_mut(table));
        }
    }
    removed
}

/// Remove every marker from the cells and the Markdown of a table.
pub(super) fn strip_table(table: &mut Table) -> usize {
    let mut removed = table.markdown.matches(is_lost_glyph).count();
    table.markdown = without_markers(&table.markdown);
    for cell in table
        .cells
        .iter_mut()
        .flatten()
        .chain(table.columns.iter_mut().flatten())
    {
        removed += cell.matches(is_lost_glyph).count();
        *cell = without_markers(cell);
    }
    removed
}

/// The warning for a document that still lacks characters after extraction. It names the OCR
/// setting only for a caller who has not configured OCR and has not switched it off.
pub(super) fn lost_glyph_warning(lost: usize, config: &ExtractionConfig) -> Option<ProcessingWarning> {
    if lost == 0 {
        return None;
    }
    let message = if config.ocr.is_none() && !config.effective_disable_ocr() {
        "Some glyphs in this PDF have no Unicode value, so their characters are missing from the text. \
         Set `ocr` to read the affected pages with OCR."
    } else {
        "Some glyphs in this PDF have no Unicode value, and their characters are missing from the text \
         that OCR did not replace."
    };
    Some(ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf"),
        message: std::borrow::Cow::Borrowed(message),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::OcrConfig;
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{ElementKind, InternalElement};

    const MARK: &str = "\u{FFFD}";

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn readable_text_keeps_a_space_where_a_glyph_inside_a_word_was_lost() {
        assert_eq!(readable_text(&format!("caf{MARK}e")), "caf e");
        assert_eq!(readable_text(&format!("1{MARK}2")), "1 2");
        assert_eq!(readable_text(&format!("the {MARK} cafe{MARK}")), "the  cafe");
        assert!(matches!(readable_text("no loss"), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn a_lost_symbol_glyph_is_stripped_and_counted_like_a_lost_glyph() {
        const SYMBOL: &str = "\u{FFFE}";
        assert_eq!(LOST_SYMBOL_GLYPH_MARKER.to_string(), SYMBOL);
        assert_eq!(
            LOST_SYMBOL_GLYPH_MARKER.len_utf8(),
            LOST_GLYPH_MARKER.len_utf8(),
            "a run counts its glyphs by one marker length"
        );
        assert!(is_lost_glyph(LOST_SYMBOL_GLYPH_MARKER) && is_lost_glyph(LOST_GLYPH_MARKER));
        assert!(!is_lost_glyph('\u{FDD1}') && !is_lost_glyph(' '));
        assert!(
            !is_lost_glyph('\u{FFFF}'),
            "U+FFFF is a glyph the decoder writes for text, not a marker"
        );
        assert_eq!(readable_text(&format!("{SYMBOL}yes no")), "yes no");
        assert_eq!(readable_text(&format!("A{SYMBOL}B")), "A B");
        let mut text = format!("ab{SYMBOL}cd{MARK}{SYMBOL}ef{SYMBOL}");
        let removed = strip_text(&mut text, None, &[]);
        assert_eq!(removed, 4, "each marker of either kind counts");
        assert_eq!(text, "ab cd ef");
    }

    /// The marker the native extractor emits is the one this module strips. ~keep
    #[cfg(feature = "pdf")]
    #[test]
    fn the_symbol_marker_is_the_one_the_native_extractor_emits() {
        assert_eq!(LOST_SYMBOL_GLYPH_MARKER, xberg_native_pdf::LOST_SYMBOL_GLYPH_MARKER);
    }

    #[test]
    fn strip_text_removes_markers_and_moves_page_boundaries() {
        // "ab" + marker (bytes 0..5), then "cd" + marker + "ef" (bytes 5..12).
        let mut text = format!("ab{MARK}cd{MARK}ef");
        let mut boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 5,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 5,
                byte_end: 12,
                page_number: 2,
            },
        ];

        let removed = strip_text(&mut text, Some(&mut boundaries), &[]);

        assert_eq!(removed, 2);
        assert_eq!(text, "ab cd ef");
        assert_eq!(
            (boundaries[0].byte_start, boundaries[0].byte_end),
            (0, 3),
            "page 1 ends after its two letters and the space that stands for the lost glyph"
        );
        assert_eq!((boundaries[1].byte_start, boundaries[1].byte_end), (3, 8));
    }

    #[test]
    fn strip_text_leaves_a_boundary_where_a_glyph_was_lost() {
        let cases = [
            ("A\u{FFFD}B", "A B"),
            ("1\u{FFFD}2", "1 2"),
            ("A\u{FFFD}\u{FFFD}\u{FFFD}B", "A B"),
            ("A \u{FFFD}B", "A B"),
            ("A\u{FFFD} B", "A B"),
            ("\u{FFFD}A", "A"),
            ("A\u{FFFD}", "A"),
            ("A\u{FFFD}\nB", "A\nB"),
        ];
        for (before, after) in cases {
            let mut text = before.to_string();
            strip_text(&mut text, None, &[]);
            assert_eq!(text, after, "input {before:?}");
        }
    }

    #[test]
    fn strip_text_leaves_text_without_markers_untouched() {
        let mut text = "plain \u{e9} text".to_string();
        let mut boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        }];
        let before: Vec<(usize, usize, u32)> = boundaries
            .iter()
            .map(|b| (b.byte_start, b.byte_end, b.page_number))
            .collect();

        assert_eq!(strip_text(&mut text, Some(&mut boundaries), &[]), 0);
        assert_eq!(text, "plain \u{e9} text");
        let after: Vec<(usize, usize, u32)> = boundaries
            .iter()
            .map(|b| (b.byte_start, b.byte_end, b.page_number))
            .collect();
        assert_eq!(after, before);
    }

    #[test]
    fn strip_document_moves_annotation_offsets_with_the_text() {
        let mut doc = InternalDocument::new("application/pdf");
        let mut element = InternalElement::text(ElementKind::Paragraph, format!("x{MARK}yz"), 0);
        // "yz" sits at bytes 4..6 while the marker is present.
        element.annotations = vec![TextAnnotation {
            start: 4,
            end: 6,
            kind: AnnotationKind::Bold,
        }];
        doc.elements.push(element);

        let removed = strip_document(&mut doc);

        assert_eq!(removed, 1);
        assert_eq!(doc.elements[0].text, "x yz");
        assert_eq!(
            (doc.elements[0].annotations[0].start, doc.elements[0].annotations[0].end),
            (2, 4)
        );
    }

    #[test]
    fn strip_page_marks_a_page_blank_when_only_markers_were_on_it() {
        let mut page = PageContent {
            page_number: 1,
            content: format!("{MARK}{MARK}\n{MARK}"),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(false),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        };

        assert_eq!(strip_page(&mut page), 3);
        assert_eq!(page.content, "\n");
        assert_eq!(page.is_blank, Some(true));
    }

    #[test]
    fn strip_table_cleans_cells_and_markdown() {
        let mut table = Table {
            cells: vec![vec![format!("a{MARK}"), "b".to_string()]],
            markdown: format!("| a{MARK} | b |"),
            columns: Some(vec![format!("a{MARK}"), "b".into()]),
            page_number: 1,
            bounding_box: None,
            ..Table::default()
        };

        assert_eq!(strip_table(&mut table), 3);
        assert_eq!(table.cells[0][0], "a");
        assert_eq!(table.markdown, "| a | b |");
        assert_eq!(table.columns.unwrap(), ["a", "b"]);
    }

    #[test]
    fn warning_stays_until_recovery_and_names_the_ocr_setting_only_when_it_helps() {
        let bare = ExtractionConfig::default();
        let with_block = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ..ExtractionConfig::default()
        };
        let switched_off = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };

        let warning = lost_glyph_warning(3, &bare).expect("a caller without OCR gets the warning");
        assert_eq!(warning.source, "pdf");
        assert!(
            warning.message.contains("no Unicode value"),
            "message: {}",
            warning.message
        );
        assert!(
            warning.message.contains("`ocr`"),
            "message names the setting: {}",
            warning.message
        );
        assert!(lost_glyph_warning(0, &bare).is_none(), "no lost glyph, no warning");
        for config in [&with_block, &switched_off] {
            let warning = lost_glyph_warning(3, config).expect("lost glyphs that OCR did not recover keep a signal");
            assert!(
                warning.message.contains("no Unicode value"),
                "message: {}",
                warning.message
            );
            assert!(
                !warning.message.contains("`ocr`"),
                "no advice to set what is already set or off: {}",
                warning.message
            );
        }
    }

    #[test]
    fn strip_text_keeps_a_replacement_character_that_belongs_to_ocr_output() {
        // Page 1 is OCR output "A<mark>B" (bytes 0..5); page 2 is native "c<mark>d" (bytes 5..10).
        let mut text = format!("A{MARK}Bc{MARK}d");
        let mut boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 5,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 5,
                byte_end: 10,
                page_number: 2,
            },
        ];

        let removed = strip_text(&mut text, Some(&mut boundaries), &[1]);

        assert_eq!(removed, 1, "only the native marker counts as a loss");
        assert_eq!(text, format!("A{MARK}Bc d"));
        assert_eq!((boundaries[0].byte_start, boundaries[0].byte_end), (0, 5));
        assert_eq!((boundaries[1].byte_start, boundaries[1].byte_end), (5, 8));
    }
}
