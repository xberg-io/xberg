use xberg::{ExtractedDocument, types::PageContent};

pub(super) fn assert_page_ranges(document: &ExtractedDocument) {
    assert!(
        !document.content.contains("| --- |"),
        "Plain document has no table separator"
    );
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
        assert!(!page.content.contains("| --- |"), "Plain page has no table separator");
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

fn forced_table_block(recognized: &PageContent) -> &str {
    assert_eq!(recognized.tables.len(), 1);
    let rows = recognized.tables[0]
        .cells
        .iter()
        .map(|row| row.join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    // Whole-document OCR also emits the cell text as individual paragraphs. ~keep
    let mut blocks = recognized.content.split("\n\n").filter(|block| *block == rows.trim());
    let block = blocks
        .next()
        .expect("the forced page contains the complete Plain table");
    assert!(blocks.next().is_none(), "the forced page has one rendered table block");
    for witness in ["Six-Year Summary", "48,210", "102,228"] {
        assert!(block.contains(witness), "the rendered table contains {witness}");
    }
    block
}

pub(super) fn scanned_table_body_is_complete(actual: &PageContent, recognized: &PageContent) -> bool {
    actual.content == forced_table_block(recognized)
}

pub(super) fn assert_scanned_table_body(actual: &PageContent, recognized: &PageContent) {
    assert_eq!(
        actual.content,
        forced_table_block(recognized),
        "the complete mixed page equals the forced Plain table block"
    );
}
