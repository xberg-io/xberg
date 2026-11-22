//! Wave-4 QA probes for the resolution-pipeline migration (shading `sh`).
//!
//! Sibling to waves 1 (paths/stroke), 2 (text), 3 (ImageMask + `Do`).
//! This wave-4 QA suite probes the shading-side corners:
//!
//! 1. **Type 2 axial edges** — non-horizontal `/Coords` (vertical,
//!    reversed, diagonal); non-default `/Domain`; explicit `/Extend`
//!    both ways.
//! 2. **Type 3 radial edges** — non-concentric circles; zero-radius
//!    inner circle (origin point); `/Extend`.
//! 3. **Colour space stress** — inline Separation/CMYK/Type-4,
//!    ICCBased, Indexed, DeviceN with multi-colorant Type-4.
//! 4. **Function-shape edges** — Type 3 stitching with 3+ sub-functions,
//!    sub-functions with different domains, Type 2 with `N != 1`.
//! 5. **State interaction** — `q ... Q`, SMask, clip path, blend mode.
//! 6. **Pass-through types** — Type 1 function-based, Type 4-7 meshes
//!    must remain unchanged because the pipeline doesn't migrate them.
//! 7. **Adversarial input** — missing `/ColorSpace`, missing `/Function`,
//!    missing `/C0` / `/C1`, empty `/Functions` array; no-panic
//!    invariant.
//! 8. **Performance** — N-shading render must hold a sane budget; one
//!    large shading should be O(pixmap size).
//!
//! Style mirrors waves 1-3: build a tiny PDF inline, render through
//! `render_with_pipeline`, compare pixmaps or sample pixels.

#![allow(dead_code)]

use std::time::Instant;
use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

// ===========================================================================
// PDF construction helpers — self-contained so a fix-pass to the
// wave-1/2/3 QA helpers can't accidentally invalidate the wave-4 invariants.
//
// All builders produce a 100×100 user-space page with a single shading
// resource named `/Sh1`. The content stream drives `/Sh1 sh` (or `q ...
// /Sh1 sh Q` for graphics-state probes). Extra resources / objects let
// individual probes declare per-page colour spaces and stand-alone
// function objects referenced from inline Separation / ICCBased arrays.
// =========================================================================== ~keep

/// Build a one-page PDF whose page resources carry a shading dict at
/// `/Sh1`. The shading dict body is the caller's responsibility — the
/// builder substitutes it verbatim into object 5. This is the most
/// flexible form: probes that need arbitrary `/Extend`, `/Domain`,
/// stitching `/Functions` arrays, or alternate `/FunctionType` values
/// build the dict text themselves.
///
/// Object numbering: 1 Catalog, 2 Pages, 3 Page, 4 Content, 5 Shading,
/// 6+ extra. `extra_objects` is `[(obj_num, body_with_obj_header)]`;
/// the obj_num must agree with the body's leading `N 0 obj`.
fn build_pdf_shading_raw(
    content_ops: &str,
    shading_body: &str,
    extra_resources: &str,
    extra_objects: &[(usize, String)],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading << /Sh1 5 0 R >> {} >> /Contents 4 0 R >>\nendobj\n",
        extra_resources
    );
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let shading_off = buf.len();
    buf.extend_from_slice(format!("5 0 obj\n{}\nendobj\n", shading_body).as_bytes());

    let mut offsets = vec![cat_off, pages_off, page_off, stream_off, shading_off];
    for (_n, body) in extra_objects {
        offsets.push(buf.len());
        buf.extend_from_slice(body.as_bytes());
    }

    let xref_off = buf.len();
    let size = offsets.len() + 1;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", size).as_bytes());
    for off in &offsets {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            size, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Convenience wrapper for a Type 2 (axial) shading with a Type 2
/// (exponential) function. Builds the shading dict from the parts.
// test helper: each parameter is a separate literal spliced into the hand-built
// shading dict; a wrapper struct would only rename the same positional list. ~keep
#[allow(clippy::too_many_arguments)]
fn build_pdf_axial_shading(
    content_ops: &str,
    space_str: &str,
    coords: &str,
    c0: &str,
    c1: &str,
    extra_shading_keys: &str,
    extra_function_keys: &str,
    extra_resources: &str,
    extra_objects: &[(usize, String)],
) -> Vec<u8> {
    let shading_body = format!(
        "<< /ShadingType 2 /ColorSpace {} /Coords {} /Domain [0 1] {} \
         /Function << /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 {} >> >>",
        space_str, coords, extra_shading_keys, c0, c1, extra_function_keys
    );
    build_pdf_shading_raw(content_ops, &shading_body, extra_resources, extra_objects)
}

/// Convenience wrapper for a Type 3 (radial) shading with a Type 2
/// (exponential) function.
// test helper: mirrors `build_pdf_axial_shading`'s positional parameter list. ~keep
#[allow(clippy::too_many_arguments)]
fn build_pdf_radial_shading(
    content_ops: &str,
    space_str: &str,
    coords_6: &str,
    c0: &str,
    c1: &str,
    extra_shading_keys: &str,
    extra_resources: &str,
    extra_objects: &[(usize, String)],
) -> Vec<u8> {
    let shading_body = format!(
        "<< /ShadingType 3 /ColorSpace {} /Coords {} /Domain [0 1] {} \
         /Function << /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> >>",
        space_str, coords_6, extra_shading_keys, c0, c1
    );
    build_pdf_shading_raw(content_ops, &shading_body, extra_resources, extra_objects)
}

/// Build a Type 4 PostScript-function object string referencing the
/// supplied program (the contents between the `{ }` braces are also
/// supplied; the helper wraps them in the brace pair).
fn type4_function_object(obj_num: usize, program: &str, range: &str) -> String {
    let stream = format!("{{ {} }}", program);
    format!(
        "{} 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range {} /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        obj_num,
        range,
        stream.len(),
        stream
    )
}

/// Build a Type 4 PostScript-function object with a multi-input domain.
fn type4_function_object_multi(obj_num: usize, program: &str, domain: &str, range: &str) -> String {
    let stream = format!("{{ {} }}", program);
    format!(
        "{} 0 obj\n<< /FunctionType 4 /Domain {} /Range {} /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        obj_num,
        domain,
        range,
        stream.len(),
        stream
    )
}

/// Render the first page. The `_enabled` argument is retained so existing
/// test bodies keep compiling after wave 5 collapsed the off/on split; the
/// pipeline is the only path now.
fn render_with_pipeline(doc: &PdfDocument, _enabled: bool) -> Vec<u8> {
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(doc, 0, &opts).expect("render_page succeeds");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    img.data
}

/// Render the first page, allowing failure without panicking. Used by
/// adversarial-input probes whose invariant is "no panic", not "render
/// succeeds".
fn render_with_pipeline_allow_fail(doc: &PdfDocument, _enabled: bool) -> Option<Vec<u8>> {
    let opts = RenderOptions::with_dpi(72).as_raw();
    render_page(doc, 0, &opts).ok().map(|img| img.data)
}

/// Sample a pixel at (x, y) on the 100×100 page.
fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let w = 100u32;
    let off = ((y * w + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

/// Sample the centre pixel of the 100×100 page.
fn center_pixel(rgba: &[u8]) -> (u8, u8, u8, u8) {
    pixel_at(rgba, 50, 50)
}

/// Count pixels with non-default (non-white) RGB in the given region.
#[allow(dead_code)]
fn count_ink_pixels(rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    let w = 100u32;
    let mut n = 0usize;
    for y in y0..y1.min(100) {
        for x in x0..x1.min(100) {
            let off = ((y * w + x) * 4) as usize;
            if rgba[off] < 250 || rgba[off + 1] < 250 || rgba[off + 2] < 250 {
                n += 1;
            }
        }
    }
    n
}

/// Probe 1 — Vertical `/Coords` (`[x0 y0 x0 y1]`). The axis runs
/// top-to-bottom rather than left-to-right. DeviceRGB short-circuits
/// at the Device-family arm of `build_logical_color` and folds the
/// same RGBA triple into the stop the renderer's `parse_color_array`
/// would have produced.
#[test]
fn qa_axial_vertical_coords_paints_top_blue_bottom_red() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[50 10 50 90]",
        "[1 0 0]",
        "[0 0 1]",
        "",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_top, g_top, b_top, _) = pixel_at(&on, 50, 15);
    let (r_bot, g_bot, b_bot, _) = pixel_at(&on, 50, 85);
    assert!(
        b_top > 200 && r_top < 60 && g_top < 60,
        "top of vertical gradient should be ~blue (C1), got ({r_top}, {g_top}, {b_top})"
    );
    assert!(
        r_bot > 200 && g_bot < 60 && b_bot < 60,
        "bottom of vertical gradient should be ~red (C0), got ({r_bot}, {g_bot}, {b_bot})"
    );
}

/// Probe 2 — Reversed `/Coords` (`[x1 y x0 y]`, i.e. coordinate-1 is
/// to the left of coordinate-0). The gradient runs right-to-left in
/// user space. The colour at the "high-x" end of the page should be
/// C0 (since C0 sits at the *first* listed point, which is at the
/// right). Declares `/Extend [true true]` so the gradient pads past
/// the axis ends with the endpoint colours — the assertions below
/// sample inside that padded region.
#[test]
fn qa_axial_reversed_coords_clamp_endpoint_colours() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[90 50 10 50]",
        "[0 1 0]",
        "[1 1 1]",
        "/Extend [true true]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_r, g_r, b_r, _) = pixel_at(&on, 95, 50);
    assert!(
        g_r > 230 && r_r < 40 && b_r < 40,
        "right edge under reversed coords must clamp to C0=green; got ({r_r}, {g_r}, {b_r})"
    );
    let (r_l, g_l, b_l, _) = pixel_at(&on, 5, 50);
    assert!(
        r_l > 230 && g_l > 230 && b_l > 230,
        "left edge under reversed coords must clamp to C1=white; got ({r_l}, {g_l}, {b_l})"
    );
}

/// Probe 3 — Non-default `/Domain` (`[-0.5 1.5]`). Per ISO 32000-1
/// §8.7.4.5.3 the gradient maps device-space t into the function's
/// domain by `x = D0 + t*(D1 - D0)`. The two endpoint colours that the
/// stops carry should be the function evaluated at the *axis*
/// endpoints (t=0 and t=1 along the geometric axis, which under
/// non-default Domain means evaluation at x=D0 and x=D1 inside the
/// function, NOT at x=0 and x=1).
///
/// The wave-4 helper reads `/C0` and `/C1` raw and hands them through
/// — it does NOT consult Domain — so for `N=1` exponential
/// interpolation the result at the axis endpoints is `C0` (for any
/// Domain that includes 0) and `C1` (for any Domain that includes 1)
/// only if Domain == [0 1].
///
/// Both `render_axial_shading` (inline) and the wave-4 helper now
/// evaluate the function at the shading's `/Domain` endpoints rather
/// than at raw 0 / 1. With `/Domain [-0.5 1.5]`, `N=1`, the geometric
/// `C0` endpoint evaluates to `(1.5, 0, -0.5)` in RGB, which falls
/// outside the `tiny_skia::Color::from_rgba` 0..1 range and lands as
/// black after the `unwrap_or(Color::BLACK)` fallback. Either way the
/// gradient does NOT paint raw `/C0` red.
#[test]
fn qa_axial_domain_other_than_unit_interval() {
    // /Domain [-0.5 1.5]. With N=1 exponential, C0=red, C1=blue, the
    // colour at the geometric C0 end of the axis evaluates the
    // function at x=-0.5: red + (-0.5)*(blue-red) = (1.5, 0, -0.5).
    // Those out-of-range RGB components are rejected by
    // `tiny_skia::Color::from_rgba` and the fallback
    // `unwrap_or(Color::BLACK)` paints the stop black. The pin: the
    // C0 endpoint must NOT paint pure red. ~keep
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[10 50 90 50]",
        "[1 0 0]",
        "[0 0 1]",
        "/Domain [-0.5 1.5]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 5, 50);
    // Spec-compliant: at /Domain[0]=-0.5 the function evaluates to
    // (1.5, 0, -0.5) which falls outside tiny-skia's normalised RGB
    // range; the stop falls back to black. Either way the C0 endpoint
    // must NOT paint pure red. ~keep
    assert!(
        r < 245 || g > 10 || b > 10,
        "with /Domain [-0.5 1.5] the C0 endpoint must evaluate the function at x=-0.5, \
         not paint raw C0 red; got ({r}, {g}, {b})"
    );
}

/// Probe 3b — Companion pin: the gradient still paints inside the
/// axis interior under a non-default `/Domain`. This is the
/// regression pin that catches a future Domain implementation that
/// stops painting altogether.
#[test]
fn qa_axial_domain_other_than_unit_interval_paints_axis_interior() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[10 50 90 50]",
        "[1 0 0]",
        "[0 0 1]",
        "/Domain [-0.5 1.5]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 50, 50);
    assert!(
        r < 250 || g < 250 || b < 250,
        "non-default /Domain shading must still paint axis interior; got ({r}, {g}, {b})"
    );
}

/// Probe 4 — Explicit `/Extend [true true]`. Per spec, when true the
/// shading extends past the geometric endpoints with the endpoint
/// colour, exactly what tiny-skia's `SpreadMode::Pad` already does.
/// So the renderer's hard-coded Pad happens to do the right thing for
/// `[true true]` — pin the visible-extension behaviour.
#[test]
fn qa_axial_extend_true_true_pads_endpoint_colours() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[40 50 60 50]",
        "[1 0 0]",
        "[0 0 1]",
        "/Extend [true true]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_l, g_l, b_l, _) = pixel_at(&on, 5, 50);
    assert!(
        r_l > 200 && g_l < 60 && b_l < 60,
        "/Extend [true true] must paint past C0 endpoint with C0 colour; got ({r_l}, {g_l}, {b_l})"
    );
    let (r_r, g_r, b_r, _) = pixel_at(&on, 95, 50);
    assert!(
        b_r > 200 && r_r < 60 && g_r < 60,
        "/Extend [true true] must paint past C1 endpoint with C1 colour; got ({r_r}, {g_r}, {b_r})"
    );
}

/// Probe 5 — Explicit `/Extend [false false]`. Per spec, when false
/// the shading must NOT paint beyond the geometric endpoints. The
/// renderer now builds an axis-perpendicular slab clip from the
/// gradient endpoints and intersects it with the inherited
/// `clip_mask`, so pixels past the axis projection retain the page
/// background instead of the SpreadMode::Pad clamp colour.
#[test]
fn qa_axial_extend_false_false_does_not_paint_past_endpoints() {
    // Axis the middle 20% of the page. With Extend [false false] the
    // pixel at x=5 (far left of axis) must stay the page background,
    // not C0 red. ~keep
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[40 50 60 50]",
        "[1 0 0]",
        "[0 0 1]",
        "/Extend [false false]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 5, 50);
    // Spec-compliant: pixels outside the axis must be the page
    // background. ~keep
    assert!(
        r > 230 && g > 230 && b > 230,
        "with /Extend [false false], pixels outside the axis must be page background; \
         got ({r}, {g}, {b})"
    );
}

/// Probe 5b — Companion positive probe: `/Extend [false false]` must
/// paint the axis itself (interior pixels along the gradient axis are
/// covered by the SpreadMode::Pad ramp) — the no-extension clip only
/// covers pixels OUTSIDE the axis projection. The earlier probe pins
/// the outside-axis page-background behaviour; this one pins the
/// inside-axis still-painted invariant.
#[test]
fn qa_axial_extend_false_false_paints_axis_interior() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[40 50 60 50]",
        "[1 0 0]",
        "[0 0 1]",
        "/Extend [false false]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 50, 50);
    assert!(
        r < 250 || g < 250 || b < 250,
        "/Extend [false false] must still paint the axis interior; got ({r}, {g}, {b})"
    );
}

// ===========================================================================
// Probes 6-8 — Type 3 radial edges.
//
// `render_radial_shading` parses `/Coords [x0 y0 r0 x1 y1 r1]` but
// **discards** `x0`, `y0`, and `r0` entirely — the start of the radial
// is hard-coded to centre `(x1, y1)` with radius 0. This pre-existing
// bug means non-concentric circles can't be rendered correctly and
// non-zero `r0` is silently dropped. The wave-4 splice doesn't touch
// this geometry — it only changes the two stop colours — but the QA
// brief explicitly asks for these probes so we pin the current
// behaviour and document the bug for a follow-up.
// =========================================================================== ~keep

/// Probe 6 — Non-concentric circles. `/Coords [x0 y0 r0 x1 y1 r1]`
/// where (x0, y0) != (x1, y1). The PDF spec (ISO 32000-1 §8.7.4.5.4)
/// defines the gradient as a family of circles interpolating between
/// `(x0, y0, r0)` and `(x1, y1, r1)`. `render_radial_shading` now
/// honours all six coordinates, so the inner-circle centre carries
/// C0 and the outer-circle centre carries C1. Declares `/Extend
/// [true true]` so the gradient paints inside the inner disk and
/// outside the outer circle — the assertion samples at the inner
/// disk's centre, which the default `/Extend [false false]` would
/// clip away.
#[test]
fn qa_radial_non_concentric_circles_uses_x0_y0() {
    // Inner circle at (20, 50) r=5; outer at (80, 50) r=30. A
    // spec-compliant renderer paints C0 red around (20, 50) and C1
    // white around (80, 50). With /Extend [true true] the gradient
    // also pads inside the inner disk with C0 colour and outside the
    // outer circle with C1. ~keep
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_radial_shading(
        content,
        "/DeviceRGB",
        "[20 50 5 80 50 30]",
        "[1 0 0]",
        "[1 1 1]",
        "/Extend [true true]",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 20, 50);
    assert!(
        r > 200 && g < 60 && b < 60,
        "non-concentric radial: pixel at the inner-circle centre (20, 50) should be ~C0 red; \
         got ({r}, {g}, {b}) — renderer is dropping x0/y0"
    );
}

/// Probe 6b — Companion non-blank pin: the non-concentric radial must
/// produce ink across the page (the gradient renders even when r0=5).
#[test]
fn qa_radial_non_concentric_circles_renders_pixmap() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_radial_shading(
        content,
        "/DeviceRGB",
        "[20 50 5 80 50 30]",
        "[1 0 0]",
        "[1 1 1]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert_eq!(
        on.len(),
        100 * 100 * 4,
        "non-concentric radial must produce a full pixmap"
    );
}

/// Probe 7 — One zero-radius circle (origin point). `/Coords
/// [x0 y0 0 x1 y1 r1]` defines a gradient growing from a point at
/// (x0, y0) outward to a circle at (x1, y1) of radius r1. This is
/// the most common radial-shading shape in real PDFs (highlight
/// gradients, spotlight effects). `render_radial_shading` now honours
/// the inner-circle origin, so the C0 endpoint lands at (x0, y0).
/// Declares `/Extend [true true]` so the gradient pads outside the
/// outer circle with C1 — the assertion uses a point near the outer
/// centre which under the default `[false false]` would clip away.
#[test]
fn qa_radial_zero_radius_inner_at_distinct_point() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_radial_shading(
        content,
        "/DeviceRGB",
        "[30 30 0 60 60 40]",
        "[1 0 0]",
        "[1 1 1]",
        "/Extend [true true]",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_apex, g_apex, b_apex, _) = pixel_at(&on, 31, 69);
    let (r_far, g_far, b_far, _) = pixel_at(&on, 60, 40);
    assert!(
        r_apex > g_apex && r_apex > b_apex,
        "near the inner point the C0 (red) channel must dominate; \
         got ({r_apex}, {g_apex}, {b_apex})"
    );
    assert!(
        r_far > 230 && g_far > 230 && b_far > 230,
        "near the outer-circle centre the C1 (white) endpoint must paint, \
         not the (x0, y0)-collapsed C0; got ({r_far}, {g_far}, {b_far})"
    );
    assert!(
        b_apex < b_far,
        "the apex side must be more C0-tinted (less blue) than the C1 side; \
         apex=({r_apex}, {g_apex}, {b_apex}) far=({r_far}, {g_far}, {b_far})"
    );
}

/// Probe 7b — Companion paint pin for the zero-radius inner point:
/// the renderer accepts the inner-zero-radius geometry without
/// panicking and produces a full pixmap.
#[test]
fn qa_radial_zero_radius_inner_at_distinct_point_renders_pixmap() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_radial_shading(
        content,
        "/DeviceRGB",
        "[30 30 0 60 60 40]",
        "[1 0 0]",
        "[1 1 1]",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert_eq!(on.len(), 100 * 100 * 4, "zero-r0 radial must produce a full pixmap");
}

/// Probe 8 — `/Extend [true true]`. The end-circle radius is small
/// (r=20), centred at (50, 50). With Extend [true true] pixels
/// outside the larger circle should be C1, pixels inside the
/// degenerate inner point should be C0. The current renderer uses
/// `SpreadMode::Pad` which happens to match the spec here — pin the
/// visible behaviour.
#[test]
fn qa_radial_extend_true_true_pads_endpoint_colours() {
    let content = "/Sh1 sh\n";
    let bytes = build_pdf_radial_shading(
        content,
        "/DeviceRGB",
        "[50 50 0 50 50 20]",
        "[1 0 0]",
        "[0 0 1]",
        "/Extend [true true]",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_c, g_c, b_c, _) = pixel_at(&on, 50, 50);
    assert!(
        r_c > 200 && g_c < 80 && b_c < 80,
        "radial centre should be ~C0 red; got ({r_c}, {g_c}, {b_c})"
    );
    let (r_e, g_e, b_e, _) = pixel_at(&on, 5, 5);
    assert!(
        b_e > 200 && r_e < 80 && g_e < 80,
        "/Extend [true true] should fill outside radius with C1 blue; got ({r_e}, {g_e}, {b_e})"
    );
}

/// Probe 9 — Inline `/ColorSpace [/Separation /Magenta /DeviceCMYK
/// <Type4>]` with `/C0 [1]`. The pipeline routes through
/// `pipeline_resolve_components` and evaluates the Type-4 tint
/// transform, so the C0 endpoint paints magenta. The legacy
/// `parse_color_array` path would have read `/C0 [1]` as `(1, 1, 1)`
/// RGB via the 1-element grayscale fallback and painted white.
#[test]
fn qa_inline_separation_devicecmyk_type4_capability_pin() {
    let type4 = "0.0 exch 0.0 0.0";
    let func_obj = type4_function_object(6, type4, "[0 1 0 1 0 1 0 1]");
    let bytes = build_pdf_axial_shading(
        "/Sh1 sh\n",
        "[/Separation /MagentaSpot /DeviceCMYK 6 0 R]",
        "[0 50 100 50]",
        "[1]",
        "[1]",
        "",
        "",
        "",
        &[(6, func_obj)],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_on, g_on, b_on, _) = pixel_at(&on, 5, 50);
    assert!(
        r_on >= 225 && g_on <= 5 && (125..=155).contains(&b_on),
        "pipeline path must paint Type-4 process-ink magenta at C0 end; got ({r_on}, {g_on}, {b_on})"
    );
}

/// Probe 10 — Shading with `/ColorSpace [/ICCBased <stream ref>]`,
/// `/N 4` (CMYK-ish ICC profile), and `/C0 [0 1 0 0]` (magenta in
/// CMYK). The pipeline routes the components through the ICCBased
/// branch which dispatches on `/N`: `N=4` → `four_as_cmyk` →
/// CMYK(0, 1, 0, 0) → RGB magenta (1, 0, 1). The legacy
/// `parse_color_array` would have read only the first three components
/// (`(0, 1, 0)` → green); this is the capability gain.
#[test]
fn qa_iccbased_n4_cmyk_endpoint_pipeline_corrects_inline_truncation() {
    let icc_stream = "6 0 obj\n<< /N 4 /Length 0 >>\nstream\n\nendstream\nendobj\n";
    let space_str = "[/ICCBased 6 0 R]";
    let bytes = build_pdf_axial_shading(
        "/Sh1 sh\n",
        space_str,
        "[0 50 100 50]",
        "[0 1 0 0]",
        "[0 1 0 0]",
        "",
        "",
        "",
        &[(6, icc_stream.to_string())],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);

    let (r_on, g_on, b_on, _) = center_pixel(&on);
    assert!(
        r_on > 225 && g_on < 20 && (125..=155).contains(&b_on),
        "pipeline must convert ICCBased N=4 /C0 [0 1 0 0] through CMYK->RGB to process-ink magenta; \
         got ({r_on}, {g_on}, {b_on})"
    );
}

/// Probe — an `/Indexed` shading endpoint resolves through the palette.
///
/// §8.6.6.3: an Indexed colour operand is an index into the lookup table,
/// and the colour is that entry evaluated in the base space. With entry 200
/// set to a distinct RGB triple, a uniform axial shading whose both endpoints
/// are index 200 paints exactly that triple — not the index read as a grey
/// level, and not black.
#[test]
fn qa_indexed_endpoint_resolves_through_the_palette() {
    let lookup_stream = {
        // Entry 200 is (24, 100, 120): bytes under 0x80, since the fixture
        // writer carries the stream as a `String`. ~keep
        let mut body = vec![0u8; 768];
        body[600..603].copy_from_slice(&[24, 100, 120]);
        let header = format!("6 0 obj\n<< /Length {} >>\nstream\n", body.len());
        let mut s = header.into_bytes();
        s.extend_from_slice(&body);
        s.extend_from_slice(b"\nendstream\nendobj\n");
        String::from_utf8(s).unwrap()
    };
    let space_str = "[/Indexed /DeviceRGB 255 6 0 R]";
    let bytes = build_pdf_axial_shading(
        "/Sh1 sh\n",
        space_str,
        "[0 50 100 50]",
        "[200]",
        "[200]",
        "",
        "",
        "",
        &[(6, lookup_stream)],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_on, g_on, b_on, a_on) = center_pixel(&on);
    assert!(
        (r_on as i32 - 24).abs() <= 3
            && (g_on as i32 - 100).abs() <= 3
            && (b_on as i32 - 120).abs() <= 3
            && a_on == 255,
        "Indexed endpoint must be palette entry 200 = (24, 100, 120); got ({r_on}, {g_on}, {b_on}, {a_on})"
    );
}

/// Probe 12 — Shading with `/ColorSpace [/DeviceN [/SpotA /SpotB]
/// /DeviceCMYK <Type4 multi-input>]`. Two colorants; the Type-4 tint
/// transform takes two inputs and emits four CMYK outputs. `/C0
/// [1 0]` (full SpotA, no SpotB) — the program "exch pop 0 1 0 0"
/// maps that to CMYK(0, 1, 0, 0) magenta. The pipeline routes through
/// `resolve_separation_or_devicen` → Type-4 evaluator → CMYK→RGB
/// magenta. The legacy `parse_color_array` would have read `[1 0]` as
/// a 2-element array and fallen to the `(0, 0, 0)` else branch
/// (black).
#[test]
fn qa_devicen_two_colorant_type4_capability_pin() {
    let program = "pop pop 0 1 0 0";
    let func_obj = type4_function_object_multi(6, program, "[0 1 0 1]", "[0 1 0 1 0 1 0 1]");
    let space_str = "[/DeviceN [/SpotA /SpotB] /DeviceCMYK 6 0 R]";
    let bytes = build_pdf_axial_shading(
        "/Sh1 sh\n",
        space_str,
        "[0 50 100 50]",
        "[1 0]",
        "[1 0]",
        "",
        "",
        "",
        &[(6, func_obj)],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);

    let (r_on, g_on, b_on, _) = center_pixel(&on);
    assert!(
        r_on > 225 && g_on < 20 && (125..=155).contains(&b_on),
        "pipeline DeviceN/CMYK/Type-4 must produce process-ink magenta; got ({r_on}, {g_on}, {b_on})"
    );
}

/// Build a stitching-function shading from N Type-2 sub-functions
/// using equal `/Bounds`. Helper returns the full shading-dict body
/// for `build_pdf_shading_raw`.
fn type3_stitching_body(space_str: &str, coords: &str, sub_c0_c1: &[(&str, &str)], extra_shading_keys: &str) -> String {
    let n = sub_c0_c1.len();
    let mut funcs = String::new();
    for (c0, c1) in sub_c0_c1 {
        funcs.push_str(&format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> ",
            c0, c1
        ));
    }
    let bounds: String = (1..n)
        .map(|i| format!("{:.4}", i as f32 / n as f32))
        .collect::<Vec<_>>()
        .join(" ");
    let encode: String = (0..n).map(|_| "0 1").collect::<Vec<_>>().join(" ");
    format!(
        "<< /ShadingType 2 /ColorSpace {} /Coords {} /Domain [0 1] {} \
         /Function << /FunctionType 3 /Domain [0 1] /Functions [{}] \
         /Bounds [{}] /Encode [{}] >> >>",
        space_str, coords, extra_shading_keys, funcs, bounds, encode
    )
}

/// Probe 13 — Stitching with 3 sub-functions. Sub 0: red→green;
/// sub 1: green→blue; sub 2: blue→yellow. Gradient C0 (at t=0) MUST
/// be sub[0]./C0 = red. Gradient C1 (at t=1) MUST be sub[2]./C1 =
/// yellow. A regression that picked sub[1]./C0 (green) or sub[1]./C1
/// (blue) as the endpoint would show up here.
#[test]
fn qa_type3_stitching_three_subfunctions_uses_first_c0_and_last_c1() {
    let body = type3_stitching_body(
        "/DeviceRGB",
        "[0 50 100 50]",
        &[("[1 0 0]", "[0 1 0]"), ("[0 1 0]", "[0 0 1]"), ("[0 0 1]", "[1 1 0]")],
        "",
    );
    let bytes = build_pdf_shading_raw("/Sh1 sh\n", &body, "", &[]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_l, g_l, b_l, _) = pixel_at(&on, 2, 50);
    assert!(
        r_l > 230 && g_l < 50 && b_l < 50,
        "stitching C0 stop must be sub[0]./C0 red; got ({r_l}, {g_l}, {b_l})"
    );
    let (r_r, g_r, b_r, _) = pixel_at(&on, 98, 50);
    assert!(
        r_r > 220 && g_r > 220 && b_r < 60,
        "stitching C1 stop must be sub[last]./C1 yellow; got ({r_r}, {g_r}, {b_r})"
    );
}

/// Probe 14 — Stitching where sub-functions have non-default Domain.
/// Per PDF spec, the sub-function's own /Domain affects its input
/// mapping, NOT the C0/C1 values it produces — C0/C1 are always the
/// outputs at the sub-function's domain endpoints. So reading
/// `first.C0` for the gradient at t=0 still gives the correct value
/// even when the sub-function's /Domain is something exotic.
#[test]
fn qa_type3_stitching_subfunction_domains_dont_perturb_endpoint() {
    let funcs = "<< /FunctionType 2 /Domain [-2 5] /C0 [1 0 0] /C1 [0 1 0] /N 1 >> \
                 << /FunctionType 2 /Domain [-2 5] /C0 [0 1 0] /C1 [0 0 1] /N 1 >>";
    let body = format!(
        "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] /Domain [0 1] \
         /Function << /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [0.5] /Encode [0 1 0 1] >> >>",
        funcs
    );
    let bytes = build_pdf_shading_raw("/Sh1 sh\n", &body, "", &[]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_l, g_l, b_l, _) = pixel_at(&on, 2, 50);
    assert!(
        r_l > 230 && g_l < 50 && b_l < 50,
        "stitching sub[0]./C0 must still be the geometric C0; got ({r_l}, {g_l}, {b_l})"
    );
    let (r_r, g_r, b_r, _) = pixel_at(&on, 98, 50);
    assert!(
        b_r > 230 && r_r < 50 && g_r < 50,
        "stitching sub[last]./C1 must still be the geometric C1; got ({r_r}, {g_r}, {b_r})"
    );
}

/// Probe 15 — Type 2 with `N != 1`. PDF §7.10.3: the exponent affects
/// the interpolation curve, not the endpoint values. `f(0) = C0` and
/// `f(1) = C1` for any N > 0. The wave-4 helper reads `C0` and `C1`
/// without consulting `N`; pin that the endpoints are correct
/// regardless of N.
#[test]
fn qa_type2_n_not_one_endpoint_extraction_unchanged() {
    let shading_body = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] /Domain [0 1] \
         /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 2 >> >>";
    let bytes = build_pdf_shading_raw("/Sh1 sh\n", shading_body, "", &[]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_l, g_l, b_l, _) = pixel_at(&on, 2, 50);
    assert!(
        r_l > 230 && g_l < 50 && b_l < 50,
        "Type-2 N=2 must still paint C0 red at the axis t=0 endpoint; \
         got ({r_l}, {g_l}, {b_l})"
    );
    let (r_r, g_r, b_r, _) = pixel_at(&on, 98, 50);
    assert!(
        b_r > 230 && r_r < 50 && g_r < 50,
        "Type-2 N=2 must still paint C1 blue at the axis t=1 endpoint; \
         got ({r_r}, {g_r}, {b_r})"
    );
}

/// Probe 16 — Type 4 PostScript function as the SHADING'S OWN
/// /Function (NOT as a Separation tint transform). The helper's match
/// arm explicitly rejects FunctionType 0 and 4 used as the shading
/// function (they produce colours at intermediate domain points, not
/// at fixed /C0 / /C1 arrays). The caller falls back to the legacy
/// gradient path which also doesn't handle Type 4 here — both code
/// paths render the default (0, 0, 0) → (1, 1, 1) endpoints. Pin
/// no-panic and a full pixmap.
#[test]
fn qa_type4_as_shading_function_helper_returns_none_falls_back() {
    let program = "{ 1.0 exch 0.0 0.0 }";
    let func_body = format!(
        "<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream",
        program.len(),
        program
    );
    let shading_body = format!(
        "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] /Domain [0 1] \
         /Function {} >>",
        func_body
    );
    let bytes = build_pdf_shading_raw("/Sh1 sh\n", &shading_body, "", &[]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on =
        render_with_pipeline_allow_fail(&doc, true).expect("Type-4-as-shading-function must not panic the renderer");
    // The helper returns None for Type 4 as shading function; the
    // caller's inline fallback paints whatever Type-4 program emits.
    // Pin the no-panic + full pixmap invariant. ~keep
    assert_eq!(on.len(), 100 * 100 * 4, "renderer must produce a full pixmap");
}

/// Probe 17 — Shading drawn inside `q ... Q`. The save/restore around
/// the shading call must not perturb the splice; the gradient must
/// still paint its axis interior after the surrounding state is
/// restored.
#[test]
fn qa_shading_inside_q_q_paints_axis_interior() {
    // `q 0.8 g /Sh1 sh Q` — the inner `0.8 g` would leak fill state
    // forward if Q didn't restore. A regression in the splice that
    // perturbed the gs stack (e.g. mutated `gs` instead of the
    // synthetic clone) would surface as a pixel delta between the
    // two render calls. ~keep
    let content = "q\n0.8 g\n/Sh1 sh\nQ\n";
    let bytes = build_pdf_axial_shading(
        content,
        "/DeviceRGB",
        "[0 50 100 50]",
        "[1 0 0]",
        "[0 0 1]",
        "",
        "",
        "",
        &[],
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 50, 50);
    assert!(
        r < 250 || g < 250 || b < 250,
        "shading inside q/Q must paint the axis centre; got ({r}, {g}, {b})"
    );
}

// Split for file-too-long (#1567): the module continues below via `include!`,
// not a sibling tests/*_part2.rs file -- Cargo auto-discovers every *.rs file
// directly under tests/ as its own test binary, so a sibling would fail to
// compile on its own and, if it somehow did, run every #[test] in it twice.
// A subdirectory is not auto-discovered. ~keep
include!("test_render_resolution_pipeline_qa_wave4/part2.rs");
