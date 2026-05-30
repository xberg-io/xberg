//! Document extractor plugin trait.
//!
//! This module defines the trait for implementing custom document extractors.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::Plugin;
use crate::types::internal::InternalDocument;
use async_trait::async_trait;
use std::path::Path;

#[cfg(not(feature = "tokio-runtime"))]
use crate::KreuzbergError;

/// Trait for document extractor plugins.
///
/// Implement this trait to add support for new document formats or to override
/// built-in extraction behavior with custom logic.
///
/// # Return Type
///
/// Extractors return `InternalDocument`, a flat intermediate representation.
/// The pipeline converts this into the public `ExtractionResult` via the
/// derivation step.
///
/// # Priority System
///
/// When multiple extractors support the same MIME type, the registry selects
/// the extractor with the highest priority value. Use this to:
/// - Override built-in extractors (priority > 50)
/// - Provide fallback extractors (priority < 50)
/// - Implement specialized extractors for specific use cases
///
/// Default priority is 50.
///
/// # Thread Safety
///
/// Extractors must be thread-safe (`Send + Sync`) to support concurrent extraction.
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, DocumentExtractor};
/// use kreuzberg::{Result, ExtractionConfig};
/// use kreuzberg::types::internal::InternalDocument;
/// use async_trait::async_trait;
/// use std::path::Path;
///
/// /// Custom PDF extractor with premium features
/// struct PremiumPdfExtractor;
///
/// impl Plugin for PremiumPdfExtractor {
///     fn name(&self) -> &str { "premium-pdf" }
///     fn version(&self) -> String { "2.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl DocumentExtractor for PremiumPdfExtractor {
///     async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
///         -> Result<InternalDocument> {
///         // Premium extraction logic with better accuracy
///         let mut doc = InternalDocument::new("pdf");
///         // ... populate doc.elements, doc.metadata, etc.
///         Ok(doc)
///     }
///
///     fn supported_mime_types(&self) -> &[&str] {
///         &["application/pdf"]
///     }
///
///     fn priority(&self) -> i32 {
///         100  // Higher than default (50) - will be preferred
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait DocumentExtractor: Plugin {
    /// Extract content from a byte array.
    ///
    /// This is the core extraction method that processes in-memory document data.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw document bytes
    /// * `mime_type` - MIME type of the document (already validated)
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// An `InternalDocument` containing the extracted elements, metadata, and tables.
    /// The pipeline will convert this into the public `ExtractionResult`.
    ///
    /// # Errors
    ///
    /// - `KreuzbergError::Parsing` - Document parsing failed
    /// - `KreuzbergError::Validation` - Invalid document structure
    /// - `KreuzbergError::Io` - I/O errors (these always bubble up)
    /// - `KreuzbergError::MissingDependency` - Required dependency not available
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument>;

    /// Extract content from a file.
    ///
    /// Default implementation reads the file and calls `extract_bytes`.
    /// Override for custom file handling, streaming, or memory optimizations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the document file
    /// * `mime_type` - MIME type of the document (already validated)
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// An `InternalDocument` containing the extracted elements, metadata, and tables.
    ///
    /// # Errors
    ///
    /// Same as `extract_bytes`, plus file I/O errors.
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        #[cfg(feature = "tokio-runtime")]
        {
            use crate::core::io;
            // Use memory-mapped I/O for large files (> 1 MiB) to avoid an extra
            // heap allocation.  `open_file_bytes` falls back to a plain read for
            // small files and on WASM where mmap is unavailable.
            let bytes = io::open_file_bytes(path)?;
            self.extract_bytes(&bytes, mime_type, config).await
        }
        #[cfg(not(feature = "tokio-runtime"))]
        {
            let _ = (path, mime_type, config);
            Err(KreuzbergError::Other(
                "File-based extraction requires the tokio-runtime feature".to_string(),
            ))
        }
    }

    /// Get the list of MIME types supported by this extractor.
    ///
    /// Can include exact MIME types and prefix patterns:
    /// - Exact: `"application/pdf"`, `"text/plain"`
    /// - Prefix: `"image/*"` (matches any image type)
    ///
    /// # Returns
    ///
    /// A slice of MIME type strings.
    fn supported_mime_types(&self) -> &[&str];

    /// Get the priority of this extractor.
    ///
    /// Higher priority extractors are preferred when multiple extractors
    /// support the same MIME type.
    ///
    /// # Priority Guidelines
    ///
    /// - **0-25**: Fallback/low-quality extractors
    /// - **26-49**: Alternative extractors
    /// - **50**: Default priority (built-in extractors)
    /// - **51-75**: Premium/enhanced extractors
    /// - **76-100**: Specialized/high-priority extractors
    ///
    /// # Returns
    ///
    /// Priority value (default: 50)
    fn priority(&self) -> i32 {
        50
    }

    /// Optional: Check if this extractor can handle a specific file.
    ///
    /// Allows for more sophisticated detection beyond MIME types.
    /// Defaults to `true` (rely on MIME type matching).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to check
    /// * `mime_type` - Detected MIME type
    ///
    /// # Returns
    ///
    /// `true` if the extractor can handle this file, `false` otherwise.
    fn can_handle(&self, _path: &Path, _mime_type: &str) -> bool {
        true
    }

    /// Attempt to get a reference to this extractor as a SyncExtractor.
    ///
    /// Returns None if the extractor doesn't support synchronous extraction.
    /// This is used for WASM and other sync-only environments.
    #[doc(hidden)]
    fn as_sync_extractor(&self) -> Option<&dyn crate::extractors::SyncExtractor> {
        None
    }
}
