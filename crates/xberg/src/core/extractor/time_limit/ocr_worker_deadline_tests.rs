//! The extraction deadline releases the caller of an explicit sparse OCR read whose Tesseract worker
//! cannot run, with no rows published.
//!
//! The public `extract` hops to the shared large-stack runtime, whose blocking pool a test cannot
//! hold, so this drives the entry point the hop runs: on a runtime with one blocking thread that
//! the test occupies, the OCR worker is queued until the caller has its answer.

use crate::XbergError;
use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::core::extractor::extract_bytes;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::Duration;

const IMAGE: &[u8] = include_bytes!("../../../../tests/fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png");
const LIMIT_SECS: u64 = 1;
const FAILSAFE: Duration = Duration::from_secs(10);

fn sparse_plain_config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "output_format": OutputFormat::Plain,
        "pages": {"extract_pages": true},
        "ocr": {"backend": "tesseract", "language": ["eng"], "tesseract_config": {"psm": 11}}
    }))
    .unwrap()
}

#[test]
fn explicit_sparse_plain_stops_without_publishing_rows() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        // Control, and the one-time setup off the clock: with the blocking thread free the same read
        // publishes its rows, so the stopped runs below are stopped at the OCR worker.
        let control = Box::pin(extract_bytes(IMAGE, "image/png", &sparse_plain_config()))
            .await
            .expect("an unlimited read must succeed");
        assert_eq!(control.metadata.additional["psm"], "11");
        assert!(
            !control.tables.is_empty(),
            "control: the sparse read publishes its rows"
        );

        for cancel in [false, true] {
            let (entered, ready) = tokio::sync::oneshot::channel();
            let (release, wait) = tokio::sync::oneshot::channel::<()>();
            let blocker = tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                let _ = wait.blocking_recv();
            });
            ready.await.unwrap();

            let token = CancellationToken::new();
            let config = ExtractionConfig {
                cancel_token: Some(token.clone()),
                extraction_timeout_secs: Some(LIMIT_SECS),
                ..sparse_plain_config()
            };
            let mut extraction = Box::pin(extract_bytes(IMAGE, "image/png", &config));
            poll_fn(|cx| {
                assert!(extraction.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;

            if cancel {
                token.cancel();
            }
            let result = tokio::time::timeout(FAILSAFE, extraction)
                .await
                .expect("the extraction deadline must release the caller before the worker");
            release.send(()).unwrap();
            blocker.await.unwrap();
            if cancel {
                assert!(matches!(result, Err(XbergError::Cancelled)), "{result:?}");
            } else {
                assert!(
                    matches!(result, Err(XbergError::Timeout { limit_ms: 1000, .. })),
                    "{result:?}"
                );
            }
        }
    });
}
