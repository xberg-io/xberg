//! The sidecar transparency gate reads ExtGState through the shared parser. These tests
//! pin the rendered output of the pages whose gate decision that move changed.

use super::*;
use crate::rendering::page_renderer::{
    PNG_FIXED_OVERHEAD, PageRenderer, PngRenderBudget, RGBA_AND_PNG_BYTES_PER_PIXEL, RenderOptions,
};
use crate::rendering::sidecar::page_declares_transparency;

pub(super) const DPI: u32 = 72;

/// Two overlapping fills in the spot ink `InkA`; the second runs under `/T gs`.
const CONTENT: &str = "/CS0 cs 0.9 scn 10 10 60 60 re f /T gs /CS0 cs 0.5 scn 40 40 50 50 re f";

const SPOT_A: &str = "/CS0 [/Separation /InkA /DeviceCMYK \
    << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>]";
const SPOT_B: &str = "/CS1 [/Separation /InkB /DeviceCMYK \
    << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>]";

fn build_page(state_body: &str, extras: &[&str]) -> PdfDocument {
    build_page_with(CONTENT, SPOT_A, state_body, extras)
}

pub(super) fn build_page_with(content: &str, colour_spaces: &str, state_body: &str, extras: &[&str]) -> PdfDocument {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offs: Vec<usize> = Vec::new();
    let mut push = |buf: &mut Vec<u8>, body: &str| {
        offs.push(buf.len());
        buf.extend_from_slice(body.as_bytes());
    };
    push(&mut buf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push(&mut buf, "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let xobjects = if content.contains("/F Do") {
        "/XObject << /F 5 0 R >>"
    } else {
        ""
    };
    let fonts = if content.contains("/F1 ") {
        "/Font << /F1 5 0 R >>"
    } else {
        ""
    };
    push(
        &mut buf,
        &format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /ColorSpace << {colour_spaces} >> {xobjects} {fonts} \
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

/// A transparency group without /K composites its paints against the page backdrop, so a
/// Lighten inside it over a tint-0 paint on the page gives no ink as well.
#[test]
fn lighten_in_a_non_knockout_group_over_a_tint_zero_paint_gives_no_ink() {
    let form = |blend: &str| {
        let body = "/CS0 cs /T gs 0.5 scn 10 10 60 60 re f";
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
             /Resources << /ColorSpace << {SPOT_A} >> /ExtGState << /T << /Type /ExtGState /BM /{blend} >> >> >> \
             /Group << /Type /Group /S /Transparency >> /Length {} >>\nstream\n{body}\nendstream\nendobj\n",
            body.len()
        )
    };
    let content = "/CS0 cs 0.8 scn 10 10 60 60 re f 0 scn 10 10 60 60 re f /F Do";
    let lighten = plate_of(content, "/BM /Normal", &[&form("Lighten")]);
    assert_eq!(at(&lighten, ONLY_FIRST.0, ONLY_FIRST.1), 0);
    let normal = plate_of(content, "/BM /Normal", &[&form("Normal")]);
    assert_eq!(
        at(&normal, ONLY_FIRST.0, ONLY_FIRST.1),
        128,
        "control: Normal in the group paints the source tint"
    );
}

const BUDGET_REQUESTED_DPI: u32 = 288;
const BUDGET_MINIMUM_DPI: u32 = 72;

/// Bytes the PNG fit charges the 100 x 100 pt page at `dpi` with one ink and `copies` transient
/// copies of the sidecar planes (the pixel side is rounded up as the fit rounds it).
fn one_ink_charge(dpi: u32, copies: usize) -> usize {
    let side = (f64::from(dpi) * 100.0 / 72.0).ceil() as usize;
    side * side * (RGBA_AND_PNG_BYTES_PER_PIXEL + (copies + 1) * (4 + 2)) + PNG_FIXED_OVERHEAD
}

/// Bytes the fit charges the 100 x 100 pt page at `dpi` for `per_pixel` run-time bytes.
fn one_ink_peak_charge(dpi: u32, per_pixel: usize) -> usize {
    let side = (f64::from(dpi) * 100.0 / 72.0).ceil() as usize;
    side * side * per_pixel + PNG_FIXED_OVERHEAD
}

/// One ink: rgba + sidecar of a canvas or group is 4 + 4 + 2 bytes per pixel.
const CANVAS: usize = 4 + 4 + 2;
/// Worst-case paint scratch: two snapshots, a non-separable blend copy and fill-stroke coverage.
const SCRATCH: usize = crate::rendering::page_renderer::MAX_PAINT_SCRATCH_BYTES_PER_PIXEL;
/// Two groups' worth of one knockout reservation: twice rgba + sidecar and twice the shape, plus
/// the shape workspace, with a shape of one ink plus a coverage plane (2).
const KNOCKOUT: usize = 2 * CANVAS + 3 * 2;
/// A soft mask inside a group: mask workspace and backdrop (3 rgba), paint snapshots, the shape
/// and the mask form's group.
const SOFT_MASK_IN_GROUP: usize = 12 + CANVAS + 2 + CANVAS;

/// A group form `number` with its own `/Resources` extras and `body`.
fn group_form_obj(number: u32, resources: &str, body: &str) -> String {
    transparency_form(
        number,
        &format!("/Resources << /ColorSpace << {SPOT_A} >> {resources} >>"),
        "",
        body,
    )
}

fn transparency_form(number: u32, resources: &str, group: &str, body: &str) -> String {
    format!(
        "{number} 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
         {resources} /Group << /Type /Group /S /Transparency {group} >> \
         /Length {} >>\nstream\n{body}\nendstream\nendobj\n",
        body.len()
    )
}

fn nested_group_page() -> PdfDocument {
    let outer = group_form_obj(5, "/XObject << /G 6 0 R >>", "/G Do");
    let inner = group_form_obj(6, "", "/CS0 cs 0.8 scn 10 10 60 60 re f");
    build_page_with("/F Do", SPOT_A, "/BM /Multiply", &[&outer, &inner])
}

// /Bridge and /G are siblings in /F's resources. /Bridge inherits those
// resources and invokes /G while its own transparency group is active. ~keep
fn inherited_sibling_group_page() -> PdfDocument {
    let outer = group_form_obj(5, "/XObject << /Bridge 6 0 R /G 7 0 R >>", "/Bridge Do");
    let bridge = transparency_form(6, "", "", "/G Do");
    let sibling = group_form_obj(7, "", "/CS0 cs 0.8 scn 10 10 60 60 re f");
    build_page_with("/F Do", SPOT_A, "/BM /Multiply", &[&outer, &bridge, &sibling])
}

#[test]
fn a_group_can_invoke_a_sibling_through_inherited_resources() {
    use crate::rendering::page_renderer::{SpotPlaneUse, spot_plane_use};

    let doc = inherited_sibling_group_page();
    assert_eq!(
        spot_plane_use(&doc, 0, &doc.get_page_resources(0).expect("resources")),
        SpotPlaneUse {
            groups: 3,
            ..SpotPlaneUse::default()
        }
    );
    let limit = one_ink_peak_charge(BUDGET_REQUESTED_DPI, 4 * CANVAS + SCRATCH);
    assert_eq!(budgeted_dpi(&doc, limit, BUDGET_MINIMUM_DPI), Some(265));
}

#[test]
fn a_shared_form_resolves_names_in_each_inherited_scope() {
    use crate::rendering::page_renderer::{SpotPlaneUse, spot_plane_use};

    let outer = group_form_obj(5, "/XObject << /A 6 0 R /B 7 0 R >>", "/A Do /B Do");
    let a = group_form_obj(6, "/XObject << /Bridge 8 0 R /G 9 0 R >>", "/Bridge Do");
    let b = group_form_obj(7, "/XObject << /Bridge 8 0 R /G 10 0 R >>", "/Bridge Do");
    let bridge = transparency_form(8, "", "/K true", "/G Do");
    let knockout = transparency_form(9, "/Resources << >>", "/K true", "");
    let nested = group_form_obj(10, "/XObject << /Inner 11 0 R >>", "/Inner Do");
    let inner = group_form_obj(11, "", "");
    let doc = build_page_with(
        "/F Do",
        SPOT_A,
        "/BM /Multiply",
        &[&outer, &a, &b, &bridge, &knockout, &nested, &inner],
    );
    // Each scope contributes a different maximum, regardless of which is scanned first. ~keep
    assert_eq!(
        spot_plane_use(&doc, 0, &doc.get_page_resources(0).expect("resources")),
        SpotPlaneUse {
            soft_mask: false,
            groups: 5,
            knockouts: 2,
        }
    );
}

fn soft_mask_in_group_page() -> PdfDocument {
    let outer = group_form_obj(5, "/XObject << /G 6 0 R >>", "/G Do");
    let mask = group_form_obj(6, "", "0.4 g 0 0 100 100 re f");
    build_page_with(
        "/T gs /F Do",
        SPOT_A,
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R >>",
        &[&outer, &mask],
    )
}

/// A group whose stroke runs under the non-separable `/Hue` blend: the widest paint scratch.
fn stroking_group_page() -> PdfDocument {
    let group = group_form_obj(5, "", "/CS0 CS 0.8 SCN 8 w 10 10 m 70 70 l S 0.5 g 5 5 20 20 re B");
    build_page_with("/T gs /F Do", SPOT_A, "/BM /Hue", &[&group])
}

fn plain_group_page() -> PdfDocument {
    let group = group_form("", "/CS0 cs 0.8 scn 10 10 60 60 re f");
    build_page_with("/F Do", SPOT_A, "/BM /Multiply", &[&group])
}

/// Renders `doc` through the PNG budget and returns the DPI it was admitted at, or `None` when refused.
fn budgeted_dpi(doc: &PdfDocument, limit: usize, minimum_dpi: u32) -> Option<u32> {
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(BUDGET_REQUESTED_DPI).as_raw());
    renderer.force_cmyk_sidecar = true;
    let budget = PngRenderBudget {
        max_working_bytes: limit,
        minimum_dpi,
    };
    renderer
        .render_page_with_budget(0, doc, Some(budget))
        .ok()
        .map(|(_, dpi)| dpi)
}

fn soft_mask_page() -> PdfDocument {
    let mask = group_form("/CS /DeviceGray", "0.4 g 0 0 100 100 re f");
    build_page_with(
        "/T gs /CS0 cs 0.5 scn 10 10 60 60 re f",
        SPOT_A,
        "/SMask << /Type /Mask /S /Luminosity /G 5 0 R >>",
        &[&mask],
    )
}

fn knockout_group_page() -> PdfDocument {
    let group = group_form("/K true", "/CS0 cs 0.8 scn 10 10 60 60 re f");
    build_page_with("/F Do", SPOT_A, "/BM /Multiply", &[&group])
}

/// The PNG budget charges the live sidecar planes, plus one transient copy of every plane only
/// when the page declares a soft mask or a knockout group. One ink: the live charge is
/// 12 + 4 + 2 = 18 and the peak 12 + 2·(4 + 2) = 24 bytes per pixel. A limit at the live
/// charge of the requested raster lowers the DPI of such a page, the peak charge admits the
/// request, and a limit under the peak at the minimum DPI refuses the page.
#[test]
fn the_page_renderer_lowers_dpi_for_the_transient_spot_plate_copy() {
    let doc = soft_mask_page();
    let live = one_ink_charge(BUDGET_REQUESTED_DPI, 0);
    assert_eq!(
        budgeted_dpi(&doc, live, BUDGET_MINIMUM_DPI),
        Some(249),
        "the transient charge lowers 288 dpi to the largest raster the limit admits"
    );
    assert_eq!(
        budgeted_dpi(&doc, one_ink_charge(BUDGET_REQUESTED_DPI, 1), BUDGET_MINIMUM_DPI),
        Some(BUDGET_REQUESTED_DPI),
        "the peak charge of the requested raster fits exactly"
    );
    assert_eq!(
        budgeted_dpi(&doc, one_ink_charge(BUDGET_MINIMUM_DPI, 1) - 1, BUDGET_MINIMUM_DPI),
        None,
        "under the peak charge even the minimum raster is refused, not lowered"
    );
}

/// The scan names what a page's resources declare. Patterns and annotation appearance
/// resources are not walked.
#[test]
fn the_page_scan_reports_the_widest_spot_plane_use() {
    use crate::rendering::page_renderer::{SpotPlaneUse, spot_plane_use};
    let scan = |doc: &PdfDocument| spot_plane_use(doc, 0, &doc.get_page_resources(0).expect("resources"));
    let used = |soft_mask, groups, knockouts| SpotPlaneUse {
        soft_mask,
        groups,
        knockouts,
    };
    assert_eq!(scan(&knockout_group_page()), used(false, 1, 1));
    assert_eq!(scan(&soft_mask_page()), used(true, 0, 0));
    assert_eq!(scan(&plain_group_page()), used(false, 1, 0));
    assert_eq!(scan(&nested_group_page()), used(false, 2, 0));
    assert_eq!(scan(&soft_mask_in_group_page()), used(true, 2, 0));
    assert_eq!(scan(&build_page("/BM /Multiply", &[])), used(false, 0, 0));
    assert_eq!(
        scan(&build_page("/SMask /None", &[])),
        used(false, 0, 0),
        "/SMask /None clears a mask"
    );
}

/// A plain transparency group reserves its own rgba and sidecar copy at run time, so the fit
/// must lower the DPI instead of letting the render fail on the knockout workspace.
#[test]
fn an_ink_page_with_a_plain_transparency_group_lowers_dpi_instead_of_failing() {
    let doc = plain_group_page();
    assert_eq!(
        budgeted_dpi(&doc, one_ink_charge(BUDGET_REQUESTED_DPI, 0), BUDGET_MINIMUM_DPI),
        Some(175),
    );
    assert_eq!(
        budgeted_dpi(
            &doc,
            one_ink_peak_charge(BUDGET_REQUESTED_DPI, 3 * CANVAS + SCRATCH),
            BUDGET_MINIMUM_DPI
        ),
        Some(BUDGET_REQUESTED_DPI),
        "the group charge admits the request and the render fits it"
    );
}

/// A knockout group reserves a larger workspace than one copy: a limit between the copy charge
/// and the knockout charge lowers the DPI and renders.
#[test]
fn an_ink_page_with_a_knockout_group_lowers_dpi_to_the_knockout_charge() {
    let doc = knockout_group_page();
    assert_eq!(
        budgeted_dpi(
            &doc,
            one_ink_peak_charge(BUDGET_REQUESTED_DPI, 3 * CANVAS + KNOCKOUT + SCRATCH),
            BUDGET_MINIMUM_DPI
        ),
        Some(BUDGET_REQUESTED_DPI),
        "the knockout charge admits the request and the render fits it"
    );
    assert_eq!(
        budgeted_dpi(&doc, one_ink_charge(BUDGET_REQUESTED_DPI, 1), BUDGET_MINIMUM_DPI),
        Some(163),
    );
}

/// A page with an ink but no soft mask or knockout group never copies the planes: it keeps its
/// requested DPI at the live charge, and a caller whose minimum DPI equals the request admits it.
#[test]
fn an_ink_page_without_a_mask_or_knockout_keeps_its_requested_dpi() {
    let doc = build_page("/BM /Multiply", &[]);
    let live = one_ink_charge(BUDGET_REQUESTED_DPI, 0);
    assert_eq!(
        budgeted_dpi(&doc, live, BUDGET_MINIMUM_DPI),
        Some(BUDGET_REQUESTED_DPI),
        "no mask, no transient copy, no lowered DPI"
    );
    assert_eq!(
        budgeted_dpi(&doc, live, BUDGET_REQUESTED_DPI),
        Some(BUDGET_REQUESTED_DPI),
        "a strict caller (minimum DPI equals the request) admits the page"
    );
    assert_eq!(
        budgeted_dpi(&doc, live - 1, BUDGET_MINIMUM_DPI),
        Some(287),
        "control: one byte under the live charge binds"
    );
}

/// A CMYK page whose sidecar declares no spot inks (transparency without a spot plate) holds
/// nothing transient: the budget charges the CMYK plane only, so the page keeps its
/// requested DPI where a wrongly charged transient copy would lower it.
#[test]
fn a_zero_ink_sidecar_page_keeps_its_requested_dpi() {
    const REQUESTED_DPI: u32 = 288;
    const MINIMUM_DPI: u32 = 72;
    let pixels = |dpi: u32| {
        let side = (f64::from(dpi) * 100.0 / 72.0).ceil() as usize;
        side * side
    };
    // The CMYK plane only: the sidecar exists but carries no ink lanes.
    let charge = |dpi: u32| pixels(dpi) * (RGBA_AND_PNG_BYTES_PER_PIXEL + 4) + PNG_FIXED_OVERHEAD;
    let mask = transparency_form(5, "/Resources << >>", "/CS /DeviceGray", "0.4 g 0 0 100 100 re f");
    let doc = build_page_with(
        "/T gs 1 0 0 rg 10 10 60 60 re f",
        "",
        "/SMask << /Type /Mask /S /Luminosity /G 5 0 R >>",
        &[&mask],
    );
    let render = |limit: usize| {
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(REQUESTED_DPI).as_raw());
        renderer.force_cmyk_sidecar = true;
        let budget = PngRenderBudget {
            max_working_bytes: limit,
            minimum_dpi: MINIMUM_DPI,
        };
        renderer.render_page_with_budget(0, &doc, Some(budget))
    };
    let (_, dpi) = render(charge(REQUESTED_DPI)).expect("a zero-ink sidecar adds no transient charge");
    assert_eq!(dpi, REQUESTED_DPI, "the sidecar exists but holds no inks");
    let (_, dpi) = render(charge(REQUESTED_DPI) - 1).expect("control: one byte under the charge binds");
    assert_eq!(dpi, 287, "one byte under the requested raster's charge lowers the DPI");
}

/// A form XObject: `group` is the group dictionary entries, `body` its content, `/CS0` is bound.
fn group_form(group: &str, body: &str) -> String {
    format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
         /Resources << /ColorSpace << {SPOT_A} >> >> /Group << /Type /Group /S /Transparency {group} >> \
         /Length {} >>\nstream\n{body}\nendstream\nendobj\n",
        body.len()
    )
}

/// A soft mask scales the backdrop-alpha plane of a spot lane the same way as the tint plane. A
/// paint of tint 0.5 under a mask of 0.4 leaves a tint of 0.2 (51) and a backdrop alpha of 0.4
/// (102); a mask that skips the alpha plane leaves 255 there.
#[test]
fn a_soft_mask_scales_the_backdrop_alpha_plane_of_a_spot_lane() {
    let mask = group_form("/CS /DeviceGray", "0.4 g 0 0 100 100 re f");
    let doc = build_page_with(
        "/T gs /CS0 cs 0.5 scn 10 10 60 60 re f",
        SPOT_A,
        "/SMask << /Type /Mask /S /Luminosity /G 5 0 R >>",
        &[&mask],
    );
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(DPI).as_raw());
    renderer.force_cmyk_sidecar = true;
    renderer.render_page(&doc, 0).expect("render page");
    let sidecar = renderer.take_cmyk_sidecar().expect("sidecar");
    let state = sidecar.spot_state();
    let plane = 100 * 100;
    let px = ONLY_FIRST.1 * 100 + ONLY_FIRST.0;
    assert_eq!(state.len(), 2 * plane, "one ink: a tint plane then an alpha plane");
    assert_eq!(state[px], 51, "tint plane: 0.5 under a 0.4 mask");
    assert_eq!(state[plane + px], 102, "alpha plane: a 0.4 mask over a full paint");
}

/// The knockout merge carries the backdrop-alpha plane out of the group as well as the tints. A
/// Multiply over the group result at tint 0.4 gives ink 1 - 0.2 * 0.6 = 0.88 (224) only if
/// the lane is marked painted; a lost alpha plane reads the lane as empty paper and gives 102.
#[test]
fn a_knockout_group_hands_its_backdrop_alpha_plane_to_the_paints_after_it() {
    let group = group_form("/K true", "/CS0 cs 0.8 scn 10 10 60 60 re f");
    let alone = plate_of("/F Do", "/BM /Normal", &[&group]);
    assert_eq!(
        at(&alone, ONLY_FIRST.0, ONLY_FIRST.1),
        204,
        "control: the group paints tint 0.8"
    );
    let after = plate_of(
        "/F Do /CS0 cs /T gs 0.4 scn 10 10 60 60 re f",
        "/BM /Multiply",
        &[&group],
    );
    let ink = at(&after, ONLY_FIRST.0, ONLY_FIRST.1);
    assert_eq!(ink, 224, "Multiply over the group result");
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

/// A group's stroke and fill-stroke under a non-separable blend reserve the widest paint scratch.
/// A limit just above the charge of a 9-byte scratch is below the run-time peak, so the fit must lower the
/// DPI and the render succeeds.
#[test]
fn a_group_stroke_under_a_non_separable_blend_lowers_dpi_instead_of_failing() {
    let doc = stroking_group_page();
    let narrow = one_ink_peak_charge(BUDGET_REQUESTED_DPI, 3 * CANVAS + 10);
    let dpi = budgeted_dpi(&doc, narrow, BUDGET_MINIMUM_DPI).expect("the fit lowers the DPI");
    assert!(
        dpi < BUDGET_REQUESTED_DPI,
        "lowered from {BUDGET_REQUESTED_DPI}, got {dpi}"
    );
    assert_eq!(
        budgeted_dpi(
            &doc,
            one_ink_peak_charge(BUDGET_REQUESTED_DPI, 3 * CANVAS + SCRATCH),
            BUDGET_MINIMUM_DPI
        ),
        Some(BUDGET_REQUESTED_DPI)
    );
}

/// Each nested group reserves its own rgba and sidecar copy.
#[test]
fn a_group_inside_a_group_is_charged_each_level() {
    let doc = nested_group_page();
    let one_level = one_ink_peak_charge(BUDGET_REQUESTED_DPI, 3 * CANVAS + SCRATCH);
    let dpi = budgeted_dpi(&doc, one_level, BUDGET_MINIMUM_DPI).expect("the fit lowers the DPI");
    assert!(
        dpi < BUDGET_REQUESTED_DPI,
        "lowered from {BUDGET_REQUESTED_DPI}, got {dpi}"
    );
    assert_eq!(
        budgeted_dpi(
            &doc,
            one_ink_peak_charge(BUDGET_REQUESTED_DPI, 4 * CANVAS + SCRATCH),
            BUDGET_MINIMUM_DPI
        ),
        Some(BUDGET_REQUESTED_DPI)
    );
}

/// A soft mask painting inside a group adds the mask workspace, backdrop and snapshots.
#[test]
fn a_soft_mask_inside_a_group_is_charged_its_workspaces() {
    let doc = soft_mask_in_group_page();
    let without_mask = one_ink_peak_charge(BUDGET_REQUESTED_DPI, 4 * CANVAS + SCRATCH);
    let dpi = budgeted_dpi(&doc, without_mask, BUDGET_MINIMUM_DPI).expect("the fit lowers the DPI");
    assert!(
        dpi < BUDGET_REQUESTED_DPI,
        "lowered from {BUDGET_REQUESTED_DPI}, got {dpi}"
    );
    assert_eq!(
        budgeted_dpi(
            &doc,
            one_ink_peak_charge(BUDGET_REQUESTED_DPI, 4 * CANVAS + SCRATCH + SOFT_MASK_IN_GROUP),
            BUDGET_MINIMUM_DPI
        ),
        Some(BUDGET_REQUESTED_DPI)
    );
}

#[test]
fn groups_reachable_only_through_a_soft_mask_are_charged() {
    use crate::rendering::page_renderer::{SpotPlaneUse, spot_plane_use};

    let mask = group_form_obj(5, "/XObject << /G 6 0 R >>", "/G Do");
    let inner = transparency_form(6, "/Resources << >>", "/K true", "0.4 g 0 0 100 100 re f");
    let doc = build_page_with(
        "/T gs /CS0 cs 0.5 scn 10 10 60 60 re f",
        SPOT_A,
        "/SMask << /Type /Mask /S /Luminosity /G 5 0 R >>",
        &[&mask, &inner],
    );
    let resources = doc.get_page_resources(0).expect("resources");
    assert_eq!(
        spot_plane_use(&doc, 0, &resources),
        SpotPlaneUse {
            soft_mask: true,
            groups: 2,
            knockouts: 1,
        }
    );
    let without_nested_groups = one_ink_charge(BUDGET_REQUESTED_DPI, 1);
    let dpi = budgeted_dpi(&doc, without_nested_groups, BUDGET_MINIMUM_DPI).expect("the reduced DPI renders");
    assert!(dpi < BUDGET_REQUESTED_DPI, "the nested group lowers DPI, got {dpi}");
}
