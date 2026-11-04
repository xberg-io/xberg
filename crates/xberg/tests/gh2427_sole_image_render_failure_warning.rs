//! A page whose only content is one image that fails to render must report the failure.
//!
//! The renderer leaves the region blank and keeps going, so OCR then reads a blank page and
//! returns an empty success. The failure has to reach `processing_warnings` even when the
//! application never opted in to the log capture (`install_pdf_render_diagnostics`), because
//! most embedders never do. Nothing in this file installs the capture.

#![cfg(feature = "pdf")]

mod helpers;

use xberg::pdf::render::{render_pdf_page_to_png, take_xberg_native_pdf_render_warnings};

#[cfg(feature = "ocr")]
fn image_failure_warnings(warnings: &[xberg::types::ProcessingWarning]) -> Vec<&xberg::types::ProcessingWarning> {
    warnings
        .iter()
        .filter(|warning| warning.source == "pdf-render" && warning.message.contains("could not rasterize"))
        .collect()
}

/// The PNG entry point returns no warnings, and a language binding cannot drain the thread's
/// buffer. Without the log capture, a warning it queued would reach the next extraction on this
/// thread, which can be for another document.
#[test]
fn the_png_entry_point_leaves_no_warning_on_the_thread_without_the_log_capture() {
    let _ = take_xberg_native_pdf_render_warnings();

    let png = render_pdf_page_to_png(
        &helpers::pdf_helpers::build_pdf_with_undecodable_image(),
        0,
        Some(72),
        None,
    )
    .expect("a page with one unrenderable image still renders, blank in that region");
    assert!(!png.is_empty());

    let warnings = take_xberg_native_pdf_render_warnings();
    assert!(warnings.is_empty(), "nothing drains this path; got {warnings:?}");
}

#[cfg(feature = "ocr")]
mod ocr_route {
    use super::{helpers, image_failure_warnings};
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    #[tokio::test]
    async fn force_ocr_of_a_page_with_a_failed_sole_image_carries_the_warning() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            force_ocr: true,
            ..Default::default()
        };
        let doc = helpers::extract_bytes_document(
            &helpers::pdf_helpers::build_pdf_with_undecodable_image(),
            "application/pdf",
            &config,
        )
        .await
        .expect("a page whose only image failed still extracts");

        let image_warnings = image_failure_warnings(&doc.processing_warnings);
        assert_eq!(
            image_warnings.len(),
            1,
            "the empty OCR result must carry the render failure; got {:?}",
            doc.processing_warnings
        );
    }
}
