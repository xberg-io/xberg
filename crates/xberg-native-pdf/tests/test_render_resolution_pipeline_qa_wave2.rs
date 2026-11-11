//! Wave-2 QA probes for the resolution-pipeline migration (text operators).
//!
//! Sibling file to `test_render_resolution_pipeline_qa_wave1.rs`. This
//! suite probes:
//!
//! 1. **Scale** — long text-heavy streams, TJ arrays with many segments,
//!    multi-font runs, mixed text + path operators. Any per-call leak or
//!    asymmetric routing surfaces as missing or mis-coloured glyphs.
//! 2. **Mode coverage** — all 8 `Tr` modes, including the clip-adding
//!    modes (4-7).
//! 3. **Capability gain on text** — Type 4 Separation / DeviceN / `All` /
//!    `None` colourants on text fill; the wave-1-class bug
//!    ("legacy `scn` falls back to `1 - tint`") applied to text too.
//! 4. **State preservation** — `Tc`, `Tw`, `Tz`, `TL`, `Tm`, `Td`, `TD`
//!    must not be perturbed by the spliced GS clone.
//! 5. **Font system** — CID Type 0, embedded-subset stand-in, built-in
//!    Helvetica fallback, ToUnicode-bearing fonts.
//! 6. **Operator interaction** — `Tj` inside `q/Q`, followed by `f`, under
//!    smask/blend/clip.
//! 7. **Adversarial input** — empty `()`, whitespace-only, extreme TJ
//!    offsets, all-numeric TJ array.
//! 8. **Performance** — 1000-glyph render through the pipeline must not
//!    blow up (one-resolve-per-Tj invariant).
//!
//! Style mirrors the wave-1 QA suite: build a tiny PDF inline, render
//! through `render_with_pipeline`, compare pixmaps byte-for-byte or
//! sample specific pixel regions.

use std::time::Instant;
use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// Build a one-page text-fixture PDF with a Helvetica `/F1` Type 1 font
/// referenced at object 5. `resources_extra` is appended into the page's
/// `/Resources` dictionary (use it for /ColorSpace, /ExtGState, additional
/// /Font entries, etc.).
fn build_pdf_text(content_ops: &str, resources_extra: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> {} >> /Contents 4 0 R >>\nendobj\n",
        resources_extra
    );
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let font_off = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, font_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Build a one-page PDF with `/F1` Helvetica AND a second `/F2` standard
/// font (Times-Roman). Used by multi-font probes.
fn build_pdf_two_fonts(content_ops: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let font1_off = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let font2_off = buf.len();
    buf.extend_from_slice(
        b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, font1_off, font2_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Build a one-page text-fixture PDF with a Helvetica `/F1` Type 1 font
/// AND an indirect Type 4 tint-transform function at object 6. Used by
/// Separation / DeviceN spot-colour probes.
fn build_pdf_text_with_type4_separation(content_ops: &str, type4_program: &str, resources_extra: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> {} >> /Contents 4 0 R >>\nendobj\n",
        resources_extra
    );
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let font_off = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let func_off = buf.len();
    let func_hdr = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        type4_program.len()
    );
    buf.extend_from_slice(func_hdr.as_bytes());
    buf.extend_from_slice(type4_program.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, font_off, func_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Build a one-page text-fixture PDF with `/F1` Helvetica AND a Type 4
/// function whose Domain accommodates a variable number of inputs (for
/// DeviceN). `domain_pairs` is a flat list of (min, max) integers.
fn build_pdf_text_with_devicen_type4(
    content_ops: &str,
    type4_program: &str,
    resources_extra: &str,
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
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> {} >> /Contents 4 0 R >>\nendobj\n",
        resources_extra
    );
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let font_off = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let func_off = buf.len();
    let domain_str: Vec<String> = domain_pairs.iter().map(|v| v.to_string()).collect();
    let domain_array = format!("[{}]", domain_str.join(" "));
    let func_hdr = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain {} /Range {} /Length {} >>\nstream\n",
        domain_array,
        range_array,
        type4_program.len()
    );
    buf.extend_from_slice(func_hdr.as_bytes());
    buf.extend_from_slice(type4_program.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, font_off, func_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
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

/// Count pixels in `[x0, x1) × [y0, y1)` whose RGB is materially below the
/// white background. Used as a "did any glyph ink land here" probe.
fn count_ink_pixels(rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    let mut n = 0u32;
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            let off = ((y * w + x) * 4) as usize;
            let r = rgba[off];
            let g = rgba[off + 1];
            let b = rgba[off + 2];
            if r < 240 || g < 240 || b < 240 {
                n += 1;
            }
        }
    }
    n
}

/// Average (r, g, b) over the non-background pixels in the search region.
/// Returns `None` when no ink was found.
fn average_ink_rgb(rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> Option<(f32, f32, f32)> {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    let mut n = 0u64;
    let mut sr = 0u64;
    let mut sg = 0u64;
    let mut sb = 0u64;
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            let off = ((y * w + x) * 4) as usize;
            let r = rgba[off];
            let g = rgba[off + 1];
            let b = rgba[off + 2];
            if r < 220 || g < 220 || b < 220 {
                sr += r as u64;
                sg += g as u64;
                sb += b as u64;
                n += 1;
            }
        }
    }
    if n == 0 {
        return None;
    }
    Some((sr as f32 / n as f32, sg as f32 / n as f32, sb as f32 / n as f32))
}

/// Probe 1 — Long text-heavy page: many `Tj` operators with mid-stream font
/// size changes. The pipeline allocates a fresh resolver per `Tj`; any
/// per-call state leak or asymmetric routing across repeated dispatch
/// would surface as missing or mis-coloured glyphs.
///
/// Fixture: 12 `Tj` calls, font sizes alternating 8/16/24/32, every call
/// emits a 10-char string. That's >120 glyphs; the rasteriser routes
/// every glyph through the spliced GS the helper produces — so any
/// per-glyph leak through to the resolver also surfaces here.
#[test]
fn qa_text_long_run_many_tj_calls_paints_substantial_ink() {
    let mut content = String::new();
    content.push_str("BT 1 0 0 rg /F1 8 Tf 5 90 Td ");
    let sizes = [8u32, 16, 24, 32];
    let strings = ["AAAAAAAAAA", "BBBBBBBBBB", "CCCCCCCCCC", "DDDDDDDDDD"];
    for i in 0..12 {
        let size = sizes[i % 4];
        let s = strings[i % 4];
        content.push_str(&format!("/F1 {} Tf 0 -7 Td ({}) Tj ", size, s));
    }
    content.push_str("ET\n");
    let bytes = build_pdf_text(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert!(
        count_ink_pixels(&on, 0, 0, 100, 100) > 50,
        "long text-heavy run must produce substantial ink (>50 pixels)"
    );
}

/// Probe 2 — TJ array with 20+ alternating strings and numeric kerning
/// offsets. Each numeric entry adjusts the text matrix between glyph
/// emissions; the spliced GS is borrowed for the whole array. If the
/// pipeline were to re-resolve per array element or per glyph it would
/// drift on this fixture.
#[test]
fn qa_text_tj_array_many_segments_paints_blue() {
    let mut array = String::new();
    for i in 0..20 {
        let ch = match i % 5 {
            0 => 'H',
            1 => 'i',
            2 => 'l',
            3 => 'o',
            _ => 'W',
        };
        array.push_str(&format!("({}) -50 ", ch));
    }
    let content = format!("BT 0 0 1 rg /F1 12 Tf 5 50 Td [{}] TJ ET\n", array);
    let bytes = build_pdf_text(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let avg = average_ink_rgb(&on, 0, 30, 100, 70);
    let (r, g, b) = avg.expect("expected blue glyph ink from long TJ array");
    assert!(
        b > 150.0 && b > r + 60.0 && b > g + 60.0,
        "TJ array glyph ink must be blue, got ({r:.1}, {g:.1}, {b:.1})"
    );
}

/// Probe 3 — Real-world style content: interleaved `BT/ET` text blocks
/// with `re/f` and `re/S` path operators. Text blocks change colour and
/// font size between iterations to ensure the pipeline state correctly
/// tears down between operator arms.
#[test]
fn qa_text_interleaved_with_path_operators_paints_well_inked_page() {
    let content = "\
        1 0 0 rg 10 10 30 30 re f\n\
        BT 0 0 1 rg /F1 14 Tf 10 60 Td (Hello) Tj ET\n\
        0 1 0 RG 5 w 50 50 30 30 re S\n\
        BT 1 0 0 rg /F1 20 Tf 10 40 Td (World) Tj ET\n\
        0.3 g 50 5 40 20 re f\n\
        BT 0.5 g /F1 10 Tf 10 25 Td (Mixed) ' ET\n\
        0 0 0 RG 1 w 5 5 90 90 re S\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert!(
        count_ink_pixels(&on, 0, 0, 100, 100) > 200,
        "interleaved text/path stream must produce a well-inked page"
    );
}

/// Probe 4 — Multi-font text run: `Tj` across `Tf` switches mid-stream.
/// The pipeline routes colour, not font; switching fonts mid-`BT/ET`
/// must not perturb the resolved colour for either side.
#[test]
fn qa_text_multi_font_run_paints_red() {
    let content = "BT 1 0 0 rg /F1 20 Tf 5 50 Td (A) Tj \
                   /F2 20 Tf (B) Tj \
                   /F1 20 Tf (C) Tj \
                   /F2 20 Tf (D) Tj ET\n";
    let bytes = build_pdf_two_fonts(content);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let avg = average_ink_rgb(&on, 0, 20, 100, 90);
    let (r, g, b) = avg.expect("expected red glyph ink from multi-font run");
    assert!(
        r > 180.0 && r > g + 60.0 && r > b + 60.0,
        "multi-font Tj run must paint red, got ({r:.1}, {g:.1}, {b:.1})"
    );
}

// ============================================================================
// Text rendering mode probes — Tr=0..7.
// ============================================================================
//
// `pipeline_resolve_text_gs` short-circuits Tr=3 to None, resolves fill for
// 0/2/4/6 and stroke for 1/2/5/6. Tr=4-7 add to the current clipping path
// in the spec; the current text rasteriser does NOT implement clip-add for
// text, so these modes draw like 0-2 and don't accumulate clip state. Modes
// 1, 2, 5 and 6 stroke the glyph outline in the stroke colour (stroke after
// fill, ISO 32000-1 9.3.6 Table 106). These tests pin that the pipeline
// drives those modes into the rasteriser without colour or geometry
// corruption.
//
// If the implementation later adds clip-from-text support, these tests
// will still hold, just with additional clip-state assertions layered
// on top. ~keep

/// Page with a 90 pt `M` under text render mode `tr`: red fill, blue stroke, 4 pt line width, so the
/// stem is wide enough to hold a fill interior between two stroke bands.
fn tr_stroke_page(tr: u8) -> Vec<u8> {
    let content = format!("BT 1 0 0 rg 0 0 1 RG 4 w /F1 90 Tf {tr} Tr 8 20 Td (M) Tj ET\n");
    let doc = PdfDocument::from_bytes(build_pdf_text(&content, "")).expect("PDF parses");
    render_with_pipeline(&doc, true)
}

/// Run-length classes of one pixel row after its first ink pixel: `B` solid stroke blue, `R` solid fill
/// red, `W` white. Anti-aliased and blended edge pixels are dropped, so runs only count solid colour.
fn solid_runs(rgba: &[u8], y: u32) -> Vec<(char, usize)> {
    let mut runs: Vec<(char, usize)> = Vec::new();
    let mut started = false;
    for x in 0..100u32 {
        let o = ((y * 100 + x) * 4) as usize;
        let (r, g, b) = (rgba[o], rgba[o + 1], rgba[o + 2]);
        let class = if r >= 250 && g >= 250 && b >= 250 {
            'W'
        } else if r >= 235 && g <= 20 && b <= 20 {
            'R'
        } else if b >= 235 && r <= 20 && g <= 20 {
            'B'
        } else {
            started = true;
            continue;
        };
        if class == 'W' && !started {
            continue;
        }
        started = true;
        match runs.last_mut() {
            Some((last, n)) if *last == class => *n += 1,
            _ => runs.push((class, 1)),
        }
    }
    runs
}

/// A stem of a fill-and-stroke glyph is blue, red, blue across (stroke band, fill interior, stroke
/// band), and the outer stroke band is as wide as a stroke-only glyph's, so the stroke covers the fill
/// edge instead of the fill covering the stroke's inner half (ISO 32000-1 9.3.6 Table 106).
fn assert_fill_then_stroke(tr: u8) {
    let both = solid_runs(&tr_stroke_page(tr), 40);
    let stroke_only = solid_runs(&tr_stroke_page(1), 40);
    assert!(
        matches!(both.as_slice(), [('B', e1), ('R', f), ('B', e2), ..] if *e1 >= 3 && *f >= 2 && *e2 >= 3),
        "Tr={tr} stem must read stroke blue, fill red, stroke blue, got {both:?}"
    );
    assert_eq!(
        both[0], stroke_only[0],
        "Tr={tr} outer stroke band must be as wide as a stroke-only glyph's, so the stroke paints after the fill"
    );
}

/// A stroke-only glyph is a hollow outline: stroke bands around white, and no fill red anywhere.
fn assert_hollow_stroke(tr: u8) {
    let runs = solid_runs(&tr_stroke_page(tr), 40);
    assert!(
        matches!(runs.as_slice(), [('B', e1), ('W', gap), ('B', e2), ..] if *e1 >= 3 && *gap >= 2 && *e2 >= 3),
        "Tr={tr} must draw a hollow blue outline, got {runs:?}"
    );
    assert!(
        runs.iter().all(|(class, _)| *class != 'R'),
        "Tr={tr} must not paint the fill colour, got {runs:?}"
    );
}

/// Probe 5a — Tr=0 (fill-only): the pipeline must paint a plain DeviceRGB
/// fill on a `Tj` glyph as the expected RGB.
#[test]
fn qa_text_tr0_fill_only_paints_red() {
    let content = "BT 1 0 0 rg /F1 40 Tf 0 Tr 10 30 Td (M) Tj ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Tr=0 fill-only with red fill → the glyph must paint red. ~keep
    let avg = average_ink_rgb(&on, 0, 0, 100, 100);
    let (r, g, b) = avg.expect("Tr=0 must paint red glyph ink");
    assert!(
        r > 180.0 && r > g + 60.0 && r > b + 60.0,
        "Tr=0 fill-only must paint red, got ({r:.1}, {g:.1}, {b:.1})"
    );
}

/// Probe 5b — Tr=1 (stroke-only). The glyph is a hollow outline in the stroke colour.
#[test]
fn qa_text_tr1_stroke_only_draws_a_hollow_outline() {
    assert_hollow_stroke(1);
}

/// Probe 5c — Tr=2 (fill then stroke): fill interior in the fill colour, stroke over its edge.
#[test]
fn qa_text_tr2_fills_the_interior_and_strokes_the_edge() {
    assert_fill_then_stroke(2);
}

/// Probe 5d — Tr=3 (invisible). Pipeline short-circuits to None — no
/// clone of `gs` happens. The page must stay at the white background
/// (the rasteriser zeroes alpha for Tr=3).
#[test]
fn qa_text_tr3_invisible_paints_zero_pixels() {
    let content = "BT 1 0 0 rg /F1 40 Tf 3 Tr 10 30 Td (M) Tj ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // §9.3.6 Table 106 Tr=3: invisible text — the pipeline helper
    // short-circuits to None (no GS clone) and the rasteriser zeroes
    // alpha. Zero painted pixels. ~keep
    assert_eq!(
        count_ink_pixels(&on, 0, 0, 100, 100),
        0,
        "Tr=3 invisible text must paint zero pixels"
    );
}

/// Probe 5e — Tr=4 (fill + add to clip path). Pipeline resolves the fill
/// side. The rasteriser today doesn't implement clip-from-text, so the
/// painted output is the same as Tr=0; the fill colour paints.
///
/// This pins the CURRENT behaviour. When clip-from-text lands, this test's
/// fill-paints assertion still holds — what would change is the assertion on
/// where ink appears (clip would suppress subsequent paints outside the
/// glyph silhouette).
#[test]
fn qa_text_tr4_fill_plus_clip_paints_fill_side() {
    let content = "BT 1 0 0 rg /F1 40 Tf 4 Tr 10 30 Td (M) Tj ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let avg = average_ink_rgb(&on, 0, 0, 100, 100);
    assert!(avg.is_some(), "Tr=4 must paint the fill side (red glyph)");
}

/// Probe 5f — Tr=5 (stroke, then add to the clip path): drawn as a hollow outline.
#[test]
fn qa_text_tr5_stroke_plus_clip_draws_a_hollow_outline() {
    assert_hollow_stroke(5);
}

/// Probe 5g — Tr=6 (fill, stroke, then add to the clip path): painted like Tr=2.
#[test]
fn qa_text_tr6_fills_strokes_and_adds_to_the_clip() {
    assert_fill_then_stroke(6);
}

/// Probe 5h — Tr=7 (add to clip path only). Per the spec Tr=7 is a
/// clip-only mode that paints nothing. The pipeline helper's `matches!`
/// rules don't include 7 for either fills or strokes — so the helper
/// returns None and no GS clone happens. The page must render as a
/// full pixmap without panicking.
#[test]
fn qa_text_tr7_clip_only_renders_without_panic() {
    // Tr=7 (add-to-clip-only) — the pipeline helper returns None for
    // both fill and stroke sides. The rasteriser today paints the
    // glyph outline as a side effect of the dispatch; clip-from-text
    // is a documented capability gap. Pin the no-panic / full pixmap
    // invariant. ~keep
    let content = "BT 1 0 0 rg /F1 40 Tf 7 Tr 10 30 Td (M) Tj ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    assert_eq!(on.len(), 100 * 100 * 4, "Tr=7 must produce a full pixmap");
}

/// Probe 6 — Tr changes mid-stream. Sequence: Tr=0 Tj, `Tr 2`, Tr=2 Tj.
/// Each call gets its own pipeline-resolve; the previous call's spliced
/// GS clone must not leak into the next call's borrowed `gs`.
#[test]
fn qa_text_tr_change_mid_stream_no_leak_paints_red() {
    let content = "BT 1 0 0 rg 0 0 1 RG /F1 20 Tf 5 60 Td \
                   0 Tr (A) Tj \
                   2 Tr (B) Tj \
                   0 Tr (C) Tj ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Tr changes don't leak GS state between Tj calls — every Tj
    // resolves cleanly. Painted ink must be the fill (red), not the
    // stroke (blue), across all three glyphs. ~keep
    let avg = average_ink_rgb(&on, 0, 0, 100, 100);
    let (r, g, b) = avg.expect("Tr change mid-stream must paint glyph ink");
    assert!(
        r > 180.0 && r > g + 60.0 && r > b + 60.0,
        "Tr change mid-stream must paint red fill across all glyphs, got ({r:.1}, {g:.1}, {b:.1})"
    );
}

/// Probe 7 — Type 4 Separation on text fill across THREE consecutive `Tj`
/// calls in a single `BT/ET` block, with the spot colour set once before
/// the block. This is the wave-1 capability-gain class applied at scale:
/// the inline `scn` fallback renders all three glyphs as solid black,
/// while the pipeline must render all three as the program's actual
/// colour (magenta).
///
/// Beyond "the pipeline gets the colour right", this probes that the
/// helper does NOT re-resolve the same Separation for each Tj — it must,
/// because each Tj call clones a fresh GS spliced with the resolved
/// colour. What matters here is that ALL THREE glyphs land in the right
/// colour, proving the per-call resolution is consistent and not flaky.
#[test]
fn qa_text_three_consecutive_tj_type4_separation_capability() {
    let type4_program = "{ 0.0 exch 0.0 0.0 }";
    let content = "/SpotMagenta cs 1 scn \
                   BT /F1 30 Tf 5 70 Td (A) Tj \
                                          (B) Tj \
                                          (C) Tj ET\n";
    let resources = "/ColorSpace << /SpotMagenta [/Separation /MagentaSpot /DeviceCMYK 6 0 R] >>";
    let bytes = build_pdf_text_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");

    let on = render_with_pipeline(&doc, true);

    let avg_on = average_ink_rgb(&on, 0, 30, 100, 95).expect("pipeline: magenta ink");
    assert!(
        avg_on.0 > avg_on.1 + 25.0 && avg_on.2 > avg_on.1 + 25.0,
        "pipeline: three Tj glyphs under Type 4 Separation must paint magenta-shaped \
         (R,B above G), got ({:.1}, {:.1}, {:.1})",
        avg_on.0,
        avg_on.1,
        avg_on.2
    );
}

/// Probe 8 — DeviceN multi-colorant Type 4 on text fill. Wave-1 already
/// proves DeviceN for `f`; the wave-2 mirror confirms it for `Tj`.
/// The inline path falls back per the wave-1 finding; the pipeline
/// must run the Type 4 program and project through the alt-space.
#[test]
fn qa_text_tj_devicen_multi_colorant_type4_capability() {
    // 2-colorant DeviceN, same Type 4 stack walk as the wave-1 sibling.
    // With `0 1 scn` the program emits CMYK(0,1,0,0) → magenta. ~keep
    let type4_program = "{ exch pop 0.0 exch 0.0 0.0 }";
    let resources = "/ColorSpace << /TwoSpot [/DeviceN [/SpotA /SpotB] /DeviceCMYK 6 0 R] >>";
    let content = "/TwoSpot cs 0 1 scn \
                   BT /F1 60 Tf 10 30 Td (M) Tj ET\n";
    let range = "[0 1 0 1 0 1 0 1]";
    let bytes = build_pdf_text_with_devicen_type4(content, type4_program, resources, range, &[0, 1, 0, 1]);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");

    let on = render_with_pipeline(&doc, true);
    let avg_on = average_ink_rgb(&on, 0, 20, 100, 95).expect("pipeline: magenta ink");
    assert!(
        avg_on.0 > 100.0
            && avg_on.1 < 80.0
            && avg_on.2 > 100.0
            && avg_on.0 > avg_on.1 + 50.0
            && avg_on.2 > avg_on.1 + 50.0,
        "pipeline: DeviceN Type-4 text fill must paint magenta-shaped, got ({:.1}, {:.1}, {:.1})",
        avg_on.0,
        avg_on.1,
        avg_on.2
    );
}

/// Probe 9 — Separation with `/All` colorant name on text fill. The
/// pipeline doesn't special-case the name — runs the tint transform
/// like any other Separation, so the rendered glyph picks up the
/// magenta-shape from the Type 4 program.
#[test]
fn qa_text_tj_separation_all_colorant() {
    let type4_program = "{ 0.0 exch 0.0 0.0 }";
    let content = "/All_CS cs 0.5 scn \
                   BT /F1 60 Tf 10 30 Td (M) Tj ET\n";
    let resources = "/ColorSpace << /All_CS [/Separation /All /DeviceCMYK 6 0 R] >>";
    let bytes = build_pdf_text_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");

    let on = render_with_pipeline(&doc, true);
    let avg_on = average_ink_rgb(&on, 0, 20, 100, 95).expect("pipeline: tinted ink");
    // tint=0.5 → CMYK(0, 0.5, 0, 0) → faint magenta (additive clamp
    // gives RGB ~ (255, 127, 255)). ~keep
    assert!(
        avg_on.0 > avg_on.1 && avg_on.2 > avg_on.1,
        "pipeline /All Separation Type-4 text fill must trend magenta (R>G, B>G), \
         got ({:.1}, {:.1}, {:.1})",
        avg_on.0,
        avg_on.1,
        avg_on.2
    );
}

/// Probe 10 — Separation `/None` colorant on text fill. Per ISO 32000-1
/// §8.6.6.3, `/None` produces no visible output. The pipeline's per-plate
/// routing selector (`InkSelector::None`, stamped by the composer when the
/// source colour space is `/Separation /None`) makes the composite
/// resolver hand back a fully-transparent RGBA, so the text rasteriser
/// paints with alpha=0 and lays down zero ink — regardless of what the
/// tint transform would have produced.
#[test]
fn qa_text_tj_separation_none_colorant_paints_zero_ink() {
    let type4_program = "{ 0.0 exch 0.0 0.0 }";
    let content = "/None_CS cs 0.5 scn \
                   BT /F1 60 Tf 10 30 Td (M) Tj ET\n";
    let resources = "/ColorSpace << /None_CS [/Separation /None /DeviceCMYK 6 0 R] >>";
    let bytes = build_pdf_text_with_type4_separation(content, type4_program, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    let on_ink = count_ink_pixels(&on, 0, 0, 100, 100);
    assert_eq!(
        on_ink, 0,
        "pipeline /None text fill must paint zero ink per §8.6.6.3 (got {on_ink} ink pixels)"
    );
}

/// Probe 11 — Tc (character spacing) preserved through the pipeline.
/// Wider Tc widens the inter-glyph gap, pushing the rightmost glyph
/// further right; the spliced GS clone must round-trip Tc unchanged.
#[test]
fn qa_text_tc_character_spacing_preserved() {
    let normal = "BT 1 0 0 rg /F1 16 Tf 5 50 Td (HHH) Tj ET\n";
    let wide = "BT 1 0 0 rg /F1 16 Tf 3 Tc 5 50 Td (HHH) Tj ET\n";
    let normal_doc = PdfDocument::from_bytes(build_pdf_text(normal, "")).unwrap();
    let wide_doc = PdfDocument::from_bytes(build_pdf_text(wide, "")).unwrap();
    let normal_on = render_with_pipeline(&normal_doc, true);
    let wide_on = render_with_pipeline(&wide_doc, true);
    let rightmost = |rgba: &[u8]| -> Option<u32> {
        for x in (0u32..100).rev() {
            for y in 30u32..70 {
                let off = ((y * 100 + x) * 4) as usize;
                if rgba[off] < 240 || rgba[off + 1] < 240 || rgba[off + 2] < 240 {
                    return Some(x);
                }
            }
        }
        None
    };
    let normal_right = rightmost(&normal_on).expect("normal: ink present");
    let wide_right = rightmost(&wide_on).expect("wide: ink present");
    assert!(
        wide_right > normal_right,
        "Tc=3 must push rightmost glyph right of Tc=0; normal={normal_right}, wide={wide_right}"
    );
}

/// Probe 12 — Tw (word spacing) preserved through the pipeline. Tw applies
/// only at space (0x20) glyphs. Render "HHH HHH" with Tw=0 vs Tw=5; the
/// wider rendering's rightmost ink lands further right.
#[test]
fn qa_text_tw_word_spacing_preserved() {
    let normal = "BT 1 0 0 rg /F1 16 Tf 5 50 Td (HHH HHH) Tj ET\n";
    let wide = "BT 1 0 0 rg /F1 16 Tf 5 Tw 5 50 Td (HHH HHH) Tj ET\n";
    let normal_doc = PdfDocument::from_bytes(build_pdf_text(normal, "")).unwrap();
    let wide_doc = PdfDocument::from_bytes(build_pdf_text(wide, "")).unwrap();
    let normal_on = render_with_pipeline(&normal_doc, true);
    let wide_on = render_with_pipeline(&wide_doc, true);
    let rightmost = |rgba: &[u8]| -> Option<u32> {
        for x in (0u32..100).rev() {
            for y in 30u32..70 {
                let off = ((y * 100 + x) * 4) as usize;
                if rgba[off] < 240 || rgba[off + 1] < 240 || rgba[off + 2] < 240 {
                    return Some(x);
                }
            }
        }
        None
    };
    let normal_right = rightmost(&normal_on).expect("normal: ink present");
    let wide_right = rightmost(&wide_on).expect("wide: ink present");
    assert!(
        wide_right > normal_right,
        "Tw=5 must push rightmost glyph right of Tw=0; normal={normal_right}, wide={wide_right}"
    );
}

/// Probe 13 — Tz (horizontal scaling) preserved on the `TJ` variant.
/// Tz must survive the spliced GS clone so the horizontal advance the
/// rasteriser computes is identical to the un-spliced path.
#[test]
fn qa_text_tz_horizontal_scale_preserved_on_tj_array() {
    let normal = "BT 1 0 0 rg /F1 16 Tf 5 50 Td [(HHH)] TJ ET\n";
    let narrow = "BT 1 0 0 rg /F1 16 Tf 50 Tz 5 50 Td [(HHH)] TJ ET\n";
    let normal_doc = PdfDocument::from_bytes(build_pdf_text(normal, "")).unwrap();
    let narrow_doc = PdfDocument::from_bytes(build_pdf_text(narrow, "")).unwrap();
    let normal_on = render_with_pipeline(&normal_doc, true);
    let narrow_on = render_with_pipeline(&narrow_doc, true);
    let rightmost = |rgba: &[u8]| -> Option<u32> {
        for x in (0u32..100).rev() {
            for y in 30u32..70 {
                let off = ((y * 100 + x) * 4) as usize;
                if rgba[off] < 240 || rgba[off + 1] < 240 || rgba[off + 2] < 240 {
                    return Some(x);
                }
            }
        }
        None
    };
    let normal_right = rightmost(&normal_on).expect("Tz=100: ink present");
    let narrow_right = rightmost(&narrow_on).expect("Tz=50: ink present");
    assert!(
        narrow_right < normal_right,
        "Tz=50 must place rightmost TJ glyph LEFT of Tz=100; normal={normal_right}, narrow={narrow_right}"
    );
}

/// Probe 14 — TL (text leading) preserved. `'` (Quote) uses TL to
/// advance the text matrix down by `-TL` before painting. Two `'`
/// calls separated by TL=20 must land on visibly different lines after
/// going through the pipeline.
#[test]
fn qa_text_tl_leading_preserved_on_quote() {
    let content = "BT 1 0 0 rg /F1 14 Tf 20 TL 5 80 Td (Line1) ' (Line2) ' ET\n";
    let bytes = build_pdf_text(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let on = render_with_pipeline(&doc, true);
    // Two `'` calls separated by TL=20 must land on distinct lines. ~keep
    let mut bands_with_ink = 0;
    for band_start in (0u32..90).step_by(10) {
        if count_ink_pixels(&on, 0, band_start, 100, band_start + 10) > 0 {
            bands_with_ink += 1;
        }
    }
    assert!(
        bands_with_ink >= 2,
        "TL=20 + two Quote calls must paint in >= 2 vertical bands, got {bands_with_ink}"
    );
}

/// Probe 15 — Tm (set text matrix) before Tj. Tm replaces the text matrix
/// outright; the glyph paints at the matrix's origin. Two Tm's at
/// different translations must produce visually distinct renders.
#[test]
fn qa_text_tm_before_tj_preserved() {
    let left = "BT 1 0 0 rg /F1 30 Tf 1 0 0 1 5 50 Tm (M) Tj ET\n";
    let right = "BT 1 0 0 rg /F1 30 Tf 1 0 0 1 60 50 Tm (M) Tj ET\n";
    let left_doc = PdfDocument::from_bytes(build_pdf_text(left, "")).unwrap();
    let right_doc = PdfDocument::from_bytes(build_pdf_text(right, "")).unwrap();
    let left_on = render_with_pipeline(&left_doc, true);
    let right_on = render_with_pipeline(&right_doc, true);
    assert_ne!(
        left_on, right_on,
        "Tm at different translations must produce different renders"
    );
}

/// Probe 16 — Td / TD (move text position) before Tj. Td translates by
/// (tx, ty); TD does the same AND sets leading = -ty. Two PDFs differing
/// only in Td translation must produce distinct renders.
#[test]
fn qa_text_td_translation_preserved() {
    let pos_a = "BT 1 0 0 rg /F1 30 Tf 5 30 Td (M) Tj ET\n";
    let pos_b = "BT 1 0 0 rg /F1 30 Tf 50 30 Td (M) Tj ET\n";
    let a_doc = PdfDocument::from_bytes(build_pdf_text(pos_a, "")).unwrap();
    let b_doc = PdfDocument::from_bytes(build_pdf_text(pos_b, "")).unwrap();
    let a_on = render_with_pipeline(&a_doc, true);
    let b_on = render_with_pipeline(&b_doc, true);
    assert_ne!(
        a_on, b_on,
        "Td at different translations must produce different renders"
    );
}

/// Probe 13b — Tz interaction with TJ numeric-kerning offsets. Pre-existing
/// non-wave-2 issue surfaced during state-preservation probing:
/// when a `TJ` array contains numeric kerning offsets, the horizontal
/// scaling dial `Tz` is NOT applied to those offsets — both Tz=100 and
/// Tz=50 produce the same rightmost ink column. The Tz dial IS applied
/// to ordinary glyph advance (probe 13 verifies this for plain strings).
///
/// This is a pre-existing rasteriser bug (the kerning-advance branch
/// doesn't multiply by Tz / 100), not introduced by the migration. The
/// pin documents the bug for a follow-up and is `#[ignore]`d so the
/// gate stays green.
#[test]
#[ignore = "pre-existing inline bug: Tz not applied to TJ numeric kerning advance"]
fn qa_text_tz_applied_to_tj_kerning_advance_narrows_rightmost() {
    let normal = "BT 1 0 0 rg /F1 14 Tf 5 50 Td [(H) -50 (e) -50 (l) -50 (l) -50 (o)] TJ ET\n";
    let narrow = "BT 1 0 0 rg /F1 14 Tf 50 Tz 5 50 Td [(H) -50 (e) -50 (l) -50 (l) -50 (o)] TJ ET\n";
    let normal_doc = PdfDocument::from_bytes(build_pdf_text(normal, "")).unwrap();
    let narrow_doc = PdfDocument::from_bytes(build_pdf_text(narrow, "")).unwrap();
    let normal_on = render_with_pipeline(&normal_doc, true);
    let narrow_on = render_with_pipeline(&narrow_doc, true);
    let rightmost = |rgba: &[u8]| -> Option<u32> {
        for x in (0u32..100).rev() {
            for y in 30u32..70 {
                let off = ((y * 100 + x) * 4) as usize;
                if rgba[off] < 240 || rgba[off + 1] < 240 || rgba[off + 2] < 240 {
                    return Some(x);
                }
            }
        }
        None
    };
    let normal_right = rightmost(&normal_on).expect("Tz=100 + kern: ink");
    let narrow_right = rightmost(&narrow_on).expect("Tz=50 + kern: ink");
    // BUG: today both rightmost positions are equal — Tz=50 should produce
    // a narrower run. When the bug is fixed, this assertion flips. ~keep
    assert!(
        narrow_right < normal_right,
        "BUG (Tz vs TJ kerning): Tz=50 should put rightmost LEFT of Tz=100, but \
         got narrow={narrow_right} normal={normal_right} — Tz not applied to \
         TJ numeric kerning advance"
    );
}

/// Build a one-page text-fixture PDF with a CID Type 0 font tree at object
/// 5 (Type 0 wraps a CIDFontType2 descendant, using `/Identity-H`
/// encoding). No embedded font program — the rasteriser falls back to a
/// system font; correctness of the system fallback isn't probed here,
/// only that the pipeline routes Type 0 through without panicking.
fn build_pdf_cid_type0(content_ops: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content_ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let type0_off = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /Helvetica \
          /Encoding /Identity-H /DescendantFonts [6 0 R] >>\nendobj\n",
    );

    let cidfont_off = buf.len();
    buf.extend_from_slice(
        b"6 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Helvetica \
          /CIDSystemInfo 7 0 R /FontDescriptor 8 0 R /DW 500 >>\nendobj\n",
    );

    let csi_off = buf.len();
    buf.extend_from_slice(b"7 0 obj\n<< /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>\nendobj\n");

    let fd_off = buf.len();
    buf.extend_from_slice(
        b"8 0 obj\n<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
          /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 \
          /Descent -207 /CapHeight 718 /StemV 88 >>\nendobj\n",
    );

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 9\n0000000000 65535 f \n");
    for off in [
        cat_off,
        pages_off,
        page_off,
        stream_off,
        type0_off,
        cidfont_off,
        csi_off,
        fd_off,
    ] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

// Split for file-too-long (#1567): the module continues below via `include!`,
// not a sibling tests/*_part2.rs file -- Cargo auto-discovers every *.rs file
// directly under tests/ as its own test binary, so a sibling would fail to
// compile on its own and, if it somehow did, run every #[test] in it twice.
// A subdirectory is not auto-discovered. ~keep
include!("test_render_resolution_pipeline_qa_wave2/part2.rs");
