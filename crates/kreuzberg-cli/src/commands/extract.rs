//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.

use anyhow::{Context, Result};
use kreuzberg::{BatchFileItem, ExtractionConfig, ExtractionResult, FileExtractionConfig, batch_extract_files_sync, extract_file_sync};
use std::path::PathBuf;
use std::time::Instant;

use crate::{WireFormat, output::{BatchEnvelope, ExtractEnvelope}, style};

/// Execute single document extraction command
pub fn extract_command(
    path: PathBuf,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    let t0 = Instant::now();
    let result = extract_file_sync(&path_str, mime_type.as_deref(), &config).with_context(|| {
        format!(
            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
            path.display()
        )
    })?;
    let extraction_time_ms = t0.elapsed().as_secs_f64() * 1000.0;

    match format {
        WireFormat::Text => {
            print!("{}", result.content);
        }
        WireFormat::Json => {
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope)
                    .context("Failed to serialize extraction result to JSON")?
            );
        }
        WireFormat::Toon => {
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
    paths: Vec<PathBuf>,
    file_configs_map: Option<std::collections::HashMap<String, serde_json::Value>>,
    config: ExtractionConfig,
    format: WireFormat,
) -> Result<()> {
    match format {
        WireFormat::Json => {
            // Run files one at a time to capture per-file wall-clock timings.
            // Per-file config overrides are honoured: files without an override use the
            // batch-level config directly; files with an override use a one-shot batch of
            // one item so the library's own merge logic applies.
            let mut results: Vec<ExtractionResult> = Vec::with_capacity(paths.len());
            let mut per_file_ms: Vec<f64> = Vec::with_capacity(paths.len());
            let total_t0 = Instant::now();

            for path in &paths {
                let path_str = path.to_string_lossy().to_string();
                let has_file_config = file_configs_map
                    .as_ref()
                    .and_then(|m| m.get(&path_str))
                    .is_some();

                let t0 = Instant::now();
                let result = if has_file_config {
                    // Delegate to the batch API (one item) so per-file merge logic is applied.
                    let file_config = file_configs_map
                        .as_ref()
                        .and_then(|m| m.get(&path_str))
                        .map(|v| {
                            serde_json::from_value::<FileExtractionConfig>(v.clone())
                                .with_context(|| format!("Failed to parse file config for '{}'", path_str))
                        })
                        .transpose()?;
                    let mut batch_results = batch_extract_files_sync(
                        vec![BatchFileItem { path: path.clone(), config: file_config }],
                        &config,
                    )
                    .with_context(|| {
                        format!(
                            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
                            path.display()
                        )
                    })?;
                    batch_results.remove(0)
                } else {
                    extract_file_sync(&path_str, None, &config).with_context(|| {
                        format!(
                            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
                            path.display()
                        )
                    })?
                };
                per_file_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
                results.push(result);
            }

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
            let results = run_batch_sync(&paths, file_configs_map.as_ref(), &config)?;
            for (i, result) in results.iter().enumerate() {
                println!("{}", style::header(&format!("=== Document {} ===", i + 1)));
                println!("{} {}", style::label("MIME Type:"), style::success(&result.mime_type));
                println!("{}\n{}", style::label("Content:"), result.content);
                println!();
            }
        }
        WireFormat::Toon => {
            let results = run_batch_sync(&paths, file_configs_map.as_ref(), &config)?;
            println!(
                "{}",
                serde_toon::to_string(&results).context("Failed to serialize batch extraction results to TOON")?
            );
        }
    }

    Ok(())
}

/// Run batch extraction using the synchronous batch API for non-JSON output paths.
fn run_batch_sync(
    paths: &[PathBuf],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    let items: Vec<BatchFileItem> = paths
        .iter()
        .map(|p| {
            let path_str = p.to_string_lossy().to_string();
            let file_config = file_configs_map
                .and_then(|m| m.get(&path_str))
                .map(|v| {
                    serde_json::from_value::<FileExtractionConfig>(v.clone())
                        .with_context(|| format!("Failed to parse file config for '{}'", path_str))
                })
                .transpose()?;
            Ok(BatchFileItem {
                path: p.clone(),
                config: file_config,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    batch_extract_files_sync(items, config)
        .context("Failed to batch extract documents. Check that all files are readable and formats are supported.")
}
