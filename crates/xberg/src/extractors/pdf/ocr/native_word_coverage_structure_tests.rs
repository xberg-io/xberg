use super::*;
use crate::types::internal::{ElementKind, InternalElement};

fn operand_table_page() -> InternalDocument {
    let mut doc = super::super::document::flat_ocr_page_document("Instruction Encoding");
    let index = doc.push_table(crate::types::Table {
        cells: vec![vec!["Operand".into(), "1".into()]],
        markdown: "| Operand | 1 |".into(),
        page_number: 1,
        ..Default::default()
    });
    doc.elements
        .push(InternalElement::text(ElementKind::Table { table_index: index }, "", 0));
    doc
}

const CERTIFIED_READ: &str = "Instruction Encoding Header | Operand | 1 |";

#[test]
fn certified_read_may_retain_native_words_omitted_by_table_structure() {
    let mut doc = operand_table_page();
    assert!(!only_segments_certified_tesseract_text_and_tables(&doc, CERTIFIED_READ));
    assert!(uses_only_certified_tesseract_text_and_tables(&doc, CERTIFIED_READ));
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &doc,
        "Instruction Encoding Header Operand"
    ));
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &doc,
        "Instruction Header Operand 1"
    ));
    let mut extra_occurrence = doc.clone();
    extra_occurrence
        .elements
        .push(InternalElement::text(ElementKind::Paragraph, "Operand", 0));
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &extra_occurrence,
        CERTIFIED_READ
    ));
    doc.elements.clear();
    doc.tables[0].cells = vec![vec![String::new()]];
    doc.tables[0].markdown = "| |".into();
    assert!(!uses_only_certified_tesseract_text_and_tables(&doc, CERTIFIED_READ));
}

#[test]
fn ink_certified_table_structure_still_rejects_extra_payload() {
    let doc = operand_table_page();
    let mut extra_asset = doc.clone();
    extra_asset.images.push(Default::default());
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &extra_asset,
        CERTIFIED_READ
    ));
    let mut explicit_words = doc.clone();
    explicit_words.prebuilt_ocr_elements = Some(Vec::new());
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &explicit_words,
        CERTIFIED_READ
    ));
    let mut extra_metadata = doc.clone();
    extra_metadata
        .metadata
        .additional
        .insert("extra".into(), serde_json::json!(true));
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &extra_metadata,
        CERTIFIED_READ
    ));
    let mut extra_cell = doc;
    extra_cell.tables[0].cells[0].push("Extra".into());
    assert!(!uses_only_certified_tesseract_text_and_tables(
        &extra_cell,
        CERTIFIED_READ
    ));
}

#[test]
fn clean_native_ink_accepts_structural_body_word_omissions() {
    let mut doc = operand_table_page();
    doc.elements[0].kind = ElementKind::Paragraph;
    doc.elements[0].text = "The paragraph".into();
    let read = "The body paragraph | Operand | 1 |";
    assert!(!only_segments_certified_tesseract_text_and_tables(&doc, read));
    // The full read's ink proof covers these body words; the native page is the output. ~keep
    assert!(ink_owned_table_structure_can_yield_to_native(&doc, read, Some(read)));
    let mut uncertified_body = doc;
    uncertified_body
        .elements
        .push(InternalElement::text(ElementKind::Paragraph, "Uncertified", 0));
    assert!(!ink_owned_table_structure_can_yield_to_native(
        &uncertified_body,
        read,
        Some(read)
    ));
}

#[test]
fn lost_glyph_or_late_only_ink_ownership_cannot_omit_a_body_word() {
    let mut doc = operand_table_page();
    doc.elements[0].kind = ElementKind::Paragraph;
    doc.elements[0].text = "The paragraph".into();
    let read = "The body paragraph | Operand | 1 |";
    let lost_layer = format!("{read}{}", crate::extractors::pdf::unmapped_glyphs::LOST_GLYPH_MARKER);
    assert!(!ink_owned_table_structure_can_yield_to_native(
        &doc,
        read,
        Some(&lost_layer)
    ));
    assert!(!ink_owned_table_structure_can_yield_to_native(&doc, read, None));
    let exact_read = "The paragraph | Operand | 1 |";
    assert!(ink_owned_table_structure_can_yield_to_native(
        &doc,
        exact_read,
        Some(&lost_layer)
    ));
    assert!(ink_owned_table_structure_can_yield_to_native(&doc, exact_read, None));
}

/// The structure a Type0 read builds that the native page replaces whole (the structure rule of the
/// Type0 keep rule). Each test below changes one thing about a page the rule accepts, so removing
/// one clause of the rule turns the test that names it red.
mod type0_structure {
    use super::*;
    use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};

    const PAGE: u32 = 1;

    fn text_page() -> InternalDocument {
        super::super::super::document::flat_ocr_page_document("The orchard harvest includes apples")
    }

    fn with_table(mut doc: InternalDocument) -> InternalDocument {
        let table_index = doc.push_table(crate::types::Table {
            cells: vec![vec!["Crates".into(), "42".into()]],
            markdown: "| Crates | 42 |".into(),
            page_number: PAGE,
            ..Default::default()
        });
        doc.elements
            .push(InternalElement::text(ElementKind::Table { table_index }, "", 0));
        doc
    }

    fn with_list(mut doc: InternalDocument) -> InternalDocument {
        doc.elements
            .push(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
        doc.elements.push(InternalElement::text(
            ElementKind::ListItem { ordered: false },
            "Crates",
            1,
        ));
        doc.elements.push(InternalElement::text(ElementKind::ListEnd, "", 0));
        doc
    }

    fn table_and_list_page() -> InternalDocument {
        with_list(with_table(text_page()))
    }

    fn structured(doc: &InternalDocument) -> bool {
        only_structures_certified_tesseract_text(doc)
    }

    fn last_element(doc: &mut InternalDocument) -> &mut InternalElement {
        doc.elements.last_mut().unwrap()
    }

    #[test]
    fn a_table_with_ocr_only_cells_prevents_shape_only_native_replacement() {
        let doc = table_and_list_page();
        assert!(!structured(&doc));
        assert!(!loss_page_yields(&doc, &[PAGE]));
    }

    #[test]
    fn an_ocr_table_without_markdown_still_prevents_native_replacement() {
        let mut doc = with_table(text_page());
        doc.tables[0].markdown.clear();
        assert!(!structured(&doc));
        assert!(!loss_page_yields(&doc, &[PAGE]));
    }

    #[test]
    fn a_read_built_into_text_and_a_list_keeps_the_native_page() {
        assert!(structured(&with_list(text_page())));
    }

    #[test]
    fn a_read_built_into_any_other_kind_of_element_does_not_keep_the_native_page() {
        for kind in [ElementKind::Code, ElementKind::QuoteStart, ElementKind::Formula] {
            let mut doc = with_list(text_page());
            doc.elements.push(InternalElement::text(kind, "apples", 0));
            assert!(!structured(&doc), "{kind:?}");
        }
    }

    #[test]
    fn a_read_with_an_image_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        assert!(structured(&doc));
        doc.images.push(Default::default());
        assert!(!structured(&doc));
    }

    #[test]
    fn a_read_with_a_link_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        assert!(structured(&doc));
        doc.uris.push(crate::types::uri::ExtractedUri {
            url: "https://example.org/".into(),
            label: None,
            page: Some(PAGE),
            kind: crate::types::uri::UriKind::Hyperlink,
        });
        assert!(!structured(&doc));
    }

    #[test]
    fn a_read_with_metadata_beyond_the_read_bookkeeping_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        doc.metadata
            .additional
            .insert("title".into(), serde_json::json!("Orchard report"));
        assert!(!structured(&doc));
    }

    #[test]
    fn zero_table_bookkeeping_allows_shape_only_native_replacement() {
        let mut doc = with_list(text_page());
        for (key, value) in [
            ("word_count", serde_json::json!(5)),
            ("table_count", serde_json::json!(0)),
            ("tables_detected", serde_json::json!(0)),
            (
                crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY,
                serde_json::json!(["6"]),
            ),
        ] {
            doc.metadata.additional.insert(key.into(), value);
            assert!(structured(&doc), "{key}");
        }
    }

    #[test]
    fn bookkeeping_with_a_value_outside_its_range_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        doc.metadata
            .additional
            .insert("table_count".into(), serde_json::json!("many"));
        assert!(!structured(&doc));
    }

    #[test]
    fn a_read_with_no_element_does_not_keep_the_native_page() {
        let mut doc = text_page();
        doc.elements.clear();
        assert!(!structured(&doc));
        assert!(structured(&text_page()));
    }

    #[test]
    fn a_list_outside_the_body_does_not_keep_the_native_page() {
        for layer in [ContentLayer::Header, ContentLayer::Footer, ContentLayer::Footnote] {
            let mut doc = with_list(text_page());
            last_element(&mut doc).layer = layer;
            assert!(!structured(&doc), "{layer:?}");
        }
    }

    #[test]
    fn positive_table_bookkeeping_prevents_shape_only_native_replacement() {
        for key in ["table_count", "tables_detected"] {
            let mut doc = with_list(text_page());
            doc.metadata.additional.insert(key.into(), serde_json::json!(1));
            assert!(!structured(&doc), "{key}");
        }
    }

    #[test]
    fn an_annotated_list_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        last_element(&mut doc).annotations.push(TextAnnotation {
            start: 0,
            end: 1,
            kind: AnnotationKind::Bold,
        });
        assert!(!structured(&doc));
    }

    #[test]
    fn a_read_with_word_elements_of_its_own_does_not_keep_the_native_page() {
        let mut doc = with_list(text_page());
        doc.prebuilt_ocr_elements = Some(Vec::new());
        assert!(!structured(&doc));
    }

    fn loss_page_yields(doc: &InternalDocument, loss_pages: &[u32]) -> bool {
        let none = ahash::AHashSet::new();
        let loss: ahash::AHashSet<u32> = loss_pages.iter().copied().collect();
        final_ocr_structure_can_yield_to_native(
            PAGE,
            doc,
            "The orchard harvest includes apples Crates 42",
            FinalOcrStructureYieldSets {
                native_words_covered_pages: &none,
                native_ink_owned_pages: &none,
                native_coverage_layers: None,
                clean_native_ink_pages: &std::collections::HashSet::new(),
                type0_loss_pages: &loss,
            },
        )
    }

    #[test]
    fn a_page_the_type0_rule_kept_yields_its_structure_to_the_native_page() {
        assert!(loss_page_yields(&with_list(text_page()), &[PAGE]));
    }

    #[test]
    fn a_type0_loss_page_retains_its_ocr_table_even_if_ink_ownership_allows_yielding() {
        let doc = operand_table_page();
        let read = "Instruction Encoding | Operand | 1 |";
        let none = ahash::AHashSet::new();
        let page = [PAGE].into_iter().collect();
        let yields = |loss| {
            final_ocr_structure_can_yield_to_native(
                PAGE,
                &doc,
                read,
                FinalOcrStructureYieldSets {
                    native_words_covered_pages: &none,
                    native_ink_owned_pages: &page,
                    native_coverage_layers: None,
                    clean_native_ink_pages: &std::collections::HashSet::new(),
                    type0_loss_pages: loss,
                },
            )
        };
        assert!(yields(&none));
        assert!(!yields(&page));
    }

    #[test]
    fn a_page_the_type0_rule_did_not_judge_keeps_its_structure() {
        let doc = with_list(text_page());
        assert!(!loss_page_yields(&doc, &[]));
        assert!(
            !loss_page_yields(&doc, &[PAGE + 1]),
            "another page's verdict is not this page's"
        );
    }

    #[test]
    fn a_page_the_type0_rule_kept_still_keeps_a_structure_with_an_image() {
        let mut doc = with_list(text_page());
        doc.images.push(Default::default());
        assert!(!loss_page_yields(&doc, &[PAGE]));
    }
}
