//! Page rendering module for converting PDF pages to images.
//!
//! This module provides functionality to render PDF pages to raster images
//! using the pure-Rust `tiny-skia` library.
//!
//! ## Features
//!
//! - Render pages to PNG/JPEG images
//! - Configurable DPI and image quality
//! - Support for text, paths, and images
//! - Transparency and blend modes
//!
//! ## Example
//!
//! ```ignore
//! use xberg_native_pdf::api::Pdf;
//! use xberg_native_pdf::rendering::{RenderOptions, ImageFormat};
//!
//! let mut pdf = Pdf::open("document.pdf")?;
//! let image = pdf.render_page(0, &RenderOptions::default())?;
//! image.save("page1.png")?;
//! ```
//!
//! ## Architecture
//!
//! The rendering pipeline:
//!
//! 1. Parse page content stream into operators
//! 2. Execute operators against graphics state machine
//! 3. Rasterize paths, text, and images to tiny-skia pixmap
//! 4. Convert to output format (PNG/JPEG)

pub(crate) mod blend_nonsep;
pub(crate) mod ext_gstate;
pub(crate) mod mesh_shading;
pub(crate) mod page_renderer;
mod path_rasterizer;
pub(crate) mod resolution;
pub(crate) mod separation_renderer;
/// CMYK + spot-ink compositing sidecar used by the page renderer to
/// hold the §11.4 transparency-composite state during a page render.
///
/// Public so integration tests can drive the §11.7.4.2 dispatch
/// classifier ([`crate::rendering::sidecar::BlendModeClass`]) directly
/// without going through a rendered pixmap. Internal storage types
/// (`CmykSidecar`) remain `pub(crate)`.
pub mod sidecar;
mod text_contrast_probe;
mod text_rasterizer;

pub use page_renderer::{ImageFormat, PageRenderer, RenderOptions, RenderedImage};
pub use separation_renderer::{SeparationPlate, render_separation, render_separations};

use crate::content::GraphicsState;
use crate::error::Result;
use tiny_skia::{Color, Paint};

/// Create a Paint configured for fill operations from graphics state.
pub(crate) fn create_fill_paint(gs: &GraphicsState, blend_mode: &str) -> Paint<'static> {
    let (r, g, b) = gs.fill_color_rgb;
    let mut paint = Paint::default();

    // Note: render_mode == 3 (invisible text) is handled in the text rendering path,
    // not here, since this paint is also used for non-text fills (paths, shapes). ~keep
    paint.set_color(Color::from_rgba(r, g, b, gs.fill_alpha).unwrap_or(Color::BLACK));

    paint.anti_alias = true;

    if blend_mode != "Normal" {
        paint.blend_mode = pdf_blend_mode_to_skia(blend_mode);
    }

    paint
}

/// Create a Paint configured for stroke operations from graphics state.
pub(crate) fn create_stroke_paint(gs: &GraphicsState, blend_mode: &str) -> Paint<'static> {
    let (r, g, b) = gs.stroke_color_rgb;
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba(r, g, b, gs.stroke_alpha).unwrap_or(Color::BLACK));
    paint.anti_alias = true;

    if blend_mode != "Normal" {
        paint.blend_mode = pdf_blend_mode_to_skia(blend_mode);
    }

    paint
}

/// Convert PDF blend mode to tiny-skia.
pub(crate) fn pdf_blend_mode_to_skia(mode: &str) -> tiny_skia::BlendMode {
    match mode {
        "Normal" => tiny_skia::BlendMode::SourceOver,
        "Multiply" => tiny_skia::BlendMode::Multiply,
        "Screen" => tiny_skia::BlendMode::Screen,
        "Overlay" => tiny_skia::BlendMode::Overlay,
        "Darken" => tiny_skia::BlendMode::Darken,
        "Lighten" => tiny_skia::BlendMode::Lighten,
        "ColorDodge" => tiny_skia::BlendMode::ColorDodge,
        "ColorBurn" => tiny_skia::BlendMode::ColorBurn,
        "HardLight" => tiny_skia::BlendMode::HardLight,
        "SoftLight" => tiny_skia::BlendMode::SoftLight,
        "Difference" => tiny_skia::BlendMode::Difference,
        "Exclusion" => tiny_skia::BlendMode::Exclusion,
        _ => tiny_skia::BlendMode::SourceOver,
    }
}

/// Largest device-space coordinate a path may reach and still be handed to
/// tiny_skia. 2^24 is where f32 stops representing adjacent integers, but it
/// is not the operative bound: tiny_skia clips edges against the pixmap
/// (`edge_clipper.rs`, engaged whenever a path is not contained in the clip)
/// and keeps rasterizing well past that. The bound that matters is where the
/// rasterizer actually gives up — beyond it the antialiased run accounting
/// desynchronizes and panics (`alpha_runs.rs` `unwrap` on a run past the
/// accumulated buffer).
///
/// Measured against tiny_skia 0.12 at 72 dpi: fills and cubics alike still
/// rasterize correctly at 5e8 device units and produce nothing at 7e8, so
/// the bound is the last value confirmed to work. Only 5e8..7e8 is
/// unmeasured, and a draw landing in it is dropped although the rasterizer
/// would have coped. The decade below was measured too, because a real
/// document turned out to live there: a page in the corpus fills to 1.3e8
/// device units, which an earlier 1e8 bound discarded.
const MAX_DEVICE_COORD: f64 = 5.0e8;

/// How far a stroke's outline may reach from its centerline, in device
/// units, before the width is narrowed. Measured separately from
/// [`MAX_DEVICE_COORD`] because it is a separate failure: a 10x10 rect
/// strokes correctly at width 1e9 (outline reach 5e9) and aborts the process
/// at 1e10 (5e10), even though the centerline is only ten units across. 5e8
/// keeps every width measured to rasterize, and its interior 5e8..5e10 is
/// likewise unmeasured.
const MAX_DEVICE_STROKE_REACH: f64 = 5.0e8;

/// Device-space bounds of `path` as `[min_x, min_y, max_x, max_y]`. `None`
/// when a corner is not finite — geometry we cannot place on the pixmap at
/// all.
fn device_bounds(path: &tiny_skia::Path, transform: tiny_skia::Transform) -> Option<[f64; 4]> {
    let (sx, kx, ky, sy, tx, ty) = (
        f64::from(transform.sx),
        f64::from(transform.kx),
        f64::from(transform.ky),
        f64::from(transform.sy),
        f64::from(transform.tx),
        f64::from(transform.ty),
    );
    let b = path.bounds();
    let (left, top) = (f64::from(b.left()), f64::from(b.top()));
    let (right, bottom) = (f64::from(b.right()), f64::from(b.bottom()));
    let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for (x, y) in [(left, top), (right, top), (left, bottom), (right, bottom)] {
        let dx = sx * x + kx * y + tx;
        let dy = ky * x + sy * y + ty;
        if !dx.is_finite() || !dy.is_finite() {
            return None;
        }
        out[0] = out[0].min(dx);
        out[1] = out[1].min(dy);
        out[2] = out[2].max(dx);
        out[3] = out[3].max(dy);
    }
    Some(out)
}

/// Whether a path's device-space bounds are representable enough to
/// rasterize. Used at the clip sites, which have no pixmap to cull against.
pub(crate) fn device_bounds_rasterizable(path: &tiny_skia::Path, transform: tiny_skia::Transform) -> bool {
    device_bounds(path, transform).is_some_and(|b| b.iter().all(|c| c.abs() <= MAX_DEVICE_COORD))
}

/// Whether a draw is worth handing to tiny_skia for a `width` x `height`
/// pixmap. Two independent questions that must not be conflated:
///
/// * Can the ink reach the pixmap? `device_reach` is how far the paint
///   spreads past the path itself (nothing for a fill, the outline reach for
///   a stroke). If it still misses, the draw paints nothing and dropping it
///   is free. This is the only test the untrimmed bounds are entitled to
///   fail — a box that runs off the page says nothing about whether the
///   shape covers the page, and `0 0 1e8 1e8 re f` on a 100 x 100 page
///   covers it completely.
/// * Is the path itself inside [`MAX_DEVICE_COORD`]? Past that there is
///   nothing the rasterizer can do with it. `device_reach` is deliberately
///   NOT folded in here: it is bounded separately by
///   [`MAX_DEVICE_STROKE_REACH`], and adding it would let a large CTM
///   multiply a two-unit line width into a rejection of the whole draw.
fn rasterizable_on(
    path: &tiny_skia::Path,
    transform: tiny_skia::Transform,
    device_reach: f64,
    width: u32,
    height: u32,
) -> bool {
    let Some([x0, y0, x1, y1]) = device_bounds(path, transform) else {
        return false;
    };
    if x1 + device_reach < 0.0
        || y1 + device_reach < 0.0
        || x0 - device_reach > f64::from(width)
        || y0 - device_reach > f64::from(height)
    {
        return false;
    }
    x0.abs().max(x1.abs()) <= MAX_DEVICE_COORD && y0.abs().max(y1.abs()) <= MAX_DEVICE_COORD
}

/// Uniform scale factor `transform` applies, for converting a path-space
/// line width into device units.
fn device_scale(transform: tiny_skia::Transform) -> f64 {
    let det = f64::from(transform.sx) * f64::from(transform.sy) - f64::from(transform.kx) * f64::from(transform.ky);
    det.abs().sqrt()
}

/// How far a stroke's outline reaches from its centerline, in path-space
/// units. tiny_skia strokes in path space before applying the transform and
/// expands the outline by up to `width/2 · miter_limit` at miter joins.
fn stroke_reach(stroke: &tiny_skia::Stroke) -> f64 {
    f64::from(stroke.width.abs()) / 2.0 * f64::from(stroke.miter_limit.max(1.0))
}

/// Narrows a stroke whose outline would reach past
/// [`MAX_DEVICE_STROKE_REACH`], returning `None` when it already fits. A
/// line that wide covers everything the pixmap can show anyway, so the width
/// is capped rather than the stroke dropped: the only pixels this changes
/// are further from the centerline than any pixmap reaches.
fn clamp_stroke_reach(stroke: &tiny_skia::Stroke, transform: tiny_skia::Transform) -> Option<tiny_skia::Stroke> {
    let reach = stroke_reach(stroke) * device_scale(transform);
    if !reach.is_finite() || reach <= MAX_DEVICE_STROKE_REACH {
        return None;
    }
    let mut clamped = stroke.clone();
    clamped.width = (f64::from(stroke.width.abs()) * MAX_DEVICE_STROKE_REACH / reach) as f32;
    Some(clamped)
}

/// Choke point for handing a fill to tiny_skia: skips the draw when it
/// misses the pixmap or its device-space bounds are beyond what the
/// rasterizer can carry. All content-derived fills must route through here
/// (or the mask/stroke siblings) rather than calling tiny_skia directly.
pub(crate) fn guarded_fill_path(
    pixmap: &mut tiny_skia::Pixmap,
    path: &tiny_skia::Path,
    paint: &Paint<'_>,
    fill_rule: tiny_skia::FillRule,
    transform: tiny_skia::Transform,
    clip_mask: Option<&tiny_skia::Mask>,
) {
    // TRACE, not WARN: this chokepoint runs per path draw — tens of
    // thousands of times per page on vector-heavy content — so a WARN here
    // would spam every build. Same rationale applies to the stroke and mask
    // siblings below. ~keep
    if !rasterizable_on(path, transform, 0.0, pixmap.width(), pixmap.height()) {
        tracing::trace!("skipping unrasterizable draw: {:?}", path.bounds());
        return;
    }
    pixmap.fill_path(path, paint, fill_rule, transform, clip_mask);
}

/// Stroke sibling of [`guarded_fill_path`]. A line width whose outline runs
/// past the rasterizer's reach is narrowed to one it can carry rather than
/// costing the whole stroke.
pub(crate) fn guarded_stroke_path(
    pixmap: &mut tiny_skia::Pixmap,
    path: &tiny_skia::Path,
    paint: &Paint<'_>,
    stroke: &tiny_skia::Stroke,
    transform: tiny_skia::Transform,
    clip_mask: Option<&tiny_skia::Mask>,
) {
    let clamped = clamp_stroke_reach(stroke, transform);
    let stroke = clamped.as_ref().unwrap_or(stroke);
    let reach = stroke_reach(stroke) * device_scale(transform);
    if !rasterizable_on(path, transform, reach, pixmap.width(), pixmap.height()) {
        tracing::trace!("skipping unrasterizable stroke: {:?}", path.bounds());
        return;
    }
    pixmap.stroke_path(path, paint, stroke, transform, clip_mask);
}

/// Mask sibling of [`guarded_fill_path`]. A skipped path leaves the mask
/// untouched, which for coverage masks means zero coverage — consistent
/// with the paint itself being skipped.
pub(crate) fn guarded_mask_fill_path(
    mask: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    transform: tiny_skia::Transform,
) {
    if !rasterizable_on(path, transform, 0.0, mask.width(), mask.height()) {
        tracing::trace!("skipping unrasterizable draw: {:?}", path.bounds());
        return;
    }
    mask.fill_path(path, fill_rule, anti_alias, transform);
}

/// Returns `Some(mode)` when the PDF blend mode name is one of the four
/// non-separable modes that tiny_skia cannot express natively. The
/// caller renders the paint into a fresh scratch pixmap with Normal
/// blending, then dispatches per-pixel composition via
/// [`blend_nonsep::compose_in_place`].
pub(crate) fn pdf_blend_mode_is_nonseparable(mode: &str) -> Option<blend_nonsep::NonSeparableBlend> {
    blend_nonsep::NonSeparableBlend::from_name(mode)
}

/// Run `paint` into a fresh scratch pixmap (same dimensions as `dest`)
/// with Normal blending, then per-pixel compose the scratch onto `dest`
/// using the given non-separable mode per ISO 32000-1:2008 §11.3.5.3.
///
/// `paint` is a closure that paints into the supplied scratch pixmap
/// using the rasteriser's normal code path; the closure must NOT set
/// the non-separable blend mode on its paint object (the dispatcher
/// substitutes Normal so the scratch captures only the source
/// contribution).
pub(crate) fn paint_with_nonsep_blend<F>(dest: &mut tiny_skia::Pixmap, mode: blend_nonsep::NonSeparableBlend, paint: F)
where
    F: FnOnce(&mut tiny_skia::Pixmap),
{
    let w = dest.width();
    let h = dest.height();
    let mut scratch = match tiny_skia::Pixmap::new(w, h) {
        Some(p) => p,
        None => {
            // Allocation failed — fall back to direct paint (degraded
            // mode is SourceOver, which is what the legacy dispatch
            // does for non-separable modes). Better than panic. ~keep
            paint(dest);
            return;
        }
    };
    paint(&mut scratch);
    blend_nonsep::compose_in_place(dest.data_mut(), scratch.data(), mode);
}

/// Render a PDF page to an image.
///
/// This is a convenience function that creates a PageRenderer and renders
/// a single page.
///
/// # Arguments
///
/// * `doc` - The PDF document
/// * `page_num` - Zero-based page number
/// * `options` - Rendering options (DPI, format, etc.)
///
/// # Returns
///
/// The rendered image as bytes in the specified format.
pub fn render_page(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    options: &RenderOptions,
) -> Result<RenderedImage> {
    render_page_impl(doc, page_num, options, None)
        .map(|(image, _, _)| image)
        .map_err(|(error, _)| error)
}

/// Internal Core bridge for admitting the per-page canvas, sidecar planes and PNG allowance.
#[doc(hidden)]
pub fn render_png_page_with_budget(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    dpi: u32,
    max_working_bytes: usize,
) -> Result<RenderedImage> {
    render_png_page_with_dpi_budget(doc, page_num, dpi, dpi, max_working_bytes).map(|(image, _)| image)
}

/// Internal Core bridge for selecting the highest admitted PNG raster DPI at or above the floor.
#[doc(hidden)]
pub fn render_png_page_with_dpi_budget(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    dpi: u32,
    minimum_dpi: u32,
    max_working_bytes: usize,
) -> Result<(RenderedImage, u32)> {
    render_png_page_with_ink_evidence(doc, page_num, dpi, minimum_dpi, max_working_bytes)
        .map(|(image, dpi, _)| (image, dpi))
}

/// Internal Core bridge returning whether a Type0 glyph lost ink during this render.
#[doc(hidden)]
pub fn render_png_page_with_ink_evidence(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    dpi: u32,
    minimum_dpi: u32,
    max_working_bytes: usize,
) -> Result<(RenderedImage, u32, bool)> {
    render_png_page_with_budget_evidence(doc, page_num, dpi, minimum_dpi, max_working_bytes).map_err(|(error, _)| error)
}

/// Internal Core bridge. The error flag identifies a reported source-image budget excess.
#[doc(hidden)]
pub fn render_png_page_with_budget_evidence(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    dpi: u32,
    minimum_dpi: u32,
    max_working_bytes: usize,
) -> std::result::Result<(RenderedImage, u32, bool), (crate::Error, bool)> {
    let budget = page_renderer::PngRenderBudget {
        max_working_bytes,
        minimum_dpi,
    };
    render_page_impl(doc, page_num, &RenderOptions::with_dpi(dpi), Some(budget))
}

/// One page rendered twice at the same size: as drawn, and with every glyph left unpainted.
#[doc(hidden)]
pub struct TextInkRasters {
    pub width: u32,
    pub height: u32,
    /// Row-major luminance of the page as drawn, top-left origin.
    pub drawn: Vec<u8>,
    /// Row-major luminance of the same raster with glyph ink removed.
    pub without_glyphs: Vec<u8>,
}

/// Internal Core bridge. Where the two rasters differ, glyph paints made the pixel; where
/// `without_glyphs` varies, something other than glyphs did.
#[doc(hidden)]
pub fn render_text_ink_rasters(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    dpi: u32,
    max_working_bytes: usize,
) -> Result<TextInkRasters> {
    let render = |without_glyphs: bool| -> Result<RenderedImage> {
        let budget = page_renderer::PngRenderBudget {
            max_working_bytes,
            minimum_dpi: dpi,
        };
        let _scope = crate::extractors::recovery_tally::RecoveryScope::enter(doc.recovery_counts());
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(dpi).as_raw());
        if without_glyphs {
            renderer = renderer.without_text_ink();
        }
        renderer
            .render_page_with_budget(page_num, doc, Some(budget))
            .map(|(image, _)| image)
    };
    let luminance = |image: &RenderedImage| -> Vec<u8> {
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| text_contrast_probe::luminance(pixel))
            .collect()
    };
    let drawn = render(false)?;
    let without = render(true)?;
    if (drawn.width, drawn.height) != (without.width, without.height) {
        return Err(crate::Error::InvalidOperation(
            "glyph-free render differs in size from the drawn render".to_string(),
        ));
    }
    Ok(TextInkRasters {
        width: drawn.width,
        height: drawn.height,
        drawn: luminance(&drawn),
        without_glyphs: luminance(&without),
    })
}

/// Check for actual pixel contrast in each glyph region of an audited literal text show.
#[doc(hidden)]
pub fn exact_text_show_has_contrast(
    doc: &crate::document::PdfDocument,
    literal: &[u8],
    glyphs: &[crate::geometry::Rect],
    max_working_bytes: usize,
) -> Result<bool> {
    use text_contrast_probe::{MAX_PROBE_BYTES, MAX_PROBE_PAGE_PIXELS, TextContrastProbe};

    // At 72 DPI the media box dimensions in points bound the verification raster in pixels.
    const PROBE_DPI: u32 = 72;
    let page = doc.get_page_info(0)?;
    let area = page.media_box.width.ceil() * page.media_box.height.ceil();
    let images = doc.page_image_handles(0)?;
    let [image] = images.as_slice() else { return Ok(false) };
    let Some(max_working_bytes) = image
        .contrast_jpeg_filter_reservation()
        .and_then(|reserved| max_working_bytes.checked_sub(reserved))
        .and_then(|remaining| remaining.checked_sub(MAX_PROBE_BYTES))
    else {
        return Ok(false);
    };
    let Some(probe) = TextContrastProbe::new(literal, glyphs) else {
        return Ok(false);
    };
    if !(0.0..=MAX_PROBE_PAGE_PIXELS).contains(&area) {
        return Ok(false);
    }
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(PROBE_DPI));
    renderer.text_contrast_probe = Some(probe);
    let budget = page_renderer::PngRenderBudget {
        max_working_bytes,
        minimum_dpi: PROBE_DPI,
    };
    renderer.render_page_with_budget(0, doc, Some(budget))?;
    Ok(renderer.text_contrast_probe.is_some_and(|probe| probe.visible()))
}

#[tracing::instrument(name = "pdf.render_page", skip_all, fields(page = page_num, dpi = options.dpi))]
fn render_page_impl(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    options: &RenderOptions,
    png_budget: Option<page_renderer::PngRenderBudget>,
) -> std::result::Result<(RenderedImage, u32, bool), (crate::Error, bool)> {
    // The rasterizer decodes text through the same font paths as extraction, so
    // it needs the document in scope for those recoveries to be counted rather
    // than lost (GH#1547). ~keep
    let _recovery_scope = crate::extractors::recovery_tally::RecoveryScope::enter(doc.recovery_counts());
    let mut renderer = PageRenderer::new(options.clone());
    let result = renderer.render_page_with_budget(page_num, doc, png_budget);
    if let Err(error) = &result {
        crate::error::trace_failure("render_page", error);
    }
    result
        .map(|(image, dpi)| (image, dpi, renderer.type0_ink_loss()))
        .map_err(|error| (error, renderer.reported_source_budget_exceeded))
}

/// Render a rectangular region of a page. `crop_rect_pt` is in PDF
/// user-space points (origin bottom-left of the page). The crop is
/// applied to the fully-rendered image at the requested DPI.
pub fn render_page_region(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    crop_rect_pt: (f32, f32, f32, f32),
    options: &RenderOptions,
) -> Result<RenderedImage> {
    // Full-page render first — the crop is a post-process on the
    // resulting raster. Wasteful if the crop is tiny, but matches
    // the semantics of every PDF viewer and avoids a parallel
    // clipped-raster code path in tiny-skia. ~keep
    let full = render_page(doc, page_num, options)?;

    let (crop_x_pt, crop_y_pt, crop_w_pt, crop_h_pt) = crop_rect_pt;
    if crop_w_pt <= 0.0 || crop_h_pt <= 0.0 {
        return Err(crate::Error::InvalidPdf(format!("invalid crop rect: {crop_rect_pt:?}")));
    }

    let media = doc.get_page_media_box(page_num)?;
    let page_h_pt = media.3 - media.1;

    // Points → pixels at the render DPI. ~keep
    let scale = options.dpi as f32 / 72.0;
    let crop_x_px = (crop_x_pt * scale).round().max(0.0) as u32;
    // Image Y is top-left origin; PDF Y is bottom-left. Flip. ~keep
    let top_y_pt = page_h_pt - (crop_y_pt + crop_h_pt);
    let crop_y_px = (top_y_pt * scale).round().max(0.0) as u32;
    let crop_w_px = (crop_w_pt * scale).round().max(1.0) as u32;
    let crop_h_px = (crop_h_pt * scale).round().max(1.0) as u32;

    // Decode, crop, re-encode using the `image` crate (already a dep). ~keep
    let full_img = image::load_from_memory(&full.data)
        .map_err(|e| crate::Error::InvalidPdf(format!("render output decode: {e}")))?;
    let x = crop_x_px.min(full_img.width().saturating_sub(1));
    let y = crop_y_px.min(full_img.height().saturating_sub(1));
    let w = crop_w_px.min(full_img.width() - x);
    let h = crop_h_px.min(full_img.height() - y);
    let cropped = full_img.crop_imm(x, y, w, h);

    let mut buf = Vec::new();
    match options.format {
        ImageFormat::Jpeg => {
            use image::ImageEncoder;
            let encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, options.jpeg_quality.clamp(1, 100));
            encoder
                .write_image(cropped.as_bytes(), w, h, cropped.color().into())
                .map_err(|e| crate::Error::InvalidPdf(format!("jpeg encode: {e}")))?;
        }
        _ => {
            use image::ImageEncoder;
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Sub)
                .write_image(cropped.as_bytes(), w, h, cropped.color().into())
                .map_err(|e| crate::Error::InvalidPdf(format!("png encode: {e}")))?;
        }
    }
    Ok(RenderedImage {
        data: buf,
        width: w,
        height: h,
        format: full.format,
    })
}

/// Render a page to fit inside a target bounding box (in pixels),
/// preserving aspect ratio. Picks the DPI that makes the larger of
/// the two page dimensions match the smaller bounding-box side.
pub fn render_page_fit(
    doc: &crate::document::PdfDocument,
    page_num: usize,
    fit_w_px: u32,
    fit_h_px: u32,
    options: &RenderOptions,
) -> Result<RenderedImage> {
    if fit_w_px == 0 || fit_h_px == 0 {
        return Err(crate::Error::InvalidPdf("fit width/height must be positive".into()));
    }
    let page_info = doc.get_page_info(page_num)?;
    // `%` is a remainder and preserves sign, so a legal negative /Rotate (e.g. -90,
    // equivalent to 270 per ISO 32000-1 s7.7.3.3 Table 30) matched neither 90 nor
    // 270 below and the page rendered unrotated. rem_euclid normalizes to 0..359,
    // matching get_page_rotation's own `((raw % 360) + 360) % 360` convention. ~keep
    let rotation = page_info.rotation.rem_euclid(360);
    let (page_w_pt, page_h_pt) = if rotation == 90 || rotation == 270 {
        (page_info.media_box.height.max(1.0), page_info.media_box.width.max(1.0))
    } else {
        (page_info.media_box.width.max(1.0), page_info.media_box.height.max(1.0))
    };

    // Compute scale as a float ratio to avoid integer-DPI quantization. ~keep
    let scale = (fit_w_px as f32 / page_w_pt).min(fit_h_px as f32 / page_h_pt);
    let mut opts = options.clone();
    opts.scale_override = Some(scale);
    render_page(doc, page_num, &opts)
}
