//! The spot mirror blends a paint against the lane's backdrop. These tests call it directly on a
//! one-row sidecar, so a blend mode that tiny-skia cannot draw in a debug build (Overlay) is
//! still reached.

use super::*;

const WIDTH: u32 = 3;

fn renderer() -> PageRenderer {
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.cmyk_sidecar = Some(CmykSidecar::new(WIDTH, 1, vec!["InkA".to_string()]));
    renderer
}

fn state(mode: &str, tint: f32, alpha: f32) -> GraphicsState {
    GraphicsState {
        blend_mode: mode.to_string(),
        fill_alpha: alpha,
        fill_spot_inks: vec![("InkA".to_string(), tint)],
        ..GraphicsState::default()
    }
}

/// Paint `tint` in `mode` over the pixels whose coverage is non-zero.
fn paint(renderer: &mut PageRenderer, mode: &str, tint: f32, alpha: f32, coverage: [u8; 3]) {
    let pixmap = Pixmap::new(WIDTH, 1).expect("pixmap");
    let snapshot = pixmap.data().to_vec();
    let gs = state(mode, tint, alpha);
    renderer.mirror_spot_paint_into_sidecar_with_coverage(&pixmap, &snapshot, Some(&coverage), &gs, true);
}

fn lane(renderer: &PageRenderer) -> Vec<u8> {
    renderer
        .cmyk_sidecar
        .as_ref()
        .expect("sidecar")
        .spot_plane(0)
        .expect("lane")
        .to_vec()
}

fn painted(renderer: &PageRenderer) -> Vec<u8> {
    let state = renderer.cmyk_sidecar.as_ref().expect("sidecar").spot_state();
    state[state.len() / 2..].to_vec()
}

#[test]
fn overlay_over_an_opaque_tint_zero_paint_gives_no_ink() {
    let mut r = renderer();
    paint(&mut r, "Normal", 0.8, 1.0, [255; 3]);
    paint(&mut r, "Normal", 0.0, 1.0, [255; 3]);
    assert_eq!(lane(&r), vec![0; 3], "control: the tint-0 paint knocked the ink out");
    paint(&mut r, "Overlay", 0.5, 1.0, [255; 3]);
    assert_eq!(
        lane(&r),
        vec![0; 3],
        "Overlay at tint 0.5 over a tint-0 backdrop gives no ink"
    );
}

#[test]
fn overlay_over_untouched_paper_keeps_the_source_ink() {
    let mut r = renderer();
    paint(&mut r, "Overlay", 0.5, 1.0, [255; 3]);
    assert_eq!(lane(&r), vec![128; 3]);
}

#[test]
fn every_separable_mode_keeps_the_source_ink_on_the_pixels_no_earlier_paint_reached() {
    for mode in [
        "Multiply",
        "Screen",
        "Overlay",
        "Darken",
        "Lighten",
        "ColorDodge",
        "ColorBurn",
        "HardLight",
        "SoftLight",
    ] {
        let mut r = renderer();
        paint(&mut r, "Normal", 0.8, 1.0, [255, 0, 0]);
        paint(&mut r, mode, 0.5, 1.0, [255, 255, 0]);
        let lane = lane(&r);
        assert_ne!(lane[1], 0, "{mode}: ink lost over untouched paper");
        assert_eq!(lane[1], 128, "{mode}: the source ink over untouched paper");
    }
}

#[test]
fn the_painted_plane_is_the_union_of_the_alphas_of_the_paints() {
    let mut r = renderer();
    assert_eq!(painted(&r), vec![0; 3]);
    paint(&mut r, "Normal", 0.0, 1.0, [255, 0, 0]);
    assert_eq!(
        painted(&r),
        vec![255, 0, 0],
        "a tint-0 paint still marks the lane painted"
    );
    paint(&mut r, "Normal", 0.5, 0.5, [0, 255, 255]);
    assert_eq!(painted(&r), vec![255, 128, 128]);
    paint(&mut r, "Normal", 0.5, 0.4, [0, 0, 255]);
    assert_eq!(painted(&r), vec![255, 128, 179], "1 - (1 - 128/255) * 0.6 = 0.701");
}

#[test]
fn a_partly_painted_lane_blends_by_its_backdrop_alpha() {
    let mut r = renderer();
    // Half alpha Normal tint 0.5: lane 0.25 (64), backdrop alpha 128/255.
    paint(&mut r, "Normal", 0.5, 0.5, [0, 255, 0]);
    assert_eq!(lane(&r), vec![0, 64, 0]);
    // Lighten at tint 0.5, opaque: blend (backdrop 0.25) gives ink 0.25; the source is 0.5.
    // Result = (1 - a_b) * 0.5 + a_b * 0.25 with a_b = 128/255.
    paint(&mut r, "Lighten", 0.5, 1.0, [0, 255, 0]);
    let a_b = 128.0_f32 / 255.0;
    let expected = (((1.0 - a_b) * 0.5 + a_b * (64.0 / 255.0)) * 255.0).round() as u8;
    assert_eq!(lane(&r)[1], expected);
}
