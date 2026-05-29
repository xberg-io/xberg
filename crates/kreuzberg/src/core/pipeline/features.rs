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
/// rendered/source text, but `result.content` is produced by `render_plain` which
/// trims trailing whitespace from each paragraph.  The raw page text therefore has
/// different byte lengths for pages that contain trailing-space artifacts from PDF
/// rendering.  This function re-derives the boundaries by locating each page's
/// **paragraph-normalised** content (each `"\n\n"`-separated segment trimmed, then
/// re-joined) inside the combined `content` string, so that the byte offsets passed
/// to the chunker are valid indices into `result.content`.
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

        // Normalise page content to match what render_plain produces: split on the
        // paragraph separator, trim each segment (PDF pages often carry trailing
        // spaces before "\n\n" that render_plain strips via paragraph.trim()), then
        // re-join.  Using the normalised form means exact-match succeeds and the
        // resulting byte_end is correct — avoiding cascading search_offset
        // over-advance that would push past subsequent pages.
        let normalized: String = page
            .content
            .split("\n\n")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        // Try normalised-exact match (primary path — handles trailing-space pages).
        if let Some(pos) = content[search_offset..].find(normalized.as_str()) {
            let byte_start = search_offset + pos;
            let byte_end = content.floor_char_boundary(byte_start + normalized.len());
            boundaries.push(PageBoundary {
                page_number: page.page_number,
                byte_start,
                byte_end,
            });
            search_offset = byte_end;
            continue;
        }

        // Fallback: search for first non-empty line of page content.
        // Use normalized.len() for byte_end so search_offset advances correctly.
        if let Some(line) = page.content.lines().find(|l| !l.trim().is_empty()).map(|l| l.trim())
            && let Some(pos) = content[search_offset..].find(line)
        {
            let byte_start = search_offset + pos;
            let raw_end = (byte_start + normalized.len()).min(content.len());
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
                    image_indices: Vec::new(),
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
        // and may have different byte offsets than the rendered content.
        let recomputed_boundaries: Option<Vec<PageBoundary>> = result
            .pages
            .as_deref()
            .map(|pages| recompute_boundaries_from_pages(&result.content, pages));

        let page_boundaries: Option<&[PageBoundary]> = recomputed_boundaries
            .as_deref()
            .filter(|s| !s.is_empty())
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

                // Populate image_indices on each chunk: collect indices of images whose
                // page_number falls within the chunk's [first_page, last_page] range.
                if let Some(ref images) = result.images
                    && let Some(ref mut chunks) = result.chunks
                {
                    for chunk in chunks.iter_mut() {
                        if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
                            chunk.metadata.image_indices = images
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, img)| {
                                    let pg = img.page_number?;
                                    (pg >= first && pg <= last).then_some(idx as u32)
                                })
                                .collect();
                        }
                    }
                }

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

#[cfg(all(test, feature = "chunking"))]
mod tests {
    use super::*;
    use crate::types::PageContent;

    fn make_page(page_number: u32, content: impl Into<String>) -> PageContent {
        PageContent {
            page_number,
            content: content.into(),
            tables: vec![],
            image_indices: vec![],
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            section_name: None,
            speaker_notes: None,
        }
    }

    // When PageContent.content matches result.content exactly, all boundaries succeed.
    #[test]
    fn recompute_boundaries_exact_match_produces_full_boundary_set() {
        let p1 = "Hello world";
        let p2 = "Second page text";
        let p3 = "Third page here";
        let content = format!("{p1}\n\n{p2}\n\n{p3}");

        let pages = vec![make_page(1, p1), make_page(2, p2), make_page(3, p3)];
        let boundaries = recompute_boundaries_from_pages(&content, &pages);

        assert_eq!(boundaries.len(), 3, "all pages should resolve to boundaries");
        assert_eq!(&content[boundaries[0].byte_start..boundaries[0].byte_end], p1);
        assert_eq!(&content[boundaries[1].byte_start..boundaries[1].byte_end], p2);
        assert_eq!(&content[boundaries[2].byte_start..boundaries[2].byte_end], p3);
    }

    // When PageContent.content is raw (control char present) but result.content has the
    // cleaned version, the affected page is silently skipped — leaving fewer boundaries
    // than pages. Documents the pre-fix failure mode.
    #[test]
    fn recompute_boundaries_raw_content_causes_skipped_pages() {
        // U+0001 between word chars → fix_pdf_control_chars replaces with '-'
        let p1_clean = "Hello world";
        let p2_raw = "ab\x01cd"; // raw page text — control char present
        let p2_clean = "ab-cd"; // what result.content contains after cleanup
        let p3_clean = "Third page";
        let content = format!("{p1_clean}\n\n{p2_clean}\n\n{p3_clean}");

        // Pre-fix scenario: page.content = raw, result.content = cleaned → mismatch
        let pages = vec![
            make_page(1, p1_clean),
            make_page(2, p2_raw), // intentionally stale raw content
            make_page(3, p3_clean),
        ];
        let boundaries = recompute_boundaries_from_pages(&content, &pages);

        // Page 2 is skipped: neither exact nor first-line search finds "ab\x01cd"
        // inside content (which has "ab-cd"). Only pages 1 and 3 resolve.
        assert_eq!(boundaries.len(), 2, "page with raw/cleaned mismatch should be skipped");
        assert_eq!(boundaries[0].page_number, 1);
        assert_eq!(boundaries[1].page_number, 3);
    }

    // When PageContent.content is the cleaned text (the fix), all pages resolve.
    #[test]
    fn recompute_boundaries_cleaned_content_resolves_all_pages() {
        let p1_clean = "Hello world";
        let p2_clean = "ab-cd"; // cleaned — matches result.content exactly
        let p3_clean = "Third page";
        let content = format!("{p1_clean}\n\n{p2_clean}\n\n{p3_clean}");

        // Post-fix scenario: page.content = cleaned, result.content = cleaned → exact match
        let pages = vec![make_page(1, p1_clean), make_page(2, p2_clean), make_page(3, p3_clean)];
        let boundaries = recompute_boundaries_from_pages(&content, &pages);

        assert_eq!(boundaries.len(), 3, "all pages should resolve after fix");
        assert_eq!(&content[boundaries[1].byte_start..boundaries[1].byte_end], p2_clean);
    }

    // PDF pages often have trailing spaces before "\n\n" paragraph separators (PDF
    // rendering artifact).  render_plain trims each paragraph via paragraph.trim(),
    // so result.content lacks those trailing spaces while page.content retains them.
    // The normalised-exact match must succeed and produce correct byte_end so that
    // subsequent pages are found without cascading search_offset over-advance.
    #[test]
    fn recompute_boundaries_trailing_space_pages_all_resolve() {
        // Simulate PDF page content with trailing spaces before "\n\n".
        let p1_raw = "Heading \n\nBody paragraph one. ";
        let p2_raw = "Second heading \n\nBody paragraph two. ";
        let p3_raw = "Conclusion. ";

        // result.content as render_plain produces it (each paragraph trimmed).
        let p1_norm = "Heading\n\nBody paragraph one.";
        let p2_norm = "Second heading\n\nBody paragraph two.";
        let p3_norm = "Conclusion.";
        let content = format!("{p1_norm}\n\n{p2_norm}\n\n{p3_norm}");

        let pages = vec![make_page(1, p1_raw), make_page(2, p2_raw), make_page(3, p3_raw)];
        let boundaries = recompute_boundaries_from_pages(&content, &pages);

        assert_eq!(boundaries.len(), 3, "all pages must resolve despite trailing spaces");
        assert_eq!(&content[boundaries[0].byte_start..boundaries[0].byte_end], p1_norm);
        assert_eq!(&content[boundaries[1].byte_start..boundaries[1].byte_end], p2_norm);
        assert_eq!(&content[boundaries[2].byte_start..boundaries[2].byte_end], p3_norm);
    }
}
