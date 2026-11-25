use benchmark_harness::aggregate::{NewConsolidatedResults, aggregate_new_format};
use benchmark_harness::system_load::SystemLoad;
use benchmark_harness::types::{
    BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat, PdfMetadata, PerformanceMetrics,
    QualityMetrics, ResourceMeasurementScope, TimingRegime,
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
            baseline_memory_bytes: 0,
            peak_memory_bytes: 100_000_000,
            peak_memory_delta_bytes: 100_000_000,
            avg_cpu_percent: 50.0,
            cpu_seconds: 50.0,
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
        framework_capabilities: FrameworkCapabilities {
            timing_regime: TimingRegime::ColdProcess,
            resource_measurement_scope: ResourceMeasurementScope::IsolatedProcess,
            ..FrameworkCapabilities::default()
        },
        pdf_metadata: None,
        ocr_status: if ocr { OcrStatus::Used } else { OcrStatus::NotUsed },
        extracted_text: None,
        system_load: None,
    }
}

#[test]
fn test_schema_version_2_12_0() {
    let results = vec![make_benchmark_result(
        "xberg-markdown-baseline",
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
            reading_order_score: None,
        }),
    )];

    let aggregated = aggregate_new_format(&results);
    assert_eq!(aggregated.schema_version, "2.12.0");
}

#[test]
fn test_per_fixture_results_populated() {
    let results = vec![
        make_benchmark_result(
            "xberg-markdown-baseline",
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
                reading_order_score: None,
            }),
        ),
        make_benchmark_result(
            "xberg-markdown-baseline",
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
                reading_order_score: None,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    assert!(!aggregated.per_fixture_results.is_empty());
    assert_eq!(aggregated.per_fixture_results.len(), 2);

    let fixture_ids: Vec<String> = aggregated
        .per_fixture_results
        .iter()
        .map(|r| r.fixture_id.clone())
        .collect();
    assert!(fixture_ids.contains(&"fixture_1".to_string()));
    assert!(fixture_ids.contains(&"fixture_2".to_string()));

    for row in &aggregated.per_fixture_results {
        assert_eq!(row.output_format, OutputFormat::Markdown);
    }
}

#[test]
fn test_plaintext_has_no_layout_percentiles() {
    let results = vec![
        make_benchmark_result(
            "docling",
            OutputFormat::Plaintext,
            "fixture_1.pdf",
            false,
            true,
            Some(QualityMetrics {
                f1_score_text: 0.90,
                f1_score_numeric: 0.85,
                f1_score_layout: None,
                quality_score: 0.88,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: true,
                reading_order_score: None,
            }),
        ),
        make_benchmark_result(
            "docling",
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
                reading_order_score: None,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

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
            "xberg",
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
                reading_order_score: None,
            }),
        ),
        make_benchmark_result(
            "xberg",
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
                reading_order_score: None,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    let markdown_key = aggregated.by_framework_mode.keys().find(|k| k.contains("markdown"));
    let plaintext_key = aggregated.by_framework_mode.keys().find(|k| k.contains("plaintext"));

    assert!(markdown_key.is_some(), "Expected markdown aggregation");
    assert!(plaintext_key.is_some(), "Expected plaintext aggregation");
}

#[test]
fn test_plaintext_frameworks_excluded_from_sf1_ranking() {
    let results = vec![
        make_benchmark_result(
            "xberg-markdown",
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
                reading_order_score: None,
            }),
        ),
        make_benchmark_result(
            "docling",
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
                reading_order_score: None,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    for ranked in &aggregated.comparison.pdf_sf1_ranking_markdown {
        assert!(!ranked.framework_mode.contains("docling"));
    }

    let has_markdown = aggregated
        .comparison
        .pdf_sf1_ranking_markdown
        .iter()
        .any(|r| r.framework_mode.contains("xberg-markdown"));
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
                reading_order_score: None,
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
                reading_order_score: None,
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
                reading_order_score: None,
            }),
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    let has_quality_percentiles = aggregated.by_framework_mode.values().any(|agg| {
        agg.by_file_type.values().any(|ft| {
            [ft.no_ocr.as_ref(), ft.with_ocr.as_ref()]
                .into_iter()
                .flatten()
                .any(|perf| {
                    if let Some(q) = &perf.quality {
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
fn xberg_pdf_baseline_and_layout_keep_exact_tf1_sf1_by_format_and_in_rankings() {
    let baseline = make_benchmark_result(
        "xberg-markdown-baseline",
        OutputFormat::Markdown,
        "shared.pdf",
        false,
        true,
        Some(QualityMetrics {
            f1_score_text: 0.81,
            f1_score_numeric: 0.72,
            f1_score_layout: Some(0.63),
            quality_score: 0.73,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
            reading_order_score: None,
        }),
    );
    let layout = make_benchmark_result(
        "xberg-markdown-layout",
        OutputFormat::Markdown,
        "shared.pdf",
        false,
        true,
        Some(QualityMetrics {
            f1_score_text: 0.91,
            f1_score_numeric: 0.82,
            f1_score_layout: Some(0.88),
            quality_score: 0.88,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
            reading_order_score: None,
        }),
    );

    let aggregated = aggregate_new_format(&[baseline, layout]);
    let baseline_quality = aggregated.by_framework_mode["xberg-markdown-baseline:single"].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .and_then(|performance| performance.quality.as_ref())
        .expect("baseline PDF quality");
    let layout_quality = aggregated.by_framework_mode["xberg-markdown-layout:single"].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .and_then(|performance| performance.quality.as_ref())
        .expect("layout PDF quality");
    assert_eq!(baseline_quality.f1_text_p50, 0.81);
    assert_eq!(baseline_quality.f1_layout_p50, Some(0.63));
    assert_eq!(layout_quality.f1_text_p50, 0.91);
    assert_eq!(layout_quality.f1_layout_p50, Some(0.88));

    let baseline_row = aggregated
        .per_fixture_results
        .iter()
        .find(|row| row.framework == "xberg-markdown-baseline")
        .expect("baseline fixture row");
    let layout_row = aggregated
        .per_fixture_results
        .iter()
        .find(|row| row.framework == "xberg-markdown-layout")
        .expect("layout fixture row");
    assert_eq!((baseline_row.f1_text, baseline_row.f1_layout), (Some(0.81), Some(0.63)));
    assert_eq!((layout_row.f1_text, layout_row.f1_layout), (Some(0.91), Some(0.88)));

    let tf1_ranking = &aggregated.comparison.pdf_tf1_ranking_markdown;
    assert_eq!(tf1_ranking.len(), 2);
    assert_eq!(tf1_ranking[0].framework_mode, "xberg-markdown-layout:single");
    assert_eq!(tf1_ranking[0].value, 0.91);
    assert_eq!(tf1_ranking[1].framework_mode, "xberg-markdown-baseline:single");
    assert_eq!(tf1_ranking[1].value, 0.81);

    let sf1_ranking = &aggregated.comparison.pdf_sf1_ranking_markdown;
    assert_eq!(sf1_ranking.len(), 2);
    assert_eq!(sf1_ranking[0].framework_mode, "xberg-markdown-layout:single");
    assert_eq!(sf1_ranking[0].value, 0.88);
    assert_eq!(sf1_ranking[1].framework_mode, "xberg-markdown-baseline:single");
    assert_eq!(sf1_ranking[1].value, 0.63);

    let json = serde_json::to_value(&aggregated).unwrap();
    assert!(json["per_fixture_results"][0]["f1_text"].is_number());
    assert!(json["per_fixture_results"][0]["f1_layout"].is_number());
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
    assert_eq!(no_ocr_row.unwrap().ocr, Some(false));
    assert_eq!(with_ocr_row.unwrap().ocr, Some(true));
}

#[test]
fn test_unknown_ocr_status_serializes_as_null() {
    let mut result = make_benchmark_result(
        "test-framework",
        OutputFormat::Markdown,
        "unknown.pdf",
        false,
        true,
        None,
    );
    result.ocr_status = OcrStatus::Unknown;

    let aggregated = aggregate_new_format(&[result]);
    assert_eq!(aggregated.per_fixture_results[0].ocr, None);
    let serialized = serde_json::to_value(&aggregated).unwrap();
    assert!(serialized["per_fixture_results"][0]["ocr"].is_null());
}

#[test]
fn test_empty_results() {
    let results = vec![];
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.schema_version, "2.12.0");
    assert!(aggregated.by_framework_mode.is_empty());
    assert!(aggregated.per_fixture_results.is_empty());
    assert_eq!(aggregated.metadata.total_results, 0);
    assert!(aggregated.comparison.pages_per_sec_ranking.is_empty());
    assert!(aggregated.comparison.cpu_seconds_ranking.is_empty());
    assert!(aggregated.comparison.pareto_frontier.is_empty());
}

/// Tier A comparative performance metrics (v2.7.0 additive fields): `pages_per_sec` and
/// `cpu_seconds` percentiles, and the `batch_size` dimension, must reach the public
/// `NewConsolidatedResults` schema produced by `aggregate_new_format`.
#[test]
fn test_pages_per_sec_and_cpu_seconds_populate_in_public_schema() {
    let mut result = make_benchmark_result(
        "xberg-markdown-baseline",
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
            reading_order_score: None,
        }),
    );
    result.pdf_metadata = Some(PdfMetadata {
        has_text_layer: true,
        detection_method: "pdftotext".to_string(),
        page_count: Some(5),
        ocr_enabled: false,
        text_quality_score: None,
    });
    result.metrics.cpu_seconds = 0.08;

    let aggregated = aggregate_new_format(&[result]);

    let performance = aggregated.by_framework_mode["xberg-markdown-baseline:single"]
        .overall_performance
        .as_ref()
        .expect("overall_performance must be populated");

    let pages_per_sec = performance
        .pages_per_sec
        .as_ref()
        .expect("pages_per_sec must be populated from PdfMetadata.page_count");
    assert_eq!(pages_per_sec.p50, 50.0);

    assert_eq!(performance.cpu_seconds.p50, 0.08);
    assert_eq!(performance.batch_size, Some(1));

    let serialized = serde_json::to_value(&aggregated).unwrap();
    let performance_json = &serialized["by_framework_mode"]["xberg-markdown-baseline:single"]["overall_performance"];
    assert_eq!(performance_json["pages_per_sec"]["p50"], 50.0);
    assert_eq!(performance_json["cpu_seconds"]["p50"], 0.08);
    assert_eq!(performance_json["batch_size"], 1);
}

#[test]
fn test_system_load_surfaces_as_contention_qualifier() {
    let mut result = make_benchmark_result(
        "xberg-markdown-baseline",
        OutputFormat::Markdown,
        "fixture_1.pdf",
        false,
        true,
        None,
    );
    result.system_load = Some(SystemLoad {
        load_avg_1m: 8.0,
        load_avg_5m: 8.0,
        load_avg_15m: 8.0,
        logical_cores: 4,
        physical_cores: 4,
    });

    let aggregated = aggregate_new_format(&[result]);

    let performance = aggregated.by_framework_mode["xberg-markdown-baseline:single"]
        .overall_performance
        .as_ref()
        .expect("overall_performance must be populated");
    let system_load = performance
        .system_load
        .as_ref()
        .expect("system_load must surface the captured contention snapshot");

    assert_eq!(system_load.total_sample_count, 1);
    assert_eq!(
        system_load.contended_sample_count, 1,
        "load_per_core of 2.0 (8.0 / 4 cores) exceeds the contention threshold"
    );
}

#[test]
fn test_pareto_frontier_reaches_public_schema() {
    let mut result = make_benchmark_result(
        "xberg-markdown-baseline",
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
            reading_order_score: None,
        }),
    );
    result.pdf_metadata = Some(PdfMetadata {
        has_text_layer: true,
        detection_method: "pdftotext".to_string(),
        page_count: Some(10),
        ocr_enabled: false,
        text_quality_score: None,
    });

    let aggregated = aggregate_new_format(&[result]);

    assert_eq!(aggregated.comparison.pareto_frontier.len(), 1);
    let point = &aggregated.comparison.pareto_frontier[0];
    assert_eq!(point.framework_mode, "xberg-markdown-baseline:single");
    assert_eq!(point.pages_per_sec, 100.0);
    assert_eq!(point.sf1, 0.88);
    assert_eq!(point.peak_memory_mb, 100.0);
}

fn quality(tf1: f64, sf1: Option<f64>, reading_order: Option<f64>) -> QualityMetrics {
    QualityMetrics {
        f1_score_text: tf1,
        f1_score_numeric: tf1,
        f1_score_layout: sf1,
        quality_score: tf1,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: reading_order,
    }
}

fn baseline_and_layout_run() -> Vec<BenchmarkResult> {
    let row = |framework: &str, sf1: f64, reading_order: f64| {
        make_benchmark_result(
            framework,
            OutputFormat::Markdown,
            "fixture_1.pdf",
            false,
            true,
            Some(quality(0.8, Some(sf1), Some(reading_order))),
        )
    };
    vec![
        row("xberg-markdown-baseline", 0.5, 0.6),
        row("xberg-markdown-layout", 0.7, 0.9),
        row("liteparse", 0.4, 0.5),
    ]
}

#[test]
fn test_schema_2_12_fields_are_serialized_with_expected_types() {
    let aggregated = aggregate_new_format(&baseline_and_layout_run());
    let value = serde_json::to_value(&aggregated).unwrap();

    let layout_impact = value["layout_impact"].as_array().expect("layout_impact array");
    assert_eq!(layout_impact.len(), 1);
    let impact = &layout_impact[0];
    assert_eq!(impact["layout_framework"], "xberg-markdown-layout");
    assert_eq!(impact["baseline_framework"], "xberg-markdown-baseline");
    assert_eq!(impact["paired_fixture_count"], 1);
    assert!((impact["sf1"]["delta_p50"].as_f64().unwrap() - 0.2).abs() < 1e-9);
    assert!(impact["reading_order"]["sample_count"].is_u64());
    for key in ["improved_fixture_ids", "regressed_fixture_ids", "unchanged_fixture_ids"] {
        assert!(impact[key].is_array(), "{key}");
    }
    assert_eq!(impact["improved_fixture_ids"][0], "fixture_1");
    assert!(impact["per_fixture"]["sf1"][0]["delta"].is_f64());

    let head_to_head = value["head_to_head"].as_array().expect("head_to_head array");
    assert_eq!(head_to_head.len(), 2, "each xberg variant against liteparse");
    for entry in head_to_head {
        assert_eq!(entry["competitor_framework"], "liteparse");
        assert_eq!(entry["shared_fixture_count"], 1);
        assert!(entry["xberg_only_fixture_ids"].as_array().unwrap().is_empty());
        assert!(entry["competitor_only_fixture_ids"].as_array().unwrap().is_empty());
        assert!(entry["quality_score"]["delta_p50"].is_f64());
    }

    let quality =
        &value["by_framework_mode"]["xberg-markdown-baseline:single"]["by_file_type"]["pdf"]["no_ocr"]["quality"];
    assert!((quality["reading_order_p50"].as_f64().unwrap() - 0.6).abs() < 1e-9);
    assert_eq!(quality["reading_order_sample_count"], 1);

    let ranking = value["comparison"]["pdf_reading_order_ranking_markdown"]
        .as_array()
        .expect("reading-order ranking");
    assert_eq!(ranking[0]["framework_mode"], "xberg-markdown-layout:single");
}

#[test]
fn test_empty_results_carry_empty_paired_sections() {
    let value = serde_json::to_value(aggregate_new_format(&[])).unwrap();

    assert_eq!(value["layout_impact"], serde_json::json!([]));
    assert_eq!(value["head_to_head"], serde_json::json!([]));
}

fn strip_keys(value: &mut serde_json::Value, keys: &[&str]) {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                map.remove(*key);
            }
            map.values_mut().for_each(|child| strip_keys(child, keys));
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|child| strip_keys(child, keys)),
        _ => {}
    }
}

#[test]
fn test_2_11_0_shaped_aggregate_still_deserializes() {
    let mut value = serde_json::to_value(aggregate_new_format(&baseline_and_layout_run())).unwrap();
    value["schema_version"] = serde_json::json!("2.11.0");
    strip_keys(
        &mut value,
        &[
            "layout_impact",
            "head_to_head",
            "pdf_reading_order_ranking_markdown",
            "pdf_reading_order_ranking_plaintext",
            "reading_order_p50",
            "reading_order_sample_count",
        ],
    );
    assert!(
        value.get("layout_impact").is_none(),
        "fixture must look like a 2.11.0 file"
    );

    let legacy: NewConsolidatedResults = serde_json::from_value(value).expect("2.11.0 aggregate deserializes");

    assert_eq!(legacy.schema_version, "2.11.0");
    assert!(legacy.layout_impact.is_empty());
    assert!(legacy.head_to_head.is_empty());
    assert!(legacy.comparison.pdf_reading_order_ranking_markdown.is_empty());
    let quality = legacy.by_framework_mode["xberg-markdown-baseline:single"].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .and_then(|bucket| bucket.quality.as_ref())
        .expect("quality percentiles");
    assert_eq!(quality.reading_order_p50, None);
    assert_eq!(quality.reading_order_sample_count, 0);
}
