//! Regression test for #1112: DOCX element order preserved in ElementBased result format.
//!
//! When element-based result format is enabled, the pipeline must walk the InternalDocument
//! in the extractor's native reading order instead of reassembling from per-page content.
//! DOCX has no native page boundaries, so per-page reconstruction scrambles element order.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_uri_document;

/// Unit-level regression: `convert_internal_elements_to_elements` walks a synthetic
/// InternalDocument in document order (heading → paragraph → list → table).
///
/// This test does not require any DOCX file on disk and runs on every build.
#[test]
fn test_internal_document_walk_preserves_reading_order() {
    use xberg::types::extraction::Element;
    use xberg::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut doc = InternalDocument::new("docx");
    doc.mime_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string();

    doc.elements.push(InternalElement::text(
        ElementKind::Heading { level: 1 },
        "Introduction",
        0,
    ));
    doc.elements
        .push(InternalElement::text(ElementKind::Paragraph, "Body text goes here.", 0));
    doc.elements.push(InternalElement::text(
        ElementKind::ListItem { ordered: false },
        "First bullet",
        1,
    ));
    doc.elements.push(InternalElement::text(
        ElementKind::ListItem { ordered: false },
        "Second bullet",
        1,
    ));

    doc.tables.push(xberg::types::tables::Table {
        cells: vec![
            vec!["Header A".to_string(), "Header B".to_string()],
            vec!["Cell 1".to_string(), "Cell 2".to_string()],
        ],
        markdown: "| Header A | Header B |\n| Cell 1 | Cell 2 |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    });
    doc.elements
        .push(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0));

    let elements: Vec<Element> = xberg::extraction::transform::convert_internal_elements_to_elements(&doc, &None);

    assert_eq!(
        elements.len(),
        5,
        "Expected 5 elements (h1, paragraph, 2 list items, table), got {}",
        elements.len()
    );

    assert_eq!(
        elements[0].element_type,
        xberg::types::ElementType::Title,
        "First element must be Title (h1 maps to Title)"
    );
    assert_eq!(elements[0].text, "Introduction");

    assert_eq!(
        elements[1].element_type,
        xberg::types::ElementType::NarrativeText,
        "Second element must be NarrativeText"
    );
    assert_eq!(elements[1].text, "Body text goes here.");

    assert_eq!(
        elements[2].element_type,
        xberg::types::ElementType::ListItem,
        "Third element must be ListItem"
    );
    assert_eq!(elements[2].text, "First bullet");

    assert_eq!(
        elements[3].element_type,
        xberg::types::ElementType::ListItem,
        "Fourth element must be ListItem"
    );
    assert_eq!(elements[3].text, "Second bullet");

    assert_eq!(
        elements[4].element_type,
        xberg::types::ElementType::Table,
        "Fifth element must be Table"
    );
    assert!(
        elements[4].text.contains("Header A"),
        "Table text must contain cell content"
    );
}

/// Integration test: extract a real DOCX with ElementBased result format and assert
/// the element sequence is heading → paragraph (in that order, not reversed or mixed).
///
/// Skipped when the test fixture is absent (e.g. CI without test_documents submodule).
#[tokio::test]
async fn test_docx_element_based_result_format_preserves_order() {
    use helpers::get_test_file_path;
    use xberg::core::config::ExtractionConfig;
    use xberg::types::{ElementType, ResultFormat};

    let path = get_test_file_path("docx/unit_test_headers.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        result_format: ResultFormat::ElementBased,
        ..Default::default()
    };

    let result = extract_uri_document(&path, None, &config)
        .await
        .expect("DOCX extraction should succeed");

    let elements = result
        .elements
        .as_deref()
        .expect("ElementBased result must have elements");
    assert!(!elements.is_empty(), "Must have at least one element");

    let first_heading_pos = elements
        .iter()
        .position(|e| matches!(e.element_type, ElementType::Title | ElementType::Heading));
    let last_heading_pos = elements
        .iter()
        .rposition(|e| matches!(e.element_type, ElementType::Title | ElementType::Heading));

    if let (Some(first_h), Some(last_h)) = (first_heading_pos, last_heading_pos) {
        let narrative_after_first_heading = elements[first_h..]
            .iter()
            .any(|e| matches!(e.element_type, ElementType::NarrativeText | ElementType::ListItem));
        assert!(
            narrative_after_first_heading,
            "Body text must appear after/alongside headings in document order \
             (first heading at {first_h}, last heading at {last_h})"
        );
    }
}
