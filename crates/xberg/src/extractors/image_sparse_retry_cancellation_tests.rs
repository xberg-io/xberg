use super::*;
use crate::cancellation::CancellationToken;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, OcrConfidence, OcrElement, OcrElementLevel};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum Fallback {
    Robust,
    Sparse,
    Failed,
    Cancelled,
}

struct SequenceBackend {
    token: CancellationToken,
    cancel_in_fallback: bool,
    fallback: Fallback,
    calls: Mutex<Vec<Option<i32>>>,
}

impl Plugin for SequenceBackend {
    fn name(&self) -> &str {
        "sparse-cancellation-sequence"
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

fn document(content: &str, confidence: f64) -> ExtractedDocument {
    ExtractedDocument {
        content: content.into(),
        ocr_elements: Some(vec![OcrElement {
            text: content.into(),
            level: OcrElementLevel::Word,
            confidence: OcrConfidence {
                recognition: confidence,
                ..Default::default()
            },
            ..Default::default()
        }]),
        ..Default::default()
    }
}

#[async_trait::async_trait]
impl OcrBackend for SequenceBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let raster = image::load_from_memory(bytes).expect("retry must receive valid image bytes");
        assert_eq!((raster.width(), raster.height()), (2, 2));
        let call_count = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(config.tesseract_config.as_ref().and_then(|value| value.psm));
            calls.len()
        };
        match call_count {
            1 => Ok(document("primary", 0.1)),
            2 => {
                if self.cancel_in_fallback {
                    self.token.cancel();
                }
                match self.fallback {
                    Fallback::Robust => Ok(document("fallback", 0.95)),
                    Fallback::Sparse => Ok(document("weak fallback", 0.1)),
                    Fallback::Failed => Err(crate::XbergError::Other("fallback OCR failed".into())),
                    Fallback::Cancelled => Err(crate::XbergError::Cancelled),
                }
            }
            _ => panic!("unexpected third OCR call"),
        }
    }
}

async fn run_retry(fallback: Fallback, cancel_in_fallback: bool) -> crate::Result<ExtractedDocument> {
    let original = OcrConfig::default();
    let mut effective = original.clone();
    apply_default_whole_image_tesseract_psm(&mut effective);
    assert!(should_retry_sparse_image_ocr(&original, &document("primary", 0.1)));
    let token = CancellationToken::new();
    let backend = Arc::new(SequenceBackend {
        token: token.clone(),
        cancel_in_fallback,
        fallback,
        calls: Mutex::new(Vec::new()),
    });
    let erased: Arc<dyn OcrBackend> = backend.clone();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let outcome = process_whole_image_ocr_with_sparse_retry(
        &erased,
        Arc::new(png.into_inner()),
        &original,
        &effective,
        SparseRetryPolicy {
            whole_page_raster: true,
            allow_standard_retry: true,
        },
        Some(&token),
    )
    .await;
    let expected = vec![
        effective.tesseract_config.as_ref().and_then(|value| value.psm),
        sparse_image_ocr_fallback_config(&effective)
            .tesseract_config
            .unwrap()
            .psm,
    ];
    assert_ne!(
        expected[0], expected[1],
        "the second call must use the sparse retry configuration"
    );
    assert_eq!(
        *backend.calls.lock().unwrap(),
        expected,
        "exactly two OCR calls must execute"
    );
    assert_eq!(token.is_cancelled(), cancel_in_fallback);
    outcome
}

#[tokio::test]
async fn should_return_cancelled_when_fallback_returns_robust_text_after_cancellation() {
    assert!(matches!(
        run_retry(Fallback::Robust, true).await,
        Err(crate::XbergError::Cancelled)
    ));
}

#[tokio::test]
async fn should_return_cancelled_when_fallback_returns_sparse_text_after_cancellation() {
    assert!(matches!(
        run_retry(Fallback::Sparse, true).await,
        Err(crate::XbergError::Cancelled)
    ));
}

#[tokio::test]
async fn should_return_cancelled_when_fallback_returns_another_error_after_cancellation() {
    assert!(matches!(
        run_retry(Fallback::Failed, true).await,
        Err(crate::XbergError::Cancelled)
    ));
}

#[tokio::test]
async fn should_keep_robust_fallback_when_not_cancelled() {
    assert_eq!(run_retry(Fallback::Robust, false).await.unwrap().content, "fallback");
}

#[tokio::test]
async fn should_keep_primary_when_uncancelled_fallback_is_sparse() {
    assert_eq!(run_retry(Fallback::Sparse, false).await.unwrap().content, "primary");
}

#[tokio::test]
async fn should_keep_primary_when_uncancelled_fallback_fails() {
    assert_eq!(run_retry(Fallback::Failed, false).await.unwrap().content, "primary");
}

#[tokio::test]
async fn should_propagate_backend_cancellation_even_when_caller_token_is_not_cancelled() {
    assert!(matches!(
        run_retry(Fallback::Cancelled, false).await,
        Err(crate::XbergError::Cancelled)
    ));
}
