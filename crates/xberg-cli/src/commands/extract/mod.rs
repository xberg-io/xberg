//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.
//!
//! The implementation is split by responsibility across submodules:
//! - `timing` - optional per-stage cold-start timing for `--format json`
//! - `images` - writing extracted images to disk
//! - `manifest` - batch input manifest parsing and per-file config resolution
//! - `runtime` - tokio runtime sizing and construction for extraction work
//! - `batch` - the batch extraction command and its result aggregation

use anyhow::{Context, Result};
use base64::Engine as _;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionErrorItem, ExtractionResult, OutputFormat};

use crate::{
    WireFormat,
    output::{ExtractEnvelope, write_text_envelope},
};

mod batch;
mod images;
mod manifest;
mod runtime;
mod timing;

pub use batch::batch_command;
pub use manifest::{BatchInputFormat, load_batch_input_manifest};
// `RUNTIME_WORKER_STACK_SIZE_BYTES`'s only external reader is `server.rs`, which is
// `#[cfg(any(feature = "api", feature = "mcp"))]`; under a feature set with neither, this
// re-export is legitimately unused even though the underlying const in `runtime.rs` is not
// (it is also read locally there). Keeping the re-export preserves the stable
// `commands::extract::RUNTIME_WORKER_STACK_SIZE_BYTES` path for builds that do enable api/mcp. ~keep
#[allow(unused_imports)]
pub(crate) use runtime::RUNTIME_WORKER_STACK_SIZE_BYTES;
// `STAGE_TIMING_ENV_VAR` is part of this module's public path for external documentation/tooling
// (see `crate::output`'s doc comment) but has no in-crate reader outside `timing.rs` itself;
// `stage_timing_requested` (re-exported alongside it) is always read from within this module, so
// only the constant needs the allow. ~keep
#[allow(unused_imports)]
pub use timing::STAGE_TIMING_ENV_VAR;
pub use timing::stage_timing_requested;

use images::write_extracted_images;
use runtime::block_on_extract;
use timing::build_stage_timings;

/// The library's DOCX renderer name, and the `metadata.output_format` of a result whose
/// `content` is a base64-encoded `.docx` package.
pub(crate) const DOCX_CONTENT_FORMAT: &str = "docx";

/// Whether `config` asks for DOCX, which produces one binary document rather than text.
pub(crate) fn requests_docx(config: &ExtractionConfig) -> bool {
    matches!(&config.output_format, OutputFormat::Custom(name) if name == DOCX_CONTENT_FORMAT)
}

/// Input source for single-document extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractInputSource {
    /// Local path or URI string.
    Uri(String),
    /// Bytes read from stdin.
    Stdin,
}

/// Execute single document extraction command.
///
/// `process_start` is the [`Instant`] captured as early as feasible in `main()`. It is used only
/// to compute `process_init_ms` for the optional stage-timing breakdown (see
/// [`stage_timing_requested`]); pass `None` to skip that measurement entirely (e.g. from tests
/// that construct this call directly).
#[expect(
    clippy::print_stdout,
    reason = "extracted content and JSON/TOON envelope are the command's stdout result output"
)]
pub fn extract_command(
    input: ExtractInputSource,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
    output_dir: Option<PathBuf>,
    process_start: Option<Instant>,
) -> Result<()> {
    let emit_stage_timing = stage_timing_requested();

    refuse_docx_to_terminal(&config, &format)?;

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
            let written = if requests_docx(&config) {
                write_docx_package(&result)
            } else {
                print!("{}", result.content);
                Ok(())
            };
            // `stdout` stays exactly the extracted content so it remains pipeable; everything
            // else the extraction produced — warnings included — goes to `stderr`.
            let mut diagnostics = std::io::stderr().lock();
            write_text_envelope(&result, extraction_time_ms, &mut diagnostics)
                .context("Failed to write the extraction envelope summary")?;
            written?;
        }
        WireFormat::Json => {
            // `getrusage` reports the peak for the process's whole lifetime, so the exact sample
            // point doesn't matter as long as it's after the work being measured.
            let peak_memory_bytes = crate::peak_memory::peak_memory_bytes().unwrap_or(0);
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
                peak_memory_bytes,
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
            // Serialize the same envelope the JSON path emits. Previously this serialized the
            // bare `ExtractedDocument`, so TOON consumers lost the timing/peak-memory fields
            // that JSON consumers get.
            let peak_memory_bytes = crate::peak_memory::peak_memory_bytes().unwrap_or(0);
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
                peak_memory_bytes,
                stage_timings,
            };
            println!(
                "{}",
                serde_toon::to_string(&envelope).context("Failed to serialize extraction result to TOON")?
            );
        }
    }

    Ok(())
}

/// Refuse DOCX output to a terminal before extracting anything, since it is binary.
fn refuse_docx_to_terminal(config: &ExtractionConfig, format: &WireFormat) -> Result<()> {
    if requests_docx(config) && matches!(format, WireFormat::Text) && std::io::stdout().is_terminal() {
        anyhow::bail!(
            "--content-format docx writes a binary document to stdout; redirect it to a file (for example \
             `> output.docx`), or use --format json to receive it base64-encoded in `content`"
        );
    }
    Ok(())
}

/// Write the `.docx` package a DOCX extraction carries base64-encoded in `content`.
///
/// Fails rather than writing text when the library fell back to plain text (for example
/// when it was built without the `office` feature), since the caller is redirecting
/// stdout into a `.docx` file.
fn write_docx_package(result: &ExtractedDocument) -> Result<()> {
    if result.metadata.output_format.as_deref() != Some(DOCX_CONTENT_FORMAT) {
        anyhow::bail!(
            "DOCX output was requested but the extraction produced {} text instead; see the warnings above",
            result.metadata.output_format.as_deref().unwrap_or("plain")
        );
    }
    let package = base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .context("DOCX output was not valid base64")?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&package)
        .context("Failed to write the DOCX document to stdout")?;
    stdout.flush().context("Failed to write the DOCX document to stdout")
}

fn extract_input_sync(
    input: ExtractInputSource,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let output = match input {
        ExtractInputSource::Uri(uri) => {
            let mut input = ExtractInput::from_uri(uri.clone());
            input.mime_type = mime_type.map(str::to_string);
            // Describe what was attempted, not why it failed: `.context()` preserves the
            // underlying error as this error's source (anyhow's `{:?}` rendering walks it via
            // "Caused by:"), so asserting a specific diagnosis here ("ensure the resource is
            // readable and the format is supported") would misreport failures that have nothing
            // to do with readability or format support — an OCR backend crash, for example.
            block_on_extract(input, config).with_context(|| format!("Failed to extract input '{uri}'"))?
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
            // See the URI branch above: describe the attempted operation only. The real cause
            // (e.g. a genuinely wrong --mime-type) still surfaces via the preserved error chain.
            block_on_extract(ExtractInput::from_bytes(data, mime_type, None), config)
                .with_context(|| format!("Failed to extract stdin input as MIME type '{mime_type}'"))?
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

    #[test]
    fn uri_to_local_path_strips_file_scheme() {
        assert_eq!(
            uri_to_local_path("file:///tmp/doc.txt").unwrap(),
            PathBuf::from("/tmp/doc.txt")
        );
    }

    /// Regression test for a real reported bug: extraction failures were wrapped in a context
    /// message ("Ensure the resource is readable and the format is supported.") that confidently
    /// diagnoses the wrong cause whenever the actual failure has nothing to do with readability
    /// or format support (e.g. an OCR backend crash on a file three other backends had just
    /// extracted successfully in the same session). anyhow's `.context()`/`.with_context()`
    /// preserve the underlying error as this error's `source()`, so the real cause was always
    /// present in the chain -- but the outermost line a user reads asserted a specific, false
    /// diagnosis instead of describing what was attempted. This exercises the real (unmocked)
    /// `extract_input_sync` failure path with a nonexistent file, which `xberg::extract` reports
    /// as `XbergError::Io` or `XbergError::Validation` (see
    /// `crates/xberg/tests/error_handling.rs::test_nonexistent_file`), to confirm the CLI's own
    /// wrapping context no longer asserts that false diagnosis.
    #[test]
    fn uri_extraction_failure_does_not_assert_a_false_readability_diagnosis() {
        let config = ExtractionConfig::default();
        let missing_uri = "test_documents_definitely_missing/does-not-exist-9f3c2a11.txt";

        let err = extract_input_sync(ExtractInputSource::Uri(missing_uri.to_string()), None, &config)
            .expect_err("extracting a nonexistent file must fail");
        let rendered = format!("{err:?}");

        assert!(
            !rendered.contains("Ensure the resource is readable and the format is supported"),
            "context must describe the attempted operation, not assert a (possibly false) \
             diagnosis; got: {rendered}"
        );
        assert!(
            rendered.contains(missing_uri),
            "context must name the input that failed to extract so the user knows what was \
             attempted; got: {rendered}"
        );
    }
}
