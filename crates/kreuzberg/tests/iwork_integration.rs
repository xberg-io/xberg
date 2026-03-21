//! Integration tests for Apple iWork format extractors.
//!
//! These tests verify that .pages, .numbers, and .key files can be
//! opened, parsed, and produce non-empty text output.

#[cfg(feature = "iwork")]
mod iwork_tests {
    use kreuzberg::core::config::ExtractionConfig;
    use kreuzberg::extractors::iwork::{keynote::KeynoteExtractor, numbers::NumbersExtractor, pages::PagesExtractor};
    use kreuzberg::plugins::DocumentExtractor;
    use std::path::PathBuf;

    fn test_doc_path(name: &str) -> PathBuf {
        let manifest = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest).join("../../test_documents/iwork").join(name)
    }

    // ── MIME type unit tests ────────────────────────────────────────────

    #[test]
    fn test_pages_extractor_mime_types() {
        let extractor = PagesExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(
            types.contains(&"application/x-iwork-pages-sffpages"),
            "Pages extractor must support its MIME type"
        );
    }

    #[test]
    fn test_numbers_extractor_mime_types() {
        let extractor = NumbersExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(
            types.contains(&"application/x-iwork-numbers-sffnumbers"),
            "Numbers extractor must support its MIME type"
        );
    }

    #[test]
    fn test_keynote_extractor_mime_types() {
        let extractor = KeynoteExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(
            types.contains(&"application/x-iwork-keynote-sffkey"),
            "Keynote extractor must support its MIME type"
        );
    }

    // ── Proto text extraction unit tests ────────────────────────────────

    #[test]
    fn test_extract_text_from_proto_basic() {
        use kreuzberg::extractors::iwork::extract_text_from_proto;

        // Protobuf: field 3, wire type 2 (length-delimited) = tag 0x1A
        let text = b"Hello World from iWork";
        let mut proto = vec![0x1A, text.len() as u8];
        proto.extend_from_slice(text);

        let extracted = extract_text_from_proto(&proto);
        assert!(
            extracted.iter().any(|s| s.contains("Hello World")),
            "Should extract the embedded UTF-8 string: {:?}",
            extracted
        );
    }

    #[test]
    fn test_extract_text_from_proto_skips_binary() {
        use kreuzberg::extractors::iwork::extract_text_from_proto;

        // Craft a proto payload with binary blob (non-UTF-8)
        let binary: Vec<u8> = (0..20).map(|i| i * 7 + 3).collect();
        let mut proto = vec![0x1A, binary.len() as u8];
        proto.extend_from_slice(&binary);

        // Should not panic and should produce no valid text strings
        let extracted = extract_text_from_proto(&proto);
        // Binary data should not produce alphabetic strings
        for s in &extracted {
            assert!(
                !s.chars().all(|c| c.is_alphabetic()),
                "Binary blob should not produce clean alphabetic strings: {s}"
            );
        }
    }

    #[test]
    fn test_extract_text_from_proto_nested() {
        use kreuzberg::extractors::iwork::extract_text_from_proto;

        // Nested message: outer field 2 wrapping an inner field 3 with text
        let inner_text = b"Nested Content";
        let mut inner = vec![0x1A, inner_text.len() as u8];
        inner.extend_from_slice(inner_text);

        let mut outer = vec![0x12, inner.len() as u8]; // field 2, wire type 2
        outer.extend_from_slice(&inner);

        let extracted = extract_text_from_proto(&outer);
        assert!(
            extracted.iter().any(|s| s.contains("Nested Content")),
            "Should extract text from nested protobuf messages: {:?}",
            extracted
        );
    }

    // ── MIME type detection integration tests ───────────────────────────

    #[test]
    fn test_mime_detection_numbers_file() {
        let path = test_doc_path("test.numbers");
        if !path.exists() {
            eprintln!("Skipping: test.numbers not found at {:?}", path);
            return;
        }

        let mime = kreuzberg::core::mime::detect_mime_type(&path, true).unwrap();
        assert_eq!(
            mime, "application/x-iwork-numbers-sffnumbers",
            "Should detect .numbers MIME type from extension"
        );
    }

    #[test]
    fn test_mime_detection_pages_file() {
        let path = test_doc_path("test.pages");
        if !path.exists() {
            eprintln!("Skipping: test.pages not found at {:?}", path);
            return;
        }

        let mime = kreuzberg::core::mime::detect_mime_type(&path, true).unwrap();
        assert_eq!(
            mime, "application/x-iwork-pages-sffpages",
            "Should detect .pages MIME type from extension"
        );
    }

    // ── Extraction integration tests ─────────────────────────────────────

    #[tokio::test]
    #[cfg(feature = "tokio-runtime")]
    async fn test_extract_numbers_document() {
        let path = test_doc_path("test.numbers");
        if !path.exists() {
            eprintln!("Skipping: test.numbers not found at {:?}", path);
            return;
        }

        let content = std::fs::read(&path).expect("Failed to read test.numbers");
        let extractor = NumbersExtractor::new();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&content, "application/x-iwork-numbers-sffnumbers", &config)
            .await
            .expect("NumbersExtractor should not fail on valid file");

        // A valid Numbers file should produce some text output
        assert!(
            !result.content.is_empty(),
            "Numbers extraction should produce non-empty text. Got: {:?}",
            &result.content[..result.content.len().min(200)]
        );
    }

    #[tokio::test]
    #[cfg(feature = "tokio-runtime")]
    async fn test_extract_pages_document() {
        let path = test_doc_path("test.pages");
        if !path.exists() {
            eprintln!("Skipping: test.pages not found at {:?}", path);
            return;
        }

        let content = std::fs::read(&path).expect("Failed to read test.pages");
        let extractor = PagesExtractor::new();
        let config = ExtractionConfig::default();

        // Extraction should not panic — it may produce empty content if the
        // fixture is a stub (non-Snappy compressed IWA), but should not error.
        let result = extractor
            .extract_bytes(&content, "application/x-iwork-pages-sffpages", &config)
            .await
            .expect("PagesExtractor should not fail on valid ZIP file");

        // For any valid .pages file, extraction should succeed (even if empty)
        assert!(
            result.mime_type.as_ref() == "application/x-iwork-pages-sffpages",
            "MIME type should be preserved in result"
        );
    }

    // ── iwa_entries listing test ─────────────────────────────────────────

    #[test]
    fn test_list_iwa_entries_numbers() {
        use kreuzberg::extractors::iwork::list_iwa_entries;

        let path = test_doc_path("test.numbers");
        if !path.exists() {
            eprintln!("Skipping: test.numbers not found at {:?}", path);
            return;
        }

        let content = std::fs::read(&path).expect("Failed to read test.numbers");
        let entries = list_iwa_entries(&content).expect("Should list IWA entries");

        assert!(!entries.is_empty(), "Numbers ZIP should contain at least one .iwa file");
        assert!(
            entries
                .iter()
                .any(|e| e.contains("Document.iwa") || e.contains("CalculationEngine.iwa")),
            "Should find expected root IWA files: {:?}",
            entries
        );
    }
}
