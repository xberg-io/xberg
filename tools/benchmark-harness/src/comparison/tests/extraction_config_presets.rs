//! Timed-cache disabling, PSM finalization, and Paddle/Sceptre/Tesseract preset tests.

use super::*;
use crate::comparison::extraction_config::{
    PADDLE_DEFAULT_QUALITY_PROFILE, PADDLE_DET_DB_BOX_THRESH_035_QUALITY_PROFILE,
    PADDLE_DET_DB_THRESH_020_QUALITY_PROFILE, PADDLE_DET_SIDE_1536_QUALITY_PROFILE,
    PADDLE_DET_SIDE_2048_QUALITY_PROFILE, PADDLE_DROP_SCORE_030_QUALITY_PROFILE, PADDLE_DROP_SCORE_040_QUALITY_PROFILE,
    PP_OCR_V5, PP_OCR_V6, SCEPTRE_MODEL_BACKEND_ORT, finalize_timed_ocr_result_cache, materialize_implicit_ocr_config,
};

#[test]
fn layout_preset_matches_ci_native_layout_semantics() {
    let config = build_extraction_config(Pipeline::Layout);

    assert!(!config.force_ocr, "native layout must not force OCR");
    assert!(config.layout.is_some(), "native layout must enable layout detection");
    assert!(
        config.use_layout_for_markdown,
        "native layout must use detections when rendering Markdown"
    );
    assert_eq!(
        config.ocr.as_ref().map(|ocr| ocr.auto_rotate),
        None,
        "native layout must not enable OCR auto-rotation"
    );
}

#[test]
fn every_layout_named_preset_uses_layout_for_markdown() {
    let layout_pipelines = [
        Pipeline::Layout,
        Pipeline::TesseractLayout,
        Pipeline::PaddleLayout,
        Pipeline::PaddleV6SmallLayout,
        Pipeline::PaddleV6SmallLayoutDetSide1024,
        Pipeline::PaddleV6SmallLayoutDetSide1536,
        Pipeline::PaddleV6SmallLayoutDetSide2048,
        Pipeline::PaddleV6SmallLayoutDetDbThresh020,
        Pipeline::PaddleV6SmallLayoutDetDbBoxThresh035,
        Pipeline::PaddleV6SmallLayoutDropScore030,
        Pipeline::PaddleV6SmallLayoutDropScore040,
        Pipeline::PaddleV6TinyLayout,
        Pipeline::PaddleServerLayout,
        Pipeline::SceptreLayout,
        Pipeline::LayoutSlanetWired,
        Pipeline::LayoutSlanetWireless,
        Pipeline::LayoutSlanetPlus,
        Pipeline::LayoutSlanetAuto,
        Pipeline::NativeLayout,
        Pipeline::NativeReadingOrder,
        Pipeline::CandleGlmOcrLayout,
        Pipeline::CandleGlmOcrLayoutChart,
    ];

    for pipeline in layout_pipelines {
        let config = build_extraction_config(pipeline);
        assert!(
            config.use_layout_for_markdown,
            "layout-named preset {} must use detections when rendering Markdown",
            pipeline.name()
        );
    }
}

#[test]
fn timed_xberg_pipeline_configs_disable_extraction_cache() {
    for pipeline in Pipeline::all_xberg() {
        let config = build_extraction_config(pipeline);

        assert!(
            !config.use_cache,
            "timed pipeline {} must measure extraction rather than a cache hit",
            pipeline.name()
        );

        let Some(ocr) = config.ocr.as_ref() else {
            continue;
        };
        // A `tesseract_config` that already existed (explicit PSM presets, e.g.
        // that leaves `tesseract_config` implicit (auto-PSM) must keep it absent — xberg only
        // auto-selects PSM when `ocr.tesseract_config` is absent, so materializing one here
        // to force `use_cache = false` would silently pin PSM to the default (3).
        if let Some(ocr_pipeline) = ocr.pipeline.as_ref() {
            for stage in &ocr_pipeline.stages {
                if stage.backend == "tesseract"
                    && let Some(tesseract_config) = stage.tesseract_config.as_ref()
                {
                    assert!(
                        !tesseract_config.use_cache,
                        "timed Tesseract stage in {} must disable its result cache",
                        pipeline.name()
                    );
                }
            }
        } else if ocr.backend == "tesseract"
            && let Some(tesseract_config) = ocr.tesseract_config.as_ref()
        {
            assert!(
                !tesseract_config.use_cache,
                "timed pipeline {} must disable the Tesseract result cache",
                pipeline.name()
            );
        }
    }
}

#[test]
fn finalize_disables_cache_and_materializes_correct_psm_for_implicit_tesseract_stages() {
    let ocr = serde_json::from_value(serde_json::json!({
        "pipeline": {
            "stages": [
                { "backend": "tesseract", "tesseract_config": { "psm": 6 } },
                { "backend": "tesseract" },
                { "backend": "paddleocr" }
            ]
        }
    }))
    .unwrap();
    let mut config = xberg::ExtractionConfig {
        ocr: Some(ocr),
        ..Default::default()
    };

    finalize_timed_ocr_result_cache(&mut config);

    let stages = &config.ocr.unwrap().pipeline.unwrap().stages;
    let explicit = stages[0].tesseract_config.as_ref().unwrap();
    assert!(
        !explicit.use_cache,
        "an already-configured stage must disable its cache"
    );
    assert_eq!(
        explicit.psm,
        Some(6),
        "disabling the cache must not touch an explicit PSM"
    );

    let implicit = stages[1]
        .tesseract_config
        .as_ref()
        .expect("an implicit stage must be materialized so its result cache is genuinely disabled");
    assert!(!implicit.use_cache);
    assert_eq!(
        implicit.psm,
        Some(crate::adapter::XBERG_WHOLE_IMAGE_TESSERACT_PSM),
        "an implicit stage's materialized PSM must match xberg's own auto-selection for the \
         default language, or it would silently regress to PSM 3"
    );
    assert_eq!(implicit.language, ["eng"]);

    assert!(stages[2].tesseract_config.is_none());
}

#[test]
fn finalize_materializes_whole_image_psm_for_implicit_ocr_with_default_language() {
    let mut config = xberg::ExtractionConfig::default();
    materialize_implicit_ocr_config(&mut config);

    finalize_timed_ocr_result_cache(&mut config);

    let ocr = config.ocr.expect("materialize_implicit_ocr_config must configure OCR");
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(
        ocr.language,
        ["eng"],
        "the parent OcrConfig.language must also read 'eng'"
    );
    let tesseract = ocr
        .tesseract_config
        .expect("implicit timed OCR must materialize a tesseract_config to genuinely disable its cache");
    assert!(!tesseract.use_cache);
    assert_eq!(
        tesseract.psm,
        Some(crate::adapter::XBERG_WHOLE_IMAGE_TESSERACT_PSM),
        "materialized PSM must match xberg's own auto-selection, or it would silently \
         regress to PSM 3"
    );
    assert_eq!(
        tesseract.language,
        ["eng"],
        "language must land on both ocr.language and the materialized tesseract_config.language"
    );
}

#[test]
fn finalize_preserves_explicit_bare_tesseract_psm() {
    let ocr = xberg::OcrConfig {
        backend: "tesseract".to_string(),
        language: vec!["eng".to_string()],
        tesseract_config: Some(xberg::TesseractConfig {
            psm: Some(6),
            use_cache: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut config = xberg::ExtractionConfig {
        ocr: Some(ocr),
        ..Default::default()
    };

    finalize_timed_ocr_result_cache(&mut config);

    let tesseract = config.ocr.unwrap().tesseract_config.unwrap();
    assert!(!tesseract.use_cache);
    assert_eq!(
        tesseract.psm,
        Some(6),
        "disabling the cache must not touch an explicit PSM"
    );
    assert_eq!(tesseract.language, ["eng"], "language must still be refreshed");
}

#[test]
fn paddle_v6_presets_pin_model_tier_and_layout() {
    let presets = [
        (Pipeline::Paddle, "medium", false),
        (Pipeline::PaddleLayout, "medium", true),
        (Pipeline::PaddleV6Small, "small", false),
        (Pipeline::PaddleV6SmallLayout, "small", true),
        (Pipeline::PaddleV6Tiny, "tiny", false),
        (Pipeline::PaddleV6TinyLayout, "tiny", true),
    ];

    for (pipeline, expected_tier, expected_layout) in presets {
        let config = build_extraction_config(pipeline);
        let ocr = config.ocr.as_ref().expect("Paddle preset must configure OCR");
        let paddle = ocr
            .paddle_ocr_settings
            .as_ref()
            .expect("Paddle preset must pin model identity");

        assert_eq!(ocr.backend, "paddleocr", "unexpected backend for {}", pipeline.name());
        assert_eq!(
            ocr.auto_rotate, None,
            "canonical Paddle preset must use the public default"
        );
        assert_eq!(
            paddle.model_version,
            PP_OCR_V6,
            "unexpected version for {}",
            pipeline.name()
        );
        assert_eq!(
            paddle.model_tier,
            expected_tier,
            "unexpected tier for {}",
            pipeline.name()
        );
        assert_eq!(
            config.layout.is_some(),
            expected_layout,
            "unexpected layout for {}",
            pipeline.name()
        );
    }
}

#[test]
fn paddle_quality_sweep_presets_pin_every_swept_dimension() {
    let cases = [
        (Pipeline::PaddleV6SmallLayoutDetSide1024, PADDLE_DEFAULT_QUALITY_PROFILE),
        (
            Pipeline::PaddleV6SmallLayoutDetSide1536,
            PADDLE_DET_SIDE_1536_QUALITY_PROFILE,
        ),
        (
            Pipeline::PaddleV6SmallLayoutDetSide2048,
            PADDLE_DET_SIDE_2048_QUALITY_PROFILE,
        ),
        (
            Pipeline::PaddleV6SmallLayoutDetDbThresh020,
            PADDLE_DET_DB_THRESH_020_QUALITY_PROFILE,
        ),
        (
            Pipeline::PaddleV6SmallLayoutDetDbBoxThresh035,
            PADDLE_DET_DB_BOX_THRESH_035_QUALITY_PROFILE,
        ),
        (
            Pipeline::PaddleV6SmallLayoutDropScore030,
            PADDLE_DROP_SCORE_030_QUALITY_PROFILE,
        ),
        (
            Pipeline::PaddleV6SmallLayoutDropScore040,
            PADDLE_DROP_SCORE_040_QUALITY_PROFILE,
        ),
    ];

    for (pipeline, expected) in cases {
        let config = build_extraction_config(pipeline);
        let ocr = config.ocr.expect("quality preset must configure OCR");
        let paddle = ocr
            .paddle_ocr_settings
            .expect("quality preset must pin Paddle settings");

        assert!(config.force_ocr);
        assert!(config.layout.is_some());
        assert_eq!(ocr.backend, "paddleocr");
        assert_eq!(paddle.model_version, PP_OCR_V6);
        assert_eq!(paddle.model_tier, "small");
        assert_eq!(paddle.det_limit_side_len, expected.det_limit_side_len);
        assert_eq!(paddle.det_db_thresh, expected.det_db_thresh);
        assert_eq!(paddle.det_db_box_thresh, expected.det_db_box_thresh);
        assert_eq!(paddle.drop_score, expected.drop_score);
    }
}

#[test]
fn paddle_rotation_presets_differ_only_by_auto_rotate() {
    let auto_rotate = build_extraction_config(Pipeline::PaddleAutoRotate);
    let no_rotate = build_extraction_config(Pipeline::PaddleNoRotate);
    let auto_ocr = auto_rotate.ocr.expect("auto-rotate preset must configure OCR");
    let no_rotate_ocr = no_rotate.ocr.expect("no-rotate preset must configure OCR");

    assert_eq!(auto_ocr.auto_rotate, Some(true));
    assert_eq!(no_rotate_ocr.auto_rotate, None);
    assert_eq!(auto_ocr.backend, no_rotate_ocr.backend);
    assert_eq!(auto_ocr.paddle_ocr_settings, no_rotate_ocr.paddle_ocr_settings);
}

#[test]
fn sceptre_pipeline_aliases_parse_to_expected_variant() {
    let aliases = [
        ("sceptre", Pipeline::Sceptre),
        ("sceptre-ort", Pipeline::Sceptre),
        ("sceptre_ort", Pipeline::Sceptre),
        ("sceptre-ort+layout", Pipeline::SceptreLayout),
        ("sceptre-ort-layout", Pipeline::SceptreLayout),
        ("sceptre-layout", Pipeline::SceptreLayout),
        ("sceptre+layout", Pipeline::SceptreLayout),
        ("sceptre-ort-autorotate", Pipeline::SceptreAutoRotate),
        ("sceptre-autorotate", Pipeline::SceptreAutoRotate),
    ];

    for (alias, expected) in aliases {
        assert_eq!(
            Pipeline::parse(alias),
            Some(expected),
            "failed to parse sceptre alias '{alias}'"
        );
    }
    assert_eq!(Pipeline::Sceptre.name(), "sceptre-ort");
    assert_eq!(Pipeline::SceptreLayout.name(), "sceptre-ort+layout");
    assert_eq!(Pipeline::SceptreAutoRotate.name(), "sceptre-ort-autorotate");
}

#[test]
fn unknown_pipeline_name_reports_the_requested_name() {
    // Regression guard: adding the sceptre variants must not turn an unrelated typo into a
    // match (e.g. via an overly broad alias), and the caller-supplied name must still surface
    // verbatim so `main::parse_pipeline_names` can build its "unknown pipeline '<name>'" error.
    assert_eq!(Pipeline::parse("sceptre-typo"), None);
    assert_eq!(Pipeline::parse("sceptr"), None);
}

#[test]
fn sceptre_presets_pin_the_ort_inference_engine_and_backend() {
    let cases = [
        (Pipeline::Sceptre, false, false),
        (Pipeline::SceptreLayout, true, false),
        (Pipeline::SceptreAutoRotate, false, true),
    ];

    for (pipeline, expected_layout, expected_auto_rotate) in cases {
        let config = build_extraction_config(pipeline);
        let ocr = config.ocr.as_ref().expect("Sceptre preset must configure OCR");
        let backend_options = ocr
            .backend_options
            .as_ref()
            .expect("Sceptre preset must pin the inference engine");

        assert!(config.force_ocr, "Sceptre preset {} must force OCR", pipeline.name());
        assert_eq!(ocr.backend, "sceptre", "unexpected backend for {}", pipeline.name());
        assert_eq!(
            backend_options["model"]["backend"],
            SCEPTRE_MODEL_BACKEND_ORT,
            "unexpected inference engine for {}",
            pipeline.name()
        );
        assert_eq!(
            config.layout.is_some(),
            expected_layout,
            "unexpected layout for {}",
            pipeline.name()
        );
        assert_eq!(
            ocr.auto_rotate,
            expected_auto_rotate.then_some(true),
            "unexpected auto_rotate for {}",
            pipeline.name()
        );
    }
}

#[test]
fn paddle_quality_sweep_presets_are_opt_in() {
    let defaults = Pipeline::all_xberg();
    for pipeline in [
        Pipeline::PaddleV6SmallLayoutDetSide1024,
        Pipeline::PaddleV6SmallLayoutDetSide1536,
        Pipeline::PaddleV6SmallLayoutDetSide2048,
        Pipeline::PaddleV6SmallLayoutDetDbThresh020,
        Pipeline::PaddleV6SmallLayoutDetDbBoxThresh035,
        Pipeline::PaddleV6SmallLayoutDropScore030,
        Pipeline::PaddleV6SmallLayoutDropScore040,
    ] {
        assert!(!defaults.contains(&pipeline), "{} must remain opt-in", pipeline.name());
        assert_eq!(
            serde_json::to_value(pipeline).expect("quality preset must serialize"),
            serde_json::Value::String(pipeline.name().to_string()),
            "{} must preserve its result identity when serialized",
            pipeline.name()
        );
    }
}

#[test]
fn legacy_paddle_server_presets_pin_v5_server_models() {
    for pipeline in [Pipeline::PaddleServer, Pipeline::PaddleServerLayout] {
        let config = build_extraction_config(pipeline);
        let ocr = config.ocr.as_ref().expect("legacy server preset must configure OCR");
        let paddle = ocr
            .paddle_ocr_settings
            .as_ref()
            .expect("legacy server preset must pin model identity");

        assert_eq!(
            ocr.auto_rotate, None,
            "canonical Paddle preset must use the public default"
        );
        assert_eq!(paddle.model_version, PP_OCR_V5);
        assert_eq!(paddle.model_tier, "server");
    }
}
