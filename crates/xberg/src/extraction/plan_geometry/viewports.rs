//! Page `/VP` viewport dictionaries on the extraction result.
//!
//! A PDF page may carry an array of viewport dictionaries. Each names a
//! rectangle and, optionally, a measure dictionary: one per plan, detail or
//! elevation on a construction sheet. The values are read from the native
//! engine and are purely additive; a page without `/VP` keeps `viewports: None`.

use crate::types::{PageContent, PageViewport};

/// Attach each page's viewport dictionaries, in page order.
///
/// Reading `/VP` cannot fail the extraction: a page whose viewports cannot be
/// read keeps `viewports: None` and the failure is logged, matching how the other
/// optional native stages degrade.
pub(crate) fn attach_page_viewports(doc: &xberg_native_pdf::PdfDocument, pages: &mut [PageContent]) {
    for page in pages {
        let page_index = page.page_number.saturating_sub(1) as usize;
        match doc.get_page_viewports(page_index) {
            Ok(viewports) if viewports.is_empty() => page.viewports = None,
            Ok(viewports) => {
                page.viewports = Some(viewports.iter().map(PageViewport::from).collect());
            }
            Err(error) => {
                tracing::debug!(
                    page = page.page_number,
                    "skipping PDF viewports that could not be read: {error}"
                );
            }
        }
    }
}
