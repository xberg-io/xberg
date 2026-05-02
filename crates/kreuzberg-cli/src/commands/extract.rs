//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.

use anyhow::{Context, Result};
use kreuzberg::{BatchFileItem, ExtractionConfig, FileExtractionConfig, batch_extract_files_sync, extract_file_sync};
use std::path::PathBuf;

use crate::{WireFormat, style};

/// Execute single document extraction command
pub fn extract_command(
    path: PathBuf,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    let result = extract_file_sync(&path_str, mime_type.as_deref(), &config).with_context(|| {
        format!(
            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
            path.display()
        )
    })?;

    match format {
        WireFormat::Text => {
            print!("{}", result.content);
        }
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).context("Failed to serialize extraction result to JSON")?
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
    let items: Vec<BatchFileItem> = if let Some(ref configs_map) = file_configs_map {
        paths
            .into_iter()
            .map(|p| {
                let path_str = p.to_string_lossy().to_string();
                let file_config = configs_map
                    .get(&path_str)
                    .map(|v| {
                        serde_json::from_value::<FileExtractionConfig>(v.clone())
                            .with_context(|| format!("Failed to parse file config for '{}'", path_str))
                    })
                    .transpose()?;
                Ok(BatchFileItem {
                    path: p,
                    config: file_config,
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        paths
            .into_iter()
            .map(|p| BatchFileItem { path: p, config: None })
            .collect()
    };

    let results = batch_extract_files_sync(items, &config).with_context(
        || "Failed to batch extract documents. Check that all files are readable and formats are supported.",
    )?;

    match format {
        WireFormat::Text => {
            for (i, result) in results.iter().enumerate() {
                println!("{}", style::header(&format!("=== Document {} ===", i + 1)));
                println!("{} {}", style::label("MIME Type:"), style::success(&result.mime_type));
                println!("{}\n{}", style::label("Content:"), result.content);
                println!();
            }
        }
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&results)
                    .context("Failed to serialize batch extraction results to JSON")?
            );
        }
        WireFormat::Toon => {
            println!(
                "{}",
                serde_toon::to_string(&results).context("Failed to serialize batch extraction results to TOON")?
            );
        }
    }

    Ok(())
}
