use xberg::{ExtractedDocument, ExtractionConfig, types::PageContent};

pub(super) fn assert_page_ranges(document: &ExtractedDocument) {
    let pages = document.pages.as_ref().expect("requested page payload");
    let structure = document.metadata.pages.as_ref().expect("page structure");
    let boundaries = structure.boundaries.as_ref().expect("Plain page byte ranges");
    assert!(!pages.is_empty());
    assert_eq!(usize::try_from(structure.total_count).unwrap(), pages.len());
    assert_eq!(boundaries.len(), pages.len());
    let mut end = 0;
    for (page, boundary) in pages.iter().zip(boundaries) {
        assert_eq!(boundary.page_number, page.page_number);
        assert!(!page.content.is_empty());
        let gap = document
            .content
            .get(end..boundary.byte_start)
            .expect("ordered UTF-8 ranges");
        assert!(gap.chars().all(char::is_whitespace), "no text is lost between pages");
        assert_eq!(
            document.content.get(boundary.byte_start..boundary.byte_end),
            Some(page.content.as_str()),
            "the complete page body equals its valid UTF-8 content range"
        );
        end = boundary.byte_end;
    }
    assert!(document.content[end..].chars().all(char::is_whitespace));
    assert!(
        document
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "page_boundaries")
    );
}

async fn plain(markdown: &str) -> String {
    super::helpers::extract_bytes_document(
        markdown.as_bytes(),
        "text/markdown",
        &ExtractionConfig {
            use_cache: false,
            disable_ocr: true,
            ..Default::default()
        },
    )
    .await
    .expect("independent public Markdown-to-Plain rendering")
    .content
}

pub(super) async fn scanned_table_body_is_complete(actual: &PageContent, recognized: &PageContent) -> bool {
    assert_eq!(recognized.tables.len(), 1);
    let table = &recognized.tables[0];
    for witness in ["Six-Year Summary", "48,210", "102,228"] {
        assert!(table.markdown.contains(witness), "the source table contains {witness}");
    }
    let canonical = plain(&table.markdown).await;
    // Picture OCR can retain a table body while page elements render its cells as Plain. ~keep
    actual.content == table.markdown.trim_end_matches('\n') || actual.content == canonical
}

pub(super) async fn assert_scanned_table_body(actual: &PageContent, recognized: &PageContent) {
    assert!(
        scanned_table_body_is_complete(actual, recognized).await,
        "the complete scanned body equals the independent source table or its canonical Plain rendering"
    );
}
