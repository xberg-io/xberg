//! Pipeline execution: extract a document through one [`Pipeline`] (live in-process extraction,
//! or reading pre-computed vendored output), score it, and run the full comparison across every
//! document and pipeline.

use super::extraction_config::{
    apply_fixture_ocr_language, build_extraction_config, compute_extraction_timeout_secs,
    finalize_timed_ocr_result_cache, materialize_implicit_ocr_config,
};
use super::pipeline::Pipeline;
use super::scoring::{StructuralBreakdown, read_vendored_cached, score_document};
use crate::Result;
use crate::corpus::{self, CorpusDocument, CorpusFilter};
use crate::quality::{compute_token_diff, tokenize};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

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
    /// Optional exact fixture `metadata.category` filter.
    pub category_filter: Option<String>,
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
    /// Order-/character-sensitive text similarity (`1 − CER`, 0.0–1.0). Report-only
    /// OCR diagnostic; NaN for empty or very long documents (see `quality`).
    #[serde(default)]
    pub char_similarity: f64,
    /// Reading order score (LIS-based, 0.0-1.0).
    #[serde(default)]
    pub order_score: f64,
    /// Per-block-type structural F1 scores (e.g. "H1" -> 0.85).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_type_sf1: HashMap<String, f64>,
    /// Per-dimension precision (report-only): low value ⇒ over-fabrication.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_type_precision: HashMap<String, f64>,
    /// Per-dimension recall (report-only): low value ⇒ omission.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_type_recall: HashMap<String, f64>,
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

/// Extract content from a document using the given pipeline.
/// Returns (content, time_ms).
pub async fn extract_pipeline(
    pipeline: Pipeline,
    doc: &crate::corpus::CorpusDocument,
    fixtures_dir: &std::path::Path,
) -> (Option<String>, f64) {
    match pipeline {
        Pipeline::Docling | Pipeline::PaddleOcrPython | Pipeline::RapidOcr => {
            extract_vendored_pipeline(pipeline, &doc.name, fixtures_dir)
        }
        _ => extract_live_pipeline(pipeline, doc).await,
    }
}

/// The `Docling`/`PaddleOcrPython`/`RapidOcr` branch of [`extract_pipeline`]: these read
/// pre-computed output from disk rather than extracting live.
fn extract_vendored_pipeline(
    pipeline: Pipeline,
    doc_name: &str,
    fixtures_dir: &std::path::Path,
) -> (Option<String>, f64) {
    let vendored_name = match pipeline {
        Pipeline::PaddleOcrPython => "paddleocr-python",
        Pipeline::RapidOcr => "rapidocr",
        _ => "docling",
    };
    match read_vendored_cached(doc_name, fixtures_dir, vendored_name) {
        Ok((content, time_ms)) => (Some(content), time_ms),
        Err(error) => {
            eprintln!("  ERROR {}/{}: {}", doc_name, pipeline.name(), error);
            (None, f64::NAN)
        }
    }
}

/// The live in-process extraction branch of [`extract_pipeline`], for every pipeline other than
/// the vendored-output ones.
async fn extract_live_pipeline(pipeline: Pipeline, doc: &crate::corpus::CorpusDocument) -> (Option<String>, f64) {
    let t = Instant::now();
    let mut config = build_extraction_config(pipeline);
    // Materialize implicit OCR before applying fixture metadata so baseline PDF extraction
    // keeps force_ocr disabled while its fallback OCR uses the requested language, then
    // finalize the OCR result-cache control on the final config. ~keep
    materialize_implicit_ocr_config(&mut config);
    apply_fixture_ocr_language(&mut config, doc);
    finalize_timed_ocr_result_cache(&mut config);
    let doc_path = doc.document_path.clone();
    let doc_name = doc.name.clone();
    let pipeline_name = pipeline.name().to_string();

    let extraction_timeout_secs = compute_extraction_timeout_secs(&doc_path, &doc.file_type, config.force_ocr);
    config.extraction_timeout_secs = Some(extraction_timeout_secs);

    let outer_timeout_secs = ((extraction_timeout_secs as f64 * 1.5).ceil() as u64).max(180);
    let extraction_future = async {
        tokio::time::timeout(
            std::time::Duration::from_secs(outer_timeout_secs),
            crate::extract_xberg_file(&doc_path, &config),
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
            eprintln!(
                "  TIMEOUT {}/{}: exceeded {}s (outer safety timeout)",
                doc_name, pipeline_name, outer_timeout_secs
            );
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

/// Run a single pipeline on a single document and score it.
///
/// Timeouts, errors, and panics produce NaN scores so they are tracked
/// in the results but excluded from aggregate averages.
pub(super) async fn run_pipeline(
    pipeline: Pipeline,
    doc: &CorpusDocument,
    gt_text: &str,
    gt_markdown: Option<&str>,
    fixtures_dir: &std::path::Path,
) -> PipelineResult {
    let (content_opt, time_ms) = extract_pipeline(pipeline, doc, fixtures_dir).await;
    let extraction_failed = content_opt.is_none();
    let content = content_opt.unwrap_or_default();
    let (tf1, structural) = if extraction_failed || (content.is_empty() && time_ms > 170_000.0) {
        (
            f64::NAN,
            StructuralBreakdown {
                sf1: f64::NAN,
                order_score: f64::NAN,
                ..Default::default()
            },
        )
    } else {
        score_document(&content, gt_text, gt_markdown)
    };

    let char_similarity = if extraction_failed {
        f64::NAN
    } else {
        crate::quality::normalized_edit_similarity(&content, gt_text)
    };

    let ext_tokens = tokenize(&content);
    let gt_tokens = tokenize(gt_text);
    let (mut missing_tokens, mut extra_tokens) = compute_token_diff(&ext_tokens, &gt_tokens);
    missing_tokens.truncate(50);
    extra_tokens.truncate(50);

    PipelineResult {
        pipeline,
        sf1: structural.sf1,
        tf1,
        char_similarity,
        order_score: structural.order_score,
        per_type_sf1: structural.per_type_sf1,
        per_type_precision: structural.per_type_precision,
        per_type_recall: structural.per_type_recall,
        time_ms,
        missing_tokens,
        extra_tokens,
        content,
    }
}

/// Run the full comparison across all documents and pipelines.
pub async fn run_comparison(config: &ComparisonConfig) -> Result<Vec<DocResult>> {
    let filter = CorpusFilter {
        file_types: None,
        require_ground_truth: true,
        require_markdown_ground_truth: false,
        name_patterns: config.name_filter.clone().into_iter().collect(),
        category: config.category_filter.clone(),
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
                let dump_dir = std::path::PathBuf::from("/tmp/xberg_compare");
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
