//! Exercise the `rendering::render_page_region` and
//! `rendering::render_page_fit` entry points.

mod common;

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page, render_page_fit, render_page_region};

fn setup() -> PdfDocument {
    let content = common::text_run_op("region fit probe", 72.0, 700.0, "Helvetica", 12.0);
    let bytes =
        common::build_pdf_with_standard_fonts(content.as_bytes(), b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]");
    PdfDocument::from_bytes(bytes).unwrap()
}

fn is_png(b: &[u8]) -> bool {
    b.len() >= 8 && b.starts_with(&[0x89, 0x50, 0x4e, 0x47])
}

#[test]
fn render_page_region_returns_clipped_png() {
    let doc = setup();
    let full = render_page(&doc, 0, &RenderOptions::with_dpi(72)).unwrap();
    let region = render_page_region(&doc, 0, (36.0, 36.0, 144.0, 144.0), &RenderOptions::with_dpi(72)).unwrap();

    assert!(is_png(&region.data));
    assert!(region.width < full.width);
    assert!(region.height < full.height);
    assert!(region.width > 0 && region.height > 0);
}

#[test]
fn render_page_region_rejects_zero_rect() {
    let doc = setup();
    let err = render_page_region(&doc, 0, (0.0, 0.0, 0.0, 0.0), &RenderOptions::with_dpi(72));
    assert!(err.is_err(), "zero-area rect should fail");
}

#[test]
fn render_page_fit_respects_box() {
    let doc = setup();
    let img = render_page_fit(&doc, 0, 200, 100, &RenderOptions::with_dpi(72)).unwrap();
    assert!(is_png(&img.data));
    assert!(img.width <= 200 + 5, "fit width {} > 200", img.width);
    assert!(img.height <= 100 + 5, "fit height {} > 100", img.height);
}

#[test]
fn render_page_fit_rejects_zero_box() {
    let doc = setup();
    assert!(render_page_fit(&doc, 0, 0, 100, &RenderOptions::with_dpi(72)).is_err());
    assert!(render_page_fit(&doc, 0, 100, 0, &RenderOptions::with_dpi(72)).is_err());
}

// ── Fit-box DPI rounding regressions ────────────────────────────────────────
//
// setup() produces a Letter page (612×792 pt).
//
// Old bug: DPI was computed as floor(fit_px * 72 / page_pt), which could lose
// up to 3 pixels compared with the requested fit box.
// Fix: compute a float scale = fit_px / page_pt and apply round(), so the
// constrained dimension is exactly fit_px. ~keep

#[test]
fn render_page_fit_constrained_width_is_exact() {
    // Letter (612×792 pt), fit to 1040×2048 — width-constrained.
    // Old code: dpi = floor(1040×72/612) = 122 → width = ceil(612×122/72) = 1037 (3 px short).
    // New code: scale = 1040/612 → width = round(612 × scale) = 1040 exactly. ~keep
    let doc = setup();
    let img = render_page_fit(&doc, 0, 1040, 2048, &RenderOptions::default()).unwrap();
    assert!(is_png(&img.data));
    assert_eq!(
        img.width, 1040,
        "width must equal fit_w (old floor-DPI gave {})",
        img.width
    );
    assert!(img.height <= 2048, "height {} must not exceed fit_h 2048", img.height);
}

#[test]
fn render_page_fit_constrained_height_is_exact() {
    // Letter (612×792 pt), fit to 2048×1040 — height-constrained.
    // scale = 1040/792 → height = round(792 × scale) = 1040 exactly. ~keep
    let doc = setup();
    let img = render_page_fit(&doc, 0, 2048, 1040, &RenderOptions::default()).unwrap();
    assert!(is_png(&img.data));
    assert_eq!(
        img.height, 1040,
        "height must equal fit_h (old floor-DPI gave {})",
        img.height
    );
    assert!(img.width <= 2048, "width {} must not exceed fit_w 2048", img.width);
}

#[test]
fn render_page_fit_never_exceeds_box() {
    // The output must never overflow the requested fit box by even 1 pixel. ~keep
    let doc = setup();
    for (fw, fh) in [(100u32, 200u32), (200, 100), (150, 150), (1040, 2048), (99, 99)] {
        let img = render_page_fit(&doc, 0, fw, fh, &RenderOptions::default()).unwrap();
        assert!(img.width <= fw, "fit ({fw}×{fh}): width {} exceeds {fw}", img.width);
        assert!(img.height <= fh, "fit ({fw}×{fh}): height {} exceeds {fh}", img.height);
    }
}

fn red_corner_region(page_dict: &[u8], origin: (f32, f32)) -> image::RgbaImage {
    let content = format!("1 0 0 rg {} {} 50 50 re f", origin.0, origin.1);
    let bytes = common::build_pdf_with_standard_fonts(content.as_bytes(), page_dict);
    let doc = PdfDocument::from_bytes(bytes).unwrap();
    let mut options = RenderOptions::with_dpi(72);
    options.honor_crop_box = true;
    let region = render_page_region(&doc, 0, (origin.0, origin.1, 50.0, 50.0), &options).unwrap();
    image::load_from_memory(&region.data).unwrap().to_rgba8()
}

fn assert_all_red(img: &image::RgbaImage) {
    assert!(
        img.width() >= 49 && img.height() >= 49,
        "{}x{}",
        img.width(),
        img.height()
    );
    for px in img.pixels() {
        assert_eq!(px.0, [255, 0, 0, 255]);
    }
}

#[test]
fn render_page_region_follows_a_crop_box_offset() {
    let img = red_corner_region(
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /CropBox [100 100 300 300]",
        (100.0, 100.0),
    );
    assert_all_red(&img);
}

#[test]
fn render_page_region_follows_the_page_rotation() {
    let img = red_corner_region(
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Rotate 90",
        (0.0, 0.0),
    );
    assert_all_red(&img);
}

fn cropped_page_region(
    request: (f32, f32, f32, f32),
) -> xberg_native_pdf::Result<xberg_native_pdf::rendering::RenderedImage> {
    let bytes = common::build_pdf_with_standard_fonts(
        b"1 0 0 rg 0 0 400 400 re f",
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /CropBox [100 100 300 300]",
    );
    let doc = PdfDocument::from_bytes(bytes).unwrap();
    let mut options = RenderOptions::with_dpi(72);
    options.honor_crop_box = true;
    render_page_region(&doc, 0, request, &options)
}

#[test]
fn render_page_region_clips_a_region_overhanging_the_raster_edge() {
    let region = cropped_page_region((90.0, 110.0, 20.0, 20.0)).unwrap();
    assert_eq!((region.width, region.height), (10, 20));
}

#[test]
fn render_page_region_rejects_a_region_outside_the_raster() {
    assert!(cropped_page_region((0.0, 0.0, 50.0, 50.0)).is_err());
}
