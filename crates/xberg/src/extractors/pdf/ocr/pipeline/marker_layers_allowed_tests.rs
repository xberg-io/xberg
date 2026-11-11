use super::*;
use crate::core::config::{OcrConfig, OcrStrategy};

fn allowed(
    probing_hidden_layer: bool,
    mixed_route_layout_active: bool,
    ocr_strategy: &OcrStrategy,
    ocr: Option<&OcrConfig>,
) -> bool {
    marker_layers_allowed(
        AllPagesFailedPolicy::PreserveNative,
        probing_hidden_layer,
        mixed_route_layout_active,
        ocr_strategy,
        ocr,
    )
}

#[test]
fn preserve_native_policy_with_every_condition_held_offers_marker_layers() {
    let ocr = OcrConfig::default();
    assert!(allowed(false, false, &OcrStrategy::Auto, Some(&ocr)));
}

#[test]
fn return_error_policy_never_offers_marker_layers() {
    let ocr = OcrConfig::default();
    assert!(!marker_layers_allowed(
        AllPagesFailedPolicy::ReturnError,
        false,
        false,
        &OcrStrategy::Auto,
        Some(&ocr),
    ));
}

#[test]
fn probing_a_hidden_layer_never_offers_marker_layers() {
    let ocr = OcrConfig::default();
    assert!(!allowed(true, false, &OcrStrategy::Auto, Some(&ocr)));
}

#[test]
fn active_mixed_route_layout_never_offers_marker_layers() {
    let ocr = OcrConfig::default();
    assert!(!allowed(false, true, &OcrStrategy::Auto, Some(&ocr)));
}

#[test]
fn a_non_auto_ocr_strategy_never_offers_marker_layers() {
    let ocr = OcrConfig::default();
    assert!(!allowed(
        false,
        false,
        &OcrStrategy::ScannedPages { min_confidence: 0.7 },
        Some(&ocr),
    ));
}

#[test]
fn an_absent_ocr_config_never_offers_marker_layers() {
    assert!(!allowed(false, false, &OcrStrategy::Auto, None));
}

#[test]
fn a_custom_ocr_pipeline_never_offers_marker_layers() {
    let mut ocr = OcrConfig::default();
    ocr.pipeline = Some(crate::core::config::OcrPipelineConfig {
        stages: Vec::new(),
        quality_thresholds: crate::core::config::OcrQualityThresholds::default(),
    });
    assert!(!allowed(false, false, &OcrStrategy::Auto, Some(&ocr)));
}

#[test]
fn an_enabled_vlm_fallback_never_offers_marker_layers() {
    let mut ocr = OcrConfig::default();
    ocr.vlm_fallback = crate::core::config::VlmFallbackPolicy::Always;
    assert!(!allowed(false, false, &OcrStrategy::Auto, Some(&ocr)));
}
