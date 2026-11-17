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
fn assert_native_queue(file: bool) {
    use std::future::poll_fn;
    use std::sync::mpsc;
    use std::task::Poll;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let extractor = PptxExtractor::new();

    for (cancel_before_poll, cancel_while_queued) in [(false, false), (false, true), (true, false)] {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let blocker = runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
        started_rx.recv().unwrap();
        let cancel = crate::cancellation::CancellationToken::new();
        if cancel_before_poll {
            cancel.cancel();
        }
        let cfg = ExtractionConfig {
            cancel_token: Some(cancel.clone()),
            ..config()
        };
        let (queued, result) = runtime.block_on(async {
            let mut read = if file {
                extractor.extract_path(fixture_path(), MIME, &cfg)
            } else {
                extractor.extract_content(DECK, MIME, &cfg)
            };
            let first = poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await;
            if cancel_while_queued {
                cancel.cancel();
            }
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            match first {
                Poll::Pending => (true, read.await),
                Poll::Ready(result) => (false, result),
            }
        });
        assert_eq!(
            queued, !cancel_before_poll,
            "native PPTX parse queues before reading; file={file}"
        );
        if cancel_before_poll || cancel_while_queued {
            assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        } else {
            assert_expected_table(&result.unwrap());
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_pptx_bytes_queue_on_native_tokio() {
    assert_native_queue(false);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn single_pptx_file_queues_on_native_tokio() {
    assert_native_queue(true);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[tokio::test]
async fn pptx_single_and_batch_keep_exact_document_shape_per_route() {
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

#[tokio::test]
async fn pptx_keeps_archive_limit_and_missing_path_errors() {
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
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    tracing::callsite::rebuild_interest_cache();
    tracing::subscriber::with_default(subscriber, || {
        for input in [
            PptxInput::Bytes(Cow::Borrowed(&bytes)),
            PptxInput::Path(path.to_string_lossy()),
        ] {
            let (_, warnings) = runtime
                .block_on(read_pptx(input, pptx_parse_options(&config()), None))
                .unwrap();
            assert!(
                warnings
                    .iter()
                    .any(|warning| warning.message.contains("comment authors were not resolved"))
            );
        }
    });
    tracing::callsite::rebuild_interest_cache();
    let capture = capture.0.lock().unwrap();
    let logged = std::str::from_utf8(&capture).unwrap();
    assert_eq!(logged.matches("failed to parse ppt/commentAuthors.xml").count(), 2);
}
