//! Reconciles a page's OCR text elements with the tables detected on the same page, shared by
//! every OCR backend that returns both.

use crate::types::extraction::BoundingBox;
use crate::types::internal::InternalElement;

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
/// A table only claims its region when it kept the region's words -- see
/// [`table_retains_region_words`] (xberg-io/xberg#1884).
pub(crate) fn drop_elements_claimed_by_tables<'a>(
    elements: Vec<InternalElement>,
    tables: impl IntoIterator<Item = (BoundingBox, &'a [Vec<String>])>,
) -> Vec<InternalElement> {
    let mut claiming_bboxes: Vec<BoundingBox> = Vec::new();
    for (bbox, cells) in tables {
        if table_retains_region_words(&elements, &bbox, cells) {
            claiming_bboxes.push(bbox);
        } else {
            tracing::warn!(
                target: "xberg::ocr::tables",
                left = bbox.x0,
                top = bbox.y0,
                right = bbox.x1,
                bottom = bbox.y1,
                table_rows = cells.len(),
                "reconstructed table dropped words from its region; keeping the region's lines in the page document"
            );
        }
    }
    if claiming_bboxes.is_empty() {
        return elements;
    }

    elements
        .into_iter()
        .filter(|element| {
            !claiming_bboxes
                .iter()
                .any(|table_bbox| element_center_within_table(element, table_bbox))
        })
        .collect()
}

/// Whether `element`'s bbox centre lies inside `table_bbox`.
///
/// Shared by [`drop_elements_claimed_by_tables`] and [`table_retains_region_words`] so a table is
/// judged for word retention against exactly the elements it would remove — two copies of this test
/// could disagree and make the retention check answer about a different set of lines than the filter
/// then deletes. An element with no geometry is not covered by any table. ~keep
fn element_center_within_table(element: &InternalElement, table_bbox: &BoundingBox) -> bool {
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
/// the table. When this returns false the lines stay and the table is still emitted in `tables`, so
/// the structured form is never lost either; the cost is the #1571 duplication for that one region,
/// which is strictly better than losing content.
///
/// Judged by [`should_adopt_table_rebuild`], the same absolute word-count rule the standalone-image
/// rebuild uses (GH#1599), and against the table's non-empty cell tokens rather than its markdown,
/// whose `|`/`---` syntax would pad the count. ~keep
fn table_retains_region_words(elements: &[InternalElement], table_bbox: &BoundingBox, cells: &[Vec<String>]) -> bool {
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
