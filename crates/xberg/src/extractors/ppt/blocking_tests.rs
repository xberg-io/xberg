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
