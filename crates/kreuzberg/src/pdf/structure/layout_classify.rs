//! Layout-detection-based paragraph classification overrides.
//!
//! When layout detection is enabled, this module applies layout hints
//! to override or augment the font-size-based paragraph classification
//! from the standard markdown pipeline.

use super::geometry::Rect;
use super::types::{LayoutHint, LayoutHintClass, PdfParagraph};

/// Apply layout detection overrides to classified paragraphs.
///
/// Uses two matching strategies:
/// 1. **Spatial matching** (heuristic pages): computes bounding boxes from segment
///    positions and matches by containment overlap.
/// 2. **Proportional matching** (structure tree pages): paragraphs without positional
///    data are matched to hints by estimated vertical position, since both are in
///    reading order.
///
/// Structure-tree headings are preserved: only paragraphs without existing
/// heading classification receive heading overrides from layout detection.
pub(crate) fn apply_layout_overrides(
    paragraphs: &mut [PdfParagraph],
    hints: &[LayoutHint],
    min_confidence: f32,
    min_containment: f32,
    body_font_size: Option<f32>,
) {
    if hints.is_empty() {
        return;
    }

    // Separate paragraphs into those with and without positional data.
    let has_any_positions = paragraphs.iter().any(|p| compute_paragraph_bbox(p).is_some());

    if has_any_positions {
        // Spatial matching for paragraphs with positional data
        apply_spatial_overrides(paragraphs, hints, min_confidence, min_containment, body_font_size);
    } else {
        // Proportional matching for structure tree pages (no positional data)
        apply_proportional_overrides(paragraphs, hints, min_confidence);
    }
}

/// Spatial matching: match paragraphs to hints by bounding box overlap.
///
/// Uses a two-tier strategy:
/// 1. **2D containment** (intersection_area / paragraph_area): best for paragraphs
///    that horizontally overlap with the layout hint.
/// 2. **Vertical-only overlap** (vertical_intersection / paragraph_height): fallback
///    for paragraphs where horizontal alignment differs (e.g., centered text vs
///    left-aligned detection box).
///
/// The vertical fallback requires higher confidence to reduce false positives.
fn apply_spatial_overrides(
    paragraphs: &mut [PdfParagraph],
    hints: &[LayoutHint],
    min_confidence: f32,
    min_containment: f32,
    body_font_size: Option<f32>,
) {
    let confident_hints: Vec<&LayoutHint> = hints.iter().filter(|h| h.confidence >= min_confidence).collect();

    for (para_idx, para) in paragraphs.iter_mut().enumerate() {
        let para_bbox = match compute_paragraph_bbox(para) {
            Some(bbox) => bbox,
            None => continue,
        };

        if para_bbox.height() <= 0.0 {
            continue;
        }

        // Try 2D containment first (most precise).
        let best_2d = confident_hints
            .iter()
            .filter_map(|hint| {
                let hint_rect = Rect::from_lbrt(hint.left, hint.bottom, hint.right, hint.top);
                let containment = para_bbox.intersection_over_self(&hint_rect);
                if containment >= min_containment {
                    Some((*hint, containment))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));

        if let Some((hint, containment)) = best_2d {
            tracing::trace!(
                para_idx,
                hint_class = ?hint.class,
                containment,
                "spatial hint match"
            );
            apply_hint_to_paragraph(para, hint, body_font_size);
        }
    }
}

/// Proportional matching: match paragraphs to hints using range-overlap.
///
/// Structure tree paragraphs have no positional data but are in reading order
/// (top-to-bottom). Layout hints have PDF coordinates with known bounding boxes.
///
/// Strategy:
/// 1. Sort hints by vertical position (top-to-bottom in reading order).
/// 2. Each paragraph occupies a fractional range `[i/n, (i+1)/n]` of the page.
/// 3. Each hint occupies a fractional range `[(page_height - top)/page_height, (page_height - bottom)/page_height]`.
/// 4. Match each paragraph to the hint with the most fractional overlap.
///
/// Structure tree paragraphs lack positional data so we cannot do spatial matching.
/// We use conservative fractional-overlap matching: each paragraph is assigned a
/// fraction of the page [i/n, (i+1)/n] and matched against hint bounding boxes
/// converted to page fractions. Only high-confidence, high-overlap matches are applied.
fn apply_proportional_overrides(paragraphs: &mut [PdfParagraph], hints: &[LayoutHint], min_confidence: f32) {
    let n = paragraphs.len();
    if n == 0 {
        return;
    }

    let confident_hints: Vec<&LayoutHint> = hints.iter().filter(|h| h.confidence >= min_confidence).collect();
    if confident_hints.is_empty() {
        return;
    }

    let page_height = hints.iter().map(|h| h.top).fold(0.0_f32, f32::max);
    if page_height <= 0.0 {
        return;
    }

    tracing::debug!(
        paragraph_count = n,
        hint_count = confident_hints.len(),
        page_height,
        "Proportional matching: structure tree paragraphs without positions"
    );

    // Precompute each hint's fractional range on the page.
    let hint_ranges: Vec<(f32, f32, &LayoutHint)> = confident_hints
        .iter()
        .map(|h| {
            let frac_start = (page_height - h.top) / page_height;
            let frac_end = (page_height - h.bottom) / page_height;
            (frac_start.max(0.0), frac_end.min(1.0), *h)
        })
        .collect();

    for (i, para) in paragraphs.iter_mut().enumerate() {
        let para_start = i as f32 / n as f32;
        let para_end = (i as f32 + 1.0) / n as f32;

        let best = hint_ranges
            .iter()
            .filter_map(|&(h_start, h_end, hint)| {
                let overlap_start = para_start.max(h_start);
                let overlap_end = para_end.min(h_end);
                let overlap = (overlap_end - overlap_start).max(0.0);
                if overlap > 0.0 { Some((hint, overlap)) } else { None }
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));

        if let Some((hint, overlap)) = best {
            let para_span = para_end - para_start;
            let overlap_frac = if para_span > 0.0 { overlap / para_span } else { 0.0 };

            match hint.class {
                LayoutHintClass::PageHeader if i == 0 && overlap_frac > 0.25 => {
                    apply_hint_to_paragraph(para, hint, None);
                }
                LayoutHintClass::PageFooter if i == n - 1 && overlap_frac > 0.25 => {
                    apply_hint_to_paragraph(para, hint, None);
                }
                LayoutHintClass::SectionHeader | LayoutHintClass::Title
                    if para.heading_level.is_none() && !para.is_code_block && overlap_frac > 0.3 =>
                {
                    para.is_list_item = false;
                    let text: String = if !para.text.is_empty() {
                        para.text.clone()
                    } else {
                        para.lines
                            .iter()
                            .flat_map(|l| l.segments.iter())
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    };
                    let word_count = text.split_whitespace().count();
                    if word_count <= super::constants::MAX_HEADING_WORD_COUNT && !is_separator_text(&text) {
                        let level = infer_heading_level_from_text(&text, hint.class);
                        para.heading_level = Some(level);
                        para.layout_class = Some(hint.class);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check if text is a separator/filler line (dashes, underscores, tildes, etc.)
/// that should never be classified as a heading.
pub(super) fn is_separator_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    let total = trimmed.chars().count();
    let alnum = trimmed.chars().filter(|c| c.is_alphanumeric()).count();
    // Pure separator: no alphanumeric characters at all
    if alnum == 0 {
        return true;
    }
    // Mostly separator: very few alphanumeric chars among filler (dashes, underscores, tildes, etc.)
    // e.g. "------------- M W _ _ _ _ _ _" or "---~ ---------"
    // Require at least 6 total chars and <15% alphanumeric ratio
    total >= 6 && (alnum as f64 / total as f64) < 0.15
}

/// Infer heading level from section numbering in the text.
///
/// Academic papers use numbering to indicate heading depth:
/// - "1 Introduction" → H2 (top-level section)
/// - "3.2 AI models" → H3 (sub-section)
/// - "3.2.1 Details" → H4 (sub-sub-section)
/// - "Layout Analysis Model" (no number) → H2 (default for SectionHeader)
pub(super) fn infer_heading_level_from_text(text: &str, hint_class: LayoutHintClass) -> u8 {
    if hint_class == LayoutHintClass::Title {
        return 1;
    }

    let trimmed = text.trim();

    // Check for section numbering pattern at the start.
    // Supports both numeric ("3.2") and alphabetic ("A.", "B.1") prefixes.
    let first_char = trimmed.chars().next().unwrap_or(' ');
    let is_alpha_prefix = first_char.is_ascii_alphabetic()
        && trimmed.len() >= 2
        && matches!(trimmed.as_bytes().get(1), Some(b'.' | b')' | b' '));

    let numbering_end = if is_alpha_prefix {
        // Alphabetic prefix: "A." or "A.1" or "A.1.2"
        // Start after the letter, continue through digits and dots
        let after_letter = &trimmed[1..];
        let rest_end = after_letter
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(0);
        1 + rest_end // include the letter
    } else {
        // Numeric prefix: "3.2.1"
        trimmed.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(0)
    };

    if numbering_end == 0 {
        // No numbering → default H2 for SectionHeader
        return 2;
    }

    let numbering = &trimmed[..numbering_end];
    // Count dots to determine depth: "3" → 0 dots → H2, "3.2" → 1 dot → H3
    let dot_count = numbering.chars().filter(|&c| c == '.').count();

    // Trailing dot (e.g., "3." or "A.") doesn't count as depth indicator
    let effective_dots = if numbering.ends_with('.') {
        dot_count.saturating_sub(1)
    } else {
        dot_count
    };

    match effective_dots {
        0 => 2, // "1 Introduction" or "A Proofs" → H2
        1 => 3, // "3.2 AI models" or "A.1 Details" → H3
        _ => 4, // "3.2.1 Details" or "A.1.2 Sub" → H4
    }
}

/// Apply a single hint's classification to a paragraph.
///
/// `body_font_size`: when provided, used to guard against promoting body-text-sized
/// paragraphs to headings (unnumbered SectionHeader at body font size is likely a
/// false positive from the layout model).
pub(super) fn apply_hint_to_paragraph(para: &mut PdfParagraph, hint: &LayoutHint, body_font_size: Option<f32>) {
    tracing::debug!(
        hint_class = ?hint.class,
        confidence = hint.confidence,
        old_heading = ?para.heading_level,
        "applying layout hint"
    );

    para.layout_class = Some(hint.class);

    // Get text from full-text path or segment path.
    let para_text: String = if !para.text.is_empty() {
        para.text.clone()
    } else {
        para.lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let word_count = para_text.split_whitespace().count();
    let is_sep = is_separator_text(&para_text);

    match hint.class {
        LayoutHintClass::Title
            if !is_sep
                && (para.heading_level.is_none() || hint.confidence >= 0.7) => {
                    if word_count <= super::constants::MAX_HEADING_WORD_COUNT {
                        para.heading_level = Some(1);
                    }
                }
        LayoutHintClass::SectionHeader
            if !is_sep
                && (para.heading_level.is_none() || hint.confidence >= 0.7) => {
                    let trimmed = para_text.trim();
                    let too_long = word_count > super::constants::MAX_HEADING_WORD_COUNT;
                    let ends_period = trimmed.ends_with('.')
                        && !super::classify::is_section_pattern(trimmed);
                    let ends_colon = trimmed.ends_with(':');
                    let is_figure = super::regions::looks_like_figure_label(trimmed);
                    let is_monospace = if !para.text.is_empty() {
                        para.is_monospace_hint()
                    } else {
                        para.lines.iter().all(|l| l.is_monospace)
                    };
                    let text_level = infer_heading_level_from_text(&para_text, hint.class);
                    // Guard: block unnumbered text near body font size (within
                    // body-1.5pt to body+0.5pt). The layout model often misclassifies
                    // bold body text as SectionHeader. Headings well below body size
                    // (e.g., 8pt headings in 12pt body) pass through.
                    //
                    // Exception: when the layout model has high confidence (>=0.7) AND
                    // the paragraph is bold, the near-body guard is relaxed — bold
                    // formatting at body size is a legitimate heading style. Only block
                    // if the text looks like a full sentence (ends with period AND >8 words).
                    let near_body = body_font_size
                        .is_some_and(|body| body > 0.0
                            && para.dominant_font_size >= body - 1.5
                            && para.dominant_font_size <= body + 0.5);
                    let is_unnumbered = text_level == 2;
                    let high_confidence_bold = hint.confidence >= 0.7 && para.is_bold;
                    let looks_like_sentence = trimmed.ends_with('.') && word_count > 8;
                    let body_size_guard = near_body && is_unnumbered
                        && !(high_confidence_bold && !looks_like_sentence);
                    if !too_long && !ends_period && !ends_colon && !is_figure && !is_monospace && !body_size_guard {
                        para.heading_level = Some(text_level);
                    }
                }
        LayoutHintClass::Code => {
            para.is_code_block = true;
            para.heading_level = None;
        }
        LayoutHintClass::Formula => {
            para.is_formula = true;
            para.heading_level = None;
        }
        LayoutHintClass::ListItem => {
            para.is_list_item = true;
        }
        LayoutHintClass::PageHeader | LayoutHintClass::PageFooter => {
            para.is_page_furniture = true;
        }
        LayoutHintClass::Text | LayoutHintClass::Caption | LayoutHintClass::Footnote
            // Layout model says this is body text, not a heading.
            // Demote font-size-classified headings when layout has high confidence.
            if para.heading_level.is_some() && hint.confidence >= 0.7 => {
                tracing::trace!(
                    ?hint.class,
                    hint_confidence = hint.confidence,
                    old_heading_level = ?para.heading_level,
                    "Demoting heading: layout model classifies as body text"
                );
                para.heading_level = None;
            }
        _ => {}
    }
}

// ParaBBox replaced by geometry::Rect.

/// Compute a paragraph's bounding box from its line segments' positional data.
///
/// Returns `None` if the paragraph has no segments with valid positional data.
///
/// In PDF coordinates (y=0 at bottom, y increases upward):
/// - `seg.y` / `seg.baseline_y` is the text baseline (near the bottom of glyphs).
/// - Text extends UPWARD from the baseline by roughly the ascent (~80% of font size).
/// - Text extends DOWNWARD from the baseline by the descent (~20% of font size).
///
/// For layout detection matching, we approximate the visual text extent as:
/// - top = baseline + height (covers ascenders)
/// - bottom = baseline (descent is small and usually within the layout hint's margin)
fn compute_paragraph_bbox(para: &PdfParagraph) -> Option<Rect> {
    // Prefer block-level bbox from structure tree (accurate block bounds).
    if let Some((left, bottom, right, top)) = para.block_bbox
        && right > left
        && top > bottom
    {
        return Some(Rect::from_lbrt(left, bottom, right, top));
    }

    // Fall back to computing bbox from segment positions (heuristic path).
    let mut left = f32::MAX;
    let mut right = f32::MIN;
    let mut bottom = f32::MAX;
    let mut top = f32::MIN;
    let mut has_data = false;

    for line in &para.lines {
        for seg in &line.segments {
            // Skip segments with no positional data
            if seg.x == 0.0 && seg.width == 0.0 && seg.y == 0.0 && seg.height == 0.0 {
                continue;
            }
            has_data = true;
            left = left.min(seg.x);
            right = right.max(seg.x + seg.width);
            // seg.y is the baseline. Text extends upward by ~font_size (seg.height).
            top = top.max(seg.y + seg.height);
            bottom = bottom.min(seg.y);
        }
    }

    if has_data {
        Some(Rect::from_lbrt(left, bottom, right, top))
    } else {
        None
    }
}

// hint_containment removed — replaced by Rect::intersection_over_self().

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::PdfLine;

    fn make_segment(text: &str, x: f32, y: f32, width: f32, height: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: y,
        }
    }

    fn make_line_at(segments: Vec<SegmentData>, baseline_y: f32) -> PdfLine {
        PdfLine {
            segments,
            baseline_y,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn make_line(segments: Vec<SegmentData>) -> PdfLine {
        make_line_at(segments, 700.0)
    }

    fn make_para(x: f32, y: f32, width: f32, height: f32) -> PdfParagraph {
        PdfParagraph {
            text: String::new(),
            lines: vec![make_line(vec![make_segment("text", x, y, width, height)])],
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

    fn make_hint(class: LayoutHintClass, confidence: f32, left: f32, bottom: f32, right: f32, top: f32) -> LayoutHint {
        LayoutHint {
            class,
            confidence,
            left,
            bottom,
            right,
            top,
        }
    }

    // ── apply_layout_overrides tests (paragraph-level, used for struct tree path) ──

    #[test]
    fn test_title_override() {
        let mut paragraphs = vec![make_para(50.0, 750.0, 500.0, 20.0)];
        let hints = vec![make_hint(LayoutHintClass::Title, 0.9, 40.0, 745.0, 560.0, 775.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(1));
        assert_eq!(paragraphs[0].layout_class, Some(LayoutHintClass::Title));
    }

    #[test]
    fn test_section_header_override() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        let hints = vec![make_hint(
            LayoutHintClass::SectionHeader,
            0.85,
            40.0,
            598.0,
            400.0,
            620.0,
        )];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_low_confidence_ignored() {
        let mut paragraphs = vec![make_para(50.0, 750.0, 500.0, 20.0)];
        let hints = vec![make_hint(LayoutHintClass::Title, 0.3, 40.0, 745.0, 560.0, 775.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, None);
        assert_eq!(paragraphs[0].layout_class, None);
    }

    #[test]
    fn test_existing_heading_overridden_by_high_confidence() {
        // High-confidence layout model overrides font-size heading level
        let mut paragraphs = vec![make_para(50.0, 750.0, 500.0, 20.0)];
        paragraphs[0].heading_level = Some(3);
        let hints = vec![make_hint(
            LayoutHintClass::SectionHeader,
            0.9,
            40.0,
            745.0,
            560.0,
            775.0,
        )];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(2)); // SectionHeader → H2
    }

    #[test]
    fn test_existing_heading_preserved_low_confidence() {
        // Low-confidence layout model does NOT override existing heading
        let mut paragraphs = vec![make_para(50.0, 750.0, 500.0, 20.0)];
        paragraphs[0].heading_level = Some(3);
        let hints = vec![make_hint(
            LayoutHintClass::SectionHeader,
            0.6, // Below 0.7 threshold
            40.0,
            745.0,
            560.0,
            775.0,
        )];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(3)); // Preserved
    }

    #[test]
    fn test_empty_hints() {
        let mut paragraphs = vec![make_para(50.0, 750.0, 500.0, 20.0)];
        apply_layout_overrides(&mut paragraphs, &[], 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_intersection_over_self_full() {
        let hint = Rect::from_lbrt(0.0, 0.0, 612.0, 792.0);
        let para = Rect::from_lbrt(50.0, 100.0, 550.0, 200.0);
        let containment = para.intersection_over_self(&hint);
        assert!(
            (containment - 1.0).abs() < 0.01,
            "Full containment expected: {}",
            containment
        );
    }

    #[test]
    fn test_intersection_over_self_none() {
        let hint = Rect::from_lbrt(0.0, 500.0, 100.0, 600.0);
        let para = Rect::from_lbrt(200.0, 100.0, 500.0, 200.0);
        let containment = para.intersection_over_self(&hint);
        assert!(
            (containment - 0.0).abs() < 0.01,
            "No containment expected: {}",
            containment
        );
    }

    // ── infer_heading_level_from_text tests ──

    #[test]
    fn test_infer_heading_level_title() {
        assert_eq!(
            infer_heading_level_from_text("Docling Report", LayoutHintClass::Title),
            1
        );
    }

    #[test]
    fn test_infer_heading_level_top_section() {
        // "3 Processing pipeline" → H2
        assert_eq!(
            infer_heading_level_from_text("3 Processing pipeline", LayoutHintClass::SectionHeader),
            2
        );
    }

    #[test]
    fn test_infer_heading_level_subsection() {
        // "3.2 AI models" → H3
        assert_eq!(
            infer_heading_level_from_text("3.2 AI models", LayoutHintClass::SectionHeader),
            3
        );
    }

    #[test]
    fn test_infer_heading_level_subsubsection() {
        // "3.2.1 Details" → H4
        assert_eq!(
            infer_heading_level_from_text("3.2.1 Details", LayoutHintClass::SectionHeader),
            4
        );
    }

    #[test]
    fn test_infer_heading_level_trailing_dot() {
        // "3. Processing" → trailing dot, still H2
        assert_eq!(
            infer_heading_level_from_text("3. Processing", LayoutHintClass::SectionHeader),
            2
        );
    }

    #[test]
    fn test_infer_heading_level_no_number() {
        // "Layout Analysis Model" → no number, default H2
        assert_eq!(
            infer_heading_level_from_text("Layout Analysis Model", LayoutHintClass::SectionHeader),
            2
        );
    }

    // ── proportional matching tests (structure tree path) ──

    #[test]
    fn test_no_positional_data_proportional_applies_page_furniture() {
        // Proportional matching only applies PageHeader/PageFooter (furniture)
        // because positional imprecision makes heading/list/code overrides unreliable.
        let mut paragraphs = vec![PdfParagraph {
            text: String::new(),
            lines: vec![make_line(vec![make_segment("text", 0.0, 0.0, 0.0, 0.0)])],
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
        }];

        // Title hint IS applied via proportional matching (heading level inferred)
        let hints = vec![make_hint(LayoutHintClass::Title, 0.9, 40.0, 0.0, 560.0, 760.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(1));
        assert_eq!(paragraphs[0].layout_class, Some(LayoutHintClass::Title));

        // Reset for next test
        paragraphs[0].heading_level = None;
        paragraphs[0].layout_class = None;

        // PageHeader hint SHOULD be applied via proportional matching
        let hints = vec![make_hint(LayoutHintClass::PageHeader, 0.9, 40.0, 0.0, 560.0, 760.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert!(paragraphs[0].is_page_furniture);
        assert_eq!(paragraphs[0].layout_class, Some(LayoutHintClass::PageHeader));
    }

    // ── is_separator_text tests ──

    #[test]
    fn test_separator_pure_dashes() {
        assert!(is_separator_text("----------"));
    }

    #[test]
    fn test_separator_underscores() {
        assert!(is_separator_text("___________"));
    }

    #[test]
    fn test_separator_mixed_with_few_alnum() {
        // < 15% alphanumeric among filler chars
        assert!(is_separator_text("------- M ---------"));
    }

    #[test]
    fn test_separator_empty_string() {
        assert!(!is_separator_text(""));
        assert!(!is_separator_text("   "));
    }

    #[test]
    fn test_separator_normal_text() {
        assert!(!is_separator_text("Hello World"));
    }

    #[test]
    fn test_separator_short_symbols() {
        // Short symbol strings are separators (no alphanumeric)
        assert!(is_separator_text("---"));
    }

    // ── infer_heading_level_from_text additional tests ──

    #[test]
    fn test_infer_heading_level_alpha_prefix() {
        // "A. Proofs" → alphabetic prefix → H2
        assert_eq!(
            infer_heading_level_from_text("A. Proofs", LayoutHintClass::SectionHeader),
            2
        );
    }

    #[test]
    fn test_infer_heading_level_alpha_subsection() {
        // "A.1 Details" → 1 effective dot → H3
        assert_eq!(
            infer_heading_level_from_text("A.1 Details", LayoutHintClass::SectionHeader),
            3
        );
    }

    #[test]
    fn test_infer_heading_level_deep_subsection() {
        // "1.2.3.4 Very deep" → 3 dots → H4 (capped at 4)
        assert_eq!(
            infer_heading_level_from_text("1.2.3.4 Very deep", LayoutHintClass::SectionHeader),
            4
        );
    }

    // ── apply_hint_to_paragraph tests ──

    #[test]
    fn test_code_override() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        paragraphs[0].heading_level = Some(2);
        let hints = vec![make_hint(LayoutHintClass::Code, 0.9, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert!(paragraphs[0].is_code_block);
        assert_eq!(paragraphs[0].heading_level, None); // Heading cleared
    }

    #[test]
    fn test_formula_override() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        let hints = vec![make_hint(LayoutHintClass::Formula, 0.9, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert!(paragraphs[0].is_formula);
    }

    #[test]
    fn test_list_item_override() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        let hints = vec![make_hint(LayoutHintClass::ListItem, 0.9, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert!(paragraphs[0].is_list_item);
    }

    #[test]
    fn test_body_text_demotes_heading() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        paragraphs[0].heading_level = Some(2);
        let hints = vec![make_hint(LayoutHintClass::Text, 0.9, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_body_text_low_confidence_preserves_heading() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        paragraphs[0].heading_level = Some(2);
        let hints = vec![make_hint(LayoutHintClass::Text, 0.6, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert_eq!(paragraphs[0].heading_level, Some(2)); // Preserved, confidence < 0.7
    }

    #[test]
    fn test_page_footer_override() {
        let mut paragraphs = vec![make_para(50.0, 600.0, 300.0, 16.0)];
        let hints = vec![make_hint(LayoutHintClass::PageFooter, 0.9, 40.0, 598.0, 400.0, 620.0)];
        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);
        assert!(paragraphs[0].is_page_furniture);
    }

    #[test]
    fn test_separator_text_not_promoted_to_heading() {
        // A line of dashes should not become a heading even if layout says SectionHeader
        let mut para = PdfParagraph {
            text: String::new(),
            lines: vec![make_line(vec![make_segment("----------", 50.0, 600.0, 300.0, 16.0)])],
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
        };
        let hint = make_hint(LayoutHintClass::SectionHeader, 0.9, 40.0, 598.0, 400.0, 620.0);
        apply_hint_to_paragraph(&mut para, &hint, None);
        assert_eq!(para.heading_level, None); // Separator not promoted
    }

    #[test]
    fn test_compute_paragraph_bbox_no_positional_data() {
        // Segments with all-zero positions should return None
        let para = PdfParagraph {
            text: String::new(),
            lines: vec![make_line(vec![make_segment("text", 0.0, 0.0, 0.0, 0.0)])],
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
        };
        assert!(compute_paragraph_bbox(&para).is_none());
    }

    #[test]
    fn test_compute_paragraph_bbox_with_block_bbox() {
        let para = PdfParagraph {
            text: String::new(),
            lines: vec![make_line(vec![make_segment("text", 0.0, 0.0, 0.0, 0.0)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: Some((50.0, 100.0, 400.0, 120.0)),
        };
        let bbox = compute_paragraph_bbox(&para).unwrap();
        assert!((bbox.left - 50.0).abs() < f32::EPSILON);
        assert!((bbox.y_min - 100.0).abs() < f32::EPSILON);
        assert!((bbox.right - 400.0).abs() < f32::EPSILON);
        assert!((bbox.y_max - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_paragraph_bbox_from_segments() {
        let para = PdfParagraph {
            text: String::new(),
            lines: vec![
                make_line_at(vec![make_segment("A", 50.0, 700.0, 100.0, 12.0)], 700.0),
                make_line_at(vec![make_segment("B", 60.0, 680.0, 120.0, 14.0)], 680.0),
            ],
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
        };
        let bbox = compute_paragraph_bbox(&para).unwrap();
        assert!((bbox.left - 50.0).abs() < f32::EPSILON);
        assert!((bbox.y_min - 680.0).abs() < f32::EPSILON);
        // right = max(50+100, 60+120) = max(150, 180) = 180
        assert!((bbox.right - 180.0).abs() < f32::EPSILON);
        // top = max(700+12, 680+14) = max(712, 694) = 712
        assert!((bbox.y_max - 712.0).abs() < f32::EPSILON);
    }
}
