//! Paired comparisons over per-fixture rows: layout-variant vs baseline-variant
//! ([`LayoutImpact`]) and xberg vs competitor ([`HeadToHead`]).
//!
//! Rows pair by fixture inside one `(output_format, mode, file_type)` bucket and only on fixtures
//! where both sides succeeded. OCR use is a property of each row, not part of the bucket: a failed
//! row never reports it (`ocr: None`), and an auto-OCR variant reports it per document, so keying
//! on it would split one fixture's rows apart. Every delta is the median of per-fixture deltas, never the
//! difference of two medians, so a fixture present on one side cannot skew the other.

use super::types::{FixtureDelta, HeadToHead, LayoutImpact, PairedDelta, PairedFixtureDeltas, PerFixtureRow};
use crate::stats::{percentile_r7, sanitize_f64};
use crate::types::{OutputFormat, ResourceMeasurementScope, TimingRegime};
use std::collections::{BTreeMap, BTreeSet};

/// `|delta|` below this on the classification metric counts as unchanged.
const UNCHANGED_THRESHOLD: f64 = 0.005;

/// Execution mode whose rows do not carry a per-document duration or RSS: a native batch has one
/// process-level measurement shared by every document, so a per-fixture timing delta is
/// meaningless there.
const BATCH_MODE: &str = "batch";

/// `(output_format rank, mode, file_type)`. The format is held as a rank because `OutputFormat` is
/// not `Ord`; see [`format_rank`] and [`format_from_rank`].
type BucketKey = (u8, String, String);

/// framework -> bucket -> fixture id -> row.
type RowIndex<'a> = BTreeMap<&'a str, BTreeMap<BucketKey, BTreeMap<&'a str, &'a PerFixtureRow>>>;

fn format_rank(format: OutputFormat) -> u8 {
    match format {
        OutputFormat::Markdown => 0,
        OutputFormat::Plaintext => 1,
    }
}

fn format_from_rank(rank: u8) -> OutputFormat {
    if rank == format_rank(OutputFormat::Plaintext) {
        OutputFormat::Plaintext
    } else {
        OutputFormat::Markdown
    }
}

fn index_rows(rows: &[PerFixtureRow]) -> RowIndex<'_> {
    let mut index: RowIndex<'_> = BTreeMap::new();
    for row in rows {
        let key: BucketKey = (
            format_rank(row.output_format),
            row.execution_mode.clone(),
            row.file_type.clone(),
        );
        let slot = index
            .entry(row.framework.as_str())
            .or_default()
            .entry(key)
            .or_default()
            .entry(row.fixture_id.as_str());
        // Repeated rows for one fixture (re-runs) keep the first success, else the first row.
        slot.and_modify(|existing| {
            if !existing.success && row.success {
                *existing = row;
            }
        })
        .or_insert(row);
    }
    index
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn sf1_of(row: &PerFixtureRow) -> Option<f64> {
    if row.output_format == OutputFormat::Plaintext {
        return None;
    }
    row.f1_layout.and_then(finite)
}

fn tf1_of(row: &PerFixtureRow) -> Option<f64> {
    row.f1_text.and_then(finite)
}

fn reading_order_of(row: &PerFixtureRow) -> Option<f64> {
    row.quality
        .as_ref()
        .and_then(|q| q.reading_order_score)
        .and_then(finite)
}

fn quality_score_of(row: &PerFixtureRow) -> Option<f64> {
    row.quality_score.and_then(finite)
}

/// Whether the row carries a per-document, cold-process timing measurement.
fn has_document_timing(row: &PerFixtureRow) -> bool {
    row.execution_mode != BATCH_MODE && row.timing_regime == TimingRegime::ColdProcess
}

fn duration_of(row: &PerFixtureRow) -> Option<f64> {
    has_document_timing(row).then_some(row.duration_ms).and_then(finite)
}

fn extraction_duration_of(row: &PerFixtureRow) -> Option<f64> {
    if !has_document_timing(row) {
        return None;
    }
    row.extraction_duration_ms.and_then(finite)
}

/// Peak RSS is only comparable when the harness measured an isolated process, mirroring the
/// memory percentiles in the per-framework aggregation.
fn peak_memory_of(row: &PerFixtureRow) -> Option<f64> {
    let isolated = row.framework_capabilities.resource_measurement_scope == ResourceMeasurementScope::IsolatedProcess;
    (has_document_timing(row) && isolated)
        .then_some(row.peak_memory_mb)
        .and_then(finite)
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    sanitize_f64(percentile_r7(&sorted, 0.50))
}

/// Per-fixture deltas (`b - a`) of `metric` over the pairs where both sides have a value.
fn fixture_deltas(
    pairs: &[(&PerFixtureRow, &PerFixtureRow)],
    metric: impl Fn(&PerFixtureRow) -> Option<f64>,
) -> Vec<FixtureDelta> {
    pairs
        .iter()
        .filter_map(|(a, b)| {
            let (baseline, layout) = (metric(a)?, metric(b)?);
            Some(FixtureDelta {
                fixture_id: a.fixture_id.clone(),
                baseline,
                layout,
                delta: sanitize_f64(layout - baseline),
            })
        })
        .collect()
}

fn summarize(deltas: &[FixtureDelta]) -> PairedDelta {
    if deltas.is_empty() {
        return PairedDelta::default();
    }
    let baselines: Vec<f64> = deltas.iter().map(|d| d.baseline).collect();
    let layouts: Vec<f64> = deltas.iter().map(|d| d.layout).collect();
    let diffs: Vec<f64> = deltas.iter().map(|d| d.delta).collect();
    PairedDelta {
        baseline_p50: median(&baselines),
        layout_p50: median(&layouts),
        delta_p50: median(&diffs),
        sample_count: deltas.len(),
    }
}

fn summarize_optional(deltas: &[FixtureDelta]) -> Option<PairedDelta> {
    (!deltas.is_empty()).then(|| summarize(deltas))
}

/// Fixtures present in both maps where both rows succeeded, as `(a, b)` rows sorted by fixture id.
fn succeeded_pairs<'a>(
    a: &BTreeMap<&'a str, &'a PerFixtureRow>,
    b: &BTreeMap<&'a str, &'a PerFixtureRow>,
) -> Vec<(&'a PerFixtureRow, &'a PerFixtureRow)> {
    a.iter()
        .filter_map(|(id, row_a)| {
            let row_b = b.get(id)?;
            (row_a.success && row_b.success).then_some((*row_a, *row_b))
        })
        .collect()
}

/// OCR use every successful row reports, or `None` when they disagree, one did not report it, or
/// none succeeded. Failed rows are ignored: they carry `ocr: None` because extraction never
/// reported it.
fn common_ocr<'a>(rows: impl IntoIterator<Item = &'a PerFixtureRow>) -> Option<bool> {
    let mut reported = rows.into_iter().filter(|row| row.success).map(|row| row.ocr);
    let first = reported.next()??;
    reported.all(|ocr| ocr == Some(first)).then_some(first)
}

/// Pairs whose first (`a`) and second (`b`) rows reported OCR use, respectively.
fn ocr_used_counts(pairs: &[(&PerFixtureRow, &PerFixtureRow)]) -> (usize, usize) {
    let used = |row: &PerFixtureRow| row.ocr == Some(true);
    (
        pairs.iter().filter(|pair| used(pair.0)).count(),
        pairs.iter().filter(|pair| used(pair.1)).count(),
    )
}

fn succeeded_ids<'a>(rows: &BTreeMap<&'a str, &'a PerFixtureRow>) -> BTreeSet<&'a str> {
    rows.iter().filter(|(_, row)| row.success).map(|(id, _)| *id).collect()
}

/// Name of the baseline variant a layout variant is compared against, if one is present.
///
/// The `layout` segment is replaced by `baseline`; failing that it is dropped
/// (`xberg-markdown-sceptre-ort-layout` pairs with `xberg-markdown-sceptre-ort`); and a trailing
/// model-version segment after it is discarded (`xberg-markdown-layout-v3` pairs with
/// `xberg-markdown-baseline`). Returns `None` for a framework without a `layout` segment.
pub(crate) fn baseline_framework_for(layout_framework: &str, present: &BTreeSet<&str>) -> Option<String> {
    let segments: Vec<&str> = layout_framework.split('-').collect();
    let at = segments.iter().position(|segment| *segment == "layout")?;

    let mut replaced = segments.clone();
    replaced[at] = "baseline";
    let mut dropped = segments.clone();
    dropped.remove(at);
    let mut candidates = vec![replaced.join("-"), dropped.join("-")];
    let is_version = |segment: &str| {
        segment
            .strip_prefix('v')
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
    };
    if segments[at + 1..].iter().all(|segment| is_version(segment)) {
        let mut truncated: Vec<&str> = segments[..at].to_vec();
        truncated.push("baseline");
        candidates.push(truncated.join("-"));
    }
    candidates.into_iter().find(|name| present.contains(name.as_str()))
}

fn is_xberg(framework: &str) -> bool {
    framework.starts_with("xberg-")
}

/// Classify per-fixture deltas into improved / regressed / unchanged ids.
fn classify(deltas: &[FixtureDelta]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (mut improved, mut regressed, mut unchanged) = (Vec::new(), Vec::new(), Vec::new());
    for d in deltas {
        let bucket = if d.delta.abs() < UNCHANGED_THRESHOLD {
            &mut unchanged
        } else if d.delta > 0.0 {
            &mut improved
        } else {
            &mut regressed
        };
        bucket.push(d.fixture_id.clone());
    }
    (improved, regressed, unchanged)
}

/// One [`LayoutImpact`] per (layout framework, baseline framework, output format, mode, file type)
/// where both frameworks have rows. A bucket whose layout rows all failed still yields an entry
/// with `paired_fixture_count == 0` so the loss is visible rather than absent.
pub(super) fn build_layout_impact(rows: &[PerFixtureRow]) -> Vec<LayoutImpact> {
    let index = index_rows(rows);
    let present: BTreeSet<&str> = index.keys().copied().collect();
    let mut out = Vec::new();

    for (&layout_framework, layout_buckets) in index.iter().filter(|(name, _)| is_xberg(name)) {
        let Some(baseline_framework) = baseline_framework_for(layout_framework, &present) else {
            continue;
        };
        let Some(baseline_buckets) = index.get(baseline_framework.as_str()) else {
            continue;
        };
        for key in layout_buckets.keys() {
            let Some(baseline_rows) = baseline_buckets.get(key) else {
                continue;
            };
            let layout_rows = &layout_buckets[key];
            let pairs = succeeded_pairs(baseline_rows, layout_rows);
            let (baseline_ocr_fixture_count, layout_ocr_fixture_count) = ocr_used_counts(&pairs);

            let sf1 = fixture_deltas(&pairs, sf1_of);
            let tf1 = fixture_deltas(&pairs, tf1_of);
            let reading_order = fixture_deltas(&pairs, reading_order_of);
            let quality_score = fixture_deltas(&pairs, quality_score_of);
            let duration_ms = fixture_deltas(&pairs, duration_of);
            let peak_memory_mb = fixture_deltas(&pairs, peak_memory_of);

            let output_format = format_from_rank(key.0);
            let classified = if output_format == OutputFormat::Markdown && !sf1.is_empty() {
                &sf1
            } else {
                &quality_score
            };
            let (improved, regressed, unchanged) = classify(classified);

            out.push(LayoutImpact {
                layout_framework: layout_framework.to_string(),
                baseline_framework: baseline_framework.clone(),
                output_format,
                mode: key.1.clone(),
                file_type: key.2.clone(),
                ocr: common_ocr(baseline_rows.values().chain(layout_rows.values()).copied()),
                baseline_ocr_fixture_count,
                layout_ocr_fixture_count,
                paired_fixture_count: pairs.len(),
                sf1: summarize_optional(&sf1),
                tf1: summarize(&tf1),
                reading_order: summarize_optional(&reading_order),
                quality_score: summarize(&quality_score),
                duration_ms: summarize(&duration_ms),
                peak_memory_mb: summarize(&peak_memory_mb),
                improved_fixture_ids: improved,
                regressed_fixture_ids: regressed,
                unchanged_fixture_ids: unchanged,
                per_fixture: PairedFixtureDeltas {
                    sf1,
                    tf1,
                    reading_order,
                    duration_ms,
                    peak_memory_mb,
                },
            });
        }
    }
    out
}

/// One [`HeadToHead`] per (xberg framework, competitor framework, output format, mode, file type)
/// in which the two share at least one fixture id. Medians cover fixtures both
/// succeeded on; the `*_only_fixture_ids` lists name fixtures only one side succeeded on.
pub(super) fn build_head_to_head(rows: &[PerFixtureRow]) -> Vec<HeadToHead> {
    let index = index_rows(rows);
    let mut out = Vec::new();

    for (&xberg_framework, xberg_buckets) in index.iter().filter(|(name, _)| is_xberg(name)) {
        for (&competitor_framework, competitor_buckets) in index.iter().filter(|(name, _)| !is_xberg(name)) {
            for key in xberg_buckets.keys() {
                let Some(competitor_rows) = competitor_buckets.get(key) else {
                    continue;
                };
                let xberg_rows = &xberg_buckets[key];
                if !xberg_rows.keys().any(|id| competitor_rows.contains_key(id)) {
                    continue;
                }
                let pairs = succeeded_pairs(competitor_rows, xberg_rows);
                let (competitor_ocr_fixture_count, xberg_ocr_fixture_count) = ocr_used_counts(&pairs);
                let (xberg_ok, competitor_ok) = (succeeded_ids(xberg_rows), succeeded_ids(competitor_rows));

                let sf1 = fixture_deltas(&pairs, sf1_of);
                let reading_order = fixture_deltas(&pairs, reading_order_of);
                let output_format = format_from_rank(key.0);
                out.push(HeadToHead {
                    xberg_framework: xberg_framework.to_string(),
                    competitor_framework: competitor_framework.to_string(),
                    output_format,
                    mode: key.1.clone(),
                    file_type: key.2.clone(),
                    ocr: common_ocr(xberg_rows.values().chain(competitor_rows.values()).copied()),
                    xberg_ocr_fixture_count,
                    competitor_ocr_fixture_count,
                    shared_fixture_count: pairs.len(),
                    xberg_only_fixture_ids: xberg_ok.difference(&competitor_ok).map(|id| id.to_string()).collect(),
                    competitor_only_fixture_ids: competitor_ok.difference(&xberg_ok).map(|id| id.to_string()).collect(),
                    duration_ms: summarize(&fixture_deltas(&pairs, duration_of)),
                    extraction_duration_ms: summarize(&fixture_deltas(&pairs, extraction_duration_of)),
                    peak_memory_mb: summarize(&fixture_deltas(&pairs, peak_memory_of)),
                    tf1: summarize(&fixture_deltas(&pairs, tf1_of)),
                    sf1: summarize_optional(&sf1),
                    reading_order: summarize_optional(&reading_order),
                    quality_score: summarize(&fixture_deltas(&pairs, quality_score_of)),
                });
            }
        }
    }
    out
}
