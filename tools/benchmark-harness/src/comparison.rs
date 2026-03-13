//! Framework comparison: run multiple extraction pipelines on the corpus and
//! compare quality (SF1, TF1) against ground truth with optional guardrails.
//!
//! Replaces the logic previously in `crates/kreuzberg/tests/framework_comparison.rs`.
//! Uses canonical scoring from [`crate::markdown_quality`] and [`crate::quality`].

use crate::Result;
use crate::corpus::{self, CorpusDocument, CorpusFilter};
use crate::markdown_quality::score_structural_quality;
use crate::quality::{compute_f1, tokenize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// PaddleOCR (force_ocr)
    Paddle,
    /// PaddleOCR + layout detection
    PaddleLayout,
    /// Docling vendored extraction (read from file)
    Docling,
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
            Pipeline::Docling => "docling",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "baseline" => Some(Pipeline::Baseline),
            "layout" => Some(Pipeline::Layout),
            "tesseract" => Some(Pipeline::Tesseract),
            "tesseract+layout" | "tesseract-layout" => Some(Pipeline::TesseractLayout),
            "paddle" => Some(Pipeline::Paddle),
            "paddle+layout" | "paddle-layout" => Some(Pipeline::PaddleLayout),
            "docling" => Some(Pipeline::Docling),
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
        ]
    }
}

/// Configuration for a comparison run.
pub struct ComparisonConfig {
    pub fixtures_dir: std::path::PathBuf,
    pub pipelines: Vec<Pipeline>,
    pub dump_outputs: bool,
    pub guardrails: bool,
    /// Optional name filter (only run docs whose name contains this)
    pub name_filter: Option<String>,
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
    #[serde(skip)]
    pub content: String,
}

/// Result of running all pipelines on one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocResult {
    pub name: String,
    pub results: Vec<PipelineResult>,
}

/// Build a kreuzberg ExtractionConfig for the given pipeline.
fn build_extraction_config(pipeline: Pipeline) -> kreuzberg::ExtractionConfig {
    use kreuzberg::core::config::{OutputFormat, layout::LayoutDetectionConfig};

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
            layout: Some(LayoutDetectionConfig {
                preset: "fast".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::Paddle => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
                backend: "paddleocr".to_string(),
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..base
        },
        Pipeline::PaddleLayout => kreuzberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(kreuzberg::core::config::OcrConfig {
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
        Pipeline::Docling => base, // Not used for extraction — read from file
    }
}

/// Run a single pipeline on a single document and score it.
async fn run_pipeline(
    pipeline: Pipeline,
    doc: &CorpusDocument,
    gt_text: &str,
    gt_markdown: Option<&str>,
) -> PipelineResult {
    let (content, time_ms) = match pipeline {
        Pipeline::Docling => {
            // Docling: read vendored (pre-computed) output — no meaningful extraction time
            let docling_dir = doc
                .document_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("vendored/docling/md"));

            let md = if let Some(dir) = docling_dir {
                let md_name = doc.document_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let md_path = dir.join(format!("{}.md", md_name));
                std::fs::read_to_string(&md_path).unwrap_or_default()
            } else {
                String::new()
            };
            (md, f64::NAN)
        }
        _ => {
            let t = Instant::now();
            let config = build_extraction_config(pipeline);
            let result = match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                kreuzberg::core::batch_mode::with_batch_mode(kreuzberg::extract_file(
                    &doc.document_path,
                    None,
                    &config,
                )),
            )
            .await
            {
                Ok(Ok(result)) => result.content,
                Ok(Err(e)) => {
                    eprintln!("  ERROR {}/{}: {}", doc.name, pipeline.name(), e);
                    String::new()
                }
                Err(_) => {
                    eprintln!("  TIMEOUT {}/{}: exceeded 60s", doc.name, pipeline.name());
                    String::new()
                }
            };
            (result, t.elapsed().as_secs_f64() * 1000.0)
        }
    };

    // Score
    let tf1 = {
        let ext_tokens = tokenize(&content);
        let gt_tokens = tokenize(gt_text);
        compute_f1(&ext_tokens, &gt_tokens)
    };

    let (sf1, order_score, per_type_sf1) = match gt_markdown {
        Some(md) => {
            let sq = score_structural_quality(&content, md);
            let per_type: HashMap<String, f64> = sq.per_type.iter().map(|(k, v)| (k.to_string(), v.f1)).collect();
            (sq.structural_f1, sq.order_score, per_type)
        }
        None => (0.0, 0.0, HashMap::new()),
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

/// Run the full comparison across all documents and pipelines.
pub async fn run_comparison(config: &ComparisonConfig) -> Result<Vec<DocResult>> {
    let filter = CorpusFilter {
        file_types: Some(vec!["pdf".to_string()]),
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
            let result = run_pipeline(pipeline, doc, &gt_text, gt_markdown.as_deref()).await;

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
        eprint!(" {:>10} {:>10}", format!("{} SF1", name), format!("{} TF1", name));
    }
    eprintln!();
    eprintln!("{}", "-".repeat(25 + pipeline_names.len() * 22));

    // Rows
    for doc in results {
        eprint!("{:<25}", doc.name);
        for pr in &doc.results {
            eprint!(" {:>9.1}% {:>9.1}%", pr.sf1 * 100.0, pr.tf1 * 100.0);
        }
        eprintln!();
    }

    // Averages
    eprintln!("{}", "-".repeat(25 + pipeline_names.len() * 22));
    eprint!("{:<25}", "AVERAGE");
    let n = results.len() as f64;
    for (i, _) in pipeline_names.iter().enumerate() {
        let avg_sf1: f64 = results.iter().map(|r| r.results[i].sf1).sum::<f64>() / n;
        let avg_tf1: f64 = results.iter().map(|r| r.results[i].tf1).sum::<f64>() / n;
        eprint!(" {:>9.1}% {:>9.1}%", avg_sf1 * 100.0, avg_tf1 * 100.0);
    }
    eprintln!();
}

/// A quality guardrail: minimum threshold for a specific document + pipeline.
struct Guardrail {
    doc_name: &'static str,
    pipeline: Pipeline,
    min_sf1: Option<f64>,
    min_tf1: Option<f64>,
}

/// Quality guardrails — minimum thresholds for specific documents and pipelines.
/// Thresholds are set to ~90% of observed baseline scores (floor).
/// Generated from initial_baseline.json (2026-03-05, git 6c7598f43).
const GUARDRAILS: &[Guardrail] = &[
    // Baseline pipeline guardrails — representative docs across score ranges
    // Perfect scores (100% SF1)
    Guardrail {
        doc_name: "extra-attrs-example",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.90),
        min_tf1: Some(0.90),
    },
    Guardrail {
        doc_name: "test-punkt",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.90),
        min_tf1: Some(0.90),
    },
    // High SF1 (>60%)
    Guardrail {
        doc_name: "pdfa_032",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.73),
        min_tf1: Some(0.88),
    },
    Guardrail {
        doc_name: "pdf_tiny_memo",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.72),
        min_tf1: Some(0.90),
    },
    Guardrail {
        doc_name: "nougat_011",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.65),
        min_tf1: Some(0.89),
    },
    Guardrail {
        doc_name: "pdf_tables",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.62),
        min_tf1: Some(0.60),
    },
    Guardrail {
        doc_name: "la-precinct-bulletin-2014-p1",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.57),
        min_tf1: Some(0.88),
    },
    // Medium SF1 (30-60%)
    Guardrail {
        doc_name: "amt_handbook_sample",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.47),
        min_tf1: Some(0.84),
    },
    Guardrail {
        doc_name: "multi_page",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.44),
        min_tf1: Some(0.90),
    },
    Guardrail {
        doc_name: "pdf_medium",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.39),
        min_tf1: Some(0.86),
    },
    Guardrail {
        doc_name: "2305.03393v1",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.37),
        min_tf1: Some(0.88),
    },
    Guardrail {
        doc_name: "pdf_google_docs",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.33),
        min_tf1: Some(0.84),
    },
    Guardrail {
        doc_name: "pdf_structure",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.28),
        min_tf1: Some(0.90),
    },
    // Tables
    Guardrail {
        doc_name: "nics-background-checks-2015-11",
        pipeline: Pipeline::Baseline,
        min_sf1: Some(0.27),
        min_tf1: Some(0.90),
    },
];

/// Check guardrails, returning a list of failure messages (empty = all passed).
pub fn check_guardrails(results: &[DocResult]) -> Vec<String> {
    let mut failures = Vec::new();

    for guardrail in GUARDRAILS {
        let Some(doc) = results.iter().find(|r| r.name == guardrail.doc_name) else {
            continue;
        };

        let Some(pr) = doc.results.iter().find(|pr| pr.pipeline == guardrail.pipeline) else {
            continue;
        };

        if let Some(min_sf1) = guardrail.min_sf1
            && pr.sf1 < min_sf1
        {
            failures.push(format!(
                "SF1 regression: {} {} SF1 {:.1}% < minimum {:.1}%",
                guardrail.doc_name,
                pr.pipeline.name(),
                pr.sf1 * 100.0,
                min_sf1 * 100.0,
            ));
        }

        if let Some(min_tf1) = guardrail.min_tf1
            && pr.tf1 < min_tf1
        {
            failures.push(format!(
                "TF1 regression: {} {} TF1 {:.1}% < minimum {:.1}%",
                guardrail.doc_name,
                pr.pipeline.name(),
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

    if config.guardrails {
        let failures = check_guardrails(&results);
        if !failures.is_empty() {
            eprintln!("\nGUARDRAIL FAILURES:");
            for f in &failures {
                eprintln!("  {}", f);
            }
            return Ok(1);
        }
        eprintln!("\nAll guardrails passed.");
    }

    Ok(0)
}
