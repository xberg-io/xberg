use super::*;

static EVENTS: std::sync::Mutex<Vec<(&'static str, usize)>> = std::sync::Mutex::new(Vec::new());

static PAGE_STARTED: tokio::sync::Notify = tokio::sync::Notify::const_new();

const PARALLEL_CASE_WORKERS: usize = 2;

const PARALLEL_CASE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

fn parallel_case(config: &ExtractionConfig) -> bool {
    config
        .security_limits
        .as_ref()
        .is_some_and(|limits| limits.max_archive_size == IMPLICIT_PARALLEL_CASE)
}

fn record_event(event: &'static str, index: usize) {
    EVENTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push((event, index));
}

pub(in super::super) fn test_page_started(config: &ExtractionConfig, index: usize) {
    if parallel_case(config) {
        record_event("page_start", index);
        PAGE_STARTED.notify_one();
    }
}

pub(super) async fn hold_until_another_page_starts(config: &ExtractionConfig, index: usize) {
    if parallel_case(config) {
        record_event("trial_start", index);
        let released = tokio::time::timeout(PARALLEL_CASE_WAIT, PAGE_STARTED.notified())
            .await
            .is_ok();
        record_event(if released { "trial_released" } else { "trial_timeout" }, index);
    }
}

fn folio_with_an_ineligible_second_page() -> Vec<u8> {
    let mut document = lopdf::Document::load_mem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf/separation-overprint-2817/folio-black.pdf"
    )))
    .unwrap();
    let first = document.get_pages()[&1];
    let mut second = document.get_object(first).unwrap().as_dict().unwrap().clone();
    // An annotation makes a page ineligible for the trial while it keeps the same lost glyphs.
    second.set("Annots", lopdf::Object::Array(Vec::new()));
    let parent = second.get(b"Parent").unwrap().as_reference().unwrap();
    let second = document.add_object(second);
    let pages = document.get_object_mut(parent).unwrap().as_dict_mut().unwrap();
    pages
        .get_mut(b"Kids")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(lopdf::Object::Reference(second));
    pages.set("Count", 2);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

// The trial of page one waits until page two starts its normal read, so a batch that held its
// other pages behind the trials would end the wait by timeout. ~keep
#[test]
fn a_slow_trial_does_not_hold_the_pages_without_one_and_a_declined_trial_is_read_normally() {
    super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || async {
        let pdf = folio_with_an_ineligible_second_page();
        let models = tempfile::tempdir().unwrap();
        std::fs::write(
            models.path().join("eng.traineddata"),
            xberg_tesseract::bundled_eng_traineddata().unwrap(),
        )
        .unwrap();
        let config = ExtractionConfig {
            use_cache: false,
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_archive_size: IMPLICIT_PARALLEL_CASE,
                ..Default::default()
            }),
            pages: Some(crate::core::config::PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            concurrency: Some(crate::core::config::concurrency::ConcurrencyConfig {
                max_threads: Some(PARALLEL_CASE_WORKERS),
                ..Default::default()
            }),
            ocr: Some(OcrConfig {
                tessdata_path: Some(models.path().to_path_buf()),
                quality_thresholds: Some(crate::core::config::OcrQualityThresholds {
                    min_ocr_mean_confidence: 50.0,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear();
        // A permit left by an earlier run would release the trial without a second page starting.
        while futures::FutureExt::now_or_never(PAGE_STARTED.notified()).is_some() {}
        let result = crate::core::extractor::extract_bytes(&pdf, "application/pdf", &config)
            .await
            .unwrap();
        let events = std::mem::take(&mut *EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        assert!(
            events.contains(&("trial_start", 0)) && !events.contains(&("trial_start", 1)),
            "only page one is eligible for a trial: {events:?}"
        );
        assert!(
            events.contains(&("trial_released", 0)) && !events.contains(&("trial_timeout", 0)),
            "page two must start while the trial of page one is still running; if page two landed in a \
             later batch, the memory-adapted batch size dropped below {PARALLEL_CASE_WORKERS}: {events:?}"
        );
        assert!(
            events.iter().filter(|event| **event == ("page_start", 0)).count() == 1,
            "the declined trial must requeue page one for exactly one normal read: {events:?}"
        );
        let normal = normal_density_control(&pdf, &config).await;
        let read = |document: &ExtractedDocument| {
            document
                .pages
                .as_deref()
                .unwrap()
                .iter()
                .map(|page| page.content.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(read(&result).len(), 2);
        assert_eq!(read(&result), read(&normal));
    });
}
