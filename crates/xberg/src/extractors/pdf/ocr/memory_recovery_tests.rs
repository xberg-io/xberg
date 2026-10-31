use super::*;
use crate::core::config::{OcrConfig, OcrPipelineConfig};
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, PageOrientationHandling, Plugin};
use crate::types::ExtractedDocument;
use image::ImageEncoder;
use lopdf::{Document, Object, Stream, dictionary};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const BACKEND: &str = "memory-budget-xobject-recovery-2339";
const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;
const RECOVERED: &str = "Recovered text from the original embedded image without decoding the over-budget page raster.";

struct RecoveryBackend {
    calls: Arc<AtomicUsize>,
    cancel_during_recovery: Option<(crate::cancellation::CancellationToken, bool)>,
    observed_content_cap: Option<Arc<AtomicUsize>>,
}

impl Plugin for RecoveryBackend {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for RecoveryBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn page_orientation_handling(&self) -> PageOrientationHandling {
        PageOrientationHandling::RequiresUpright
    }
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            bytes.starts_with(&[0xff, 0xd8]),
            "the rejected page PNG must never reach OCR"
        );
        let limits = config.security_limits.clone().unwrap_or_default();
        if let Some(observed) = &self.observed_content_cap {
            observed.store(limits.max_content_size, Ordering::SeqCst);
        }
        let decoded = crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(bytes, &limits)?;
        assert_eq!(decoded.dimensions(), (32, 24));
        if let Some((token, return_content)) = &self.cancel_during_recovery {
            token.cancel();
            if !return_content {
                return Err(crate::XbergError::Cancelled);
            }
        }
        Ok(ExtractedDocument {
            content: RECOVERED.to_string(),
            ..Default::default()
        })
    }
}

struct BackendGuard;

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = crate::plugins::unregister_ocr_backend(BACKEND);
    }
}

fn register_backend() -> (BackendGuard, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    crate::plugins::register_ocr_backend(Arc::new(RecoveryBackend {
        calls: calls.clone(),
        cancel_during_recovery: None,
        observed_content_cap: None,
    }))
    .unwrap();
    (BackendGuard, calls)
}

fn limits() -> SecurityLimits {
    SecurityLimits {
        max_content_size: usize::try_from(estimate_png_encode_page_peak_bytes(WIDTH, HEIGHT).unwrap()).unwrap(),
        ..Default::default()
    }
}

fn raster() -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(WIDTH, HEIGHT, image::Rgb([245, 245, 245])))
}

fn encode_png(image: &image::DynamicImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_bytes(),
            image.width(),
            image.height(),
            image::ColorType::Rgb8.into(),
        )
        .unwrap();
    bytes
}

fn assert_decode_budget_failure(image: &image::DynamicImage) {
    let limits = limits();
    validate_png_encode_pages_individually(std::iter::once(image), &limits)
        .expect("the real page-raster encode budget passes");
    let bytes = Arc::new(encode_png(image));
    let error = upright_raster_for_backend(
        &bytes,
        WIDTH,
        HEIGHT,
        90,
        PageOrientationHandling::RequiresUpright,
        Some(&limits),
    )
    .expect_err("rotation's image-decode peak exceeds the same limit before decoding");
    assert!(matches!(&error, crate::XbergError::Validation { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains("Image dimensions 400x300 require"), "{message}");
    assert!(message.contains("live image-processing bytes"), "{message}");
}

fn embedded_image_pdf() -> Vec<u8> {
    let pixels = image::RgbImage::from_pixel(32, 24, image::Rgb([0, 0, 0]));
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(&pixels, 32, 24, image::ColorType::Rgb8.into())
        .unwrap();
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let image = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 32, "Height" => 24,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        jpeg,
    ));
    let content = document.add_object(Stream::new(dictionary! {}, b"q 72 0 0 72 0 0 cm /Im0 Do Q".to_vec()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(72), Object::Integer(72)],
        "Rotate" => 90, "Contents" => content, "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

async fn run_case(
    content: Option<&[u8]>,
    source: Option<&xberg_native_pdf::PdfDocument>,
    page_index_offset: usize,
) -> OcrRunResult<OcrPipelineOutput> {
    run_case_with_image_capture(content, source, page_index_offset, false).await
}

async fn run_case_with_image_capture(
    content: Option<&[u8]>,
    source: Option<&xberg_native_pdf::PdfDocument>,
    page_index_offset: usize,
    capture_images: bool,
) -> OcrRunResult<OcrPipelineOutput> {
    let pipeline: OcrPipelineConfig = serde_json::from_value(serde_json::json!({
        "stages": [{"backend": BACKEND, "priority": 1}],
    }))
    .unwrap();
    let limits = limits();
    let config = ExtractionConfig {
        use_cache: false,
        images: capture_images.then(|| crate::core::config::ImageExtractionConfig {
            extract_images: true,
            include_page_rasters: true,
            ..Default::default()
        }),
        security_limits: Some(limits.clone()),
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            security_limits: Some(limits),
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [raster()];
    assert_decode_budget_failure(&images[0]);
    Box::pin(run_ocr_pipeline_for_page(
        content,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        90,
        false,
        Some(72.0 / HEIGHT as f32),
        None,
        page_index_offset,
        source,
    ))
    .await
}

#[test]
fn page_raster_budget_passes_but_decode_peak_remains_a_hard_stop() {
    assert_decode_budget_failure(&raster());
}

#[tokio::test]
#[serial_test::serial]
async fn detached_page_recovers_available_source_after_decode_budget_failure() {
    let (_guard, calls) = register_backend();
    let bytes = embedded_image_pdf();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    let output = run_case(None, Some(&doc), 0)
        .await
        .expect("available source image must remain recoverable");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(output.5, vec![RECOVERED.to_string()]);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "only the bounded original JPEG reaches OCR"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn content_backed_page_retains_existing_decode_budget_recovery() {
    let (_guard, calls) = register_backend();
    let bytes = embedded_image_pdf();
    let output = run_case(Some(&bytes), None, 0)
        .await
        .expect("existing content-backed recovery succeeds");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial]
async fn decode_budget_failure_without_source_still_reports_the_error() {
    let (_guard, calls) = register_backend();
    let error = run_case(None, None, 0)
        .await
        .expect_err("no source means there is no safe recovery")
        .error
        .to_string();
    assert!(error.contains("All OCR pipeline backends failed"), "{error}");
    assert!(error.contains("live image-processing bytes"), "{error}");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "never forward the over-budget PNG to OCR"
    );
}

fn embedded_image_after_blank_page() -> Vec<u8> {
    let mut doc = Document::load_mem(&embedded_image_pdf()).unwrap();
    let pages = doc.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
    let image_page = doc.get_pages()[&1];
    let blank = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(72), Object::Integer(72)],
    });
    let page_tree = doc.get_object_mut(pages).unwrap().as_dict_mut().unwrap();
    page_tree.set("Kids", vec![Object::Reference(blank), Object::Reference(image_page)]);
    page_tree.set("Count", 2);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
#[serial_test::serial]
async fn detached_page_recovery_does_not_return_unrequested_document_pages() {
    let (_guard, calls) = register_backend();
    let bytes = embedded_image_after_blank_page();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    assert_eq!(doc.page_count().unwrap(), 2);
    let output = run_case(None, Some(&doc), 1)
        .await
        .expect("the requested source page remains recoverable");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(
        output.5,
        vec![RECOVERED.to_string()],
        "recover only this detached page, not the whole source PDF"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial]
async fn detached_page_recovery_uses_the_source_page_number_in_markers() {
    let (_guard, calls) = register_backend();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_after_blank_page()).unwrap();
    let config = ExtractionConfig {
        pages: Some(crate::core::config::PageConfig {
            insert_page_markers: true,
            marker_format: "[page {page_num}]\n".to_string(),
            ..Default::default()
        }),
        security_limits: Some(limits()),
        ..Default::default()
    };
    let ocr_config = OcrConfig {
        security_limits: Some(limits()),
        ..Default::default()
    };
    let outcome = recover_pipeline_page_from_image_xobjects(&doc, 1, &config, &ocr_config, BACKEND)
        .await
        .unwrap()
        .expect("the second source page contains a recoverable JPEG");
    assert_eq!(outcome.text, format!("[page 2]\n{RECOVERED}"));
    assert_eq!(outcome.page_texts, vec![RECOVERED.to_string()]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial]
async fn cancelled_detached_page_recovery_does_not_call_the_backend() {
    let (_guard, calls) = register_backend();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_pdf()).unwrap();
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let config = ExtractionConfig {
        cancel_token: Some(token),
        ..Default::default()
    };
    let result = recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &OcrConfig::default(), BACKEND).await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

async fn assert_cancellation_during_recovery(return_content: bool) {
    let token = crate::cancellation::CancellationToken::new();
    let calls = Arc::new(AtomicUsize::new(0));
    crate::plugins::register_ocr_backend(Arc::new(RecoveryBackend {
        calls: calls.clone(),
        cancel_during_recovery: Some((token.clone(), return_content)),
        observed_content_cap: None,
    }))
    .unwrap();
    let _guard = BackendGuard;
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_pdf()).unwrap();
    let config = ExtractionConfig {
        cancel_token: Some(token.clone()),
        ..Default::default()
    };
    let result = recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &OcrConfig::default(), BACKEND).await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "cancellation occurs inside the backend"
    );
    assert!(token.is_cancelled());
    assert!(
        matches!(result, Err(crate::XbergError::Cancelled)),
        "recovery must preserve cancellation even when the backend returns content or its error is ignored"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn cancellation_during_recovery_discards_backend_content() {
    assert_cancellation_during_recovery(true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn cancellation_during_recovery_survives_a_backend_cancelled_error() {
    assert_cancellation_during_recovery(false).await;
}

fn assert_rotation_remains_a_decode_budget_failure(cap: usize) {
    let image = raster();
    let limits = SecurityLimits {
        max_content_size: cap,
        ..Default::default()
    };
    validate_png_encode_pages_individually(std::iter::once(&image), &limits).unwrap();
    let error = upright_raster_for_backend(
        &Arc::new(encode_png(&image)),
        WIDTH,
        HEIGHT,
        90,
        PageOrientationHandling::RequiresUpright,
        Some(&limits),
    )
    .expect_err("both explicit limits must reject page decoding, not page encoding");
    assert!(error.to_string().contains("live image-processing bytes"));
}

async fn run_case_with_explicit_caps(
    doc: &xberg_native_pdf::PdfDocument,
    extraction_cap: usize,
    ocr_cap: usize,
) -> OcrRunResult<OcrPipelineOutput> {
    let pipeline: OcrPipelineConfig = serde_json::from_value(serde_json::json!({
        "stages": [{"backend": BACKEND, "priority": 1}],
    }))
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: Some(SecurityLimits {
            max_content_size: extraction_cap,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            security_limits: Some(SecurityLimits {
                max_content_size: ocr_cap,
                ..Default::default()
            }),
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    Box::pin(run_ocr_pipeline_for_page(
        None,
        Some(&[raster()]),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        90,
        false,
        Some(72.0 / HEIGHT as f32),
        None,
        0,
        Some(doc),
    ))
    .await
}

async fn assert_recovery_uses_the_lower_explicit_cap(extraction_cap: usize, ocr_cap: usize) {
    assert_rotation_remains_a_decode_budget_failure(extraction_cap);
    assert_rotation_remains_a_decode_budget_failure(ocr_cap);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(AtomicUsize::new(usize::MAX));
    crate::plugins::register_ocr_backend(Arc::new(RecoveryBackend {
        calls: calls.clone(),
        cancel_during_recovery: None,
        observed_content_cap: Some(observed.clone()),
    }))
    .unwrap();
    let _guard = BackendGuard;
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_pdf()).unwrap();
    let output = run_case_with_explicit_caps(&doc, extraction_cap, ocr_cap)
        .await
        .expect("recover the original bounded JPEG after the PNG decode limit fails");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only one JPEG reaches the backend");
    assert_eq!(observed.load(Ordering::SeqCst), extraction_cap.min(ocr_cap));
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_backend_observes_the_lower_extraction_cap() {
    assert_recovery_uses_the_lower_explicit_cap(1_462_144, 1_463_144).await;
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_backend_observes_the_lower_ocr_cap() {
    assert_recovery_uses_the_lower_explicit_cap(1_463_144, 1_462_144).await;
}

fn embedded_jpeg_after_unsupported_image_on_second_page() -> Vec<u8> {
    let mut pdf = Document::load_mem(&embedded_image_after_blank_page()).unwrap();
    let page = pdf.get_pages()[&2];
    let jpeg = pdf
        .get_object(page)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Im0")
        .unwrap()
        .as_reference()
        .unwrap();
    let mut skipped = pdf.get_object(jpeg).unwrap().as_stream().unwrap().clone();
    let mut encoded = skipped
        .content
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    encoded.push('>');
    skipped.set_content(encoded.into_bytes());
    skipped.dict.set(
        "Filter",
        vec![
            Object::Name(b"ASCIIHexDecode".to_vec()),
            Object::Name(b"DCTDecode".to_vec()),
        ],
    );
    let skipped = pdf.add_object(skipped);
    assert_ne!(
        skipped, jpeg,
        "the skipped and retained images are distinct source objects"
    );
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"q 72 0 0 72 0 0 cm /Im0 Do /Im1 Do Q".to_vec(),
    ));
    let page = pdf.get_object_mut(page).unwrap().as_dict_mut().unwrap();
    page.set("Contents", content);
    page.set(
        "Resources",
        dictionary! { "XObject" => dictionary! { "Im0" => skipped, "Im1" => jpeg } },
    );
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

async fn assert_detached_image_preserves_source_identity(bytes: Vec<u8>, expected_index: usize) {
    let (_guard, calls) = register_backend();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    assert_eq!(doc.page_count().unwrap(), 2);
    let handles = doc.page_image_handles(1).unwrap();
    assert_eq!(handles.len(), expected_index + 1);
    assert_eq!(handles[expected_index].paint_order, expected_index);
    assert_eq!(
        handles[expected_index].filter_chain,
        vec![xberg_native_pdf::PdfFilter::DCTDecode]
    );
    if expected_index > 0 {
        assert_eq!(
            handles[0].filter_chain.len(),
            2,
            "the first painted image has an unsupported outer filter"
        );
    }
    let output = run_case_with_image_capture(None, Some(&doc), 1, true)
        .await
        .expect("the original JPEG remains recoverable");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(output.5, vec![RECOVERED.to_string()]);
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only the supported JPEG reaches OCR");
    let images = output.6.expect("recovered source image metadata");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].page_number, Some(2));
    assert_eq!(images[0].image_index, expected_index as u32);
    assert_eq!(
        images[0].source_path.as_deref(),
        Some(format!("xobject:page2:{expected_index}").as_str())
    );
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_preserves_the_source_index_after_an_unsupported_first_image() {
    assert_detached_image_preserves_source_identity(embedded_jpeg_after_unsupported_image_on_second_page(), 1).await;
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_preserves_the_source_index_when_no_images_are_skipped() {
    assert_detached_image_preserves_source_identity(embedded_image_after_blank_page(), 0).await;
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_warning_reports_stage_failure_without_claiming_a_blank_raster() {
    let (_guard, calls) = register_backend();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_after_blank_page()).unwrap();
    let output = run_case(None, Some(&doc), 1)
        .await
        .expect("recover after the raster decode budget fails");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let document = output.3.expect("pipeline warnings remain attached to recovered text");
    let warnings = document
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.contains("embedded image bytes"))
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].source.as_ref(), "ocr");
    assert_eq!(
        warnings[0].message.as_ref(),
        "Page 2 contains 1 image XObject(s); OCR was retried on the embedded image bytes after all page OCR stages failed."
    );
    for inaccurate_claim in ["rendered blank", "could not draw"] {
        assert!(
            !warnings[0].message.contains(inaccurate_claim),
            "a valid nonblank raster exists"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn legacy_document_recovery_retains_its_existing_warning() {
    let (_guard, calls) = register_backend();
    let bytes = embedded_image_after_blank_page();
    let recovery = recover_pipeline_document_from_image_xobjects(
        Some(&bytes),
        &ExtractionConfig::default(),
        &OcrConfig::default(),
        BACKEND,
    )
    .await
    .unwrap()
    .expect("the existing document recovery finds the second page's JPEG");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(recovery.warnings.len(), 1);
    assert_eq!(recovery.warnings[0].source.as_ref(), "ocr");
    assert_eq!(
        recovery.warnings[0].message.as_ref(),
        "Page 2 rendered blank but contains 1 image XObject(s) the PDF rasterizer could not draw; OCR was retried on the embedded image bytes."
    );
    assert_eq!(recovery.images.len(), 1);
    assert_eq!(
        recovery.images[0].description.as_deref(),
        Some("recovered from raw image XObject (embedded_jpeg) after the page rasterizer produced a blank page")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn detached_recovery_image_description_reports_stage_failure_without_claiming_a_blank_raster() {
    let (_guard, calls) = register_backend();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(embedded_image_after_blank_page()).unwrap();
    let output = run_case_with_image_capture(None, Some(&doc), 1, true)
        .await
        .expect("recover after a nonblank page raster's decode budget fails");
    assert_eq!(output.0, RECOVERED);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let images = output.6.expect("requested recovered source image metadata");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].page_number, Some(2));
    assert_eq!(images[0].source_path.as_deref(), Some("xobject:page2:0"));
    assert_eq!(
        images[0].description.as_deref(),
        Some("recovered from raw image XObject (embedded_jpeg) after all page OCR stages failed")
    );
}
