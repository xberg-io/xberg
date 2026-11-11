//! Every text route that applies the horizontal scale `Th` (ISO 32000-1 §9.4.4):
//! each test fails when `Th` is dropped from the route's glyph placement, glyph
//! outline or returned advance.

use super::tests::make_vertical_test_font;
use super::*;
use crate::fonts::FontInfo;

const FONT_PX: f32 = 40.0;
const ORIGIN_X: f32 = 150.0;
const GLYPHS: usize = 3;

fn gs_at(tz: f32) -> GraphicsState {
    let mut gs = GraphicsState::new();
    gs.font_size = FONT_PX;
    gs.text_wmode = 0;
    gs.horizontal_scaling = tz;
    gs
}

fn base() -> Transform {
    Transform::from_scale(1.0, -1.0).post_translate(ORIGIN_X, 70.0)
}

fn ink_columns(pixmap: &Pixmap) -> Option<(u32, u32)> {
    let width = pixmap.width();
    let alpha = |x: u32, y: u32| pixmap.data()[((y * width + x) * 4 + 3) as usize] > 127;
    let inked = |x: u32| (0..pixmap.height()).any(|y| alpha(x, y));
    let first = (0..width).find(|&x| inked(x))?;
    let last = (0..width).rev().find(|&x| inked(x))?;
    Some((first, last))
}

fn ink_extent(pixmap: &Pixmap) -> f32 {
    let (_, last) = ink_columns(pixmap).expect("route must paint");
    (last + 1) as f32 - ORIGIN_X
}

/// Renders at Tz 100 and Tz 50 and requires the returned advance and the ink
/// extent measured from the origin (placement of every glyph plus the outline
/// width of the last) to halve.
fn assert_route_halves(route: impl Fn(&mut Pixmap, &GraphicsState) -> f32) {
    let run = |tz: f32| {
        let mut pixmap = Pixmap::new(400, 100).unwrap();
        let advance = route(&mut pixmap, &gs_at(tz));
        (advance, ink_extent(&pixmap))
    };
    let (full_advance, full_extent) = run(100.0);
    let (half_advance, half_extent) = run(50.0);
    assert!(full_advance > 0.0, "route must advance, got {full_advance}");
    assert!(
        (half_advance / full_advance - 0.5).abs() < 0.001,
        "advance must halve: {full_advance} -> {half_advance}"
    );
    assert!(
        (half_extent / full_extent - 0.5).abs() < 0.04,
        "ink extent must halve (placement and outline): {full_extent} -> {half_extent}"
    );
}

fn dejavu() -> Arc<Vec<u8>> {
    Arc::new(crate::fonts::bundled::DEJAVU_SANS.to_vec())
}

/// The system database, not an empty one: the CJK fallback face is cached
/// process-wide from whichever database asks first. ~keep
fn rasterizer() -> TextRasterizer {
    TextRasterizer::new()
}

fn unicode_route(text: &str, paint_missing_glyphs: bool) -> impl Fn(&mut Pixmap, &GraphicsState) -> f32 {
    let text = text.to_string();
    move |pixmap, gs| {
        rasterizer()
            .render_unicode_text(
                pixmap,
                &text,
                text.as_bytes(),
                None,
                None,
                dejavu(),
                0,
                &Paint::default(),
                base(),
                gs,
                None,
                "synthetic",
                false,
                paint_missing_glyphs,
            )
            .unwrap()
    }
}

#[test]
fn unicode_route_applies_horizontal_scaling() {
    assert_route_halves(unicode_route(&"H".repeat(GLYPHS), false));
}

/// Ignored in the shared suite: the CJK fallback face is cached process-wide from whichever database
/// asks first, and tests built on an empty database cache "none". Run alone with
/// `cargo test -p xberg-native-pdf --features cjk-render-fallback --lib unicode_cjk_fallback -- --ignored`.
#[cfg(feature = "cjk-render-fallback")]
#[test]
#[ignore = "needs a process whose first CJK fallback lookup uses the system font database"]
fn unicode_cjk_fallback_route_applies_horizontal_scaling() {
    assert_route_halves(unicode_route(&"\u{53E3}".repeat(GLYPHS), false));
}

#[test]
fn unicode_missing_glyph_box_route_applies_horizontal_scaling() {
    assert_route_halves(unicode_route(&"\u{E000}".repeat(GLYPHS), true));
}

#[test]
fn cid_direct_route_applies_horizontal_scaling() {
    let font_bytes = dejavu();
    let gid = OutlineFace::new(&font_bytes, 0)
        .and_then(|face| face.glyph_index('H'))
        .expect("DejaVu maps H")
        .to_u32() as u16;
    let mut font = make_vertical_test_font();
    font.wmode = 0;
    let bytes: Vec<u8> = (0..GLYPHS).flat_map(|_| gid.to_be_bytes()).collect();
    assert_route_halves(move |pixmap, gs| {
        rasterizer()
            .render_cid_direct(
                pixmap,
                &bytes,
                &font,
                &font_bytes,
                0,
                &Paint::default(),
                base(),
                gs,
                None,
            )
            .unwrap()
    });
}

#[cfg(feature = "cjk-render-fallback")]
fn substituted_font() -> FontInfo {
    let mut font = make_vertical_test_font();
    font.wmode = 0;
    font.cjk_substitution = Some(crate::fonts::predefined_cidfont::CharacterCollection::AdobeJapan1);
    font
}

/// CID 1200 (U+4E00) three times.
#[cfg(feature = "cjk-render-fallback")]
const CJK_CODES: [u8; 6] = [0x04, 0xB0, 0x04, 0xB0, 0x04, 0xB0];

#[cfg(feature = "cjk-render-fallback")]
#[test]
fn substituted_cjk_route_applies_horizontal_scaling() {
    let font = substituted_font();
    assert_route_halves(|pixmap, gs| {
        rasterizer()
            .render_substituted_cjk(
                pixmap,
                &CJK_CODES,
                &font,
                crate::fonts::predefined_cidfont::CharacterCollection::AdobeJapan1,
                &Paint::default(),
                base(),
                gs,
                None,
            )
            .unwrap()
    });
}

#[cfg(feature = "cjk-render-fallback")]
#[test]
fn measure_only_cjk_route_applies_horizontal_scaling() {
    let font = substituted_font();
    let advance = |tz: f32| {
        rasterizer()
            .measure_only_advance(&CJK_CODES, &font, &gs_at(tz))
            .unwrap()
    };
    assert!((advance(100.0) - 3.0 * FONT_PX).abs() < 0.01);
    assert!((advance(50.0) - 1.5 * FONT_PX).abs() < 0.01);
}

#[test]
fn text_fallback_route_stretches_the_box_with_the_pitch() {
    assert_route_halves(|pixmap, gs| {
        rasterizer()
            .render_text_fallback(pixmap, "HHH", &Paint::default(), base(), gs, None)
            .unwrap()
    });
}

#[test]
fn text_fallback_route_draws_nothing_at_zero_scaling() {
    let mut pixmap = Pixmap::new(400, 100).unwrap();
    let advance = rasterizer()
        .render_text_fallback(&mut pixmap, "HHH", &Paint::default(), base(), &gs_at(0.0), None)
        .unwrap();
    assert_eq!(advance, 0.0);
    assert!(ink_columns(&pixmap).is_none(), "Tz 0 must draw nothing");
}

#[test]
fn text_fallback_route_mirrors_to_the_left_at_negative_scaling() {
    let paint = Paint::default();
    let ink_at = |tz: f32| {
        let mut pixmap = Pixmap::new(400, 100).unwrap();
        rasterizer()
            .render_text_fallback(&mut pixmap, "HHH", &paint, base(), &gs_at(tz), None)
            .unwrap();
        pixmap
    };
    let mirrored = ink_columns(&ink_at(-50.0)).expect("negative Tz draws");
    let forward = ink_columns(&ink_at(50.0)).expect("positive Tz draws");
    assert!(
        mirrored.1 as f32 <= ORIGIN_X + 1.0,
        "mirrored ink must sit left of the origin: {mirrored:?}"
    );
    assert_eq!(ORIGIN_X - mirrored.0 as f32, (forward.1 + 1) as f32 - ORIGIN_X);
}

fn simple_font() -> FontInfo {
    let mut font = make_vertical_test_font();
    font.subtype = "Type1".to_string();
    font.wmode = 0;
    font.cid_font_type = None;
    font.cid_to_gid_map = None;
    font.default_width = 500.0;
    font
}

#[test]
fn measured_advance_scales_width_spacing_and_word_spacing_by_th() {
    let font = simple_font();
    let measure = |tz: f32| {
        let mut gs = gs_at(tz);
        gs.char_space = 2.0;
        gs.word_space = 10.0;
        measure_text_bytes(b"a b", &gs, Some(&font))
    };
    let unscaled = 3.0 * (0.5 * FONT_PX + 2.0) + 10.0;
    assert!((measure(100.0) - unscaled).abs() < 0.01);
    assert!((measure(50.0) - unscaled * 0.5).abs() < 0.01);
}

#[test]
fn measured_advance_without_font_scales_by_th() {
    let measure = |tz: f32| {
        let mut gs = gs_at(tz);
        gs.char_space = 2.0;
        gs.word_space = 10.0;
        measure_text_bytes(b"a b", &gs, None)
    };
    let unscaled = 3.0 * (0.6 * FONT_PX + 2.0) + 10.0;
    assert!((measure(100.0) - unscaled).abs() < 0.01);
    assert!((measure(50.0) - unscaled * 0.5).abs() < 0.01);
}
