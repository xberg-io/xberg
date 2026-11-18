#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

//! A backend that cancels every page, with no cancel token on the config, must make the public
//! `extract` call fail as `Cancelled`, not as an aggregated OCR or plugin error (#3175). An
//! ordinary failure on every page keeps the first page's error as a typed source.

use std::error::Error as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use xberg::core::config::ExtractInput;
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::ExtractedDocument;
use xberg::{ExtractionConfig, OcrConfig, XbergError};

const BACKEND: &str = "backend-cancel-3175";
const FAILURE: &str = "backend-cancel-3175 page failure";

#[derive(Clone, Copy)]
enum Outcome {
    Cancel,
    Fail,
}

struct EveryPage(Outcome, Arc<AtomicUsize>);

impl Plugin for EveryPage {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for EveryPage {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.1.fetch_add(1, Ordering::SeqCst);
        Err(match self.0 {
            Outcome::Cancel => XbergError::Cancelled,
            Outcome::Fail => XbergError::Ocr {
                message: FAILURE.to_owned(),
                source: None,
            },
        })
    }
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ocr")
            .join(name),
    )
    .expect("fixture must exist")
}

async fn extract_with(outcome: Outcome, bytes: Vec<u8>, mime: &str) -> xberg::Result<xberg::ExtractionResult> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: BACKEND.to_owned(),
            ..Default::default()
        }),
        force_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let _ = unregister_ocr_backend(BACKEND);
    let calls = Arc::new(AtomicUsize::new(0));
    register_ocr_backend(Arc::new(EveryPage(outcome, Arc::clone(&calls)))).expect("the backend registers");
    let result = xberg::extract(ExtractInput::from_bytes(bytes, mime, None), &config).await;
    unregister_ocr_backend(BACKEND).expect("the backend unregisters");
    assert!(calls.load(Ordering::SeqCst) > 0, "the backend never ran");
    result
}

#[tokio::test]
#[serial_test::serial]
async fn a_backend_cancel_on_every_pdf_page_returns_cancelled() {
    let result = extract_with(Outcome::Cancel, fixture("scanned_hello.pdf"), "application/pdf").await;
    assert!(matches!(result, Err(XbergError::Cancelled)), "got {result:?}");
}

#[tokio::test]
#[serial_test::serial]
async fn a_backend_cancel_on_a_single_image_returns_cancelled() {
    let result = extract_with(Outcome::Cancel, fixture("bank_statement_example.png"), "image/png").await;
    assert!(matches!(result, Err(XbergError::Cancelled)), "got {result:?}");
}

#[tokio::test]
#[serial_test::serial]
async fn an_ordinary_failure_on_every_pdf_page_keeps_the_first_error_as_source() {
    let result = extract_with(Outcome::Fail, fixture("scanned_hello.pdf"), "application/pdf").await;
    let error = result.expect_err("every page failed");
    assert!(matches!(error, XbergError::Ocr { .. }), "got {error:?}");
    let mut chain = error.source();
    let mut first_page_error = None;
    while let Some(source) = chain {
        if let Some(XbergError::Ocr { message, .. }) = source.downcast_ref::<XbergError>() {
            first_page_error = Some(message.clone());
        }
        chain = source.source();
    }
    assert_eq!(first_page_error.as_deref(), Some(FAILURE), "source chain of {error:?}");
}
