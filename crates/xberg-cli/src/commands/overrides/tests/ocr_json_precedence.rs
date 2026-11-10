#[cfg(feature = "ocr-surface")]
use super::super::*;
#[cfg(feature = "ocr-surface")]
use super::{config_from_json, default_overrides};
#[cfg(feature = "ocr-surface")]
use xberg::{ExtractionConfig, LlmConfig, OcrConfig};

#[cfg(feature = "ocr-surface")]
#[test]
fn should_keep_config_json_quality_thresholds_when_ocr_flags_are_also_given() {
    let mut config =
        config_from_json(r#"{"ocr":{"quality_thresholds":{"max_ocr_output_fragmented_word_ratio":0.99}}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("tesseract".to_string()),
        ocr_language: Some("eng".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(ocr.language, vec!["eng".to_string()]);
    let thresholds = ocr
        .quality_thresholds
        .expect("quality_thresholds has no CLI flag, so --config-json must remain its only source");
    assert_eq!(thresholds.max_ocr_output_fragmented_word_ratio, 0.99);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn should_keep_config_json_tessdata_path_when_ocr_flags_are_also_given() {
    let mut config = config_from_json(r#"{"ocr":{"tessdata_path":"/opt/tessdata"}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("tesseract".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(
        ocr.tessdata_path,
        Some(std::path::PathBuf::from("/opt/tessdata")),
        "tessdata_path has no CLI flag and must survive --ocr/--ocr-backend"
    );
}

/// Every OCR field that has no CLI flag, populated so
/// `should_keep_every_non_flag_ocr_field_when_ocr_flags_are_also_given` can assert
/// each one survives `apply()` untouched.
#[cfg(feature = "ocr-surface")]
fn ocr_config_with_every_non_flag_field_set() -> OcrConfig {
    OcrConfig {
        backend: "tesseract".to_string(),
        tesseract_config: Some(xberg::TesseractConfig {
            language: vec!["eng".to_string()],
            use_cache: false,
            ..Default::default()
        }),
        quality_thresholds: Some(xberg::OcrQualityThresholds {
            max_ocr_output_fragmented_word_ratio: 0.99,
            ..Default::default()
        }),
        pipeline: Some(xberg::OcrPipelineConfig {
            stages: vec![xberg::OcrPipelineStage {
                backend: "tesseract".to_string(),
                priority: 100,
                language: Some(vec!["eng".to_string()]),
                tesseract_config: None,
                paddle_ocr_config: None,
                paddle_ocr_settings: None,
                vlm_config: None,
                backend_options: None,
            }],
            quality_thresholds: Default::default(),
        }),
        vlm_config: Some(LlmConfig {
            model: "openai/gpt-4o".to_string(),
            ..Default::default()
        }),
        vlm_fallback: xberg::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 },
        vlm_prompt: Some("custom prompt".to_string()),
        paddle_ocr_settings: Some(xberg::PaddleOcrConfig::new("en").with_model_version("pp-ocrv5")),
        backend_options: Some(serde_json::json!({"mode": "fast"})),
        tessdata_path: Some(std::path::PathBuf::from("/opt/tessdata")),
        ..OcrConfig::default()
    }
}

#[cfg(feature = "ocr-surface")]
#[test]
fn should_keep_every_non_flag_ocr_field_when_ocr_flags_are_also_given() {
    let mut config = ExtractionConfig {
        ocr: Some(ocr_config_with_every_non_flag_field_set()),
        ..Default::default()
    };
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("tesseract".to_string()),
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(ocr.language, vec!["deu".to_string()]);

    let tesseract = ocr.tesseract_config.clone().expect("tesseract_config must survive");
    assert_eq!(
        tesseract.language,
        vec!["deu".to_string()],
        "--ocr-language must propagate into the preserved nested Tesseract config"
    );
    assert!(
        !tesseract.use_cache,
        "fields no CLI flag names must stay exactly as configured"
    );

    let thresholds = ocr.quality_thresholds.clone().expect("quality_thresholds must survive");
    assert_eq!(thresholds.max_ocr_output_fragmented_word_ratio, 0.99);

    let pipeline = ocr.pipeline.clone().expect("pipeline must survive");
    assert_eq!(pipeline.stages.len(), 1);
    assert_eq!(pipeline.stages[0].language, Some(vec!["deu".to_string()]));

    let vlm = ocr.vlm_config.clone().expect("vlm_config must survive");
    assert_eq!(vlm.model, "openai/gpt-4o");
    assert_eq!(
        ocr.vlm_fallback,
        xberg::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 }
    );
    assert_eq!(ocr.vlm_prompt.as_deref(), Some("custom prompt"));
    assert_eq!(
        ocr.paddle_ocr_settings
            .as_ref()
            .map(|paddle| paddle.model_version.as_str()),
        Some("pp-ocrv5")
    );
    assert_eq!(ocr.backend_options, Some(serde_json::json!({"mode": "fast"})));
    assert_eq!(ocr.tessdata_path, Some(std::path::PathBuf::from("/opt/tessdata")));
}

/// Guards the other half of the precedence rule: the fix must not make
/// `--config-json` win over the flag. Passes before and after the fix.
#[cfg(feature = "ocr-surface")]
#[test]
fn should_let_ocr_backend_flag_win_over_config_json_backend() {
    let mut config = config_from_json(r#"{"ocr":{"backend":"sceptre"}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("tesseract".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(ocr.backend, "tesseract");
}

/// Same guard for `--ocr-language`. Passes before and after the fix.
#[cfg(feature = "ocr-surface")]
#[test]
fn should_let_ocr_language_flag_win_over_config_json_language() {
    let mut config = config_from_json(r#"{"ocr":{"language":["fra"]}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_language: Some("deu".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(ocr.language, vec!["deu".to_string()]);
}

#[cfg(feature = "ocr-surface")]
#[test]
fn should_keep_config_json_auto_rotate_when_no_auto_rotate_flag_is_given() {
    let mut config = config_from_json(r#"{"ocr":{"auto_rotate":true}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ocr_backend: Some("tesseract".to_string()),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert!(
        ocr.auto_rotate == Some(true),
        "auto_rotate came from --config-json, and no --ocr-auto-rotate flag was given"
    );
}

#[cfg(feature = "ocr-surface")]
#[test]
fn should_keep_config_json_language_when_only_the_ocr_flag_is_given() {
    let mut config = config_from_json(r#"{"ocr":{"language":["deu","fra"]}}"#);
    let overrides = ExtractionOverrides {
        ocr: Some(true),
        ..default_overrides()
    };

    overrides.apply(&mut config);

    let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
    assert_eq!(ocr.language, vec!["deu".to_string(), "fra".to_string()]);
}
