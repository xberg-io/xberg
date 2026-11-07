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
    let native_pdf = xberg_native_pdf::PdfDocument::open_from_bytes(bytes.clone()).unwrap();
    let page = native_pdf.extract_page_text(0).unwrap();
    assert_eq!(
        page.spans
            .iter()
            .map(|span| span.text.matches(char::REPLACEMENT_CHARACTER).count())
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
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
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
        let native = extract(true, false);
        assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
        assert!(
            native.content.contains(MAPPED_EXPRESSION),
            "{format:?}: Native source expression"
        );
        let forced = extract(false, true);
        assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
        assert!(!forced.content.trim().is_empty());
        assert!(!forced.content.contains(MAPPED_EXPRESSION));
        let automatic = extract(false, false);
        assert_eq!(
            automatic.extraction_method,
            Some(ExtractionMethod::Native),
            "{format:?}"
        );
        assert!(!automatic.content.contains(char::REPLACEMENT_CHARACTER), "{format:?}");
        assert_eq!(automatic.content, native.content, "{format:?}");
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
