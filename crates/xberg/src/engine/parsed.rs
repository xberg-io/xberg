//! A reduced, single-operation parse memo.
//!
//! [`ParsedDocument`] caches the cheap-but-repeated parse facts for one logical
//! extraction operation: the detected MIME type, the page count, the shared
//! source bytes, and — lazily — the per-page rendered PNGs. It deliberately
//! does **not** cache a live PDFium handle: PDFium reloads the document per page
//! under a global lock, so a retained handle buys nothing (see ADR-0049 / the
//! engine-seams plan's "REDUCED memo" decision).
//!
//! The intended use is an orchestrator that rasterizes the same document more
//! than once within a single structured-extraction call (e.g. a text attempt
//! that escalates to a vision fallback): it builds one `ParsedDocument`, then
//! calls [`ParsedDocument::rendered_pages`] wherever pages are needed, paying
//! the render cost at most once per DPI.
//!
//! This type is Rust-only and not part of the language-binding surface (see
//! `alef.toml` `[crates.exclude]`).

use std::sync::Arc;

use parking_lot::Mutex;

use crate::Result;
use crate::engine::structured::rasterize::{
    PageImage, RasterizeError, RasterizeOptions, render_all_pages, render_all_pages_with_options,
};
use crate::extractors::security::SecurityLimits;

/// MIME type reported for PDF documents.
const PDF_MIME: &str = "application/pdf";

/// A single-operation memo of a document's parse facts and rendered pages.
///
/// Cheaply shareable: the source bytes are held behind an [`Arc`], and the
/// render cache is interior-mutable so [`rendered_pages`](Self::rendered_pages)
/// can take `&self`.
pub struct ParsedDocument {
    mime_type: String,
    page_count: u32,
    source: Arc<[u8]>,
    security_limits: SecurityLimits,
    /// Memoized pages for the last DPI and image size options.
    rendered: Mutex<Option<(u32, RasterizeOptions, Vec<PageImage>)>>,
}

impl ParsedDocument {
    /// Build a memo from shared source bytes: detect the MIME type and count
    /// pages (PDF via [`crate::pdf::render::pdf_page_count`]; every other type is
    /// treated as a single page). Does not render — rendering is lazy.
    ///
    /// # Errors
    ///
    /// Propagates MIME-detection and PDF page-count failures.
    pub fn from_bytes(source: Arc<[u8]>) -> Result<Self> {
        Self::from_bytes_with_security_limits(source, &SecurityLimits::default())
    }

    /// Build a memo with caller-selected image decoding and PNG encoding limits.
    ///
    /// The limits are captured for this operation and its render cache. Only
    /// `max_content_size` applies to images; PDF rendering keeps its existing
    /// behavior and does not enforce these page or memory limits.
    ///
    /// # Errors
    ///
    /// Propagates MIME-detection and PDF page-count failures.
    pub fn from_bytes_with_security_limits(source: Arc<[u8]>, security_limits: &SecurityLimits) -> Result<Self> {
        let mime_type = crate::core::mime::detect_mime_type_from_bytes(&source)?;
        let page_count = if mime_type.eq_ignore_ascii_case(PDF_MIME) {
            crate::pdf::render::pdf_page_count(&source, None)? as u32
        } else {
            1
        };
        Ok(Self {
            mime_type,
            page_count,
            source,
            security_limits: security_limits.clone(),
            rendered: Mutex::new(None),
        })
    }

    /// The detected MIME type.
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    /// The page count (1 for non-PDF inputs).
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// The shared source bytes.
    pub fn source(&self) -> &Arc<[u8]> {
        &self.source
    }

    /// Render every page at `dpi`, memoizing the result. A second call at the
    /// same DPI returns the cached PNGs without re-rendering.
    ///
    /// # Errors
    ///
    /// Propagates [`RasterizeError`] from the underlying renderer.
    pub fn rendered_pages(&self, dpi: u32) -> std::result::Result<Vec<PageImage>, RasterizeError> {
        self.rendered_pages_with_options(dpi, RasterizeOptions::default())
    }

    /// Render with caller-selected image sizing, memoizing by both DPI and options.
    ///
    /// `RasterizeOptions::default()` retains the legacy full-resolution image
    /// behavior. The memo's captured security limits apply to every render.
    ///
    /// # Errors
    ///
    /// Propagates [`RasterizeError`] from the underlying renderer.
    pub fn rendered_pages_with_options(
        &self,
        dpi: u32,
        options: RasterizeOptions,
    ) -> std::result::Result<Vec<PageImage>, RasterizeError> {
        let mut guard = self.rendered.lock();
        if let Some((cached_dpi, cached_options, pages)) = guard.as_ref()
            && *cached_dpi == dpi
            && *cached_options == options
        {
            return Ok(pages.clone());
        }
        let pages = if options == RasterizeOptions::default() {
            render_all_pages(&self.source, &self.mime_type, dpi, &self.security_limits)?
        } else {
            render_all_pages_with_options(&self.source, &self.mime_type, dpi, &self.security_limits, options)?
        };
        *guard = Some((dpi, options, pages.clone()));
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const DPI: u32 = 150;

    /// A valid 1x1 PNG built via the `image` crate (matches the rasterizer's own
    /// test fixtures, so it always decodes).
    fn one_pixel_png() -> Vec<u8> {
        let img = image::RgbImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("encode 1x1 png");
        out
    }

    #[test]
    fn from_bytes_detects_image_and_counts_one_page() {
        let png = one_pixel_png();
        let parsed = ParsedDocument::from_bytes(Arc::from(png.as_slice())).expect("parse memo builds");
        assert!(
            parsed.mime_type().starts_with("image/"),
            "detected an image MIME, got {}",
            parsed.mime_type()
        );
        assert_eq!(parsed.page_count(), 1, "an image is a single page");
        assert_eq!(
            parsed.source().as_ref(),
            png.as_slice(),
            "memo retains the shared source bytes"
        );
    }

    #[test]
    fn rendered_pages_is_memoized_and_matches_direct_render() {
        let png = one_pixel_png();
        let parsed = ParsedDocument::from_bytes(Arc::from(png.as_slice())).expect("parse memo builds");

        let first = parsed.rendered_pages(DPI).expect("first render succeeds");
        let second = parsed.rendered_pages(DPI).expect("second render hits the memo");

        assert_eq!(first.len(), 1, "one page rendered");
        assert_eq!(first[0].page_number, 1, "1-based page numbering");
        assert_eq!(
            first[0].png_bytes, second[0].png_bytes,
            "second call returns the memoized PNG"
        );

        let direct =
            render_all_pages(&png, "image/png", DPI, &SecurityLimits::default()).expect("direct rasterize succeeds");
        assert_eq!(direct.len(), 1, "direct render yields one page");
        assert_eq!(
            first[0].png_bytes, direct[0].png_bytes,
            "memo render is identical to a direct rasterize"
        );
    }
}
