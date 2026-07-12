//! Main PDF-to-Markdown pipeline orchestrator (oxide backend).

use std::borrow::Cow;

use crate::pdf::error::Result;
use crate::pdf::hierarchy::{BoundingBox, SegmentData, TextBlock, assign_heading_levels_smart, cluster_font_sizes};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use super::assembly::assemble_internal_document;
use super::classify::{
    classify_paragraphs, demote_heading_runs, demote_unnumbered_subsections, mark_arxiv_noise,
    mark_cross_page_repeating_short_text, mark_cross_page_repeating_text, refine_heading_hierarchy,
};
use super::constants::{FULL_LINE_FRACTION, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO};
use super::lines::is_cjk_char;
use super::paragraphs::{merge_continuation_paragraphs, split_embedded_list_items};
use super::text_repair::{
    apply_to_all_segments, clean_duplicate_punctuation, collapse_spaced_hyphens,
    expand_ligatures_with_space_absorption, normalize_text_encoding, normalize_unicode_text,
    repair_contextual_ligatures, repair_ligature_spaces,
};
use super::types::{LayoutHint, PdfParagraph};

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

    let mut all_blocks: Vec<TextBlock> = Vec::new();
    let empty_bbox = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
    // The text is carried so `assign_heading_levels_smart` can pick the body
    // cluster by character mass (char-weighted body size). Leaving it empty makes
    // every cluster tie at length 0, so `max_by_key` falls back to the smallest
    // font as "body" and over-promotes every larger run to a heading.
    for &i in heuristic_pages {
        for seg in &all_page_segments[i] {
            if seg.text.trim().is_empty() {
                continue;
            }
            all_blocks.push(TextBlock {
                text: seg.text.clone(),
                bbox: empty_bbox,
                font_size: seg.font_size,
            });
        }
    }
    for &i in &struct_tree_needs_classify {
        if let Some(paragraphs) = &struct_tree_results[i] {
            for para in paragraphs {
                let text = if !para.text.is_empty() {
                    para.text.clone()
                } else {
                    para.lines
                        .iter()
                        .flat_map(|l| l.segments.iter())
                        .map(|s| s.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                all_blocks.push(TextBlock {
                    text,
                    bbox: empty_bbox,
                    font_size: para.dominant_font_size,
                });
            }
        }
    }

    let heading_map = if all_blocks.is_empty() {
        Vec::new()
    } else {
        let paragraph_count = all_blocks.len();
        let effective_k = if paragraph_count < 20 {
            k_clusters.min(2usize.max(paragraph_count / 4))
        } else {
            k_clusters
        };

        let clusters = cluster_font_sizes(&all_blocks, effective_k)?;
        let mut map = assign_heading_levels_smart(&clusters, MIN_HEADING_FONT_RATIO, MIN_HEADING_FONT_GAP);

        let has_any_heading = map.iter().any(|(_, level)| level.is_some());
        if !has_any_heading && !heuristic_pages.is_empty() {
            let first_page = heuristic_pages[0];
            let first_seg_font = all_page_segments[first_page]
                .iter()
                .find(|s| !s.text.trim().is_empty())
                .map(|s| s.font_size);

            if let Some(first_font) = first_seg_font {
                let mut sizes: Vec<f32> = all_blocks.iter().map(|b| b.font_size).collect();
                sizes.sort_by(|a, b| a.total_cmp(b));
                let median = if sizes.is_empty() { 0.0 } else { sizes[sizes.len() / 2] };

                if median > 0.0
                    && first_font >= median * 1.2
                    && let Some(entry) = map.iter_mut().find(|(fs, _)| (*fs - first_font).abs() < 0.5)
                {
                    entry.1 = Some(1);
                }
            }
        }

        map
    };

    Ok((heading_map, struct_tree_needs_classify))
}

/// Build a heading map from structure-tree-assigned roles on segments.
///
/// Instead of clustering font sizes heuristically, this examines the
/// `assigned_role` field on each segment (populated from the PDF structure tree).
/// Each unique font size is mapped to the heading level most commonly assigned
/// to segments at that size. Font sizes with no assigned role are treated as body text.
fn build_heading_map_from_assigned_roles(all_page_segments: &[Vec<SegmentData>]) -> Vec<(f32, Option<u8>)> {
    use std::collections::HashMap;

    let mut size_roles: HashMap<u32, Vec<Option<u8>>> = HashMap::new();
    for page_segs in all_page_segments {
        for seg in page_segs {
            if seg.text.trim().is_empty() {
                continue;
            }
            let key = (seg.font_size * 10.0).round() as u32;
            size_roles.entry(key).or_default().push(seg.assigned_role);
        }
    }

    let mut heading_map: Vec<(f32, Option<u8>)> = size_roles
        .into_iter()
        .map(|(quantized_size, roles)| {
            let font_size = quantized_size as f32 / 10.0;
            let total = roles.len();
            let mut level_counts: HashMap<u8, usize> = HashMap::new();
            let mut none_count = 0usize;
            for role in &roles {
                match role {
                    Some(level) => *level_counts.entry(*level).or_default() += 1,
                    None => none_count += 1,
                }
            }
            let dominant_level = level_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .and_then(|(level, count)| if count * 2 >= total { Some(level) } else { None });

            if none_count > total / 2 && dominant_level.is_none() {
                (font_size, None)
            } else {
                (font_size, dominant_level)
            }
        })
        .collect();

    heading_map.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    heading_map
}

/// Promote an untagged document-title font tier above structure-tree headings.
///
/// Word processors tag the document title with a non-heading structure type
/// (e.g. LibreOffice's "Title" style resolves to a non-`H*` element), so
/// `build_heading_map_from_assigned_roles` classifies it as body text even
/// though it is visually the top-level heading. When such a tier exists —
/// strictly larger than every tagged heading font, bold, few segments, and
/// present on the first page — assign it level 1 and demote all tagged
/// heading levels by one (Title = h1, tagged H1 = h2, ...), matching the
/// pandoc/HTML convention that the document title outranks section headings.
///
/// Returns `true` when a title tier was promoted. The caller must then also
/// demote the per-segment `assigned_role` values (see
/// `demote_assigned_roles`), because paragraph classification honours
/// `assigned_role` directly, bypassing the heading map.
fn promote_untagged_document_title(
    heading_map: &mut [(f32, Option<u8>)],
    all_page_segments: &[Vec<SegmentData>],
) -> bool {
    /// A title is a handful of segments at most; more means a body/pull-quote tier.
    const MAX_TITLE_SEGMENTS: usize = 3;

    let Some(max_heading_font) = heading_map
        .iter()
        .filter(|(_, level)| level.is_some())
        .map(|(font, _)| *font)
        .fold(None, |acc: Option<f32>, f| Some(acc.map_or(f, |a| a.max(f))))
    else {
        return false;
    };

    let candidate = heading_map
        .iter()
        .position(|(font, level)| level.is_none() && *font > max_heading_font);
    let Some(candidate_idx) = candidate else {
        return false;
    };
    let candidate_font = heading_map[candidate_idx].0;

    let mut tier_segments = 0usize;
    let mut all_bold = true;
    let mut on_first_page = false;
    for (page_idx, page_segs) in all_page_segments.iter().enumerate() {
        for seg in page_segs {
            if seg.text.trim().is_empty() || (seg.font_size - candidate_font).abs() >= 0.05 {
                continue;
            }
            tier_segments += 1;
            all_bold &= seg.is_bold;
            on_first_page |= page_idx == 0;
        }
    }
    if tier_segments == 0 || tier_segments > MAX_TITLE_SEGMENTS || !all_bold || !on_first_page {
        return false;
    }

    tracing::debug!(
        title_font = candidate_font,
        max_heading_font,
        tier_segments,
        "structure tree: promoting untagged document-title tier to h1, demoting tagged levels"
    );
    for (font, level) in heading_map.iter_mut() {
        if let Some(l) = level {
            *level = Some((*l + 1).min(6));
        } else if (*font - candidate_font).abs() < 0.05 {
            *level = Some(1);
        }
    }
    true
}

/// Demote every structure-tree-assigned heading role by one level (capped at 6).
///
/// Companion to `promote_untagged_document_title`: paragraph classification
/// (`bridge.rs`) uses `assigned_role` directly as "the author's stated intent",
/// so the map-level demotion must be mirrored on the segments themselves.
fn demote_assigned_roles(all_page_segments: &mut [Vec<SegmentData>]) {
    for page_segs in all_page_segments.iter_mut() {
        for seg in page_segs.iter_mut() {
            if let Some(role) = seg.assigned_role {
                seg.assigned_role = Some((role + 1).min(6));
            }
        }
    }
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
    /// Y-coordinates of paragraph gaps detected from segment boundaries.
    paragraph_gap_ys: Vec<f32>,
    /// When true, paragraphs classified as `PageHeader` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_headers`.
    include_headers: bool,
    /// When true, paragraphs classified as `PageFooter` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_footers`.
    include_footers: bool,
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
        paragraph_gap_ys,
        include_headers,
        include_footers,
    } = input;

    if let Some(mut paragraphs) = struct_paragraphs {
        apply_text_repair_to_structure_tree_paragraphs(&mut paragraphs, true);
        if needs_classify {
            tracing::debug!(
                page = i,
                "PDF structure pipeline: classifying struct tree page via font-size clustering"
            );
            classify_paragraphs(&mut paragraphs, heading_map);
        }
        merge_continuation_paragraphs(&mut paragraphs);
        if let Some(ref hints) = page_hints {
            super::layout_classify::apply_layout_overrides(&mut paragraphs, hints, 0.5, 0.2, doc_body_font_size);
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers);
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
        tracing::debug!(
            page = i,
            segments = page_segments.len(),
            has_layout_hints = page_hints.is_some(),
            "process_single_page: heuristic path"
        );
        let page_segments = filter_segments_by_table_bboxes(page_segments, &table_bboxes);
        let mut paragraphs = blocks_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys);
        merge_continuation_paragraphs(&mut paragraphs);
        tracing::debug!(
            page = i,
            paragraphs = paragraphs.len(),
            "heuristic paragraphs classified"
        );
        if let Some(ref hints) = page_hints {
            super::layout_classify::apply_layout_overrides(&mut paragraphs, hints, 0.5, 0.2, doc_body_font_size);
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers);
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

/// Multiple of the median line height a whitespace band must exceed to count
/// as a paragraph break. Normal line pitch leaves well under one line height of
/// whitespace; a blank line leaves more than one.
const PARAGRAPH_GAP_HEIGHT_FACTOR: f32 = 1.5;

/// Detect paragraph-break y-positions from horizontal whitespace bands.
///
/// Segments are clustered into visual lines after sorting by y — stream order
/// is not positional (multi-column PDFs interleave columns, which a pairwise
/// scan misreads as phantom gaps). A break is recorded only where the band
/// between two consecutive lines is taller than
/// [`PARAGRAPH_GAP_HEIGHT_FACTOR`] × the median line height: pure blank-line
/// spacing. Bands between two monospace lines are skipped, because code
/// listings legitimately contain blank lines inside one logical block.
///
/// Without this, the heuristic path only breaks paragraphs on font/bold/list
/// changes, fusing visually separated blocks (standalone headings, display
/// formulas) into surrounding prose.
fn compute_paragraph_gap_ys(segments: &[SegmentData]) -> Vec<f32> {
    if segments.len() < 2 {
        return Vec::new();
    }

    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by(|&a, &b| {
        segments[b]
            .y
            .partial_cmp(&segments[a].y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    struct LineBand {
        top: f32,
        bottom: f32,
        height: f32,
        monospace: bool,
        anchor_y: f32,
    }
    let mut lines: Vec<LineBand> = Vec::new();
    for &i in &order {
        let seg = &segments[i];
        let tolerance = (seg.height * 0.5).max(1.0);
        match lines.last_mut() {
            Some(line) if (seg.y - line.anchor_y).abs() <= tolerance => {
                line.top = line.top.max(seg.y + seg.height);
                line.bottom = line.bottom.min(seg.y);
                line.height = line.height.max(seg.height);
                line.monospace &= seg.is_monospace;
            }
            _ => lines.push(LineBand {
                top: seg.y + seg.height,
                bottom: seg.y,
                height: seg.height,
                monospace: seg.is_monospace,
                anchor_y: seg.y,
            }),
        }
    }
    if lines.len() < 2 {
        return Vec::new();
    }

    let mut heights: Vec<f32> = lines.iter().map(|l| l.height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let gap_threshold = heights[heights.len() / 2] * PARAGRAPH_GAP_HEIGHT_FACTOR;

    let mut gap_ys = Vec::new();
    for pair in lines.windows(2) {
        let gap = pair[0].bottom - pair[1].top;
        if gap > gap_threshold && !(pair[0].monospace && pair[1].monospace) {
            gap_ys.push((pair[0].bottom + pair[1].top) / 2.0);
        }
    }
    gap_ys
}

/// Convert a flat list of text segments into grouped paragraphs.
///
/// Groups consecutive segments by font changes, bold changes, list markers, and
/// paragraph gap positions. Each group is then classified via `finalize_paragraph`.
fn blocks_to_paragraphs(
    lines: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
) -> Vec<PdfParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    let gap_info = super::classify::precompute_gap_info(heading_map);

    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<&SegmentData> = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let should_break = if current_lines.is_empty() {
            false
        } else {
            let prev = current_lines.last().unwrap();
            let font_change = (line.font_size - prev.font_size).abs() > 1.5;
            let bold_change = line.is_bold != prev.is_bold;
            let starts_new_line = (line.baseline_y - prev.baseline_y).abs() > 0.5;
            let has_same_line_follower = lines
                .get(line_idx + 1)
                .is_some_and(|next| (next.baseline_y - line.baseline_y).abs() <= 0.5);
            let is_list = looks_like_list_item(&line.text)
                || (starts_new_line && has_same_line_follower && is_bare_list_marker(&line.text));
            let crossed_gap = paragraph_gap_ys.iter().any(|&gap_y| {
                let (upper, lower) = if prev.baseline_y > line.baseline_y {
                    (prev.baseline_y, line.baseline_y)
                } else {
                    (line.baseline_y, prev.baseline_y)
                };
                gap_y < upper && gap_y > lower
            });
            font_change || bold_change || is_list || crossed_gap
        };

        if should_break && !current_lines.is_empty() {
            if let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info) {
                paragraphs.push(para);
            }
            current_lines.clear();
        }
        current_lines.push(line);
    }

    if !current_lines.is_empty()
        && let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info)
    {
        paragraphs.push(para);
    }

    tracing::debug!(
        input_lines = lines.len(),
        output_paragraphs = paragraphs.len(),
        headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
        lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
        "blocks_to_paragraphs complete"
    );

    paragraphs
}

/// Reconstruct PdfLine objects from a flat list of SegmentData, grouping by baseline_y.
///
/// This preserves inline formatting information (is_bold, is_italic, is_monospace)
/// at the segment level so that the assembly layer can emit properly annotated markdown
/// with bold/italic emphasis.
fn reconstruct_pdf_lines(segments: &[&SegmentData]) -> Vec<super::types::PdfLine> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<super::types::PdfLine> = Vec::new();
    let mut current_baseline = segments[0].baseline_y;
    let mut current_segments: Vec<SegmentData> = Vec::new();

    for seg in segments {
        if (seg.baseline_y - current_baseline).abs() > 0.5 {
            if !current_segments.is_empty() {
                let dominant_font_size = current_segments.iter().map(|s| s.font_size).fold(0.0, |a, b| {
                    if a > 0.0 && b > a / 2.0 && b < a * 2.0 {
                        (a + b) / 2.0
                    } else {
                        a.max(b)
                    }
                });
                let is_bold = current_segments.iter().filter(|s| s.is_bold).count() > current_segments.len() / 2;
                let is_monospace = current_segments.iter().all(|s| s.is_monospace);
                lines.push(super::types::PdfLine {
                    segments: current_segments.clone(),
                    baseline_y: current_baseline,
                    dominant_font_size,
                    is_bold,
                    is_monospace,
                });
            }
            current_baseline = seg.baseline_y;
            current_segments.clear();
        }
        current_segments.push((*seg).clone());
    }

    if !current_segments.is_empty() {
        let dominant_font_size = current_segments.iter().map(|s| s.font_size).fold(0.0, |a, b| {
            if a > 0.0 && b > a / 2.0 && b < a * 2.0 {
                (a + b) / 2.0
            } else {
                a.max(b)
            }
        });
        let is_bold = current_segments.iter().filter(|s| s.is_bold).count() > current_segments.len() / 2;
        let is_monospace = current_segments.iter().all(|s| s.is_monospace);
        lines.push(super::types::PdfLine {
            segments: current_segments,
            baseline_y: current_baseline,
            dominant_font_size,
            is_bold,
            is_monospace,
        });
    }

    lines
}

/// Build a PdfParagraph from a group of consecutive lines with compatible font properties.
fn finalize_paragraph(
    lines: &[&SegmentData],
    heading_map: &[(f32, Option<u8>)],
    gap_info: &super::classify::GapInfo,
) -> Option<PdfParagraph> {
    if lines.is_empty() {
        return None;
    }

    let text: String = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first = lines[0];
    let word_count = trimmed.split_whitespace().count();
    let is_bold = lines.iter().filter(|l| l.is_bold).count() > lines.len() / 2;

    let reconstructed_lines = reconstruct_pdf_lines(lines);

    let structure_tree_role = {
        let role_counts: std::collections::HashMap<u8, usize> =
            lines
                .iter()
                .filter_map(|l| l.assigned_role)
                .fold(std::collections::HashMap::new(), |mut acc, level| {
                    *acc.entry(level).or_default() += 1;
                    acc
                });
        role_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(level, _)| level)
    };
    if let Some(level) = structure_tree_role {
        let para_text = trimmed.to_string();
        let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);
        return Some(PdfParagraph {
            text: para_text,
            lines: reconstructed_lines,
            dominant_font_size: first.font_size,
            heading_level: Some(level),
            is_bold,
            is_list_item: looks_like_list_item(trimmed),
            is_code_block: first.is_monospace && lines.len() > 1,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        });
    }

    let page_number_like = word_count <= 10 && is_page_number_pattern(trimmed);

    let mut heading_level = super::classify::find_heading_level(first.font_size, heading_map, gap_info);
    if heading_level.is_some()
        && (word_count > 20 || super::layout_classify::is_separator_text(trimmed) || page_number_like)
    {
        heading_level = None;
    }

    if heading_level.is_none()
        && is_bold
        && (1..=8).contains(&word_count)
        && lines.len() == 1
        && !trimmed.ends_with('.')
        && !trimmed.ends_with(':')
        && !trimmed.ends_with(',')
        && !trimmed.ends_with(';')
        && !trimmed.contains('@')
        && !trimmed.contains('(')
        && !trimmed.contains(',')
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
        && !super::layout_classify::is_separator_text(trimmed)
        && !super::regions::looks_like_figure_label(trimmed)
    {
        heading_level = Some(2);
    }

    if heading_level.is_none() {
        let body_font_size = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(centroid, _)| *centroid)
            .unwrap_or(0.0);
        let min_heading_threshold = body_font_size * super::constants::MIN_HEADING_FONT_RATIO;
        if body_font_size > 0.0
            && first.font_size >= min_heading_threshold
            && first.font_size > body_font_size + 0.5
            && word_count <= super::constants::MAX_BOLD_HEADING_WORD_COUNT
            && lines.len() <= 2
            && !trimmed.ends_with(':')
            && !trimmed.contains('@')
            && (super::classify::is_section_pattern(trimmed) || is_structural_heading_word(trimmed))
            && !super::layout_classify::is_separator_text(trimmed)
            && !super::regions::looks_like_figure_label(trimmed)
            && !looks_like_list_item(trimmed)
            && !page_number_like
        {
            heading_level = Some(2);
        }
    }

    let is_list_item = heading_level.is_none() && looks_like_list_item(trimmed);
    let is_code_block =
        heading_level.is_none() && !is_list_item && lines.iter().all(|l| l.is_monospace) && lines.len() >= 2;

    let is_page_furniture = heading_level.is_none()
        && !is_list_item
        && !is_code_block
        && word_count <= 10
        && is_page_number_pattern(trimmed);

    tracing::debug!(
        font_size = first.font_size,
        is_bold,
        word_count,
        heading_level = ?heading_level,
        is_list_item,
        is_code_block,
        is_page_furniture,
        text_preview = %&trimmed.chars().take(60).collect::<String>(),
        "classified paragraph"
    );

    let para_text = trimmed.to_string();
    let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);

    Some(PdfParagraph {
        text: para_text,
        lines: reconstructed_lines,
        dominant_font_size: first.font_size,
        heading_level,
        is_bold,
        is_list_item,
        is_code_block,
        is_formula: false,
        is_page_furniture,
        layout_class: None,
        caption_for: None,
        block_bbox: Some({
            let left = lines.iter().map(|l| l.x).fold(f32::MAX, f32::min);
            let bottom = lines.iter().map(|l| l.baseline_y).fold(f32::MAX, f32::min);
            let right = lines.iter().map(|l| l.x + l.width).fold(f32::MIN, f32::max);
            let top = lines.iter().map(|l| l.baseline_y + l.height).fold(f32::MIN, f32::max);
            (left, bottom, right, top)
        }),
        word_count,
    })
}

/// Check if text is ENTIRELY a list marker with no item text after it.
///
/// Word processors often emit list numbering as its own text run, so the
/// marker ("1.", "a)", "(2)", "•") and the item body arrive as separate
/// spans on the same line. `looks_like_list_item` rejects those markers
/// because it requires trailing text; this predicate accepts them.
fn is_bare_list_marker(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 5 {
        return false;
    }
    if matches!(t, "•" | "·" | "◦" | "▪" | "–" | "—" | "-" | "*") {
        return true;
    }
    if let Some(inner) = t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        return !inner.is_empty() && inner.chars().all(|c| c.is_alphanumeric());
    }
    if let Some(body) = t.strip_suffix('.').or_else(|| t.strip_suffix(')')) {
        return !body.is_empty() && body.chars().count() <= 2 && body.chars().all(|c| c.is_alphanumeric());
    }
    false
}

/// Check if text starts with a common list marker.
fn looks_like_list_item(text: &str) -> bool {
    let t = text.trim_start();

    if t.starts_with('•')
        || t.starts_with('·')
        || t.starts_with('◦')
        || t.starts_with('▪')
        || t.starts_with('–')
        || t.starts_with('—')
    {
        return true;
    }

    if let Some(rest) = t.strip_prefix("- ") {
        return rest.chars().next().is_some_and(|c| c.is_alphabetic());
    }

    let mut chars = t.chars().peekable();

    if chars.peek() == Some(&'(') {
        chars.next();
        if chars.peek().is_some_and(|c| c.is_alphanumeric()) {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_alphanumeric()) {
                chars.next();
            }
            if chars.peek() == Some(&')') {
                chars.next();
                return chars.peek().is_some_and(|c| c.is_whitespace()) && {
                    chars.next();
                    chars.peek().is_some_and(|c| c.is_alphabetic())
                };
            }
        }
        return false;
    }

    if super::classify::is_numbered_section_heading(t) {
        return false;
    }

    if chars.peek().is_some_and(|c| c.is_alphanumeric()) {
        let mut num_len = 0;
        let mut all_digits = true;
        let mut all_roman = true;
        while let Some(&c) = chars.peek() {
            if !c.is_alphanumeric() {
                break;
            }
            all_digits &= c.is_ascii_digit();
            all_roman &= matches!(c.to_ascii_lowercase(), 'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm');
            chars.next();
            num_len += 1;
        }
        let marker_like = all_digits || num_len == 1 || all_roman;
        if num_len <= 4 && marker_like && (chars.peek() == Some(&'.') || chars.peek() == Some(&')')) {
            chars.next();
            return chars.peek().is_some_and(|c| c.is_whitespace()) && {
                chars.next();
                chars.peek().is_some_and(|c| c.is_alphabetic())
            };
        }
    }

    false
}

/// Check if text is a well-known structural heading word.
///
/// These single-word headings appear frequently in academic papers and reports
/// and are reliable heading indicators when combined with a larger-than-body font.
fn is_structural_heading_word(text: &str) -> bool {
    let t = text.trim();
    matches!(
        t,
        "Abstract"
            | "References"
            | "Appendix"
            | "Acknowledgments"
            | "Acknowledgements"
            | "Conclusion"
            | "Conclusions"
            | "Bibliography"
            | "Contents"
            | "Index"
            | "Glossary"
            | "Summary"
            | "Discussion"
            | "Methods"
            | "Results"
            | "Methodology"
    )
}

/// Check if text matches common page number patterns.
///
/// Detects standalone page numbers, "Page X", "Page X of Y", Roman numerals,
/// and similar patterns that appear as page furniture.
fn is_page_number_pattern(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.chars().all(|c| c.is_ascii_digit()) && t.len() <= 4 {
        return true;
    }
    let lower = t.to_lowercase();
    if lower.starts_with("page ") {
        return true;
    }
    if (t.starts_with("- ") || t.starts_with("– ")) && (t.ends_with(" -") || t.ends_with(" –")) {
        let inner = t
            .trim_start_matches("- ")
            .trim_start_matches("– ")
            .trim_end_matches(" -")
            .trim_end_matches(" –")
            .trim();
        if inner.chars().all(|c| c.is_ascii_digit()) && inner.len() <= 4 {
            return true;
        }
    }
    if t.len() <= 5 && t.chars().all(|c| matches!(c, 'i' | 'v' | 'x' | 'I' | 'V' | 'X')) {
        return true;
    }
    false
}

/// Build a structured `InternalDocument` from pre-extracted per-page segments.
///
/// This is the oxide-backend entry point. It accepts segments already extracted
/// via `oxide::hierarchy::extract_all_segments` and runs the same font-clustering,
/// heading-classification, paragraph-assembly, and post-processing stages without
/// requiring a PDF document.
///
/// Image positions can be supplied to insert image placeholders into the document.
/// Layout hints (from RT-DETR layout detection) are optional; when present they
/// drive furniture marking, heading overrides, and table region detection.
///
/// Returns the assembled `InternalDocument`.
pub(crate) struct SegmentStructureConfig<'a> {
    pub k_clusters: usize,
    pub tables: &'a [crate::types::Table],
    pub strip_repeating_text: bool,
    pub include_headers: bool,
    pub include_footers: bool,
    pub used_structure_tree: bool,
    pub image_positions: &'a [(u32, u32)],
    pub images: Option<&'a [crate::types::ExtractedImage]>,
    pub inject_placeholders: bool,
    pub layout_hints: Option<&'a [Vec<LayoutHint>]>,
    pub allow_single_column: bool,
    pub cancel_token: Option<&'a crate::cancellation::CancellationToken>,
    #[cfg(feature = "layout-detection")]
    pub layout_images: Option<&'a [image::RgbImage]>,
    #[cfg(feature = "layout-detection")]
    pub layout_results: Option<&'a [super::types::PageLayoutResult]>,
    #[cfg(feature = "layout-detection")]
    pub table_model: crate::core::config::layout::TableModel,
    #[cfg(feature = "layout-detection")]
    pub table_overlap_preference: crate::core::config::layout::TableOverlapPreference,
    #[cfg(feature = "layout-detection")]
    pub acceleration: Option<&'a crate::core::config::acceleration::AccelerationConfig>,
}

pub(crate) fn extract_document_structure_from_segments(
    mut all_page_segments: Vec<Vec<SegmentData>>,
    config: SegmentStructureConfig<'_>,
) -> Result<crate::types::internal::InternalDocument> {
    let SegmentStructureConfig {
        k_clusters,
        tables,
        strip_repeating_text,
        include_headers,
        include_footers,
        used_structure_tree,
        image_positions,
        images,
        inject_placeholders,
        layout_hints,
        allow_single_column,
        cancel_token,
        #[cfg(feature = "layout-detection")]
        layout_images,
        #[cfg(feature = "layout-detection")]
        layout_results,
        #[cfg(feature = "layout-detection")]
        table_model,
        #[cfg(feature = "layout-detection")]
        table_overlap_preference,
        #[cfg(feature = "layout-detection")]
        acceleration,
    } = config;
    let page_count = all_page_segments.len();
    tracing::debug!(
        page_count,
        used_structure_tree,
        "oxide structure pipeline: starting from pre-extracted segments"
    );

    let struct_tree_results: Vec<Option<Vec<PdfParagraph>>> = vec![None; page_count];
    let heuristic_pages: Vec<usize> = (0..page_count).collect();

    let (heading_map, doc_body_font_size) = if used_structure_tree {
        let mut heading_map = build_heading_map_from_assigned_roles(&all_page_segments);
        if promote_untagged_document_title(&mut heading_map, &all_page_segments) {
            demote_assigned_roles(&mut all_page_segments);
        }
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        tracing::debug!(
            heading_map_len = heading_map.len(),
            "oxide structure pipeline: heading map from structure tree"
        );
        (heading_map, doc_body_font_size)
    } else {
        let (heading_map, _struct_tree_needs_classify) =
            build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)?;
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        (heading_map, doc_body_font_size)
    };

    let page_heights: Vec<f32> = all_page_segments
        .iter()
        .map(|segs| segs.iter().map(|s| s.y + s.height).fold(0.0_f32, f32::max).max(792.0))
        .collect();

    let mut layout_tables: Vec<crate::types::Table> = Vec::new();
    if let Some(hints_pages) = layout_hints {
        struct TablePageData {
            page_idx: usize,
            words: Vec<crate::pdf::table_reconstruct::HocrWord>,
            page_height: f32,
        }
        let mut table_pages: Vec<TablePageData> = Vec::new();

        #[allow(clippy::needless_range_loop)]
        for page_idx in 0..page_count {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!(page_idx, "oxide structure pipeline: cancelled during table page prep");
                break;
            }
            let Some(hints) = hints_pages.get(page_idx) else {
                continue;
            };
            if !hints
                .iter()
                .any(|h| h.class_name == super::types::LayoutHintClass::Table)
            {
                continue;
            }
            #[cfg(feature = "layout-detection")]
            let page_height = layout_results
                .and_then(|results| results.get(page_idx))
                .map(|pr| pr.page_height_pts)
                .unwrap_or(page_heights[page_idx]);
            #[cfg(not(feature = "layout-detection"))]
            let page_height = page_heights[page_idx];
            let words = crate::pdf::table_reconstruct::segments_to_words(&all_page_segments[page_idx], page_height);
            if words.is_empty() {
                tracing::trace!(
                    page = page_idx,
                    "oxide layout table extraction: no words from segments, skipping"
                );
                continue;
            }
            tracing::trace!(
                page = page_idx,
                word_count = words.len(),
                page_height,
                "oxide layout table extraction: page prepared"
            );
            table_pages.push(TablePageData {
                page_idx,
                words,
                page_height,
            });
        }

        #[cfg(feature = "layout-detection")]
        {
            use crate::core::config::layout::TableModel;
            use std::cell::RefCell;

            let use_model_inference = table_model != TableModel::Disabled;

            thread_local! {
                static TL_TATR: RefCell<Option<crate::layout::models::tatr::TatrModel>> = const { RefCell::new(None) };
                static TL_SLANET: RefCell<Option<crate::layout::models::slanet::SlanetModel>> = const { RefCell::new(None) };
                static TL_SLANET_ALT: RefCell<Option<crate::layout::models::slanet::SlanetModel>> = const { RefCell::new(None) };
                static TL_CLASSIFIER: RefCell<Option<crate::layout::models::table_classifier::TableClassifier>> = const { RefCell::new(None) };
            }

            let slanet_variant = match table_model {
                TableModel::SlanetWired => Some("slanet_wired"),
                TableModel::SlanetWireless => Some("slanet_wireless"),
                TableModel::SlanetPlus => Some("slanet_plus"),
                TableModel::SlanetAuto => Some("slanet_wired"),
                TableModel::Tatr | TableModel::Disabled => None,
            };
            let is_auto = table_model == TableModel::SlanetAuto;

            let model_name = match table_model {
                TableModel::Tatr => "TATR",
                TableModel::SlanetWired | TableModel::SlanetWireless | TableModel::SlanetPlus => "SLANeXT",
                TableModel::SlanetAuto => "SLANeXT (auto)",
                TableModel::Disabled => "disabled",
            };

            let has_table_model = if use_model_inference {
                let available = match table_model {
                    TableModel::Tatr => crate::layout::is_tatr_available(),
                    TableModel::SlanetWired
                    | TableModel::SlanetWireless
                    | TableModel::SlanetPlus
                    | TableModel::SlanetAuto => crate::layout::is_slanet_available(),
                    TableModel::Disabled => false,
                };

                if !available && !table_pages.is_empty() {
                    return Err(crate::pdf::error::PdfError::TextExtractionFailed(format!(
                        "Layout detection found table regions but {model_name} model is not available. \
                         Ensure the ONNX model is downloaded. Tables cannot be extracted without it."
                    )));
                }
                available
            } else {
                false
            };

            if has_table_model {
                if let (Some(images @ [_, ..]), Some(results @ [_, ..])) = (layout_images, layout_results) {
                    #[cfg(not(target_arch = "wasm32"))]
                    let parallel_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .par_iter()
                        .map(|tp| {
                            if let Some(variant) = slanet_variant {
                                TL_SLANET.with(|cell| {
                                    let mut slanet_ref = cell.borrow_mut();
                                    if slanet_ref.is_none() {
                                        *slanet_ref = crate::layout::take_or_create_slanet(variant, acceleration);
                                    }
                                });
                                if is_auto {
                                    TL_SLANET_ALT.with(|cell| {
                                        let mut alt_ref = cell.borrow_mut();
                                        if alt_ref.is_none() {
                                            *alt_ref =
                                                crate::layout::take_or_create_slanet("slanet_wireless", acceleration);
                                        }
                                    });
                                    TL_CLASSIFIER.with(|cell| {
                                        let mut cls_ref = cell.borrow_mut();
                                        if cls_ref.is_none() {
                                            *cls_ref = crate::layout::take_or_create_table_classifier(acceleration);
                                        }
                                    });
                                }

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

                                        let mut classifier_pair = if is_auto {
                                            let alt = TL_SLANET_ALT.with(|c| c.borrow_mut().take());
                                            let cls = TL_CLASSIFIER.with(|c| c.borrow_mut().take());
                                            match (cls, alt) {
                                                (Some(c), Some(a)) => Some((c, a)),
                                                (c, a) => {
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

                                        let classifier_arg = classifier_pair.as_mut().map(|(cls, alt)| {
                                            (
                                                cls as &mut crate::layout::models::table_classifier::TableClassifier,
                                                alt as &mut crate::layout::models::slanet::SlanetModel,
                                            )
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
                                TL_TATR.with(|cell| {
                                    let mut tatr_ref = cell.borrow_mut();
                                    if tatr_ref.is_none() {
                                        *tatr_ref = crate::layout::take_or_create_tatr(acceleration);
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
                                        if !tatr_tables.is_empty() {
                                            return tatr_tables;
                                        }
                                    }

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
                    #[cfg(target_arch = "wasm32")]
                    let parallel_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .iter()
                        .map(|tp| {
                            if let (Some(page_image), Some(page_result)) =
                                (images.get(tp.page_idx), results.get(tp.page_idx))
                            {
                                let hints = &hints_pages[tp.page_idx];
                                TL_TATR.with(|cell| {
                                    let mut tatr_ref = cell.borrow_mut();
                                    if tatr_ref.is_none() {
                                        *tatr_ref = crate::layout::take_or_create_tatr(acceleration);
                                    }
                                    let Some(tatr) = tatr_ref.as_mut() else {
                                        return Vec::new();
                                    };
                                    let tatr_tables = super::regions::recognize_tables_for_native_page(
                                        page_image,
                                        hints,
                                        &tp.words,
                                        page_result,
                                        tp.page_height,
                                        tp.page_idx,
                                        tatr,
                                    );
                                    if !tatr_tables.is_empty() {
                                        return tatr_tables;
                                    }
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
                                Vec::new()
                            }
                        })
                        .collect();
                    layout_tables.extend(parallel_tables.into_iter().flatten());
                } else {
                    for tp in &table_pages {
                        if cancel_token.is_some_and(|t| t.is_cancelled()) {
                            tracing::debug!("oxide structure pipeline: cancelled during heuristic table extraction");
                            break;
                        }
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
            } else {
                for tp in &table_pages {
                    if cancel_token.is_some_and(|t| t.is_cancelled()) {
                        tracing::debug!("oxide structure pipeline: cancelled during heuristic table extraction");
                        break;
                    }
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
        for tp in &table_pages {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("oxide structure pipeline: cancelled during heuristic table extraction");
                break;
            }
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

    tracing::debug!(
        layout_tables_found = layout_tables.len(),
        "oxide layout table extraction complete"
    );

    let extracted_table_bboxes_by_page: ahash::AHashMap<usize, Vec<crate::types::BoundingBox>> = {
        let mut map: ahash::AHashMap<usize, Vec<crate::types::BoundingBox>> = ahash::AHashMap::new();
        for table in tables.iter().chain(layout_tables.iter()) {
            if let Some(ref bb) = table.bounding_box {
                map.entry(table.page_number.saturating_sub(1) as usize)
                    .or_default()
                    .push(*bb);
            }
        }
        tracing::debug!(
            native_tables = tables.len(),
            layout_tables = layout_tables.len(),
            pages_with_bboxes = map.len(),
            "oxide table bbox suppression map built"
        );
        map
    };

    #[cfg(feature = "layout-detection")]
    let validations_by_page: ahash::AHashMap<usize, Vec<super::regions::layout_validation::RegionValidation>> = {
        let mut map = ahash::AHashMap::new();
        if let (Some(images), Some(results), Some(hints_pages)) = (layout_images, layout_results, layout_hints) {
            for page_idx in 0..page_count {
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
                            "oxide layout validation: found empty regions"
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

    let effective_layout_hints = layout_hints;
    let page_inputs: Vec<PageInput> = (0..page_count)
        .map(|i| {
            let heuristic_segments = std::mem::take(&mut all_page_segments[i]);
            let paragraph_gap_ys = compute_paragraph_gap_ys(&heuristic_segments);
            PageInput {
                page_index: i,
                struct_paragraphs: None,
                heuristic_segments,
                page_hints: effective_layout_hints.and_then(|h| h.get(i)).cloned(),
                table_bboxes: extracted_table_bboxes_by_page.get(&i).cloned().unwrap_or_default(),
                hint_validations: validations_by_page.get(&i).cloned().unwrap_or_default(),
                needs_classify: false,
                paragraph_gap_ys,
                include_headers,
                include_footers,
            }
        })
        .collect();

    if cancel_token.is_some_and(|t| t.is_cancelled()) {
        return Err(crate::pdf::error::PdfError::TextExtractionFailed(
            "extraction cancelled".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_par_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size))
        .collect();
    #[cfg(target_arch = "wasm32")]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size))
        .collect();

    refine_heading_hierarchy(&mut all_page_paragraphs);
    demote_unnumbered_subsections(&mut all_page_paragraphs);
    demote_heading_runs(&mut all_page_paragraphs);

    if strip_repeating_text {
        mark_cross_page_repeating_text(&mut all_page_paragraphs, &page_heights);
        mark_cross_page_repeating_short_text(&mut all_page_paragraphs);
    }
    mark_arxiv_noise(&mut all_page_paragraphs);
    for page in &mut all_page_paragraphs {
        retain_page_furniture_safely(page);
    }
    if strip_repeating_text {
        deduplicate_paragraphs(&mut all_page_paragraphs);
    }

    let total_paragraphs: usize = all_page_paragraphs.iter().map(|p| p.len()).sum();
    tracing::debug!(
        total_paragraphs,
        heading_map_len = heading_map.len(),
        "oxide structure pipeline: paragraph extraction complete, assembling document"
    );

    let native_count = tables.len();
    let mut combined_tables: Vec<crate::types::Table> = tables.iter().cloned().chain(layout_tables).collect();
    #[cfg(feature = "layout-detection")]
    let overlap_preference = table_overlap_preference;
    #[cfg(not(feature = "layout-detection"))]
    let overlap_preference = crate::core::config::layout::TableOverlapPreference::Content;
    deduplicate_overlapping_tables(&mut combined_tables, native_count, overlap_preference);
    let effective_image_positions = if inject_placeholders { image_positions } else { &[] };
    let mut doc = assemble_internal_document(all_page_paragraphs, &combined_tables, images, effective_image_positions);

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

    tracing::debug!(
        elements = doc.elements.len(),
        "oxide structure pipeline: assembly complete"
    );

    Ok(doc)
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

/// Apply all 5 text repair passes in a single traversal over a segment's text.
///
/// Returns `Cow::Borrowed` if nothing changed, `Cow::Owned` otherwise.
fn fused_text_repairs(text: &str) -> Cow<'_, str> {
    let t1 = normalize_text_encoding(text);
    let t2 = repair_ligature_spaces(&t1);
    let t3 = expand_ligatures_with_space_absorption(&t2);
    let t3b = collapse_spaced_hyphens(&t3);
    let t4 = normalize_unicode_text(&t3b);
    let t5 = clean_duplicate_punctuation(&t4);
    match (&t1, &t2, &t3, &t3b, &t4, &t5) {
        (
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
        ) => Cow::Borrowed(text),
        _ => Cow::Owned(t5.into_owned()),
    }
}

/// Deduplicate tables that overlap on the same page.
///
/// When both native oxide detection and layout-based table extraction produce tables
/// for the same region, they can overlap. Tables at index `< native_count` are native;
/// the rest are layout (TATR/SLANeXT) tables. `preference` decides which side wins for a
/// mixed native/layout overlap; for same-origin overlaps (or [`TableOverlapPreference::Content`])
/// the table with more content (cell count + markdown length) is kept.
fn deduplicate_overlapping_tables(
    tables: &mut Vec<crate::types::Table>,
    native_count: usize,
    preference: crate::core::config::layout::TableOverlapPreference,
) {
    use crate::core::config::layout::TableOverlapPreference;

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
            if let (Some(a), Some(b)) = (&tables[i].bounding_box, &tables[j].bounding_box) {
                let inter_x = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
                let inter_y = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
                let intersection = inter_x * inter_y;
                let area_a = (a.x1 - a.x0) * (a.y1 - a.y0);
                let area_b = (b.x1 - b.x0) * (b.y1 - b.y0);
                let min_area = area_a.min(area_b);

                if min_area > 0.0 && intersection / min_area > 0.5 {
                    let i_is_native = i < native_count;
                    let j_is_native = j < native_count;
                    let mixed_origin = i_is_native != j_is_native;
                    let remove = match preference {
                        TableOverlapPreference::Native if mixed_origin => {
                            if i_is_native {
                                j
                            } else {
                                i
                            }
                        }
                        TableOverlapPreference::Layout if mixed_origin => {
                            if i_is_native {
                                i
                            } else {
                                j
                            }
                        }
                        _ => {
                            let content_a = tables[i].cells.len() + tables[i].markdown.len();
                            let content_b = tables[j].cells.len() + tables[j].markdown.len();
                            if content_a >= content_b { j } else { i }
                        }
                    };
                    to_remove.insert(remove);
                    if remove == i {
                        break;
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

/// Clear `is_page_furniture` on paragraphs whose `layout_class` was set to
/// `PageHeader` or `PageFooter` by the layout model, when the caller has opted
/// in to keeping those regions via `include_headers` / `include_footers`.
///
/// This must run **before** `retain_page_furniture_safely`, which physically
/// removes furniture paragraphs via `.retain()`. Un-marking here ensures that
/// user-opted-in header/footer paragraphs survive that pass.
fn un_mark_layout_furniture_per_config(paragraphs: &mut [PdfParagraph], include_headers: bool, include_footers: bool) {
    if !include_headers && !include_footers {
        return;
    }
    for para in paragraphs.iter_mut() {
        if !para.is_page_furniture {
            continue;
        }
        match para.layout_class {
            Some(super::types::LayoutHintClass::PageHeader) if include_headers => {
                para.is_page_furniture = false;
            }
            Some(super::types::LayoutHintClass::PageFooter) if include_footers => {
                para.is_page_furniture = false;
            }
            _ => {}
        }
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
        return;
    }

    if furniture_count >= total {
        for para in paragraphs.iter_mut() {
            para.is_page_furniture = false;
        }
        return;
    }

    let total_alphanum: usize = paragraphs.iter().map(paragraph_alphanum_len).sum();

    if total_alphanum > 0 {
        let furniture_alphanum: usize = paragraphs
            .iter()
            .filter(|p| p.is_page_furniture)
            .map(paragraph_alphanum_len)
            .sum();

        if furniture_alphanum * 100 > total_alphanum * 30 {
            for para in paragraphs.iter_mut() {
                para.is_page_furniture = false;
            }
            return;
        }
    }

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
    let max_right_edge = para
        .lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.x + s.width)
        .fold(0.0_f32, f32::max);

    if max_right_edge <= 0.0 {
        dehyphenate_hyphen_only(para);
        return;
    }

    let threshold = max_right_edge * FULL_LINE_FRACTION;

    let n = para.lines.len();
    for i in 0..(n - 1) {
        let trailing_right = para.lines[i].segments.last().map(|s| s.x + s.width).unwrap_or(0.0);
        if trailing_right < threshold {
            continue;
        }

        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let has_trailing_hyphen = trailing_text.ends_with('-');
        if !has_trailing_hyphen {
            continue;
        }

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let joined_word = format!("{trailing_word}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Hyphen-only dehyphenation (no position data required).
///
/// Only joins lines when the trailing segment ends with an explicit hyphen.
/// Used for structure tree pages where x/width may be zero.
fn dehyphenate_hyphen_only(para: &mut PdfParagraph) {
    let n = para.lines.len();
    for i in 0..(n - 1) {
        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if s.text.ends_with('-') => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let joined_word = format!("{trailing_word}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Detect whether a set of paragraphs contains any font-size variation.
///
/// Variation is defined as any paragraph whose font size differs from the first
/// non-zero size by more than 0.5pt. Used to decide whether structure-tree pages
/// need font-size clustering for heading assignment.
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

        for &idx in to_remove.iter().rev() {
            page.remove(idx);
        }
    }
}

/// Normalize paragraph text for deduplication comparison.
///
/// Uses `para.text` when populated (heuristic path), otherwise assembles text
/// from segment data (structure tree path, used in tests).
fn paragraph_text_normalized(para: &PdfParagraph) -> String {
    let raw = if para.text.is_empty() {
        para.lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        para.text.clone()
    };
    raw.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Check if a paragraph is a candidate for non-consecutive deduplication.
fn is_dedup_candidate(p: &PdfParagraph) -> bool {
    p.heading_level.is_none()
        && !p.is_list_item
        && !p.is_code_block
        && !p.is_formula
        && !p.is_page_furniture
        && p.caption_for.is_none()
}

fn apply_text_repair_to_structure_tree_paragraphs(paragraphs: &mut Vec<PdfParagraph>, has_positions: bool) {
    apply_to_all_segments(paragraphs, fused_text_repairs);
    dehyphenate_paragraphs(paragraphs, has_positions);
    split_embedded_list_items(paragraphs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::{PdfLine, PdfParagraph};

    /// Helper: a table at `bbox` on `page` whose only content is `markdown`
    /// (so content weight == markdown length; empty cells).
    fn ov_table(page: u32, bbox: (f64, f64, f64, f64), markdown: &str) -> crate::types::Table {
        let (x0, y0, x1, y1) = bbox;
        crate::types::Table {
            cells: Vec::new(),
            markdown: markdown.to_string(),
            page_number: page,
            bounding_box: Some(crate::types::BoundingBox { x0, y0, x1, y1 }),
        }
    }

    #[test]
    fn dedup_content_preference_keeps_larger_table() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Content);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "bbbbbbbbbb",
            "Content keeps the larger (layout) table"
        );
    }

    #[test]
    fn dedup_native_preference_keeps_native_even_when_smaller() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "a",
            "Native preference keeps native over a larger layout table"
        );
    }

    #[test]
    fn dedup_layout_preference_keeps_layout_even_when_smaller() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "aaaaaaaaaa"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "b"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Layout);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "b",
            "Layout preference keeps layout over a larger native table"
        );
    }

    #[test]
    fn dedup_native_preference_falls_back_to_content_for_same_origin() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 2, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "bbbbbbbbbb",
            "same-origin overlap falls back to content"
        );
    }

    #[test]
    fn dedup_non_overlapping_tables_both_kept() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (200.0, 200.0, 300.0, 300.0), "b"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 2, "non-overlapping tables are both kept");
    }

    /// Helper: segment with font metadata for title-promotion tests.
    fn role_seg(text: &str, font_size: f32, is_bold: bool, assigned_role: Option<u8>) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 72.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            assigned_role,
        }
    }

    /// A bold, first-page, larger-than-any-tagged-heading tier must be promoted
    /// to h1 with the tagged hierarchy shifted down one level.
    #[test]
    fn promote_title_shifts_tagged_heading_levels_down() {
        let pages = vec![vec![
            role_seg("Titre du document", 28.0, true, None),
            role_seg("Titre 1", 18.0, true, Some(1)),
            role_seg("Titre 2", 16.0, true, Some(2)),
            role_seg("body text", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(promote_untagged_document_title(&mut map, &pages));

        let level_of = |font: f32| map.iter().find(|(f, _)| (*f - font).abs() < 0.05).and_then(|(_, l)| *l);
        assert_eq!(level_of(28.0), Some(1), "title tier must become h1");
        assert_eq!(level_of(18.0), Some(2), "tagged H1 must demote to h2");
        assert_eq!(level_of(16.0), Some(3), "tagged H2 must demote to h3");
        assert_eq!(level_of(12.0), None, "body must stay body");
    }

    /// No untagged tier above the largest tagged heading → no promotion.
    #[test]
    fn promote_title_no_candidate_leaves_map_unchanged() {
        let pages = vec![vec![
            role_seg("Heading", 18.0, true, Some(1)),
            role_seg("body", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        let before = map.clone();
        assert!(!promote_untagged_document_title(&mut map, &pages));
        assert_eq!(map, before);
    }

    /// A non-bold large tier (e.g. a pull quote) must not be mistaken for a title.
    #[test]
    fn promote_title_requires_bold() {
        let pages = vec![vec![
            role_seg("large quote", 28.0, false, None),
            role_seg("Heading", 18.0, true, Some(1)),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!promote_untagged_document_title(&mut map, &pages));
    }

    /// A large tier appearing only after page 0 is not a document title.
    #[test]
    fn promote_title_requires_first_page() {
        let pages = vec![
            vec![role_seg("Heading", 18.0, true, Some(1))],
            vec![role_seg("Big banner later", 28.0, true, None)],
        ];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!promote_untagged_document_title(&mut map, &pages));
    }

    /// Role demotion mirrors the map shift on segments (bridge.rs reads roles directly).
    #[test]
    fn demote_assigned_roles_shifts_and_caps() {
        let mut pages = vec![vec![
            role_seg("h1", 18.0, true, Some(1)),
            role_seg("h6", 8.0, true, Some(6)),
            role_seg("body", 12.0, false, None),
        ]];
        demote_assigned_roles(&mut pages);
        assert_eq!(pages[0][0].assigned_role, Some(2));
        assert_eq!(pages[0][1].assigned_role, Some(6), "level 6 must cap, not overflow");
        assert_eq!(pages[0][2].assigned_role, None);
    }

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
            assigned_role: None,
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
        let word_count = PdfParagraph::compute_word_count("", &lines);
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
            word_count,
        }
    }

    fn paragraph_text(paragraph: &PdfParagraph) -> String {
        paragraph
            .lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn test_structure_tree_page_runs_text_repair_before_assembly() {
        let paragraphs = vec![para(vec![line(vec![seg(
            "Intro\u{00AD}duction, , body • first item • second item",
            10.0,
            320.0,
        )])])];

        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: Some(paragraphs),
                heuristic_segments: Vec::new(),
                page_hints: None,
                table_bboxes: Vec::new(),
                hint_validations: Vec::new(),
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
            },
            &[],
            None,
        );

        assert_eq!(output.len(), 3);
        assert_eq!(paragraph_text(&output[0]), "Introduction, body");
        assert!(output[1].is_list_item);
        assert_eq!(paragraph_text(&output[1]), "first item");
        assert!(output[2].is_list_item);
        assert_eq!(paragraph_text(&output[2]), "second item");
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
        assert_eq!(p.lines[0].segments[0].text, "some \u{4E00}-");
    }

    #[test]
    fn test_real_world_software_no_join_without_hyphen() {
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
        let mut p = para(vec![
            line(vec![seg("first part", 10.0, 200.0), seg("soft", 220.0, 280.0)]),
            line(vec![seg("ware next words", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p);
        assert_eq!(p.lines[0].segments[1].text, "soft");
        assert_eq!(p.lines[1].segments[0].text, "ware next words");
    }

    fn para_with_font_size(font_size: f32) -> PdfParagraph {
        let lines = vec![line(vec![seg("text", 0.0, 100.0)])];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
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
            word_count,
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
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.3)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_zero_sizes_ignored() {
        let paragraphs = vec![para_with_font_size(0.0), para_with_font_size(0.0)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    use crate::pdf::structure::types::LayoutHintClass;

    fn furniture_para_with_class(class: LayoutHintClass) -> PdfParagraph {
        let lines = vec![line(vec![seg("ACME", 0.0, 50.0)])];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 10.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: true,
            layout_class: Some(class),
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn test_include_headers_clears_page_header_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
        un_mark_layout_furniture_per_config(&mut paras, true, false);
        assert!(
            !paras[0].is_page_furniture,
            "PageHeader furniture must be cleared when include_headers=true"
        );
    }

    #[test]
    fn test_include_footers_clears_page_footer_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
        un_mark_layout_furniture_per_config(&mut paras, false, true);
        assert!(
            !paras[0].is_page_furniture,
            "PageFooter furniture must be cleared when include_footers=true"
        );
    }

    #[test]
    fn test_include_headers_false_preserves_page_header_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
        un_mark_layout_furniture_per_config(&mut paras, false, false);
        assert!(
            paras[0].is_page_furniture,
            "PageHeader furniture must remain when include_headers=false"
        );
    }

    #[test]
    fn test_include_headers_does_not_clear_page_footer_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
        un_mark_layout_furniture_per_config(&mut paras, true, false);
        assert!(
            paras[0].is_page_furniture,
            "PageFooter furniture must remain when only include_headers=true"
        );
    }

    #[test]
    fn test_include_headers_does_not_clear_non_layout_furniture() {
        let mut para = para(vec![line(vec![seg("repeating", 0.0, 80.0)])]);
        para.is_page_furniture = true;
        para.layout_class = None;
        let mut paras = vec![para];
        un_mark_layout_furniture_per_config(&mut paras, true, true);
        assert!(
            paras[0].is_page_furniture,
            "Heuristic furniture (no layout_class) must not be cleared"
        );
    }

    #[test]
    fn test_un_mark_is_noop_when_both_flags_false() {
        let mut paras = vec![
            furniture_para_with_class(LayoutHintClass::PageHeader),
            furniture_para_with_class(LayoutHintClass::PageFooter),
        ];
        un_mark_layout_furniture_per_config(&mut paras, false, false);
        assert!(paras[0].is_page_furniture);
        assert!(paras[1].is_page_furniture);
    }

    #[test]
    fn test_deduplicate_paragraphs_removes_consecutive_duplicates() {
        let p1 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
        let p2 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
        let p3 = para(vec![line(vec![full_line_seg("Different content here")])]);
        let mut pages = vec![vec![p1, p2, p3]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(pages[0].len(), 2, "consecutive duplicate should be removed");
    }

    #[test]
    fn test_deduplicate_paragraphs_removes_non_consecutive_body_duplicates() {
        let p1 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        let p2 = para(vec![line(vec![full_line_seg("Some intervening paragraph")])]);
        let p3 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        let mut pages = vec![vec![p1, p2, p3]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(pages[0].len(), 2, "non-consecutive body duplicate should be removed");
    }

    #[test]
    fn test_deduplicate_paragraphs_preserves_non_consecutive_headings() {
        let mut h = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        h.heading_level = Some(2);
        let filler = para(vec![line(vec![full_line_seg("Some other content between them")])]);
        let mut h2 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        h2.heading_level = Some(2);
        let mut pages = vec![vec![h, filler, h2]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(
            pages[0].len(),
            3,
            "non-consecutive heading duplicates must be preserved"
        );
    }

    /// Verify that the index offset formula used for image mapping is correct.
    #[test]
    fn test_image_index_offset_mapping() {
        let indices: Vec<usize> = vec![50, 52, 54];
        let indices_set: ahash::AHashSet<usize> = indices.iter().copied().collect();
        let first_idx_on_page = indices.iter().copied().min().unwrap_or(0);

        let mut matched: Vec<usize> = Vec::new();
        for current_image in 0..5usize {
            let global_idx = first_idx_on_page + current_image;
            if indices_set.contains(&global_idx) {
                matched.push(global_idx);
            }
        }

        assert_eq!(
            matched,
            vec![50, 52, 54],
            "offset formula must yield exactly the requested global indices"
        );

        assert!(
            !indices_set.contains(&49usize),
            "index 49 is before the page range and must not match"
        );

        assert!(
            !indices_set.contains(&55usize),
            "index 55 was not requested and must not match"
        );
    }

    /// Helper: build a minimal SegmentData for heading-map tests.
    fn seg_with_font(text: &str, font_size: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 10.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            assigned_role: None,
        }
    }

    fn seg_at(text: &str, x: f32, y: f32, height: f32, monospace: bool) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width: 200.0,
            height,
            font_size: height,
            is_bold: false,
            is_italic: false,
            is_monospace: monospace,
            baseline_y: y,
            assigned_role: None,
        }
    }

    #[test]
    fn test_compute_paragraph_gap_ys_detects_blank_line_gap() {
        let segments = vec![
            seg_at("line one", 10.0, 700.0, 12.0, false),
            seg_at("line two", 10.0, 684.0, 12.0, false),
            seg_at("new paragraph", 10.0, 644.0, 12.0, false),
        ];
        let gaps = compute_paragraph_gap_ys(&segments);
        assert_eq!(gaps.len(), 1, "only the blank-line jump is a paragraph gap");
        assert!(
            gaps[0] > 656.0 && gaps[0] < 684.0,
            "gap midpoint between the paragraphs, got {}",
            gaps[0]
        );
    }

    #[test]
    fn test_compute_paragraph_gap_ys_ignores_same_line_runs_and_tight_lines() {
        let segments = vec![
            seg_at("run a", 10.0, 700.0, 12.0, false),
            seg_at("run b", 80.0, 700.0, 12.0, false),
            seg_at("next line", 10.0, 685.0, 12.0, false),
        ];
        assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
    }

    #[test]
    fn test_compute_paragraph_gap_ys_immune_to_column_major_stream_order() {
        let segments = vec![
            seg_at("A top", 10.0, 700.0, 12.0, false),
            seg_at("A mid", 10.0, 685.0, 12.0, false),
            seg_at("A bot", 10.0, 670.0, 12.0, false),
            seg_at("B top", 300.0, 700.0, 12.0, false),
            seg_at("B mid", 300.0, 685.0, 12.0, false),
            seg_at("B bot", 300.0, 670.0, 12.0, false),
        ];
        assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
    }

    #[test]
    fn test_finalize_paragraph_page_number_is_furniture_not_heading() {
        let heading_map = vec![(12.0, Some(2)), (9.0, None)];
        let gap_info = crate::pdf::structure::classify::precompute_gap_info(&heading_map);
        let seg = seg_at("1", 300.0, 50.0, 12.0, false);
        let para = finalize_paragraph(&[&seg], &heading_map, &gap_info).expect("paragraph");
        assert_eq!(para.heading_level, None, "page number must not become a heading");
        assert!(para.is_page_furniture, "page number must be marked furniture");
    }

    #[test]
    fn test_compute_paragraph_gap_ys_skips_blank_lines_inside_code_blocks() {
        let segments = vec![
            seg_at("let x = 1;", 10.0, 700.0, 12.0, true),
            seg_at("let y = 2;", 10.0, 660.0, 12.0, true),
            seg_at("Prose resumes here.", 10.0, 620.0, 12.0, false),
        ];
        let gaps = compute_paragraph_gap_ys(&segments);
        assert_eq!(gaps.len(), 1, "only the code→prose boundary is a gap");
        assert!(
            gaps[0] > 632.0 && gaps[0] < 660.0,
            "gap sits between code and prose, got {}",
            gaps[0]
        );
    }

    /// 5-paragraph doc (1 title at 14pt + 4 body at 11pt) with k_clusters=4.
    /// The adaptive clamp should reduce clusters to max(2, 5/4)=max(2,1)=2,
    /// and then the font-size difference (14 vs 11, ratio≈1.27 ≥ 1.2) should
    /// produce a heading_level=1 for the 14pt entry.
    #[test]
    fn test_build_heading_map_short_doc_title_gets_heading_level_1() {
        let title_seg = seg_with_font("My Title", 14.0);
        let body_seg1 = seg_with_font("Body paragraph one.", 11.0);
        let body_seg2 = seg_with_font("Body paragraph two.", 11.0);
        let body_seg3 = seg_with_font("Body paragraph three.", 11.0);
        let body_seg4 = seg_with_font("Body paragraph four.", 11.0);

        let all_page_segments = vec![vec![title_seg, body_seg1, body_seg2, body_seg3, body_seg4]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];
        let k_clusters = 4;

        let (heading_map, _) =
            build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)
                .expect("build_heading_map must succeed");

        let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
        assert!(
            title_entry.is_some(),
            "heading_map must contain an entry near 14pt; got: {heading_map:?}"
        );
        assert_eq!(
            title_entry.unwrap().1,
            Some(1),
            "14pt title in a 5-paragraph doc must get heading_level=1; got: {heading_map:?}"
        );
    }

    /// Verify that the adaptive k clamp doesn't over-reduce for larger documents
    /// (≥20 paragraphs keeps k_clusters unchanged).
    #[test]
    fn test_build_heading_map_large_doc_k_not_reduced() {
        let mut segs: Vec<SegmentData> = (0..4).map(|i| seg_with_font(&format!("Heading {i}"), 18.0)).collect();
        segs.extend((0..20).map(|i| seg_with_font(&format!("Body text paragraph {i}."), 12.0)));

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        let heading_entry = heading_map.iter().find(|(fs, _)| (*fs - 18.0).abs() < 1.0);
        assert!(
            heading_entry.is_some_and(|(_, level)| level.is_some()),
            "18pt entries in a 24-paragraph doc must have a heading level; got: {heading_map:?}"
        );
    }

    /// Uniform-font short document: when all paragraphs share the same font size,
    /// no heading cluster is found by k-means. The fallback must detect the first-page
    /// segment as a title when its font is ≥ 1.2× median — but here all fonts are equal
    /// so no fallback should fire.
    #[test]
    fn test_build_heading_map_uniform_font_no_spurious_heading() {
        let segs: Vec<SegmentData> = (0..5).map(|i| seg_with_font(&format!("Para {i}"), 12.0)).collect();

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "uniform-font doc must produce no headings; got: {heading_map:?}"
        );
    }

    /// Fallback title detection: 5-paragraph doc where first segment is 14pt,
    /// others are 11pt. Ratio 14/11 ≈ 1.27 ≥ 1.2 — fallback must fire only when
    /// k-means would fail to assign a heading.  This exercises the same fixture as
    /// `test_build_heading_map_short_doc_title_gets_heading_level_1` but specifically
    /// with k=1 (no clustering possible) to force the fallback path.
    #[test]
    fn test_build_heading_map_fallback_title_when_k_equals_1() {
        let title_seg = seg_with_font("Document Title", 14.0);
        let body_segs: Vec<SegmentData> = (0..4)
            .map(|i| seg_with_font(&format!("Body paragraph {i}."), 11.0))
            .collect();

        let mut segs = vec![title_seg];
        segs.extend(body_segs);

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 1)
            .expect("build_heading_map must succeed");

        let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
        let _ = title_entry;
    }

    /// Segment with explicit font_size and baseline_y for heuristic-path tests.
    fn seg_heuristic(text: &str, font_size: f32, baseline_y: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 10.0,
            y: baseline_y,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
            assigned_role: None,
        }
    }

    /// Heuristic path: two segments at different font sizes (triggering a split in
    /// blocks_to_paragraphs) that are a sentence continuation should be re-joined
    /// by merge_continuation_paragraphs.
    #[test]
    fn test_heuristic_path_merges_font_split_continuation() {
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![
                    seg_heuristic("een indicative", 12.0, 700.0),
                    seg_heuristic("van toenemende merkbekendheid", 13.8, 680.0),
                ],
                page_hints: None,
                table_bboxes: vec![],
                hint_validations: vec![],
                needs_classify: false,
                paragraph_gap_ys: vec![],
                include_headers: true,
                include_footers: true,
            },
            &[],
            None,
        );
        assert_eq!(
            output.len(),
            1,
            "continuation paragraph split by font change should be merged on heuristic path"
        );
        assert!(
            output[0].text.is_empty(),
            "merged paragraph must have cleared text so assembly joins from segments"
        );
        let all_text: String = output[0]
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains("een indicative"),
            "first fragment must survive in merged segments; got: {all_text:?}"
        );
        assert!(
            all_text.contains("van toenemende merkbekendheid"),
            "second fragment must survive in merged segments; got: {all_text:?}"
        );
    }

    /// Heuristic path: a sentence-terminating paragraph followed by an
    /// uppercase-starting paragraph must NOT be merged.
    #[test]
    fn test_heuristic_path_does_not_merge_terminated_sentences() {
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![
                    seg_heuristic("The first sentence ends here.", 12.0, 700.0),
                    seg_heuristic("New sentence starts uppercase.", 13.8, 680.0),
                ],
                page_hints: None,
                table_bboxes: vec![],
                hint_validations: vec![],
                needs_classify: false,
                paragraph_gap_ys: vec![],
                include_headers: true,
                include_footers: true,
            },
            &[],
            None,
        );
        assert_eq!(
            output.len(),
            2,
            "terminated sentence followed by uppercase must not be merged"
        );
    }

    /// Verify that non-contiguous index ranges across pages are handled correctly.
    #[test]
    fn test_image_index_offset_non_contiguous_pages() {
        let page1_indices: Vec<usize> = vec![0, 1];
        let page2_indices: Vec<usize> = vec![100, 101];

        for (indices, expected_first) in [(&page1_indices, 0usize), (&page2_indices, 100usize)] {
            let first_idx = indices.iter().copied().min().unwrap_or(0);
            assert_eq!(
                first_idx, expected_first,
                "first_idx_on_page must equal the minimum index in the slice"
            );

            let set: ahash::AHashSet<usize> = indices.iter().copied().collect();
            for current_image in 0..2usize {
                let global_idx = first_idx + current_image;
                assert!(
                    set.contains(&global_idx),
                    "global index {global_idx} must be found for page with first_idx={first_idx}"
                );
            }
        }
    }
}

#[cfg(test)]
mod list_marker_tests {
    use super::{is_bare_list_marker, looks_like_list_item};

    #[test]
    fn bare_markers_are_detected() {
        assert!(is_bare_list_marker("1."));
        assert!(is_bare_list_marker("12)"));
        assert!(is_bare_list_marker("a)"));
        assert!(is_bare_list_marker("(2)"));
        assert!(is_bare_list_marker("•"));
    }

    #[test]
    fn prose_fragments_are_not_bare_markers() {
        assert!(!is_bare_list_marker("etc."));
        assert!(!is_bare_list_marker("Inc."));
        assert!(!is_bare_list_marker("Item"));
        assert!(!is_bare_list_marker(""));
    }

    #[test]
    fn newline_separated_marker_and_text_is_a_list_item() {
        assert!(looks_like_list_item("1.\nÉnumération 1"));
        assert!(looks_like_list_item("1. First point"));
        assert!(looks_like_list_item("(2)\nsecond item"));
    }

    #[test]
    fn section_headings_are_not_list_items() {
        assert!(!looks_like_list_item("3.2 Methods"));
        assert!(!looks_like_list_item("IV. Results"));
        assert!(!looks_like_list_item("1. INTRODUCTION"));
    }

    #[test]
    fn prose_words_ending_with_period_are_not_list_markers() {
        assert!(!looks_like_list_item("tua. At vero eos et accusam"));
        assert!(!looks_like_list_item("etc. and more prose"));
        assert!(looks_like_list_item("a. first item"));
        assert!(looks_like_list_item("iv. fourth item"));
    }
}
