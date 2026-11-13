#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use xberg::core::config::PageConfig;
use xberg::types::ExtractionMethod;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const EXPECTED: &str = include_str!("fixtures/pdf/separation-overprint-2817/sepop.ocr.txt");

#[test]
fn automatic_ocr_keeps_overprinted_separation_text_and_native_sibling() {
    let tessdata = tempfile::tempdir().expect("create an isolated tessdata directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).expect("seed bundled English data");
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let expected: Vec<_> = EXPECTED.split_whitespace().collect();
    assert_eq!(expected.len(), 219);
    let cases = [
        (
            "no overprint",
            include_bytes!("fixtures/pdf/separation-overprint-2817/separation-no-overprint-mixed.pdf").as_slice(),
        ),
        (
            "overprint",
            include_bytes!("fixtures/pdf/separation-overprint-2817/sepop-mixed.pdf").as_slice(),
        ),
    ];
    for (label, bytes) in cases {
        let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config)
            .expect("extract the public mixed PDF");
        assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed), "{label}");
        let pages = result.pages.as_deref().expect("requested page output");
        let [ocr, native] = pages else {
            panic!("{label}: expected two pages, got {}", pages.len());
        };
        assert_eq!(ocr.content.split_whitespace().collect::<Vec<_>>(), expected, "{label}");
        assert_eq!(
            native.content.trim(),
            "Native sibling control remains unchanged.",
            "{label}"
        );
        assert!(
            result
                .content
                .split_whitespace()
                .eq(expected.iter().copied().chain(native.content.split_whitespace())),
            "{label}: document transcript differs from its pages"
        );
    }
}
