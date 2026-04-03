//! Main PDF-to-Markdown pipeline orchestrator.

use std::borrow::Cow;

use crate::pdf::error::Result;
use crate::pdf::hierarchy::{BoundingBox, SegmentData, TextBlock, assign_heading_levels_smart, cluster_font_sizes};
use pdfium_render::prelude::*;
use rayon::prelude::*;

use super::assembly::assemble_internal_document;
use super::bridge::{ImagePosition, extracted_blocks_to_paragraphs, filter_sidebar_blocks, objects_to_page_data};
use super::classify::{
    classify_paragraphs, demote_heading_runs, demote_unnumbered_subsections, mark_cross_page_repeating_text,
    refine_heading_hierarchy,
};
use super::constants::{
    FULL_LINE_FRACTION, MIN_FONT_SIZE, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO, PAGE_BOTTOM_MARGIN_FRACTION,
    PAGE_TOP_MARGIN_FRACTION,
};
use super::lines::is_cjk_char;
use super::paragraphs::{merge_continuation_paragraphs, split_embedded_list_items};
use super::text_repair::{
    apply_ligature_repairs, apply_to_all_segments, build_ligature_repair_map, clean_duplicate_punctuation,
    expand_ligatures_with_space_absorption, normalize_text_encoding, normalize_unicode_text,
    repair_broken_word_spacing, repair_contextual_ligatures, repair_ligature_spaces, text_has_broken_word_spacing,
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

        let page_t = crate::utils::timing::Instant::now();
        match extract_page_content(&page) {
            Ok(extraction) if extraction.method == ExtractionMethod::StructureTree && !extraction.blocks.is_empty() => {
                tracing::trace!(
                    page = i,
                    method = ?extraction.method,
                    block_count = extraction.blocks.len(),
                    "PDF structure pipeline: page extracted via structure tree"
                );
                // Log the roles of the first few blocks for debugging
                for (bi, block) in extraction.blocks.iter().take(10).enumerate() {
                    tracing::trace!(
                        page = i,
                        block_index = bi,
                        role = ?block.role,
                        text_preview = block.text.chars().take(60).collect::<String>(),
                        font_size = ?block.font_size,
                        is_bold = block.is_bold,
                        child_count = block.children.len(),
                        "PDF structure pipeline: structure tree block"
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
                    apply_to_all_segments(&mut paragraphs, |t| apply_ligature_repairs(t, &repair_map));
                }
                // Then apply contextual ligature repair for fonts where
                // pdfium doesn't flag encoding errors. Check the actual
                // paragraph text (not page.text()) since structure tree
                // text may differ from the page text layer.
                {
                    let all_text = build_page_text(&paragraphs);
                    if text_has_ligature_corruption(&all_text) {
                        apply_to_all_segments(&mut paragraphs, repair_contextual_ligatures);
                    }
                    // Repair broken word spacing (single-letter fragments like "M ust")
                    if text_has_broken_word_spacing(&all_text) {
                        apply_to_all_segments(&mut paragraphs, repair_broken_word_spacing);
                    }
                }
                // Fused text normalization pass: apply all 5 text repairs in a single
                // traversal instead of 5 separate passes over all segments.
                apply_to_all_segments(&mut paragraphs, fused_text_repairs);
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
                    "PDF structure pipeline: structure tree paragraphs after conversion"
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
                        "PDF structure pipeline: structure tree has font variation but no headings, will classify via font-size clustering"
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
        let page_ms = page_t.elapsed_ms();
        if page_ms > 2000.0 {
            tracing::warn!(page = i, elapsed_ms = page_ms, "slow structure tree extraction");
        }
    }

    tracing::debug!(
        heuristic_page_count = heuristic_pages.len(),
        struct_tree_ok = struct_tree_results.iter().filter(|r| r.is_some()).count(),
        "PDF structure pipeline: stage 0 complete"
    );

    Ok((struct_tree_results, heuristic_pages, has_font_encoding_issues))
}

/// Stage 1: Extract segments from heuristic pages via pdfium text/object APIs.
///
/// Returns (all_page_segments indexed by page, image_positions, paragraph_gap_ys per page, page_heights).
#[allow(clippy::type_complexity)]
fn extract_heuristic_segments(
    pages: &PdfPages,
    page_count: PdfPageIndex,
    heuristic_pages: &[usize],
    top_margin: Option<f32>,
    bottom_margin: Option<f32>,
    has_layout_hints: bool,
) -> (Vec<Vec<SegmentData>>, Vec<ImagePosition>, Vec<Vec<f32>>, Vec<f32>) {
    let stage1_start = crate::utils::timing::Instant::now();
    let mut all_page_segments: Vec<Vec<SegmentData>> = vec![Vec::new(); page_count as usize];
    let mut all_page_gap_ys: Vec<Vec<f32>> = vec![Vec::new(); page_count as usize];
    let mut page_heights: Vec<f32> = vec![0.0; page_count as usize];
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

        page_heights[i] = page.height().value;
        let page_t = crate::utils::timing::Instant::now();
        let (mut segments, image_positions, paragraph_gap_ys) = objects_to_page_data(&page, i + 1, &mut image_offset);
        let page_ms = page_t.elapsed_ms();
        if page_ms > 1000.0 {
            tracing::warn!(
                "slow objects_to_page_data page {}: {:.0}ms, {} segments",
                i + 1,
                page_ms,
                segments.len()
            );
        }

        // Filter tiny text in-place (avoids cloning all segments).
        // Only filter if at least some segments would survive.
        if segments
            .iter()
            .any(|s| s.font_size >= MIN_FONT_SIZE && !s.text.trim().is_empty())
        {
            segments.retain(|s| s.font_size >= MIN_FONT_SIZE);
        }

        // When layout hints are available, skip geometric margin filtering and
        // standalone page number removal. The layout model handles PageHeader/
        // PageFooter classification more accurately than hard margin cutoffs.
        if !has_layout_hints {
            let page_height = page.height().value;
            let top_frac = top_margin.unwrap_or(PAGE_TOP_MARGIN_FRACTION).clamp(0.0, 0.5);
            let bottom_frac = bottom_margin.unwrap_or(PAGE_BOTTOM_MARGIN_FRACTION).clamp(0.0, 0.5);
            let top_cutoff = page_height * (1.0 - top_frac);
            let bottom_cutoff = page_height * bottom_frac;

            // Check if margin filtering would leave content before applying
            let would_survive = segments.iter().any(|s| {
                !s.text.trim().is_empty()
                    && (s.baseline_y == 0.0 || (s.baseline_y <= top_cutoff && s.baseline_y >= bottom_cutoff))
            });
            if would_survive {
                segments
                    .retain(|s| s.baseline_y == 0.0 || (s.baseline_y <= top_cutoff && s.baseline_y >= bottom_cutoff));
            }

            filter_standalone_page_numbers(&mut segments);
        }
        let filtered = segments;

        all_page_segments[i] = filtered;
        all_page_gap_ys[i] = paragraph_gap_ys;
        all_image_positions.extend(image_positions);
    }

    tracing::debug!(
        stage1_ms = stage1_start.elapsed_ms(),
        total_segments = all_page_segments.iter().map(|s| s.len()).sum::<usize>(),
        "PDF structure pipeline: stage 1 complete"
    );

    (all_page_segments, all_image_positions, all_page_gap_ys, page_heights)
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
) -> Result<(Vec<(f32, Option<u8>)>, ahash::AHashSet<usize>)> {
    // Identify structure tree pages that have font size variation but no
    // heading signals — these need font-size-based heading classification.
    // Pages with no font variation are left as plain paragraphs (classify
    // would incorrectly assign headings based on unrelated pages' font data).
    let struct_tree_needs_classify: ahash::AHashSet<usize> = struct_tree_results
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

/// Per-page input bundle for Stage 3 parallel processing.
///
/// Each page's data is pre-extracted before `into_par_iter` so all threads
/// receive owned, non-overlapping slices of the document's data.
struct PageInput {
    /// Index of this page in the document (0-based).
    page_index: usize,
    /// Paragraphs from the PDF structure tree, if extraction succeeded.
    struct_paragraphs: Option<Vec<PdfParagraph>>,
    /// Segments from heuristic extraction (non-empty only when `struct_paragraphs` is `None`).
    heuristic_segments: Vec<SegmentData>,
    /// Layout hints for this page, if layout detection was run.
    page_hints: Option<Vec<LayoutHint>>,
    /// Bounding boxes of tables that were successfully extracted for this page.
    table_bboxes: Vec<crate::types::BoundingBox>,
    /// Per-hint validation results from CC analysis (parallel to page_hints).
    /// Empty when layout-detection is not active.
    #[allow(dead_code)]
    hint_validations: Vec<super::regions::layout_validation::RegionValidation>,
    /// Whether this page's structure-tree paragraphs need font-size classification.
    needs_classify: bool,
    /// Y-coordinates of paragraph gaps detected from pdfium segment boundaries.
    paragraph_gap_ys: Vec<f32>,
}

/// Process a single page's data through Stage 3: classification, text repair,
/// layout overrides, dehyphenation, and list splitting.
///
/// This function is intentionally free of any shared mutable state so it can be
/// called from multiple threads via `rayon::par_iter`.
fn process_single_page(
    input: PageInput,
    heading_map: &[(f32, Option<u8>)],
    doc_body_font_size: Option<f32>,
) -> Vec<PdfParagraph> {
    let PageInput {
        page_index: i,
        struct_paragraphs,
        heuristic_segments,
        page_hints,
        table_bboxes,
        hint_validations: _,
        needs_classify,
        paragraph_gap_ys: _,
    } = input;

    if let Some(mut paragraphs) = struct_paragraphs {
        // Structure tree pages: use the PDF's own paragraph structure.
        // The structure tree preserves the author's intended paragraph boundaries
        // and heading hierarchy. Layout overrides (apply_layout_overrides) handle
        // classification corrections from the layout model without destroying
        // paragraph structure.
        //
        // Apply heading classification to struct tree pages that have
        // font size variation but no structure-tree-level headings.
        if needs_classify {
            tracing::debug!(
                page = i,
                "PDF structure pipeline: classifying struct tree page via font-size clustering"
            );
            classify_paragraphs(&mut paragraphs, heading_map);
        }
        // Merge consecutive body-text paragraphs from structure tree.
        // Many PDFs tag each visual line as a separate <P>, causing over-splitting.
        merge_continuation_paragraphs(&mut paragraphs);
        // Apply layout detection overrides when available.
        if let Some(ref hints) = page_hints {
            super::layout_classify::apply_layout_overrides(&mut paragraphs, hints, 0.5, 0.2, doc_body_font_size);
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
            retain_page_furniture_safely(&mut paragraphs);
        }
        paragraphs
    } else {
        let page_segments = heuristic_segments;
        // Full-text extraction: blocks from page.text().all() with char-indexed
        // font metadata. Both layout and no-layout paths use the same extraction.
        // Layout hints refine classifications when available.
        tracing::debug!(
            page = i,
            segments = page_segments.len(),
            has_layout_hints = page_hints.is_some(),
            "process_single_page: heuristic path"
        );
        let page_segments = filter_segments_by_table_bboxes(page_segments, &table_bboxes);
        let mut paragraphs = super::bridge::blocks_to_paragraphs(page_segments, heading_map, &input.paragraph_gap_ys);
        tracing::debug!(
            page = i,
            paragraphs = paragraphs.len(),
            "heuristic paragraphs classified"
        );
        if let Some(ref hints) = page_hints {
            super::layout_classify::apply_layout_overrides(&mut paragraphs, hints, 0.5, 0.2, doc_body_font_size);
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
        }
        retain_page_furniture_safely(&mut paragraphs);
        paragraphs
    }
}

/// Render a PDF document as structured `InternalDocument`, with tables interleaved.
///
/// Returns (InternalDocument, has_font_encoding_issues).
#[allow(clippy::too_many_arguments)]
pub fn extract_document_structure(
    document: &PdfDocument,
    k_clusters: usize,
    tables: &[crate::types::Table],
    top_margin: Option<f32>,
    bottom_margin: Option<f32>,
    layout_hints: Option<&[Vec<LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::DynamicImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::layout_runner::PageLayoutResult]>,
    #[cfg(not(feature = "layout-detection"))] _layout_results: Option<()>,
    allow_single_column: bool,
    #[cfg(feature = "layout-detection")] table_model: crate::core::config::layout::TableModel,
    #[cfg(not(feature = "layout-detection"))] _table_model: Option<()>,
) -> Result<(crate::types::internal::InternalDocument, bool)> {
    let pages = document.pages();
    let page_count = pages.len();
    let total_table_hints = layout_hints
        .map(|h| {
            h.iter()
                .flat_map(|p| p.iter())
                .filter(|hint| matches!(hint.class, super::types::LayoutHintClass::Table))
                .count()
        })
        .unwrap_or(0);
    tracing::trace!(
        has_layout = layout_hints.is_some(),
        total_table_hints,
        "Starting structure with tables pipeline"
    );
    tracing::debug!(page_count, "PDF structure pipeline: starting render");

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

    // Experimental: use pdf_oxide for text extraction when the feature is enabled.
    // pdf_oxide parses PDF content streams directly with adaptive TJ-offset thresholds,
    // producing cleaner word spacing for fonts with broken CMaps.
    #[cfg(feature = "pdf-oxide")]
    let oxide_segments: Option<Vec<Vec<SegmentData>>> = {
        let t = crate::utils::timing::Instant::now();
        let result = crate::pdf::oxide_text::extract_segments_with_oxide(page_count as usize);
        if let Some(ref segs) = result {
            let total: usize = segs.iter().map(|s| s.len()).sum();
            tracing::debug!(
                total_segments = total,
                elapsed_ms = t.elapsed_ms(),
                "pdf_oxide text extraction complete"
            );
        }
        result
    };
    #[cfg(not(feature = "pdf-oxide"))]
    let oxide_segments: Option<Vec<Vec<SegmentData>>> = None;

    // Stage 1: Extract segments from pages that need heuristic extraction.
    // Use pdf_oxide segments when available, fall back to pdfium.
    let (mut all_page_segments, all_image_positions, mut all_page_gap_ys, page_heights) =
        if let Some(oxide_segs) = oxide_segments {
            // Use pdf_oxide segments for heuristic pages, pdfium for images only.
            let mut all_segs = oxide_segs;
            // Ensure vector is large enough (pdf_oxide may return fewer pages)
            all_segs.resize_with(page_count as usize, Vec::new);
            // Still need pdfium for image positions
            let has_hints = layout_hints.is_some();
            let (_, image_positions, _, page_heights) = extract_heuristic_segments(
                pages,
                page_count,
                &heuristic_pages,
                top_margin,
                bottom_margin,
                has_hints,
            );
            (
                all_segs,
                image_positions,
                vec![Vec::new(); page_count as usize],
                page_heights,
            )
        } else {
            let has_hints = layout_hints.is_some();
            extract_heuristic_segments(
                pages,
                page_count,
                &heuristic_pages,
                top_margin,
                bottom_margin,
                has_hints,
            )
        };

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

    // Extract tables from layout-detected Table regions.
    // Uses the same segments as region assembly (from Stage 1) converted to
    // word-level HocrWords, ensuring consistency between table extraction
    // and text flow. Falls back to character-level extraction for pages
    // without heuristic segments (structure-tree-only pages).
    let mut layout_tables: Vec<crate::types::Table> = Vec::new();
    if let Some(hints_pages) = layout_hints {
        // Phase 1 (sequential): Prepare words per page. This may require pdfium
        // calls for structure-tree-only pages, so it must be sequential.
        struct TablePageData {
            page_idx: usize,
            words: Vec<crate::pdf::table_reconstruct::HocrWord>,
            page_height: f32,
        }
        let mut table_pages: Vec<TablePageData> = Vec::new();

        #[allow(clippy::needless_range_loop)]
        for page_idx in 0..page_count as usize {
            let Some(hints) = hints_pages.get(page_idx) else {
                continue;
            };
            if !hints.iter().any(|h| h.class == super::types::LayoutHintClass::Table) {
                continue;
            }

            // Always use pdfium's character API for per-word positions.
            // Our SegmentData has line-level granularity (not per-word), so
            // segments_to_words() would give imprecise bboxes that break
            // word-to-cell matching. extract_words_from_page() uses
            // page.text().chars() for accurate per-word bounding boxes.
            let page = pages.get(page_idx as PdfPageIndex).map_err(|e| {
                crate::pdf::error::PdfError::TextExtractionFailed(format!(
                    "Failed to get page {} for table extraction: {:?}",
                    page_idx, e
                ))
            })?;
            let page_height = page.height().value;
            let (words, page_height) = match crate::pdf::table::extract_words_from_page(&page, 0.0) {
                Ok(w) => (w, page_height),
                Err(e) => {
                    tracing::debug!(page = page_idx, error = %e, "table extraction: word extraction failed");
                    continue;
                }
            };

            if words.is_empty() {
                tracing::trace!(page = page_idx, "Table extraction: no words found, skipping");
                continue;
            }
            tracing::trace!(
                page = page_idx,
                word_count = words.len(),
                page_height,
                "Table page prepared"
            );
            table_pages.push(TablePageData {
                page_idx,
                words,
                page_height,
            });
        }

        // Phase 2 (parallel): Run table structure inference + heuristic fallback.
        // Supports TATR (default) and SLANeXT table models via `table_model` config.
        #[cfg(feature = "layout-detection")]
        {
            use crate::core::config::layout::TableModel;
            use std::cell::RefCell;

            // When Disabled, skip model inference entirely and go straight to heuristic path.
            let use_model_inference = table_model != TableModel::Disabled;

            thread_local! {
                static TL_TATR: RefCell<Option<crate::layout::models::tatr::TatrModel>> = const { RefCell::new(None) };
                static TL_SLANET: RefCell<Option<crate::layout::models::slanet::SlanetModel>> = const { RefCell::new(None) };
                static TL_SLANET_ALT: RefCell<Option<crate::layout::models::slanet::SlanetModel>> = const { RefCell::new(None) };
                static TL_CLASSIFIER: RefCell<Option<crate::layout::models::table_classifier::TableClassifier>> = const { RefCell::new(None) };
            }

            // Determine which table model to use and seed thread-locals
            let slanet_variant = match table_model {
                TableModel::SlanetWired => Some("slanet_wired"),
                TableModel::SlanetWireless => Some("slanet_wireless"),
                TableModel::SlanetPlus => Some("slanet_plus"),
                TableModel::SlanetAuto => Some("slanet_wired"), // primary=wired, alt=wireless
                TableModel::Tatr | TableModel::Disabled => None,
            };
            let is_auto = table_model == TableModel::SlanetAuto;

            let has_table_model = if !use_model_inference {
                false
            } else if let Some(variant) = slanet_variant {
                // SLANeXT path
                let seed = if layout_images.is_some() {
                    crate::layout::take_or_create_slanet(variant)
                } else {
                    None
                };
                let has = seed.is_some();
                if let Some(model) = seed {
                    TL_SLANET.with(|cell| {
                        *cell.borrow_mut() = Some(model);
                    });
                }
                // For auto mode, also seed wireless model + classifier
                if is_auto && has {
                    if let Some(alt) = crate::layout::take_or_create_slanet("slanet_wireless") {
                        TL_SLANET_ALT.with(|cell| {
                            *cell.borrow_mut() = Some(alt);
                        });
                    }
                    if let Some(cls) = crate::layout::take_or_create_table_classifier() {
                        TL_CLASSIFIER.with(|cell| {
                            *cell.borrow_mut() = Some(cls);
                        });
                    }
                }
                has
            } else {
                // TATR path (default)
                let seed = if layout_images.is_some() {
                    crate::layout::take_or_create_tatr()
                } else {
                    None
                };
                let has = seed.is_some();
                if let Some(model) = seed {
                    TL_TATR.with(|cell| {
                        *cell.borrow_mut() = Some(model);
                    });
                }
                has
            };

            tracing::debug!(
                has_table_model,
                table_model = %table_model,
                table_page_count = table_pages.len(),
                "Table extraction phase 2: model availability"
            );
            if use_model_inference && !has_table_model && !table_pages.is_empty() {
                let model_name = slanet_variant.unwrap_or("tatr");
                return Err(crate::pdf::error::PdfError::TextExtractionFailed(format!(
                    "Layout detection found table regions but {model_name} model is not available. \
                         Ensure the ONNX model is downloaded. Tables cannot be extracted without it."
                )));
            }

            if has_table_model {
                if let (Some(images), Some(results)) = (layout_images, layout_results) {
                    let parallel_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .par_iter()
                        .map(|tp| {
                            if let Some(variant) = slanet_variant {
                                // SLANeXT path — ensure models are loaded in thread-local
                                TL_SLANET.with(|cell| {
                                    let mut slanet_ref = cell.borrow_mut();
                                    if slanet_ref.is_none() {
                                        *slanet_ref = crate::layout::take_or_create_slanet(variant);
                                    }
                                });
                                if is_auto {
                                    TL_SLANET_ALT.with(|cell| {
                                        let mut alt_ref = cell.borrow_mut();
                                        if alt_ref.is_none() {
                                            *alt_ref = crate::layout::take_or_create_slanet("slanet_wireless");
                                        }
                                    });
                                    TL_CLASSIFIER.with(|cell| {
                                        let mut cls_ref = cell.borrow_mut();
                                        if cls_ref.is_none() {
                                            *cls_ref = crate::layout::take_or_create_table_classifier();
                                        }
                                    });
                                }

                                // Now borrow all needed models and run recognition
                                TL_SLANET.with(|slanet_cell| {
                                    let mut slanet_ref = slanet_cell.borrow_mut();
                                    let Some(slanet) = slanet_ref.as_mut() else {
                                        tracing::warn!("SLANeXT model unavailable in worker thread");
                                        return Vec::new();
                                    };

                                    if let (Some(page_image), Some(page_result)) =
                                        (images.get(tp.page_idx), results.get(tp.page_idx))
                                    {
                                        let hints = &hints_pages[tp.page_idx];

                                        // In auto mode, build the classifier tuple
                                        // Since we can't borrow multiple RefCells simultaneously,
                                        // take the alt model and classifier temporarily.
                                        let mut classifier_pair = if is_auto {
                                            let alt = TL_SLANET_ALT.with(|c| c.borrow_mut().take());
                                            let cls = TL_CLASSIFIER.with(|c| c.borrow_mut().take());
                                            match (cls, alt) {
                                                (Some(c), Some(a)) => Some((c, a)),
                                                (c, a) => {
                                                    // Put back anything we took
                                                    if let Some(cls) = c {
                                                        TL_CLASSIFIER.with(|cell| {
                                                            *cell.borrow_mut() = Some(cls);
                                                        });
                                                    }
                                                    if let Some(alt) = a {
                                                        TL_SLANET_ALT.with(|cell| {
                                                            *cell.borrow_mut() = Some(alt);
                                                        });
                                                    }
                                                    None
                                                }
                                            }
                                        } else {
                                            None
                                        };

                                        let classifier_arg =
                                            classifier_pair.as_mut().map(|(cls, alt)| {
                                                (cls as &mut crate::layout::models::table_classifier::TableClassifier,
                                                         alt as &mut crate::layout::models::slanet::SlanetModel)
                                            });

                                        let slanet_tables = super::regions::recognize_tables_slanet(
                                            page_image,
                                            hints,
                                            &tp.words,
                                            page_result,
                                            tp.page_height,
                                            tp.page_idx,
                                            slanet,
                                            classifier_arg,
                                        );

                                        // Return borrowed models
                                        if let Some((cls, alt)) = classifier_pair {
                                            TL_CLASSIFIER.with(|cell| {
                                                *cell.borrow_mut() = Some(cls);
                                            });
                                            TL_SLANET_ALT.with(|cell| {
                                                *cell.borrow_mut() = Some(alt);
                                            });
                                        }

                                        if !slanet_tables.is_empty() {
                                            return slanet_tables;
                                        }
                                    }

                                    // Fallback: heuristic
                                    let hints = &hints_pages[tp.page_idx];
                                    super::regions::extract_tables_from_layout_hints(
                                        &tp.words,
                                        hints,
                                        tp.page_idx,
                                        tp.page_height,
                                        0.5,
                                        allow_single_column,
                                    )
                                })
                            } else {
                                // TATR path (default)
                                TL_TATR.with(|cell| {
                                    let mut tatr_ref = cell.borrow_mut();
                                    if tatr_ref.is_none() {
                                        *tatr_ref = crate::layout::take_or_create_tatr();
                                    }
                                    let Some(tatr) = tatr_ref.as_mut() else {
                                        tracing::warn!("TATR model unavailable in worker thread");
                                        return Vec::new();
                                    };

                                    if let (Some(page_image), Some(page_result)) =
                                        (images.get(tp.page_idx), results.get(tp.page_idx))
                                    {
                                        let hints = &hints_pages[tp.page_idx];
                                        let tatr_tables = super::regions::recognize_tables_for_native_page(
                                            page_image,
                                            hints,
                                            &tp.words,
                                            page_result,
                                            tp.page_height,
                                            tp.page_idx,
                                            tatr,
                                        );
                                        tracing::trace!(
                                            page = tp.page_idx,
                                            tatr_tables = tatr_tables.len(),
                                            "TATR table recognition result"
                                        );
                                        if !tatr_tables.is_empty() {
                                            return tatr_tables;
                                        }
                                    }

                                    // Fallback: heuristic
                                    let hints = &hints_pages[tp.page_idx];
                                    super::regions::extract_tables_from_layout_hints(
                                        &tp.words,
                                        hints,
                                        tp.page_idx,
                                        tp.page_height,
                                        0.5,
                                        allow_single_column,
                                    )
                                })
                            }
                        })
                        .collect();

                    for tables in parallel_tables {
                        layout_tables.extend(tables);
                    }

                    // Return thread-local models to global cache
                    if let Some(variant) = slanet_variant {
                        TL_SLANET.with(|cell| {
                            if let Some(model) = cell.borrow_mut().take() {
                                crate::layout::return_slanet(variant, model);
                            }
                        });
                        if is_auto {
                            TL_SLANET_ALT.with(|cell| {
                                if let Some(model) = cell.borrow_mut().take() {
                                    crate::layout::return_slanet("slanet_wireless", model);
                                }
                            });
                            TL_CLASSIFIER.with(|cell| {
                                if let Some(model) = cell.borrow_mut().take() {
                                    crate::layout::return_table_classifier(model);
                                }
                            });
                        }
                    } else {
                        TL_TATR.with(|cell| {
                            if let Some(model) = cell.borrow_mut().take() {
                                crate::layout::return_tatr(model);
                            }
                        });
                    }
                }
            } else {
                // No TATR — run heuristic fallback sequentially
                tracing::debug!(
                    table_page_count = table_pages.len(),
                    "Running heuristic table extraction (no TATR)"
                );
                for tp in &table_pages {
                    let hints = &hints_pages[tp.page_idx];
                    layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                        &tp.words,
                        hints,
                        tp.page_idx,
                        tp.page_height,
                        0.5,
                        allow_single_column,
                    ));
                }
            }
        }

        #[cfg(not(feature = "layout-detection"))]
        {
            // No layout detection — run heuristic fallback sequentially
            for tp in &table_pages {
                let hints = &hints_pages[tp.page_idx];
                layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                    &tp.words,
                    hints,
                    tp.page_idx,
                    tp.page_height,
                    0.5,
                    allow_single_column,
                ));
            }
        }
    }

    tracing::debug!(tables_found = layout_tables.len(), "Table extraction complete");

    // Build per-page index of successfully extracted table bounding boxes.
    // This tells assign_segments_to_regions which Table bboxes actually produced
    // output, so it only suppresses segments for those — not for failed extractions.
    // Include BOTH layout-detected tables and heuristic tables (from extraction.rs)
    // to prevent text duplication when a table is extracted by the heuristic path.
    let extracted_table_bboxes_by_page: ahash::AHashMap<usize, Vec<crate::types::BoundingBox>> = {
        let mut map: ahash::AHashMap<usize, Vec<crate::types::BoundingBox>> = ahash::AHashMap::new();
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

    // Validate layout regions via connected component analysis.
    // Regions flagged as Empty should not suppress segments.
    #[cfg(feature = "layout-detection")]
    let validations_by_page: ahash::AHashMap<usize, Vec<super::regions::layout_validation::RegionValidation>> = {
        let mut map = ahash::AHashMap::new();
        if let (Some(images), Some(results), Some(hints_pages)) = (layout_images, layout_results, layout_hints) {
            for page_idx in 0..page_count as usize {
                if let (Some(img), Some(res), Some(hints)) =
                    (images.get(page_idx), results.get(page_idx), hints_pages.get(page_idx))
                {
                    let validations = super::regions::layout_validation::validate_page_regions(img, hints, res);
                    if validations.contains(&super::regions::layout_validation::RegionValidation::Empty) {
                        tracing::debug!(
                            page = page_idx,
                            empty_count = validations
                                .iter()
                                .filter(|v| **v == super::regions::layout_validation::RegionValidation::Empty)
                                .count(),
                            "layout validation: found empty regions"
                        );
                    }
                    map.insert(page_idx, validations);
                }
            }
        }
        map
    };
    #[cfg(not(feature = "layout-detection"))]
    let validations_by_page: ahash::AHashMap<usize, Vec<super::regions::layout_validation::RegionValidation>> =
        ahash::AHashMap::new();

    // Stage 3: Per-page structured extraction (parallelised with rayon).
    //
    // Pre-split all per-page owned data into a Vec<PageInput> so that rayon can
    // hand each worker an independent chunk with no shared mutable state.
    // Shared read-only data (heading_map, layout_hints, etc.) is accessed via
    // immutable references, which are Sync and safe to share across threads.
    let page_inputs: Vec<PageInput> = (0..page_count as usize)
        .map(|i| PageInput {
            page_index: i,
            struct_paragraphs: struct_tree_results[i].take(),
            heuristic_segments: std::mem::take(&mut all_page_segments[i]),
            page_hints: layout_hints.and_then(|h| h.get(i)).cloned(),
            table_bboxes: extracted_table_bboxes_by_page.get(&i).cloned().unwrap_or_default(),
            hint_validations: validations_by_page.get(&i).cloned().unwrap_or_default(),
            needs_classify: struct_tree_needs_classify.contains(&i),
            paragraph_gap_ys: std::mem::take(&mut all_page_gap_ys[i]),
        })
        .collect();

    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_par_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size))
        .collect();

    // Refine heading hierarchy across the document: merge split titles and
    // demote numbered section headings when a title H1 is detected.
    refine_heading_hierarchy(&mut all_page_paragraphs);
    demote_unnumbered_subsections(&mut all_page_paragraphs);
    demote_heading_runs(&mut all_page_paragraphs);

    // Mark short text that repeats across many pages as furniture (headers/footers/watermarks).
    mark_cross_page_repeating_text(&mut all_page_paragraphs, &page_heights);
    for page in &mut all_page_paragraphs {
        retain_page_furniture_safely(page);
    }

    // Deduplicate paragraphs with identical text within each page.
    // Catches bold/shadow rendering artifacts (consecutive duplicates)
    // and table content rendered as both table and body text.
    deduplicate_paragraphs(&mut all_page_paragraphs);

    let total_paragraphs: usize = all_page_paragraphs.iter().map(|p| p.len()).sum();
    tracing::debug!(
        heuristic_page_count = heuristic_pages.len(),
        total_paragraphs,
        heading_map_len = heading_map.len(),
        "PDF structure pipeline: stage 3 complete, assembling document"
    );

    // Stage 4: Assemble InternalDocument with tables interleaved
    // Combine heuristic tables (from extraction.rs) with layout-detected tables,
    // then deduplicate overlapping tables on the same page.
    let mut combined_tables: Vec<crate::types::Table> = tables.iter().cloned().chain(layout_tables).collect();
    deduplicate_overlapping_tables(&mut combined_tables);

    // Convert image positions to (page_idx, image_index) pairs for the assembler
    let image_pos_pairs: Vec<(usize, usize)> = all_image_positions
        .iter()
        .map(|img| (img.page_number, img.image_index))
        .collect();

    tracing::debug!(
        combined_tables = combined_tables.len(),
        image_positions = image_pos_pairs.len(),
        total_paragraphs = all_page_paragraphs.iter().map(|p| p.len()).sum::<usize>(),
        "stage 4: assembling document"
    );

    let mut doc = assemble_internal_document(all_page_paragraphs, &combined_tables, &image_pos_pairs);

    let element_count = doc.elements.len();
    tracing::debug!(element_count, "PDF structure pipeline: assembly complete");

    // Stage 5: Element-level text normalization.
    // Apply ligature repair and Unicode normalization to each element's text
    // so that any text that bypassed per-segment processing is also cleaned up.
    for elem in &mut doc.elements {
        if elem.text.is_empty() {
            continue;
        }
        let t1 = repair_contextual_ligatures(&elem.text);
        let t2 = expand_ligatures_with_space_absorption(&t1);
        let t3 = normalize_unicode_text(&t2);
        if let Cow::Owned(normalized) = t3 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t2 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t1 {
            elem.text = normalized;
        }
    }

    Ok((doc, has_font_encoding_issues))
}

/// Filter out segments that overlap >=50% with any table bounding box.
///
/// Segments with zero area or empty text are always kept.
fn filter_segments_by_table_bboxes(
    segments: Vec<SegmentData>,
    table_bboxes: &[crate::types::BoundingBox],
) -> Vec<SegmentData> {
    if table_bboxes.is_empty() {
        return segments;
    }
    segments
        .into_iter()
        .filter(|seg| {
            let seg_area = seg.width * seg.height;
            if seg_area <= 0.0 || seg.text.trim().is_empty() {
                return true;
            }
            !table_bboxes.iter().any(|bb| {
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
        .collect()
}

/// Concatenate all segment texts from paragraphs into a single string for analysis.
fn build_page_text(paragraphs: &[PdfParagraph]) -> String {
    let mut all_text = String::new();
    for p in paragraphs {
        for l in &p.lines {
            for s in &l.segments {
                if !all_text.is_empty() {
                    all_text.push(' ');
                }
                all_text.push_str(&s.text);
            }
        }
    }
    all_text
}

/// Apply all 5 text repair passes in a single traversal over a segment's text.
///
/// Returns `Cow::Borrowed` if nothing changed, `Cow::Owned` otherwise.
fn fused_text_repairs(text: &str) -> Cow<'_, str> {
    let t1 = normalize_text_encoding(text);
    let t2 = repair_ligature_spaces(&t1);
    let t3 = expand_ligatures_with_space_absorption(&t2);
    let t4 = normalize_unicode_text(&t3);
    let t5 = clean_duplicate_punctuation(&t4);
    match (&t1, &t2, &t3, &t4, &t5) {
        (Cow::Borrowed(_), Cow::Borrowed(_), Cow::Borrowed(_), Cow::Borrowed(_), Cow::Borrowed(_)) => {
            Cow::Borrowed(text)
        }
        _ => Cow::Owned(t5.into_owned()),
    }
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

    let mut to_remove = ahash::AHashSet::new();

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
                        to_remove.insert(j);
                    } else {
                        to_remove.insert(i);
                    }
                }
            }
        }
    }

    let mut idx = 0;
    tables.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
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
        .map(|seg| seg.text.bytes().filter(|b| b.is_ascii_alphanumeric()).count())
        .sum()
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
            // Clone to break the immutable borrow on `para` before mutating it.
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

/// Deduplicate paragraphs with identical text within each page.
///
/// Two-pass approach:
/// 1. Consecutive duplicates: remove back-to-back identical paragraphs
///    (catches bold/shadow rendering artifacts).
/// 2. Non-consecutive duplicates: remove body-text paragraphs whose
///    normalized text was already seen on the same page (catches table
///    content rendered as both table and body text).
///
/// Only deduplicates body text — headings, list items, code blocks,
/// formulas, and captions are preserved even if duplicated.
fn deduplicate_paragraphs(all_pages: &mut [Vec<PdfParagraph>]) {
    for page in all_pages.iter_mut() {
        if page.len() < 2 {
            continue;
        }

        // Pass 1: Remove consecutive duplicates.
        let mut i = 0;
        while i + 1 < page.len() {
            let a_text = paragraph_text_normalized(&page[i]);
            let b_text = paragraph_text_normalized(&page[i + 1]);
            if a_text.len() >= 5 && a_text == b_text {
                page.remove(i + 1);
            } else {
                i += 1;
            }
        }

        // Pass 2: Remove non-consecutive body-text duplicates.
        let mut seen = ahash::AHashSet::new();
        let mut to_remove = Vec::new();
        for (idx, para) in page.iter().enumerate() {
            if !is_dedup_candidate(para) {
                continue;
            }
            let text = paragraph_text_normalized(para);
            if text.len() < 15 {
                continue;
            }
            if !seen.insert(text) {
                to_remove.push(idx);
            }
        }
        for idx in to_remove.into_iter().rev() {
            page.remove(idx);
        }
    }
}

/// Extract normalized text from a paragraph for dedup comparison.
/// Builds result directly without intermediate Vec allocations.
fn paragraph_text_normalized(p: &PdfParagraph) -> String {
    let mut result = String::new();
    for line in &p.lines {
        for seg in &line.segments {
            for word in seg.text.split_whitespace() {
                if !result.is_empty() {
                    result.push(' ');
                }
                for c in word.chars() {
                    for lc in c.to_lowercase() {
                        result.push(lc);
                    }
                }
            }
        }
    }
    result
}

/// Check if a paragraph is eligible for deduplication.
fn is_dedup_candidate(p: &PdfParagraph) -> bool {
    p.heading_level.is_none()
        && !p.is_list_item
        && !p.is_code_block
        && !p.is_formula
        && !p.is_page_furniture
        && p.caption_for.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::{PdfLine, PdfParagraph};

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
            text: String::new(),
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
            text: String::new(),
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
