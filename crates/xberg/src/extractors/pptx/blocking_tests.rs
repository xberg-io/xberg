use super::*;

const MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const DECK: &[u8] = include_bytes!("../../../tests/fixtures/office/merged_table.pptx");

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn shared_pool() -> &'static crate::core::runtime::OfficeBlockingPool {
    crate::core::runtime::global_runtime()
        .unwrap()
        .block_on(async { crate::core::runtime::current_office_pool().unwrap() })
}

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
#[serial_test::serial(office_admission)]
fn single_pptx_paths_and_bytes_wait_for_admission_and_observe_cancellation() {
    use std::future::poll_fn;
    use std::task::Poll;
    use std::time::Duration;

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let extractor = PptxExtractor::new();
    let pool = shared_pool();
    for is_path in [true, false] {
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
                    pool.admission.acquire_many(pool.slots.try_into().unwrap()),
                )
                .await
                .expect("acquire every PPTX parser slot")
                .unwrap();
                let mut read = if is_path {
                    extractor.extract_path(fixture_path(), MIME, &cfg)
                } else {
                    extractor.extract_content(DECK, MIME, &cfg)
                };
                let first = poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await;
                if cancel_while_queued {
                    cancel.cancel();
                    let result = tokio::time::timeout(Duration::from_millis(500), read)
                        .await
                        .expect("cancellation must wake the waiter before releasing the permits");
                    assert_eq!(pool.admission.available_permits(), 0);
                    return (first.is_pending(), result);
                }
                drop(permits);
                match first {
                    Poll::Pending => (true, read.await),
                    Poll::Ready(result) => (false, result),
                }
            });
            assert_eq!(queued, !cancel_before_poll, "shared parse must wait for a slot");
            if cancel_before_poll || cancel_while_queued {
                assert!(matches!(result, Err(crate::XbergError::Cancelled)));
            } else {
                assert_expected_table(&result.unwrap());
            }
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial(office_admission)]
fn batch_pptx_paths_complete_inline_while_the_single_parse_pool_is_full() {
    use std::future::poll_fn;
    use std::task::Poll;
    use std::time::Duration;

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let pool = shared_pool();
    let extractor = PptxExtractor::new();
    let cfg = config();
    let result = runtime.block_on(async {
        let _permits = tokio::time::timeout(
            Duration::from_secs(5),
            pool.admission.acquire_many(pool.slots.try_into().unwrap()),
        )
        .await
        .expect("acquire every PPTX slot")
        .unwrap();
        let mut path = std::pin::pin!(crate::core::batch_mode::with_batch_mode(extractor.extract_path(
            fixture_path(),
            MIME,
            &cfg
        )));
        poll_fn(|cx| Poll::Ready(path.as_mut().poll(cx))).await
    });
    let Poll::Ready(result) = result else {
        panic!("PPTX path batch should complete inline");
    };
    assert_expected_table(&result.unwrap());
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial(office_admission)]
fn the_public_pptx_deadline_times_out_while_admission_is_held() {
    use std::time::Duration;

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let pool = shared_pool();
    runtime.block_on(async {
        let _permits = tokio::time::timeout(
            Duration::from_secs(5),
            pool.admission.acquire_many(pool.slots.try_into().unwrap()),
        )
        .await
        .expect("acquire every PPTX slot")
        .unwrap();
        let config = ExtractionConfig {
            use_cache: false,
            extraction_timeout_secs: Some(1),
            ..config()
        };
        let input = crate::ExtractInput::from_bytes(DECK.to_vec(), MIME, None);
        let result = tokio::time::timeout(Duration::from_secs(5), crate::extract(input, &config))
            .await
            .expect("the deadline must finish before any permit is released");
        assert!(
            matches!(result, Err(crate::XbergError::Timeout { limit_ms: 1000, .. })),
            "{result:?}"
        );
        assert_eq!(pool.admission.available_permits(), 0);
    });
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial(office_admission)]
fn direct_foreign_pptx_calls_complete_with_their_only_blocking_worker_occupied() {
    use std::time::Duration;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let pool = shared_pool();
    let _permits = runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(5),
            pool.admission.acquire_many(pool.slots.try_into().unwrap()),
        )
        .await
        .expect("hold every dedicated PPTX slot")
        .unwrap()
    });
    let handle = runtime.handle().clone();
    let parse = runtime.spawn_blocking(move || {
        handle.block_on(async {
            let extractor = PptxExtractor::new();
            let cfg = config();
            let path = extractor.extract_path(fixture_path(), MIME, &cfg);
            let bytes = extractor.extract_content(DECK, MIME, &cfg);
            let batch = crate::core::batch_mode::with_batch_mode(extractor.extract_content(DECK, MIME, &cfg));
            for document in [
                tokio::time::timeout(Duration::from_secs(5), path).await,
                tokio::time::timeout(Duration::from_secs(5), bytes).await,
                tokio::time::timeout(Duration::from_secs(5), batch).await,
            ] {
                assert_expected_table(&document.expect("foreign runtime must stay inline").unwrap());
            }
        });
    });
    runtime.block_on(parse).unwrap();
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn dropped_pptx_future_stops_at_a_checkpoint_and_keeps_its_permit_until_then() {
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
    let token = CancellationToken::new();
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (checkpoint_tx, checkpoint_rx) = mpsc::sync_channel(1);
    let parse = move |cancel: PptxReadCancellation<'_>| {
        started_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        checkpoint_tx.send(cancel.check().is_err()).unwrap();
        Ok(())
    };
    let permit = runtime
        .block_on(crate::core::runtime::acquire_office_slot(
            Arc::clone(&admission),
            Some(&token),
        ))
        .unwrap();
    let mut future = Box::pin(parse_on_worker(runtime.handle(), Some(&token), permit, false, parse));
    let first = runtime.block_on(poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))));
    assert!(first.is_pending());
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(future);
    assert_eq!(admission.available_permits(), 0);
    release_tx.send(()).unwrap();
    assert!(checkpoint_rx.recv_timeout(Duration::from_secs(5)).unwrap());
    assert!(
        !token.is_cancelled(),
        "dropping one PPTX read must not cancel other operations"
    );
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

#[cfg(all(feature = "excel", feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial(office_admission)]
fn pptx_excel_and_docx_hold_the_same_blocking_admission_slot() {
    use std::time::Duration;

    use crate::core::runtime::{global_runtime, poll_with_blocking_worker_held};
    use crate::extractors::{DocxExtractor, ExcelExtractor};

    const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
    const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
    let runtime = global_runtime().unwrap();
    let pool = shared_pool();
    let held = runtime
        .block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                pool.admission.acquire_many((pool.slots - 1).try_into().unwrap()),
            )
            .await
        })
        .unwrap()
        .unwrap();
    let cfg = config();
    let pptx = PptxExtractor::new();
    let excel = ExcelExtractor::new();
    let docx = DocxExtractor::new();
    let workbook = include_bytes!("../../../tests/fixtures/office/merged_header.xlsx");
    let document = include_bytes!("../../../tests/fixtures/office/word_footnote_ids.docx");
    for (mime, read) in [
        (MIME, pptx.extract_content(DECK, MIME, &cfg)),
        (XLSX, excel.extract_content(workbook, XLSX, &cfg)),
        (DOCX, docx.extract_content(document, DOCX, &cfg)),
    ] {
        let (queued, entered, result) = poll_with_blocking_worker_held(runtime, read, || {
            assert!(
                pool.admission.try_acquire().is_err(),
                "{mime} entered without the shared permit"
            );
        });
        assert!(queued && entered, "{mime} did not reach the shared worker");
        assert_eq!(result.unwrap().mime_type, mime);
    }
    drop(held);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[tokio::test]
async fn pptx_byte_batch_worker_panic_stays_a_parsing_error() {
    let admission = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let permit = crate::core::runtime::acquire_office_slot(admission.clone(), None)
        .await
        .unwrap();
    let result = parse_on_worker(
        &tokio::runtime::Handle::current(),
        None,
        permit,
        true,
        |_| -> Result<()> { panic!("synthetic PPTX byte batch read panic") },
    )
    .await;
    assert!(matches!(result, Err(crate::XbergError::Parsing { .. })));
    assert_eq!(admission.available_permits(), 1);
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
    let worker_ids = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&worker_ids);
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || {
            observed.lock().unwrap().push(tokio::runtime::Handle::current().id());
            writer.clone()
        })
        .finish();
    let runtime = crate::core::runtime::global_runtime().unwrap();
    let dedicated_id = shared_pool().runtime.handle().id();
    assert_ne!(runtime.handle().id(), dedicated_id);
    tracing::callsite::rebuild_interest_cache();
    tracing::subscriber::with_default(subscriber, || {
        let extractor = PptxExtractor::new();
        for doc in [
            runtime
                .block_on(extractor.extract_path(&path, MIME, &config()))
                .unwrap(),
            runtime
                .block_on(extractor.extract_content(&bytes, MIME, &config()))
                .unwrap(),
        ] {
            assert!(
                doc.processing_warnings
                    .iter()
                    .any(|warning| warning.message.contains("comment authors were not resolved"))
            );
        }
    });
    tracing::callsite::rebuild_interest_cache();
    let capture = capture.0.lock().unwrap();
    let logged = std::str::from_utf8(&capture).unwrap();
    assert_eq!(logged.matches("failed to parse ppt/commentAuthors.xml").count(), 2);
    let worker_ids = worker_ids.lock().unwrap();
    assert!(!worker_ids.is_empty() && worker_ids.iter().all(|id| *id == dedicated_id));
}
