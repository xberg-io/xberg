//! Mathematical formula extracted from a document.

use serde::{Deserialize, Serialize};

use super::extraction::BoundingBox;

/// A mathematical formula detected and recognized in a document.
///
/// Populated by the layout-guided formula pipeline: regions classified as
/// `LayoutClass::Formula` are routed to the formula OCR task, which returns the
/// LaTeX source for the region. The field is always present on
/// [`ExtractionResult`](super::extraction::ExtractionResult) but only populated
/// when the `layout-detection` feature is active and the document contains
/// formula regions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Formula {
    /// LaTeX source of the recognized formula, without surrounding `$$` delimiters.
    pub latex: String,

    /// Bounding box of the formula region on its page.
    pub bbox: BoundingBox,

    /// 1-indexed page number the formula appears on.
    pub page: u32,
}
