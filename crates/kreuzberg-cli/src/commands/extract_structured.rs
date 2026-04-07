//! Extract structured command - Extract structured data from documents using an LLM.
//!
//! Reads a JSON schema file, configures LLM-based structured extraction, and
//! outputs the structured result parsed from the document.

use anyhow::{Context, Result};
use kreuzberg::{LlmConfig, StructuredExtractionConfig, extract_file_sync};
use std::path::PathBuf;

use crate::WireFormat;

/// Arguments for the extract-structured command.
pub struct ExtractStructuredArgs {
    pub path: PathBuf,
    pub schema_path: PathBuf,
    pub model: String,
    pub api_key: Option<String>,
    pub prompt: Option<String>,
    pub schema_name: Option<String>,
    pub strict: bool,
    pub config_path: Option<PathBuf>,
    pub format: WireFormat,
}

/// Execute the extract-structured command.
///
/// Reads a JSON schema from `schema_path`, builds an `ExtractionConfig` with
/// `structured_extraction` configured, extracts the document, and outputs the
/// `structured_output` field from the result.
pub fn extract_structured_command(args: ExtractStructuredArgs) -> Result<()> {
    let ExtractStructuredArgs {
        path,
        schema_path,
        model,
        api_key,
        prompt,
        schema_name,
        strict,
        config_path,
        format,
    } = args;
    // 1. Read and parse the JSON schema file
    let schema_str = std::fs::read_to_string(&schema_path).with_context(|| {
        format!(
            "Failed to read JSON schema file '{}'. Ensure the file exists and is readable.",
            schema_path.display()
        )
    })?;
    let schema: serde_json::Value = serde_json::from_str(&schema_str).with_context(|| {
        format!(
            "Failed to parse JSON schema from '{}'. Ensure the file contains valid JSON.",
            schema_path.display()
        )
    })?;

    // 2. Build ExtractionConfig with structured_extraction
    let mut config = super::load_config(config_path)?;

    let llm_config = LlmConfig {
        model,
        api_key,
        base_url: None,
        timeout_secs: None,
        max_retries: None,
        temperature: None,
        max_tokens: None,
    };

    config.structured_extraction = Some(StructuredExtractionConfig {
        schema,
        schema_name: schema_name.unwrap_or_else(|| "extraction".to_string()),
        schema_description: None,
        strict,
        prompt,
        llm: llm_config,
    });

    // 3. Call kreuzberg::extract_file_sync()
    let path_str = path.to_string_lossy().to_string();
    let result = extract_file_sync(&path_str, None, &config).with_context(|| {
        format!(
            "Failed to extract structured data from '{}'. Ensure the file is readable and the LLM configuration is correct.",
            path.display()
        )
    })?;

    // 4. Output result.structured_output (or error if None)
    let structured = result.structured_output.with_context(|| {
        "Structured extraction completed but returned no structured output. \
         This may indicate the LLM failed to produce valid structured data matching the schema."
    })?;

    match format {
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&structured).context("Failed to serialize structured output to JSON")?
            );
        }
        WireFormat::Toon => {
            println!(
                "{}",
                serde_toon::to_string(&structured).context("Failed to serialize structured output to TOON")?
            );
        }
        WireFormat::Text => {
            // For text mode, pretty-print the JSON value
            println!(
                "{}",
                serde_json::to_string_pretty(&structured).context("Failed to serialize structured output to text")?
            );
        }
    }

    Ok(())
}
