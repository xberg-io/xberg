//! Character utilities for text assembly: CJK detection and spacing logic.

use crate::pdf::hierarchy::SegmentData;

/// Minimum horizontal gap between two same-line segments, expressed as a fraction of
/// the trailing segment's font size, that indicates a genuine word space rather than a
/// kerning-run split of a single word. This matches pdf_oxide's main span-joining
/// convention. Zero and negative gaps remain joined, preserving kerning-run repair.
const SEGMENT_GAP_SPACE_RATIO: f32 = 0.15;

/// Returns true if the character is a CJK ideograph, Hiragana, Katakana, or Hangul.
pub(super) fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x4E00..=0x9FFF
        | 0x3040..=0x309F
        | 0x30A0..=0x30FF
        | 0xAC00..=0xD7AF
        | 0x3400..=0x4DBF
        | 0xF900..=0xFAFF
        | 0x20000..=0x2A6DF
        | 0x2A700..=0x2B73F
        | 0x2B740..=0x2B81F
        | 0x2B820..=0x2CEAF
        | 0x2CEB0..=0x2EBEF
        | 0x30000..=0x3134F
        | 0x31350..=0x323AF
        | 0x2F800..=0x2FA1F
    )
}

/// Returns true if a space should be inserted between two adjacent text chunks.
/// CJK text should not have spaces between them.
pub(super) fn needs_space_between(prev: &str, next: &str) -> bool {
    let prev_ends_cjk = prev.chars().last().is_some_and(is_cjk_char);
    let next_starts_cjk = next.chars().next().is_some_and(is_cjk_char);
    !(prev_ends_cjk && next_starts_cjk)
}

/// Returns true if a space should be inserted between the last word of `prev_seg`
/// and the first word of `next_seg`, using segment geometry to distinguish a real
/// word gap from a kerning-run split of one word across two spans.
///
/// pdf_oxide sometimes splits a single word into multiple text spans at kerning-run
/// boundaries (e.g. "elit" -> "eli" + "t"). Those spans are visually adjacent (or
/// overlapping) on the same baseline, unlike spans separated by an actual space
/// character. When the two segments sit on different lines (a wrapped-line reflow),
/// geometry is not meaningful and a space is always inserted, matching prior behavior.
pub(super) fn segments_need_space(
    prev_seg: &SegmentData,
    prev_word: &str,
    next_seg: &SegmentData,
    next_word: &str,
) -> bool {
    if !needs_space_between(prev_word, next_word) {
        return false;
    }

    // A kerning-run split never changes style mid-word: pdf_oxide only fragments a
    // single styled run. When the two segments differ in weight/slant/pitch this is a
    // real word boundary (e.g. an inline bold or italic run), not a span split, so a
    // space is required regardless of how tight the horizontal gap is.
    if prev_seg.is_bold != next_seg.is_bold
        || prev_seg.is_italic != next_seg.is_italic
        || prev_seg.is_monospace != next_seg.is_monospace
    {
        return true;
    }

    let eff_height = next_seg.height.max(prev_seg.height).max(next_seg.font_size * 0.5);
    let same_line = (prev_seg.baseline_y - next_seg.baseline_y).abs() < eff_height * 0.5;
    if !same_line {
        return true;
    }

    let prev_end_x = prev_seg.x + prev_seg.width;
    let x_gap = next_seg.x - prev_end_x;
    x_gap > next_seg.font_size * SEGMENT_GAP_SPACE_RATIO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cjk_char_basic() {
        assert!(is_cjk_char('\u{4E00}'));
        assert!(is_cjk_char('\u{3042}'));
        assert!(is_cjk_char('\u{30A2}'));
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

    fn segment(text: &str, x: f32, width: f32, font_size: f32, baseline_y: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: baseline_y,
            width,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
            assigned_role: None,
        }
    }

    #[test]
    fn test_segments_need_space_kerning_split_stays_joined() {
        let prev = segment("eli", 100.0, 15.0, 10.0, 700.0);
        let next = segment("t", 115.0, 5.0, 10.0, 700.0);
        assert!(!segments_need_space(&prev, "eli", &next, "t"));
    }

    #[test]
    fn test_segments_need_space_distinct_words_insert_space() {
        // Two distinct words on one baseline with a real word space: "office" then "is".
        // The 10pt gap clears the configured ratio, so a space is inserted.
        let prev = segment("office", 100.0, 30.0, 10.0, 700.0);
        let next = segment("is", 140.0, 8.0, 10.0, 700.0);
        assert!(segments_need_space(&prev, "office", &next, "is"));
    }

    #[test]
    fn test_segments_need_space_two_point_word_gap_inserts_space() {
        let prev = segment("MongoKit", 100.0, 40.0, 10.0, 700.0);
        let next = segment("is", 142.0, 8.0, 10.0, 700.0);
        assert!(segments_need_space(&prev, "MongoKit", &next, "is"));
    }

    #[test]
    fn test_segments_need_space_style_change_inserts_space() {
        // Distinct words across an inline style change (plain -> bold) can abut with a
        // tiny gap (here x_gap = 1pt, well under the configured threshold), but a style change is a
        // real word boundary, not a kerning split, so a space must be inserted. Guards
        // the inline-bold-run regression.
        let prev = segment("plain", 10.0, 20.0, 20.0, 100.0);
        let next = {
            let mut s = segment("bold", 31.0, 20.0, 20.0, 100.0);
            s.is_bold = true;
            s
        };
        assert!(segments_need_space(&prev, "plain", &next, "bold"));
    }

    #[test]
    fn test_segments_need_space_tower_kerning_split_joins() {
        // "Tower" split by pdf_oxide into "T" + "ower" at a kerning boundary. The "ower"
        // span starts fractionally before the "T" span ends (x_gap ~= -1pt), under the
        // configured positive-gap threshold, so they rejoin into "Tower" with NO space.
        // This is the exact case that previously produced "T ower" (issue #1291).
        let prev = segment("T", 100.0, 7.0, 10.0, 700.0);
        let next = segment("ower", 106.0, 22.0, 10.0, 700.0);
        assert!(!segments_need_space(&prev, "T", &next, "ower"));
    }

    #[test]
    fn test_segments_need_space_positive_kerning_gap_stays_joined() {
        let prev = segment("T", 100.0, 7.0, 10.0, 700.0);
        let below_threshold = segment("ower", 108.0, 22.0, 10.0, 700.0);
        let at_threshold = segment("ower", 108.5, 22.0, 10.0, 700.0);
        let above_threshold = segment("ower", 108.6, 22.0, 10.0, 700.0);

        assert!(!segments_need_space(&prev, "T", &below_threshold, "ower"));
        assert!(!segments_need_space(&prev, "T", &at_threshold, "ower"));
        assert!(segments_need_space(&prev, "T", &above_threshold, "ower"));
    }

    #[test]
    fn test_segments_need_space_different_line_always_spaces() {
        let prev = segment("end", 500.0, 20.0, 10.0, 700.0);
        let next = segment("start", 40.0, 30.0, 10.0, 685.0);
        assert!(segments_need_space(&prev, "end", &next, "start"));
    }

    #[test]
    fn test_segments_need_space_cjk_adjacent_never_spaces() {
        let prev = segment("\u{4E00}", 100.0, 12.0, 12.0, 700.0);
        let next = segment("\u{4E01}", 112.0, 12.0, 12.0, 700.0);
        assert!(!segments_need_space(&prev, "\u{4E00}", &next, "\u{4E01}"));
    }
}
