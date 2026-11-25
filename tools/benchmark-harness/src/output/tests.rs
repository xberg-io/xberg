use super::*;
use crate::types::{FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics, QualityMetrics};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

fn create_benchmark_result(
    framework: &str,
    success: bool,
    duration_ms: u64,
    extraction_duration_ms: Option<u64>,
    throughput_bps: f64,
    memory_bytes: u64,
) -> BenchmarkResult {
    BenchmarkResult {
        framework: framework.to_string(),
        file_path: PathBuf::from(format!("/tmp/{}.txt", framework)),
        file_size: 1024,
        success,
        error_message: if success { None } else { Some("Test error".to_string()) },
        error_kind: if success {
            ErrorKind::None
        } else {
            ErrorKind::HarnessError
        },
        duration: Duration::from_millis(duration_ms),
        extraction_duration: extraction_duration_ms.map(Duration::from_millis),
        subprocess_overhead: extraction_duration_ms.map(|ed| Duration::from_millis(duration_ms.saturating_sub(ed))),
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: memory_bytes,
            peak_memory_delta_bytes: memory_bytes,
            avg_cpu_percent: 50.0,
            cpu_seconds: 50.0,
            throughput_bytes_per_sec: throughput_bps,
            p50_memory_bytes: memory_bytes,
            p95_memory_bytes: memory_bytes,
            p99_memory_bytes: memory_bytes,
            ..Default::default()
        },
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: "txt".to_string(),
        framework_capabilities: FrameworkCapabilities::default(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        output_format: OutputFormat::Markdown,
    }
}

#[test]
fn test_write_json() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results.json");

    let results = vec![BenchmarkResult {
        framework: "test-framework".to_string(),
        file_path: PathBuf::from("/tmp/test.txt"),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_secs(1),
        extraction_duration: None,
        subprocess_overhead: None,
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: 10_000_000,
            peak_memory_delta_bytes: 10_000_000,
            avg_cpu_percent: 50.0,
            cpu_seconds: 50.0,
            throughput_bytes_per_sec: 1024.0,
            p50_memory_bytes: 8_000_000,
            p95_memory_bytes: 9_500_000,
            p99_memory_bytes: 9_900_000,
            ..Default::default()
        },
        quality: Some(QualityMetrics {
            f1_score_text: 0.91,
            f1_score_numeric: 0.83,
            f1_score_layout: Some(0.74),
            quality_score: 0.85,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
            reading_order_score: None,
        }),
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: "txt".to_string(),
        framework_capabilities: Default::default(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        output_format: OutputFormat::Markdown,
    }];

    write_json(&results, &output_path).unwrap();

    assert!(output_path.exists());

    let contents = fs::read_to_string(&output_path).unwrap();
    let parsed: Vec<BenchmarkResult> = serde_json::from_str(&contents).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].framework, "test-framework");
    let quality = parsed[0].quality.as_ref().expect("quality metrics round-trip");
    assert_eq!(quality.f1_score_text, 0.91);
    assert_eq!(quality.f1_score_layout, Some(0.74));

    let raw: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(raw[0]["quality"]["f1_score_text"], 0.91);
    assert_eq!(raw[0]["quality"]["f1_score_layout"], 0.74);
}

#[test]
fn write_json_rejects_every_invalid_numeric_quality_contract_value() {
    let temp_dir = TempDir::new().unwrap();
    for (field, value) in [
        ("f1_score_text", f64::NAN),
        ("f1_score_numeric", -0.01),
        ("f1_score_layout", 1.01),
        ("quality_score", f64::INFINITY),
    ] {
        let output_path = temp_dir.path().join(format!("{field}.json"));
        let mut result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let mut quality = QualityMetrics {
            f1_score_text: 0.9,
            f1_score_numeric: 0.8,
            f1_score_layout: Some(0.7),
            quality_score: 0.85,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
            reading_order_score: None,
        };
        match field {
            "f1_score_text" => quality.f1_score_text = value,
            "f1_score_numeric" => quality.f1_score_numeric = value,
            "f1_score_layout" => quality.f1_score_layout = Some(value),
            "quality_score" => quality.quality_score = value,
            _ => unreachable!(),
        }
        result.quality = Some(quality);

        let error = write_json(&[result], &output_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&format!("{field} must be a finite value in [0, 1]")),
            "unexpected validation error for {field}: {error}"
        );
        assert!(!output_path.exists());
    }
}

#[test]
fn write_json_preserves_historical_plaintext_sf1_without_schema_context() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results.json");
    let mut result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    result.output_format = OutputFormat::Plaintext;
    result.quality = Some(QualityMetrics {
        f1_score_text: 0.9,
        f1_score_numeric: 0.8,
        f1_score_layout: Some(0.7),
        quality_score: 0.85,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    write_json(&[result], &output_path).expect("generic writer remains backward-compatible");
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();
    assert_eq!(value[0]["quality"]["f1_score_layout"], 0.7);
}

#[test]
fn test_write_json_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("subdir/results.json");

    let results = vec![];

    write_json(&results, &output_path).unwrap();

    assert!(output_path.exists());
    assert!(output_path.parent().unwrap().exists());
}

#[test]
fn write_json_rejects_failed_result_without_error_kind() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results.json");
    let mut result = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    result.error_kind = ErrorKind::None;

    let error = write_json(&[result], &output_path).unwrap_err();

    assert!(error.to_string().contains("success=false but error_kind is None"));
    assert!(!output_path.exists());
}

#[test]
fn test_framework_stats_extraction_duration_all_present() {
    let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
    let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);
    let results = vec![&result1, &result2, &result3];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.successful, 3);
    assert!(stats.avg_extraction_duration_ms.is_some());
    assert!(stats.median_extraction_duration_ms.is_some());
    assert!(stats.p95_extraction_duration_ms.is_some());

    assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
    assert!((stats.median_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
}

#[test]
fn test_framework_stats_extraction_duration_all_none() {
    let result1 = create_benchmark_result("framework1", true, 100, None, 1_000_000.0, 10_000_000);
    let result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
    let result3 = create_benchmark_result("framework1", true, 200, None, 1_000_000.0, 10_000_000);
    let results = vec![&result1, &result2, &result3];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.successful, 3);
    assert!(stats.avg_extraction_duration_ms.is_none());
    assert!(stats.median_extraction_duration_ms.is_none());
    assert!(stats.p95_extraction_duration_ms.is_none());
}

#[test]
fn test_framework_stats_extraction_duration_mixed_some_none() {
    let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
    let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);
    let results = vec![&result1, &result2, &result3];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.successful, 3);
    assert!(stats.avg_extraction_duration_ms.is_some());
    assert!(stats.median_extraction_duration_ms.is_some());

    assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
}

#[test]
fn test_framework_stats_extraction_duration_filters_nan() {
    let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
    let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);

    let results = vec![&result1, &result2, &result3];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert!(stats.avg_extraction_duration_ms.is_some());
    assert_eq!(stats.avg_extraction_duration_ms.unwrap(), 120.0);
}

#[test]
fn test_framework_stats_extraction_duration_empty_results() {
    let results: Vec<&BenchmarkResult> = vec![];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 0);
    assert_eq!(stats.successful, 0);
    assert_eq!(stats.success_rate, 0.0);
    assert_eq!(stats.avg_duration_ms, 0.0);
    assert_eq!(stats.median_duration_ms, 0.0);
    assert_eq!(stats.p95_duration_ms, 0.0);
    assert!(stats.avg_extraction_duration_ms.is_none());
    assert!(stats.median_extraction_duration_ms.is_none());
    assert!(stats.p95_extraction_duration_ms.is_none());
}

#[test]
fn test_framework_stats_extraction_duration_only_failed_results() {
    let result1 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    let result2 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    let results = vec![&result1, &result2];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 2);
    assert_eq!(stats.successful, 0);
    assert!(stats.avg_extraction_duration_ms.is_none());
    assert!(stats.median_extraction_duration_ms.is_none());
    assert!(stats.p95_extraction_duration_ms.is_none());
}

#[test]
fn test_framework_stats_extraction_duration_single_value() {
    let result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let results = vec![&result];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 1);
    assert_eq!(stats.successful, 1);
    assert_eq!(stats.avg_extraction_duration_ms.unwrap(), 80.0);
    assert_eq!(stats.median_extraction_duration_ms.unwrap(), 80.0);
    assert_eq!(stats.p95_extraction_duration_ms.unwrap(), 80.0);
}

#[test]
fn test_framework_stats_success_rate_with_extraction_duration() {
    let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
    let result3 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    let results = vec![&result1, &result2, &result3];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.success_rate, 1.0);

    assert!(stats.avg_extraction_duration_ms.is_some());
    assert!((stats.avg_extraction_duration_ms.unwrap() - 100.0).abs() < 0.1);
}

#[test]
fn framework_stats_success_rate_excludes_infrastructure_failures() {
    let success = create_benchmark_result("framework1", true, 100, None, 1_000_000.0, 10_000_000);
    let infrastructure_failure = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    let mut framework_failure = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
    framework_failure.error_kind = ErrorKind::FrameworkError;
    let results = vec![&success, &infrastructure_failure, &framework_failure];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 3);
    assert_eq!(stats.successful, 1);
    assert_eq!(stats.harness_errors, 1);
    assert_eq!(stats.framework_errors, 1);
    assert_eq!(stats.success_rate, 0.5);
}

#[test]
fn test_framework_stats_does_not_divide_batch_throughput_anchor_by_cardinality() {
    let capability = crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::DoclingJobkit,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: false,
    };
    let mut anchor = create_benchmark_result("framework1", true, 100, Some(10), 3_000_000.0, 10_000_000);
    let mut sibling1 = create_benchmark_result("framework1", true, 900, Some(20), 0.0, 90_000_000);
    let mut sibling2 = create_benchmark_result("framework1", true, 1_700, Some(30), 0.0, 170_000_000);
    for (index, result) in [&mut anchor, &mut sibling1, &mut sibling2].into_iter().enumerate() {
        result.framework_capabilities.batch_support = true;
        result.framework_capabilities.batch_capability = Some(capability);
        result.framework_capabilities.batch_performance_sample = Some(index == 0);
    }
    let results = vec![&anchor, &sibling1, &sibling2];

    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.performance_samples, 1);
    assert_eq!(stats.avg_throughput_mbps, 3.0);
    assert_eq!(stats.avg_duration_ms, 100.0);
    assert_eq!(stats.avg_peak_memory_mb, 10.0);
    assert_eq!(stats.avg_extraction_duration_ms, Some(20.0));
}

#[test]
fn test_framework_stats_large_number_extraction_durations() {
    let mut results = vec![];
    for i in 1..=100 {
        results.push(create_benchmark_result(
            "framework1",
            true,
            i * 10,
            Some(i * 8),
            1_000_000.0,
            10_000_000,
        ));
    }

    let result_refs: Vec<&BenchmarkResult> = results.iter().collect();
    let stats = calculate_framework_stats(&result_refs);

    assert_eq!(stats.count, 100);
    assert_eq!(stats.successful, 100);

    let expected_avg = 8.0 * (1..=100).sum::<u64>() as f64 / 100.0;
    assert!((stats.avg_extraction_duration_ms.unwrap() - expected_avg).abs() < 1.0);

    assert!(stats.median_extraction_duration_ms.is_some());
    assert!(stats.p95_extraction_duration_ms.is_some());
}

#[test]
fn test_analyze_by_extension_with_extraction_duration() {
    let results = vec![
        create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000),
        create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000),
    ];

    let report = analyze_by_extension(&results);

    assert!(report.by_extension.contains_key("txt"));
    let ext_analysis = &report.by_extension["txt"];
    assert!(ext_analysis.framework_stats.contains_key("framework1"));

    let framework_stats = &ext_analysis.framework_stats["framework1"];
    assert!(framework_stats.avg_extraction_duration_ms.is_some());
    assert!(framework_stats.median_extraction_duration_ms.is_some());
    assert!(framework_stats.p95_extraction_duration_ms.is_some());
}

#[test]
fn test_analyze_by_extension_mixed_extraction_duration() {
    let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    result1.file_extension = "pdf".to_string();

    let mut result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
    result2.file_extension = "pdf".to_string();

    let results = vec![result1, result2];

    let report = analyze_by_extension(&results);

    assert!(report.by_extension.contains_key("pdf"));
    let ext_analysis = &report.by_extension["pdf"];
    let framework_stats = &ext_analysis.framework_stats["framework1"];

    assert!(framework_stats.avg_extraction_duration_ms.is_some());
    assert_eq!(framework_stats.avg_extraction_duration_ms.unwrap(), 80.0);
}

#[test]
fn test_framework_stats_quality_absent_when_no_quality_metrics() {
    let result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    let results = vec![&result];

    let stats = calculate_framework_stats(&results);

    assert!(stats.avg_f1_text.is_none());
    assert!(stats.avg_f1_numeric.is_none());
    assert!(stats.avg_f1_layout.is_none());
    assert!(stats.avg_quality_score.is_none());
}

#[test]
fn test_framework_stats_preserves_mean_tf1_and_sf1() {
    let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    result1.quality = Some(QualityMetrics {
        f1_score_text: 0.80,
        f1_score_numeric: 0.90,
        f1_score_layout: Some(0.60),
        quality_score: 0.75,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    let mut result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
    result2.quality = Some(QualityMetrics {
        f1_score_text: 0.90,
        f1_score_numeric: 0.95,
        f1_score_layout: Some(0.70),
        quality_score: 0.85,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: true,
        reading_order_score: None,
    });

    let results = vec![&result1, &result2];
    let stats = calculate_framework_stats(&results);

    assert!((stats.avg_f1_text.unwrap() - 0.85).abs() < 1e-9);
    assert!((stats.avg_f1_numeric.unwrap() - 0.925).abs() < 1e-9);
    assert!((stats.avg_f1_layout.unwrap() - 0.65).abs() < 1e-9);
    assert!((stats.avg_quality_score.unwrap() - 0.80).abs() < 1e-9);
}

#[test]
fn test_framework_stats_layout_none_when_no_result_reports_it() {
    let mut result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    result.quality = Some(QualityMetrics {
        f1_score_text: 0.80,
        f1_score_numeric: 0.90,
        f1_score_layout: None,
        quality_score: 0.75,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    let results = vec![&result];
    let stats = calculate_framework_stats(&results);

    assert!(stats.avg_f1_text.is_some());
    assert!(stats.avg_f1_layout.is_none());
}

#[test]
fn test_framework_stats_quality_excludes_failed_results() {
    let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
    result1.quality = Some(QualityMetrics {
        f1_score_text: 0.80,
        f1_score_numeric: 0.90,
        f1_score_layout: Some(0.60),
        quality_score: 0.75,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    // Failed result: create_benchmark_result forces quality to None for failures anyway,
    // but assert explicitly that it never contributes to the mean. ~keep
    let result2 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);

    let results = vec![&result1, &result2];
    let stats = calculate_framework_stats(&results);

    assert_eq!(stats.count, 2);
    assert_eq!(stats.successful, 1);
    assert!((stats.avg_f1_text.unwrap() - 0.80).abs() < 1e-9);
}
