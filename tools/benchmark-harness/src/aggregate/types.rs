//! Schema types for the aggregated benchmark output (v2.12.0).
//!
//! Pure data definitions only — see [`super`] for the aggregation logic that populates them.

use crate::types::{OutputFormat, TimingRegime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version for the aggregated output format.
pub const SCHEMA_VERSION: &str = "2.12.0";

/// Consolidated results using aggregation format v2.8.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConsolidatedResults {
    /// Schema version for this output format
    pub schema_version: String,
    /// Aggregated results grouped by framework:output_format:mode combination
    pub by_framework_mode: HashMap<String, FrameworkModeAggregation>,
    /// Disk sizes for each framework
    pub disk_sizes: HashMap<String, crate::types::DiskSizeInfo>,
    /// Cross-framework comparison rankings
    pub comparison: ComparisonData,
    /// Per-fixture results (one row per framework:output_format:execution_mode:fixture_id:ocr)
    pub per_fixture_results: Vec<PerFixtureRow>,
    /// Metadata about the consolidation
    pub metadata: ConsolidationMetadata,
    /// Run provenance sidecars folded in from every consolidated input directory (v2.8.0+).
    ///
    /// [`super::aggregate_new_format`] always leaves this empty: it has no filesystem access and
    /// only sees already-loaded [`crate::types::BenchmarkResult`]s. The `consolidate` CLI command
    /// populates it after aggregation by pairing [`crate::consolidate::load_run_provenance`]'s
    /// output with the same input directories passed to [`crate::consolidate::load_run_results`].
    /// `#[serde(default)]` so aggregates produced before this field existed still deserialize.
    #[serde(default)]
    pub run_provenance: Vec<crate::consolidate::RunProvenanceRecord>,
    /// Cohort-wide failure roll-up (framework-fault vs infrastructure), broken out per
    /// framework-mode and per file type. `#[serde(default)]` so pre-2.9.0 aggregates still
    /// deserialize.
    #[serde(default)]
    pub failure_summary: FailureSummary,
    /// Capability-aware format-support matrix: for every framework in the run and every observed
    /// file type, which pairs the framework declares no support for. Distinguishes "this
    /// framework structurally cannot read this format" from "absent" or "attempted and failed".
    /// `#[serde(default)]` so aggregates produced before this field existed still deserialize.
    #[serde(default)]
    pub format_support: FormatSupportMatrix,
    /// Layout-variant vs baseline-variant deltas on the fixtures where both succeeded, one entry
    /// per (layout framework, baseline framework, output format, mode, file type) (v2.12.0+). `#[serde(default)]` so pre-2.12.0 aggregates still deserialize.
    #[serde(default)]
    pub layout_impact: Vec<LayoutImpact>,
    /// xberg vs competitor deltas on the fixtures where both succeeded, one entry per
    /// (xberg framework, competitor framework, output format, mode, file type) (v2.12.0+). `#[serde(default)]` so pre-2.12.0 aggregates still deserialize.
    #[serde(default)]
    pub head_to_head: Vec<HeadToHead>,
}

/// Paired comparison of one metric over the fixtures both sides produced a value for.
///
/// `baseline` is the reference side (the baseline variant in [`LayoutImpact`], the competitor in
/// [`HeadToHead`]); `layout` is the compared side (the layout variant, respectively xberg).
/// `delta_p50` is the median of the per-fixture deltas `layout - baseline`, NOT the difference of
/// the two medians. `sample_count` is the number of fixtures with a value on both sides; the three
/// medians are `None` exactly when it is zero, so "not measured" never reads as "no difference".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PairedDelta {
    /// Median of the reference side's per-fixture values.
    pub baseline_p50: Option<f64>,
    /// Median of the compared side's per-fixture values.
    pub layout_p50: Option<f64>,
    /// Median of the per-fixture deltas (`layout - baseline`).
    pub delta_p50: Option<f64>,
    /// Fixtures contributing to the three medians.
    pub sample_count: usize,
}

/// One fixture's value on both sides of a paired comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureDelta {
    pub fixture_id: String,
    /// Reference-side value (see [`PairedDelta`]).
    pub baseline: f64,
    /// Compared-side value (see [`PairedDelta`]).
    pub layout: f64,
    /// `layout - baseline`.
    pub delta: f64,
}

/// Per-fixture values behind the [`PairedDelta`]s of a [`LayoutImpact`], sorted by fixture id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PairedFixtureDeltas {
    pub sf1: Vec<FixtureDelta>,
    pub tf1: Vec<FixtureDelta>,
    pub reading_order: Vec<FixtureDelta>,
    pub duration_ms: Vec<FixtureDelta>,
    pub peak_memory_mb: Vec<FixtureDelta>,
}

/// Effect of the layout pass: a layout variant against its baseline variant (v2.12.0+).
///
/// Variants pair by name, the layout variant's `layout` segment replaced by `baseline`
/// (`xberg-markdown-layout` with `xberg-markdown-baseline`), or dropped when no such framework
/// exists (`xberg-markdown-sceptre-ort-layout` with `xberg-markdown-sceptre-ort`). Only fixtures
/// where both variants succeeded are paired; duration and memory are further restricted to
/// single-file, cold-process measurements (and isolated-process RSS for memory) because batch rows
/// do not carry per-document timing.
///
/// OCR use does not split entries: a fixture pairs whatever OCR each side reported, since a failed
/// row never reports it and an auto-OCR variant decides per document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutImpact {
    pub layout_framework: String,
    pub baseline_framework: String,
    pub output_format: OutputFormat,
    pub mode: String,
    pub file_type: String,
    /// OCR use reported by every successful row on both sides; `None` when they disagree, any did
    /// not report it, or none succeeded.
    pub ocr: Option<bool>,
    /// Paired fixtures on which the baseline variant reported OCR use.
    #[serde(default)]
    pub baseline_ocr_fixture_count: usize,
    /// Paired fixtures on which the layout variant reported OCR use.
    #[serde(default)]
    pub layout_ocr_fixture_count: usize,
    /// Fixtures where both variants succeeded.
    pub paired_fixture_count: usize,
    /// Structural F1; `None` for plaintext output or when no fixture carries it.
    pub sf1: Option<PairedDelta>,
    pub tf1: PairedDelta,
    /// `None` when no paired fixture carries a reading-order score.
    pub reading_order: Option<PairedDelta>,
    pub quality_score: PairedDelta,
    pub duration_ms: PairedDelta,
    pub peak_memory_mb: PairedDelta,
    /// Fixtures whose delta is at least `0.005` in the layout variant's favor, judged per fixture
    /// on SF1 when both sides carry it, else on `quality_score` (always for plaintext).
    pub improved_fixture_ids: Vec<String>,
    /// Fixtures whose delta is at most `-0.005` on the same metric.
    pub regressed_fixture_ids: Vec<String>,
    /// Fixtures with an absolute delta below `0.005` on the same metric.
    pub unchanged_fixture_ids: Vec<String>,
    /// Paired fixtures with neither SF1 nor `quality_score` on both sides (no ground truth). The
    /// four id lists partition the paired fixtures.
    #[serde(default)]
    pub unscored_fixture_ids: Vec<String>,
    pub per_fixture: PairedFixtureDeltas,
}

/// xberg against one competitor on the fixtures both succeeded on (v2.12.0+).
///
/// All [`PairedDelta`]s read with `baseline` = competitor, `layout` = xberg, so a positive
/// `delta_p50` on a quality metric means xberg scored higher. Duration and memory carry
/// `sample_count == 0` for batch mode and warm-process rows, as in [`LayoutImpact`]. OCR use does
/// not split entries, as in [`LayoutImpact`]: xberg auto-OCRing a scan the competitor read without
/// OCR is still one shared fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadToHead {
    pub xberg_framework: String,
    pub competitor_framework: String,
    pub output_format: OutputFormat,
    pub mode: String,
    pub file_type: String,
    /// OCR use reported by every successful row on both sides; `None` when they disagree, any did
    /// not report it, or none succeeded.
    pub ocr: Option<bool>,
    /// Shared fixtures on which xberg reported OCR use.
    #[serde(default)]
    pub xberg_ocr_fixture_count: usize,
    /// Shared fixtures on which the competitor reported OCR use.
    #[serde(default)]
    pub competitor_ocr_fixture_count: usize,
    /// Fixtures where both frameworks succeeded.
    pub shared_fixture_count: usize,
    /// Fixtures xberg succeeded on and the competitor did not (absent or failed).
    pub xberg_only_fixture_ids: Vec<String>,
    /// Fixtures the competitor succeeded on and xberg did not (absent or failed).
    pub competitor_only_fixture_ids: Vec<String>,
    pub duration_ms: PairedDelta,
    pub extraction_duration_ms: PairedDelta,
    pub peak_memory_mb: PairedDelta,
    pub tf1: PairedDelta,
    /// Structural F1; `None` for plaintext output or when no shared fixture carries it.
    pub sf1: Option<PairedDelta>,
    /// `None` when no shared fixture carries a reading-order score.
    pub reading_order: Option<PairedDelta>,
    pub quality_score: PairedDelta,
}

/// Declared format-support coverage across the run.
///
/// Sourced from each framework's declared capabilities (the same table the runner routes on),
/// not from per-framework outcomes. `file_types` contains formats observed anywhere in the
/// consolidated results; normal release aggregates include xberg's full-corpus run, making that
/// the complete corpus format set. xberg never appears in `unsupported`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormatSupportMatrix {
    /// Every file type observed in the consolidated results, sorted and de-duplicated.
    pub file_types: Vec<String>,
    /// Per logical framework, the observed file types it declares no support for, sorted. A
    /// framework absent from this map supports every observed file type.
    pub unsupported: std::collections::BTreeMap<String, Vec<String>>,
}

/// Per-fixture benchmark result row
///
/// The scalar fields below (`duration_ms`, `peak_memory_mb`, `f1_text`, …) are the original
/// v2.3.0 convenience projection and are kept as-is for backward compatibility. The fields added
/// in v2.8.0 (`file_size` onward) make each row losslessly carry every measured field from its
/// source `BenchmarkResult`, including ones with no earlier scalar equivalent (e.g. the free-text
/// `error_message`, or the full `quality.missing_tokens`/`extra_tokens` token lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerFixtureRow {
    /// Framework name
    pub framework: String,
    /// Output format (markdown or plaintext)
    pub output_format: OutputFormat,
    /// Execution mode (single, batch, etc.)
    pub execution_mode: String,
    /// Process lifecycle represented by this measurement. ~keep
    #[serde(default = "crate::types::unknown_timing_regime")]
    pub timing_regime: TimingRegime,
    /// Whether OCR was actually used, or `null` when the framework did not report it.
    pub ocr: Option<bool>,
    /// Fixture ID (e.g., from file path)
    pub fixture_id: String,
    /// File type/extension
    pub file_type: String,
    /// Total duration in milliseconds
    pub duration_ms: f64,
    /// Peak memory usage in MB
    pub peak_memory_mb: f64,
    /// Text F1 score (optional)
    pub f1_text: Option<f64>,
    /// Layout F1 score (optional, only for markdown mode)
    pub f1_layout: Option<f64>,
    /// Numeric F1 score (optional)
    pub f1_numeric: Option<f64>,
    /// Overall quality score (optional)
    pub quality_score: Option<f64>,
    /// Whether extraction was correct (optional)
    pub correct: Option<bool>,
    /// Whether extraction succeeded
    pub success: bool,
    /// Error kind if failed (optional)
    pub error_kind: Option<String>,

    /// File size in bytes of the source document (v2.8.0+).
    #[serde(default)]
    pub file_size: u64,
    /// Raw throughput in bytes/sec, prior to the `peak_memory_mb`-style MB conversion
    /// used elsewhere on this row (v2.8.0+).
    #[serde(default)]
    pub throughput_bytes_per_sec: f64,
    /// Average CPU usage percentage (0-100) for this extraction (v2.8.0+).
    #[serde(default)]
    pub avg_cpu_percent: f64,
    /// Total process-tree CPU-time consumed, in core-seconds (v2.8.0+).
    #[serde(default)]
    pub cpu_seconds: f64,
    /// RSS captured immediately after the monitor attached to the target (v2.8.0+).
    #[serde(default)]
    pub baseline_memory_bytes: u64,
    /// Peak RSS above the captured baseline (v2.8.0+).
    #[serde(default)]
    pub peak_memory_delta_bytes: u64,
    /// How `peak_memory_mb` was measured; see `crate::types::MemoryMethod` (v2.12.0+).
    #[serde(default)]
    pub memory_method: crate::types::MemoryMethod,
    /// Framework-reported peak RSS, diagnostics only; never folded into `peak_memory_mb`
    /// (v2.12.0+).
    #[serde(default)]
    pub self_reported_peak_memory_bytes: Option<u64>,
    /// 50th percentile memory usage in bytes, from this single measurement's own resource
    /// sampler timeline — not a cross-fixture percentile (v2.8.0+).
    #[serde(default)]
    pub p50_memory_bytes: u64,
    /// 95th percentile memory usage in bytes (see `p50_memory_bytes`) (v2.8.0+).
    #[serde(default)]
    pub p95_memory_bytes: u64,
    /// 99th percentile memory usage in bytes (see `p50_memory_bytes`) (v2.8.0+).
    #[serde(default)]
    pub p99_memory_bytes: u64,
    /// Pure extraction time reported by the framework, in milliseconds (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction_duration_ms: Option<f64>,
    /// Subprocess overhead outside framework-reported extraction work, in milliseconds
    /// (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subprocess_overhead_ms: Option<f64>,
    /// Cold start duration, in milliseconds (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cold_start_duration_ms: Option<f64>,
    /// Free-text error message, when the extraction failed (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Full quality metrics, including the token-level `missing_tokens`/`extra_tokens` detail
    /// that has no scalar equivalent among this row's `f1_*`/`quality_score`/`correct` fields
    /// (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<crate::types::QualityMetrics>,
    /// PDF-specific metadata (text layer detection, OCR strategy), when the fixture is a PDF
    /// (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_metadata: Option<crate::types::PdfMetadata>,
    /// Framework capability metadata as reported at the time of this extraction, including
    /// `batch_capability` (entry point/timing scope), which has no other home in this schema
    /// (v2.8.0+).
    #[serde(default)]
    pub framework_capabilities: crate::types::FrameworkCapabilities,
    /// System load captured at measurement time, when recorded (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_load: Option<crate::system_load::SystemLoad>,
    /// Per-iteration results, when multiple iterations were run for this fixture (v2.8.0+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<crate::types::IterationResult>,
    /// Statistical analysis of durations across iterations, when multiple iterations were run
    /// (v2.8.0+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<crate::types::DurationStatistics>,
}

/// Cross-framework comparison rankings and deltas.
///
/// Quality-based ranking values, including Pareto SF1, are the reported median multiplied by
/// accountable success coverage. Raw quality percentiles remain available in
/// [`PerformancePercentiles::quality`]. ~keep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonData {
    /// Frameworks ranked by median throughput (highest first)
    pub throughput_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by median memory usage (lowest first)
    pub memory_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by quality score (highest first) — markdown only. Plaintext-only
    /// frameworks are never scored against layout-inclusive quality, so they are excluded
    /// here (see module-level docs).
    pub quality_ranking_markdown: Vec<RankedFramework>,
    /// Frameworks ranked by quality score (highest first) — plaintext only.
    pub quality_ranking_plaintext: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by overall quality score (highest first) — markdown only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_quality_ranking_markdown: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by overall quality score (highest first) — plaintext only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_quality_ranking_plaintext: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by text F1 / TF1 (highest first) — markdown only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_tf1_ranking_markdown: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by text F1 / TF1 (highest first) — plaintext only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_tf1_ranking_plaintext: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by structural F1 / SF1 (highest first) — markdown only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_sf1_ranking_markdown: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by reading-order score (highest first) — markdown only. Report
    /// only: reading order is not part of `quality_score` (v2.12.0+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_reading_order_ranking_markdown: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by reading-order score (highest first) — plaintext only
    /// (v2.12.0+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_reading_order_ranking_plaintext: Vec<RankedFramework>,
    /// Frameworks ranked by median pages/sec (highest first). Only frameworks with at least one
    /// PDF pages/sec observation are included (see `PerformancePercentiles.pages_per_sec`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages_per_sec_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by median CPU-seconds consumed (lowest first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cpu_seconds_ranking: Vec<RankedFramework>,
    /// Performance deltas relative to the fastest framework (throughput-based)
    pub deltas_vs_baseline: HashMap<String, DeltaMetrics>,
    /// Non-dominated frontier over (pages/sec ↑, SF1 ↑, peak-RSS ↓), markdown frameworks only.
    /// See [`ParetoPoint`] for the dominance rule and eligibility criteria.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pareto_frontier: Vec<ParetoPoint>,
    /// Frameworks that were attempted (present in `by_framework_mode`) but produced no ranked
    /// entry in one or more of `throughput_ranking`/`memory_ranking`/`cpu_seconds_ranking`/
    /// `pages_per_sec_ranking`/`pareto_frontier`, with a human-readable reason.
    ///
    /// Every one of those rankings is gated on having at least one usable performance sample; a
    /// framework whose every row failed or was reclassified simply has none, and used to vanish
    /// from all five with nothing in this struct recording that it ran at all — a reader saw only
    /// however many frameworks made it into the charts, with no way to tell a framework was
    /// attempted and excluded from one that was never run. This makes that absence explicit
    /// instead of silent (Defect S4). ~keep
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unranked_frameworks: Vec<UnrankedFramework>,
}

/// One framework excluded from a ranking, with why. See [`ComparisonData::unranked_frameworks`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrankedFramework {
    /// Framework:mode key, matching `RankedFramework.framework_mode`.
    pub framework_mode: String,
    /// Human-readable reason this framework produced no ranked entry.
    pub reason: String,
}

/// One non-dominated point in the (pages/sec, SF1, peak-RSS) multi-objective comparison.
///
/// A candidate is on the frontier when no other candidate **dominates** it: dominance requires
/// being at least as good on every objective and strictly better on at least one.
/// `pages_per_sec` and `sf1` are maximized; `peak_memory_mb` is minimized.
///
/// Restricted to markdown frameworks that have both an SF1 term and at least one pages/sec
/// observation: plaintext-only frameworks never carry SF1 (see module-level docs), and a
/// framework with no PDF page-count data has no pages/sec axis to compare on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoPoint {
    /// Framework:mode key, matching `RankedFramework.framework_mode`.
    pub framework_mode: String,
    /// Median pages/sec (higher is better).
    pub pages_per_sec: f64,
    /// Coverage-adjusted median structural F1 / SF1 (higher is better).
    pub sf1: f64,
    /// Median peak RSS in MB (lower is better).
    pub peak_memory_mb: f64,
}

/// A framework entry in a ranking.
///
/// For `throughput_ranking`, `memory_ranking`, `cpu_seconds_ranking`, and `pages_per_sec_ranking`,
/// `rank` and `relative` are scoped to this entry's own `(output_format, mode)` segment (see
/// [`Self::output_format`] and [`Self::mode`]) — `rank == 1` / `relative == 1.0` mean "best within
/// this segment," not "best overall." Markdown vs plaintext serialization cost and single-file vs
/// batch-amortized process overhead are not comparable, so these four rankings never pool across
/// segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedFramework {
    /// Framework:mode key (e.g., "xberg-markdown-baseline:single" or "docling:markdown:single")
    pub framework_mode: String,
    /// Rank (1-based)
    pub rank: usize,
    /// The metric value used for ranking
    pub value: f64,
    /// Ratio relative to the best in this ranking (1.0 = best)
    pub relative: f64,
    /// True when this framework:mode is sourced from a cell the release contract marks
    /// `optional` (best-effort, e.g. MinerU — see [`crate::bench_matrix::MatrixEntry::optional`]).
    /// Optional cells can be partially failed or under-sampled relative to the pinned corpus and
    /// still land in a ranking with no distinguishing flag; consumers should not treat an
    /// optional entry's rank as directly comparable to a contract-verified one. `#[serde(default)]`
    /// so aggregates produced before this field existed still deserialize (defaults to `false`,
    /// i.e. contract-verified, which is correct for every pre-existing ranking entry).
    #[serde(default)]
    pub optional: bool,
    /// Output format of the `(output_format, mode)` segment `rank`/`relative` are scoped to.
    /// `#[serde(default)]` so aggregates produced before this field existed still deserialize.
    #[serde(default)]
    pub output_format: OutputFormat,
    /// Mode ("single", "batch", "sync", "async") of the `(output_format, mode)` segment
    /// `rank`/`relative` are scoped to. `#[serde(default)]` so aggregates produced before this
    /// field existed still deserialize (defaults to `""`).
    #[serde(default)]
    pub mode: String,
}

/// Performance deltas relative to baseline (highest throughput framework)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaMetrics {
    /// Throughput delta in MB/s (negative = slower than baseline)
    pub throughput_delta_mbs: f64,
    /// Throughput delta as percentage relative to baseline
    pub throughput_delta_percent: f64,
    /// Memory delta in MB (positive = more memory than baseline)
    pub memory_delta_mb: f64,
    /// Memory delta as percentage relative to baseline
    pub memory_delta_percent: f64,
}

/// Metadata about the consolidation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationMetadata {
    /// Number of benchmark results included
    pub total_results: usize,
    /// Number of unique frameworks
    pub framework_count: usize,
    /// Number of unique file types
    pub file_type_count: usize,
    /// File types the "overall" markdown quality ranking is actually computed over: the
    /// intersection of file types every markdown candidate framework attempted. When this
    /// degenerates to a single type (e.g. `["pdf"]`, because a PDF-only framework like
    /// liteparse/mineru is in the pool), `quality_ranking_markdown` is NOT a true all-format
    /// "overall" ranking — it reflects only these types. Consumers must read it accordingly.
    #[serde(default)]
    pub shared_corpus_markdown: Vec<String>,
    /// File types the "overall" plaintext quality ranking is computed over. Same semantics as
    /// [`Self::shared_corpus_markdown`].
    #[serde(default)]
    pub shared_corpus_plaintext: Vec<String>,
    /// Timestamp of consolidation
    pub timestamp: String,
    /// Frameworks for which two or more results reported a different `installation_size`
    /// (`disk_sizes` keeps only the last-seen value per framework). Empty in the overwhelmingly
    /// common case where a framework's installation size is stable across every result that
    /// reports it (v2.8.0+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disk_size_conflicts: Vec<String>,
}

/// Failure counts split by cause. Framework-fault kinds ([`crate::types::ErrorKind::FrameworkError`],
/// [`crate::types::ErrorKind::EmptyContent`], [`crate::types::ErrorKind::ZeroOverlap`],
/// [`crate::types::ErrorKind::LayoutFailed`], [`crate::types::ErrorKind::Timeout`]) are the framework's own fault — it was handed a supported
/// document and failed — and penalize its quality/success rate (see
/// [`super::failures::is_framework_fault_failure`]). Infrastructure kinds
/// ([`crate::types::ErrorKind::HarnessError`], [`crate::types::ErrorKind::ConfigSetupError`]) are
/// our own harness's fault and never penalize a framework.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureCounts {
    pub framework_errors: usize,
    pub empty_content: usize,
    /// Non-empty output that shared zero tokens with a non-empty ground truth — distinct from
    /// `empty_content` (which means the framework produced no content at all).
    #[serde(default)]
    pub zero_overlap: usize,
    /// Layout-bearing rows whose layout pass failed or was skipped.
    #[serde(default)]
    pub layout_failures: usize,
    pub timeouts: usize,
    /// Sum of the framework-fault kinds above (these penalize the score).
    pub framework_fault_total: usize,
    pub harness_errors: usize,
    pub config_setup_errors: usize,
    /// Sum of the infrastructure kinds above (these never penalize the score).
    pub infra_total: usize,
}

/// Cohort-wide failure roll-up: the same per-framework-mode error counts that live on each
/// [`PerformancePercentiles`], summed to the cohort level and broken out per framework-mode and per
/// file type, with the framework-fault vs infrastructure split preserved throughout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureSummary {
    /// Every failure in the cohort, across all frameworks and documents.
    pub total: FailureCounts,
    /// Failures keyed by aggregate framework-mode key (as in `by_framework_mode`).
    pub by_framework_mode: std::collections::BTreeMap<String, FailureCounts>,
    /// Failures keyed by document file extension, summed across frameworks.
    pub by_file_type: std::collections::BTreeMap<String, FailureCounts>,
}

/// Aggregated results for a specific framework, output format, and mode combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkModeAggregation {
    /// Framework name (base name without mode suffix)
    pub framework: String,
    /// Output format (markdown or plaintext)
    pub output_format: OutputFormat,
    /// Mode: "single", "batch", "sync", "async"
    pub mode: String,
    /// Process lifecycle shared by every result in this aggregation. ~keep
    #[serde(default = "crate::types::unknown_timing_regime")]
    pub timing_regime: TimingRegime,
    /// Cold start duration statistics (if available)
    pub cold_start: Option<DurationPercentiles>,
    /// Process metrics deduplicated across all file-type and OCR buckets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overall_performance: Option<PerformancePercentiles>,
    /// Results grouped by file type
    pub by_file_type: HashMap<String, FileTypeAggregation>,
}

/// Aggregated results for a specific file type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeAggregation {
    /// File type (extension)
    pub file_type: String,
    /// Results without OCR
    pub no_ocr: Option<PerformancePercentiles>,
    /// Results with OCR
    pub with_ocr: Option<PerformancePercentiles>,
}

/// Performance percentiles for a group of results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePercentiles {
    /// Number of successful document samples used for quality calculations.
    pub successful_sample_count: usize,
    /// Number of process-level samples used for duration, throughput, and RSS.
    ///
    /// Native batches contribute one sample regardless of document cardinality.
    #[serde(default)]
    pub performance_sample_count: usize,
    /// Total number of samples in this group (including failed)
    pub total_sample_count: usize,
    /// Number of framework-side extraction errors (not our fault)
    pub framework_errors: usize,
    /// Number of harness-side errors (potentially our fault)
    pub harness_errors: usize,
    /// Number of configuration/setup errors (missing dependencies, env issues)
    pub config_setup_errors: usize,
    /// Number of extractions that timed out
    pub timeouts: usize,
    /// Number of extractions that returned empty content
    pub empty_content: usize,
    /// Number of extractions that returned non-empty output sharing zero tokens with a
    /// non-empty ground truth — distinct from `empty_content` (v2.10.0+).
    #[serde(default)]
    pub zero_overlap: usize,
    /// Number of layout-bearing extractions whose layout pass failed or was skipped.
    #[serde(default)]
    pub layout_failures: usize,
    /// Unique error messages with occurrence counts
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub error_details: HashMap<String, usize>,
    /// Throughput percentiles (p50, p95, p99) in MB/s
    pub throughput: Percentiles,
    /// Memory percentiles (p50, p95, p99) in MB
    pub memory: Percentiles,
    /// Duration percentiles (p50, p95, p99) in ms
    pub duration: Percentiles,
    /// Success rate as percentage (0-100)
    pub success_rate_percent: f64,
    /// Extraction duration percentiles (p50, p95, p99) in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_duration: Option<Percentiles>,
    /// Quality score percentiles (p50, p95, p99) — 0.0 to 1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<QualityPercentiles>,
    /// Pages-per-second percentiles, derived from `PdfMetadata.page_count` divided by wall-clock
    /// duration. `None` when no result in this group carries a known PDF page count (e.g.
    /// non-PDF file types, or a page count the harness could not detect). For a native batch,
    /// the page counts of every document sharing one `batch_sample_id` are summed and divided by
    /// the shared batch makespan, mirroring how `throughput` is computed for batches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages_per_sec: Option<Percentiles>,
    /// Total process-tree CPU-time percentiles, in core-seconds
    /// (see `PerformanceMetrics::cpu_seconds` for the integration methodology and its
    /// sample-interval-bounded precision).
    #[serde(default)]
    pub cpu_seconds: Percentiles,
    /// Approximate number of documents processed per one measured process invocation in this
    /// group: `Some(1)` for single-file mode; for batch mode, the modal document count per
    /// deduped performance sample (`total_sample_count / performance_sample_count`, rounded).
    /// `None` when the group has no successful performance samples to derive a ratio from.
    /// Surfaced so peak-RSS (and other performance metrics) can be read "keyed by batch size"
    /// without adding a new axis to the `by_framework_mode` aggregate key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
    /// System-load contention qualifier aggregated from `BenchmarkResult.system_load` samples in
    /// this group. `None` when no result in the group carries a load snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_load: Option<SystemLoadPercentiles>,
    /// Number of successful performance samples excluded from the `throughput` percentiles
    /// because their `throughput_bytes_per_sec` was zero, negative, or non-finite (v2.8.0+).
    ///
    /// The exclusion itself is unchanged from pre-v2.8.0 behavior (throughput percentiles have
    /// always required a positive, finite value); this field only makes the exclusion visible
    /// instead of silent. A nonzero count does not necessarily indicate a problem — for example
    /// a batch's non-anchor rows legitimately report `0.0` throughput (see
    /// `crate::types::successful_performance_samples`) — but it lets a consumer distinguish "no
    /// samples" from "some samples, all excluded."
    #[serde(default)]
    pub throughput_excluded_sample_count: usize,
    /// Performance samples per `memory_method` (`rusage_max_rss`, `sampled`, `self_reported`) behind
    /// `memory`, so a bucket mixing measurement methods is visible (v2.12.0+).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub memory_methods: std::collections::BTreeMap<String, usize>,
}

/// Aggregated system-load contention qualifier for a group of results.
///
/// Lets a consumer judge whether a bucket's timing data is comparable to an idle-machine
/// baseline: see `crate::system_load::SystemLoad` for why load figures are read *relatively*
/// (was this bucket measured under similar or worse contention than another) rather than as an
/// absolute number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoadPercentiles {
    /// 50th percentile of `SystemLoad::load_per_core()` across the group's samples.
    pub load_per_core_p50: f64,
    /// 95th percentile of `SystemLoad::load_per_core()` across the group's samples.
    pub load_per_core_p95: f64,
    /// Number of samples for which `SystemLoad::is_contended()` was true.
    pub contended_sample_count: usize,
    /// Total number of results in the group carrying a system-load snapshot.
    pub total_sample_count: usize,
}

/// Quality percentile values (p50, p95, p99) for all F1 metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPercentiles {
    /// Text F1 50th percentile (TF1 median)
    pub f1_text_p50: f64,
    /// Text F1 95th percentile
    pub f1_text_p95: f64,
    /// Text F1 99th percentile
    pub f1_text_p99: f64,
    /// Numeric F1 50th percentile
    pub f1_numeric_p50: f64,
    /// Numeric F1 95th percentile
    pub f1_numeric_p95: f64,
    /// Numeric F1 99th percentile
    pub f1_numeric_p99: f64,
    /// Layout/structural F1 50th percentile (SF1 median) — None for plaintext-only frameworks
    pub f1_layout_p50: Option<f64>,
    /// Layout/structural F1 95th percentile — None for plaintext-only frameworks
    pub f1_layout_p95: Option<f64>,
    /// Layout/structural F1 99th percentile — None for plaintext-only frameworks
    pub f1_layout_p99: Option<f64>,
    /// Overall quality score 50th percentile
    pub quality_score_p50: f64,
    /// Overall quality score 95th percentile
    pub quality_score_p95: f64,
    /// Overall quality score 99th percentile
    pub quality_score_p99: f64,
    /// Reading-order score 50th percentile over the results that carry one; report-only, not part
    /// of `quality_score`. `None` when no result carries it (v2.12.0+).
    #[serde(default)]
    pub reading_order_p50: Option<f64>,
    /// Number of results whose `quality.reading_order_score` is present (v2.12.0+).
    #[serde(default)]
    pub reading_order_sample_count: usize,
}

/// Minimum sample count required to report a `p95` value without fabricating precision the
/// sample can't support. R-7 interpolation places the percentile at index `p * (n - 1)`; below
/// roughly `1 / (1 - p)` samples that index coincides with (or sits right next to) the maximum
/// observed value, so the "percentile" is really just the largest sample or two wearing a
/// statistical label. Real benchmark cohorts are frequently 4-8 fixtures — well under this
/// threshold — so `p95` is `None` rather than a fabricated number there (Defect S1). ~keep
pub const MIN_SAMPLES_FOR_P95: usize = 20;

/// As [`MIN_SAMPLES_FOR_P95`], for `p99` (`1 / (1 - 0.99) = 100` samples). (Defect S1) ~keep
pub const MIN_SAMPLES_FOR_P99: usize = 100;

/// Percentile values for a metric.
///
/// `p95`/`p99` are suppressed (`None`) rather than fabricated when `sample_count` is below the
/// statistically supported minimum for that percentile — see [`MIN_SAMPLES_FOR_P95`] and
/// [`MIN_SAMPLES_FOR_P99`]. `sample_count` and `std_dev` are always reported so a reader can
/// judge the underlying distribution (and how much to trust it) even when p95/p99 are
/// suppressed. (Defect S1) ~keep
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Percentiles {
    /// 50th percentile (median). `0.0` when `sample_count == 0`.
    pub p50: f64,
    /// 95th percentile, or `None` when `sample_count < MIN_SAMPLES_FOR_P95`.
    pub p95: Option<f64>,
    /// 99th percentile, or `None` when `sample_count < MIN_SAMPLES_FOR_P99`.
    pub p99: Option<f64>,
    /// Number of values this percentile group was computed from.
    #[serde(default)]
    pub sample_count: usize,
    /// Sample standard deviation (Bessel-corrected) of the underlying values. `0.0` for
    /// `sample_count <= 1`. A dispersion measure that stays meaningful even when the sample
    /// count is too small to support p95/p99.
    #[serde(default)]
    pub std_dev: f64,
}

/// Duration percentiles in milliseconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationPercentiles {
    /// Number of samples with cold start data
    pub sample_count: usize,
    /// 50th percentile (median) in ms
    pub p50_ms: f64,
    /// 95th percentile in ms
    pub p95_ms: f64,
    /// 99th percentile in ms
    pub p99_ms: f64,
}
