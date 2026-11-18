//! Character sets used by native PDF spacing and reading-order heuristics.

fn is_basic_kana_or_han(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF)
}

/// Includes Han compatibility ideographs and halfwidth katakana.
fn is_kana_or_han_with_compatibility(c: char) -> bool {
    is_basic_kana_or_han(c) || matches!(c as u32, 0xF900..=0xFAFF | 0xFF66..=0xFF9F)
}

// The spacing and prose rules use the whole Unicode block through U+D7AF. ~keep
fn is_hangul_syllable_block(c: char) -> bool {
    matches!(c as u32, 0xAC00..=0xD7AF)
}

fn is_table_spacing_cjk(c: char) -> bool {
    is_basic_kana_or_han(c) || is_hangul_syllable_block(c) || matches!(c as u32, 0x20000..=0x2A6DF)
}

fn is_table_spacing_operator(c: char) -> bool {
    matches!(
        c as u32,
        0xFF0B | 0xFF0D | 0xFF1A | 0xFF1B | 0xFF1C..=0xFF1E
            | 0x2260 | 0x2248 | 0x2264..=0x2265 | 0x00B5 | 0x03BC
            | 0x00B1 | 0x00D7 | 0x00F7
    )
}

/// Matches pdftotext by keeping ideographs and adjacent fullwidth or math operators together.
pub(crate) fn suppress_table_cell_space(left: char, right: char) -> bool {
    let left_cjk = is_table_spacing_cjk(left);
    let right_cjk = is_table_spacing_cjk(right);
    (left_cjk || is_table_spacing_operator(left))
        && (right_cjk || is_table_spacing_operator(right))
        && (left_cjk || right_cjk)
}

pub(crate) fn is_vertical_text_cjk(c: char) -> bool {
    is_kana_or_han_with_compatibility(c)
}

pub(crate) fn is_spatial_prose_cjk(c: char) -> bool {
    is_kana_or_han_with_compatibility(c) || is_hangul_syllable_block(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_pdf_cjk_sets_keep_their_original_boundaries() {
        let cases = [
            (0x303F, [false, false, false]),
            (0x3040, [true, true, true]),
            (0x30FF, [true, true, true]),
            (0x3100, [false, false, false]),
            (0x3400, [true, true, true]),
            (0x4DBF, [true, true, true]),
            (0x4DC0, [false, false, false]),
            (0x4E00, [true, true, true]),
            (0x9FFF, [true, true, true]),
            (0xAC00, [true, false, true]),
            (0xD7AF, [true, false, true]),
            (0xD7B0, [false, false, false]),
            (0xF900, [false, true, true]),
            (0xFAFF, [false, true, true]),
            (0xFB00, [false, false, false]),
            (0xFF65, [false, false, false]),
            (0xFF66, [false, true, true]),
            (0xFF9F, [false, true, true]),
            (0xFFA0, [false, false, false]),
            (0x20000, [true, false, false]),
            (0x2A6DF, [true, false, false]),
            (0x2A6E0, [false, false, false]),
        ];
        for (code, expected) in cases {
            let c = char::from_u32(code).expect("Unicode scalar");
            let actual = [
                is_table_spacing_cjk(c),
                is_vertical_text_cjk(c),
                is_spatial_prose_cjk(c),
            ];
            assert_eq!(actual, expected, "U+{code:04X}");
        }
    }

    #[test]
    fn table_spacing_operators_require_a_cjk_neighbor() {
        assert!(suppress_table_cell_space('漢', '字'));
        for operator in ['＋', '＜', '≤', 'μ', '±'] {
            assert!(suppress_table_cell_space('漢', operator));
            assert!(suppress_table_cell_space(operator, '字'));
            assert!(!suppress_table_cell_space(operator, operator));
            assert!(!suppress_table_cell_space(operator, 'A'));
        }
        assert!(!suppress_table_cell_space('漢', '！'));
        assert!(!suppress_table_cell_space('漢', 'A'));
    }
}
