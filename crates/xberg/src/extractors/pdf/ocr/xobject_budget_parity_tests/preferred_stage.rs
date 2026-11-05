//! Routing-only policy observations after real native-page rotation admission fails. ~keep
use super::super::{
    PageOcrHints, estimate_png_encode_page_peak_bytes, recover_pipeline_document_from_image_xobjects,
    run_ocr_pipeline_for_page, upright_raster_for_backend, validate_png_encode_pages_individually,
};
use super::support::{Observed, Records, TEXT, fixture};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig};
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, PageOrientationHandling, Plugin};
use crate::types::ExtractedDocument;
use image::ImageEncoder;
use std::sync::{Arc, Mutex};
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

const PREFERRED: &str = "preferred-stage-routing-2339";
const SECONDARY: &str = "secondary-stage-routing-2339";
const CANCEL: &str = "post-cancel-routing-2339";

struct Observer {
    name: &'static str,
    records: Records,
    cancel: Option<crate::cancellation::CancellationToken>,
}

impl Plugin for Observer {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Observer {
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
        self.records.lock().unwrap().push(Observed {
            bytes: bytes.to_vec(),
            config: config.clone(),
        });
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        Ok(ExtractedDocument {
            content: TEXT.to_string(),
            ..Default::default()
        })
    }
}

struct Guard(Vec<&'static str>);
impl Drop for Guard {
    fn drop(&mut self) {
        for name in &self.0 {
            crate::plugins::unregister_ocr_backend(name).unwrap();
        }
    }
}

fn observer(name: &'static str, cancel: Option<crate::cancellation::CancellationToken>) -> Records {
    let records = Arc::new(Mutex::new(Vec::new()));
    crate::plugins::register_ocr_backend(Arc::new(Observer {
        name,
        records: records.clone(),
        cancel,
    }))
    .unwrap();
    records
}

struct Frame {
    pdf: Vec<u8>,
    native: xberg_native_pdf::PdfDocument,
    original: Vec<u8>,
    raster: image::DynamicImage,
    cap: usize,
    source_dpi: f64,
    points_per_pixel: f32,
}

fn rotated_source() -> (Vec<u8>, xberg_native_pdf::PdfDocument, Vec<u8>) {
    let original = fixture(1, false);
    let mut pdf = lopdf::Document::load_mem(&original.bytes).unwrap();
    let pages = pdf.get_pages();
    assert_eq!(pages.len(), 1);
    pdf.get_object_mut(pages[&1])
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Rotate", 90);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(native.page_count().unwrap(), 1);
    assert_eq!(native.get_page_rotation(0).unwrap(), 90);
    assert_eq!(native.get_page_media_box(0).unwrap(), (0.0, 0.0, 72.0, 72.0));
    assert!(native.extract_text_auto(0).unwrap().trim().is_empty());
    let handles = native.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].paint_order, 0);
    assert_eq!((handles[0].width, handles[0].height), (32, 24));
    assert_eq!((handles[0].bbox.width, handles[0].bbox.height), (32.0, 24.0));
    assert_eq!(handles[0].filter_chain, vec![xberg_native_pdf::PdfFilter::DCTDecode]);
    assert_eq!(
        handles[0]
            .raw_compressed_bytes_with_limit(original.streams[0].len())
            .unwrap(),
        original.streams[0]
    );
    drop(handles);
    (bytes, native, original.streams.into_iter().next().unwrap())
}

fn frame() -> Frame {
    let (pdf, native, original) = rotated_source();
    let mut options = RenderOptions::default();
    options.dpi = 400;
    options.format = ImageFormat::Png;
    let rendered = render_page(&native, 0, &options).unwrap();
    assert_eq!((rendered.width, rendered.height), (400, 400));
    let (png, width, height) =
        crate::pdf::render::normalize_rendered_page_for_ocr(rendered.data, rendered.width, rendered.height, 90)
            .unwrap();
    let raster = image::DynamicImage::ImageRgb8(image::load_from_memory(&png).unwrap().into_rgb8());
    assert_eq!((raster.width(), raster.height()), (width, height));
    assert!(
        raster
            .as_rgb8()
            .unwrap()
            .pixels()
            .any(|pixel| pixel.0.iter().all(|value| *value < 200)),
        "actual native renderer painted the real dark source image"
    );
    let cap = usize::try_from(estimate_png_encode_page_peak_bytes(width, height).unwrap()).unwrap();
    assert!(original.len() < cap);
    let limits = SecurityLimits {
        max_content_size: cap,
        ..Default::default()
    };
    validate_png_encode_pages_individually(std::iter::once(&raster), &limits).unwrap();
    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new(&mut encoded)
        .write_image(raster.as_bytes(), width, height, image::ColorType::Rgb8.into())
        .unwrap();
    let encoded = Arc::new(encoded);
    assert_eq!(image::load_from_memory(&encoded).unwrap().width(), width);
    let error = upright_raster_for_backend(
        &encoded,
        width,
        height,
        90,
        PageOrientationHandling::RequiresUpright,
        Some(&limits),
    )
    .expect_err("real page encoding fits but the real rotation decode peak does not");
    assert!(matches!(error, crate::XbergError::Validation { .. }), "{error:?}");
    let higher = SecurityLimits::default();
    assert!(higher.max_content_size > cap);
    let accepted = upright_raster_for_backend(
        &encoded,
        width,
        height,
        90,
        PageOrientationHandling::RequiresUpright,
        Some(&higher),
    )
    .expect("same real source and rotation succeed under a genuinely larger cap");
    assert!(!accepted.0.is_empty());
    assert_eq!((accepted.1, accepted.2, accepted.3), (height, width, 90));
    let source_dpi = 72.0 * f64::from(width) / 72.0;
    Frame {
        pdf,
        native,
        original,
        raster,
        cap,
        source_dpi,
        points_per_pixel: 72.0 / width as f32,
    }
}

fn pipeline(cap: usize, matching: bool, unavailable: bool) -> (ExtractionConfig, OcrPipelineConfig, serde_json::Value) {
    let preprocessing = serde_json::json!({
        "target_dpi":300,"auto_rotate":false,"deskew":false,"denoise":false,
        "contrast_enhance":true,"binarization_method":"none","invert_colors":false,"normalize_shaded_rows":false
    });
    let preferred = serde_json::json!({
        "backend":PREFERRED,"priority":100,"language":["fra"],
        "tesseract_config":{"psm":6,"preprocessing":preprocessing},
        "backend_options":{"route_marker":"preferred","source_dpi":72}
    });
    let mut stages = vec![
        preferred.clone(),
        serde_json::json!({
            "backend":SECONDARY,"priority":1,"language":["deu"],
            "backend_options":{"route_marker":"secondary","source_dpi":72}
        }),
    ];
    if unavailable {
        stages.push(serde_json::json!({
            "backend":"absent-preferred-stage-routing-2339","priority":1000,"language":["ita"],
            "backend_options":{"route_marker":"unavailable","source_dpi":72}
        }));
    }
    let pipeline: OcrPipelineConfig = serde_json::from_value(serde_json::json!({"stages":stages})).unwrap();
    let mut ocr: OcrConfig = if matching {
        serde_json::from_value(serde_json::json!({
            "backend":PREFERRED,"language":["fra"],"tesseract_config":preferred["tesseract_config"],
            "backend_options":preferred["backend_options"]
        }))
        .unwrap()
    } else {
        serde_json::from_value(serde_json::json!({
            "backend":PREFERRED,"language":["eng"],
            "backend_options":{"route_marker":"base","source_dpi":72}
        }))
        .unwrap()
    };
    ocr.security_limits = Some(SecurityLimits {
        max_content_size: cap,
        ..Default::default()
    });
    ocr.pipeline = Some(pipeline.clone());
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: ocr.security_limits.clone(),
        ocr: Some(ocr),
        ..Default::default()
    };
    let expected = serde_json::to_value(
        serde_json::from_value::<crate::types::TesseractConfig>(preferred["tesseract_config"].clone()).unwrap(),
    )
    .unwrap();
    (config, pipeline, expected)
}

async fn assert_preferred(detached: bool, matching: bool, unavailable: bool) {
    let frame = frame();
    let primary = observer(PREFERRED, None);
    let secondary = observer(SECONDARY, None);
    let _guard = Guard(vec![PREFERRED, SECONDARY]);
    let (config, pipeline, expected_tesseract) = pipeline(frame.cap, matching, unavailable);
    assert_eq!(pipeline.stages.len(), if unavailable { 3 } else { 2 });
    if unavailable {
        assert!(
            crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .get("absent-preferred-stage-routing-2339")
                .is_err()
        );
    }
    let images = [frame.raster];
    let result = run_ocr_pipeline_for_page(
        if detached { None } else { Some(&frame.pdf) },
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        90,
        false,
        Some(frame.points_per_pixel),
        Some(PageOcrHints {
            source_dpi: Some(frame.source_dpi),
            known_full_page_scan: false,
            single_block_pages: None,
            native_page_layers: None,
            ..Default::default()
        }),
        0,
        Some(super::super::DetachedOcrPage {
            document: &frame.native,
            #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
            orientation_retry: None,
        }),
    )
    .await
    .expect("actual preparation failures reach existing final original-image retry");
    assert_eq!(result.0, TEXT);
    assert_eq!(result.5, vec![TEXT.to_string()]);
    assert!(
        secondary.lock().unwrap().is_empty(),
        "failed page stages must not call the secondary observer"
    );
    let calls = primary.lock().unwrap();
    assert_eq!(calls.len(), 1, "only the original-image retry can reach a backend");
    assert_eq!(calls[0].bytes, frame.original, "no page PNG or substituted source");
    let observed = &calls[0].config;
    assert_eq!(observed.security_limits.as_ref().unwrap().max_content_size, frame.cap);
    assert_eq!(observed.language, vec!["fra".to_string()]);
    let options = observed.backend_options.as_ref().unwrap();
    assert_eq!(options["route_marker"], "preferred");
    assert_eq!(
        options["source_dpi"].as_f64(),
        Some(72.0),
        "original source density, not rendered400DPI"
    );
    assert_eq!(options["use_cache"], false);
    assert_eq!(
        serde_json::to_value(&observed.tesseract_config).unwrap(),
        expected_tesseract
    );
}

#[tokio::test]
#[serial_test::serial]
async fn whole_retry_uses_preferred_stage_policy_before_page_hints() {
    assert_preferred(false, false, false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn detached_retry_uses_preferred_stage_policy_before_page_hints() {
    assert_preferred(true, false, false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn unavailable_higher_priority_stage_does_not_supply_retry_policy() {
    assert_preferred(false, false, true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn matching_base_and_preferred_policy_is_a_transport_control() {
    assert_preferred(false, true, false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn whole_retry_discards_content_when_the_backend_cancels_during_await() {
    let frame = fixture(1, false);
    let token = crate::cancellation::CancellationToken::new();
    let calls = observer(CANCEL, Some(token.clone()));
    let _guard = Guard(vec![CANCEL]);
    let (mut config, mut ocr) = super::support::config(16_384, 16_384);
    ocr.backend = CANCEL.to_string();
    ocr.backend_options = Some(serde_json::json!({"source_dpi":72}));
    config.ocr = Some(ocr.clone());
    config.cancel_token = Some(token.clone());
    assert!(!token.is_cancelled());
    let result = recover_pipeline_document_from_image_xobjects(Some(&frame.bytes), &config, &ocr, CANCEL).await;
    assert!(token.is_cancelled());
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].bytes, frame.streams[0]);
    assert!(
        matches!(result, Err(crate::XbergError::Cancelled)),
        "late successful text must not escape cancellation"
    );
}
