//! Helper functions and utilities for extraction operations.
//!
//! This module provides shared utilities used across extraction modules.

use crate::plugins::DocumentExtractor;
use crate::types::{ErrorMetadata, ExtractionResult, Metadata};
#[cfg(test)]
use crate::utils::{PoolSizeHint, estimate_pool_size};
use crate::{KreuzbergError, Result};
use std::borrow::Cow;
use std::sync::Arc;

/// Get an extractor from the registry.
///
/// This function acquires the registry read lock and retrieves the appropriate
/// extractor for the given MIME type.
///
/// When the `otel` feature is enabled, the returned extractor is wrapped in an
/// [`InstrumentedExtractor`](crate::plugins::extractor::instrumented::InstrumentedExtractor)
/// that adds tracing spans and metrics automatically.
///
/// # Performance
///
/// RwLock read + HashMap lookup is ~100ns, fast enough without caching.
/// Removed thread-local cache to avoid Tokio work-stealing scheduler issues.
pub(in crate::core::extractor) fn get_extractor(mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
    let registry = crate::plugins::registry::get_document_extractor_registry();
    let registry_read = registry.read();
    let extractor = registry_read.get(mime_type)?;

    #[cfg(feature = "otel")]
    {
        Ok(Arc::new(
            crate::plugins::extractor::instrumented::InstrumentedExtractor::new(extractor),
        ))
    }

    #[cfg(not(feature = "otel"))]
    {
        Ok(extractor)
    }
}

/// Get optimal pool sizing hint for a document.
///
/// This function calculates recommended pool sizes based on the document's
/// file size and MIME type. The hint can be used to create appropriately
/// sized thread pools for extraction, reducing memory waste from over-allocation.
///
/// # Arguments
///
/// * `file_size` - The size of the file in bytes
/// * `mime_type` - The MIME type of the document
///
/// # Returns
///
/// A `PoolSizeHint` with recommended pool configurations
///
/// # Example
///
/// ```rust,ignore
/// use kreuzberg::core::extractor::get_pool_sizing_hint;
///
/// let hint = get_pool_sizing_hint(5_000_000, "application/pdf");
/// println!("Recommended string buffers: {}", hint.string_buffer_count);
/// ```
/// Build an error `ExtractionResult` for failed batch items.
///
/// Used by both tokio-based batch functions and WASM synchronous fallbacks
/// to construct a uniform error result.
pub(crate) fn error_extraction_result(e: &KreuzbergError, elapsed_ms: Option<u64>) -> ExtractionResult {
    let metadata = Metadata {
        error: Some(ErrorMetadata {
            error_type: format!("{:?}", e),
            message: e.to_string(),
        }),
        extraction_duration_ms: elapsed_ms,
        ..Default::default()
    };

    ExtractionResult {
        content: format!("Error: {}", e),
        mime_type: Cow::Borrowed("text/plain"),
        metadata,
        ..Default::default()
    }
}

#[cfg(test)]
#[inline]
pub(crate) fn get_pool_sizing_hint(file_size: u64, mime_type: &str) -> PoolSizeHint {
    estimate_pool_size(file_size, mime_type)
}
