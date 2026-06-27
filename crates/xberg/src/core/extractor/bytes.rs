//! Byte array extraction operations.
//!
//! This module handles extraction from in-memory byte arrays, including:
//! - MIME type validation
//! - Legacy format conversion (DOC, PPT)
//! - Extraction pipeline orchestration

use crate::Result;
#[cfg(not(feature = "office"))]
use crate::XbergError;
use crate::core::config::ExtractionConfig;
use crate::core::mime::{LEGACY_POWERPOINT_MIME_TYPE, LEGACY_WORD_MIME_TYPE};
use crate::types::ExtractionResult;

use super::file::extract_bytes_with_extractor;

/// Extract content from a byte array.
///
/// This is the main entry point for in-memory extraction. It performs the following steps:
/// 1. Validate MIME type
/// 2. Handle legacy format conversion if needed
/// 3. Select appropriate extractor from registry
/// 4. Extract content
/// 5. Run post-processing pipeline
///
/// # Arguments
///
/// * `content` - The byte array to extract
/// * `mime_type` - MIME type of the content
/// * `config` - Extraction configuration
///
/// # Returns
///
/// An `ExtractionResult` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns `XbergError::Validation` if MIME type is invalid.
/// Returns `XbergError::UnsupportedFormat` if MIME type is not supported.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::core::extractor::extract_bytes;
/// use xberg::core::config::ExtractionConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let bytes = b"Hello, world!";
/// let result = extract_bytes(bytes, "text/plain", &config).await?;
/// println!("Content: {}", result.content);
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, content),
    fields(
        { crate::telemetry::conventions::OPERATION } = crate::telemetry::conventions::operations::EXTRACT_BYTES,
        { crate::telemetry::conventions::DOCUMENT_MIME_TYPE } = mime_type,
        { crate::telemetry::conventions::DOCUMENT_SIZE_BYTES } = content.len(),
        { crate::telemetry::conventions::OTEL_STATUS_CODE } = tracing::field::Empty,
        { crate::telemetry::conventions::ERROR_TYPE } = tracing::field::Empty,
        { crate::telemetry::conventions::ERROR_MESSAGE } = tracing::field::Empty,
    )
))]
pub(crate) async fn extract_bytes(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    use crate::core::mime;

    let extraction_future = async {
        if config.force_ocr && config.effective_disable_ocr() {
            return Err(crate::XbergError::Validation {
                message: "force_ocr and disable_ocr cannot both be true".to_string(),
                source: None,
            });
        }

        let validated_mime = if mime_type == "application/octet-stream" {
            // When tree-sitter is configured, check if content is recognized source code.
            // This allows octet-stream files with tree-sitter config to be detected as code.
            #[cfg(feature = "tree-sitter")]
            {
                if config.tree_sitter.is_some() {
                    if let Ok(text) = std::str::from_utf8(content) {
                        let trimmed = text.trim_start();
                        if tree_sitter_language_pack::detect_language_from_content(trimmed).is_some() {
                            // Recognize as source code when tree-sitter can detect a language.
                            mime::SOURCE_CODE_MIME_TYPE.to_string()
                        } else {
                            mime::detect_mime_type_from_bytes(content)?
                        }
                    } else {
                        mime::detect_mime_type_from_bytes(content)?
                    }
                } else {
                    mime::detect_mime_type_from_bytes(content)?
                }
            }
            #[cfg(not(feature = "tree-sitter"))]
            {
                let _ = config;
                mime::detect_mime_type_from_bytes(content)?
            }
        } else {
            mime::validate_mime_type(mime_type)?
        };

        // Native DOC/PPT extractors are registered in the plugin registry.
        // When the office feature is disabled, these MIME types are unsupported.
        #[cfg(not(feature = "office"))]
        match validated_mime.as_str() {
            LEGACY_WORD_MIME_TYPE => {
                return Err(XbergError::UnsupportedFormat(
                    "Legacy Word extraction requires the `office` feature".to_string(),
                ));
            }
            LEGACY_POWERPOINT_MIME_TYPE => {
                return Err(XbergError::UnsupportedFormat(
                    "Legacy PowerPoint extraction requires the `office` feature".to_string(),
                ));
            }
            _ => {}
        }

        // Suppress unused import warnings when office feature is enabled
        #[cfg(feature = "office")]
        {
            let _ = LEGACY_WORD_MIME_TYPE;
            let _ = LEGACY_POWERPOINT_MIME_TYPE;
        }

        extract_bytes_with_extractor(content, &validated_mime, config).await
    };

    #[cfg(feature = "tokio-runtime")]
    let result = if let Some(secs) = config.extraction_timeout_secs {
        let start = std::time::Instant::now();
        match tokio::time::timeout(std::time::Duration::from_secs(secs), extraction_future).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                if let Some(ref token) = config.cancel_token {
                    token.cancel();
                }
                Err(crate::XbergError::Timeout {
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    limit_ms: secs * 1000,
                })
            }
        }
    } else {
        extraction_future.await
    };

    #[cfg(not(feature = "tokio-runtime"))]
    let result = {
        if config.extraction_timeout_secs.is_some() {
            return Err(crate::XbergError::Validation {
                message: "extraction_timeout_secs requires the 'tokio-runtime' feature to be enabled".to_string(),
                source: None,
            });
        }
        extraction_future.await
    };

    #[cfg(feature = "otel")]
    if let Err(ref e) = result {
        crate::telemetry::spans::record_error_on_current_span(e);
    }

    result
}
