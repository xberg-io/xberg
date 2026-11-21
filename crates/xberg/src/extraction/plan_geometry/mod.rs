//! Construction-drawing structure on top of page vector geometry.
//!
//! [`viewports`] reads a PDF page's `/VP` viewport dictionaries onto
//! [`PageContent::viewports`](crate::types::PageContent). The value is purely
//! additive and only appears when the PDF defines `/VP`.

#[cfg(feature = "pdf")]
pub mod viewports;
