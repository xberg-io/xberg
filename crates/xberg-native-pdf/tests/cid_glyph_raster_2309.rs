use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const DIRECT: &[u8] = include_bytes!("fixtures/cid_glyph_raster_2309/direct.pdf");
const GOOD: &[u8] = include_bytes!("fixtures/cid_glyph_raster_2309/good.pdf");

fn raster(pdf: &[u8]) -> image::RgbaImage {
    let document = PdfDocument::from_bytes(pdf.to_vec()).expect("valid embedded CID subset PDF");
    assert_eq!(document.page_count().unwrap(), 1);
    let rendered = render_page(&document, 0, &RenderOptions::with_dpi(150)).expect("render CID glyphs");
    assert_eq!((rendered.width, rendered.height), (1275, 1650));
    image::load_from_memory(&rendered.data).unwrap().to_rgba8()
}

fn assert_twenty_source_lines_have_ink(image: &image::RgbaImage) {
    for row in 0..20 {
        let top = ((40 + row * 14) * 150 / 72) as u32;
        let bottom = ((54 + row * 14) * 150 / 72) as u32;
        let ink = (top..bottom)
            .flat_map(|y| (140..1200).map(move |x| image.get_pixel(x, y)))
            .filter(|pixel| pixel[0] < 128 && pixel[1] < 128 && pixel[2] < 128)
            .count();
        assert!(
            ink > 200,
            "source line {row} must contain glyph outlines, found {ink} dark pixels"
        );
    }
}

fn assert_undecodable_run_uses_its_original_cid_outlines(pdf: &[u8]) {
    let reference = raster(DIRECT);
    assert_twenty_source_lines_have_ink(&reference);
    let actual = raster(pdf);
    assert_twenty_source_lines_have_ink(&actual);
    assert!(
        actual.as_raw() == reference.as_raw(),
        "Unicode loss must not change any CID glyph pixel or position"
    );
}

#[test]
fn should_paint_cid_truetype_outlines_when_tounicode_maps_every_code_to_nul() {
    assert_undecodable_run_uses_its_original_cid_outlines(include_bytes!(
        "fixtures/cid_glyph_raster_2309/unicode_nul_mapping.pdf"
    ));
}

#[test]
fn should_paint_cid_truetype_outlines_when_tounicode_omits_every_code() {
    assert_undecodable_run_uses_its_original_cid_outlines(include_bytes!("fixtures/cid_glyph_raster_2309/missing.pdf"));
}

#[test]
fn should_preserve_the_entire_cid_run_when_only_some_unicode_codes_are_missing() {
    assert_undecodable_run_uses_its_original_cid_outlines(include_bytes!("fixtures/cid_glyph_raster_2309/partial.pdf"));
}

#[test]
fn should_keep_good_tounicode_text_with_a_two_entry_font_cmap() {
    let document = PdfDocument::from_bytes(GOOD.to_vec()).unwrap();
    let text = document.extract_text(0).unwrap();
    let expected = [
        "The committee met on Tuesday to review the plan for the new library.",
        "Members asked about the schedule and the budget for the parking area.",
        "The architect said that building work would start after the winter.",
        "A vote on the final design is planned for the next regular meeting.",
    ]
    .repeat(5)
    .join(" ");
    assert_eq!(
        text.split_whitespace().collect::<Vec<_>>(),
        expected.split_whitespace().collect::<Vec<_>>()
    );
}

#[test]
fn should_paint_all_lines_with_a_complete_embedded_unicode_map() {
    let image = raster(include_bytes!("fixtures/cid_glyph_raster_2309/good_full.pdf"));
    assert_twenty_source_lines_have_ink(&image);
}

#[test]
fn should_keep_nonidentity_cmap_runs_on_their_existing_unicode_route() {
    use sha2::{Digest, Sha256};
    let image = raster(include_bytes!("fixtures/cid_glyph_raster_2309/custom.pdf"));
    assert_twenty_source_lines_have_ink(&image);
    let hash: String = Sha256::digest(image.as_raw())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        hash, "be0c8fc8b3cb2a54e5c0bf1cbc4ac3adfeff839f580cd6251581701f8be3c2df",
        "custom code-to-CID geometry must retain its baseline Unicode route"
    );
}

#[test]
fn ink_loss_distinguishes_unpainted_type0_glyphs_from_recovered_decode_misses() {
    let fixtures: [(&[u8], bool); 5] = [
        (include_bytes!("fixtures/cid_glyph_raster_2309/custom.pdf"), true),
        (include_bytes!("fixtures/cid_glyph_raster_2309/partial.pdf"), false),
        (include_bytes!("fixtures/cid_glyph_raster_2309/missing.pdf"), false),
        (GOOD, false),
        (DIRECT, false),
    ];
    for (source, expected_loss) in fixtures {
        let document = PdfDocument::from_bytes(source.to_vec()).unwrap();
        let (rendered, dpi, actual_loss) =
            xberg_native_pdf::rendering::render_png_page_with_ink_evidence(&document, 0, 150, 150, 64 * 1024 * 1024)
                .unwrap();
        assert_eq!((rendered.width, rendered.height, dpi), (1275, 1650, 150));
        assert_eq!(actual_loss, expected_loss);
    }
}
