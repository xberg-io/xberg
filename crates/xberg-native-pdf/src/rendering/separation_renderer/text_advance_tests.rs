//! Skipped text on a separation plate still advances the text matrix by the rasterizer's measure.
//! Each test shows a glyph that paints nothing, then a visible one, and requires the plate to equal
//! the plate of the visible glyph placed at the advance by hand. It runs through the walker: each test covers
//! one of the two call sites of `measure_text_advance`.

use super::gate_parity_tests::{DPI, build_page_with};
use super::*;

/// One glyph, `H`, 1000 units wide: at 20 pt and Tz 50 the advance is 10 pt.
const FONT: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
    /FirstChar 72 /LastChar 72 /Widths [1000] >>";

fn plate_of(content: &str, state_body: &str) -> Vec<u8> {
    let font_object = format!("5 0 obj\n{FONT}\nendobj\n");
    let doc = build_page_with(content, "", state_body, &[font_object.as_str()]);
    let inks = vec!["Cyan".to_string()];
    let plates = render_plates_for_inks(&doc, 0, DPI, &inks, &inks).expect("plate renders");
    plates.into_iter().next().expect("one plate").data
}

fn has_ink(data: &[u8]) -> bool {
    data.iter().any(|&v| v > 0)
}

/// The reference: the visible glyph alone, placed where the skipped one's advance puts it.
fn reference() -> Vec<u8> {
    let plate = plate_of("1 0 0 0 k BT /F1 20 Tf 50 Tz 20 10 Td (H) Tj ET", "");
    assert!(has_ink(&plate), "the reference glyph must ink the plate");
    plate
}

#[test]
fn invisible_text_advances_the_plate_by_the_rasterizer_measure() {
    let plate = plate_of("1 0 0 0 k BT /F1 20 Tf 50 Tz 10 10 Td 3 Tr (H) Tj 0 Tr (H) Tj ET", "");
    assert_eq!(plate, reference());
}

#[test]
fn unpainted_text_advances_the_plate_by_the_rasterizer_measure() {
    // With overprint and OPM 1 a zero Cyan component leaves the Cyan plate alone, so the first
    // glyph reaches no plate; the second, in nonzero Cyan, paints.
    let plate = plate_of(
        "/T gs 0 0 0 0 k BT /F1 20 Tf 50 Tz 10 10 Td (H) Tj 1 0 0 0 k (H) Tj ET",
        "/op true /OP true /OPM 1",
    );
    assert_eq!(plate, reference());
}
