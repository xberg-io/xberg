//! Cross-framework comparison rankings: throughput/memory/quality/pages-per-sec/cpu-seconds
//! rankings, deltas-vs-baseline, the Pareto frontier, and pinned-cohort scoping.

use super::percentiles::{parse_aggregate_key, weighted_avg};
use super::types::{
    ComparisonData, DeltaMetrics, FrameworkModeAggregation, ParetoPoint, PerformancePercentiles, RankedFramework,
    UnrankedFramework,
};
use crate::types::OutputFormat;
use std::collections::HashMap;

/// Samples for which extraction quality is attributable to the framework. Infrastructure errors
/// are excluded because they provide no evidence about framework quality. ~keep
fn accountable_sample_count(performance: &PerformancePercentiles) -> usize {
    performance.successful_sample_count
        + performance.framework_errors
        + performance.timeouts
        + performance.empty_content
        + performance.zero_overlap
        + performance.layout_failures
}

/// Convert a reported quality percentile into a ranking value that also reflects extraction
/// coverage. Percentiles remain unchanged in the aggregate schema; rankings multiply them by the
/// fraction of accountable samples that succeeded so a meaningful minority of failures cannot be
/// hidden above the median. Infrastructure failures are absent from that fraction. ~keep
fn coverage_adjusted_quality(value: f64, performance: &PerformancePercentiles) -> f64 {
    let accountable = accountable_sample_count(performance);
    if accountable == 0 {
        return f64::NAN;
    }
    value * performance.successful_sample_count as f64 / accountable as f64
}

/// Build the overall (all-file-types) quality ranking for one output format, restricted to a
/// **shared corpus** and counting fully-failed buckets against the framework.
///
/// # Semantics (Bug A: mismatched per-framework corpora)
///
/// Frameworks in real benchmark runs attempt wildly different sets of file types (e.g.
/// `liteparse` is PDF-only, `docling` never attempts `json`/`txt`, `xberg` runs the full
/// corpus). Naively weighting each framework's own quality mean by whatever file types *it*
/// happened to attempt makes the "overall" ranking compare non-comparable bases — a framework
/// that only ever attempted its best file type would look artificially strong.
///
/// The fix: restrict the overall ranking to the **intersection of file types every candidate
/// framework (of this output format) attempted** — "attempted" meaning at least one accountable
/// sample in at least one of `no_ocr`/`with_ocr` for that file type, regardless of success. Only
/// that shared set feeds the weighted mean, so every ranked framework is scored on the same corpus.
///
/// With a single candidate framework for a format, the "intersection" is trivially that
/// framework's own attempted file types — there is nothing to restrict against, so it is ranked
/// on everything it ran (rank 1 by construction). The shared-corpus restriction only bites once
/// two or more frameworks of the same format disagree on which file types they attempted.
///
/// Judgment call: if the shared set is empty (no candidates, or candidates share no file type at
/// all), there is no meaningful "overall" comparison to make — this function returns an empty
/// ranking rather than fabricating one from a partial/non-shared basis. Callers should treat an
/// empty result as "no shared-corpus overall ranking available for this format" and rely on the
/// per-file-type (e.g. `pdf_*`) rankings instead.
///
/// # Semantics (Bug B: 0-success buckets silently dropped)
///
/// Within the shared file-type set, a bucket a framework *attempted but completely failed*
/// (`successful_sample_count == 0`, accountable failures > 0) must drag its mean down — it is
/// not neutral, it is a failure. Such buckets contribute a quality value of `0.0`, weighted by
/// the bucket's accountable sample count (successful samples plus framework-fault failures).
/// Infrastructure failures carry no weight. This is distinct from a file type the framework never
/// attempted at all, which is excluded entirely by the shared-corpus restriction above (that's not
/// a failure, it's missing data, and including it would penalize frameworks for corpora they were
/// never run against).
/// Resolve the shared corpus for a format: the file types every candidate framework of that
/// format actually attempted (any accountable samples in either OCR bucket), intersected
/// across all candidates. This is the exact basis on which the "overall" quality ranking for
/// the format is computed; a single-format framework in the pool collapses it to that one type.
/// Returned sorted for stable metadata output.
pub(crate) fn resolve_shared_corpus_file_types(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
    format: OutputFormat,
) -> Vec<String> {
    let mut shared_file_types: Option<std::collections::HashSet<&str>> = None;
    for agg in by_framework_mode.values().filter(|agg| agg.output_format == format) {
        let attempted: std::collections::HashSet<&str> = agg
            .by_file_type
            .iter()
            .filter(|(_, ft)| {
                [&ft.no_ocr, &ft.with_ocr, &ft.unknown_ocr]
                    .into_iter()
                    .flatten()
                    .any(|perf| accountable_sample_count(perf) > 0)
            })
            .map(|(file_type, _)| file_type.as_str())
            .collect();
        shared_file_types = Some(match shared_file_types {
            Some(existing) => existing.intersection(&attempted).copied().collect(),
            None => attempted,
        });
    }
    let mut out: Vec<String> = shared_file_types
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    out.sort();
    out
}

fn build_shared_corpus_quality_ranking(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
    format: OutputFormat,
    optional_keys: &std::collections::HashSet<String>,
) -> Vec<RankedFramework> {
    let candidates: Vec<(&String, &FrameworkModeAggregation)> = by_framework_mode
        .iter()
        .filter(|(_, agg)| agg.output_format == format)
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    let shared_file_types = resolve_shared_corpus_file_types(by_framework_mode, format);

    if shared_file_types.is_empty() {
        return Vec::new();
    }

    let mut qual: Vec<(String, f64)> = Vec::new();
    for (key, agg) in candidates {
        let mut contributions: Vec<(f64, usize)> = Vec::new();
        for file_type in &shared_file_types {
            let Some(ft) = agg.by_file_type.get(file_type.as_str()) else {
                continue;
            };
            for perf in [&ft.no_ocr, &ft.with_ocr, &ft.unknown_ocr].into_iter().flatten() {
                let weight = accountable_sample_count(perf);
                if weight == 0 {
                    continue;
                }
                let value = perf
                    .quality
                    .as_ref()
                    .map(|q| coverage_adjusted_quality(q.quality_score_p50, perf))
                    .unwrap_or(0.0);
                contributions.push((value, weight));
            }
        }
        let mean = weighted_avg(&contributions);
        if mean.is_finite() {
            qual.push((key.clone(), mean));
        }
    }

    qual.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let baseline_qual = qual.first().map(|r| r.1).unwrap_or(1.0);
    qual.iter()
        .enumerate()
        .map(|(i, (k, v))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_qual > 0.0 { *v / baseline_qual } else { 1.0 },
            optional: optional_keys.contains(k),
            output_format: format,
            mode: parse_aggregate_key(k).1.to_string(),
        })
        .collect()
}

/// Aggregate keys (see [`super::make_aggregate_key`]) for every matrix cell either pinned cohort
/// contract marks `optional` (best-effort, e.g. MinerU). Used to flag [`RankedFramework::optional`]
/// so a ranking consumer can tell a contract-verified entry apart from a best-effort one that may
/// be partially failed or under-sampled relative to the pinned corpus.
fn optional_aggregate_keys(cohort: Option<crate::bench_matrix::Cohort>) -> std::collections::HashSet<String> {
    cohort
        .into_iter()
        .flat_map(|cohort| cohort.contract().matrix)
        .filter(|entry| entry.optional)
        .map(|entry| entry.aggregate_key())
        .collect()
}

/// Which direction is "better" when sorting a ranking's raw values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    /// Higher value is better (e.g. throughput, pages/sec).
    Descending,
    /// Lower value is better (e.g. memory).
    Ascending,
}

/// Group ranking candidates into deterministic `(output_format, mode)` segments. Markdown vs
/// plaintext serialization cost and single-file vs batch-amortized process overhead are not
/// comparable, so `throughput_ranking`, `memory_ranking`, `cpu_seconds_ranking`, and
/// `pages_per_sec_ranking` are each computed within a segment, never pooled across one.
/// `BTreeMap` keeps segment order deterministic when the caller flattens the map back to a
/// `Vec<RankedFramework>`.
fn group_by_segment<T>(
    items: Vec<T>,
    segment_of: impl Fn(&T) -> (OutputFormat, &str),
) -> std::collections::BTreeMap<(String, String), Vec<T>> {
    let mut segments: std::collections::BTreeMap<(String, String), Vec<T>> = std::collections::BTreeMap::new();
    for item in items {
        let (output_format, mode) = segment_of(&item);
        segments
            .entry((output_format.to_string(), mode.to_string()))
            .or_default()
            .push(item);
    }
    segments
}

/// Build a `(output_format, mode)`-segmented ranking: rank and `relative` are computed within
/// each segment against that segment's own best value, never against a value from a different
/// segment. The flat output is ordered by segment (deterministically, via `group_by_segment`'s
/// `BTreeMap`), then by rank within each segment. Used for `throughput_ranking`, `memory_ranking`,
/// and `pages_per_sec_ranking` (`cpu_seconds_ranking` has its own reference-value logic and is
/// built separately). Non-finite values are dropped before ranking, matching the pooled behavior
/// this replaces.
fn build_segmented_ranking(
    mut items: Vec<(String, f64, OutputFormat, String)>,
    direction: SortDirection,
    optional_keys: &std::collections::HashSet<String>,
) -> Vec<RankedFramework> {
    items.retain(|(_, value, ..)| value.is_finite());
    let segments = group_by_segment(items, |(_, _, fmt, mode)| (*fmt, mode.as_str()));

    let mut ranking = Vec::new();
    for mut group in segments.into_values() {
        match direction {
            SortDirection::Descending => group.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0))),
            SortDirection::Ascending => group.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0))),
        }
        let best = group.first().map(|(_, value, ..)| *value).unwrap_or(1.0);
        for (i, (key, value, output_format, mode)) in group.into_iter().enumerate() {
            ranking.push(RankedFramework {
                framework_mode: key.clone(),
                rank: i + 1,
                value,
                relative: if best > 0.0 { value / best } else { 1.0 },
                optional: optional_keys.contains(&key),
                output_format,
                mode,
            });
        }
    }
    ranking
}

pub(crate) fn comparison_for_cohort(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
    cohort: crate::bench_matrix::Cohort,
) -> ComparisonData {
    build_comparison(by_framework_mode, Some(cohort))
}

/// Apply cohort-specific optional flags after filesystem provenance has been folded into an
/// aggregate. Optionality cannot be inferred from aggregate keys alone because the same framework
/// cell can be required in one cohort and best-effort in another. ~keep
pub fn apply_pinned_cohort_comparison(aggregate: &mut super::types::NewConsolidatedResults) -> crate::Result<()> {
    let scoped_records: Vec<Option<&str>> = aggregate
        .run_provenance
        .iter()
        .map(|record| {
            record
                .provenance
                .as_ref()
                .and_then(|provenance| provenance.corpus.cohort.as_deref())
        })
        .collect();
    let cohort_names: std::collections::BTreeSet<&str> = scoped_records.iter().flatten().copied().collect();
    if cohort_names.is_empty() {
        return Ok(());
    }
    if cohort_names.len() != 1 || scoped_records.iter().any(Option::is_none) {
        return Err(crate::Error::Benchmark(format!(
            "cannot apply one benchmark contract to mixed or incomplete cohorts: {}. Each cohort has its own \
             contract (required cells, rankings), so consolidate each cohort's run directories separately \
             (one `consolidate` invocation per cohort, with its own --output) and do not mix in runs that \
             lack cohort provenance",
            cohort_names.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    let cohort_name = *cohort_names.first().expect("one cohort name");
    let Some(cohort) = crate::bench_matrix::Cohort::ALL
        .into_iter()
        .find(|cohort| cohort.contract().manifest_name == cohort_name)
    else {
        return Err(crate::Error::Benchmark(format!(
            "cannot apply unknown benchmark cohort contract: {cohort_name}"
        )));
    };
    aggregate.comparison = comparison_for_cohort(&aggregate.by_framework_mode, cohort);
    Ok(())
}

/// Collected inputs for cross-framework comparison rankings, gathered in one pass over
/// `by_framework_mode`. See [`build_comparison`].
struct CollectedComparisonMetrics {
    /// Key, throughput p50, memory p50, output format, mode.
    metrics: Vec<(String, f64, f64, OutputFormat, String)>,
    cpu_seconds_metrics: Vec<(String, f64, OutputFormat, String)>,
    pages_per_sec_metrics: Vec<(String, f64, OutputFormat, String)>,
    unranked_frameworks: Vec<UnrankedFramework>,
}

/// One pass over `by_framework_mode` collecting the raw metric tuples every ranking below is
/// built from, plus the frameworks excluded from one or more rankings (Defect S4).
fn collect_comparison_metrics(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
) -> CollectedComparisonMetrics {
    // Key, value(s), output format, mode. `throughput_ranking`, `memory_ranking`,
    // `cpu_seconds_ranking`, and `pages_per_sec_ranking` must never pool across
    // `(output_format, mode)` segments — see `build_segmented_ranking`. ~keep
    let mut metrics: Vec<(String, f64, f64, OutputFormat, String)> = Vec::new();
    let mut cpu_seconds_metrics: Vec<(String, f64, OutputFormat, String)> = Vec::new();
    let mut pages_per_sec_metrics: Vec<(String, f64, OutputFormat, String)> = Vec::new();
    // Frameworks attempted (present in `by_framework_mode`) but excluded from a ranking below,
    // recorded so the exclusion is never silent (Defect S4). ~keep
    let mut unranked_frameworks: Vec<UnrankedFramework> = Vec::new();

    for (key, agg) in by_framework_mode {
        let Some(performance) = agg
            .overall_performance
            .as_ref()
            .filter(|performance| performance.performance_sample_count > 0)
        else {
            let total = agg
                .overall_performance
                .as_ref()
                .map(|p| p.total_sample_count)
                .unwrap_or(0);
            unranked_frameworks.push(UnrankedFramework {
                framework_mode: key.clone(),
                reason: format!(
                    "no usable performance samples ({total} total result(s), none timing-eligible); \
                     excluded from throughput_ranking, memory_ranking, cpu_seconds_ranking, \
                     pages_per_sec_ranking, and pareto_frontier"
                ),
            });
            continue;
        };

        metrics.push((
            key.clone(),
            performance.throughput.p50,
            performance.memory.p50,
            agg.output_format,
            agg.mode.clone(),
        ));
        // A `cpu_seconds.sample_count == 0` framework had every performance sample measure
        // exactly `0.0` core-seconds — the monitoring-resolution floor (Defect S2), not a real
        // measurement — and `build_percentiles`/`calculate_percentiles` already excluded those
        // rows from the distribution. Exclude the framework from `cpu_seconds_ranking` too
        // (it would otherwise report a fabricated `0.0` p50 and rank first), and record why
        // instead of letting it silently vanish. It still appears in every other ranking, since
        // only its CPU-time measurement is unusable. ~keep
        if performance.cpu_seconds.sample_count > 0 {
            cpu_seconds_metrics.push((
                key.clone(),
                performance.cpu_seconds.p50,
                agg.output_format,
                agg.mode.clone(),
            ));
        } else {
            unranked_frameworks.push(UnrankedFramework {
                framework_mode: key.clone(),
                reason: "every performance sample measured 0.0 CPU-seconds (below monitoring \
                          resolution, not a real measurement); excluded from cpu_seconds_ranking"
                    .to_string(),
            });
        }
        if let Some(pages_per_sec) = &performance.pages_per_sec {
            pages_per_sec_metrics.push((key.clone(), pages_per_sec.p50, agg.output_format, agg.mode.clone()));
        }
    }

    CollectedComparisonMetrics {
        metrics,
        cpu_seconds_metrics,
        pages_per_sec_metrics,
        unranked_frameworks,
    }
}

/// Baseline for `deltas_vs_baseline` is the highest-throughput entry within each
/// `(output_format, mode)` segment, not a single cross-segment winner — mirroring
/// `throughput_ranking`'s own per-segment scoping (see `build_segmented_ranking`). ~keep
fn build_deltas_vs_baseline(metrics: &[(String, f64, f64, OutputFormat, String)]) -> HashMap<String, DeltaMetrics> {
    let mut deltas_vs_baseline = HashMap::new();
    let delta_segments = group_by_segment(metrics.to_vec(), |(_, _, _, fmt, mode)| (*fmt, mode.as_str()));
    for group in delta_segments.values() {
        let mut finite_by_throughput: Vec<&(String, f64, f64, OutputFormat, String)> =
            group.iter().filter(|(_, thr, ..)| thr.is_finite()).collect();
        finite_by_throughput.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let Some(baseline) = finite_by_throughput.first().copied() else {
            continue;
        };
        for (k, thr, mem_val, ..) in group {
            if k == &baseline.0 {
                continue;
            }
            deltas_vs_baseline.insert(
                k.clone(),
                DeltaMetrics {
                    throughput_delta_mbs: thr - baseline.1,
                    throughput_delta_percent: if baseline.1 > 0.0 {
                        ((thr - baseline.1) / baseline.1) * 100.0
                    } else {
                        0.0
                    },
                    memory_delta_mb: mem_val - baseline.2,
                    memory_delta_percent: if baseline.2 > 0.0 {
                        ((mem_val - baseline.2) / baseline.2) * 100.0
                    } else {
                        0.0
                    },
                },
            );
        }
    }
    deltas_vs_baseline
}

/// Collect per-framework PDF quality/TF1/SF1 contributions used for `pdf_quality_ranking_*`,
/// `pdf_tf1_ranking_*`, and `pdf_sf1_ranking_markdown`.
///
/// Bug B: a PDF bucket a framework *attempted but completely failed*
/// (`successful_sample_count == 0`, accountable failures > 0) must count against it — quality
/// contribution 0.0, weighted by accountable samples — instead of being silently dropped (which
/// let e.g. a framework failing 100% of PDFs escape any quality penalty). TF1/SF1 use the same
/// 0.0-on-full-failure treatment for consistency. ~keep
fn collect_pdf_metrics(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
) -> Vec<(String, f64, f64, f64, OutputFormat)> {
    let mut pdf_metrics: Vec<(String, f64, f64, f64, OutputFormat)> = Vec::new();
    for (key, agg) in by_framework_mode {
        if let Some(pdf_ft) = agg.by_file_type.get("pdf") {
            let mut qualities: Vec<(f64, usize)> = Vec::new();
            let mut tf1s: Vec<(f64, usize)> = Vec::new();
            let mut sf1s: Vec<(f64, usize)> = Vec::new();
            for perf in [&pdf_ft.no_ocr, &pdf_ft.with_ocr, &pdf_ft.unknown_ocr]
                .into_iter()
                .flatten()
            {
                let weight = accountable_sample_count(perf);
                if weight == 0 {
                    continue;
                }
                let (quality_value, tf1_value, sf1_value) = match &perf.quality {
                    Some(q) => (
                        coverage_adjusted_quality(q.quality_score_p50, perf),
                        coverage_adjusted_quality(q.f1_text_p50, perf),
                        q.f1_layout_p50.map(|value| coverage_adjusted_quality(value, perf)),
                    ),
                    None => (0.0, 0.0, None),
                };
                qualities.push((quality_value, weight));
                tf1s.push((tf1_value, weight));
                // SF1 has no defined "failure" value for plaintext-only frameworks (they never
                // carry a layout term at all), so a missing layout score only contributes 0.0
                // when the bucket was a genuine failure (no quality at all), not when the
                // framework is plaintext-only and layout is simply not applicable. ~keep
                match sf1_value {
                    Some(layout) => sf1s.push((layout, weight)),
                    None if perf.quality.is_none() => sf1s.push((0.0, weight)),
                    None => {}
                }
            }
            let q = weighted_avg(&qualities);
            let t = weighted_avg(&tf1s);
            let s = weighted_avg(&sf1s);
            if q.is_finite() {
                pdf_metrics.push((key.clone(), q, t, s, agg.output_format));
            }
        }
    }
    pdf_metrics
}

/// Per-framework coverage-adjusted PDF reading-order score, weighted by accountable samples across
/// the OCR buckets. As in [`collect_pdf_metrics`], a bucket that was attempted but failed entirely
/// (no quality at all) contributes 0.0 at its accountable weight, so failing a whole bucket costs
/// rank. A bucket whose successes simply carry no reading-order score (too few anchors matched) is
/// skipped, and a framework with no reading-order sample in any bucket is omitted rather than
/// scored 0.0: the score is report-only.
fn collect_pdf_reading_order_metrics(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
) -> Vec<(String, f64, OutputFormat)> {
    let mut metrics = Vec::new();
    for (key, agg) in by_framework_mode {
        let Some(pdf_ft) = agg.by_file_type.get("pdf") else {
            continue;
        };
        let mut contributions: Vec<(f64, usize)> = Vec::new();
        let mut has_sample = false;
        for perf in [&pdf_ft.no_ocr, &pdf_ft.with_ocr, &pdf_ft.unknown_ocr]
            .into_iter()
            .flatten()
        {
            let weight = accountable_sample_count(perf);
            if weight == 0 {
                continue;
            }
            match &perf.quality {
                Some(quality) => {
                    if let Some(value) = quality.reading_order_p50 {
                        has_sample = true;
                        contributions.push((coverage_adjusted_quality(value, perf), weight));
                    }
                }
                None => contributions.push((0.0, weight)),
            }
        }
        let mean = weighted_avg(&contributions);
        if has_sample && mean.is_finite() {
            metrics.push((key.clone(), mean, agg.output_format));
        }
    }
    metrics
}

/// Per-format PDF quality/TF1/SF1 rankings built from [`collect_pdf_metrics`]'s output. Plaintext
/// never carries an SF1 term (see module docs), so there is no `sf1_plaintext` field.
struct PdfRankings {
    quality_markdown: Vec<RankedFramework>,
    quality_plaintext: Vec<RankedFramework>,
    tf1_markdown: Vec<RankedFramework>,
    tf1_plaintext: Vec<RankedFramework>,
    sf1_markdown: Vec<RankedFramework>,
    reading_order_markdown: Vec<RankedFramework>,
    reading_order_plaintext: Vec<RankedFramework>,
}

/// Rank `items` descending into one PDF ranking. Shared by the PDF quality/TF1/SF1/reading-order
/// rankings only, which are pre-filtered to a single output format before calling and are NOT part
/// of the `(output_format, mode)` segmentation this module applies to `throughput_ranking`/
/// `memory_ranking`/`cpu_seconds_ranking`/`pages_per_sec_ranking` (see `build_segmented_ranking`)
/// — out of scope for that defect. ~keep
fn rank_pdf_items(
    items: &mut Vec<(String, f64)>,
    format: OutputFormat,
    optional_keys: &std::collections::HashSet<String>,
) -> Vec<RankedFramework> {
    items.retain(|(_, v)| v.is_finite());
    items.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let best = items.first().map(|r| r.1).unwrap_or(1.0);
    items
        .iter()
        .enumerate()
        .map(|(i, (k, v))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if best > 0.0 { *v / best } else { 1.0 },
            optional: optional_keys.contains(k),
            output_format: format,
            mode: parse_aggregate_key(k).1.to_string(),
        })
        .collect()
}

fn build_pdf_rankings(
    pdf_metrics: Vec<(String, f64, f64, f64, OutputFormat)>,
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
    optional_keys: &std::collections::HashSet<String>,
) -> PdfRankings {
    let build_ranking =
        |items: &mut Vec<(String, f64)>, format: OutputFormat| rank_pdf_items(items, format, optional_keys);

    // As with the all-file-types quality ranking above, PDF quality must also be split by
    // output format — plaintext-only frameworks never carry an SF1 term and must never be
    // pooled against markdown frameworks' layout-inclusive quality score. ~keep
    let mut pdf_qual_markdown: Vec<(String, f64)> = pdf_metrics
        .iter()
        .filter(|(_, _, _, _, fmt)| *fmt == OutputFormat::Markdown)
        .map(|(k, q, _, _, _)| (k.clone(), *q))
        .collect();
    let mut pdf_qual_plaintext: Vec<(String, f64)> = pdf_metrics
        .iter()
        .filter(|(_, _, _, _, fmt)| *fmt == OutputFormat::Plaintext)
        .map(|(k, q, _, _, _)| (k.clone(), *q))
        .collect();
    let mut pdf_tf1_markdown: Vec<(String, f64)> = pdf_metrics
        .iter()
        .filter(|(_, _, _, _, fmt)| *fmt == OutputFormat::Markdown)
        .map(|(k, _, t, _, _)| (k.clone(), *t))
        .collect();
    let mut pdf_tf1_plaintext: Vec<(String, f64)> = pdf_metrics
        .iter()
        .filter(|(_, _, _, _, fmt)| *fmt == OutputFormat::Plaintext)
        .map(|(k, _, t, _, _)| (k.clone(), *t))
        .collect();
    let mut pdf_sf1_markdown: Vec<(String, f64)> = pdf_metrics
        .iter()
        .filter(|(_, _, _, _, fmt)| *fmt == OutputFormat::Markdown)
        .map(|(k, _, _, s, _)| (k.clone(), *s))
        .collect();

    let reading_order_metrics = collect_pdf_reading_order_metrics(by_framework_mode);
    let reading_order_for = |format: OutputFormat| -> Vec<(String, f64)> {
        reading_order_metrics
            .iter()
            .filter(|(_, _, fmt)| *fmt == format)
            .map(|(k, v, _)| (k.clone(), *v))
            .collect()
    };
    let mut reading_order_markdown = reading_order_for(OutputFormat::Markdown);
    let mut reading_order_plaintext = reading_order_for(OutputFormat::Plaintext);

    PdfRankings {
        quality_markdown: build_ranking(&mut pdf_qual_markdown, OutputFormat::Markdown),
        quality_plaintext: build_ranking(&mut pdf_qual_plaintext, OutputFormat::Plaintext),
        tf1_markdown: build_ranking(&mut pdf_tf1_markdown, OutputFormat::Markdown),
        tf1_plaintext: build_ranking(&mut pdf_tf1_plaintext, OutputFormat::Plaintext),
        sf1_markdown: build_ranking(&mut pdf_sf1_markdown, OutputFormat::Markdown),
        reading_order_markdown: build_ranking(&mut reading_order_markdown, OutputFormat::Markdown),
        reading_order_plaintext: build_ranking(&mut reading_order_plaintext, OutputFormat::Plaintext),
    }
}

/// cpu_seconds is lower-is-better, so the natural baseline is the smallest value. But native
/// single-file frameworks (e.g. liteparse/xberg) routinely report exactly 0.0 core-seconds, and
/// a 0.0 baseline is undefined, and the old `if baseline > 0.0 { .. } else { 1.0 }` guard used to
/// fall through to `1.0` for *every* row once that happened — including rows with real,
/// materially different positive cpu_seconds — making `relative` meaningless whenever any
/// framework hit the 0.0 floor.
///
/// Fix: use the smallest *positive* cpu_seconds value in the ranking as the reference point
/// instead of the true (possibly-zero) minimum. This subsumes the old behavior when the true
/// positive row still gets a finite ratio against the smallest positive cost observed. Only when
/// literally every row is 0.0 does `reference` stay 0.0, in which case every row's `relative`
/// degenerates to 0.0 (all tied for best) rather than the old, misleading all-`1.0`. The
/// smallest-positive-or-fallback search runs independently within each `(output_format, mode)`
/// segment, since `cpu_seconds_ranking` must never pool across segments either. ~keep
fn build_cpu_seconds_ranking(
    mut cpu_seconds_metrics: Vec<(String, f64, OutputFormat, String)>,
    optional_keys: &std::collections::HashSet<String>,
) -> Vec<RankedFramework> {
    cpu_seconds_metrics.retain(|(_, v, ..)| v.is_finite());
    let cpu_seconds_segments = group_by_segment(cpu_seconds_metrics, |(_, _, fmt, mode)| (*fmt, mode.as_str()));
    let mut cpu_seconds_ranking: Vec<RankedFramework> = Vec::new();
    for mut group in cpu_seconds_segments.into_values() {
        group.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let reference_cpu_seconds = group
            .iter()
            .map(|(_, v, ..)| *v)
            .find(|v| *v > 0.0)
            .unwrap_or_else(|| group.first().map(|(_, v, ..)| *v).unwrap_or(0.0));
        for (i, (k, v, fmt, mode)) in group.into_iter().enumerate() {
            cpu_seconds_ranking.push(RankedFramework {
                framework_mode: k.clone(),
                rank: i + 1,
                value: v,
                relative: if reference_cpu_seconds > 0.0 {
                    v / reference_cpu_seconds
                } else {
                    0.0
                },
                optional: optional_keys.contains(&k),
                output_format: fmt,
                mode,
            });
        }
    }
    cpu_seconds_ranking
}

/// Build cross-framework comparison rankings from aggregated data
///
/// Uses the framework-mode-wide process aggregation so a native batch contributes
/// once even when its document rows span several file-type or OCR buckets.
pub(super) fn build_comparison(
    by_framework_mode: &HashMap<String, FrameworkModeAggregation>,
    cohort: Option<crate::bench_matrix::Cohort>,
) -> ComparisonData {
    let competitive: HashMap<String, FrameworkModeAggregation> = by_framework_mode
        .iter()
        .filter(|(_, aggregation)| aggregation.timing_regime == crate::types::TimingRegime::ColdProcess)
        .map(|(key, aggregation)| (key.clone(), aggregation.clone()))
        .collect();
    let by_framework_mode = &competitive;
    let optional_keys = optional_aggregate_keys(cohort);
    let collected = collect_comparison_metrics(by_framework_mode);

    let throughput_ranking = build_segmented_ranking(
        collected
            .metrics
            .iter()
            .map(|(k, thr, _, fmt, mode)| (k.clone(), *thr, *fmt, mode.clone()))
            .collect(),
        SortDirection::Descending,
        &optional_keys,
    );
    let memory_ranking = build_segmented_ranking(
        collected
            .metrics
            .iter()
            .map(|(k, _, mem, fmt, mode)| (k.clone(), *mem, *fmt, mode.clone()))
            .collect(),
        SortDirection::Ascending,
        &optional_keys,
    );

    let quality_ranking_markdown =
        build_shared_corpus_quality_ranking(by_framework_mode, OutputFormat::Markdown, &optional_keys);
    let quality_ranking_plaintext =
        build_shared_corpus_quality_ranking(by_framework_mode, OutputFormat::Plaintext, &optional_keys);

    let deltas_vs_baseline = build_deltas_vs_baseline(&collected.metrics);

    let pdf_metrics = collect_pdf_metrics(by_framework_mode);
    let pdf_rankings = build_pdf_rankings(pdf_metrics, by_framework_mode, &optional_keys);

    let pages_per_sec_ranking = build_segmented_ranking(
        collected.pages_per_sec_metrics,
        SortDirection::Descending,
        &optional_keys,
    );

    let cpu_seconds_ranking = build_cpu_seconds_ranking(collected.cpu_seconds_metrics, &optional_keys);

    let pareto_frontier = build_pareto_frontier(by_framework_mode);

    // `by_framework_mode` is a `HashMap`, so iteration order (and thus push order above) is
    // nondeterministic; sort for reproducible output.
    let mut unranked_frameworks = collected.unranked_frameworks;
    unranked_frameworks.sort_by(|a, b| {
        a.framework_mode
            .cmp(&b.framework_mode)
            .then_with(|| a.reason.cmp(&b.reason))
    });

    ComparisonData {
        throughput_ranking,
        memory_ranking,
        quality_ranking_markdown,
        quality_ranking_plaintext,
        pdf_quality_ranking_markdown: pdf_rankings.quality_markdown,
        pdf_quality_ranking_plaintext: pdf_rankings.quality_plaintext,
        pdf_tf1_ranking_markdown: pdf_rankings.tf1_markdown,
        pdf_tf1_ranking_plaintext: pdf_rankings.tf1_plaintext,
        pdf_sf1_ranking_markdown: pdf_rankings.sf1_markdown,
        pdf_reading_order_ranking_markdown: pdf_rankings.reading_order_markdown,
        pdf_reading_order_ranking_plaintext: pdf_rankings.reading_order_plaintext,
        pages_per_sec_ranking,
        cpu_seconds_ranking,
        deltas_vs_baseline,
        pareto_frontier,
        unranked_frameworks,
    }
}

/// Build the non-dominated (pages/sec ↑, SF1 ↑, peak-RSS ↓) frontier across markdown frameworks.
///
/// See [`ParetoPoint`] for the dominance rule and eligibility criteria (markdown output format,
/// a defined SF1 term, and at least one pages/sec observation).
fn build_pareto_frontier(by_framework_mode: &HashMap<String, FrameworkModeAggregation>) -> Vec<ParetoPoint> {
    let candidates: Vec<ParetoPoint> = by_framework_mode
        .iter()
        .filter(|(_, agg)| agg.output_format == OutputFormat::Markdown)
        .filter_map(|(key, agg)| {
            let performance = agg
                .overall_performance
                .as_ref()
                .filter(|performance| performance.performance_sample_count > 0)?;
            let pages_per_sec = performance.pages_per_sec.as_ref()?.p50;
            let sf1 = coverage_adjusted_quality(performance.quality.as_ref()?.f1_layout_p50?, performance);
            let peak_memory_mb = performance.memory.p50;
            if !pages_per_sec.is_finite() || !sf1.is_finite() || !peak_memory_mb.is_finite() {
                return None;
            }
            Some(ParetoPoint {
                framework_mode: key.clone(),
                pages_per_sec,
                sf1,
                peak_memory_mb,
            })
        })
        .collect();

    let mut frontier: Vec<ParetoPoint> = candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other.framework_mode != candidate.framework_mode
                    && other.pages_per_sec >= candidate.pages_per_sec
                    && other.sf1 >= candidate.sf1
                    && other.peak_memory_mb <= candidate.peak_memory_mb
                    && (other.pages_per_sec > candidate.pages_per_sec
                        || other.sf1 > candidate.sf1
                        || other.peak_memory_mb < candidate.peak_memory_mb)
            })
        })
        .cloned()
        .collect();
    frontier.sort_by(|a, b| a.framework_mode.cmp(&b.framework_mode));
    frontier
}
