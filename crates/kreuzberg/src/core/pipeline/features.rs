//! Feature processing logic.
//!
//! This module handles feature-specific processing like chunking,
//! embedding generation, and language detection.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::types::{ExtractionResult, ProcessingWarning};
use std::borrow::Cow;

/// Push a warning and insert a matching metadata error entry in one call.
///
/// Avoids the repeated three-step pattern of converting the error to a `String`,
/// pushing a `ProcessingWarning`, and inserting a `serde_json::Value::String` into
/// `result.metadata.additional`.
fn push_warning_and_meta(
    result: &mut ExtractionResult,
    source: &'static str,
    meta_key: &'static str,
    error: impl std::fmt::Display,
) {
    let error_msg = error.to_string();
    result.processing_warnings.push(ProcessingWarning {
        source: Cow::Borrowed(source),
        message: Cow::Owned(error_msg.clone()),
    });
    result
        .metadata
        .additional
        .insert(Cow::Borrowed(meta_key), serde_json::Value::String(error_msg));
}

/// Execute chunking if configured.
pub(super) fn execute_chunking(result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
    #[cfg(feature = "chunking")]
    if let Some(ref chunking_config) = config.chunking {
        let resolved_config = chunking_config.resolve_preset();
        let chunking_config = &resolved_config;
        let page_boundaries = result.metadata.pages.as_ref().and_then(|ps| ps.boundaries.as_deref());

        match crate::chunking::chunk_text(&result.content, chunking_config, page_boundaries) {
            Ok(chunking_result) => {
                result.chunks = Some(chunking_result.chunks);

                if let Some(ref chunks) = result.chunks {
                    // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                    // chunk_count is derivable from result.chunks.len().
                    result.metadata.additional.insert(
                        Cow::Borrowed("chunk_count"),
                        serde_json::Value::Number(serde_json::Number::from(chunks.len())),
                    );
                }

                #[cfg(feature = "embeddings")]
                if let Some(ref embedding_config) = chunking_config.embedding
                    && let Some(ref mut chunks) = result.chunks
                {
                    match crate::embeddings::generate_embeddings_for_chunks(chunks, embedding_config) {
                        Ok(()) => {
                            // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                            // embeddings_generated is derivable from result.chunks having non-None embeddings.
                            result
                                .metadata
                                .additional
                                .insert(Cow::Borrowed("embeddings_generated"), serde_json::Value::Bool(true));
                        }
                        Err(e) => {
                            tracing::warn!("Embedding generation failed: {e}. Check that ONNX Runtime is installed.");
                            // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                            push_warning_and_meta(result, "embedding", "embedding_error", e);
                        }
                    }
                }

                #[cfg(not(feature = "embeddings"))]
                if chunking_config.embedding.is_some() {
                    tracing::warn!(
                        "Embedding config provided but embeddings feature is not enabled. Recompile with --features embeddings."
                    );
                    // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                    push_warning_and_meta(result, "embedding", "embedding_error", "Embeddings feature not enabled");
                }
            }
            Err(e) => {
                // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                push_warning_and_meta(result, "chunking", "chunking_error", e);
            }
        }
    }

    #[cfg(not(feature = "chunking"))]
    if config.chunking.is_some() {
        // DEPRECATED: kept for backward compatibility; will be removed in next major version.
        push_warning_and_meta(result, "chunking", "chunking_error", "Chunking feature not enabled");
    }

    Ok(())
}

/// Execute language detection if configured.
pub(super) fn execute_language_detection(result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
    #[cfg(feature = "language-detection")]
    if let Some(ref lang_config) = config.language_detection {
        match crate::language_detection::detect_languages(&result.content, lang_config) {
            Ok(detected) => {
                result.detected_languages = detected;
            }
            Err(e) => {
                // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                push_warning_and_meta(result, "language_detection", "language_detection_error", e);
            }
        }
    }

    #[cfg(not(feature = "language-detection"))]
    if config.language_detection.is_some() {
        // DEPRECATED: kept for backward compatibility; will be removed in next major version.
        push_warning_and_meta(
            result,
            "language_detection",
            "language_detection_error",
            "Language detection feature not enabled",
        );
    }

    Ok(())
}

/// Execute token reduction if configured.
pub(super) fn execute_token_reduction(result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
    #[cfg(feature = "quality")]
    if let Some(ref tr_config) = config.token_reduction {
        let level = crate::text::token_reduction::ReductionLevel::from(tr_config.mode.as_str());

        if !matches!(level, crate::text::token_reduction::ReductionLevel::Off) {
            let impl_config = crate::text::token_reduction::TokenReductionConfig {
                level,
                ..Default::default()
            };

            let lang_hint: Option<&str> = result
                .detected_languages
                .as_deref()
                .and_then(|langs| langs.first().map(|s| s.as_str()));

            match crate::text::token_reduction::reduce_tokens(&result.content, &impl_config, lang_hint) {
                Ok(reduced) => {
                    result.content = reduced;
                }
                Err(e) => {
                    push_warning_and_meta(result, "token_reduction", "token_reduction_error", e);
                }
            }
        }
    }

    #[cfg(not(feature = "quality"))]
    if config.token_reduction.is_some() {
        push_warning_and_meta(
            result,
            "token_reduction",
            "token_reduction_error",
            "Token reduction requires the quality feature",
        );
    }

    Ok(())
}
