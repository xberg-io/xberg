#![cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]

mod helpers;

use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::{ExtractionMethod, ImagePreprocessingConfig, TesseractConfig};

const PNG: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/records.png");
const RAW_PNG: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/raw-records.png");
const TIFF: &[u8] = include_bytes!("fixtures/ocr_unknown_dpi_2530/records.tiff");

fn assert_recovered_records(bytes: &[u8], mime: &str, ocr: OcrConfig, target_dpi: i32, psm: Option<&str>) {
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(ocr),
        ..Default::default()
    };
    let output = helpers::extract_bytes_document_blocking(bytes, mime, &config).expect("bounded OCR succeeds");
    assert_eq!(output.extraction_method, Some(ExtractionMethod::Ocr));
    let (separator, terminator) = if psm == Some("6") { (":", ".") } else { ("", "") };
    let expected = (1..=30)
        .map(|record| {
            format!("Record {record:02}{separator} the quarterly accounts match the signed report{terminator}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(expected.split_whitespace().count(), 270);
    assert_eq!(
        output.content.split_whitespace().collect::<Vec<_>>().join(" "),
        expected,
        "{mime} must retain all ordered source records"
    );
    if let Some(psm) = psm {
        assert_eq!(output.metadata.additional["psm"], psm);
    }
    let dpi = output.metadata.image_preprocessing.unwrap();
    let dimensions = dpi.new_dimensions.expect("bounded Lanczos downscale");
    assert_eq!(
        (dpi.original_dimensions.width, dpi.original_dimensions.height),
        (4000, 3000)
    );
    assert!(dimensions.width < 4000 && dimensions.height < 3000);
    assert_eq!(dpi.resample_method, "LANCZOS3");
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
    assert_eq!(dpi.target_dpi, target_dpi);
}

#[test]
fn oversized_unknown_dpi_png_and_tiff_keep_every_ordered_record() {
    for (bytes, mime) in [(PNG, "image/png"), (TIFF, "image/tiff")] {
        assert_recovered_records(bytes, mime, OcrConfig::default(), 300, None);
    }
}

#[test]
fn oversized_densityless_png_with_a_raw_background_keeps_every_ordered_record() {
    assert_recovered_records(RAW_PNG, "image/png", OcrConfig::default(), 300, Some("11"));
}

fn explicit_preprocessing() -> OcrConfig {
    OcrConfig {
        tesseract_config: Some(TesseractConfig {
            psm: Some(6),
            preprocessing: Some(ImagePreprocessingConfig {
                target_dpi: 280,
                deskew: false,
                contrast_enhance: true,
                binarization_method: "none".into(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn oversized_densityless_png_keeps_the_explicit_preprocessing_and_psm() {
    assert_recovered_records(PNG, "image/png", explicit_preprocessing(), 280, Some("6"));
}

#[test]
fn oversized_densityless_tiff_keeps_the_explicit_preprocessing_and_psm() {
    assert_recovered_records(TIFF, "image/tiff", explicit_preprocessing(), 280, Some("6"));
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
