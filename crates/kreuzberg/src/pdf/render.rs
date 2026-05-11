//! PDF page rendering using pdf_oxide.

use crate::Result;
use crate::error::KreuzbergError;

/// Render a single PDF page to PNG bytes.
///
/// Returns raw PNG-encoded bytes for the specified page at the given DPI.
/// Uses pdf_oxide with tiny-skia for pure-Rust rendering.
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
    let options = pdf_oxide::rendering::RenderOptions::with_dpi(render_dpi);
    let rendered =
        pdf_oxide::rendering::render_page(&doc, page_index, &options).map_err(|e| KreuzbergError::Parsing {
            message: format!("Failed to render page {page_index}: {e}"),
            source: None,
        })?;

    Ok(rendered.data)
}
