//! Regression coverage for GH#1912: when a page's OCR comes back blank or fails, the
//! whole-document OCR route retries OCR on the page's embedded image XObjects. The per-page
//! route (`force_ocr_pages`, scanned pages, the per-page fallback) now retries the same way,
//! with and without an OCR pipeline.
//!
//! A stub registered as `tesseract` returns no text (or an error) for the page render and a
//! known word for the embedded raster. The raster is not square and the page is, so the stub
//! tells the two calls apart by the image size. The document has two pages and only the second
//! carries the raster, so a retry that reads the wrong page recovers nothing. OCR of extracted
//! embedded images is off, so the retry is the only call that sees the raster.

#![allow(deprecated)]
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use xberg::ExtractedDocument;
use xberg::OcrPageFailure;
use xberg::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

/// Page size in points.
const PAGE_PT: u32 = 200;
/// Embedded raster size in pixels. Not square, so no page render has this size.
const RASTER_WIDTH_PX: u32 = 300;
const RASTER_HEIGHT_PX: u32 = 420;
/// The word the stub reads from the embedded raster.
const RECOVERED_WORD: &str = "Quarterly";
/// The warning text every embedded-image retry adds.
const RETRY_WARNING: &str = "OCR was retried on the embedded image bytes";
/// The warning text of a failed page that the retry recovered.
const FAILED_AND_RECOVERED_WARNING: &str = "its text was recovered from the page's embedded image XObjects";

/// What the stub does with the page render.
#[derive(Clone, Copy)]
enum RenderOutcome {
    Blank,
    Fails,
}

/// The width and height from a PNG header, or `None` for bytes that are not a PNG.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

struct StubBackend {
    render: RenderOutcome,
}

impl Plugin for StubBackend {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for StubBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        if png_size(image_bytes) == Some((RASTER_WIDTH_PX, RASTER_HEIGHT_PX)) {
            let mut recovered = ExtractedDocument::default();
            recovered.content = RECOVERED_WORD.to_string();
            return Ok(recovered);
        }
        match self.render {
            RenderOutcome::Blank => Ok(ExtractedDocument::default()),
            RenderOutcome::Fails => Err(xberg::XbergError::Ocr {
                message: "stub render failure".to_string(),
                source: None,
            }),
        }
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// A two-page PDF: page 1 (object 6) is empty, page 2 (object 3) paints one grey DeviceGray
/// raster over the page.
fn pdf_with_image_on_page_two() -> Vec<u8> {
    let raster = vec![0xA0u8; (RASTER_WIDTH_PX * RASTER_HEIGHT_PX) as usize];
    let content = format!("q {PAGE_PT} 0 0 {PAGE_PT} 0 0 cm /Im0 Do Q\n");

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [6 0 R 3 0 R] /Count 2>>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents 4 0 R \
             /Resources <</XObject <</Im0 5 0 R>> >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "4 0 obj\n<</Length {}>>\nstream\n{content}\nendstream\nendobj\n",
            content.len() + 1
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<</Type /XObject /Subtype /Image /Width {RASTER_WIDTH_PX} /Height {RASTER_HEIGHT_PX} \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {}>>\nstream\n",
            raster.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&raster);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!("6 0 obj\n<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R>>\nendobj\n").as_bytes(),
    );

    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// `force_ocr_pages` on the page with the raster, with or without one Tesseract pipeline stage.
fn per_page_config(with_pipeline: bool) -> ExtractionConfig {
    let pipeline = with_pipeline.then(|| OcrPipelineConfig {
        stages: vec![OcrPipelineStage {
            backend: "tesseract".to_string(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        }],
        quality_thresholds: Default::default(),
    });
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            pipeline,
            ..Default::default()
        }),
        force_ocr_pages: Some(vec![2]),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    }
}

/// Extract the fixture with the stub backend.
fn extract(render: RenderOutcome, config: &ExtractionConfig) -> ExtractedDocument {
    let pdf = pdf_with_image_on_page_two();
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(StubBackend { render })).expect("the stub backend registers");
    let result = extract_bytes_document_blocking(&pdf, "application/pdf", config);
    unregister_ocr_backend("tesseract").expect("the stub backend unregisters");
    result.expect("the retry must recover the page, so extraction must not error")
}

fn assert_recovered(result: &ExtractedDocument, expected_warning: &str) {
    assert!(
        result.content.contains(RECOVERED_WORD),
        "the page's text must come from its embedded image; content: {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(expected_warning)),
        "the retry must be reported; warnings: {:?}",
        result.processing_warnings
    );
}

/// The one record a failed and recovered page 2 gives.
fn recovered_page_two() -> Vec<OcrPageFailure> {
    vec![OcrPageFailure {
        page: 2,
        error: "OCR error: stub render failure".to_string(),
        recovered: true,
    }]
}

#[test]
#[serial_test::serial]
fn a_blank_page_is_retried_on_its_embedded_image_without_a_pipeline() {
    let result = extract(RenderOutcome::Blank, &per_page_config(false));
    assert_recovered(&result, RETRY_WARNING);
    assert_eq!(result.ocr_page_failures, vec![], "a blank page is not a failed page");
}

#[test]
#[serial_test::serial]
fn a_blank_page_is_retried_on_its_embedded_image_with_a_pipeline() {
    let result = extract(RenderOutcome::Blank, &per_page_config(true));
    assert_recovered(&result, RETRY_WARNING);
    assert_eq!(result.ocr_page_failures, vec![], "a blank page is not a failed page");
}

#[test]
#[serial_test::serial]
fn a_failed_page_recovered_from_its_embedded_image_is_recorded_without_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(false));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(result.ocr_page_failures, recovered_page_two());
}

#[test]
#[serial_test::serial]
fn a_failed_page_recovered_from_its_embedded_image_is_recorded_with_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(true));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(result.ocr_page_failures, recovered_page_two());
}

#[test]
#[serial_test::serial]
fn a_failed_page_is_retried_on_its_embedded_image_without_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(false));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
}

#[test]
#[serial_test::serial]
fn a_failed_page_is_retried_on_its_embedded_image_with_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(true));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
}
