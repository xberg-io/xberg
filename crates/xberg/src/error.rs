//! Error types for Xberg.
//!
//! This module defines all error types used throughout the library. All errors
//! inherit from `XbergError` and follow Rust error handling best practices:
//!
//! - Use `thiserror` for automatic `Error` trait implementation
//! - Preserve error chains with `#[source]` attributes
//! - Include context in error messages (file paths, config values, etc.)
//!
//! # Error Handling Philosophy
//!
//! **System errors MUST always bubble up unchanged:**
//! - `XbergError::Io` (from `std::io::Error`) - File system errors, permission errors
//! - These indicate real system problems that users need to know about
//! - Never wrap or suppress these - they must surface to enable bug reports
//!
//! **Application errors are wrapped with context:**
//! - `Parsing` - Document format errors, corrupt files
//! - `Validation` - Invalid configuration or parameters
//! - `Ocr` - OCR processing failures
//! - `MissingDependency` - Missing optional system dependencies
//!
//! # Example
//!
//! ```rust
//! use xberg::{XbergError, Result};
//!
//! fn process_image_file(path: &str) -> Result<String> {
//!     // IO errors bubble up automatically via ?
//!     let content = std::fs::read_to_string(path)?;
//!
//!     // Application errors include context
//!     if content.is_empty() {
//!         return Err(XbergError::validation(
//!             format!("File is empty: {}", path)
//!         ));
//!     }
//!
//!     Ok(content)
//! }
//! ```
use thiserror::Error;

/// Result type alias using `XbergError`.
///
/// This is the standard return type for all fallible operations in Xberg.
pub type Result<T> = std::result::Result<T, XbergError>;

/// Main error type for all Xberg operations.
///
/// All errors in Xberg use this enum, which preserves error chains
/// and provides context for debugging.
///
/// # Variants
///
/// - `Io` - File system and I/O errors (always bubble up)
/// - `Parsing` - Document parsing errors (corrupt files, unsupported features)
/// - `Ocr` - OCR processing errors
/// - `Validation` - Input validation errors (invalid paths, config, parameters)
/// - `Cache` - Cache operation errors (non-fatal, can be ignored)
/// - `ImageProcessing` - Image manipulation errors
/// - `Serialization` - JSON/MessagePack serialization errors
/// - `MissingDependency` - Missing optional dependencies (tesseract, etc.)
/// - `Plugin` - Plugin-specific errors
/// - `LockPoisoned` - Mutex/RwLock poisoning (should not happen in normal operation)
/// - `UnsupportedFormat` - Unsupported MIME type or file format
/// - `Other` - Catch-all for uncommon errors
/// # FFI error codes — a stable public contract
///
/// Each variant carries `#[cfg_attr(alef, alef(error_code = N))]`. Without it alef has no stable
/// numeric taxonomy to bind to, so `alef_ffi_error_code()` collapses every variant to a single
/// unknown code and EVERY language binding loses the ability to tell error kinds apart --
/// `errors.Is(err, ErrOcr)` in Go, the `checkLastError` switch in Java, and Zig's typed error sets
/// all degrade to one catch-all. Refusing to guess is correct on alef's part; supplying the codes
/// is our job.
///
/// The blocks are `XbergError` 1000-1017, `HeuristicsError` 1100-1101, `LoadError` 1200-1205,
/// `ResolveError` 1300, chosen to leave room to grow and to stay clear of alef's own reserved
/// 0-4 (`None`/`Conversion`/`Unknown`/`Panic`/`InvalidHandle`).
///
/// These numbers cross the FFI boundary and are compiled into released bindings, so they are
/// append-only: **never renumber or reuse a code**, and give a new variant the next free number in
/// its block rather than inserting one in declaration order. ~keep
#[derive(Debug, Error)]
pub enum XbergError {
    /// A file system or I/O operation failed. These errors always bubble up unchanged.
    #[error("IO error: {0}")]
    #[cfg_attr(alef, alef(error_code = 1000))]
    Io(
        #[from]
        #[cfg_attr(alef, alef(skip))]
        std::io::Error,
    ),

    /// Document parsing failed (e.g. corrupt file, unsupported format feature).
    #[error("Parsing error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1001))]
    Parsing {
        /// Human-readable description of what failed during parsing.
        message: String,
        #[source]
        /// Underlying error that caused the parsing failure, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An OCR engine returned an error or produced unusable output.
    #[error("OCR error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1002))]
    Ocr {
        /// Human-readable description of the OCR failure.
        message: String,
        #[source]
        /// Underlying error from the OCR backend, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Invalid configuration or input parameters were supplied.
    #[error("Validation error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1003))]
    Validation {
        /// Human-readable description of the validation failure.
        message: String,
        #[source]
        /// Underlying error that triggered validation failure, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A cache read or write operation failed.
    #[error("Cache error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1004))]
    Cache {
        /// Human-readable description of the cache failure.
        message: String,
        #[source]
        /// Underlying error from the cache layer, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An image manipulation operation (resize, decode, DPI conversion) failed.
    #[error("Image processing error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1005))]
    ImageProcessing {
        /// Human-readable description of the image processing failure.
        message: String,
        #[source]
        /// Underlying error from the image library, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// JSON or MessagePack serialization/deserialization failed.
    #[error("Serialization error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1006))]
    Serialization {
        /// Human-readable description of the serialization failure.
        message: String,
        #[source]
        /// Underlying serde error, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A required optional system dependency (e.g. `tesseract`) was not found.
    #[error("Missing dependency: {0}")]
    #[cfg_attr(alef, alef(error_code = 1007))]
    MissingDependency(String),

    /// A registered plugin returned an error during extraction.
    #[error("Plugin error in '{plugin_name}': {message}")]
    #[cfg_attr(alef, alef(error_code = 1008))]
    Plugin {
        /// Human-readable description of what the plugin failed to do.
        message: String,
        /// Name of the plugin that reported the error.
        plugin_name: String,
    },

    /// An internal `Mutex` or `RwLock` was found in a poisoned state.
    #[error("Lock poisoned: {0}")]
    #[cfg_attr(alef, alef(error_code = 1009))]
    LockPoisoned(String),

    /// The document's MIME type is not supported by any registered extractor.
    #[error("Unsupported format: {0}")]
    #[cfg_attr(alef, alef(error_code = 1010))]
    UnsupportedFormat(String),

    /// The embedding model or embedding pipeline returned an error.
    #[error("Embedding error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1011))]
    Embedding {
        /// Human-readable description of the embedding failure.
        message: String,
        #[source]
        /// Underlying error from the embedding backend, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The reranker model or reranking pipeline returned an error.
    ///
    #[error("Reranking error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1012))]
    Reranking {
        /// Human-readable description of the reranking failure.
        message: String,
        #[source]
        /// Underlying error from the reranker backend, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Audio/video transcription failed.
    #[error("Transcription error: {message}")]
    #[cfg_attr(alef, alef(error_code = 1013))]
    Transcription {
        /// Human-readable description of the transcription failure.
        message: String,
        #[source]
        /// Underlying error from the transcription backend, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The extraction operation exceeded the configured time limit.
    #[error("Extraction timed out after {elapsed_ms}ms (limit: {limit_ms}ms)")]
    #[cfg_attr(alef, alef(error_code = 1014))]
    Timeout {
        /// Wall-clock milliseconds elapsed before the timeout was detected.
        elapsed_ms: u64,
        /// Configured timeout limit in milliseconds.
        limit_ms: u64,
    },

    /// The extraction stopped after a cooperative cancellation request.
    ///
    /// Rust callers can request cancellation through
    /// [`crate::cancellation::CancellationToken`]. The REST async-jobs API uses the
    /// same mechanism. Generated language bindings do not currently expose
    /// in-process cancellation.
    #[error("Extraction cancelled")]
    #[cfg_attr(alef, alef(error_code = 1015))]
    Cancelled,

    /// A security policy was violated (e.g. zip bomb, oversized archive).
    #[error("Security violation: {message}")]
    #[cfg_attr(alef, alef(error_code = 1016))]
    Security {
        /// Human-readable description of the security violation.
        message: String,
        #[source]
        /// Underlying security error, if available.
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A catch-all for uncommon errors that do not fit another variant.
    #[error("{0}")]
    #[cfg_attr(alef, alef(error_code = 1017))]
    Other(String),
}

impl From<crate::extractors::security::SecurityError> for XbergError {
    fn from(err: crate::extractors::security::SecurityError) -> Self {
        let message = err.to_string();
        XbergError::Security {
            message,
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
impl From<calamine::Error> for XbergError {
    fn from(err: calamine::Error) -> Self {
        XbergError::Parsing {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for XbergError {
    fn from(err: serde_json::Error) -> Self {
        XbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<rmp_serde::encode::Error> for XbergError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        XbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<rmp_serde::decode::Error> for XbergError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        XbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(feature = "pdf")]
impl From<crate::pdf::error::PdfError> for XbergError {
    fn from(err: crate::pdf::error::PdfError) -> Self {
        if matches!(err, crate::pdf::error::PdfError::Cancelled) {
            return XbergError::Cancelled;
        }
        XbergError::Parsing {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

macro_rules! error_constructor {
    ($name:ident, $variant:ident) => {
        pastey::paste! {
            #[doc = "Create a " $variant " error"]
            pub fn $name<S: Into<String>>(message: S) -> Self {
                Self::$variant {
                    message: message.into(),
                    source: None,
                }
            }

            #[doc = "Create a " $variant " error with source"]
            pub fn [<$name _with_source>]<S: Into<String>, E: std::error::Error + Send + Sync + 'static>(
                message: S,
                source: E,
            ) -> Self {
                Self::$variant {
                    message: message.into(),
                    source: Some(Box::new(source)),
                }
            }
        }
    };
}

impl XbergError {
    error_constructor!(parsing, Parsing);
    error_constructor!(ocr, Ocr);
    error_constructor!(validation, Validation);
    error_constructor!(cache, Cache);
    error_constructor!(image_processing, ImageProcessing);
    error_constructor!(serialization, Serialization);
    error_constructor!(embedding, Embedding);
    error_constructor!(reranking, Reranking);
    error_constructor!(security, Security);
    error_constructor!(transcription, Transcription);
}

/// Marks a PDF OCR aggregate's private diagnostic cause while preserving the backend error
/// itself as the next link in the typed source chain. ~keep
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[derive(Debug, Error)]
#[error("PDF OCR backend failure")]
struct PdfOcrFailureCause {
    #[source]
    backend: XbergError,
    // Constructed from aggregate counts and fixed text. ~keep
    cli_message: String,
}

impl XbergError {
    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn all_page_ocr_failure(message: String, cli_message: String, first_error: Self) -> Self {
        Self::Ocr {
            message,
            source: Some(Box::new(PdfOcrFailureCause {
                backend: first_error,
                cli_message,
            })),
        }
    }

    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    pub(crate) fn ocr_pipeline_failure(message: String, cli_message: &str, first_error: Option<Self>) -> Self {
        Self::Parsing {
            message,
            source: first_error.map(|backend| {
                Box::new(PdfOcrFailureCause {
                    backend,
                    cli_message: cli_message.to_owned(),
                }) as _
            }),
        }
    }

    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn is_all_page_ocr_failure(&self) -> bool {
        #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
        {
            matches!(self, Self::Ocr { source: Some(source), .. } if source.is::<PdfOcrFailureCause>())
        }
        #[cfg(not(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline"))))]
        {
            false
        }
    }

    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn pdf_ocr_trace_category(&self) -> String {
        use std::error::Error as _;

        match self
            .source()
            .and_then(|source| source.downcast_ref::<PdfOcrFailureCause>())
        {
            Some(cause) => cause.backend.extraction_diagnostic_category(),
            None => self.extraction_diagnostic_category(),
        }
    }

    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[cfg_attr(alef, alef(skip))]
    fn extraction_diagnostic_category(&self) -> String {
        use std::error::Error as _;

        let category = self.extraction_error_type();
        let io_error = match self {
            Self::Io(error) => Some(error),
            _ => self.source().and_then(|source| source.downcast_ref::<std::io::Error>()),
        };
        match io_error {
            Some(error) => format!("{category} ({:?})", error.kind()),
            None => category.to_owned(),
        }
    }

    /// Prepare an extraction error for the CLI renderer using the aggregate's safe summary.
    #[doc(hidden)]
    #[cfg_attr(alef, alef(skip))]
    pub fn into_cli_extraction_diagnostic(self) -> Self {
        #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
        {
            let mut error = self;
            if let Self::Ocr { message, source } | Self::Parsing { message, source } = &mut error
                && let Some(cause) = source
                    .as_ref()
                    .and_then(|cause| cause.downcast_ref::<PdfOcrFailureCause>())
            {
                let headline = &cause.cli_message;
                let category = cause.backend.extraction_diagnostic_category();
                *message = format!("{headline}; first failure category: {category}");
                *source = None;
            }
            error
        }
        #[cfg(not(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline"))))]
        self
    }
}

/// HTTP status category for [`XbergError`], used to select the REST API response status.
///
/// This is not part of the public API: it exists solely so `api::error::ApiError`'s
/// `From<XbergError>` conversion has a single canonical source for its status-code grouping
/// instead of a private copy of the match.
#[cfg(feature = "api")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub(crate) enum ApiStatusCategory {
    /// Maps to HTTP 400 Bad Request.
    Validation,
    /// Maps to HTTP 422 Unprocessable Entity.
    Unprocessable,
    /// Maps to HTTP 500 Internal Server Error.
    Internal,
}

/// JSON-RPC error category for [`XbergError`], used to select the MCP error code.
///
/// This is not part of the public API: it exists solely so `mcp::errors::map_xberg_error_to_mcp`
/// has a single canonical source for its error-code grouping instead of a private copy of the
/// match.
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub(crate) enum McpErrorCategory {
    /// Maps to JSON-RPC code -32602 (`INVALID_PARAMS`).
    InvalidParams,
    /// Maps to JSON-RPC code -32700 (`PARSE_ERROR`).
    ParseError,
    /// Maps to JSON-RPC code -32800 (`REQUEST_CANCELLED`).
    Cancelled,
    /// Maps to JSON-RPC code -32603 (`INTERNAL_ERROR`).
    Internal,
}

impl XbergError {
    /// Returns the canonical PascalCase error-type label used in REST API error responses.
    ///
    /// Exhaustive over every variant. This is the single source of truth for
    /// `api::types::ErrorResponse::error_type`; do not duplicate this match elsewhere.
    #[cfg(feature = "api")]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn api_error_type(&self) -> &'static str {
        match self {
            XbergError::Validation { .. } => "ValidationError",
            XbergError::Parsing { .. } => "ParsingError",
            XbergError::Ocr { .. } if self.is_all_page_ocr_failure() => "PluginError",
            XbergError::Ocr { .. } => "OCRError",
            XbergError::Io(_) => "IOError",
            XbergError::Cache { .. } => "CacheError",
            XbergError::ImageProcessing { .. } => "ImageProcessingError",
            XbergError::Serialization { .. } => "SerializationError",
            XbergError::MissingDependency(_) => "MissingDependencyError",
            XbergError::Plugin { .. } => "PluginError",
            XbergError::LockPoisoned(_) => "LockPoisonedError",
            XbergError::UnsupportedFormat(_) => "UnsupportedFormatError",
            XbergError::Embedding { .. } => "EmbeddingError",
            XbergError::Timeout { .. } => "TimeoutError",
            XbergError::Other(_) => "Error",
            XbergError::Cancelled => "CancelledError",
            XbergError::Security { .. } => "SecurityError",
            XbergError::Transcription { .. } => "TranscriptionError",
            XbergError::Reranking { .. } => "RerankingError",
        }
    }

    /// Returns the canonical HTTP status category used by the REST API error conversion.
    ///
    /// This is the single source of truth for `api::error::ApiError`'s `From<XbergError>`
    /// status-code selection; do not duplicate this match elsewhere.
    #[cfg(feature = "api")]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn api_status_category(&self) -> ApiStatusCategory {
        match self {
            XbergError::Validation { .. } | XbergError::UnsupportedFormat(_) => ApiStatusCategory::Validation,
            XbergError::Ocr { .. } if self.is_all_page_ocr_failure() => ApiStatusCategory::Internal,
            XbergError::Parsing { .. } | XbergError::Ocr { .. } => ApiStatusCategory::Unprocessable,
            _ => ApiStatusCategory::Internal,
        }
    }

    /// Returns the canonical snake_case error-type label used in batch extraction error items.
    ///
    /// This is the single source of truth for
    /// `core::config::extraction::types::ExtractionErrorItem::error_type`; do not duplicate this
    /// match elsewhere.
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn extraction_error_type(&self) -> &'static str {
        match self {
            XbergError::Io(_) => "io",
            XbergError::Parsing { .. } => "parsing",
            XbergError::Ocr { .. } if self.is_all_page_ocr_failure() => "plugin",
            XbergError::Ocr { .. } => "ocr",
            XbergError::Validation { .. } => "validation",
            XbergError::Cache { .. } => "cache",
            XbergError::ImageProcessing { .. } => "image_processing",
            XbergError::Serialization { .. } => "serialization",
            XbergError::MissingDependency(_) => "missing_dependency",
            XbergError::Plugin { .. } => "plugin",
            XbergError::LockPoisoned(_) => "lock_poisoned",
            XbergError::UnsupportedFormat(_) => "unsupported_format",
            XbergError::Embedding { .. } => "embedding",
            XbergError::Reranking { .. } => "reranking",
            XbergError::Transcription { .. } => "transcription",
            XbergError::Timeout { .. } => "timeout",
            XbergError::Cancelled => "cancelled",
            XbergError::Security { .. } => "security",
            XbergError::Other(_) => "other",
        }
    }

    /// Returns the canonical numeric error code used in batch extraction error items.
    ///
    /// These values match the stable FFI error codes declared on [`XbergError`], with the
    /// legacy plugin code retained for all-page OCR failures. This is the single source of truth for
    /// `core::config::extraction::types::ExtractionErrorItem::code`; do not duplicate this match
    /// elsewhere.
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn extraction_error_code(&self) -> u32 {
        match self {
            XbergError::Io(_) => 1000,
            XbergError::Parsing { .. } => 1001,
            XbergError::Ocr { .. } if self.is_all_page_ocr_failure() => 1008,
            XbergError::Ocr { .. } => 1002,
            XbergError::Validation { .. } => 1003,
            XbergError::Cache { .. } => 1004,
            XbergError::ImageProcessing { .. } => 1005,
            XbergError::Serialization { .. } => 1006,
            XbergError::MissingDependency(_) => 1007,
            XbergError::Plugin { .. } => 1008,
            XbergError::LockPoisoned(_) => 1009,
            XbergError::UnsupportedFormat(_) => 1010,
            XbergError::Embedding { .. } => 1011,
            XbergError::Reranking { .. } => 1012,
            XbergError::Transcription { .. } => 1013,
            XbergError::Timeout { .. } => 1014,
            XbergError::Cancelled => 1015,
            XbergError::Security { .. } => 1016,
            XbergError::Other(_) => 1017,
        }
    }

    /// Returns the canonical JSON-RPC error category used by the MCP error conversion.
    ///
    /// This is the single source of truth for `mcp::errors::map_xberg_error_to_mcp`'s error-code
    /// selection; do not duplicate this match elsewhere.
    #[cfg(feature = "mcp")]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn mcp_error_category(&self) -> McpErrorCategory {
        match self {
            XbergError::Validation { .. }
            | XbergError::UnsupportedFormat(_)
            | XbergError::MissingDependency(_)
            | XbergError::Security { .. } => McpErrorCategory::InvalidParams,
            XbergError::Parsing { .. } => McpErrorCategory::ParseError,
            XbergError::Cancelled => McpErrorCategory::Cancelled,
            _ => McpErrorCategory::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[test]
    fn cli_diagnostic_omits_raw_pdf_ocr_sources_from_debug() {
        use std::error::Error as _;

        const PATH: &str = "/tmp/PATH_SENTINEL/model.onnx";
        const SECRET: &str = "SECRET_SENTINEL";
        let backend_failure =
            || XbergError::ocr_with_source("page read failed", std::io::Error::other(format!("{PATH} {SECRET}")));
        let ordinary_failure =
            XbergError::parsing_with_source("ordinary failure", std::io::Error::other("invalid header"));
        let ordinary_debug = format!("{ordinary_failure:?}");
        assert_eq!(
            format!("{:?}", ordinary_failure.into_cli_extraction_diagnostic()),
            ordinary_debug
        );
        assert!(ordinary_debug.contains("invalid header"));

        let raw_message = format!("backend failed opening {PATH}");
        for (error, raw_prefix, safe_message) in [
            (
                XbergError::all_page_ocr_failure(raw_message.clone(), "every page failed".into(), backend_failure()),
                "OCR error",
                "OCR error: every page failed; first failure category: ocr (Other)",
            ),
            (
                XbergError::ocr_pipeline_failure(raw_message.clone(), "every stage failed", Some(backend_failure())),
                "Parsing error",
                "Parsing error: every stage failed; first failure category: ocr (Other)",
            ),
        ] {
            let raw = format!("{error:?}");
            assert!(raw.contains(PATH) && raw.contains(SECRET), "{raw}");
            assert_eq!(error.to_string(), format!("{raw_prefix}: {raw_message}"));
            let diagnostic = error.into_cli_extraction_diagnostic();
            assert_eq!(diagnostic.to_string(), safe_message);
            assert!(diagnostic.source().is_none());
            let rendered = format!("{diagnostic:?}");
            assert!(!rendered.contains(PATH) && !rendered.contains(SECRET), "{rendered}");
        }
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let krz_err: XbergError = io_err.into();
        assert!(matches!(krz_err, XbergError::Io(_)));
        assert!(krz_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_parsing_error() {
        let err = XbergError::parsing("invalid format");
        assert_eq!(err.to_string(), "Parsing error: invalid format");
    }

    #[test]
    fn test_parsing_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "bad data");
        let err = XbergError::parsing_with_source("invalid format", source);
        assert_eq!(err.to_string(), "Parsing error: invalid format");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_ocr_error() {
        let err = XbergError::ocr("OCR failed");
        assert_eq!(err.to_string(), "OCR error: OCR failed");
    }

    #[test]
    fn test_ocr_error_with_source() {
        let source = std::io::Error::other("tesseract failed");
        let err = XbergError::ocr_with_source("OCR failed", source);
        assert_eq!(err.to_string(), "OCR error: OCR failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_validation_error() {
        let err = XbergError::validation("invalid input");
        assert_eq!(err.to_string(), "Validation error: invalid input");
    }

    #[test]
    fn test_validation_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad param");
        let err = XbergError::validation_with_source("invalid input", source);
        assert_eq!(err.to_string(), "Validation error: invalid input");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_cache_error() {
        let err = XbergError::cache("cache write failed");
        assert_eq!(err.to_string(), "Cache error: cache write failed");
    }

    #[test]
    fn test_cache_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "cannot write");
        let err = XbergError::cache_with_source("cache write failed", source);
        assert_eq!(err.to_string(), "Cache error: cache write failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_image_processing_error() {
        let err = XbergError::image_processing("resize failed");
        assert_eq!(err.to_string(), "Image processing error: resize failed");
    }

    #[test]
    fn test_image_processing_error_with_source() {
        let source = std::io::Error::other("image decode failed");
        let err = XbergError::image_processing_with_source("resize failed", source);
        assert_eq!(err.to_string(), "Image processing error: resize failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_serialization_error() {
        let err = XbergError::serialization("JSON parse error");
        assert_eq!(err.to_string(), "Serialization error: JSON parse error");
    }

    #[test]
    fn test_serialization_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "bad format");
        let err = XbergError::serialization_with_source("JSON parse error", source);
        assert_eq!(err.to_string(), "Serialization error: JSON parse error");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_missing_dependency_error() {
        let err = XbergError::MissingDependency("tesseract not found".to_string());
        assert_eq!(err.to_string(), "Missing dependency: tesseract not found");
    }

    #[test]
    fn test_plugin_error() {
        let err = XbergError::Plugin {
            message: "extraction failed".to_string(),
            plugin_name: "pdf-extractor".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin error in 'pdf-extractor': extraction failed");
    }

    #[test]
    fn test_unsupported_format_error() {
        let err = XbergError::UnsupportedFormat("application/unknown".to_string());
        assert_eq!(err.to_string(), "Unsupported format: application/unknown");
    }

    #[test]
    fn test_other_error() {
        let err = XbergError::Other("unexpected error".to_string());
        assert_eq!(err.to_string(), "unexpected error");
    }

    #[test]
    #[cfg(any(feature = "excel", feature = "excel-wasm"))]
    fn test_calamine_error_conversion() {
        let cal_err = calamine::Error::Msg("invalid Excel file");
        let krz_err: XbergError = cal_err.into();
        assert!(matches!(krz_err, XbergError::Parsing { .. }));
        assert!(krz_err.to_string().contains("Parsing error"));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let krz_err: XbergError = json_err.into();
        assert!(matches!(krz_err, XbergError::Serialization { .. }));
        assert!(krz_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_rmp_encode_error_conversion() {
        use std::collections::HashMap;
        let mut map: HashMap<Vec<u8>, String> = HashMap::new();
        map.insert(vec![255, 255], "test".to_string());

        let result = rmp_serde::to_vec(&map);
        if let Err(rmp_err) = result {
            let krz_err: XbergError = rmp_err.into();
            assert!(matches!(krz_err, XbergError::Serialization { .. }));
        }
    }

    #[test]
    fn test_rmp_decode_error_conversion() {
        let invalid_msgpack = vec![0xFF, 0xFF, 0xFF];
        let rmp_err = rmp_serde::from_slice::<String>(&invalid_msgpack).unwrap_err();
        let krz_err: XbergError = rmp_err.into();
        assert!(matches!(krz_err, XbergError::Serialization { .. }));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_error_conversion() {
        let pdf_err = crate::pdf::error::PdfError::InvalidPdf("corrupt PDF".to_string());
        let krz_err: XbergError = pdf_err.into();
        assert!(matches!(krz_err, XbergError::Parsing { .. }));
    }

    #[test]
    fn test_error_debug() {
        let err = XbergError::validation("test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Validation"));
    }

    #[test]
    fn test_lock_poisoned_error() {
        let err = XbergError::LockPoisoned("Registry lock poisoned".to_string());
        assert_eq!(err.to_string(), "Lock poisoned: Registry lock poisoned");
    }

    #[test]
    fn test_io_error_bubbles_unchanged() {
        fn read_file() -> Result<String> {
            let content = std::fs::read_to_string("/nonexistent/file.txt")?;
            Ok(content)
        }

        let result = read_file();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XbergError::Io(_)));
    }

    #[test]
    fn test_io_error_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let krz_err: XbergError = io_err.into();
        assert!(matches!(krz_err, XbergError::Io(_)));
        assert!(krz_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let krz_err: XbergError = io_err.into();
        assert!(matches!(krz_err, XbergError::Io(_)));
        assert!(krz_err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_io_error_invalid_data_vs_parsing() {
        let io_err = std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupted data");
        let krz_err: XbergError = io_err.into();
        assert!(matches!(krz_err, XbergError::Io(_)));

        let parse_err = XbergError::parsing("corrupted format");
        assert!(matches!(parse_err, XbergError::Parsing { .. }));
    }

    fn plugin_error() -> XbergError {
        XbergError::Plugin {
            message: "plugin failed".to_string(),
            plugin_name: "test-plugin".to_string(),
        }
    }

    #[cfg(feature = "api")]
    #[test]
    fn should_map_every_variant_to_its_canonical_api_error_type() {
        assert_eq!(XbergError::Io(std::io::Error::other("t")).api_error_type(), "IOError");
        assert_eq!(XbergError::parsing("t").api_error_type(), "ParsingError");
        assert_eq!(XbergError::ocr("t").api_error_type(), "OCRError");
        assert_eq!(XbergError::validation("t").api_error_type(), "ValidationError");
        assert_eq!(XbergError::cache("t").api_error_type(), "CacheError");
        assert_eq!(
            XbergError::image_processing("t").api_error_type(),
            "ImageProcessingError"
        );
        assert_eq!(XbergError::serialization("t").api_error_type(), "SerializationError");
        assert_eq!(
            XbergError::MissingDependency("t".to_string()).api_error_type(),
            "MissingDependencyError"
        );
        assert_eq!(plugin_error().api_error_type(), "PluginError");
        assert_eq!(
            XbergError::LockPoisoned("t".to_string()).api_error_type(),
            "LockPoisonedError"
        );
        assert_eq!(
            XbergError::UnsupportedFormat("t/mime".to_string()).api_error_type(),
            "UnsupportedFormatError"
        );
        assert_eq!(XbergError::embedding("t").api_error_type(), "EmbeddingError");
        assert_eq!(
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2
            }
            .api_error_type(),
            "TimeoutError"
        );
        assert_eq!(XbergError::Other("t".to_string()).api_error_type(), "Error");
        assert_eq!(XbergError::Cancelled.api_error_type(), "CancelledError");
        assert_eq!(XbergError::security("t").api_error_type(), "SecurityError");
        assert_eq!(XbergError::transcription("t").api_error_type(), "TranscriptionError");
        assert_eq!(XbergError::reranking("t").api_error_type(), "RerankingError");
    }

    #[cfg(feature = "api")]
    #[test]
    fn should_categorize_validation_and_unsupported_format_as_bad_request() {
        assert_eq!(
            XbergError::validation("t").api_status_category(),
            ApiStatusCategory::Validation
        );
        assert_eq!(
            XbergError::UnsupportedFormat("t".to_string()).api_status_category(),
            ApiStatusCategory::Validation
        );
    }

    #[cfg(feature = "api")]
    #[test]
    fn should_categorize_parsing_and_ocr_as_unprocessable_entity() {
        assert_eq!(
            XbergError::parsing("t").api_status_category(),
            ApiStatusCategory::Unprocessable
        );
        assert_eq!(
            XbergError::ocr("t").api_status_category(),
            ApiStatusCategory::Unprocessable
        );
    }

    #[cfg(feature = "api")]
    #[test]
    fn should_default_remaining_variants_to_internal_server_error() {
        assert_eq!(
            XbergError::Io(std::io::Error::other("t")).api_status_category(),
            ApiStatusCategory::Internal
        );
        assert_eq!(XbergError::Cancelled.api_status_category(), ApiStatusCategory::Internal);
        assert_eq!(
            XbergError::Other("t".to_string()).api_status_category(),
            ApiStatusCategory::Internal
        );
        assert_eq!(plugin_error().api_status_category(), ApiStatusCategory::Internal);
    }

    #[test]
    fn should_map_every_variant_to_its_canonical_extraction_error_type() {
        assert_eq!(XbergError::Io(std::io::Error::other("t")).extraction_error_type(), "io");
        assert_eq!(XbergError::parsing("t").extraction_error_type(), "parsing");
        assert_eq!(XbergError::ocr("t").extraction_error_type(), "ocr");
        assert_eq!(XbergError::validation("t").extraction_error_type(), "validation");
        assert_eq!(XbergError::cache("t").extraction_error_type(), "cache");
        assert_eq!(
            XbergError::image_processing("t").extraction_error_type(),
            "image_processing"
        );
        assert_eq!(XbergError::serialization("t").extraction_error_type(), "serialization");
        assert_eq!(
            XbergError::MissingDependency("t".to_string()).extraction_error_type(),
            "missing_dependency"
        );
        assert_eq!(plugin_error().extraction_error_type(), "plugin");
        assert_eq!(
            XbergError::LockPoisoned("t".to_string()).extraction_error_type(),
            "lock_poisoned"
        );
        assert_eq!(
            XbergError::UnsupportedFormat("t/mime".to_string()).extraction_error_type(),
            "unsupported_format"
        );
        assert_eq!(XbergError::embedding("t").extraction_error_type(), "embedding");
        assert_eq!(XbergError::reranking("t").extraction_error_type(), "reranking");
        assert_eq!(XbergError::transcription("t").extraction_error_type(), "transcription");
        assert_eq!(
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2
            }
            .extraction_error_type(),
            "timeout"
        );
        assert_eq!(XbergError::Cancelled.extraction_error_type(), "cancelled");
        assert_eq!(XbergError::security("t").extraction_error_type(), "security");
        assert_eq!(XbergError::Other("t".to_string()).extraction_error_type(), "other");
    }

    #[test]
    fn should_map_every_variant_to_its_canonical_ffi_error_code() {
        assert_eq!(XbergError::Io(std::io::Error::other("t")).extraction_error_code(), 1000);
        assert_eq!(XbergError::parsing("t").extraction_error_code(), 1001);
        assert_eq!(XbergError::ocr("t").extraction_error_code(), 1002);
        assert_eq!(XbergError::validation("t").extraction_error_code(), 1003);
        assert_eq!(XbergError::cache("t").extraction_error_code(), 1004);
        assert_eq!(XbergError::image_processing("t").extraction_error_code(), 1005);
        assert_eq!(XbergError::serialization("t").extraction_error_code(), 1006);
        assert_eq!(
            XbergError::MissingDependency("t".to_string()).extraction_error_code(),
            1007
        );
        assert_eq!(plugin_error().extraction_error_code(), 1008);
        assert_eq!(XbergError::LockPoisoned("t".to_string()).extraction_error_code(), 1009);
        assert_eq!(
            XbergError::UnsupportedFormat("t/mime".to_string()).extraction_error_code(),
            1010
        );
        assert_eq!(XbergError::embedding("t").extraction_error_code(), 1011);
        assert_eq!(XbergError::reranking("t").extraction_error_code(), 1012);
        assert_eq!(XbergError::transcription("t").extraction_error_code(), 1013);
        assert_eq!(
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2
            }
            .extraction_error_code(),
            1014
        );
        assert_eq!(XbergError::Cancelled.extraction_error_code(), 1015);
        assert_eq!(XbergError::security("t").extraction_error_code(), 1016);
        assert_eq!(XbergError::Other("t".to_string()).extraction_error_code(), 1017);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn should_categorize_client_input_errors_as_invalid_params_for_mcp() {
        assert_eq!(
            XbergError::validation("t").mcp_error_category(),
            McpErrorCategory::InvalidParams
        );
        assert_eq!(
            XbergError::UnsupportedFormat("t".to_string()).mcp_error_category(),
            McpErrorCategory::InvalidParams
        );
        assert_eq!(
            XbergError::MissingDependency("t".to_string()).mcp_error_category(),
            McpErrorCategory::InvalidParams
        );
        assert_eq!(
            XbergError::security("t").mcp_error_category(),
            McpErrorCategory::InvalidParams
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn should_categorize_parsing_as_parse_error_for_mcp() {
        assert_eq!(
            XbergError::parsing("t").mcp_error_category(),
            McpErrorCategory::ParseError
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn should_categorize_cancelled_as_cancelled_for_mcp() {
        assert_eq!(XbergError::Cancelled.mcp_error_category(), McpErrorCategory::Cancelled);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn should_default_remaining_variants_to_internal_for_mcp() {
        assert_eq!(
            XbergError::Io(std::io::Error::other("t")).mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(XbergError::ocr("t").mcp_error_category(), McpErrorCategory::Internal);
        assert_eq!(XbergError::cache("t").mcp_error_category(), McpErrorCategory::Internal);
        assert_eq!(
            XbergError::image_processing("t").mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::serialization("t").mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(plugin_error().mcp_error_category(), McpErrorCategory::Internal);
        assert_eq!(
            XbergError::LockPoisoned("t".to_string()).mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::embedding("t").mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2
            }
            .mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::Other("t".to_string()).mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::transcription("t").mcp_error_category(),
            McpErrorCategory::Internal
        );
        assert_eq!(
            XbergError::reranking("t").mcp_error_category(),
            McpErrorCategory::Internal
        );
    }
}
