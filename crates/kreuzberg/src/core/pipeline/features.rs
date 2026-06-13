//! Feature processing logic.
//!
//! This module handles feature-specific processing like chunking,
//! embedding generation, and language detection.

use crate::Result;
use crate::core::config::ExtractionConfig;
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

/// Recompute page boundaries and write them back to `result.metadata.pages.boundaries`.
///
/// Boundaries stored during extraction are computed against the raw native text.  For
/// scanned/rasterized PDFs that text is empty (no embedded text layer), so every
/// boundary is a degenerate zero-length span.  After OCR fills `result.pages` with
/// real content this function re-derives boundaries against `result.content` (the
/// fully rendered string) and stores them so the API response and chunker both see
/// correct byte offsets.
///
/// No-op when `result.pages` is `None` or `result.metadata.pages` is `None`.
pub(super) fn refresh_page_boundaries(result: &mut ExtractionResult) {
    let pages = match result.pages.as_deref() {
        Some(p) if !p.is_empty() => p,
        _ => return,
    };

    let recomputed = recompute_boundaries_from_pages(&result.content, pages);
    if recomputed.is_empty() {
        return;
    }

    if let Some(ref mut page_structure) = result.metadata.pages {
        page_structure.boundaries = Some(recomputed);
    }
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

        // refresh_page_boundaries has already recomputed and stored correct offsets;
        // use them directly for chunk page-range attribution.
        let page_boundaries: Option<&[PageBoundary]> =
            result.metadata.pages.as_ref().and_then(|ps| ps.boundaries.as_deref());

        // When a non-plain output format is requested, formatted_content holds the
        // pre-rendered output (markdown, HTML, etc.) that will become result.content after
        // apply_output_format.  Chunk it directly so chunks[].content carries the same
        // formatted representation as the top-level content field.
        //
        // Page boundaries were computed against plain-text content via recompute_boundaries_from_pages
        // and cannot be remapped to the formatted string without re-derivation; they are omitted
        // in the non-plain path (chunk page metadata is a follow-up improvement).
        //
        // When output_format == Plain, formatted_content may be temporarily set as a heading
        // source only (the chunker_only_markdown path in mod.rs).  In that case chunk the plain
        // content and pass formatted_content as heading_source so the markdown chunker can build
        // heading hierarchy without altering chunk content.
        let (chunk_input, effective_page_boundaries, heading_source) =
            if config.output_format != crate::core::config::OutputFormat::Plain {
                match result.formatted_content.as_deref() {
                    Some(formatted) => (formatted, None, None),
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

#[cfg(test)]
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

    // --- Issue #1095: degenerate page boundaries after OCR on scanned PDFs ---

    fn make_scanned_pdf_result(ocr_pages: &[(&str, u32)]) -> ExtractionResult {
        // Simulate what the pipeline produces for a scanned PDF:
        // - result.content = concatenated OCR text
        // - result.pages = per-page OCR content
        // - result.metadata.pages.boundaries = stale degenerate offsets (byte_start == byte_end)
        //   computed against the empty native text before OCR ran
        let content = ocr_pages.iter().map(|(t, _)| *t).collect::<Vec<_>>().join("\n\n");

        let pages: Vec<PageContent> = ocr_pages.iter().map(|(text, num)| make_page(*num, *text)).collect();

        // Degenerate boundaries: every page has byte_start == byte_end (scanner artifact)
        let stale_boundaries: Vec<PageBoundary> = ocr_pages
            .iter()
            .enumerate()
            .map(|(i, (_, num))| PageBoundary {
                page_number: *num,
                byte_start: i * 2, // separator-only offsets from empty native text
                byte_end: i * 2,
            })
            .collect();

        let page_structure = crate::types::PageStructure {
            total_count: ocr_pages.len() as u32,
            unit_type: crate::types::PageUnitType::Page,
            boundaries: Some(stale_boundaries),
            pages: None,
        };

        ExtractionResult {
            content,
            pages: Some(pages),
            metadata: crate::types::Metadata {
                pages: Some(page_structure),
                ..Default::default()
            },
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        }
    }

    #[test]
    fn refresh_page_boundaries_fixes_degenerate_offsets_after_ocr() {
        let mut result = make_scanned_pdf_result(&[
            ("Investment and Subscription Agreement", 1),
            ("dated 13.12.2021", 2),
            ("entered into by and between", 3),
        ]);

        // Verify precondition: stale boundaries are degenerate (byte_start == byte_end)
        let stale = result.metadata.pages.as_ref().unwrap().boundaries.as_ref().unwrap();
        for b in stale {
            assert_eq!(b.byte_start, b.byte_end, "stale boundary must be degenerate before fix");
        }

        refresh_page_boundaries(&mut result);

        let updated = result.metadata.pages.as_ref().unwrap().boundaries.as_ref().unwrap();

        assert_eq!(updated.len(), 3, "all pages must have a boundary");
        for b in updated {
            assert!(
                b.byte_start < b.byte_end,
                "boundary must be non-degenerate after refresh"
            );
        }
        // Each boundary must point to its page content within result.content
        assert_eq!(
            &result.content[updated[0].byte_start..updated[0].byte_end],
            "Investment and Subscription Agreement"
        );
        assert_eq!(
            &result.content[updated[1].byte_start..updated[1].byte_end],
            "dated 13.12.2021"
        );
        assert_eq!(
            &result.content[updated[2].byte_start..updated[2].byte_end],
            "entered into by and between"
        );
    }

    #[test]
    fn refresh_page_boundaries_no_op_when_pages_absent() {
        let mut result = ExtractionResult {
            content: "some content".to_string(),
            pages: None,
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };
        // Must not panic and metadata remains untouched
        refresh_page_boundaries(&mut result);
        assert!(result.metadata.pages.is_none());
    }

    #[test]
    fn refresh_page_boundaries_no_op_when_metadata_pages_absent() {
        let pages = vec![make_page(1, "some text")];
        let mut result = ExtractionResult {
            content: "some text".to_string(),
            pages: Some(pages),
            metadata: crate::types::Metadata {
                pages: None,
                ..Default::default()
            },
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };
        // Has result.pages but no metadata.pages — must not panic
        refresh_page_boundaries(&mut result);
        assert!(result.metadata.pages.is_none());
    }

    // refresh_page_boundaries must produce identical boundaries on a second call.
    // recompute_boundaries_from_pages is deterministic given the same content and pages,
    // so the written-back boundaries become the source truth and re-searching them yields
    // the same offsets.
    #[test]
    fn refresh_page_boundaries_is_idempotent() {
        let mut result = make_scanned_pdf_result(&[("First page", 1), ("Second page", 2)]);

        refresh_page_boundaries(&mut result);
        let after_first = result
            .metadata
            .pages
            .as_ref()
            .unwrap()
            .boundaries
            .as_ref()
            .unwrap()
            .iter()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect::<Vec<_>>();

        refresh_page_boundaries(&mut result);
        let after_second = result
            .metadata
            .pages
            .as_ref()
            .unwrap()
            .boundaries
            .as_ref()
            .unwrap()
            .iter()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect::<Vec<_>>();

        assert_eq!(after_first, after_second, "refresh must be idempotent");
    }

    // Native PDFs: boundaries were computed against raw extractor text; after
    // refresh they must point into result.content (render_plain output).
    // Simulates trailing-space pages — the typical native PDF artifact that
    // causes raw-extractor offsets to differ from result.content offsets.
    #[test]
    fn refresh_page_boundaries_normalises_native_pdf_offsets_to_result_content() {
        let p1_raw = "First page content. "; // trailing space from raw extraction
        let p2_raw = "Second page content. "; // trailing space from raw extraction

        // result.content as render_plain produces it (each paragraph trimmed)
        let p1_clean = "First page content.";
        let p2_clean = "Second page content.";
        let content = format!("{p1_clean}\n\n{p2_clean}");

        // Simulate raw-extractor boundaries (offsets into raw text, not result.content)
        let mut raw = String::new();
        let b1_start = raw.len();
        raw.push_str(p1_raw);
        let b1_end = raw.len();
        raw.push_str("\n\n");
        let b2_start = raw.len();
        raw.push_str(p2_raw);
        let b2_end = raw.len();
        // Verify that raw offsets indeed differ from result.content offsets
        assert_ne!(
            b1_end,
            p1_clean.len(),
            "raw extractor end must differ from render_plain end"
        );

        let stale_boundaries = vec![
            PageBoundary {
                byte_start: b1_start,
                byte_end: b1_end,
                page_number: 1,
            },
            PageBoundary {
                byte_start: b2_start,
                byte_end: b2_end,
                page_number: 2,
            },
        ];
        let page_structure = crate::types::PageStructure {
            total_count: 2,
            unit_type: crate::types::PageUnitType::Page,
            boundaries: Some(stale_boundaries),
            pages: None,
        };
        let mut result = ExtractionResult {
            content,
            pages: Some(vec![make_page(1, p1_raw), make_page(2, p2_raw)]),
            metadata: crate::types::Metadata {
                pages: Some(page_structure),
                ..Default::default()
            },
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        refresh_page_boundaries(&mut result);

        let updated = result.metadata.pages.as_ref().unwrap().boundaries.as_ref().unwrap();
        assert_eq!(updated.len(), 2);
        assert_eq!(
            &result.content[updated[0].byte_start..updated[0].byte_end],
            p1_clean,
            "page 1 boundary must point to trimmed content in result.content"
        );
        assert_eq!(
            &result.content[updated[1].byte_start..updated[1].byte_end],
            p2_clean,
            "page 2 boundary must point to trimmed content in result.content"
        );
    }

    // --- Issue #1073: chunk content must match output_format ---

    #[cfg(feature = "chunking")]
    fn make_result_with_formatted(plain: &str, formatted: &str) -> ExtractionResult {
        ExtractionResult {
            content: plain.to_string(),
            formatted_content: Some(formatted.to_string()),
            mime_type: std::borrow::Cow::Borrowed("application/pdf"),
            ..Default::default()
        }
    }

    #[cfg(feature = "chunking")]
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

    #[cfg(feature = "chunking")]
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

    #[cfg(feature = "chunking")]
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

    #[cfg(feature = "chunking")]
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

    #[cfg(feature = "chunking")]
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

    #[cfg(feature = "chunking")]
    #[test]
    fn chunk_page_metadata_is_none_for_non_plain_output_format() {
        // Known limitation (#1074): when output_format != Plain, page boundaries computed
        // against plain-text offsets are not valid for the formatted string and are omitted.
        // This test documents the expected behaviour so future changes don't silently regress it.
        let plain = "Page one content\n\nPage two content";
        let markdown = "# Page one\n\nPage one content\n\n# Page two\n\nPage two content";

        let config = markdown_chunking_config();
        let mut result = make_result_with_formatted(plain, markdown);

        execute_chunking(&mut result, &config).unwrap();

        let chunks = result.chunks.expect("chunks must be populated");
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(
                chunk.metadata.first_page.is_none(),
                "first_page must be None for non-plain output_format (tracked in #1074), got: {:?}",
                chunk.metadata.first_page
            );
            assert!(
                chunk.metadata.last_page.is_none(),
                "last_page must be None for non-plain output_format (tracked in #1074), got: {:?}",
                chunk.metadata.last_page
            );
        }
    }
}
