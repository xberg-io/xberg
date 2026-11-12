use super::{
    ExtractionConfig, PinnedAvailableMemory, build_minimal_multi_page_pdf, extract_mixed_ocr_native_with_layout_pages,
};
use crate::cancellation::CancellationToken;
use crate::core::config::{ConcurrencyConfig, OcrConfig};
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const BACKEND: &str = "retry-timeout-stop-test";
const FAILURE: &str = "synthetic page recognition failure";

struct StopOnFailure {
    token: CancellationToken,
    calls: AtomicUsize,
    stop_on_call: usize,
    fail_every_call: bool,
}

impl crate::plugins::Plugin for StopOnFailure {
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
impl crate::plugins::OcrBackend for StopOnFailure {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let stopping = self.calls.fetch_add(1, Ordering::Relaxed) + 1 == self.stop_on_call;
        if stopping {
            self.token.request_stop();
        }
        if stopping || self.fail_every_call {
            return Err(crate::XbergError::Ocr {
                message: FAILURE.to_string(),
                source: None,
            });
        }
        Ok(crate::types::ExtractedDocument {
            content: "Text read successfully before the time limit.".to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }
}

struct RegisteredOcrBackendGuard;

impl Drop for RegisteredOcrBackendGuard {
    fn drop(&mut self) {
        let _ = crate::plugins::unregister_ocr_backend(BACKEND);
    }
}

fn setup(
    page_numbers: Vec<u32>,
    fail_every_call: bool,
) -> (Arc<StopOnFailure>, ExtractionConfig, RegisteredOcrBackendGuard) {
    let token = CancellationToken::new();
    let backend = Arc::new(StopOnFailure {
        token: token.clone(),
        calls: AtomicUsize::new(0),
        stop_on_call: page_numbers.len(),
        fail_every_call,
    });
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();
    let config = ExtractionConfig {
        cancel_token: Some(token),
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(if fail_every_call { page_numbers.len() } else { 1 }),
            ..Default::default()
        }),
        force_ocr_pages: Some(page_numbers),
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };
    (backend, config, RegisteredOcrBackendGuard)
}

#[tokio::test]
#[serial]
async fn final_failed_page_keeps_its_warning_and_marks_the_run_cut_short() {
    let (backend, config, _registration) = setup(vec![1, 2, 3], false);
    let native_text = "Native text on the final page";
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: native_text.len(),
        page_number: 3,
    }];
    let run = extract_mixed_ocr_native_with_layout_pages(
        native_text,
        &boundaries,
        (&[1, 2, 3], &[]),
        &build_minimal_multi_page_pdf(3),
        &config,
        None,
        (&[], &[]),
    )
    .await
    .expect("the first two pages were read");
    assert_eq!(backend.calls.load(Ordering::Relaxed), 3);

    let (_, _, _, _, _, _, _, _, warnings, failures) = run.output;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].page, 3);
    assert!(failures[0].error.contains(FAILURE), "{failures:?}");
    assert!(failures[0].recovered, "{failures:?}");
    assert!(
        warnings.iter().any(|warning| {
            warning.source == "ocr"
                && warning.message.contains(FAILURE)
                && warning.message.ends_with("the page's native text was kept.")
        }),
        "{warnings:?}"
    );
    assert_eq!(backend.token.first_unread_page(), Some(3));
    assert!(backend.token.cut_short_by_time_limit());
}

#[tokio::test]
#[serial]
async fn a_stop_that_skips_the_only_failed_pages_retry_returns_timeout() {
    let (backend, config, _registration) = setup(vec![1], false);
    let result = extract_mixed_ocr_native_with_layout_pages(
        "",
        &[],
        (&[1], &[]),
        &build_minimal_multi_page_pdf(1),
        &config,
        None,
        (&[], &[]),
    )
    .await;
    assert_eq!(backend.calls.load(Ordering::Relaxed), 1);
    assert!(
        matches!(result, Err(crate::XbergError::Timeout { .. })),
        "expected a timeout, got {:?}",
        result.err()
    );
    assert_eq!(backend.token.first_unread_page(), Some(1));
}

#[tokio::test]
#[serial]
async fn a_stop_accounts_for_every_pending_failure_in_the_batch() {
    let _memory = PinnedAvailableMemory::set(1024 * 1024 * 1024);
    let (backend, mut config, _registration) = setup(vec![1, 2], true);
    config.force_ocr_pages = None;
    let run = extract_mixed_ocr_native_with_layout_pages(
        "",
        &[],
        (&[1, 2], &[]),
        &build_minimal_multi_page_pdf(2),
        &config,
        None,
        (&[], &[]),
    )
    .await
    .expect("this route preserves the result after OCR failures");
    assert_eq!(backend.calls.load(Ordering::Relaxed), 2);

    let (_, _, _, _, _, _, _, _, warnings, mut failures) = run.output;
    failures.sort_unstable_by_key(|failure| failure.page);
    let pages: Vec<_> = failures.iter().map(|failure| failure.page).collect();
    assert_eq!(pages, [1, 2]);
    for failure in &failures {
        assert!(failure.error.contains(FAILURE), "{failure:?}");
        assert!(!failure.recovered, "{failure:?}");
    }
    let failure_warnings = warnings.iter().filter(|warning| warning.message.contains(FAILURE));
    assert_eq!(failure_warnings.count(), 2);
    assert_eq!(backend.token.first_unread_page(), Some(1));
}
