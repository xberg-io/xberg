use super::Engine;
use crate::{ExtractInput, ExtractionConfig};

const IMAGE: &[u8] = include_bytes!("../../tests/fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png");

fn plain_sparse_config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "output_format": "plain",
        "pages": {"extract_pages": true},
        "ocr": {"backend": "tesseract", "language": ["eng"]}
    }))
    .unwrap()
}

#[test]
fn explicit_sparse_plain_stops_without_publishing_rows() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(check_stopped_sparse_ocr)
        .unwrap()
        .join()
        .unwrap();
}

fn check_stopped_sparse_ocr() {
    use crate::{XbergError, cancellation::CancellationToken};
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use std::time::Duration;

    let engine = Engine::new_default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for cancel in [false, true] {
            let (entered, ready) = tokio::sync::oneshot::channel();
            let (release, wait) = tokio::sync::oneshot::channel::<()>();
            let blocker = tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                let _ = wait.blocking_recv();
            });
            ready.await.unwrap();

            let token = CancellationToken::new();
            let mut config = plain_sparse_config();
            config.cancel_token = Some(token.clone());
            config.extraction_timeout_secs = Some(1);
            let ocr = config.ocr.as_mut().unwrap();
            ocr.tesseract_config.get_or_insert_default().psm = Some(11);
            // Poll where the only blocking worker is occupied. The public wrapper
            // would run the same extraction on the shared runtime instead. ~keep
            let input = ExtractInput::from_bytes(IMAGE.to_vec(), "image/png", None);
            let mut extraction = std::pin::pin!(engine.extract_on_current_stack(input, &config));
            poll_fn(|cx| {
                assert!(extraction.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;

            if cancel {
                token.cancel();
            }
            let result = tokio::time::timeout(Duration::from_secs(10), extraction)
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
