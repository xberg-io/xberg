//! The sidecar transparency gate reads ExtGState through the shared parser. These tests
//! pin the rendered output of the pages whose gate decision that move changed.

use super::*;
use crate::rendering::page_renderer::{PageRenderer, RenderOptions};
use crate::rendering::sidecar::page_declares_transparency;

const DPI: u32 = 72;

/// Two overlapping fills in the spot ink `InkA`; the second runs under `/T gs`.
const CONTENT: &str = "/CS0 cs 0.9 scn 10 10 60 60 re f /T gs /CS0 cs 0.5 scn 40 40 50 50 re f";

const SPOT_A: &str = "/CS0 [/Separation /InkA /DeviceCMYK \
    << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>]";
const SPOT_B: &str = "/CS1 [/Separation /InkB /DeviceCMYK \
    << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>]";

fn build_page(state_body: &str, extras: &[&str]) -> PdfDocument {
    build_page_with(CONTENT, SPOT_A, state_body, extras)
}

fn build_page_with(content: &str, colour_spaces: &str, state_body: &str, extras: &[&str]) -> PdfDocument {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offs: Vec<usize> = Vec::new();
    let mut push = |buf: &mut Vec<u8>, body: &str| {
        offs.push(buf.len());
        buf.extend_from_slice(body.as_bytes());
    };
    push(&mut buf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push(&mut buf, "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    push(
        &mut buf,
        &format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /ColorSpace << {colour_spaces} >> \
             /ExtGState << /T << /Type /ExtGState {state_body} >> >> >> >>\nendobj\n"
        ),
    );
    push(
        &mut buf,
        &format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len()
        ),
    );
    for extra in extras {
        push(&mut buf, extra);
    }
    let xref_off = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offs.len() + 1).as_bytes());
    for off in &offs {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n",
            offs.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(buf).expect("synthetic PDF parses")
}

fn gate(doc: &PdfDocument) -> bool {
    page_declares_transparency(doc, &doc.get_page_resources(0).expect("resources"))
}

fn plate(state_body: &str, extras: &[&str]) -> Vec<u8> {
    plate_of(CONTENT, state_body, extras)
}

fn plate_of(content: &str, state_body: &str, extras: &[&str]) -> Vec<u8> {
    let inks = vec!["InkA".to_string()];
    let doc = build_page_with(content, SPOT_A, state_body, extras);
    let plates = render_plates_for_inks(&doc, 0, DPI, &inks, &inks).expect("render plates");
    assert_eq!(plates.len(), 1);
    plates.into_iter().next().expect("one plate").data
}

fn page(state_body: &str, force_sidecar: bool) -> Vec<u8> {
    page_with(state_body, &[], force_sidecar)
}

fn page_with(state_body: &str, extras: &[&str], force_sidecar: bool) -> Vec<u8> {
    let doc = build_page(state_body, extras);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(DPI).as_raw());
    renderer.force_cmyk_sidecar = force_sidecar;
    renderer.render_page(&doc, 0).expect("render page").data
}

#[test]
fn indirect_blend_array_element_blends_like_a_direct_name() {
    let direct = plate("/BM /Multiply", &[]);
    let indirect = plate("/BM [5 0 R]", &["5 0 obj\n/Multiply\nendobj\n"]);
    let unblended = plate("/BM /Normal", &[]);
    assert!(direct != unblended, "Multiply must change the plate");
    assert!(indirect == direct, "indirect /BM array element must blend as /Multiply");
}

#[test]
fn malformed_soft_mask_stays_off_the_composite_path_and_renders_as_normal() {
    let normal_plate = plate("/BM /Normal", &[]);
    let normal_page = page("/BM /Normal", false);
    assert!(normal_page.iter().any(|&b| b != 255), "page is blank");
    for state in [
        "/SMask << /S /Alpha >>",
        "/SMask << /S /Alpha /G 7 >>",
        "/SMask 3",
        "/SMask 99 0 R",
    ] {
        assert!(!gate(&build_page(state, &[])), "{state}: gate fired");
        assert!(plate(state, &[]) == normal_plate, "{state}: plate differs from Normal");
        assert!(page(state, false) == normal_page, "{state}: page differs from Normal");
        assert!(
            page(state, true) == normal_page,
            "{state}: page with sidecar differs from Normal"
        );
    }
}

#[test]
fn unrecognised_blend_names_read_as_normal() {
    let normal_plate = plate("/BM /Normal", &[]);
    let normal_page = page("/BM /Normal", false);
    for state in ["/BM /NoSuchMode", "/BM /Compatible"] {
        assert!(!gate(&build_page(state, &[])), "{state}: gate fired");
        assert!(plate(state, &[]) == normal_plate, "{state}: plate differs from Normal");
        assert!(page(state, false) == normal_page, "{state}: page differs from Normal");
        assert!(
            page(state, true) == normal_page,
            "{state}: page with sidecar differs from Normal"
        );
    }
}

const SIDE: usize = 100;

fn at(data: &[u8], x: usize, y: usize) -> u8 {
    data[y * SIDE + x]
}

/// Sample points of the two-rectangle page, in pixels from the top left at 72 dpi: only the
/// first rectangle, both rectangles, and only the second rectangle.
const ONLY_FIRST: (usize, usize) = (20, 80);
const OVERLAP: (usize, usize) = (55, 45);
const ONLY_SECOND: (usize, usize) = (80, 20);

/// Multiply on subtractive tints: complement, multiply, complement.
fn stacked(backdrop: u8, source: f32) -> u8 {
    let b = backdrop as f32 / 255.0;
    ((1.0 - (1.0 - b) * (1.0 - source)) * 255.0).round() as u8
}

fn assert_multiply_plate_matches_page(label: &str, state: &str, extras: &[&str]) {
    let normal = plate("/BM /Normal", &[]);
    let multiply = plate(state, extras);
    let at_first = at(&multiply, ONLY_FIRST.0, ONLY_FIRST.1);
    let at_second = at(&multiply, ONLY_SECOND.0, ONLY_SECOND.1);
    let at_overlap = at(&multiply, OVERLAP.0, OVERLAP.1);
    assert_eq!(
        at_first,
        at(&normal, ONLY_FIRST.0, ONLY_FIRST.1),
        "{label}: only the first rectangle"
    );
    assert!(
        at_second > 0,
        "{label}: ink is lost where only the second rectangle covers"
    );
    assert_eq!(
        at_second,
        at(&normal, ONLY_SECOND.0, ONLY_SECOND.1),
        "{label}: only the second rectangle"
    );
    assert_eq!(
        at_overlap,
        stacked(at_first, 0.5),
        "{label}: overlap stacks the two inks"
    );
    assert!(at_overlap > at_first, "{label}: overlap is denser than the backdrop");
    let rendered = page_with(state, extras, false);
    let white = |x: usize, y: usize| rendered[(y * SIDE + x) * 4..(y * SIDE + x) * 4 + 3] == [255, 255, 255];
    assert!(
        !white(ONLY_SECOND.0, ONLY_SECOND.1),
        "{label}: control, the page shows ink there"
    );
}

#[test]
fn multiply_plate_keeps_the_ink_the_page_shows() {
    assert_multiply_plate_matches_page("direct name", "/BM /Multiply", &[]);
}

#[test]
fn multiply_through_an_indirect_array_element_keeps_the_ink_the_page_shows() {
    assert_multiply_plate_matches_page("indirect array", "/BM [5 0 R]", &["5 0 obj\n/Multiply\nendobj\n"]);
}

#[test]
fn darken_plate_keeps_the_ink_where_only_the_source_covers() {
    let darken = plate("/BM /Darken", &[]);
    let normal = plate("/BM /Normal", &[]);
    assert_eq!(
        at(&darken, ONLY_SECOND.0, ONLY_SECOND.1),
        at(&normal, ONLY_SECOND.0, ONLY_SECOND.1)
    );
    assert!(at(&darken, ONLY_SECOND.0, ONLY_SECOND.1) > 0);
}

/// Control, green on main: over untouched paper Screen and Lighten already gave the source ink,
/// and Difference and Exclusion are replaced by Normal on spot lanes, so none of the four ever
/// reached the blend this change fixes.
#[test]
fn control_screen_lighten_difference_exclusion_keep_the_source_ink_over_paper() {
    let normal = plate("/BM /Normal", &[]);
    for mode in ["Screen", "Lighten", "Difference", "Exclusion"] {
        let state = format!("/BM /{mode}");
        let blended = plate(&state, &[]);
        let sample = at(&blended, ONLY_SECOND.0, ONLY_SECOND.1);
        assert_eq!(sample, at(&normal, ONLY_SECOND.0, ONLY_SECOND.1), "{mode}");
        assert!(sample > 0, "{mode}: ink is lost over an empty area");
    }
}

/// Ink that Color Dodge and Color Burn leave in the overlap, from the additive formulas of
/// ISO 32000-1 11.3.5 with the source tint 0.5: the backdrop complement is `(255 - b) / 255`.
fn dodge_overlap(backdrop: u8) -> u8 {
    let cb = (255 - backdrop as i32) as f32 / 255.0;
    ((1.0 - (cb / 0.5).min(1.0)) * 255.0).round() as u8
}

fn burn_overlap(backdrop: u8) -> u8 {
    let cb = (255 - backdrop as i32) as f32 / 255.0;
    ((1.0 - (1.0 - ((1.0 - cb) / 0.5).min(1.0))) * 255.0).round() as u8
}

#[test]
fn color_dodge_and_color_burn_plates_keep_the_ink_over_paper_and_follow_the_additive_formulas() {
    let normal = plate("/BM /Normal", &[]);
    let first = at(&normal, ONLY_FIRST.0, ONLY_FIRST.1);
    let second = at(&normal, ONLY_SECOND.0, ONLY_SECOND.1);
    for (mode, overlap) in [("ColorDodge", dodge_overlap(first)), ("ColorBurn", burn_overlap(first))] {
        let state = format!("/BM /{mode}");
        let blended = plate(&state, &[]);
        assert_eq!(
            at(&blended, ONLY_SECOND.0, ONLY_SECOND.1),
            second,
            "{mode}: ink is lost where only the second rectangle covers"
        );
        assert_eq!(
            at(&blended, ONLY_FIRST.0, ONLY_FIRST.1),
            first,
            "{mode}: first rectangle"
        );
        assert_eq!(at(&blended, OVERLAP.0, OVERLAP.1), overlap, "{mode}: overlap");
    }
}

/// Alpha of a page pixel is always 255 here, so the colour channels alone tell how much ink the
/// page shows. The red channel stays 255 for the ink `InkA`, so the green channel is the gauge.
fn green(data: &[u8], (x, y): (usize, usize)) -> u8 {
    data[(y * SIDE + x) * 4 + 1]
}

/// Screen and Lighten put less ink in the overlap than the first rectangle holds, on the plate
/// and on the page. (Over untouched paper the page shows white for both modes while the plate
/// keeps the source ink; that page difference is not part of this change.)
#[test]
fn screen_and_lighten_plate_and_page_both_lighten_the_overlap() {
    let normal_plate = plate("/BM /Normal", &[]);
    let normal_page = page_with("/BM /Normal", &[], false);
    assert_eq!(
        green(&normal_page, OVERLAP),
        green(&normal_page, ONLY_SECOND),
        "control: Normal paints the source colour in the overlap"
    );
    for mode in ["Screen", "Lighten"] {
        let state = format!("/BM /{mode}");
        let plates = plate(&state, &[]);
        let rendered = page_with(&state, &[], false);
        assert!(
            at(&plates, OVERLAP.0, OVERLAP.1) < at(&plates, ONLY_FIRST.0, ONLY_FIRST.1),
            "{mode} plate: the overlap holds less ink than the first rectangle"
        );
        assert!(
            green(&rendered, OVERLAP) > green(&rendered, ONLY_FIRST),
            "{mode} page: the overlap is lighter than the first rectangle"
        );
        assert!(
            at(&plates, OVERLAP.0, OVERLAP.1) <= at(&normal_plate, OVERLAP.0, OVERLAP.1),
            "{mode} plate: no denser than Normal"
        );
    }
}

/// An opaque tint-0 paint is a backdrop, not untouched paper: Lighten at tint 0.5 over it gives
/// no ink. Treating a tint of 0 as "no backdrop" keeps 128 here. Overlay, the blend of the
/// reported case, cannot be drawn by tiny-skia in a debug build; the spot mirror tests cover it.
const KNOCKOUT_THEN_BLEND: &str =
    "/CS0 cs 0.8 scn 10 10 60 60 re f 0 scn 10 10 60 60 re f /T gs 0.5 scn 10 10 60 60 re f";

#[test]
fn lighten_over_an_explicit_tint_zero_paint_gives_no_ink() {
    let lighten = plate_of(KNOCKOUT_THEN_BLEND, "/BM /Lighten", &[]);
    assert_eq!(at(&lighten, ONLY_FIRST.0, ONLY_FIRST.1), 0);
    let normal = plate_of(KNOCKOUT_THEN_BLEND, "/BM /Normal", &[]);
    assert_eq!(
        at(&normal, ONLY_FIRST.0, ONLY_FIRST.1),
        128,
        "control: Normal paints the source tint"
    );
}

fn sidecar_allocated(state_body: &str) -> bool {
    let doc = build_page(state_body, &[]);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(DPI).as_raw());
    renderer.force_cmyk_sidecar = true;
    renderer.render_page(&doc, 0).expect("render page");
    renderer.take_cmyk_sidecar().is_some()
}

#[test]
fn the_page_renderer_allocates_the_sidecar_only_when_the_page_declares_transparency_or_overprint() {
    assert!(!sidecar_allocated("/BM /Normal"), "an opaque page needs no sidecar");
    assert!(!sidecar_allocated("/ca 1.0"), "an opaque alpha needs no sidecar");
    assert!(sidecar_allocated("/BM /Multiply"), "a blend mode needs the sidecar");
    assert!(sidecar_allocated("/ca 0.5"), "an alpha below one needs the sidecar");
    assert!(
        sidecar_allocated("/OP true"),
        "overprint needs the sidecar on the page path"
    );
}

#[test]
fn a_page_without_transparency_keeps_the_per_plate_knockout() {
    let content = "/CS0 cs 0.9 scn 10 10 60 60 re f /CS1 cs 0.5 scn 40 40 50 50 re f";
    let spots = format!("{SPOT_A} {SPOT_B}");
    let doc = build_page_with(content, &spots, "/BM /Normal", &[]);
    let inks = vec!["InkA".to_string(), "InkB".to_string()];
    assert!(!gate(&doc), "the page declares no transparency");
    let composite = render_plates_via_composite(&doc, 0, DPI, &inks, &inks).expect("composite plates");
    assert!(
        composite.iter().all(|p| p.data.iter().all(|&b| b == 0)),
        "control: the composite route draws no ink on a page without transparency"
    );
    let direct = render_plates_for_inks(&doc, 0, DPI, &inks, &inks).expect("direct plates");
    assert!(
        at(&direct[0].data, ONLY_FIRST.0, ONLY_FIRST.1) > 0,
        "InkA is drawn where only it covers"
    );
    assert!(
        at(&direct[1].data, ONLY_SECOND.0, ONLY_SECOND.1) > 0,
        "InkB is drawn where only it covers"
    );
    assert_eq!(
        at(&direct[0].data, OVERLAP.0, OVERLAP.1),
        0,
        "the opaque InkB paint knocks out InkA"
    );
}
