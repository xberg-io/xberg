//! Ligature glyphs at the glyph stage (GH#2322): a space glyph that takes no room, a ToUnicode value of
//! U+0000 for a named glyph, and a ligature presentation form that must come out as letters. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use crate::fonts::Encoding;
use std::collections::HashMap;

const LIGATURE_CODE: u32 = 0x80;

/// A ToUnicode map in which only the ligature code is mapped, to `target` (hex code points, no separators).
fn ligature_cmap(target: &str) -> crate::fonts::LazyCMap {
    let body = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def /CMapType 2 def\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n1 beginbfchar\n<80> <{target}>\nendbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end\n"
    );
    crate::fonts::LazyCMap::new(body.into_bytes())
}

/// A simple font: every glyph is half an em wide, the space a quarter em, the ligature code one em.
fn ligature_font(target: &str, glyph_name: Option<&str>) -> FontInfo {
    let mut font = create_test_font();
    font.base_font = "Helvetica".to_string();
    font.widths = Some(
        (32u32..=0x84)
            .map(|code| match code {
                32 => 250.0,
                LIGATURE_CODE => 1000.0,
                _ => 500.0,
            })
            .collect(),
    );
    font.first_char = Some(32);
    font.last_char = Some(0x84);
    font.to_unicode = Some(ligature_cmap(target));
    if let Some(name) = glyph_name {
        font.diff_glyph_names.insert(LIGATURE_CODE as u8, name.to_string());
    }
    font
}

fn extractor_with(font: FontInfo) -> TextExtractor<'static> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    extractor
}

fn span_text(extractor: &mut TextExtractor, stream: &[u8]) -> String {
    let spans = extractor.extract_text_spans(stream).unwrap();
    assert!(!spans.is_empty(), "the stream produced no spans");
    spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("")
}

fn char_text(extractor: &mut TextExtractor, stream: &[u8]) -> String {
    let chars = extractor.extract(stream).unwrap();
    assert!(!chars.is_empty(), "the stream produced no characters");
    chars.iter().map(|c| c.char).collect()
}

/// The producer pulls the pen back by the width of the space (250 units), draws the space inside the
/// ligature box, and the next letter starts at the ligature's right edge.
const OVERLAP_SPACE: &[u8] = b"BT /F1 10 Tf 72 700 Td [(de\\200) 250 ( ne)] TJ ET";

#[test]
fn space_glyph_inside_the_ligature_box_is_not_a_word_break_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut extractor, OVERLAP_SPACE), "define");
}

#[test]
fn space_glyph_inside_the_ligature_box_is_not_a_word_break_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(char_text(&mut extractor, OVERLAP_SPACE), "define");
}

#[test]
fn space_glyph_with_its_own_room_after_a_ligature_stays_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(of \\200ne if)] TJ ET"),
        "of fine if"
    );
}

#[test]
fn space_glyph_with_its_own_room_after_a_ligature_stays_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        char_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(of \\200ne if)] TJ ET"),
        "of fine if"
    );
}

/// A producer may pull the pen back before a space to place a word gap. Only a ligature has a box
/// that a space can start inside of, so a glyph of one letter keeps its space.
const PULLED_BACK_SPACE: &[u8] = b"BT /F1 10 Tf 72 700 Td [(ab) 250 ( cd)] TJ ET";

#[test]
fn space_after_a_pulled_back_glyph_that_is_not_a_ligature_stays_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut extractor, PULLED_BACK_SPACE), "ab cd");
}

#[test]
fn space_after_a_pulled_back_glyph_that_is_not_a_ligature_stays_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(char_text(&mut extractor, PULLED_BACK_SPACE), "ab cd");
}

/// A kern before a space of 2.5 units. 120 units (48%) and 150 units (60%) leave part of the space
/// between the words, so it stays; 240 units (96%) puts the space inside the ligature box.
fn kern_stream(kern: u32) -> Vec<u8> {
    format!("BT /F1 10 Tf 72 700 Td [(de\\200) {kern} ( ne)] TJ ET").into_bytes()
}

#[test]
fn partial_pullback_keeps_the_space_in_spans_and_chars() {
    for kern in [120, 150] {
        let mut spans = extractor_with(ligature_font("00660069", None));
        let mut chars = extractor_with(ligature_font("00660069", None));
        assert_eq!(
            span_text(&mut spans, &kern_stream(kern)),
            "defi ne",
            "spans, kern {kern}"
        );
        assert_eq!(
            char_text(&mut chars, &kern_stream(kern)),
            "defi ne",
            "chars, kern {kern}"
        );
    }
}

#[test]
fn near_full_pullback_drops_the_space_in_spans_and_chars() {
    let mut spans = extractor_with(ligature_font("00660069", None));
    let mut chars = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut spans, &kern_stream(240)), "define");
    assert_eq!(char_text(&mut chars, &kern_stream(240)), "define");
}

/// A pull-back that belongs to a glyph before the ligature does not reach a space after the ligature.
#[test]
fn pullback_before_a_ligature_does_not_reach_the_space_after_it() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(a) 250 (\\200 ne)] TJ ET"),
        "afi ne"
    );
}

/// The absorbed space gives its width to the glyph before it: the six letters of "define" are 5 units
/// each in `char_widths` (the ligature 10, pulled back by 2.5, plus the space 2.5), and the text matrix
/// ends where the glyphs end (72 + 30).
#[test]
fn absorbed_space_width_stays_in_the_glyph_widths_and_the_matrix() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    let spans = extractor
        .extract_text_spans(b"BT /F1 10 Tf 72 700 Td [(de\\200) 250 ( ne)] TJ")
        .unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "define");
    let widths = &spans[0].char_widths;
    assert_eq!(widths.len(), 6);
    for w in widths {
        assert!((w - 5.0).abs() < 0.01, "{widths:?}");
    }
    let end = extractor.state_stack.current().text_matrix.e;
    assert!((end - 102.0).abs() < 0.01, "{end}");
}

/// The char path looks at the baseline: a space on another line is no word break to drop, however its x
/// compares with the ligature before it.
#[test]
fn space_on_another_baseline_stays_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        char_text(
            &mut extractor,
            b"BT /F1 10 Tf 72 700 Td (de\\200) Tj 1 0 0 1 89.5 690 Tm ( ne) Tj ET"
        ),
        "defi ne"
    );
}

/// Text turned by 180 degrees runs towards smaller x, which reads as an overlap with the previous glyph
/// unless the rotation is checked. The characters come back in reading order of the page, so the test
/// counts them: seven characters, one of them the space.
#[test]
fn space_in_rotated_text_stays_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    let text = char_text(&mut extractor, b"BT /F1 10 Tf -1 0 0 -1 300 700 Tm (de\\200 ne) Tj ET");
    assert_eq!(text.chars().count(), 7, "{text:?}");
    assert_eq!(text.chars().filter(|c| *c == ' ').count(), 1, "{text:?}");
}

/// The rotation of the glyph before the space and the rotation of the space itself are each checked: a
/// ligature turned by 180 degrees and an upright space that lies over it keep the space, and so does an
/// upright ligature with a turned space over it. Three characters each: two letters and the space.
#[test]
fn space_next_to_a_turned_ligature_stays_in_chars() {
    let mut turned_first = extractor_with(ligature_font("00660069", None));
    let text = char_text(
        &mut turned_first,
        b"BT /F1 10 Tf -1 0 0 -1 300 700 Tm (\\200) Tj 1 0 0 1 297 700 Tm ( ) Tj ET",
    );
    assert_eq!(text.chars().count(), 3, "{text:?}");
    let mut turned_second = extractor_with(ligature_font("00660069", None));
    let text = char_text(
        &mut turned_second,
        b"BT /F1 10 Tf 1 0 0 1 280 700 Tm (\\200) Tj -1 0 0 -1 281 700 Tm ( ) Tj ET",
    );
    assert_eq!(text.chars().count(), 3, "{text:?}");
}

/// A Type0 font: two-byte codes, 1 = d, 2 = e, 3 = the ligature (one em), 4 = n, 0x20 = space (a quarter em).
fn type0_ligature_font(ligature_target: &str) -> FontInfo {
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Identity;
    font.cid_widths = Some(HashMap::from([
        (1, 500.0),
        (2, 500.0),
        (3, 1000.0),
        (4, 500.0),
        (0x20, 250.0),
    ]));
    let body = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def /CMapType 2 def\n\
         1 begincodespacerange <0000> <FFFF> endcodespacerange\n5 beginbfchar\n<0001> <0064>\n<0002> <0065>\n\
         <0003> <{ligature_target}>\n<0004> <006E>\n<0020> <0020>\nendbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end\n"
    );
    font.to_unicode = Some(crate::fonts::LazyCMap::new(body.into_bytes()));
    font
}

const TYPE0_OVERLAP_SPACE: &[u8] = b"BT /F1 10 Tf 72 700 Td [<000100020003> 250 <0020 0004 0002>] TJ ET";

/// The two paths read one string the same way for a Type0 font: the space that takes no room is gone
/// from the spans and from the characters.
#[test]
fn type0_space_glyph_inside_the_ligature_box_drops_in_spans_and_chars() {
    let mut spans = extractor_with(type0_ligature_font("00660069"));
    let mut chars = extractor_with(type0_ligature_font("00660069"));
    assert_eq!(span_text(&mut spans, TYPE0_OVERLAP_SPACE), "define");
    assert_eq!(char_text(&mut chars, TYPE0_OVERLAP_SPACE), "define");
}

#[test]
fn type0_space_after_a_glyph_of_one_letter_stays_in_spans_and_chars() {
    let stream: &[u8] = b"BT /F1 10 Tf 72 700 Td [<00010002> 250 <0020 0004 0002>] TJ ET";
    let mut spans = extractor_with(type0_ligature_font("00660069"));
    let mut chars = extractor_with(type0_ligature_font("00660069"));
    assert_eq!(span_text(&mut spans, stream), "de ne");
    assert_eq!(char_text(&mut chars, stream), "de ne");
}

/// A Type0 code that yields two letters has two widths, so the widths stay as long as the text.
#[test]
fn type0_ligature_code_has_one_width_for_each_letter() {
    let font = type0_ligature_font("FB01");
    let letters = font.char_to_unicode(3);
    let mut extractor = extractor_with(font);
    let spans = extractor
        .extract_text_spans(b"BT /F1 10 Tf 72 700 Td <000100020003> Tj ET")
        .unwrap();
    assert_eq!(spans[0].text, "defi");
    assert_eq!(spans[0].char_widths.len(), 4, "{letters:?} {:?}", spans[0].char_widths);
    assert!(
        (spans[0].char_widths[3] - 5.0).abs() < 0.01,
        "{:?}",
        spans[0].char_widths
    );
}

#[test]
fn type0_ligature_code_in_a_string_array_has_one_width_for_each_letter() {
    let mut extractor = extractor_with(type0_ligature_font("FB01"));
    let spans = extractor
        .extract_text_spans(b"BT /F1 10 Tf 72 700 Td [<000100020003>] TJ ET")
        .unwrap();
    assert_eq!(spans[0].text, "defi");
    assert_eq!(spans[0].char_widths.len(), 4, "{:?}", spans[0].char_widths);
    assert!(
        (spans[0].char_widths[3] - 5.0).abs() < 0.01,
        "{:?}",
        spans[0].char_widths
    );
}

/// The glyph-name fallback for a ToUnicode value of U+0000 is for simple fonts. A Type0 code keeps the
/// replacement character even when a name exists for its low byte.
#[test]
fn type0_to_unicode_zero_ignores_the_glyph_name() {
    let mut font = type0_ligature_font("0000");
    font.diff_glyph_names.insert(3, "fi".to_string());
    assert_eq!(font.char_to_unicode(3).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn to_unicode_zero_is_a_miss_and_the_glyph_name_decides() {
    let font = ligature_font("0000", Some("fi"));
    assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some("fi"));
}

/// The name that the embedded program gives to the code taken as a glyph index is not the font's own
/// name for the code: it must not turn a broken mapping into a letter.
#[test]
fn to_unicode_zero_ignores_the_embedded_program_name() {
    let font = ligature_font("0000", None);
    let mut names = vec![None; 0x81];
    names[LIGATURE_CODE as usize] = Some("fi".to_string());
    font.embedded_glyph_names.set(Some(names)).unwrap();
    assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn to_unicode_zero_without_a_glyph_name_stays_unmapped() {
    let font = ligature_font("0000", None);
    assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn to_unicode_zero_ligature_comes_out_as_letters_in_spans() {
    let mut extractor = extractor_with(ligature_font("0000", Some("ffi")));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td (o\\200ce) Tj ET"),
        "office"
    );
}

#[test]
fn ligature_presentation_forms_come_out_as_letters() {
    for (target, letters) in [
        ("FB00", "ff"),
        ("FB01", "fi"),
        ("FB02", "fl"),
        ("FB03", "ffi"),
        ("FB04", "ffl"),
        ("FB05", "st"),
        ("FB06", "st"),
    ] {
        let font = ligature_font(target, None);
        assert_eq!(
            font.char_to_unicode(LIGATURE_CODE).as_deref(),
            Some(letters),
            "{target}"
        );
    }
}

#[test]
fn ligature_presentation_form_spreads_its_advance_over_its_letters() {
    let mut extractor = extractor_with(ligature_font("FB01", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td (de\\200ne) Tj ET"),
        "define"
    );
}

/// A ligature drawn by `Tj` and a space drawn by the `TJ` that follows it: the two operators read the
/// space the same way, in the spans and in the characters.
#[test]
fn ligature_in_tj_then_pulled_back_space_in_tj_array_drops_in_spans_and_chars() {
    let stream: &[u8] = b"BT /F1 10 Tf 72 700 Td (de\\200) Tj [240 ( ne)] TJ ET";
    let mut spans = extractor_with(ligature_font("00660069", None));
    let mut chars = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut spans, stream), "define", "spans");
    assert_eq!(char_text(&mut chars, stream), "define", "chars");
}

#[test]
fn type0_ligature_in_tj_then_pulled_back_space_in_tj_array_drops_in_spans_and_chars() {
    let stream: &[u8] = b"BT /F1 10 Tf 72 700 Td <000100020003> Tj [240 <0020 0004 0002>] TJ ET";
    let mut spans = extractor_with(type0_ligature_font("00660069"));
    let mut chars = extractor_with(type0_ligature_font("00660069"));
    assert_eq!(span_text(&mut spans, stream), "define", "spans");
    assert_eq!(char_text(&mut chars, stream), "define", "chars");
}

/// A glyph of one letter drawn by `Tj` keeps the space that a `TJ` pull-back leaves, and a letter drawn
/// after the ligature clears the ligature, so the pull-back that follows it keeps its space.
#[test]
fn tj_glyph_that_is_not_a_ligature_keeps_the_space_in_a_following_tj_array() {
    for stream in [
        &b"BT /F1 10 Tf 72 700 Td (ab) Tj [240 ( cd)] TJ ET"[..],
        &b"BT /F1 10 Tf 72 700 Td (de\\200) Tj (xy) Tj [240 ( ne)] TJ ET"[..],
    ] {
        let mut spans = extractor_with(ligature_font("00660069", None));
        let mut chars = extractor_with(ligature_font("00660069", None));
        let (s, c) = (span_text(&mut spans, stream), char_text(&mut chars, stream));
        assert_eq!(s, c, "spans and chars agree");
        assert!(s.contains(' '), "{s:?}");
    }
}

/// A Type0 code that yields two letters has two widths in vertical writing too.
#[test]
fn type0_vertical_ligature_code_has_one_width_for_each_letter() {
    for stream in [
        &b"BT /F1 10 Tf 72 700 Td <000100020003> Tj ET"[..],
        &b"BT /F1 10 Tf 72 700 Td [<000100020003>] TJ ET"[..],
    ] {
        let mut font = type0_ligature_font("FB01");
        font.wmode = 1;
        let mut extractor = extractor_with(font);
        let spans = extractor.extract_text_spans(stream).unwrap();
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        let widths: usize = spans.iter().map(|s| s.char_widths.len()).sum();
        assert_eq!(text, "defi");
        assert_eq!(widths, text.chars().count(), "{stream:?}");
    }
}

/// The character path: a space that the pull-back left in place ends the ligature, so a second space
/// that starts inside the first one is not taken for one inside a ligature box. At 100 units the two
/// spaces lie 15 units apart, far enough to both stay after the removal of overlapping characters.
#[test]
fn kept_space_ends_the_ligature_for_the_next_space_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        char_text(
            &mut extractor,
            b"BT /F1 100 Tf 72 700 Td [(de\\200) 150 ( ) 400 ( ) (ne)] TJ ET"
        ),
        "defi  ne"
    );
}

/// A Type0 pull-back of 60% leaves part of the space between the words, as it does for a simple font.
#[test]
fn type0_partial_pullback_keeps_the_space_in_spans_and_chars() {
    let stream: &[u8] = b"BT /F1 10 Tf 72 700 Td [<000100020003> 150 <0020 0004 0002>] TJ ET";
    let mut spans = extractor_with(type0_ligature_font("00660069"));
    let mut chars = extractor_with(type0_ligature_font("00660069"));
    assert_eq!(span_text(&mut spans, stream), "defi ne", "spans");
    assert_eq!(char_text(&mut chars, stream), "defi ne", "chars");
}

/// Character spacing widens the space to 4.5 units, so a pull-back of 2.26 units covers about half of
/// it and the space keeps its room, in both paths.
#[test]
fn char_spacing_widens_the_space_so_a_pullback_of_226_keeps_it() {
    let stream: &[u8] = b"BT /F1 10 Tf 2 Tc 72 700 Td [(de\\200) 226 ( ne)] TJ ET";
    let mut spans = extractor_with(ligature_font("00660069", None));
    let mut chars = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut spans, stream), "defi ne", "spans");
    assert_eq!(char_text(&mut chars, stream), "defi ne", "chars");
}

/// A setting that changes between the `Tj` and the `TJ` ends the run, so the ligature does not carry
/// into the array and the space keeps its room.
#[test]
fn colour_change_between_tj_and_tj_array_ends_the_ligature() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(
            &mut extractor,
            b"BT /F1 10 Tf 72 700 Td (de\\200) Tj 1 0 0 rg [240 ( ne)] TJ ET"
        ),
        "defi ne"
    );
}

/// A `Tm` between the `Tj` and the `TJ` sets the pen again. The pull-back of the array no longer refers
/// to the ligature, so the space keeps its room at every distance, in both paths and both font kinds.
#[test]
fn text_matrix_between_tj_and_tj_array_ends_the_ligature() {
    for gap in [95, 102, 132] {
        let simple = format!("BT /F1 10 Tf 72 700 Td (de\\200) Tj 1 0 0 1 {gap} 700 Tm [240 ( ne)] TJ ET");
        let type0 =
            format!("BT /F1 10 Tf 72 700 Td <000100020003> Tj 1 0 0 1 {gap} 700 Tm [240 <0020 0004 0002>] TJ ET");
        for (stream, font) in [
            (simple, ligature_font("00660069", None)),
            (type0, type0_ligature_font("00660069")),
        ] {
            let (mut spans, mut chars) = (extractor_with(font.clone()), extractor_with(font));
            assert_eq!(
                span_text(&mut spans, stream.as_bytes()),
                "defi ne",
                "spans, Tm at {gap}"
            );
            assert_eq!(
                char_text(&mut chars, stream.as_bytes()),
                "defi ne",
                "chars, Tm at {gap}"
            );
        }
    }
}

/// Each setting that the run captured ends the ligature when it changes between the `Tj` and the `TJ`.
/// Each pull-back below covers the whole space under the new setting, so a run that went on across the
/// change would drop the space. The spans are checked; the characters ignore the change, as before.
#[test]
fn setting_change_between_tj_and_tj_array_ends_the_ligature() {
    for (setting, kern) in [
        ("2 Tc", 450),
        ("3 Tw", 550),
        ("150 Tz", 240),
        ("1 Tr", 240),
        ("2 Ts", 240),
    ] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (de\\200) Tj {setting} [{kern} ( ne)] TJ ET");
        let mut spans = extractor_with(ligature_font("00660069", None));
        assert_eq!(span_text(&mut spans, stream.as_bytes()), "defi ne", "after {setting}");
    }
}

/// Vertical writing does not carry a ligature into an array: the pull-back is a horizontal notion, so the
/// `Tj` and the `TJ` stay two spans, as before.
#[test]
fn type0_vertical_tj_then_tj_array_stays_two_spans() {
    let mut font = type0_ligature_font("00660069");
    font.wmode = 1;
    let mut extractor = extractor_with(font);
    let spans = extractor
        .extract_text_spans(b"BT /F1 10 Tf 72 700 Td <000100020003> Tj [240 <0020 0004 0002>] TJ ET")
        .unwrap();
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["defi", " ne"]);
}

/// A ligature drawn by `Tj`, then an array of one offset, then a `Tj`: the run goes on through the array,
/// as it does when the ligature sits in the array itself.
#[test]
fn ligature_in_tj_then_offset_array_then_tj_reads_like_the_array_form() {
    let spans_of = |stream: &[u8]| -> Vec<String> {
        let mut extractor = extractor_with(ligature_font("00660069", None));
        extractor
            .extract_text_spans(stream)
            .unwrap()
            .iter()
            .map(|s| s.text.clone())
            .collect()
    };
    let tj_form = spans_of(b"BT /F1 10 Tf 72 700 Td (de\\200) Tj [240] TJ (ne) Tj ET");
    let array_form = spans_of(b"BT /F1 10 Tf 72 700 Td [(de\\200) 240] TJ (ne) Tj ET");
    assert_eq!(tj_form, ["define"]);
    assert_eq!(array_form, tj_form);
}

/// A `Tj` of UTF-8 text in a font without a ToUnicode map ends the ligature before it, so the space in
/// the array that follows keeps its room.
#[test]
fn utf8_tj_after_a_ligature_ends_the_ligature() {
    let mut font = ligature_font("00660069", Some("fi"));
    font.to_unicode = None;
    let mut extractor = extractor_with(font);
    let text = span_text(
        &mut extractor,
        b"BT /F1 10 Tf 72 700 Td (de\\200) Tj (\\320\\257) Tj [240 ( ne)] TJ ET",
    );
    assert!(text.ends_with("\u{42F} ne"), "{text:?}");
}

/// The word "define" with the ligature drawn by a show of its own, between two text matrices, as some
/// word processors write it (GH#2664). The glyphs are drawn at 10 points in user space, by a font of size
/// `10 / scale` under a text matrix of scale `scale` (a 1-point font at scale 10). The show after the
/// ligature starts 0.02 points before the ligature ends.
fn own_show_ligature_stream(type0: bool, scale: f32) -> String {
    let size = 10.0 / scale;
    let tm = |x: f32| format!("{scale} 0 0 {scale} {x:.2} 700 Tm");
    let (head, ligature, tail) = if type0 {
        ("<00010002>", "<0003>", "<00040002>")
    } else {
        ("(de)", "(\\200)", "(ne)")
    };
    format!(
        "BT /F1 {size} Tf {} [{head}] TJ {} {ligature} Tj {} [{tail}] TJ ET",
        tm(72.0),
        tm(82.0),
        tm(91.98)
    )
}

fn own_show_fonts() -> [(bool, FontInfo); 2] {
    [
        (false, ligature_font("00660069", None)),
        (true, type0_ligature_font("00660069")),
    ]
}

/// A text matrix that scales the glyphs does not open a gap after a ligature drawn by its own show: the
/// run that holds the ligature is as wide as the ligature, so the next show joins it.
#[test]
fn ligature_in_its_own_show_under_a_scaled_text_matrix_joins_the_word_in_spans() {
    for scale in [1.0, 0.75, 0.5, 10.0] {
        for (type0, font) in own_show_fonts() {
            let mut extractor = extractor_with(font);
            let stream = own_show_ligature_stream(type0, scale);
            assert_eq!(
                span_text(&mut extractor, stream.as_bytes()),
                "define",
                "scale {scale}, type0 {type0}"
            );
        }
    }
}

/// A run that goes on across a text matrix keeps the width of its glyphs in user space, whatever the
/// matrix scale: the same glyphs drawn at the same user-space size give runs of the same width.
#[test]
fn run_across_a_scaled_text_matrix_keeps_its_user_space_width() {
    let width_of = |stream: &[u8]| -> f32 {
        let mut extractor = extractor_with(ligature_font("00660069", None));
        let spans = extractor.extract_text_spans(stream).unwrap();
        assert_eq!(spans.len(), 1, "one run expected: {spans:?}");
        assert_eq!(spans[0].text, "den");
        spans[0].bbox.width
    };
    let unscaled = width_of(b"BT /F1 10 Tf 1 0 0 1 72 700 Tm (de) Tj 1 0 0 1 82 700 Tm (n) Tj ET");
    let scaled = width_of(b"BT /F1 20 Tf 0.5 0 0 0.5 72 700 Tm (de) Tj 0.5 0 0 0.5 82 700 Tm (n) Tj ET");
    let enlarged = width_of(b"BT /F1 1 Tf 10 0 0 10 72 700 Tm (de) Tj 10 0 0 10 82 700 Tm (n) Tj ET");
    assert!((unscaled - 15.0).abs() < 0.01, "unscaled run width {unscaled}");
    assert!(
        (enlarged - unscaled).abs() < 0.01,
        "enlarged run width {enlarged}, unscaled {unscaled}"
    );
    assert!(
        (scaled - unscaled).abs() < 0.01,
        "scaled run width {scaled}, unscaled {unscaled}"
    );
}
