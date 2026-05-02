//! Post-processor plugin trait.
//!
//! This module defines traits for implementing custom post-processing logic.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::Plugin;
use crate::types::ExtractionResult;
use async_trait::async_trait;

/// Processing stages for post-processors.
///
/// Post-processors are executed in stage order (Early → Middle → Late).
/// Use stages to control the order of post-processing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum ProcessingStage {
    /// Early stage - foundational processing.
    ///
    /// Use for:
    /// - Language detection
    /// - Character encoding normalization
    /// - Entity extraction (NER)
    /// - Text quality scoring
    #[default]
    Early,

    /// Middle stage - content transformation.
    ///
    /// Use for:
    /// - Keyword extraction
    /// - Token reduction
    /// - Text summarization
    /// - Semantic analysis
    Middle,

    /// Late stage - final enrichment.
    ///
    /// Use for:
    /// - Custom user hooks
    /// - Analytics/logging
    /// - Final validation
    /// - Output formatting
    Late,
}

/// Trait for post-processor plugins.
///
/// Post-processors transform or enrich extraction results after the initial
/// extraction is complete. They can:
/// - Clean and normalize text
/// - Add metadata (language, keywords, entities)
/// - Split content into chunks
/// - Score quality
/// - Apply custom transformations
///
/// # Processing Order
///
/// Post-processors are executed in stage order:
/// 1. **Early** - Language detection, entity extraction
/// 2. **Middle** - Keyword extraction, token reduction
/// 3. **Late** - Custom hooks, final validation
///
/// Within each stage, processors are executed in registration order.
///
/// # Error Handling
///
/// Post-processor errors are non-fatal by default - they're captured in metadata
/// and execution continues. To make errors fatal, return an error from `process()`.
///
/// # Thread Safety
///
/// Post-processors must be thread-safe (`Send + Sync`).
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
/// use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
/// use async_trait::async_trait;
///
/// /// Add word count metadata to extraction results
/// struct WordCountProcessor;
///
/// impl Plugin for WordCountProcessor {
///     fn name(&self) -> &str { "word-count" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl PostProcessor for WordCountProcessor {
///     async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
///         -> Result<()> {
///         // Count words
///         let word_count = result.content.split_whitespace().count();
///
///         // Add to metadata
///         result.metadata.additional.insert("word_count".to_string().into(), serde_json::json!(word_count));
///
///         Ok(())
///     }
///
///     fn processing_stage(&self) -> ProcessingStage {
///         ProcessingStage::Early
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait PostProcessor: Plugin {
    /// Process an extraction result.
    ///
    /// Transform or enrich the extraction result. Can modify:
    /// - `content` - The extracted text
    /// - `metadata` - Add or update metadata fields
    /// - `tables` - Modify or enhance table data
    ///
    /// # Arguments
    ///
    /// * `result` - Mutable reference to the extraction result to process
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` if processing succeeded, `Err(...)` for fatal failures.
    ///
    /// # Errors
    ///
    /// Return errors for fatal processing failures. Non-fatal errors should be
    /// captured in metadata directly on the result.
    ///
    /// # Performance
    ///
    /// This signature avoids unnecessary cloning of large extraction results by
    /// taking a mutable reference instead of ownership. Processors modify the
    /// result in place.
    ///
    /// # Example - Language Detection
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct LanguageDetector;
    /// # impl Plugin for LanguageDetector {
    /// #     fn name(&self) -> &str { "language-detector" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for LanguageDetector {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Early }
    /// async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Detect language (simplified - use real detection library in practice)
    ///     let language = "en"; // Placeholder detection
    ///
    ///     // Add to metadata
    ///     result.metadata.additional.insert("detected_language".to_string().into(), serde_json::json!(language));
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    ///
    /// # Example - Text Cleaning
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct TextCleaner;
    /// # impl Plugin for TextCleaner {
    /// #     fn name(&self) -> &str { "text-cleaner" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for TextCleaner {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Middle }
    /// async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Remove excessive whitespace
    ///     result.content = result
    ///         .content
    ///         .split_whitespace()
    ///         .collect::<Vec<_>>()
    ///         .join(" ");
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()>;

    /// Get the processing stage for this post-processor.
    ///
    /// Determines when this processor runs in the pipeline.
    ///
    /// # Returns
    ///
    /// The `ProcessingStage` (Early, Middle, or Late).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct MyProcessor;
    /// # impl Plugin for MyProcessor {
    /// #     fn name(&self) -> &str { "my-processor" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for MyProcessor {
    /// #     async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// fn processing_stage(&self) -> ProcessingStage {
    ///     ProcessingStage::Early  // Run before other processors
    /// }
    /// # }
    /// ```
    fn processing_stage(&self) -> ProcessingStage;

    /// Optional: Check if this processor should run for a given result.
    ///
    /// Allows conditional processing based on MIME type, metadata, or content.
    /// Defaults to `true` (always run).
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to check
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// `true` if the processor should run, `false` to skip.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct PdfOnlyProcessor;
    /// # impl Plugin for PdfOnlyProcessor {
    /// #     fn name(&self) -> &str { "pdf-only" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for PdfOnlyProcessor {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Middle }
    /// #     async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// /// Only process PDF documents
    /// fn should_process(&self, result: &ExtractionResult, config: &ExtractionConfig) -> bool {
    ///     result.mime_type == "application/pdf"
    /// }
    /// # }
    /// ```
    fn should_process(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        true
    }

    /// Optional: Estimate processing time in milliseconds.
    ///
    /// Used for logging and debugging. Defaults to 0 (unknown).
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to estimate for
    ///
    /// # Returns
    ///
    /// Estimated processing time in milliseconds.
    fn estimated_duration_ms(&self, _result: &ExtractionResult) -> u64 {
        0
    }

    /// Execution priority within the processing stage.
    ///
    /// Higher values run first within the same `ProcessingStage`. Defaults to 50.
    /// Use 0-49 for fallback processors, 50 for normal processors, and 51-255
    /// for high-priority processors that should run early in their stage.
    fn priority(&self) -> i32 {
        50
    }
}
