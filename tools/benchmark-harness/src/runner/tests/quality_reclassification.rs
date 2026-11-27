//! Tests for the `run()` quality-scoring pass: zero-overlap reclassification and the cases
//! that must be left alone (genuinely empty content, a real match, an empty ground truth).

use super::support::{run_scripted_quality_case, run_scripted_quality_case_in_family};
use crate::Result;
use crate::adapter::FrameworkAdapter;
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::registry::AdapterRegistry;
use crate::runner::BenchmarkRunner;
use crate::types::{BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn zero_overlap_result_is_reclassified_as_zero_overlap_failure() {
    let result = run_scripted_quality_case("expected reference text", "totally unrelated garbage").await;

    let quality = result.quality.as_ref().expect("quality is still populated");
    assert_eq!(quality.f1_score_text, 0.0);
    assert!(!result.success, "zero-overlap result must be reclassified as a failure");
    assert_eq!(result.error_kind, ErrorKind::ZeroOverlap);
    assert!(
        result.error_message.is_some(),
        "a reclassified failure must carry an error_message (output.rs result-state invariant)"
    );
}

/// Distinguishes the two failure kinds `EmptyContent` now overlaps with in name only: a
/// genuinely-empty result (no `extracted_text` at all, as adapters report it) is never routed
/// through the quality-based reclassification loop, so its `ErrorKind::EmptyContent` (set by
/// the adapter before the quality loop ever runs) is left untouched — proving it is not the
/// same code path as the zero-overlap-garbage case covered above.
#[tokio::test]
async fn genuinely_empty_result_keeps_empty_content_and_is_not_reclassified() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("document.pdf"), b"pdf").unwrap();
    std::fs::write(temp.path().join("ground_truth.txt"), "expected reference text").unwrap();
    let fixture_path = temp.path().join("fixture.json");
    std::fs::write(
        &fixture_path,
        serde_json::json!({
            "document": "document.pdf",
            "file_type": "pdf",
            "file_size": 3,
            "ground_truth": {
                "text_file": "ground_truth.txt",
                "source": "manual"
            }
        })
        .to_string(),
    )
    .unwrap();

    struct EmptyContentAdapter;

    #[async_trait::async_trait]
    impl FrameworkAdapter for EmptyContentAdapter {
        fn name(&self) -> &str {
            "empty-content-adapter"
        }

        fn timing_regime(&self) -> crate::types::TimingRegime {
            crate::types::TimingRegime::ColdProcess
        }

        fn supports_format(&self, file_type: &str) -> bool {
            file_type == "pdf"
        }

        fn supported_output_formats(&self) -> Vec<OutputFormat> {
            vec![OutputFormat::Markdown]
        }

        async fn extract(
            &self,
            file_path: &Path,
            _timeout: Duration,
            _force_ocr: bool,
            _ocr_language: Option<&str>,
            output_format: OutputFormat,
        ) -> Result<BenchmarkResult> {
            Ok(BenchmarkResult {
                framework: self.name().to_string(),
                output_format,
                file_path: file_path.to_path_buf(),
                file_size: 1,
                success: false,
                error_message: Some("Framework returned empty content".to_string()),
                error_kind: ErrorKind::EmptyContent,
                duration: Duration::from_millis(1),
                extraction_duration: None,
                subprocess_overhead: None,
                metrics: PerformanceMetrics::default(),
                quality: None,
                iterations: vec![],
                statistics: None,
                cold_start_duration: None,
                file_extension: "pdf".to_string(),
                framework_capabilities: FrameworkCapabilities::default(),
                pdf_metadata: None,
                ocr_status: OcrStatus::NotUsed,
                extracted_text: None,
                system_load: None,
                fixture_family: None,
            })
        }
    }

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(EmptyContentAdapter)).unwrap();
    let config = BenchmarkConfig {
        benchmark_mode: BenchmarkMode::SingleFile,
        measure_quality: true,
        warmup_iterations: 0,
        benchmark_iterations: 1,
        ..Default::default()
    };
    let mut runner = BenchmarkRunner::new(config, registry);
    runner.load_fixtures(&fixture_path).unwrap();

    let mut results = runner.run(&["empty-content-adapter".to_string()]).await.unwrap();
    let result = results.remove(0);

    assert!(!result.success);
    assert_eq!(
        result.error_kind,
        ErrorKind::EmptyContent,
        "a genuinely empty result must not be swept into ZeroOverlap by the quality loop"
    );
    assert!(
        result.quality.is_none(),
        "quality is never computed when extracted_text is None"
    );
}

#[tokio::test]
async fn nonzero_overlap_result_is_not_reclassified() {
    let result = run_scripted_quality_case("expected reference text", "expected reference text").await;

    let quality = result.quality.as_ref().expect("quality is populated");
    assert!(quality.f1_score_text > 0.0);
    assert!(result.success, "a genuine partial/full match must stay a success");
    assert_eq!(result.error_kind, ErrorKind::None);
}

#[tokio::test]
async fn empty_ground_truth_is_not_reclassified() {
    // Ground truth is empty (after trim); f1_score_text is 0.0 by definition (extracted is
    // non-empty, truth is empty), but the fixture itself carries no signal to score against,
    // so the result must not be punished as an empty-content failure.
    let result = run_scripted_quality_case("   ", "some extracted text").await;

    let quality = result.quality.as_ref().expect("quality is populated");
    assert_eq!(quality.f1_score_text, 0.0);
    assert!(
        result.success,
        "empty ground truth must never be reclassified as a failure"
    );
    assert_eq!(result.error_kind, ErrorKind::None);
}

#[tokio::test]
async fn result_carries_the_fixture_family_its_fixture_declares() {
    let result = run_scripted_quality_case_in_family("expected text", "expected text", Some("ocr_image")).await;

    assert_eq!(result.fixture_family.as_deref(), Some("ocr_image"));
}

#[tokio::test]
async fn result_has_no_fixture_family_when_its_fixture_declares_none() {
    let result = run_scripted_quality_case("expected text", "expected text").await;

    assert_eq!(result.fixture_family, None);
}
