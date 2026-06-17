//! PDF annotation extraction using pdfium-render.
//!
//! Extracts annotations (text notes, highlights, links, stamps, underlines,
//! strikeouts, etc.) from all pages of a PDF document.

use crate::types::{BoundingBox, PdfAnnotation, PdfAnnotationType};
use pdfium_render::prelude::*;

/// Extract annotations from all pages of a PDF document.
///
/// Iterates over every page and every annotation on each page, mapping
/// pdfium annotation subtypes to [`PdfAnnotationType`] and collecting
/// content text and bounding boxes where available.
///
/// Annotations that cannot be read are silently skipped.
///
/// # Arguments
///
/// * `document` - A reference to the loaded pdfium `PdfDocument`.
///
/// # Returns
///
/// A `Vec<PdfAnnotation>` containing all successfully extracted annotations.
pub(crate) fn extract_annotations_from_document(document: &PdfDocument<'_>) -> Vec<PdfAnnotation> {
    let mut annotations = Vec::new();

    for (page_index, page) in document.pages().iter().enumerate() {
        let page_number = page_index + 1;
        let page_annotations = page.annotations();

        for annotation in page_annotations.iter() {
            // Skip widget (form field) and popup annotations -- they are not
            // user-facing content annotations.
            let pdfium_type = annotation.annotation_type();
            if matches!(
                pdfium_type,
                PdfPageAnnotationType::Widget | PdfPageAnnotationType::XfaWidget | PdfPageAnnotationType::Popup
            ) {
                continue;
            }

            let annotation_type = map_annotation_type(pdfium_type);

            // Extract content text. For link annotations, try to get the URI.
            let content = extract_annotation_content(&annotation);

            // Extract bounding box.
            let bounding_box = annotation.bounds().ok().map(|rect| BoundingBox {
                x0: rect.left().value as f64,
                y0: rect.bottom().value as f64,
                x1: rect.right().value as f64,
                y1: rect.top().value as f64,
            });

            annotations.push(PdfAnnotation {
                annotation_type,
                content,
                page_number,
                bounding_box,
            });
        }
    }

    annotations
}

/// Map a pdfium annotation subtype to our `PdfAnnotationType` enum.
fn map_annotation_type(pdfium_type: PdfPageAnnotationType) -> PdfAnnotationType {
    match pdfium_type {
        PdfPageAnnotationType::Text => PdfAnnotationType::Text,
        PdfPageAnnotationType::FreeText => PdfAnnotationType::Text,
        PdfPageAnnotationType::Highlight => PdfAnnotationType::Highlight,
        PdfPageAnnotationType::Link => PdfAnnotationType::Link,
        PdfPageAnnotationType::Stamp => PdfAnnotationType::Stamp,
        PdfPageAnnotationType::Underline => PdfAnnotationType::Underline,
        PdfPageAnnotationType::Strikeout => PdfAnnotationType::StrikeOut,
        _ => PdfAnnotationType::Other,
    }
}

/// Extract content text from an annotation.
///
/// For link annotations, attempts to retrieve the URI from the associated
/// action. Falls back to the generic `contents()` method for all types.
fn extract_annotation_content(annotation: &PdfPageAnnotation<'_>) -> Option<String> {
    // For link annotations, try to extract the URI.
    if let Some(link_annot) = annotation.as_link_annotation()
        && let Ok(link) = link_annot.link()
        && let Some(action) = link.action()
        && let Some(uri_action) = action.as_uri_action()
        && let Ok(uri) = uri_action.uri()
        && !uri.is_empty()
    {
        return Some(uri);
    }

    // Fall back to the generic annotation contents.
    let contents = annotation.contents();
    contents.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_annotation_type_text() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Text),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_type_free_text() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::FreeText),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_type_highlight() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Highlight),
            PdfAnnotationType::Highlight
        );
    }

    #[test]
    fn test_map_annotation_type_link() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Link),
            PdfAnnotationType::Link
        );
    }

    #[test]
    fn test_map_annotation_type_stamp() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Stamp),
            PdfAnnotationType::Stamp
        );
    }

    #[test]
    fn test_map_annotation_type_underline() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Underline),
            PdfAnnotationType::Underline
        );
    }

    #[test]
    fn test_map_annotation_type_strikeout() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Strikeout),
            PdfAnnotationType::StrikeOut
        );
    }

    #[test]
    fn test_map_annotation_type_other() {
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Ink),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Circle),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_type(PdfPageAnnotationType::Square),
            PdfAnnotationType::Other
        );
    }
}
