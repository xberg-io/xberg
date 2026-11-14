use super::*;
use crate::extractors::security::SecurityLimits;
use xberg_native_pdf::PdfDocument;

use crate::ocr_pixel_phase_fixture as fixture;

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
fn small_text_promotion_preserves_explicit_targets_and_render_budget() {
    let pdf = PdfDocument::from_bytes(fixture::pdf(7, 0)).unwrap();
    let limits = SecurityLimits::default();
    assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (300, 300));
    assert_eq!(
        planned_pdf_ocr_render_dpi_with_preprocessing(&pdf, 0, None, &limits, Some(200)),
        (200, 200)
    );
    let limits = SecurityLimits {
        max_content_size: 4 * 1024 * 1024,
        ..limits
    };
    let (requested, planned) = planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits);
    assert_eq!(requested, 150);
    assert!(planned > 0 && planned < 150);
    let area = 18 * u64::try_from(planned).unwrap().pow(2);
    assert!(area * 11 + OCR_PNG_ENCODE_FIXED_BYTES <= limits.max_content_size as u64);
}

#[test]
fn sparse_or_large_small_text_pages_keep_default_density() {
    let dense = PdfDocument::from_bytes(fixture::pdf(7, 0)).unwrap();
    assert_eq!(
        small_text_render_dpi(&dense, 0, 1000.0, 1000.0, usize::MAX),
        DEFAULT_PDF_RENDER_DPI
    );
    let sparse =
        PdfDocument::from_bytes(fixture::content_pdf("BT /F1 7 Tf 36 180 Td (Page one) Tj ET".into())).unwrap();
    assert_eq!(
        planned_pdf_ocr_render_dpi(&sparse, 0, None, &SecurityLimits::default()),
        (150, 150)
    );
}

#[test]
fn letter_and_a4_promote_small_text_only_on_single_pages() {
    let limits = SecurityLimits::default();
    for (name, bytes, _) in fixture::density_controls() {
        let pdf = PdfDocument::from_bytes(bytes).unwrap();
        let expected = match name {
            "letter-small" | "a4-small" => 300,
            _ => 150,
        };
        for index in 0..pdf.page_count().unwrap() {
            assert_eq!(
                planned_pdf_ocr_render_dpi(&pdf, index, None, &limits),
                (expected, expected),
                "{name}, page {index}"
            );
        }
    }
}

#[test]
fn invisible_small_text_does_not_promote_visible_ordinary_text() {
    let limits = SecurityLimits::default();
    for (mode, expected) in [(0, 300), (3, 150), (7, 150)] {
        let content = format!(
            "BT /F1 12 Tf 36 500 Td (Visible ordinary text) Tj ET BT /F1 7 Tf {mode} Tr 36 450 Td ({}) Tj ET",
            fixture::LINES[0]
        );
        let pdf = PdfDocument::from_bytes(fixture::content_pdf_with_pages(content, 612.0, 792.0, false)).unwrap();
        let layer = pdf.extract_text(0).unwrap();
        assert!(
            layer.contains(fixture::LINES[0]),
            "small text must exist in the layer: {layer}"
        );
        assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (expected, expected));
    }
}

#[test]
fn zero_alpha_fill_does_not_promote_visible_ordinary_text() {
    let limits = SecurityLimits::default();
    let content = format!(
        "BT /F1 12 Tf 36 500 Td (Visible ordinary text) Tj ET q /NoInk gs BT /F1 7 Tf 0 Tr 36 450 Td ({}) Tj ET Q",
        fixture::LINES[0]
    );
    let bytes = fixture::content_pdf_with_resources(content, "/ExtGState << /NoInk << /Type /ExtGState /ca 0 >> >>");
    let pdf = PdfDocument::from_bytes(bytes).unwrap();
    let layer = pdf.extract_text(0).unwrap();
    assert!(
        layer.contains(fixture::LINES[0]),
        "small text must exist in the layer: {layer}"
    );
    assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (150, 150));
}

#[test]
fn logo_opaque_graphics_state_and_restored_alpha_allow_small_text() {
    let limits = SecurityLimits::default();
    let text = format!("BT /F1 7 Tf 0 Tr 36 450 Td ({}) Tj ET", fixture::LINES[0]);
    let states = "/ExtGState << /Opaque << /ca 1 >> /NoInk << /ca 0 >> >>";
    for bytes in [
        fixture::content_pdf_with_logo(format!("q 8 0 0 8 36 500 cm /Logo Do Q {text}")),
        fixture::content_pdf_with_resources(format!("/Opaque gs {text}"), states),
        fixture::content_pdf_with_resources(format!("q /NoInk gs {text} Q {text}"), states),
    ] {
        let pdf = PdfDocument::from_bytes(bytes).unwrap();
        assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (300, 300));
        assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (300, 300));
    }
}

#[test]
fn later_partial_image_covering_small_text_keeps_default_density() {
    let text = format!("BT /F1 7 Tf 36 450 Td ({}) Tj ET", fixture::LINES[0]);
    let cover = "q 500 0 0 40 20 430 cm /Logo Do Q";
    for (content, expected) in [(format!("{cover} {text}"), 300), (format!("{text} {cover}"), 150)] {
        let pdf = PdfDocument::from_bytes(fixture::content_pdf_with_logo(content)).unwrap();
        let limits = SecurityLimits::default();
        assert_eq!(planned_pdf_ocr_render_dpi(&pdf, 0, None, &limits), (expected, expected));
    }
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
