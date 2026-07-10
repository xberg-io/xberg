//! PDF page rendering using pdf_oxide.

use crate::Result;
use crate::error::XbergError;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
use lopdf::{Document, ObjectId};

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

/// Maximum /Parent hops when resolving an inherited /Rotate attribute.
/// Bounds the walk so a malformed PDF with a parent cycle cannot loop forever.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
const MAX_ROTATE_INHERITANCE_DEPTH: usize = 32;

/// Resolve a page's effective /Rotate value, following /Parent inheritance
/// per the PDF spec (a page without its own /Rotate inherits from its Pages
/// ancestors). Returns `None` when no ancestor defines it.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
fn resolve_inherited_rotation(doc: &Document, page_id: ObjectId) -> Option<i64> {
    let mut dict = doc.get_object(page_id).ok()?.as_dict().ok()?;
    for _ in 0..MAX_ROTATE_INHERITANCE_DEPTH {
        if let Ok(rotate_obj) = dict.get(b"Rotate") {
            return rotate_obj.as_i64().ok();
        }
        let parent_id = dict.get(b"Parent").ok()?.as_reference().ok()?;
        dict = doc.get_object(parent_id).ok()?.as_dict().ok()?;
    }
    None
}

/// Read per-page /Rotate values for a whole document, normalized to
/// 0/90/180/270 (negative multiples of 90 are folded via `rem_euclid`).
///
/// Parses the PDF once with lopdf; a parse failure or missing attribute
/// yields 0 (no rotation) for the affected pages. lopdf's `get_pages()`
/// map is keyed by 1-based page number, which is the authoritative page
/// order (object IDs are not ordered by page).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn get_page_rotations(pdf_bytes: &[u8], page_count: usize) -> Vec<u32> {
    let mut rotations = vec![0u32; page_count];
    let Ok(doc) = Document::load_mem(pdf_bytes) else {
        return rotations;
    };
    for (page_number, page_id) in doc.get_pages() {
        let index = (page_number as usize).saturating_sub(1);
        if index >= page_count {
            continue;
        }
        if let Some(rotate_int) = resolve_inherited_rotation(&doc, page_id) {
            rotations[index] = rotate_int.rem_euclid(360) as u32;
        }
    }
    rotations
}

/// Rotate a decoded page image per the page's normalized /Rotate value.
/// No-op for 0 or non-quarter-turn values.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn rotate_dynamic_image(img: image::DynamicImage, rotation_degrees: u32) -> image::DynamicImage {
    match rotation_degrees % 360 {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    }
}

/// Rotate PNG-encoded page bytes per the page's /Rotate value.
///
/// Fast path: rotation 0 returns the input unchanged (no decode). Rotated
/// pages pay one decode + re-encode, which only happens for documents that
/// actually carry /Rotate. Returns the (possibly new) PNG bytes with the
/// post-rotation width and height.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn rotate_png_page_if_needed(
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    rotation_degrees: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    if rotation_degrees.is_multiple_of(360) {
        return Ok((png_data, width, height));
    }
    let img = image::load_from_memory(&png_data).map_err(|e| XbergError::Parsing {
        message: format!("failed to decode rendered page for rotation correction: {e}"),
        source: None,
    })?;
    let rotated = rotate_dynamic_image(img, rotation_degrees);
    let (w, h) = (rotated.width(), rotated.height());
    let mut buf = Vec::new();
    rotated
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| XbergError::Parsing {
            message: format!("failed to re-encode rotated page: {e}"),
            source: None,
        })?;
    Ok((buf, w, h))
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

/// Open (and optionally authenticate) a PDF document from raw bytes.
///
/// Parsing the cross-reference table and trailer is the expensive part of
/// working with a PDF; rendering a page only reads the already-parsed
/// structures. Callers that need several pages should open the document once
/// with this helper and reuse the returned handle across
/// [`render_open_pdf_page_to_png`] calls rather than re-opening per page.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the PDF cannot be opened or authenticated.
pub(crate) fn open_pdf_document(pdf_bytes: &[u8], password: Option<&str>) -> Result<pdf_oxide::PdfDocument> {
    let doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| XbergError::Parsing {
        message: format!("Failed to open PDF: {e}"),
        source: None,
    })?;

    if let Some(pwd) = password {
        doc.authenticate(pwd.as_bytes()).map_err(|e| XbergError::Parsing {
            message: format!("Failed to authenticate PDF: {e}"),
            source: None,
        })?;
    }

    Ok(doc)
}

/// Read the page count from an already-open document.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the page count cannot be read.
pub(crate) fn document_page_count(doc: &pdf_oxide::PdfDocument) -> Result<usize> {
    doc.page_count().map_err(|e| XbergError::Parsing {
        message: format!("Failed to read page count: {e}"),
        source: None,
    })
}

/// Render one page of an already-open document to PNG bytes via the
/// extreme-dimension DPI safeguard.
///
/// This is the per-page primitive shared by [`render_pdf_page_to_png`] (which
/// opens the document, then delegates) and batch callers that open once and
/// render every page from a single parsed handle. `page_index` is assumed to be
/// in range; out-of-range indices surface as the underlying rasterizer error.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the page cannot be rendered.
pub(crate) fn render_open_pdf_page_to_png(
    doc: &pdf_oxide::PdfDocument,
    page_index: usize,
    dpi: Option<i32>,
) -> Result<Vec<u8>> {
    let render_dpi = dpi.unwrap_or(150).max(1) as u32;
    let rendered = render_page_with_safeguards(doc, page_index, render_dpi).map_err(|e| XbergError::Parsing {
        message: format!("Failed to render page {page_index}: {e}"),
        source: None,
    })?;

    Ok(rendered.data)
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
/// Returns `XbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or rendered, or if `page_index` is out of range.
pub fn render_pdf_page_to_png(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<u8>> {
    let doc = open_pdf_document(pdf_bytes, password)?;

    let page_count = document_page_count(&doc)?;
    if page_index >= page_count {
        return Err(XbergError::Parsing {
            message: format!("Page index {page_index} out of range (document has {page_count} pages)"),
            source: None,
        });
    }

    render_open_pdf_page_to_png(&doc, page_index, dpi)
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
/// Returns `XbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or its page count read.
pub fn pdf_page_count(pdf_bytes: &[u8], password: Option<&str>) -> Result<usize> {
    let doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| XbergError::Parsing {
        message: format!("Failed to open PDF: {e}"),
        source: None,
    })?;

    if let Some(pwd) = password {
        doc.authenticate(pwd.as_bytes()).map_err(|e| XbergError::Parsing {
            message: format!("Failed to authenticate PDF: {e}"),
            source: None,
        })?;
    }

    doc.page_count().map_err(|e| XbergError::Parsing {
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
        let dpi = choose_safe_dpi(612.0, 792.0, 150);
        assert_eq!(dpi, 150);
    }

    #[test]
    fn test_choose_safe_dpi_extreme_wide_reduced() {
        let dpi = choose_safe_dpi(20000.0, 200.0, 150);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_render_pdf_page_to_png_very_wide_does_not_panic_or_hard_fail() {
        let wide_pdf = build_minimal_pdf_with_mediabox(20000.0, 300.0);
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
            matches!(err, XbergError::Parsing { .. }),
            "expected a Parsing error, got: {err:?}"
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_0_degrees_is_noop() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 0);
        assert_eq!((rotated.width(), rotated.height()), (100, 150));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_90_degrees_swaps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 90);
        assert_eq!((rotated.width(), rotated.height()), (150, 100));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_180_degrees_keeps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 180);
        assert_eq!((rotated.width(), rotated.height()), (100, 150));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_270_degrees_swaps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 270);
        assert_eq!((rotated.width(), rotated.height()), (150, 100));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_no_rotate_attribute_yields_zeroes() {
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        assert_eq!(get_page_rotations(&pdf, 1), vec![0]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_unparsable_bytes_yield_zeroes() {
        assert_eq!(get_page_rotations(b"not a pdf", 3), vec![0, 0, 0]);
    }
}
