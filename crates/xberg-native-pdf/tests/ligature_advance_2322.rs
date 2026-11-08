//! A ligature code in an identity-encoded CIDFontType2 run decodes to two or three letters. Shaping
//! those letters reads each glyph's PDF width by its letter index, so every later glyph of the run takes
//! a later code's width ("fi nancial" in the OCR route). The run is painted by CID instead, the route a
//! font with no Unicode cmap takes, so its raster equals that reference pixel for pixel.
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const DIRECT: &[u8] = include_bytes!("fixtures/ligature_advance_2322/direct.pdf");
const PLAIN_DIRECT: &[u8] = include_bytes!("fixtures/ligature_advance_2322/plain_direct.pdf");

fn raster(pdf: &[u8]) -> image::RgbaImage {
    let document = PdfDocument::from_bytes(pdf.to_vec()).expect("valid embedded CID subset PDF");
    assert_eq!(document.page_count().unwrap(), 1);
    let rendered = render_page(&document, 0, &RenderOptions::with_dpi(150)).expect("render CID glyphs");
    assert_eq!((rendered.width, rendered.height), (1275, 1650));
    image::load_from_memory(&rendered.data).unwrap().to_rgba8()
}

fn assert_seven_lines_have_ink(image: &image::RgbaImage) {
    for row in 0..7 {
        let top = ((792 - 720 - 10 + row * 20) * 150 / 72) as u32;
        let bottom = ((792 - 720 + 4 + row * 20) * 150 / 72) as u32;
        let ink = (top..bottom)
            .flat_map(|y| (140..1200).map(move |x| image.get_pixel(x, y)))
            .filter(|pixel| pixel[0] < 128 && pixel[1] < 128 && pixel[2] < 128)
            .count();
        assert!(
            ink > 200,
            "line {row} must contain glyph outlines, found {ink} dark pixels"
        );
    }
}

fn assert_painted_by_cid(pdf: &[u8]) {
    let reference = raster(DIRECT);
    assert_seven_lines_have_ink(&reference);
    let actual = raster(pdf);
    assert_seven_lines_have_ink(&actual);
    assert!(
        actual.as_raw() == reference.as_raw(),
        "a run with a ligature code must place every glyph at its own code's advance, as the CID route does"
    );
}

#[test]
fn should_paint_ligature_codes_by_cid_when_tounicode_lists_their_letters() {
    assert_painted_by_cid(include_bytes!("fixtures/ligature_advance_2322/letters.pdf"));
}

#[test]
fn should_paint_ligature_codes_by_cid_when_tounicode_gives_the_ligature_code_point() {
    assert_painted_by_cid(include_bytes!("fixtures/ligature_advance_2322/codepoint.pdf"));
}

#[test]
fn should_keep_runs_of_single_letter_codes_on_the_shaping_route() {
    // The font keeps its GSUB ligatures: shaping joins the separate f and i codes into one ligature glyph,
    // which the CID route never draws, so equal rasters would mean the run left the shaping route. ~keep
    let reference = raster(PLAIN_DIRECT);
    assert_seven_lines_have_ink(&reference);
    let actual = raster(include_bytes!("fixtures/ligature_advance_2322/plain.pdf"));
    assert_seven_lines_have_ink(&actual);
    assert!(
        actual.as_raw() != reference.as_raw(),
        "a run whose codes each decode to one letter must stay on the Unicode shaping route"
    );
}

#[test]
fn should_paint_a_code_that_decodes_to_a_letter_and_an_accent_by_cid() {
    // The route follows the count of characters a code decodes to, not the ligature code points. The
    // ToUnicode of `e` is U+0065 U+0301, so every run with an `e` leaves the shaping route. The reference is
    // the same codes with no font cmap, where the font would join the separate f and i codes by GSUB. ~keep
    let reference = raster(PLAIN_DIRECT);
    assert_seven_lines_have_ink(&reference);
    let actual = raster(include_bytes!("fixtures/ligature_advance_2322/accent.pdf"));
    assert_seven_lines_have_ink(&actual);
    assert!(
        actual.as_raw() == reference.as_raw(),
        "a code that decodes to a letter and a combining accent must be painted by CID, not shaped"
    );
}

#[test]
fn should_paint_a_code_that_decodes_to_no_characters_by_cid() {
    // The code of `e` decodes to a control character, which the decoder drops with no drop record, so the run
    // has fewer characters than codes. Shaping that run would skip every `e`; the CID route paints it. ~keep
    let reference = raster(PLAIN_DIRECT);
    assert_seven_lines_have_ink(&reference);
    let actual = raster(include_bytes!("fixtures/ligature_advance_2322/fewer.pdf"));
    assert_seven_lines_have_ink(&actual);
    assert!(
        actual.as_raw() == reference.as_raw(),
        "a code that decodes to no characters must be painted by CID, not shaped without its glyph"
    );
}
