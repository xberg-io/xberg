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
    exercise_source_page(
        document,
        index,
        config,
        run,
        raster,
        (decoded.width(), decoded.height()),
    )
}

fn exercise_source_page<'a>(
    document: &'a Arc<PdfDocument>,
    index: usize,
    config: &'a ExtractionConfig,
    run: &'a Arc<Run>,
    raster: Arc<Vec<u8>>,
    dimensions: (u32, u32),
) -> impl std::future::Future<Output = Vec<crate::types::ProcessingWarning>> + 'a {
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
                    dimensions,
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
    assert_eq!(
        run.budget.remaining(&config),
        proof_limits(&config).max_content_size / 2,
        "failed crop OCR must retain its full source share"
    );
    assert!(
        !run.budget.limited(),
        "one retained source share is not document exhaustion"
    );
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
    let document = Arc::new(PdfDocument::from_bytes(early_decline_pages("budget")).unwrap());
    let run = Arc::new(Run::default());
    assert!(run.budget.begin_page(0, &config));
    assert!(run.budget.charge(1, &config));
    assert_eq!(
        run.budget.remaining(&config),
        proof_limits(&config).max_content_size - 1
    );
    let (later, observed) = prepare_read(config.ocr.clone().unwrap(), &config, 1, 0, Some("ប០ប"), &run);
    assert!(
        !observed,
        "an eligible source page uses the single slot despite remaining bytes"
    );
    assert_eq!(later.backend_options, config.ocr.as_ref().unwrap().backend_options);
    let future = exercise_page(&document, 1, &config, &run);
    let events = FontEvents::default();
    let warnings = with_runtime(events.clone(), future);
    assert!(
        events.0.lock().unwrap().is_empty(),
        "the page cap must stop before fonts"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning.message.contains("document budget"))
    );
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
    for document in [
        matching_document(),
        Arc::new(PdfDocument::from_bytes(late_decline_pages()).unwrap()),
    ] {
        let (_root, mut config) = invalid_auxiliary_models();
        let token = crate::cancellation::CancellationToken::new();
        config.cancel_token = Some(token.clone());
        assert!(eligible(&config, 0), "admission starts before cancellation");
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
        assert_eq!(
            run.budget.remaining(&config),
            proof_limits(&config).max_content_size / 2,
            "cancellation after font work retains its entire bounded reservation"
        );
    }
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
    let allowance = run.budget.reserve_work(&config).unwrap();
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let subscriber = tracing_subscriber::registry().with(CancelOnShape(token, observed.clone()));
    let result = tracing::subscriber::with_default(subscriber, || {
        verify_source(
            raster,
            &proof,
            root.path().to_str().unwrap(),
            &config,
            &run,
            Some(allowance),
        )
    });
    assert!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        "the real glyph shape renderer must run before cancellation"
    );
    assert_eq!(
        run.budget.remaining(&config),
        proof_limits(&config).max_content_size / 2,
        "cancellation after glyph rendering must retain the whole source share"
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
    let original = pdf.get_dictionary(first).unwrap().clone();
    let parent = original.get(b"Parent").unwrap().as_reference().unwrap();
    let mut children = vec![lopdf::Object::Reference(first)];
    for _ in 1..3 {
        let mut page = original.clone();
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
        children.push(pdf.add_object(page).into());
    }
    let root = pdf.get_dictionary_mut(parent).unwrap();
    root.set("Kids", children);
    root.set("Count", 3);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_retain_source_reservation_when_a_late_decline_fails_the_proof() {
    let (_root, config) = invalid_auxiliary_models();
    let bytes = late_decline_pages();
    let document = Arc::new(PdfDocument::from_bytes(bytes.clone()).unwrap());
    assert_eq!(document.page_count().unwrap(), 3);
    let shadow = PdfDocument::from_bytes(bytes).unwrap();
    let (raster, _, _) = verifier_fixture();
    assert!(
        matches!(
            source_numbers(&shadow, 0, raster.capacity(), &config).unwrap(),
            Preparation::UnmeasuredDecline
        ),
        "the real ambiguous-physical-byte profile must decline"
    );
    let run = Arc::new(Run::default());
    let first = exercise_page(&document, 0, &config, &run);
    let second = exercise_page(&document, 1, &config, &run);
    let third = exercise_page(&document, 2, &config, &run);
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
            proof_limits(&config).max_content_size / 2,
            "a late source decline must retain its full bounded share"
        );
        warnings.extend(second.await);
        let after_second = events.0.lock().unwrap().len();
        assert!(
            after_second > count,
            "a second font proof may run only with another charged share"
        );
        assert_eq!(
            run.budget.remaining(&config),
            0,
            "two unmeasured shares exhaust the run"
        );
        warnings.extend(third.await);
        assert_eq!(
            events.0.lock().unwrap().len(),
            after_second,
            "a third page must not repeat uncharged font work"
        );
        assert!(
            run.processor.get().is_none(),
            "declined pages must not initialize the crop processor"
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

fn warm_native_number_fonts(document: &PdfDocument, pages: usize) {
    for index in 0..pages {
        assert!(
            document
                .extract_chars(index)
                .unwrap()
                .iter()
                .any(|glyph| glyph.char == '០')
        );
    }
}

fn early_decline_pages(profile: &str) -> Vec<u8> {
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let mut pdf = lopdf::Document::load_mem(source).unwrap();
    let first = *pdf.get_pages().values().next().unwrap();
    let page = pdf.get_dictionary(first).unwrap().clone();
    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    let second = pdf.add_object(page);
    match profile {
        "rotation" => pdf.get_dictionary_mut(first).unwrap().set("Rotate", 90),
        "crop" => pdf
            .get_dictionary_mut(first)
            .unwrap()
            .set("CropBox", vec![1.into(), 1.into(), 10.into(), 10.into()]),
        "actual_text" => pdf
            .get_dictionary_mut(first)
            .unwrap()
            .set("ActualText", lopdf::Object::string_literal("hidden")),
        "inline_actual_text" | "decode_error" | "oversized_contents" => {
            let id = pdf
                .get_dictionary(first)
                .unwrap()
                .get(b"Contents")
                .unwrap()
                .as_reference()
                .unwrap();
            let mut content = pdf.get_object(id).unwrap().as_stream().unwrap().clone();
            if profile == "decode_error" {
                content.set_plain_content(b"invalid zlib stream".to_vec());
                content.dict.set("Filter", "FlateDecode");
            } else {
                content.set_plain_content(b"/Span << /ActualText (hidden) >> BDC EMC".to_vec());
            }
            if profile == "oversized_contents" {
                content
                    .dict
                    .set("Padding", lopdf::Object::string_literal(vec![b'x'; 64 * 1024]));
            }
            let content = pdf.add_object(content);
            pdf.get_dictionary_mut(first).unwrap().set("Contents", content);
        }
        "form" | "oversized_form" => {
            let mut dictionary = lopdf::dictionary! {"Subtype" => "Form"};
            if profile == "oversized_form" {
                dictionary.set("Padding", lopdf::Object::string_literal(vec![b'x'; 64 * 1024]));
            }
            let form = pdf.add_object(lopdf::Stream::new(dictionary, Vec::new()));
            let resources = pdf.get_dictionary(first).unwrap().get(b"Resources").unwrap().clone();
            let mut resources = match resources {
                lopdf::Object::Reference(id) => pdf.get_dictionary(id).unwrap().clone(),
                lopdf::Object::Dictionary(dict) => dict,
                other => panic!("{other:?}"),
            };
            resources.set("XObject", lopdf::dictionary! {"Fm" => form});
            pdf.get_dictionary_mut(first).unwrap().set("Resources", resources);
        }
        "ambiguous_font" | "unsupported_audit" => {
            let resources = pdf.get_dictionary(first).unwrap().get(b"Resources").unwrap().clone();
            let mut resources = match resources {
                lopdf::Object::Reference(id) => pdf.get_dictionary(id).unwrap().clone(),
                lopdf::Object::Dictionary(dict) => dict,
                other => panic!("{other:?}"),
            };
            let fonts = resources.get(b"Font").unwrap();
            let mut fonts = match fonts {
                lopdf::Object::Reference(id) => pdf.get_dictionary(*id).unwrap().clone(),
                lopdf::Object::Dictionary(dict) => dict.clone(),
                other => panic!("{other:?}"),
            };
            let mut changed = 0;
            for (_, font) in fonts.iter_mut() {
                let mut dictionary = pdf.get_dictionary(font.as_reference().unwrap()).unwrap().clone();
                let Ok(cmap) = dictionary.get(b"ToUnicode") else {
                    continue;
                };
                if profile == "unsupported_audit" {
                    dictionary.set("Encoding", "Identity-H");
                    *font = pdf.add_object(dictionary).into();
                    changed += 1;
                    continue;
                }
                let cmap = cmap.as_reference().unwrap();
                let mut cmap = pdf.get_object(cmap).unwrap().as_stream().unwrap().clone();
                let text = String::from_utf8(cmap.decompressed_content().unwrap()).unwrap();
                assert!(text.contains("<02> <17E0>"));
                cmap.set_plain_content(
                    text.replace("endcmap", "1 beginbfchar\n<03> <17E0>\nendbfchar\nendcmap")
                        .into_bytes(),
                );
                dictionary.set("ToUnicode", pdf.add_object(cmap));
                *font = pdf.add_object(dictionary).into();
                changed += 1;
            }
            assert_eq!(changed, 1);
            resources.set("Font", fonts);
            pdf.get_dictionary_mut(first).unwrap().set("Resources", resources);
        }
        "budget" => {}
        "font" => {
            let font = pdf.add_object(lopdf::dictionary! {"Type" => "Font", "Subtype" => "Type0"});
            pdf.get_dictionary_mut(first).unwrap().set(
                "Resources",
                lopdf::dictionary! {
                    "Font" => lopdf::dictionary! {"F1" => font}
                },
            );
        }
        _ => panic!("unknown profile"),
    }
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
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn should_recover_the_second_page_after_an_early_unsupported_source_decline() {
    let (root, config) = invalid_auxiliary_models();
    std::fs::copy(
        std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned model"),
        root.path().join("khm.traineddata"),
    )
    .unwrap();
    let (raster, proof, character) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let metadata = crate::types::Metadata::default();
    for profile in [
        "rotation",
        "crop",
        "actual_text",
        "font",
        "inline_actual_text",
        "form",
        "ambiguous_font",
    ] {
        let bytes = early_decline_pages(profile);
        let document = Arc::new(PdfDocument::from_bytes(bytes.clone()).unwrap());
        assert_eq!(document.page_count().unwrap(), 2);
        let measured_font_charge = if profile == "ambiguous_font" {
            warm_native_number_fonts(&document, 1);
            let shadow = PdfDocument::from_bytes(bytes).unwrap();
            warm_native_number_fonts(&shadow, 1);
            let Preparation::MeasuredDecline(measured) =
                source_numbers(&shadow, 0, raster.capacity(), &config).unwrap()
            else {
                panic!("the native font pass must leave a measurable ambiguous-font decline");
            };
            Some(measured + raster.capacity())
        } else {
            None
        };
        let run = Arc::new(Run::default());
        let initial = run.budget.remaining(&config);
        let first = exercise_source_page(
            &document,
            0,
            &config,
            &run,
            raster.clone(),
            (decoded.width(), decoded.height()),
        );
        let events = FontEvents::default();
        with_runtime(events.clone(), async {
            let first = first.await;
            let early = ["rotation", "crop", "actual_text", "font"].contains(&profile);
            if early {
                assert!(
                    events.0.lock().unwrap().is_empty(),
                    "early {profile} must precede fonts"
                );
            }
            let remaining = run.budget.remaining(&config);
            if early {
                assert_eq!(remaining, initial, "{profile} is metadata-only");
            } else if let Some(charge) = measured_font_charge {
                assert_eq!(
                    remaining,
                    initial - charge,
                    "only the measured font proof may be charged"
                );
            } else {
                assert!(
                    remaining > initial / 2 && remaining < initial,
                    "{profile} must settle completed work"
                );
            }
            assert!(
                !run.budget.limited(),
                "early {profile} is unsupported, not budget exhaustion"
            );
            assert_eq!(first.len(), 1);
            assert!(first[0].message.contains("could not be bound"));
            let mut located = paragraph("VOU", "VOU");
            let segment = &mut located.lines[0].segments[0];
            let sx = decoded.width() as f32 / proof.media.width;
            let sy = decoded.height() as f32 / proof.media.height;
            segment.x = character.bbox.x * sx - 10.0;
            segment.y = character.bbox.y * sy;
            segment.width = character.bbox.width * sx + 20.0;
            segment.height = character.bbox.height * sy;
            let mut paragraphs = [located];
            let mut read = "\n\tVOU \r\n".to_owned();
            let mut warnings = Vec::new();
            assert!(
                recover(
                    PageSource {
                        document: document.clone(),
                        index: 1,
                        layer: "ប០ប",
                        raster: raster.clone(),
                        dimensions: (decoded.width(), decoded.height()),
                        metadata: &metadata
                    },
                    &mut paragraphs,
                    &mut read,
                    &config,
                    &run,
                    &mut warnings
                )
                .await,
                "supported page after {profile}"
            );
            assert_eq!(
                read, "\n\tVOU ០ \r\n",
                "painted zero must preserve accepted spacing after {profile}"
            );
            assert!(!events.0.lock().unwrap().is_empty(), "supported page must load fonts");
            assert!(warnings.is_empty(), "{profile}: {warnings:?}");
        });
    }
}

#[test]
fn should_not_spend_source_page_slots_on_unaccepted_primary_reads() {
    let (_root, config) = invalid_auxiliary_models();
    let run = Run::default();
    for page in 0..16 {
        let (_, observed) = prepare_read(config.ocr.clone().unwrap(), &config, page, 0, Some("ប០ប"), &run);
        assert!(
            observed,
            "unaccepted read on page {page} must not spend a source proof slot"
        );
    }
    assert!(
        run.budget.begin_page(16, &config),
        "accepted source work still has page allowance"
    );
}

#[test]
fn should_retain_source_reservation_after_content_decode_or_budget_decline() {
    for profile in ["inline_actual_text", "form", "budget"] {
        let (_root, mut config) = invalid_auxiliary_models();
        if profile == "budget" {
            config.security_limits = Some(crate::extractors::security::SecurityLimits {
                max_content_size: 1024 * 1024,
                ..Default::default()
            });
        }
        let document = Arc::new(PdfDocument::from_bytes(early_decline_pages(profile)).unwrap());
        let run = Arc::new(Run::default());
        let warnings = with_runtime(FontEvents::default(), exercise_page(&document, 0, &config, &run));
        let initial = proof_limits(&config).max_content_size;
        let remaining = run.budget.remaining(&config);
        if profile == "budget" {
            assert_eq!(remaining, initial / 2, "an unmeasured budget refusal retains its share");
        } else {
            assert!(
                remaining > initial / 2 && remaining < initial,
                "{profile} must charge measured nonzero work"
            );
        }
        assert!(
            !run.budget.limited(),
            "{profile}: a single share is not document exhaustion"
        );
        assert!(run.processor.get().is_none(), "{profile} must stop before crop OCR");
        assert_eq!(warnings.len(), 1, "{profile}: {warnings:?}");
        assert!(warnings[0].message.contains("could not be bound"), "{profile}");
    }
}

#[test]
fn should_release_metadata_declines_without_consuming_source_work() {
    let (_root, config) = invalid_auxiliary_models();
    let expected = proof_limits(&config).max_content_size;
    let mut observed = Vec::new();
    for profile in ["rotation", "crop", "actual_text", "font"] {
        let document = Arc::new(PdfDocument::from_bytes(early_decline_pages(profile)).unwrap());
        let run = Arc::new(Run::default());
        let future = exercise_page(&document, 0, &config, &run);
        let events = FontEvents::default();
        let warnings = with_runtime(events.clone(), future);
        observed.push((profile, run.budget.remaining(&config), run.budget.limited()));
        assert!(
            events.0.lock().unwrap().is_empty(),
            "{profile}: must precede native fonts"
        );
        assert!(run.processor.get().is_none(), "{profile}: must precede crop OCR");
        assert_eq!(warnings.len(), 1, "{profile}");
    }
    assert_eq!(
        observed,
        vec![
            ("rotation", expected, false),
            ("crop", expected, false),
            ("actual_text", expected, false),
            ("font", expected, false)
        ],
        "every metadata-only decline must release its reservation"
    );
}

#[test]
fn should_splice_numbers_into_the_second_accepted_paragraph() {
    let mut first = paragraph("first 1", "first 1");
    first.lines[0].segments[0].y = 200.0;
    let mut second = paragraph("left\t right", "left");
    let mut right = segment("right");
    right.x = 80.0;
    second.lines[0].segments.push(right);
    let mut paragraphs = [first, second];
    let mut read = "\nfirst  1\r\n\r\nleft\t right\n".to_owned();
    assert!(insert_into_read(
        vec![number("០", 30.0)],
        &mut paragraphs,
        &mut read,
        (1.0, 1.0),
        100
    ));
    assert_eq!(paragraphs[0].text, "first 1");
    assert_eq!(paragraphs[1].text, "left\t ០ right");
    assert_eq!(read, "\nfirst  1\r\n\r\nleft\t ០ right\n");
}

fn two_late_then_supported_pages(profile: &str) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(&early_decline_pages(profile)).unwrap();
    let pages = pdf.get_pages();
    let first = *pages.get(&1).unwrap();
    let last = *pages.get(&2).unwrap();
    let page = pdf.get_dictionary(first).unwrap().clone();
    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    let second = pdf.add_object(page);
    let root = pdf.get_dictionary_mut(parent).unwrap();
    root.set("Kids", vec![first.into(), second.into(), last.into()]);
    root.set("Count", 3);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_allow_supported_source_work_after_bounded_late_declines() {
    let (_root, config) = invalid_auxiliary_models();
    let (raster, _, _) = verifier_fixture();
    let initial = proof_limits(&config).max_content_size;
    let decoded = image::load_from_memory(&raster).unwrap();
    let dimensions = (decoded.width(), decoded.height());
    let mut observed = Vec::new();
    for profile in ["inline_actual_text", "form", "ambiguous_font"] {
        let bytes = two_late_then_supported_pages(profile);
        let shadow = PdfDocument::from_bytes(bytes.clone()).unwrap();
        if profile == "ambiguous_font" {
            warm_native_number_fonts(&shadow, 2);
        }
        for index in 0..2 {
            assert!(
                matches!(
                    source_numbers(&shadow, index, raster.capacity(), &config).unwrap(),
                    Preparation::MeasuredDecline(_)
                ),
                "{profile} page {index} must decline after admission"
            );
        }
        assert!(
            matches!(
                source_numbers(&shadow, 2, raster.capacity(), &config).unwrap(),
                Preparation::Ready(_)
            ),
            "{profile} third page must be a supported source"
        );
        let document = Arc::new(PdfDocument::from_bytes(bytes).unwrap());
        if profile == "ambiguous_font" {
            warm_native_number_fonts(&document, 2);
        }
        let run = Arc::new(Run::default());
        let events = FontEvents::default();
        with_runtime(events.clone(), async {
            let mut before = initial;
            let mut charged = true;
            let mut unproved = true;
            for index in 0..2 {
                let warnings = exercise_source_page(&document, index, &config, &run, raster.clone(), dimensions).await;
                let remaining = run.budget.remaining(&config);
                charged &= remaining < before;
                before = remaining;
                unproved &= warnings.len() == 1 && warnings[0].message.contains("could not be bound");
            }
            let after_declines = events.0.lock().unwrap().len();
            let _ = exercise_source_page(&document, 2, &config, &run, raster.clone(), dimensions).await;
            observed.push((
                profile,
                charged,
                before > 0,
                unproved,
                events.0.lock().unwrap().len() > after_declines,
            ));
        });
    }
    assert_eq!(
        observed,
        vec![
            ("inline_actual_text", true, true, true, true),
            ("form", true, true, true, true),
            ("ambiguous_font", true, true, true, true),
        ],
        "two completed late declines must charge real work and let a third supported proof run"
    );
}

fn eight_unsupported_then_supported_pages(profile: &str) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(&early_decline_pages(profile)).unwrap();
    let pages = pdf.get_pages();
    let first = *pages.get(&1).unwrap();
    let last = *pages.get(&2).unwrap();
    let page = pdf.get_dictionary(first).unwrap().clone();
    let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    let mut children = vec![lopdf::Object::Reference(first)];
    for _ in 1..8 {
        children.push(pdf.add_object(page.clone()).into());
    }
    children.push(last.into());
    let root = pdf.get_dictionary_mut(parent).unwrap();
    root.set("Kids", children);
    root.set("Count", 9);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_exclude_unsupported_sources_from_eligible_page_limit() {
    let (_root, config) = invalid_auxiliary_models();
    let mut observed = Vec::new();
    let (raster, _, _) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let dimensions = (decoded.width(), decoded.height());
    for profile in ["rotation", "crop", "actual_text", "font"] {
        let document = Arc::new(PdfDocument::from_bytes(eight_unsupported_then_supported_pages(profile)).unwrap());
        assert_eq!(document.page_count().unwrap(), 9);
        let run = Arc::new(Run::default());
        let events = FontEvents::default();
        let early: Vec<_> = (0..8)
            .map(|page| exercise_source_page(&document, page, &config, &run, raster.clone(), dimensions))
            .collect();
        let supported = exercise_source_page(&document, 8, &config, &run, raster.clone(), dimensions);
        with_runtime(events.clone(), async {
            for (page, future) in early.into_iter().enumerate() {
                let warnings = future.await;
                assert_eq!(warnings.len(), 1, "{profile} page {page}");
                assert!(warnings[0].message.contains("could not be bound"));
            }
            assert!(
                events.0.lock().unwrap().is_empty(),
                "{profile}: early declines do no font work"
            );
            let available = run.budget.page_available(8, &config);
            let warnings = supported.await;
            observed.push((
                profile,
                available,
                !events.0.lock().unwrap().is_empty(),
                run.budget.limited(),
                warnings
                    .iter()
                    .any(|warning| warning.message.contains("document budget")),
            ));
        });
    }
    assert_eq!(
        observed,
        vec![
            ("rotation", true, true, false, false),
            ("crop", true, true, false, false),
            ("actual_text", true, true, false, false),
            ("font", true, true, false, false),
        ],
        "eight metadata-only declines must not displace a supported page's proof slot"
    );
}

#[test]
fn should_keep_unmeasured_shares_within_the_frozen_document_limit() {
    let mut config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_content_size: 101,
            ..Default::default()
        }),
        ..Default::default()
    };
    let budget = budget::Budget::default();
    assert_eq!(budget.reserve_work(&config), Some(51));
    budget.retain_failed_work();
    assert_eq!(budget.remaining(&config), 50);
    assert!(!budget.limited(), "one retained share is not document exhaustion");
    config.security_limits.as_mut().unwrap().max_content_size = 1024;
    assert_eq!(
        budget.reserve_work(&config),
        Some(50),
        "later config cannot enlarge the frozen ceiling"
    );
    budget.retain_failed_work();
    assert_eq!(budget.remaining(&config), 0);
    assert!(budget.limited(), "two full shares really exhaust the document bytes");
    assert_eq!(budget.reserve_work(&config), None);
}

#[test]
fn should_enforce_source_reservation_as_an_allocation_ceiling() {
    let (root, config) = invalid_auxiliary_models();
    let (raster, proof, _) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let measured = raster.capacity() + proof.shape.retained_bytes() + decoded.to_rgb8().as_raw().len();
    let total = measured + measured / 2;
    let reserve_config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_content_size: total,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
            &raster,
            reserve_config.security_limits.as_ref().unwrap(),
            (raster.capacity() + proof.shape.retained_bytes()) as u64,
        )
        .is_ok(),
        "the fixture must fit the complete document allowance"
    );
    let run = Run::default();
    let reserved = run.budget.reserve_work(&reserve_config).unwrap();
    let result = verify_source(
        raster,
        &proof,
        root.path().to_str().unwrap(),
        &config,
        &run,
        Some(reserved),
    );
    assert!(result.is_err(), "the source raster cannot fit inside its reservation");
    assert!(
        run.processor.get().is_none(),
        "refusal must precede native OCR initialization"
    );
    assert_eq!(
        run.budget.remaining(&config),
        total - reserved,
        "refusal retains its whole reservation"
    );
}

#[test]
fn should_fail_closed_when_measured_source_work_exceeds_its_reservation() {
    let config = ExtractionConfig::default();
    let budget = budget::Budget::default();
    let reserved = budget.reserve_work(&config).unwrap();
    assert!(!budget.settle_work(reserved, reserved + 1));
    assert_eq!(
        budget.remaining(&config),
        0,
        "an unexpected accounting overrun must stop the run"
    );
    assert!(budget.limited());
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn should_recover_the_third_page_after_two_late_source_declines() {
    let (root, config) = invalid_auxiliary_models();
    std::fs::copy(
        std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned model"),
        root.path().join("khm.traineddata"),
    )
    .unwrap();
    let (raster, proof, character) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let metadata = crate::types::Metadata::default();
    for profile in ["inline_actual_text", "form", "ambiguous_font"] {
        let document = Arc::new(PdfDocument::from_bytes(two_late_then_supported_pages(profile)).unwrap());
        assert_eq!(document.page_count().unwrap(), 3);
        if profile == "ambiguous_font" {
            warm_native_number_fonts(&document, 2);
        }
        let run = Arc::new(Run::default());
        let initial = run.budget.remaining(&config);
        let events = FontEvents::default();
        with_runtime(events.clone(), async {
            for index in 0..2 {
                let first = exercise_source_page(
                    &document,
                    index,
                    &config,
                    &run,
                    raster.clone(),
                    (decoded.width(), decoded.height()),
                )
                .await;
                assert_eq!(first.len(), 1);
                assert!(first[0].message.contains("could not be bound"), "{profile}: {first:?}");
            }
            assert!(run.budget.remaining(&config) < initial, "late work must stay charged");
            assert!(
                run.budget.remaining(&config) > 0,
                "third page needs remaining allowance: {profile}"
            );
            assert!(!run.budget.limited());
            let mut located = paragraph("VOU", "VOU");
            let segment = &mut located.lines[0].segments[0];
            let sx = decoded.width() as f32 / proof.media.width;
            let sy = decoded.height() as f32 / proof.media.height;
            segment.x = character.bbox.x * sx - 10.0;
            segment.y = character.bbox.y * sy;
            segment.width = character.bbox.width * sx + 20.0;
            segment.height = character.bbox.height * sy;
            let mut paragraphs = [located];
            let mut read = "\n\tVOU \r\n".to_owned();
            let mut warnings = Vec::new();
            assert!(
                recover(
                    PageSource {
                        document: document.clone(),
                        index: 2,
                        layer: "ប០ប",
                        raster: raster.clone(),
                        dimensions: (decoded.width(), decoded.height()),
                        metadata: &metadata
                    },
                    &mut paragraphs,
                    &mut read,
                    &config,
                    &run,
                    &mut warnings
                )
                .await,
                "supported third page after two {profile} declines"
            );
            assert_eq!(
                read, "\n\tVOU ០ \r\n",
                "painted zero must preserve accepted spacing after {profile}"
            );
            assert!(!events.0.lock().unwrap().is_empty(), "supported page must load fonts");
            assert!(warnings.is_empty(), "{profile}: {warnings:?}");
        });
    }
}

#[test]
fn should_retain_reservation_for_unmeasured_source_failures() {
    let (_root, config) = invalid_auxiliary_models();
    let (raster, _, _) = verifier_fixture();
    let decoded = image::load_from_memory(&raster).unwrap();
    let dimensions = (decoded.width(), decoded.height());
    for profile in [
        "decode_error",
        "unsupported_audit",
        "oversized_form",
        "oversized_contents",
    ] {
        let bytes = early_decline_pages(profile);
        let shadow = PdfDocument::from_bytes(bytes.clone()).unwrap();
        let document = Arc::new(PdfDocument::from_bytes(bytes).unwrap());
        let outcome = source_numbers(&shadow, 0, raster.capacity(), &config);
        if profile == "decode_error" {
            assert!(outcome.is_err(), "the real Flate decoder must error");
        } else {
            assert!(
                matches!(outcome.unwrap(), Preparation::UnmeasuredDecline),
                "{profile} must keep opaque source work unmeasured"
            );
        }
        let run = Arc::new(Run::default());
        let initial = run.budget.remaining(&config);
        with_runtime(FontEvents::default(), async {
            let warnings = exercise_source_page(&document, 0, &config, &run, raster.clone(), dimensions).await;
            assert_eq!(
                run.budget.remaining(&config),
                initial / 2,
                "{profile} must retain the full share"
            );
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].message.contains("could not be bound"));
            assert!(run.processor.get().is_none());
        });
    }
}

#[test]
fn should_not_settle_a_cancelled_measured_decline() {
    struct Measured(bool);
    impl tracing::field::Visit for Measured {
        fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
            if field.name() == "measured" {
                self.0 = value;
            }
        }
        fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
    }
    struct CancelMeasured(crate::cancellation::CancellationToken);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CancelMeasured {
        fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            if event.metadata().target().ends_with("number_recovery::shape_proof") {
                let mut measured = Measured(false);
                event.record(&mut measured);
                if measured.0 {
                    self.0.cancel();
                }
            }
        }
    }
    let (_root, mut config) = invalid_auxiliary_models();
    let token = crate::cancellation::CancellationToken::new();
    config.cancel_token = Some(token.clone());
    let document = Arc::new(PdfDocument::from_bytes(late_decline_pages()).unwrap());
    warm_native_number_fonts(&document, 1);
    let run = Arc::new(Run::default());
    let subscriber = tracing_subscriber::registry().with(CancelMeasured(token.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(exercise_page(&document, 0, &config, &run));
    });
    assert!(
        token.is_cancelled(),
        "the real prepared decline must be measured before cancellation"
    );
    assert_eq!(
        run.budget.remaining(&config),
        proof_limits(&config).max_content_size / 2,
        "cancellation at measured disposition must retain the whole share"
    );
    assert!(run.processor.get().is_none());
}
