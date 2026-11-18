use super::*;
use std::future::poll_fn;
use std::path::Path;
use std::task::Poll;
use std::time::Duration;

const DECK: &[u8] = include_bytes!("../../../tests/fixtures/ppt/embedded_word_object.ppt");

fn fixture_path() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ppt/embedded_word_object.ppt"
    ))
}

#[test]
#[serial_test::serial(office_admission)]
fn native_ppt_path_cancels_before_opening_while_the_shared_pool_is_full() {
    let runtime = crate::core::runtime::global_runtime().unwrap();
    runtime.block_on(async {
        let pool = crate::core::runtime::current_office_pool().unwrap();
        let permits = tokio::time::timeout(
            Duration::from_secs(5),
            pool.admission.acquire_many(pool.slots.try_into().unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("not-present.ppt");
        let cancel = crate::cancellation::CancellationToken::new();
        let config = ExtractionConfig {
            cancel_token: Some(cancel.clone()),
            ..Default::default()
        };
        let extractor = PptExtractor::new();
        let mut read = extractor.extract_path(&missing, LEGACY_POWERPOINT_MIME_TYPE, &config);
        assert!(poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await.is_pending());
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), read).await.unwrap();
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        assert_eq!(pool.admission.available_permits(), 0);
        drop(permits);
        let result = extractor
            .extract_path(&missing, LEGACY_POWERPOINT_MIME_TYPE, &ExtractionConfig::default())
            .await;
        assert!(matches!(result, Err(crate::XbergError::Io(_))));
    });
}

#[test]
#[serial_test::serial(office_admission)]
fn legacy_ppt_file_worker_releases_admission_before_extracting_the_real_word_child() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct ChildAdmission {
        pool: &'static crate::core::runtime::OfficeBlockingPool,
        seen: Arc<AtomicUsize>,
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for ChildAdmission {
        fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
            if event.metadata().target() == "xberg::ppt::embedded_dispatch" {
                assert_eq!(self.pool.admission.available_permits(), self.pool.slots);
                self.seen.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    let runtime = crate::core::runtime::global_runtime().unwrap();
    let pool = runtime.block_on(async { crate::core::runtime::current_office_pool().unwrap() });
    let seen = Arc::new(AtomicUsize::new(0));
    let subscriber = tracing_subscriber::registry().with(ChildAdmission {
        pool,
        seen: Arc::clone(&seen),
    });
    tracing::callsite::rebuild_interest_cache();
    let extractor = PptExtractor::new();
    let config = ExtractionConfig::default();
    let file = tracing::subscriber::with_default(subscriber, || {
        runtime
            .block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(5),
                    extractor.extract_path(fixture_path(), LEGACY_POWERPOINT_MIME_TYPE, &config),
                )
                .await
            })
            .unwrap()
            .unwrap()
    });
    tracing::callsite::rebuild_interest_cache();
    assert_eq!(seen.load(Ordering::Relaxed), 1);
    let bytes = runtime
        .block_on(extractor.extract_content(DECK, LEGACY_POWERPOINT_MIME_TYPE, &config))
        .unwrap();
    let [child] = file.children.as_deref().unwrap() else {
        panic!("one embedded Word child expected");
    };
    assert_eq!(child.path, "slide2/oleObject1.bin");
    assert!(child.result.content.contains("Iron pentacarbonyl"));
    assert_eq!(
        serde_json::to_value(&file.children).unwrap(),
        serde_json::to_value(&bytes.children).unwrap()
    );
}

#[test]
#[serial_test::serial(office_admission)]
fn native_ppt_file_worker_holds_a_slot_and_keeps_tracing_through_cancellation() {
    let runtime = crate::core::runtime::global_runtime().unwrap();
    let pool = runtime.block_on(async { crate::core::runtime::current_office_pool().unwrap() });
    let extractor = PptExtractor::new();
    let cancel = crate::cancellation::CancellationToken::new();
    let config = ExtractionConfig {
        cancel_token: Some(cancel.clone()),
        ..Default::default()
    };
    let (pending, entered, result) = crate::core::runtime::poll_with_blocking_worker_held(
        runtime,
        extractor.extract_path(fixture_path(), LEGACY_POWERPOINT_MIME_TYPE, &config),
        || {
            assert_eq!(pool.admission.available_permits(), pool.slots - 1);
            cancel.cancel();
        },
    );
    assert!(pending && entered);
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    assert_eq!(pool.admission.available_permits(), pool.slots);
}

#[test]
#[serial_test::serial(office_admission)]
fn native_ppt_single_bytes_and_batch_paths_bypass_file_admission() {
    let runtime = crate::core::runtime::global_runtime().unwrap();
    runtime.block_on(async {
        let pool = crate::core::runtime::current_office_pool().unwrap();
        let _permits = tokio::time::timeout(
            Duration::from_secs(5),
            pool.admission.acquire_many(pool.slots.try_into().unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        let extractor = PptExtractor::new();
        let config = ExtractionConfig {
            max_archive_depth: 0,
            ..Default::default()
        };
        let bytes = extractor.extract_content(DECK, LEGACY_POWERPOINT_MIME_TYPE, &config);
        let path = crate::core::batch_mode::with_batch_mode(extractor.extract_path(
            fixture_path(),
            LEGACY_POWERPOINT_MIME_TYPE,
            &config,
        ));
        for result in [
            tokio::time::timeout(Duration::from_secs(5), bytes).await,
            tokio::time::timeout(Duration::from_secs(5), path).await,
        ] {
            assert_eq!(result.unwrap().unwrap().mime_type, LEGACY_POWERPOINT_MIME_TYPE);
        }
    });
}

fn make_large_deck(path: &Path) {
    use std::io::Write as _;

    std::fs::write(path, DECK).unwrap();
    let mut compound = cfb::open_rw(path).unwrap();
    compound
        .create_stream("/QualificationPadding")
        .unwrap()
        .write_all(&vec![0; 1024 * 1024])
        .unwrap();
    compound.flush().unwrap();
    assert!(std::fs::metadata(path).unwrap().len() > 1024 * 1024);
}

#[test]
fn owned_large_ppt_snapshot_survives_truncation_and_enforces_its_cap() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("deck.ppt");
    make_large_deck(&path);
    let limit = std::fs::metadata(&path).unwrap().len() as usize;
    let cancel = PptReadCancellation::default();
    assert!(matches!(
        PptExtractor::read_file_snapshot(&path, limit - 1, cancel),
        Err(crate::XbergError::Validation { .. })
    ));
    let snapshot = PptExtractor::read_file_snapshot(&path, limit, cancel).unwrap();
    std::fs::File::create(&path).unwrap();
    let deck = crate::extraction::ppt::extract_ppt_text_with_cancellation(&snapshot, false, false, cancel).unwrap();
    assert_eq!(deck.slide_count, 3);
    let (objects, _) =
        crate::extraction::ppt::extract_ppt_embedded_objects_with_cancellation(&snapshot, limit, cancel).unwrap();
    assert_eq!(objects.len(), 1);
}

#[test]
#[serial_test::serial(office_admission)]
fn public_ppt_uri_timeout_leaves_an_owned_snapshot_and_abandons_active_records() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct RecordGate {
        reached: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        release: Mutex<mpsc::Receiver<()>>,
        discovered: Arc<AtomicBool>,
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for RecordGate {
        fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
            match event.metadata().target() {
                "xberg::ppt::record_walk" => {
                    if let Some(reached) = self.reached.lock().unwrap().take() {
                        reached.send(()).unwrap();
                        let _ = self.release.lock().unwrap().recv_timeout(Duration::from_secs(10));
                    }
                }
                "xberg::ppt::embedding_walk" => self.discovered.store(true, Ordering::Relaxed),
                _ => {}
            }
        }
    }

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("deck.ppt");
    make_large_deck(&path);
    let (reached, ready) = tokio::sync::oneshot::channel();
    let (release, gate) = mpsc::sync_channel(1);
    let discovered = Arc::new(AtomicBool::new(false));
    let subscriber = tracing_subscriber::registry().with(RecordGate {
        reached: Mutex::new(Some(reached)),
        release: Mutex::new(gate),
        discovered: Arc::clone(&discovered),
    });
    tracing::callsite::rebuild_interest_cache();
    let runtime = crate::core::runtime::global_runtime().unwrap();
    tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let pool = crate::core::runtime::current_office_pool().unwrap();
            let _remaining = pool
                .admission
                .acquire_many((pool.slots - 1).try_into().unwrap())
                .await
                .unwrap();
            let config = ExtractionConfig {
                use_cache: false,
                extraction_timeout_secs: Some(1),
                ..Default::default()
            };
            let input = crate::ExtractInput::from_uri(path.to_str().unwrap());
            let mut read = std::pin::pin!(crate::extract(input, &config));
            tokio::select! {
                ready = tokio::time::timeout(Duration::from_secs(5), ready) => ready.unwrap().unwrap(),
                result = &mut read => panic!("PPT parser never entered the active record walk: {result:?}"),
            }
            assert_eq!(pool.admission.available_permits(), 0);
            let result = tokio::time::timeout(Duration::from_secs(5), read).await.unwrap();
            assert!(matches!(result, Err(crate::XbergError::Timeout { limit_ms: 1000, .. })));
            std::fs::File::create(&path).unwrap();
            release.send(()).unwrap();
            let _worker_exited = tokio::time::timeout(Duration::from_secs(5), pool.admission.acquire())
                .await
                .unwrap()
                .unwrap();
            assert!(!discovered.load(Ordering::Relaxed));
        })
    });
    tracing::callsite::rebuild_interest_cache();
}
