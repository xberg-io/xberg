use super::synchronize_plain_pages;
use crate::OutputFormat;
use crate::extraction::derive::{derive_extraction_result, derive_renderer_input};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractedDocument, PageContent, PageStructure, PageUnitType};

fn recovered_document() -> InternalDocument {
    let mut document = InternalDocument::new("pdf");
    for (page, text) in [(1, "Page 1 café"), (2, "Total O ១២៣"), (2, "paid")] {
        document.push_element(InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page));
    }
    document
}

fn intermediate_reads() -> [String; 2] {
    ["Page 1 café".into(), "Total O ១២៣\n\n\npaid".into()]
}

fn attach_page_handoff(document: &mut InternalDocument, reads: &[String]) -> String {
    // ~keep The OCR pipeline joins reads with two newlines; the PDF extractor carries these reads as prebuilt pages.
    let text = reads.join("\n\n");
    document.metadata.pages = Some(PageStructure {
        total_count: 2,
        unit_type: PageUnitType::Page,
        boundaries: super::super::super::exact_boundaries_for_page_texts(&text, reads),
        pages: None,
    });
    document.prebuilt_pages = Some(
        reads
            .iter()
            .enumerate()
            .map(|(index, content)| PageContent {
                page_number: index as u32 + 1,
                content: content.clone(),
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
            })
            .collect(),
    );
    text
}

fn synchronized_document() -> InternalDocument {
    let mut document = recovered_document();
    let mut reads = intermediate_reads();
    assert!(synchronize_plain_pages(
        &document,
        &[false, true],
        &mut reads,
        &OutputFormat::Plain,
    ));
    assert_eq!(reads, ["Page 1 café", "Total O ១២៣\n\npaid"]);
    assert_eq!(
        attach_page_handoff(&mut document, &reads),
        "Page 1 café\n\nTotal O ១២៣\n\npaid",
    );
    document
}

fn assert_final_page_ranges(result: &ExtractedDocument) {
    assert_eq!(result.content, "Page 1 café\n\nTotal O ១២៣\n\npaid");
    let pages = result.pages.as_ref().expect("both OCR pages must be published");
    assert_eq!(
        pages
            .iter()
            .map(|page| (page.page_number, page.content.as_str()))
            .collect::<Vec<_>>(),
        [(1, "Page 1 café"), (2, "Total O ១២៣\n\npaid")],
    );
    let structure = result.metadata.pages.as_ref().expect("page metadata must survive");
    assert_eq!(structure.total_count, 2);
    assert_eq!(structure.unit_type, PageUnitType::Page);
    let boundaries = structure.boundaries.as_ref().expect("valid page ranges must survive");
    assert_eq!(
        boundaries
            .iter()
            .map(|range| (range.page_number, range.byte_start, range.byte_end))
            .collect::<Vec<_>>(),
        [(1, 0, 12), (2, 14, 37)],
    );
    for range in boundaries {
        assert!(range.byte_start <= range.byte_end);
        assert!(range.byte_end <= result.content.len());
        assert!(result.content.is_char_boundary(range.byte_start));
        assert!(result.content.is_char_boundary(range.byte_end));
    }
    // ~keep Page ranges exclude the inter-page separator, so their only gap must be those exact two bytes.
    assert_eq!(boundaries[0].byte_end + 2, boundaries[1].byte_start);
    assert_eq!(result.content.get(12..14), Some("\n\n"));
    assert_eq!(result.content.get(0..12), Some("Page 1 café"));
    assert_eq!(result.content.get(14..37), Some("Total O ១២៣\n\npaid"));
    assert_eq!(boundaries[1].byte_end, result.content.len());
    assert!(
        result
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "page_boundaries"),
        "synchronized page ranges must not be discarded: {:?}",
        result.processing_warnings,
    );
}

#[test]
fn synchronized_second_page_survives_extraction_result_derivation() {
    let result = derive_extraction_result(synchronized_document(), false, OutputFormat::Plain);
    assert_final_page_ranges(&result);
}

#[test]
fn synchronized_second_page_survives_renderer_input_derivation() {
    let result = derive_renderer_input(synchronized_document());
    assert_final_page_ranges(&result);
}

#[test]
fn plain_derivation_repairs_an_unsynchronized_prebuilt_page() {
    let mut document = recovered_document();
    assert_eq!(
        attach_page_handoff(&mut document, &intermediate_reads()),
        "Page 1 café\n\nTotal O ១២៣\n\n\npaid",
    );
    let result = derive_extraction_result(document, false, OutputFormat::Plain);
    assert_final_page_ranges(&result);
}

#[test]
fn synchronization_keeps_the_unrecovered_first_page_read() {
    let mut document = recovered_document();
    document.elements[0].text = "Different IR text on an unrecovered page".into();
    let mut reads = intermediate_reads();
    assert!(synchronize_plain_pages(
        &document,
        &[false, true],
        &mut reads,
        &OutputFormat::Plain,
    ));
    assert_eq!(reads, ["Page 1 café", "Total O ១២៣\n\npaid"]);
}
