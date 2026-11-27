//! Miscellaneous coverage: quality ground-truth loading edge cases, profiling-config helpers,
//! and framework-size enrichment across sync/async/batch row-name suffixes.

use crate::config::BenchmarkConfig;
use crate::fixture::FixtureManager;
use crate::registry::AdapterRegistry;
use crate::runner::BenchmarkRunner;
use crate::runner::helpers::{calculate_amplified_iterations, load_quality_ground_truth};
use crate::types::{ErrorKind, OutputFormat};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn quality_ground_truth_fails_if_file_disappears_after_fixture_load() {
    use crate::fixture::{Fixture, GroundTruth};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("fixture.json");
    let ground_truth_path = temp_dir.path().join("ground_truth.txt");
    std::fs::write(&ground_truth_path, "expected").unwrap();
    let fixture = Fixture {
        document: PathBuf::from("document.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("ground_truth.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "manual".to_string(),
        }),
    };
    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
    let mut fixtures = FixtureManager::new();
    fixtures.load_fixture(&fixture_path).unwrap();
    std::fs::remove_file(ground_truth_path).unwrap();

    let error = load_quality_ground_truth(&fixtures).unwrap_err();
    assert!(error.to_string().contains("failed to read requested text ground truth"));
}

#[test]
fn quality_ground_truth_uses_markdown_when_text_is_not_supplied() {
    use crate::fixture::{Fixture, GroundTruth};

    let temp_dir = tempfile::TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("fixture.json");
    std::fs::write(temp_dir.path().join("ground_truth.md"), "# Expected").unwrap();
    let fixture = Fixture {
        document: PathBuf::from("document.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: None,
            markdown_file: Some(PathBuf::from("ground_truth.md")),
            fields_json: None,
            formulas_json: None,
            source: "markdown_file".to_string(),
        }),
    };
    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
    let mut fixtures = FixtureManager::new();
    fixtures.load_fixture(&fixture_path).unwrap();

    let (text, markdown) = load_quality_ground_truth(&fixtures).unwrap();
    let document_path = temp_dir.path().join("document.pdf");
    assert_eq!(text.get(&document_path).map(String::as_str), Some("# Expected"));
    assert_eq!(markdown.get(&document_path).map(String::as_str), Some("# Expected"));
}

#[test]
fn test_calculate_amplified_iterations() {
    assert_eq!(calculate_amplified_iterations(100, 1000), 10);
    assert_eq!(calculate_amplified_iterations(500, 1000), 2);
    assert_eq!(calculate_amplified_iterations(2000, 1000), 1);
    assert_eq!(calculate_amplified_iterations(0, 1000), 1);
    assert_eq!(calculate_amplified_iterations(1, 1000), 1000);
}

#[test]
fn test_profiling_config_optimal_frequency() {
    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(50), 500);
    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(99), 500);

    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(500), 500);

    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(1000), 500);

    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(5000), 100);

    assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(10000), 100);
}

#[test]
fn test_profiling_config_validation() {
    let mut config = crate::ProfilingConfig::default();

    assert!(config.validate().is_ok());

    config.sampling_frequency = 50;
    assert!(config.validate().is_err());

    config.sampling_frequency = 20000;
    assert!(config.validate().is_err());

    config.sampling_frequency = 1000;
    assert!(config.validate().is_ok());

    config.batch_size = 0;
    assert!(config.validate().is_err());

    config.batch_size = 10;
    assert!(config.validate().is_ok());

    config.sample_count_threshold = 0;
    assert!(config.validate().is_err());

    config.sample_count_threshold = 500;
    assert!(config.validate().is_ok());
}

#[test]
fn test_framework_size_enrichment_with_suffix_stripping() {
    use crate::types::{BenchmarkResult, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
    use std::time::Duration;

    let config = BenchmarkConfig::default();
    let registry = AdapterRegistry::new();
    let runner = BenchmarkRunner::new(config, registry);

    let mut result_sync = BenchmarkResult {
        framework: "xberg-python-sync".to_string(),
        output_format: OutputFormat::Markdown,
        file_path: PathBuf::from("/test/file.pdf"),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_millis(100),
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
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        fixture_family: None,
    };

    let mut result_async = BenchmarkResult {
        framework: "xberg-python-async".to_string(),
        output_format: OutputFormat::Markdown,
        file_path: PathBuf::from("/test/file.pdf"),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_millis(100),
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
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        fixture_family: None,
    };

    let mut result_batch = BenchmarkResult {
        framework: "xberg-python-batch".to_string(),
        output_format: OutputFormat::Markdown,
        file_path: PathBuf::from("/test/file.pdf"),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_millis(100),
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
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        fixture_family: None,
    };

    assert!(result_sync.framework_capabilities.installation_size.is_none());
    assert!(result_async.framework_capabilities.installation_size.is_none());
    assert!(result_batch.framework_capabilities.installation_size.is_none());

    runner.enrich_with_framework_size(&mut result_sync);
    runner.enrich_with_framework_size(&mut result_async);
    runner.enrich_with_framework_size(&mut result_batch);

    if let Some(size_info) = &result_sync.framework_capabilities.installation_size {
        assert!(size_info.size_bytes > 0, "Size should be positive");
        assert!(!size_info.method.is_empty(), "Method should be set");
        assert!(!size_info.description.is_empty(), "Description should be set");

        assert!(result_async.framework_capabilities.installation_size.is_some());
        assert!(result_batch.framework_capabilities.installation_size.is_some());

        let async_size = result_async.framework_capabilities.installation_size.as_ref().unwrap();
        let batch_size = result_batch.framework_capabilities.installation_size.as_ref().unwrap();

        assert_eq!(async_size.size_bytes, size_info.size_bytes);
        assert_eq!(batch_size.size_bytes, size_info.size_bytes);
    }
}
