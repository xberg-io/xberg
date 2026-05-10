//! Core types for benchmark results and metrics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Output format for document extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Markdown output format with structure preservation
    #[default]
    Markdown,
    /// Plain text output format
    Plaintext,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Markdown => write!(f, "markdown"),
            OutputFormat::Plaintext => write!(f, "plaintext"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "plaintext" | "text" | "txt" => Ok(OutputFormat::Plaintext),
            _ => Err(format!(
                "unknown output format: {}. Valid: markdown, md, plaintext, text, txt",
                s
            )),
        }
    }
}

/// Default output format for backward compatibility with old results
fn default_output_format() -> OutputFormat {
    OutputFormat::Markdown
}

/// OCR usage status for a benchmark extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OcrStatus {
    /// OCR was used for this extraction
    Used,
    /// OCR was not used for this extraction
    NotUsed,
    /// Unknown whether OCR was used
    #[default]
    Unknown,
}

/// Categorizes the source of a benchmark error.
///
/// This distinction is critical: framework errors are the framework's fault
/// (e.g. pdfplumber can't parse a malformed PDF), while harness errors are
/// our fault (e.g. timeout, process crash, invalid output format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The framework itself reported an extraction error (returned `{"error": "..."}`)
    /// This is NOT our fault - the framework couldn't handle this file.
    FrameworkError,
    /// A harness-level error: process crash, invalid JSON output, etc.
    /// This IS potentially our fault or an infrastructure issue.
    HarnessError,
    /// Extraction timed out (exceeded the configured timeout duration).
    Timeout,
    /// Framework returned empty or missing content (ran but produced nothing).
    EmptyContent,
    /// No error occurred
    #[default]
    None,
}

/// Complete benchmark result for a single file extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Framework that performed the extraction
    pub framework: String,

    /// Output format used for extraction (markdown or plaintext)
    #[serde(default = "default_output_format")]
    pub output_format: OutputFormat,

    /// Path to the test document
    pub file_path: PathBuf,

    /// File size in bytes
    pub file_size: u64,

    /// Whether extraction succeeded
    pub success: bool,

    /// Error message if extraction failed
    pub error_message: Option<String>,

    /// Categorizes the error source (framework vs harness)
    #[serde(default)]
    pub error_kind: ErrorKind,

    /// Total wall-clock duration (process spawn + extraction)
    /// For single iteration: the actual duration
    /// For multiple iterations: mean duration across all iterations
    pub duration: Duration,

    /// Pure extraction time (reported by subprocess via _extraction_time_ms)
    /// Only available for external frameworks with internal timing
    pub extraction_duration: Option<Duration>,

    /// Subprocess overhead (duration - extraction_duration)
    /// Only available when extraction_duration is present
    pub subprocess_overhead: Option<Duration>,

    /// Performance metrics (averaged across iterations if multiple)
    pub metrics: PerformanceMetrics,

    /// Quality metrics (if ground truth available)
    pub quality: Option<QualityMetrics>,

    /// Individual iteration results (empty for single iteration)
    pub iterations: Vec<IterationResult>,

    /// Statistical analysis of durations across iterations
    /// Only present when multiple iterations were run
    pub statistics: Option<DurationStatistics>,

    /// Cold start duration: Time from framework not loaded to ready and warm state
    /// This is measured during the first warmup extraction and represents the
    /// initial framework load time (imports, initializations, etc.)
    pub cold_start_duration: Option<Duration>,

    /// File extension without dot (e.g., "pdf", "docx")
    /// Extracted from file_path for per-extension analysis
    pub file_extension: String,

    /// Framework capability metadata at time of extraction
    /// Contains OCR support, batch support, async support flags
    pub framework_capabilities: FrameworkCapabilities,

    /// PDF-specific metadata (only present for PDF files)
    /// Includes text layer detection results and OCR strategy
    pub pdf_metadata: Option<PdfMetadata>,

    /// OCR usage status for this extraction
    #[serde(default)]
    pub ocr_status: OcrStatus,

    /// Extracted text content (for quality assessment)
    /// Not serialized to output JSON to save space
    #[serde(skip)]
    pub extracted_text: Option<String>,
}

impl BenchmarkResult {
    /// Create a framework key combining framework name, output format, and execution mode
    /// Format: "{framework}:{output_format}:{execution_mode}"
    /// Example: "kreuzberg-rust:markdown:batch"
    pub fn framework_key(&self, execution_mode: &str) -> String {
        format!("{}:{}:{}", self.framework, self.output_format, execution_mode)
    }
}

/// Performance metrics collected during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,

    /// Average CPU usage percentage (0-100)
    pub avg_cpu_percent: f64,

    /// Throughput in bytes per second
    pub throughput_bytes_per_sec: f64,

    /// 50th percentile memory usage in bytes
    pub p50_memory_bytes: u64,

    /// 95th percentile memory usage in bytes
    pub p95_memory_bytes: u64,

    /// 99th percentile memory usage in bytes
    pub p99_memory_bytes: u64,
}

/// Quality metrics comparing extraction output to ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Text token F1 score (0.0-1.0)
    pub f1_score_text: f64,

    /// Numeric token F1 score (0.0-1.0)
    pub f1_score_numeric: f64,

    /// Layout/structure F1 score (0.0-1.0), optional for plaintext mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f1_score_layout: Option<f64>,

    /// Overall text quality score (0.0-1.0)
    pub quality_score: f64,

    /// Tokens in ground truth but missing/under-represented in extraction (recall misses).
    /// Each entry is (token, deficit_count). Sorted by count descending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_tokens: Vec<(String, usize)>,

    /// Tokens in extraction but not in ground truth or over-represented (precision misses).
    /// Each entry is (token, surplus_count). Sorted by count descending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_tokens: Vec<(String, usize)>,

    /// Whether the extraction is considered correct (quality_score >= 0.95).
    #[serde(default)]
    pub correct: bool,
}

/// Framework capability metadata
///
/// Records the capabilities of the framework at the time of extraction,
/// enabling proper analysis and comparison of results based on framework features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameworkCapabilities {
    /// Extensions this framework supports (e.g., ["pdf", "docx"])
    #[serde(default)]
    pub supported_extensions: Vec<String>,

    /// Whether framework supports OCR
    #[serde(default)]
    pub ocr_support: bool,

    /// Whether framework supports batch processing
    #[serde(default)]
    pub batch_support: bool,

    /// Whether framework supports async extraction
    #[serde(default)]
    pub async_support: bool,

    /// Output formats this framework supports
    #[serde(default)]
    pub supported_output_formats: Vec<OutputFormat>,

    /// Framework version
    #[serde(default)]
    pub version: String,

    /// Disk installation size (if known)
    #[serde(default)]
    pub installation_size: Option<DiskSizeInfo>,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// Disk installation size information for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSizeInfo {
    /// Total size in bytes (package + system deps)
    pub size_bytes: u64,

    /// Package-only size in bytes (before adding system deps)
    #[serde(default)]
    pub package_bytes: u64,

    /// System dependency size in bytes (libreoffice, tesseract, ffmpeg, etc.)
    #[serde(default)]
    pub system_deps_bytes: u64,

    /// ML model size in bytes (auto-downloaded on first use)
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub model_bytes: u64,

    /// Measurement method (e.g., "binary_size", "pip_package", "npm_package")
    pub method: String,

    /// Human-readable description
    pub description: String,

    /// Breakdown of system dependency sizes by package name
    /// Keys are package names (e.g., "poppler-utils"), values are installed sizes in bytes.
    /// Only populated when runtime measurement via dpkg-query succeeds.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub system_deps_detail: HashMap<String, u64>,
}

/// PDF-specific metadata
///
/// Contains PDF text layer detection results and OCR strategy used.
/// Only populated for PDF documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMetadata {
    /// Whether PDF has a quality text layer
    /// Detected via pdftotext/pdffonts/pypdf
    pub has_text_layer: bool,

    /// Detection method used ("pdftotext", "pdffonts", "pypdf", "fallback")
    pub detection_method: String,

    /// Number of pages in the PDF
    pub page_count: Option<u32>,

    /// Whether OCR was enabled for this extraction
    pub ocr_enabled: bool,

    /// Text extraction quality hint (0.0-1.0)
    /// 0.0 = scanned image, 1.0 = native text
    pub text_quality_score: Option<f64>,
}

/// Result from a single benchmark iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationResult {
    /// Iteration number (0-indexed)
    pub iteration: usize,

    /// Total wall-clock duration for this iteration
    pub duration: Duration,

    /// Pure extraction time (if available from subprocess)
    pub extraction_duration: Option<Duration>,

    /// Performance metrics for this iteration
    pub metrics: PerformanceMetrics,
}

/// Statistical analysis of durations across multiple iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStatistics {
    /// Mean duration
    pub mean: Duration,

    /// Median duration
    pub median: Duration,

    /// Standard deviation (in milliseconds as f64)
    pub std_dev_ms: f64,

    /// Minimum duration
    pub min: Duration,

    /// Maximum duration
    pub max: Duration,

    /// 95th percentile duration
    pub p95: Duration,

    /// 99th percentile duration
    pub p99: Duration,

    /// Number of iterations included in statistics
    pub sample_count: usize,
}
