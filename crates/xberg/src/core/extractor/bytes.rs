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
use crate::types::ExtractedDocument;

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
/// An `ExtractedDocument` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns `XbergError::Validation` if MIME type is invalid.
/// Returns `XbergError::UnsupportedFormat` if MIME type is not supported.
///
/// # Example
///
/// This function is crate-internal; the public entry point that reaches it is
/// [`crate::extract`] with a bytes input.
///
/// ```rust,no_run
/// use xberg::{ExtractInput, ExtractionConfig, extract};
///
/// # async fn example() -> xberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let input = ExtractInput::from_bytes(b"Hello, world!".to_vec(), "text/plain", None);
/// let output = extract(input, &config).await?;
/// println!("Content: {}", output.results[0].content);
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
) -> Result<ExtractedDocument> {
    extract_bytes_with_layout_pages(content, mime_type, config, None).await
}

pub(crate) async fn extract_bytes_with_layout_pages(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
    pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
) -> Result<ExtractedDocument> {
    // `token.cancel()` below needs a token to signal, but `config.cancel_token` is
    // `None` on every binding-driven and CLI-driven call (see
    // `ExtractionConfig::ensure_cancel_token`) — install an internal fallback so a
    // timeout actually stops the extraction instead of merely returning
    // `Err(Timeout)` while the spawned work keeps running. A caller-supplied token
    // is always left untouched. Gated identically to the timeout block below: on
    // wasm32 / without `tokio-runtime` there is no timeout to enforce, so no token
    // is ever needed.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let owned_config_with_cancel_token;
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let config: &ExtractionConfig = if config.extraction_timeout_secs.is_some() {
        let mut owned = config.clone();
        owned.ensure_cancel_token();
        // Each timed extraction gets its own soft-stop state, so a caller's token that is
        // reused for another extraction never starts out stopped.
        owned.cancel_token = owned
            .cancel_token
            .as_ref()
            .map(crate::cancellation::CancellationToken::scoped);
        super::time_limit::arm(
            owned.cancel_token.as_ref(),
            owned.extraction_timeout_secs.unwrap_or_default(),
        );
        owned_config_with_cancel_token = owned;
        &owned_config_with_cancel_token
    } else {
        config
    };

    let extraction_future = Box::pin(run_byte_extraction(content, mime_type, config, pages));
    // Type-erase before adding the OCR scope to bound the Send proof for native batches. ~keep
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let extraction_future: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ExtractedDocument>> + Send + '_>,
    > = extraction_future;
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let extraction_future = Box::pin(super::cacheability::with_ocr_cancellation(
        config.cancel_token.as_ref(),
        extraction_future,
    ));

    // without a JS/WASI shim), which aborts the whole module with an uncatchable
    // `unreachable` trap. `tokio-runtime` can be enabled transitively on that target
    // (e.g. by `layout-tract` inside `wasm-target`), so gating on the feature alone is
    // not enough — explicitly exclude wasm32 here, matching `run_timed_extraction` in
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let result = if let Some(secs) = config.extraction_timeout_secs {
        let started = std::time::Instant::now();
        let mut extraction_future = extraction_future;
        match tokio::time::timeout(std::time::Duration::from_secs(secs), &mut extraction_future).await {
            Ok(inner) => inner,
            Err(_elapsed) => match super::time_limit::begin_grace(config.cancel_token.as_ref(), secs, started) {
                Ok(token) => {
                    let outcome =
                        tokio::time::timeout(super::time_limit::PARTIAL_RESULT_GRACE, &mut extraction_future).await;
                    super::time_limit::finish_grace(outcome, token, secs, started)
                }
                Err(timeout) => Err(timeout),
            },
        }
    } else {
        extraction_future.await
    };

    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    let result = {
        // Without a usable tokio timer (no 'tokio-runtime' feature, or the WASM build,
        // where `std::time::Instant::now()` panics) there is no timer to enforce a
        // timeout, but the default ExtractionConfig sets extraction_timeout_secs, so
        // erroring here would reject every default call. Ignore the unenforceable
        // limit and run the extraction instead. ~keep
        if config.extraction_timeout_secs.is_some() {
            tracing::debug!(
                "extraction_timeout_secs is ignored on this target (no usable tokio timer); running without a timeout"
            );
        }
        extraction_future.await
    };

    #[cfg(feature = "otel")]
    if let Err(ref e) = result {
        crate::telemetry::spans::record_error_on_current_span(e);
    }

    result
}

/// Resolve the MIME type extraction should run under.
///
/// A caller-supplied `application/octet-stream` carries no information, so the
/// bytes are sniffed instead; anything else is validated as given.
fn resolve_validated_mime(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<String> {
    use crate::core::mime;

    let validated_mime = if mime::is_generic_mime_type(mime_type) {
        #[cfg(feature = "tree-sitter")]
        {
            if config.tree_sitter.is_some() {
                if let Ok(text) = std::str::from_utf8(content) {
                    let trimmed = text.trim_start();
                    if tree_sitter_language_pack::detect_language_from_content(trimmed).is_some() {
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

    Ok(validated_mime)
}

/// Validate the configuration, resolve the MIME type, and run the extractor chain.
///
/// Split out of [`extract_bytes`] so the timeout and cancellation wrapper there stays
/// readable; the returned future is still only polled at the original await point.
async fn run_byte_extraction(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
    pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
) -> Result<ExtractedDocument> {
    if config.force_ocr && config.effective_disable_ocr() {
        return Err(crate::XbergError::Validation {
            message: "force_ocr and disable_ocr cannot both be true".to_string(),
            source: None,
        });
    }

    if matches!(
        config.ocr_strategy,
        crate::core::config::OcrStrategy::ScannedPages { .. }
    ) && config.effective_disable_ocr()
    {
        return Err(crate::XbergError::Validation {
            message: "ocr_strategy selects scanned pages for OCR, but disable_ocr is true".to_string(),
            source: None,
        });
    }

    let validated_mime = resolve_validated_mime(content, mime_type, config)?;

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

    #[cfg(feature = "office")]
    {
        let _ = LEGACY_WORD_MIME_TYPE;
        let _ = LEGACY_POWERPOINT_MIME_TYPE;
    }

    if pages.is_some() && validated_mime != "application/pdf" {
        return Err(crate::XbergError::validation(
            "explicit layout-page selection accepts PDF input only".to_string(),
        ));
    }
    Box::pin(super::file::extract_bytes_with_extractor_and_layout_pages(
        content,
        &validated_mime,
        config,
        pages,
    ))
    .await
}

#[cfg(test)]
mod declared_type_tests;
