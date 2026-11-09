//! Reconciles a page's OCR text elements with the tables detected on the same page, shared by
//! every OCR backend that returns both.

use std::collections::{HashMap, HashSet};

use crate::types::extraction::BoundingBox;
use crate::types::internal::{InternalDocument, InternalElement};

/// Word-count shortfall a markdown table rebuild is allowed relative to the content it would
/// replace before the rebuild is rejected as content loss (GH#1599). Zero: table syntax (`|`,
/// `---`) itself adds whitespace-separated tokens, so a rebuild that faithfully reformats
/// existing prose into a table is never word-count-negative -- only a rebuild that actually
/// dropped page content comes in under the original count.
const TABLE_REBUILD_MIN_WORD_RETENTION: usize = 0;

/// Whether a markdown table rebuild should replace `original_content`.
///
/// `build_content_with_inline_tables` reconstructs page content from a crude y-position
/// clustering heuristic that is far less robust than the hOCR-derived content it replaces.
/// Non-emptiness alone (the previous guard) cannot distinguish a legitimate rebuild from one
/// that silently dropped most of the page -- see GH#1599, where OCR markdown output lost a
/// large amount of content that plain output retained. Comparing absolute word counts catches
/// that: a rebuild is adopted only when it does not lose material.
pub(crate) fn should_adopt_table_rebuild(original_content: &str, rebuilt_content: &str) -> bool {
    let original_word_count = original_content.split_whitespace().count();
    let rebuilt_word_count = rebuilt_content.split_whitespace().count();
    rebuilt_word_count + TABLE_REBUILD_MIN_WORD_RETENTION >= original_word_count
}

/// Drop the page elements whose text a detected table already carries (#1571).
///
/// An OCR backend builds its page elements and its tables from the same recognition pass but
/// never reconciles them, so every consumer of the page document receives a table's text twice:
/// once as ordinary text elements and once as the table. An element is claimed when its bbox
/// centre lies inside a table's bbox. Run this where the elements and the table bboxes still
/// share the backend's pixel space; downstream consumers rescale the two separately. ~keep
///
/// A table claims the entire region when it kept the region's words. A lossy table can
/// claim only individual elements it matches to complete rows (xberg-io/xberg#1884).
pub(crate) fn drop_elements_claimed_by_tables<'a>(
    elements: Vec<InternalElement>,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
) -> Vec<InternalElement> {
    let mut claiming_bboxes: Vec<BoundingBox> = Vec::new();
    let mut claimed_lines = HashSet::new();
    for (bbox, cells) in tables {
        if table_retains_region_words(&elements, &bbox, cells) {
            claiming_bboxes.push(bbox);
        } else {
            claim_matching_table_rows(&elements, &bbox, cells, &mut claimed_lines);
            tracing::warn!(
                target: "xberg::ocr::tables",
                left = bbox.x0,
                top = bbox.y0,
                right = bbox.x1,
                bottom = bbox.y1,
                table_rows = cells.len(),
                "reconstructed table dropped words from its region; keeping lines without a matching row"
            );
        }
    }
    if claiming_bboxes.is_empty() && claimed_lines.is_empty() {
        return elements;
    }

    elements
        .into_iter()
        .enumerate()
        .filter_map(|(index, element)| {
            let claimed = claimed_lines.contains(&index)
                || claiming_bboxes
                    .iter()
                    .any(|table_bbox| element_center_within_table(&element, table_bbox));
            (!claimed).then_some(element)
        })
        .collect()
}

/// A row may replace one single-line element with the same ordered tokens. Spending each
/// row once preserves repeated source lines, and unmatched text stays intact.
fn claim_matching_table_rows(
    elements: &[InternalElement],
    bbox: &BoundingBox,
    cells: &[Vec<String>],
    claimed_lines: &mut HashSet<usize>,
) {
    let mut rows = ExactTableRows::new(cells);
    for (index, element) in elements.iter().enumerate() {
        if !claimed_lines.contains(&index)
            && element_center_within_table(element, bbox)
            && !element.text.trim().contains(['\n', '\r'])
            && rows.claim(element.text.split_whitespace())
        {
            claimed_lines.insert(index);
        }
    }
}

pub(crate) struct ExactTableRows {
    rows: HashMap<String, usize>,
    ignore_marks: bool,
}

impl ExactTableRows {
    pub(crate) fn new(cells: &[Vec<String>]) -> Self {
        Self::with_marks(cells, false)
    }

    /// Rows compared without their leader tokens (made only of `. \u{b7} _ \u{2026} =`), so a dot leader
    /// or a stray `=` between a label and its values does not keep a line whose every word the table
    /// carries (#2783). Signs, dashes, brackets and currency symbols stay significant.
    pub(crate) fn ignoring_marks(cells: &[Vec<String>]) -> Self {
        Self::with_marks(cells, true)
    }

    fn with_marks(cells: &[Vec<String>], ignore_marks: bool) -> Self {
        let mut rows = HashMap::new();
        for row in cells {
            let tokens = row_tokens(row.iter().flat_map(|cell| cell.split_whitespace()), ignore_marks);
            if !tokens.is_empty() {
                *rows.entry(tokens).or_default() += 1;
            }
        }
        Self { rows, ignore_marks }
    }

    pub(crate) fn claim<'a>(&mut self, tokens: impl Iterator<Item = &'a str>) -> bool {
        let key = row_tokens(tokens, self.ignore_marks);
        let Some(remaining) = self.rows.get_mut(&key).filter(|count| **count > 0) else {
            return false;
        };
        *remaining -= 1;
        true
    }
}

fn is_leader_mark(character: char) -> bool {
    matches!(character, '.' | '\u{b7}' | '_' | '\u{2026}' | '=')
}

fn row_tokens<'a>(tokens: impl Iterator<Item = &'a str>, ignore_marks: bool) -> String {
    tokens
        .filter(|token| !ignore_marks || !token.chars().all(is_leader_mark))
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`drop_elements_claimed_by_tables`] over a page document, recording in
/// `ocr_text_claimed_by_tables` whether it removed every text element of the page. ~keep
pub(crate) fn drop_document_elements_claimed_by_tables<'a>(
    document: &mut InternalDocument,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
) {
    let element_count = document.elements.len();
    document.elements = drop_elements_claimed_by_tables(std::mem::take(&mut document.elements), tables);
    document.ocr_text_claimed_by_tables = document.elements.len() < element_count
        && document.elements.iter().all(|element| element.text.trim().is_empty());
}

/// Whether `element`'s bbox centre lies inside `table_bbox`.
///
/// Shared by [`drop_elements_claimed_by_tables`] and [`table_retains_region_words`] so a table is
/// judged for word retention against exactly the elements it would remove — two copies of this test
/// could disagree and make the retention check answer about a different set of lines than the filter
/// then deletes. An element with no geometry is not covered by any table. ~keep
pub(crate) fn element_center_within_table(element: &InternalElement, table_bbox: &BoundingBox) -> bool {
    let Some(bbox) = element.bbox.as_ref() else {
        return false;
    };
    let center_x = (bbox.x0 + bbox.x1) / 2.0;
    let center_y = (bbox.y0 + bbox.y1) / 2.0;
    center_x >= table_bbox.x0 && center_x <= table_bbox.x1 && center_y >= table_bbox.y0 && center_y <= table_bbox.y1
}

/// Whether `cells` still carries the words of the elements `table_bbox` covers
/// (xberg-io/xberg#1884).
///
/// Reconstruction can lose a region's words outright: a wrapped label splits across two rows, a
/// value glues onto its label, the interior cells of a sparse row vanish (`73 | | | |` for a line
/// that read `73 4 4 4 4 -`). Removing the region's lines on the strength of a table that no longer
/// carries them deletes those words from the page entirely — they are in neither the paragraphs nor
/// the table. When this returns false, only exact row matches can be removed from the page document.
/// The table is still emitted in `tables`.
///
/// Judged by [`should_adopt_table_rebuild`], the same absolute word-count rule the standalone-image
/// rebuild uses (GH#1599), and against the table's non-empty cell tokens rather than its markdown,
/// whose `|`/`---` syntax would pad the count. ~keep
pub(crate) fn table_retains_region_words(
    elements: &[InternalElement],
    table_bbox: &BoundingBox,
    cells: &[Vec<String>],
) -> bool {
    let region_text = elements
        .iter()
        .filter(|element| element_center_within_table(element, table_bbox))
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let cell_text = cells
        .iter()
        .flatten()
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    should_adopt_table_rebuild(&region_text, &cell_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;

    fn paragraph_with_bbox(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> InternalElement {
        let mut elem = InternalElement::text(ElementKind::Paragraph, text, 0);
        elem.bbox = Some(BoundingBox { x0, y0, x1, y1 });
        elem
    }

    fn bbox(x0: f64, y0: f64, x1: f64, y1: f64) -> BoundingBox {
        BoundingBox { x0, y0, x1, y1 }
    }

    /// One row of cells from whitespace-separated text, for tables that must retain a paragraph's words.
    fn cells_of(text: &str) -> Vec<Vec<String>> {
        vec![text.split_whitespace().map(str::to_string).collect()]
    }

    fn row(cells: &[&str]) -> Vec<Vec<String>> {
        vec![cells.iter().map(|cell| cell.to_string()).collect()]
    }

    fn claims_ignoring_marks(cells: &[&str], line: &str) -> bool {
        ExactTableRows::ignoring_marks(&row(cells)).claim(line.split_whitespace())
    }

    #[test]
    fn rows_ignoring_marks_claim_a_line_with_a_leader_and_a_stray_mark_but_exact_rows_do_not() {
        let cells = row(&["Pool repairs", "14", "140,506"]);
        let line = "Pool repairs ......... 14 = 140,506";

        assert!(!ExactTableRows::new(&cells).claim(line.split_whitespace()));
        assert!(ExactTableRows::ignoring_marks(&cells).claim(line.split_whitespace()));
        assert!(!claims_ignoring_marks(
            &["Pool repairs", "14", "140,506"],
            "Pool repairs ... 15 = 140,506"
        ));
    }

    #[test]
    fn rows_ignoring_marks_keep_signs_brackets_separators_and_dashes_significant() {
        assert!(!claims_ignoring_marks(&["Item", "5"], "Item (5)"));
        assert!(!claims_ignoring_marks(&["Item", "12"], "Item -12"));
        assert!(!claims_ignoring_marks(&["Item", "1,234"], "Item 1.234"));
        assert!(!claims_ignoring_marks(&["Item", "$5"], "Item 5"));
        assert!(!claims_ignoring_marks(&["Item", "\u{2014}", "5"], "Item 5"));
        assert!(claims_ignoring_marks(&["Item", "\u{2014}", "5"], "Item \u{2014} 5"));
        assert!(claims_ignoring_marks(&["Item", "-", "5"], "Item ..... - 5"));
    }

    #[test]
    fn rows_ignoring_marks_never_claim_a_row_made_only_of_leader_marks() {
        assert!(!claims_ignoring_marks(&["....."], "....."));
        assert!(!claims_ignoring_marks(&["=", "..."], "= ..."));
        assert!(!claims_ignoring_marks(&["Item", "5"], "....."));
    }

    #[test]
    fn rows_ignoring_marks_spend_each_identical_row_once() {
        let cells = vec![
            vec!["Fee".to_string(), "5".to_string()],
            vec!["Fee".to_string(), "5".to_string()],
        ];
        let mut rows = ExactTableRows::ignoring_marks(&cells);

        assert!(rows.claim("Fee ..... 5".split_whitespace()));
        assert!(rows.claim("Fee = 5".split_whitespace()));
        assert!(!rows.claim("Fee 5".split_whitespace()));
    }

    #[test]
    fn drop_elements_claimed_by_tables_drops_paragraph_inside_table_bbox() {
        // A paragraph whose bbox is fully inside (so its centre is inside) a detected
        // table's bbox must be removed -- this is the #1571 duplication itself: the
        // paragraph's words are also the table's cells.
        let elements = vec![paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0)];
        let cells = cells_of("Apple 50 10 00");

        let filtered = drop_elements_claimed_by_tables(elements, [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert!(
            filtered.is_empty(),
            "paragraph centred inside the table bbox must be dropped"
        );
    }

    #[test]
    fn drop_elements_claimed_by_tables_keeps_paragraph_adjacent_to_table() {
        // Precision guard (#1571): a paragraph that merely overlaps a table's bbox edge,
        // with its centre outside the bbox, must survive -- the word-centre rule must not
        // over-delete prose that sits next to (not inside) a table.
        let elements = vec![
            paragraph_with_bbox("Vehicle Maintenance Guide", 10.0, 0.0, 90.0, 15.0),
            paragraph_with_bbox("Apple 50 10 00", 10.0, 50.0, 90.0, 70.0),
        ];
        let cells = cells_of("Apple 50 10 00");

        let filtered = drop_elements_claimed_by_tables(elements, [(bbox(0.0, 40.0, 100.0, 140.0), cells.as_slice())]);

        assert_eq!(
            filtered.len(),
            1,
            "only the paragraph centred inside the table bbox should be dropped"
        );
        assert_eq!(filtered[0].text, "Vehicle Maintenance Guide");
    }

    #[test]
    fn drop_elements_claimed_by_tables_keeps_lines_whose_words_the_table_dropped() {
        // xberg-io/xberg#1884: reconstruction lost the interior cells of a sparse row -- the line
        // read "73 4 4 4 4 -" and the table carries only "73". Removing the line would delete those
        // five words from the page: they are in neither the paragraphs nor the table.
        let elements = vec![paragraph_with_bbox("73 4 4 4 4 -", 10.0, 10.0, 90.0, 30.0)];
        let sparse_row = vec![vec![
            "73".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]];

        let filtered =
            drop_elements_claimed_by_tables(elements, [(bbox(0.0, 0.0, 100.0, 100.0), sparse_row.as_slice())]);

        assert_eq!(
            filtered.len(),
            1,
            "a table that dropped the region's words must not claim it"
        );
        assert_eq!(filtered[0].text, "73 4 4 4 4 -");
    }

    #[test]
    fn lossy_table_claims_only_one_exact_line_per_complete_row() {
        let elements = vec![
            paragraph_with_bbox("Item  Count", 10.0, 10.0, 90.0, 20.0),
            paragraph_with_bbox("Apple 50", 10.0, 20.0, 90.0, 30.0),
            paragraph_with_bbox("Apple 50", 10.0, 30.0, 90.0, 40.0),
            paragraph_with_bbox("Pear 61 -", 10.0, 40.0, 90.0, 50.0),
            paragraph_with_bbox("Item Count", 10.0, 110.0, 90.0, 120.0),
        ];
        let cells = ["Item Count", "Apple 50", "Pear 61"].map(|row| cells_of(row).remove(0));

        let filtered = drop_elements_claimed_by_tables(elements, [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert_eq!(
            filtered.iter().map(|element| element.text.as_str()).collect::<Vec<_>>(),
            ["Apple 50", "Pear 61 -", "Item Count"]
        );
    }

    #[test]
    fn drop_elements_claimed_by_tables_claims_only_the_tables_that_retained_their_words() {
        // The decision is per table, not per page (#1884): a faithful table still removes its own
        // region's duplicate lines even when another table on the same page lost words.
        let elements = vec![
            paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0),
            paragraph_with_bbox("Pear 61 11 01", 10.0, 210.0, 90.0, 230.0),
        ];
        let apple = cells_of("Apple 50 10 00");
        let pear = cells_of("Pear");

        let filtered = drop_elements_claimed_by_tables(
            elements,
            [
                (bbox(0.0, 0.0, 100.0, 100.0), apple.as_slice()),
                (bbox(0.0, 200.0, 100.0, 300.0), pear.as_slice()),
            ],
        );

        assert_eq!(filtered.len(), 1, "{filtered:?}");
        assert_eq!(filtered[0].text, "Pear 61 11 01");
    }

    #[test]
    fn drop_elements_claimed_by_tables_is_noop_without_tables() {
        let elements = vec![paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0)];

        let filtered = drop_elements_claimed_by_tables(elements, []);

        assert_eq!(filtered.len(), 1, "no tables detected means nothing should be filtered");
    }

    #[test]
    fn drop_elements_claimed_by_tables_keeps_elements_without_bbox() {
        let mut elem = InternalElement::text(ElementKind::Paragraph, "no geometry", 0);
        elem.bbox = None;
        let cells = vec![vec!["cell".to_string()]];

        let filtered = drop_elements_claimed_by_tables(vec![elem], [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert_eq!(
            filtered.len(),
            1,
            "an element with no bbox cannot be tested against a table and must survive"
        );
    }

    #[test]
    fn drop_document_elements_claimed_by_tables_records_a_page_the_tables_emptied() {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![paragraph_with_bbox("Apple 50 10 00", 10.0, 10.0, 90.0, 30.0)];
        let cells = cells_of("Apple 50 10 00");

        drop_document_elements_claimed_by_tables(&mut document, [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert!(document.elements.is_empty());
        assert!(document.ocr_text_claimed_by_tables);
    }

    #[test]
    fn drop_document_elements_claimed_by_tables_does_not_record_a_page_that_was_already_empty() {
        let mut document = InternalDocument::new("pdf");
        let cells = cells_of("TICKET CP 2 60,000");

        drop_document_elements_claimed_by_tables(&mut document, [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert!(
            !document.ocr_text_claimed_by_tables,
            "a blank pass has no text for the tables to claim"
        );
    }

    #[test]
    fn drop_document_elements_claimed_by_tables_does_not_record_a_page_that_keeps_text() {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![
            paragraph_with_bbox("Vehicle Maintenance Guide", 10.0, 0.0, 90.0, 15.0),
            paragraph_with_bbox("Apple 50 10 00", 10.0, 50.0, 90.0, 70.0),
        ];
        let cells = cells_of("Apple 50 10 00");

        drop_document_elements_claimed_by_tables(&mut document, [(bbox(0.0, 40.0, 100.0, 140.0), cells.as_slice())]);

        assert_eq!(document.elements.len(), 1);
        assert!(!document.ocr_text_claimed_by_tables);
    }

    #[test]
    fn should_reject_table_rebuild_that_drops_words_relative_to_original() {
        let original = "Site inspections this quarter covered fourteen locations across \
            the northern basin and several access roads remain washed out following spring runoff";
        let rebuilt = "| Site | inspections |";

        assert!(
            !should_adopt_table_rebuild(original, rebuilt),
            "a rebuild with far fewer words than the original must not replace it"
        );
    }

    #[test]
    fn should_adopt_table_rebuild_that_retains_or_exceeds_original_word_count() {
        let original = "Name Age\nAlice 30\nBob 40";
        let rebuilt = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 40 |";

        assert!(
            should_adopt_table_rebuild(original, rebuilt),
            "a rebuild that faithfully reformats the same content into a table must still be adopted"
        );
    }
}
