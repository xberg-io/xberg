//! Reconciles a page's OCR text elements with the tables detected on the same page, shared by
//! every OCR backend that returns both.

use std::collections::HashMap;

use crate::table_core::HocrWord;
use crate::types::extraction::BoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrBoundingGeometry, OcrElementLevel};

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

/// Give the words of table candidates that the shape rule rejected back to the page document.
///
/// The table path reads every word of the page, but the page document does not: the dictionary
/// noise filter drops a whole line of unreadable words (#783). Before the shape rule such a line
/// still reached the output inside a table cell. Once the rule keeps the region as text, the line
/// would sit in neither the text nor a table. A word of a rejected region that no element over the
/// region carries comes back as a text line, placed by its height on the page. Returns whether a
/// line was added. ~keep
pub(crate) fn restore_words_of_rejected_regions(
    document: &mut InternalDocument,
    regions: &[Vec<HocrWord>],
    page: u32,
) -> bool {
    let mut restored = false;
    for words in regions {
        let lost = words_missing_from_elements(&document.elements, words);
        for line in group_into_lines(lost) {
            insert_line(document, &line, page);
            restored = true;
        }
    }
    if restored {
        document.ocr_text_claimed_by_tables = false;
    }
    restored
}

/// The letters and digits of a word in lower case: the form that two spellings of one word share.
fn word_key(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The words of a region that the elements over the region do not carry, in reading order. Each
/// element word is spent once, so a word that repeats in the region is restored as often as it is
/// missing. A word with no letter or digit is never restored.
fn words_missing_from_elements<'a>(elements: &[InternalElement], words: &'a [HocrWord]) -> Vec<&'a HocrWord> {
    let Some(region) = words_bounding_box(words) else {
        return Vec::new();
    };
    let mut carried: HashMap<String, usize> = HashMap::new();
    for element in elements {
        if element.bbox.as_ref().is_some_and(|bbox| boxes_overlap(bbox, &region)) {
            for token in element.text.split_whitespace() {
                *carried.entry(word_key(token)).or_default() += 1;
            }
        }
    }
    words
        .iter()
        .filter(|word| {
            let key = word_key(&word.text);
            if key.is_empty() {
                return false;
            }
            match carried.get_mut(&key) {
                Some(count) if *count > 0 => {
                    *count -= 1;
                    false
                }
                _ => true,
            }
        })
        .collect()
}

fn words_bounding_box<'a>(words: impl IntoIterator<Item = &'a HocrWord>) -> Option<BoundingBox> {
    words.into_iter().fold(None, |extent: Option<BoundingBox>, word| {
        let (x0, y0) = (f64::from(word.left), f64::from(word.top));
        let (x1, y1) = (x0 + f64::from(word.width), y0 + f64::from(word.height));
        Some(match extent {
            None => BoundingBox { x0, y0, x1, y1 },
            Some(e) => BoundingBox {
                x0: e.x0.min(x0),
                y0: e.y0.min(y0),
                x1: e.x1.max(x1),
                y1: e.y1.max(y1),
            },
        })
    })
}

fn boxes_overlap(a: &BoundingBox, b: &BoundingBox) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

/// Split consecutive words into lines: a word joins the line when its centre is within half a
/// word height of the line's first word.
fn group_into_lines(words: Vec<&HocrWord>) -> Vec<Vec<&HocrWord>> {
    let centre = |word: &HocrWord| word.top + word.height / 2;
    let mut lines: Vec<Vec<&HocrWord>> = Vec::new();
    for word in words {
        let joins = |line: &&mut Vec<&HocrWord>| {
            let first = line[0];
            centre(first).abs_diff(centre(word)) <= first.height.max(word.height) / 2
        };
        match lines.last_mut().filter(joins) {
            Some(line) => line.push(word),
            None => lines.push(vec![word]),
        }
    }
    lines
}

/// Add `line` as one text element, before the first element that sits lower on the page.
fn insert_line(document: &mut InternalDocument, line: &[&HocrWord], page: u32) {
    let Some(extent) = words_bounding_box(line.iter().copied()) else {
        return;
    };
    let text = line.iter().map(|word| word.text.trim()).collect::<Vec<_>>().join(" ");
    let centre = (extent.y0 + extent.y1) / 2.0;
    let index = document
        .elements
        .iter()
        .position(|element| {
            element
                .bbox
                .as_ref()
                .is_some_and(|bbox| (bbox.y0 + bbox.y1) / 2.0 > centre)
        })
        .unwrap_or(document.elements.len());
    let page = document
        .elements
        .iter()
        .find_map(|element| element.page)
        .unwrap_or(page);
    let mut element = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Block,
        },
        text,
        0,
    )
    .with_page(page)
    .with_index(index as u32);
    element.ocr_geometry = Some(OcrBoundingGeometry::Rectangle {
        left: extent.x0 as u32,
        top: extent.y0 as u32,
        width: (extent.x1 - extent.x0) as u32,
        height: (extent.y1 - extent.y0) as u32,
    });
    element.bbox = Some(extent);
    document.elements.insert(index, element);
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn word(text: &str, left: u32, top: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width: 100,
            height: 20,
            confidence: 90.0,
        }
    }

    fn page_texts(document: &InternalDocument) -> Vec<&str> {
        document.elements.iter().map(|element| element.text.as_str()).collect()
    }

    fn page_with_two_lines() -> InternalDocument {
        let mut document = InternalDocument::new("ocr");
        document.push_element(paragraph_with_bbox("Alpha Beta", 0.0, 0.0, 300.0, 20.0));
        document.push_element(paragraph_with_bbox("Omega", 0.0, 200.0, 300.0, 220.0));
        document
    }

    #[test]
    fn restore_words_of_rejected_regions_returns_a_line_the_page_dropped_in_reading_order() {
        let mut document = page_with_two_lines();
        let region = vec![
            word("Alpha", 0, 0),
            word("Beta", 120, 0),
            word("gamma", 0, 100),
            word("delta", 120, 100),
            word("Omega", 0, 200),
        ];

        assert!(restore_words_of_rejected_regions(&mut document, &[region], 1));

        assert_eq!(page_texts(&document), ["Alpha Beta", "gamma delta", "Omega"]);
        let restored = &document.elements[1];
        assert_eq!(restored.page, Some(1));
        assert_eq!(
            restored.bbox.as_ref().map(|b| (b.x0, b.y0, b.x1, b.y1)),
            Some((0.0, 100.0, 220.0, 120.0))
        );
    }

    #[test]
    fn restore_words_of_rejected_regions_leaves_a_page_that_carries_every_word() {
        let mut document = page_with_two_lines();
        document.ocr_text_claimed_by_tables = true;
        let region = vec![
            word("alpha,", 0, 0),
            word("BETA", 120, 0),
            word("--", 240, 0),
            word("Omega", 0, 200),
        ];

        assert!(!restore_words_of_rejected_regions(&mut document, &[region], 1));

        assert_eq!(page_texts(&document), ["Alpha Beta", "Omega"]);
        assert!(
            document.ocr_text_claimed_by_tables,
            "an untouched page keeps its claim flag"
        );
    }

    #[test]
    fn restore_words_of_rejected_regions_counts_a_repeated_word_once_per_copy() {
        let mut document = InternalDocument::new("ocr");
        document.push_element(paragraph_with_bbox("Total", 0.0, 0.0, 300.0, 20.0));
        let region = vec![word("Total", 0, 0), word("Total", 120, 0)];

        assert!(restore_words_of_rejected_regions(&mut document, &[region], 1));

        assert_eq!(page_texts(&document), ["Total", "Total"]);
    }

    #[test]
    fn restore_words_of_rejected_regions_ignores_an_element_far_from_the_region() {
        let mut document = InternalDocument::new("ocr");
        document.push_element(paragraph_with_bbox("gamma", 0.0, 900.0, 300.0, 920.0));
        let region = vec![word("gamma", 0, 0), word("delta", 120, 0)];

        assert!(restore_words_of_rejected_regions(&mut document, &[region], 1));

        assert_eq!(page_texts(&document), ["gamma delta", "gamma"]);
    }

    #[test]
    fn restore_words_of_rejected_regions_clears_the_claim_flag_of_a_page_it_refills() {
        let mut document = InternalDocument::new("ocr");
        document.ocr_text_claimed_by_tables = true;

        assert!(restore_words_of_rejected_regions(
            &mut document,
            &[vec![word("gamma", 0, 0)]],
            3
        ));

        assert_eq!(page_texts(&document), ["gamma"]);
        assert_eq!(document.elements[0].page, Some(3));
        assert!(!document.ocr_text_claimed_by_tables);
    }

    #[test]
    fn restore_words_of_rejected_regions_is_a_noop_without_regions() {
        let mut document = page_with_two_lines();

        assert!(!restore_words_of_rejected_regions(&mut document, &[], 1));
        assert!(!restore_words_of_rejected_regions(&mut document, &[Vec::new()], 1));
        assert_eq!(page_texts(&document), ["Alpha Beta", "Omega"]);
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
