//! PDF page rendering using pdf_oxide.

use crate::Result;
use crate::error::KreuzbergError;

/// Reasonable max pixel dimension (on either axis) for a rendered page before we
/// force a lower DPI. This prevents Pixmap allocation failures or OOM for
/// extremely wide/tall technical diagrams, CAD exports, etc. while still
/// producing a usable raster for OCR/VLM (which are robust to moderate downscaling).
///
/// Chosen as 16384px because a 20000pt-wide page at the default 150 DPI produces
/// ~41667px on the long axis (20000 * 150 / 72), which triggers Pixmap creation
/// or rasterization failures inside pdf_oxide/tiny-skia for real vector-heavy
/// content. 16384 is high enough for normal documents (A3 landscape at 300dpi ~
/// 3500px) but catches the extreme cases reported in #1078. See the regression
/// test in this module for the exact repro input that previously failed.
const MAX_RENDER_DIMENSION_PX: f32 = 16384.0;

/// Compute a safe DPI for the given page MediaBox so that the rendered pixel
/// size stays within practical limits for the underlying rasterizer (tiny-skia
/// Pixmap + path/text rasterization in pdf_oxide).
///
/// Falls back to 72 DPI minimum. Returns the (possibly reduced) DPI to use.
fn choose_safe_dpi(w_pt: f32, h_pt: f32, base_dpi: u32) -> u32 {
    if w_pt <= 0.0 || h_pt <= 0.0 {
        return base_dpi.max(72);
    }
    let scale = base_dpi as f32 / 72.0;
    let w_px = w_pt * scale;
    let h_px = h_pt * scale;
    let max_dim = w_px.max(h_px);
    if max_dim <= MAX_RENDER_DIMENSION_PX {
        return base_dpi;
    }
    let factor = MAX_RENDER_DIMENSION_PX / max_dim;
    (base_dpi as f32 * factor).max(72.0) as u32
}

/// Fetch page MediaBox (in points) with a sane Letter fallback.
fn get_page_dimensions_pt(doc: &pdf_oxide::PdfDocument, page_index: usize) -> (f32, f32) {
    doc.get_page_media_box(page_index)
        .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
        .unwrap_or((612.0, 792.0))
}

/// Render a page using safeguards for extreme dimensions (wide vector diagrams,
/// CAD sheets, etc.). This is the root-cause fix for render failures on such
/// inputs during force_ocr / VLM / layout paths.
///
/// Uses the opened document (so callers that batch multiple pages only parse once).
pub(crate) fn render_page_with_safeguards(
    doc: &pdf_oxide::PdfDocument,
    page_index: usize,
    base_dpi: u32,
) -> std::result::Result<pdf_oxide::rendering::RenderedImage, pdf_oxide::Error> {
    let (w_pt, h_pt) = get_page_dimensions_pt(doc, page_index);
    let safe_dpi = choose_safe_dpi(w_pt, h_pt, base_dpi);
    if safe_dpi != base_dpi {
        tracing::warn!(
            page = page_index + 1,
            original_dpi = base_dpi,
            effective_dpi = safe_dpi,
            width_pt = w_pt,
            height_pt = h_pt,
            "reducing render DPI for page due to extreme dimensions (wide vector-heavy PDF or similar)"
        );
    }
    let options = pdf_oxide::rendering::RenderOptions::with_dpi(safe_dpi);
    pdf_oxide::rendering::render_page(doc, page_index, &options)
}

/// Render a single PDF page to PNG bytes.
///
/// Returns raw PNG-encoded bytes for the specified page at the given DPI.
/// Uses pdf_oxide with tiny-skia for pure-Rust rendering.
///
/// For pages with extreme dimensions (very wide vector diagrams, etc.) the
/// effective DPI may be automatically reduced to avoid rasterizer failure.
/// A warning is logged when this happens.
///
/// # Arguments
///
/// * `pdf_bytes` - Raw PDF file bytes
/// * `page_index` - Zero-based page index
/// * `dpi` - Resolution in dots per inch (default: 150)
/// * `password` - Optional password for encrypted PDFs
///
/// # Errors
///
/// Returns `KreuzbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or rendered, or if `page_index` is out of range.
pub fn render_pdf_page_to_png(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<u8>> {
    let doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to open PDF: {e}"),
        source: None,
    })?;

    if let Some(pwd) = password {
        doc.authenticate(pwd.as_bytes()).map_err(|e| KreuzbergError::Parsing {
            message: format!("Failed to authenticate PDF: {e}"),
            source: None,
        })?;
    }

    let page_count = doc.page_count().map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to read page count: {e}"),
        source: None,
    })?;

    if page_index >= page_count {
        return Err(KreuzbergError::Parsing {
            message: format!("Page index {page_index} out of range (document has {page_count} pages)"),
            source: None,
        });
    }

    let render_dpi = dpi.unwrap_or(150).max(1) as u32;
    // Use the safeguarded path (wide-page fix) plus the image-only-page fast path
    // (JPEG 2000 pages pdf_oxide cannot rasterize). See `render_page_png`.
    let (png, _w, _h) = render_page_png(&doc, page_index, render_dpi)?;
    Ok(png)
}

/// Render a page to PNG bytes for raster consumers, returning `(png, width, height)`.
///
/// Fast path: image-only pages whose dominant image uses a filter pdf_oxide cannot
/// rasterize (JPEG 2000 / `/JPXDecode`) are decoded directly via the in-tree decoder
/// set (see [`crate::pdf::oxide::image_page`]) instead of rasterizing to a blank
/// page. All other pages rasterize through pdf_oxide exactly as before, with no
/// extra decode/encode work.
#[cfg(feature = "pdf")]
pub(crate) fn render_page_png(
    doc: &pdf_oxide::PdfDocument,
    page_index: usize,
    base_dpi: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    #[cfg(feature = "ocr")]
    {
        if let Some(img) = crate::pdf::oxide::image_page::render_image_only_page(doc, page_index, true) {
            return encode_dynamic_image_to_png(&img);
        }
    }
    let rendered = render_page_with_safeguards(doc, page_index, base_dpi).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to render page {}: {e}", page_index + 1),
        source: None,
    })?;
    Ok((rendered.data, rendered.width, rendered.height))
}

/// Render a page to a `DynamicImage` for the OCR / layout pipelines.
///
/// Combines two recoveries for pages pdf_oxide cannot rasterize:
/// 1. Proactive: an image-only page whose dominant image uses an unsupported filter
///    (JPEG 2000) is decoded directly, skipping a render that would come back blank.
/// 2. Safety net: if pdf_oxide renders an image-bearing page blank anyway (an image
///    it silently dropped), retry the direct decode before handing OCR a blank page.
///
/// Normal pages rasterize through pdf_oxide unchanged.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(crate) fn render_page_dynamic_image(
    doc: &pdf_oxide::PdfDocument,
    page_index: usize,
    base_dpi: u32,
) -> Result<image::DynamicImage> {
    #[cfg(feature = "ocr")]
    {
        if let Some(img) = crate::pdf::oxide::image_page::render_image_only_page(doc, page_index, true) {
            return Ok(img);
        }
    }
    let rendered = render_page_with_safeguards(doc, page_index, base_dpi).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to render page {}: {e}", page_index + 1),
        source: None,
    })?;
    let img = image::load_from_memory(&rendered.data).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to decode rendered page {}: {e}", page_index + 1),
        source: None,
    })?;
    #[cfg(feature = "ocr")]
    {
        if is_effectively_blank(&img)
            && let Some(recovered) = crate::pdf::oxide::image_page::render_image_only_page(doc, page_index, false)
        {
            return Ok(recovered);
        }
    }
    Ok(img)
}

/// PNG-encode a recovered page image, returning `(png, width, height)`.
#[cfg(all(feature = "pdf", feature = "ocr"))]
pub(crate) fn encode_dynamic_image_to_png(img: &image::DynamicImage) -> Result<(Vec<u8>, u32, u32)> {
    let (w, h) = (img.width(), img.height());
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to PNG-encode recovered page image: {e}"),
        source: None,
    })?;
    Ok((buf.into_inner(), w, h))
}

/// Whether a rendered page is effectively blank (uniform near-white or fully
/// transparent). Used to trigger the image-only-page safety net. Samples a coarse
/// grid rather than every pixel so the check stays cheap for full-resolution scans.
#[cfg(all(feature = "pdf", feature = "ocr"))]
fn is_effectively_blank(img: &image::DynamicImage) -> bool {
    use image::GenericImageView;

    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return true;
    }
    // ~64 samples per axis: enough to catch any real content, negligible cost.
    let step_x = (w / 64).max(1);
    let step_y = (h / 64).max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let px = img.get_pixel(x, y);
            // Opaque and not near-white on any channel => real content.
            if px[3] > 10 && (px[0] < 250 || px[1] < 250 || px[2] < 250) {
                return false;
            }
            x += step_x;
        }
        y += step_y;
    }
    true
}

/// Count the pages in a PDF without rendering any of them.
///
/// Opens the document and returns its page count from the PDF structure. No page
/// is rasterized, so this is cheap relative to `render_pdf_page_to_png` — use it
/// when you only need the count (e.g. to drive a render loop over the pages).
///
/// # Arguments
///
/// * `pdf_bytes` - Raw PDF file bytes
/// * `password` - Optional password for encrypted PDFs
///
/// # Errors
///
/// Returns `KreuzbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or its page count read.
pub fn pdf_page_count(pdf_bytes: &[u8], password: Option<&str>) -> Result<usize> {
    let doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to open PDF: {e}"),
        source: None,
    })?;

    if let Some(pwd) = password {
        doc.authenticate(pwd.as_bytes()).map_err(|e| KreuzbergError::Parsing {
            message: format!("Failed to authenticate PDF: {e}"),
            source: None,
        })?;
    }

    doc.page_count().map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to read page count: {e}"),
        source: None,
    })
}

/// Build a minimal valid single-page PDF with the given MediaBox (in points).
/// Used to test the wide-page / extreme-dimension safeguard in the renderer.
/// Note: the generated PDF has no content stream or /Resources. It is sufficient
/// to exercise the MediaBox-based DPI guard, but real-world wide vector diagrams
/// with complex paths may exercise additional failure modes in the rasterizer.
/// This is a known limitation of the in-memory test; a real repro PDF from #1078
/// was used during manual verification.
#[cfg(all(test, feature = "pdf"))]
pub(crate) fn build_minimal_pdf_with_mediabox(w: f32, h: f32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let obj1_offset = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    let obj2_offset = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1>>\nendobj\n");

    let obj3_offset = buf.len();
    // Note: MediaBox as array of 4 numbers [llx lly urx ury]
    let mb = format!("[0 0 {} {}]", w, h);
    buf.extend_from_slice(format!("3 0 obj\n<</Type /Page /MediaBox {} /Parent 2 0 R>>\nendobj\n", mb).as_bytes());

    let xref_offset = buf.len();

    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(b"0 4\n");
    buf.extend_from_slice(b"0000000000 65535 f \n");
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());

    buf.extend_from_slice(b"trailer\n<</Size 4 /Root 1 0 R>>\n");
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    buf
}

#[cfg(all(test, feature = "pdf"))]
mod tests {
    use super::*;

    #[test]
    fn test_choose_safe_dpi_normal_page_unchanged() {
        // A4-ish at 150 dpi -> well under the limit
        let dpi = choose_safe_dpi(612.0, 792.0, 150);
        assert_eq!(dpi, 150);
    }

    #[test]
    fn test_choose_safe_dpi_extreme_wide_reduced() {
        // Very wide single-page diagram (e.g. 20 000 pt wide)
        // 20000pt × (150/72) = 41666.6px → factor ≈ 0.393 → 59 DPI → clamped to 72
        let dpi = choose_safe_dpi(20000.0, 200.0, 150);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_render_pdf_page_to_png_very_wide_does_not_panic_or_hard_fail() {
        // This would previously trigger the exact failure mode in #1078
        // (render inside pdf_oxide fails for extreme MediaBox during OCR paths).
        let wide_pdf = build_minimal_pdf_with_mediabox(20000.0, 300.0);
        // Should succeed (possibly at reduced DPI internally) instead of returning Err.
        let res = render_pdf_page_to_png(&wide_pdf, 0, None, None);
        assert!(
            res.is_ok(),
            "wide page render should succeed thanks to safeguard, got: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_pdf_page_count_single_page() {
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let count = pdf_page_count(&pdf, None).expect("page count should succeed for a valid PDF");
        assert_eq!(count, 1, "minimal single-page PDF must report 1 page");
    }

    #[test]
    fn test_pdf_page_count_invalid_pdf_errors() {
        let err = pdf_page_count(b"not a pdf", None).expect_err("invalid PDF bytes must error");
        assert!(
            matches!(err, KreuzbergError::Parsing { .. }),
            "expected a Parsing error, got: {err:?}"
        );
    }
}

/// Build a minimal single-page PDF whose only content is one full-page image
/// XObject encoded with `/JPXDecode` (JPEG 2000). pdf_oxide cannot rasterize such a
/// page (it has no JPEG 2000 decoder), so this reproduces the #1158 blank-page bug.
#[cfg(all(test, feature = "pdf", feature = "ocr"))]
fn build_minimal_pdf_with_jpx_image(jp2: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    let mut offsets = [0usize; 6]; // indices 1..=5

    buf.extend_from_slice(b"%PDF-1.5\n");

    offsets[1] = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    offsets[2] = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1>>\nendobj\n");

    offsets[3] = buf.len();
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] \
             /Resources <</XObject <</Im0 4 0 R>>>> /Contents 5 0 R>>\nendobj\n"
        )
        .as_bytes(),
    );

    // Image XObject: raw JPEG 2000 codestream under /JPXDecode.
    offsets[4] = buf.len();
    buf.extend_from_slice(
        format!(
            "4 0 obj\n<</Type /XObject /Subtype /Image /Width {w} /Height {h} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode /Length {}>>\nstream\n",
            jp2.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(jp2);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    // Content stream draws the image scaled to the page box.
    let content = format!("q {w} 0 0 {h} 0 0 cm /Im0 Do Q\n");
    offsets[5] = buf.len();
    buf.extend_from_slice(
        format!("5 0 obj\n<</Length {}>>\nstream\n{content}endstream\nendobj\n", content.len()).as_bytes(),
    );

    let xref_offset = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n");
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets[1..=5] {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<</Size 6 /Root 1 0 R>>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );

    buf
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
mod jpx_tests {
    use super::*;

    /// A small tracked JPEG 2000 fixture (decodes to a dark logo, so any correct
    /// render is far from blank white).
    fn jp2_fixture() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/images/rust-logo-512x512-blk.jp2");
        std::fs::read(&path).unwrap_or_else(|e| panic!("jp2 fixture {} missing: {e}", path.display()))
    }

    /// The #1158 repro: a full-page JPEG 2000 page must render with real content
    /// through the public API, not a blank white bitmap.
    #[test]
    fn jpx_page_renders_non_blank_via_public_api() {
        let pdf = build_minimal_pdf_with_jpx_image(&jp2_fixture(), 512, 512);
        let png = render_pdf_page_to_png(&pdf, 0, Some(150), None).expect("JPEG 2000 page should render");
        let luma = image::load_from_memory(&png).expect("output is a valid image").to_luma8();
        let min = luma.iter().copied().min().unwrap_or(255);
        let max = luma.iter().copied().max().unwrap_or(255);
        assert!(
            min != 255 || max != 255,
            "JPEG 2000 page rendered blank white (min={min}, max={max}); pdf_oxide dropped the image and recovery did not fire"
        );
    }

    /// The proactive path fires for the unsupported (JPEG 2000) filter and decodes
    /// the page image directly.
    #[test]
    fn render_image_only_page_recovers_jpx() {
        let pdf = build_minimal_pdf_with_jpx_image(&jp2_fixture(), 512, 512);
        let doc = pdf_oxide::PdfDocument::from_bytes(pdf).expect("doc opens");
        let img = crate::pdf::oxide::image_page::render_image_only_page(&doc, 0, true)
            .expect("proactive recovery should decode the JPEG 2000 page image");
        assert!(img.width() > 0 && img.height() > 0);
    }

    /// A page with no image XObject must never trigger recovery, under either
    /// trigger, so normal vector/text pages are untouched.
    #[test]
    fn render_image_only_page_skips_non_image_page() {
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let doc = pdf_oxide::PdfDocument::from_bytes(pdf).expect("doc opens");
        assert!(crate::pdf::oxide::image_page::render_image_only_page(&doc, 0, true).is_none());
        assert!(crate::pdf::oxide::image_page::render_image_only_page(&doc, 0, false).is_none());
    }

    /// Parity with the removed `PdfPageIterator`: `pdf_page_count` +
    /// `render_pdf_page_to_png` together cover page-count and per-page rasterization
    /// (what lilbee's `rasterize_pdf` relied on), including for JPEG 2000 scans.
    #[test]
    fn page_count_and_render_cover_page_iterator_usage() {
        let pdf = build_minimal_pdf_with_jpx_image(&jp2_fixture(), 512, 512);
        assert_eq!(pdf_page_count(&pdf, None).expect("page count"), 1);
        let png = render_pdf_page_to_png(&pdf, 0, Some(150), None).expect("render");
        assert!(!png.is_empty(), "per-page PNG must not be empty");
    }
}
