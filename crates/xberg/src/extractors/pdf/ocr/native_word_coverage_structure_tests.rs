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
