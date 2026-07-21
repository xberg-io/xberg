//! Fast in-process diagnostic for Xberg batch scheduling.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use xberg::core::config::ConcurrencyConfig;
use xberg::{ExtractedDocument, ExtractionConfig};

use crate::{Error, Result, extract_xberg_file, extract_xberg_files};

/// Inputs and iteration controls for a batch diagnostic run.
#[derive(Debug, Clone)]
pub struct BatchDiagnosticConfig {
    pub inputs: Vec<PathBuf>,
    pub batch_size: usize,
    pub warmup_iterations: usize,
    pub iterations: usize,
    pub extraction_config_json: Option<String>,
    pub max_threads: Option<usize>,
    pub max_concurrent_extractions: Option<usize>,
}

/// Compact measurements suitable for terminal output or JSON automation.
#[derive(Debug, Clone, Serialize)]
pub struct BatchDiagnosticReport {
    pub batch_size: usize,
    pub iterations: usize,
    pub sequential_median_ms: f64,
    pub batch_median_ms: f64,
    pub speedup: f64,
    pub sequential_documents_per_second: f64,
    pub batch_documents_per_second: f64,
    pub outputs_match: bool,
    pub sequential_samples_ms: Vec<f64>,
    pub batch_samples_ms: Vec<f64>,
}

/// Compare warmed sequential extraction with the public native batch API.
pub async fn run_batch_diagnostic(config: &BatchDiagnosticConfig) -> Result<BatchDiagnosticReport> {
    validate_config(config)?;
    let inputs = expanded_inputs(config);
    let extraction_config = resolve_extraction_config(config)?;

    for _ in 0..config.warmup_iterations {
        let sequential = extract_sequential(&inputs, &extraction_config).await?;
        let batch = extract_batch(&inputs, &extraction_config).await?;
        ensure_equivalent(&inputs, &sequential, &batch)?;
    }

    let mut sequential_samples = Vec::with_capacity(config.iterations);
    let mut batch_samples = Vec::with_capacity(config.iterations);
    for iteration in 0..config.iterations {
        let (sequential, sequential_elapsed, batch, batch_elapsed) = if iteration % 2 == 0 {
            let (sequential, sequential_elapsed) = timed_sequential(&inputs, &extraction_config).await?;
            let (batch, batch_elapsed) = timed_batch(&inputs, &extraction_config).await?;
            (sequential, sequential_elapsed, batch, batch_elapsed)
        } else {
            let (batch, batch_elapsed) = timed_batch(&inputs, &extraction_config).await?;
            let (sequential, sequential_elapsed) = timed_sequential(&inputs, &extraction_config).await?;
            (sequential, sequential_elapsed, batch, batch_elapsed)
        };
        ensure_equivalent(&inputs, &sequential, &batch)?;
        sequential_samples.push(duration_ms(sequential_elapsed));
        batch_samples.push(duration_ms(batch_elapsed));
    }

    let sequential_median_ms = median(&sequential_samples);
    let batch_median_ms = median(&batch_samples);
    let batch_size = inputs.len();
    Ok(BatchDiagnosticReport {
        batch_size,
        iterations: config.iterations,
        sequential_median_ms,
        batch_median_ms,
        speedup: sequential_median_ms / batch_median_ms,
        sequential_documents_per_second: documents_per_second(batch_size, sequential_median_ms),
        batch_documents_per_second: documents_per_second(batch_size, batch_median_ms),
        outputs_match: true,
        sequential_samples_ms: sequential_samples,
        batch_samples_ms: batch_samples,
    })
}

fn validate_config(config: &BatchDiagnosticConfig) -> Result<()> {
    if config.inputs.is_empty() {
        return Err(Error::Config("at least one input is required".into()));
    }
    if config.batch_size == 0 || config.iterations == 0 {
        return Err(Error::Config(
            "batch size and iterations must be greater than zero".into(),
        ));
    }
    if config.max_threads == Some(0) || config.max_concurrent_extractions == Some(0) {
        return Err(Error::Config(
            "maximum threads and concurrent extractions must be greater than zero".into(),
        ));
    }
    if config.batch_size < config.inputs.len() {
        return Err(Error::Config(format!(
            "batch size {} is smaller than the {} explicit inputs",
            config.batch_size,
            config.inputs.len()
        )));
    }
    if let Some(path) = config.inputs.iter().find(|path| !path.is_file()) {
        return Err(Error::DocumentNotFound(path.clone()));
    }
    Ok(())
}

fn resolve_extraction_config(config: &BatchDiagnosticConfig) -> Result<ExtractionConfig> {
    let mut extraction_config = match config.extraction_config_json.as_deref() {
        Some(raw) => parse_extraction_config_json(raw)?,
        None => ExtractionConfig::default(),
    };

    extraction_config.use_cache = false;
    disable_ocr_result_caches(&mut extraction_config, &config.inputs)?;
    if let Some(max_concurrent_extractions) = config.max_concurrent_extractions {
        extraction_config.max_concurrent_extractions = Some(max_concurrent_extractions);
    }
    if let Some(max_threads) = config.max_threads {
        extraction_config
            .concurrency
            .get_or_insert_with(ConcurrencyConfig::default)
            .max_threads = Some(max_threads);
    }
    if extraction_config.max_concurrent_extractions == Some(0)
        || extraction_config
            .concurrency
            .as_ref()
            .and_then(|value| value.max_threads)
            == Some(0)
    {
        return Err(Error::Config(
            "configured maximum threads and concurrent extractions must be greater than zero".into(),
        ));
    }
    Ok(extraction_config)
}

fn disable_ocr_result_caches(config: &mut ExtractionConfig, inputs: &[PathBuf]) -> Result<()> {
    if config.disable_ocr || config.ocr.as_ref().is_some_and(|ocr| !ocr.enabled) {
        return Ok(());
    }

    if config.ocr.is_none() && !config.force_ocr && config.force_ocr_pages.as_ref().is_none_or(Vec::is_empty) {
        reject_implicit_ocr_caches(config, inputs)?;
        return Ok(());
    }
    let ocr = config.ocr.get_or_insert_with(xberg::OcrConfig::default);
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        for stage in &mut pipeline.stages {
            if stage.backend == "tesseract" {
                stage
                    .tesseract_config
                    .get_or_insert_with(xberg::TesseractConfig::default)
                    .use_cache = false;
            }
        }
    } else if ocr.backend == "tesseract" {
        ocr.tesseract_config
            .get_or_insert_with(xberg::TesseractConfig::default)
            .use_cache = false;
    }
    Ok(())
}

fn reject_implicit_ocr_caches(config: &ExtractionConfig, inputs: &[PathBuf]) -> Result<()> {
    if matches!(config.ocr_strategy, xberg::OcrStrategy::ScannedPages { .. }) {
        return Err(implicit_ocr_config_error("scanned-page OCR"));
    }
    if config
        .pdf_options
        .as_ref()
        .is_some_and(|options| options.ocr_inline_images)
    {
        return Err(implicit_ocr_config_error("PDF inline-image OCR"));
    }
    for input in inputs {
        let mime = xberg::detect_mime_type(input.to_string_lossy().into_owned(), true)
            .map_err(|error| Error::Config(format!("failed to detect input type for {}: {error}", input.display())))?;
        if mime.starts_with("image/") {
            return Err(implicit_ocr_config_error("image OCR"));
        }
    }
    Ok(())
}

fn implicit_ocr_config_error(path: &str) -> Error {
    Error::Config(format!(
        "{path} uses an implicit OCR configuration whose backend cache cannot be disabled safely; provide an explicit `ocr` configuration"
    ))
}

fn parse_extraction_config_json(raw: &str) -> Result<ExtractionConfig> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| Error::Config(format!("invalid --config-json JSON: {error}")))?;
    if !value.is_object() {
        return Err(Error::Config(
            "invalid --config-json extraction configuration: expected a JSON object".into(),
        ));
    }
    let mut ignored = Vec::new();
    let extraction = serde_ignored::deserialize(value, |path| ignored.push(path.to_string()))
        .map_err(|error| Error::Config(format!("invalid --config-json extraction configuration: {error}")))?;
    if ignored.is_empty() {
        Ok(extraction)
    } else {
        Err(Error::Config(format!(
            "invalid --config-json extraction configuration: unknown field(s): {}",
            ignored.join(", ")
        )))
    }
}

fn expanded_inputs(config: &BatchDiagnosticConfig) -> Vec<PathBuf> {
    config.inputs.iter().cycle().take(config.batch_size).cloned().collect()
}

async fn timed_sequential(inputs: &[PathBuf], config: &ExtractionConfig) -> Result<(Vec<ExtractedDocument>, Duration)> {
    let started = Instant::now();
    let documents = extract_sequential(inputs, config).await?;
    Ok((documents, started.elapsed()))
}

async fn timed_batch(inputs: &[PathBuf], config: &ExtractionConfig) -> Result<(Vec<ExtractedDocument>, Duration)> {
    let started = Instant::now();
    let documents = extract_batch(inputs, config).await?;
    Ok((documents, started.elapsed()))
}

async fn extract_sequential(inputs: &[PathBuf], config: &ExtractionConfig) -> Result<Vec<ExtractedDocument>> {
    let mut documents = Vec::with_capacity(inputs.len());
    for path in inputs {
        documents.push(extract_xberg_file(path, config).await.map_err(extraction_error)?);
    }
    Ok(documents)
}

async fn extract_batch(inputs: &[PathBuf], config: &ExtractionConfig) -> Result<Vec<ExtractedDocument>> {
    extract_xberg_files(inputs, config).await.map_err(extraction_error)
}

fn extraction_error(error: xberg::XbergError) -> Error {
    Error::Benchmark(format!("Xberg extraction failed: {error}"))
}

fn ensure_equivalent(inputs: &[PathBuf], sequential: &[ExtractedDocument], batch: &[ExtractedDocument]) -> Result<()> {
    validate_batch_mapping(inputs, batch)?;
    if sequential.len() != batch.len() {
        return Err(Error::Benchmark(format!(
            "sequential returned {} documents but batch returned {}",
            sequential.len(),
            batch.len()
        )));
    }
    for (index, (sequential, batch)) in sequential.iter().zip(batch).enumerate() {
        if normalized_document(sequential)? != normalized_document(batch)? {
            return Err(Error::Benchmark(format!(
                "sequential and batch payloads differ at input index {index} ({})",
                inputs[index].display()
            )));
        }
    }
    Ok(())
}

fn validate_batch_mapping(inputs: &[PathBuf], batch: &[ExtractedDocument]) -> Result<()> {
    if inputs.len() != batch.len() {
        return Err(Error::Benchmark(format!(
            "batch returned {} documents for {} inputs",
            batch.len(),
            inputs.len()
        )));
    }
    for (expected_index, (input, document)) in inputs.iter().zip(batch).enumerate() {
        let actual_index = document
            .metadata
            .additional
            .get("source_index")
            .and_then(serde_json::Value::as_u64);
        if actual_index != Some(expected_index as u64) {
            return Err(Error::Benchmark(format!(
                "batch result at position {expected_index} has source_index {actual_index:?}"
            )));
        }
        let expected_source = input.to_string_lossy();
        let actual_source = document
            .metadata
            .additional
            .get("source_uri")
            .and_then(serde_json::Value::as_str);
        if actual_source != Some(expected_source.as_ref()) {
            return Err(Error::Benchmark(format!(
                "batch result at position {expected_index} has source_uri {actual_source:?}, expected {expected_source}"
            )));
        }
    }
    Ok(())
}

fn normalized_document(document: &ExtractedDocument) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(document)?;
    let metadata = value
        .get_mut("metadata")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| Error::Benchmark("serialized extraction result has no metadata object".into()))?;

    metadata.remove("extraction_duration_ms");
    if let Some(additional) = metadata
        .get_mut("additional")
        .and_then(serde_json::Value::as_object_mut)
    {
        additional.remove("source_index");
    }
    Ok(value)
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn documents_per_second(batch_size: usize, duration_ms: f64) -> f64 {
    batch_size as f64 * 1_000.0 / duration_ms
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_inputs_cycles_deterministically() {
        let config = BatchDiagnosticConfig {
            inputs: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            batch_size: 5,
            warmup_iterations: 0,
            iterations: 1,
            extraction_config_json: None,
            max_threads: None,
            max_concurrent_extractions: None,
        };
        assert_eq!(
            expanded_inputs(&config),
            ["a.txt", "b.txt", "a.txt", "b.txt", "a.txt"].map(PathBuf::from)
        );
    }

    #[test]
    fn median_handles_even_and_odd_samples() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn validation_rejects_batch_size_smaller_than_explicit_inputs() {
        let config = BatchDiagnosticConfig {
            inputs: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            batch_size: 1,
            warmup_iterations: 0,
            iterations: 1,
            extraction_config_json: None,
            max_threads: None,
            max_concurrent_extractions: None,
        };
        assert!(matches!(
            validate_config(&config),
            Err(Error::Config(message)) if message.contains("explicit inputs")
        ));
    }

    fn annotated_document(content: &str, source_index: u64, source_uri: &str) -> ExtractedDocument {
        let mut document = ExtractedDocument::default();
        document.content = content.into();
        document
            .metadata
            .additional
            .insert("source_index".into(), source_index.into());
        document
            .metadata
            .additional
            .insert("source_uri".into(), source_uri.into());
        document
    }

    #[test]
    fn batch_mapping_rejects_out_of_order_results() {
        let inputs = vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")];
        let batch = vec![annotated_document("b", 1, "b.txt"), annotated_document("a", 0, "a.txt")];
        assert!(matches!(
            validate_batch_mapping(&inputs, &batch),
            Err(Error::Benchmark(message)) if message.contains("position 0")
        ));
    }

    #[test]
    fn measured_mismatch_is_an_error_even_without_warmup() {
        let inputs = vec![PathBuf::from("a.txt")];
        let sequential = vec![annotated_document("expected", 0, "a.txt")];
        let batch = vec![annotated_document("different", 0, "a.txt")];
        assert!(matches!(
            ensure_equivalent(&inputs, &sequential, &batch),
            Err(Error::Benchmark(message)) if message.contains("payloads differ")
        ));
    }

    fn diagnostic_config(extraction_config_json: Option<&str>) -> BatchDiagnosticConfig {
        BatchDiagnosticConfig {
            inputs: vec![fixture_path("pdf/tiny.pdf")],
            batch_size: 1,
            warmup_iterations: 0,
            iterations: 1,
            extraction_config_json: extraction_config_json.map(str::to_owned),
            max_threads: None,
            max_concurrent_extractions: None,
        }
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents")
            .join(relative)
    }

    #[test]
    fn config_json_enables_heuristic_markdown() {
        let config = diagnostic_config(Some(r#"{"output_format":"markdown","disable_ocr":true}"#));
        let extraction = resolve_extraction_config(&config).unwrap();

        assert_eq!(extraction.output_format, xberg::OutputFormat::Markdown);
        assert!(extraction.disable_ocr);
        assert!(!extraction.use_cache);
    }

    #[test]
    fn default_diagnostic_preserves_absent_ocr_config() {
        let extraction = resolve_extraction_config(&diagnostic_config(None)).unwrap();

        assert!(extraction.ocr.is_none());
    }

    #[test]
    fn diagnostic_rejects_implicit_image_ocr_cache() {
        let mut config = diagnostic_config(None);
        config.inputs = vec![fixture_path("images/ocr_image.jpg")];

        assert!(matches!(
            resolve_extraction_config(&config),
            Err(Error::Config(message)) if message.contains("explicit `ocr`")
        ));
    }

    #[test]
    fn diagnostic_rejects_implicit_scanned_page_ocr_cache() {
        let config = diagnostic_config(Some(
            r#"{"ocr_strategy":{"mode":"scanned_pages","min_confidence":0.5}}"#,
        ));

        assert!(matches!(
            resolve_extraction_config(&config),
            Err(Error::Config(message)) if message.contains("scanned-page OCR")
        ));
    }

    #[test]
    fn diagnostic_rejects_implicit_inline_image_ocr_cache() {
        let config = diagnostic_config(Some(r#"{"pdf_options":{"ocr_inline_images":true}}"#));

        assert!(matches!(
            resolve_extraction_config(&config),
            Err(Error::Config(message)) if message.contains("inline-image OCR")
        ));
    }

    #[test]
    fn config_json_enables_layout_markdown() {
        let config = diagnostic_config(Some(
            r#"{"output_format":"markdown","disable_ocr":true,"layout":{},"use_layout_for_markdown":true}"#,
        ));
        let extraction = resolve_extraction_config(&config).unwrap();

        assert!(extraction.layout.is_some());
        assert!(extraction.use_layout_for_markdown);
    }

    #[test]
    fn config_json_enables_forced_tesseract_ocr() {
        let config = diagnostic_config(Some(
            r#"{"output_format":"markdown","ocr":{"enabled":true,"backend":"tesseract","language":["eng"]},"force_ocr":true}"#,
        ));
        let extraction = resolve_extraction_config(&config).unwrap();

        assert!(extraction.force_ocr);
        let ocr = extraction.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert!(!ocr.tesseract_config.unwrap().use_cache);
    }

    #[test]
    fn diagnostic_overrides_explicit_tesseract_cache() {
        let config = diagnostic_config(Some(
            r#"{"ocr":{"backend":"tesseract","tesseract_config":{"use_cache":true}},"force_ocr":true}"#,
        ));
        let extraction = resolve_extraction_config(&config).unwrap();

        assert!(!extraction.ocr.unwrap().tesseract_config.unwrap().use_cache);
    }

    #[test]
    fn diagnostic_disables_tesseract_pipeline_stage_caches() {
        let config = diagnostic_config(Some(
            r#"{"ocr":{"pipeline":{"stages":[{"backend":"tesseract","priority":100},{"backend":"tesseract","priority":50,"tesseract_config":{"use_cache":true}}]}}}"#,
        ));
        let extraction = resolve_extraction_config(&config).unwrap();
        let stages = extraction.ocr.unwrap().pipeline.unwrap().stages;

        assert!(
            stages
                .iter()
                .all(|stage| !stage.tesseract_config.as_ref().unwrap().use_cache)
        );
    }

    #[test]
    fn diagnostic_settings_override_json_cache_and_concurrency() {
        let mut config = diagnostic_config(Some(
            r#"{"use_cache":true,"max_concurrent_extractions":9,"concurrency":{"max_threads":7}}"#,
        ));
        config.max_threads = Some(2);
        config.max_concurrent_extractions = Some(3);
        let extraction = resolve_extraction_config(&config).unwrap();

        assert!(!extraction.use_cache);
        assert_eq!(extraction.max_concurrent_extractions, Some(3));
        assert_eq!(extraction.concurrency.unwrap().max_threads, Some(2));
    }

    #[test]
    fn config_json_rejects_non_objects_and_unknown_fields() {
        for raw in [
            "[]",
            r#"{"unknown_diagnostic_field":true}"#,
            r#"{"concurrency":{"max_threds":4}}"#,
            r#"{"ocr":{"backend":"tesseract","langauge":["eng"]}}"#,
        ] {
            let error = resolve_extraction_config(&diagnostic_config(Some(raw))).unwrap_err();
            assert!(matches!(
                error,
                Error::Config(message) if message.contains("--config-json")
            ));
        }
    }

    #[test]
    fn config_json_rejects_zero_concurrency_limits() {
        for raw in [
            r#"{"max_concurrent_extractions":0}"#,
            r#"{"concurrency":{"max_threads":0}}"#,
        ] {
            assert!(matches!(
                resolve_extraction_config(&diagnostic_config(Some(raw))),
                Err(Error::Config(message)) if message.contains("greater than zero")
            ));
        }
    }

    #[tokio::test]
    async fn diagnostic_uses_equivalent_public_extraction_paths() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.txt");
        std::fs::write(&input, "fast deterministic batch diagnostic").unwrap();
        let report = run_batch_diagnostic(&BatchDiagnosticConfig {
            inputs: vec![input],
            batch_size: 2,
            warmup_iterations: 1,
            iterations: 1,
            extraction_config_json: None,
            max_threads: Some(2),
            max_concurrent_extractions: Some(2),
        })
        .await
        .unwrap();

        assert_eq!(report.batch_size, 2);
        assert!(report.outputs_match);
        assert!(report.sequential_median_ms > 0.0);
        assert!(report.batch_median_ms > 0.0);
    }
}
