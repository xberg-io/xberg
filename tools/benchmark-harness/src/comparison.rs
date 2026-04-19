//! Framework comparison: run multiple extraction pipelines on the corpus and
//! compare quality (SF1, TF1) against ground truth with optional guardrails.
//!
//! Replaces the logic previously in `crates/kreuzberg/tests/framework_comparison.rs`.
//! Uses canonical scoring from [`crate::markdown_quality`] and [`crate::quality`].
//!
//! # Pipeline semantics
//!
//! Each [`Pipeline`] variant represents a distinct extraction configuration:
//!
//! - **Baseline / Layout**: native pdfium text extraction; `Layout` adds layout
//!   detection for heading/table identification without OCR.
//! - **Tesseract / TesseractLayout**: Tesseract OCR with `force_ocr`; the
//!   `Layout` suffix adds layout detection on top.
//! - **Paddle / PaddleLayout / PaddleServer / PaddleServerLayout**: PaddleOCR
//!   at mobile or server model tier, with optional layout detection.
//! - **Docling / PaddleOcrPython / RapidOcr**: vendored pipelines whose
//!   outputs are pre-computed and read from disk (no live extraction).
//! - **LayoutSlanet***: layout detection with explicit SLANeXT table model
//!   variants (wired, wireless, plus, auto).
//!
//! # Guardrail thresholds
//!
//! When `guardrails` is enabled, the comparison enforces per-document minimum
//! SF1 and TF1 thresholds. Thresholds are set to approximately 90% of observed
//! scores (a floor) to catch regressions without false-positive failures from
//! run-to-run variance. The guardrail table is updated manually after
//! significant quality improvements.

use crate::Result;
use crate::corpus::{self, CorpusDocument, CorpusFilter};
use crate::markdown_quality::score_structural_quality;
use crate::quality::{compute_f1, compute_token_diff, tokenize};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Extraction pipeline identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pipeline {
    /// Native pdfium text extraction (no OCR, no layout)
    Baseline,
    /// Native pdfium + layout detection
    Layout,
    /// Tesseract OCR (force_ocr)
    Tesseract,
    /// Tesseract OCR + layout detection
    TesseractLayout,
    /// PaddleOCR mobile tier (force_ocr) — default model tier
    Paddle,
    /// PaddleOCR mobile tier + layout detection — default model tier
    PaddleLayout,
    /// PaddleOCR server tier (force_ocr, explicit server models)
    PaddleServer,
    /// PaddleOCR server tier + layout detection
    PaddleServerLayout,
    /// Tesseract OCR with auto_rotate enabled
    TesseractAutoRotate,
    /// PaddleOCR without auto_rotate (for comparison)
    PaddleNoRotate,
    /// Docling vendored extraction (read from file)
    Docling,
    /// PaddleOCR Python vendored extraction (read from file)
    PaddleOcrPython,
    /// RapidOCR vendored extraction (read from file)
    RapidOcr,
    /// Native pdfium + layout detection + SLANeXT wired table model (forced)
    LayoutSlanetWired,
    /// Native pdfium + layout detection + SLANeXT wireless table model (forced)
    LayoutSlanetWireless,
    /// Native pdfium + layout detection + SLANet_plus table model
    LayoutSlanetPlus,
    /// Native pdfium + layout detection + classifier-routed SLANeXT (wired/wireless auto)
    LayoutSlanetAuto,
    /// pdf_oxide backend text extraction (no OCR, no layout)
    PdfOxide,
    /// pdf_oxide backend + layout detection
    PdfOxideLayout,
}

impl Pipeline {
    pub fn name(&self) -> &'static str {
        match self {
            Pipeline::Baseline => "baseline",
            Pipeline::Layout => "layout",
            Pipeline::Tesseract => "tesseract",
            Pipeline::TesseractLayout => "tesseract+layout",
            Pipeline::Paddle => "paddle",
            Pipeline::PaddleLayout => "paddle+layout",
            Pipeline::PaddleServer => "paddle-server",
            Pipeline::PaddleServerLayout => "paddle-server+layout",
            Pipeline::TesseractAutoRotate => "tesseract-autorotate",
            Pipeline::PaddleNoRotate => "paddle-norotate",
            Pipeline::Docling => "docling",
            Pipeline::PaddleOcrPython => "paddleocr-python",
            Pipeline::RapidOcr => "rapidocr",
            Pipeline::LayoutSlanetWired => "layout+slanet-wired",
            Pipeline::LayoutSlanetWireless => "layout+slanet-wireless",
            Pipeline::LayoutSlanetPlus => "layout+slanet-plus",
            Pipeline::LayoutSlanetAuto => "layout+slanet-auto",
            Pipeline::PdfOxide => "pdf-oxide",
            Pipeline::PdfOxideLayout => "pdf-oxide+layout",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "baseline" => Some(Pipeline::Baseline),
            "layout" => Some(Pipeline::Layout),
            "tesseract" => Some(Pipeline::Tesseract),
            "tesseract+layout" | "tesseract-layout" => Some(Pipeline::TesseractLayout),
            "paddle" | "paddle-mobile" => Some(Pipeline::Paddle),
            "paddle+layout" | "paddle-layout" | "paddle-mobile+layout" | "paddle-mobile-layout" => {
                Some(Pipeline::PaddleLayout)
            }
            "paddle-server" => Some(Pipeline::PaddleServer),
            "paddle-server+layout" | "paddle-server-layout" => Some(Pipeline::PaddleServerLayout),
            "tesseract-autorotate" => Some(Pipeline::TesseractAutoRotate),
            "paddle-norotate" => Some(Pipeline::PaddleNoRotate),
            "docling" => Some(Pipeline::Docling),
            "paddleocr-python" => Some(Pipeline::PaddleOcrPython),
            "rapidocr" => Some(Pipeline::RapidOcr),
            "layout+slanet-wired" | "layout-slanet-wired" => Some(Pipeline::LayoutSlanetWired),
            "layout+slanet-wireless" | "layout-slanet-wireless" => Some(Pipeline::LayoutSlanetWireless),
            "layout+slanet-plus" | "layout-slanet-plus" => Some(Pipeline::LayoutSlanetPlus),
            "layout+slanet-auto" | "layout-slanet-auto" | "layout+slanet" | "layout-slanet" => {
                Some(Pipeline::LayoutSlanetAuto)
            }
            "pdf-oxide" | "pdf_oxide" | "oxide" => Some(Pipeline::PdfOxide),
            "pdf-oxide+layout" | "pdf-oxide-layout" | "oxide+layout" | "oxide-layout" => Some(Pipeline::PdfOxideLayout),
            _ => None,
        }
    }

    /// All pipelines that use kreuzberg in-process extraction.
    pub fn all_kreuzberg() -> Vec<Pipeline> {
        vec![
            Pipeline::Baseline,
            Pipeline::Layout,
            Pipeline::Tesseract,
            Pipeline::TesseractLayout,
            Pipeline::Paddle,
            Pipeline::PaddleLayout,
            Pipeline::PaddleServer,
            Pipeline::PaddleServerLayout,
            Pipeline::PdfOxide,
            Pipeline::PdfOxideLayout,
        ]
    }
}

/// Configuration for a comparison run.
pub struct ComparisonConfig {
    pub fixtures_dir: std::path::PathBuf,
    pub pipelines: Vec<Pipeline>,
    pub dump_outputs: bool,
    pub guardrails: bool,
    /// Path to a JSON guardrails file. When `guardrails` is true this file
    /// is loaded instead of using hardcoded thresholds.
    pub guardrails_file: Option<std::path::PathBuf>,
    /// Optional name filter (only run docs whose name contains this)
    pub name_filter: Option<String>,
    /// Optional path to write full comparison results as JSON.
    pub json_output: Option<std::path::PathBuf>,
    /// Run noise detection on extracted outputs.
    pub noise: bool,
    /// Enable diagnostic diff mode for poor-scoring documents.
    pub diagnose: bool,
    /// SF1 threshold below which to generate diagnostics.
    pub diagnose_threshold: f64,
}

/// Result of running one pipeline on one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub pipeline: Pipeline,
    pub sf1: f64,
    pub tf1: f64,
    /// Reading order score (LIS-based, 0.0-1.0).
    #[serde(default)]
    pub order_score: f64,
    /// Per-block-type structural F1 scores (e.g. "H1" -> 0.85).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_type_sf1: HashMap<String, f64>,
    pub time_ms: f64,
    /// Top tokens present in GT but missing/under-represented in extraction (recall misses).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_tokens: Vec<(String, usize)>,
    /// Top tokens present in extraction but absent/over-represented vs GT (precision misses).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_tokens: Vec<(String, usize)>,
    #[serde(skip)]
    pub content: String,
}

/// Result of running all pipelines on one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocResult {
    pub name: String,
    pub file_type: String,
    pub results: Vec<PipelineResult>,
}

/// Build a kreuzberg ExtractionConfig for the given pipeline.
pub fn build_extraction_config(pipeline: Pipeline) -> kreuzberg::ExtractionConfig {
    use kreuzberg::core::config::{OutputFormat, layout::LayoutDetectionConfig};

    let base = kreuzberg::ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    match pipeline {
        Pipeline::Baseline => base,
        Pipeline::Layout => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig::default()),
            // Enable OCR fallback for pages with no native text (image-only pages).
            // With force_ocr=false (default), kreuzberg auto-detects empty pages
            // and falls back to tesseract OCR only when needed.
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Tesseract => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::TesseractLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..base
        },
        Pipeline::Paddle => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                auto_rotate: true,
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PaddleLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                auto_rotate: true,
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..base
        },
        Pipeline::PaddleServer => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                auto_rotate: true,
                paddle_ocr_config: Some(serde_json::json!({"model_tier": "server"})),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PaddleServerLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                auto_rotate: true,
                paddle_ocr_config: Some(serde_json::json!({"model_tier": "server"})),
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..base
        },
        Pipeline::TesseractAutoRotate => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                auto_rotate: true,
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PaddleNoRotate => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                auto_rotate: false,
                ..Default::default()
            }),
            ..base
        },
        Pipeline::LayoutSlanetAuto => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                table_model: kreuzberg::core::config::layout::TableModel::SlanetAuto,
                ..Default::default()
            }),
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PdfOxide => kreuzberg::ExtractionConfig {
            pdf_options: Some(kreuzberg::PdfConfig {
                backend: kreuzberg::PdfBackend::PdfOxide,
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PdfOxideLayout => kreuzberg::ExtractionConfig {
            pdf_options: Some(kreuzberg::PdfConfig {
                backend: kreuzberg::PdfBackend::PdfOxide,
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Docling | Pipeline::PaddleOcrPython | Pipeline::RapidOcr => base, // Not used for extraction — read from file
        Pipeline::LayoutSlanetWired => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                table_model: kreuzberg::core::config::layout::TableModel::SlanetWired,
                ..Default::default()
            }),
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::LayoutSlanetWireless => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                table_model: kreuzberg::core::config::layout::TableModel::SlanetWireless,
                ..Default::default()
            }),
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::LayoutSlanetPlus => kreuzberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                table_model: kreuzberg::core::config::layout::TableModel::SlanetPlus,
                ..Default::default()
            }),
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
    }
}

/// Score extracted content against ground truth, returning (tf1, sf1, order_score, per_type_sf1).
pub fn score_document(
    content: &str,
    gt_text: &str,
    gt_markdown: Option<&str>,
) -> (f64, f64, f64, HashMap<String, f64>) {
    let tf1 = {
        let ext_tokens = tokenize(content);
        let gt_tokens = tokenize(gt_text);
        compute_f1(&ext_tokens, &gt_tokens)
    };
    let (sf1, order_score, per_type_sf1) = match gt_markdown {
        Some(md) => {
            let sq = score_structural_quality(content, md);
            let per_type: HashMap<String, f64> = sq.per_type.iter().map(|(k, v)| (k.to_string(), v.f1)).collect();
            (sq.structural_f1, sq.order_score, per_type)
        }
        None => (0.0, 0.0, HashMap::new()),
    };
    (tf1, sf1, order_score, per_type_sf1)
}

/// Read vendored markdown + cached timing for a single document.
/// Returns (content, time_ms) where time_ms is NaN if no cached timing exists.
pub fn read_vendored_cached(doc_name: &str, fixtures_dir: &std::path::Path, vendored_name: &str) -> (String, f64) {
    let vendored_dir = fixtures_dir
        .parent()
        .unwrap_or(fixtures_dir)
        .join("vendored")
        .join(vendored_name);
    let md_path = vendored_dir.join("md").join(format!("{}.md", doc_name));
    let timing_path = vendored_dir.join("timing").join(format!("{}.ms", doc_name));
    let md = std::fs::read_to_string(&md_path).unwrap_or_default();
    let cached_ms = std::fs::read_to_string(&timing_path)
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(f64::NAN);
    (md, cached_ms)
}

/// Extract content from a document using the given pipeline.
/// Returns (content, time_ms).
pub async fn extract_pipeline(
    pipeline: Pipeline,
    doc: &crate::corpus::CorpusDocument,
    fixtures_dir: &std::path::Path,
) -> (Option<String>, f64) {
    match pipeline {
        Pipeline::Docling | Pipeline::PaddleOcrPython | Pipeline::RapidOcr => {
            let vendored_name = match pipeline {
                Pipeline::PaddleOcrPython => "paddleocr-python",
                Pipeline::RapidOcr => "rapidocr",
                _ => "docling",
            };
            let (content, time_ms) = read_vendored_cached(&doc.name, fixtures_dir, vendored_name);
            (Some(content), time_ms)
        }
        _ => {
            let t = Instant::now();
            let config = build_extraction_config(pipeline);
            let doc_path = doc.document_path.clone();
            let doc_name = doc.name.clone();
            let pipeline_name = pipeline.name().to_string();

            // Use AssertUnwindSafe + catch_unwind to handle panics in comrak or
            // other extraction code without crashing the entire benchmark run.
            let extraction_future = async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(180),
                    kreuzberg::core::batch_mode::with_batch_mode(kreuzberg::extract_file(&doc_path, None, &config)),
                )
                .await
            };

            let result = match std::panic::AssertUnwindSafe(extraction_future).catch_unwind().await {
                Ok(Ok(Ok(result))) => Some(result.content),
                Ok(Ok(Err(e))) => {
                    eprintln!("  ERROR {}/{}: {}", doc_name, pipeline_name, e);
                    None
                }
                Ok(Err(_)) => {
                    eprintln!("  TIMEOUT {}/{}: exceeded 180s", doc_name, pipeline_name);
                    None
                }
                Err(panic_info) => {
                    let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else {
                        format!("{:?}", panic_info)
                    };
                    eprintln!(
                        "  PANIC {}/{}: {}\n    document: {}\n    pipeline: {}\n    file_type: {}",
                        doc_name,
                        pipeline_name,
                        panic_msg,
                        doc_path.display(),
                        pipeline_name,
                        doc.file_type,
                    );
                    None
                }
            };
            (result, t.elapsed().as_secs_f64() * 1000.0)
        }
    }
}

/// Run a single pipeline on a single document and score it.
///
/// Timeouts, errors, and panics produce NaN scores so they are tracked
/// in the results but excluded from aggregate averages.
async fn run_pipeline(
    pipeline: Pipeline,
    doc: &CorpusDocument,
    gt_text: &str,
    gt_markdown: Option<&str>,
    fixtures_dir: &std::path::Path,
) -> PipelineResult {
    let (content_opt, time_ms) = extract_pipeline(pipeline, doc, fixtures_dir).await;

    let content = content_opt.unwrap_or_default();
    let (tf1, sf1, order_score, per_type_sf1) = if content.is_empty() && time_ms > 170_000.0 {
        // Likely a timeout — mark as NaN to exclude from averages
        (f64::NAN, f64::NAN, f64::NAN, std::collections::HashMap::new())
    } else {
        score_document(&content, gt_text, gt_markdown)
    };

    let ext_tokens = tokenize(&content);
    let gt_tokens = tokenize(gt_text);
    let (mut missing_tokens, mut extra_tokens) = compute_token_diff(&ext_tokens, &gt_tokens);
    missing_tokens.truncate(50);
    extra_tokens.truncate(50);

    PipelineResult {
        pipeline,
        sf1,
        tf1,
        order_score,
        per_type_sf1,
        time_ms,
        missing_tokens,
        extra_tokens,
        content,
    }
}

/// Run the full comparison across all documents and pipelines.
pub async fn run_comparison(config: &ComparisonConfig) -> Result<Vec<DocResult>> {
    let filter = CorpusFilter {
        file_types: None, // All formats with ground truth
        require_ground_truth: true,
        require_markdown_ground_truth: true,
        name_patterns: config.name_filter.clone().into_iter().collect(),
        ..Default::default()
    };

    let docs = corpus::build_corpus(&config.fixtures_dir, &filter)?;
    eprintln!(
        "Comparing {} documents across {} pipelines",
        docs.len(),
        config.pipelines.len()
    );

    let mut results = Vec::new();

    for doc in &docs {
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

        for &pipeline in &config.pipelines {
            let result = run_pipeline(pipeline, doc, &gt_text, gt_markdown.as_deref(), &config.fixtures_dir).await;

            if config.dump_outputs {
                let dump_dir = std::path::PathBuf::from("/tmp/kreuzberg_compare");
                let _ = std::fs::create_dir_all(&dump_dir);
                let _ = std::fs::write(
                    dump_dir.join(format!("{}_{}.md", doc.name, pipeline.name())),
                    &result.content,
                );
            }

            pipeline_results.push(result);
        }

        results.push(DocResult {
            name: doc.name.clone(),
            file_type: doc.file_type.clone(),
            results: pipeline_results,
        });
    }

    Ok(results)
}

/// Print a formatted comparison table to stderr.
pub fn print_comparison_table(results: &[DocResult]) {
    if results.is_empty() {
        eprintln!("No results to display.");
        return;
    }

    // Collect all pipeline names from results
    let pipeline_names: Vec<&str> = results
        .first()
        .map(|r| r.results.iter().map(|pr| pr.pipeline.name()).collect())
        .unwrap_or_default();

    // Header
    eprint!("{:<25}", "Document");
    for name in &pipeline_names {
        eprint!(
            " {:>10} {:>10} {:>8}",
            format!("{} SF1", name),
            format!("{} TF1", name),
            format!("{} ms", name),
        );
    }
    eprintln!();
    eprintln!("{}", "-".repeat(25 + pipeline_names.len() * 30));

    // Rows
    for doc in results {
        eprint!("{:<25}", doc.name);
        for pr in &doc.results {
            let time_str = if pr.time_ms.is_nan() || pr.time_ms <= 0.0 {
                "---".to_string()
            } else {
                format!("{:.0}", pr.time_ms)
            };
            eprint!(" {:>9.1}% {:>9.1}% {:>8}", pr.sf1 * 100.0, pr.tf1 * 100.0, time_str);
        }
        eprintln!();
    }

    // Averages
    eprintln!("{}", "-".repeat(25 + pipeline_names.len() * 30));
    eprint!("{:<25}", "AVERAGE");
    for (i, _) in pipeline_names.iter().enumerate() {
        let sf1_vals: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].sf1)
            .filter(|v| !v.is_nan())
            .collect();
        let tf1_vals: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].tf1)
            .filter(|v| !v.is_nan())
            .collect();
        let avg_sf1 = if sf1_vals.is_empty() {
            f64::NAN
        } else {
            sf1_vals.iter().sum::<f64>() / sf1_vals.len() as f64
        };
        let avg_tf1 = if tf1_vals.is_empty() {
            f64::NAN
        } else {
            tf1_vals.iter().sum::<f64>() / tf1_vals.len() as f64
        };
        let times: Vec<f64> = results
            .iter()
            .map(|r| r.results[i].time_ms)
            .filter(|t| !t.is_nan() && *t > 0.0)
            .collect();
        let avg_time = if times.is_empty() {
            f64::NAN
        } else {
            times.iter().sum::<f64>() / times.len() as f64
        };
        let time_str = if avg_time.is_nan() {
            "---".to_string()
        } else {
            format!("{:.0}", avg_time)
        };
        eprint!(" {:>9.1}% {:>9.1}% {:>8}", avg_sf1 * 100.0, avg_tf1 * 100.0, time_str);
    }
    eprintln!();

    // Report timeouts/errors per pipeline
    for (i, name) in pipeline_names.iter().enumerate() {
        let failed = results.iter().filter(|r| r.results[i].sf1.is_nan()).count();
        if failed > 0 {
            eprintln!("  {}: {} timeouts/errors (excluded from averages)", name, failed);
        }
    }
}

/// Print a per-format summary table to stderr, grouping documents by file_type.
pub fn print_per_format_summary(results: &[DocResult]) {
    if results.is_empty() {
        return;
    }

    // Collect pipeline names from the first result
    let pipeline_names: Vec<&str> = results
        .first()
        .map(|r| r.results.iter().map(|pr| pr.pipeline.name()).collect())
        .unwrap_or_default();

    // Group by file_type
    let mut by_format: std::collections::BTreeMap<&str, Vec<&DocResult>> = std::collections::BTreeMap::new();
    for doc in results {
        by_format.entry(&doc.file_type).or_default().push(doc);
    }

    eprintln!("\nPer-Format Summary:");

    // Header
    eprint!("{:<12} {:>5}", "Format", "Count");
    for name in &pipeline_names {
        eprint!("  {:>10} {:>10}", format!("{} SF1", name), format!("{} TF1", name));
    }
    eprintln!();
    let line_width = 12 + 5 + pipeline_names.len() * 22;
    eprintln!("{}", "\u{2500}".repeat(line_width));

    // Rows
    for (format, docs) in &by_format {
        eprint!("{:<12} {:>5}", format, docs.len());
        for (i, _) in pipeline_names.iter().enumerate() {
            let sf1_vals: Vec<f64> = docs.iter().map(|d| d.results[i].sf1).filter(|v| !v.is_nan()).collect();
            let tf1_vals: Vec<f64> = docs.iter().map(|d| d.results[i].tf1).filter(|v| !v.is_nan()).collect();
            let avg_sf1 = if sf1_vals.is_empty() {
                f64::NAN
            } else {
                sf1_vals.iter().sum::<f64>() / sf1_vals.len() as f64
            };
            let avg_tf1 = if tf1_vals.is_empty() {
                f64::NAN
            } else {
                tf1_vals.iter().sum::<f64>() / tf1_vals.len() as f64
            };
            eprint!("  {:>9.1}% {:>9.1}%", avg_sf1 * 100.0, avg_tf1 * 100.0);
        }
        eprintln!();
    }
}

/// Per-format aggregation entry for JSON output.
#[derive(Debug, Clone, Serialize)]
struct FormatSummary {
    count: usize,
    pipelines: Vec<FormatPipelineSummary>,
}

/// Per-pipeline averages within a format group.
#[derive(Debug, Clone, Serialize)]
struct FormatPipelineSummary {
    pipeline: String,
    avg_sf1: f64,
    avg_tf1: f64,
}

/// Overall summary for JSON output.
#[derive(Debug, Clone, Serialize)]
struct OverallSummary {
    total_documents: usize,
    pipelines: Vec<FormatPipelineSummary>,
}

/// Top-level JSON output structure.
#[derive(Debug, Clone, Serialize)]
struct ComparisonJsonOutput {
    documents: Vec<DocResult>,
    by_format: std::collections::BTreeMap<String, FormatSummary>,
    overall: OverallSummary,
}

/// Write full comparison results (documents + per-format + overall) to a JSON file.
pub fn write_comparison_json(results: &[DocResult], path: &std::path::Path) -> Result<()> {
    let pipeline_names: Vec<String> = results
        .first()
        .map(|r| r.results.iter().map(|pr| pr.pipeline.name().to_string()).collect())
        .unwrap_or_default();

    // Per-format aggregation
    let mut by_format_map: std::collections::BTreeMap<String, Vec<&DocResult>> = std::collections::BTreeMap::new();
    for doc in results {
        by_format_map.entry(doc.file_type.clone()).or_default().push(doc);
    }

    let by_format: std::collections::BTreeMap<String, FormatSummary> = by_format_map
        .iter()
        .map(|(format, docs)| {
            let _n = docs.len() as f64;
            let pipelines = pipeline_names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let sf1_vals: Vec<f64> = docs.iter().map(|d| d.results[i].sf1).filter(|v| !v.is_nan()).collect();
                    let tf1_vals: Vec<f64> = docs.iter().map(|d| d.results[i].tf1).filter(|v| !v.is_nan()).collect();
                    let avg_sf1 = if sf1_vals.is_empty() {
                        0.0
                    } else {
                        sf1_vals.iter().sum::<f64>() / sf1_vals.len() as f64
                    };
                    let avg_tf1 = if tf1_vals.is_empty() {
                        0.0
                    } else {
                        tf1_vals.iter().sum::<f64>() / tf1_vals.len() as f64
                    };
                    FormatPipelineSummary {
                        pipeline: name.clone(),
                        avg_sf1,
                        avg_tf1,
                    }
                })
                .collect();
            (
                format.clone(),
                FormatSummary {
                    count: docs.len(),
                    pipelines,
                },
            )
        })
        .collect();

    // Overall
    let _n = results.len() as f64;
    let overall_pipelines = pipeline_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let sf1_vals: Vec<f64> = results
                .iter()
                .map(|r| r.results[i].sf1)
                .filter(|v| !v.is_nan())
                .collect();
            let tf1_vals: Vec<f64> = results
                .iter()
                .map(|r| r.results[i].tf1)
                .filter(|v| !v.is_nan())
                .collect();
            let avg_sf1 = if sf1_vals.is_empty() {
                0.0
            } else {
                sf1_vals.iter().sum::<f64>() / sf1_vals.len() as f64
            };
            let avg_tf1 = if tf1_vals.is_empty() {
                0.0
            } else {
                tf1_vals.iter().sum::<f64>() / tf1_vals.len() as f64
            };
            FormatPipelineSummary {
                pipeline: name.clone(),
                avg_sf1,
                avg_tf1,
            }
        })
        .collect();

    let output = ComparisonJsonOutput {
        documents: results.to_vec(),
        by_format,
        overall: OverallSummary {
            total_documents: results.len(),
            pipelines: overall_pipelines,
        },
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::Error::Io)?;
    }
    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| crate::Error::Benchmark(format!("Failed to serialize comparison JSON: {}", e)))?;
    std::fs::write(path, json).map_err(crate::Error::Io)?;

    Ok(())
}

/// Data-driven guardrails configuration loaded from JSON.
#[derive(Debug, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    pub version: String,
    pub generated_at: String,
    pub threshold_factor: f64,
    pub contracts: Vec<GuardrailContract>,
}

/// A single guardrail contract: minimum threshold for a specific document + pipeline.
#[derive(Debug, Serialize, Deserialize)]
pub struct GuardrailContract {
    pub doc: String,
    pub pipeline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_sf1: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_tf1: Option<f64>,
}

/// Load a guardrails configuration from a JSON file.
pub fn load_guardrails(path: &Path) -> Result<GuardrailsConfig> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| crate::Error::Benchmark(format!("Failed to read guardrails file {}: {}", path.display(), e)))?;
    serde_json::from_str(&data).map_err(|e| crate::Error::Benchmark(format!("Failed to parse guardrails file: {}", e)))
}

/// Generate a guardrails configuration from benchmark results.
///
/// Each document+pipeline pair with meaningful scores produces a contract
/// whose minimum thresholds are the observed score multiplied by
/// `threshold_factor` (e.g. 0.9 means 90% of observed score).
pub fn generate_guardrails(results: &[DocResult], threshold_factor: f64) -> GuardrailsConfig {
    let mut contracts = Vec::new();
    for doc in results {
        for result in &doc.results {
            // Skip NaN scores (timeouts/errors)
            if result.sf1.is_nan() || result.tf1.is_nan() {
                continue;
            }
            // Only include docs with meaningful scores
            if result.sf1 < 0.01 && result.tf1 < 0.01 {
                continue;
            }
            contracts.push(GuardrailContract {
                doc: doc.name.clone(),
                pipeline: result.pipeline.name().to_string(),
                min_sf1: if result.sf1 > 0.01 {
                    Some((result.sf1 * threshold_factor * 100.0).round() / 100.0)
                } else {
                    None
                },
                min_tf1: if result.tf1 > 0.01 {
                    Some((result.tf1 * threshold_factor * 100.0).round() / 100.0)
                } else {
                    None
                },
            });
        }
    }
    GuardrailsConfig {
        version: "1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        threshold_factor,
        contracts,
    }
}

/// Check guardrails from a loaded config, returning a list of failure messages (empty = all passed).
pub fn check_guardrails(results: &[DocResult], config: &GuardrailsConfig) -> Vec<String> {
    let mut failures = Vec::new();

    for contract in &config.contracts {
        let Some(doc) = results.iter().find(|r| r.name == contract.doc) else {
            continue;
        };

        let Some(pr) = doc.results.iter().find(|pr| pr.pipeline.name() == contract.pipeline) else {
            continue;
        };

        if let Some(min_sf1) = contract.min_sf1
            && pr.sf1 < min_sf1
        {
            failures.push(format!(
                "SF1 regression: {} {} SF1 {:.1}% < minimum {:.1}%",
                contract.doc,
                contract.pipeline,
                pr.sf1 * 100.0,
                min_sf1 * 100.0,
            ));
        }

        if let Some(min_tf1) = contract.min_tf1
            && pr.tf1 < min_tf1
        {
            failures.push(format!(
                "TF1 regression: {} {} TF1 {:.1}% < minimum {:.1}%",
                contract.doc,
                contract.pipeline,
                pr.tf1 * 100.0,
                min_tf1 * 100.0,
            ));
        }
    }

    failures
}

/// Run comparison with guardrails and return exit code (0 = pass, 1 = fail).
pub async fn run_with_guardrails(config: &ComparisonConfig) -> Result<i32> {
    let results = run_comparison(config).await?;
    print_comparison_table(&results);
    print_per_format_summary(&results);

    if let Some(ref json_path) = config.json_output {
        write_comparison_json(&results, json_path)?;
        eprintln!("\nComparison JSON written to: {}", json_path.display());
    }

    if config.guardrails {
        let guardrails_path = config
            .guardrails_file
            .as_deref()
            .unwrap_or_else(|| Path::new("guardrails.json"));
        let guardrails_config = load_guardrails(guardrails_path)?;
        eprintln!(
            "Loaded {} guardrail contracts from {} (v{}, factor {})",
            guardrails_config.contracts.len(),
            guardrails_path.display(),
            guardrails_config.version,
            guardrails_config.threshold_factor,
        );
        let failures = check_guardrails(&results, &guardrails_config);
        if !failures.is_empty() {
            eprintln!("\nGUARDRAIL FAILURES:");
            for f in &failures {
                eprintln!("  {}", f);
            }
            return Ok(1);
        }
        eprintln!("\nAll guardrails passed.");
    }

    if config.noise {
        print_noise_summary(&results);
    }

    if config.diagnose {
        run_diagnostics(config, &results)?;
    }

    Ok(0)
}

/// Run diagnostic diff mode on documents with SF1 below the threshold.
fn run_diagnostics(config: &ComparisonConfig, results: &[DocResult]) -> Result<()> {
    use crate::diagnostics::{diagnose_document, write_diagnostic_files};

    // Rebuild corpus to access GT paths
    let filter = CorpusFilter {
        file_types: None,
        require_ground_truth: true,
        require_markdown_ground_truth: true,
        name_patterns: config.name_filter.clone().into_iter().collect(),
        ..Default::default()
    };
    let docs = corpus::build_corpus(&config.fixtures_dir, &filter)?;
    let doc_map: HashMap<String, &CorpusDocument> = docs.iter().map(|d| (d.name.clone(), d)).collect();

    let mut diagnosed_count = 0;

    for doc_result in results {
        let corpus_doc = match doc_map.get(&doc_result.name) {
            Some(d) => d,
            None => continue,
        };

        let gt_text = corpus_doc
            .ground_truth_text
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();

        let gt_markdown = corpus_doc
            .ground_truth_markdown
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok());

        for pr in &doc_result.results {
            if pr.sf1 < config.diagnose_threshold {
                let diag = diagnose_document(
                    &doc_result.name,
                    &doc_result.file_type,
                    pr.pipeline.name(),
                    &pr.content,
                    &gt_text,
                    gt_markdown.as_deref(),
                );

                if let Err(e) = write_diagnostic_files(&diag, gt_markdown.as_deref(), &pr.content) {
                    eprintln!(
                        "  Warning: failed to write diagnostics for {}/{}: {}",
                        doc_result.name,
                        pr.pipeline.name(),
                        e
                    );
                }

                diagnosed_count += 1;
            }
        }
    }

    if diagnosed_count > 0 {
        eprintln!(
            "\nDiagnosed {} document(s) with SF1 < {:.0}% -> /tmp/kreuzberg_diagnose/",
            diagnosed_count,
            config.diagnose_threshold * 100.0
        );
    } else {
        eprintln!(
            "\nNo documents below SF1 threshold ({:.0}%) — no diagnostics generated.",
            config.diagnose_threshold * 100.0
        );
    }

    Ok(())
}

/// Run noise detection on all pipeline outputs and print summary.
fn print_noise_summary(results: &[DocResult]) {
    use crate::noise_detection::{Severity, detect_noise};
    use std::collections::HashMap;

    eprintln!("\n{:=<70}", "");
    eprintln!("NOISE DETECTION SUMMARY");
    eprintln!("{:=<70}", "");

    let mut total_docs_with_noise = 0;
    let mut total_issues = 0;
    let mut kind_counts: HashMap<String, usize> = HashMap::new();
    let mut noisy_docs: Vec<(String, String, usize, usize, usize)> = Vec::new(); // (doc, pipeline, errors, warnings, infos)

    for doc_result in results {
        for pr in &doc_result.results {
            if pr.content.is_empty() {
                continue;
            }
            let report = detect_noise(&pr.content);
            if report.issues.is_empty() {
                continue;
            }
            total_docs_with_noise += 1;
            total_issues += report.issues.len();
            for issue in &report.issues {
                *kind_counts.entry(format!("{:?}", issue.kind)).or_insert(0) += 1;
            }
            let errors = report.issues.iter().filter(|i| i.severity == Severity::Error).count();
            let warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
            let infos = report.issues.iter().filter(|i| i.severity == Severity::Info).count();
            noisy_docs.push((
                doc_result.name.clone(),
                pr.pipeline.name().to_string(),
                errors,
                warnings,
                infos,
            ));
        }
    }

    if total_docs_with_noise == 0 {
        eprintln!("No noise detected in any extracted output.");
        return;
    }

    eprintln!(
        "{} documents with noise issues ({} total issues)",
        total_docs_with_noise, total_issues
    );

    // Print kind breakdown
    eprintln!("\nBy kind:");
    let mut sorted_kinds: Vec<_> = kind_counts.into_iter().collect();
    sorted_kinds.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (kind, count) in &sorted_kinds {
        eprintln!("  {:<30} {}", kind, count);
    }

    // Print top noisy docs (sorted by errors desc, then warnings desc)
    noisy_docs.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));
    let show = noisy_docs.len().min(20);
    eprintln!("\nTop {} noisy documents:", show);
    eprintln!(
        "  {:<30} {:<15} {:>6} {:>6} {:>6}",
        "Document", "Pipeline", "Errors", "Warns", "Infos"
    );
    for (doc, pipeline, errors, warnings, infos) in noisy_docs.iter().take(show) {
        eprintln!(
            "  {:<30} {:<15} {:>6} {:>6} {:>6}",
            doc, pipeline, errors, warnings, infos
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_document_identical() {
        let text = "Hello world this is a test document";
        let (tf1, sf1, order_score, per_type) = score_document(text, text, None);
        assert!(
            (tf1 - 1.0).abs() < f64::EPSILON,
            "TF1 should be 1.0 for identical text, got {tf1}"
        );
        assert!(
            (sf1 - 0.0).abs() < f64::EPSILON,
            "SF1 should be 0.0 when no markdown GT"
        );
        assert!(
            (order_score - 0.0).abs() < f64::EPSILON,
            "order_score should be 0.0 when no markdown GT"
        );
        assert!(per_type.is_empty(), "per_type should be empty when no markdown GT");
    }

    #[test]
    fn test_score_document_no_markdown_gt() {
        let content = "Some extracted content here";
        let gt_text = "Some ground truth content here";
        let (tf1, sf1, order_score, per_type) = score_document(content, gt_text, None);
        assert!(
            tf1 > 0.0 && tf1 < 1.0,
            "TF1 should be between 0 and 1 for partially matching text, got {tf1}"
        );
        assert!(
            (sf1 - 0.0).abs() < f64::EPSILON,
            "SF1 should be 0.0 when no markdown GT"
        );
        assert!((order_score - 0.0).abs() < f64::EPSILON);
        assert!(per_type.is_empty());
    }

    #[test]
    fn test_score_document_empty() {
        let (tf1, sf1, order_score, per_type) = score_document("", "", None);
        // F1 of two empty token sets: compute_f1 returns 1.0 when both are empty
        // (or 0.0 depending on implementation). Just check it doesn't panic.
        let _ = tf1;
        assert!((sf1 - 0.0).abs() < f64::EPSILON);
        assert!((order_score - 0.0).abs() < f64::EPSILON);
        assert!(per_type.is_empty());
    }

    #[test]
    fn test_score_document_completely_different() {
        let content = "alpha bravo charlie";
        let gt_text = "delta echo foxtrot";
        let (tf1, _sf1, _order_score, _per_type) = score_document(content, gt_text, None);
        assert!(
            (tf1 - 0.0).abs() < f64::EPSILON,
            "TF1 should be 0.0 for completely different text, got {tf1}"
        );
    }

    #[test]
    fn test_score_document_with_structure() {
        let content = "# Heading\n\nSome paragraph text.\n";
        let gt_markdown = "# Heading\n\nSome paragraph text.\n";
        let gt_text = "Heading Some paragraph text.";
        let (tf1, sf1, _order_score, per_type) = score_document(content, gt_text, Some(gt_markdown));
        assert!(tf1 > 0.0, "TF1 should be positive, got {tf1}");
        assert!(
            sf1 > 0.0,
            "SF1 should be positive when structural GT is provided, got {sf1}"
        );
        // per_type may or may not have entries depending on scoring internals
        let _ = per_type;
    }

    #[test]
    fn test_pipeline_config_deterministic() {
        for pipeline in Pipeline::all_kreuzberg() {
            let config = build_extraction_config(pipeline);
            // Verify the config is valid (doesn't panic) and has markdown output
            assert_eq!(
                format!("{:?}", config.output_format),
                format!("{:?}", kreuzberg::core::config::OutputFormat::Markdown),
                "Pipeline {:?} should produce Markdown config",
                pipeline
            );
        }
    }

    #[test]
    fn test_read_vendored_cached_missing() {
        let tmp = std::env::temp_dir().join("benchmark_harness_test_nonexistent");
        let (content, time_ms) = read_vendored_cached("no_such_doc", &tmp, "no_such_vendor");
        assert!(content.is_empty(), "Content should be empty for missing vendored file");
        assert!(time_ms.is_nan(), "time_ms should be NaN for missing timing file");
    }

    #[test]
    fn test_pipeline_parse_roundtrip() {
        let all_names = [
            "baseline",
            "layout",
            "tesseract",
            "tesseract+layout",
            "paddle",
            "paddle+layout",
            "paddle-server",
            "paddle-server+layout",
            "tesseract-autorotate",
            "paddle-norotate",
            "docling",
            "paddleocr-python",
            "rapidocr",
            "layout+slanet-wired",
            "layout+slanet-wireless",
            "layout+slanet-plus",
            "layout+slanet-auto",
            "pdf-oxide",
            "pdf-oxide+layout",
        ];
        for name in all_names {
            let pipeline = Pipeline::parse(name).unwrap_or_else(|| panic!("Failed to parse pipeline '{name}'"));
            let roundtrip = pipeline.name();
            let reparsed =
                Pipeline::parse(roundtrip).unwrap_or_else(|| panic!("Failed to reparse pipeline '{roundtrip}'"));
            assert_eq!(pipeline, reparsed, "Roundtrip failed for '{name}' -> '{roundtrip}'");
        }
    }
}
