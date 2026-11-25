//! Quality-ranking, framework-fault-vs-infrastructure-failure, and failure-summary tests.

use super::support::{create_test_result, ranking_value};
use super::*;
use crate::types::{ErrorKind, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
use std::path::PathBuf;
use std::time::Duration;

/// Regression test for the plaintext/markdown quality-ranking pooling bug.
///
/// A plaintext-only framework (scored with no structural/SF1 term) must never be pooled
/// into a markdown (layout-inclusive) quality ranking alongside frameworks that carry a
/// structural penalty. See module-level docs ("Output format support") for the contract.
#[test]
fn test_quality_ranking_never_pools_plaintext_into_markdown() {
    let mut markdown_result = create_test_result(
        "xberg-markdown-baseline",
        "pdf",
        OcrStatus::NotUsed,
        100,
        1_000_000.0,
        10_000_000,
    );
    markdown_result.output_format = OutputFormat::Markdown;
    markdown_result.quality = Some(crate::types::QualityMetrics {
        f1_score_text: 0.7,
        f1_score_numeric: 0.7,
        f1_score_layout: Some(0.5),
        quality_score: 0.5 * 0.7 + 0.2 * 0.7 + 0.3 * 0.5,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    // A plaintext-only competitor (e.g. Apache Tika): higher raw quality_score because it
    // never incurs the structural (SF1) penalty markdown frameworks carry. ~keep
    let mut plaintext_result =
        create_test_result("apache-tika", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    plaintext_result.output_format = OutputFormat::Plaintext;
    plaintext_result.quality = Some(crate::types::QualityMetrics {
        f1_score_text: 0.95,
        f1_score_numeric: 0.95,
        f1_score_layout: None,
        quality_score: 0.6 * 0.95 + 0.4 * 0.95,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    let results = vec![markdown_result, plaintext_result];
    let aggregated = aggregate_new_format(&results);

    let markdown_keys: std::collections::HashSet<&str> = aggregated
        .comparison
        .quality_ranking_markdown
        .iter()
        .map(|r| r.framework_mode.as_str())
        .collect();
    let plaintext_keys: std::collections::HashSet<&str> = aggregated
        .comparison
        .quality_ranking_plaintext
        .iter()
        .map(|r| r.framework_mode.as_str())
        .collect();

    assert!(
        !markdown_keys.iter().any(|k| k.contains("apache-tika")),
        "plaintext-only framework 'apache-tika' must never appear in the markdown \
         (layout-inclusive) quality ranking, found in: {:?}",
        markdown_keys
    );
    assert!(
        markdown_keys.iter().any(|k| k.contains("xberg-markdown-baseline")),
        "markdown framework should appear in the markdown quality ranking, found: {:?}",
        markdown_keys
    );
    assert!(
        plaintext_keys.iter().any(|k| k.contains("apache-tika")),
        "plaintext framework should appear in the plaintext quality ranking, found: {:?}",
        plaintext_keys
    );

    let pdf_markdown_keys: std::collections::HashSet<&str> = aggregated
        .comparison
        .pdf_quality_ranking_markdown
        .iter()
        .map(|r| r.framework_mode.as_str())
        .collect();
    assert!(
        !pdf_markdown_keys.iter().any(|k| k.contains("apache-tika")),
        "plaintext-only framework must never appear in pdf_quality_ranking_markdown, found: {:?}",
        pdf_markdown_keys
    );
}

#[test]
fn test_calculate_percentiles_extraction_duration_no_extraction_some_failed() {
    let result1_failed = BenchmarkResult {
        framework: "test".to_string(),
        file_path: PathBuf::from("test1.pdf"),
        file_size: 1024,
        success: false,
        error_message: Some("Error".to_string()),
        error_kind: ErrorKind::HarnessError,
        duration: Duration::from_millis(0),
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
            ..Default::default(),
        },
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
        output_format: OutputFormat::Markdown,
    };

    let result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);

    let refs = vec![&result1_failed, &result2];
    let percentiles = calculate_percentiles(&refs);

    assert!(percentiles.extraction_duration.is_none());
    // The failed sample is a HarnessError (infrastructure fault) and so is excluded from the
    // success-rate denominator: 1 success / 1 accountable sample = 100%.
    assert_eq!(percentiles.success_rate_percent, 100.0);
}

fn result_with_quality(framework: &str, file_ext: &str, quality_score: f64, success: bool) -> BenchmarkResult {
    let mut result = create_test_result(framework, file_ext, OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    result.success = success;
    if success {
        result.quality = Some(crate::types::QualityMetrics {
            f1_score_text: quality_score,
            f1_score_numeric: quality_score,
            f1_score_layout: Some(quality_score),
            quality_score,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
            reading_order_score: None,
        });
    } else {
        result.error_message = Some("extraction failed".to_string());
        result.error_kind = ErrorKind::FrameworkError;
    }
    result
}

/// Raw percentiles describe successful extractions; coverage-adjusted rankings apply the
/// framework-fault penalty exactly once. Here one 0.9 success plus one framework failure keeps
/// raw p50 at 0.9, has a 50% success rate, and ranks at 0.45.
#[test]
fn test_framework_fault_failure_penalizes_quality_and_success_rate() {
    let success = result_with_quality("fw", "pdf", 0.9, true);
    let failure = result_with_quality("fw", "pdf", 0.0, false); // helper sets FrameworkError
    assert_eq!(failure.error_kind, ErrorKind::FrameworkError);

    let refs = vec![&success, &failure];
    let percentiles = calculate_percentiles(&refs);

    assert_eq!(percentiles.success_rate_percent, 50.0);
    assert_eq!(percentiles.framework_errors, 1);
    let quality = percentiles.quality.expect("successful quality must be reported");
    assert_eq!(quality.quality_score_p50, 0.9);

    let aggregated = aggregate_new_format(&[success, failure]);
    assert!((ranking_value(&aggregated.comparison.quality_ranking_markdown, "fw") - 0.45).abs() < 1e-9);
}

/// An infrastructure failure (HarnessError / ConfigSetupError) must NOT penalize the framework:
/// it is excluded from both the success-rate denominator and the quality percentiles, so a
/// single success alongside one HarnessError still reads as 100% success and full quality.
#[test]
fn test_infra_failure_does_not_penalize_quality_or_success_rate() {
    let success = result_with_quality("fw", "pdf", 0.9, true);
    let mut infra_failure = result_with_quality("fw", "pdf", 0.0, false);
    infra_failure.error_kind = ErrorKind::HarnessError;

    let refs = vec![&success, &infra_failure];
    let percentiles = calculate_percentiles(&refs);

    assert_eq!(percentiles.success_rate_percent, 100.0);
    assert_eq!(percentiles.harness_errors, 1);
    let quality = percentiles.quality.expect("quality must reflect the successful sample");
    assert!(
        (quality.quality_score_p50 - 0.9).abs() < 1e-9,
        "infra failure must not inject a 0.0 quality sample; p50 should stay 0.9, got {}",
        quality.quality_score_p50
    );
}

/// Mirrors runner.rs's quality-scoring-loop silent-zero reclassification: a result that
/// started `success=true` / `ErrorKind::None` but scored `f1_score_text == 0.0` against a
/// non-empty ground truth is flipped to `success=false` / `ErrorKind::ZeroOverlap` before it
/// reaches aggregation. Aggregation must treat that flipped result exactly like any other
/// framework-fault failure: excluded from quality percentiles/rankings but counted against
/// coverage/success-rate stats — never pooled as a legitimate 0.0 quality sample.
#[test]
fn reclassified_zero_overlap_result_is_excluded_from_quality_and_counted_as_failure() {
    let success = result_with_quality("fw", "pdf", 0.9, true);
    let mut reclassified = create_test_result("fw", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    reclassified.success = false;
    reclassified.error_kind = ErrorKind::ZeroOverlap;
    reclassified.quality = Some(crate::types::QualityMetrics {
        f1_score_text: 0.0,
        f1_score_numeric: 0.0,
        f1_score_layout: Some(0.0),
        quality_score: 0.0,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: None,
    });

    // under zero_overlap (not empty_content — the two are now distinguishable).
    let mut counts = FailureCounts::default();
    counts.record(&reclassified);
    assert_eq!(counts.zero_overlap, 1);
    assert_eq!(counts.empty_content, 0);
    assert_eq!(counts.framework_fault_total, 1);
    assert_eq!(counts.infra_total, 0);

    let refs = vec![&success, &reclassified];
    let percentiles = calculate_percentiles(&refs);

    // Coverage/failure stats: the flipped result counts against the success rate, under
    // zero_overlap rather than empty_content.
    assert_eq!(percentiles.zero_overlap, 1);
    assert_eq!(percentiles.empty_content, 0);
    assert_eq!(percentiles.success_rate_percent, 50.0);

    // Quality percentiles: only the genuine success contributes; the flipped 0.0 sample is
    // excluded rather than pooled as a legitimate quality score.
    let quality = percentiles.quality.expect("successful quality must be reported");
    assert_eq!(quality.quality_score_p50, 0.9);

    // Quality ranking is coverage-adjusted: 0.9 * (1 accountable success / 2 accountable samples).
    let aggregated = aggregate_new_format(&[success, reclassified]);
    assert!((ranking_value(&aggregated.comparison.quality_ranking_markdown, "fw") - 0.45).abs() < 1e-9);
}

/// A layout-bearing row whose layout pass failed is the variant's own fault: it counts against
/// the success rate and the framework-fault total, never as harness infrastructure.
#[test]
fn layout_failed_result_is_a_framework_fault_failure() {
    let success = result_with_quality("fw", "pdf", 0.9, true);
    let mut failed = create_test_result("fw", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    failed.success = false;
    failed.error_kind = ErrorKind::LayoutFailed;
    failed.error_message = Some("layout detection failed".to_string());

    let mut counts = FailureCounts::default();
    counts.record(&failed);
    assert_eq!(counts.layout_failures, 1);
    assert_eq!(counts.framework_errors, 0);
    assert_eq!(counts.framework_fault_total, 1);
    assert_eq!(counts.infra_total, 0);

    let percentiles = calculate_percentiles(&[&success, &failed]);
    assert_eq!(percentiles.layout_failures, 1);
    assert_eq!(percentiles.total_sample_count, 2);
    assert_eq!(percentiles.success_rate_percent, 50.0);
}

/// The Defect-A regression test: a zero-overlap-flipped result (`success = false`,
/// `error_kind = ZeroOverlap`) must still contribute its own duration/throughput/memory
/// measurements to the group's performance percentiles, because the framework really did run
/// to completion and really did produce output in that time — only its *quality* is
/// disqualified, not its *timing*. Before the fix, `successful_performance_samples` gated on
/// raw `success`, silently dropping this row from every speed distribution and biasing a
/// garbage-producing competitor's percentiles toward looking artificially slow.
#[test]
fn zero_overlap_result_still_contributes_its_performance_sample() {
    let success = create_test_result("fw", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    let mut zero_overlap = create_test_result("fw", "pdf", OcrStatus::NotUsed, 200, 2_000_000.0, 20_000_000);
    zero_overlap.success = false;
    zero_overlap.error_kind = ErrorKind::ZeroOverlap;
    zero_overlap.error_message = Some("zero ground-truth token overlap".to_string());

    let refs = vec![&success, &zero_overlap];
    let percentiles = calculate_percentiles(&refs);

    assert_eq!(
        percentiles.performance_sample_count, 2,
        "the zero-overlap row must still count as a performance sample"
    );
    assert_eq!(
        percentiles.successful_sample_count, 1,
        "only the genuine success counts toward quality"
    );
    assert_eq!(percentiles.zero_overlap, 1);
    assert_eq!(percentiles.success_rate_percent, 50.0);

    // Duration percentiles computed over both [100ms, 200ms] via R-7 interpolation. n=2 is
    // far below MIN_SAMPLES_FOR_P95, so p95/p99 are suppressed rather than fabricated
    // (Defect S1) — only p50 is reported for a group this small.
    assert_eq!(percentiles.duration.p50, 150.0);
    assert_eq!(percentiles.duration.p95, None);
    assert_eq!(percentiles.duration.p99, None);
    assert_eq!(percentiles.duration.sample_count, 2);

    // Throughput percentiles computed over both [1.0, 2.0] MB/s via R-7 interpolation.
    assert_eq!(percentiles.throughput.p50, 1.5);
    assert_eq!(percentiles.throughput.p95, None);
    assert_eq!(percentiles.throughput.p99, None);

    // Memory percentiles computed over both [10.0, 20.0] MB via R-7 interpolation.
    assert_eq!(percentiles.memory.p50, 15.0);
}

/// The cohort failure roll-up must sum errors to the cohort total and break them out per
/// framework-mode and per file type, keeping the framework-fault vs infrastructure split.
#[test]
fn failure_summary_rolls_up_by_cause_framework_mode_and_file_type() {
    let docling_docx_fail = result_with_quality("docling", "docx", 0.0, false); // FrameworkError
    let docling_docx_ok = result_with_quality("docling", "docx", 0.9, true);
    let mut xberg_pdf_timeout = result_with_quality("xberg-markdown-baseline", "pdf", 0.0, false);
    xberg_pdf_timeout.error_kind = ErrorKind::Timeout;
    let mut xberg_pdf_infra = result_with_quality("xberg-markdown-baseline", "pdf", 0.0, false);
    xberg_pdf_infra.error_kind = ErrorKind::HarnessError;

    let results = vec![docling_docx_fail, docling_docx_ok, xberg_pdf_timeout, xberg_pdf_infra];
    let summary = build_failure_summary(&results);

    // Cohort total: 1 framework error + 1 timeout (fault), 1 harness error (infra).
    assert_eq!(summary.total.framework_errors, 1);
    assert_eq!(summary.total.timeouts, 1);
    assert_eq!(summary.total.harness_errors, 1);
    assert_eq!(summary.total.framework_fault_total, 2);
    assert_eq!(summary.total.infra_total, 1);

    // Per framework-mode: docling's single FrameworkError, xberg's timeout + harness error.
    let docling = summary.by_framework_mode.get("docling:markdown:single").unwrap();
    assert_eq!(docling.framework_fault_total, 1);
    assert_eq!(docling.infra_total, 0);
    let xberg = summary.by_framework_mode.get("xberg-markdown-baseline:single").unwrap();
    assert_eq!(xberg.timeouts, 1);
    assert_eq!(xberg.harness_errors, 1);
    assert_eq!(xberg.framework_fault_total, 1);
    assert_eq!(xberg.infra_total, 1);

    // Per file type: docx one fault, pdf one fault + one infra.
    assert_eq!(summary.by_file_type.get("docx").unwrap().framework_fault_total, 1);
    let pdf = summary.by_file_type.get("pdf").unwrap();
    assert_eq!(pdf.framework_fault_total, 1);
    assert_eq!(pdf.infra_total, 1);
}

/// Regression test for Bug A: the overall quality ranking must only compare frameworks on
/// the file types they *all* attempted (shared corpus), not on whatever subset each
/// framework happened to run.
///
/// `framework-partial` only ever attempts `pdf`, scoring high (0.9) there. `framework-full`
/// attempts `pdf` (scoring lower, 0.6) plus `json` (scoring very low, 0.1) — a file type
/// `framework-partial` never touched. Before the fix, `framework-full`'s overall mean would
/// be dragged down by `json` while `framework-partial` was judged on `pdf` alone, an
/// apples-to-oranges comparison. After the fix, both are ranked on the shared corpus (`pdf`
/// only), so `framework-partial` (0.9) correctly outranks `framework-full` (0.6) — and
/// `framework-full`'s `json` score must not appear in the shared-corpus mean at all.
#[test]
fn test_quality_ranking_restricted_to_shared_corpus() {
    let results = vec![
        result_with_quality("framework-partial", "pdf", 0.9, true),
        result_with_quality("framework-full", "pdf", 0.6, true),
        result_with_quality("framework-full", "json", 0.1, true),
    ];

    let aggregated = aggregate_new_format(&results);
    let ranking = &aggregated.comparison.quality_ranking_markdown;

    let partial = ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-partial"))
        .expect("framework-partial should be present in the shared-corpus ranking");
    let full = ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-full"))
        .expect("framework-full should be present in the shared-corpus ranking");

    assert!(
        (partial.value - 0.9).abs() < 1e-9,
        "framework-partial's shared-corpus (pdf-only) mean should be 0.9, got {}",
        partial.value
    );
    assert!(
        (full.value - 0.6).abs() < 1e-9,
        "framework-full's shared-corpus mean must only reflect pdf (0.6), not be diluted by \
         its json-only score; got {}",
        full.value
    );
    assert_eq!(
        partial.rank, 1,
        "framework-partial (0.9) should outrank framework-full (0.6) on shared pdf corpus"
    );
    assert_eq!(full.rank, 2);
}

/// Regression test for Bug B: a framework that attempted a file type but failed on every
/// sample must rank BELOW a framework that succeeded on that same file type, not be
/// silently excluded from the comparison as if it had never run at all.
///
/// `framework-ok` succeeds on all its `pdf` samples (quality 0.8). `framework-crashed`
/// attempts the same `pdf` file type but fails on every sample (mirrors docling failing
/// 100% of a PDF corpus). Before the fix, `framework-crashed`'s zero-success pdf bucket was
/// dropped entirely, so it would not appear in the ranking (or would be silently absent from
/// the comparison) despite having completely failed. After the fix, `framework-crashed`
/// contributes a quality value of 0.0 for that bucket and must rank strictly below
/// `framework-ok`.
#[test]
fn test_fully_failed_bucket_ranks_below_succeeding_framework() {
    let results = vec![
        result_with_quality("framework-ok", "pdf", 0.8, true),
        result_with_quality("framework-ok", "pdf", 0.8, true),
        result_with_quality("framework-crashed", "pdf", 0.0, false),
        result_with_quality("framework-crashed", "pdf", 0.0, false),
    ];

    let aggregated = aggregate_new_format(&results);
    let ranking = &aggregated.comparison.quality_ranking_markdown;

    let ok = ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-ok"))
        .expect("framework-ok should be present");
    let crashed = ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-crashed"))
        .expect(
            "framework-crashed must appear in the ranking (as a 0.0 contribution), not be \
             silently dropped for having zero successes",
        );

    assert!(
        crashed.value < ok.value,
        "a fully-failed framework must score below a succeeding one: crashed={}, ok={}",
        crashed.value,
        ok.value
    );
    assert!(
        (crashed.value - 0.0).abs() < 1e-9,
        "fully-failed bucket should contribute 0.0, got {}",
        crashed.value
    );
    assert!(ok.rank < crashed.rank, "framework-ok must outrank framework-crashed");

    let pdf_ranking = &aggregated.comparison.pdf_quality_ranking_markdown;
    let pdf_crashed = pdf_ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-crashed"))
        .expect("framework-crashed must appear in pdf_quality_ranking_markdown as a 0.0 entry");
    let pdf_ok = pdf_ranking
        .iter()
        .find(|r| r.framework_mode.contains("framework-ok"))
        .expect("framework-ok must appear in pdf_quality_ranking_markdown");
    assert!(pdf_ok.rank < pdf_crashed.rank);
}
