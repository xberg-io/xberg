//! Heading classification for paragraphs using font-size clustering.

use super::constants::{MAX_BOLD_HEADING_WORD_COUNT, MAX_HEADING_DISTANCE_MULTIPLIER, MAX_HEADING_WORD_COUNT};
use super::types::PdfParagraph;

/// Classify paragraphs as headings or body using the global heading map and bold heuristic.
pub(super) fn classify_paragraphs(paragraphs: &mut [PdfParagraph], heading_map: &[(f32, Option<u8>)]) {
    for para in paragraphs.iter_mut() {
        let word_count: usize = para
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.split_whitespace().count())
            .sum();

        // Pass 1: font-size-based heading classification
        let heading_level = find_heading_level(para.dominant_font_size, heading_map);

        if let Some(level) = heading_level
            && word_count <= MAX_HEADING_WORD_COUNT
        {
            para.heading_level = Some(level);
            continue;
        }

        // Pass 2: bold short paragraphs → section headings (H2)
        if para.is_bold && !para.is_list_item && word_count <= MAX_BOLD_HEADING_WORD_COUNT {
            para.heading_level = Some(2);
        }

        // Pass 3: code blocks should never be headings
        if para.is_code_block {
            para.heading_level = None;
        }
    }
}

/// Find the heading level for a given font size by matching against the cluster centroids.
pub(super) fn find_heading_level(font_size: f32, heading_map: &[(f32, Option<u8>)]) -> Option<u8> {
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

    // Compute average inter-cluster gap
    let mut centroids: Vec<f32> = heading_map.iter().map(|(c, _)| *c).collect();
    centroids.sort_by(|a, b| a.total_cmp(b));
    let gaps: Vec<f32> = centroids.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let avg_gap = if gaps.is_empty() {
        f32::INFINITY
    } else {
        gaps.iter().sum::<f32>() / gaps.len() as f32
    };

    if best_distance > MAX_HEADING_DISTANCE_MULTIPLIER * avg_gap {
        return None;
    }

    best_level
}

/// Refine heading levels across the entire document.
///
/// 1. Merges consecutive H1 headings at the document start into one title.
/// 2. Demotes numbered section headings from H1 to H2 when a non-numbered title H1 exists.
pub(super) fn refine_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    // Step 1: Merge consecutive leading H1s on the first page (split titles).
    if let Some(first_page) = all_pages.first_mut() {
        let h1_run_end = first_page.iter().take_while(|p| p.heading_level == Some(1)).count();

        if h1_run_end > 1 {
            let mut merged_lines = std::mem::take(&mut first_page[0].lines);
            for para in &first_page[1..h1_run_end] {
                merged_lines.extend(para.lines.clone());
            }
            first_page[0].lines = merged_lines;
            first_page.drain(1..h1_run_end);
        }
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

/// Extract plain text from a paragraph.
fn paragraph_plain_text(para: &PdfParagraph) -> String {
    para.lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
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
        let mut paragraphs = vec![make_paragraph(18.0, 20)]; // > MAX_HEADING_WORD_COUNT
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_find_heading_level_empty_map() {
        assert_eq!(find_heading_level(12.0, &[]), None);
    }

    #[test]
    fn test_find_heading_level_single_entry() {
        assert_eq!(find_heading_level(12.0, &[(12.0, Some(1))]), Some(1));
    }

    #[test]
    fn test_find_heading_level_outlier_rejected() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        // Font size 50.0 is way too far from any centroid
        assert_eq!(find_heading_level(50.0, &heading_map), None);
    }

    #[test]
    fn test_find_heading_level_close_match() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        assert_eq!(find_heading_level(15.5, &heading_map), Some(2));
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
                text: "one two three four five six".to_string(),
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
        }];
        // 3 segments × 6 words = 18 words > MAX_HEADING_WORD_COUNT
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }
}
