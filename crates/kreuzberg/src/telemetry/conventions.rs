//! Semantic conventions for kreuzberg telemetry.
//!
//! This module defines constant attribute names used across all kreuzberg
//! instrumentation. These follow the OpenTelemetry semantic conventions pattern
//! with a `kreuzberg.` namespace prefix.
//!
//! # Namespace Structure
//!
//! - `kreuzberg.operation` — top-level operation type
//! - `kreuzberg.document.*` — document-level attributes
//! - `kreuzberg.extractor.*` — extractor plugin attributes
//! - `kreuzberg.pipeline.*` — post-processing pipeline attributes
//! - `kreuzberg.cache.*` — extraction cache attributes
//! - `kreuzberg.batch.*` — batch extraction attributes
//! - `kreuzberg.ocr.*` — OCR backend attributes
//! - `kreuzberg.model.*` — ML model inference attributes
//! - `kreuzberg.error.*` — error classification attributes

// ---------------------------------------------------------------------------
// Operation
// ---------------------------------------------------------------------------

/// The top-level operation being performed.
///
/// Values: `extract_file`, `extract_bytes`, `batch_extract`, `pipeline`,
///         `cache_lookup`, `cache_write`.
pub const OPERATION: &str = "kreuzberg.operation";

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// Detected MIME type of the document (e.g. `application/pdf`).
pub const DOCUMENT_MIME_TYPE: &str = "kreuzberg.document.mime_type";

/// Size of the input document in bytes.
pub const DOCUMENT_SIZE_BYTES: &str = "kreuzberg.document.size_bytes";

/// Sanitised filename (no directory path — avoids PII in traces).
pub const DOCUMENT_FILENAME: &str = "kreuzberg.document.filename";

// ---------------------------------------------------------------------------
// Extractor
// ---------------------------------------------------------------------------

/// Plugin name of the extractor that handled the request (e.g. `pdf-extractor`).
pub const EXTRACTOR_NAME: &str = "kreuzberg.extractor.name";

/// Priority value of the selected extractor (0–100).
pub const EXTRACTOR_PRIORITY: &str = "kreuzberg.extractor.priority";

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Current pipeline stage.
///
/// Values: `extraction`, `post_processing.early`, `post_processing.middle`,
///         `post_processing.late`, `validation`, `chunking`,
///         `language_detection`, `token_reduction`.
pub const PIPELINE_STAGE: &str = "kreuzberg.pipeline.stage";

/// Name of the individual post-processor being executed.
pub const PIPELINE_PROCESSOR_NAME: &str = "kreuzberg.pipeline.processor_name";

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Whether the extraction cache was hit (`true` / `false`).
pub const CACHE_HIT: &str = "kreuzberg.cache.hit";

/// Cache key (content hash + config fingerprint).
pub const CACHE_KEY: &str = "kreuzberg.cache.key";

// ---------------------------------------------------------------------------
// Batch
// ---------------------------------------------------------------------------

/// Number of items in a batch extraction request.
pub const BATCH_SIZE: &str = "kreuzberg.batch.size";

/// Zero-based index of the current item within a batch.
pub const BATCH_INDEX: &str = "kreuzberg.batch.index";

// ---------------------------------------------------------------------------
// OCR
// ---------------------------------------------------------------------------

/// OCR backend name (e.g. `tesseract`, `paddle`).
pub const OCR_BACKEND: &str = "kreuzberg.ocr.backend";

/// ISO 639 language code(s) used for OCR (e.g. `eng`, `eng+deu`).
pub const OCR_LANGUAGE: &str = "kreuzberg.ocr.language";

// ---------------------------------------------------------------------------
// Model inference
// ---------------------------------------------------------------------------

/// Name or identifier of the ML model (e.g. `rtdetr-layout`, `paddle-det-server`).
pub const MODEL_NAME: &str = "kreuzberg.model.name";

/// Model inference wall-clock duration in milliseconds.
pub const MODEL_INFERENCE_MS: &str = "kreuzberg.model.inference_ms";

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// The `KreuzbergError` variant name (e.g. `Parsing`, `Timeout`, `UnsupportedFormat`).
pub const ERROR_TYPE: &str = "kreuzberg.error.type";

// ---------------------------------------------------------------------------
// Standard OTel overrides (for convenience)
// ---------------------------------------------------------------------------

/// Sanitize a file path to return only the filename (no directory).
///
/// Prevents PII from appearing in traces.
#[cfg(any(feature = "otel", feature = "tower-service"))]
pub(crate) fn sanitize_filename(path: &std::path::Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")
}

/// OpenTelemetry status code (`OK` or `ERROR`).
pub const OTEL_STATUS_CODE: &str = "otel.status_code";

/// Human-readable error message.
pub const ERROR_MESSAGE: &str = "error.message";

// ---------------------------------------------------------------------------
// Operation values (for use with OPERATION)
// ---------------------------------------------------------------------------

/// Canonical values for the `kreuzberg.operation` span attribute.
pub mod operations {
    /// Single-file extraction by path.
    pub const EXTRACT_FILE: &str = "extract_file";
    /// Single-file extraction from an in-memory byte slice.
    pub const EXTRACT_BYTES: &str = "extract_bytes";
    /// Batch extraction of multiple inputs.
    pub const BATCH_EXTRACT: &str = "batch_extract";
    /// Full extraction pipeline (extraction + post-processing + validation).
    pub const PIPELINE: &str = "pipeline";
    /// Cache read attempt (hit or miss).
    pub const CACHE_LOOKUP: &str = "cache_lookup";
    /// Cache write after a successful extraction.
    pub const CACHE_WRITE: &str = "cache_write";
}

// ---------------------------------------------------------------------------
// Pipeline stage values (for use with PIPELINE_STAGE)
// ---------------------------------------------------------------------------

/// Canonical values for the `kreuzberg.pipeline.stage` span attribute.
pub mod stages {
    /// Core document extraction stage.
    pub const EXTRACTION: &str = "extraction";
    /// Early post-processing (runs before middle and late processors).
    pub const POST_PROCESSING_EARLY: &str = "post_processing.early";
    /// Middle post-processing priority band.
    pub const POST_PROCESSING_MIDDLE: &str = "post_processing.middle";
    /// Late post-processing (runs after middle processors).
    pub const POST_PROCESSING_LATE: &str = "post_processing.late";
    /// Validator pass after post-processing.
    pub const VALIDATION: &str = "validation";
    /// Text chunking stage.
    pub const CHUNKING: &str = "chunking";
    /// Language detection stage.
    pub const LANGUAGE_DETECTION: &str = "language_detection";
    /// Token-reduction / summarization stage.
    pub const TOKEN_REDUCTION: &str = "token_reduction";
}

// ---------------------------------------------------------------------------
// Metric names
// ---------------------------------------------------------------------------

/// Canonical OpenTelemetry metric names in the `kreuzberg.*` namespace.
pub mod metrics {
    /// Counter: total extractions (labels: mime_type, extractor, status).
    pub const EXTRACTION_TOTAL: &str = "kreuzberg.extraction.total";

    /// Counter: cache hits.
    pub const CACHE_HITS: &str = "kreuzberg.extraction.cache.hits";

    /// Counter: cache misses.
    pub const CACHE_MISSES: &str = "kreuzberg.extraction.cache.misses";

    /// Counter: total batch requests (labels: status).
    pub const BATCH_TOTAL: &str = "kreuzberg.batch.total";

    /// Histogram: extraction wall-clock duration in ms (labels: mime_type, extractor).
    pub const EXTRACTION_DURATION_MS: &str = "kreuzberg.extraction.duration_ms";

    /// Histogram: input document size in bytes (labels: mime_type).
    pub const EXTRACTION_INPUT_BYTES: &str = "kreuzberg.extraction.input_size_bytes";

    /// Histogram: output content size in bytes (labels: mime_type).
    pub const EXTRACTION_OUTPUT_BYTES: &str = "kreuzberg.extraction.output_size_bytes";

    /// Histogram: pipeline stage duration in ms (labels: stage).
    pub const PIPELINE_DURATION_MS: &str = "kreuzberg.pipeline.duration_ms";

    /// Histogram: OCR duration in ms (labels: backend, language).
    pub const OCR_DURATION_MS: &str = "kreuzberg.ocr.duration_ms";

    /// Histogram: batch total duration in ms.
    pub const BATCH_DURATION_MS: &str = "kreuzberg.batch.duration_ms";

    /// Gauge (UpDownCounter): currently in-flight extractions.
    pub const CONCURRENT_EXTRACTIONS: &str = "kreuzberg.extraction.concurrent";
}

#[cfg(all(test, any(feature = "otel", feature = "tower-service")))]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn sanitize_filename_normal_path() {
        let path = Path::new("/home/user/doc.pdf");
        assert_eq!(sanitize_filename(path), "doc.pdf");
    }

    #[test]
    fn sanitize_filename_root_file() {
        let path = Path::new("doc.pdf");
        assert_eq!(sanitize_filename(path), "doc.pdf");
    }

    #[test]
    fn sanitize_filename_empty_path_returns_unknown() {
        // An empty path has no file_name component.
        let path = Path::new("");
        assert_eq!(sanitize_filename(path), "unknown");
    }

    #[cfg(unix)]
    #[test]
    fn sanitize_filename_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        // 0xFF is not valid UTF-8.
        let bad = OsStr::from_bytes(&[0xFF, 0xFE]);
        let path = Path::new(bad);
        assert_eq!(sanitize_filename(path), "unknown");
    }
}
