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
    drop_elements_claimed_by_tables_with_matching(elements, tables, false)
}

/// [`drop_elements_claimed_by_tables`] for a route that keeps dot leaders in its cells: a line
/// whose only extra tokens are leader artifacts is still the row the table carries (#2503).
pub(crate) fn drop_elements_claimed_by_tables_ignoring_marks<'a>(
    elements: Vec<InternalElement>,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
) -> Vec<InternalElement> {
    drop_elements_claimed_by_tables_with_matching(elements, tables, true)
}

fn drop_elements_claimed_by_tables_with_matching<'a>(
    elements: Vec<InternalElement>,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
    ignore_marks: bool,
) -> Vec<InternalElement> {
    let mut claiming_bboxes: Vec<BoundingBox> = Vec::new();
    let mut claimed_lines = HashSet::new();
    for (bbox, cells) in tables {
        if table_retains_region_words(&elements, &bbox, cells) {
            claiming_bboxes.push(bbox);
        } else {
            claim_matching_table_rows(&elements, &bbox, cells, &mut claimed_lines, ignore_marks);
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

/// Claim only the lines that are a complete table row, never the table's rectangle.
///
/// A row proof rebuilds the page document with the rows it proved removed (`#2503`), so the
/// surviving lines are the ones it could not match. Judging those few lines as a region against
/// the table's full cell set says the table retained them and promotes it to a bbox claim, which
/// then deletes every line centred in the table -- including one whose words the table lacks. A
/// caller that just ran a proof must use this rows-only claim.
fn drop_elements_matching_table_rows<'a>(
    elements: Vec<InternalElement>,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
    ignore_marks: bool,
) -> Vec<InternalElement> {
    let mut claimed_lines = HashSet::new();
    for (bbox, cells) in tables {
        claim_matching_table_rows(&elements, &bbox, cells, &mut claimed_lines, ignore_marks);
    }
    if claimed_lines.is_empty() {
        return elements;
    }
    elements
        .into_iter()
        .enumerate()
        .filter_map(|(index, element)| (!claimed_lines.contains(&index)).then_some(element))
        .collect()
}

/// A row may replace one single-line element with the same ordered tokens. Spending each
/// row once preserves repeated source lines, and unmatched text stays intact. `ignore_marks`
/// compares leader artifacts as absent for a route that keeps dot leaders in its cells (#2503);
/// every other token still has to match in order.
fn claim_matching_table_rows(
    elements: &[InternalElement],
    bbox: &BoundingBox,
    cells: &[Vec<String>],
    claimed_lines: &mut HashSet<usize>,
    ignore_marks: bool,
) {
    let mut rows = if ignore_marks {
        ExactTableRows::ignoring_marks(cells)
    } else {
        ExactTableRows::new(cells)
    };
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

    /// Rows compared without the leader artifacts a recognizer picks up between a label and its
    /// values. The detected table and the recognized line disagree on those tokens -- a cell
    /// holds `14`, the line reads `....... 14`, or the leader picks up a stray dash as
    /// `.....-.` -- so comparing every token leaves the line in the page next to the row the
    /// table already carries (#2503). Every alphanumeric token, sign, bracket and currency symbol
    /// still has to match in order, so numbers cannot be traded between rows.
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

/// Whether an OCR token is a leader artifact rather than data.
///
/// It has to contain a leader mark and no letter or digit, so `.....`, `=`, `.....-.` and
/// `............--` are dropped while a lone `-`, `$`, `(5)`, `1,234` and `1.234` stay
/// significant (#2783).
fn is_leader_artifact(token: &str) -> bool {
    token.chars().any(is_leader_mark) && !token.chars().any(char::is_alphanumeric)
}

fn row_tokens<'a>(tokens: impl Iterator<Item = &'a str>, ignore_marks: bool) -> String {
    tokens
        .filter(|token| !ignore_marks || !is_leader_artifact(token))
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`drop_elements_claimed_by_tables`] over a page document, recording in
/// `ocr_text_claimed_by_tables` whether it removed every text element of the page. ~keep
pub(crate) fn drop_document_elements_claimed_by_tables<'a>(
    document: &mut InternalDocument,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
    ignore_marks: bool,
) {
    let element_count = document.elements.len();
    let elements = std::mem::take(&mut document.elements);
    document.elements = if ignore_marks {
        drop_elements_claimed_by_tables_ignoring_marks(elements, tables)
    } else {
        drop_elements_claimed_by_tables(elements, tables)
    };
    document.ocr_text_claimed_by_tables |= document.elements.len() < element_count
        && document.elements.iter().all(|element| element.text.trim().is_empty());
}

/// [`drop_elements_matching_table_rows`] over a page document. Call this after a row proof
/// replaced the document; it never promotes the table to a bbox claim (#2503).
pub(crate) fn drop_document_elements_matching_table_rows<'a>(
    document: &mut InternalDocument,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
    ignore_marks: bool,
) {
    let element_count = document.elements.len();
    let elements = std::mem::take(&mut document.elements);
    document.elements = drop_elements_matching_table_rows(elements, tables, ignore_marks);
    document.ocr_text_claimed_by_tables |= document.elements.len() < element_count
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

    fn claims(cells: &[&str], line: &str) -> bool {
        ExactTableRows::ignoring_marks(&row(cells)).claim(line.split_whitespace())
    }

    #[test]
    fn rows_claim_a_line_with_a_leader_and_a_stray_mark_but_exact_rows_do_not() {
        let cells = row(&["Pool repairs", "14", "140,506"]);
        let line = "Pool repairs ......... 14 = 140,506";

        assert!(!ExactTableRows::new(&cells).claim(line.split_whitespace()));
        assert!(claims(&["Pool repairs", "14", "140,506"], line));
        assert!(!claims(
            &["Pool repairs", "14", "140,506"],
            "Pool repairs ... 15 = 140,506"
        ));
    }

    #[test]
    fn rows_claim_a_line_whose_leader_picked_up_a_dash() {
        assert!(claims(
            &["Storm drains", "36", "845.60", "338,224", "331,760", "(6,464)"],
            "Storm drains ............-. 36 845.60 338,224 331,760 (6,464)"
        ));
        assert!(claims(
            &["Code review", "25", "1,510.35"],
            "Code review ............-- 25 1,510.35"
        ));
    }

    #[test]
    fn rows_keep_signs_brackets_separators_and_dashes_significant() {
        assert!(!claims(&["Item", "5"], "Item (5)"));
        assert!(!claims(&["Item", "12"], "Item -12"));
        assert!(!claims(&["Item", "1,234"], "Item 1.234"));
        assert!(!claims(&["Item", "$5"], "Item 5"));
        assert!(!claims(&["Item", "\u{2014}", "5"], "Item 5"));
        assert!(claims(&["Item", "\u{2014}", "5"], "Item \u{2014} 5"));
        assert!(claims(&["Item", "-", "5"], "Item ..... - 5"));
    }

    #[test]
    fn rows_never_claim_a_row_made_only_of_leader_marks() {
        assert!(!claims(&["....."], "....."));
        assert!(!claims(&["=", "..."], "= ..."));
        assert!(!claims(&["Item", "5"], "....."));
    }

    #[test]
    fn rows_claim_a_line_when_the_table_split_its_cells_differently() {
        // The detector and the recognized line disagree on where the cells end; the ordered
        // tokens are the same, so the row is still the one the line carries (#2503).
        let cells = vec![vec![
            "Parks and".to_string(),
            "trails 14".to_string(),
            "1,204.50".to_string(),
            "628,382".to_string(),
            "544,118".to_string(),
            "(84,264)".to_string(),
        ]];
        let mut rows = ExactTableRows::ignoring_marks(&cells);

        assert!(rows.claim("Parks and trails 14 1,204.50 628,382 544,118 (84,264)".split_whitespace()));
        assert!(!rows.claim("Parks and trails 14 1,204.50 628,382 544,118 (84,265)".split_whitespace()));
    }

    #[test]
    fn rows_do_not_claim_a_line_with_an_extra_number() {
        let cells = row(&["Parks and trails", "14", "1,204.50"]);
        let mut rows = ExactTableRows::ignoring_marks(&cells);

        assert!(!rows.claim("Parks and trails 14 1,204.50 628,382".split_whitespace()));
    }

    #[test]
    fn rows_spend_each_identical_row_once() {
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
    fn leader_aware_claim_removes_a_line_whose_only_extra_tokens_are_leaders() {
        // A six-column scan at PSM 6 keeps short leader runs the table cell does not carry. The
        // table carries every word of the line, so the line is the row's duplicate (#2503).
        let elements = vec![
            paragraph_with_bbox(
                "Transit passes .. 40 176,798 223.86 87,244 872.97",
                10.0,
                10.0,
                90.0,
                30.0,
            ),
            paragraph_with_bbox(
                "Code review ... 84 847,072 (2,106) 74,261 946.49",
                10.0,
                30.0,
                90.0,
                50.0,
            ),
        ];
        let cells = [
            "Transit passes 40 176,798 223.86 87,244 872.97",
            "Code review 84 847,072 (2,106) 74,261 946.49",
        ]
        .map(|row| cells_of(row).remove(0));

        let table = [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())];
        let filtered = drop_elements_claimed_by_tables_ignoring_marks(elements.clone(), table);
        assert!(filtered.is_empty(), "leader rows are the table's own rows");

        let exact = drop_elements_claimed_by_tables(elements, table);
        assert_eq!(
            exact.len(),
            2,
            "the exact comparison keeps both lines, so the new path is what claims them"
        );
    }

    #[test]
    fn leader_aware_claim_keeps_a_row_whose_value_differs() {
        let elements = vec![paragraph_with_bbox(
            "Transit passes .. 40 176,798 223.86 87,244 872.97",
            10.0,
            10.0,
            90.0,
            30.0,
        )];
        let cells = ["Transit passes 40 176,798 223.86 87,244 872.98"].map(|row| cells_of(row).remove(0));

        let filtered = drop_elements_claimed_by_tables_ignoring_marks(
            elements,
            [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())],
        );

        assert_eq!(filtered.len(), 1, "a value the table does not carry must keep its line");
    }

    #[test]
    fn rows_only_claim_keeps_a_leftover_line_the_table_does_not_carry() {
        // After a row proof the document holds only its leftovers: one row whose short leader the
        // cell lacks, and one line with a value the table lost. Comparing those few leftovers as a
        // region against the full cell set says the table retained them and promotes it to a bbox
        // claim, which would delete the unmatched line too (#2503).
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![
            paragraph_with_bbox("Transit passes .. 40 176,798", 10.0, 10.0, 90.0, 30.0),
            paragraph_with_bbox("Snow removal 63 195,184", 10.0, 40.0, 90.0, 60.0),
        ];
        let cells = [
            "Transit passes 40 176,798",
            "Snow removal 63 195,181",
            "Code review 84 847,072",
        ]
        .map(|row| cells_of(row).remove(0));
        let table = [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())];

        let whole_region = drop_elements_claimed_by_tables_ignoring_marks(document.elements.clone(), table);
        assert!(
            whole_region.is_empty(),
            "the region claim would delete the line whose value the table lost"
        );

        drop_document_elements_matching_table_rows(&mut document, table, true);

        assert_eq!(document.elements.len(), 1);
        assert_eq!(document.elements[0].text, "Snow removal 63 195,184");
        assert!(!document.ocr_text_claimed_by_tables);
    }

    #[test]
    fn exact_claim_keeps_a_line_whose_only_extra_token_is_a_leader() {
        // The non-leader route keeps the extra leader token: only a leader-aware caller drops it.
        let elements = vec![paragraph_with_bbox("Apple 50 ..... 10 00", 10.0, 10.0, 90.0, 30.0)];
        let cells = ["Apple 50 10 00"].map(|row| cells_of(row).remove(0));

        let filtered = drop_elements_claimed_by_tables(elements, [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())]);

        assert_eq!(
            filtered.len(),
            1,
            "the exact comparison must keep the extra leader token"
        );
    }

    #[test]
    fn a_claim_never_clears_a_page_already_recorded_as_claimed() {
        let cells = cells_of("Apple 50 10 00");
        let table = [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())];

        let mut document = InternalDocument::new("pdf");
        document.ocr_text_claimed_by_tables = true;
        drop_document_elements_matching_table_rows(&mut document, table, true);
        assert!(document.ocr_text_claimed_by_tables);

        let mut document = InternalDocument::new("pdf");
        document.ocr_text_claimed_by_tables = true;
        drop_document_elements_claimed_by_tables(&mut document, table, false);
        assert!(document.ocr_text_claimed_by_tables);
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

        drop_document_elements_claimed_by_tables(
            &mut document,
            [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())],
            false,
        );

        assert!(document.elements.is_empty());
        assert!(document.ocr_text_claimed_by_tables);
    }

    #[test]
    fn drop_document_elements_claimed_by_tables_does_not_record_a_page_that_was_already_empty() {
        let mut document = InternalDocument::new("pdf");
        let cells = cells_of("TICKET CP 2 60,000");

        drop_document_elements_claimed_by_tables(
            &mut document,
            [(bbox(0.0, 0.0, 100.0, 100.0), cells.as_slice())],
            false,
        );

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

        drop_document_elements_claimed_by_tables(
            &mut document,
            [(bbox(0.0, 40.0, 100.0, 140.0), cells.as_slice())],
            false,
        );

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
