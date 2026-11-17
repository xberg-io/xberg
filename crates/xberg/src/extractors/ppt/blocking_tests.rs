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
fn single_ppt_shared_runtime_honors_already_cancelled_request() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let config = ExtractionConfig {
        max_archive_depth: 0,
        cancel_token: Some(cancel),
        ..Default::default()
    };
    let err = crate::core::runtime::global_runtime()
        .unwrap()
        .block_on(PptExtractor::new().extract_content(NATIVE_TABLES, LEGACY_POWERPOINT_MIME_TYPE, &config))
        .unwrap_err();
    assert!(matches!(err, crate::XbergError::Cancelled));
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn cancelled_shared_ppt_worker_skips_parsing() {
    use std::future::poll_fn;
    use std::sync::{Mutex, mpsc};
    use std::task::Poll;
    use std::thread::{self, ThreadId};
    use std::time::Duration;
    use tracing::Instrument as _;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{Layer, layer::Context};

    const DEADLINE: Duration = Duration::from_secs(5);
    const INVALID_DECK: &[u8] = b"broken OLE file";

    struct WorkerGate {
        caller: ThreadId,
        entered: mpsc::Sender<()>,
        resume: Mutex<mpsc::Receiver<()>>,
    }

    impl<S: tracing::Subscriber> Layer<S> for WorkerGate {
        fn on_enter(&self, _id: &tracing::span::Id, _context: Context<'_, S>) {
            if thread::current().id() != self.caller {
                self.entered.send(()).unwrap();
                self.resume.lock().unwrap().recv_timeout(DEADLINE).unwrap();
            }
        }
    }

    let extractor = PptExtractor::new();
    let mut config = ExtractionConfig {
        max_archive_depth: 0,
        ..Default::default()
    };
    let control =
        futures::executor::block_on(extractor.extract_content(INVALID_DECK, LEGACY_POWERPOINT_MIME_TYPE, &config));
    assert!(matches!(control, Err(crate::XbergError::Parsing { .. })));

    let cancel = CancellationToken::new();
    config.cancel_token = Some(cancel.clone());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let subscriber = tracing_subscriber::registry().with(WorkerGate {
        caller: thread::current().id(),
        entered: entered_tx,
        resume: Mutex::new(resume_rx),
    });
    let runtime = crate::core::runtime::global_runtime().unwrap();
    let result = tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let parse = extractor.extract_content(INVALID_DECK, LEGACY_POWERPOINT_MIME_TYPE, &config);
            let mut parse = std::pin::pin!(parse.instrument(tracing::info_span!("ppt_cancel_before_parse")));
            assert!(poll_fn(|cx| Poll::Ready(parse.as_mut().poll(cx))).await.is_pending());
            entered_rx
                .recv_timeout(DEADLINE)
                .expect("PPT worker entered before cancellation");
            cancel.cancel();
            resume_tx.send(()).unwrap();
            tokio::time::timeout(DEADLINE, parse).await.expect("PPT worker resumed")
        })
    });
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn direct_ppt_from_the_only_foreign_blocking_worker_completes() {
    use std::time::Duration;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let handle = runtime.handle().clone();
    let parse = runtime.spawn_blocking(move || {
        let config = ExtractionConfig {
            max_archive_depth: 0,
            ..Default::default()
        };
        handle.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                PptExtractor::new().extract_content(NATIVE_TABLES, LEGACY_POWERPOINT_MIME_TYPE, &config),
            )
            .await
        })
    });
    let document = runtime
        .block_on(parse)
        .unwrap()
        .expect("direct PPT extraction cannot wait on its own blocking worker")
        .unwrap();
    check_native_table_deck(document);
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn ppt_single_and_batch_keep_exact_native_table_deck_shape() {
    crate::core::runtime::global_runtime()
        .unwrap()
        .block_on(assert_ppt_route_parity());
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn assert_ppt_route_parity() {
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
    let runtime = crate::core::runtime::global_runtime().unwrap();
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_thread_ids(true)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    // Match the width used by tracing-subscriber's thread-ID formatter. ~keep
    let caller_thread = format!("{:0>2?}", std::thread::current().id());
    let event_threads = || {
        let output = capture.0.lock().unwrap();
        std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .filter(|line| line.contains("no live slide list resolved from the persist chain; using stream order"))
            .map(|line| line.contains(&caller_thread))
            .collect::<Vec<_>>()
    };

    tracing::callsite::rebuild_interest_cache();
    let (inline, offloaded) = tracing::subscriber::with_default(subscriber, || {
        let inline = futures::executor::block_on(extract()).unwrap();
        assert_eq!(event_threads(), [true], "inline parse logs on the caller thread");
        let offloaded = runtime.block_on(extract()).unwrap();
        (inline, offloaded)
    });
    tracing::callsite::rebuild_interest_cache();
    assert_eq!(
        event_threads(),
        [true, false],
        "offloaded parse logs on a different thread"
    );
    assert_eq!(
        serde_json::to_value(&inline).unwrap(),
        serde_json::to_value(&offloaded).unwrap()
    );
    check_native_table_deck(offloaded);
}
