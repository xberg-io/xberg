use super::*;
use std::sync::{Arc, Mutex};
use tracing_subscriber::prelude::*;

fn invalid_auxiliary_models() -> (tempfile::TempDir, ExtractionConfig) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("eng.traineddata"), b"unused primary model").unwrap();
    std::fs::write(root.path().join("khm.traineddata"), b"invalid auxiliary model").unwrap();
    let config = ExtractionConfig {
        ocr: Some(crate::OcrConfig {
            tessdata_path: Some(root.path().into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(model_directory(&config).as_deref(), root.path().to_str());
    (root, config)
}

#[tokio::test]
async fn should_keep_offset_seven_primary_cache_policy_and_operation_cacheability() {
    let (_root, config) = invalid_auxiliary_models();
    for (offset, expected) in [(0, true), (7, false)] {
        crate::core::extractor::cacheability::scope(async {
            let before = config.ocr.clone().unwrap();
            let (actual, observed) = prepare_read(before.clone(), &config, 7, offset, Some("ប០ប"), &Run::default());
            assert_eq!(observed, expected, "offset {offset}");
            assert_eq!(
                crate::core::extractor::cacheability::is_cacheable(),
                !expected,
                "offset {offset}"
            );
            if !expected {
                assert_eq!(actual.backend_options, before.backend_options);
            } else {
                assert_eq!(actual.backend_options.unwrap()["use_cache"], false);
            }
        })
        .await;
    }
}

#[derive(Clone, Default)]
struct FontEvents(Arc<Mutex<Vec<std::thread::ThreadId>>>, Option<Arc<FontGate>>);
struct FontGate {
    entered: tokio::sync::Notify,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    seen: std::sync::atomic::AtomicBool,
    timely: std::sync::atomic::AtomicBool,
}
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for FontEvents {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() == "xberg_native_pdf::fonts::font_dict" {
            self.0.lock().unwrap().push(std::thread::current().id());
            if let Some(gate) = &self.1
                && !gate.seen.swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                gate.entered.notify_one();
                let timely = gate
                    .release
                    .lock()
                    .unwrap()
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .is_ok();
                gate.timely.store(timely, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

fn with_runtime<T>(events: FontEvents, future: impl std::future::Future<Output = T>) -> T {
    let subscriber = tracing_subscriber::registry().with(events);
    tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    })
}

fn exercise_page<'a>(
    document: &'a Arc<PdfDocument>,
    index: usize,
    config: &'a ExtractionConfig,
    run: &'a Arc<Run>,
) -> impl std::future::Future<Output = Vec<crate::types::ProcessingWarning>> + 'a {
    let input = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let (raster, _, _) = verifier_fixture_from(input);
    let decoded = image::load_from_memory(&raster).unwrap();
    let metadata = crate::types::Metadata::default();
    async move {
        let mut paragraphs = [paragraph("VOU", "VOU")];
        let mut read = "VOU".to_owned();
        let mut warnings = Vec::new();
        assert!(
            !recover(
                PageSource {
                    document: Arc::clone(document),
                    index,
                    layer: "ប០ប",
                    raster,
                    dimensions: (decoded.width(), decoded.height()),
                    metadata: &metadata
                },
                &mut paragraphs,
                &mut read,
                config,
                run,
                &mut warnings
            )
            .await
        );
        assert_eq!(
            read, "VOU",
            "an unverified auxiliary model must retain the accepted read"
        );
        warnings
    }
}

fn matching_document() -> Arc<PdfDocument> {
    Arc::new(
        PdfDocument::from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
            ))
            .to_vec(),
        )
        .unwrap(),
    )
}

#[test]
fn should_cancel_before_source_admission_and_processor_initialization() {
    let (_root, mut config) = invalid_auxiliary_models();
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    config.cancel_token = Some(token);
    let document = matching_document();
    let run = Arc::new(Run::default());
    let events = FontEvents::default();
    with_runtime(events.clone(), exercise_page(&document, 0, &config, &run));
    assert!(
        run.processor.get().is_none(),
        "cancelled recovery must not initialize a processor"
    );
    assert!(
        events.0.lock().unwrap().is_empty(),
        "cancellation must precede native source/font admission"
    );
}

#[test]
fn should_prepare_numeric_source_on_a_blocking_worker() {
    let (_root, config) = invalid_auxiliary_models();
    let document = matching_document();
    let run = Arc::new(Run::default());
    let events = FontEvents::default();
    let async_thread = std::thread::current().id();
    // Build the raster before observing the actual recovery entry point.
    let (raster, _, _) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let metadata = crate::types::Metadata::default();
    with_runtime(events.clone(), async {
        let mut paragraphs = [paragraph("VOU", "VOU")];
        let mut read = "VOU".to_owned();
        let mut warnings = Vec::new();
        recover(
            PageSource {
                document: Arc::clone(&document),
                index: 0,
                layer: "ប០ប",
                raster,
                dimensions: (decoded.width(), decoded.height()),
                metadata: &metadata,
            },
            &mut paragraphs,
            &mut read,
            &config,
            &run,
            &mut warnings,
        )
        .await;
    });
    let captured = events.0.lock().unwrap();
    assert!(!captured.is_empty(), "the real native font preparation must run");
    assert!(
        captured.iter().all(|thread| *thread != async_thread),
        "native source work must not block the async worker: {captured:?}"
    );
}

#[test]
fn should_bound_crop_attempts_across_one_run() {
    let (root, mut config) = invalid_auxiliary_models();
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_iterations: 2,
        ..Default::default()
    });
    let (raster, proof, _) = verifier_fixture();
    let run = Run::default();
    for _ in 0..2 {
        assert!(
            verify_source(
                raster.clone(),
                &proof,
                root.path().to_str().unwrap(),
                &config,
                &run,
                None
            )
            .is_err(),
            "an admitted attempt must reach the deliberately invalid model"
        );
    }
    let exhausted = verify_source(raster, &proof, root.path().to_str().unwrap(), &config, &run, None);
    assert!(
        exhausted.is_ok_and(|numbers| numbers.is_empty()),
        "the document crop cap must stop before another engine call"
    );
}

#[test]
fn should_allow_async_heartbeat_while_native_source_preparation_waits() {
    let (_root, config) = invalid_auxiliary_models();
    let document = matching_document();
    let run = Arc::new(Run::default());
    let recovery = exercise_page(&document, 0, &config, &run);
    let (release, receiver) = std::sync::mpsc::channel();
    let gate = Arc::new(FontGate {
        entered: tokio::sync::Notify::new(),
        release: Mutex::new(receiver),
        seen: false.into(),
        timely: false.into(),
    });
    let events = FontEvents(Arc::default(), Some(gate.clone()));
    with_runtime(events, async {
        tokio::join!(recovery, async {
            tokio::time::timeout(std::time::Duration::from_secs(5), gate.entered.notified())
                .await
                .expect("actual native preparation must execute");
            release.send(()).unwrap();
        });
    });
    assert!(
        gate.timely.load(std::sync::atomic::Ordering::SeqCst),
        "the async heartbeat must release native work before its deadline"
    );
}

#[test]
fn should_bound_page_admission_across_one_document_and_keep_reads() {
    let (_root, mut config) = invalid_auxiliary_models();
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_iterations: 1,
        ..Default::default()
    });
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let mut pdf = lopdf::Document::load_mem(source).unwrap();
    let first = *pdf.get_pages().values().next().unwrap();
    let mut page = pdf.get_dictionary(first).unwrap().clone();
    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    page.set("Parent", parent);
    let second = pdf.add_object(page);
    let root = pdf.get_dictionary_mut(parent).unwrap();
    root.set(
        "Kids",
        vec![lopdf::Object::Reference(first), lopdf::Object::Reference(second)],
    );
    root.set("Count", 2);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let document = Arc::new(PdfDocument::from_bytes(bytes).unwrap());
    assert_eq!(document.page_count().unwrap(), 2);
    let run = Arc::new(Run::default());
    let (_, admitted) = prepare_read(config.ocr.clone().unwrap(), &config, 0, 0, Some("ប០ប"), &run);
    let (later, observed) = prepare_read(config.ocr.clone().unwrap(), &config, 1, 0, Some("ប០ប"), &run);
    assert!(admitted);
    assert!(!observed, "page admission must stop before source or crop work");
    assert_eq!(later.backend_options, config.ocr.as_ref().unwrap().backend_options);
    let events = FontEvents::default();
    let first = exercise_page(&document, 0, &config, &run);
    let second = exercise_page(&document, 1, &config, &run);
    with_runtime(events.clone(), async {
        let mut warnings = first.await;
        let initial = events.0.lock().unwrap().len();
        assert!(initial > 0, "the first admitted page really performs source work");
        warnings.extend(second.await);
        assert_eq!(
            events.0.lock().unwrap().len(),
            initial,
            "a second page must not repeat native work after the run cap"
        );
        assert!(
            warnings.iter().any(|warning| warning.message.contains("budget")),
            "declining additional verification must be caller-visible"
        );
    });
}

#[test]
fn should_observe_cancellation_after_admission_before_engine_initialization() {
    struct CancelOnFont(
        crate::cancellation::CancellationToken,
        Arc<std::sync::atomic::AtomicBool>,
    );
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CancelOnFont {
        fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            if event.metadata().target() == "xberg_native_pdf::fonts::font_dict" {
                self.1.store(true, std::sync::atomic::Ordering::SeqCst);
                self.0.cancel();
            }
        }
    }
    let (_root, mut config) = invalid_auxiliary_models();
    let token = crate::cancellation::CancellationToken::new();
    config.cancel_token = Some(token.clone());
    assert!(eligible(&config, 0), "admission starts before cancellation");
    let document = matching_document();
    let run = Arc::new(Run::default());
    let future = exercise_page(&document, 0, &config, &run);
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let subscriber = tracing_subscriber::registry().with(CancelOnFont(token, observed.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    });
    assert!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        "the native source path must cancel an already admitted page"
    );
    assert!(
        run.processor.get().is_none(),
        "a cancellation during source proof must prevent engine initialization"
    );
}

#[test]
fn should_bound_accounted_source_work_bytes_across_one_run() {
    let (root, mut config) = invalid_auxiliary_models();
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: 64 * 1024 * 1024,
        ..Default::default()
    });
    let (raster, proof, _) = verifier_fixture();
    let mut run = Run::default();
    let mut stopped = false;
    for attempt in 0..8 {
        let result = verify_source(
            raster.clone(),
            &proof,
            root.path().to_str().unwrap(),
            &config,
            &run,
            None,
        );
        let initialized = run.processor.take().is_some();
        if attempt == 0 {
            assert!(
                initialized && result.is_err(),
                "the first admitted source/crop must reach the deliberately invalid auxiliary model"
            );
        }
        if !initialized {
            stopped = true;
            break;
        }
    }
    assert!(
        stopped,
        "cumulative verifier bytes must stop repeated work before the page/crop count limits"
    );
}

#[tokio::test]
async fn should_warn_without_a_model_even_when_accepted_geometry_is_unavailable() {
    let (root, config) = invalid_auxiliary_models();
    std::fs::remove_file(root.path().join("khm.traineddata")).unwrap();
    assert!(model_directory(&config).is_none());
    let document = matching_document();
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let integrity = rendered_integrity(source);
    let (raster, _, _) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let metadata = crate::types::Metadata::default();
    let run = Arc::new(Run::default());
    for located in [false, true] {
        let mut paragraphs = [paragraph("VOU", "VOU")];
        let mut text = if located { "different accepted read" } else { "VOU" }.to_owned();
        let before = text.clone();
        let mut warnings = Vec::new();
        let prepared = PreparedPage {
            policy: PagePolicy {
                page_index_offset: 0,
                accepted: false,
                scale_only: true,
                deferred_structure: false,
                layout: false,
                transformed: false,
                raster: Some(&integrity),
                structured: false,
                builtin: true,
            },
            source: Some(PageSource {
                document: document.clone(),
                index: 0,
                layer: "ប០ប",
                raster: raster.clone(),
                dimensions: (decoded.width(), decoded.height()),
                metadata: &metadata,
            }),
            config: &config,
            run: &run,
        };
        assert!(
            !prepared
                .apply(
                    AcceptedRead {
                        accepted: true,
                        paragraphs: located.then_some(&mut paragraphs),
                        text: &mut text,
                        warnings: &mut warnings
                    },
                    &[],
                    &[]
                )
                .await
        );
        assert_eq!(text, before);
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| warning.message.contains("no khm.traineddata"))
                .count(),
            1,
            "accepted page must expose the actual missing auxiliary model"
        );
    }
}

#[test]
fn should_observe_cancellation_during_shape_proof_before_native_engine_start() {
    struct CancelOnShape(
        crate::cancellation::CancellationToken,
        Arc<std::sync::atomic::AtomicBool>,
    );
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CancelOnShape {
        fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            if event.metadata().target() == "xberg_native_pdf::fonts::font_dict" {
                self.1.store(true, std::sync::atomic::Ordering::SeqCst);
                self.0.cancel();
            }
        }
    }
    let (root, mut config) = invalid_auxiliary_models();
    let token = crate::cancellation::CancellationToken::new();
    config.cancel_token = Some(token.clone());
    let (raster, proof, _) = verifier_fixture();
    let run = Run::default();
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let subscriber = tracing_subscriber::registry().with(CancelOnShape(token, observed.clone()));
    let result = tracing::subscriber::with_default(subscriber, || {
        verify_source(raster, &proof, root.path().to_str().unwrap(), &config, &run, None)
    });
    assert!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        "the real glyph shape renderer must run before cancellation"
    );
    assert!(
        result.is_ok_and(|numbers| numbers.is_empty()),
        "cancellation during glyph proof must not reach the deliberately invalid native engine"
    );
}

fn late_decline_pages() -> Vec<u8> {
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let mut pdf = lopdf::Document::load_mem(source).unwrap();
    let mut changed = 0;
    for object in pdf.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else { continue };
        let Ok(content) = stream.decompressed_content() else {
            continue;
        };
        let Ok(content) = String::from_utf8(content) else {
            continue;
        };
        if content.contains("<02> <17E0>") {
            stream.set_plain_content(
                content
                    .replace("endcmap", "1 beginbfchar\n<03> <17E0>\nendbfchar\nendcmap")
                    .into_bytes(),
            );
            changed += 1;
        }
    }
    assert_eq!(changed, 1);
    let first = *pdf.get_pages().values().next().unwrap();
    let mut page = pdf.get_dictionary(first).unwrap().clone();
    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    let resources = page.get(b"Resources").unwrap();
    let mut resources = match resources {
        lopdf::Object::Reference(id) => pdf.get_dictionary(*id).unwrap().clone(),
        lopdf::Object::Dictionary(dict) => dict.clone(),
        other => panic!("{other:?}"),
    };
    let fonts = resources.get(b"Font").unwrap();
    let mut fonts = match fonts {
        lopdf::Object::Reference(id) => pdf.get_dictionary(*id).unwrap().clone(),
        lopdf::Object::Dictionary(dict) => dict.clone(),
        other => panic!("{other:?}"),
    };
    for (_, font) in fonts.iter_mut() {
        let old = font.as_reference().unwrap();
        let copy = pdf.get_object(old).unwrap().clone();
        *font = pdf.add_object(copy).into();
    }
    resources.set("Font", fonts);
    page.set("Resources", resources);
    let second = pdf.add_object(page);
    let root = pdf.get_dictionary_mut(parent).unwrap();
    root.set(
        "Kids",
        vec![lopdf::Object::Reference(first), lopdf::Object::Reference(second)],
    );
    root.set("Count", 2);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_retain_source_reservation_when_a_late_decline_fails_the_proof() {
    let (_root, config) = invalid_auxiliary_models();
    let bytes = late_decline_pages();
    let document = Arc::new(PdfDocument::from_bytes(bytes.clone()).unwrap());
    assert_eq!(document.page_count().unwrap(), 2);
    let shadow = PdfDocument::from_bytes(bytes).unwrap();
    let (raster, _, _) = verifier_fixture();
    assert!(
        source_numbers(&shadow, 0, raster.capacity(), &config)
            .unwrap()
            .is_none(),
        "the real ambiguous-physical-byte profile must decline"
    );
    let run = Arc::new(Run::default());
    let first = exercise_page(&document, 0, &config, &run);
    let second = exercise_page(&document, 1, &config, &run);
    let events = FontEvents::default();
    with_runtime(events.clone(), async {
        let mut warnings = first.await;
        let count = events.0.lock().unwrap().len();
        assert!(
            count > 0,
            "the declined first proof must really load native fonts after admission"
        );
        assert_eq!(
            run.budget.remaining(&config),
            0,
            "a late source decline cannot refund unmeasured preparation work"
        );
        warnings.extend(second.await);
        assert_eq!(
            events.0.lock().unwrap().len(),
            count,
            "the next page must not repeat uncharged font/source work"
        );
        assert!(
            run.processor.get().is_none(),
            "neither declined page may initialize the crop processor"
        );
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| warning.message.contains("document budget"))
                .count(),
            1,
            "the exhausted document budget is caller-visible exactly once"
        );
    });
}
