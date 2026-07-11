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
use crate::engine::structured::rasterize::{PageImage, RasterizeError, render_all_pages};

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
    /// Memoized `(dpi, pages)` from the last render. A render at a different DPI
    /// replaces it (a single operation renders at one DPI).
    rendered: Mutex<Option<(u32, Vec<PageImage>)>>,
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
    #[cfg_attr(alef, alef(skip))]
    pub fn rendered_pages(&self, dpi: u32) -> std::result::Result<Vec<PageImage>, RasterizeError> {
        let mut guard = self.rendered.lock();
        if let Some((cached_dpi, pages)) = guard.as_ref()
            && *cached_dpi == dpi
        {
            return Ok(pages.clone());
        }
        let pages = render_all_pages(&self.source, &self.mime_type, dpi)?;
        *guard = Some((dpi, pages.clone()));
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
        // Memoized: the second call returns byte-identical pages without re-rendering.
        assert_eq!(
            first[0].png_bytes, second[0].png_bytes,
            "second call returns the memoized PNG"
        );

        // The memo's render equals a direct render of the same bytes.
        let direct = render_all_pages(&png, "image/png", DPI).expect("direct rasterize succeeds");
        assert_eq!(direct.len(), 1, "direct render yields one page");
        assert_eq!(
            first[0].png_bytes, direct[0].png_bytes,
            "memo render is identical to a direct rasterize"
        );
    }
}
