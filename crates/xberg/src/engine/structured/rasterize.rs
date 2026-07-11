//! PDF / image rasterization for the structured-extraction mechanism.
//!
//! For PDF input, pages are rendered at a caller-supplied DPI via the crate's
//! [`crate::pdf::render`] helpers. For image MIMEs, the input is decoded and
//! re-encoded as a single PNG. The module is lazy: when the call mode decides
//! `Skip` or `TextOnly`, no rasterization happens at all.
//!
//! The mechanism has no notion of object-storage persistence: there is no
//! persistence context and no storage write loop. Persisting rasters is a
//! caller concern, layered on top of the returned [`PageImage`] values.

use std::io::Cursor;

use thiserror::Error;

// Imported from heuristics — the only call-mode source of truth.
use crate::heuristics::StructuredCallMode;

/// A rendered page ready for inline-base64 transport to the vision model.
#[derive(Debug, Clone)]
pub struct PageImage {
    /// 1-based page number (matches the citation envelope convention).
    pub page_number: u32,
    /// PNG-encoded page bytes.
    pub png_bytes: Vec<u8>,
}

/// Errors produced while rendering pages.
#[derive(Debug, Error)]
pub enum RasterizeError {
    /// PDF parsing or page rendering failed.
    #[error("pdf rendering failed: {0}")]
    Pdf(String),
    /// Image decoding or PNG re-encoding failed.
    #[error("image decoding failed: {0}")]
    Image(String),
    /// The supplied MIME type is neither `application/pdf` nor `image/*`.
    #[error("unsupported mime type: {0}")]
    UnsupportedMime(String),
}

/// Render pages for the given call mode. Returns an empty vec when the
/// heuristic does not require vision (`Skip` / `TextOnly` /
/// `TextOnlyWithVisionFallback`).
///
/// `dpi` is a caller parameter; the mechanism does not impose a default.
/// `TextOnlyWithVisionFallback` is handled
/// by the orchestrator — rasterization only happens if a fallback escalates.
#[cfg_attr(alef, alef(skip))]
pub async fn pages_for_call(
    bytes: &[u8],
    mime: &str,
    mode: StructuredCallMode,
    dpi: u32,
) -> Result<Vec<PageImage>, RasterizeError> {
    let pages = match mode {
        StructuredCallMode::Skip | StructuredCallMode::TextOnly | StructuredCallMode::TextOnlyWithVisionFallback => {
            // TextOnlyWithVisionFallback is handled by the orchestrator — rasterize only if fallback escalates.
            Vec::new()
        }
        StructuredCallMode::VisionOnly | StructuredCallMode::TextPlusVision => {
            // Rendering is CPU-bound (PDF xref parse + per-page raster, or image
            // decode/re-encode). Run it on a blocking thread so a large document
            // never stalls the async runtime's worker threads.
            let bytes = bytes.to_vec();
            let mime = mime.to_string();
            #[cfg(feature = "tokio-runtime")]
            {
                tokio::task::spawn_blocking(move || render_all_pages(&bytes, &mime, dpi))
                    .await
                    .map_err(|e| RasterizeError::Pdf(format!("rasterization task failed: {e}")))??
            }
            #[cfg(not(feature = "tokio-runtime"))]
            {
                render_all_pages(&bytes, &mime, dpi)?
            }
        }
    };

    Ok(pages)
}

/// Render every page of `bytes`, regardless of call mode: PDF input renders one
/// PNG per page at `dpi`; `image/*` input decodes and re-encodes to a single
/// PNG (DPI is ignored). Anything else yields [`RasterizeError::UnsupportedMime`].
///
/// This is the rendering primitive behind both [`pages_for_call`] and the
/// [`ParsedDocument`](crate::engine::parsed::ParsedDocument) render memo.
pub(crate) fn render_all_pages(bytes: &[u8], mime: &str, dpi: u32) -> Result<Vec<PageImage>, RasterizeError> {
    let mime_lc = mime.to_ascii_lowercase();
    if mime_lc == "application/pdf" {
        render_pdf(bytes, dpi)
    } else if mime_lc.starts_with("image/") {
        render_image(bytes)
    } else {
        Err(RasterizeError::UnsupportedMime(mime.into()))
    }
}

fn render_pdf(bytes: &[u8], dpi: u32) -> Result<Vec<PageImage>, RasterizeError> {
    // Parse the document once and render every page from the same handle. The
    // expensive work is the xref/trailer parse in `open_pdf_document`; rendering
    // a page only reads the already-parsed structures, so reusing the handle
    // avoids re-parsing the file once per page.
    let document = crate::pdf::render::open_pdf_document(bytes, None)
        .map_err(|e| RasterizeError::Pdf(format!("failed to open PDF: {e}")))?;

    let page_count = crate::pdf::render::document_page_count(&document)
        .map_err(|e| RasterizeError::Pdf(format!("failed to read page count: {e}")))?;

    let mut pages = Vec::with_capacity(page_count);

    for page_idx in 0..page_count {
        let png_bytes = crate::pdf::render::render_open_pdf_page_to_png(&document, page_idx, Some(dpi as i32))
            .map_err(|e| RasterizeError::Pdf(format!("failed to render page {}: {e}", page_idx + 1)))?;

        pages.push(PageImage {
            page_number: (page_idx + 1) as u32,
            png_bytes,
        });
    }

    Ok(pages)
}

fn render_image(bytes: &[u8]) -> Result<Vec<PageImage>, RasterizeError> {
    // Decode + re-encode as PNG via the `image` crate. Single-page result.
    let img =
        image::load_from_memory(bytes).map_err(|e| RasterizeError::Image(format!("failed to decode image: {e}")))?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| RasterizeError::Image(format!("failed to encode PNG: {e}")))?;

    Ok(vec![PageImage {
        page_number: 1,
        png_bytes,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_pixel_png() -> Vec<u8> {
        // Minimal 1x1 PNG built via the image crate.
        let img = image::RgbImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[tokio::test]
    async fn skip_mode_returns_no_pages() {
        let pages = pages_for_call(&[], "application/pdf", StructuredCallMode::Skip, 200)
            .await
            .unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn text_only_mode_returns_no_pages() {
        let pages = pages_for_call(&[], "application/pdf", StructuredCallMode::TextOnly, 200)
            .await
            .unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn image_mime_returns_single_page() {
        let png = one_pixel_png();
        let pages = pages_for_call(&png, "image/png", StructuredCallMode::VisionOnly, 200)
            .await
            .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        assert!(pages[0].png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]), "PNG magic");
    }

    #[tokio::test]
    async fn unsupported_mime_errors() {
        let res = pages_for_call(&[], "application/zip", StructuredCallMode::VisionOnly, 200).await;
        assert!(matches!(res, Err(RasterizeError::UnsupportedMime(_))));
    }

    #[tokio::test]
    async fn text_only_with_vision_fallback_returns_no_pages_initially() {
        let pages = pages_for_call(
            &[],
            "application/pdf",
            StructuredCallMode::TextOnlyWithVisionFallback,
            200,
        )
        .await
        .unwrap();
        assert!(pages.is_empty());
    }

    // No multi-page PDF fixture exists under crates/xberg, so this exercises the
    // open-once batch path (`render_pdf` -> `open_pdf_document` +
    // `render_open_pdf_page_to_png`) on a single-page document and asserts the
    // output is byte-identical to the per-call public path
    // (`render_pdf_page_to_png`, which opens then delegates to the same
    // primitive). 1-based page ordering is covered by the `page_number`
    // assignment exercised here; broader multi-page ordering coverage would need
    // a real multi-page fixture (limitation).
    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn pdf_render_open_once_matches_per_call_bytes() {
        const DPI: u32 = 200;
        let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);

        let pages = render_all_pages(&pdf, "application/pdf", DPI).expect("render_all_pages should succeed");
        assert_eq!(pages.len(), 1, "single-page PDF must yield exactly one page");
        assert_eq!(pages[0].page_number, 1, "page numbering must be 1-based");
        assert!(pages[0].png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]), "PNG magic");

        let per_call = crate::pdf::render::render_pdf_page_to_png(&pdf, 0, Some(DPI as i32), None)
            .expect("per-call render should succeed");
        assert_eq!(
            pages[0].png_bytes, per_call,
            "open-once path must produce byte-identical output to the per-call path"
        );
    }
}
