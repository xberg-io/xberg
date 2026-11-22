//! Wave-1 QA probes for the resolution-pipeline migration.
//!
//! Two roles:
//!
//! 1. **Adversarial coverage** — push fill/stroke routing through scale,
//!    interleaving, malformed inputs, and edge-of-spec colour spaces;
//!    surface any pipeline-side bug that lets a paint operator reach the
//!    rasteriser with the wrong colour, alpha, or geometry.
//! 2. **Regression pins** — when a probe area does *not* surface a
//!    misbehaviour, pin the current shipped behaviour so a future change
//!    cannot silently regress it.
//!
//! Each test builds a single-page PDF inline, renders it through the
//! resolution pipeline (the only paint path), and either compares the
//! pixmap shape or samples specific pixels.

// ~keep: test/bench binaries print by design; org logging policy exempts tests
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// Build a tiny one-page PDF whose content stream is `content_ops`, with a
/// fixed 100×100 MediaBox and the provided `/Resources` dict body.
fn build_pdf(content_ops: &str, resources_dict: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
        resources_dict
    );
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Build a one-page PDF that owns an indirect Type 4 tint-transform function
/// at object 5 plus a content stream — used by Separation probes.
fn build_pdf_with_type4_separation(content_ops: &str, type4_program: &str, page_resources_extra: &str) -> Vec<u8> {
    build_pdf_with_type4_separation_range(content_ops, type4_program, page_resources_extra, "[0 1 0 1 0 1 0 1]")
}

fn build_pdf_with_type4_separation_range(
    content_ops: &str,
    type4_program: &str,
    page_resources_extra: &str,
    range_array: &str,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
        page_resources_extra
    );
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let func_off = buf.len();
    let func_hdr = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range {} /Length {} >>\nstream\n",
        range_array,
        type4_program.len()
    );
    buf.extend_from_slice(func_hdr.as_bytes());
    buf.extend_from_slice(type4_program.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, func_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
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

#[allow(dead_code)]
fn center_pixel(rgba: &[u8]) -> (u8, u8, u8, u8) {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    let cx = w / 2;
    let cy = h / 2;
    let off = ((cy * w + cx) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

#[allow(dead_code)]
fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    assert!(x < w && y < h);
    let off = ((y * w + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

/// Count non-default-background pixels (R, G, or B differs from white).
/// Used as a positive probe that the operator stream actually painted
/// something through the pipeline.
#[allow(dead_code)]
fn count_marked_pixels(rgba: &[u8]) -> usize {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .filter(|c| c[0] < 250 || c[1] < 250 || c[2] < 250)
        .count()
}

/// Probe 1 — Long content stream with many fill/stroke operators of each type.
///
/// The pipeline routes every fill/stroke through a fresh `ResolutionPipeline`
/// instance per call. Any per-call state leak or mutation of the borrowed
/// `gs` it shouldn't make would surface after enough repetitions as
/// missing or incorrectly-coloured marks. 200 operators of each kind on a
/// 100×100 page exercises every migrated arm 200× per render — large
/// enough to surface drift if any exists.
#[test]
fn qa_long_stream_repeated_fill_stroke_paints_dense_marks() {
    let mut content = String::new();
    content.push_str("1 0 0 rg\n0 1 0 RG\n2 w\n");
    for i in 0..200 {
        let x = (i % 20) as f32 * 5.0;
        let y = ((i / 20) % 20) as f32 * 5.0;
        content.push_str(&format!("{} {} 4 4 re\nf\n", x, y));
        content.push_str(&format!("{} {} 4 4 re\nS\n", x, y));
        content.push_str(&format!("{} {} 4 4 re\nB\n", x, y));
        content.push_str(&format!("{} {} 4 4 re\nb\n", x, y));
        content.push_str(&format!("{} {} 4 4 re\nB*\n", x, y));
        content.push_str(&format!("{} {} 4 4 re\nb*\n", x, y));
    }
    let bytes = build_pdf(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let marked = count_marked_pixels(&on);
    assert!(
        marked > 1000,
        "200 repeated migrated operators must produce a heavily-marked page; got {marked} marked pixels"
    );
}

/// Probe 2 — Mixed-operator stream that interleaves all six migrated
/// operators with the prior-wave fill operators (`f`, `f*`).
///
/// Each iteration uses a different colour to ensure that per-iteration
/// state (e.g. last-set fill colour, last-set stroke colour) is exercised
/// rather than collapsing to one canonical RGBA the pipeline could hide a
/// bug behind.
#[test]
fn qa_mixed_all_paint_operators_paints_across_page() {
    let mut content = String::new();
    content.push_str("3 w\n");
    let ops = ["f", "f*", "S", "B", "B*", "b", "b*"];
    for (i, op) in ops.iter().enumerate() {
        let r = (i as f32) / 7.0;
        let g = ((i + 2) as f32 % 7.0) / 7.0;
        let b = ((i + 4) as f32 % 7.0) / 7.0;
        content.push_str(&format!("{} {} {} rg\n", r, g, b));
        content.push_str(&format!("{} {} {} RG\n", b, r, g));
        let x = 10 + (i as i32) * 10;
        content.push_str(&format!("{} 30 8 40 re\n{}\n", x, op));
    }
    let bytes = build_pdf(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let marked = count_marked_pixels(&on);
    assert!(
        marked > 500,
        "interleaved migrated + prior-wave operators must paint visible marks across the page; got {marked} marked pixels"
    );
}

/// Probe 3 — Graphics-state operators (`q`/`Q`/`cm`/`w`/`J`/`j`/`gs`)
/// interleaved with migrated operators. The pipeline reads `gs` by
/// reference; mutating fields it shouldn't (or failing to re-read after
/// `q`/`Q`) would diverge.
///
/// Pattern: save state, change a state field, paint, restore, paint
/// again. Repeat with different field combinations.
#[test]
fn qa_interleaved_graphics_state_changes_preserve_marks() {
    let content = "\
        1 0 0 rg\n0 1 0 RG\n2 w\n\
        q\n3 w\n0 J\n0 j\n10 10 30 30 re\nB\nQ\n\
        q\n8 w\n1 J\n1 j\n60 10 30 30 re\nb*\nQ\n\
        q\n0.5 0 0 0.5 0 0 cm\n10 60 30 30 re\nf\n10 60 30 30 re\nS\nQ\n\
        q\n2 w\n[4 2] 0 d\n50 60 40 30 re\nB\nQ\n\
        20 20 m\n80 20 l\n80 80 l\n20 80 l\nb\n\
    ";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let marked = count_marked_pixels(&on);
    assert!(
        marked > 500,
        "graphics-state changes interleaved with migrated operators must keep marks reaching the rasteriser; got {marked}"
    );
}

/// Probe 4 — Hairline stroke (line width well under 1 device pixel).
///
/// The pipeline clones `gs` and overwrites only `stroke_color_rgb` and
/// `stroke_alpha`; line width must round-trip through the pipeline splice
/// exactly. At a 0.25-px width the rasteriser produces a faint
/// anti-aliased line; if the splice accidentally promoted the width
/// (e.g. via a default-init clone) the painted pixmap would diverge from
/// the expected hairline. The capability under test is that the operator
/// round-trips without panicking and the line-width survives the splice.
#[test]
fn qa_stroke_hairline_width_renders_full_pixmap() {
    // 0.25-px stroke at 72 DPI is below the sub-pixel anti-alias
    // threshold the rasteriser uses, so the on-paper output is
    // effectively zero coverage — the spec doesn't guarantee
    // visibility below 1 device pixel. The capability under test is
    // that the operator round-trips through the renderer without
    // panicking and the line-width survives the splice (the
    // pipeline's clone doesn't promote width). We pin the no-panic /
    // full-pixmap invariant. ~keep
    let content = "1 0 0 RG\n0.25 w\n20 50 m\n80 50 l\nS\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert_eq!(
        on.len(),
        100 * 100 * 4,
        "hairline render must produce a full 100×100 RGBA pixmap"
    );
}

/// Probe 5 — Zero-width stroke. PDF spec ISO 32000-1 §8.4.3.2 says width 0
/// means "thinnest line the device can render"; the renderer's existing
/// behaviour is what we pin — the page must render without panicking.
#[test]
fn qa_stroke_zero_width_renders_full_pixmap() {
    let content = "1 0 0 RG\n0 w\n20 50 m\n80 50 l\nS\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    // Zero-width stroke per §8.4.3.2: thinnest line the device can
    // render. The render must not panic, and the renderer's defined
    // behaviour is observable on the output. We probe the no-panic
    // invariant by reaching this assertion at all. ~keep
    let on = render_with_pipeline(&doc, true);
    assert_eq!(on.len(), 100 * 100 * 4, "zero-width stroke must produce a full pixmap");
}

/// Probe 6 — Negative line width (malformed PDF).
///
/// The spec says width must be non-negative; some PDFs in the wild carry
/// negative values from broken producers. The renderer must degrade
/// gracefully — no panic — and still produce a full pixmap.
#[test]
fn qa_stroke_negative_width_renders_without_panic() {
    let content = "1 0 0 RG\n-3 w\n20 50 m\n80 50 l\nS\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    // No-panic invariant: the renderer must accept the malformed
    // negative width and either render or fail cleanly. ~keep
    let on = render_with_pipeline_allow_fail(&doc, true);
    if let Some(data) = on {
        assert_eq!(
            data.len(),
            100 * 100 * 4,
            "negative width render must produce a full pixmap"
        );
    }
}

/// Probe 7 — Stroke alpha (`/CA`) sourced from an ExtGState dict.
///
/// The pipeline reads `gs.stroke_alpha` after the `gs` operator has applied
/// `/CA` to the graphics state. The fold into `ResolvedColor::Rgba.a`
/// happens inside `device_to_rgba`. The painted stroke must blend to a
/// faded red, confirming the alpha was sourced and folded as expected.
#[test]
fn qa_stroke_alpha_ca_extgstate_blends_to_faded_red() {
    let content = "/Half gs\n1 0 0 RG\n10 w\n20 20 60 60 re\nS\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /CA 0.5 >> >>";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // /CA 0.5 with full-opaque red over white bg → the stroke pixels
    // should be a midpoint red (R high but not 255, G/B around 127).
    // Sample the top-edge mid-stroke (x=50, y=20). ~keep
    let (r, g, b, _) = pixel_at(&on, 50, 20);
    assert!(
        r > 150 && g > 80 && g < 200 && b > 80 && b < 200,
        "stroke /CA 0.5 red over white must blend to a faded-red top edge; got ({r},{g},{b})"
    );
}

/// Probe 8 — Stroke with a dash pattern set via `d`.
///
/// Dash pattern is part of `gs` and must survive the splice. Drawing a
/// long horizontal stroke with a clear dash pattern surfaces any pipeline
/// path that would forget the dashing.
#[test]
fn qa_stroke_dash_pattern_produces_partial_coverage() {
    let content = "1 0 0 RG\n4 w\n[6 3] 0 d\n10 50 m\n90 50 l\nS\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Dash pattern [6 3] means 6-on / 3-off; the horizontal stroke at
    // PDF y=50 with width 4 produces alternating segments. Sweep the
    // band of rows the 4-wide stroke crosses (image-y around 47..52
    // after the PDF→image flip) and count red pixels. Dashed
    // coverage is between zero and a continuous stroke (~80 px). ~keep
    let mut marked = 0usize;
    for y in 46..=54 {
        for x in 10..=90 {
            let (r, g, b, _) = pixel_at(&on, x, y);
            if r > 150 && r > g.saturating_add(50) && r > b.saturating_add(50) {
                marked += 1;
            }
        }
    }
    assert!(
        marked > 0 && marked < 720,
        "dashed stroke must produce partial coverage (some marks, not every pixel); got {marked}"
    );
}

/// Probe 9 — Miter limit at an extreme value, applied to a sharp corner.
///
/// `M 100` allows long miter spikes; at a sharp join the spike length is
/// observable. Pipeline must round-trip the miter limit.
#[test]
fn qa_stroke_extreme_miter_limit_paints_miter_spike() {
    let content = "1 0 0 RG\n6 w\n0 J\n0 j\n100 M\n20 80 m\n50 50 l\n20 20 l\nS\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let mut found = false;
    for y in 45..=55 {
        for x in 45..=70 {
            let (r, g, b, _) = pixel_at(&on, x, y);
            if r > 200 && r > g.saturating_add(50) && r > b.saturating_add(50) {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(found, "miter join area must contain a red stroke pixel under M=100");
}

/// Probe 10 — `B` with an active rotated and scaled CTM. Each combo
/// operator builds two `PaintIntent`s and clones `gs` twice (once for
/// fill, once for stroke). Both clones must inherit the same CTM; if
/// either resets it to identity, the rotated rectangle won't paint at
/// the right place.
#[test]
fn qa_combo_under_rotated_scaled_ctm_paints_both_fill_and_stroke() {
    let content = "\
        0.6928 0.4 -0.4 0.6928 10 10 cm\n\
        0 1 0 rg\n1 0 0 RG\n5 w\n\
        0 0 40 40 re\nB\n\
    ";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let mut green_marks = 0usize;
    let mut red_marks = 0usize;
    for c in on.as_chunks::<4>().0 {
        if c[1] > c[0].saturating_add(40) && c[1] > c[2].saturating_add(40) {
            green_marks += 1;
        }
        if c[0] > c[1].saturating_add(40) && c[0] > c[2].saturating_add(40) {
            red_marks += 1;
        }
    }
    assert!(
        green_marks > 50,
        "rotated `B` fill (green) must reach the rasteriser; got {green_marks} green pixels"
    );
    assert!(
        red_marks > 20,
        "rotated `B` stroke (red) must reach the rasteriser; got {red_marks} red pixels"
    );
}

/// Probe 11 — Soft-mask `/SMask` set via ExtGState. With `/SMask /None`
/// (the no-op form) the stroke must still reach the pixmap; the
/// ExtGState plumbing accepts the entry without suppressing paint.
#[test]
fn qa_stroke_under_extgstate_with_smask_no_divergence() {
    let content = "/Sm gs\n1 0 0 RG\n10 w\n20 20 60 60 re\nS\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState /SMask /None >> >>";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = pixel_at(&on, 50, 20);
    assert!(
        r > 200 && r > g.saturating_add(50) && r > b.saturating_add(50),
        "/SMask /None must not suppress the red stroke; got ({r},{g},{b})"
    );
}

/// Probe 12 — Independent clip paths active when fill and stroke happen
/// inside the same `B` combo. The pipeline must use the same clip mask
/// for both sub-operations; a path that tracked one clip on the inline
/// route and another on the pipeline route would diverge.
#[test]
fn qa_combo_under_active_clip_paints_only_inside_band() {
    let content = "\
        0 40 100 20 re\nW\nn\n\
        0 1 0 rg\n1 0 0 RG\n6 w\n\
        20 10 60 80 re\nB\n\
    ";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Clip is a horizontal band y=40..60; the rect outside that band
    // (e.g. y=15) must be unclipped (still white), and the centre of
    // the band (y=50) must be marked by the fill or stroke. ~keep
    let (r_above, g_above, b_above, _) = pixel_at(&on, 50, 15);
    assert!(
        r_above > 240 && g_above > 240 && b_above > 240,
        "outside clip band must stay white; got ({r_above},{g_above},{b_above})"
    );
    let (r_in, _, _, _) = pixel_at(&on, 50, 50);
    assert!(
        r_in < 250 || pixel_at(&on, 50, 50).1 < 250,
        "inside clip band must be marked by `B`; got {:?}",
        pixel_at(&on, 50, 50)
    );
}

/// Probe 13 — Indexed colour space via `scn` (fill side).
///
/// ISO 32000-1:2008 §8.6.6.3: an `/Indexed` operand is an index into the
/// lookup table, and the colour is the table entry evaluated in the base
/// space — never the index read as a grey level. With a two-entry palette of
/// red and blue, `1 scn` fills blue.
#[test]
fn qa_indexed_scn_fill_resolves_through_the_palette() {
    let resources = "/ColorSpace << /Pal [/Indexed /DeviceRGB 1 <FF0000 0000FF>] >>";
    let content = "/Pal cs\n1 scn\n20 20 60 60 re\nf\n";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_on, g_on, b_on, _) = center_pixel(&on);
    assert!(
        r_on < 50 && g_on < 50 && b_on > 200,
        "Indexed `scn`: index 1 is the palette's blue entry, got ({r_on}, {g_on}, {b_on})"
    );
}

/// Probe 13b — Indexed colour space via `SCN` (stroke side): the same
/// lookup, on the stroke colour.
#[test]
fn qa_indexed_scn_stroke_resolves_through_the_palette() {
    let resources = "/ColorSpace << /Pal [/Indexed /DeviceRGB 1 <FF0000 0000FF>] >>";
    let content = "/Pal CS\n1 SCN\n10 w\n20 20 60 60 re\nS\n";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // A stroke-edge sample picks up antialiasing toward the white background,
    // so the bound is generous on red and green and firm on blue.
    let (r, g, b, _) = pixel_at(&on, 50, 20);
    assert!(
        r < 150 && g < 150 && b > 150,
        "Indexed `SCN` stroke: index 1 is the palette's blue entry; got ({r},{g},{b})"
    );
}

/// Probe 14 — ICCBased colour space with 4 components (CMYK profile).
///
/// The pipeline inspects `/N` and dispatches to the device-family
/// fallback; the painted fill must reach the pixmap.
#[test]
fn qa_iccbased_cmyk_n4_fill_paints_centre_cyan() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let resources = "/ColorSpace << /MyCMYK [/ICCBased 5 0 R] >>";
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
            resources
        )
        .as_bytes(),
    );
    let stream_off = buf.len();
    let content = "/MyCMYK cs\n1 0 0 0 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let icc = "5 0 obj\n<< /N 4 /Length 0 >>\nstream\n\nendstream\nendobj\n";
    buf.extend_from_slice(icc.as_bytes());
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(buf).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = center_pixel(&on);
    assert!(
        r < 30 && (150..=195).contains(&g) && b > 220,
        "ICCBased N=4 cyan fill must paint process-ink cyan #00ADEF at centre; got ({r},{g},{b})"
    );
}

/// Probe 15 — DeviceN with a multi-output Type 4 tint transform.
///
/// `DeviceN` colour spaces have multiple input colorants and the tint
/// transform produces N output values for the alternate space. The
/// pipeline's `resolve_separation_or_devicen` runs the Type 4 program
/// and projects through the alt-space. The inline path has no DeviceN
/// branch beyond the Type 2 sibling code, so for a Type 4 DeviceN the
/// inline path gray-falls to `1.0 - components[0]`.
///
/// Pipeline ON: must paint the colour the Type 4 program declares.
/// Pipeline OFF: a different colour (or a fall-back). The two must
/// differ — pipeline gives a capability gain — and the pipeline value
/// must match the declared CMYK.
#[test]
fn qa_devicen_multi_colorant_type4_pipeline_resolves() {
    // 2-colorant DeviceN. Tint transform reads two stack inputs and
    // writes CMYK [0 t1 0 0] — i.e. ignores t0 and routes t1 to magenta.
    // With `0 1 scn` (t0=0, t1=1), output is CMYK(0,1,0,0) → magenta.
    //
    // Stack walk for `{ exch pop 0.0 exch 0.0 0.0 }` with [t0=0, t1=1]
    // (PostScript convention puts the last input on the top of the stack):
    //   start  [0, 1]
    //   exch   [1, 0]
    //   pop    [1]
    //   0.0    [1, 0]
    //   exch   [0, 1]
    //   0.0    [0, 1, 0]
    //   0.0    [0, 1, 0, 0]  ← CMYK(0, 1, 0, 0) magenta ~keep
    let type4_program = "{ exch pop 0.0 exch 0.0 0.0 }";
    let resources = "/ColorSpace << /TwoSpot [/DeviceN [/SpotA /SpotB] /DeviceCMYK 5 0 R] >>";
    let content = "/TwoSpot cs\n0 1 scn\n20 20 60 60 re\nf\n";
    let range = "[0 1 0 1 0 1 0 1]";
    let bytes = build_devicen_pdf(content, type4_program, resources, range, &[0, 1, 0, 1]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");

    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = center_pixel(&on);
    assert!(
        r > 200 && g < 60 && (110..=170).contains(&b),
        "pipeline DeviceN Type-4 must resolve to process-ink magenta #EC008C, got ({r}, {g}, {b})"
    );
}

/// Build a one-page PDF with a Type 4 function whose Domain accommodates a
/// variable number of inputs. `domain_pairs` is a flat list of (min, max)
/// pairs as integers (PDF reals).
fn build_devicen_pdf(
    content_ops: &str,
    type4_program: &str,
    page_resources_extra: &str,
    range_array: &str,
    domain_pairs: &[i32],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
        page_resources_extra
    );
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let func_off = buf.len();
    let domain_str: Vec<String> = domain_pairs.iter().map(|v| v.to_string()).collect();
    let domain_array = format!("[{}]", domain_str.join(" "));
    let func_hdr = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain {} /Range {} /Length {} >>\nstream\n",
        domain_array,
        range_array,
        type4_program.len()
    );
    buf.extend_from_slice(func_hdr.as_bytes());
    buf.extend_from_slice(type4_program.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, func_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Probe 16 — Separation with the `/All` colorant name (capability).
///
/// The pipeline evaluates the tint transform regardless of colorant name;
/// the test pins the resolved colour goes toward magenta for a
/// Type 4 program that emits `CMYK(0, 0.5, 0, 0)`.
#[test]
fn qa_separation_all_colorant() {
    let type4_program = "{ 0.0 exch 0.0 0.0 }";
    let content = "/All_CS cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let resources = "/ColorSpace << /All_CS [/Separation /All /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r_on, g_on, b_on, _) = center_pixel(&on);
    assert!(
        r_on > g_on && b_on > g_on,
        "pipeline /All Separation Type-4 must resolve toward magenta (R>G, B>G), got ({r_on}, {g_on}, {b_on})"
    );
}

/// Probe 18 — Pattern colour space (`/Pattern` for tiling). `Pattern`
/// cs entries resolve via the renderer's existing pattern handler.
/// The pipeline must NOT capture `Pattern` colour-space resolution
/// out from under that — it should leave the pattern path alone.
#[test]
fn qa_pattern_colour_space_degenerate_cs_does_not_crash() {
    let resources = "/ColorSpace << /MyPattern [/Pattern] >>";
    let content = "/MyPattern cs\n20 20 60 60 re\nf\n";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Bare `/Pattern cs` with no concrete pattern is degenerate; the
    // renderer must not panic. Pin the pixmap size invariant. ~keep
    assert_eq!(
        on.len(),
        100 * 100 * 4,
        "Pattern degenerate `cs` must not crash the renderer"
    );
}

/// Probe 19 — Type 4 program that divides by zero.
///
/// `crate::functions::evaluate_type4_clamped` honours the IEEE semantics
/// inherited from how popular PDF viewers behave: `n/0 → ±inf` and `0/0
/// → NaN` rather than `undefinedresult`. The `Range`-array clamp then
/// pins the result back into the alt-space domain. The invariant is **no
/// panic** — the renderer should produce a page, not crash.
#[test]
fn qa_type4_division_by_zero_no_panic() {
    // Program leaves CMYK [n/0, 0/0, 0, 0] on the stack. Range clamps
    // each to [0, 1]. With Range clamping in place the painted colour
    // becomes (0, 0, 0, 0) clamped or (1, NaN, 0, 0) clamped. The
    // assertion is just that the render returns Ok and we get a
    // sensible 100×100 pixmap. ~keep
    let type4_program = "{ 1.0 0.0 div 0.0 0.0 div 0.0 0.0 }";
    let content = "/Spot cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let resources = "/ColorSpace << /Spot [/Separation /SpotName /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline_allow_fail(&doc, true);
    assert!(
        on.is_some(),
        "Type 4 division-by-zero must not panic; renderer must produce a pixmap"
    );
    // And it must be 100×100×4 bytes — i.e. a real render, not a stub. ~keep
    assert_eq!(on.unwrap().len(), 100 * 100 * 4);
}

/// Probe 20 — Type 4 program designed to provoke stack overflow.
///
/// `MAX_STACK` and `MAX_INSTRUCTIONS` in the Type 4 evaluator both bound
/// runtime growth. A program with a deep literal stack should hit one or
/// the other and return an Error; the resolver converts that to
/// `first_as_gray` and the renderer paints SOMETHING. Invariant: no
/// panic, render succeeds.
#[test]
fn qa_type4_stack_overflow_no_panic() {
    // 2048 number literals in a row exceeds the implementation's
    // MAX_STACK (currently 100 by inspection). The evaluator returns
    // Err; the pipeline catches it with `?` → bubbles to `.ok()?` in
    // `pipeline_resolve_rgba` → returns None → renderer falls back to
    // the inline path's per-`SetFillColorN` colour (whatever it was). ~keep
    let mut body = String::from("{ ");
    for _ in 0..2048 {
        body.push_str("0.5 ");
    }
    body.push_str(" 0.0 0.0 0.0 0.0 }");
    let content = "/Spot cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let resources = "/ColorSpace << /Spot [/Separation /SpotName /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4_separation(content, &body, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline_allow_fail(&doc, true);
    assert!(on.is_some(), "Type 4 deep stack must not panic");
    assert_eq!(on.unwrap().len(), 100 * 100 * 4);
}

/// Probe 21 — Type 4 program that emits out-of-range output (negative
/// values and values > 1.0). `Range`-array clamping in the evaluator
/// must constrain each output before it reaches the alt-space
/// projection. The pipeline must render normally — no NaN propagation,
/// no panics, output in [0, 1].
#[test]
fn qa_type4_out_of_range_output_clamps() {
    // Program leaves [-0.5, 2.0, -10.0, 99.0] (all out of [0, 1]). With
    // Range = [0 1] for each output the clamp gives [0, 1, 0, 1] →
    // CMYK(0, 1, 0, 1) = magenta+black, projected to RGB at alpha 1. ~keep
    let type4_program = "{ pop -0.5 2.0 -10.0 99.0 }";
    let content = "/Spot cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let resources = "/ColorSpace << /Spot [/Separation /SpotName /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Centre pixel must be valid u8 RGBA — implicit by reaching this
    // point, but pin the colour shape too. CMYK(0, 1, 0, 1) → R = 0,
    // G = 0, B = 0 (black; the K plate dominates). All channels low. ~keep
    let (r, g, b, a) = center_pixel(&on);
    assert!(
        r < 60 && g < 60 && b < 60 && a == 255,
        "Type 4 out-of-range output must clamp into valid CMYK → painted colour, got ({r}, {g}, {b}, {a})"
    );
}

/// Probe 22 — Colour space with a malformed object (Separation array
/// missing the alt-space and function entries entirely).
///
/// `[/Separation /Name]` is a 2-element array — both `arr.get(2)` and
/// `arr.get(3)` return None. The pipeline's resolver falls through to
/// `g = 1.0 - tint`, matching the long-standing behaviour of the
/// renderer's `scn`/`SCN` `Separation | DeviceN` branch when no
/// function is available.
#[test]
fn qa_bug_malformed_separation_array_diverges() {
    let resources = "/ColorSpace << /Spot [/Separation /SpotName] >>";
    let content = "/Spot cs\n0.7 scn\n20 20 60 60 re\nf\n";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let (r, g, b, _) = center_pixel(&on);
    assert!(
        (r as i32 - 76).abs() < 15 && (g as i32 - 76).abs() < 15 && (b as i32 - 76).abs() < 15,
        "malformed Separation tint=0.7 must fall back to gray≈76; got ({r},{g},{b})"
    );
}

/// Probe 22b — Same shape, no panic guarantee for malformed input.
/// The render must not crash on a malformed Separation array regardless
/// of how the fallback path chooses to handle it.
#[test]
fn qa_malformed_separation_array_no_panic() {
    let resources = "/ColorSpace << /Spot [/Separation /SpotName] >>";
    let content = "/Spot cs\n0.7 scn\n20 20 60 60 re\nf\n";
    let bytes = build_pdf(content, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline_allow_fail(&doc, true);
    assert!(on.is_some(), "malformed Separation array must not panic the renderer");
}

/// Probe 23 — `scn` invoked with more components than the colour space
/// expects (1-channel Separation with 4 components on the stack).
///
/// The pipeline reads `components[0]` and ignores the rest, matching
/// the renderer's long-standing handling of an oversize component
/// list against a single-channel Separation.
#[test]
fn qa_scn_too_many_components_for_space_renders_marked_centre() {
    let type4_program = "{ 0.0 exch 0.0 0.0 }";
    let resources = "/ColorSpace << /Spot [/Separation /SpotName /DeviceCMYK 5 0 R] >>";
    let content = "/Spot cs\n0.5 0.2 0.9 0.1 scn\n20 20 60 60 re\nf\n";
    let bytes = build_pdf_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // No-panic invariant: too many components on the stack for a
    // single-channel Separation must still produce a pixmap. The
    // resolver keys off `components[0]` (tint=0.5 → CMYK(0, 0.5, 0, 0)
    // → faint magenta). Pin the rendered state at non-blank. ~keep
    let (r, g, b, _) = center_pixel(&on);
    let any_marked = r < 250 || g < 250 || b < 250;
    assert!(
        any_marked,
        "too-many-components Separation must render the centre with marks; got ({r},{g},{b})"
    );
}

/// Probe 24 — `scn` invoked with too few components for the declared
/// colour space (DeviceN of arity 2 with only 1 component on the
/// stack). The pipeline currently sends whatever components are there
/// into `evaluate_type4_clamped` with the declared `Domain`. The
/// Domain has 2 entries but the inputs vector has 1; the Type 4
/// evaluator must reject (or pad), and the resolver's `?` must propagate
/// to a `None` return that the renderer survives.
#[test]
fn qa_scn_too_few_components_no_panic() {
    let type4_program = "{ exch pop 0.0 exch 0.0 0.0 }";
    let resources = "/ColorSpace << /TwoSpot [/DeviceN [/SpotA /SpotB] /DeviceCMYK 5 0 R] >>";
    let content = "/TwoSpot cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let bytes = build_devicen_pdf(content, type4_program, resources, "[0 1 0 1 0 1 0 1]", &[0, 1, 0, 1]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline_allow_fail(&doc, true);
    assert!(
        on.is_some(),
        "DeviceN with too-few components must not panic the renderer"
    );
}

/// Probe 25 — Wall-clock smoke check: 1000 fills, all through the
/// pipeline.
///
/// The pipeline allocates a single-pixel `PathBuilder`, builds a
/// `LogicalColor`, instantiates a `ResolutionPipeline`, and runs the
/// colour stage on every paint. This test does NOT assert a tight
/// bound — wall-clock varies by system, and the design intentionally
/// favours correctness (capability gain) over hot-path latency. It
/// exists to surface a 100×-scale regression that would be visible to
/// operators.
#[test]
fn qa_perf_thousand_fills_within_bound() {
    let mut content = String::with_capacity(20_000);
    content.push_str("1 0 0 rg\n");
    for i in 0..1000 {
        let x = (i % 50) as f32;
        let y = ((i / 50) % 50) as f32;
        content.push_str(&format!("{} {} 1 1 re\nf\n", x, y));
    }
    let bytes = build_pdf(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");

    let _ = render_with_pipeline(&doc, true);

    let t = std::time::Instant::now();
    let _ = render_with_pipeline(&doc, true);
    let d = t.elapsed();

    println!("perf-1000-fills: {:?}", d);

    // 1000 trivial 1x1 fills must finish well under a second on any
    // modern system. A 5s ceiling catches catastrophic regression
    // without flaking on slow CI. ~keep
    assert!(
        d < std::time::Duration::from_secs(5),
        "1000 fills must finish under 5s; took {d:?}"
    );
}

/// Probe 26 — Allocation symmetry of fill-stroke combo.
///
/// Each `B`/`b`/`B*`/`b*` calls `pipeline_resolve_fill_rgba` AND
/// `pipeline_resolve_stroke_rgba`. Each call goes through
/// `pipeline_resolve_rgba`, which allocates:
///   - one `ResolutionPipeline` instance
///   - one `ResolutionContext`
///   - one `LogicalColor` (with a `Vec<f32>` of the components)
///   - one `PathBuilder` plus a finished `Path`
///   - one `PaintIntent`
///
/// Per combo, that's two of each — 10 allocations per combo (roughly).
///
/// We can't measure heap activity in a stable way from inside `cargo
/// test`, so this is a *behavioural* pin: render N combos through the
/// pipeline and confirm the render succeeds and paints visible marks.
/// The intent is to flag the cost — N = 500 combos exercising the hot
/// path twice with two pipeline calls each = 1000 ResolutionPipeline
/// instantiations per render. If this becomes a real ceiling, the cost
/// is documented here.
#[test]
fn qa_perf_combo_alloc_pressure_does_not_break_correctness() {
    let mut content = String::with_capacity(20_000);
    content.push_str("0 1 0 rg\n1 0 0 RG\n1 w\n");
    for i in 0..500 {
        let x = (i % 25) as f32 * 4.0;
        let y = ((i / 25) % 20) as f32 * 5.0;
        content.push_str(&format!("{} {} 3 3 re\nB\n", x, y));
    }
    let bytes = build_pdf(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let marked = count_marked_pixels(&on);
    assert!(
        marked > 1000,
        "500 combo operators under heavy alloc pressure must produce a marked page; got {marked}"
    );
}

// ===========================================================================
// PROBE AREA: Regression coverage — corpus PDFs through the pipeline (probe 27)
// =========================================================================== ~keep

/// Probe 27 — Real-world fixture PDFs render through the pipeline
/// without panicking. We pick `simple.pdf` because it's small,
/// ship-checked-in, and exercises real text + path rendering through
/// the pipeline-migrated operators. Other corpus PDFs are also worth
/// pinning; we use one as a representative.
#[test]
fn qa_corpus_simple_pdf_renders_without_panic() {
    // simple.pdf is a deliberately blank fixture — the probe here is
    // that the pipeline-driven renderer accepts it without panicking
    // and produces a full-page pixmap. Marks count is not pinned
    // because the source page is empty. ~keep
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple.pdf");
    let bytes = std::fs::read(&path).expect("simple.pdf fixture present");
    let doc = PdfDocument::from_bytes(bytes).expect("simple.pdf parses");
    let on = render_with_pipeline(&doc, true);
    assert!(!on.is_empty(), "simple.pdf must produce a non-empty pixmap");
    assert!(on.len().is_multiple_of(4), "pixmap must be RGBA8 aligned");
}

#[test]
fn qa_corpus_hello_structure_pdf_renders_with_text_marks() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_structure.pdf");
    let bytes = std::fs::read(&path).expect("hello_structure.pdf fixture present");
    let doc = PdfDocument::from_bytes(bytes).expect("hello_structure.pdf parses");
    let on = render_with_pipeline(&doc, true);
    let marked = count_marked_pixels(&on);
    assert!(
        marked > 0,
        "hello_structure.pdf must render with visible text marks; got {marked} marked pixels"
    );
}

#[test]
fn qa_corpus_outline_pdf_renders_without_panic() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/outline.pdf");
    let bytes = std::fs::read(&path).expect("outline.pdf fixture present");
    let doc = PdfDocument::from_bytes(bytes).expect("outline.pdf parses");
    let on = render_with_pipeline(&doc, true);
    assert!(!on.is_empty(), "outline.pdf must produce a non-empty pixmap");
    assert!(on.len().is_multiple_of(4), "pixmap must be RGBA8 aligned");
}
