//! Main PDF-to-Markdown pipeline orchestrator.

use crate::pdf::error::Result;
use crate::pdf::hierarchy::{BoundingBox, SegmentData, TextBlock, assign_heading_levels_smart, cluster_font_sizes};
use pdfium_render::prelude::*;

use super::assembly::assemble_markdown_with_tables;
use super::bridge::{ImagePosition, extracted_blocks_to_paragraphs, filter_sidebar_blocks, objects_to_page_data};
use super::classify::{
    classify_paragraphs, demote_heading_runs, demote_unnumbered_subsections, mark_cross_page_repeating_text,
    refine_heading_hierarchy,
};
use super::columns::split_segments_into_columns;
use super::constants::{
    FULL_LINE_FRACTION, MIN_FONT_SIZE, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO, PAGE_BOTTOM_MARGIN_FRACTION,
    PAGE_TOP_MARGIN_FRACTION,
};
use super::lines::{is_cjk_char, segments_to_lines};
use super::paragraphs::{lines_to_paragraphs, merge_continuation_paragraphs, split_embedded_list_items};
use super::render::inject_image_placeholders;
use super::text_repair::{
    apply_ligature_repairs, apply_to_all_segments, build_ligature_repair_map, normalize_unicode_text,
    repair_broken_word_spacing, repair_contextual_ligatures, text_has_broken_word_spacing,
    text_has_ligature_corruption,
};
use super::types::{LayoutHint, PdfParagraph};

/// Stage 0: Try structure tree extraction for each page.
///
/// Returns (struct_tree_results, heuristic_pages, has_font_encoding_issues).
#[allow(clippy::type_complexity)]
fn extract_structure_tree_pages(
    pages: &PdfPages,
    page_count: PdfPageIndex,
) -> Result<(Vec<Option<Vec<PdfParagraph>>>, Vec<usize>, bool)> {
    let mut struct_tree_results: Vec<Option<Vec<PdfParagraph>>> = Vec::with_capacity(page_count as usize);
    let mut heuristic_pages: Vec<usize> = Vec::new();
    let mut has_font_encoding_issues = false;

    for i in 0..page_count {
        let page = pages.get(i).map_err(|e| {
            crate::pdf::error::PdfError::TextExtractionFailed(format!("Failed to get page {}: {:?}", i, e))
        })?;

        let page_t = std::time::Instant::now();
        match extract_page_content(&page) {
            Ok(extraction) if extraction.method == ExtractionMethod::StructureTree && !extraction.blocks.is_empty() => {
                tracing::trace!(
                    page = i,
                    method = ?extraction.method,
                    block_count = extraction.blocks.len(),
                    "PDF markdown pipeline: page extracted via structure tree"
                );
                // Log the roles of the first few blocks for debugging
                for (bi, block) in extraction.blocks.iter().take(10).enumerate() {
                    tracing::trace!(
                        page = i,
                        block_index = bi,
                        role = ?block.role,
                        text_preview = &block.text[..block.text.len().min(60)],
                        font_size = ?block.font_size,
                        is_bold = block.is_bold,
                        child_count = block.children.len(),
                        "PDF markdown pipeline: structure tree block"
                    );
                }
                let page_width = page.width().value;
                let filtered_blocks = filter_sidebar_blocks(&extraction.blocks, page_width);
                let mut paragraphs = extracted_blocks_to_paragraphs(&filtered_blocks);
                // Apply ligature repair to structure tree text (the structure tree
                // path bypasses chars_to_segments where repair normally happens).
                // First try error-flag-based repair, then fall back to contextual
                // heuristic for fonts where pdfium doesn't flag the encoding errors.
                // Try error-flag-based repair first (most accurate).
                if let Some(repair_map) = build_ligature_repair_map(&page) {
                    has_font_encoding_issues = true;
                    apply_to_all_segments(&mut paragraphs, |t| apply_ligature_repairs(t, &repair_map).into());
                }
                // Then apply contextual ligature repair for fonts where
                // pdfium doesn't flag encoding errors. Check the actual
                // paragraph text (not page.text()) since structure tree
                // text may differ from the page text layer.
                {
                    let mut all_text = String::new();
                    for p in &paragraphs {
                        for l in &p.lines {
                            for s in &l.segments {
                                if !all_text.is_empty() {
                                    all_text.push(' ');
                                }
                                all_text.push_str(&s.text);
                            }
                        }
                    }
                    if text_has_ligature_corruption(&all_text) {
                        apply_to_all_segments(&mut paragraphs, repair_contextual_ligatures);
                    }
                    // Repair broken word spacing (single-letter fragments like "M ust")
                    if text_has_broken_word_spacing(&all_text) {
                        apply_to_all_segments(&mut paragraphs, repair_broken_word_spacing);
                    }
                }
                // Normalize Unicode characters (curly quotes, fraction slash, etc.)
                apply_to_all_segments(&mut paragraphs, normalize_unicode_text);
                // Dehyphenate: rejoin trailing hyphens. Use positional
                // data for full-line checks when bounds are available.
                let has_positions = paragraphs.iter().any(|p| {
                    p.lines
                        .iter()
                        .any(|l| l.segments.iter().any(|s| s.width > 0.0 || s.x > 0.0))
                });
                dehyphenate_paragraphs(&mut paragraphs, has_positions);
                // Split paragraphs with embedded bullet characters (•) into
                // separate list item paragraphs (common in structure tree PDFs).
                split_embedded_list_items(&mut paragraphs);
                let heading_count = paragraphs.iter().filter(|p| p.heading_level.is_some()).count();
                let bold_count = paragraphs.iter().filter(|p| p.is_bold).count();
                let has_font_variation = has_font_size_variation(&paragraphs);
                tracing::trace!(
                    page = i,
                    paragraph_count = paragraphs.len(),
                    heading_count,
                    bold_count,
                    has_font_variation,
                    "PDF markdown pipeline: structure tree paragraphs after conversion"
                );
                if paragraphs.is_empty() {
                    struct_tree_results.push(None);
                    heuristic_pages.push(i as usize);
                } else if heading_count == 0 && has_font_variation {
                    // Structure tree has text with font size variation but no
                    // heading tags. Add to heuristic extraction for font-size
                    // clustering data; heading classification will be applied
                    // to these paragraphs in Stage 3.
                    tracing::debug!(
                        page = i,
                        "PDF markdown pipeline: structure tree has font variation but no headings, will classify via font-size clustering"
                    );
                    struct_tree_results.push(Some(paragraphs));
                    heuristic_pages.push(i as usize);
                } else {
                    struct_tree_results.push(Some(paragraphs));
                }
            }
            Ok(_) => {
                struct_tree_results.push(None);
                heuristic_pages.push(i as usize);
            }
            Err(_) => {
                struct_tree_results.push(None);
                heuristic_pages.push(i as usize);
            }
        }
        let page_ms = page_t.elapsed().as_secs_f64() * 1000.0;
        if page_ms > 2000.0 {
            tracing::warn!(page = i, elapsed_ms = page_ms, "slow structure tree extraction");
        }
    }

    tracing::debug!(
        heuristic_page_count = heuristic_pages.len(),
        struct_tree_ok = struct_tree_results.iter().filter(|r| r.is_some()).count(),
        "PDF markdown pipeline: stage 0 complete"
    );

    Ok((struct_tree_results, heuristic_pages, has_font_encoding_issues))
}

/// Stage 1: Extract segments from heuristic pages via pdfium text/object APIs.
///
/// Returns (all_page_segments indexed by page, image_positions).
fn extract_heuristic_segments(
    pages: &PdfPages,
    page_count: PdfPageIndex,
    heuristic_pages: &[usize],
    top_margin: Option<f32>,
    bottom_margin: Option<f32>,
) -> (Vec<Vec<SegmentData>>, Vec<ImagePosition>) {
    let stage1_start = std::time::Instant::now();
    let mut all_page_segments: Vec<Vec<SegmentData>> = vec![Vec::new(); page_count as usize];
    let mut all_image_positions: Vec<ImagePosition> = Vec::new();
    let mut image_offset = 0usize;

    for &i in heuristic_pages {
        let page = match pages.get(i as PdfPageIndex) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to get page {} for heuristic extraction: {:?}", i, e);
                continue;
            }
        };

        let page_t = std::time::Instant::now();
        let (segments, image_positions) = objects_to_page_data(&page, i + 1, &mut image_offset);
        let page_ms = page_t.elapsed().as_secs_f64() * 1000.0;
        if page_ms > 1000.0 {
            tracing::warn!(
                "slow objects_to_page_data page {}: {:.0}ms, {} segments",
                i + 1,
                page_ms,
                segments.len()
            );
        }

        // Filter out segments in page margins (headers/footers/page numbers)
        let page_height = page.height().value;
        let top_frac = top_margin.unwrap_or(PAGE_TOP_MARGIN_FRACTION).clamp(0.0, 0.5);
        let bottom_frac = bottom_margin.unwrap_or(PAGE_BOTTOM_MARGIN_FRACTION).clamp(0.0, 0.5);
        let top_cutoff = page_height * (1.0 - top_frac);
        let bottom_cutoff = page_height * bottom_frac;

        // Filter tiny text first (always applied).
        // If font-size filtering removes ALL text content, fall back to unfiltered
        // segments — some PDFs report unscaled font_size=1 when the actual rendered
        // size comes from the font matrix, so the filter would incorrectly discard
        // all content.
        let font_filtered: Vec<SegmentData> = segments
            .iter()
            .filter(|s| s.font_size >= MIN_FONT_SIZE)
            .cloned()
            .collect();
        let font_filtered = if font_filtered.iter().any(|s| !s.text.trim().is_empty()) {
            font_filtered
        } else {
            segments
        };

        let has_content = font_filtered.iter().any(|s| !s.text.trim().is_empty());

        // Apply margin filtering to remove headers/footers/page numbers.
        // If margin filtering removes ALL content, fall back to unfiltered
        // segments — this handles PDFs where pdfium reports baseline_y values
        // that fall outside the expected margin bands.
        let mut filtered: Vec<SegmentData> = if has_content {
            let margin_filtered: Vec<SegmentData> = font_filtered
                .iter()
                .filter(|s| {
                    if s.baseline_y == 0.0 {
                        return true;
                    }
                    s.baseline_y <= top_cutoff && s.baseline_y >= bottom_cutoff
                })
                .cloned()
                .collect();

            if margin_filtered.iter().any(|s| !s.text.trim().is_empty()) {
                margin_filtered
            } else {
                // Margin filter removed everything — skip it for this page
                font_filtered
            }
        } else {
            font_filtered
        };

        // Remove standalone page numbers: short numeric-only segments that are isolated
        // (no other segment on the same baseline)
        filter_standalone_page_numbers(&mut filtered);

        all_page_segments[i] = filtered;
        all_image_positions.extend(image_positions);
    }

    tracing::debug!(
        stage1_ms = stage1_start.elapsed().as_secs_f64() * 1000.0,
        total_segments = all_page_segments.iter().map(|s| s.len()).sum::<usize>(),
        "PDF markdown pipeline: stage 1 complete"
    );

    (all_page_segments, all_image_positions)
}

/// Stage 2: Cluster font sizes globally and assign heading levels.
///
/// Returns (heading_map, set of struct-tree page indices needing font-size classification).
#[allow(clippy::type_complexity)]
fn build_heading_map(
    all_page_segments: &[Vec<SegmentData>],
    struct_tree_results: &[Option<Vec<PdfParagraph>>],
    heuristic_pages: &[usize],
    k_clusters: usize,
) -> Result<(Vec<(f32, Option<u8>)>, std::collections::HashSet<usize>)> {
    // Identify structure tree pages that have font size variation but no
    // heading signals — these need font-size-based heading classification.
    // Pages with no font variation are left as plain paragraphs (classify
    // would incorrectly assign headings based on unrelated pages' font data).
    let struct_tree_needs_classify: std::collections::HashSet<usize> = struct_tree_results
        .iter()
        .enumerate()
        .filter_map(|(i, result)| {
            result.as_ref().and_then(|paragraphs| {
                let has_headings = paragraphs.iter().any(|p| p.heading_level.is_some());
                if !has_headings && has_font_size_variation(paragraphs) {
                    Some(i)
                } else {
                    None
                }
            })
        })
        .collect();

    // Build TextBlocks from heuristic pages + struct tree pages needing classification.
    let mut all_blocks: Vec<TextBlock> = Vec::new();
    let empty_bbox = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
    for &i in heuristic_pages {
        for seg in &all_page_segments[i] {
            if seg.text.trim().is_empty() {
                continue;
            }
            all_blocks.push(TextBlock {
                text: String::new(),
                bbox: empty_bbox,
                font_size: seg.font_size,
            });
        }
    }
    // Include font sizes from struct tree pages that need classification.
    for &i in &struct_tree_needs_classify {
        if let Some(paragraphs) = &struct_tree_results[i] {
            for para in paragraphs {
                all_blocks.push(TextBlock {
                    text: String::new(),
                    bbox: empty_bbox,
                    font_size: para.dominant_font_size,
                });
            }
        }
    }

    let heading_map = if all_blocks.is_empty() {
        Vec::new()
    } else {
        let clusters = cluster_font_sizes(&all_blocks, k_clusters)?;
        assign_heading_levels_smart(&clusters, MIN_HEADING_FONT_RATIO, MIN_HEADING_FONT_GAP)
    };

    Ok((heading_map, struct_tree_needs_classify))
}

/// Render a PDF document as markdown, with tables interleaved at their positions.
///
/// Returns (markdown, has_font_encoding_issues).
#[allow(clippy::too_many_arguments)]
pub fn render_document_as_markdown_with_tables(
    document: &PdfDocument,
    k_clusters: usize,
    tables: &[crate::types::Table],
    top_margin: Option<f32>,
    bottom_margin: Option<f32>,
    page_marker_format: Option<&str>,
    layout_hints: Option<&[Vec<LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::DynamicImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::layout_runner::PageLayoutResult]>,
    #[cfg(not(feature = "layout-detection"))] _layout_results: Option<()>,
) -> Result<(String, bool)> {
    let pages = document.pages();
    let page_count = pages.len();
    tracing::debug!(page_count, "PDF markdown pipeline: starting render");

    let mut has_font_encoding_issues = false;

    // Previously this forced heuristic extraction when layout hints were present,
    // but apply_layout_overrides() now supports proportional matching for structure
    // tree pages without positional data. Forcing heuristic extraction degrades text
    // quality (chars_to_segments garbles words), so we let structure tree extraction
    // proceed normally and apply layout hints in Stage 3.

    // Stage 0: Try structure tree extraction for each page.
    let (mut struct_tree_results, heuristic_pages, struct_tree_font_issues) =
        extract_structure_tree_pages(pages, page_count)?;
    has_font_encoding_issues |= struct_tree_font_issues;

    // Stage 1: Extract segments from pages that need heuristic extraction.
    // Uses pdfium's page objects API (via PdfParagraph::from_objects) for spatial analysis
    // and text grouping, plus image detection for position-aware placeholders.
    let (mut all_page_segments, all_image_positions) =
        extract_heuristic_segments(pages, page_count, &heuristic_pages, top_margin, bottom_margin);

    // Detect font encoding issues on heuristic pages.
    for &i in &heuristic_pages {
        let page = pages.get(i as PdfPageIndex).map_err(|e| {
            crate::pdf::error::PdfError::TextExtractionFailed(format!("Failed to get page {}: {:?}", i, e))
        })?;
        if build_ligature_repair_map(&page).is_some() {
            has_font_encoding_issues = true;
            break;
        }
    }

    // Stage 2: Global font-size clustering (heuristic pages + struct tree pages needing classification).
    let (heading_map, struct_tree_needs_classify) =
        build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)?;

    // Compute the document-level body text font size from the heading map.
    // This is the centroid of the cluster NOT classified as a heading.
    // Used by layout detection to distinguish section headers (larger font)
    // from bold sub-headings at body text size.
    let doc_body_font_size: Option<f32> = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(size, _)| *size);

    // Extract tables from layout-detected Table regions using character-level
    // word extraction. This must happen before segments are consumed by Stage 3.
    let mut layout_tables: Vec<crate::types::Table> = Vec::new();
    if let Some(hints_pages) = layout_hints {
        // Try SLANet for neural table structure recognition when layout images are available.
        #[cfg(feature = "layout-detection")]
        let mut slanet_model = if layout_images.is_some() {
            crate::layout::take_or_create_slanet()
        } else {
            None
        };

        for &page_idx in &heuristic_pages {
            // Skip heuristic table extraction for pages where the structure tree
            // already succeeded. The structure tree path handles all content for
            // these pages; adding a heuristically-extracted table would duplicate
            // content and produce garbled markdown (especially for RTL documents
            // where column detection is unreliable).
            if struct_tree_results.get(page_idx).is_some_and(|r| r.is_some()) {
                continue;
            }

            let Some(hints) = hints_pages.get(page_idx) else {
                continue;
            };
            if !hints.iter().any(|h| h.class == super::types::LayoutHintClass::Table) {
                continue;
            }

            let page = pages.get(page_idx as PdfPageIndex).map_err(|e| {
                crate::pdf::error::PdfError::TextExtractionFailed(format!(
                    "Failed to get page {} for table extraction: {:?}",
                    page_idx, e
                ))
            })?;

            let page_height = page.height().value;

            // Extract character-level words from the page (accurate positions)
            let words = match crate::pdf::table::extract_words_from_page(&page, 0.0) {
                Ok(w) if !w.is_empty() => w,
                _ => continue,
            };

            // Try SLANet path first if we have images, results, and model
            #[cfg(feature = "layout-detection")]
            if let (Some(images), Some(results), Some(ref mut slanet)) =
                (layout_images, layout_results, slanet_model.as_mut())
                && let (Some(page_image), Some(page_result)) = (images.get(page_idx), results.get(page_idx))
            {
                let slanet_tables = super::regions::recognize_tables_for_native_page(
                    page_image,
                    hints,
                    &words,
                    page_result,
                    page_height,
                    page_idx,
                    slanet,
                );
                if !slanet_tables.is_empty() {
                    layout_tables.extend(slanet_tables);
                    continue;
                }
            }

            // Fallback: heuristic table reconstruction
            layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                &words,
                hints,
                page_idx,
                page_height,
                0.5,
            ));
        }

        // Return SLANet model to global cache for reuse by future extractions
        #[cfg(feature = "layout-detection")]
        if let Some(model) = slanet_model.take() {
            crate::layout::return_slanet(model);
        }
    }

    // Build per-page index of successfully extracted table bounding boxes.
    // This tells assign_segments_to_regions which Table bboxes actually produced
    // output, so it only suppresses segments for those — not for failed extractions.
    // Include BOTH layout-detected tables and heuristic tables (from extraction.rs)
    // to prevent text duplication when a table is extracted by the heuristic path.
    let extracted_table_bboxes_by_page: std::collections::HashMap<usize, Vec<crate::types::BoundingBox>> = {
        let mut map: std::collections::HashMap<usize, Vec<crate::types::BoundingBox>> =
            std::collections::HashMap::new();
        for table in &layout_tables {
            if let Some(ref bb) = table.bounding_box {
                // Table.page_number is 1-indexed, convert to 0-indexed
                map.entry(table.page_number.saturating_sub(1)).or_default().push(*bb);
            }
        }
        // Also include heuristic tables from extraction.rs — these have bboxes
        // but weren't previously used for segment suppression, causing text
        // duplication on pages where heuristic extraction finds a table.
        for table in tables {
            if let Some(ref bb) = table.bounding_box {
                map.entry(table.page_number.saturating_sub(1)).or_default().push(*bb);
            }
        }
        tracing::debug!(
            layout_tables = layout_tables.len(),
            heuristic_tables = tables.len(),
            pages_with_bboxes = map.len(),
            total_bboxes = map.values().map(|v| v.len()).sum::<usize>(),
            "table bbox suppression map built"
        );
        map
    };

    // Stage 3: Per-page structured extraction.
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = Vec::with_capacity(page_count as usize);
    for i in 0..page_count as usize {
        if let Some(mut paragraphs) = struct_tree_results[i].take() {
            // Apply heading classification to struct tree pages that have
            // font size variation but no structure-tree-level headings.
            if struct_tree_needs_classify.contains(&i) {
                tracing::debug!(
                    page = i,
                    "PDF markdown pipeline: classifying struct tree page via font-size clustering"
                );
                classify_paragraphs(&mut paragraphs, &heading_map);
            }
            // Merge consecutive body-text paragraphs from structure tree.
            // Many PDFs tag each visual line as a separate <P>, causing over-splitting.
            merge_continuation_paragraphs(&mut paragraphs);
            // Apply layout detection overrides when available.
            if let Some(hints) = layout_hints.and_then(|h| h.get(i)) {
                super::layout_classify::apply_layout_overrides(&mut paragraphs, hints, 0.5, 0.2);
                retain_page_furniture_safely(&mut paragraphs);
            }
            all_page_paragraphs.push(paragraphs);
        } else {
            let page_segments = std::mem::take(&mut all_page_segments[i]);
            let mut paragraphs = if let Some(hints) = layout_hints.and_then(|h| h.get(i))
                && !hints.is_empty()
                // Only use layout-guided assembly if the page has text-class hints.
                // If all hints are Table/Picture, the layout path adds no value and
                // causes text duplication (segments fall through as unassigned body
                // text alongside the extracted table).
                && hints.iter().any(|h| {
                    h.confidence >= 0.5
                        && !matches!(
                            h.class,
                            super::types::LayoutHintClass::Table | super::types::LayoutHintClass::Picture
                        )
                }) {
                // Layout-guided assembly: assign segments to layout regions
                // BEFORE line/paragraph assembly, ensuring paragraph boundaries
                // align with the model's structural predictions.
                let page_table_bboxes = extracted_table_bboxes_by_page
                    .get(&i)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                super::regions::assemble_region_paragraphs(
                    page_segments,
                    hints,
                    &heading_map,
                    0.5,
                    doc_body_font_size,
                    i,
                    page_table_bboxes,
                )
            } else {
                // Standard pipeline: XY-Cut → lines → paragraphs → classify
                // First, suppress segments that overlap with successfully extracted tables
                // to prevent text duplication (table content appearing as both a table and body text).
                let page_table_bboxes = extracted_table_bboxes_by_page
                    .get(&i)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                if !page_table_bboxes.is_empty() {
                    tracing::debug!(
                        page = i,
                        table_bboxes = page_table_bboxes.len(),
                        segments = page_segments.len(),
                        bbox_0 = ?page_table_bboxes.first().map(|b| (b.x0, b.y0, b.x1, b.y1)),
                        seg_0 = ?page_segments.first().map(|s| (s.x, s.y, s.width, s.height)),
                        "standard pipeline: table suppression active"
                    );
                }
                let page_segments = if !page_table_bboxes.is_empty() {
                    page_segments
                        .into_iter()
                        .filter(|seg| {
                            let seg_area = seg.width * seg.height;
                            if seg_area <= 0.0 || seg.text.trim().is_empty() {
                                return true; // Keep zero-area/empty segments
                            }
                            !page_table_bboxes.iter().any(|bb| {
                                let inter_left = seg.x.max(bb.x0 as f32);
                                let inter_right = (seg.x + seg.width).min(bb.x1 as f32);
                                let inter_bottom = seg.y.max(bb.y0 as f32);
                                let inter_top = (seg.y + seg.height).min(bb.y1 as f32);
                                if inter_left >= inter_right || inter_bottom >= inter_top {
                                    return false;
                                }
                                let inter_area = (inter_right - inter_left) * (inter_top - inter_bottom);
                                inter_area / seg_area >= 0.5
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    page_segments
                };
                let column_groups = split_segments_into_columns(&page_segments);
                let mut paras: Vec<PdfParagraph> = if column_groups.len() <= 1 {
                    let lines = segments_to_lines(page_segments);
                    lines_to_paragraphs(lines)
                } else {
                    let mut all_paras = Vec::new();
                    for group in column_groups {
                        let col_segments: Vec<_> = group.into_iter().map(|idx| page_segments[idx].clone()).collect();
                        let lines = segments_to_lines(col_segments);
                        all_paras.extend(lines_to_paragraphs(lines));
                    }
                    all_paras
                };
                classify_paragraphs(&mut paras, &heading_map);
                // Apply layout hint overrides to the standard pipeline output.
                // This path runs when the page didn't qualify for region-based
                // assembly (multi-column, oversized regions, no text-class hints).
                // Without this, layout detection results are silently discarded.
                if let Some(hints) = layout_hints.and_then(|h| h.get(i)) {
                    super::layout_classify::apply_layout_overrides(&mut paras, hints, 0.5, 0.2);
                }
                merge_continuation_paragraphs(&mut paras);
                paras
            };
            retain_page_furniture_safely(&mut paragraphs);
            // Apply contextual ligature repair to heuristic pages where
            // chars_to_segments didn't catch encoding issues (pdfium
            // doesn't always flag broken ToUnicode CMaps).
            {
                let mut all_text = String::new();
                for p in &paragraphs {
                    for l in &p.lines {
                        for s in &l.segments {
                            if !all_text.is_empty() {
                                all_text.push(' ');
                            }
                            all_text.push_str(&s.text);
                        }
                    }
                }
                if text_has_ligature_corruption(&all_text) {
                    apply_to_all_segments(&mut paragraphs, repair_contextual_ligatures);
                }
                // Repair broken word spacing (single-letter fragments like "M ust")
                // caused by broken font CMap/ToUnicode tables.
                if text_has_broken_word_spacing(&all_text) {
                    apply_to_all_segments(&mut paragraphs, repair_broken_word_spacing);
                }
            }
            // Normalize Unicode characters (curly quotes, fraction slash, etc.)
            apply_to_all_segments(&mut paragraphs, normalize_unicode_text);
            // Dehyphenate: heuristic path has positional data for
            // full-line detection, enabling both hyphen and no-hyphen joins.
            dehyphenate_paragraphs(&mut paragraphs, true);
            all_page_paragraphs.push(paragraphs);
        }
    }

    // Refine heading hierarchy across the document: merge split titles and
    // demote numbered section headings when a title H1 is detected.
    refine_heading_hierarchy(&mut all_page_paragraphs);
    demote_unnumbered_subsections(&mut all_page_paragraphs);
    demote_heading_runs(&mut all_page_paragraphs);

    // Mark short text that repeats across many pages as furniture (headers/footers/watermarks).
    mark_cross_page_repeating_text(&mut all_page_paragraphs);
    for page in &mut all_page_paragraphs {
        retain_page_furniture_safely(page);
    }

    let total_paragraphs: usize = all_page_paragraphs.iter().map(|p| p.len()).sum();
    tracing::debug!(
        heuristic_page_count = heuristic_pages.len(),
        total_paragraphs,
        heading_map_len = heading_map.len(),
        "PDF markdown pipeline: stage 3 complete, assembling markdown"
    );

    // Stage 4: Assemble markdown with tables interleaved
    // Combine heuristic tables (from extraction.rs) with layout-detected tables,
    // then deduplicate overlapping tables on the same page.
    let mut combined_tables: Vec<crate::types::Table> = tables.iter().cloned().chain(layout_tables).collect();
    deduplicate_overlapping_tables(&mut combined_tables);
    let markdown = assemble_markdown_with_tables(all_page_paragraphs, &combined_tables, page_marker_format);
    tracing::debug!(
        markdown_len = markdown.len(),
        has_headings = markdown.contains("# "),
        "PDF markdown pipeline: assembly complete"
    );

    // Stage 5: Inject image placeholders from positions collected during object extraction
    let final_markdown = if all_image_positions.is_empty() {
        markdown
    } else {
        let image_metadata: Vec<crate::types::ExtractedImage> = all_image_positions
            .iter()
            .map(|img| crate::types::ExtractedImage {
                data: bytes::Bytes::new(),
                format: std::borrow::Cow::Borrowed("unknown"),
                image_index: img.image_index,
                page_number: Some(img.page_number),
                width: None,
                height: None,
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
            })
            .collect();
        inject_image_placeholders(&markdown, &image_metadata)
    };

    // Stage 6: Final document-level normalization.
    // Apply ligature repair and Unicode normalization to the fully assembled
    // markdown so that any text that bypassed per-segment processing (e.g.
    // OCR results, table cells, injected image captions) is also cleaned up.
    let final_markdown = repair_contextual_ligatures(&final_markdown);
    let final_markdown = normalize_unicode_text(&final_markdown);

    Ok((final_markdown.into_owned(), has_font_encoding_issues))
}

/// Deduplicate tables that overlap on the same page.
///
/// When both heuristic and layout-based table extraction produce tables for the
/// same region, they can overlap. This keeps the table with more content (non-empty
/// cells or longer markdown) and discards the duplicate.
fn deduplicate_overlapping_tables(tables: &mut Vec<crate::types::Table>) {
    if tables.len() < 2 {
        return;
    }

    let mut to_remove = Vec::new();

    for i in 0..tables.len() {
        if to_remove.contains(&i) {
            continue;
        }
        for j in (i + 1)..tables.len() {
            if to_remove.contains(&j) {
                continue;
            }
            if tables[i].page_number != tables[j].page_number {
                continue;
            }
            // Check bbox overlap
            if let (Some(a), Some(b)) = (&tables[i].bounding_box, &tables[j].bounding_box) {
                let inter_x = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
                let inter_y = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
                let intersection = inter_x * inter_y;
                let area_a = (a.x1 - a.x0) * (a.y1 - a.y0);
                let area_b = (b.x1 - b.x0) * (b.y1 - b.y0);
                let min_area = area_a.min(area_b);

                if min_area > 0.0 && intersection / min_area > 0.5 {
                    // Keep the one with more content
                    let content_a = tables[i].cells.len() + tables[i].markdown.len();
                    let content_b = tables[j].cells.len() + tables[j].markdown.len();
                    if content_a >= content_b {
                        to_remove.push(j);
                    } else {
                        to_remove.push(i);
                    }
                }
            }
        }
    }

    to_remove.sort_unstable();
    to_remove.dedup();
    for idx in to_remove.into_iter().rev() {
        tables.remove(idx);
    }
}

/// Filter page furniture paragraphs with a safety valve.
///
/// Removes paragraphs marked as page furniture (headers/footers) by layout
/// detection. If removing ALL furniture-marked paragraphs would leave zero
/// content, the furniture markings are cleared instead — better to include
/// headers/footers than to produce empty output. This handles layout models
/// misclassifying body text as page furniture on non-standard document types
/// (e.g., legal transcripts, cover pages).
fn retain_page_furniture_safely(paragraphs: &mut Vec<PdfParagraph>) {
    let total = paragraphs.len();
    let furniture_count = paragraphs.iter().filter(|p| p.is_page_furniture).count();

    if furniture_count == 0 {
        return; // Nothing to filter
    }

    if furniture_count >= total {
        // All paragraphs marked as furniture — model likely wrong.
        // Clear furniture markings to preserve content.
        for para in paragraphs.iter_mut() {
            para.is_page_furniture = false;
        }
        return;
    }

    // Safety valve: if stripping furniture would remove >30% of total text
    // content, the layout model likely misclassified substantive content.
    // In that case, clear furniture markings entirely rather than risk
    // dropping document titles, section headers, or other real content.
    let total_alphanum: usize = paragraphs.iter().map(paragraph_alphanum_len).sum();

    if total_alphanum > 0 {
        let furniture_alphanum: usize = paragraphs
            .iter()
            .filter(|p| p.is_page_furniture)
            .map(paragraph_alphanum_len)
            .sum();

        if furniture_alphanum * 100 > total_alphanum * 30 {
            // Removing furniture would drop >30% of text — likely misclassified.
            for para in paragraphs.iter_mut() {
                para.is_page_furniture = false;
            }
            return;
        }
    }

    // Per-paragraph guard: don't strip furniture paragraphs that contain
    // substantive content (>80 alphanumeric chars). Short page numbers,
    // dates, and running titles are typically well under this threshold,
    // while misclassified body text or document titles exceed it.
    const MIN_SUBSTANTIVE_CHARS: usize = 80;

    paragraphs.retain(|p| {
        if !p.is_page_furniture {
            return true;
        }
        paragraph_alphanum_len(p) > MIN_SUBSTANTIVE_CHARS
    });
}

/// Count alphanumeric characters in a paragraph's text content.
fn paragraph_alphanum_len(para: &PdfParagraph) -> usize {
    para.lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .flat_map(|seg| seg.text.chars())
        .filter(|c| c.is_alphanumeric())
        .count()
}

/// Remove standalone page numbers from segments.
///
/// A standalone page number is a short numeric-only segment that has no other
/// segment sharing its approximate baseline (i.e., it sits alone on its line).
fn filter_standalone_page_numbers(segments: &mut Vec<SegmentData>) {
    if segments.is_empty() {
        return;
    }

    // Identify candidate page number indices
    let tolerance = 3.0_f32; // baseline proximity tolerance in points
    let candidates: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let trimmed = s.text.trim();
            !trimmed.is_empty() && trimmed.len() <= 4 && trimmed.chars().all(|c| c.is_ascii_digit())
        })
        .filter(|(idx, s)| {
            // Check that no other segment shares this baseline
            !segments
                .iter()
                .enumerate()
                .any(|(j, other)| j != *idx && (other.baseline_y - s.baseline_y).abs() < tolerance)
        })
        .map(|(idx, _)| idx)
        .collect();

    // Remove in reverse order to preserve indices
    for &idx in candidates.iter().rev() {
        segments.remove(idx);
    }
}

/// Dehyphenate paragraphs by rejoining words split across line boundaries.
///
/// When `has_positions` is true (heuristic extraction path), both explicit
/// trailing hyphens and implicit breaks (no hyphen, full line) are handled.
/// When false (structure tree path with x=0, width=0), only explicit trailing
/// hyphens are rejoined to avoid false positives.
fn dehyphenate_paragraphs(paragraphs: &mut [PdfParagraph], has_positions: bool) {
    for para in paragraphs.iter_mut() {
        if para.is_code_block || para.lines.len() < 2 {
            continue;
        }
        if has_positions {
            dehyphenate_paragraph_lines(para);
        } else {
            dehyphenate_hyphen_only(para);
        }
    }
}

/// Core dehyphenation with position-based full-line detection.
///
/// For each line boundary, checks whether the line extends close to the right
/// margin. If so, attempts to rejoin the trailing word of one line with the
/// leading word of the next.
fn dehyphenate_paragraph_lines(para: &mut PdfParagraph) {
    // Compute max right edge across all lines.
    let max_right_edge = para
        .lines
        .iter()
        .filter_map(|line| line.segments.last().map(|seg| seg.x + seg.width))
        .fold(0.0_f32, f32::max);

    if max_right_edge <= 0.0 {
        // No positional data — fall back to hyphen-only.
        dehyphenate_hyphen_only(para);
        return;
    }

    let threshold = max_right_edge * FULL_LINE_FRACTION;

    // Process line boundaries from last to first so index shifts don't
    // invalidate earlier indices.
    let line_count = para.lines.len();
    for i in (0..line_count - 1).rev() {
        let line_right = para.lines[i]
            .segments
            .last()
            .map(|seg| seg.x + seg.width)
            .unwrap_or(0.0);
        let is_full_line = line_right >= threshold;

        if !is_full_line {
            continue;
        }

        // Get trailing word from last segment of current line.
        let trailing_seg_text: &str = match para.lines[i].segments.last() {
            Some(seg) if !seg.text.is_empty() => &seg.text,
            _ => continue,
        };
        let trailing_word = match trailing_seg_text.split_whitespace().next_back() {
            Some(w) => w,
            None => continue,
        };

        // Get leading word from first segment of next line.
        let leading_seg_text: &str = match para.lines[i + 1].segments.first() {
            Some(seg) if !seg.text.is_empty() => &seg.text,
            _ => continue,
        };
        let leading_word = match leading_seg_text.split_whitespace().next() {
            Some(w) => w,
            None => continue,
        };

        // Skip if either word contains CJK characters.
        if trailing_word.chars().any(is_cjk_char) || leading_word.chars().any(is_cjk_char) {
            continue;
        }

        // Case 1: trailing hyphen
        if let Some(stem) = trailing_word.strip_suffix('-')
            && !stem.is_empty()
            && leading_word.starts_with(|c: char| c.is_lowercase())
        {
            let joined = format!("{}{}", stem, leading_word);
            let tw = trailing_word.to_string();
            let lw = leading_word.to_string();
            apply_dehyphenation_join(para, i, &tw, &lw, &joined);
            continue;
        }

        // Case 2 (removed): no-hyphen full-line word joining was too aggressive.
        // It incorrectly joined separate words like "through" + "several" →
        // "throughseveral" at every line boundary where text wraps to the margin.
        // Unhyphenated word splits are extremely rare in PDFs — explicit hyphens
        // (Case 1) cover the vast majority of real word breaks.
    }
}

/// Fallback dehyphenation for structure tree path (no positional data).
///
/// Only handles Case 1: explicit trailing hyphens with lowercase continuation.
fn dehyphenate_hyphen_only(para: &mut PdfParagraph) {
    let line_count = para.lines.len();
    for i in (0..line_count - 1).rev() {
        let trailing_seg_text: &str = match para.lines[i].segments.last() {
            Some(seg) if !seg.text.is_empty() => &seg.text,
            _ => continue,
        };
        let trailing_word = match trailing_seg_text.split_whitespace().next_back() {
            Some(w) => w,
            None => continue,
        };

        if !trailing_word.ends_with('-') {
            continue;
        }

        let leading_seg_text: &str = match para.lines[i + 1].segments.first() {
            Some(seg) if !seg.text.is_empty() => &seg.text,
            _ => continue,
        };
        let leading_word = match leading_seg_text.split_whitespace().next() {
            Some(w) => w,
            None => continue,
        };

        if trailing_word.chars().any(is_cjk_char) || leading_word.chars().any(is_cjk_char) {
            continue;
        }

        let stem = &trailing_word[..trailing_word.len() - 1];
        if !stem.is_empty() && leading_word.starts_with(|c: char| c.is_lowercase()) {
            let joined = format!("{}{}", stem, leading_word);
            let tw = trailing_word.to_string();
            let lw = leading_word.to_string();
            apply_dehyphenation_join(para, i, &tw, &lw, &joined);
        }
    }
}

/// Mutate segment text to apply a dehyphenation join.
///
/// Replaces the trailing word in the last segment of `line_idx` with `joined`,
/// and removes the leading word from the first segment of `line_idx + 1`.
fn apply_dehyphenation_join(
    para: &mut PdfParagraph,
    line_idx: usize,
    trailing_word: &str,
    leading_word: &str,
    joined: &str,
) {
    // Replace trailing word in last segment of current line.
    if let Some(seg) = para.lines[line_idx].segments.last_mut()
        && let Some(pos) = seg.text.rfind(trailing_word)
    {
        seg.text.replace_range(pos..pos + trailing_word.len(), joined);
    }

    // Remove leading word from first segment of next line.
    if let Some(seg) = para.lines[line_idx + 1].segments.first_mut()
        && let Some(pos) = seg.text.find(leading_word)
    {
        let end = pos + leading_word.len();
        // Also remove any trailing whitespace after the removed word.
        let trim_end = seg.text[end..]
            .find(|c: char| !c.is_whitespace())
            .map_or(seg.text.len(), |off| end + off);
        seg.text.replace_range(pos..trim_end, "");
    }
}

/// Check if paragraphs have meaningful font size variation.
///
/// Returns true if there are at least 2 distinct non-zero font sizes,
/// indicating that font-size clustering could identify heading candidates.
fn has_font_size_variation(paragraphs: &[PdfParagraph]) -> bool {
    let mut first_size: Option<f32> = None;
    for para in paragraphs {
        let size = para.dominant_font_size;
        if size <= 0.0 {
            continue;
        }
        match first_size {
            None => first_size = Some(size),
            Some(fs) if (size - fs).abs() > 0.5 => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::markdown::types::{PdfLine, PdfParagraph};

    /// Helper: create a segment with positional data.
    fn seg(text: &str, x: f32, width: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: 0.0,
            width,
            height: 12.0,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 0.0,
        }
    }

    fn line(segments: Vec<SegmentData>) -> PdfLine {
        PdfLine {
            segments,
            baseline_y: 0.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn para(lines: Vec<PdfLine>) -> PdfParagraph {
        PdfParagraph {
            lines,
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        }
    }

    /// Full-width line at x=10, width=490 → right edge 500.
    fn full_line_seg(text: &str) -> SegmentData {
        seg(text, 10.0, 490.0)
    }

    /// Short line at x=10, width=100 → right edge 110 (well below 500*0.85=425).
    fn short_line_seg(text: &str) -> SegmentData {
        seg(text, 10.0, 100.0)
    }

    #[test]
    fn test_case1_trailing_hyphen_full_line() {
        let mut p = para(vec![
            line(vec![full_line_seg("some soft-")]),
            line(vec![seg("ware is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "some software");
        assert_eq!(p.lines[1].segments[0].text, "is great");
    }

    #[test]
    fn test_case2_no_hyphen_full_line_no_join() {
        // Case 2 (no-hyphen joining) was removed — too many false positives
        // (e.g., "through" + "several" → "throughseveral"). Words without
        // hyphens are now left as-is.
        let mut p = para(vec![
            line(vec![full_line_seg("the soft")]),
            line(vec![seg("ware is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "the soft");
        assert_eq!(p.lines[1].segments[0].text, "ware is great");
    }

    #[test]
    fn test_short_line_no_join() {
        let mut p = para(vec![
            line(vec![short_line_seg("hello")]),
            line(vec![full_line_seg("world and more")]),
        ]);
        let original_trailing = p.lines[0].segments[0].text.clone();
        let original_leading = p.lines[1].segments[0].text.clone();
        dehyphenate_paragraph_lines(&mut p);
        // Short line → no joining.
        assert_eq!(p.lines[0].segments[0].text, original_trailing);
        assert_eq!(p.lines[1].segments[0].text, original_leading);
    }

    #[test]
    fn test_code_block_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some soft-")]),
            line(vec![seg("ware is code", 10.0, 200.0)]),
        ]);
        p.is_code_block = true;
        let mut paragraphs = vec![p];
        dehyphenate_paragraphs(&mut paragraphs, true);
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "some soft-");
    }

    #[test]
    fn test_uppercase_leading_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some text")]),
            line(vec![seg("Next sentence here", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        // Uppercase leading word → no joining.
        assert_eq!(p.lines[0].segments[0].text, "some text");
        assert_eq!(p.lines[1].segments[0].text, "Next sentence here");
    }

    #[test]
    fn test_cjk_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some \u{4E00}-")]),
            line(vec![seg("text here", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        // CJK trailing word → no joining.
        assert_eq!(p.lines[0].segments[0].text, "some \u{4E00}-");
    }

    #[test]
    fn test_real_world_software_no_join_without_hyphen() {
        // Without hyphen, words are not joined (Case 2 removed).
        let mut p = para(vec![
            line(vec![full_line_seg("advanced soft")]),
            line(vec![seg("ware development", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "advanced soft");
        assert_eq!(p.lines[1].segments[0].text, "ware development");
    }

    #[test]
    fn test_real_world_hardware_no_join_without_hyphen() {
        // Without hyphen, words are not joined (Case 2 removed).
        let mut p = para(vec![
            line(vec![full_line_seg("modern hard")]),
            line(vec![seg("ware components", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "modern hard");
        assert_eq!(p.lines[1].segments[0].text, "ware components");
    }

    #[test]
    fn test_leading_word_with_trailing_punctuation_no_join() {
        // Without hyphen, words are not joined (Case 2 removed).
        let mut p = para(vec![
            line(vec![full_line_seg("the soft")]),
            line(vec![seg("ware, which is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "the soft");
        assert_eq!(p.lines[1].segments[0].text, "ware, which is great");
    }

    #[test]
    fn test_hyphen_only_fallback() {
        let mut p = para(vec![
            line(vec![seg("some soft-", 0.0, 0.0)]),
            line(vec![seg("ware is great", 0.0, 0.0)]),
        ]);
        dehyphenate_hyphen_only(&mut p);
        assert_eq!(p.lines[0].segments[0].text, "some software");
        assert_eq!(p.lines[1].segments[0].text, "is great");
    }

    #[test]
    fn test_hyphen_only_uppercase_not_joined() {
        let mut p = para(vec![
            line(vec![seg("some well-", 0.0, 0.0)]),
            line(vec![seg("Known thing", 0.0, 0.0)]),
        ]);
        dehyphenate_hyphen_only(&mut p);
        // Uppercase leading → not joined.
        assert_eq!(p.lines[0].segments[0].text, "some well-");
    }

    #[test]
    fn test_single_line_paragraph_skipped() {
        let mut paragraphs = vec![para(vec![line(vec![full_line_seg("single line")])])];
        dehyphenate_paragraphs(&mut paragraphs, true);
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "single line");
    }

    #[test]
    fn test_multi_segment_line_no_join_without_hyphen() {
        // Without hyphen, words are not joined even across segments (Case 2 removed).
        let mut p = para(vec![
            line(vec![
                seg("first part", 10.0, 200.0),
                seg("soft", 220.0, 280.0), // right edge = 500
            ]),
            line(vec![seg("ware next words", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[1].text, "soft");
        assert_eq!(p.lines[1].segments[0].text, "ware next words");
    }

    // ── has_font_size_variation tests ──

    fn para_with_font_size(font_size: f32) -> PdfParagraph {
        PdfParagraph {
            lines: vec![line(vec![seg("text", 0.0, 100.0)])],
            dominant_font_size: font_size,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        }
    }

    #[test]
    fn test_has_font_size_variation_empty() {
        assert!(!has_font_size_variation(&[]));
    }

    #[test]
    fn test_has_font_size_variation_single_size() {
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.0)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_different_sizes() {
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(18.0)];
        assert!(has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_small_difference_ignored() {
        // 0.3pt difference is within 0.5pt tolerance
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.3)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_zero_sizes_ignored() {
        let paragraphs = vec![para_with_font_size(0.0), para_with_font_size(0.0)];
        assert!(!has_font_size_variation(&paragraphs));
    }
}
