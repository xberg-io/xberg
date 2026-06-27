//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::{
    ExtractInput, ExtractInputKind, ExtractedImage, ExtractionConfig, ExtractionErrorItem, ExtractionOutput,
    ExtractionResult, FileExtractionConfig, extract_batch_sync, extract_sync,
};

use crate::{
    WireFormat,
    output::{BatchEnvelope, ExtractEnvelope},
    style,
};

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

/// Execute single document extraction command
pub fn extract_command(
    input: ExtractInputSource,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    let t0 = Instant::now();
    let result = extract_input_sync(input, mime_type.as_deref(), &config)?;
    let extraction_time_ms = t0.elapsed().as_secs_f64() * 1000.0;

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
            // Run files one at a time to capture per-file wall-clock timings.
            // Per-file config overrides are honoured: files without an override use the
            // batch-level config directly; files with an override use a one-shot batch of
            // one item so the library's own merge logic applies.
            let mut results: Vec<ExtractionResult> = Vec::with_capacity(uris.len());
            let mut errors: Vec<ExtractionErrorItem> = Vec::new();
            let mut per_file_ms: Vec<f64> = Vec::with_capacity(uris.len());
            let total_t0 = Instant::now();

            for uri in &uris {
                let t0 = Instant::now();
                let output = extract_uri_output_sync(uri, file_configs_map.as_ref(), &config)?;
                per_file_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
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
) -> Result<ExtractionResult> {
    let output = match input {
        ExtractInputSource::Uri(uri) => {
            let mut input = ExtractInput::uri(uri);
            input.mime_type = mime_type.map(str::to_string);
            extract_sync(input, config)
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
            extract_sync(ExtractInput::bytes(data, mime_type, None), config).with_context(|| {
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
) -> Result<Vec<ExtractionResult>> {
    let inputs = build_batch_inputs(uris, file_configs_map)?;
    let output = extract_batch_sync(inputs, config).context(
        "Failed to batch extract documents. Check that all resources are readable and formats are supported.",
    )?;
    fail_if_errors(&output.errors)?;
    Ok(output.results)
}

fn extract_uri_output_sync(
    uri: &str,
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
    config: &ExtractionConfig,
) -> Result<ExtractionOutput> {
    let input = build_extract_input(uri, file_configs_map)?;
    extract_sync(input, config).with_context(|| {
        format!(
            "Failed to extract '{}'. Ensure the resource is readable and supported.",
            uri
        )
    })
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

fn single_result_from_output(mut output: ExtractionOutput) -> Result<ExtractionResult> {
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
        // If a document has images at index 3 and 7 (gaps due to filtered images),
        // the files must be image_3.* and image_7.* to match markdown references.
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
}
