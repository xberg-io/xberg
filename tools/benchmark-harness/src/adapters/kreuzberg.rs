//! Kreuzberg adapter for Wave 2 benchmark harness.
//!
//! Provides subprocess-based extraction via kreuzberg with support for:
//! - Three pipelines: baseline, layout, paddle-ocr
//! - Single-file and batch extraction modes
//! - JSON envelope parsing (ExtractEnvelope and BatchEnvelope)

use crate::{
    adapters::subprocess::SubprocessAdapter,
    error::Result,
    types::{KreuzbergPipeline, OutputFormat},
};
use std::path::PathBuf;
use which::which;

/// Creates a Kreuzberg adapter for the given pipeline and configuration.
///
/// # Arguments
/// * `pipeline` - The pipeline variant (baseline, layout, paddle-ocr)
/// * `output_format` - Output format for extraction (markdown or plaintext)
/// * `batch` - Whether to use batch extraction mode
///
/// # Returns
/// * `Ok(SubprocessAdapter)` - Configured adapter ready for extraction
/// * `Err(Error)` - If kreuzberg cannot be located
pub fn create_kreuzberg_adapter(
    pipeline: KreuzbergPipeline,
    output_format: OutputFormat,
    batch: bool,
) -> Result<SubprocessAdapter> {
    let cli_path = locate_kreuzberg_cli()?;

    // Map output format to CLI flag
    let content_format = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plain",
    };

    // Build command arguments
    let subcommand = if batch { "batch" } else { "extract" };
    let mut args = vec![
        subcommand.to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--content-format".to_string(),
        content_format.to_string(),
    ];

    // Add pipeline-specific flags
    match pipeline {
        KreuzbergPipeline::Baseline => {
            // No additional flags for baseline
        }
        KreuzbergPipeline::Layout => {
            args.push("--layout".to_string());
            args.push("true".to_string());
            args.push("--use-layout-for-markdown".to_string());
            args.push("true".to_string());
        }
        KreuzbergPipeline::PaddleOcr => {
            args.push("--ocr".to_string());
            args.push("true".to_string());
            args.push("--ocr-backend".to_string());
            args.push("paddle-ocr".to_string());
            args.push("--force-ocr".to_string());
            args.push("true".to_string());
        }
    }

    // Forward-compat marker: always specify pdf-backend
    args.push("--pdf-backend".to_string());
    args.push("pdf-oxide".to_string());

    let format_slug = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plaintext",
    };
    let framework_name = if batch {
        format!("kreuzberg-{}-{}-batch", format_slug, pipeline.as_str())
    } else {
        format!("kreuzberg-{}-{}", format_slug, pipeline.as_str())
    };
    let supported_formats = vec![
        "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "txt", "md", "html", "xml", "json", "odt", "ods", "odp",
        "epub", "rtf", "csv", "json", "yaml", "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "zip", "tar",
        "gz", "7z",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let adapter = if batch {
        SubprocessAdapter::with_batch_support(&framework_name, cli_path, args, vec![], supported_formats)
    } else {
        SubprocessAdapter::new(&framework_name, cli_path, args, vec![], supported_formats)
    };

    Ok(adapter)
}

/// Locates the kreuzberg executable.
///
/// Searches in priority order:
/// 1. `target/release/kreuzberg`
/// 2. `target/debug/kreuzberg`
/// 3. `which kreuzberg`
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the executable
/// * `Err(Error)` - If kreuzberg cannot be found
fn locate_kreuzberg_cli() -> Result<PathBuf> {
    // Try release build first
    let release_path = PathBuf::from("target/release/kreuzberg");
    if release_path.exists() {
        return Ok(release_path);
    }

    // Try debug build
    let debug_path = PathBuf::from("target/debug/kreuzberg");
    if debug_path.exists() {
        return Ok(debug_path);
    }

    // Try system PATH
    if let Ok(path) = which("kreuzberg") {
        return Ok(path);
    }

    Err(crate::Error::Benchmark(
        "kreuzberg binary not found. Build with: cargo build --release -p kreuzberg-cli --features all".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_baseline_str() {
        assert_eq!(KreuzbergPipeline::Baseline.as_str(), "baseline");
    }

    #[test]
    fn test_pipeline_layout_str() {
        assert_eq!(KreuzbergPipeline::Layout.as_str(), "layout");
    }

    #[test]
    fn test_pipeline_paddle_ocr_str() {
        assert_eq!(KreuzbergPipeline::PaddleOcr.as_str(), "paddle-ocr");
    }

    #[test]
    fn test_output_format_markdown() {
        assert_eq!(OutputFormat::Markdown.to_string(), "markdown");
    }

    #[test]
    fn test_output_format_plaintext() {
        assert_eq!(OutputFormat::Plaintext.to_string(), "plaintext");
    }
}
