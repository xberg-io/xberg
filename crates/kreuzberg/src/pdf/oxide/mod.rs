//! pdf_oxide backend for PDF extraction.
//!
//! Provides text extraction,
//! metadata parsing, annotation extraction, image extraction, table detection,
//! form field extraction, and font metrics for heading hierarchy detection.

pub(crate) mod annotations;
pub(crate) mod forms;
pub(crate) mod hierarchy;
/// Pure-Rust rendering of image-only pages whose image uses a filter pdf_oxide
/// cannot rasterize (JPEG 2000). Requires a JPEG 2000 decoder (`ocr` feature).
#[cfg(feature = "ocr")]
pub(crate) mod image_page;
pub(crate) mod images;
pub(crate) mod metadata;
pub(crate) mod table;
pub(crate) mod text;

use crate::Result;
use crate::error::KreuzbergError;

/// Wraps a [`pdf_oxide::PdfDocument`] with convenient constructors that map
/// pdf_oxide errors into [`KreuzbergError::Parsing`].
pub(crate) struct OxideDocument {
    pub doc: pdf_oxide::PdfDocument,
}

impl OxideDocument {
    /// Open a PDF from in-memory bytes.
    pub(crate) fn open_bytes(bytes: &[u8]) -> Result<Self> {
        let doc = pdf_oxide::PdfDocument::from_bytes(bytes.to_vec()).map_err(|e| KreuzbergError::Parsing {
            message: format!("pdf_oxide: failed to load bytes: {e}"),
            source: None,
        })?;
        Ok(Self { doc })
    }
}
