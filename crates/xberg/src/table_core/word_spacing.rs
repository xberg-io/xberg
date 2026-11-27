//! Separator between two recognized words on one line.
//!
//! Chinese and Japanese are written without spaces between words, yet Tesseract's hOCR and TSV split
//! a CJK line into one word per character or short run. Joining those words with a space turns
//! `安徽省建设用地` into `安徽 省 建设 用 地`. Hangul is excluded: Korean separates words with spaces.

/// Gap, in multiples of the taller word's height, from which two adjacent CJK words keep a space.
/// Tesseract chi_sim words inside a run sit at most 0.65 of a glyph height apart on clean 300 dpi
/// pages; a column or field gap measures two heights or more. ~keep
const CJK_GAP_SPACE_RATIO: f64 = 1.0;

/// Whether `character` belongs to a script written without spaces between words: Han ideographs,
/// kana, Bopomofo, CJK punctuation and the full-width forms used beside them.
pub(crate) fn is_unspaced_script_char(character: char) -> bool {
    matches!(
        character as u32,
        0x2E80..=0x2FDF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3100..=0x312F
            | 0x31A0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE1F
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x323AF
    )
}

/// Whether two words read on one line, `previous` then `next`, are joined without a space.
///
/// `gap` is the horizontal distance from `previous`'s right edge to `next`'s left edge, and `height`
/// the taller of the two words, both in pixels; `None` when either word has no geometry. Two CJK
/// words join unless they sit a glyph height or more apart; a space next to Latin, digits or any
/// other script is kept.
pub(crate) fn joins_without_space(previous: &str, next: &str, gap: Option<(f64, f64)>) -> bool {
    previous
        .trim_end()
        .chars()
        .next_back()
        .is_some_and(is_unspaced_script_char)
        && next.trim_start().chars().next().is_some_and(is_unspaced_script_char)
        && gap.is_none_or(|(gap, height)| height <= 0.0 || gap < height * CJK_GAP_SPACE_RATIO)
}

/// Gap and taller height of two axis-aligned word boxes given as `(left, top, width, height)`.
pub(crate) fn box_gap(previous: (u32, u32, u32, u32), next: (u32, u32, u32, u32)) -> (f64, f64) {
    let previous_right = f64::from(previous.0) + f64::from(previous.2);
    (f64::from(next.0) - previous_right, f64::from(previous.3.max(next.3)))
}

/// [`joins_without_space`] for two Tesseract words on one line, measured from their boxes.
pub(crate) fn hocr_words_join_without_space(previous: &super::HocrWord, next: &super::HocrWord) -> bool {
    let gap = box_gap(
        (previous.left, previous.top, previous.width, previous.height),
        (next.left, next.top, next.width, next.height),
    );
    joins_without_space(&previous.text, &next.text, Some(gap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_cjk_words_join_without_a_space() {
        let cases = [
            ("安徽", "省", true),
            ("建设", "用", true),
            ("同时", "，", true),
            ("（", "公", true),
            ("停车", "区", true),
            ("ひら", "カタ", true),
            ("〈2020", "年", false),
            ("年", "2020", false),
            ("表", "7.5", false),
            ("Q_", "(peu/d", false),
            ("Hello", "World", false),
            ("한국어", "문장", false),
            ("中文", "Latin", false),
        ];
        for (previous, next, expected) in cases {
            assert_eq!(
                joins_without_space(previous, next, None),
                expected,
                "{previous:?} then {next:?}"
            );
        }
    }

    #[test]
    fn cjk_words_a_glyph_height_apart_keep_their_space() {
        assert!(joins_without_space("公", "路", Some((30.0, 46.0))));
        assert!(!joins_without_space("公路", "六", Some((46.0, 46.0))));
        assert!(!joins_without_space("基准值", "编制", Some((447.0, 41.0))));
        assert!(joins_without_space("车", "比例", Some((-17.0, 57.0))));
        assert!(joins_without_space("车", "道", Some((5.0, 0.0))));
    }

    #[test]
    fn box_gap_measures_from_the_right_edge_to_the_next_left_edge() {
        assert_eq!(box_gap((100, 10, 40, 30), (150, 12, 20, 45)), (10.0, 45.0));
        assert_eq!(box_gap((100, 10, 40, 30), (120, 12, 20, 25)), (-20.0, 30.0));
    }
}
