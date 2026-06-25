use std::path::Path;
use xberg::types::{ElementType, ResultFormat};
use xberg::{ExtractionConfig, OutputFormat, extract_bytes_sync};

/// Verifies that numbered chapter headings in an untagged ReportLab PDF are
/// classified as Heading/Title, not ListItem (#961).
#[test]
fn numbered_chapters_in_untagged_pdf_become_headings() {
    let path = Path::new("test_documents/pdf/multipage_marketing.pdf");
    if !path.exists() {
        eprintln!("skipping: test_documents/pdf/multipage_marketing.pdf not found");
        return;
    }

    let bytes = std::fs::read(path).expect("failed to read PDF");
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        result_format: ResultFormat::ElementBased,
        ..Default::default()
    };

    let result = extract_bytes_sync(&bytes, "application/pdf", &config).expect("extraction failed");
    let elements = result.elements.unwrap_or_default();

    let chapter_list_items: Vec<_> = elements
        .iter()
        .filter(|e| {
            e.element_type == ElementType::ListItem && e.text.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .collect();

    let numbered_headings: Vec<_> = elements
        .iter()
        .filter(|e| {
            matches!(e.element_type, ElementType::Heading | ElementType::Title)
                && e.text.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .collect();

    assert!(
        chapter_list_items.is_empty(),
        "numbered chapter headings must not be ListItem; got: {:?}",
        chapter_list_items.iter().map(|e| &e.text).collect::<Vec<_>>()
    );
    assert!(
        !numbered_headings.is_empty(),
        "at least one numbered chapter heading must be promoted to Heading/Title"
    );
}
