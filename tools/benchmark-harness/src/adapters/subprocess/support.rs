//! Small shared types and output-parsing helpers used across the subprocess adapter's
//! execution and result-mapping paths.

use crate::monitoring::ResourceStats;
use crate::types::ErrorKind;
use crate::{Error, Result};
use std::time::Duration;

pub(super) struct MeasuredCommandOutcome {
    pub(super) output: Option<std::process::Output>,
    pub(super) duration: Duration,
    pub(super) resource_stats: ResourceStats,
    pub(super) error: Option<Error>,
}

pub(super) struct SubprocessExecution {
    pub(super) stdout: String,
    pub(super) duration: Duration,
    pub(super) resource_stats: ResourceStats,
    pub(super) error: Option<Error>,
}

/// Extract JSON content from raw stdout, stripping non-JSON prefix lines.
///
/// Some runtimes (notably Elixir's BEAM VM) emit log messages to stdout
/// during module initialization before the script can redirect them. This
/// function finds the earliest `[` or `{` character and returns everything
/// from that point, ignoring any preceding log lines. Whichever delimiter
/// appears first wins — must not bias toward `[` because object outputs
/// (e.g. xberg-cli's envelope) contain nested arrays.
pub(super) fn extract_json_from_stdout(raw: &str) -> &str {
    let bracket = raw.find('[');
    let brace = raw.find('{');
    let pos = match (bracket, brace) {
        (Some(b), Some(c)) => Some(b.min(c)),
        (Some(b), None) => Some(b),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };
    match pos {
        Some(p) => &raw[p..],
        None => raw,
    }
}

/// Marker printed by our extraction scripts (e.g. `docling_extract.py`,
/// `markitdown_extract.py`) to stderr when the *framework itself* raises during
/// extraction: `print(f"Error extracting with {Framework}: {e}", file=sys.stderr)`
/// followed by a non-zero exit. This is a framework-side crash, not ours — see
/// [`error_to_error_kind`].
pub(super) const FRAMEWORK_CRASH_STDERR_MARKER: &str = "error extracting with";

/// Substrings whose presence in a lowercased error message indicate a missing dependency,
/// model, or library rather than a harness-side failure. Extracted from `error_to_error_kind`
/// to keep that function's cyclomatic complexity under the crate's limit. ~keep
fn indicates_missing_dependency(msg_lower: &str) -> bool {
    let single_phrase_matches = ["tessdata", "import error", "importerror"];
    if single_phrase_matches.iter().any(|phrase| msg_lower.contains(phrase)) {
        return true;
    }

    let paired_phrase_matches: [[&str; 2]; 3] = [
        ["torch.", "not found"],
        ["partition_", "not available"],
        ["tesseract", "not found"],
    ];
    if paired_phrase_matches
        .iter()
        .any(|[first, second]| msg_lower.contains(first) && msg_lower.contains(second))
    {
        return true;
    }

    let module_missing =
        msg_lower.contains("module") && (msg_lower.contains("not found") || msg_lower.contains("not installed"));
    let missing_native_library = msg_lower.contains("no such file")
        && (msg_lower.contains(".so") || msg_lower.contains(".dylib") || msg_lower.contains(".dll"));
    let missing_model_or_library =
        msg_lower.contains("failed to find") && (msg_lower.contains("model") || msg_lower.contains("library"));

    module_missing || missing_native_library || missing_model_or_library
}

/// Map a harness `Error` to the appropriate `ErrorKind`.
///
/// Detects config/setup errors (missing dependencies, environment issues) vs
/// actual harness infrastructure failures vs framework-side crashes.
///
/// Subprocess non-zero exits are wrapped as `Error::Benchmark` regardless of
/// *why* the subprocess died, so the message text (which embeds captured
/// stderr — see `execute_subprocess`/`execute_subprocess_batch`) is inspected
/// here to distinguish three cases:
/// 1. The framework crashed while extracting (our extraction scripts print
///    `"Error extracting with {Framework}: ..."` to stderr before exiting
///    non-zero) → `FrameworkError`, not our fault.
/// 2. A missing dependency/model/library (config/setup issue) → `ConfigSetupError`.
/// 3. Anything else (spawn failure, our own panics, unexpected subprocess death)
///    → `HarnessError`, potentially our fault.
pub(super) fn error_to_error_kind(e: &Error) -> ErrorKind {
    match e {
        Error::Timeout(_) => ErrorKind::Timeout,
        Error::FrameworkError(_) => ErrorKind::FrameworkError,
        Error::EmptyContent(_) => ErrorKind::EmptyContent,
        Error::LayoutFailed(_) => ErrorKind::LayoutFailed,
        Error::Benchmark(msg) | Error::Config(msg) => {
            let msg_lower = msg.to_lowercase();

            if indicates_missing_dependency(&msg_lower) {
                ErrorKind::ConfigSetupError
            } else if msg_lower.contains(FRAMEWORK_CRASH_STDERR_MARKER) {
                ErrorKind::FrameworkError
            } else {
                ErrorKind::HarnessError
            }
        }
        _ => ErrorKind::HarnessError,
    }
}

pub(super) const MIN_VALID_DURATION_SECS: f64 = 0.000_001;

pub(super) fn bytes_per_second(bytes: u64, duration: Duration) -> f64 {
    if duration.as_secs_f64() >= MIN_VALID_DURATION_SECS {
        bytes as f64 / duration.as_secs_f64()
    } else {
        0.0
    }
}

/// Detect a PDF's page count using the harness-side (framework-agnostic) `xberg` page counter.
///
/// This is intentionally independent of whatever a competing framework self-reports, so the
/// resulting `pages_per_sec` aggregate metric compares every framework against the same
/// ground truth. Returns `None` when the file cannot be read or does not parse as a PDF.
pub(super) fn detect_pdf_page_count(path: &std::path::Path) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;
    xberg::pdf_page_count(&bytes, None).ok().map(|count| count as u32)
}

#[derive(Debug)]
pub(super) struct ParsedBatchOutput {
    pub(super) items: Vec<serde_json::Value>,
    pub(super) errors: Vec<ParsedBatchError>,
    pub(super) reported_total_duration: Option<Duration>,
    pub(super) per_file_durations: Vec<Option<Duration>>,
}

#[derive(Debug)]
pub(super) struct ParsedBatchError {
    pub(super) index: usize,
    pub(super) message: String,
}

fn parse_batch_errors(raw: &serde_json::Value) -> Result<Vec<ParsedBatchError>> {
    let Some(errors) = raw.get("errors") else {
        return Ok(Vec::new());
    };
    let errors = errors
        .as_array()
        .ok_or_else(|| Error::Benchmark("batch envelope field 'errors' must be an array".to_string()))?;
    errors
        .iter()
        .enumerate()
        .map(|(position, error)| {
            let index = error
                .get("index")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| {
                    Error::Benchmark(format!(
                        "batch envelope errors[{position}] is missing a valid input index"
                    ))
                })?;
            let message = error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| {
                    Error::Benchmark(format!(
                        "batch envelope errors[{position}] is missing a non-empty message"
                    ))
                })?;
            Ok(ParsedBatchError {
                index,
                message: message.to_string(),
            })
        })
        .collect()
}

pub(super) fn duration_from_ms(value: &serde_json::Value, field: &str) -> Result<Duration> {
    let milliseconds = value
        .as_f64()
        .filter(|milliseconds| milliseconds.is_finite() && *milliseconds >= 0.0)
        .ok_or_else(|| Error::Benchmark(format!("batch output field '{field}' must be a non-negative number")))?;
    Ok(Duration::from_secs_f64(milliseconds / 1000.0))
}

pub(super) fn parse_batch_output(stdout: &str) -> Result<ParsedBatchOutput> {
    let raw: serde_json::Value = serde_json::from_str(stdout)
        .map_err(|error| Error::Benchmark(format!("Failed to parse batch output as JSON: {error}")))?;

    if let Some(results) = raw.get("results") {
        let items = results
            .as_array()
            .cloned()
            .ok_or_else(|| Error::Benchmark("batch output field 'results' must be an array".to_string()))?;
        let reported_total_duration = raw
            .get("total_ms")
            .ok_or_else(|| Error::Benchmark("batch envelope is missing required 'total_ms'".to_string()))
            .and_then(|value| duration_from_ms(value, "total_ms"))?;
        let per_file_values = raw
            .get("per_file_ms")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| Error::Benchmark("batch envelope is missing required 'per_file_ms' array".to_string()))?;
        let per_file_durations = per_file_values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if value.is_null() {
                    Ok(None)
                } else {
                    duration_from_ms(value, &format!("per_file_ms[{index}]")).map(Some)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let errors = parse_batch_errors(&raw)?;

        return Ok(ParsedBatchOutput {
            items,
            errors,
            reported_total_duration: Some(reported_total_duration),
            per_file_durations,
        });
    }

    let items = match raw {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![raw],
        _ => {
            return Err(Error::Benchmark(
                "batch output must be a JSON array, object, or Xberg batch envelope".to_string(),
            ));
        }
    };
    let per_file_durations = items
        .iter()
        .map(|item| {
            item.get("_extraction_time_ms")
                .map(|value| duration_from_ms(value, "_extraction_time_ms"))
                .transpose()
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ParsedBatchOutput {
        items,
        errors: Vec::new(),
        reported_total_duration: None,
        per_file_durations,
    })
}

pub(super) fn validate_batch_item(item: &serde_json::Value) -> (bool, Option<String>, ErrorKind) {
    if let Some(error_value) = item.get("error") {
        let error_message = error_value.as_str().unwrap_or("unknown error");
        if !error_message.is_empty() {
            let kind = if error_message.contains("timed out") {
                ErrorKind::Timeout
            } else {
                ErrorKind::FrameworkError
            };
            return (false, Some(error_message.to_string()), kind);
        }
    }

    match item.get("content").and_then(serde_json::Value::as_str) {
        Some(content) if !content.trim().is_empty() => (true, None, ErrorKind::None),
        Some(_) => (
            false,
            Some("Framework returned empty content".to_string()),
            ErrorKind::EmptyContent,
        ),
        None => (
            false,
            Some("No content extracted (unsupported format or empty result)".to_string()),
            ErrorKind::EmptyContent,
        ),
    }
}

/// Check if verbose benchmark debugging is enabled via BENCHMARK_DEBUG env var.
pub(super) fn is_debug_enabled() -> bool {
    std::env::var("BENCHMARK_DEBUG").is_ok()
}
