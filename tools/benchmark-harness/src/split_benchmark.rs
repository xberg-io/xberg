//! Split-boundary evaluation for multi-document PDFs (`split_and_extract`, #1186).
//!
//! The core [`xberg::split_and_extract`] partitions one PDF into per-sub-document
//! segments from a single parse. Two strategies exist: caller-supplied
//! `PageRanges` (deterministic) and `Auto` (heuristic boundary detection via
//! [`xberg::heuristics::multidoc`]). This harness measures three things per
//! fixture:
//!
//! 1. **Boundary accuracy** — precision / recall / F1 of `Auto`'s detected
//!    segment starts against a ground-truth manifest. This is the signal that
//!    drives `MultidocThresholds` tuning. Detection is scored by calling
//!    [`boundaries_from_extraction_result`] directly with each candidate
//!    threshold set, so a full sweep costs **one parse per document**
//!    (extract-once / score-many).
//! 2. **Reconstruction fidelity** — running `split_and_extract` with the GT page
//!    ranges and concatenating the segment text must reproduce a single
//!    whole-document extraction (text F1 ≈ 1.0). This guards the single-parse
//!    partition against dropping or reordering content.
//! 3. **Single-parse win** — wall-clock of one `split_and_extract` call vs a
//!    naive baseline that re-parses the whole PDF once per segment.
//!
//! Ground truth is a `*.split.json` sidecar next to each fixture PDF:
//!
//! ```json
//! { "document": "three_docs_a.pdf",
//!   "boundaries": [ {"start_page":1,"end_page":1},
//!                   {"start_page":2,"end_page":9},
//!                   {"start_page":10,"end_page":11} ] }
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::quality::compute_quality;
use xberg::core::config::PageConfig;
use xberg::heuristics::multidoc::{
    BoundaryReason, DocumentBoundary, MultidocThresholds, boundaries_from_extraction_result,
};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, SplitConfig, SplitStrategy, split_and_extract};

/// Threshold sweep grid: `density_shift_threshold` candidates.
const DENSITY_GRID: &[f32] = &[0.20, 0.25, 0.30, 0.35, 0.40];
/// Threshold sweep grid: `bigram_overlap_min` candidates.
const OVERLAP_GRID: &[f32] = &[0.05, 0.10, 0.15, 0.20];

/// One ground-truth page span in the split manifest (1-indexed, inclusive).
#[derive(Debug, Clone, Copy, Deserialize)]
struct GtBoundary {
    start_page: u32,
    end_page: u32,
}

/// A `*.split.json` ground-truth manifest.
#[derive(Debug, Clone, Deserialize)]
struct SplitManifest {
    /// Path to the PDF, relative to the manifest's directory.
    document: String,
    /// True sub-document page spans, in order.
    boundaries: Vec<GtBoundary>,
}

/// A resolved split fixture: manifest plus the absolute PDF path.
#[derive(Debug, Clone)]
struct SplitFixture {
    name: String,
    document_path: PathBuf,
    boundaries: Vec<GtBoundary>,
}

/// Configuration for the split-boundary benchmark.
#[derive(Debug, Clone)]
pub struct SplitBenchmarkConfig {
    /// Directory scanned recursively for `*.split.json` manifests.
    pub fixtures_dir: PathBuf,
    /// Run the `MultidocThresholds` sweep grid in addition to the default pass.
    pub sweep: bool,
    /// If set, write a `split-boundary-guardrails.json` at this path from the
    /// default-threshold results (`min_f1` = `0.9 ×` observed).
    pub guardrails_out: Option<PathBuf>,
}

impl Default for SplitBenchmarkConfig {
    fn default() -> Self {
        Self {
            fixtures_dir: PathBuf::from("tools/benchmark-harness/fixtures/split"),
            sweep: false,
            guardrails_out: None,
        }
    }
}

/// Per-document split benchmark result under the default thresholds.
#[derive(Debug, Clone)]
pub struct SplitDocResult {
    pub name: String,
    pub pages: u32,
    /// Number of internal GT boundaries (segments − 1).
    pub gt_internal: usize,
    /// Internal boundaries `Auto` actually detected (page 1 excluded).
    pub detected_internal: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    /// Text F1 of concatenated GT-range segments vs whole-doc extraction.
    pub reconstruction_tf1: f64,
    /// Whether every segment's `counts` matched its partitioned collections.
    pub counts_ok: bool,
    /// Wall-clock of one `split_and_extract(PageRanges(gt))` call.
    pub split_ms: f64,
    /// Wall-clock of re-parsing the whole PDF once per GT segment.
    pub naive_ms: f64,
    /// Number of GT segments.
    pub segments: usize,
}

/// One cell of the threshold sweep: aggregate F1 across all fixtures.
#[derive(Debug, Clone, Copy)]
pub struct SweepCell {
    pub density_shift_threshold: f32,
    pub bigram_overlap_min: f32,
    pub mean_f1: f64,
}

/// Guardrail contract for a single split fixture.
#[derive(Debug, Clone, Serialize)]
struct BoundaryContract {
    doc: String,
    min_f1: f64,
}

/// `split-boundary-guardrails.json` schema (mirrors `guardrails.json`).
#[derive(Debug, Clone, Serialize)]
struct BoundaryGuardrails {
    version: String,
    threshold_factor: f64,
    contracts: Vec<BoundaryContract>,
}

/// Build a `PageConfig` with per-page content extraction forced on — the same
/// parse configuration `split_and_extract` uses internally, so the document we
/// score boundaries on matches what `Auto` sees in production.
fn pages_on_config() -> ExtractionConfig {
    ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Fold detected boundaries into contiguous segment-start pages.
///
/// Mirrors the private `fold_boundaries_into_ranges` in `core/split.rs`: only
/// real boundary reasons seed a new segment; page 1 always starts one.
fn segment_starts(boundaries: &[DocumentBoundary], total_pages: u32) -> BTreeSet<u32> {
    let mut starts: BTreeSet<u32> = boundaries
        .iter()
        .filter(|b| {
            matches!(
                b.reason,
                BoundaryReason::PageOneMarker | BoundaryReason::LetterheadReset | BoundaryReason::DensityShift
            )
        })
        .map(|b| b.start_page)
        .filter(|&p| (1..=total_pages).contains(&p))
        .collect();
    starts.insert(1);
    starts
}

/// Precision / recall / F1 over *internal* segment-start pages (page 1 excluded
/// — every document trivially starts a segment there).
fn boundary_prf1(predicted: &BTreeSet<u32>, truth: &BTreeSet<u32>) -> (f64, f64, f64) {
    let pred: BTreeSet<u32> = predicted.iter().copied().filter(|&p| p != 1).collect();
    let gt: BTreeSet<u32> = truth.iter().copied().filter(|&p| p != 1).collect();

    // Both empty → a single-document PDF correctly left unsplit: vacuously perfect.
    if pred.is_empty() && gt.is_empty() {
        return (1.0, 1.0, 1.0);
    }
    let tp = pred.intersection(&gt).count() as f64;
    let precision = if pred.is_empty() { 1.0 } else { tp / pred.len() as f64 };
    let recall = if gt.is_empty() { 1.0 } else { tp / gt.len() as f64 };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    (precision, recall, f1)
}

/// Load every `*.split.json` manifest under `dir` and resolve its PDF path.
fn load_split_fixtures(dir: &Path) -> Result<Vec<SplitFixture>> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".split.json"))
            {
                out.push(path);
            }
        }
        Ok(())
    }

    let mut manifest_paths = Vec::new();
    if dir.is_dir() {
        walk(dir, &mut manifest_paths).map_err(Error::Io)?;
    }
    manifest_paths.sort();

    let mut fixtures = Vec::new();
    for manifest_path in manifest_paths {
        let raw = std::fs::read_to_string(&manifest_path).map_err(Error::Io)?;
        let manifest: SplitManifest = serde_json::from_str(&raw)
            .map_err(|e| Error::Benchmark(format!("invalid split manifest {}: {e}", manifest_path.display())))?;
        let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        let document_path = base.join(&manifest.document);
        let name = manifest_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.trim_end_matches(".split.json").to_string())
            .unwrap_or_default();
        fixtures.push(SplitFixture {
            name,
            document_path,
            boundaries: manifest.boundaries,
        });
    }
    Ok(fixtures)
}

/// Extract the whole document once (pages forced on) for boundary scoring.
async fn extract_whole(bytes: &[u8]) -> Result<ExtractedDocument> {
    let input = ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None);
    let output = xberg::extract(input, &pages_on_config())
        .await
        .map_err(|e| Error::Benchmark(format!("whole-document extraction failed: {e}")))?;
    output
        .results
        .into_iter()
        .next()
        .ok_or_else(|| Error::Benchmark("extraction produced no document".to_string()))
}

/// Run the split-boundary benchmark under default thresholds.
pub async fn run_split_benchmark(config: &SplitBenchmarkConfig) -> Result<Vec<SplitDocResult>> {
    let fixtures = load_split_fixtures(&config.fixtures_dir)?;
    eprintln!(
        "Split benchmark: {} fixture(s) under {}",
        fixtures.len(),
        config.fixtures_dir.display()
    );

    let mut results = Vec::new();
    for fixture in &fixtures {
        let bytes = match std::fs::read(&fixture.document_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  SKIP {} (read failed: {e})", fixture.name);
                continue;
            }
        };

        let doc = match extract_whole(&bytes).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  SKIP {} ({e})", fixture.name);
                continue;
            }
        };
        let total_pages = doc.pages.as_ref().map_or(0, Vec::len) as u32;
        if total_pages == 0 {
            eprintln!("  SKIP {} (not page-addressable)", fixture.name);
            continue;
        }

        // --- Boundary accuracy under default thresholds ---
        let gt_starts: BTreeSet<u32> = fixture.boundaries.iter().map(|b| b.start_page).collect();
        let boundaries = boundaries_from_extraction_result(&doc, &MultidocThresholds::default());
        let predicted = segment_starts(&boundaries, total_pages);
        let (precision, recall, f1) = boundary_prf1(&predicted, &gt_starts);

        // --- Reconstruction fidelity + single-parse timing (GT ranges) ---
        let gt_ranges: Vec<std::ops::RangeInclusive<u32>> =
            fixture.boundaries.iter().map(|b| b.start_page..=b.end_page).collect();
        let split_cfg = SplitConfig {
            strategy: SplitStrategy::PageRanges(gt_ranges.clone()),
            ..Default::default()
        };

        let t = Instant::now();
        let segments = split_and_extract(&bytes, &split_cfg)
            .await
            .map_err(|e| Error::Benchmark(format!("split_and_extract failed on {}: {e}", fixture.name)))?;
        let split_ms = t.elapsed().as_secs_f64() * 1000.0;

        let reconstructed = segments
            .iter()
            .map(|s| s.document.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let reconstruction_tf1 = compute_quality(&reconstructed, &doc.content).f1_score_text;

        // Per-segment counts must match the partitioned collections.
        let counts_ok = segments.iter().all(|s| {
            let d = &s.document;
            d.counts.pages == d.pages.as_ref().map_or(0, Vec::len)
                && d.counts.tables == d.tables.len()
                && d.counts.images == d.images.as_ref().map_or(0, Vec::len)
        });

        // Naive baseline: re-parse the whole PDF once per GT segment.
        let t = Instant::now();
        for _ in 0..gt_ranges.len() {
            let _ = extract_whole(&bytes).await?;
        }
        let naive_ms = t.elapsed().as_secs_f64() * 1000.0;

        results.push(SplitDocResult {
            name: fixture.name.clone(),
            pages: total_pages,
            gt_internal: gt_starts.iter().filter(|&&p| p != 1).count(),
            detected_internal: predicted.iter().filter(|&&p| p != 1).count(),
            precision,
            recall,
            f1,
            reconstruction_tf1,
            counts_ok,
            split_ms,
            naive_ms,
            segments: segments.len(),
        });
    }

    if let Some(path) = &config.guardrails_out {
        write_boundary_guardrails(&results, path)?;
    }

    Ok(results)
}

/// Run the `MultidocThresholds` sweep grid, scoring aggregate boundary F1 per
/// cell with one parse per document (extract-once / score-many).
pub async fn run_threshold_sweep(config: &SplitBenchmarkConfig) -> Result<Vec<SweepCell>> {
    let fixtures = load_split_fixtures(&config.fixtures_dir)?;

    // Parse every fixture once; keep (doc, total_pages, gt_starts) in memory.
    let mut parsed: Vec<(ExtractedDocument, u32, BTreeSet<u32>)> = Vec::new();
    for fixture in &fixtures {
        let Ok(bytes) = std::fs::read(&fixture.document_path) else {
            continue;
        };
        let Ok(doc) = extract_whole(&bytes).await else { continue };
        let total_pages = doc.pages.as_ref().map_or(0, Vec::len) as u32;
        if total_pages == 0 {
            continue;
        }
        let gt_starts: BTreeSet<u32> = fixture.boundaries.iter().map(|b| b.start_page).collect();
        parsed.push((doc, total_pages, gt_starts));
    }

    let mut cells = Vec::new();
    for &density in DENSITY_GRID {
        for &overlap in OVERLAP_GRID {
            let thresholds = MultidocThresholds {
                density_shift_threshold: density,
                bigram_overlap_min: overlap,
            };
            let mut f1_sum = 0.0;
            for (doc, total_pages, gt_starts) in &parsed {
                let boundaries = boundaries_from_extraction_result(doc, &thresholds);
                let predicted = segment_starts(&boundaries, *total_pages);
                let (_, _, f1) = boundary_prf1(&predicted, gt_starts);
                f1_sum += f1;
            }
            let mean_f1 = if parsed.is_empty() {
                0.0
            } else {
                f1_sum / parsed.len() as f64
            };
            cells.push(SweepCell {
                density_shift_threshold: density,
                bigram_overlap_min: overlap,
                mean_f1,
            });
        }
    }
    Ok(cells)
}

/// Write a `split-boundary-guardrails.json` from default-threshold results.
fn write_boundary_guardrails(results: &[SplitDocResult], path: &Path) -> Result<()> {
    const THRESHOLD_FACTOR: f64 = 0.9;
    let contracts = results
        .iter()
        .map(|r| BoundaryContract {
            doc: r.name.clone(),
            min_f1: (r.f1 * THRESHOLD_FACTOR * 1000.0).round() / 1000.0,
        })
        .collect();
    let guardrails = BoundaryGuardrails {
        version: "1.0".to_string(),
        threshold_factor: THRESHOLD_FACTOR,
        contracts,
    };
    let json = serde_json::to_string_pretty(&guardrails)
        .map_err(|e| Error::Benchmark(format!("failed to serialize boundary guardrails: {e}")))?;
    std::fs::write(path, json).map_err(Error::Io)?;
    eprintln!(
        "Wrote {} boundary contract(s) to {}",
        guardrails.contracts.len(),
        path.display()
    );
    Ok(())
}

/// Print the per-document default-threshold results table.
pub fn print_split_table(results: &[SplitDocResult]) {
    eprintln!(
        "\n{:<22} {:>5} {:>7} {:>6} {:>6} {:>6} {:>8} {:>6} {:>9} {:>9}",
        "Document", "pages", "det/gt", "prec", "recall", "F1", "recon", "cnts", "split ms", "naive ms",
    );
    eprintln!("{}", "-".repeat(96));
    for r in results {
        eprintln!(
            "{:<22} {:>5} {:>7} {:>6.2} {:>6.2} {:>6.2} {:>8.3} {:>6} {:>9.0} {:>9.0}",
            truncate(&r.name, 22),
            r.pages,
            format!("{}/{}", r.detected_internal, r.gt_internal),
            r.precision,
            r.recall,
            r.f1,
            r.reconstruction_tf1,
            if r.counts_ok { "ok" } else { "BAD" },
            r.split_ms,
            r.naive_ms,
        );
    }
    if results.is_empty() {
        eprintln!("(no fixtures)");
        return;
    }
    let n = results.len() as f64;
    let mean = |f: &dyn Fn(&SplitDocResult) -> f64| results.iter().map(f).sum::<f64>() / n;
    let mean_f1 = mean(&|r| r.f1);
    let mean_recon = mean(&|r| r.reconstruction_tf1);
    let total_split: f64 = results.iter().map(|r| r.split_ms).sum();
    let total_naive: f64 = results.iter().map(|r| r.naive_ms).sum();
    let speedup = if total_split > 0.0 {
        total_naive / total_split
    } else {
        0.0
    };
    let counts_all_ok = results.iter().all(|r| r.counts_ok);
    eprintln!("{}", "-".repeat(96));
    eprintln!(
        "mean boundary F1 = {mean_f1:.3}   mean reconstruction TF1 = {mean_recon:.3}   \
         counts {}   single-parse speedup = {speedup:.2}x",
        if counts_all_ok { "OK" } else { "FAILED" },
    );
}

/// Print the threshold sweep grid, highlighting the best cell.
pub fn print_sweep_table(cells: &[SweepCell]) {
    eprintln!("\nMultidocThresholds sweep (mean boundary F1):");
    eprintln!(
        "{:>22} | {}",
        "density \\ overlap",
        OVERLAP_GRID
            .iter()
            .map(|o| format!("{o:>7.2}"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    eprintln!("{}", "-".repeat(24 + OVERLAP_GRID.len() * 8));
    for &density in DENSITY_GRID {
        let row = OVERLAP_GRID
            .iter()
            .map(|&overlap| {
                cells
                    .iter()
                    .find(|c| c.density_shift_threshold == density && c.bigram_overlap_min == overlap)
                    .map(|c| format!("{:>7.3}", c.mean_f1))
                    .unwrap_or_else(|| "     -".to_string())
            })
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!("{density:>22.2} | {row}");
    }
    if let Some(best) = cells.iter().max_by(|a, b| a.mean_f1.total_cmp(&b.mean_f1)) {
        eprintln!(
            "\nBest: density_shift_threshold = {:.2}, bigram_overlap_min = {:.2}  (mean F1 = {:.3})",
            best.density_shift_threshold, best.bigram_overlap_min, best.mean_f1,
        );
        eprintln!("Default: density_shift_threshold = 0.30, bigram_overlap_min = 0.10");
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() > max { &s[..max] } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prf1_perfect_when_both_empty() {
        let empty = BTreeSet::new();
        assert_eq!(boundary_prf1(&empty, &empty), (1.0, 1.0, 1.0));
    }

    #[test]
    fn prf1_page_one_is_ignored() {
        // Only page 1 predicted/truth → treated as empty internal sets → perfect.
        let starts: BTreeSet<u32> = [1].into_iter().collect();
        assert_eq!(boundary_prf1(&starts, &starts), (1.0, 1.0, 1.0));
    }

    #[test]
    fn prf1_exact_match_scores_one() {
        let pred: BTreeSet<u32> = [1, 3, 5].into_iter().collect();
        let truth: BTreeSet<u32> = [1, 3, 5].into_iter().collect();
        let (p, r, f1) = boundary_prf1(&pred, &truth);
        assert!((p - 1.0).abs() < 1e-9 && (r - 1.0).abs() < 1e-9 && (f1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn prf1_partial_detection() {
        // GT starts at 3 and 5; predicted only 3 (plus false positive 7).
        let pred: BTreeSet<u32> = [1, 3, 7].into_iter().collect();
        let truth: BTreeSet<u32> = [1, 3, 5].into_iter().collect();
        let (p, r, _f1) = boundary_prf1(&pred, &truth);
        assert!((p - 0.5).abs() < 1e-9, "precision: 1 of 2 predicted correct");
        assert!((r - 0.5).abs() < 1e-9, "recall: 1 of 2 truth found");
    }

    #[test]
    fn segment_starts_always_includes_page_one() {
        let starts = segment_starts(&[], 5);
        assert!(starts.contains(&1));
        assert_eq!(starts.len(), 1);
    }
}
