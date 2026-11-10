use super::*;
use crate::extractors::security::SecurityLimits;
use xberg_native_pdf::PdfDocument;

fn page() -> PdfDocument {
    PdfDocument::from_bytes(crate::pdf::render::build_minimal_pdf_with_mediabox(432.0, 216.0)).unwrap()
}

#[test]
fn configured_preprocessing_raises_only_the_unconfigured_render_target() {
    let pdf = page();
    let limits = SecurityLimits::default();
    assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (150, 150));
    assert_eq!(
        planned_pdf_ocr_render_dpi_with_preprocessing(&pdf, 0, None, &limits, Some(300)),
        (300, 300)
    );
    let images = crate::core::config::ImageExtractionConfig {
        target_dpi: 150,
        auto_adjust_dpi: false,
        ..Default::default()
    };
    assert_eq!(
        planned_pdf_ocr_render_dpi_with_preprocessing(&pdf, 0, Some(&images), &limits, Some(300)),
        (150, 150)
    );
}

#[test]
fn preprocessing_keeps_full_page_scans_at_their_original_density() {
    let bytes = crate::pdf::render::build_full_page_raster_pdf((100.0, 100.0), (400, 400), 1.0, 0);
    let pdf = PdfDocument::from_bytes(bytes).unwrap();
    let limits = SecurityLimits::default();
    assert_eq!(
        planned_pdf_ocr_render_dpi_with_preprocessing(&pdf, 0, None, &limits, Some(300)),
        (288, 288)
    );
}

#[test]
fn promoted_density_is_capped_before_render_allocation() {
    let pdf = page();
    let limits = SecurityLimits {
        max_content_size: 4 * 1024 * 1024,
        ..Default::default()
    };
    let (requested, planned) = planned_pdf_ocr_render_dpi_with_preprocessing(&pdf, 0, None, &limits, Some(300));
    assert_eq!(requested, 300);
    assert!(planned > 0 && planned < 300);
    let width = (6.0 * f64::from(planned)).ceil() as u64;
    let height = (3.0 * f64::from(planned)).ceil() as u64;
    assert!(width * height * 11 + OCR_PNG_ENCODE_FIXED_BYTES <= limits.max_content_size as u64);
    let mut exceeded = false;
    let (rendered, evidence) =
        crate::pdf::render::render_pdf_ocr_page_with_budget_evidence(&pdf, 0, None, &limits, &mut exceeded, Some(300))
            .unwrap();
    assert!(evidence.empty_text_warning(0).is_some());
    let actual_dpi = rendered.width / 6;
    assert!(actual_dpi > 0 && actual_dpi <= planned as u32);
    assert_eq!((rendered.width, rendered.height), (6 * actual_dpi, 3 * actual_dpi));
    assert!(
        u64::from(rendered.width) * u64::from(rendered.height) * 11 + OCR_PNG_ENCODE_FIXED_BYTES
            <= limits.max_content_size as u64
    );
    assert!(!exceeded);
}

#[test]
fn zero_render_budget_is_an_error_not_an_unbounded_fallback() {
    let pdf = page();
    let limits = SecurityLimits {
        max_content_size: 0,
        ..Default::default()
    };
    let mut exceeded = false;
    let error =
        crate::pdf::render::render_pdf_ocr_page_with_budget_evidence(&pdf, 0, None, &limits, &mut exceeded, Some(300))
            .err()
            .expect("zero-byte cap rejects the canvas");
    assert!(
        error.to_string().contains("security_limits.max_content_size"),
        "{error}"
    );
    assert!(!exceeded, "canvas refusal must not claim a source-image overflow");
}

#[test]
fn actual_scan_render_preserves_detected_source_density_before_preprocessing() {
    let bytes = crate::pdf::render::build_full_page_raster_pdf((144.0, 144.0), (300, 300), 1.0, 0);
    let pdf = PdfDocument::from_bytes(bytes).unwrap();
    let limits = SecurityLimits::default();
    assert_eq!(crate::pdf::scan_detect::full_page_raster_density(&pdf, 0), Some(150.0));
    let mut exceeded = false;
    let (rendered, _) =
        crate::pdf::render::render_pdf_ocr_page_with_budget_evidence(&pdf, 0, None, &limits, &mut exceeded, Some(300))
            .unwrap();
    assert_eq!((rendered.width, rendered.height), (300, 300));
    assert!(!exceeded);
}
