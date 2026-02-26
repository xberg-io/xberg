//! Line building from segments using baseline proximity.

use crate::pdf::hierarchy::SegmentData;

use super::constants::BASELINE_Y_TOLERANCE_FRACTION;
use super::types::PdfLine;

/// Group segments into lines by baseline proximity.
///
/// Segments are sorted by baseline_y descending (top-to-bottom reading order),
/// then by x ascending. Adjacent segments within baseline tolerance are grouped
/// into the same line.
pub(super) fn segments_to_lines(segments: Vec<SegmentData>) -> Vec<PdfLine> {
    if segments.is_empty() {
        return Vec::new();
    }

    // Sort segments by baseline_y DESCENDING (top-to-bottom), then x ascending.
    let mut sorted = segments;
    sorted.sort_by(|a, b| b.baseline_y.total_cmp(&a.baseline_y).then_with(|| a.x.total_cmp(&b.x)));

    let mut lines: Vec<PdfLine> = Vec::new();
    let first = sorted.remove(0);
    // Fix tolerance to the first segment's font size so it doesn't shrink
    // as smaller segments (subscripts, superscripts) are added to the line.
    let mut line_tolerance_fs = first.font_size.max(1.0);
    let mut current_segments: Vec<SegmentData> = vec![first];

    for seg in sorted {
        let current_baseline =
            current_segments.iter().map(|s| s.baseline_y).sum::<f32>() / current_segments.len() as f32;

        if (seg.baseline_y - current_baseline).abs() < BASELINE_Y_TOLERANCE_FRACTION * line_tolerance_fs {
            current_segments.push(seg);
        } else {
            lines.push(finalize_line(current_segments));
            // Reset tolerance to the new line's first segment
            line_tolerance_fs = seg.font_size.max(1.0);
            current_segments = vec![seg];
        }
    }

    if !current_segments.is_empty() {
        lines.push(finalize_line(current_segments));
    }

    lines
}

/// Build a PdfLine from a set of segments, sorting them left-to-right.
fn finalize_line(mut segments: Vec<SegmentData>) -> PdfLine {
    segments.sort_by(|a, b| a.x.total_cmp(&b.x));

    let baseline_y = segments.iter().map(|s| s.baseline_y).sum::<f32>() / segments.len() as f32;
    let dominant_font_size = most_frequent_font_size(segments.iter().map(|s| s.font_size));

    let bold_count = segments.iter().filter(|s| s.is_bold).count();
    let mono_count = segments.iter().filter(|s| s.is_monospace).count();
    let majority = segments.len().div_ceil(2);

    PdfLine {
        baseline_y,
        dominant_font_size,
        is_bold: bold_count >= majority,
        is_monospace: mono_count >= majority,
        segments,
    }
}

/// Compute the most frequent font size from an iterator, quantized to 0.5pt.
pub(super) fn most_frequent_font_size(sizes: impl Iterator<Item = f32>) -> f32 {
    let mut counts: Vec<(i32, usize)> = Vec::new();
    for fs in sizes {
        let key = (fs * 2.0).round() as i32;
        if let Some(entry) = counts.iter_mut().find(|(k, _)| *k == key) {
            entry.1 += 1;
        } else {
            counts.push((key, 1));
        }
    }
    if counts.is_empty() {
        return 0.0;
    }
    counts.sort_by_key(|b| std::cmp::Reverse(b.1));
    counts[0].0 as f32 / 2.0
}

/// Returns true if the character is a CJK ideograph, Hiragana, Katakana, or Hangul.
pub(super) fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x4E00..=0x9FFF     // CJK Unified Ideographs
        | 0x3040..=0x309F   // Hiragana
        | 0x30A0..=0x30FF   // Katakana
        | 0xAC00..=0xD7AF   // Hangul Syllables
        | 0x3400..=0x4DBF   // CJK Extension A
        | 0xF900..=0xFAFF   // CJK Compatibility Ideographs
        | 0x20000..=0x2A6DF // CJK Extension B
        | 0x2A700..=0x2B73F // CJK Extension C
        | 0x2B740..=0x2B81F // CJK Extension D
        | 0x2B820..=0x2CEAF // CJK Extension E
        | 0x2CEB0..=0x2EBEF // CJK Extension F
        | 0x30000..=0x3134F // CJK Extension G
        | 0x31350..=0x323AF // CJK Extension H
        | 0x2F800..=0x2FA1F // CJK Compatibility Ideographs Supplement
    )
}

/// Returns true if a space should be inserted between two adjacent text chunks.
/// CJK text should not have spaces between them.
pub(super) fn needs_space_between(prev: &str, next: &str) -> bool {
    let prev_ends_cjk = prev.chars().last().is_some_and(is_cjk_char);
    let next_starts_cjk = next.chars().next().is_some_and(is_cjk_char);
    !(prev_ends_cjk && next_starts_cjk)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_segment(
        text: &str,
        x: f32,
        baseline_y: f32,
        width: f32,
        font_size: f32,
        is_bold: bool,
        is_italic: bool,
    ) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: baseline_y,
            width,
            height: font_size,
            font_size,
            is_bold,
            is_italic,
            is_monospace: false,
            baseline_y,
        }
    }

    fn plain_segment(text: &str, x: f32, baseline_y: f32, width: f32, font_size: f32) -> SegmentData {
        make_segment(text, x, baseline_y, width, font_size, false, false)
    }

    #[test]
    fn test_segments_to_lines_single_line() {
        let segments = vec![
            plain_segment("Hello", 10.0, 700.0, 40.0, 12.0),
            plain_segment("world", 55.0, 700.0, 40.0, 12.0),
        ];
        let lines = segments_to_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].segments.len(), 2);
    }

    #[test]
    fn test_segments_to_lines_two_lines() {
        let segments = vec![
            plain_segment("Line1", 10.0, 700.0, 40.0, 12.0),
            plain_segment("Line2", 10.0, 680.0, 40.0, 12.0),
        ];
        let lines = segments_to_lines(segments);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_segments_to_lines_sorted_left_to_right() {
        let segments = vec![
            plain_segment("second", 100.0, 700.0, 50.0, 12.0),
            plain_segment("first", 10.0, 700.0, 40.0, 12.0),
        ];
        let lines = segments_to_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].segments[0].text, "first");
        assert_eq!(lines[0].segments[1].text, "second");
    }

    #[test]
    fn test_segments_to_lines_empty() {
        let lines = segments_to_lines(vec![]);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_segments_to_lines_bold_majority() {
        let segments = vec![
            make_segment("Bold", 10.0, 700.0, 40.0, 12.0, true, false),
            make_segment("Bold2", 55.0, 700.0, 40.0, 12.0, true, false),
            make_segment("Normal", 100.0, 700.0, 50.0, 12.0, false, false),
        ];
        let lines = segments_to_lines(segments);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_bold); // 2 of 3 are bold
    }

    #[test]
    fn test_is_cjk_char_basic() {
        assert!(is_cjk_char('\u{4E00}')); // CJK
        assert!(is_cjk_char('\u{3042}')); // Hiragana
        assert!(is_cjk_char('\u{30A2}')); // Katakana
        assert!(!is_cjk_char('A'));
        assert!(!is_cjk_char(' '));
    }

    #[test]
    fn test_needs_space_between() {
        assert!(needs_space_between("hello", "world"));
        assert!(!needs_space_between("\u{4E00}", "\u{4E01}"));
        assert!(needs_space_between("hello", "\u{4E00}"));
        assert!(needs_space_between("\u{4E00}", "hello"));
    }
}
