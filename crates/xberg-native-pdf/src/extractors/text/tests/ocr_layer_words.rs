//! An OCR text layer (render mode 3) shows each recognised word as its own string at the word's box
//! on the scan, with gaps far below a space glyph (Acrobat Paper Capture, scanned.pdf: 0.03 to
//! 0.18 em). Those strings are separate words; visible text with the same gaps keeps the merge's
//! kerning decision. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

const SIZE: f32 = 12.0;

/// Times-Roman with a `/Widths` array that copies the Adobe metrics.
fn widths_font() -> FontInfo {
    let bare = create_test_font();
    let mut font = create_test_font();
    font.widths = Some((32u32..=126).map(|code| bare.get_glyph_width(code as u16)).collect());
    font.first_char = Some(32);
    font.last_char = Some(126);
    font
}

/// One line of strings, each placed by its own `Tm`, `gap_em` after the end of the previous one.
fn positioned_line(render_mode: u8, words: &[&str], gap_em: f32) -> String {
    let font = widths_font();
    let mut x = 100.0;
    let mut stream = format!("BT /F1 1 Tf {render_mode} Tr ");
    for word in words {
        stream.push_str(&format!("{SIZE} 0 0 {SIZE} {x} 600 Tm ({word}) Tj "));
        let advance: f32 = word.bytes().map(|b| font.get_glyph_width(u16::from(b))).sum();
        x += advance / 1000.0 * SIZE + gap_em * SIZE;
    }
    stream.push_str("ET");
    stream
}

fn text(stream: &str) -> String {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), widths_font());
    let spans = extractor.extract_text_spans(stream.as_bytes()).unwrap();
    spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn ocr_layer_words_shown_apart_stay_apart() {
    for gap_em in [0.03, 0.1] {
        let invisible = text(&positioned_line(
            3,
            &["serving", "retirees", "EXECUTIVE", "COMMITTEE"],
            gap_em,
        ));
        assert_eq!(
            invisible
                .split(['|', ' '])
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>(),
            ["serving", "retirees", "EXECUTIVE", "COMMITTEE"],
            "gap {gap_em} em: {invisible:?}"
        );
    }
}

#[test]
fn visible_text_with_the_same_gaps_keeps_the_kerning_merge() {
    let visible = text(&positioned_line(0, &["serving", "retirees"], 0.03));
    assert_eq!(visible, "servingretirees");
}

#[test]
fn an_ocr_layer_that_shows_each_glyph_alone_keeps_its_words() {
    let glyphs = text(&positioned_line(3, &["w", "o", "r", "d"], 0.01));
    assert_eq!(glyphs, "word");
}

#[test]
fn an_ocr_layer_word_split_into_abutting_strings_stays_one_word() {
    for gap_em in [0.0, 0.005] {
        let abutting = text(&positioned_line(3, &["wor", "ld"], gap_em));
        assert_eq!(abutting, "world", "gap {gap_em} em");
    }
    let apart = text(&positioned_line(3, &["wor", "ld"], 0.03));
    assert_eq!(
        apart.split(['|', ' ']).filter(|w| !w.is_empty()).collect::<Vec<_>>(),
        ["wor", "ld"],
        "{apart:?}"
    );
}
