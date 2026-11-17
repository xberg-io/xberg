use super::*;

const NATIVE_TABLES: &[u8] = include_bytes!("../../../tests/fixtures/ppt/native_tables.ppt");

fn check_native_table_deck(doc: InternalDocument) {
    assert_eq!(doc.metadata.additional["slide_count"], 2);
    let result =
        crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Markdown);
    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.tables[0].page_number, 1);
    assert_eq!(result.tables[0].cells[1], ["North", "27,039", "253"]);
    assert_eq!(result.tables[1].page_number, 2);
    assert_eq!(result.tables[1].cells[0], ["Site", "Quarter totals", ""]);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_ppt_parse_queues_on_native_tokio_and_honors_cancellation() {
    use crate::cancellation::CancellationToken;
    use std::future::poll_fn;
    use std::sync::mpsc;
    use std::task::Poll;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let extractor = PptExtractor::new();

    for (before_submit, while_queued) in [(false, false), (false, true), (true, false)] {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let blocker = runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
        started_rx.recv().unwrap();
        let cancel = CancellationToken::new();
        if before_submit {
            cancel.cancel();
        }
        let config = ExtractionConfig {
            max_archive_depth: 0,
            cancel_token: Some(cancel.clone()),
            ..Default::default()
        };
        let (queued, result) = runtime.block_on(async {
            let mut extraction = extractor.extract_content(NATIVE_TABLES, LEGACY_POWERPOINT_MIME_TYPE, &config);
            let first = poll_fn(|context| Poll::Ready(extraction.as_mut().poll(context))).await;
            if while_queued {
                cancel.cancel();
            }
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            match first {
                Poll::Pending => (true, extraction.await),
                Poll::Ready(result) => (false, result),
            }
        });

        assert_eq!(
            queued, !before_submit,
            "native PPT read must queue unless already cancelled"
        );
        if before_submit || while_queued {
            assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        } else {
            check_native_table_deck(result.unwrap());
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[tokio::test]
async fn ppt_single_and_batch_keep_exact_native_table_deck_shape() {
    let extractor = PptExtractor::new();
    let config = ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };
    let extraction = || extractor.extract_content(NATIVE_TABLES, LEGACY_POWERPOINT_MIME_TYPE, &config);
    let single = extraction().await.unwrap();
    let batch = crate::core::batch_mode::with_batch_mode(extraction()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&single).unwrap(),
        serde_json::to_value(&batch).unwrap()
    );
    let fixture = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ppt/native_tables.ppt"
    ));
    let file = extractor
        .extract_path(fixture, LEGACY_POWERPOINT_MIME_TYPE, &config)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&single).unwrap(),
        serde_json::to_value(&file).unwrap()
    );
    check_native_table_deck(single);
}

#[test]
fn ppt_parses_the_same_deck_without_a_tokio_runtime() {
    let extractor = PptExtractor::new();
    let config = ExtractionConfig {
        max_archive_depth: 0,
        ..Default::default()
    };
    let result =
        futures::executor::block_on(extractor.extract_content(NATIVE_TABLES, LEGACY_POWERPOINT_MIME_TYPE, &config));
    check_native_table_deck(result.unwrap());
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_ppt_fallback_log_reaches_the_callers_thread_local_subscriber() {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut deck = cfb::CompoundFile::open(io::Cursor::new(NATIVE_TABLES.to_vec())).unwrap();
    deck.remove_stream("/Current User").unwrap();
    let content = deck.into_inner().into_inner();
    let config = ExtractionConfig {
        max_archive_depth: 0,
        ..Default::default()
    };
    let extractor = PptExtractor::new();
    let extract = || extractor.extract_content(&content, LEGACY_POWERPOINT_MIME_TYPE, &config);
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    let count_events = || {
        let output = capture.0.lock().unwrap();
        std::str::from_utf8(&output)
            .unwrap()
            .matches("no live slide list resolved from the persist chain; using stream order")
            .count()
    };

    tracing::callsite::rebuild_interest_cache();
    let (inline, offloaded) = tracing::subscriber::with_default(subscriber, || {
        let inline = futures::executor::block_on(extract()).unwrap();
        assert_eq!(count_events(), 1, "inline parse emits the fallback log");
        let offloaded = runtime.block_on(extract()).unwrap();
        (inline, offloaded)
    });
    tracing::callsite::rebuild_interest_cache();
    assert_eq!(count_events(), 2, "offloaded parse emits the fallback log");
    assert_eq!(
        serde_json::to_value(&inline).unwrap(),
        serde_json::to_value(&offloaded).unwrap()
    );
    check_native_table_deck(offloaded);
}
