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
    // Use the safeguarded path so public API also benefits from the wide-page fix.
    let rendered = render_page_with_safeguards(&doc, page_index, render_dpi).map_err(|e| KreuzbergError::Parsing {
        message: format!("Failed to render page {page_index}: {e}"),
        source: None,
    })?;

    Ok(rendered.data)
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
