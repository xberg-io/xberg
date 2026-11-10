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
