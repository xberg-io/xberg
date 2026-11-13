#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::OutputFormat;
use xberg::types::ExtractionMethod;
use xberg::{ExtractionConfig, OcrConfig, PageConfig};

const MAPPED_EXPRESSION: &str = "Vmax = 1000 V";
const LOST_GLYPH_WARNING: &str =
    "Page 1 retained native text; 8 glyphs have no Unicode value and its OCR read did not recover them.";

#[test]
fn automatic_ocr_preserves_the_mapped_expression_on_the_nist_checklist() {
    let path = std::env::var_os("XBERG_NIST_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_file_path("pdf/regression_pages/nist-sp811-2008-p7.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "71c2a76af332a9b796899a78e99ce2d835026d624048cf239069276fcacce79b"
    );
    let native_pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    let page = native_pdf.extract_page_text(0).unwrap();
    assert_eq!(
        page.spans
            .iter()
            .map(|span| span.text.matches(xberg_native_pdf::is_lost_glyph_marker).count())
            .sum::<usize>(),
        8
    );

    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    for format in [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
        OutputFormat::Json,
        OutputFormat::DocTags,
    ] {
        let extract = |disable_ocr, force_ocr| {
            let config = ExtractionConfig {
                use_cache: false,
                disable_ocr,
                force_ocr,
                output_format: format.clone(),
                ocr: Some(OcrConfig {
                    tessdata_path: Some(models.path().to_path_buf()),
                    ..Default::default()
                }),
                pages: Some(PageConfig {
                    extract_pages: true,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let document = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
            assert_eq!(document.pages.as_ref().unwrap().len(), 1);
            document
        };
        let automatic = extract(false, false);
        assert_eq!(
            automatic
                .processing_warnings
                .iter()
                .filter(|warning| warning.source == "ocr" && warning.message == LOST_GLYPH_WARNING)
                .count(),
            1,
            "{format:?}: expected one per-page warning, got {:?}",
            automatic.processing_warnings
        );
        let native = extract(true, false);
        assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
        assert!(
            native
                .processing_warnings
                .iter()
                .all(|warning| warning.message != LOST_GLYPH_WARNING),
            "{format:?}: disabled OCR cannot report a failed read"
        );
        assert_eq!(
            automatic.extraction_method,
            Some(ExtractionMethod::Native),
            "{format:?}"
        );
        assert!(!automatic.content.contains(char::REPLACEMENT_CHARACTER), "{format:?}");
        assert_eq!(automatic.content, native.content, "{format:?}");
        if !matches!(format, OutputFormat::Plain | OutputFormat::Markdown) {
            continue;
        }
        assert!(
            native.content.contains(MAPPED_EXPRESSION),
            "{format:?}: Native source expression"
        );
        let forced = extract(false, true);
        assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
        assert!(!forced.content.trim().is_empty());
        assert!(!forced.content.contains(MAPPED_EXPRESSION));
        assert!(
            forced
                .processing_warnings
                .iter()
                .all(|warning| warning.message != LOST_GLYPH_WARNING),
            "{format:?}: forced OCR did not retain the native page"
        );
        for (projection, content) in [
            ("document", &automatic.content),
            ("page", &automatic.pages.as_ref().unwrap()[0].content),
        ] {
            assert_eq!(
                content.matches(MAPPED_EXPRESSION).count(),
                1,
                "{format:?} {projection}: automatic OCR lost the independently mapped expression {MAPPED_EXPRESSION:?}"
            );
        }
    }
}
