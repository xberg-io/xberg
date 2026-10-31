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
use std::num::NonZeroU32;

use thiserror::Error;

use crate::extractors::security::SecurityLimits;
use crate::heuristics::StructuredCallMode;

/// A rendered page ready for inline-base64 transport to the vision model.
#[derive(Debug, Clone)]
pub struct PageImage {
    /// 1-based page number (matches the citation envelope convention).
    pub page_number: u32,
    /// PNG-encoded page bytes.
    pub png_bytes: Vec<u8>,
}

/// Caller-selected image rendering options. PDF rendering remains DPI-driven.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RasterizeOptions {
    /// Maximum width or height of image inputs, preserving aspect ratio and
    /// never enlarging them. `None` retains the original image dimensions.
    /// The source image must still fit the caller's decoding budget.
    pub max_image_edge: Option<NonZeroU32>,
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
pub async fn pages_for_call(
    bytes: &[u8],
    mime: &str,
    mode: StructuredCallMode,
    dpi: u32,
) -> Result<Vec<PageImage>, RasterizeError> {
    pages_for_call_with_security_limits(bytes, mime, mode, dpi, &SecurityLimits::default()).await
}

/// Render pages with caller-selected image decoding and PNG encoding limits.
///
/// Only `max_content_size` applies to image inputs. PDF rendering retains its
/// existing behavior; this method does not enforce PDF page or memory limits.
/// Nonvision call modes return without decoding, regardless of the limits.
pub async fn pages_for_call_with_security_limits(
    bytes: &[u8],
    mime: &str,
    mode: StructuredCallMode,
    dpi: u32,
    security_limits: &SecurityLimits,
) -> Result<Vec<PageImage>, RasterizeError> {
    pages_for_call_with_options(bytes, mime, mode, dpi, security_limits, RasterizeOptions::default()).await
}

/// Render pages with an optional caller-selected image size cap.
///
/// Image inputs are decoded within `security_limits`, then reduced before PNG
/// encoding. The resize accounts for both pixel buffers while they coexist;
/// encoding applies the budget again after the source buffer is released.
/// Smaller images are unchanged. PDF input and nonvision modes retain their
/// existing behavior. No model-specific size limit is selected by this API.
///
/// A downstream caller can pass its existing `max_page_edge` policy:
/// ```no_run
/// # use std::num::NonZeroU32;
/// # use xberg::engine::structured::rasterize::{PageImage, RasterizeError, RasterizeOptions, pages_for_call_with_options};
/// # use xberg::extractors::security::SecurityLimits;
/// # use xberg::heuristics::StructuredCallMode;
/// # async fn render(bytes: &[u8], max_page_edge: NonZeroU32) -> Result<Vec<PageImage>, RasterizeError> {
/// let options = RasterizeOptions {
///     max_image_edge: Some(max_page_edge),
/// };
/// pages_for_call_with_options(
///     bytes, "image/jpeg", StructuredCallMode::VisionOnly, 200,
///     &SecurityLimits::default(), options,
/// ).await
/// # }
/// ```
pub async fn pages_for_call_with_options(
    bytes: &[u8],
    mime: &str,
    mode: StructuredCallMode,
    dpi: u32,
    security_limits: &SecurityLimits,
    options: RasterizeOptions,
) -> Result<Vec<PageImage>, RasterizeError> {
    let pages = match mode {
        StructuredCallMode::Skip | StructuredCallMode::TextOnly | StructuredCallMode::TextOnlyWithVisionFallback => {
            Vec::new()
        }
        StructuredCallMode::VisionOnly | StructuredCallMode::TextPlusVision => {
            let bytes = bytes.to_vec();
            let mime = mime.to_string();
            let security_limits = security_limits.clone();
            #[cfg(feature = "tokio-runtime")]
            {
                tokio::task::spawn_blocking(move || {
                    render_all_pages_with_options(&bytes, &mime, dpi, &security_limits, options)
                })
                .await
                .map_err(|e| RasterizeError::Pdf(format!("rasterization task failed: {e}")))??
            }
            #[cfg(not(feature = "tokio-runtime"))]
            {
                render_all_pages_with_options(&bytes, &mime, dpi, &security_limits, options)?
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
pub(crate) fn render_all_pages(
    bytes: &[u8],
    mime: &str,
    dpi: u32,
    security_limits: &SecurityLimits,
) -> Result<Vec<PageImage>, RasterizeError> {
    render_all_pages_with_options(bytes, mime, dpi, security_limits, RasterizeOptions::default())
}

pub(crate) fn render_all_pages_with_options(
    bytes: &[u8],
    mime: &str,
    dpi: u32,
    security_limits: &SecurityLimits,
    options: RasterizeOptions,
) -> Result<Vec<PageImage>, RasterizeError> {
    let mime_lc = mime.to_ascii_lowercase();
    if mime_lc == "application/pdf" {
        render_pdf(bytes, dpi)
    } else if mime_lc.starts_with("image/") {
        render_image(bytes, security_limits, options)
    } else {
        Err(RasterizeError::UnsupportedMime(mime.into()))
    }
}

fn render_pdf(bytes: &[u8], dpi: u32) -> Result<Vec<PageImage>, RasterizeError> {
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

fn render_image(
    bytes: &[u8],
    security_limits: &SecurityLimits,
    options: RasterizeOptions,
) -> Result<Vec<PageImage>, RasterizeError> {
    let img = crate::extraction::image_decode::decode_standard_image_with_security_limits(bytes, security_limits)
        .map_err(|error| RasterizeError::Image(format!("failed to decode image: {error}")))?;
    let img = resize_image(img, security_limits, options)?;
    const PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
    const PNG_ENCODE_FIXED_OVERHEAD_BYTES: u64 = 256 * 1024;
    crate::extraction::image_decode::validate_dynamic_image_additional_live_bytes(
        &img,
        security_limits,
        PNG_ENCODE_BYTES_PER_PIXEL,
        PNG_ENCODE_FIXED_OVERHEAD_BYTES,
    )
    .map_err(|error| RasterizeError::Image(format!("failed to encode image within security budget: {error}")))?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| RasterizeError::Image(format!("failed to encode PNG: {e}")))?;

    Ok(vec![PageImage {
        page_number: 1,
        png_bytes,
    }])
}

fn resize_image(
    image: image::DynamicImage,
    limits: &SecurityLimits,
    options: RasterizeOptions,
) -> Result<image::DynamicImage, RasterizeError> {
    let Some(edge) = options.max_image_edge.map(NonZeroU32::get) else {
        return Ok(image);
    };
    let longest = image.width().max(image.height());
    if longest <= edge {
        return Ok(image);
    }
    let scaled = |dimension: u32| {
        let value = ((u64::from(dimension) * u64::from(edge) + u64::from(longest) / 2) / u64::from(longest)).max(1);
        u32::try_from(value).map_err(|error| RasterizeError::Image(format!("invalid resized dimension: {error}")))
    };
    let (width, height) = (scaled(image.width())?, scaled(image.height())?);
    let source_bytes = u64::try_from(image.as_bytes().len())
        .map_err(|error| RasterizeError::Image(format!("image buffer size is not addressable: {error}")))?;
    let destination_bytes =
        crate::extraction::image_decode::decoded_byte_count(width, height, u64::from(image.color().bytes_per_pixel()))
            .map_err(|error| RasterizeError::Image(format!("failed to size resized image: {error}")))?;
    crate::extraction::image_decode::validate_image_live_bytes(
        image.width(),
        image.height(),
        source_bytes,
        destination_bytes,
        limits,
    )
    .map_err(|error| RasterizeError::Image(format!("failed to resize image within security budget: {error}")))?;

    // Thumbnail sampling allocates only the destination pixel buffer; the
    // filtered resize path additionally allocates a full-width float image.
    Ok(image.thumbnail_exact(width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_pixel_png() -> Vec<u8> {
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

    #[test]
    fn image_rasterization_rejects_oversized_declared_dimensions_before_decode() {
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

        let error = render_all_pages(&bytes, "image/bmp", 200, &SecurityLimits::default())
            .expect_err("structured image rasterization must use the default decode budget");

        assert!(matches!(error, RasterizeError::Image(_)));
        assert!(error.to_string().contains("6000x6000"));
        assert!(error.to_string().contains("security_limits.max_content_size"));
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

    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn pdf_render_open_once_matches_per_call_bytes() {
        const DPI: u32 = 200;
        let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);

        let pages = render_all_pages(&pdf, "application/pdf", DPI, &SecurityLimits::default())
            .expect("render_all_pages should succeed");
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

#[cfg(test)]
#[path = "rasterize_security_tests.rs"]
mod security_tests;

#[cfg(test)]
#[path = "rasterize_options_tests.rs"]
mod options_tests;
