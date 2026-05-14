//! PDF annotation types.

use super::extraction::BoundingBox;
use serde::{Deserialize, Serialize};

/// Type of PDF annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PdfAnnotationType {
    /// Sticky note / text annotation
    Text,
    /// Highlighted text region
    Highlight,
    /// Hyperlink annotation
    Link,
    /// Rubber stamp annotation
    Stamp,
    /// Underline text markup
    Underline,
    /// Strikeout text markup
    StrikeOut,
    /// Any other annotation type
    Other,
}

/// A PDF annotation extracted from a document page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PdfAnnotation {
    /// The type of annotation.
    pub annotation_type: PdfAnnotationType,
    /// Text content of the annotation (e.g., comment text, link URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Page number where the annotation appears (1-indexed).
    pub page_number: u32,
    /// Bounding box of the annotation on the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<BoundingBox>,
}
