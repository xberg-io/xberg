#![cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]

mod helpers;

use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::ExtractionMethod;

const PNG: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/records.png");
const TIFF: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/records.tiff");

#[test]
fn oversized_unknown_dpi_png_and_tiff_keep_every_ordered_record() {
    let expected = (1..=30)
        .map(|record| format!("Record {record:02} the quarterly accounts match the signed report"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(expected.split_whitespace().count(), 270);
    for (bytes, mime) in [(PNG, "image/png"), (TIFF, "image/tiff")] {
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };
        let output = helpers::extract_bytes_document_blocking(bytes, mime, &config).unwrap();
        assert_eq!(output.extraction_method, Some(ExtractionMethod::Ocr));
        assert_eq!(
            output.content.split_whitespace().collect::<Vec<_>>().join(" "),
            expected,
            "{mime} must retain all ordered source records"
        );
        let dpi = output.metadata.image_preprocessing.unwrap();
        let dimensions = dpi.new_dimensions.expect("bounded Lanczos downscale");
        assert_eq!(
            (dpi.original_dimensions.width, dpi.original_dimensions.height),
            (4000, 3000)
        );
        assert!(dimensions.width < 4000 && dimensions.height < 3000);
        assert_eq!(dpi.resample_method, "LANCZOS3");
        assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
        assert_eq!(dpi.target_dpi, 300);
    }
}

#[cfg(feature = "pdf")]
#[test]
fn a_large_scanned_pdf_keeps_its_heading_under_the_native_ocr_budget() {
    const SCAN: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/large-scan.pdf");
    let output = helpers::extract_bytes_document_blocking(
        SCAN,
        "application/pdf",
        &ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            security_limits: Some(xberg::SecurityLimits {
                max_content_size: 64 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("the raster OCR fits the configured workspace");

    assert_eq!(output.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(output.content.lines().next(), Some("Quarterly Accounts Report"));
    let dpi = output.metadata.image_preprocessing.expect("budgeted raster upscale");
    assert_eq!(dpi.target_dpi, 300);
    assert!(dpi.scale_factor > 1.0 && dpi.final_dpi < dpi.target_dpi, "{dpi:?}");
}
