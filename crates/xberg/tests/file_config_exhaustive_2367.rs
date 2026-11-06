use xberg::{ExtractInput, ExtractInputKind, ExtractionConfig, FileExtractionConfig};

#[test]
fn old_exhaustive_input_and_file_config_literals_still_compile() {
    let file = FileExtractionConfig {
        mime_detection_policy: None,
        enable_quality_processing: None,
        ocr: None,
        force_ocr: None,
        ocr_strategy: None,
        force_ocr_pages: None,
        disable_ocr: None,
        chunking: None,
        content_filter: None,
        images: None,
        #[cfg(feature = "pdf")]
        pdf_options: None,
        token_reduction: None,
        language_detection: None,
        pages: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        keywords: None,
        postprocessor: None,
        #[cfg(feature = "html")]
        html_options: None,
        #[cfg(feature = "html")]
        html_output: None,
        result_format: None,
        output_format: None,
        include_document_structure: None,
        strict_parse: None,
        #[cfg(feature = "layout-types")]
        layout: None,
        #[cfg(feature = "transcription-types")]
        transcription: None,
        timeout_secs: None,
        #[cfg(feature = "tree-sitter")]
        tree_sitter: None,
        structured_extraction: None,
        url: None,
        ner: None,
        redaction: None,
        summarization: None,
        translation: None,
        page_classification: None,
        chunk_classification: None,
        captioning: None,
        qr_codes: None,
    };
    let input = ExtractInput {
        kind: ExtractInputKind::Bytes,
        bytes: Some(b"old-client".to_vec()),
        uri: None,
        mime_type: Some("text/plain".into()),
        filename: None,
        config: Some(file),
    };
    let base = ExtractionConfig::default();
    assert!(!base.use_layout_for_markdown);
    drop(xberg::extract_batch(vec![input], &base));
}
