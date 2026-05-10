//! JSON envelope types for CLI output.
//!
//! When `--format json` is used, extraction results are wrapped in these envelopes
//! so tooling (such as the benchmark harness) can read timing information without
//! parsing stderr or running a separate profiling tool.

use kreuzberg::ExtractionResult;
use serde::Serialize;

/// Single-file extraction result with wall-clock timing.
///
/// Emitted to stdout by `kreuzberg extract --format json`.
#[derive(Debug, Serialize)]
pub struct ExtractEnvelope {
    /// The extraction result (content, metadata, tables, …).
    pub result: ExtractionResult,
    /// Wall-clock time for the extraction call in milliseconds.
    pub extraction_time_ms: f64,
}

/// Batch extraction results with per-file and total timing.
///
/// Emitted to stdout by `kreuzberg batch --format json`.
#[derive(Debug, Serialize)]
pub struct BatchEnvelope {
    /// One result per input file, in input order.
    pub results: Vec<ExtractionResult>,
    /// Total wall-clock time for the whole batch in milliseconds.
    pub total_ms: f64,
    /// Per-file wall-clock times in milliseconds, aligned with `results`.
    pub per_file_ms: Vec<f64>,
}
