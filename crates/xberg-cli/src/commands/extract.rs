//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use xberg::{
    ExtractInput, ExtractInputKind, ExtractedDocument, ExtractedImage, ExtractionConfig, ExtractionErrorItem,
    ExtractionResult, FileExtractionConfig, extract, extract_batch,
};

use crate::{
    WireFormat,
    output::{BatchEnvelope, ExtractEnvelope, StageTimings},
    style,
};

/// Environment variable that enables per-stage cold-start timing in `xberg extract --format json`.
///
/// Set to `1` (or any non-empty value) to include a `stage_timings` object in the JSON output
/// envelope. Disabled by default so the timing path costs nothing (no extra `Instant::now()`
/// calls, no allocation) when not requested.
pub const STAGE_TIMING_ENV_VAR: &str = "XBERG_EMIT_STAGE_TIMING";

const DEFAULT_MAX_BATCH_CONCURRENCY: usize = 8;

/// Returns `true` when [`STAGE_TIMING_ENV_VAR`] is set to a non-empty value.
///
/// Checked once per invocation; callers should cache the result rather than re-reading the
/// environment on every stage boundary.
pub fn stage_timing_requested() -> bool {
    std::env::var(STAGE_TIMING_ENV_VAR).is_ok_and(|v| !v.is_empty())
}

/// Builds the [`StageTimings`] breakdown for a completed extraction.
///
/// `process_start` is the [`Instant`] captured in `main()` (or `None` if unavailable);
/// `extraction_start` is the [`Instant`] captured immediately before the extraction call;
/// `extraction_time_ms` is the already-computed wall-clock duration of that call.
///
/// `ort_session_and_inference_ms` is populated (as a coarse approximation — see the field's doc
/// comment on [`StageTimings`]) whenever the extraction config has layout or OCR enabled, since
/// both may invoke ONNX Runtime.
fn build_stage_timings(
    process_start: Option<Instant>,
    extraction_start: Instant,
    extraction_time_ms: f64,
    config: &ExtractionConfig,
) -> StageTimings {
    let process_init_ms = process_start.map(|start| extraction_start.duration_since(start).as_secs_f64() * 1000.0);
    #[cfg(feature = "layout-detection")]
    let layout_active = config.layout.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let layout_active = false;
    let ort_active = layout_active || config.ocr.is_some();
    StageTimings {
        process_init_ms: process_init_ms.unwrap_or(0.0),
        first_parse_ms: extraction_time_ms,
        ort_session_and_inference_ms: ort_active.then_some(extraction_time_ms),
    }
}

/// Input source for single-document extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractInputSource {
    /// Local path or URI string.
    Uri(String),
    /// Bytes read from stdin.
    Stdin,
}

/// Batch input manifest format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum BatchInputFormat {
    /// JSON array, or object with an `inputs` array.
    Json,
    /// One JSON string/object per line.
    Jsonl,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchManifest {
    Inputs { inputs: Vec<BatchManifestItem> },
    Array(Vec<BatchManifestItem>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchManifestItem {
    Uri(String),
    Object {
        uri: Option<String>,
        url: Option<String>,
        path: Option<String>,
    },
}

/// Write extracted images to `output_dir`, using the same `image_{index}.{format}` naming
/// convention the markdown renderer uses for its `![](image_N.ext)` references.
///
/// Images with empty data (placeholder `.bin` entries) are skipped — they have no bytes to write.
fn write_extracted_images(images: &[ExtractedImage], output_dir: &Path) -> Result<()> {
    for img in images {
        if img.data.is_empty() {
            continue;
        }
        let filename = format!("image_{}.{}", img.image_index, img.format);
        let dest = output_dir.join(&filename);
        std::fs::write(&dest, &img.data).with_context(|| format!("Failed to write image file '{}'", dest.display()))?;
    }
    Ok(())
}

/// Execute single document extraction command.
///
/// `process_start` is the [`Instant`] captured as early as feasible in `main()`. It is used only
/// to compute `process_init_ms` for the optional stage-timing breakdown (see
/// [`stage_timing_requested`]); pass `None` to skip that measurement entirely (e.g. from tests
/// that construct this call directly).
pub fn extract_command(
    input: ExtractInputSource,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
    output_dir: Option<PathBuf>,
    process_start: Option<Instant>,
) -> Result<()> {
    let emit_stage_timing = stage_timing_requested();

    let t0 = Instant::now();
    let result = extract_input_sync(input, mime_type.as_deref(), &config)?;
    let elapsed = t0.elapsed();
    let extraction_time_ms = elapsed.as_secs_f64() * 1000.0;

    let stage_timings = emit_stage_timing.then(|| build_stage_timings(process_start, t0, extraction_time_ms, &config));

    match format {
        WireFormat::Text => {
            if let Some(images) = &result.images {
                let dir = output_dir.as_deref().unwrap_or(Path::new("."));
                write_extracted_images(images, dir)?;
            }
            print!("{}", result.content);
        }
        WireFormat::Json => {
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
                stage_timings,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).context("Failed to serialize extraction result to JSON")?
            );
        }
        WireFormat::Toon => {
            if let Some(images) = &result.images {
                let dir = output_dir.as_deref().unwrap_or(Path::new("."));
                write_extracted_images(images, dir)?;
            }
            println!(
                "{}",
                serde_toon::to_string(&result).context("Failed to serialize extraction result to TOON")?
            );
        }
    }

    Ok(())
}

/// Execute batch extraction command with optional per-file configuration overrides
pub fn batch_command(
    uris: Vec<String>,
    file_configs_map: Option<std::collections::HashMap<String, serde_json::Value>>,
    config: ExtractionConfig,
    format: WireFormat,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    match format {
        WireFormat::Json => {
            let mut results: Vec<ExtractedDocument> = Vec::with_capacity(uris.len());
            let mut errors: Vec<ExtractionErrorItem> = Vec::new();
            let mut per_file_ms: Vec<f64> = Vec::with_capacity(uris.len());
            let total_t0 = Instant::now();

            let inputs = build_batch_inputs(&uris, file_configs_map.as_ref())?;
            let outputs = run_json_batch_sync(inputs, &config)?;
            for (index, (elapsed_ms, output)) in outputs.into_iter().enumerate() {
                let output = output.with_context(|| {
                    format!(
                        "Failed to extract '{}'. Ensure the resource is readable and supported.",
                        uris[index]
                    )
                })?;
                per_file_ms.push(elapsed_ms);
                results.extend(output.results);
                errors.extend(output.errors);
            }

            fail_if_errors(&errors)?;
            let total_ms = total_t0.elapsed().as_secs_f64() * 1000.0;
            let envelope = BatchEnvelope {
                results,
                total_ms,
                per_file_ms,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope)
                    .context("Failed to serialize batch extraction results to JSON")?
            );
        }
        WireFormat::Text => {
            let results = run_batch_sync(&uris, file_configs_map.as_ref(), &config)?;
            let dir = output_dir.as_deref().unwrap_or(Path::new("."));
            for (i, result) in results.iter().enumerate() {
                if let Some(images) = &result.images {
                    write_extracted_images(images, dir)?;
                }
                println!("{}", style::header(&format!("=== Document {} ===", i + 1)));
                println!("{} {}", style::label("MIME Type:"), style::success(&result.mime_type));
                println!("{}\n{}", style::label("Content:"), result.content);
                println!();
            }
        }
        WireFormat::Toon => {
            let results = run_batch_sync(&uris, file_configs_map.as_ref(), &config)?;
            let dir = output_dir.as_deref().unwrap_or(Path::new("."));
            for result in &results {
                if let Some(images) = &result.images {
                    write_extracted_images(images, dir)?;
                }
            }
            println!(
                "{}",
                serde_toon::to_string(&results).context("Failed to serialize batch extraction results to TOON")?
            );
        }
    }

    Ok(())
}

fn extract_input_sync(
    input: ExtractInputSource,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let output = match input {
        ExtractInputSource::Uri(uri) => {
            let mut input = ExtractInput::from_uri(uri);
            input.mime_type = mime_type.map(str::to_string);
            block_on_extract(input, config)
                .context("Failed to extract URI input. Ensure the resource is readable and the format is supported.")?
        }
        ExtractInputSource::Stdin => {
            let mime_type = mime_type.unwrap_or("text/plain");
            let mut data = Vec::new();
            std::io::stdin()
                .read_to_end(&mut data)
                .context("Failed to read extraction input from stdin")?;
            if data.is_empty() {
                anyhow::bail!("No input received from stdin.");
            }
            block_on_extract(ExtractInput::from_bytes(data, mime_type, None), config).with_context(|| {
                format!("Failed to extract stdin input as MIME type '{mime_type}'. Ensure --mime-type is correct.")
            })?
        }
    };
    single_result_from_output(output)
}

pub fn uri_to_local_path(uri: &str) -> Result<PathBuf> {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        anyhow::bail!("Cannot convert HTTP(S) URL '{uri}' to a local filesystem path.");
    }

    Ok(PathBuf::from(uri.strip_prefix("file://").unwrap_or(uri)))
}

pub fn load_batch_input_manifest(path: &Path, format: BatchInputFormat) -> Result<Vec<String>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read batch input manifest '{}'", path.display()))?;
    match format {
        BatchInputFormat::Json => parse_batch_json_manifest(&raw),
        BatchInputFormat::Jsonl => parse_batch_jsonl_manifest(&raw),
    }
}

fn parse_batch_json_manifest(raw: &str) -> Result<Vec<String>> {
    let manifest: BatchManifest = serde_json::from_str(raw).context("Failed to parse batch input manifest as JSON")?;
    let items = match manifest {
        BatchManifest::Inputs { inputs } | BatchManifest::Array(inputs) => inputs,
    };
    manifest_items_to_uris(items)
}

fn parse_batch_jsonl_manifest(raw: &str) -> Result<Vec<String>> {
    let mut items = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item: BatchManifestItem = serde_json::from_str(trimmed)
            .with_context(|| format!("Failed to parse JSONL batch input on line {}", index + 1))?;
        items.push(item);
    }
    manifest_items_to_uris(items)
}

fn manifest_items_to_uris(items: Vec<BatchManifestItem>) -> Result<Vec<String>> {
    items
        .into_iter()
        .map(|item| match item {
            BatchManifestItem::Uri(uri) => Ok(uri),
            BatchManifestItem::Object { uri, url, path } => uri
                .or(url)
                .or(path)
                .ok_or_else(|| anyhow::anyhow!("Batch input object must include one of uri, url, or path")),
        })
        .collect()
}

/// Run batch extraction using the synchronous batch API for non-JSON output paths.
fn run_batch_sync(
    uris: &[String],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractedDocument>> {
    let inputs = build_batch_inputs(uris, file_configs_map)?;
    let output = block_on_extract_batch(inputs, config).context(
        "Failed to batch extract documents. Check that all resources are readable and formats are supported.",
    )?;
    fail_if_errors(&output.errors)?;
    Ok(output.results)
}

fn batch_concurrency_limit(config: &ExtractionConfig) -> usize {
    config
        .max_concurrent_extractions
        .or_else(|| {
            config
                .concurrency
                .as_ref()
                .and_then(|concurrency| concurrency.max_threads)
        })
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
                .min(DEFAULT_MAX_BATCH_CONCURRENCY)
        })
        .max(1)
}

async fn collect_ordered_bounded<T, O, F, Fut>(items: Vec<T>, limit: usize, operation: F) -> Result<Vec<O>>
where
    T: Send + 'static,
    O: Send + 'static,
    F: Fn(usize, T) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = O> + Send + 'static,
{
    let item_count = items.len();
    let mut pending = items.into_iter().enumerate();
    let mut tasks = tokio::task::JoinSet::new();

    for (index, item) in pending.by_ref().take(limit.max(1)) {
        let operation = operation.clone();
        tasks.spawn(async move { (index, operation(index, item).await) });
    }

    let mut ordered = Vec::with_capacity(item_count);
    ordered.resize_with(item_count, || None);
    while let Some(completed) = tasks.join_next().await {
        let (index, output) = completed.context("A concurrent batch extraction task failed to join")?;
        ordered[index] = Some(output);

        if let Some((next_index, next_item)) = pending.next() {
            let operation = operation.clone();
            tasks.spawn(async move { (next_index, operation(next_index, next_item).await) });
        }
    }

    ordered
        .into_iter()
        .enumerate()
        .map(|(index, output)| output.with_context(|| format!("Batch extraction omitted input {index}")))
        .collect()
}

fn run_json_batch_sync(
    inputs: Vec<ExtractInput>,
    config: &ExtractionConfig,
) -> Result<Vec<(f64, xberg::Result<ExtractionResult>)>> {
    let runtime = tokio::runtime::Runtime::new().context("Failed to create the batch extraction runtime")?;
    let max_concurrent = batch_concurrency_limit(config);
    let config = Arc::new(config.clone());

    runtime.block_on(collect_ordered_bounded(inputs, max_concurrent, move |_index, input| {
        let config = Arc::clone(&config);
        async move {
            let started = Instant::now();
            let output = extract(input, &config).await;
            (started.elapsed().as_secs_f64() * 1000.0, output)
        }
    }))
}

fn block_on_extract(input: ExtractInput, config: &ExtractionConfig) -> xberg::Result<ExtractionResult> {
    tokio::runtime::Runtime::new()?.block_on(extract(input, config))
}

fn block_on_extract_batch(inputs: Vec<ExtractInput>, config: &ExtractionConfig) -> xberg::Result<ExtractionResult> {
    tokio::runtime::Runtime::new()?.block_on(extract_batch(inputs, config))
}

fn build_batch_inputs(
    uris: &[String],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Vec<ExtractInput>> {
    uris.iter()
        .map(|uri| build_extract_input(uri, file_configs_map))
        .collect()
}

fn build_extract_input(
    uri: &str,
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<ExtractInput> {
    let file_config = file_configs_map
        .and_then(|m| m.get(uri))
        .map(|v| {
            serde_json::from_value::<FileExtractionConfig>(v.clone())
                .with_context(|| format!("Failed to parse file config for '{}'", uri))
        })
        .transpose()?;

    Ok(ExtractInput {
        kind: ExtractInputKind::Uri,
        uri: Some(uri.to_string()),
        config: file_config,
        ..Default::default()
    })
}

fn single_result_from_output(mut output: ExtractionResult) -> Result<ExtractedDocument> {
    fail_if_errors(&output.errors)?;
    if output.results.len() != 1 {
        anyhow::bail!("Expected one extraction result, got {}.", output.results.len());
    }
    Ok(output.results.remove(0))
}

fn fail_if_errors(errors: &[ExtractionErrorItem]) -> Result<()> {
    if let Some(error) = errors.first() {
        anyhow::bail!(
            "Extraction failed for input {} ({}): {}",
            error.index,
            error.source,
            error.message
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::borrow::Cow;
    use tempfile::tempdir;
    use xberg::ExtractedImage;

    /// Lock around `STAGE_TIMING_ENV_VAR` to keep these tests deterministic in the
    /// multi-threaded test runner, following the same pattern as
    /// `commands::overrides::tests::with_env_var`.
    static STAGE_TIMING_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[allow(unsafe_code)]
    fn with_stage_timing_env<R>(value: Option<&str>, f: impl FnOnce() -> R) -> R {
        let _guard = STAGE_TIMING_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var(STAGE_TIMING_ENV_VAR).ok();
        unsafe {
            match value {
                Some(v) => std::env::set_var(STAGE_TIMING_ENV_VAR, v),
                None => std::env::remove_var(STAGE_TIMING_ENV_VAR),
            }
        }
        let result = f();
        unsafe {
            match previous {
                Some(v) => std::env::set_var(STAGE_TIMING_ENV_VAR, v),
                None => std::env::remove_var(STAGE_TIMING_ENV_VAR),
            }
        }
        result
    }

    #[test]
    fn stage_timing_requested_is_false_when_env_var_unset() {
        with_stage_timing_env(None, || {
            assert!(!stage_timing_requested());
        });
    }

    #[test]
    fn stage_timing_requested_is_false_when_env_var_empty() {
        with_stage_timing_env(Some(""), || {
            assert!(!stage_timing_requested());
        });
    }

    #[test]
    fn stage_timing_requested_is_true_when_env_var_set() {
        with_stage_timing_env(Some("1"), || {
            assert!(stage_timing_requested());
        });
    }

    #[test]
    fn build_stage_timings_reports_process_init_and_first_parse() {
        let process_start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let extraction_start = Instant::now();
        let config = ExtractionConfig::default();

        let timings = build_stage_timings(Some(process_start), extraction_start, 42.0, &config);

        assert!(
            timings.process_init_ms >= 5.0,
            "expected process_init_ms >= 5.0 (slept 5ms before extraction_start), got {}",
            timings.process_init_ms
        );
        assert_eq!(timings.first_parse_ms, 42.0);
        assert_eq!(
            timings.ort_session_and_inference_ms, None,
            "default ExtractionConfig has no layout/ocr, so ORT sub-stage should be absent"
        );
    }

    #[test]
    fn build_stage_timings_reports_zero_process_init_when_process_start_missing() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig::default();

        let timings = build_stage_timings(None, extraction_start, 10.0, &config);

        assert_eq!(timings.process_init_ms, 0.0);
        assert_eq!(timings.first_parse_ms, 10.0);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn build_stage_timings_populates_ort_field_when_layout_active() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig {
            layout: Some(xberg::LayoutDetectionConfig::default()),
            ..ExtractionConfig::default()
        };

        let timings = build_stage_timings(None, extraction_start, 1171.0, &config);

        assert_eq!(timings.ort_session_and_inference_ms, Some(1171.0));
    }

    #[test]
    fn build_stage_timings_populates_ort_field_when_ocr_active() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig {
            ocr: Some(xberg::OcrConfig::default()),
            ..ExtractionConfig::default()
        };

        let timings = build_stage_timings(None, extraction_start, 500.0, &config);

        assert_eq!(timings.ort_session_and_inference_ms, Some(500.0));
    }

    fn make_image(index: u32, format: &'static str, data: &[u8]) -> ExtractedImage {
        ExtractedImage {
            data: Bytes::copy_from_slice(data),
            format: Cow::Borrowed(format),
            image_index: index,
            ..Default::default()
        }
    }

    #[test]
    fn write_extracted_images_creates_files_with_correct_names() {
        let dir = tempdir().unwrap();
        let images = vec![
            make_image(0, "png", b"\x89PNG\r\n"),
            make_image(1, "jpeg", b"\xff\xd8\xff"),
        ];

        write_extracted_images(&images, dir.path()).unwrap();

        assert!(dir.path().join("image_0.png").exists());
        assert!(dir.path().join("image_1.jpeg").exists());
        assert_eq!(std::fs::read(dir.path().join("image_0.png")).unwrap(), b"\x89PNG\r\n");
    }

    #[test]
    fn write_extracted_images_skips_empty_data() {
        let dir = tempdir().unwrap();
        let images = vec![make_image(0, "bin", b"")];

        write_extracted_images(&images, dir.path()).unwrap();

        assert!(
            !dir.path().join("image_0.bin").exists(),
            "empty-data image must not be written"
        );
    }

    #[test]
    fn write_extracted_images_uses_image_index_not_position() {
        let dir = tempdir().unwrap();
        let images = vec![make_image(3, "png", b"abc"), make_image(7, "png", b"def")];

        write_extracted_images(&images, dir.path()).unwrap();

        assert!(dir.path().join("image_3.png").exists());
        assert!(dir.path().join("image_7.png").exists());
        assert!(!dir.path().join("image_0.png").exists());
        assert!(!dir.path().join("image_1.png").exists());
    }

    #[test]
    fn parse_batch_json_manifest_accepts_inputs_object() {
        let uris = parse_batch_json_manifest(r#"{"inputs":["a.txt",{"path":"b.txt"}]}"#).unwrap();
        assert_eq!(uris, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn parse_batch_jsonl_manifest_accepts_string_and_object_lines() {
        let uris = parse_batch_jsonl_manifest("\"a.txt\"\n{\"uri\":\"b.txt\"}\n").unwrap();
        assert_eq!(uris, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn uri_to_local_path_strips_file_scheme() {
        assert_eq!(
            uri_to_local_path("file:///tmp/doc.txt").unwrap(),
            PathBuf::from("/tmp/doc.txt")
        );
    }

    #[test]
    fn batch_concurrency_limit_prefers_explicit_batch_limit() {
        let mut config = ExtractionConfig {
            max_concurrent_extractions: Some(3),
            ..ExtractionConfig::default()
        };
        config.concurrency.get_or_insert_with(Default::default).max_threads = Some(1);

        assert_eq!(batch_concurrency_limit(&config), 3);
    }

    #[test]
    fn batch_concurrency_limit_falls_back_to_global_thread_limit() {
        let mut config = ExtractionConfig::default();
        config.concurrency.get_or_insert_with(Default::default).max_threads = Some(2);

        assert_eq!(batch_concurrency_limit(&config), 2);
    }

    #[tokio::test]
    async fn bounded_collector_limits_parallelism_and_preserves_input_order() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let outputs = collect_ordered_bounded(vec![40_u64, 5, 30, 1], 2, {
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            move |_index, delay_ms| {
                let active = Arc::clone(&active);
                let peak = Arc::clone(&peak);
                async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    delay_ms
                }
            }
        })
        .await
        .unwrap();

        assert_eq!(outputs, vec![40, 5, 30, 1]);
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn json_batch_extracts_in_input_order_with_one_timing_per_input() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        std::fs::write(&first, "first document").unwrap();
        std::fs::write(&second, "second document").unwrap();
        let uris = vec![first.display().to_string(), second.display().to_string()];
        let inputs = build_batch_inputs(&uris, None).unwrap();
        let config = ExtractionConfig {
            max_concurrent_extractions: Some(2),
            ..ExtractionConfig::default()
        };

        let outputs = run_json_batch_sync(inputs, &config).unwrap();

        assert_eq!(outputs.len(), uris.len());
        assert!(outputs.iter().all(|(elapsed_ms, _)| *elapsed_ms >= 0.0));
        let contents: Vec<String> = outputs
            .into_iter()
            .map(|(_, output)| single_result_from_output(output.unwrap()).unwrap().content)
            .collect();
        assert_eq!(contents, vec!["first document", "second document"]);
    }
}
