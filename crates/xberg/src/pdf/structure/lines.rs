//! Character utilities for text assembly: CJK detection and spacing logic.

use crate::pdf::hierarchy::SegmentData;

/// Minimum horizontal gap between two same-line segments, expressed as a fraction of
/// the trailing segment's font size, that indicates a genuine word space rather than a
/// kerning-run split of a single word. This matches xberg_native_pdf's main span-joining
/// convention. Zero and negative gaps remain joined, preserving kerning-run repair.
const SEGMENT_GAP_SPACE_RATIO: f32 = 0.15;

/// Maximum horizontal gap between two same-line, same-font-size segments, expressed as a
/// fraction of the larger segment's font size, that identifies two spans as one word split
/// across a font-resource change (xberg-io/xberg#1566) rather than a genuine inter-word
/// space. This guard can only ever JOIN two spans, never split them, so it cannot regress a
/// document that already renders correctly.
///
/// ~keep The value is chosen for HEADROOM ON BOTH SIDES, not to sit at either bound. The
/// reported defect measures 0.008 em, and `test_segments_need_space_style_change_inserts_space`
/// pins a gap the suite treats as a genuine word boundary at exactly 0.05 em. Setting this to
/// 0.05 would leave the outcome at that bound decided by f32 rounding of `font_size * ratio`:
/// at the test's 20 pt it rounds to exactly 1.0 and the strict `<` keeps the test passing, but
/// at 9 pt the same ratio rounds *above* 0.45 and the guard would fire on a gap the suite calls
/// a word boundary. A threshold that flips on font size is not a threshold. 0.025 em sits ~3x
/// above the observed defect and 2x below the suite's word-boundary bound at every font size.
const TOUCHING_SPAN_GAP_EM_RATIO: f32 = 0.025;

/// Maximum baseline difference, in points, for the "touching spans" guard in
/// [`segments_are_touching`]. Tighter than `segments_need_space`'s `eff_height`-scaled
/// same-line check (which must tolerate wrapped-line reflow noise), because this guard only
/// fires for spans painted on the exact same text line by adjacent PDF spans.
const TOUCHING_SPAN_BASELINE_TOLERANCE: f32 = 0.05;

/// Maximum font-size difference for the "touching spans" guard, expressed as a fraction of
/// the larger of the two font sizes.
const TOUCHING_SPAN_FONT_SIZE_TOLERANCE_RATIO: f32 = 0.01;

/// Maximum baseline difference, in points, for two runs to count as one visual line rather
/// than two lines split by a real line break.
///
/// ~keep Deliberately a second constant rather than an import of pipeline.rs's private
/// `INLINE_STYLE_BASELINE_TOLERANCE` (same value, 0.5pt): that const also drives four
/// unrelated inline-style-grouping call sites in pipeline.rs, and coupling this module to it
/// would let an unrelated change there silently retune dehyphenation. xberg-io/xberg#1581 was
/// caused by a THIRD, looser notion of "line break" living at the dehyphenation call sites in
/// assembly.rs (no geometry check at all); this gives assembly.rs the same test pipeline.rs's
/// `spans_visual_line_break` already applies, instead of a fourth definition.
const LINE_BREAK_BASELINE_TOLERANCE: f32 = 0.5;

/// True when `trailing` and `leading` sit on genuinely different visual lines rather than
/// being two runs split mid-line by a style-run or font-resource boundary.
///
/// Used to gate hyphen-joining in `assembly.rs`: a suspended hyphen inside one line
/// (`onderhouds- en`) must never be welded, while a hyphen that really trails a wrapped line
/// must still be.
pub(crate) fn crosses_visual_line_break(trailing: &SegmentData, leading: &SegmentData) -> bool {
    if !trailing.has_same_rotation(leading) {
        return false;
    }
    let trailing_baseline = trailing.upright_baseline();
    let leading_baseline = leading.upright_baseline();
    trailing_baseline.is_finite()
        && leading_baseline.is_finite()
        && (trailing_baseline - leading_baseline).abs() > LINE_BREAK_BASELINE_TOLERANCE
}

/// Returns true when `prev_seg`/`next_seg` are two halves of a single word split across a
/// mid-word font-resource change (xberg-io/xberg#1566): same rotation frame, same baseline,
/// same font size, a gap far below a genuine word space, and a word character immediately on
/// each side of the boundary. Checked before any bold/italic/monospace comparison because a
/// mid-word subset-font switch can flip those derived flags (see `SegmentData`) despite the
/// two spans being visually one continuous word.
pub(crate) fn segments_are_touching(
    prev_seg: &SegmentData,
    prev_word: &str,
    next_seg: &SegmentData,
    next_word: &str,
) -> bool {
    let (Some(prev_last_char), Some(next_first_char)) = (prev_word.chars().last(), next_word.chars().next()) else {
        return false;
    };
    if !prev_last_char.is_alphanumeric() || !next_first_char.is_alphanumeric() {
        return false;
    }

    // ~keep An explicitly drawn space at the boundary outranks any geometry. The producer
    // emitted a space glyph, and its advance is already inside `prev_seg`'s extent, so the
    // measured gap to the next segment collapses to ~0 and reads as "touching" — exactly the
    // shape this guard fires on. Words are whitespace-split, so `prev_word`/`next_word` never
    // carry that space and cannot reveal it; only the segment text can. Checking it here
    // rather than relying on `segments_need_space`'s own `has_explicit_boundary_space` is what
    // keeps `segments_to_words` (which calls this directly) from fusing `abc ` + `def` into
    // `abcdef`.
    if prev_seg.text.chars().last().is_some_and(char::is_whitespace)
        || next_seg.text.chars().next().is_some_and(char::is_whitespace)
    {
        return false;
    }

    if !prev_seg.has_same_rotation(next_seg) {
        return false;
    }

    let baseline_gap = (prev_seg.upright_baseline() - next_seg.upright_baseline()).abs();
    if baseline_gap > TOUCHING_SPAN_BASELINE_TOLERANCE {
        return false;
    }

    let max_font_size = prev_seg.font_size.max(next_seg.font_size).max(1.0);
    if (prev_seg.font_size - next_seg.font_size).abs() > max_font_size * TOUCHING_SPAN_FONT_SIZE_TOLERANCE_RATIO {
        return false;
    }

    let (_, prev_end) = prev_seg.upright_advance_extent();
    let (next_start, _) = next_seg.upright_advance_extent();
    (next_start - prev_end).abs() < max_font_size * TOUCHING_SPAN_GAP_EM_RATIO
}

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

/// A raised number at the start of a paragraph can label a note definition.
pub(super) fn raised_numeric_prefix_has_word_gap(marker: &SegmentData, body: &SegmentData) -> bool {
    const MAX_MARKER_DIGITS: usize = 3;
    let number = marker.text.trim();
    let (_, marker_end) = marker.upright_advance_extent();
    let (body_start, _) = body.upright_advance_extent();
    marker.font_size < body.font_size
        && marker.upright_baseline() > body.upright_baseline()
        && (1..=MAX_MARKER_DIGITS).contains(&number.len())
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && body.text.trim_start().starts_with(char::is_alphabetic)
        && body.text.split_whitespace().nth(1).is_some()
        && body_start - marker_end > body.font_size * SEGMENT_GAP_SPACE_RATIO
}

/// Returns true if a space should be inserted between the last word of `prev_seg`
/// and the first word of `next_seg`, using segment geometry to distinguish a real
/// word gap from a kerning-run split of one word across two spans.
///
/// xberg_native_pdf sometimes splits a single word into multiple text spans at kerning-run
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

    if segments_are_touching(prev_seg, prev_word, next_seg, next_word) {
        return false;
    }

    let has_explicit_boundary_space = prev_seg.text.chars().last().is_some_and(char::is_whitespace)
        || next_seg.text.chars().next().is_some_and(char::is_whitespace);
    if has_explicit_boundary_space {
        return true;
    }

    if !prev_seg.has_same_rotation(next_seg) {
        return true;
    }

    if prev_seg.is_bold != next_seg.is_bold
        || prev_seg.is_italic != next_seg.is_italic
        || prev_seg.is_monospace != next_seg.is_monospace
    {
        return true;
    }

    let eff_height = next_seg.height.max(prev_seg.height).max(next_seg.font_size * 0.5);
    let same_line = (prev_seg.upright_baseline() - next_seg.upright_baseline()).abs() < eff_height * 0.5;
    if !same_line {
        return true;
    }

    let (prev_start, prev_end) = prev_seg.upright_advance_extent();
    let (next_start, _) = next_seg.upright_advance_extent();
    let advance_gap = next_start - prev_end;

    // A fragment that jumps substantially backwards on the advance axis is a
    // reordered cell/field rather than a kerning-run continuation. Preserve a
    // word boundary so a bad upstream permutation cannot fuse identifiers
    // such as `700004` + `2` into `7000042`.
    // Leftward advance is normal for RTL text, so keep that case on the
    // existing bidi path. ~keep
    let font_size = prev_seg.font_size.max(next_seg.font_size).max(1.0);
    let has_rtl_text = prev_word
        .chars()
        .chain(next_word.chars())
        .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32));
    let severe_backtrack = next_start <= prev_start + 0.5 && advance_gap < -font_size;
    if severe_backtrack && !has_rtl_text {
        return true;
    }

    advance_gap > next_seg.font_size * SEGMENT_GAP_SPACE_RATIO
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
            rotation_degrees: 0.0,
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
    fn test_segments_need_space_explicit_whitespace_overrides_negative_overlap() {
        let prev = segment("infos. ", 100.0, 35.0, 10.0, 700.0);
        let next = segment("MongoKit", 133.5, 40.0, 10.0, 700.0);
        assert!(segments_need_space(&prev, "infos.", &next, "MongoKit"));
    }

    #[test]
    fn test_segments_need_space_negative_overlap_without_whitespace_stays_joined() {
        let prev = segment("Mongo", 100.0, 30.0, 10.0, 700.0);
        let next = segment("Kit", 129.0, 15.0, 10.0, 700.0);
        assert!(!segments_need_space(&prev, "Mongo", &next, "Kit"));
    }

    #[test]
    fn test_segments_need_space_distinct_words_insert_space() {
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
    fn issue_1560_reordered_numeric_fragments_cannot_fuse() {
        let prev = segment("700004", 68.279, 35.0, 8.6, 623.481);
        let next = segment("2", 50.0, 5.0, 8.6, 622.401);
        let same_baseline_next = segment("2", 50.0, 5.0, 8.6, 623.481);

        assert!(segments_need_space(&prev, "700004", &next, "2"));
        assert!(segments_need_space(&prev, "700004", &same_baseline_next, "2"));
    }

    #[test]
    fn issue_1560_rtl_backtracking_keeps_existing_join_behavior() {
        let prev = segment("אבג", 68.279, 35.0, 8.6, 623.481);
        let next = segment("ד", 50.0, 5.0, 8.6, 622.401);

        assert!(!segments_need_space(&prev, "אבג", &next, "ד"));
    }

    #[test]
    fn test_segments_need_space_different_line_always_spaces() {
        let prev = segment("end", 500.0, 20.0, 10.0, 700.0);
        let next = segment("start", 40.0, 30.0, 10.0, 685.0);
        assert!(segments_need_space(&prev, "end", &next, "start"));
    }

    #[test]
    fn test_segments_need_space_cjk_adjacent_never_spaces() {
        let prev = segment("\u{4E00} ", 100.0, 12.0, 12.0, 700.0);
        let next = segment("\u{4E01}", 112.0, 12.0, 12.0, 700.0);
        assert!(!segments_need_space(&prev, "\u{4E00}", &next, "\u{4E01}"));
    }

    #[test]
    fn test_segments_need_space_uses_rotated_advance_axis() {
        let mut prev = segment("Engine", 100.0, 20.0, 10.0, 100.0);
        prev.rotation_degrees = 90.0;
        let mut next = segment("oil", 100.0, 10.0, 10.0, 125.0);
        next.rotation_degrees = 90.0;

        assert!(segments_need_space(&prev, "Engine", &next, "oil"));
    }

    #[test]
    fn test_segments_need_space_separates_different_rotation_frames() {
        let prev = segment("body", 100.0, 20.0, 10.0, 100.0);
        let mut next = segment("footer", 120.0, 20.0, 10.0, 100.0);
        next.rotation_degrees = 90.0;

        assert!(segments_need_space(&prev, "body", &next, "footer"));
    }

    #[test]
    fn issue_1566_touching_spans_with_differing_style_stay_joined() {
        let mut prev = segment("2 per ketel, pri", 287.864, 46.631, 8.999996, 331.641);
        let mut next = segment("js per meter", 334.564, 41.161, 8.999996, 331.641);
        prev.is_bold = false;
        next.is_bold = true;

        assert!(!segments_need_space(&prev, "pri", &next, "js"));
    }

    #[test]
    fn issue_1566_guard_never_swallows_an_explicitly_drawn_space() {
        // The space glyph's advance is inside `prev`'s width, so the segments measure as
        // touching -- but the producer drew that space and it must survive.
        let prev = segment("alpha ", 10.0, 30.0, 9.0, 100.0);
        let mut next = segment("beta", 40.0, 20.0, 9.0, 100.0);
        next.is_bold = true;

        assert!(!segments_are_touching(&prev, "alpha", &next, "beta"));
        assert!(segments_need_space(&prev, "alpha", &next, "beta"));
    }

    #[test]
    fn issue_1566_guard_stays_below_the_suites_word_boundary_gap_at_every_font_size() {
        // `test_segments_need_space_style_change_inserts_space` treats 0.05 em as a genuine
        // word boundary. A guard sitting at that bound would flip on f32 rounding of
        // `font_size * ratio`; assert the margin holds at a small font size too.
        for &font_size in &[9.0_f32, 20.0] {
            let gap = font_size * 0.05;
            let prev = segment("plain", 10.0, 20.0, font_size, 100.0);
            let mut next = segment("bold", 10.0 + 20.0 + gap, 20.0, font_size, 100.0);
            next.is_bold = true;

            assert!(
                !segments_are_touching(&prev, "plain", &next, "bold"),
                "a 0.05 em gap must not read as touching at font_size {font_size}"
            );
            assert!(segments_need_space(&prev, "plain", &next, "bold"));
        }
    }
}
