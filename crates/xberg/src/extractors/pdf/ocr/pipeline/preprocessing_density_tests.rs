use super::*;

fn configuration() -> ExtractionConfig {
    ExtractionConfig {
        force_ocr: true,
        ocr: Some(crate::core::config::OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                preprocessing: Some(Default::default()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn configured_density_requires_builtin_forced_ocr() {
    let config = configuration();
    let ocr = config.ocr.as_ref().unwrap();
    assert_eq!(configured_preprocessing_render_dpi(&config, ocr, true), Some(300));
    assert_eq!(configured_preprocessing_render_dpi(&config, ocr, false), None);
    let automatic = ExtractionConfig {
        force_ocr: false,
        ..config.clone()
    };
    assert_eq!(configured_preprocessing_render_dpi(&automatic, ocr, true), None);
    let explicit = ExtractionConfig {
        images: Some(Default::default()),
        ..config.clone()
    };
    assert_eq!(configured_preprocessing_render_dpi(&explicit, ocr, true), None);
}

#[test]
fn implicit_or_lower_normalization_does_not_promote_rendering() {
    let config = configuration();
    let mut ocr = config.ocr.as_ref().unwrap().clone();
    ocr.tesseract_config.as_mut().unwrap().preprocessing = None;
    assert_eq!(configured_preprocessing_render_dpi(&config, &ocr, true), None);
    ocr.tesseract_config.as_mut().unwrap().preprocessing = Some(crate::types::ImagePreprocessingConfig {
        target_dpi: 150,
        ..Default::default()
    });
    assert_eq!(configured_preprocessing_render_dpi(&config, &ocr, true), None);
}

#[test]
fn promoted_batch_reserves_the_page_cap_and_never_less_than_before() {
    let old = UNMEASURED_OCR_PAGE_WORKING_SET_BYTES;
    let cap = old * 4;
    assert_eq!(preprocessing_render_reservation(None, cap), old);
    assert_eq!(preprocessing_render_reservation(Some(300), cap), cap);
    assert_eq!(preprocessing_render_reservation(Some(300), 1), old);
    let available = 512 * 1024 * 1024 + cap * 2;
    assert_eq!(
        adapt_batch_size_to_memory_inner(8, 0, preprocessing_render_reservation(Some(300), cap), Some(available)),
        2
    );
}

#[test]
fn plain_mixed_route_promotes_the_configured_density() {
    assert!(mixed_route_promotes_preprocessing_dpi(false, false));
}

#[test]
fn mixed_route_with_layout_detection_keeps_the_source_render() {
    assert!(!mixed_route_promotes_preprocessing_dpi(true, false));
}

#[test]
fn mixed_route_probing_hidden_layer_keeps_the_source_render() {
    assert!(!mixed_route_promotes_preprocessing_dpi(false, true));
}

#[test]
fn promoted_mixed_route_batch_working_set_reserves_the_page_cap() {
    let base = UNMEASURED_OCR_PAGE_WORKING_SET_BYTES;
    assert_eq!(mixed_route_batch_working_set(base, None, base * 4), base);
    assert_eq!(mixed_route_batch_working_set(base, Some(300), base * 4), base * 4);
}

#[test]
fn preprocessing_render_reservation_bounds_the_promoted_working_set() {
    let cap = 8 * 1024 * 1024;
    assert_eq!(
        preprocessing_render_reservation(None, cap),
        UNMEASURED_OCR_PAGE_WORKING_SET_BYTES
    );
    assert_eq!(
        preprocessing_render_reservation(Some(300), cap),
        UNMEASURED_OCR_PAGE_WORKING_SET_BYTES.max(cap)
    );
    assert_eq!(
        mixed_route_batch_working_set(0, Some(300), cap),
        preprocessing_render_reservation(Some(300), cap)
    );
    assert_eq!(mixed_route_batch_working_set(0, None, cap), 0);
}

#[test]
fn explicit_pipeline_and_vlm_keep_their_render_route() {
    let config = configuration();
    let mut ocr = config.ocr.as_ref().unwrap().clone();
    ocr.pipeline = Some(crate::core::config::OcrPipelineConfig {
        stages: Vec::new(),
        quality_thresholds: Default::default(),
    });
    assert_eq!(configured_preprocessing_render_dpi(&config, &ocr, true), None);
    ocr.pipeline = None;
    ocr.vlm_fallback = crate::core::config::VlmFallbackPolicy::Always;
    assert_eq!(configured_preprocessing_render_dpi(&config, &ocr, true), None);
}

#[cfg(all(paddle_ocr, feature = "tokio-runtime"))]
#[test]
fn first_page_budget_probe_accounts_for_configured_preprocessing_density() {
    let document =
        xberg_native_pdf::PdfDocument::from_bytes(crate::pdf::render::build_minimal_pdf_with_mediabox(432.0, 216.0))
            .unwrap();
    let mut config = configuration();
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: 8 * 1024 * 1024,
        ..Default::default()
    });
    let limits = config.security_limits.as_ref().unwrap();
    assert_eq!(
        crate::image::dpi::planned_pdf_ocr_render_dpi(&document, 0, None, limits),
        (150, 150)
    );
    assert!(
        render_budget_bounds_first_page(&document, &config),
        "promoted 300 DPI is bounded even though 150 fits"
    );
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[path = "../../../../../tests/ocr_pixel_phase_2711/fixture.rs"]
mod fixture;

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
async fn read_source_dimensions(config: &ExtractionConfig, bytes: &[u8]) -> (usize, usize) {
    let result = extract_with_ocr_for_page(
        Some(bytes),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        config,
        None,
        0,
        true,
        None,
        None,
        0,
        None,
    )
    .await
    .expect("actual direct backend route");
    let metadata = result.10.get(&1).expect("source render provenance");
    (metadata.original_dimensions.width, metadata.original_dimensions.height)
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
async fn read_public_dimensions(config: &ExtractionConfig, bytes: &[u8]) -> (usize, usize) {
    let mut config = config.clone();
    config.use_cache = false;
    config.pages = Some(crate::core::config::PageConfig {
        extract_pages: true,
        ..Default::default()
    });
    let output = crate::extract(
        crate::ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    let metadata = output.results[0].pages.as_ref().unwrap()[0]
        .image_preprocessing
        .as_ref()
        .unwrap();
    (metadata.original_dimensions.width, metadata.original_dimensions.height)
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
fn isolated_identity_child() -> bool {
    const CHILD: &str = "XBERG_DENSITY_IDENTITY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let selector = concat!(
        module_path!(),
        "::replacement_tesseract_keeps_actual_source_render_at_150"
    );
    let selector = selector.strip_prefix("xberg::").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", selector, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("1 passed; 0 failed; 0 ignored"), "{stdout}");
    true
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[tokio::test]
async fn replacement_tesseract_keeps_actual_source_render_at_150() {
    if isolated_identity_child() {
        return;
    }
    let mut config = configuration();
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let ocr = config.ocr.as_mut().unwrap();
    ocr.tessdata_path = Some(models.path().to_path_buf());
    ocr.tesseract_config.as_mut().unwrap().use_cache = false;
    let bytes = fixture::localized_pdf("gap-left");
    crate::plugins::clear_ocr_backends().unwrap();
    assert_eq!(read_public_dimensions(&config, &bytes).await, (1800, 900));
    assert_eq!(read_source_dimensions(&config, &bytes).await, (1800, 900));
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let original = registry.read().get("tesseract").unwrap();
    registry
        .write()
        .register(std::sync::Arc::new(
            crate::ocr::tesseract_backend::TesseractBackend::new(),
        ))
        .unwrap();
    let direct = read_source_dimensions(&config, &bytes).await;
    let public = read_public_dimensions(&config, &bytes).await;
    registry.write().register(original).unwrap();
    assert_eq!(
        direct,
        (900, 450),
        "same name/type does not establish built-in instance identity"
    );
    assert_eq!(
        public,
        (900, 450),
        "the public synthesized pipeline must preserve replacement identity"
    );
}

// The helper tests above stay green if the call sites in
// `extract_mixed_ocr_native_with_layout_inputs` stop consulting the helpers, so the pins
// below drive that real mixed route. On the promoted route they assert the render the route
// produced; on the excluded routes, where the supplied rasters and the probing config keep
// the render regardless, they assert the batch width the promotion's reservation still
// reaches.

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
fn prepared_layout_inputs(pdf: &[u8]) -> PreparedLayoutInputs {
    let limits = crate::extractors::security::SecurityLimits::default();
    let document = xberg_native_pdf::PdfDocument::from_bytes(pdf.to_vec()).unwrap();
    let (rendered, _) = crate::pdf::render::render_pdf_ocr_page_with_dpi(&document, 0, None, &limits).unwrap();
    let image =
        crate::extraction::image_decode::decode_standard_image_with_security_limits(&rendered.data, &limits).unwrap();
    (
        vec![image],
        vec![crate::layout::DetectionResult {
            page_width: rendered.width,
            page_height: rendered.height,
            detections: Vec::new(),
        }],
    )
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
async fn real_route_run(layout: Option<&PreparedLayoutInputs>) -> MixedOcrRun {
    let mut config = configuration();
    config.concurrency = Some(crate::core::config::ConcurrencyConfig {
        max_threads: Some(2),
        ..Default::default()
    });
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let ocr = config.ocr.as_mut().unwrap();
    ocr.tessdata_path = Some(models.path().to_path_buf());
    ocr.tesseract_config.as_mut().unwrap().use_cache = false;
    let bytes = fixture::localized_pdf("gap-left");
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];
    extract_mixed_ocr_native_with_layout_inputs(
        "",
        &boundaries,
        MixedOcrPageSelection {
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis: None,
        },
        &bytes,
        &config,
        AllPagesFailedPolicy::PreserveNative,
        match layout {
            Some(prepared) => MixedLayoutInputs::Prepared(Some(prepared)),
            None => MixedLayoutInputs::Resolve(None),
        },
    )
    .await
    .unwrap()
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
use std::sync::Arc;

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
use tracing_subscriber::layer::SubscriberExt as _;

/// Pins the free-memory figure the mixed route sizes its batch from, and restores the real
/// reading on drop, so the batch-width pins below assert the behaviour under test rather than
/// whatever the machine running them happens to have free. Process-wide, hence `#[serial]`.
#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
struct PinnedAvailableMemory(());

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
impl PinnedAvailableMemory {
    /// 512 MiB above `usable`: the route reserves 512 MiB plus the document before dividing.
    fn usable(usable_bytes: usize, document_bytes: usize) -> Self {
        assert_ne!(usable_bytes, 0, "zero means 'not pinned', so it cannot be pinned");
        TEST_AVAILABLE_MEMORY.store(
            512 * 1024 * 1024 + document_bytes + usable_bytes,
            std::sync::atomic::Ordering::SeqCst,
        );
        Self(())
    }
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
impl Drop for PinnedAvailableMemory {
    fn drop(&mut self) {
        TEST_AVAILABLE_MEMORY.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Captures the mixed route's "Reduced the per-page OCR batch" events: the one observable the
/// promotion's render reservation has on a run, and so the one that pins the call sites.
#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[derive(Clone, Default)]
struct BatchReductions(Arc<std::sync::Mutex<Vec<(usize, usize)>>>);

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
struct ReductionFields(std::collections::BTreeMap<String, String>);

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
impl tracing::field::Visit for ReductionFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
impl<S> tracing_subscriber::Layer<S> for BatchReductions
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = ReductionFields(Default::default());
        event.record(&mut fields);
        if fields.0.get("message").map(String::as_str) != Some("Reduced the per-page OCR batch to fit available memory")
        {
            return;
        }
        let parse = |name: &str| fields.0.get(name).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        self.0.lock().unwrap().push((parse("configured"), parse("adapted")));
    }
}

/// Drives the real mixed route under a pinned free-memory reading and the batch capture, and
/// returns the run so a test can assert the batch width and the render it produced together.
#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
fn under_batch_capture(
    capture: &BatchReductions,
    usable_bytes: usize,
    document_bytes: usize,
    run: impl std::future::Future<Output = MixedOcrRun>,
) -> MixedOcrRun {
    let _pin = PinnedAvailableMemory::usable(usable_bytes, document_bytes);
    tracing::subscriber::with_default(tracing_subscriber::Registry::default().with(capture.clone()), || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run)
    })
}

/// The batch pin works on the promoted route: the reservation is what pushes a 150 DPI page's
/// working set over the pinned headroom. This proves the capture sees a real reduction, and
/// that removing the `mixed_route_batch_working_set(...)` argument at the call site breaks it.
#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[test]
#[serial_test::serial]
fn promoted_mixed_route_sizes_the_batch_from_the_preprocessing_reservation() {
    let capture = BatchReductions::default();
    let bytes = fixture::localized_pdf("gap-left");
    under_batch_capture(&capture, 100 * 1024 * 1024, bytes.len(), real_route_run(None));
    assert_eq!(
        capture.0.lock().unwrap().as_slice(),
        [(2, 1)],
        "a promoted page must shrink the 2-wide batch to 1 on 100 MiB of headroom"
    );
}

#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[tokio::test]
#[serial_test::serial]
async fn plain_mixed_route_promotes_through_the_real_call() {
    let metadata = real_route_run(None).await.output.6.remove(&1).unwrap();
    assert_eq!(
        (metadata.original_dimensions.width, metadata.original_dimensions.height),
        (1800, 900),
        "the plain mixed route must still render at the configured 300 DPI"
    );
}

/// The wiring pin: with layout detection active the route keeps the source render and must not
/// size its batch from a promotion reservation. On the prepared-layout path the route consumes
/// the supplied rasters as-is, so the render dimension alone cannot see the promotion gate —
/// the batch width is the one place the excluded promotion still reaches. Asserts no reduction
/// event fired: restoring the promotion at the call site shrinks the batch and fires one.
#[cfg(all(feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#[test]
#[serial_test::serial]
fn layout_active_mixed_route_keeps_the_source_render_through_the_real_call() {
    let capture = BatchReductions::default();
    let bytes = fixture::localized_pdf("gap-left");
    let prepared = prepared_layout_inputs(&bytes);
    let mut run = under_batch_capture(
        &capture,
        100 * 1024 * 1024,
        bytes.len(),
        real_route_run(Some(&prepared)),
    );
    assert!(
        capture.0.lock().unwrap().is_empty(),
        "active layout detection must not let a promotion reservation shrink the batch: {:?}",
        capture.0.lock().unwrap()
    );
    let metadata = run
        .output
        .6
        .remove(&1)
        .expect("the OCR page records its render provenance");
    assert_eq!(
        (metadata.original_dimensions.width, metadata.original_dimensions.height),
        (900, 450),
        "active layout detection must keep the 150 DPI source render"
    );
}
