#[cfg(feature = "ocr-surface")]
use super::super::*;
#[cfg(feature = "ocr-surface")]
use super::default_overrides;
#[cfg(feature = "ocr-surface")]
use xberg::{ExtractionConfig, OcrConfig};

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_default_language_tesseract() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(ocr.language, vec!["eng".to_string()]);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_default_language_paddleocr() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("paddle-ocr".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "paddle-ocr");
    assert_eq!(ocr.language, vec!["en".to_string()]);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_default_language_sceptre() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("sceptre".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.expect("OCR config should be set");
    assert_eq!(ocr.backend, "sceptre");
    assert_eq!(ocr.language, vec!["eng".to_string()]);
}

/// Regression test for the empirically observed CLI defect: running
/// `xberg extract doc.pdf --ocr-scanned-pages --ocr-backend sceptre` (no
/// `--ocr true`) silently ran tesseract instead of sceptre, because
/// `apply_ocr` only materialised `config.ocr` on `--ocr true`, so
/// `apply_ocr_fields` — which assigns `backend` — never ran, while
/// `--ocr-scanned-pages` set `config.ocr_strategy` unconditionally and
/// triggered OCR to run anyway with the default backend. Before the fix
/// (i.e. reverting `has_ocr_field_flag` back to just `self.ocr ==
/// Some(true)`), `config.ocr` stays `None` here — `ocr.backend` is never
/// even reachable — so this assertion fails without the fix.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_backend_flag_selects_backend_without_ocr_true_flag() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr_backend: Some("sceptre".to_string()),
        ocr_scanned_pages: true,
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config
        .ocr
        .expect("--ocr-backend must materialise an OCR config even without --ocr true");
    assert_eq!(ocr.backend, "sceptre");
    assert_eq!(
        config.ocr_strategy,
        xberg::OcrStrategy::ScannedPages {
            min_confidence: xberg::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE
        }
    );
}

/// Regression test for #656: `--ocr-scanned-pages` alone (no `--extract-pages`, no
/// `--ocr true`, no other `--ocr-*` field flag) previously left `config.pages` as `None`.
/// The PDF backend only tracks per-page byte boundaries when `config.pages` is `Some`
/// (`pdf::native::text::extract_text_from_native_document`'s `page_config` branch), and the
/// mixed OCR route needs those boundaries to splice OCR text back into the native text
/// (`extractors/pdf/mod.rs`). Without them, that route always fell back to "no page
/// boundaries available; using native text" -- an empty result for a scan, i.e. a
/// zero-byte, exit-0 extraction. Before the fix (i.e. removing the
/// `config.pages.get_or_insert_with(Default::default)` call), `config.pages` stays `None`
/// here and this assertion fails.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_scanned_pages_alone_materialises_page_boundary_tracking() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr_scanned_pages: true,
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let pages = config
        .pages
        .expect("--ocr-scanned-pages must materialise page-boundary tracking on its own");
    assert!(
        !pages.extract_pages,
        "boundary tracking must not silently turn on the `pages` output array"
    );
}

/// Companion guard: materialising `config.pages` for boundary tracking must not clobber an
/// explicit `--extract-pages true` given alongside `--ocr-scanned-pages`.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_scanned_pages_does_not_override_explicit_extract_pages_flag() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr_scanned_pages: true,
        extract_pages: Some(true),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let pages = config.pages.expect("pages config should be set");
    assert!(pages.extract_pages, "--extract-pages true must still take effect");
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_validate_unknown_ocr_backend_rejected() {
    let overrides = ExtractionOverrides {
        ocr_backend: Some("unsupported-ocr".to_string()),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert!(err.to_string().contains("Invalid OCR backend"));
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_override_tesseract() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_language: Some("fra".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(ocr.language, vec!["fra".to_string()]);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_override_paddleocr() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("paddle-ocr".to_string()),
        ocr_language: Some("ch".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "paddle-ocr");
    assert_eq!(ocr.language, vec!["ch".to_string()]);
}

/// `--ocr-language` alone (no `--ocr true`, no pre-existing `config.ocr`) must
/// still materialise an OCR config carrying the requested language — naming a
/// field is enough to select it, exactly like `--ocr-backend`. Before the fix,
/// this flag was silently discarded whenever `config.ocr` was still `None`.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_without_ocr_flag_no_existing_config() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.expect("--ocr-language alone must materialise an OCR config");
    assert_eq!(ocr.language, vec!["deu".to_string()]);
    assert_eq!(ocr.backend, "tesseract", "backend keeps its compiled-in default");
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_without_ocr_flag_existing_config() {
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: None,
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: None,
            vlm_fallback: Default::default(),
            vlm_prompt: None,
            acceleration: None,
            security_limits: None,
            tessdata_bytes: None,
            tessdata_path: None,
            backend_options: None,
        }),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(ocr.language, vec!["deu".to_string()]);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_updates_existing_nested_tesseract_config() {
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(xberg::TesseractConfig {
                language: vec!["eng".to_string()],
                use_cache: false,
                ..Default::default()
            }),
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: None,
            vlm_fallback: Default::default(),
            vlm_prompt: None,
            acceleration: None,
            security_limits: None,
            tessdata_bytes: None,
            tessdata_path: None,
            backend_options: None,
        }),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.language, vec!["deu".to_string()]);
    let tesseract = ocr.tesseract_config.unwrap();
    assert_eq!(tesseract.language, vec!["deu".to_string()]);
    assert!(!tesseract.use_cache);
}

/// `--ocr-no-cache` alone (no `--ocr true`, no pre-existing `config.ocr`) must be a
/// no-op: `config.ocr` must stay `None`.
///
/// Regression test for #693. The flag's entire contract is "bypass the Tesseract OCR
/// cache, and nothing else" (see its doc comment), but a prior version of
/// `apply_ocr_no_cache` materialised `tesseract_config` from `TesseractConfig::default()`
/// whenever it was still `None`, purely to have somewhere to write `use_cache: false`.
/// That flipped `tesseract_config` from `None` to `Some(..)`, which
/// `crates/xberg/src/extractors/image.rs::apply_default_tesseract_psm` (and its siblings)
/// treat as "the caller already made an explicit PSM choice" — disarming the
/// `WHOLE_IMAGE_TESSERACT_PSM = 11` default for whole-page image OCR and leaving
/// `TesseractConfig::default()`'s `psm = 3` instead. Measured on a real scan: 217
/// recognised words without `--ocr-no-cache` vs. 194 with it, from that PSM change alone.
///
/// Against the code before this fix (i.e. `apply_ocr_no_cache` using
/// `ocr.tesseract_config.get_or_insert_with(|| TesseractConfig { language, ..Default::default() })`
/// and `has_ocr_field_flag` including `self.ocr_no_cache.is_some()`), this assertion
/// fails: `config.ocr` is `Some(..)` with `tesseract_config` also `Some(TesseractConfig {
/// psm: 3, use_cache: false, .. })` instead of `None`.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_no_cache_alone_is_a_no_op_without_existing_tesseract_config() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr_no_cache: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);

    assert!(
        config.ocr.is_none(),
        "--ocr-no-cache alone must not materialise config.ocr: doing so (even just to \
             carry use_cache) would disarm the whole-image PSM default in \
             extractors/image.rs::apply_default_tesseract_psm and silently change what \
             Tesseract recognises"
    );
}

/// Companion to the no-op test above for the case where `config.ocr` already exists
/// (e.g. set by `--ocr true`) but `tesseract_config` itself does not. `--ocr-no-cache`
/// must still leave `tesseract_config` as `None` rather than materialising it.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_no_cache_leaves_tesseract_config_none_when_ocr_config_exists_without_it() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_no_cache: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must materialise config.ocr");
    assert!(
        ocr.tesseract_config.is_none(),
        "--ocr-no-cache must not materialise tesseract_config on its own, even when \
             config.ocr already exists from --ocr true"
    );
}

/// When `tesseract_config` is already set (e.g. by a loaded config file),
/// `--ocr-no-cache` must flip `use_cache` and change *nothing else*: every other field
/// of `TesseractConfig` that reaches the OCR engine must come out identical.
///
/// Regression test for #693's core claim ("a flag whose entire contract is caching must
/// not move recognition output"), and for the class of bug a `use_cache`-only assertion
/// would miss. Against the code before this fix, this assertion still happens to pass
/// for this particular case (tesseract_config already `Some`, so the old
/// `get_or_insert_with` was a no-op and only `use_cache` changed) — the failure mode
/// this fix targets is exercised by
/// `test_ocr_no_cache_alone_is_a_no_op_without_existing_tesseract_config` above, which
/// covers the case this test cannot: `tesseract_config` starting out `None`.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_no_cache_changes_only_use_cache_when_tesseract_config_already_set() {
    let non_default_tesseract_config = xberg::TesseractConfig {
        language: vec!["fra".to_string(), "deu".to_string()],
        psm: Some(11),
        output_format: "hocr".to_string(),
        oem: 1,
        min_confidence: 42.5,
        preprocessing: Some(xberg::ImagePreprocessingConfig {
            target_dpi: 600,
            auto_rotate: true,
            deskew: false,
            denoise: true,
            contrast_enhance: true,
            binarization_method: "sauvola".to_string(),
            invert_colors: true,
        }),
        enable_table_detection: false,
        table_min_confidence: 0.75,
        table_column_threshold: 12,
        table_row_threshold_ratio: 0.9,
        use_cache: true,
        classify_use_pre_adapted_templates: false,
        language_model_ngram_on: true,
        tessedit_dont_blkrej_good_wds: false,
        tessedit_dont_rowrej_good_wds: false,
        tessedit_enable_dict_correction: false,
        tessedit_char_whitelist: "0123456789".to_string(),
        tessedit_char_blacklist: "@#".to_string(),
        tessedit_use_primary_params_model: false,
        textord_space_size_is_variable: false,
        thresholding_method: "sauvola".to_string(),
    };
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".to_string(),
            language: vec!["fra".to_string(), "deu".to_string()],
            tesseract_config: Some(non_default_tesseract_config.clone()),
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: None,
            vlm_fallback: Default::default(),
            vlm_prompt: None,
            acceleration: None,
            security_limits: None,
            tessdata_bytes: None,
            tessdata_path: None,
            backend_options: None,
        }),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr_no_cache: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);

    let tesseract = config.ocr.unwrap().tesseract_config.unwrap();
    assert!(!tesseract.use_cache, "--ocr-no-cache true must disable use_cache");
    assert_eq!(tesseract.language, non_default_tesseract_config.language);
    assert_eq!(tesseract.psm, non_default_tesseract_config.psm);
    assert_eq!(tesseract.output_format, non_default_tesseract_config.output_format);
    assert_eq!(tesseract.oem, non_default_tesseract_config.oem);
    assert_eq!(tesseract.min_confidence, non_default_tesseract_config.min_confidence);
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.target_dpi),
        non_default_tesseract_config
            .preprocessing
            .as_ref()
            .map(|p| p.target_dpi)
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.auto_rotate),
        non_default_tesseract_config
            .preprocessing
            .as_ref()
            .map(|p| p.auto_rotate)
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.deskew),
        non_default_tesseract_config.preprocessing.as_ref().map(|p| p.deskew)
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.denoise),
        non_default_tesseract_config.preprocessing.as_ref().map(|p| p.denoise)
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.contrast_enhance),
        non_default_tesseract_config
            .preprocessing
            .as_ref()
            .map(|p| p.contrast_enhance)
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.binarization_method.clone()),
        non_default_tesseract_config
            .preprocessing
            .as_ref()
            .map(|p| p.binarization_method.clone())
    );
    assert_eq!(
        tesseract.preprocessing.as_ref().map(|p| p.invert_colors),
        non_default_tesseract_config
            .preprocessing
            .as_ref()
            .map(|p| p.invert_colors)
    );
    assert_eq!(
        tesseract.enable_table_detection,
        non_default_tesseract_config.enable_table_detection
    );
    assert_eq!(
        tesseract.table_min_confidence,
        non_default_tesseract_config.table_min_confidence
    );
    assert_eq!(
        tesseract.table_column_threshold,
        non_default_tesseract_config.table_column_threshold
    );
    assert_eq!(
        tesseract.table_row_threshold_ratio,
        non_default_tesseract_config.table_row_threshold_ratio
    );
    assert_eq!(
        tesseract.classify_use_pre_adapted_templates,
        non_default_tesseract_config.classify_use_pre_adapted_templates
    );
    assert_eq!(
        tesseract.language_model_ngram_on,
        non_default_tesseract_config.language_model_ngram_on
    );
    assert_eq!(
        tesseract.tessedit_dont_blkrej_good_wds,
        non_default_tesseract_config.tessedit_dont_blkrej_good_wds
    );
    assert_eq!(
        tesseract.tessedit_dont_rowrej_good_wds,
        non_default_tesseract_config.tessedit_dont_rowrej_good_wds
    );
    assert_eq!(
        tesseract.tessedit_enable_dict_correction,
        non_default_tesseract_config.tessedit_enable_dict_correction
    );
    assert_eq!(
        tesseract.tessedit_char_whitelist,
        non_default_tesseract_config.tessedit_char_whitelist
    );
    assert_eq!(
        tesseract.tessedit_char_blacklist,
        non_default_tesseract_config.tessedit_char_blacklist
    );
    assert_eq!(
        tesseract.tessedit_use_primary_params_model,
        non_default_tesseract_config.tessedit_use_primary_params_model
    );
    assert_eq!(
        tesseract.textord_space_size_is_variable,
        non_default_tesseract_config.textord_space_size_is_variable
    );
    assert_eq!(
        tesseract.thresholding_method,
        non_default_tesseract_config.thresholding_method
    );
}

/// `--ocr-no-cache false` (explicitly re-enabling) must flip an already-disabled
/// `tesseract_config.use_cache` back to `true` rather than being a one-way switch.
#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_no_cache_false_re_enables_an_already_disabled_cache() {
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(xberg::TesseractConfig {
                language: vec!["eng".to_string()],
                use_cache: false,
                ..Default::default()
            }),
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: None,
            vlm_fallback: Default::default(),
            vlm_prompt: None,
            acceleration: None,
            security_limits: None,
            tessdata_bytes: None,
            tessdata_path: None,
            backend_options: None,
        }),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr_no_cache: Some(false),
        ..default_overrides()
    };
    overrides.apply(&mut config);

    let ocr = config.ocr.unwrap();
    assert!(ocr.tesseract_config.unwrap().use_cache);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_language_updates_tesseract_pipeline_stages() {
    let tesseract_config = xberg::TesseractConfig {
        language: vec!["eng".to_string()],
        use_cache: false,
        ..Default::default()
    };
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            pipeline: Some(xberg::OcrPipelineConfig {
                stages: vec![
                    xberg::OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: Some(vec!["eng".to_string()]),
                        tesseract_config: Some(tesseract_config),
                        paddle_ocr_config: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                    xberg::OcrPipelineStage {
                        backend: "paddle-ocr".to_string(),
                        priority: 90,
                        language: Some(vec!["en".to_string()]),
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                ],
                quality_thresholds: Default::default(),
            }),
            ..OcrConfig::default()
        }),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let stages = &config.ocr.unwrap().pipeline.unwrap().stages;
    assert_eq!(stages[0].language, Some(vec!["deu".to_string()]));
    assert_eq!(
        stages[0].tesseract_config.as_ref().unwrap().language,
        vec!["deu".to_string()]
    );
    assert_eq!(stages[1].backend, "paddle-ocr");
    assert_eq!(stages[1].priority, 90);
    assert_eq!(stages[1].language, Some(vec!["en".to_string()]));
    assert!(stages[1].tesseract_config.is_none());
    assert!(stages[1].paddle_ocr_config.is_none());
    assert!(stages[1].vlm_config.is_none());
    assert!(stages[1].backend_options.is_none());
}

#[cfg(feature = "ocr-surface")]
#[test]
fn test_ocr_disabled_ignores_language() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        ocr: Some(false),
        ocr_language: Some("fra".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    assert!(config.ocr.is_none());
}
