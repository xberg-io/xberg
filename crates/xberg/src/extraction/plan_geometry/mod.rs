//! Construction-drawing structure on top of page vector geometry.
//!
//! - [`drawing_set`] parses the text conventions a drawing set uses: title-block
//!   sheet metadata, feet-and-inch dimensions and view scale ratios. It needs
//!   only text, so it is available in every build.
//! - [`viewports`] reads a PDF page's `/VP` viewport dictionaries onto
//!   [`PageContent::viewports`](crate::types::PageContent).
//! - [`write`] appends a measured Polygon annotation to an existing PDF.
//! - `regions` (opt-in, `plan-regions` feature) closes line segments into faces
//!   with areas.
//!
//! The viewport read runs for every PDF page and adds only the optional
//! `viewports` field; a page without `/VP` is unchanged. Everything else is
//! opt-in: writing needs an explicit call and the closed-region pass is off by
//! default.

pub mod drawing_set;

#[cfg(feature = "pdf")]
pub mod viewports;

#[cfg(feature = "pdf")]
pub mod write;

#[cfg(feature = "plan-regions")]
pub mod regions;
