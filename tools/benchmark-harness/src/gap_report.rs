//! Per-document gap report.
//!
//! Pivots a `results.json` run (a flat `Vec<BenchmarkResult>`) by document and
//! ranks the documents where competitors (LiteParse, Docling) beat Xberg, split
//! by text quality (TF1) vs structure quality (SF1). This is the iteration
//! work-queue for Phase 1: each losing document is a concrete target, and the
//! TF1-vs-SF1 split says whether we are losing tokens (text extraction) or
//! structure (reading order / headings / tables).
//!
//! Outputs:
//! - `per_document.json` — the full pivot (every document, every framework).
//! - `gaps.md` — human-readable, ranked: where competitors beat our heuristics
//!   path, where `xberg-layout` trails Docling, and where we already win.

use crate::Result;
use crate::types::BenchmarkResult;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// One framework's scores for a single document.
#[derive(Debug, Clone, Serialize)]
pub struct FrameworkScore {
    pub framework: String,
    pub success: bool,
    /// Text token F1 (`f1_score_text`).
    pub tf1: Option<f64>,
    /// Numeric token F1 (`f1_score_numeric`).
    pub numeric_f1: Option<f64>,
    /// Structural F1 (`f1_score_layout`).
    pub sf1: Option<f64>,
    /// Combined quality (`quality_score`).
    pub combined: Option<f64>,
    pub duration_ms: f64,
    /// Ground-truth tokens most under-represented in the extraction.
    pub top_missing_tokens: Vec<String>,
}

/// A document with every framework's scores plus computed deficits.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentRow {
    pub document: String,
    pub file_size: u64,
    pub scores: Vec<FrameworkScore>,
    /// Combined quality of the Xberg heuristics (baseline) path.
    pub xberg_combined: Option<f64>,
    /// Best competitor by combined quality on this document.
    pub best_competitor: Option<String>,
    pub best_competitor_combined: Option<f64>,
    /// `best_competitor_combined - xberg_combined`; positive means we lose.
    pub combined_deficit: Option<f64>,
    /// TF1 / SF1 deficits against the best competitor (positive means we lose).
    pub tf1_deficit: Option<f64>,
    pub sf1_deficit: Option<f64>,
}

/// Configuration for which frameworks play which role in the comparison.
#[derive(Debug, Clone)]
pub struct GapConfig {
    /// Our model-free heuristics path (compared against competitors).
    pub baseline: String,
    /// Our routed ML-layout path (compared against Docling).
    pub layout: String,
    /// Competitor frameworks to rank against.
    pub competitors: Vec<String>,
}

impl Default for GapConfig {
    fn default() -> Self {
        Self {
            baseline: "xberg-markdown-baseline".to_string(),
            layout: "xberg-markdown-layout".to_string(),
            competitors: vec!["liteparse".to_string(), "docling".to_string()],
        }
    }
}

/// Strip the `-batch`/`-sync`/`-async` mode suffix so a framework matches
/// regardless of the mode a given run used.
fn base_framework(name: &str) -> &str {
    name.trim_end_matches("-batch")
        .trim_end_matches("-sync")
        .trim_end_matches("-async")
}

fn score_from_result(result: &BenchmarkResult) -> FrameworkScore {
    let quality = result.quality.as_ref();
    FrameworkScore {
        framework: base_framework(&result.framework).to_string(),
        success: result.success,
        tf1: quality.map(|q| q.f1_score_text),
        numeric_f1: quality.map(|q| q.f1_score_numeric),
        sf1: quality.and_then(|q| q.f1_score_layout),
        combined: quality.map(|q| q.quality_score),
        duration_ms: result.duration.as_secs_f64() * 1000.0,
        top_missing_tokens: quality
            .map(|q| q.missing_tokens.iter().take(5).map(|(t, _)| t.clone()).collect())
            .unwrap_or_default(),
    }
}

/// Build the per-document pivot with computed deficits.
pub fn build_document_rows(results: &[BenchmarkResult], config: &GapConfig) -> Vec<DocumentRow> {
    // document -> framework(base name) -> score. ~keep
    let mut by_doc: BTreeMap<String, BTreeMap<String, (FrameworkScore, u64)>> = BTreeMap::new();

    for result in results {
        let doc = result
            .file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| result.file_path.to_string_lossy().into_owned());
        let score = score_from_result(result);
        by_doc
            .entry(doc)
            .or_default()
            .insert(score.framework.clone(), (score, result.file_size));
    }

    let mut rows: Vec<DocumentRow> = Vec::with_capacity(by_doc.len());
    for (doc, frameworks) in by_doc {
        let file_size = frameworks.values().map(|(_, size)| *size).max().unwrap_or(0);

        let xberg = frameworks.get(&config.baseline).map(|(s, _)| s);
        let xberg_combined = xberg.and_then(|s| s.combined.filter(|_| s.success));

        // Best competitor by combined quality (successful only).
        let best = config
            .competitors
            .iter()
            .filter_map(|name| frameworks.get(name).map(|(s, _)| s))
            .filter(|s| s.success)
            .filter_map(|s| s.combined.map(|c| (s, c)))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let (best_competitor, best_competitor_combined) = match best {
            Some((s, c)) => (Some(s.framework.clone()), Some(c)),
            None => (None, None),
        };

        let combined_deficit = match (best_competitor_combined, xberg_combined) {
            (Some(comp), Some(us)) => Some(comp - us),
            _ => None,
        };
        let best_score = best.map(|(s, _)| s);
        let tf1_deficit = match (best_score.and_then(|s| s.tf1), xberg.and_then(|s| s.tf1)) {
            (Some(comp), Some(us)) => Some(comp - us),
            _ => None,
        };
        let sf1_deficit = match (best_score.and_then(|s| s.sf1), xberg.and_then(|s| s.sf1)) {
            (Some(comp), Some(us)) => Some(comp - us),
            _ => None,
        };

        let mut scores: Vec<FrameworkScore> = frameworks.into_values().map(|(s, _)| s).collect();
        scores.sort_by(|a, b| a.framework.cmp(&b.framework));

        rows.push(DocumentRow {
            document: doc,
            file_size,
            scores,
            xberg_combined,
            best_competitor,
            best_competitor_combined,
            combined_deficit,
            tf1_deficit,
            sf1_deficit,
        });
    }

    rows
}

fn fmt_opt(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.3}")).unwrap_or_else(|| "—".to_string())
}

fn fmt_delta(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v:+.3}"),
        None => "—".to_string(),
    }
}

/// Render the gap report markdown from the per-document rows.
pub fn render_markdown(rows: &[DocumentRow], config: &GapConfig) -> String {
    let mut out = String::new();
    out.push_str("# PDF quality gap report\n\n");
    out.push_str(&format!(
        "Heuristics path: `{}` — Layout path: `{}` — Competitors: {}\n\n",
        config.baseline,
        config.layout,
        config
            .competitors
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", "),
    ));

    // Documents where a competitor beats our heuristics path, worst first.
    let mut losses: Vec<&DocumentRow> = rows
        .iter()
        .filter(|r| r.combined_deficit.map(|d| d > 0.0).unwrap_or(false))
        .collect();
    losses.sort_by(|a, b| {
        b.combined_deficit
            .partial_cmp(&a.combined_deficit)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let comparable = rows.iter().filter(|r| r.combined_deficit.is_some()).count();
    let wins = comparable - losses.len();
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- Documents comparable (heuristics + a competitor both scored): **{comparable}**\n"
    ));
    out.push_str(&format!(
        "- Documents where a competitor beats us: **{}**\n",
        losses.len()
    ));
    out.push_str(&format!("- Documents where we beat every competitor: **{wins}**\n"));
    if comparable > 0 {
        let mean_deficit = losses.iter().filter_map(|r| r.combined_deficit).sum::<f64>() / losses.len().max(1) as f64;
        out.push_str(&format!(
            "- Mean combined deficit on lost docs: **{mean_deficit:+.3}**\n"
        ));
    }
    out.push('\n');

    out.push_str("## Where competitors beat our heuristics path (worst first)\n\n");
    out.push_str(
        "| Document | Best competitor | Δcombined | Δtext (TF1) | Δstruct (SF1) | us combined | comp combined | likely losing |\n",
    );
    out.push_str("|---|---|--:|--:|--:|--:|--:|---|\n");
    for row in &losses {
        let losing = match (row.tf1_deficit, row.sf1_deficit) {
            (Some(t), Some(s)) if t > s + 0.02 => "text",
            (Some(t), Some(s)) if s > t + 0.02 => "structure",
            (Some(_), Some(_)) => "both",
            _ => "—",
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            row.document,
            row.best_competitor.as_deref().unwrap_or("—"),
            fmt_delta(row.combined_deficit),
            fmt_delta(row.tf1_deficit),
            fmt_delta(row.sf1_deficit),
            fmt_opt(row.xberg_combined),
            fmt_opt(row.best_competitor_combined),
            losing,
        ));
    }
    out.push('\n');

    // Diagnostic: top missing tokens for the worst 10 losses.
    out.push_str("## Token-loss diagnostics (worst 10 lost docs)\n\n");
    for row in losses.iter().take(10) {
        if let Some(us) = row.scores.iter().find(|s| s.framework == config.baseline)
            && !us.top_missing_tokens.is_empty()
        {
            out.push_str(&format!(
                "- **{}**: missing `{}`\n",
                row.document,
                us.top_missing_tokens.join("`, `"),
            ));
        }
    }
    out.push('\n');

    // Layout path vs Docling.
    out.push_str("## Where `xberg-layout` trails Docling\n\n");
    out.push_str("| Document | xberg-layout combined | docling combined | Δcombined |\n");
    out.push_str("|---|--:|--:|--:|\n");
    let mut layout_rows: Vec<(&DocumentRow, f64, f64, f64)> = Vec::new();
    for row in rows {
        let layout = row.scores.iter().find(|s| s.framework == config.layout && s.success);
        let docling = row.scores.iter().find(|s| s.framework == "docling" && s.success);
        if let (Some(l), Some(d)) = (layout, docling)
            && let (Some(lc), Some(dc)) = (l.combined, d.combined)
            && dc > lc
        {
            layout_rows.push((row, lc, dc, dc - lc));
        }
    }
    layout_rows.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    for (row, lc, dc, delta) in &layout_rows {
        out.push_str(&format!(
            "| {} | {:.3} | {:.3} | {:+.3} |\n",
            row.document, lc, dc, delta
        ));
    }
    if layout_rows.is_empty() {
        out.push_str("| _(none — layout matches or beats Docling everywhere it ran)_ |  |  |  |\n");
    }
    out.push('\n');

    out
}

/// Load-per-core mean spread above which two frameworks are considered to have
/// run under materially different load — timing across them is then not
/// apples-to-apples for that run.
const LOAD_SPREAD_WARN: f64 = 0.15;

/// One framework's system-load profile across a run.
#[derive(Debug, Clone, Serialize)]
pub struct FrameworkLoad {
    pub framework: String,
    /// Results that carried a load sample.
    pub measured: usize,
    pub mean_load_per_core: f64,
    pub max_load_per_core: f64,
    /// Results measured while the machine was contended (`SystemLoad::is_contended`).
    pub contended: usize,
    pub logical_cores: usize,
}

/// Group results by framework and summarize the load each was measured under.
pub fn build_load_summary(results: &[BenchmarkResult]) -> Vec<FrameworkLoad> {
    let mut by_fw: BTreeMap<String, Vec<crate::system_load::SystemLoad>> = BTreeMap::new();
    for result in results {
        if let Some(load) = result.system_load {
            by_fw
                .entry(base_framework(&result.framework).to_string())
                .or_default()
                .push(load);
        }
    }
    by_fw
        .into_iter()
        .map(|(framework, loads)| {
            let measured = loads.len();
            let sum: f64 = loads.iter().map(|l| l.load_per_core()).sum();
            let max = loads.iter().map(|l| l.load_per_core()).fold(0.0_f64, f64::max);
            let contended = loads.iter().filter(|l| l.is_contended()).count();
            let logical_cores = loads.first().map(|l| l.logical_cores).unwrap_or(0);
            FrameworkLoad {
                framework,
                measured,
                mean_load_per_core: if measured > 0 { sum / measured as f64 } else { 0.0 },
                max_load_per_core: max,
                contended,
                logical_cores,
            }
        })
        .collect()
}

/// Render the per-framework system-load section for the gap report.
pub fn render_load_summary(loads: &[FrameworkLoad]) -> String {
    let mut out = String::new();
    out.push_str("## System load during measurement (apples-to-apples check)\n\n");
    if loads.iter().all(|l| l.measured == 0) {
        out.push_str("_No load samples recorded — this run predates system-load capture. Re-run to populate._\n\n");
        return out;
    }
    out.push_str(
        "Load-per-core = 1-minute load average ÷ logical cores, captured per result. It \
         includes the workload itself, so the useful reads are (a) whether frameworks ran under a \
         *similar* load and (b) how many results were measured while the machine was contended \
         (load/core > 0.7).\n\n",
    );
    out.push_str("| Framework | results | mean load/core | max load/core | contended |\n");
    out.push_str("|---|--:|--:|--:|--:|\n");
    for l in loads {
        out.push_str(&format!(
            "| {} | {} | {:.2} | {:.2} | {}/{} |\n",
            l.framework, l.measured, l.mean_load_per_core, l.max_load_per_core, l.contended, l.measured,
        ));
    }
    out.push('\n');

    let means: Vec<f64> = loads
        .iter()
        .filter(|l| l.measured > 0)
        .map(|l| l.mean_load_per_core)
        .collect();
    let min = means.iter().copied().fold(f64::INFINITY, f64::min);
    let max = means.iter().copied().fold(0.0_f64, f64::max);
    if means.len() > 1 {
        let spread = max - min;
        if spread > LOAD_SPREAD_WARN {
            out.push_str(&format!(
                "> ⚠️ Frameworks were measured under materially different load (mean load/core \
                 spread {spread:.2}). Cross-framework timing is **not** apples-to-apples for this \
                 run — re-run on a quiet machine before trusting speed deltas.\n\n",
            ));
        } else {
            out.push_str(&format!(
                "> Frameworks ran under comparable load (mean load/core spread {spread:.2}); \
                 timing is broadly comparable.\n\n",
            ));
        }
    }
    out
}

/// Load a run's `results.json` from `results_dir`, build the pivot, and write
/// `per_document.json` + `gaps.md` into `output_dir`.
pub fn generate(results_dir: &Path, output_dir: &Path, config: &GapConfig) -> Result<()> {
    let results = crate::load_run_results(results_dir)?;
    let rows = build_document_rows(&results, config);

    std::fs::create_dir_all(output_dir).map_err(crate::Error::Io)?;

    let per_doc_path = output_dir.join("per_document.json");
    let json = serde_json::to_string_pretty(&rows)
        .map_err(|e| crate::Error::Benchmark(format!("JSON serialization failed: {e}")))?;
    std::fs::write(&per_doc_path, json).map_err(crate::Error::Io)?;

    let mut markdown = render_markdown(&rows, config);
    markdown.push_str(&render_load_summary(&build_load_summary(&results)));

    let gaps_path = output_dir.join("gaps.md");
    std::fs::write(&gaps_path, markdown).map_err(crate::Error::Io)?;

    println!("Per-document pivot written to: {}", per_doc_path.display());
    println!("Gap report written to: {}", gaps_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameworkCapabilities, OutputFormat, PerformanceMetrics, QualityMetrics};
    use std::path::PathBuf;
    use std::time::Duration;

    fn result(framework: &str, doc: &str, tf1: f64, sf1: f64, combined: f64, success: bool) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            output_format: OutputFormat::Markdown,
            file_path: PathBuf::from(format!("/fixtures/{doc}")),
            file_size: 1000,
            success,
            error_message: None,
            error_kind: Default::default(),
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 0,
                peak_memory_delta_bytes: 0,
                avg_cpu_percent: 0.0,
                throughput_bytes_per_sec: 0.0,
                p50_memory_bytes: 0,
                p95_memory_bytes: 0,
                p99_memory_bytes: 0,
            },
            quality: success.then(|| QualityMetrics {
                f1_score_text: tf1,
                f1_score_numeric: 0.0,
                f1_score_layout: Some(sf1),
                quality_score: combined,
                missing_tokens: vec![("widget".to_string(), 3)],
                extra_tokens: vec![],
                correct: combined >= 0.95,
            }),
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: Default::default(),
            extracted_text: None,
            system_load: None,
        }
    }

    #[test]
    fn should_flag_document_where_competitor_beats_heuristics() {
        let results = vec![
            result("xberg-markdown-baseline", "a.pdf", 0.70, 0.60, 0.66, true),
            result("liteparse", "a.pdf", 0.85, 0.80, 0.83, true),
        ];
        let rows = build_document_rows(&results, &GapConfig::default());
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.best_competitor.as_deref(), Some("liteparse"));
        assert!((row.combined_deficit.unwrap() - (0.83 - 0.66)).abs() < 1e-9);
        assert!((row.tf1_deficit.unwrap() - 0.15).abs() < 1e-9);
        assert!((row.sf1_deficit.unwrap() - 0.20).abs() < 1e-9);
    }

    #[test]
    fn should_pick_best_competitor_across_multiple() {
        let results = vec![
            result("xberg-markdown-baseline", "a.pdf", 0.70, 0.60, 0.66, true),
            result("liteparse", "a.pdf", 0.85, 0.80, 0.83, true),
            result("docling", "a.pdf", 0.95, 0.99, 0.97, true),
        ];
        let rows = build_document_rows(&results, &GapConfig::default());
        assert_eq!(rows[0].best_competitor.as_deref(), Some("docling"));
        assert!((rows[0].best_competitor_combined.unwrap() - 0.97).abs() < 1e-9);
    }

    #[test]
    fn should_ignore_failed_competitor_runs() {
        let results = vec![
            result("xberg-markdown-baseline", "a.pdf", 0.70, 0.60, 0.66, true),
            result("docling", "a.pdf", 0.0, 0.0, 0.0, false),
        ];
        let rows = build_document_rows(&results, &GapConfig::default());
        assert!(rows[0].best_competitor.is_none());
        assert!(rows[0].combined_deficit.is_none());
    }

    #[test]
    fn should_strip_batch_suffix_when_grouping() {
        let results = vec![
            result("xberg-markdown-baseline-batch", "a.pdf", 0.70, 0.60, 0.66, true),
            result("liteparse-batch", "a.pdf", 0.85, 0.80, 0.83, true),
        ];
        let rows = build_document_rows(&results, &GapConfig::default());
        assert_eq!(rows[0].best_competitor.as_deref(), Some("liteparse"));
        assert!(rows[0].combined_deficit.unwrap() > 0.0);
    }

    fn with_load(mut r: BenchmarkResult, one_min: f64, cores: usize) -> BenchmarkResult {
        r.system_load = Some(crate::system_load::SystemLoad {
            load_avg_1m: one_min,
            load_avg_5m: one_min,
            load_avg_15m: one_min,
            logical_cores: cores,
            physical_cores: cores,
        });
        r
    }

    #[test]
    fn should_summarize_per_framework_load() {
        let results = vec![
            with_load(result("xberg-markdown-layout", "a.pdf", 0.7, 0.6, 0.66, true), 14.0, 14),
            with_load(result("xberg-markdown-layout", "b.pdf", 0.7, 0.6, 0.66, true), 7.0, 14),
            with_load(result("liteparse", "a.pdf", 0.8, 0.8, 0.8, true), 7.0, 14),
        ];
        let summary = build_load_summary(&results);
        let layout = summary.iter().find(|l| l.framework == "xberg-markdown-layout").unwrap();
        assert_eq!(layout.measured, 2);
        assert!((layout.mean_load_per_core - 0.75).abs() < 1e-9); // (1.0 + 0.5) / 2 ~keep
        assert!((layout.max_load_per_core - 1.0).abs() < 1e-9);
        assert_eq!(layout.contended, 1); // only the load=14 result is contended ~keep
    }

    #[test]
    fn should_warn_when_frameworks_ran_under_different_load() {
        let results = vec![
            with_load(result("xberg-markdown-layout", "a.pdf", 0.7, 0.6, 0.66, true), 14.0, 14),
            with_load(result("liteparse", "a.pdf", 0.8, 0.8, 0.8, true), 1.0, 14),
        ];
        let md = render_load_summary(&build_load_summary(&results));
        assert!(md.contains("not** apples-to-apples") || md.contains("materially different load"));
    }

    #[test]
    fn should_note_missing_load_samples() {
        let results = vec![result("liteparse", "a.pdf", 0.8, 0.8, 0.8, true)];
        let md = render_load_summary(&build_load_summary(&results));
        assert!(md.contains("predates system-load capture"));
    }
}
