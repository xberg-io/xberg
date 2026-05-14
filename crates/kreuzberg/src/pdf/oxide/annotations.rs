//! Annotation extraction using the pdf_oxide backend.
//!
//! Maps pdf_oxide's `Annotation` types to Kreuzberg's `PdfAnnotation` model,
//! extracting content text, bounding boxes, and link URIs.

use super::OxideDocument;
use crate::types::{BoundingBox, PdfAnnotation, PdfAnnotationType};

/// Extract annotations from all pages of a PDF document using pdf_oxide.
///
/// Iterates over every page and every annotation on each page, mapping
/// pdf_oxide annotation subtypes to [`PdfAnnotationType`] and collecting
/// content text and bounding boxes where available.
///
/// Widget (form field) and Popup annotations are skipped as they are not
/// user-facing content annotations.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
///
/// # Returns
///
/// A `Vec<PdfAnnotation>` containing all successfully extracted annotations.
pub(crate) fn extract_annotations(doc: &mut OxideDocument) -> Vec<PdfAnnotation> {
    let page_count = match doc.doc.page_count() {
        Ok(count) => count,
        Err(e) => {
            tracing::debug!("pdf_oxide: failed to get page count for annotations: {e}");
            return Vec::new();
        }
    };

    let mut annotations = Vec::new();

    for page_index in 0..page_count {
        let page_number = (page_index + 1) as u32;

        let page_annotations = match doc.doc.get_annotations(page_index) {
            Ok(annots) => annots,
            Err(e) => {
                tracing::debug!(page = page_index, "pdf_oxide: failed to get annotations: {e}");
                continue;
            }
        };

        for annot in page_annotations {
            // Skip Widget (form field) and Popup annotations
            if matches!(
                annot.subtype_enum,
                pdf_oxide::AnnotationSubtype::Widget | pdf_oxide::AnnotationSubtype::Popup
            ) {
                continue;
            }

            let annotation_type = map_annotation_subtype(annot.subtype_enum);

            // Extract content: for Link annotations, try URI from action first
            let content = extract_annotation_content(&annot);

            // Extract bounding box from rect [x1, y1, x2, y2]
            let bounding_box = annot.rect.map(|rect| BoundingBox {
                x0: rect[0],
                y0: rect[1],
                x1: rect[2],
                y1: rect[3],
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

/// Map a pdf_oxide annotation subtype to Kreuzberg's `PdfAnnotationType`.
fn map_annotation_subtype(subtype: pdf_oxide::AnnotationSubtype) -> PdfAnnotationType {
    match subtype {
        pdf_oxide::AnnotationSubtype::Text | pdf_oxide::AnnotationSubtype::FreeText => PdfAnnotationType::Text,
        pdf_oxide::AnnotationSubtype::Highlight => PdfAnnotationType::Highlight,
        pdf_oxide::AnnotationSubtype::Link => PdfAnnotationType::Link,
        pdf_oxide::AnnotationSubtype::Stamp => PdfAnnotationType::Stamp,
        pdf_oxide::AnnotationSubtype::Underline => PdfAnnotationType::Underline,
        pdf_oxide::AnnotationSubtype::StrikeOut => PdfAnnotationType::StrikeOut,
        _ => PdfAnnotationType::Other,
    }
}

/// Extract content text from a pdf_oxide annotation.
///
/// For Link annotations, attempts to retrieve the URI from the associated
/// action. Falls back to the generic `contents` field for all types.
fn extract_annotation_content(annot: &pdf_oxide::Annotation) -> Option<String> {
    // For Link annotations, try to extract the URI from the action
    if annot.subtype_enum == pdf_oxide::AnnotationSubtype::Link
        && let Some(ref action) = annot.action
    {
        match action {
            pdf_oxide::LinkAction::Uri(uri) if !uri.is_empty() => {
                return Some(uri.clone());
            }
            _ => {}
        }
    }

    // Fall back to the generic annotation contents
    annot.contents.as_ref().filter(|s| !s.is_empty()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_annotation_subtype_text() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Text),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_subtype_free_text() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::FreeText),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_subtype_highlight() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Highlight),
            PdfAnnotationType::Highlight
        );
    }

    #[test]
    fn test_map_annotation_subtype_link() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Link),
            PdfAnnotationType::Link
        );
    }

    #[test]
    fn test_map_annotation_subtype_stamp() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Stamp),
            PdfAnnotationType::Stamp
        );
    }

    #[test]
    fn test_map_annotation_subtype_underline() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Underline),
            PdfAnnotationType::Underline
        );
    }

    #[test]
    fn test_map_annotation_subtype_strikeout() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::StrikeOut),
            PdfAnnotationType::StrikeOut
        );
    }

    #[test]
    fn test_map_annotation_subtype_other() {
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Ink),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Circle),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_subtype(pdf_oxide::AnnotationSubtype::Square),
            PdfAnnotationType::Other
        );
    }

    #[test]
    fn test_extract_annotation_content_uri() {
        let annot = pdf_oxide::Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Link".to_string()),
            subtype_enum: pdf_oxide::AnnotationSubtype::Link,
            contents: None,
            rect: None,
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: Some(pdf_oxide::LinkAction::Uri("https://example.com".to_string())),
            quad_points: None,
            color: None,
            opacity: None,
            flags: pdf_oxide::AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            raw_dict: None,
        };

        let content = extract_annotation_content(&annot);
        assert_eq!(content, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_extract_annotation_content_fallback() {
        let annot = pdf_oxide::Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Text".to_string()),
            subtype_enum: pdf_oxide::AnnotationSubtype::Text,
            contents: Some("A note".to_string()),
            rect: None,
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: None,
            quad_points: None,
            color: None,
            opacity: None,
            flags: pdf_oxide::AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            raw_dict: None,
        };

        let content = extract_annotation_content(&annot);
        assert_eq!(content, Some("A note".to_string()));
    }
}
