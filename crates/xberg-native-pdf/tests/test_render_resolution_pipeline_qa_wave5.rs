//! Wave-5 QA probes for the resolution-pipeline migration finale.
//!
//! Wave 5 deleted the inline tint-transform arms in `SetFillColor` /
//! `SetFillColorN` / `SetStrokeColorN`, deleted the inline
//! `evaluate_shading_function` + `parse_color_array` helpers, retired the
//! `NATIVE_PDF_RESOLUTION_PIPELINE` env-var toggle (and the cached truth
//! value on `PageRenderer`), and added a per-plate `SeparationBackend`
//! implementation of `PaintBackend` driven by the pipeline's resolved
//! commands.
//!
//! These probes are wave-5-specific:
//!
//! 1. The toggle is gone — env-var settings must have no effect on output.
//! 2. Capabilities the inline arms cannot reach (Type 4 Separation under
//!    `scn` / `SC` / `SCN`, Type 4 shading endpoints) must still be pinned
//!    end-to-end through the full operator dispatcher (not just the
//!    helper).
//! 3. The new shading `None`-endpoint fallback (black-to-white default)
//!    replaces the old `evaluate_shading_function` recovery — pin the
//!    behaviour so a follow-up cannot silently regress it.
//! 4. SeparationBackend byte-for-byte equivalence with the inline
//!    `fill_separation` helper, with additional path / transform / tint
//!    combinations beyond the one the in-source unit test covers.
//! 5. Real-world-shaped corpus rendering: verify that opening a corpus
//!    PDF and rendering page 0 still produces a non-blank pixmap. (Larger
//!    "compare against expected pixmap" tests are out of scope here; this
//!    is the sanity-check tier.)
//!
//! Style mirrors waves 1-4: build a tiny PDF inline, call `render_page`,
//! either pin specific pixel values or assert a structural property.

#![allow(dead_code)]

use xberg_child_test::{child_test_root, run_child_test};
use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// Build a one-page PDF whose content stream is `content_ops`, with a
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

/// Build a one-page PDF with an indirect Type-4 tint-transform function
/// at object 5. The function body's `/Range` defaults to
/// `[0 1 0 1 0 1 0 1]` (DeviceCMYK output ranges). `content_ops` is the
/// page content stream; `page_resources_extra` lets the caller add a
/// `/ColorSpace << /CS1 [/Separation /Foo /DeviceCMYK 5 0 R] >>`
/// declaration.
fn build_pdf_with_type4(content_ops: &str, type4_program: &str, page_resources_extra: &str) -> Vec<u8> {
    build_pdf_with_type4_range(content_ops, type4_program, page_resources_extra, "[0 1 0 1 0 1 0 1]")
}

fn build_pdf_with_type4_range(
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

/// Render the first page at 72 DPI, returning the raw RGBA8 bytes.
fn render(doc: &PdfDocument) -> Vec<u8> {
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(doc, 0, &opts).expect("render_page succeeds");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    img.data
}

/// Render the first page, allowing failure without panicking.
fn render_allow_fail(doc: &PdfDocument) -> Option<Vec<u8>> {
    let opts = RenderOptions::with_dpi(72).as_raw();
    render_page(doc, 0, &opts).ok().map(|img| img.data)
}

fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    assert!(x < w && y < h);
    let off = ((y * w + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

fn center_pixel(rgba: &[u8]) -> (u8, u8, u8, u8) {
    pixel_at(rgba, 50, 50)
}

fn build_pdf_cmyk_fill_rect(c: f32, m: f32, y: f32, k: f32) -> Vec<u8> {
    let content = format!("{} {} {} {} k\n10 10 80 80 re\nf\n", c, m, y, k);
    build_pdf(&content, "")
}

/// Probe 8a — DeviceCMYK pure cyan: only the Cyan plate carries ink at
/// the rect, all other process plates are zero across the rect (no
/// knock-out painted on inks the source colour doesn't name, because
/// the default OP=false would knock them out — but the per-plate
/// shipping renderer's behaviour is "untouched if value is zero" via
/// `tint_for_ink`; we pin whatever the shipping behaviour is).
#[test]
fn qa_wave5_separation_cmyk_pure_cyan_plate_carries_tint() {
    use xberg_native_pdf::rendering::render_separation;
    let bytes = build_pdf_cmyk_fill_rect(0.5, 0.0, 0.0, 0.0);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let cyan = render_separation(&doc, 0, "Cyan", 72).expect("render Cyan plate");
    let magenta = render_separation(&doc, 0, "Magenta", 72).expect("render Magenta plate");
    let yellow = render_separation(&doc, 0, "Yellow", 72).expect("render Yellow plate");
    let black = render_separation(&doc, 0, "Black", 72).expect("render Black plate");
    let idx = (50 * cyan.width as usize) + 50;
    let cyan_value = cyan.data[idx];
    let magenta_value = magenta.data[idx];
    let yellow_value = yellow.data[idx];
    let black_value = black.data[idx];
    assert!(
        (126..=129).contains(&cyan_value),
        "Cyan plate must carry tint ~0.5 (127/128); got {cyan_value}"
    );
    assert_eq!(magenta_value, 0, "Magenta plate untouched (tint 0)");
    assert_eq!(yellow_value, 0, "Yellow plate untouched (tint 0)");
    assert_eq!(black_value, 0, "Black plate untouched (tint 0)");
}

/// Probe 8b — Multi-plate routing: DeviceCMYK fill at (0.5, 0.25, 0.0,
/// 0.7). All four process plates should carry their respective tints.
#[test]
fn qa_wave5_separation_cmyk_mixed_routes_each_plate_independently() {
    use xberg_native_pdf::rendering::render_separations;
    let bytes = build_pdf_cmyk_fill_rect(0.5, 0.25, 0.0, 0.7);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let plates = render_separations(&doc, 0, 72).expect("render separations");
    let by_ink: std::collections::HashMap<String, &xberg_native_pdf::rendering::SeparationPlate> =
        plates.iter().map(|p| (p.ink_name.clone(), p)).collect();
    let cyan = by_ink["Cyan"];
    let magenta = by_ink["Magenta"];
    let yellow = by_ink["Yellow"];
    let black = by_ink["Black"];
    let idx = (50 * cyan.width as usize) + 50;
    let cyan_value = cyan.data[idx];
    let magenta_value = magenta.data[idx];
    let yellow_value = yellow.data[idx];
    let black_value = black.data[idx];
    assert!((126..=129).contains(&cyan_value), "Cyan ~0.5; got {cyan_value}");
    // 0.25 → 63 or 64 (.25 * 255 = 63.75). ~keep
    assert!((63..=65).contains(&magenta_value), "Magenta ~0.25; got {magenta_value}");
    assert_eq!(yellow_value, 0, "Yellow at tint 0 → no ink");
    // 0.7 → 178 or 179 (.7 * 255 = 178.5). ~keep
    assert!((177..=180).contains(&black_value), "Black ~0.7; got {black_value}");
}

/// Probe 8c — Multi-plate routing: full black `0 0 0 1 k` paints only
/// the Black plate at full coverage. Process inks other than Black are
/// untouched.
#[test]
fn qa_wave5_separation_cmyk_full_black_paints_only_black_plate() {
    use xberg_native_pdf::rendering::render_separations;
    let bytes = build_pdf_cmyk_fill_rect(0.0, 0.0, 0.0, 1.0);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let plates = render_separations(&doc, 0, 72).expect("render separations");
    let by_ink: std::collections::HashMap<String, &xberg_native_pdf::rendering::SeparationPlate> =
        plates.iter().map(|p| (p.ink_name.clone(), p)).collect();
    let black = by_ink["Black"];
    let cyan = by_ink["Cyan"];
    let idx = (50 * black.width as usize) + 50;
    let black_value = black.data[idx];
    let cyan_value = cyan.data[idx];
    assert_eq!(black_value, 255, "Black plate at full tint = 255");
    assert_eq!(cyan_value, 0, "Cyan plate untouched");
}

/// Probe 2 — CMYK PDF with a Type-4 Separation spot colour at the
/// public `render_separation` surface. The separation walker routes
/// the paint through the resolution pipeline, which evaluates the
/// Type-4 alternate-CMYK tint transform and hands the resulting CMYK
/// decomposition to the per-plate InkRouter. For
/// `TYPE4_MAGENTA = { 0.0 exch 0.0 0.0 }` and tint=1 the alternate
/// resolves to CMYK(0, 1, 0, 0), so the Magenta plate carries the
/// full tint at the rect.
///
/// This pins the bug-fixed value, not the bug value.
#[test]
fn qa_wave5_separation_renderer_type4_spot_paints_magenta_plate() {
    let content = "/CS1 cs\n1 scn\n10 10 80 80 re\nf\n";
    let resources = "/ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4(content, TYPE4_MAGENTA, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    use xberg_native_pdf::rendering::render_separation;
    let magenta = render_separation(&doc, 0, "Magenta", 72).expect("render Magenta plate");
    let idx = (50 * magenta.width as usize) + 50;
    let value = magenta.data[idx];
    assert_eq!(
        value, 255,
        "Type-4 Separation spot at tint=1 must resolve to Magenta plate at 255; got {value}"
    );
}

/// Probe 2b — Same Type-4 Separation under the *composite* (RGB)
/// renderer must paint magenta. The composite path migrated to the
/// pipeline in waves 1-4; this is the capability the wave-5 commit
/// message describes as "the headline capability the migration
/// closes". We pin it from the public composite-rendering API so a
/// regression that re-introduces the `1.0 - tint` fallback would
/// fail here.
#[test]
fn qa_wave5_type4_separation_composite_renders_magenta() {
    let content = "/CS1 cs\n1 scn\n10 10 80 80 re\nf\n";
    let resources = "/ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4(content, TYPE4_MAGENTA, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, _) = center_pixel(&pixmap);
    assert!(
        r > 225 && g < 20 && (125..=155).contains(&b),
        "Type-4 Separation on composite path must render magenta; got ({r}, {g}, {b})"
    );
}

/// Probe 6 — Deferral pin for `ColorResolver` not emitting
/// `PerChannel` / `Cmyk` end-to-end for Separation / DeviceN. The
/// resolver always projects compound colour spaces down to
/// `ResolvedColor::Rgba`, then folds CMYK → RGB via §10.3.5
/// additive-clamp. This means:
///
///   - `OverprintResolver` (run after `ColorResolver`) sees `Rgba`
///     and produces an empty `participating` channel set — so
///     `InkRouter::route` returns `Skip` for every plate, even when
///     overprint is enabled.
///   - The new `SeparationBackend::paint` therefore skips every plate
///     for any Separation/DeviceN source, which would manifest as
///     "spot plates stay empty" when the operator walker eventually
///     drives `SeparationBackend` (today it doesn't — the deferral is
///     paired with `separation_renderer.rs` still using `tint_for_ink`).
///
/// We pin this from the composite-path side: a CMYK fill at
/// `1 0 0 0 k` (pure cyan) renders cyan on the composite. The
/// pipeline produces `Rgba { r: 0, g: 1, b: 1, a }`, and the
/// `OverprintResolver` discards the CMYK channel decomposition. This
/// is documented deferral #2 in the wave-5 acceptance notes.
///
/// Tracking name: WAVE5-DEFER-COLORRESOLVER-RGBA-ONLY-FOR-COMPOUND-SPACES.
///
/// The pin shape: the composite output is process-ink cyan
/// #00ADEF = (0, 173, 239) - the tetralinear cyan corner, NOT the old
/// additive-clamp (0, 255, 255). A follow-up that wires the resolver to emit
/// `ResolvedColor::Cmyk` for DeviceCMYK sources would not change this
/// composite output (the composite backend would still see the
/// folded RGBA), so the pin survives the deferral closure.
#[test]
fn qa_wave5_defer_color_resolver_rgba_only_for_compound_spaces() {
    let content = "1 0 0 0 k\n10 10 80 80 re\nf\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, a) = center_pixel(&pixmap);
    assert!(
        r < 5 && (160..=185).contains(&g) && b > 225 && a == 255,
        "DeviceCMYK(1,0,0,0) -> process-ink cyan #00ADEF = (0, 173, 239); \
         got ({r}, {g}, {b}, {a})"
    );
}

/// Probe 7 — SeparationBackend equivalence with the inline
/// `fill_separation` helper, with a different path / transform /
/// tint combination than the in-source unit test. The in-source test
/// covers a Cyan-only fill at tint 0.5 with identity transform and a
/// 10×10 axis-aligned rect. This probe extends the equivalence to:
///
///   - A Magenta plate at tint 0.7,
///   - 50×50 rotated rectangle under a non-identity CTM,
///   - Under a `q ... Q` (graphics state save/restore) bracket.
///
/// Because `SeparationBackend` is `pub(crate)` we can't drive it
/// directly from an integration test. We rely on the shipping
/// `render_separation` API to exercise the same `fill_separation`
/// inline helper the backend's unit test compares against — by
/// construction (same CTM, same tint, same path) the inline helper
/// path through `render_separation` and the wave-5 backend's path
/// produce identical pixmaps. This probe pins the inline path so a
/// regression that breaks `fill_separation` (which the backend's
/// unit test is byte-compared against) breaks here too.
#[test]
fn qa_wave5_separation_inline_path_magenta_rotated_rect() {
    use xberg_native_pdf::rendering::render_separation;
    let content = "q\n0.7071 0.7071 -0.7071 0.7071 50 0 cm\n0 0.7 0 0 k\n0 0 50 50 re\nf\nQ\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let magenta = render_separation(&doc, 0, "Magenta", 72).expect("render Magenta plate");
    let any_inked: bool = magenta.data.iter().any(|&v| v >= 100);
    assert!(
        any_inked,
        "rotated Magenta-only fill must produce inked pixels with value >= 100"
    );
    // And the value where it's inked is the tint exactly (255 * 0.7 ≈ 178/179). ~keep
    let max_value = magenta.data.iter().copied().max().unwrap_or(0);
    assert!(
        (175..=181).contains(&max_value),
        "Magenta plate peak value should be ~179 (tint 0.7); got {max_value}"
    );
    let cyan = render_separation(&doc, 0, "Cyan", 72).expect("render Cyan plate");
    let cyan_max = cyan.data.iter().copied().max().unwrap_or(0);
    assert_eq!(cyan_max, 0, "Cyan plate must be fully untouched");
}

const TYPE4_MAGENTA: &str = "{ 0.0 exch 0.0 0.0 }";

fn type4_magenta_separation_resources() -> &'static str {
    "/ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >>"
}

fn type4_magenta_content_stream() -> &'static str {
    "/CS1 cs\n1 scn\n10 10 80 80 re\nf\n"
}

const PIPELINE_ENV: &str = "NATIVE_PDF_RESOLUTION_PIPELINE";
const CHILD_EXPECTED_ENV: &str = "XBERG_QA_WAVE5_EXPECTED_PIPELINE";
const CHILD_ROOT: &str = "XBERG_QA_WAVE5_RENDER_ROOT";
const CHILD_TEST: &str = "render_pipeline_env_child";
const CHILD_MISMATCH: &str = "wave-5 child received the wrong pipeline value";
const CHILD_PIXELS: &str = "page.rgba";

fn run_render_pipeline_child(root: &std::path::Path, value: Option<&str>, expected: Option<&str>) {
    run_child_test(CHILD_TEST, CHILD_ROOT, root, |command| {
        command.env_remove(PIPELINE_ENV).env_remove(CHILD_EXPECTED_ENV);
        if let Some(value) = value {
            command.env(PIPELINE_ENV, value);
        }
        if let Some(expected) = expected {
            command.env(CHILD_EXPECTED_ENV, expected);
        }
    });
}

fn render_with_pipeline_env(value: Option<&str>) -> Vec<u8> {
    let root = tempfile::tempdir().expect("create scratch directory for the rendered pixels");
    run_render_pipeline_child(root.path(), value, value);
    let rgba = std::fs::read(root.path().join(CHILD_PIXELS)).expect("read the isolated child's render");
    assert_eq!(rgba.len(), 100 * 100 * 4, "expected a 100×100 RGBA render");
    rgba
}

#[test]
#[ignore = "subprocess entry point for the wave-5 environment probes"]
fn render_pipeline_env_child() {
    let Some(root) = child_test_root(CHILD_ROOT) else {
        return;
    };
    let expected = std::env::var_os(CHILD_EXPECTED_ENV);
    let value = std::env::var_os(PIPELINE_ENV);
    assert!(value == expected, "{CHILD_MISMATCH}");
    let bytes = build_pdf_with_type4(
        type4_magenta_content_stream(),
        TYPE4_MAGENTA,
        type4_magenta_separation_resources(),
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    std::fs::write(root.join(CHILD_PIXELS), render(&doc)).expect("write the rendered pixels");
}

#[test]
fn qa_wave5_render_child_rejects_a_wrong_pipeline_value() {
    let root = tempfile::tempdir().expect("create scratch directory for the render check");
    let panic = std::panic::catch_unwind(|| run_render_pipeline_child(root.path(), Some("1"), Some("0")))
        .expect_err("the render child must reject a wrong pipeline value");
    let diagnostic = panic
        .downcast::<String>()
        .expect("child helper reports an owned diagnostic");
    assert!(
        diagnostic.contains("test render_pipeline_env_child ... FAILED"),
        "{diagnostic}"
    );
    assert!(
        diagnostic.contains("test result: FAILED. 0 passed; 1 failed; 0 ignored;"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains(CHILD_MISMATCH), "{diagnostic}");
    assert!(!root.path().join(CHILD_PIXELS).exists(), "the child must not render");
}

/// The retired toggle must not affect the output when set to 1.
#[test]
fn qa_wave5_env_var_one_is_inert() {
    let baseline = render_with_pipeline_env(None);
    let with_one = render_with_pipeline_env(Some("1"));

    assert_eq!(
        baseline, with_one,
        "post-wave-5: NATIVE_PDF_RESOLUTION_PIPELINE=1 must not change output"
    );

    let (r, g, b, _) = center_pixel(&with_one);
    assert!(
        r > 225 && g < 20 && (125..=155).contains(&b),
        "Type-4 Separation fill must paint magenta; got ({r}, {g}, {b})"
    );
}

/// The retired toggle must not affect the output when set to 0.
#[test]
fn qa_wave5_env_var_zero_is_inert() {
    let baseline = render_with_pipeline_env(None);
    let with_zero = render_with_pipeline_env(Some("0"));

    assert_eq!(
        baseline, with_zero,
        "post-wave-5: NATIVE_PDF_RESOLUTION_PIPELINE=0 must not change output"
    );

    let (r, g, b, _) = center_pixel(&with_zero);
    assert!(
        r > 225 && g < 20 && (125..=155).contains(&b),
        "Type-4 Separation fill must still paint magenta (toggle gone); got ({r}, {g}, {b})"
    );
}

/// The retired toggle ignores empty and nonnumeric values as well.
#[test]
fn qa_wave5_env_var_arbitrary_values_inert() {
    let baseline = render_with_pipeline_env(None);
    let with_empty = render_with_pipeline_env(Some(""));
    let with_true = render_with_pipeline_env(Some("true"));
    let with_false_caps = render_with_pipeline_env(Some("FALSE"));
    let with_garbage = render_with_pipeline_env(Some("garbage-string"));

    assert_eq!(baseline, with_empty);
    assert_eq!(baseline, with_true);
    assert_eq!(baseline, with_false_caps);
    assert_eq!(baseline, with_garbage);
}

/// Helper: build a one-page PDF with a shading resource at /Sh1 and a
/// content stream that paints it. The shading body is the caller's
/// responsibility — probes use this to inject malformed dicts.
fn build_pdf_shading_raw(shading_body: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
                /Resources << /Shading << /Sh1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let content = "/Sh1 sh\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let shading_off = buf.len();
    buf.extend_from_slice(format!("5 0 obj\n{}\nendobj\n", shading_body).as_bytes());
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, shading_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Probe 13 — Type 1 (function-based) shading: the pipeline does NOT
/// resolve endpoints for type-1 shadings (they're whole-grid functions,
/// not endpoint-based). Inline path used to handle this; post-wave-5
/// the dispatcher falls through the `_ => log::debug + Ok(())` arm,
/// which paints nothing.
///
/// Pin: no panic; the shading produces no visible ink.
#[test]
fn qa_wave5_shading_type1_falls_through_unsupported_arm() {
    let body = "<< /ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1] \
                /Function << /FunctionType 2 /Domain [0 1] \
                /C0 [0 0 0] /C1 [1 1 1] /N 1 >> >>";
    let bytes = build_pdf_shading_raw(body);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "Type 1 shading must not panic");
    let (r, g, b, _) = center_pixel(&out.unwrap());
    assert!(
        r > 250 && g > 250 && b > 250,
        "Type 1 shading falls through to unsupported arm; expect blank page; got ({r}, {g}, {b})"
    );
}

/// Probe 14a — Malformed `/ShadingType 99` (not a real spec value).
/// Dispatcher's `_ => log::debug + Ok(())` arm catches it. No panic;
/// nothing painted.
#[test]
fn qa_wave5_shading_type_99_falls_through_unsupported_arm() {
    let body = "<< /ShadingType 99 /ColorSpace /DeviceRGB /Coords [0 0 100 100] \
                /Function << /FunctionType 2 /Domain [0 1] \
                /C0 [0 0 0] /C1 [1 1 1] /N 1 >> >>";
    let bytes = build_pdf_shading_raw(body);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "malformed /ShadingType 99 must not panic");
    let (r, g, b, _) = center_pixel(&out.unwrap());
    assert!(
        r > 250 && g > 250 && b > 250,
        "ShadingType 99 → unsupported arm → blank; got ({r}, {g}, {b})"
    );
}

/// Probe 14b — Absent `/ShadingType` key. Code defaults to 0
/// (`unwrap_or(0)`); that's also not a real PDF type, so it falls
/// through.
#[test]
fn qa_wave5_shading_missing_shading_type_defaults_to_zero_and_falls_through() {
    let body = "<< /ColorSpace /DeviceRGB /Coords [0 0 100 100] \
                /Function << /FunctionType 2 /Domain [0 1] \
                /C0 [0 0 0] /C1 [1 1 1] /N 1 >> >>";
    let bytes = build_pdf_shading_raw(body);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "missing /ShadingType must not panic");
}

/// Probe 13c — Type 2 (axial) shading with the resolver unable to
/// resolve endpoints (missing `/Function`). Post-wave-5 the fallback
/// is a black-to-white gradient (the safety-net default folded into
/// `render_axial_shading` after the inline `evaluate_shading_function`
/// helper was deleted). The pre-wave-5 behaviour was the same default
/// produced by a different code path (the inline fallback's `Color::BLACK`
/// for unparseable arrays).
#[test]
fn qa_wave5_shading_axial_missing_function_falls_to_black_white_default() {
    let body = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] \
                /Domain [0 1] >>";
    let bytes = build_pdf_shading_raw(body);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "missing /Function must not panic");
    let pixmap = out.unwrap();
    let (r_left, _, _, _) = pixel_at(&pixmap, 1, 50);
    let (r_right, g_right, b_right, _) = pixel_at(&pixmap, 98, 50);
    assert!(
        r_left < 50,
        "axial-shading C0-default end should be ~black; got R={r_left}"
    );
    assert!(
        r_right > 200 && g_right > 200 && b_right > 200,
        "axial-shading C1-default end should be ~white; got ({r_right}, {g_right}, {b_right})"
    );
}

/// Probe 15 — `pipeline_resolve_paint_gs` path arm: `f` operator after
/// `cs / scn`, Type-4 Separation, Tr=0. Pin: magenta centre.
#[test]
fn qa_wave5_pipeline_resolve_paint_gs_path_fill_type4_separation() {
    let bytes = build_pdf_with_type4(
        type4_magenta_content_stream(),
        TYPE4_MAGENTA,
        type4_magenta_separation_resources(),
    );
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, _) = center_pixel(&pixmap);
    assert!(
        r > 225 && g < 20 && (125..=155).contains(&b),
        "Type 4 Separation path fill must produce magenta; got ({r}, {g}, {b})"
    );
}

/// Probe 16 — `pipeline_resolve_text_colors` text arm: `Tj` after
/// `cs / scn` with Tr=0 (fill only). The text rasteriser splices the
/// resolved RGBA into its `current_gs` clone. With Type-4 Separation
/// magenta, the rendered glyphs must be magenta.
#[test]
fn qa_wave5_pipeline_resolve_text_colors_type4_separation_magenta() {
    let content = "BT /F1 24 Tf 10 50 Td /CS1 cs 1 scn 0 Tr (W) Tj ET\n";
    let resources = "/Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> \
                     /ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >>";
    let bytes = build_pdf_with_type4(content, TYPE4_MAGENTA, resources);
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let mut found_magenta = false;
    for y in 30..70 {
        for x in 10..40 {
            let (r, g, b, _) = pixel_at(&pixmap, x, y);
            if r > 200 && b > 200 && r > g.saturating_add(50) && b > g.saturating_add(50) {
                found_magenta = true;
                break;
            }
        }
        if found_magenta {
            break;
        }
    }
    assert!(
        found_magenta,
        "Type 4 Separation Tj text must produce at least one magenta pixel"
    );
}

/// Probe 17 — `pipeline_resolve_components` gs-free arm: shading
/// endpoint resolution with Type-4 Separation `/C0`. The shading's
/// `/ColorSpace` is `[/Separation /MagentaSpot /DeviceCMYK <funcRef>]`;
/// the `/Function` is a Type-2 exponential with `/C0 [1] /C1 [1]` (both
/// endpoints at full tint). Both endpoints resolve to the same magenta;
/// the whole pixmap should be a magenta band.
#[test]
fn qa_wave5_pipeline_resolve_components_shading_type4_separation() {
    let type4 = "{ 0.0 exch 0.0 0.0 }";
    let content = "/Sh1 sh\n";
    let shading_body = "<< /ShadingType 2 \
                        /ColorSpace [/Separation /MagentaSpot /DeviceCMYK 6 0 R] \
                        /Coords [0 50 100 50] /Domain [0 1] \
                        /Function << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [1] /N 1 >> >>";
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
                /Resources << /Shading << /Sh1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let shading_off = buf.len();
    buf.extend_from_slice(format!("5 0 obj\n{}\nendobj\n", shading_body).as_bytes());
    let func_off = buf.len();
    let func_hdr = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        type4.len()
    );
    buf.extend_from_slice(func_hdr.as_bytes());
    buf.extend_from_slice(type4.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, shading_off, func_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("PDF parses");
    let pixmap = render(&doc);
    for x in [5u32, 25, 50, 75, 95] {
        let (r, g, b, _) = pixel_at(&pixmap, x, 50);
        assert!(
            r > 225 && g < 30 && (125..=155).contains(&b),
            "shading endpoint (x={x}): Type-4 Separation must paint magenta; got ({r}, {g}, {b})"
        );
    }
}

// ===========================================================================
// PROBE 19-22: Performance pins for the wave-5 collapsed path
//
// Wave 5 retired the off-vs-on parity perf bounds (which became
// meaningless once the off path was deleted). What remains is a
// wall-clock absolute budget: the only path must not regress beyond a
// generous ceiling under cargo's parallel test runner. Numbers are
// sized to absorb worst-case scheduling jitter — they pin "is this
// catastrophically slow" rather than micro-optimisation drift.
// =========================================================================== ~keep

/// Probe 19 — 1000 DeviceRGB fill rectangles. Pin wall-clock under a
/// generous bound that tolerates parallel-scheduling pressure.
#[test]
fn qa_wave5_perf_thousand_device_rgb_fills_within_bound() {
    let mut content = String::from("1 0 0 rg\n");
    for i in 0..1000u32 {
        let x = (i % 50) as f32 * 2.0;
        let y = ((i / 50) % 50) as f32 * 2.0;
        content.push_str(&format!("{} {} 2 2 re\nf\n", x, y));
    }
    let bytes = build_pdf(&content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let start = std::time::Instant::now();
    let _ = render(&doc);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "1000 device-RGB fills must render in < 5s; took {:.2?}",
        elapsed
    );
}

/// Probe 23 — Empty content stream. No operators. No panic.
#[test]
fn qa_wave5_empty_content_stream_no_panic() {
    let bytes = build_pdf("", "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "empty content stream must not panic");
    let (r, g, b, _) = center_pixel(&out.unwrap());
    assert!(r > 250 && g > 250 && b > 250, "empty page must be white");
}

/// Probe 24 - DeviceCMYK full black via `0 0 0 1 k`. On an RGB composite
/// target the resolver folds CMYK -> RGB via the process-ink converter.
/// CMYK(0,0,0,1) is the measured K-ink corner #231F20 = (35, 31, 32),
/// NOT pure black (0, 0, 0) - the exact defect this probe pins.
#[test]
fn qa_wave5_device_cmyk_full_black_renders_k_ink_on_rgb_target() {
    let content = "0 0 0 1 k\n10 10 80 80 re\nf\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, _) = center_pixel(&pixmap);
    assert!(
        (25..=50).contains(&r) && (20..=45).contains(&g) && (20..=45).contains(&b),
        "DeviceCMYK(0,0,0,1) must render as the process-ink K-ink #231F20 on RGB, \
         not pure black; got ({r}, {g}, {b})"
    );
}

/// Probe 24b - DeviceCMYK pure cyan `1 0 0 0 k` -> process-ink #00ADEF = (0, 173, 239). Pin.
#[test]
fn qa_wave5_device_cmyk_pure_cyan_renders_rgb_cyan_on_rgb_target() {
    let content = "1 0 0 0 k\n10 10 80 80 re\nf\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, _) = center_pixel(&pixmap);
    assert!(
        r < 20 && (160..=185).contains(&g) && b > 225,
        "DeviceCMYK(1,0,0,0) must render as process-ink cyan on RGB; got ({r}, {g}, {b})"
    );
}

/// Probe 24c — DeviceRGB white via `1 1 1 rg`. Trivial sanity that the
/// pipeline doesn't change baseline RGB behaviour.
#[test]
fn qa_wave5_device_rgb_white_renders_white() {
    let content = "1 1 1 rg\n10 10 80 80 re\nf\n";
    let bytes = build_pdf(content, "");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let pixmap = render(&doc);
    let (r, g, b, _) = center_pixel(&pixmap);
    assert!(
        r > 250 && g > 250 && b > 250,
        "DeviceRGB(1,1,1) must render white; got ({r}, {g}, {b})"
    );
}

/// Probe 25 — Form XObject containing a shading. Wave 5 changed the
/// shading dispatch's fallback (when the pipeline can't resolve
/// endpoints). Form XObjects share the operator walker, so the
/// dispatch must still produce a sensible result when reached from
/// inside a Form's content stream.
#[test]
fn qa_wave5_form_xobject_containing_axial_shading() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
                /Resources << /XObject << /Form1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let page_content = "/Form1 Do\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(page_content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let form_off = buf.len();
    let form_content = "/Sh1 sh\n";
    let form_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << /Shading << /Sh1 6 0 R >> >> /Length {} >>\nstream\n",
        form_content.len()
    );
    buf.extend_from_slice(form_hdr.as_bytes());
    buf.extend_from_slice(form_content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let shading_off = buf.len();
    let shading = "6 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] \
                   /Domain [0 1] /Function << /FunctionType 2 /Domain [0 1] \
                   /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >>\nendobj\n";
    buf.extend_from_slice(shading.as_bytes());
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, form_off, shading_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("PDF parses");
    let out = render_allow_fail(&doc);
    assert!(out.is_some(), "Form XObject containing axial shading must not panic");
    let pixmap = out.unwrap();
    let (r0, _, b0, _) = pixel_at(&pixmap, 2, 50);
    let (r1, _, b1, _) = pixel_at(&pixmap, 97, 50);
    assert!(r0 > 150 && b0 < 100, "left edge should lean red; got R={r0} B={b0}");
    assert!(b1 > 150 && r1 < 100, "right edge should lean blue; got R={r1} B={b1}");
}

/// Probe 1 — `tests/fixtures/simple.pdf` renders to a valid RGBA8
/// pixmap (no panic, right dimensions). The fixture is a 612×792
/// PDF 1.4 page with no /Contents stream; the page is white by
/// definition. We pin the rendering pipeline against "still produces a
/// pixmap" — that property held pre-wave-5 and must continue to hold.
#[test]
fn qa_wave5_corpus_simple_pdf_renders_without_panic() {
    let bytes = std::fs::read("tests/fixtures/simple.pdf").expect("fixture exists");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds on simple.pdf");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    assert_eq!(img.width, 612);
    assert_eq!(img.height, 792);
    assert_eq!(img.data.len() as u32, img.width * img.height * 4);
    let (r, g, b, _) = (img.data[0], img.data[1], img.data[2], img.data[3]);
    assert!(r > 250 && g > 250 && b > 250, "empty page must be white");
}

/// Probe 1b — `tests/fixtures/hello_structure.pdf` (tagged PDF / struct
/// tree) renders without panic. The pipeline migration shouldn't
/// affect text extraction quality, but rendering tagged content
/// exercises the operator walker through the same code path as a
/// plain page.
#[test]
fn qa_wave5_corpus_hello_structure_pdf_renders_without_panic() {
    let bytes = std::fs::read("tests/fixtures/hello_structure.pdf").expect("fixture exists");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds on hello_structure.pdf");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    assert_eq!(img.data.len() as u32, img.width * img.height * 4);
}

/// Probe 1c — `tests/fixtures/outline.pdf` (outline tree, empty page).
/// Wave 5 didn't touch outline parsing, but rendering the page
/// exercises the same operator walker — pin no-panic.
#[test]
fn qa_wave5_corpus_outline_pdf_renders_without_panic() {
    let bytes = std::fs::read("tests/fixtures/outline.pdf").expect("fixture exists");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds on outline.pdf");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    assert_eq!(img.data.len() as u32, img.width * img.height * 4);
}

/// Probe 3 — `tests/fixtures/multi_column_table.pdf` (tagged + table
/// extraction friendly). Sanity check that rendering still works after
/// wave 5 — this is a non-trivial multi-page document with real
/// content, so it does need to paint something.
#[test]
fn qa_wave5_corpus_multi_column_table_pdf_renders_non_blank() {
    let bytes = std::fs::read("tests/fixtures/multi_column_table.pdf").expect("fixture exists");
    let doc = PdfDocument::from_bytes(bytes).expect("PDF parses");
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds on multi_column_table.pdf");
    let any_non_white = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .any(|px| px[0] < 200 || px[1] < 200 || px[2] < 200);
    assert!(
        any_non_white,
        "multi_column_table.pdf must produce at least one non-white pixel"
    );
}
