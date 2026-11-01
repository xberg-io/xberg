//! The explicit OCR pipeline decodes an oversized standalone JPEG at a reduced size (#2339).
use super::*;
use crate::core::config::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds};
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use std::sync::{Arc, Mutex};

const BACKEND: &str = "reduced-pipeline-jpeg-2339";
const WIDTH: u16 = 10007;
const HEIGHT: u16 = 6343;
const MAX_CONTENT_SIZE: usize = 512 * 1024 * 1024;

type Calls = Arc<Mutex<Vec<((u32, u32), Option<f64>)>>>;

struct Recorder(Calls);

impl Plugin for Recorder {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Recorder {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let dimensions = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .expect("the stage receives a readable image");
        let source_dpi = config
            .backend_options
            .as_ref()
            .and_then(|options| options.get(crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION))
            .and_then(serde_json::Value::as_f64);
        self.0.lock().unwrap().push((dimensions, source_dpi));
        Ok(ExtractedDocument {
            content: "HELLO WORLD".into(),
            ..Default::default()
        })
    }
}

struct Registration;

impl Drop for Registration {
    fn drop(&mut self) {
        crate::plugins::unregister_ocr_backend(BACKEND).expect("remove owned test backend");
    }
}

fn pipeline_config() -> (ExtractionConfig, OcrPipelineConfig) {
    let pipeline = OcrPipelineConfig {
        stages: vec![OcrPipelineStage {
            backend: BACKEND.into(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        }],
        quality_thresholds: OcrQualityThresholds {
            pipeline_min_quality: 0.0,
            ..Default::default()
        },
    };
    let limits = SecurityLimits {
        max_content_size: MAX_CONTENT_SIZE,
        ..Default::default()
    };
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: Some(limits.clone()),
        ocr: Some(OcrConfig {
            backend_options: Some(serde_json::json!({ "source_dpi": 600 })),
            security_limits: Some(limits),
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    (config, pipeline)
}

#[tokio::test]
#[serial_test::serial]
async fn explicit_pipeline_decodes_a_600_dpi_scan_at_the_finest_scale_whose_encode_fits() {
    let full_encode = crate::extractors::pdf::ocr::estimate_png_encode_page_peak_bytes(10007, 6343).unwrap();
    assert_eq!(
        full_encode, 635_006_154,
        "the full-size encode exceeds the 512 MiB limit"
    );
    let rgb = vec![245; usize::from(WIDTH) * usize::from(HEIGHT) * 3];
    let jpeg = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, WIDTH, HEIGHT);
    drop(rgb);
    let calls = Calls::default();
    crate::plugins::register_ocr_backend(Arc::new(Recorder(calls.clone()))).unwrap();
    let _registration = Registration;
    let (config, pipeline) = pipeline_config();
    let result = ImageExtractor::new()
        .extract_with_ocr_pipeline(&jpeg, &config, &pipeline)
        .await;
    assert!(result.is_ok(), "{:?}", result.err());
    // The 1/2 scale holds 5004x3172 pixels, and the stage sees the source DPI divided by 2.
    assert_eq!(calls.lock().unwrap().as_slice(), &[((5004, 3172), Some(300.0))]);
}
