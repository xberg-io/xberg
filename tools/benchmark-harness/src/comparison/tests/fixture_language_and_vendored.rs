//! Fixture-OCR-language application and vendored-output reading tests.

use super::*;
use crate::comparison::execution::run_pipeline;
use crate::comparison::extraction_config::{
    apply_fixture_ocr_language, finalize_timed_ocr_result_cache, materialize_implicit_ocr_config,
};
use crate::corpus::CorpusDocument;
use std::collections::HashMap;
use std::path::Path;

#[test]
fn fixture_language_applies_to_materialized_implicit_ocr_without_forcing_it() {
    let mut metadata = HashMap::new();
    metadata.insert("ocr_language".to_string(), serde_json::json!(" deu + eng "));
    let doc = CorpusDocument {
        name: "multilingual".to_string(),
        document_path: std::path::PathBuf::new(),
        file_type: "png".to_string(),
        file_size: 0,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata,
        fixture_path: std::path::PathBuf::new(),
    };

    for pipeline in [Pipeline::Baseline, Pipeline::Native] {
        let mut config = build_extraction_config(pipeline);
        materialize_implicit_ocr_config(&mut config);
        apply_fixture_ocr_language(&mut config, &doc);
        finalize_timed_ocr_result_cache(&mut config);

        let ocr = config.ocr.expect("timed extraction must materialize fallback OCR");
        assert_eq!(ocr.language, ["deu", "eng"]);
        assert_eq!(ocr.backend, "tesseract");
        assert!(
            ocr.tesseract_config.is_none() && ocr.backend_options.is_none(),
            "either field would switch xberg off its default OCR route"
        );
        assert!(!config.use_cache, "request-level use_cache=false keeps OCR cold");
        assert!(!config.force_ocr, "{} must retain fallback-only OCR", pipeline.name());
    }
}

#[test]
fn fixture_language_keeps_implicit_tesseract_pipeline_on_auto_psm() {
    let mut metadata = HashMap::new();
    metadata.insert("ocr_language".to_string(), serde_json::json!("jpn_vert"));
    let doc = CorpusDocument {
        name: "vertical-japanese".to_string(),
        document_path: std::path::PathBuf::new(),
        file_type: "jpeg".to_string(),
        file_size: 0,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata,
        fixture_path: std::path::PathBuf::new(),
    };
    let mut config = build_extraction_config(Pipeline::TesseractLayout);

    materialize_implicit_ocr_config(&mut config);
    apply_fixture_ocr_language(&mut config, &doc);
    finalize_timed_ocr_result_cache(&mut config);

    let ocr = config.ocr.expect("Tesseract pipeline must configure OCR");
    assert_eq!(ocr.language, ["jpn_vert"]);
    assert!(
        ocr.tesseract_config.is_none(),
        "xberg must pick the vertical PSM itself from the language"
    );
    assert!(!config.use_cache);
}

#[test]
fn fixture_language_refreshes_stale_implicit_stage_language() {
    // BLOCKER 2 regression: a Tesseract pipeline stage that already pins its own `language`
    // must NOT stay stale after `apply_fixture_ocr_language` runs: xberg prefers a present
    // stage language over the parent's.
    let mut metadata = HashMap::new();
    metadata.insert("ocr_language".to_string(), serde_json::json!("jpn_vert"));
    let doc = CorpusDocument {
        name: "vertical-japanese".to_string(),
        document_path: std::path::PathBuf::new(),
        file_type: "jpeg".to_string(),
        file_size: 0,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata,
        fixture_path: std::path::PathBuf::new(),
    };
    let ocr = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "pipeline": {
            "stages": [
                { "backend": "tesseract", "language": ["eng"] }
            ]
        }
    }))
    .unwrap();
    let mut config = xberg::ExtractionConfig {
        ocr: Some(ocr),
        ..Default::default()
    };

    materialize_implicit_ocr_config(&mut config);
    apply_fixture_ocr_language(&mut config, &doc);
    finalize_timed_ocr_result_cache(&mut config);

    let stages = config.ocr.unwrap().pipeline.unwrap().stages;
    assert_eq!(
        stages[0].language,
        Some(vec!["jpn_vert".to_string()]),
        "the stale per-stage language override must be refreshed to the fixture's language"
    );
    assert!(
        stages[0].tesseract_config.is_none(),
        "an implicit stage keeps xberg's automatic PSM and default route"
    );
    assert!(!config.use_cache);
}

#[test]
fn fixture_language_preserves_explicit_paddle_backend_and_model_tier() {
    let mut metadata = HashMap::new();
    metadata.insert("ocr_language".to_string(), serde_json::json!(" deu + eng "));
    let doc = CorpusDocument {
        name: "multilingual".to_string(),
        document_path: std::path::PathBuf::new(),
        file_type: "png".to_string(),
        file_size: 0,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata,
        fixture_path: std::path::PathBuf::new(),
    };
    let mut config = build_extraction_config(Pipeline::PaddleV6SmallLayout);

    materialize_implicit_ocr_config(&mut config);
    apply_fixture_ocr_language(&mut config, &doc);
    finalize_timed_ocr_result_cache(&mut config);

    let ocr = config.ocr.expect("Paddle preset must configure OCR");
    assert_eq!(ocr.language, ["deu", "eng"]);
    assert_eq!(ocr.backend, "paddleocr");
    assert_eq!(ocr.paddle_ocr_settings.expect("model identity").model_tier, "small");
}

fn create_vendored_fixture(root: &Path, content: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let fixtures_dir = root.join("fixtures");
    let fixture_path = fixtures_dir.join("example.json");
    let vendored_dir = root.join("vendored").join("docling");
    std::fs::create_dir_all(vendored_dir.join("md")).unwrap();
    std::fs::create_dir_all(vendored_dir.join("timing")).unwrap();
    std::fs::create_dir_all(&fixtures_dir).unwrap();
    std::fs::write(&fixture_path, "{}").unwrap();
    std::fs::write(vendored_dir.join("md/example.md"), content).unwrap();
    std::fs::write(vendored_dir.join("timing/example.ms"), "12.5").unwrap();
    (fixtures_dir, fixture_path)
}

#[test]
fn read_vendored_cached_resolves_fixture_directory() {
    let temp = tempfile::tempdir().unwrap();
    let (fixtures_dir, _) = create_vendored_fixture(temp.path(), "cached markdown");

    let (content, time_ms) = read_vendored_cached("example", &fixtures_dir, "docling").unwrap();

    assert_eq!(content, "cached markdown");
    assert_eq!(time_ms, 12.5);
}

#[tokio::test]
async fn comparison_includes_text_only_ground_truth() {
    let temp = tempfile::tempdir().unwrap();
    let (fixtures_dir, fixture_path) = create_vendored_fixture(temp.path(), "ground truth text");
    std::fs::write(fixtures_dir.join("document.txt"), "source document").unwrap();
    std::fs::write(fixtures_dir.join("ground-truth.txt"), "ground truth text").unwrap();
    std::fs::write(
        fixture_path,
        serde_json::json!({
            "document": "document.txt",
            "file_type": "txt",
            "file_size": 15,
            "ground_truth": {
                "text_file": "ground-truth.txt",
                "source": "manual"
            }
        })
        .to_string(),
    )
    .unwrap();
    let config = ComparisonConfig {
        fixtures_dir,
        pipelines: vec![Pipeline::Docling],
        dump_outputs: false,
        guardrails: false,
        guardrails_file: None,
        name_filter: None,
        category_filter: None,
        json_output: None,
        noise: false,
        diagnose: false,
        diagnose_threshold: 0.0,
    };

    let results = run_comparison(&config).await.unwrap();

    assert_eq!(results.len(), 1);
    assert!((results[0].results[0].tf1 - 1.0).abs() < f64::EPSILON);
    assert!(results[0].results[0].sf1.is_nan());
    assert!(results[0].results[0].order_score.is_nan());
}

#[test]
fn read_vendored_cached_resolves_fixture_file() {
    let temp = tempfile::tempdir().unwrap();
    let (_, fixture_path) = create_vendored_fixture(temp.path(), "cached markdown");

    let (content, time_ms) = read_vendored_cached("example", &fixture_path, "docling").unwrap();

    assert_eq!(content, "cached markdown");
    assert_eq!(time_ms, 12.5);
}

#[test]
fn read_vendored_cached_ignores_unrelated_nested_vendored_dir() {
    let temp = tempfile::tempdir().unwrap();
    let _fixture = create_vendored_fixture(temp.path(), "cached markdown");
    let nested_fixtures = temp.path().join("fixtures/nested");
    let nested_fixture = nested_fixtures.join("example.json");
    std::fs::create_dir_all(nested_fixtures.join("vendored/other-framework")).unwrap();
    std::fs::write(&nested_fixture, "{}").unwrap();

    for fixtures_path in [&nested_fixtures, &nested_fixture] {
        let (content, time_ms) = read_vendored_cached("example", fixtures_path, "docling").unwrap();
        assert_eq!(content, "cached markdown");
        assert_eq!(time_ms, 12.5);
    }
}

#[test]
fn read_vendored_cached_rejects_missing_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let fixtures_dir = temp.path().join("fixtures");
    std::fs::create_dir_all(temp.path().join("vendored/docling/md")).unwrap();
    std::fs::create_dir_all(&fixtures_dir).unwrap();

    let error = read_vendored_cached("missing", &fixtures_dir, "docling").unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Failed to read vendored docling output for missing")
    );
}

#[test]
fn read_vendored_cached_rejects_empty_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let (fixtures_dir, _) = create_vendored_fixture(temp.path(), "  \n");

    let error = read_vendored_cached("example", &fixtures_dir, "docling").unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Vendored docling output for example is empty")
    );
}

#[tokio::test]
async fn vendored_cache_miss_produces_nan_pipeline_scores() {
    let temp = tempfile::tempdir().unwrap();
    let fixtures_dir = temp.path().join("fixtures");
    std::fs::create_dir_all(temp.path().join("vendored/docling/md")).unwrap();
    std::fs::create_dir_all(&fixtures_dir).unwrap();
    let doc = CorpusDocument {
        name: "missing".to_string(),
        document_path: temp.path().join("missing.pdf"),
        file_type: "pdf".to_string(),
        file_size: 0,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata: HashMap::new(),
        fixture_path: fixtures_dir.join("missing.json"),
    };

    let result = run_pipeline(
        Pipeline::Docling,
        &doc,
        "ground truth",
        Some("# Ground truth"),
        &fixtures_dir,
    )
    .await;

    assert!(result.sf1.is_nan());
    assert!(result.tf1.is_nan());
    assert!(result.order_score.is_nan());
    assert!(result.time_ms.is_nan());
}
