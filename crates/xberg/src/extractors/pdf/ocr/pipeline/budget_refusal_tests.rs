use super::*;
use crate::core::config::OcrConfig;
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use std::error::Error as _;
use std::sync::Arc;

const BLANK: &str = "budget-refusal-blank-backend";
const TALL_JPEG: &[u8] = include_bytes!("../../../../../../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf");

struct BlankBackend;

impl Plugin for BlankBackend {
    fn name(&self) -> &str {
        BLANK
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for BlankBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        Ok(crate::ExtractedDocument::default())
    }
}

struct BackendGuard;
impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(BLANK);
    }
}

async fn read_refused_page(with_pipeline: bool) -> OcrRunResult<MixedOcrRun> {
    register_ocr_backend(Arc::new(BlankBackend)).unwrap();
    let _guard = BackendGuard;
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: BLANK.to_owned(),
            pipeline: with_pipeline.then(|| {
                serde_json::from_value(serde_json::json!({ "stages": [{"backend": BLANK, "priority": 100}] })).unwrap()
            }),
            ..Default::default()
        }),
        security_limits: Some(SecurityLimits {
            max_content_size: 16 * 1024 * 1024,
            ..Default::default()
        }),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];
    extract_mixed_ocr_native_with_layout_inputs(
        "",
        &boundaries,
        MixedOcrPageSelection {
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis: None,
        },
        TALL_JPEG,
        &config,
        AllPagesFailedPolicy::ReturnError,
        MixedLayoutInputs::Resolve(None),
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn empty_embedded_image_retry_surfaces_the_original_render_refusal() {
    for with_pipeline in [false, true] {
        let Err(failed) = read_refused_page(with_pipeline).await else {
            panic!("pipeline={with_pipeline}: a refused page that recovered no text reported success");
        };
        assert!(
            failed
                .error
                .source()
                .is_some_and(|source| source.is::<crate::extractors::pdf::ocr::rendering::SourceImageBudgetExceeded>()),
            "pipeline={with_pipeline}: the typed budget refusal must survive: {:?}",
            failed.error
        );
    }
}

#[test]
fn retry_causes_are_capped_and_keep_the_typed_source() {
    use crate::extractors::pdf::ocr::pipeline::with_retry_causes;
    let original = crate::XbergError::Parsing {
        message: "Failed to render PDF page 1".to_owned(),
        source: Some(Box::new(std::io::Error::other("render refused"))),
    };
    let causes: Vec<String> = (0..5)
        .map(|i| format!("image {i}: exceeds a configured size limit"))
        .collect();
    let crate::XbergError::Parsing { message, source } = with_retry_causes(original, &causes) else {
        panic!("a parsing error stays a parsing error");
    };
    assert!(
        message.contains("image 2: exceeds a configured size limit; +2 more"),
        "{message}"
    );
    assert!(!message.contains("image 3"), "{message}");
    assert_eq!(source.expect("typed source kept").to_string(), "render refused");

    let untouched = crate::XbergError::Parsing {
        message: "Failed to render PDF page 1".to_owned(),
        source: None,
    };
    let crate::XbergError::Parsing { message, .. } = with_retry_causes(untouched, &[]) else {
        panic!("a parsing error stays a parsing error");
    };
    assert_eq!(message, "Failed to render PDF page 1");
}

struct RecordingBackend(Arc<std::sync::Mutex<Vec<Option<serde_json::Value>>>>);

impl Plugin for RecordingBackend {
    fn name(&self) -> &str {
        "recording-source-dpi-backend"
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for RecordingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        self.0.lock().unwrap().push(config.backend_options.clone());
        Ok(crate::ExtractedDocument::default())
    }
}

async fn options_seen_by_the_backend(
    use_proven_source_dpi: bool,
    caller_options: Option<serde_json::Value>,
) -> Vec<Option<serde_json::Value>> {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let backend: Arc<dyn OcrBackend> = Arc::new(RecordingBackend(seen.clone()));
    let (document, ..) = open_pdf_for_full_ocr(TALL_JPEG).unwrap();
    let mut budget = crate::extractors::security::SecurityBudget::from_limits(&SecurityLimits::default());
    let ocr_config = OcrConfig {
        backend_options: caller_options,
        ..Default::default()
    };
    let mut context = XObjectRecoveryContext::with_budget(&mut budget, None);
    context.use_proven_source_dpi = use_proven_source_dpi;
    let outcome =
        super::super::rendering::recover_page_text_from_image_xobjects(&backend, &document, 0, 1, &ocr_config, context)
            .await
            .unwrap()
            .expect("the page paints one JPEG");
    assert_eq!(outcome.attempted, 1);
    seen.lock().unwrap().clone()
}

#[tokio::test]
async fn the_retry_hands_the_backend_the_confirmed_resolution_only_when_asked() {
    let dpi = |seen: &[Option<serde_json::Value>]| {
        seen.iter()
            .map(|options| {
                options
                    .as_ref()
                    .and_then(|o| o.get("source_dpi"))
                    .and_then(serde_json::Value::as_f64)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(dpi(&options_seen_by_the_backend(true, None).await), [Some(150.0)]);
    assert_eq!(dpi(&options_seen_by_the_backend(false, None).await), [None]);
    let caller = Some(serde_json::json!({ "source_dpi": 96.0 }));
    assert_eq!(dpi(&options_seen_by_the_backend(true, caller).await), [Some(96.0)]);
}
