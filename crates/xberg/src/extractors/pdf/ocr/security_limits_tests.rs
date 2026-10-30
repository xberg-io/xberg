use super::pipeline::{estimate_png_encode_page_peak_bytes, extract_with_ocr, run_ocr_pipeline};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig};
use crate::extraction::image_decode::ImageDecodeBudget;
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use std::sync::{Arc, Mutex};

const BACKENDS: [&str; 2] = ["request-limits-first-stage", "request-limits-second-stage"];
const RAISED_LIMIT: usize = 512 * 1024 * 1024;

type ObservedRequest = (&'static str, Option<SecurityLimits>);

struct CapturingBackend {
    name: &'static str,
    seen: Arc<Mutex<Vec<ObservedRequest>>>,
}

impl Plugin for CapturingBackend {
    fn name(&self) -> &str {
        self.name
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
impl OcrBackend for CapturingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.seen
            .lock()
            .unwrap()
            .push((self.name, config.security_limits.clone()));
        Ok(ExtractedDocument::default())
    }
}

struct BackendGuard;

impl Drop for BackendGuard {
    fn drop(&mut self) {
        for backend in BACKENDS {
            let _ = crate::plugins::unregister_ocr_backend(backend);
        }
    }
}

fn pipeline() -> OcrPipelineConfig {
    serde_json::from_value(serde_json::json!({
        "stages": [
            {"backend": BACKENDS[0], "priority": 2},
            {"backend": BACKENDS[1], "priority": 1}
        ],
        "quality_thresholds": {"pipeline_min_quality": 1.0}
    }))
    .unwrap()
}

async fn run_request(config: &ExtractionConfig, use_pipeline: bool) {
    let images = [image::DynamicImage::new_rgb8(4, 4)];
    if use_pipeline {
        run_ocr_pipeline(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            config,
            &pipeline(),
            None,
            None,
        )
        .await
        .unwrap();
    } else {
        extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            config,
            None,
        )
        .await
        .unwrap();
    }
}

fn assert_reported_page_budgets(limits: &SecurityLimits, permitted: bool) {
    for (width, height, expected_bytes) in [(3_193, 4_125, 131_973_394), (2_894, 4_260, 123_546_544)] {
        let peak = estimate_png_encode_page_peak_bytes(width, height).unwrap();
        assert_eq!(peak, expected_bytes);
        let result = ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak);
        if permitted {
            result.expect("the configured 512 MiB budget must admit the reported page");
        } else {
            let error = result.expect_err("the default 100 MiB budget must refuse the reported page");
            assert_eq!(
                error.to_string(),
                format!(
                    "Validation error: Image dimensions {width}x{height} require {peak} live image-processing bytes, \
                     exceeding or invalid under security_limits.max_content_size ({} bytes)",
                    limits.max_content_size
                )
            );
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn should_apply_request_limits_to_direct_and_every_pipeline_backend() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let _guard = BackendGuard;
    for name in BACKENDS {
        crate::plugins::register_ocr_backend(Arc::new(CapturingBackend {
            name,
            seen: Arc::clone(&seen),
        }))
        .unwrap();
    }
    for use_pipeline in [false, true] {
        for configured_limit in [
            None,
            Some(SecurityLimits::default().max_content_size),
            Some(RAISED_LIMIT),
        ] {
            let config = ExtractionConfig {
                use_cache: false,
                ocr: Some(OcrConfig {
                    backend: BACKENDS[0].to_string(),
                    ..Default::default()
                }),
                security_limits: configured_limit.map(|max_content_size| SecurityLimits {
                    max_content_size,
                    ..Default::default()
                }),
                ..Default::default()
            };
            run_request(&config, use_pipeline).await;
            let observations = std::mem::take(&mut *seen.lock().unwrap());
            let expected_backends = if use_pipeline { &BACKENDS[..] } else { &BACKENDS[..1] };
            assert_eq!(observations.len(), expected_backends.len());
            for ((backend, limits), expected_backend) in observations.into_iter().zip(expected_backends) {
                assert_eq!(backend, *expected_backend);
                assert_eq!(limits.as_ref().map(|value| value.max_content_size), configured_limit);
                assert_reported_page_budgets(&limits.unwrap_or_default(), configured_limit == Some(RAISED_LIMIT));
            }
        }
    }
}
