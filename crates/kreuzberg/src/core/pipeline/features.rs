//! Feature processing logic.
//!
//! This module handles feature-specific processing like chunking,
//! embedding generation, and language detection.

use crate::Result;
use crate::core::config::ExtractionConfig;
#[cfg(feature = "chunking")]
use crate::types::PageBoundary;
use crate::types::{ExtractionResult, ProcessingWarning};
use std::borrow::Cow;

/// Recompute page boundaries against the rendered `content` string.
///
/// `PageBoundary` offsets produced during extraction are computed against raw
/// pdfium/source text, but `result.content` is produced by `render_plain` which
/// may have different byte lengths (e.g. normalised whitespace, Unicode
/// normalisation, dropped control characters).  This function re-derives the
/// boundaries by locating each page's rendered content inside the combined
/// `content` string, so that the byte offsets passed to the chunker are valid
/// indices into `result.content`.
///
/// Pages whose content cannot be found are silently skipped (the chunker will
/// still produce output, just without page-range metadata for those pages).
#[cfg(feature = "chunking")]
fn recompute_boundaries_from_pages(content: &str, pages: &[crate::types::PageContent]) -> Vec<PageBoundary> {
    let mut boundaries = Vec::with_capacity(pages.len());
    let mut search_offset = 0usize;

    for page in pages {
        if page.content.trim().is_empty() {
            boundaries.push(PageBoundary {
                page_number: page.page_number,
                byte_start: search_offset,
                byte_end: search_offset,
            });
            continue;
        }

        // Try exact match first
        if let Some(pos) = content[search_offset..].find(&page.content) {
            let byte_start = search_offset + pos;
            let byte_end = content.floor_char_boundary(byte_start + page.content.len());
            boundaries.push(PageBoundary {
                page_number: page.page_number,
                byte_start,
                byte_end,
            });
            search_offset = byte_end;
            continue;
        }

        // Fallback: search for first non-empty line of page content
        if let Some(line) = page.content.lines().find(|l| !l.trim().is_empty()).map(|l| l.trim())
            && let Some(pos) = content[search_offset..].find(line)
        {
            let byte_start = search_offset + pos;
            let raw_end = (byte_start + page.content.len()).min(content.len());
            let byte_end = content.floor_char_boundary(raw_end);
            boundaries.push(PageBoundary {
                page_number: page.page_number,
                byte_start,
                byte_end,
            });
            search_offset = byte_end;
            continue;
        }

        // Last resort: skip this page
        tracing::debug!(
            page = page.page_number,
            "Could not locate page content in rendered text — skipping page boundary"
        );
    }

    boundaries
}

/// Map TSLP `CodeChunk`s directly to kreuzberg `Chunk`s, bypassing text-splitter.
///
/// When the extraction result contains code intelligence with non-empty chunks,
/// those chunks already represent semantically meaningful code boundaries produced
/// by tree-sitter. Using text-splitter would break these boundaries.
#[cfg(feature = "tree-sitter")]
fn try_code_chunks(result: &ExtractionResult) -> Option<Vec<crate::types::extraction::Chunk>> {
    use crate::types::extraction::{Chunk, ChunkMetadata, ChunkType, HeadingContext, HeadingLevel};

    let code_chunks = match &result.metadata.format {
        Some(crate::types::metadata::FormatMetadata::Code(pr)) if !pr.chunks.is_empty() => &pr.chunks,
        _ => return None,
    };

    let total_chunks = code_chunks.len();
    let chunks: Vec<Chunk> = code_chunks
        .iter()
        .enumerate()
        .map(|(i, cc)| {
            // All code chunks are classified as CodeBlock regardless of node type.
            let chunk_type = ChunkType::CodeBlock;

            // Build heading context from context_path.
            let heading_context = if cc.metadata.context_path.is_empty() {
                None
            } else {
                Some(HeadingContext {
                    headings: cc
                        .metadata
                        .context_path
                        .iter()
                        .enumerate()
                        .map(|(depth, name)| HeadingLevel {
                            level: (depth as u8).saturating_add(2).min(6),
                            text: name.clone(),
                        })
                        .collect(),
                })
            };

            Chunk {
                content: cc.content.clone(),
                chunk_type,
                embedding: None,
                metadata: ChunkMetadata {
                    byte_start: cc.start_byte,
                    byte_end: cc.end_byte,
                    token_count: None,
                    chunk_index: i,
                    total_chunks,
                    first_page: None,
                    last_page: None,
                    heading_context,
                },
            }
        })
        .collect();

    Some(chunks)
}

/// Execute chunking if configured.
pub(super) fn execute_chunking(result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
    #[cfg(feature = "chunking")]
    if let Some(ref chunking_config) = config.chunking {
        // For code extractions with TSLP chunks, bypass text-splitter and map directly.
        #[cfg(feature = "tree-sitter")]
        if let Some(code_chunks) = try_code_chunks(result) {
            result.chunks = Some(code_chunks);

            let resolved_config = chunking_config.resolve_preset();
            #[cfg(feature = "embeddings")]
            if let Some(ref embedding_config) = resolved_config.embedding
                && let Some(ref mut chunks) = result.chunks
                && let Err(e) = crate::embeddings::generate_embeddings_for_chunks(chunks, embedding_config)
            {
                tracing::warn!("Embedding generation failed: {e}. Check that ONNX Runtime is installed.");
                result.processing_warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("embedding"),
                    message: Cow::Owned(e.to_string()),
                });
            }

            #[cfg(not(feature = "embeddings"))]
            if resolved_config.embedding.is_some() {
                tracing::warn!(
                    "Embedding config provided but embeddings feature is not enabled. Recompile with --features embeddings."
                );
                result.processing_warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("embedding"),
                    message: Cow::Borrowed("Embeddings feature not enabled"),
                });
            }

            return Ok(());
        }

        let resolved_config = chunking_config.resolve_preset();
        let chunking_config = &resolved_config;

        // Recompute page boundaries against `result.content` (rendered by `render_plain`)
        // if per-page content is available.  The boundaries stored in
        // `result.metadata.pages.boundaries` were computed against the raw extractor text
        // and may have different byte offsets than the rendered content (fix for #636).
        let recomputed_boundaries: Option<Vec<PageBoundary>> = result
            .pages
            .as_deref()
            .map(|pages| recompute_boundaries_from_pages(&result.content, pages));

        let page_boundaries: Option<&[PageBoundary]> = recomputed_boundaries
            .as_deref()
            .or_else(|| result.metadata.pages.as_ref().and_then(|ps| ps.boundaries.as_deref()));

        // Pass formatted_content (markdown) for heading context resolution when available.
        // Plain-text rendering strips heading markers, but the markdown chunker needs them
        // to build the heading hierarchy for chunk metadata.
        let heading_source = result.formatted_content.as_deref();
        match crate::chunking::chunk_text_with_heading_source(
            &result.content,
            chunking_config,
            page_boundaries,
            heading_source,
        ) {
            Ok(chunking_result) => {
                result.chunks = Some(chunking_result.chunks);

                #[cfg(feature = "embeddings")]
                if let Some(ref embedding_config) = chunking_config.embedding
                    && let Some(ref mut chunks) = result.chunks
                    && let Err(e) = crate::embeddings::generate_embeddings_for_chunks(chunks, embedding_config)
                {
                    tracing::warn!("Embedding generation failed: {e}. Check that ONNX Runtime is installed.");
                    result.processing_warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("embedding"),
                        message: Cow::Owned(e.to_string()),
                    });
                }

                #[cfg(not(feature = "embeddings"))]
                if chunking_config.embedding.is_some() {
                    tracing::warn!(
                        "Embedding config provided but embeddings feature is not enabled. Recompile with --features embeddings."
                    );
                    result.processing_warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("embedding"),
                        message: Cow::Borrowed("Embeddings feature not enabled"),
                    });
                }
            }
            Err(e) => {
                result.processing_warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("chunking"),
                    message: Cow::Owned(e.to_string()),
                });
            }
        }
    }

    #[cfg(not(feature = "chunking"))]
    if config.chunking.is_some() {
        result.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("chunking"),
            message: Cow::Borrowed("Chunking feature not enabled"),
        });
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
                result.processing_warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("language_detection"),
                    message: Cow::Owned(e.to_string()),
                });
            }
        }
    }

    #[cfg(not(feature = "language-detection"))]
    if config.language_detection.is_some() {
        result.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("language_detection"),
            message: Cow::Borrowed("Language detection feature not enabled"),
        });
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
                    result.processing_warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("token_reduction"),
                        message: Cow::Owned(e.to_string()),
                    });
                }
            }
        }
    }

    #[cfg(not(feature = "quality"))]
    if config.token_reduction.is_some() {
        result.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("token_reduction"),
            message: Cow::Borrowed("Token reduction requires the quality feature"),
        });
    }

    Ok(())
}
