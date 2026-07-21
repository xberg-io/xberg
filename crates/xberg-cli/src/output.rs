//! JSON envelope types for CLI output.
//!
//! When `--format json` is used, extraction results are wrapped in these envelopes
//! so tooling (such as the benchmark harness) can read timing information without
//! parsing stderr or running a separate profiling tool.

use serde::Serialize;
use xberg::ExtractedDocument;

/// Per-stage cold-start timing breakdown for a single `xberg extract` invocation.
///
/// Only populated when stage timing is requested (see
/// [`crate::commands::extract::stage_timing_requested`]). Every duration is measured with
/// [`std::time::Instant`] (never wall-clock/system time) and reported in milliseconds.
///
/// # Stage coverage
///
/// - `process_init_ms` and `first_parse_ms` are measured directly at the CLI boundary and are
///   always accurate when this struct is present.
/// - `ort_session_and_inference_ms` is a coarse approximation: the core library does not expose
///   a public hook for ONNX Runtime session creation or first-inference timing (the closest
///   internal signal, `xberg::layout::inference_timings`, is `pub(crate)` and not reachable from
///   the CLI). When layout/OCR features that use ORT are active, this field reports the *total*
///   extraction wall time minus `first_parse_ms` as an upper bound that includes ORT session
///   creation, inference, and any other post-parse processing — it is not a clean sub-stage
///   measurement. See the doc comment on the field itself for the precise caveat.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StageTimings {
    /// Time from process start (`main()` entry) to the point the extraction call begins,
    /// covering CLI argument parsing, logging setup, and config loading/merging.
    pub process_init_ms: f64,
    /// Wall-clock time for the core library's extraction call to return.
    ///
    /// Named "first parse" because this is the first (and only, for `extract`) document parse
    /// performed by the process. Includes any OCR/layout/ORT work performed during extraction.
    pub first_parse_ms: f64,
    /// Approximate ONNX Runtime session-creation-plus-first-inference cost, present only when a
    /// layout/OCR configuration that uses ORT is active for this extraction.
    ///
    /// This is **not** independently measured — the core extraction API has no public timing
    /// hook for ORT session creation or inference. It is reported as `first_parse_ms` again
    /// (the coarsest bound available at the CLI boundary): the whole extraction call, most of
    /// which is expected to be ORT session creation and inference on a cold-start layout
    /// extraction. Treat it as "extraction time when ORT-backed features are active", not as an
    /// isolated ORT sub-stage duration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ort_session_and_inference_ms: Option<f64>,
}

/// Single-file extraction result with wall-clock timing.
///
/// Emitted to stdout by `xberg extract --format json`.
#[derive(Debug, Serialize)]
pub struct ExtractEnvelope {
    /// The extracted document (content, metadata, tables, ...).
    pub result: ExtractedDocument,
    /// Wall-clock time for the extraction call in milliseconds.
    pub extraction_time_ms: f64,
    /// Per-stage cold-start timing breakdown, present only when stage timing was requested via
    /// the `XBERG_EMIT_STAGE_TIMING` environment variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_timings: Option<StageTimings>,
}

/// Batch extraction results with per-file and total timing.
///
/// Emitted to stdout by `xberg batch --format json`.
#[derive(Debug, Serialize)]
pub struct BatchEnvelope {
    /// Extraction results in input order. A single input may yield multiple results.
    pub results: Vec<ExtractedDocument>,
    /// Total wall-clock time for the whole batch in milliseconds.
    pub total_ms: f64,
    /// Per-input wall-clock times in milliseconds, aligned with the input list.
    ///
    /// This has one entry per requested input even when an input yields multiple
    /// entries in `results`.
    pub per_file_ms: Vec<f64>,
}
