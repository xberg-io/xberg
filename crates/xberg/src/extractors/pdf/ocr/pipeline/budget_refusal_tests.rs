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
