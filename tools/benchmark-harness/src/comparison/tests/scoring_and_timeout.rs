//! Extraction-timeout scaling and text/structural scoring tests.

use super::*;
use crate::comparison::extraction_config::{
    EXTRACTION_TIMEOUT_BASE_SECS, FORCED_OCR_TIMEOUT_BASE_SECS, compute_extraction_timeout_secs,
};
use std::path::Path;

#[test]
fn forced_ocr_uses_inference_appropriate_timeout() {
    let path = Path::new("fixture.png");

    assert_eq!(
        compute_extraction_timeout_secs(path, "png", false),
        EXTRACTION_TIMEOUT_BASE_SECS
    );
    assert_eq!(
        compute_extraction_timeout_secs(path, "png", true),
        FORCED_OCR_TIMEOUT_BASE_SECS
    );
}

#[test]
fn test_score_document_identical() {
    let text = "Hello world this is a test document";
    let (tf1, structural) = score_document(text, text, None);
    assert!(
        (tf1 - 1.0).abs() < f64::EPSILON,
        "TF1 should be 1.0 for identical text, got {tf1}"
    );
    assert!(structural.sf1.is_nan(), "SF1 should be unavailable when no markdown GT");
    assert!(
        structural.order_score.is_nan(),
        "order_score should be unavailable when no markdown GT"
    );
    assert!(
        structural.per_type_sf1.is_empty(),
        "per_type should be empty when no markdown GT"
    );
}

#[test]
fn test_score_document_no_markdown_gt() {
    let content = "Some extracted content here";
    let gt_text = "Some ground truth content here";
    let (tf1, structural) = score_document(content, gt_text, None);
    assert!(
        tf1 > 0.0 && tf1 < 1.0,
        "TF1 should be between 0 and 1 for partially matching text, got {tf1}"
    );
    assert!(structural.sf1.is_nan(), "SF1 should be unavailable when no markdown GT");
    assert!(structural.order_score.is_nan());
    assert!(structural.per_type_sf1.is_empty());
}

#[test]
fn test_score_document_empty() {
    let (tf1, structural) = score_document("", "", None);
    let _ = tf1;
    assert!(structural.sf1.is_nan());
    assert!(structural.order_score.is_nan());
    assert!(structural.per_type_sf1.is_empty());
}

#[test]
fn test_score_document_completely_different() {
    let content = "alpha bravo charlie";
    let gt_text = "delta echo foxtrot";
    let (tf1, _structural) = score_document(content, gt_text, None);
    assert!(
        (tf1 - 0.0).abs() < f64::EPSILON,
        "TF1 should be 0.0 for completely different text, got {tf1}"
    );
}

#[test]
fn score_document_excludes_leading_front_matter_from_text_and_structure() {
    let body = "# Report\n\nBody text here.\n";
    let with_front_matter = format!("---\ntitle: Complex Table Test\nmeta-generator: pandoc\n---\n\n{body}");
    let gt_text = "Report Body text here.";

    let (tf1, structural) = score_document(&with_front_matter, gt_text, Some(body));
    let (body_tf1, body_structural) = score_document(body, gt_text, Some(body));

    assert_eq!(tf1, 1.0);
    assert_eq!(tf1, body_tf1);
    assert_eq!(structural.sf1, body_structural.sf1);
}

#[test]
fn score_document_keeps_a_later_dashed_block_as_body_text() {
    let content = "Intro.\n\n---\ntitle: kept\n---\n";
    let (tf1, _) = score_document(content, "Intro. title: kept", None);

    assert_eq!(tf1, 1.0);
}

#[test]
fn test_score_document_with_structure() {
    let content = "# Heading\n\nSome paragraph text.\n";
    let gt_markdown = "# Heading\n\nSome paragraph text.\n";
    let gt_text = "Heading Some paragraph text.";
    let (tf1, structural) = score_document(content, gt_text, Some(gt_markdown));
    assert!(tf1 > 0.0, "TF1 should be positive, got {tf1}");
    assert!(
        structural.sf1 > 0.0,
        "SF1 should be positive when structural GT is provided, got {}",
        structural.sf1
    );
    assert_eq!(structural.per_type_sf1.len(), structural.per_type_precision.len());
    assert_eq!(structural.per_type_sf1.len(), structural.per_type_recall.len());
}

#[test]
fn test_pipeline_config_deterministic() {
    for pipeline in Pipeline::all_xberg() {
        let config = build_extraction_config(pipeline);
        assert_eq!(
            format!("{:?}", config.output_format),
            format!("{:?}", xberg::core::config::OutputFormat::Markdown),
            "Pipeline {:?} should produce Markdown config",
            pipeline
        );
    }
}
