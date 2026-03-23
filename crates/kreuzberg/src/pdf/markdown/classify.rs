//! Heading classification for paragraphs using font-size clustering.

use super::constants::{MAX_BOLD_HEADING_WORD_COUNT, MAX_HEADING_DISTANCE_MULTIPLIER, MAX_HEADING_WORD_COUNT};
use super::regions::looks_like_figure_label;
use super::types::PdfParagraph;

/// Classify paragraphs as headings or body using the global heading map and bold heuristic.
pub(super) fn classify_paragraphs(paragraphs: &mut [PdfParagraph], heading_map: &[(f32, Option<u8>)]) {
    let gap_info = precompute_gap_info(heading_map);
    // Body font size = centroid of the cluster with no heading level
    let body_font_size = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(centroid, _)| *centroid)
        .unwrap_or(0.0);
    for para in paragraphs.iter_mut() {
        let word_count: usize = para
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.split_whitespace().count())
            .sum();

        // Pass 1: font-size-based heading classification.
        // When the layout model says Text, only promote to heading if there is
        // strong typographic evidence: the font size maps to a heading cluster
        // AND the paragraph is bold (sub-headings the model missed).
        let layout_says_text = para.layout_class == Some(super::types::LayoutHintClass::Text);
        let heading_level = find_heading_level(para.dominant_font_size, heading_map, &gap_info);
        let heading_level = if layout_says_text {
            // Override layout Text only when bold + heading font size → sub-heading
            if para.is_bold && heading_level.is_some() {
                heading_level
            } else {
                None
            }
        } else {
            heading_level
        };

        if let Some(level) = heading_level
            && word_count <= MAX_HEADING_WORD_COUNT
        {
            let text: String = para
                .lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !super::layout_classify::is_separator_text(&text) {
                para.heading_level = Some(level);
                continue;
            }
        }

        // Pass 2: bold or italic short paragraphs → section headings (H2).
        // Some documents use italic instead of bold for section titles.
        let is_italic = !para.lines.is_empty() && para.lines.iter().all(|l| l.segments.iter().all(|s| s.is_italic));
        // When the layout model says Text, only allow bold promotion if the font
        // size is distinctly larger than body text (a sub-heading the model missed).
        // Pure body-size bold text (e.g., emphasis) stays as body.
        let layout_text_overridable = if layout_says_text {
            body_font_size > 0.0 && para.dominant_font_size > body_font_size + 0.5
        } else {
            true
        };
        if (para.is_bold || is_italic)
            && !para.is_list_item
            && layout_text_overridable
            && word_count <= MAX_BOLD_HEADING_WORD_COUNT
        {
            let text: String = para
                .lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let t = text.trim();
            // Italic-only paragraphs need extra guards: academic papers use italic
            // for author names, affiliations, emails which shouldn't be headings.
            let italic_ok = if is_italic && !para.is_bold {
                !t.contains('@') && !t.contains(',') && t.chars().next().is_some_and(|c| c.is_uppercase())
            } else {
                true
            };
            // Guard: very short text (1-2 words) at body font size is typically a
            // figure label (e.g., "Untightened nut"), not a real heading.
            let too_short_at_body =
                word_count <= 2 && body_font_size > 0.0 && para.dominant_font_size <= body_font_size + 0.5;
            let period_ok = !t.ends_with('.') || is_section_pattern(t);
            if italic_ok
                && !too_short_at_body
                && period_ok
                && !t.ends_with(':')
                && !looks_like_figure_label(t)
                && !super::layout_classify::is_separator_text(t)
            {
                para.heading_level = Some(2);
            }
        }

        // Pass 3: code blocks should never be headings
        if para.is_code_block {
            para.heading_level = None;
        }
    }
}

/// Find the heading level for a given font size by matching against the cluster centroids.
pub(super) fn find_heading_level(font_size: f32, heading_map: &[(f32, Option<u8>)], gap_info: &GapInfo) -> Option<u8> {
    if heading_map.is_empty() {
        return None;
    }
    if heading_map.len() == 1 {
        return heading_map[0].1;
    }

    let mut best_distance = f32::INFINITY;
    let mut best_level: Option<u8> = None;
    for &(centroid, level) in heading_map {
        let dist = (font_size - centroid).abs();
        if dist < best_distance {
            best_distance = dist;
            best_level = level;
        }
    }

    if best_distance > MAX_HEADING_DISTANCE_MULTIPLIER * gap_info.avg_gap {
        return None;
    }

    best_level
}

pub(super) struct GapInfo {
    avg_gap: f32,
}

pub(super) fn precompute_gap_info(heading_map: &[(f32, Option<u8>)]) -> GapInfo {
    if heading_map.len() <= 1 {
        return GapInfo { avg_gap: f32::INFINITY };
    }

    let mut centroids: Vec<f32> = heading_map.iter().map(|(c, _)| *c).collect();
    centroids.sort_by(|a, b| a.total_cmp(b));
    let gaps: Vec<f32> = centroids.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let avg_gap = if gaps.is_empty() {
        f32::INFINITY
    } else {
        gaps.iter().sum::<f32>() / gaps.len() as f32
    };

    GapInfo { avg_gap }
}

/// Refine heading levels across the entire document.
///
/// 1. Promotes the first heading to H1 when no H1 exists (title inference).
/// 2. Merges consecutive H1 headings at the same font size into one title (any page).
/// 3. Demotes numbered section headings from H1 to H2 when a non-numbered title H1 exists.
pub(super) fn refine_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    // Step 0: If no H1 exists but H2+ headings do, promote the first heading
    // on page 0 (or the largest font heading) to H1. Many docs have a title
    // that gets classified as H2 by bold promotion or font clustering.
    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count == 0 {
        let has_any_heading = all_pages
            .iter()
            .flat_map(|page| page.iter())
            .any(|p| p.heading_level.is_some());
        if has_any_heading {
            promote_title_heading(all_pages);
        }
    }

    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    // Step 1: Merge consecutive H1s at the same font size on each page.
    // Split titles like "KAISUN HOLDINGS" / "LIMITED" appear as consecutive
    // H1 paragraphs with the same font size.
    for page in all_pages.iter_mut() {
        merge_consecutive_h1s(page);
    }

    // Re-count after merging
    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    // Step 2: Demote numbered section headings.
    // If the first H1 is a title (not starting with a number), demote subsequent
    // numbered H1s to H2.
    let first_h1_is_title = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .find(|p| p.heading_level == Some(1))
        .is_some_and(|p| !starts_with_section_number(&paragraph_plain_text(p)));

    if !first_h1_is_title {
        return;
    }

    let mut found_first = false;
    for page in all_pages.iter_mut() {
        for para in page.iter_mut() {
            if para.heading_level == Some(1) {
                if !found_first {
                    found_first = true;
                    continue;
                }
                if starts_with_section_number(&paragraph_plain_text(para)) {
                    para.heading_level = Some(2);
                }
            }
        }
    }
}

/// Check if text looks like a section/legal heading that legitimately ends with a period.
/// Uses language-agnostic structural signals only:
/// - Starts with § (universal section symbol)
/// - All-caps short text (e.g., "ARTICLE IV.", "CHAPITRE 3.")
/// - Starts with a section number (e.g., "3.2. Methods")
pub(super) fn is_section_pattern(text: &str) -> bool {
    let t = text.trim();
    if t.starts_with('§') {
        return true;
    }
    // All-caps short text ending with period is likely a structural heading
    let words = t.split_whitespace().count();
    if words <= 6 && t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
        return true;
    }
    // Starts with section number: "3.2. Methods" or "162-56. Place"
    starts_with_section_number(t)
}

/// Check if text starts with a section number pattern (e.g., "1 ", "2.1 ", "A.").
fn starts_with_section_number(text: &str) -> bool {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let next = bytes[digit_end];
        return next == b' ' || next == b'.' || next == b')';
    }
    false
}

/// Demote unnumbered H2 headings to H3 when they appear between numbered H2 sections.
///
/// In documents with numbered sections (e.g., "1 INTRODUCTION", "5 EXPERIMENTS"),
/// unnumbered headings between consecutive numbered H2s are typically sub-sections.
/// For example, "Baselines for Object Detection" between "5 EXPERIMENTS" and
/// "6 CONCLUSION" should be H3, not H2.
///
/// Only applies when the document has at least 3 numbered H2 headings, indicating
/// a consistent numbering scheme.
pub(super) fn demote_unnumbered_subsections(all_pages: &mut [Vec<PdfParagraph>]) {
    // Collect all H2 headings with their position and numbered status
    let mut h2_info: Vec<(usize, usize, bool)> = Vec::new(); // (page_idx, para_idx, is_numbered)
    for (page_idx, page) in all_pages.iter().enumerate() {
        for (para_idx, para) in page.iter().enumerate() {
            if para.heading_level == Some(2) {
                let text = paragraph_plain_text(para);
                h2_info.push((page_idx, para_idx, starts_with_section_number(&text)));
            }
        }
    }

    let numbered_count = h2_info.iter().filter(|(_, _, numbered)| *numbered).count();
    if numbered_count < 3 {
        return; // Not enough numbered sections to establish a pattern
    }

    // Find ranges: between consecutive numbered H2s, demote unnumbered H2s to H3
    let numbered_positions: Vec<usize> = h2_info
        .iter()
        .enumerate()
        .filter(|(_, (_, _, numbered))| *numbered)
        .map(|(idx, _)| idx)
        .collect();

    for window in numbered_positions.windows(2) {
        let start = window[0];
        let end = window[1];
        // Demote unnumbered H2s between these two numbered H2s
        for &(page_idx, para_idx, is_numbered) in &h2_info[start + 1..end] {
            if !is_numbered {
                // Don't demote headings confirmed by the layout model.
                let layout_confirmed = matches!(
                    all_pages[page_idx][para_idx].layout_class,
                    Some(super::types::LayoutHintClass::SectionHeader | super::types::LayoutHintClass::Title)
                );
                if !layout_confirmed {
                    all_pages[page_idx][para_idx].heading_level = Some(3);
                }
            }
        }
    }
}

/// Demote long runs of consecutive same-level headings to body text.
///
/// When the layout model (or font-size classification) produces 4+ consecutive
/// headings at the same level with no intervening body text, they're likely
/// misclassified (e.g., song lyrics, list items, short centered paragraphs).
/// Real documents rarely have more than 3 consecutive headings.
pub(super) fn demote_heading_runs(all_pages: &mut [Vec<PdfParagraph>]) {
    const MAX_CONSECUTIVE: usize = 3;

    for page in all_pages.iter_mut() {
        let mut run_start = 0;
        while run_start < page.len() {
            let Some(level) = page[run_start].heading_level else {
                run_start += 1;
                continue;
            };

            // Find the end of this consecutive same-level heading run
            let mut run_end = run_start + 1;
            while run_end < page.len() && page[run_end].heading_level == Some(level) {
                run_end += 1;
            }

            let run_len = run_end - run_start;
            if run_len > MAX_CONSECUTIVE {
                // Demote all but the first heading in the run, unless
                // the layout model confirmed it as a heading region.
                for para in &mut page[run_start + 1..run_end] {
                    let layout_confirmed = matches!(
                        para.layout_class,
                        Some(super::types::LayoutHintClass::SectionHeader | super::types::LayoutHintClass::Title)
                    );
                    if !layout_confirmed {
                        para.heading_level = None;
                    }
                }
            }

            run_start = run_end;
        }
    }
}

/// Extract plain text from a paragraph.
fn paragraph_plain_text(para: &PdfParagraph) -> String {
    para.lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Promote the most likely title heading to H1 when no H1 exists.
///
/// Strategy:
/// 1. If any heading has layout_class == Title, promote it to H1.
/// 2. Otherwise, on the first page only, promote the heading with the largest
///    font size IF it's clearly larger than other headings (at least 1.5pt gap).
fn promote_title_heading(all_pages: &mut [Vec<PdfParagraph>]) {
    // First check for layout-confirmed Title
    for page in all_pages.iter_mut() {
        for para in page.iter_mut() {
            if para.heading_level.is_some() && para.layout_class == Some(super::types::LayoutHintClass::Title) {
                para.heading_level = Some(1);
                return;
            }
        }
    }

    // Only promote on the first page, and only if one heading is clearly
    // larger than others (to avoid false promotion when all headings are same size)
    if all_pages.is_empty() {
        return;
    }
    let page = &all_pages[0];
    let headings: Vec<(usize, f32)> = page
        .iter()
        .enumerate()
        .filter(|(_, p)| p.heading_level.is_some())
        .map(|(i, p)| (i, p.dominant_font_size))
        .collect();

    if headings.is_empty() {
        return;
    }

    // If there's exactly one heading on page 0, promote it
    if headings.len() == 1 {
        all_pages[0][headings[0].0].heading_level = Some(1);
        return;
    }

    // If multiple headings, only promote if the largest is clearly bigger (1.5pt gap)
    let max_size = headings.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);
    let second_max = headings
        .iter()
        .map(|(_, s)| *s)
        .filter(|s| *s < max_size)
        .fold(0.0f32, f32::max);

    if max_size - second_max >= 1.5
        && let Some(&(idx, _)) = headings.iter().find(|(_, s)| *s == max_size)
    {
        all_pages[0][idx].heading_level = Some(1);
    }
}

/// Merge consecutive H1 paragraphs at the same font size into a single heading.
///
/// Split titles (e.g., "KAISUN HOLDINGS" on one line, "LIMITED" on the next)
/// often produce separate H1 paragraphs. When they share the same font size
/// they should be a single heading.
fn merge_consecutive_h1s(page: &mut Vec<PdfParagraph>) {
    let mut i = 0;
    while i < page.len() {
        if page[i].heading_level != Some(1) {
            i += 1;
            continue;
        }
        // Find the run of consecutive H1s at the same font size.
        let base_fs = page[i].dominant_font_size;
        let mut run_end = i + 1;
        while run_end < page.len()
            && page[run_end].heading_level == Some(1)
            && (page[run_end].dominant_font_size - base_fs).abs() < 0.5
        {
            run_end += 1;
        }
        if run_end - i > 1 {
            // Merge lines from paragraphs [i+1..run_end] into page[i].
            let mut merged_lines = std::mem::take(&mut page[i].lines);
            for para in &page[i + 1..run_end] {
                merged_lines.extend(para.lines.clone());
            }
            page[i].lines = merged_lines;
            page.drain(i + 1..run_end);
        }
        i += 1;
    }
}

/// Detect short paragraphs that repeat on many pages (headers, footers, watermarks).
///
/// Paragraphs whose normalized text appears on >50% of pages are marked as
/// page furniture. Only considers short paragraphs (≤8 words) to avoid
/// false positives on repeated body text.
///
/// Two normalization passes are used:
/// 1. **Exact**: lowercase + trim — catches identical running headers.
/// 2. **Fuzzy**: alphanumeric-only — catches variants caused by PDF extraction
///    artefacts such as `©` → `O` (so `© ISO 2021` and `OISO 2021` both
///    reduce to `oiso2021`), missing spaces, or punctuation differences.
///    Fuzzy matching is only applied to candidates with ≥6 alphanumeric
///    characters to avoid spurious matches on very short strings.
pub(super) fn mark_cross_page_repeating_text(all_pages: &mut [Vec<PdfParagraph>]) {
    if all_pages.len() < 4 {
        return; // Not enough pages to detect repeats meaningfully.
    }

    // Build per-page candidate list: (exact_normalized, alphanum_key) pairs.
    // Collect all exact keys per page first (deduped), then count occurrences.
    let mut text_page_count: ahash::AHashMap<String, usize> = ahash::AHashMap::new();
    // Also track alphanum_key → set-of-exact-keys for the fuzzy pass.
    let mut alphanum_to_exact: ahash::AHashMap<String, ahash::AHashSet<String>> = ahash::AHashMap::new();
    // Count per alphanum_key across pages (deduped per page).
    let mut alphanum_page_count: ahash::AHashMap<String, usize> = ahash::AHashMap::new();

    for page in all_pages.iter() {
        // Use sets per page to count each key only once per page.
        let mut seen_exact: ahash::AHashSet<String> = ahash::AHashSet::new();
        let mut seen_alphanum: ahash::AHashSet<String> = ahash::AHashSet::new();
        for para in page {
            if para.is_page_furniture {
                continue;
            }
            // Include headings in cross-page repeat detection: a heading
            // appearing on >50% of pages is a running header, not a
            // section heading. The layout model often misclassifies these.
            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            let word_count = normalized.split_whitespace().count();
            if word_count == 0 || word_count > 8 {
                continue;
            }

            // Pass 1: exact normalized key.
            if seen_exact.insert(normalized.clone()) {
                *text_page_count.entry(normalized.clone()).or_insert(0) += 1;
            }

            // Pass 2: alphanumeric-only key for fuzzy matching.
            // Only build the key for candidates with enough alphanumeric chars
            // to avoid false positives on very short strings like page numbers.
            let alphanum_key: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
            if alphanum_key.len() >= 6 {
                alphanum_to_exact
                    .entry(alphanum_key.clone())
                    .or_default()
                    .insert(normalized.clone());
                if seen_alphanum.insert(alphanum_key.clone()) {
                    *alphanum_page_count.entry(alphanum_key).or_insert(0) += 1;
                }
            }
        }
    }

    let threshold = all_pages.len() / 2;

    // Exact-match repeating set.
    let mut repeating: ahash::AHashSet<String> = text_page_count
        .into_iter()
        .filter(|(_, count)| *count > threshold)
        .map(|(text, _)| text)
        .collect();

    // Fuzzy-match repeating set: an alphanum key that appears on >50% of pages
    // means ALL exact variants that share that key are running headers.
    for (alphanum_key, count) in alphanum_page_count {
        if count > threshold
            && let Some(exact_variants) = alphanum_to_exact.get(&alphanum_key)
        {
            for variant in exact_variants {
                repeating.insert(variant.clone());
            }
        }
    }

    if repeating.is_empty() {
        return;
    }

    // Mark matching paragraphs as furniture (including headings that are
    // actually running headers misclassified by the layout model).
    // Check both exact key and alphanum key against the repeating set.
    for page in all_pages.iter_mut() {
        for para in page.iter_mut() {
            if para.is_page_furniture {
                continue;
            }
            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            if repeating.contains(&normalized) {
                para.is_page_furniture = true;
                para.heading_level = None; // Demote from heading
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;

    fn make_paragraph(font_size: f32, segment_count: usize) -> PdfParagraph {
        let segments: Vec<SegmentData> = (0..segment_count)
            .map(|i| SegmentData {
                text: format!("word{}", i),
                x: i as f32 * 50.0,
                y: 700.0,
                width: 40.0,
                height: font_size,
                font_size,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
            })
            .collect();

        PdfParagraph {
            lines: vec![super::super::types::PdfLine {
                segments,
                baseline_y: 700.0,
                dominant_font_size: font_size,
                is_bold: false,
                is_monospace: false,
            }],
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
    fn test_classify_heading() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(18.0, 3)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_classify_body() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(12.0, 5)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_too_many_segments_for_heading() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(18.0, 21)]; // > MAX_HEADING_WORD_COUNT
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_find_heading_level_empty_map() {
        let gap_info = precompute_gap_info(&[]);
        assert_eq!(find_heading_level(12.0, &[], &gap_info), None);
    }

    #[test]
    fn test_find_heading_level_single_entry() {
        let heading_map = vec![(12.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        assert_eq!(find_heading_level(12.0, &heading_map, &gap_info), Some(1));
    }

    #[test]
    fn test_find_heading_level_outlier_rejected() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        // Font size 50.0 is way too far from any centroid
        assert_eq!(find_heading_level(50.0, &heading_map, &gap_info), None);
    }

    #[test]
    fn test_find_heading_level_close_match() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        assert_eq!(find_heading_level(15.5, &heading_map, &gap_info), Some(2));
    }

    #[test]
    fn test_classify_bold_short_paragraph_promoted_to_heading() {
        let heading_map = vec![(12.0, None)]; // no heading clusters
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.lines[0].is_bold = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_classify_bold_long_paragraph_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 20); // too many words
        para.is_bold = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_bold_list_item_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.is_list_item = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_few_segments_many_words_not_heading() {
        // 3 segments but each contains many words — total word count exceeds threshold
        let segments: Vec<SegmentData> = (0..3)
            .map(|i| SegmentData {
                text: "one two three four five six seven".to_string(),
                x: i as f32 * 200.0,
                y: 700.0,
                width: 180.0,
                height: 18.0,
                font_size: 18.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
            })
            .collect();

        let mut paragraphs = vec![PdfParagraph {
            lines: vec![super::super::types::PdfLine {
                segments,
                baseline_y: 700.0,
                dominant_font_size: 18.0,
                is_bold: false,
                is_monospace: false,
            }],
            dominant_font_size: 18.0,
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
        // 3 segments × 7 words = 21 words > MAX_HEADING_WORD_COUNT
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    fn make_h1(font_size: f32, text: &str) -> PdfParagraph {
        let mut p = make_paragraph(font_size, 1);
        p.lines[0].segments[0].text = text.to_string();
        p.heading_level = Some(1);
        p
    }

    #[test]
    fn test_merge_consecutive_h1s_same_font() {
        let mut page = vec![
            make_h1(24.0, "KAISUN HOLDINGS"),
            make_h1(24.0, "LIMITED"),
            make_paragraph(12.0, 3), // body
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].heading_level, Some(1));
        assert_eq!(page[0].lines.len(), 2);
    }

    #[test]
    fn test_merge_h1s_different_font_no_merge() {
        let mut page = vec![make_h1(24.0, "Title"), make_h1(18.0, "Subtitle")];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2); // Not merged — different font sizes.
    }

    #[test]
    fn test_cross_page_repeating_text() {
        let make_body = |text: &str| {
            let mut p = make_paragraph(12.0, 1);
            p.lines[0].segments[0].text = text.to_string();
            p
        };
        let mut pages = vec![
            vec![make_body("Page 1 of 10"), make_body("Unique content A")],
            vec![make_body("Page 1 of 10"), make_body("Unique content B")],
            vec![make_body("Page 1 of 10"), make_body("Unique content C")],
            vec![make_body("Page 1 of 10"), make_body("Unique content D")],
        ];
        mark_cross_page_repeating_text(&mut pages);
        // "Page 1 of 10" appears on all 4 pages (>50%) → furniture
        assert!(pages[0][0].is_page_furniture);
        assert!(!pages[0][1].is_page_furniture);
    }

    #[test]
    fn test_cross_page_repeating_marks_repeated_headings_as_furniture() {
        let mut pages = vec![];
        for _ in 0..6 {
            let mut h = make_h1(24.0, "Chapter");
            h.heading_level = Some(1);
            pages.push(vec![h, make_paragraph(12.0, 3)]);
        }
        mark_cross_page_repeating_text(&mut pages);
        // Headings repeating on >50% of pages are running headers —
        // they should be marked as furniture and demoted from heading.
        assert!(pages[0][0].is_page_furniture);
        assert!(pages[0][0].heading_level.is_none());
    }

    #[test]
    fn test_cross_page_repeating_fuzzy_matches_iso_variants() {
        // Simulate ISO running header extracted with copyright-symbol artefacts:
        // "© ISO 2021 – All rights reserved" appears on every page but with
        // varying extraction artefacts (© → O, spaces dropped, punctuation varies).
        let make_body = |text: &str| {
            let mut p = make_paragraph(12.0, 1);
            p.lines[0].segments[0].text = text.to_string();
            p
        };
        let mut pages = vec![
            // Even pages: copyright symbol extracted as "O" with spaces
            vec![
                make_body("O ISO 2021 All rights reserved"),
                make_body("Section content A"),
            ],
            vec![
                make_body("O ISO 2021 All rights reserved"),
                make_body("Section content B"),
            ],
            vec![
                make_body("O ISO 2021 All rights reserved"),
                make_body("Section content C"),
            ],
            // Odd pages: copyright symbol merged, no spaces, punctuation variant
            vec![make_body("OISO 2021Allrightsreserved"), make_body("Section content D")],
            vec![make_body("OISO 2021Allrightsreserved"), make_body("Section content E")],
            vec![make_body("OISO 2021Allrightsreserved"), make_body("Section content F")],
        ];
        mark_cross_page_repeating_text(&mut pages);
        // Both variants reduce to "oiso2021allrightsreserved" (alphanum-only key)
        // and appear on all 6 pages (>50%) → both should be marked as furniture.
        assert!(
            pages[0][0].is_page_furniture,
            "even-page copyright variant should be furniture"
        );
        assert!(
            pages[3][0].is_page_furniture,
            "odd-page copyright variant should be furniture"
        );
        // Unique body content should NOT be marked as furniture.
        assert!(!pages[0][1].is_page_furniture);
        assert!(!pages[3][1].is_page_furniture);
    }

    #[test]
    fn test_layout_text_bold_heading_font_promoted() {
        // Sub-heading: layout model says Text, but bold + heading font size → promote
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(16.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_layout_text_non_bold_heading_font_not_promoted() {
        // Layout says Text, heading font size but NOT bold → stay as body
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(16.0, 3);
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_body_font_not_promoted_pass1() {
        // Layout says Text, bold but body font size → Pass 1 skips (no heading cluster match)
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Pass 1 won't promote (body font), Pass 2 won't either (body font <= body + 0.5)
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_larger_font_promoted_pass2() {
        // Layout says Text, bold + font distinctly larger than body → Pass 2 promotes to H2
        let heading_map = vec![(12.0, None)]; // no heading clusters
        let mut para = make_paragraph(14.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }
}
