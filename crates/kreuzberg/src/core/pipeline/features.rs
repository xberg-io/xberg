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
fn try_code_chunks(_result: &ExtractionResult) -> Option<Vec<crate::types::extraction::Chunk>> {
    // FormatMetadata::Code is a unit variant — the structured ProcessResult payload
    // is no longer attached. Code extractions fall back to standard text-based
    // chunking via the default pipeline.
    None
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

        // For non-plain output formats, re-derive page boundaries against formatted_content.
        // The plain-text boundaries above are byte-offset invalid for the formatted string
        // (e.g. markdown headings shift all subsequent offsets).  recompute_boundaries_from_pages
        // uses substring search, so the page text is still found verbatim inside the formatted
        // string and the returned offsets are valid indices into formatted_content.
        // Caveat: HTML output HTML-escapes special characters (&amp;, &lt;, etc.), so pages
        // whose content contains &, <, or > will not match and silently produce no provenance.
        let formatted_boundaries: Option<Vec<PageBoundary>> =
            if config.output_format != crate::core::config::OutputFormat::Plain {
                result.formatted_content.as_deref().and_then(|formatted| {
                    result
                        .pages
                        .as_deref()
                        .map(|pages| recompute_boundaries_from_pages(formatted, pages))
                })
            } else {
                None
            };

        // When a non-plain output format is requested, formatted_content holds the
        // pre-rendered output (markdown, HTML, etc.) that will become result.content after
        // apply_output_format.  Chunk it directly so chunks[].content carries the same
        // formatted representation as the top-level content field.
        //
        // When output_format == Plain, formatted_content may be temporarily set as a heading
        // source only (the chunker_only_markdown path in mod.rs).  In that case chunk the plain
        // content and pass formatted_content as heading_source so the markdown chunker can build
        // heading hierarchy without altering chunk content.
        let (chunk_input, effective_page_boundaries, heading_source) =
            if config.output_format != crate::core::config::OutputFormat::Plain {
                match result.formatted_content.as_deref() {
                    Some(formatted) => {
                        let fmt_boundaries = formatted_boundaries.as_deref().filter(|s| !s.is_empty());
                        (formatted, fmt_boundaries, None)
                    }
                    None => (result.content.as_str(), page_boundaries, None),
                }
            } else {
                (
                    result.content.as_str(),
                    page_boundaries,
                    result.formatted_content.as_deref(),
                )
            };

        match crate::chunking::chunk_text_with_heading_source(
            chunk_input,
            chunking_config,
            effective_page_boundaries,
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
            sheet_name: None,
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

    // --- Issue #1073: chunk content must match output_format ---

    fn make_result_with_formatted(plain: &str, formatted: &str) -> ExtractionResult {
        ExtractionResult {
            content: plain.to_string(),
            formatted_content: Some(formatted.to_string()),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        }
    }

    fn make_result_with_pages_and_formatted(
        plain: &str,
        formatted: &str,
        pages: Vec<crate::types::PageContent>,
    ) -> ExtractionResult {
        ExtractionResult {
            content: plain.to_string(),
            formatted_content: Some(formatted.to_string()),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        }
    }

    fn markdown_chunking_config() -> crate::core::config::ExtractionConfig {
        crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Markdown,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn chunks_content_is_markdown_when_output_format_is_markdown() {
        let plain = "SH-001 Luca Bianchi Common Germany 3500000\nSH-002 Jeni Doe Common Singapore 2800000";
        let markdown = "| SH-001 | Luca Bianchi | Common | Germany | 3,500,000 |\n\
                        | SH-002 | Jeni Doe | Common | Singapore | 2,800,000 |";

        let config = markdown_chunking_config();
        let mut result = make_result_with_formatted(plain, markdown);

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(
                chunk.content.contains('|'),
                "chunk content must be markdown (contain '|'), got: {:?}",
                chunk.content
            );
        }
        // Plain space-separated form must not appear
        for chunk in &chunks {
            assert!(
                !chunk.content.starts_with("SH-001 Luca"),
                "chunk content must not be plain text, got: {:?}",
                chunk.content
            );
        }
        // formatted_content must not be consumed (apply_output_format needs it)
        assert!(
            result.formatted_content.is_some(),
            "formatted_content must not be consumed by chunking"
        );
    }

    #[test]
    fn chunks_content_is_plain_when_output_format_is_plain() {
        // output_format=Plain with markdown chunker: chunks must stay plain text even when
        // formatted_content is temporarily set for heading-context (chunker_only_markdown path).
        let plain = "# Heading\n\nRow one content\nRow two content";
        let heading_source = "# Heading\n\nRow one content\nRow two content";

        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Plain,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Markdown,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain.to_string(),
            formatted_content: Some(heading_source.to_string()), // simulates chunker_only_markdown
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        // Content must come from plain, not re-formatted
        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_content.contains("Row one content") || all_content.contains("Heading"),
            "plain-mode chunks must contain source text, got: {:?}",
            all_content
        );
        // formatted_content must not be consumed
        assert!(
            result.formatted_content.is_some(),
            "Plain path must not consume formatted_content"
        );
    }

    #[test]
    fn chunks_content_matches_when_no_formatted_content_and_markdown_format() {
        // Edge: output_format=Markdown but formatted_content is None (e.g. structured extractor)
        // Chunker must fall back to result.content without panicking.
        let plain = "Some plain text without markdown pre-render";

        let config = markdown_chunking_config();
        let mut result = ExtractionResult {
            content: plain.to_string(),
            formatted_content: None,
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content, plain);
    }

    #[test]
    fn chunks_content_uses_formatted_content_for_djot_output_format() {
        // Djot goes through the same != Plain branch as Markdown.
        // Verify the branch is not accidentally Markdown-only.
        let plain = "row one data\nrow two data";
        let djot = "{row one | data}\n{row two | data}"; // synthetic djot-like formatting

        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Djot,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = make_result_with_formatted(plain, djot);

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        // Chunk content must come from the djot formatted string, not plain text
        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(
            all_content.contains('{'),
            "chunk content must use djot formatted_content, got: {:?}",
            all_content
        );
        assert!(
            !all_content.starts_with("row one data\nrow two"),
            "chunk content must not be plain text, got: {:?}",
            all_content
        );
    }

    #[test]
    fn chunk_page_metadata_is_none_when_pages_field_absent() {
        // Without result.pages populated there are no page-content substrings to locate
        // in formatted_content, so formatted_boundaries stays None and no page provenance
        // can be derived regardless of output_format.
        let plain = "Page one content\n\nPage two content";
        let markdown = "# Page one\n\nPage one content\n\n# Page two\n\nPage two content";

        let config = markdown_chunking_config();
        // result.pages is NOT set — formatted_boundaries will be None.
        let mut result = make_result_with_formatted(plain, markdown);

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(
                chunk.metadata.first_page.is_none(),
                "first_page must be None when result.pages is absent, got: {:?}",
                chunk.metadata.first_page
            );
        }
    }

    #[test]
    fn chunk_page_provenance_present_for_markdown_output_with_pages() {
        // Two-page document: page text appears verbatim inside the markdown formatted string.
        // recompute_boundaries_from_pages must locate those substrings within formatted_content
        // so that chunks carry valid first_page / last_page metadata.
        let p1 = "Introduction text for page one";
        let p2 = "Conclusion text for page two";
        let plain = format!("{p1}\n\n{p2}");
        let markdown = format!("# Introduction\n\n{p1}\n\n# Conclusion\n\n{p2}");

        let pages = vec![make_page(1, p1), make_page(2, p2)];
        let config = markdown_chunking_config();
        let mut result = make_result_with_pages_and_formatted(&plain, &markdown, pages);

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty(), "chunks must be non-empty");
        let has_provenance = chunks.iter().any(|c| c.metadata.first_page.is_some());
        assert!(
            has_provenance,
            "at least one chunk must carry first_page when result.pages is populated and output_format=Markdown"
        );
    }

    #[test]
    fn chunk_page_provenance_single_page_markdown_output() {
        // Single-page document: result.chunks must be Some([...]) (not null) and the chunk
        // must carry first_page = Some(1) when result.pages is populated.
        let p1 = "Single page content for the document";
        let markdown = format!("# Document\n\n{p1}");

        let pages = vec![make_page(1, p1)];
        let config = markdown_chunking_config();
        let mut result = ExtractionResult {
            content: p1.to_string(),
            formatted_content: Some(markdown),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result
            .chunks
            .expect("chunks must be Some(...) — not null — for single-page with chunking configured");
        assert!(!chunks.is_empty(), "chunks must be non-empty when content is present");
        let has_page_one = chunks.iter().any(|c| c.metadata.first_page == Some(1));
        assert!(
            has_page_one,
            "single-page chunk must have first_page = Some(1) for markdown output, got: {:?}",
            chunks.iter().map(|c| c.metadata.first_page).collect::<Vec<_>>()
        );
    }

    #[test]
    fn chunk_page_provenance_plain_output_unaffected_by_formatted_boundaries() {
        // Plain output path must still derive boundaries from result.pages against
        // result.content — not from formatted_content.
        let p1 = "First page text";
        let p2 = "Second page text";
        let plain = format!("{p1}\n\n{p2}");
        let heading_source = format!("# Doc\n\n{p1}\n\n# End\n\n{p2}");

        let pages = vec![make_page(1, p1), make_page(2, p2)];
        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Plain,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Markdown,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain.clone(),
            formatted_content: Some(heading_source),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        // Plain path: chunk content comes from result.content, not formatted_content
        for chunk in &chunks {
            assert!(
                !chunk.content.contains("# Doc"),
                "plain-output chunks must not contain markdown heading syntax"
            );
        }
        // Boundaries still attributed via plain-content recomputation
        let has_provenance = chunks.iter().any(|c| c.metadata.first_page.is_some());
        assert!(
            has_provenance,
            "plain-output chunks must carry page provenance when result.pages is set"
        );
    }

    #[test]
    fn chunk_page_provenance_html_output_ascii_content() {
        // OutputFormat::Html with plain-ASCII page text: page text appears verbatim inside
        // the HTML string (no HTML-escape transformation needed), so provenance succeeds.
        let p1 = "Introduction section content";
        let p2 = "Conclusion section content";
        let plain = format!("{p1}\n\n{p2}");
        // Simulate what render_html produces: paragraphs wrapped in <p> tags.
        let html = format!("<p>{p1}</p>\n<p>{p2}</p>");

        let pages = vec![make_page(1, p1), make_page(2, p2)];
        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Html,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain,
            formatted_content: Some(html),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated for HTML output");
        assert!(!chunks.is_empty());
        let has_provenance = chunks.iter().any(|c| c.metadata.first_page.is_some());
        assert!(
            has_provenance,
            "HTML output with ASCII page text must carry page provenance; got: {:?}",
            chunks.iter().map(|c| c.metadata.first_page).collect::<Vec<_>>()
        );
    }

    #[test]
    fn chunk_page_provenance_html_output_degrades_silently_for_html_special_chars() {
        // HTML output HTML-escapes &, <, >: page text "AT&T" becomes "AT&amp;T" in the
        // formatted string.  The verbatim substring search misses it, so that page produces
        // no provenance.  This is a known limitation; the test documents it so a future fix
        // cannot regress the behaviour silently.
        let p1_raw = "AT&T quarterly report";
        let plain = p1_raw.to_string();
        // render_html escapes & → &amp;
        let html = "<p>AT&amp;T quarterly report</p>".to_string();

        let pages = vec![make_page(1, p1_raw)];
        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Html,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain,
            formatted_content: Some(html),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must still be produced");
        assert!(!chunks.is_empty());
        // Page text was not found due to HTML escaping — provenance silently absent.
        for chunk in &chunks {
            assert!(
                chunk.metadata.first_page.is_none(),
                "HTML-escaped page text must produce no provenance (known limitation), got: {:?}",
                chunk.metadata.first_page
            );
        }
    }

    #[test]
    fn chunk_page_provenance_djot_output_with_pages() {
        // Djot output travels the same non-Plain branch as Markdown.
        // Verify provenance is populated when page text appears verbatim in djot content.
        let p1 = "Djot page one text";
        let p2 = "Djot page two text";
        let plain = format!("{p1}\n\n{p2}");
        // Synthetic djot: headings use `#` like markdown; body text is unchanged.
        let djot = format!("# Section One\n\n{p1}\n\n# Section Two\n\n{p2}");

        let pages = vec![make_page(1, p1), make_page(2, p2)];
        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Djot,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain,
            formatted_content: Some(djot),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated for Djot output");
        assert!(!chunks.is_empty());
        let has_provenance = chunks.iter().any(|c| c.metadata.first_page.is_some());
        assert!(
            has_provenance,
            "Djot output must carry page provenance when result.pages is populated"
        );
    }

    #[test]
    fn chunk_page_provenance_multi_chunk_single_page() {
        // When one page produces multiple chunks (content > max_characters), every
        // attributed chunk must have first_page == last_page == 1.
        let p1 = "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi";
        let plain = p1.to_string();
        let markdown = format!("# Doc\n\n{p1}");

        let pages = vec![make_page(1, p1)];
        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            chunking: Some(crate::core::config::ChunkingConfig {
                // Small cap forces multiple chunks from the single page
                max_characters: 20,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: plain,
            formatted_content: Some(markdown),
            pages: Some(pages),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(chunks.len() > 1, "small cap must produce multiple chunks");
        for chunk in &chunks {
            if chunk.metadata.first_page.is_some() {
                assert_eq!(
                    chunk.metadata.first_page,
                    Some(1),
                    "all chunks of a single-page document must have first_page = Some(1)"
                );
                assert_eq!(
                    chunk.metadata.last_page,
                    Some(1),
                    "all chunks of a single-page document must have last_page = Some(1)"
                );
            }
        }
        let attributed = chunks.iter().filter(|c| c.metadata.first_page.is_some()).count();
        assert!(attributed > 0, "at least one chunk must be attributed to page 1");
    }

    fn plain_chunking_config() -> crate::core::config::ExtractionConfig {
        crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Plain,
            chunking: Some(crate::core::config::ChunkingConfig {
                max_characters: 2000,
                overlap: 0,
                trim: true,
                chunker_type: crate::chunking::ChunkerType::Text,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn chunk_page_provenance_single_page_plain_output() {
        // Regression lock: plain branch must pass page_boundaries to the chunker even for
        // single-page documents.  Do not gate this on "more than one page".
        let p1 = "Single page plain text content for the document";
        let config = plain_chunking_config();
        let mut result = ExtractionResult {
            content: p1.to_string(),
            pages: Some(vec![make_page(1, p1)]),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be Some for plain single-page");
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert_eq!(chunk.metadata.first_page, Some(1));
            assert_eq!(chunk.metadata.last_page, Some(1));
        }
    }

    #[test]
    fn chunk_page_provenance_single_page_plain_output_content_empty_produces_empty_chunks() {
        // Empty content (scanned page without OCR) must yield Some([]), not None.
        // chunks: null in the API always means chunking was not configured.
        let config = plain_chunking_config();
        let mut result = ExtractionResult {
            content: String::new(),
            pages: Some(vec![make_page(1, "")]),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result
            .chunks
            .expect("chunks must be Some([]) not None for empty content");
        assert!(chunks.is_empty());
    }
}
