use super::*;

const MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const DECK: &[u8] = include_bytes!("../../../tests/fixtures/office/merged_table.pptx");

fn fixture_path() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/office/merged_table.pptx"
    ))
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        max_archive_depth: 0,
        include_document_structure: true,
        output_format: crate::core::config::OutputFormat::Markdown,
        ..Default::default()
    }
}

fn assert_expected_table(doc: &InternalDocument) {
    let result = crate::extraction::derive::derive_extraction_result(
        doc.clone(),
        true,
        crate::core::config::OutputFormat::Markdown,
    );
    assert_eq!(result.mime_type, MIME);
    let table = result.tables.first().expect("the fixture contains a merged table");
    let row = table
        .cells
        .iter()
        .find(|row| row.iter().any(|cell| cell == "101"))
        .expect("the source fuse row is retained");
    let values = ["101", "40A", "Blower"].map(|value| row.iter().position(|cell| cell == value).unwrap());
    assert!(values[0] < values[1] && values[1] < values[2], "{row:?}");
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_pptx_file_waits_for_admission_on_shared_runtime() {
    use std::future::poll_fn;
    use std::task::Poll;
    use std::time::Duration;

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let extractor = PptxExtractor::new();
    for (cancel_before_poll, cancel_while_queued) in [(false, false), (false, true), (true, false)] {
        let cancel = CancellationToken::new();
        if cancel_before_poll {
            cancel.cancel();
        }
        let cfg = ExtractionConfig {
            cancel_token: Some(cancel.clone()),
            ..config()
        };
        let (queued, result) = runtime.block_on(async {
            let permits = tokio::time::timeout(
                Duration::from_secs(5),
                PATH_PARSERS.acquire_many(MAX_ACTIVE_PATH_PARSERS),
            )
            .await
            .expect("acquire every PPTX path slot")
            .unwrap();
            let mut read = extractor.extract_path(fixture_path(), MIME, &cfg);
            let first = poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await;
            if cancel_while_queued {
                cancel.cancel();
            }
            drop(permits);
            match first {
                Poll::Pending => (true, read.await),
                Poll::Ready(result) => (false, result),
            }
        });
        assert_eq!(queued, !cancel_before_poll, "shared path parse must wait for a slot");
        if cancel_before_poll || cancel_while_queued {
            assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        } else {
            assert_expected_table(&result.unwrap());
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_pptx_bytes_and_batch_paths_ignore_held_path_permits() {
    use std::future::poll_fn;
    use std::task::Poll;
    use std::time::Duration;

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let extractor = PptxExtractor::new();
    let cfg = config();
    let results = runtime.block_on(async {
        let _permits = tokio::time::timeout(
            Duration::from_secs(5),
            PATH_PARSERS.acquire_many(MAX_ACTIVE_PATH_PARSERS),
        )
        .await
        .expect("acquire every PPTX path slot")
        .unwrap();
        let bytes = poll_fn(|cx| {
            let mut read = extractor.extract_content(DECK, MIME, &cfg);
            Poll::Ready(read.as_mut().poll(cx))
        })
        .await;
        let mut path = std::pin::pin!(crate::core::batch_mode::with_batch_mode(extractor.extract_path(
            fixture_path(),
            MIME,
            &cfg
        )));
        let batch_path = poll_fn(|cx| Poll::Ready(path.as_mut().poll(cx))).await;
        [bytes, batch_path]
    });
    for result in results {
        let Poll::Ready(result) = result else {
            panic!("PPTX byte single and path batch routes should complete inline");
        };
        assert_expected_table(&result.unwrap());
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn direct_pptx_path_from_the_only_foreign_blocking_worker_completes() {
    use std::time::Duration;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let handle = runtime.handle().clone();
    let parse = runtime.spawn_blocking(move || {
        handle.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                PptxExtractor::new().extract_path(fixture_path(), MIME, &config()),
            )
            .await
        })
    });
    let document = runtime
        .block_on(parse)
        .unwrap()
        .expect("direct path cannot wait on its own blocking worker")
        .unwrap();
    assert_expected_table(&document);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn dropped_pptx_path_future_keeps_its_permit_until_the_parser_returns() {
    use std::future::poll_fn;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::task::Poll;
    use std::time::Duration;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let admission = Arc::new(tokio::sync::Semaphore::new(1));
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let parse = move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Err(crate::XbergError::Cancelled)
    };
    let mut future = Box::pin(run_pptx_path_worker(
        runtime.handle(),
        Arc::clone(&admission),
        None,
        parse,
    ));
    let first = runtime.block_on(poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))));
    assert!(first.is_pending());
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(future);
    assert_eq!(admission.available_permits(), 0);
    release_tx.send(()).unwrap();
    let permit = runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(5), admission.acquire()).await })
        .expect("parser releases its permit")
        .unwrap();
    drop(permit);
    assert_eq!(admission.available_permits(), 1);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn pptx_single_and_batch_keep_exact_document_shape_per_route() {
    crate::core::runtime::global_runtime().unwrap().block_on(async {
        let extractor = PptxExtractor::new();
        let cfg = config();
        for file in [false, true] {
            let read = || {
                if file {
                    extractor.extract_path(fixture_path(), MIME, &cfg)
                } else {
                    extractor.extract_content(DECK, MIME, &cfg)
                }
            };
            let single = read().await.unwrap();
            let batch = crate::core::batch_mode::with_batch_mode(read()).await.unwrap();
            assert_expected_table(&single);
            assert_eq!(
                serde_json::to_value(single).unwrap(),
                serde_json::to_value(batch).unwrap()
            );
        }
    });
}

#[test]
fn pptx_reads_the_same_fixture_without_a_tokio_runtime() {
    let extractor = PptxExtractor::new();
    let cfg = config();
    for result in [
        futures::executor::block_on(extractor.extract_content(DECK, MIME, &cfg)),
        futures::executor::block_on(extractor.extract_path(fixture_path(), MIME, &cfg)),
    ] {
        assert_expected_table(&result.unwrap());
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn pptx_keeps_archive_limit_and_missing_path_errors() {
    crate::core::runtime::global_runtime().unwrap().block_on(async {
        let extractor = PptxExtractor::new();
        let cfg = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_files_in_archive: 1,
                ..Default::default()
            }),
            ..config()
        };
        for result in [
            extractor.extract_content(DECK, MIME, &cfg).await,
            extractor.extract_path(fixture_path(), MIME, &cfg).await,
        ] {
            assert!(matches!(result, Err(crate::XbergError::Validation { ref message, .. })
            if message.contains("exceeds configured limit of 1")));
        }
        let temp = tempfile::tempdir().unwrap();
        let result = extractor
            .extract_path(&temp.path().join("missing.pptx"), MIME, &config())
            .await;
        assert!(matches!(result, Err(crate::XbergError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound));
    });
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_pptx_parse_warnings_reach_the_callers_thread_local_subscriber() {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

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

    let mut deck = ZipWriter::new_append(io::Cursor::new(DECK.to_vec())).unwrap();
    deck.start_file("ppt/commentAuthors.xml", SimpleFileOptions::default())
        .unwrap();
    deck.write_all(b"<commentAuthors>").unwrap();
    let bytes = deck.finish().unwrap().into_inner();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("comment-authors.pptx");
    std::fs::write(&path, &bytes).unwrap();

    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || writer.clone())
        .finish();
    let runtime = crate::core::runtime::global_runtime().unwrap();
    tracing::callsite::rebuild_interest_cache();
    tracing::subscriber::with_default(subscriber, || {
        let doc = runtime
            .block_on(PptxExtractor::new().extract_path(&path, MIME, &config()))
            .unwrap();
        assert!(
            doc.processing_warnings
                .iter()
                .any(|warning| warning.message.contains("comment authors were not resolved"))
        );
    });
    tracing::callsite::rebuild_interest_cache();
    let capture = capture.0.lock().unwrap();
    let logged = std::str::from_utf8(&capture).unwrap();
    assert_eq!(logged.matches("failed to parse ppt/commentAuthors.xml").count(), 1);
}
