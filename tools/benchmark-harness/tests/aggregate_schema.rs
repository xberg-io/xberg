use benchmark_harness::aggregate::aggregate_new_format;
use benchmark_harness::types::{
    BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics, QualityMetrics,
};
use std::path::PathBuf;
use std::time::Duration;

fn make_benchmark_result(
    framework: &str,
    output_format: OutputFormat,
    file_name: &str,
    ocr: bool,
    success: bool,
    quality: Option<QualityMetrics>,
) -> BenchmarkResult {
    BenchmarkResult {
        framework: framework.to_string(),
        output_format,
        file_path: PathBuf::from(file_name),
        file_size: 10240,
        success,
        error_message: if success { None } else { Some("test error".to_string()) },
        error_kind: if success {
            ErrorKind::None
        } else {
            ErrorKind::FrameworkError
        },
        duration: Duration::from_millis(100),
        extraction_duration: Some(Duration::from_millis(80)),
        subprocess_overhead: Some(Duration::from_millis(20)),
        metrics: PerformanceMetrics {
            peak_memory_bytes: 100_000_000,
            avg_cpu_percent: 50.0,
            throughput_bytes_per_sec: 102_400.0,
            p50_memory_bytes: 90_000_000,
            p95_memory_bytes: 95_000_000,
            p99_memory_bytes: 99_000_000,
        },
        quality,
        iterations: vec![],
        statistics: None,
        cold_start_duration: Some(Duration::from_millis(500)),
        file_extension: "pdf".to_string(),
        framework_capabilities: FrameworkCapabilities::default(),
        pdf_metadata: None,
        ocr_status: if ocr { OcrStatus::Used } else { OcrStatus::NotUsed },
        extracted_text: None,
    }
}

#[test]
fn test_schema_version_2_3_0() {
    let results = vec![make_benchmark_result(
        "kreuzberg-rust",
        OutputFormat::Markdown,
        "test.pdf",
        false,
        true,
        Some(QualityMetrics {
            f1_score_text: 0.95,
            f1_score_numeric: 0.90,
            f1_score_layout: Some(0.88),
            quality_score: 0.91,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: true,
        }),
    )];

    let aggregated = aggregate_new_format(&results);
    assert_eq!(aggregated.schema_version, "2.3.0");
}

#[test]
fn test_per_fixture_results_populated() {
    let results = vec![
        make_benchmark_result(
            "kreuzberg-rust",
            OutputFormat::Markdown,
            "fixture_1.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.95,
                f1_score_numeric: 0.90,
                f1_score_layout: Some(0.88),
                quality_score: 0.91,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
        make_benchmark_result(
            "kreuzberg-rust",
            OutputFormat::Markdown,
            "fixture_2.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.92,
                f1_score_numeric: 0.88,
                f1_score_layout: Some(0.85),
                quality_score: 0.88,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    assert!(!aggregated.per_fixture_results.is_empty());
    assert_eq!(aggregated.per_fixture_results.len(), 2);

    // Check that fixture_id is correctly extracted from file path
    let fixture_ids: Vec<String> = aggregated
        .per_fixture_results
        .iter()
        .map(|r| r.fixture_id.clone())
        .collect();
    assert!(fixture_ids.contains(&"fixture_1".to_string()));
    assert!(fixture_ids.contains(&"fixture_2".to_string()));

    // Check that output_format is preserved
    for row in &aggregated.per_fixture_results {
        assert_eq!(row.output_format, OutputFormat::Markdown);
    }
}

#[test]
fn test_plaintext_has_no_layout_percentiles() {
    let results = vec![
        make_benchmark_result(
            "pdfplumber",
            OutputFormat::Plaintext,
            "fixture_1.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.90,
                f1_score_numeric: 0.85,
                f1_score_layout: None, // Plaintext mode has no layout
                quality_score: 0.88,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
        make_benchmark_result(
            "pdfplumber",
            OutputFormat::Plaintext,
            "fixture_2.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.91,
                f1_score_numeric: 0.86,
                f1_score_layout: None,
                quality_score: 0.89,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    // Find the plaintext aggregation
    let plaintext_key = aggregated
        .by_framework_mode
        .keys()
        .find(|k| k.contains("plaintext"))
        .cloned();

    assert!(plaintext_key.is_some(), "Expected to find plaintext aggregation key");

    if let Some(key) = plaintext_key
        && let Some(agg) = aggregated.by_framework_mode.get(&key)
        && let Some(pdf_ft) = agg.by_file_type.get("pdf")
        && let Some(perf) = &pdf_ft.no_ocr
        && let Some(quality) = &perf.quality
    {
        assert_eq!(quality.f1_layout_p50, None);
        assert_eq!(quality.f1_layout_p95, None);
        assert_eq!(quality.f1_layout_p99, None);
    }
}

#[test]
fn test_output_format_in_aggregation_key() {
    let results = vec![
        make_benchmark_result(
            "kreuzberg",
            OutputFormat::Markdown,
            "test.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.95,
                f1_score_numeric: 0.90,
                f1_score_layout: Some(0.88),
                quality_score: 0.91,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
        make_benchmark_result(
            "kreuzberg",
            OutputFormat::Plaintext,
            "test.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.92,
                f1_score_numeric: 0.88,
                f1_score_layout: None,
                quality_score: 0.90,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    // Should have two separate aggregations: one for markdown, one for plaintext
    let markdown_key = aggregated.by_framework_mode.keys().find(|k| k.contains("markdown"));
    let plaintext_key = aggregated.by_framework_mode.keys().find(|k| k.contains("plaintext"));

    assert!(markdown_key.is_some(), "Expected markdown aggregation");
    assert!(plaintext_key.is_some(), "Expected plaintext aggregation");
}

#[test]
fn test_plaintext_frameworks_excluded_from_sf1_ranking() {
    let results = vec![
        // Markdown framework for PDF
        make_benchmark_result(
            "kreuzberg-markdown",
            OutputFormat::Markdown,
            "test.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.95,
                f1_score_numeric: 0.90,
                f1_score_layout: Some(0.88),
                quality_score: 0.91,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
        // Plaintext-only framework
        make_benchmark_result(
            "pdfplumber",
            OutputFormat::Plaintext,
            "test.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.92,
                f1_score_numeric: 0.88,
                f1_score_layout: None,
                quality_score: 0.90,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    // plaintext frameworks should NOT appear in pdf_sf1_ranking_markdown
    for ranked in &aggregated.comparison.pdf_sf1_ranking_markdown {
        assert!(!ranked.framework_mode.contains("pdfplumber"));
    }

    // markdown frameworks SHOULD appear in pdf_sf1_ranking_markdown
    let has_markdown = aggregated
        .comparison
        .pdf_sf1_ranking_markdown
        .iter()
        .any(|r| r.framework_mode.contains("kreuzberg-markdown"));
    assert!(has_markdown, "Expected markdown framework in SF1 ranking");
}

#[test]
fn test_quality_percentiles_all_three() {
    let results = vec![
        make_benchmark_result(
            "test-framework",
            OutputFormat::Markdown,
            "fixture_1.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.95,
                f1_score_numeric: 0.90,
                f1_score_layout: Some(0.88),
                quality_score: 0.91,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
        make_benchmark_result(
            "test-framework",
            OutputFormat::Markdown,
            "fixture_2.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.80,
                f1_score_numeric: 0.75,
                f1_score_layout: Some(0.70),
                quality_score: 0.75,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: false,
            }),
        ),
        make_benchmark_result(
            "test-framework",
            OutputFormat::Markdown,
            "fixture_3.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.92,
                f1_score_numeric: 0.87,
                f1_score_layout: Some(0.85),
                quality_score: 0.88,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    // Find the aggregation with quality metrics
    let has_quality_percentiles = aggregated.by_framework_mode.values().any(|agg| {
        agg.by_file_type.values().any(|ft| {
            [ft.no_ocr.as_ref(), ft.with_ocr.as_ref()]
                .into_iter()
                .flatten()
                .any(|perf| {
                    if let Some(q) = &perf.quality {
                        // Check that all three percentiles are present
                        q.f1_text_p50 > 0.0
                            && q.f1_text_p95 > 0.0
                            && q.f1_text_p99 >= 0.0
                            && q.quality_score_p50 > 0.0
                            && q.quality_score_p95 > 0.0
                            && q.quality_score_p99 >= 0.0
                    } else {
                        false
                    }
                })
        })
    });

    assert!(
        has_quality_percentiles,
        "Expected quality percentiles with p50, p95, p99"
    );
}

#[test]
fn test_ocr_flag_in_per_fixture() {
    let results = vec![
        make_benchmark_result(
            "test-framework",
            OutputFormat::Markdown,
            "no_ocr.pdf",
            false,
            true,
            None,
        ),
        make_benchmark_result(
            "test-framework",
            OutputFormat::Markdown,
            "with_ocr.png",
            true,
            true,
            None,
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    let no_ocr_row = aggregated.per_fixture_results.iter().find(|r| r.fixture_id == "no_ocr");
    let with_ocr_row = aggregated
        .per_fixture_results
        .iter()
        .find(|r| r.fixture_id == "with_ocr");

    assert!(no_ocr_row.is_some());
    assert!(with_ocr_row.is_some());
    assert!(!no_ocr_row.unwrap().ocr);
    assert!(with_ocr_row.unwrap().ocr);
}

#[test]
fn test_empty_results() {
    let results = vec![];
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.schema_version, "2.3.0");
    assert!(aggregated.by_framework_mode.is_empty());
    assert!(aggregated.per_fixture_results.is_empty());
    assert_eq!(aggregated.metadata.total_results, 0);
}
