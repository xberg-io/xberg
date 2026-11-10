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
fn explicit_density_requires_direct_builtin_forced_ocr() {
    let config = configuration();
    let ocr = config.ocr.as_ref().unwrap();
    assert_eq!(explicit_preprocessing_render_dpi(&config, ocr, true, true), Some(300));
    assert_eq!(explicit_preprocessing_render_dpi(&config, ocr, true, false), None);
    assert_eq!(explicit_preprocessing_render_dpi(&config, ocr, false, true), None);
    let automatic = ExtractionConfig {
        force_ocr: false,
        ..config.clone()
    };
    assert_eq!(explicit_preprocessing_render_dpi(&automatic, ocr, true, true), None);
    let explicit = ExtractionConfig {
        images: Some(Default::default()),
        ..config.clone()
    };
    assert_eq!(explicit_preprocessing_render_dpi(&explicit, ocr, true, true), None);
}

#[test]
fn implicit_or_lower_normalization_does_not_promote_rendering() {
    let config = configuration();
    let mut ocr = config.ocr.as_ref().unwrap().clone();
    ocr.tesseract_config.as_mut().unwrap().preprocessing = None;
    assert_eq!(explicit_preprocessing_render_dpi(&config, &ocr, true, true), None);
    ocr.tesseract_config.as_mut().unwrap().preprocessing = Some(crate::types::ImagePreprocessingConfig {
        target_dpi: 150,
        ..Default::default()
    });
    assert_eq!(explicit_preprocessing_render_dpi(&config, &ocr, true, true), None);
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
fn explicit_or_synthesized_pipelines_keep_their_render_route() {
    let config = configuration();
    let mut ocr = config.ocr.as_ref().unwrap().clone();
    ocr.pipeline = Some(crate::core::config::OcrPipelineConfig {
        stages: Vec::new(),
        quality_thresholds: Default::default(),
    });
    assert_eq!(explicit_preprocessing_render_dpi(&config, &ocr, true, true), None);
    ocr.pipeline = None;
    ocr.vlm_fallback = crate::core::config::VlmFallbackPolicy::Always;
    assert_eq!(explicit_preprocessing_render_dpi(&config, &ocr, true, true), None);
}
