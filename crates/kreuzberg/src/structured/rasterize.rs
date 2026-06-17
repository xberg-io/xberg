//! Render document pages to PNG rasters for vision calls.
//!
//! Returns an empty vec for modes that do not require vision (`Skip` / `TextOnly` /
//! `TextOnlyWithVisionFallback`). For `VisionOnly` and `TextPlusVision`, PDFs are
//! rendered page-by-page using `crate::pdf::render::render_pdf_page_to_png` and
//! raster images are decoded and re-encoded as PNG via the `image` crate.
//!
//! CPU-bound work is offloaded to `tokio::task::spawn_blocking` when the
//! `tokio-runtime` feature is active; it runs inline otherwise (the caller is
//! expected to be on a blocking thread in that case).

use std::io::Cursor;

use crate::heuristics::StructuredCallMode;

use super::{PageImage, StructuredError, VisionConfig};

/// Render pages required by `mode`.
///
/// Returns `Ok(vec![])` when the call mode does not require vision pages
/// (`Skip`, `TextOnly`, `TextOnlyWithVisionFallback`).
///
/// # Errors
///
/// Returns [`StructuredError::Rasterize`] if PDF rendering or image decode/encode
/// fails, or [`StructuredError::UnsupportedMime`] when a vision mode is requested
/// for an unsupported MIME type.
pub async fn pages_for_call(
    bytes: &[u8],
    mime: &str,
    mode: StructuredCallMode,
    vision: &VisionConfig,
) -> Result<Vec<PageImage>, StructuredError> {
    match mode {
        StructuredCallMode::Skip
        | StructuredCallMode::TextOnly
        | StructuredCallMode::TextOnlyWithVisionFallback => Ok(vec![]),

        StructuredCallMode::VisionOnly | StructuredCallMode::TextPlusVision => {
            let mime_lc = mime.to_ascii_lowercase();
            if mime_lc == "application/pdf" {
                rasterize_pdf(bytes, vision.dpi).await
            } else if mime_lc.starts_with("image/") {
                rasterize_image(bytes).await
            } else {
                Err(StructuredError::UnsupportedMime(mime.to_string()))
            }
        }
    }
}

/// Render every page of a PDF to PNG at `dpi`.
///
/// When the `tokio-runtime` feature is active, the CPU work runs inside
/// `spawn_blocking` to avoid stalling the async executor.
async fn rasterize_pdf(bytes: &[u8], dpi: u32) -> Result<Vec<PageImage>, StructuredError> {
    #[cfg(feature = "tokio-runtime")]
    {
        let bytes = bytes.to_vec();
        tokio::task::spawn_blocking(move || render_pdf_blocking(&bytes, dpi))
            .await
            .map_err(|e| StructuredError::Rasterize(format!("spawn_blocking panicked: {e}")))?
    }
    #[cfg(not(feature = "tokio-runtime"))]
    {
        render_pdf_blocking(bytes, dpi)
    }
}

/// Synchronous PDF rendering — opens the document once to get the page count,
/// then iterates using [`crate::pdf::render::render_pdf_page_to_png`].
fn render_pdf_blocking(bytes: &[u8], dpi: u32) -> Result<Vec<PageImage>, StructuredError> {
    let doc = pdf_oxide::PdfDocument::from_bytes(bytes.to_vec())
        .map_err(|e| StructuredError::Rasterize(format!("failed to open PDF: {e}")))?;

    let page_count = doc
        .page_count()
        .map_err(|e| StructuredError::Rasterize(format!("failed to read page count: {e}")))?;

    let mut pages = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        let png_bytes =
            crate::pdf::render::render_pdf_page_to_png(bytes, page_index, Some(dpi as i32), None)
                .map_err(|e| {
                    StructuredError::Rasterize(format!(
                        "failed to render page {}: {e}",
                        page_index + 1
                    ))
                })?;

        pages.push(PageImage {
            page_number: (page_index + 1) as u32,
            png_bytes,
        });
    }

    Ok(pages)
}

/// Decode a raster image and re-encode it as PNG. Returns a single-page result.
///
/// When the `tokio-runtime` feature is active, the CPU work runs inside
/// `spawn_blocking` to avoid stalling the async executor.
async fn rasterize_image(bytes: &[u8]) -> Result<Vec<PageImage>, StructuredError> {
    #[cfg(feature = "tokio-runtime")]
    {
        let bytes = bytes.to_vec();
        tokio::task::spawn_blocking(move || render_image_blocking(&bytes))
            .await
            .map_err(|e| StructuredError::Rasterize(format!("spawn_blocking panicked: {e}")))?
    }
    #[cfg(not(feature = "tokio-runtime"))]
    {
        render_image_blocking(bytes)
    }
}

/// Synchronous image decode + PNG re-encode.
fn render_image_blocking(bytes: &[u8]) -> Result<Vec<PageImage>, StructuredError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| StructuredError::Rasterize(format!("failed to decode image: {e}")))?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| StructuredError::Rasterize(format!("failed to encode PNG: {e}")))?;

    Ok(vec![PageImage {
        page_number: 1,
        png_bytes,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 1×1 PNG using the `image` crate.
    fn one_pixel_png() -> Vec<u8> {
        let img = image::RgbImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn default_vision() -> VisionConfig {
        VisionConfig::default()
    }

    #[tokio::test]
    async fn skip_mode_returns_empty_vec() {
        let pages =
            pages_for_call(&[], "application/pdf", StructuredCallMode::Skip, &default_vision())
                .await
                .unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn text_only_mode_returns_empty_vec() {
        let pages =
            pages_for_call(&[], "application/pdf", StructuredCallMode::TextOnly, &default_vision())
                .await
                .unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn text_only_with_vision_fallback_returns_empty_vec() {
        let pages = pages_for_call(
            &[],
            "application/pdf",
            StructuredCallMode::TextOnlyWithVisionFallback,
            &default_vision(),
        )
        .await
        .unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn png_image_in_vision_only_returns_one_page() {
        let png = one_pixel_png();
        let pages =
            pages_for_call(&png, "image/png", StructuredCallMode::VisionOnly, &default_vision())
                .await
                .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        // Verify PNG magic bytes: 0x89 P N G
        assert!(
            pages[0].png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "expected PNG magic bytes"
        );
    }

    #[tokio::test]
    async fn jpeg_image_in_text_plus_vision_returns_one_page() {
        // Build a minimal JPEG via the image crate.
        let img = image::RgbImage::new(2, 2);
        let mut jpeg_bytes = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)
            .unwrap();

        let pages = pages_for_call(
            &jpeg_bytes,
            "image/jpeg",
            StructuredCallMode::TextPlusVision,
            &default_vision(),
        )
        .await
        .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        assert!(!pages[0].png_bytes.is_empty());
    }

    #[tokio::test]
    async fn unsupported_mime_returns_error() {
        let result =
            pages_for_call(&[], "application/zip", StructuredCallMode::VisionOnly, &default_vision())
                .await;
        assert!(
            matches!(result, Err(StructuredError::UnsupportedMime(ref m)) if m == "application/zip"),
            "expected UnsupportedMime(application/zip), got: {result:?}"
        );
    }

    #[tokio::test]
    async fn unsupported_mime_in_text_plus_vision_returns_error() {
        let result = pages_for_call(
            &[],
            "text/plain",
            StructuredCallMode::TextPlusVision,
            &default_vision(),
        )
        .await;
        assert!(matches!(result, Err(StructuredError::UnsupportedMime(_))));
    }

    // PDF rendering tests — gated on the `pdf` feature (which `structured` implies).
    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn minimal_pdf_vision_only_returns_one_page() {
        let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
        let pages =
            pages_for_call(&pdf, "application/pdf", StructuredCallMode::VisionOnly, &default_vision())
                .await
                .unwrap();
        assert_eq!(pages.len(), 1, "single-page PDF should yield one PageImage");
        assert_eq!(pages[0].page_number, 1);
        assert!(
            pages[0].png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "rendered page should be PNG"
        );
    }

    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn pdf_text_plus_vision_page_numbers_are_one_indexed() {
        let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
        let pages = pages_for_call(
            &pdf,
            "application/pdf",
            StructuredCallMode::TextPlusVision,
            &default_vision(),
        )
        .await
        .unwrap();
        assert!(!pages.is_empty());
        assert_eq!(pages[0].page_number, 1, "page numbers must be 1-indexed");
    }
}
