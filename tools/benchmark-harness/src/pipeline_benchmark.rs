//! 6-path pipeline benchmark: exhaustive quality + timing comparison across
//! all extraction configurations on the full PDF corpus.
//!
//! | ID | Name              | Config                                           |
//! |----|-------------------|--------------------------------------------------|
//! | P1 | native            | output_format: Markdown                          |
//! | P2 | native+layout     | output_format: Markdown, layout: fast             |
//! | P3 | tesseract         | output_format: Markdown, ocr: tesseract, force    |
//! | P4 | tesseract+layout  | P3 + layout: fast                                |
//! | P5 | paddleocr         | output_format: Markdown, ocr: paddleocr, force    |
//! | P6 | paddleocr+layout  | P5 + layout: fast                                |

use crate::Result;
use crate::comparison::{Pipeline, PipelineResult};
use crate::corpus::{self, CorpusDocument, CorpusFilter};
use crate::markdown_quality::{MdBlockType, parse_markdown_blocks, score_structural_quality_normalized};
use crate::quality::{compute_f1, tokenize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Which pipeline paths to include.
pub struct PipelineBenchmarkConfig {
    pub fixtures_dir: PathBuf,
    pub paths: Vec<Pipeline>,
    pub doc_filter: Vec<String>,
    pub dump_outputs: bool,
    pub json_output: Option<PathBuf>,
    pub sort_by: SortMetric,
    pub bottom_n: Option<usize>,
    pub triage_blocks: bool,
}

/// Metric to sort by in triage view.
#[derive(Debug, Clone, Copy, Default)]
pub enum SortMetric {
    #[default]
    Sf1,
    Tf1,
    Time,
}

impl SortMetric {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sf1" => Some(SortMetric::Sf1),
            "tf1" => Some(SortMetric::Tf1),
            "time" => Some(SortMetric::Time),
            _ => None,
        }
    }

    fn extract(&self, pr: &PipelineResult) -> f64 {
        match self {
            SortMetric::Sf1 => pr.sf1,
            SortMetric::Tf1 => pr.tf1,
            SortMetric::Time => {
                if pr.time_ms.is_nan() {
                    f64::NEG_INFINITY
                } else {
                    -pr.time_ms // negate so ascending sort = slowest first
                }
            }
        }
    }
}

/// Result for one document across all selected pipeline paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDocResult {
    pub name: String,
    pub file_size: u64,
    pub results: Vec<PipelineResult>,
}

/// Per-pipeline aggregate statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineAggregate {
    pub pipeline: String,
    pub mean_sf1: f64,
    pub mean_tf1: f64,
    pub mean_time_ms: f64,
    pub p50_sf1: f64,
    pub p50_tf1: f64,
    pub p50_time_ms: f64,
    pub p90_time_ms: f64,
}

/// Full benchmark run summary for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunSummary {
    pub timestamp: String,
    pub git_sha: String,
    pub doc_count: usize,
    pub pipeline_count: usize,
    pub aggregates: Vec<PipelineAggregate>,
    pub docs: Vec<PipelineDocResult>,
}

/// Default 6-path set.
pub fn default_paths() -> Vec<Pipeline> {
    vec![
        Pipeline::Baseline,
        Pipeline::Layout,
        Pipeline::Tesseract,
        Pipeline::TesseractLayout,
        Pipeline::Paddle,
        Pipeline::PaddleLayout,
    ]
}

/// Build extraction config for a pipeline.
fn build_config(pipeline: Pipeline) -> kreuzberg::ExtractionConfig {
    use kreuzberg::core::config::{OcrConfig, OutputFormat, layout::LayoutDetectionConfig};

    let base = kreuzberg::ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    match pipeline {
        Pipeline::Baseline => base,
        Pipeline::Layout => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                preset: "fast".to_string(),
                ..Default::default()
            }),
            // Include OCR config for automatic fallback on image-only/scanned PDFs.
            // Not force_ocr — native text is preferred when quality is sufficient.
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Tesseract => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::TesseractLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig {
                preset: "fast".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Paddle => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PaddleLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig {
                preset: "fast".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Docling => base, // Docling reads from file, not used here
    }
}

/// Per-doc extraction timeout (seconds).
const DOC_TIMEOUT_SECS: u64 = 60;

async fn extract_and_score(
    pipeline: Pipeline,
    doc: &CorpusDocument,
    gt_text: &str,
    gt_markdown: Option<&str>,
    fixtures_dir: &Path,
) -> PipelineResult {
    let (content, time_ms) = if pipeline == Pipeline::Docling {
        // Docling: read vendored (pre-computed) output — no extraction timing
        let vendored_path = fixtures_dir
            .join("vendored/docling/md")
            .join(format!("{}.md", doc.name));
        let md = match std::fs::read_to_string(&vendored_path) {
            Ok(md) => md,
            Err(_) => {
                eprintln!(
                    "  SKIP {}/docling: no vendored output at {}",
                    doc.name,
                    vendored_path.display()
                );
                String::new()
            }
        };
        // Report NaN for vendored output — no meaningful extraction time
        (md, f64::NAN)
    } else {
        let t = Instant::now();
        let config = build_config(pipeline);
        let doc_path = doc.document_path.clone();
        // Use extract_file_sync on a blocking thread so the timeout can work.
        let handle = tokio::task::spawn_blocking(move || kreuzberg::extract_file_sync(&doc_path, None, &config));
        let result = match tokio::time::timeout(std::time::Duration::from_secs(DOC_TIMEOUT_SECS), handle).await {
            Ok(Ok(Ok(r))) => r.content,
            Ok(Ok(Err(e))) => {
                eprintln!("  ERROR {}/{}: {}", doc.name, pipeline.name(), e);
                String::new()
            }
            Ok(Err(e)) => {
                eprintln!("  TASK ERROR {}/{}: {}", doc.name, pipeline.name(), e);
                String::new()
            }
            Err(_) => {
                eprintln!(
                    "  TIMEOUT {}/{}: exceeded {}s",
                    doc.name,
                    pipeline.name(),
                    DOC_TIMEOUT_SECS
                );
                String::new()
            }
        };
        (result, t.elapsed().as_secs_f64() * 1000.0)
    };

    let tf1 = compute_f1(&tokenize(&content), &tokenize(gt_text));
    let (sf1, order_score, per_type_sf1) = match gt_markdown {
        Some(md) => {
            // Skip SF1 for documents without structural ground truth
            // (all-Paragraph docs produce meaningless 0% scores)
            let gt_blocks = parse_markdown_blocks(md);
            let has_structure = gt_blocks
                .iter()
                .any(|b| !matches!(b.block_type, MdBlockType::Paragraph));

            if !has_structure {
                (f64::NAN, f64::NAN, HashMap::new())
            } else {
                // Cap content to 50K chars to prevent scoring from taking too long
                let capped = if content.len() > 50_000 {
                    // Find a valid UTF-8 boundary near 50K
                    let mut end = 50_000;
                    while end > 0 && !content.is_char_boundary(end) {
                        end -= 1;
                    }
                    &content[..end]
                } else {
                    &content
                };
                // Use heading-level-normalized scoring (H1≡H2≡H3 etc.)
                let sq = score_structural_quality_normalized(capped, md);
                let per_type: HashMap<String, f64> = sq.per_type.iter().map(|(k, v)| (k.to_string(), v.f1)).collect();
                (sq.structural_f1, sq.order_score, per_type)
            }
        }
        None => (f64::NAN, f64::NAN, HashMap::new()),
    };

    PipelineResult {
        pipeline,
        sf1,
        tf1,
        order_score,
        per_type_sf1,
        time_ms,
        content,
    }
}

/// Run the pipeline benchmark.
pub async fn run_pipeline_benchmark(config: &PipelineBenchmarkConfig) -> Result<Vec<PipelineDocResult>> {
    let filter = CorpusFilter {
        file_types: Some(vec!["pdf".to_string()]),
        require_ground_truth: true,
        name_patterns: config.doc_filter.clone(),
        ..Default::default()
    };

    let docs = corpus::build_corpus(&config.fixtures_dir, &filter)?;
    eprintln!(
        "Pipeline benchmark: {} documents, {} paths",
        docs.len(),
        config.paths.len()
    );

    let dump_dir = if config.dump_outputs {
        let dir = PathBuf::from("/tmp/kreuzberg_pipeline");
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    } else {
        None
    };

    let mut results = Vec::new();
    let total = docs.len();

    for (idx, doc) in docs.iter().enumerate() {
        eprint!("\r[{}/{}] {} ...", idx + 1, total, doc.name);
        let gt_text = doc
            .ground_truth_text
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let gt_markdown = doc
            .ground_truth_markdown
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok());

        let mut pipeline_results = Vec::new();

        for &pipeline in &config.paths {
            let pr = extract_and_score(pipeline, doc, &gt_text, gt_markdown.as_deref(), &config.fixtures_dir).await;

            if let Some(ref dir) = dump_dir {
                let doc_dir = dir.join(&doc.name);
                let _ = std::fs::create_dir_all(&doc_dir);
                let _ = std::fs::write(doc_dir.join(format!("{}.md", pipeline.name())), &pr.content);
                // Also dump ground truth for comparison
                if let Some(ref gt_md) = gt_markdown {
                    let _ = std::fs::write(doc_dir.join("ground_truth.md"), gt_md);
                }
                let _ = std::fs::write(doc_dir.join("ground_truth_text.txt"), &gt_text);
            }

            pipeline_results.push(pr);
        }

        let best_sf1 = pipeline_results.iter().map(|r| r.sf1).fold(0.0_f64, f64::max);
        let best_time = pipeline_results
            .iter()
            .map(|r| r.time_ms)
            .filter(|t| !t.is_nan())
            .fold(f64::INFINITY, f64::min);
        if best_time.is_infinite() {
            eprint!(
                "\r[{}/{}] {:<30} SF1:{:.0}%\n",
                idx + 1,
                total,
                doc.name,
                best_sf1 * 100.0,
            );
        } else {
            eprint!(
                "\r[{}/{}] {:<30} SF1:{:.0}% {:.0}ms\n",
                idx + 1,
                total,
                doc.name,
                best_sf1 * 100.0,
                best_time
            );
        }

        results.push(PipelineDocResult {
            name: doc.name.clone(),
            file_size: doc.file_size,
            results: pipeline_results,
        });
    }

    Ok(results)
}

/// Print a per-document + aggregate matrix table.
pub fn print_pipeline_table(results: &[PipelineDocResult], sort_by: SortMetric, bottom_n: Option<usize>) {
    if results.is_empty() {
        eprintln!("No results.");
        return;
    }

    // Optionally sort and truncate for triage view
    let display_results: Vec<&PipelineDocResult> = if let Some(n) = bottom_n {
        let mut sorted: Vec<&PipelineDocResult> = results.iter().collect();
        // Sort by the worst (min) score across all pipelines for the chosen metric
        sorted.sort_by(|a, b| {
            let a_worst = a
                .results
                .iter()
                .map(|pr| sort_by.extract(pr))
                .fold(f64::INFINITY, f64::min);
            let b_worst = b
                .results
                .iter()
                .map(|pr| sort_by.extract(pr))
                .fold(f64::INFINITY, f64::min);
            a_worst.partial_cmp(&b_worst).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(n).collect()
    } else {
        results.iter().collect()
    };

    let pipelines: Vec<&str> = results[0].results.iter().map(|r| r.pipeline.name()).collect();

    // Header
    eprint!("{:<30}", "Document");
    for p in &pipelines {
        eprint!(" {:>8} {:>8} {:>7}", format!("{} SF1", p), "TF1", "ms");
    }
    eprintln!();
    eprintln!("{}", "-".repeat(30 + pipelines.len() * 26));

    for doc in &display_results {
        eprint!(
            "{:<30}",
            if doc.name.len() > 29 {
                &doc.name[..29]
            } else {
                &doc.name
            }
        );
        for pr in &doc.results {
            if pr.time_ms.is_nan() {
                eprint!(" {:>7.1}% {:>7.1}% {:>7}", pr.sf1 * 100.0, pr.tf1 * 100.0, "N/A");
            } else {
                eprint!(" {:>7.1}% {:>7.1}% {:>7.0}", pr.sf1 * 100.0, pr.tf1 * 100.0, pr.time_ms);
            }
        }
        eprintln!();
    }

    // Averages (always over all results, not just displayed)
    let n = results.len() as f64;
    let total_docs = results.len();
    eprintln!("{}", "-".repeat(30 + pipelines.len() * 26));
    eprint!("{:<30}", "AVERAGE");
    for (i, _) in pipelines.iter().enumerate() {
        let sf1_vals: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].sf1)
            .filter(|v| !v.is_nan())
            .collect();
        let sf1 = if !sf1_vals.is_empty() {
            sf1_vals.iter().sum::<f64>() / sf1_vals.len() as f64
        } else {
            0.0
        };
        let tf1: f64 = results.iter().map(|r| r.results[i].tf1).sum::<f64>() / n;
        let time_vals: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].time_ms)
            .filter(|v| !v.is_nan())
            .collect();
        if time_vals.is_empty() {
            eprint!(" {:>7.1}% {:>7.1}% {:>7}", sf1 * 100.0, tf1 * 100.0, "N/A");
        } else {
            let ms: f64 = time_vals.iter().sum::<f64>() / time_vals.len() as f64;
            eprint!(" {:>7.1}% {:>7.1}% {:>7.0}", sf1 * 100.0, tf1 * 100.0, ms);
        }
    }
    eprintln!();
    // Report how many docs were excluded from SF1 average
    let sf1_excluded: usize = results.iter().map(|r| r.results[0].sf1).filter(|v| v.is_nan()).count();
    if sf1_excluded > 0 {
        eprintln!(
            "  (SF1 averaged over {}/{} docs; {} paragraph-only docs excluded)",
            total_docs - sf1_excluded,
            total_docs,
            sf1_excluded
        );
    }
}

/// Print per-block-type F1 breakdown for triage.
pub fn print_triage_blocks(results: &[PipelineDocResult], sort_by: SortMetric, bottom_n: usize) {
    if results.is_empty() {
        return;
    }

    let block_types = ["H1", "H2", "H3", "Table", "Code", "ListItem", "Paragraph"];

    // Sort and take bottom N
    let mut sorted: Vec<&PipelineDocResult> = results.iter().collect();
    sorted.sort_by(|a, b| {
        let a_worst = a
            .results
            .iter()
            .map(|pr| sort_by.extract(pr))
            .fold(f64::INFINITY, f64::min);
        let b_worst = b
            .results
            .iter()
            .map(|pr| sort_by.extract(pr))
            .fold(f64::INFINITY, f64::min);
        a_worst.partial_cmp(&b_worst).unwrap_or(std::cmp::Ordering::Equal)
    });
    let display: Vec<&PipelineDocResult> = sorted.into_iter().take(bottom_n).collect();

    eprintln!("\nPer-block-type F1 breakdown (bottom {} documents):", bottom_n);

    for doc in &display {
        eprintln!("\n  {}", doc.name);
        for pr in &doc.results {
            let blocks_str: String = block_types
                .iter()
                .filter_map(|bt| pr.per_type_sf1.get(*bt).map(|v| format!("{}:{:.0}%", bt, v * 100.0)))
                .collect::<Vec<_>>()
                .join("  ");
            eprintln!(
                "    {:<18} SF1:{:.0}%  {}",
                pr.pipeline.name(),
                pr.sf1 * 100.0,
                blocks_str
            );
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Compute per-pipeline aggregate statistics.
pub fn compute_aggregates(results: &[PipelineDocResult]) -> Vec<PipelineAggregate> {
    if results.is_empty() {
        return Vec::new();
    }

    let n = results.len() as f64;
    let num_pipelines = results[0].results.len();
    let mut aggregates = Vec::new();

    for i in 0..num_pipelines {
        let pipeline_name = results[0].results[i].pipeline.name().to_string();

        // Filter NaN values from SF1 (docs without structural ground truth)
        let mut sf1s: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].sf1)
            .filter(|v| !v.is_nan())
            .collect();
        let mut tf1s: Vec<f64> = results.iter().map(|r| r.results[i].tf1).collect();
        let mut times: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].time_ms)
            .filter(|v| !v.is_nan())
            .collect();

        sf1s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        tf1s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let sf1_n = sf1s.len() as f64;

        aggregates.push(PipelineAggregate {
            pipeline: pipeline_name,
            mean_sf1: if sf1_n > 0.0 {
                sf1s.iter().sum::<f64>() / sf1_n
            } else {
                0.0
            },
            mean_tf1: tf1s.iter().sum::<f64>() / n,
            mean_time_ms: if times.is_empty() {
                f64::NAN
            } else {
                times.iter().sum::<f64>() / times.len() as f64
            },
            p50_sf1: percentile(&sf1s, 0.5),
            p50_tf1: percentile(&tf1s, 0.5),
            p50_time_ms: percentile(&times, 0.5),
            p90_time_ms: percentile(&times, 0.9),
        });
    }

    aggregates
}

/// Build a full run summary for JSON serialization.
pub fn build_summary(results: &[PipelineDocResult]) -> PipelineRunSummary {
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let timestamp = chrono::Utc::now().to_rfc3339();

    PipelineRunSummary {
        timestamp,
        git_sha,
        doc_count: results.len(),
        pipeline_count: results.first().map(|r| r.results.len()).unwrap_or(0),
        aggregates: compute_aggregates(results),
        docs: results.to_vec(),
    }
}

/// Write the run summary to a JSON file.
pub fn write_json_output(results: &[PipelineDocResult], path: &std::path::Path) -> Result<()> {
    let summary = build_summary(results);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::Error::Io)?;
    }
    let json = serde_json::to_string_pretty(&summary)
        .map_err(|e| crate::Error::Benchmark(format!("Failed to serialize: {}", e)))?;
    std::fs::write(path, json).map_err(crate::Error::Io)?;
    eprintln!("JSON output written to: {}", path.display());
    Ok(())
}
