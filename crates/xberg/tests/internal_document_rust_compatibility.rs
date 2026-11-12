use xberg::types::internal::InternalDocument;

#[test]
fn downstream_callers_can_construct_internal_document_exhaustively() {
    let document = InternalDocument {
        elements: Vec::new(),
        relationships: Vec::new(),
        source_format: "pdf".into(),
        metadata: Default::default(),
        images: Vec::new(),
        tables: Vec::new(),
        diagrams: Vec::new(),
        uris: Vec::new(),
        uris_dropped: 0,
        children: None,
        mime_type: "application/octet-stream".to_string(),
        processing_warnings: Vec::new(),
        ocr_page_failures: Vec::new(),
        annotations: None,
        prebuilt_pages: None,
        ocr_text_pages: Vec::new(),
        pre_rendered_content: None,
        prebuilt_ocr_elements: None,
        llm_usage: None,
        revisions: None,
        ocr_text_only: false,
        append_ocr_text: false,
        escape_markdown: true,
        include_watermarks: false,
        page_marker_format: None,
        page_numbers: Vec::new(),
        table_anchors: false,
        form_fields: Vec::new(),
        formulas: Vec::new(),
        recorded_formulas: Vec::new(),
        #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
        ocr_coordinate_frame: None,
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        ocr_text_claimed_by_tables: false,
    };
    assert_eq!(document.source_format, "pdf");
}
