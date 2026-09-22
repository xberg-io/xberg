//! Centralized image OCR processing.
//!
//! Provides a shared function for processing extracted images with OCR,
//! used by DOCX, PPTX, Jupyter, Markdown, and other extractors.
//!
//! # Recursion Prevention
//!
//! The OCR results produced here set `images: None` to prevent any
//! downstream consumer from triggering further image extraction on
//! OCR output. This breaks the potential cycle:
//! document → extract images → OCR images → (no further image extraction).
//!
//! # Concurrency
//!
//! Image OCR tasks within one extraction operation are processed with a bounded
//! concurrency limit derived from the general thread budget
//! (`core::config::concurrency::resolve_thread_budget`) to prevent resource
//! exhaustion when documents contain many embedded images.
//!
//! This limit is deliberately *not* derived from any VLM-specific request limit
//! (e.g. `OcrConfig::vlm_config::max_concurrency`), even when the configured backend
//! or fallback policy can reach a VLM: this call site mixes CPU-bound raster/OCR work
//! with potential remote requests, and a per-extraction VLM knob would still leave
//! aggregate provider concurrency across concurrent extractions unbounded (see #1465).
//! A real, global provider-side limit is enforced once per shared LLM client instead —
//! see [`crate::llm::client::create_client`].

use crate::types::{ExtractedDocument, ExtractedImage};

/// Process extracted images with OCR if configured.
///
/// For each image, spawns an async OCR task using the backend from the registry
/// and stores the result in `image.ocr_result`. If OCR is not configured or
/// fails for an individual image, that image's `ocr_result` remains `None`.
///
/// This function is the single shared implementation used by all
/// document extractors (DOCX, PPTX, Jupyter, Markdown, etc.).
///
/// # Recursion Safety
///
/// The produced `ExtractedDocument` for each image explicitly sets
/// `images: None`, preventing further image extraction cycles when
/// OCR results are consumed by archive or recursive extraction paths.
///
/// # Concurrency
///
/// Concurrency within the current extraction is bounded by the general thread
/// budget (never a VLM-specific request limit — see the module docs) using a
/// replenished task set, so queued images do not create an unbounded number of
/// futures. Concurrent document extractions each enforce their own limit.
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
pub(crate) async fn process_images_with_ocr(
    mut images: Vec<ExtractedImage>,
    config: &crate::core::config::ExtractionConfig,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> crate::Result<Vec<ExtractedImage>> {
    if images.is_empty() || config.ocr.is_none() {
        return Ok(images);
    }

    let ocr_config = config.ocr.as_ref().unwrap();
    let output_format = config.output_format.clone();
    let acceleration = ocr_config.acceleration.clone();
    // GH#1554: `OcrConfig::security_limits` has no other way to reach a caller's configured
    // limits — the `OcrBackend::process_image` trait method takes only `OcrConfig`, not
    // `ExtractionConfig` — so it must be copied onto each per-image clone here, mirroring
    // `acceleration` immediately above. `ExtractionConfig::security_limits` is the source of
    // truth; a backend seeing `None` here must fall back to `SecurityLimits::default()`,
    // never disable the check. ~keep
    let security_limits = config.security_limits.clone();

    use std::collections::VecDeque;
    use tokio::task::JoinSet;

    let max_tasks = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    type OcrTaskResult = (usize, crate::Result<ExtractedDocument>);
    type PendingOcrTask = (usize, bytes::Bytes, crate::core::config::OcrConfig);
    let mut join_set: JoinSet<OcrTaskResult> = JoinSet::new();
    let mut pending: VecDeque<PendingOcrTask> = VecDeque::with_capacity(images.len());

    for (idx, image) in images.iter().enumerate() {
        let image_data = image.data.clone();
        let mut ocr_config_clone = ocr_config.clone();
        ocr_config_clone.output_format = Some(output_format.clone());
        ocr_config_clone.acceleration = acceleration.clone();
        // Conditional for the same reason as the standalone-image route (GH#1651): do not
        // replace a directly-set `OcrConfig::security_limits` with `None`. ~keep
        if let Some(limits) = security_limits.clone() {
            ocr_config_clone.security_limits = Some(limits);
        }
        pending.push_back((idx, image_data, ocr_config_clone));
    }

    let spawn_task = |join_set: &mut JoinSet<OcrTaskResult>, (idx, image_data, ocr_config_clone): PendingOcrTask| {
        join_set.spawn(async move {
            let backend = {
                let registry = crate::plugins::registry::get_ocr_backend_registry();
                let registry = registry.read();
                match registry.get(&ocr_config_clone.backend) {
                    Ok(b) => b.clone(),
                    Err(e) => {
                        return (
                            idx,
                            Err(crate::XbergError::Ocr {
                                message: format!("OCR backend '{}' not found: {}", ocr_config_clone.backend, e),
                                source: None,
                            }),
                        );
                    }
                }
            };

            let ocr_result = backend.process_image(&image_data, &ocr_config_clone).await;
            (idx, ocr_result)
        });
    };

    while join_set.len() < max_tasks {
        let Some(task) = pending.pop_front() else {
            break;
        };
        spawn_task(&mut join_set, task);
    }

    while let Some(join_result) = join_set.join_next().await {
        let (idx, ocr_result) = join_result.map_err(|e| crate::XbergError::Ocr {
            message: format!("OCR task panicked: {}", e),
            source: None,
        })?;

        match ocr_result {
            Ok(extraction_result) => {
                // Keep the backend's result whole. Rebuilding it field-by-field silently
                // dropped everything the backend populated besides content/mime_type/
                // ocr_elements — tables, metadata (OCR language, PSM, confidence),
                // formulas, llm_usage (VLM cost accounting), detected_languages and
                // processing_warnings. The PDF inline-image path already stores the
                // backend result unmodified; mirror it here.
                let mut ocr_document = extraction_result;
                // Recursion guard: OCR output must never carry nested images, or an
                // archive/recursive consumer would extract images out of OCR output.
                ocr_document.images = None;
                ocr_config.apply_public_element_policy(&mut ocr_document);
                images[idx].ocr_result = Some(Box::new(ocr_document));
            }
            Err(e) => {
                warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("image_ocr"),
                    message: std::borrow::Cow::Owned(format!("Image {} OCR failed: {}", idx, e)),
                });
                images[idx].ocr_result = None;
            }
        }

        if let Some(task) = pending.pop_front() {
            spawn_task(&mut join_set, task);
        }
    }

    Ok(images)
}

#[cfg(all(test, feature = "ocr", feature = "tokio-runtime"))]
mod tests {
    use std::borrow::Cow;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;
    use bytes::Bytes;
    use tokio::sync::Notify;

    use super::*;
    use crate::core::config::{ConcurrencyConfig, LlmConfig, OcrConfig, VlmFallbackPolicy};
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

    const BACKEND_NAME: &str = "thread-budget-concurrency-test-backend";
    const POLICY_BACKEND_NAME: &str = "embedded-image-element-policy-test-backend";

    struct RegistrationGuard;

    impl Drop for RegistrationGuard {
        fn drop(&mut self) {
            let _ = crate::plugins::unregister_ocr_backend(BACKEND_NAME);
        }
    }

    /// An OCR backend that counts every call that starts, then parks forever on a
    /// [`Notify`] the test never fires.
    ///
    /// `process_images_with_ocr` spawns exactly `min(max_tasks, images.len())` tasks in a
    /// synchronous loop before its first `.await` point (the `while join_set.len() <
    /// max_tasks` loop), and only spawns a replacement task once an existing one
    /// completes. Since every call here blocks forever, no replacement is ever spawned, so
    /// the final call count is a deterministic fact about `max_tasks` — not a race won by
    /// however many tasks happen to be "in flight" at some observed instant.
    struct GatedBackend {
        calls: Arc<AtomicUsize>,
        gate: Arc<Notify>,
    }

    const SECURITY_LIMITS_CAPTURE_BACKEND_NAME: &str = "security-limits-capture-test-backend";

    /// Captures the `security_limits` on the `OcrConfig` it receives, so the test can assert
    /// on what actually reached the backend rather than trusting the caller's intent.
    struct SecurityLimitsCaptureBackend {
        observed_max_content_size: Arc<std::sync::Mutex<Option<Option<usize>>>>,
    }

    impl Plugin for SecurityLimitsCaptureBackend {
        fn name(&self) -> &str {
            SECURITY_LIMITS_CAPTURE_BACKEND_NAME
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

    #[async_trait]
    impl OcrBackend for SecurityLimitsCaptureBackend {
        async fn process_image(&self, _image_bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            let observed = config.security_limits.as_ref().map(|limits| limits.max_content_size);
            *self.observed_max_content_size.lock().unwrap() = Some(observed);
            Ok(ExtractedDocument::default())
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    /// GH#1554 regression: `ExtractionConfig::security_limits` must reach the `OcrConfig`
    /// handed to `OcrBackend::process_image` for embedded-image OCR (DOCX/PPTX/etc.), the
    /// same way `OcrConfig::acceleration` already does. Before this fix, `OcrConfig` had no
    /// `security_limits` field at all, so a caller's configured, possibly higher, limit could
    /// never reach a backend through this path — every backend silently decoded under
    /// `SecurityLimits::default()` regardless of what the caller configured.
    #[tokio::test]
    async fn extraction_config_security_limits_reach_embedded_image_ocr_config() {
        let observed_max_content_size = Arc::new(std::sync::Mutex::new(None));
        crate::plugins::register_ocr_backend(Arc::new(SecurityLimitsCaptureBackend {
            observed_max_content_size: Arc::clone(&observed_max_content_size),
        }))
        .expect("register security-limits capture OCR backend");
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = crate::plugins::unregister_ocr_backend(SECURITY_LIMITS_CAPTURE_BACKEND_NAME);
            }
        }
        let _guard = Guard;

        let configured_limit = 200 * 1024 * 1024;
        let config = crate::core::config::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: SECURITY_LIMITS_CAPTURE_BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: configured_limit,
                ..Default::default()
            }),
            ..Default::default()
        };
        let images = vec![ExtractedImage {
            data: Bytes::from_static(b"image"),
            ..Default::default()
        }];
        let mut warnings = Vec::new();

        process_images_with_ocr(images, &config, &mut warnings)
            .await
            .expect("embedded-image OCR must succeed");

        assert!(warnings.is_empty());
        let observed = observed_max_content_size.lock().unwrap();
        assert_eq!(
            *observed,
            Some(Some(configured_limit)),
            "OcrConfig::security_limits must carry the caller's ExtractionConfig::security_limits"
        );
    }

    struct PolicyIgnoringBackend;

    impl Plugin for PolicyIgnoringBackend {
        fn name(&self) -> &str {
            POLICY_BACKEND_NAME
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

    #[async_trait]
    impl OcrBackend for PolicyIgnoringBackend {
        async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            Ok(ExtractedDocument {
                content: "embedded OCR".to_string(),
                ocr_elements: Some(vec![crate::types::OcrElement {
                    text: "backend element".to_string(),
                    page_number: 1,
                    ..Default::default()
                }]),
                ..Default::default()
            })
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    impl Plugin for GatedBackend {
        fn name(&self) -> &str {
            BACKEND_NAME
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

    #[async_trait]
    impl OcrBackend for GatedBackend {
        async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            // Never notified: this call parks here for the rest of the test.
            self.gate.notified().await;
            Ok(ExtractedDocument {
                content: "unreachable".to_string(),
                mime_type: Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    /// Regression test for GH#1465.
    ///
    /// Before the fix, image OCR concurrency was `resolve_ocr_concurrency`, which prefers
    /// `OcrConfig::vlm_config::max_concurrency` over the general thread budget whenever
    /// `vlm_fallback` is not `Disabled` — even though this call site mixes CPU-bound OCR
    /// work with, at most, occasional remote VLM requests (see the module docs). A small
    /// general thread budget (2) paired with a much larger VLM limit (6) must now bound
    /// concurrency at 2, not 6: the general thread budget governs this CPU-bound batch
    /// size unconditionally.
    #[tokio::test]
    async fn general_thread_budget_bounds_image_ocr_batch_not_vlm_max_concurrency() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(Notify::new());
        crate::plugins::register_ocr_backend(Arc::new(GatedBackend {
            calls: Arc::clone(&calls),
            gate: Arc::clone(&gate),
        }))
        .expect("register gated OCR backend");
        let _registration = RegistrationGuard;

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                vlm_fallback: VlmFallbackPolicy::Always,
                vlm_config: Some(LlmConfig {
                    model: "test/model".to_string(),
                    max_concurrency: Some(6),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            concurrency: Some(ConcurrencyConfig {
                max_threads: Some(2),
                max_concurrent_ocr: None,
            }),
            ..Default::default()
        };
        let images = (0..6)
            .map(|_| ExtractedImage {
                data: Bytes::from_static(b"image"),
                ..Default::default()
            })
            .collect();
        let mut warnings = Vec::new();

        // None of the 6 spawned tasks can ever complete (the gate is never notified), so
        // this always times out. The timeout only gives the runtime a chance to run every
        // task that was actually spawned before the test inspects `calls`; dropping the
        // timed-out future aborts them via `JoinSet`'s `Drop` impl.
        let _ = tokio::time::timeout(
            Duration::from_millis(200),
            process_images_with_ocr(images, &config, &mut warnings),
        )
        .await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "expected the general thread budget (2), not the larger VLM max_concurrency (6), \
             to bound the number of image OCR tasks started concurrently"
        );
    }

    #[tokio::test]
    async fn custom_backend_cannot_bypass_embedded_image_element_policy() {
        crate::plugins::register_ocr_backend(Arc::new(PolicyIgnoringBackend))
            .expect("register policy-ignoring OCR backend");
        let config = crate::core::config::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: POLICY_BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let images = vec![ExtractedImage {
            data: Bytes::from_static(b"image"),
            ..Default::default()
        }];
        let mut warnings = Vec::new();

        let images = process_images_with_ocr(images, &config, &mut warnings)
            .await
            .expect("embedded-image OCR must succeed");
        let nested = images[0].ocr_result.as_ref().expect("OCR result must be preserved");

        assert_eq!(nested.content, "embedded OCR");
        assert!(nested.ocr_elements.is_none());
        assert!(warnings.is_empty());
        crate::plugins::unregister_ocr_backend(POLICY_BACKEND_NAME).unwrap();
    }
}
