//! `calculate_percentiles`/native-batch dedup/cold-start tests.

use super::support::create_test_result;
use super::*;
use crate::aggregate::percentiles::build_percentiles;
use crate::types::{ErrorKind, OcrStatus, PerformanceMetrics};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_calculate_percentiles() {
    let results = [
        create_test_result("xberg", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000),
        create_test_result("xberg", "pdf", OcrStatus::NotUsed, 200, 2_000_000.0, 20_000_000),
        create_test_result("xberg", "pdf", OcrStatus::NotUsed, 300, 3_000_000.0, 30_000_000),
    ];

    let refs: Vec<&BenchmarkResult> = results.iter().collect();
    let percentiles = calculate_percentiles(&refs);

    assert_eq!(percentiles.successful_sample_count, 3);
    assert_eq!(percentiles.total_sample_count, 3);
    assert_eq!(percentiles.success_rate_percent, 100.0);

    // Exact values, not just truthiness: durations [100, 200, 300]ms, throughputs
    // [1.0, 2.0, 3.0]MB/s, memory [10.0, 20.0, 30.0]MB, cpu_seconds fixed at 50.0.
    assert_eq!(percentiles.duration.p50, 200.0);
    assert_eq!(percentiles.duration.sample_count, 3);
    assert_eq!(percentiles.duration.std_dev, 100.0);
    assert_eq!(percentiles.throughput.p50, 2.0);
    assert_eq!(percentiles.throughput.sample_count, 3);
    assert_eq!(percentiles.throughput.std_dev, 1.0);
    assert_eq!(percentiles.memory.p50, 20.0);
    assert_eq!(percentiles.memory.sample_count, 3);
    assert_eq!(percentiles.memory.std_dev, 10.0);
    assert_eq!(percentiles.cpu_seconds.p50, 50.0);
    assert_eq!(percentiles.cpu_seconds.sample_count, 3);
    assert_eq!(percentiles.cpu_seconds.std_dev, 0.0);

    // n=3 is far below MIN_SAMPLES_FOR_P95 (20): p95/p99 must be suppressed rather than
    // fabricated as (nearly) the maximum sample (Defect S1) — this is the anti-pattern this
    // test used to only check `p50 > 0.0` for.
    assert_eq!(percentiles.duration.p95, None);
    assert_eq!(percentiles.duration.p99, None);
    assert_eq!(percentiles.throughput.p95, None);
    assert_eq!(percentiles.throughput.p99, None);
    assert_eq!(percentiles.memory.p95, None);
    assert_eq!(percentiles.memory.p99, None);
}

#[test]
fn performance_percentiles_use_raw_iteration_samples() {
    let mut result = create_test_result("xberg", "pdf", OcrStatus::NotUsed, 20, 2_000_000.0, 20_000_000);
    result.iterations = [(1, 10, 1_000_000.0, 10_000_000), (2, 30, 3_000_000.0, 30_000_000)]
        .into_iter()
        .map(
            |(iteration, duration_ms, throughput, memory)| crate::types::IterationResult {
                iteration,
                success: Some(true),
                error_kind: Some(ErrorKind::None),
                duration: Duration::from_millis(duration_ms),
                extraction_duration: Some(Duration::from_millis(duration_ms - 1)),
                subprocess_overhead: Some(Duration::from_millis(1)),
                metrics: PerformanceMetrics {
                    peak_memory_bytes: memory,
                    peak_memory_delta_bytes: memory,
                    throughput_bytes_per_sec: throughput,
                    ..Default::default()
                },
                batch_sample_id: None,
                batch_performance_sample: None,
            },
        )
        .collect();

    let percentiles = calculate_percentiles(&[&result]);

    assert_eq!(percentiles.performance_sample_count, 2);
    assert_eq!(percentiles.duration.sample_count, 2);
    assert_eq!(percentiles.duration.p50, 20.0);
    assert_eq!(percentiles.throughput.p50, 2.0);
    assert_eq!(percentiles.memory.p50, 20.0);
    assert_eq!(
        percentiles.extraction_duration.as_ref().map(|values| values.p50),
        Some(19.0)
    );
}

#[test]
fn raw_successful_invocation_survives_a_later_failed_iteration() {
    let mut result = create_test_result("xberg", "pdf", OcrStatus::NotUsed, 20, 2_000_000.0, 20_000_000);
    result.success = false;
    result.error_kind = ErrorKind::FrameworkError;
    result.iterations = [
        (1, true, ErrorKind::None, 10),
        (2, false, ErrorKind::FrameworkError, 30),
    ]
    .into_iter()
    .map(
        |(iteration, success, error_kind, duration_ms)| crate::types::IterationResult {
            iteration,
            success: Some(success),
            error_kind: Some(error_kind),
            duration: Duration::from_millis(duration_ms),
            extraction_duration: Some(Duration::from_millis(duration_ms)),
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                throughput_bytes_per_sec: 1_000_000.0,
                ..Default::default()
            },
            batch_sample_id: None,
            batch_performance_sample: None,
        },
    )
    .collect();

    let percentiles = calculate_percentiles(&[&result]);

    assert_eq!(percentiles.successful_sample_count, 0);
    assert_eq!(percentiles.performance_sample_count, 1);
    assert_eq!(percentiles.duration.p50, 10.0);
}

/// Defect S1 boundary regression: `p95` must flip from suppressed to reported at exactly
/// `MIN_SAMPLES_FOR_P95` samples, and `p99` at exactly `MIN_SAMPLES_FOR_P99`. This pins down
/// the exact threshold rather than only exercising values comfortably above or below it.
#[test]
fn p95_p99_suppression_flips_at_exact_sample_thresholds() {
    let build = |n: usize| -> Percentiles {
        let values: Vec<f64> = (0..n).map(|i| i as f64).collect();
        build_percentiles(&values)
    };

    let just_under_p95 = build(MIN_SAMPLES_FOR_P95 - 1);
    assert_eq!(just_under_p95.sample_count, MIN_SAMPLES_FOR_P95 - 1);
    assert_eq!(just_under_p95.p95, None);

    let at_p95_threshold = build(MIN_SAMPLES_FOR_P95);
    assert_eq!(at_p95_threshold.sample_count, MIN_SAMPLES_FOR_P95);
    assert!(at_p95_threshold.p95.is_some());

    let just_under_p99 = build(MIN_SAMPLES_FOR_P99 - 1);
    assert_eq!(just_under_p99.sample_count, MIN_SAMPLES_FOR_P99 - 1);
    assert_eq!(just_under_p99.p99, None);
    // A sample count that supports p95 but not p99 must report one and suppress the other.
    assert!(just_under_p99.p95.is_some());

    let at_p99_threshold = build(MIN_SAMPLES_FOR_P99);
    assert_eq!(at_p99_threshold.sample_count, MIN_SAMPLES_FOR_P99);
    assert!(at_p99_threshold.p99.is_some());
}

#[test]
fn batch_process_metrics_are_sampled_once_while_item_durations_are_preserved() {
    let capability = crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::XbergCliExtractBatch,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: true,
    };
    let mut results = [
        create_test_result("xberg-batch", "pdf", OcrStatus::NotUsed, 100, 3_000_000.0, 10_000_000),
        create_test_result("xberg-batch", "pdf", OcrStatus::NotUsed, 900, 0.0, 90_000_000),
        create_test_result("xberg-batch", "pdf", OcrStatus::NotUsed, 1_700, 0.0, 170_000_000),
    ];
    for (index, result) in results.iter_mut().enumerate() {
        result.framework_capabilities.batch_support = true;
        result.framework_capabilities.batch_capability = Some(capability);
        result.framework_capabilities.batch_performance_sample = Some(index == 0);
        result.extraction_duration = Some(Duration::from_millis((index as u64 + 1) * 10));
    }

    let refs: Vec<&BenchmarkResult> = results.iter().collect();
    let percentiles = calculate_percentiles(&refs);

    assert_eq!(percentiles.successful_sample_count, 3);
    assert_eq!(percentiles.performance_sample_count, 1);
    assert_eq!(percentiles.duration.p50, 100.0);
    assert_eq!(percentiles.memory.p50, 10.0);
    assert_eq!(percentiles.throughput.p50, 3.0);
    assert_eq!(
        percentiles.extraction_duration.as_ref().map(|values| values.p50),
        Some(20.0)
    );
}

#[test]
fn mixed_batch_buckets_retain_one_process_sample_independent_of_input_order() {
    let build_results = |reversed: bool| {
        let mut results = vec![
            create_test_result("xberg-batch", "pdf", OcrStatus::NotUsed, 100, 3_000_000.0, 10_000_000),
            create_test_result("xberg-batch", "docx", OcrStatus::Used, 100, 3_000_000.0, 10_000_000),
        ];
        if reversed {
            results.reverse();
        }
        let capability = crate::types::BatchCapability {
            entry_point: crate::types::BatchEntryPoint::XbergCliExtractBatch,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: true,
        };
        for (index, result) in results.iter_mut().enumerate() {
            result.framework_capabilities.batch_support = true;
            result.framework_capabilities.batch_capability = Some(capability);
            result.framework_capabilities.batch_performance_sample = Some(index == 0);
            result.framework_capabilities.batch_sample_id = Some(format!("mixed-batch-{reversed}"));
        }
        results
    };

    for reversed in [false, true] {
        let aggregated = aggregate_new_format(&build_results(reversed));
        let framework = &aggregated.by_framework_mode["xberg:markdown:batch"];
        let overall = framework.overall_performance.as_ref().expect("overall process metrics");
        let pdf = framework.by_file_type["pdf"]
            .no_ocr
            .as_ref()
            .expect("PDF no-OCR metrics");
        let docx = framework.by_file_type["docx"]
            .with_ocr
            .as_ref()
            .expect("DOCX OCR metrics");

        assert_eq!(overall.performance_sample_count, 1);
        assert_eq!(pdf.performance_sample_count, 1);
        assert_eq!(docx.performance_sample_count, 1);
        assert_eq!(overall.throughput.p50, 3.0);
        assert_eq!(pdf.throughput.p50, 3.0);
        assert_eq!(docx.throughput.p50, 3.0);
        assert_eq!(aggregated.comparison.throughput_ranking[0].value, 3.0);
    }
}

/// While `PerformancePercentiles.performance_sample_count` dedupes a native batch down to
/// one process-level sample (see `batch_process_metrics_are_sampled_once_...` above),
/// `per_fixture_results` must still carry every per-document row: quality metrics,
/// `error_message`, `pdf_metadata`, and every other per-document field are only meaningful
/// per document, not per batch process. Regression-locks oracle item 7 (B2): batch
/// per-document rows are not lost, only the process-level percentile sample is deduped. ~keep
#[test]
fn batch_mode_preserves_every_per_document_row_in_per_fixture_results() {
    let capability = crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::XbergCliExtractBatch,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: true,
    };
    let mut results = Vec::new();
    for (index, name) in ["doc_a", "doc_b", "doc_c", "doc_d"].iter().enumerate() {
        let mut result = create_test_result(
            "xberg-batch",
            "pdf",
            OcrStatus::NotUsed,
            100 + index as u64,
            3_000_000.0,
            10_000_000,
        );
        result.file_path = PathBuf::from(format!("{name}.pdf"));
        result.framework_capabilities.batch_support = true;
        result.framework_capabilities.batch_capability = Some(capability);
        result.framework_capabilities.batch_performance_sample = Some(index == 0);
        result.framework_capabilities.batch_sample_id = Some("batch-of-4".to_string());
        results.push(result);
    }

    let aggregated = aggregate_new_format(&results);

    // The process-level metrics are deduped to exactly one performance sample...
    let overall = aggregated.by_framework_mode["xberg:markdown:batch"]
        .overall_performance
        .as_ref()
        .expect("overall process metrics");
    assert_eq!(overall.performance_sample_count, 1);
    assert_eq!(overall.successful_sample_count, 4);

    // ...but every per-document row survives in per_fixture_results, none deduped away.
    assert_eq!(aggregated.per_fixture_results.len(), 4);
    let fixture_ids: std::collections::HashSet<&str> = aggregated
        .per_fixture_results
        .iter()
        .map(|row| row.fixture_id.as_str())
        .collect();
    assert_eq!(
        fixture_ids,
        std::collections::HashSet::from(["doc_a", "doc_b", "doc_c", "doc_d"])
    );
}

#[test]
fn repeated_identical_semantic_batches_remain_independent_process_samples() {
    let capability = crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::DoclingJobkit,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: false,
    };
    let mut results = Vec::new();
    for (sample_id, duration, throughput, memory) in [
        ("invocation-1", 100, 1_000_000.0, 10_000_000),
        ("invocation-2", 300, 3_000_000.0, 30_000_000),
    ] {
        for sibling in 0..2 {
            let mut result =
                create_test_result("docling-batch", "pdf", OcrStatus::NotUsed, duration, throughput, memory);
            result.framework_capabilities.batch_support = true;
            result.framework_capabilities.batch_capability = Some(capability);
            result.framework_capabilities.batch_performance_sample = Some(sibling == 0);
            result.framework_capabilities.batch_sample_id = Some(sample_id.to_string());
            results.push(result);
        }
    }

    let aggregated = aggregate_new_format(&results);
    let overall = aggregated.by_framework_mode["docling:markdown:batch"]
        .overall_performance
        .as_ref()
        .expect("overall process metrics");

    assert_eq!(overall.performance_sample_count, 2);
    assert_eq!(overall.duration.p50, 200.0);
    assert_eq!(overall.throughput.p50, 2.0);
    assert_eq!(overall.memory.p50, 20.0);
}

#[test]
fn batch_capable_single_zero_throughput_is_an_explicit_process_sample() {
    let mut result = create_test_result("docling", "pdf", OcrStatus::NotUsed, 100, 0.0, 10_000_000);
    result.framework_capabilities.batch_support = true;
    result.framework_capabilities.batch_capability = Some(crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::DoclingJobkit,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: false,
    });
    result.framework_capabilities.batch_performance_sample = Some(true);

    let percentiles = calculate_percentiles(&[&result]);

    assert_eq!(percentiles.performance_sample_count, 1);
    assert_eq!(percentiles.duration.p50, 100.0);
    assert_eq!(percentiles.memory.p50, 10.0);
}

#[test]
fn legacy_batch_rows_fall_back_to_the_positive_throughput_anchor() {
    let capability = crate::types::BatchCapability {
        entry_point: crate::types::BatchEntryPoint::DoclingJobkit,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing: false,
    };
    let mut anchor = create_test_result("docling", "pdf", OcrStatus::NotUsed, 100, 3_000_000.0, 10_000_000);
    let mut sibling = create_test_result("docling", "pdf", OcrStatus::NotUsed, 100, 0.0, 10_000_000);
    for result in [&mut anchor, &mut sibling] {
        result.framework_capabilities.batch_support = true;
        result.framework_capabilities.batch_capability = Some(capability);
    }

    let percentiles = calculate_percentiles(&[&anchor, &sibling]);

    assert_eq!(percentiles.performance_sample_count, 1);
    assert_eq!(percentiles.throughput.p50, 3.0);
}

#[test]
fn test_aggregate_cold_starts() {
    let results = [
        create_test_result("xberg", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000),
        create_test_result("xberg", "pdf", OcrStatus::NotUsed, 200, 2_000_000.0, 20_000_000),
    ];

    let refs: Vec<&BenchmarkResult> = results.iter().collect();
    let cold_starts = aggregate_cold_starts(&refs);

    assert!(cold_starts.is_some());
    let cold_starts = cold_starts.unwrap();
    assert_eq!(cold_starts.sample_count, 2);
    assert!(cold_starts.p50_ms > 0.0);
}

#[test]
fn test_ocr_unknown_is_not_mislabeled() {
    let results = vec![BenchmarkResult {
        framework: "test-framework".to_string(),
        file_path: PathBuf::from("/tmp/test1.pdf"),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_millis(100),
        extraction_duration: None,
        subprocess_overhead: None,
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: 10_000_000,
            peak_memory_delta_bytes: 10_000_000,
            avg_cpu_percent: 50.0,
            cpu_seconds: 50.0,
            throughput_bytes_per_sec: 10_240.0,
            p50_memory_bytes: 8_000_000,
            p95_memory_bytes: 9_500_000,
            p99_memory_bytes: 9_900_000,
            ..Default::default()
        },
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: Some(Duration::from_millis(200)),
        file_extension: "pdf".to_string(),
        framework_capabilities: Default::default(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
        output_format: OutputFormat::Markdown,
        fixture_family: None,
    }];

    let aggregated = aggregate_new_format(&results);

    let framework_mode = aggregated
        .by_framework_mode
        .get("test-framework:markdown:single")
        .unwrap();
    let file_type = framework_mode.by_file_type.get("pdf").unwrap();
    assert!(file_type.no_ocr.is_none());
    assert!(file_type.with_ocr.is_none());
}

#[test]
fn test_failed_results_excluded_from_percentiles() {
    let results = vec![
        BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test1.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 10_000_000,
                peak_memory_delta_bytes: 10_000_000,
                avg_cpu_percent: 50.0,
                cpu_seconds: 50.0,
                throughput_bytes_per_sec: 10_240.0,
                p50_memory_bytes: 8_000_000,
                p95_memory_bytes: 9_500_000,
                p99_memory_bytes: 9_900_000,
                ..Default::default()
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: Default::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: None,
            system_load: None,
            output_format: OutputFormat::Markdown,
            fixture_family: None,
        },
        BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test2.pdf"),
            file_size: 2048,
            success: false,
            error_message: Some("Test error".to_string()),
            error_kind: ErrorKind::HarnessError,
            duration: Duration::from_secs(0),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 0,
                peak_memory_delta_bytes: 0,
                avg_cpu_percent: 0.0,
                cpu_seconds: 0.0,
                throughput_bytes_per_sec: 0.0,
                p50_memory_bytes: 0,
                p95_memory_bytes: 0,
                p99_memory_bytes: 0,
                ..Default::default()
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: Default::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: None,
            system_load: None,
            output_format: OutputFormat::Markdown,
            fixture_family: None,
        },
    ];

    let aggregated = aggregate_new_format(&results);

    let framework_mode = aggregated
        .by_framework_mode
        .get("test-framework:markdown:single")
        .unwrap();
    let file_type = framework_mode.by_file_type.get("pdf").unwrap();
    let no_ocr = file_type.no_ocr.as_ref().unwrap();

    assert_eq!(no_ocr.successful_sample_count, 1);
    assert_eq!(no_ocr.total_sample_count, 2);
    // The failed sample is a HarnessError (our infrastructure's fault), so it is excluded from
    // the success-rate denominator: 1 success / 1 accountable sample = 100%. It still shows up
    // in total_sample_count and is excluded from the performance percentiles (duration.p50).
    assert_eq!(no_ocr.success_rate_percent, 100.0);
    assert_eq!(no_ocr.duration.p50, 100.0);
}
