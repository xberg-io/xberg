//! Reading-order aggregation, layout-impact and head-to-head tests.

use super::support::create_test_result;
use super::*;
use crate::aggregate::paired::baseline_framework_for;
use crate::types::{ErrorKind, OcrStatus, QualityMetrics};
use std::collections::BTreeSet;
use std::path::PathBuf;

const EPS: f64 = 1e-9;

fn quality(tf1: f64, sf1: Option<f64>, score: f64, reading_order: Option<f64>) -> QualityMetrics {
    QualityMetrics {
        f1_score_text: tf1,
        f1_score_numeric: tf1,
        f1_score_layout: sf1,
        quality_score: score,
        missing_tokens: vec![],
        extra_tokens: vec![],
        correct: false,
        reading_order_score: reading_order,
    }
}

/// A successful markdown PDF result for `fixture` with the given scores.
fn scored(framework: &str, fixture: &str, tf1: f64, sf1: Option<f64>, score: f64) -> BenchmarkResult {
    let mut result = create_test_result(framework, "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
    result.file_path = PathBuf::from(format!("{fixture}.pdf"));
    result.quality = Some(quality(tf1, sf1, score, None));
    result
}

/// A framework error as every production failure path builds it: extraction never reported OCR
/// use, so the status is `Unknown`.
fn failed(framework: &str, fixture: &str) -> BenchmarkResult {
    let mut result = scored(framework, fixture, 0.0, None, 0.0);
    result.success = false;
    result.error_kind = ErrorKind::FrameworkError;
    result.error_message = Some("boom".to_string());
    result.quality = None;
    result.ocr_status = OcrStatus::Unknown;
    result
}

/// A failure scored after extraction (empty content), which keeps the reported OCR status and so
/// lands in an OCR bucket of the per-framework aggregation.
fn failed_after_extraction(framework: &str, fixture: &str) -> BenchmarkResult {
    let mut result = scored(framework, fixture, 0.0, None, 0.0);
    result.success = false;
    result.error_kind = ErrorKind::EmptyContent;
    result.error_message = Some("empty content".to_string());
    result.quality = None;
    result
}

fn with_ocr(mut result: BenchmarkResult) -> BenchmarkResult {
    result.ocr_status = OcrStatus::Used;
    result
}

fn with_reading_order(mut result: BenchmarkResult, value: Option<f64>) -> BenchmarkResult {
    if let Some(q) = result.quality.as_mut() {
        q.reading_order_score = value;
    }
    result
}

fn with_duration_ms(mut result: BenchmarkResult, ms: u64) -> BenchmarkResult {
    result.duration = std::time::Duration::from_millis(ms);
    result
}

fn p50(value: Option<f64>) -> f64 {
    value.expect("a delta with samples carries its medians")
}

fn ids(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

const BASELINE: &str = "xberg-markdown-baseline";
const LAYOUT: &str = "xberg-markdown-layout";

#[test]
fn layout_impact_pairs_only_fixtures_where_both_succeeded() {
    let results = vec![
        scored(BASELINE, "a", 0.5, Some(0.5), 0.5),
        scored(BASELINE, "b", 0.5, Some(0.5), 0.5),
        scored(BASELINE, "c", 0.5, Some(0.5), 0.5),
        scored(LAYOUT, "a", 0.6, Some(0.7), 0.6),
        scored(LAYOUT, "b", 0.6, Some(0.7), 0.6),
        failed(LAYOUT, "c"),
    ];
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.layout_impact.len(), 1);
    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.layout_framework, LAYOUT);
    assert_eq!(impact.baseline_framework, BASELINE);
    assert_eq!(impact.paired_fixture_count, 2);
    let sf1_ids: Vec<&str> = impact.per_fixture.sf1.iter().map(|d| d.fixture_id.as_str()).collect();
    assert_eq!(sf1_ids, ["a", "b"]);
    assert_eq!(impact.sf1.as_ref().unwrap().sample_count, 2);
}

#[test]
fn layout_impact_delta_p50_is_median_of_fixture_deltas_not_difference_of_medians() {
    // baseline sf1 0.1/0.5/0.9 (median 0.5), layout 0.9/0.6/1.0 (median 0.9): the difference of
    // medians is 0.4 but the per-fixture deltas are 0.8/0.1/0.1, whose median is 0.1.
    let baselines = [("a", 0.1), ("b", 0.5), ("c", 0.9)];
    let layouts = [("a", 0.9), ("b", 0.6), ("c", 1.0)];
    let mut results = Vec::new();
    for (fixture, sf1) in baselines {
        results.push(scored(BASELINE, fixture, 0.5, Some(sf1), 0.5));
    }
    for (fixture, sf1) in layouts {
        results.push(scored(LAYOUT, fixture, 0.5, Some(sf1), 0.5));
    }
    let aggregated = aggregate_new_format(&results);

    let sf1 = aggregated.layout_impact[0].sf1.as_ref().expect("markdown carries sf1");
    assert!((p50(sf1.baseline_p50) - 0.5).abs() < EPS);
    assert!((p50(sf1.layout_p50) - 0.9).abs() < EPS);
    assert!((p50(sf1.delta_p50) - 0.1).abs() < EPS, "got {:?}", sf1.delta_p50);
    assert!((p50(sf1.layout_p50) - p50(sf1.baseline_p50) - 0.4).abs() < EPS);
}

#[test]
fn layout_impact_classifies_with_half_percent_threshold() {
    let cases = [
        ("up_small", 0.504),
        ("down_small", 0.496),
        ("up_big", 0.506),
        ("down_big", 0.494),
    ];
    let mut results = Vec::new();
    for (fixture, layout_sf1) in cases {
        results.push(scored(BASELINE, fixture, 0.5, Some(0.5), 0.5));
        results.push(scored(LAYOUT, fixture, 0.5, Some(layout_sf1), 0.5));
    }
    let aggregated = aggregate_new_format(&results);
    let impact = &aggregated.layout_impact[0];

    assert_eq!(ids(&impact.improved_fixture_ids), ["up_big"]);
    assert_eq!(ids(&impact.regressed_fixture_ids), ["down_big"]);
    assert_eq!(ids(&impact.unchanged_fixture_ids), ["down_small", "up_small"]);
}

#[test]
fn layout_impact_classifies_fixtures_without_sf1_on_quality_score() {
    let mut unscored_layout = scored(LAYOUT, "no_gt", 0.5, None, 0.5);
    unscored_layout.quality = None;
    let mut unscored_baseline = scored(BASELINE, "no_gt", 0.5, None, 0.5);
    unscored_baseline.quality = None;
    let results = vec![
        scored(BASELINE, "sf1_up", 0.5, Some(0.5), 0.5),
        scored(LAYOUT, "sf1_up", 0.5, Some(0.7), 0.4),
        scored(BASELINE, "score_up", 0.5, None, 0.5),
        scored(LAYOUT, "score_up", 0.5, None, 0.6),
        scored(BASELINE, "score_down", 0.5, None, 0.5),
        scored(LAYOUT, "score_down", 0.5, None, 0.3),
        scored(BASELINE, "score_same", 0.5, None, 0.5),
        scored(LAYOUT, "score_same", 0.5, None, 0.5),
        unscored_baseline,
        unscored_layout,
    ];
    let aggregated = aggregate_new_format(&results);
    let impact = &aggregated.layout_impact[0];

    assert_eq!(impact.paired_fixture_count, 5);
    assert_eq!(impact.sf1.as_ref().unwrap().sample_count, 1);
    // sf1_up is judged on SF1 (+0.2) although its quality_score fell.
    assert_eq!(ids(&impact.improved_fixture_ids), ["score_up", "sf1_up"]);
    assert_eq!(ids(&impact.regressed_fixture_ids), ["score_down"]);
    assert_eq!(ids(&impact.unchanged_fixture_ids), ["score_same"]);
    assert_eq!(ids(&impact.unscored_fixture_ids), ["no_gt"]);
    let classified = impact.improved_fixture_ids.len()
        + impact.regressed_fixture_ids.len()
        + impact.unchanged_fixture_ids.len()
        + impact.unscored_fixture_ids.len();
    assert_eq!(classified, impact.paired_fixture_count);
}

#[test]
fn layout_impact_has_no_sf1_for_plaintext_and_classifies_on_quality_score() {
    let mut results = Vec::new();
    for (framework, fixture, score) in [
        ("xberg-plaintext-baseline", "a", 0.5),
        ("xberg-plaintext-layout", "a", 0.7),
    ] {
        let mut result = scored(framework, fixture, score, None, score);
        result.output_format = OutputFormat::Plaintext;
        results.push(result);
    }
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.layout_impact.len(), 1);
    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.output_format, OutputFormat::Plaintext);
    assert!(impact.sf1.is_none());
    assert!(impact.per_fixture.sf1.is_empty());
    assert_eq!(ids(&impact.improved_fixture_ids), ["a"]);
}

#[test]
fn layout_impact_emits_zero_pair_entry_when_every_layout_row_failed() {
    // Baseline rows report `ocr: Some(false)`, failed layout rows `ocr: None`; they must still
    // share one bucket.
    let results = vec![
        scored(BASELINE, "a", 0.5, Some(0.5), 0.5),
        scored(BASELINE, "b", 0.5, Some(0.5), 0.5),
        failed(LAYOUT, "a"),
        failed(LAYOUT, "b"),
    ];
    let aggregated = aggregate_new_format(&results);
    let rows = &aggregated.per_fixture_results;
    assert!(rows.iter().filter(|r| r.framework == LAYOUT).all(|r| r.ocr.is_none()));
    assert!(
        rows.iter()
            .filter(|r| r.framework == BASELINE)
            .all(|r| r.ocr == Some(false))
    );

    assert_eq!(aggregated.layout_impact.len(), 1);
    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.paired_fixture_count, 0);
    assert_eq!(impact.ocr, Some(false), "OCR comes from the successful baseline rows");
    assert!(impact.sf1.is_none());
    assert_eq!(impact.tf1.sample_count, 0);
    assert_eq!(impact.tf1.delta_p50, None);
}

#[test]
fn layout_impact_skips_duration_deltas_for_batch_but_keeps_quality() {
    let results = vec![
        scored("xberg-markdown-baseline-batch", "a", 0.5, Some(0.5), 0.5),
        scored("xberg-markdown-layout-batch", "a", 0.6, Some(0.6), 0.6),
    ];
    let aggregated = aggregate_new_format(&results);

    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.mode, "batch");
    assert_eq!(impact.tf1.sample_count, 1);
    assert_eq!(impact.duration_ms.sample_count, 0);
    assert_eq!(impact.peak_memory_mb.sample_count, 0);
    // Unmeasured is null, not a 0.0 that reads as "no difference".
    assert_eq!(impact.duration_ms, PairedDelta::default());
    assert_eq!(impact.duration_ms.delta_p50, None);
    assert_eq!(impact.peak_memory_mb.baseline_p50, None);
    let json = serde_json::to_value(&impact.duration_ms).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"baseline_p50": null, "layout_p50": null, "delta_p50": null, "sample_count": 0})
    );
}

#[test]
fn layout_impact_reports_duration_and_memory_deltas_in_single_mode() {
    let results = vec![
        with_duration_ms(scored(BASELINE, "a", 0.5, Some(0.5), 0.5), 100),
        with_duration_ms(scored(LAYOUT, "a", 0.5, Some(0.5), 0.5), 250),
    ];
    let aggregated = aggregate_new_format(&results);

    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.duration_ms.sample_count, 1);
    assert!((p50(impact.duration_ms.delta_p50) - 150.0).abs() < EPS);
    assert_eq!(impact.peak_memory_mb.sample_count, 1);
}

#[test]
fn layout_impact_pairs_a_fixture_across_reported_ocr_use() {
    let results = vec![
        scored(BASELINE, "a", 0.5, Some(0.5), 0.5),
        scored(BASELINE, "b", 0.5, Some(0.5), 0.5),
        with_ocr(scored(LAYOUT, "a", 0.5, Some(0.6), 0.5)),
        scored(LAYOUT, "b", 0.5, Some(0.6), 0.5),
    ];
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.layout_impact.len(), 1);
    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.paired_fixture_count, 2);
    assert_eq!(impact.ocr, None, "the sides disagree on OCR use");
    assert_eq!(impact.baseline_ocr_fixture_count, 0);
    assert_eq!(impact.layout_ocr_fixture_count, 1);
}

#[test]
fn layout_impact_reports_ocr_when_every_successful_row_agrees() {
    let results = vec![
        with_ocr(scored(BASELINE, "a", 0.5, Some(0.5), 0.5)),
        with_ocr(scored(LAYOUT, "a", 0.5, Some(0.6), 0.5)),
        failed(LAYOUT, "b"),
    ];
    let aggregated = aggregate_new_format(&results);

    let impact = &aggregated.layout_impact[0];
    assert_eq!(impact.ocr, Some(true));
    assert_eq!(impact.baseline_ocr_fixture_count, 1);
    assert_eq!(impact.layout_ocr_fixture_count, 1);
}

#[test]
fn baseline_name_resolution_covers_known_layout_variants() {
    let present: BTreeSet<&str> = [
        "xberg-markdown-baseline",
        "xberg-markdown-baseline-paddle",
        "xberg-markdown-sceptre-ort",
        "xberg-markdown-layout",
        "xberg-markdown-layout-paddle",
        "xberg-markdown-sceptre-ort-layout",
        "xberg-markdown-layout-v3",
        "xberg-markdown-ocr",
    ]
    .into_iter()
    .collect();

    let resolve = |name: &str| baseline_framework_for(name, &present);
    assert_eq!(
        resolve("xberg-markdown-layout").as_deref(),
        Some("xberg-markdown-baseline")
    );
    assert_eq!(
        resolve("xberg-markdown-layout-paddle").as_deref(),
        Some("xberg-markdown-baseline-paddle")
    );
    assert_eq!(
        resolve("xberg-markdown-sceptre-ort-layout").as_deref(),
        Some("xberg-markdown-sceptre-ort")
    );
    assert_eq!(
        resolve("xberg-markdown-layout-v3").as_deref(),
        Some("xberg-markdown-baseline")
    );
    assert_eq!(resolve("xberg-markdown-ocr"), None);
}

#[test]
fn baseline_name_resolution_does_not_pair_a_variant_with_an_unrelated_baseline() {
    let present: BTreeSet<&str> = ["xberg-markdown-baseline", "xberg-markdown-layout-paddle"]
        .into_iter()
        .collect();

    assert_eq!(baseline_framework_for("xberg-markdown-layout-paddle", &present), None);
}

#[test]
fn head_to_head_excludes_fixtures_only_one_side_ran() {
    // liteparse ran 3 of the 4 fixtures xberg ran, and one fixture xberg did not.
    let mut results = vec![
        scored(BASELINE, "f1", 0.8, Some(0.8), 0.8),
        scored(BASELINE, "f2", 0.8, Some(0.8), 0.8),
        scored(BASELINE, "f3", 0.8, Some(0.8), 0.8),
        scored(BASELINE, "f4", 0.0, Some(0.0), 0.0),
        scored("liteparse", "f1", 0.6, Some(0.5), 0.6),
        scored("liteparse", "f2", 0.6, Some(0.5), 0.6),
        scored("liteparse", "f3", 0.6, Some(0.5), 0.6),
        scored("liteparse", "f5", 0.6, Some(0.5), 0.6),
    ];
    results.push(failed("liteparse", "f6"));
    results.push(scored(BASELINE, "f6", 0.8, Some(0.8), 0.8));
    let aggregated = aggregate_new_format(&results);

    let h2h = aggregated
        .head_to_head
        .iter()
        .find(|h| h.xberg_framework == BASELINE && h.competitor_framework == "liteparse")
        .expect("liteparse shares fixtures with xberg");
    assert_eq!(h2h.shared_fixture_count, 3);
    assert_eq!(ids(&h2h.xberg_only_fixture_ids), ["f4", "f6"]);
    assert_eq!(ids(&h2h.competitor_only_fixture_ids), ["f5"]);
    // f4 (xberg tf1 0.0) is not shared, so it must not pull xberg's median down.
    assert!((p50(h2h.tf1.layout_p50) - 0.8).abs() < EPS);
    assert!((p50(h2h.tf1.baseline_p50) - 0.6).abs() < EPS);
    assert!((p50(h2h.tf1.delta_p50) - 0.2).abs() < EPS);
    assert_eq!(h2h.tf1.sample_count, 3);
}

#[test]
fn head_to_head_compares_a_fixture_xberg_auto_ocred_and_the_competitor_did_not() {
    let results = vec![
        with_ocr(scored(BASELINE, "scan", 0.8, Some(0.8), 0.8)),
        scored(BASELINE, "native", 0.8, Some(0.8), 0.8),
        scored("liteparse", "scan", 0.4, Some(0.4), 0.4),
        scored("liteparse", "native", 0.6, Some(0.6), 0.6),
    ];
    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.head_to_head.len(), 1, "{:?}", aggregated.head_to_head);
    let h2h = &aggregated.head_to_head[0];
    assert_eq!(h2h.shared_fixture_count, 2);
    assert!(h2h.xberg_only_fixture_ids.is_empty());
    assert!(h2h.competitor_only_fixture_ids.is_empty());
    assert_eq!(h2h.ocr, None);
    assert_eq!(h2h.xberg_ocr_fixture_count, 1);
    assert_eq!(h2h.competitor_ocr_fixture_count, 0);
    assert_eq!(h2h.tf1.sample_count, 2);
}

#[test]
fn head_to_head_requires_a_shared_fixture_and_matching_bucket() {
    let mut plaintext_competitor = scored("tika", "f1", 0.6, None, 0.6);
    plaintext_competitor.output_format = OutputFormat::Plaintext;
    let disjoint = scored("docling", "other", 0.6, Some(0.5), 0.6);
    let results = vec![
        scored(BASELINE, "f1", 0.8, Some(0.8), 0.8),
        plaintext_competitor,
        disjoint,
    ];
    let aggregated = aggregate_new_format(&results);

    assert!(aggregated.head_to_head.is_empty(), "{:?}", aggregated.head_to_head);
}

#[test]
fn head_to_head_pairs_every_xberg_variant_with_each_competitor() {
    let results = vec![
        scored(BASELINE, "f1", 0.8, Some(0.8), 0.8),
        scored(LAYOUT, "f1", 0.9, Some(0.9), 0.9),
        scored("docling", "f1", 0.7, Some(0.7), 0.7),
        scored("liteparse", "f1", 0.6, Some(0.6), 0.6),
    ];
    let aggregated = aggregate_new_format(&results);

    let pairs: BTreeSet<(&str, &str)> = aggregated
        .head_to_head
        .iter()
        .map(|h| (h.xberg_framework.as_str(), h.competitor_framework.as_str()))
        .collect();
    assert_eq!(
        pairs,
        BTreeSet::from([
            (BASELINE, "docling"),
            (BASELINE, "liteparse"),
            (LAYOUT, "docling"),
            (LAYOUT, "liteparse"),
        ])
    );
}

#[test]
fn reading_order_p50_uses_only_results_that_carry_a_score() {
    let results = vec![
        with_reading_order(scored(BASELINE, "a", 0.5, Some(0.5), 0.5), Some(0.2)),
        with_reading_order(scored(BASELINE, "b", 0.5, Some(0.5), 0.5), None),
        with_reading_order(scored(BASELINE, "c", 0.5, Some(0.5), 0.5), Some(0.8)),
    ];
    let aggregated = aggregate_new_format(&results);

    let quality = aggregated.by_framework_mode["xberg-markdown-baseline:single"].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .and_then(|p| p.quality.as_ref())
        .expect("quality percentiles");
    assert_eq!(quality.reading_order_sample_count, 2);
    assert!((quality.reading_order_p50.unwrap() - 0.5).abs() < EPS);
}

#[test]
fn reading_order_p50_is_none_when_no_result_carries_a_score() {
    let aggregated = aggregate_new_format(&[scored(BASELINE, "a", 0.5, Some(0.5), 0.5)]);

    let quality = aggregated.by_framework_mode["xberg-markdown-baseline:single"].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .and_then(|p| p.quality.as_ref())
        .expect("quality percentiles");
    assert_eq!(quality.reading_order_sample_count, 0);
    assert_eq!(quality.reading_order_p50, None);
}

#[test]
fn reading_order_ranking_orders_frameworks_and_omits_those_without_a_score() {
    let results = vec![
        with_reading_order(scored(BASELINE, "a", 0.5, Some(0.5), 0.5), Some(0.6)),
        with_reading_order(scored(LAYOUT, "a", 0.5, Some(0.5), 0.5), Some(0.9)),
        scored("docling", "a", 0.5, Some(0.5), 0.5),
    ];
    let aggregated = aggregate_new_format(&results);

    let ranking = &aggregated.comparison.pdf_reading_order_ranking_markdown;
    let keys: Vec<&str> = ranking.iter().map(|r| r.framework_mode.as_str()).collect();
    assert_eq!(keys, ["xberg-markdown-layout:single", "xberg-markdown-baseline:single"]);
    assert!((ranking[0].value - 0.9).abs() < EPS);
    assert!(aggregated.comparison.pdf_reading_order_ranking_plaintext.is_empty());
}

#[test]
fn reading_order_ranking_penalizes_a_fully_failed_ocr_bucket() {
    // docling fails every OCR PDF after extraction (OCR status known, so the rows land in the
    // with_ocr bucket with no quality); liteparse succeeds everywhere. docling's no-OCR score alone
    // (0.8) would outrank liteparse; the failed bucket must pull it to (0.8 * 2 + 0.0 * 2) / 4.
    let mut results = Vec::new();
    for fixture in ["n1", "n2"] {
        results.push(with_reading_order(
            scored("docling", fixture, 0.5, Some(0.5), 0.5),
            Some(0.8),
        ));
        results.push(with_reading_order(
            scored("liteparse", fixture, 0.5, Some(0.5), 0.5),
            Some(0.6),
        ));
    }
    for fixture in ["s1", "s2"] {
        results.push(with_ocr(failed_after_extraction("docling", fixture)));
        results.push(with_ocr(with_reading_order(
            scored("liteparse", fixture, 0.5, Some(0.5), 0.5),
            Some(0.6),
        )));
    }
    let aggregated = aggregate_new_format(&results);

    let ranking = &aggregated.comparison.pdf_reading_order_ranking_markdown;
    let keys: Vec<&str> = ranking.iter().map(|r| r.framework_mode.as_str()).collect();
    assert_eq!(keys, ["liteparse:markdown:single", "docling:markdown:single"]);
    assert!((ranking[0].value - 0.6).abs() < EPS, "got {}", ranking[0].value);
    assert!((ranking[1].value - 0.4).abs() < EPS, "got {}", ranking[1].value);
}

#[test]
fn reading_order_ranking_omits_a_framework_whose_only_bucket_failed() {
    let results = vec![
        failed_after_extraction("docling", "a"),
        with_reading_order(scored("liteparse", "a", 0.5, Some(0.5), 0.5), Some(0.6)),
    ];
    let aggregated = aggregate_new_format(&results);

    let keys: Vec<&str> = aggregated
        .comparison
        .pdf_reading_order_ranking_markdown
        .iter()
        .map(|r| r.framework_mode.as_str())
        .collect();
    assert_eq!(keys, ["liteparse:markdown:single"]);
}

#[test]
fn reading_order_ranking_is_coverage_adjusted() {
    let results = vec![
        with_reading_order(scored(BASELINE, "a", 0.5, Some(0.5), 0.5), Some(0.8)),
        failed_after_extraction(BASELINE, "b"),
    ];
    let aggregated = aggregate_new_format(&results);

    let entry = &aggregated.comparison.pdf_reading_order_ranking_markdown[0];
    assert!((entry.value - 0.4).abs() < EPS, "got {}", entry.value);
}
