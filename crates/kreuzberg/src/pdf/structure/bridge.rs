//! Bridge between pdfium extraction APIs and the PDF pipeline.
//!
//! Two conversion paths:
//! 1. Structure tree: `ExtractedBlock` → `PdfParagraph` (for tagged PDFs)
//! 2. Full-text heuristic: `PdfPage` → `Vec<SegmentData>` (for untagged PDFs)
//!
//! The heuristic path uses `page.text().all()` as the sole source of text content
//! (correct word spacing, no garbled characters). The character-indexed API
//! (`page.text().chars()`) provides per-character font size, position, and
//! bold/italic metadata — used only for layout classification, never for text.
//!
//! Text is split into paragraph blocks by `\n\n` boundaries from pdfium's spatial
//! analysis. Font metadata is looked up per block for heading/style classification.
//!
//! Falls back to the page objects API when `page.text()` fails entirely.

use std::borrow::Cow;

use crate::pdf::hierarchy::SegmentData;
use pdfium_render::prelude::*;

use super::text_repair::{apply_ligature_repairs, build_ligature_repair_map, normalize_text_encoding};
use super::types::PdfParagraph;

// Alias to distinguish from our local PdfParagraph type.
use pdfium_render::prelude::PdfParagraph as PdfiumParagraph;

/// Position and metadata of an image detected during object-based extraction.
#[derive(Debug, Clone)]
pub(super) struct ImagePosition {
    /// 1-indexed page number.
    pub page_number: usize,
    /// Global image index across the document.
    pub image_index: usize,
}

/// Filter sidebar artifacts from structure tree extracted blocks.
///
/// Removes blocks that appear to be sidebar text (e.g., arXiv identifiers
/// rendered vertically along page margins). Detection criteria:
/// - Block has bounds in the leftmost or rightmost margin (< 8% or > 92% of page width)
/// - Block text is very short (≤ 3 characters trimmed)
/// - At least 3 such blocks exist (to avoid false positives on legitimate margin content)
pub(super) fn filter_sidebar_blocks(blocks: &[ExtractedBlock], page_width: f32) -> Cow<'_, [ExtractedBlock]> {
    if page_width <= 0.0 {
        return Cow::Borrowed(blocks);
    }

    let left_cutoff = page_width * 0.08;
    let right_cutoff = page_width * 0.92;

    // Count short-text blocks in margins
    let sidebar_count = count_sidebar_blocks(blocks, left_cutoff, right_cutoff);

    if sidebar_count < 3 {
        return Cow::Borrowed(blocks);
    }

    // Filter them out
    Cow::Owned(filter_blocks_recursive(blocks, left_cutoff, right_cutoff))
}

fn count_sidebar_blocks(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> usize {
    let mut count = 0;
    for block in blocks {
        if !block.children.is_empty() {
            count += count_sidebar_blocks(&block.children, left_cutoff, right_cutoff);
        } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
            count += 1;
        }
    }
    count
}

fn is_sidebar_block(block: &ExtractedBlock, left_cutoff: f32, right_cutoff: f32) -> bool {
    let trimmed = block.text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 3 {
        return false;
    }
    if let Some(bounds) = &block.bounds {
        let left = bounds.left().value;
        let right = bounds.right().value;
        // Block is entirely within left or right margin
        right < left_cutoff || left > right_cutoff
    } else {
        false
    }
}

fn filter_blocks_recursive(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> Vec<ExtractedBlock> {
    blocks
        .iter()
        .filter_map(|block| {
            if !block.children.is_empty() {
                let filtered_children = filter_blocks_recursive(&block.children, left_cutoff, right_cutoff);
                if filtered_children.is_empty() {
                    return None;
                }
                Some(ExtractedBlock {
                    children: filtered_children,
                    ..block.clone()
                })
            } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
                None
            } else {
                Some(block.clone())
            }
        })
        .collect()
}

/// Convert extracted blocks from the structure tree API into PdfParagraphs.
///
/// Converts via the unified DTO path:
/// `ExtractedBlock` → `PageContent` (via `adapters::from_structure_tree`) →
/// `Vec<PdfParagraph>` (via `content_convert::content_to_paragraphs`).
pub(super) fn extracted_blocks_to_paragraphs(blocks: &[ExtractedBlock]) -> Vec<PdfParagraph> {
    let page_content = super::adapters::from_structure_tree(blocks);
    super::content_convert::content_to_paragraphs(&page_content)
}

/// Convert full-text blocks (from `extract_page_blocks`) to classified PdfParagraphs.
///
/// Each SegmentData from the block extraction represents one `\n\n`-separated
/// paragraph from `page.text().all()`. This function classifies each block
/// conservatively and produces PdfParagraphs with the `text` field populated
/// (the full-text path — assembly reads `para.text` directly).
///
/// Classification rules (conservative — err on the side of Paragraph):
/// - Font significantly larger than body → Heading (level from heading_map)
/// - Starts with bullet/number marker → ListItem
/// - All monospace + multiple lines → CodeBlock
/// - Everything else → Paragraph
pub(super) fn blocks_to_paragraphs(
    lines: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
) -> Vec<PdfParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    let gap_info = super::classify::precompute_gap_info(heading_map);

    // Group consecutive lines into paragraphs. A new paragraph starts when:
    // - Line's baseline_y crosses a segment gap position (from pdfium segment analysis)
    // - Font size changes significantly (>1.5pt)
    // - Bold changes
    // - Line starts with a list marker
    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<&SegmentData> = Vec::new();

    for line in &lines {
        let should_break = if current_lines.is_empty() {
            false
        } else {
            let prev = current_lines.last().unwrap();
            let font_change = (line.font_size - prev.font_size).abs() > 1.5;
            let bold_change = line.is_bold != prev.is_bold;
            let is_list = looks_like_list_item(&line.text);
            // Segment gap: a paragraph break exists between prev and current
            // if a gap_y falls between their baselines.
            let crossed_gap = paragraph_gap_ys.iter().any(|&gap_y| {
                // prev is above current in PDF coords (prev.baseline_y > line.baseline_y)
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

    // Finalize last paragraph.
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

/// Build a PdfParagraph from a group of consecutive lines with compatible font properties.
fn finalize_paragraph(
    lines: &[&SegmentData],
    heading_map: &[(f32, Option<u8>)],
    gap_info: &super::classify::GapInfo,
) -> Option<PdfParagraph> {
    if lines.is_empty() {
        return None;
    }

    // Join line texts with newlines (preserving full_text content exactly).
    let text: String = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first = lines[0];
    let word_count = trimmed.split_whitespace().count();
    let is_bold = lines.iter().filter(|l| l.is_bold).count() > lines.len() / 2;

    // Conservative heading detection.
    // Pass 1: font-size-based — significantly larger font than body.
    let mut heading_level = super::classify::find_heading_level(first.font_size, heading_map, gap_info);
    if heading_level.is_some() && (word_count > 20 || super::layout_classify::is_separator_text(trimmed)) {
        heading_level = None;
    }

    // Pass 2: bold-at-body-size → H2. Very conservative — only when we have
    // strong evidence this is a heading, not just bold emphasis:
    // - Must be bold + single line + short (≤8 words)
    // - Must NOT end with period, colon, comma, semicolon (sentence fragments)
    // - Must NOT contain common body-text signals (@, parentheses, commas)
    // - Must start with uppercase letter or digit (section numbering)
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

    // Pass 3: font-size-above-body detection for short paragraphs.
    // Catches section headings whose font size is meaningfully larger than body
    // but was merged into the body cluster by k-means (e.g., 12pt headings vs
    // 10pt body in LaTeX documents). Since we don't have bold confirmation,
    // require stronger evidence: text must match a section heading pattern
    // (starts with section number like "3.1 Methods" or is a known structural
    // heading word like "References", "Appendix").
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
        {
            heading_level = Some(2);
        }
    }

    let is_list_item = heading_level.is_none() && looks_like_list_item(trimmed);
    let is_code_block =
        heading_level.is_none() && !is_list_item && lines.iter().all(|l| l.is_monospace) && lines.len() >= 2;

    // Page furniture detection: mark standalone page numbers as furniture.
    // These are short text fragments that match common page number patterns
    // (e.g., "1", "Page 3 of 10", "- 5 -", Roman numerals). Cross-page
    // repeating text detection (in classify.rs) handles running headers
    // and footers; this catches page numbers which vary per page.
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

    Some(PdfParagraph {
        text: trimmed.to_string(),
        lines: Vec::new(),
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
            // Union of all lines' bboxes for precise paragraph bounds.
            let left = lines.iter().map(|l| l.x).fold(f32::MAX, f32::min);
            let bottom = lines.iter().map(|l| l.baseline_y).fold(f32::MAX, f32::min);
            let right = lines.iter().map(|l| l.x + l.width).fold(f32::MIN, f32::max);
            let top = lines.iter().map(|l| l.baseline_y + l.height).fold(f32::MIN, f32::max);
            (left, bottom, right, top)
        }),
    })
}

/// Check if text starts with a common list marker.
fn looks_like_list_item(text: &str) -> bool {
    let t = text.trim_start();

    // Bullet characters — high confidence markers
    if t.starts_with('•')
        || t.starts_with('·')
        || t.starts_with('◦')
        || t.starts_with('▪')
        || t.starts_with('–')
        || t.starts_with('—')
    {
        return true;
    }

    // Hyphen-dash list: only if followed by space + alphabetic word.
    // Rejects "- 1145/3620665..." (bibliography) and "- 1 PDF backends" (subsection).
    if let Some(rest) = t.strip_prefix("- ") {
        return rest.chars().next().is_some_and(|c| c.is_alphabetic());
    }

    // Numbered / lettered patterns: "1.", "2)", "a.", "a)", "i.", "(1)", "(a)"
    let mut chars = t.chars().peekable();

    // Parenthesized: "(1)" or "(a)" — require closing paren + space + word
    if chars.peek() == Some(&'(') {
        chars.next();
        if chars.peek().is_some_and(|c| c.is_alphanumeric()) {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_alphanumeric()) {
                chars.next();
            }
            if chars.peek() == Some(&')') {
                chars.next();
                // Must be followed by space then an alphabetic character
                return chars.peek() == Some(&' ') && {
                    chars.next();
                    chars.peek().is_some_and(|c| c.is_alphabetic())
                };
            }
        }
        return false;
    }

    // "1." / "1)" / "a." / "a)" etc.
    // Exclude section heading patterns: "I. INTRODUCTION", "II. BASICS" etc.
    if super::classify::is_section_pattern(t) {
        return false;
    }

    if chars.peek().is_some_and(|c| c.is_alphanumeric()) {
        let mut num_len = 0;
        while chars.peek().is_some_and(|c| c.is_alphanumeric()) {
            chars.next();
            num_len += 1;
        }
        if num_len <= 4 && (chars.peek() == Some(&'.') || chars.peek() == Some(&')')) {
            chars.next();
            return chars.peek() == Some(&' ') && {
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
    // Standalone number: "1", "42", "103"
    if t.chars().all(|c| c.is_ascii_digit()) && t.len() <= 4 {
        return true;
    }
    // "Page X" or "Page X of Y" (case-insensitive)
    let lower = t.to_lowercase();
    if lower.starts_with("page ") {
        return true;
    }
    // "- X -" or "– X –" (centered page numbers with dashes)
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
    // Roman numerals: "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii"
    if t.len() <= 5 && t.chars().all(|c| matches!(c, 'i' | 'v' | 'x' | 'I' | 'V' | 'X')) {
        return true;
    }
    false
}

/// Extract text blocks and image positions from a PDF page.
///
/// Uses `page.text().all()` for text content and `page.text().chars()` for
/// per-character font/position metadata. Text is split into paragraph blocks
/// by `\n\n` boundaries from pdfium's spatial analysis.
///
/// Falls back to the page objects API when `page.text()` fails entirely.
///
/// Also detects image objects and records their positions for interleaving.
pub(super) fn objects_to_page_data(
    page: &PdfPage,
    page_number: usize,
    image_offset: &mut usize,
) -> (Vec<SegmentData>, Vec<ImagePosition>, Vec<f32>) {
    let objects: Vec<PdfPageObject> = page.objects().iter().collect();

    // Image scan BEFORE text extraction.
    let mut images = Vec::new();
    for obj in &objects {
        if obj.as_image_object().is_some() {
            images.push(ImagePosition {
                page_number,
                image_index: *image_offset,
            });
            *image_offset += 1;
        }
    }

    // Primary extraction: full-text blocks with char-indexed font metadata.
    if let Some((segments, _full_text, gap_ys)) = extract_page_blocks(page) {
        return (segments, images, gap_ys);
    }

    // Last resort: page objects API with column detection.
    // Used when page.text() fails entirely (rare edge case).
    let mut segments = Vec::new();
    let column_groups = super::columns::split_objects_into_columns(&objects);
    let column_vecs = partition_objects_by_columns(objects, &column_groups);
    for column_objects in &column_vecs {
        let paragraphs: Vec<PdfiumParagraph> = PdfiumParagraph::from_objects(column_objects);
        extract_paragraphs_to_segments(paragraphs, &mut segments);
    }

    // Apply ligature repair for last-resort path.
    if let Some(repair_map) = build_ligature_repair_map(page) {
        for seg in &mut segments {
            if let Cow::Owned(s) = apply_ligature_repairs(&seg.text, &repair_map) {
                seg.text = s;
            }
        }
    }

    (segments, images, Vec::new())
}

// ── Full-text block extraction ──

/// Full-text block extraction from a PDF page.
///
/// Uses `page.text().all()` for text content (correct word spacing, no garbled
/// characters). Splits on `\n\n` to get paragraph blocks. Uses the char-indexed
/// API (`page.text().chars()`) ONLY for per-block font size, position, and
/// bold/italic metadata — never for text content or word spacing.
///
/// Per-character font and position metadata from pdfium's char-indexed API.
struct CharFontInfo {
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    is_monospace: bool,
    baseline_y: f32,
    x: f32,
    top: f32,
}

/// Returns one `SegmentData` per paragraph block, where `.text` is the exact
/// text from `page.text().all()` with spacing preserved.
fn extract_page_blocks(page: &PdfPage) -> Option<(Vec<SegmentData>, String, Vec<f32>)> {
    let text_api = page.text().ok()?;
    let full_text = text_api.all();
    if full_text.trim().is_empty() {
        return None;
    }

    // Collect segment bounding boxes (cheap: 1 FFI per segment, bounds cached).
    // Compute paragraph gap y-positions: where consecutive segments have a large
    // vertical gap (>1.5x average segment height), that's a paragraph boundary.
    let pdfium_segments = text_api.segments();
    let seg_count = pdfium_segments.len();
    let mut segment_bboxes: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(seg_count);
    for i in 0..seg_count {
        if let Ok(seg) = pdfium_segments.get(i) {
            let b = seg.bounds();
            segment_bboxes.push((b.left().value, b.bottom().value, b.right().value, b.top().value));
        }
    }

    // Find paragraph gap positions from segment boundaries.
    // A gap between segment[i].bottom and segment[i+1].top that exceeds
    // 1.5x the average segment height indicates a paragraph break.
    // Store the y-coordinate of each gap midpoint.
    let avg_seg_height: f32 = if segment_bboxes.len() > 1 {
        segment_bboxes.iter().map(|(_, b, _, t)| (t - b).abs()).sum::<f32>() / segment_bboxes.len() as f32
    } else {
        12.0
    };
    let gap_threshold = avg_seg_height * 1.5;
    let mut paragraph_gap_ys: Vec<f32> = Vec::new();
    for i in 1..segment_bboxes.len() {
        let prev_bottom = segment_bboxes[i - 1].1;
        let curr_top = segment_bboxes[i].3;
        let gap = prev_bottom - curr_top; // PDF coords: prev is higher, so prev_bottom > curr_top
        if gap > gap_threshold {
            // Midpoint of the gap in PDF y-coordinates
            paragraph_gap_ys.push((prev_bottom + curr_top) / 2.0);
        }
    }
    tracing::debug!(
        segments = segment_bboxes.len(),
        avg_seg_height,
        gap_threshold,
        paragraph_gaps = paragraph_gap_ys.len(),
        "segment-based paragraph gap detection"
    );

    // Build per-char font/position map from the char-indexed API.
    // These are in pdfium's internal order (may differ from full_text reading order).
    // We sort them into reading order to align 1:1 with full_text's non-ws chars.
    let all_chars = text_api.chars();
    if all_chars.is_empty() {
        return None;
    }

    // Scan page objects for /Artifact content marks (PDF spec tagged content).
    // Build a set of artifact marked-content-IDs (MCIDs) for O(1) char lookup.
    // Watermark artifacts → exclude from text. Other artifacts → tag as furniture.
    let mut watermark_mcids: ahash::AHashSet<i32> = ahash::AHashSet::new();
    let mut artifact_mcids: ahash::AHashSet<i32> = ahash::AHashSet::new();
    for obj in page.objects().iter() {
        if let Some(mcid) = obj.marked_content_id() {
            for mark in obj.content_marks().iter() {
                if mark.name().as_deref() == Some("Artifact") {
                    if mark.param_string_value("Type").as_deref() == Some("Watermark") {
                        watermark_mcids.insert(mcid);
                    } else {
                        artifact_mcids.insert(mcid);
                    }
                    break;
                }
            }
        }
    }
    // For objects without MCIDs, fall back to per-char text_object check.
    // This handles untagged PDFs where artifacts have marks but no MCID.
    let mut watermark_char_indices: ahash::AHashSet<usize> = ahash::AHashSet::new();
    let mut artifact_char_indices: ahash::AHashSet<usize> = ahash::AHashSet::new();
    let has_mcid_artifacts = !watermark_mcids.is_empty() || !artifact_mcids.is_empty();
    if !has_mcid_artifacts {
        // No MCID-based artifacts found — try per-char text_object check.
        // Cache: skip if previous char had same text_object (consecutive chars share objects).
        let mut prev_mcid: i32 = -999;
        let mut prev_is_watermark = false;
        let mut prev_is_artifact = false;
        for (char_idx, ch) in all_chars.iter().enumerate() {
            if let Ok(text_obj) = ch.text_object() {
                let mcid = text_obj.marked_content_id().unwrap_or(-1);
                if mcid != prev_mcid {
                    prev_mcid = mcid;
                    prev_is_watermark = false;
                    prev_is_artifact = false;
                    for mark in text_obj.content_marks().iter() {
                        if mark.name().as_deref() == Some("Artifact") {
                            if mark.param_string_value("Type").as_deref() == Some("Watermark") {
                                prev_is_watermark = true;
                            } else {
                                prev_is_artifact = true;
                            }
                            break;
                        }
                    }
                }
                if prev_is_watermark {
                    watermark_char_indices.insert(char_idx);
                } else if prev_is_artifact {
                    artifact_char_indices.insert(char_idx);
                }
            }
        }
    }
    let has_any_artifacts =
        has_mcid_artifacts || !watermark_char_indices.is_empty() || !artifact_char_indices.is_empty();
    if has_any_artifacts {
        tracing::debug!(
            watermark_mcids = watermark_mcids.len(),
            artifact_mcids = artifact_mcids.len(),
            watermark_chars = watermark_char_indices.len(),
            artifact_chars = artifact_char_indices.len(),
            total_chars = all_chars.len(),
            "PDF artifact marks detected"
        );
    }

    let mut char_infos: Vec<CharFontInfo> = Vec::with_capacity(all_chars.len());
    for (char_idx, ch) in all_chars.iter().enumerate() {
        if ch.unicode_char().is_none_or(|c| c.is_whitespace()) {
            continue;
        }
        // Skip watermark characters — check MCID-based or char-index-based sets.
        if !watermark_char_indices.is_empty() && watermark_char_indices.contains(&char_idx) {
            continue;
        }
        if has_mcid_artifacts
            && let Ok(text_obj) = ch.text_object()
            && let Some(mcid) = text_obj.marked_content_id()
            && watermark_mcids.contains(&mcid)
        {
            continue;
        }
        let bounds = match ch.tight_bounds() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let fs_raw = ch.scaled_font_size().value;
        let fs = if fs_raw > 0.0 { fs_raw } else { 12.0 };
        let info = ch.font_info();
        let mono = crate::pdf::text_data::is_truly_monospace(ch.font_is_fixed_pitch(), &info.0);
        let origin_y = ch.origin().map(|o| o.1.value).unwrap_or(bounds.bottom().value);
        let origin_x = ch.origin().map(|o| o.0.value).unwrap_or(bounds.left().value);

        char_infos.push(CharFontInfo {
            font_size: fs,
            is_bold: info.1,
            is_italic: info.2,
            is_monospace: mono,
            baseline_y: origin_y,
            x: origin_x,
            top: bounds.top().value,
        });
    }

    // Sort char_infos into reading order (top-to-bottom, left-to-right)
    // using line bucketing to handle baseline variation within a line.
    sort_chars_reading_order(&mut char_infos);

    // Walk full_text LINE BY LINE (\n-separated) and char_infos in parallel.
    // Each non-ws char in full_text gets the next CharFontInfo (reading order aligned).
    // One SegmentData per text line, with font properties from char API.
    let mut info_idx = 0usize;
    let mut segments: Vec<SegmentData> = Vec::new();

    for line in full_text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Aggregate font properties for this line's non-ws chars.
        let mut font_sizes: Vec<f32> = Vec::new();
        let mut bold_count = 0usize;
        let mut italic_count = 0usize;
        let mut mono_count = 0usize;
        let mut first_baseline_y = 0.0f32;
        let mut first_x = 0.0f32;
        let mut char_count = 0usize;

        for c in line.chars() {
            if c.is_whitespace() {
                continue;
            }
            if info_idx < char_infos.len() {
                let info = &char_infos[info_idx];
                font_sizes.push(info.font_size);
                if info.is_bold {
                    bold_count += 1;
                }
                if info.is_italic {
                    italic_count += 1;
                }
                if info.is_monospace {
                    mono_count += 1;
                }
                if char_count == 0 {
                    first_baseline_y = info.baseline_y;
                    first_x = info.x;
                }
                info_idx += 1;
            }
            char_count += 1;
        }

        if char_count == 0 {
            continue;
        }

        let dominant_fs = most_frequent_font_size(&font_sizes);
        let half = char_count / 2;

        // Find the segment bbox that best covers this line (by baseline_y overlap).
        let (seg_x, seg_w, seg_h) = segment_bboxes
            .iter()
            .find(|(_, bottom, _, top)| first_baseline_y >= *bottom && first_baseline_y <= *top)
            .map(|(left, bottom, right, top)| (*left, right - left, top - bottom))
            .unwrap_or((first_x, 500.0, dominant_fs));

        segments.push(SegmentData {
            text: trimmed.to_string(),
            x: seg_x,
            y: first_baseline_y,
            width: seg_w.max(dominant_fs),
            height: seg_h.max(dominant_fs),
            font_size: dominant_fs,
            is_bold: bold_count > half,
            is_italic: italic_count > half,
            is_monospace: mono_count > half,
            baseline_y: first_baseline_y,
        });
    }

    tracing::debug!(
        lines = segments.len(),
        char_infos = char_infos.len(),
        info_consumed = info_idx,
        "extract_page_blocks: line-level segments from full_text"
    );

    if segments.is_empty() {
        None
    } else {
        Some((segments, full_text, paragraph_gap_ys))
    }
}

/// Sort character font infos into reading order (top-to-bottom, left-to-right).
///
/// Groups chars into lines by baseline_y (tolerance = 50% of font size),
/// sorts lines top-to-bottom, then chars within each line left-to-right.
fn sort_chars_reading_order(infos: &mut Vec<CharFontInfo>) {
    if infos.len() <= 1 {
        return;
    }

    // Assign each char to a line bucket.
    let mut line_y_sums: Vec<(f64, f64)> = Vec::new(); // (y_sum, count)
    let mut line_ids: Vec<usize> = vec![0; infos.len()];

    for (i, info) in infos.iter().enumerate() {
        let tolerance = info.font_size * 0.5;
        let matched = line_y_sums.iter().position(|(sum, count)| {
            let avg = (*sum / *count) as f32;
            (info.baseline_y - avg).abs() <= tolerance
        });
        match matched {
            Some(line_idx) => {
                line_y_sums[line_idx].0 += info.baseline_y as f64;
                line_y_sums[line_idx].1 += 1.0;
                line_ids[i] = line_idx;
            }
            None => {
                line_ids[i] = line_y_sums.len();
                line_y_sums.push((info.baseline_y as f64, 1.0));
            }
        }
    }

    // Sort lines by average Y descending (PDF coords: higher Y = higher on page).
    let mut line_order: Vec<(usize, f32)> = line_y_sums
        .iter()
        .enumerate()
        .map(|(i, (sum, count))| (i, (*sum / *count) as f32))
        .collect();
    line_order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut line_rank = vec![0usize; line_y_sums.len()];
    for (rank, (old_id, _)) in line_order.iter().enumerate() {
        line_rank[*old_id] = rank;
    }

    // Build sort indices: by line rank, then by X within line.
    let mut indices: Vec<usize> = (0..infos.len()).collect();
    indices.sort_by(|&a, &b| {
        line_rank[line_ids[a]]
            .cmp(&line_rank[line_ids[b]])
            .then_with(|| infos[a].x.partial_cmp(&infos[b].x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Reorder in-place.
    let sorted: Vec<CharFontInfo> = indices
        .into_iter()
        .map(|i| CharFontInfo {
            font_size: infos[i].font_size,
            is_bold: infos[i].is_bold,
            is_italic: infos[i].is_italic,
            is_monospace: infos[i].is_monospace,
            baseline_y: infos[i].baseline_y,
            x: infos[i].x,
            top: infos[i].top,
        })
        .collect();
    *infos = sorted;
}

/// Return the most frequent font size from a list, quantized to 0.5pt bins.
fn most_frequent_font_size(sizes: &[f32]) -> f32 {
    if sizes.is_empty() {
        return 12.0;
    }
    let mut counts: Vec<(i32, usize)> = Vec::new(); // (quantized_key, count)
    for &s in sizes {
        let key = (s * 2.0).round() as i32; // 0.5pt bins
        if let Some(entry) = counts.iter_mut().find(|(k, _)| *k == key) {
            entry.1 += 1;
        } else {
            counts.push((key, 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts[0].0 as f32 / 2.0
}

/// Partition page objects into column groups by moving objects out of the source vec.
///
/// Each column group is a `Vec<usize>` of indices into `objects`. This function
/// consumes the objects vec and returns one `Vec<PdfPageObject>` per column.
fn partition_objects_by_columns<'a>(
    objects: Vec<PdfPageObject<'a>>,
    column_groups: &[Vec<usize>],
) -> Vec<Vec<PdfPageObject<'a>>> {
    if column_groups.len() <= 1 {
        return vec![objects];
    }

    let total = objects.len();
    let num_columns = column_groups.len();
    let mut col_for_obj = vec![0usize; total];
    for (col_idx, group) in column_groups.iter().enumerate() {
        for &obj_idx in group {
            if obj_idx < total {
                col_for_obj[obj_idx] = col_idx;
            }
        }
    }

    let mut result: Vec<Vec<PdfPageObject<'a>>> = (0..num_columns).map(|_| Vec::new()).collect();
    for (i, obj) in objects.into_iter().enumerate() {
        result[col_for_obj[i]].push(obj);
    }

    result
}

/// Convert pdfium paragraphs into SegmentData, preserving per-line positions.
fn extract_paragraphs_to_segments(paragraphs: Vec<PdfiumParagraph>, segments: &mut Vec<SegmentData>) {
    for para in paragraphs {
        for line in para.into_lines() {
            let line_baseline = line.bottom.value;
            let line_left = line.left.value;
            let mut running_x = line_left;

            for fragment in &line.fragments {
                match fragment {
                    PdfParagraphFragment::StyledString(styled) => {
                        let text = normalize_text_encoding(styled.text());
                        if text.trim().is_empty() {
                            continue;
                        }

                        let font_size = styled.font_size().value;
                        let is_bold = styled.is_bold();
                        let is_italic = styled.is_italic();
                        let is_monospace = styled.is_monospace();
                        let estimated_width = text.len() as f32 * font_size * 0.5;

                        segments.push(SegmentData {
                            text: text.into_owned(),
                            x: running_x,
                            y: line_baseline,
                            width: estimated_width,
                            height: font_size,
                            font_size,
                            is_bold,
                            is_italic,
                            is_monospace,
                            baseline_y: line_baseline,
                        });

                        running_x += estimated_width;
                    }
                    PdfParagraphFragment::NonTextObject(_) | PdfParagraphFragment::LineBreak { .. } => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(role: ContentRole, text: &str) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    fn make_block_with_font(role: ContentRole, text: &str, font_size: f32) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(font_size),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn test_heading_block() {
        // Heading must have meaningfully larger font than body for validation to pass
        let blocks = vec![
            make_block_with_font(ContentRole::Heading { level: 2 }, "Section Title", 18.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line one", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line two", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line three", 12.0),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 4);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_heading_trusted_from_structure_tree() {
        // Structure tree heading tags are trusted (author-intent metadata),
        // even when font size matches body text.
        let blocks = vec![
            make_block(ContentRole::Heading { level: 3 }, "Not really a heading"),
            make_block(ContentRole::Paragraph, "Body text"),
            make_block(ContentRole::Paragraph, "More body text"),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].heading_level, Some(3)); // Trusted from structure tree
    }

    #[test]
    fn test_body_block() {
        let blocks = vec![make_block(ContentRole::Paragraph, "Body text")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].heading_level, None);
        assert!(!paragraphs[0].is_list_item);
    }

    #[test]
    fn test_list_item_block() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::ListItem {
                label: Some("1.".to_string()),
            },
            text: "First item".to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert!(paragraphs[0].is_list_item);
        // Check that the label is prepended
        let first_seg_text = &paragraphs[0].lines[0].segments[0].text;
        assert_eq!(first_seg_text, "1.");
    }

    #[test]
    fn test_empty_text_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_whitespace_only_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "   ")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_children_processed() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::Other("Table".to_string()),
            text: String::new(),
            bounds: None,
            font_size: None,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: vec![
                make_block(ContentRole::Paragraph, "Cell 1"),
                make_block(ContentRole::Paragraph, "Cell 2"),
            ],
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 2);
    }

    // ── is_page_number_pattern tests ──

    #[test]
    fn test_page_number_standalone_digit() {
        assert!(is_page_number_pattern("1"));
        assert!(is_page_number_pattern("42"));
        assert!(is_page_number_pattern("103"));
        assert!(is_page_number_pattern("9999"));
    }

    #[test]
    fn test_page_number_too_long() {
        assert!(!is_page_number_pattern("12345"));
    }

    #[test]
    fn test_page_number_page_prefix() {
        assert!(is_page_number_pattern("Page 3"));
        assert!(is_page_number_pattern("Page 10 of 25"));
        assert!(is_page_number_pattern("page 1"));
    }

    #[test]
    fn test_page_number_dashed() {
        assert!(is_page_number_pattern("- 5 -"));
    }

    #[test]
    fn test_page_number_roman() {
        assert!(is_page_number_pattern("iii"));
        assert!(is_page_number_pattern("IV"));
        assert!(is_page_number_pattern("xii"));
    }

    #[test]
    fn test_page_number_not_text() {
        assert!(!is_page_number_pattern("Abstract"));
        assert!(!is_page_number_pattern("Hello World"));
        assert!(!is_page_number_pattern(""));
    }

    // ── is_structural_heading_word tests ──

    #[test]
    fn test_structural_heading_words() {
        assert!(is_structural_heading_word("Abstract"));
        assert!(is_structural_heading_word("References"));
        assert!(is_structural_heading_word("Appendix"));
        assert!(is_structural_heading_word("Conclusion"));
    }

    #[test]
    fn test_structural_heading_non_matches() {
        assert!(!is_structural_heading_word("Version 1.0"));
        assert!(!is_structural_heading_word("Hello"));
        assert!(!is_structural_heading_word(""));
    }

    // ── blocks_to_paragraphs heading detection tests ──

    #[test]
    fn test_bold_heading_detection_pass2() {
        // Bold short paragraph at body font size should become H2
        let heading_map = vec![(17.0, Some(1)), (10.0, None)];
        let lines = vec![
            SegmentData {
                text: "Bold Section".to_string(),
                x: 0.0,
                y: 700.0,
                width: 100.0,
                height: 10.0,
                font_size: 10.0,
                is_bold: true,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
            },
            SegmentData {
                text: "Body text follows here with more words.".to_string(),
                x: 0.0,
                y: 680.0,
                width: 400.0,
                height: 10.0,
                font_size: 10.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 680.0,
            },
        ];
        let paragraphs = blocks_to_paragraphs(lines, &heading_map, &[]);
        assert!(
            paragraphs.len() >= 2,
            "expected >=2 paragraphs, got {}",
            paragraphs.len()
        );
        assert_eq!(paragraphs[0].heading_level, Some(2));
        assert_eq!(paragraphs[0].text, "Bold Section");
    }

    #[test]
    fn test_font_size_above_body_heading_detection_pass3() {
        // Non-bold paragraph at 12pt with section number, body at 10pt → H2
        let heading_map = vec![(17.0, Some(1)), (10.0, None)];
        let lines = vec![
            SegmentData {
                text: "2 Getting Started".to_string(),
                x: 0.0,
                y: 700.0,
                width: 100.0,
                height: 12.0,
                font_size: 12.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
            },
            SegmentData {
                text: "To use Docling you can simply install it.".to_string(),
                x: 0.0,
                y: 680.0,
                width: 400.0,
                height: 10.0,
                font_size: 10.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 680.0,
            },
        ];
        let paragraphs = blocks_to_paragraphs(lines, &heading_map, &[]);
        assert!(
            paragraphs.len() >= 2,
            "expected >=2 paragraphs, got {}",
            paragraphs.len()
        );
        assert_eq!(paragraphs[0].heading_level, Some(2));
        assert_eq!(paragraphs[0].text, "2 Getting Started");
    }

    #[test]
    fn test_pass3_no_false_positive_without_section_pattern() {
        // Non-bold paragraph at 12pt WITHOUT section number → not heading
        let heading_map = vec![(17.0, Some(1)), (10.0, None)];
        let lines = vec![
            SegmentData {
                text: "Version 1.0".to_string(),
                x: 0.0,
                y: 700.0,
                width: 100.0,
                height: 12.0,
                font_size: 12.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
            },
            SegmentData {
                text: "Body text here.".to_string(),
                x: 0.0,
                y: 680.0,
                width: 400.0,
                height: 10.0,
                font_size: 10.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 680.0,
            },
        ];
        let paragraphs = blocks_to_paragraphs(lines, &heading_map, &[]);
        assert_eq!(paragraphs[0].heading_level, None, "Version 1.0 should not be a heading");
    }

    #[test]
    fn test_page_furniture_standalone_number() {
        // Standalone page number should be marked as furniture
        let heading_map = vec![(10.0, None)];
        let lines = vec![SegmentData {
            text: "42".to_string(),
            x: 0.0,
            y: 50.0,
            width: 20.0,
            height: 10.0,
            font_size: 10.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 50.0,
        }];
        let paragraphs = blocks_to_paragraphs(lines, &heading_map, &[]);
        assert_eq!(paragraphs.len(), 1);
        assert!(
            paragraphs[0].is_page_furniture,
            "standalone page number should be furniture"
        );
    }
}
