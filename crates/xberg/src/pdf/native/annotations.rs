//! Annotation extraction using the xberg_native_pdf backend.
//!
//! Maps xberg_native_pdf's `Annotation` types to Xberg's `PdfAnnotation` model,
//! extracting content text, bounding boxes, and link URIs.

use super::NativeDocument;
use crate::types::{AnnotationPoint, BoundingBox, MeasureData, PdfAnnotation, PdfAnnotationType, ProcessingWarning};

/// Extract annotations from all pages of a PDF document using xberg_native_pdf.
///
/// Iterates over every page and every annotation on each page, mapping
/// xberg_native_pdf annotation subtypes to [`PdfAnnotationType`] and collecting
/// content text and bounding boxes where available.
///
/// Widget (form field) and Popup annotations are skipped as they are not
/// user-facing content annotations.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the native document
///
/// # Returns
///
/// A `Vec<PdfAnnotation>` containing all successfully extracted annotations, and a
/// `Vec<ProcessingWarning>` describing any pages whose annotations could not be
/// read (issue #72). When the document's page count itself cannot be determined,
/// annotations are empty and a single warning is returned.
pub(crate) fn extract_annotations(doc: &mut NativeDocument) -> (Vec<PdfAnnotation>, Vec<ProcessingWarning>) {
    let (annotations, warnings, _) = extract_annotations_matching(doc, None, None, |_, _, _, _| true);
    (annotations, warnings)
}

pub(crate) fn extract_visible_free_text_annotations(
    doc: &mut NativeDocument,
    eligible_pages: &[bool],
) -> (Vec<PdfAnnotation>, Vec<ProcessingWarning>) {
    let excluded_layers = xberg_native_pdf::optional_content::compute_default_off_ocgs(&doc.doc);
    let (annotations, warnings, _) = extract_annotations_matching(
        doc,
        Some(excluded_layers),
        Some(eligible_pages),
        visible_free_text_annotation,
    );
    (annotations, warnings)
}

fn extract_annotations_matching(
    doc: &mut NativeDocument,
    excluded_layers: Option<std::collections::HashSet<String>>,
    eligible_pages: Option<&[bool]>,
    include: impl Fn(
        &xberg_native_pdf::PdfDocument,
        &xberg_native_pdf::Annotation,
        Option<(f32, f32, f32, f32)>,
        &std::collections::HashSet<String>,
    ) -> bool,
) -> (Vec<PdfAnnotation>, Vec<ProcessingWarning>, usize) {
    let page_count = match doc.doc.page_count() {
        Ok(count) => count,
        Err(e) => {
            tracing::debug!("xberg_native_pdf: failed to get page count for annotations: {e}");
            return (Vec::new(), vec![page_count_failure_warning(&e)], 0);
        }
    };

    let mut annotations = Vec::new();
    let mut warnings = Vec::new();
    let needs_visibility = excluded_layers.is_some();
    let excluded_layers = excluded_layers.unwrap_or_default();

    for page_index in 0..page_count {
        if eligible_pages.is_some_and(|pages| !pages.get(page_index).copied().unwrap_or(false)) {
            continue;
        }
        let page_number = (page_index + 1) as u32;
        let page_annotations = match doc.doc.get_annotations(page_index) {
            Ok(annots) => annots,
            Err(e) => {
                tracing::debug!(page = page_index, "xberg_native_pdf: failed to get annotations: {e}");
                warnings.push(page_annotations_failure_warning(page_number, &e));
                continue;
            }
        };
        if page_annotations.is_empty() {
            continue;
        }

        let visible_page_box = if needs_visibility {
            match doc.doc.get_page_info(page_index) {
                Ok(page) => {
                    let visible = page.crop_box.unwrap_or(page.media_box);
                    Some((
                        visible.x,
                        visible.y,
                        visible.x + visible.width,
                        visible.y + visible.height,
                    ))
                }
                Err(error) => {
                    warnings.push(page_geometry_failure_warning(page_number, &error));
                    continue;
                }
            }
        } else {
            None
        };

        for annot in page_annotations {
            if !include(&doc.doc, &annot, visible_page_box, &excluded_layers)
                || matches!(
                    annot.subtype_enum,
                    xberg_native_pdf::AnnotationSubtype::Widget | xberg_native_pdf::AnnotationSubtype::Popup
                )
            {
                continue;
            }

            annotations.push(build_pdf_annotation(&doc.doc, &annot, page_index, page_number));
        }
    }

    (annotations, warnings, page_count)
}

/// Convert one native annotation into the public `PdfAnnotation` shape.
fn build_pdf_annotation(
    doc: &xberg_native_pdf::PdfDocument,
    annot: &xberg_native_pdf::Annotation,
    page_index: usize,
    page_number: u32,
) -> PdfAnnotation {
    let annotation_type = map_annotation_subtype(annot.subtype_enum);

    let content = extract_annotation_content(annot);

    let bounding_box = annot.rect.map(|rect| BoundingBox {
        x0: rect[0],
        y0: rect[1],
        x1: rect[2],
        y1: rect[3],
    });

    let quad_points: Option<Vec<BoundingBox>> = annot
        .quad_points
        .as_ref()
        .map(|quads| quads.iter().map(quad_to_bounding_box).collect());

    let marked_text = if is_text_markup(annot.subtype_enum) {
        quad_points
            .as_deref()
            .and_then(|boxes| extract_marked_text(doc, page_index, boxes))
    } else {
        None
    };

    PdfAnnotation {
        annotation_type,
        content,
        page_number,
        bounding_box,
        author: annot.author.clone().filter(|s| !s.is_empty()),
        modified: annot.modification_date.clone().filter(|s| !s.is_empty()),
        color: annot.color.as_deref().and_then(color_to_hex),
        subject: annot.subject.clone().filter(|s| !s.is_empty()),
        quad_points,
        marked_text,
        vertices: annot
            .vertices
            .as_ref()
            .map(|vertices| vertices.iter().map(|[x, y]| AnnotationPoint { x: *x, y: *y }).collect()),
        ink_list: annot.ink_list.as_ref().map(|strokes| {
            strokes
                .iter()
                .map(|stroke| stroke.iter().map(|[x, y]| AnnotationPoint { x: *x, y: *y }).collect())
                .collect()
        }),
        measure: annot.measure.as_ref().map(MeasureData::from),
    }
}

fn visible_free_text_annotation(
    doc: &xberg_native_pdf::PdfDocument,
    annotation: &xberg_native_pdf::Annotation,
    visible_page_box: Option<(f32, f32, f32, f32)>,
    excluded_layers: &std::collections::HashSet<String>,
) -> bool {
    if annotation.subtype_enum != xberg_native_pdf::AnnotationSubtype::FreeText
        || annotation.flags.is_invisible()
        || annotation.flags.is_hidden()
        || annotation.flags.contains(xberg_native_pdf::AnnotationFlags::NO_VIEW)
        || annotation
            .opacity
            .is_some_and(|opacity| !opacity.is_finite() || opacity <= 0.0)
    {
        return false;
    }

    let (Some(rect), Some(visible_page_box)) = (annotation.rect, visible_page_box) else {
        return false;
    };
    if !rect_has_visible_page_area(rect, visible_page_box) {
        return false;
    }

    if annotation
        .raw_dict
        .as_ref()
        .and_then(|dictionary| dictionary.get("OC"))
        .is_some_and(|optional_content| {
            xberg_native_pdf::optional_content::annotation_is_excluded(optional_content, doc, excluded_layers)
        })
    {
        return false;
    }

    let Some(dictionary) = annotation.raw_dict.as_ref() else {
        return true;
    };
    if !dictionary.contains_key("AP") {
        return true;
    }
    let Some(contents) = annotation.contents.as_deref().map(normalize_annotation_text) else {
        return false;
    };
    doc.extract_annotation_appearance_text(annotation, excluded_layers)
        .as_deref()
        .map(normalize_annotation_text)
        .is_some_and(|appearance| appearance == contents)
}

fn normalize_annotation_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn rect_has_visible_page_area(rect: [f64; 4], visible_page_box: (f32, f32, f32, f32)) -> bool {
    let [x0, y0, x1, y1] = rect;
    let (page_x0, page_y0, page_x1, page_y1) = visible_page_box;
    let page = [
        f64::from(page_x0),
        f64::from(page_y0),
        f64::from(page_x1),
        f64::from(page_y1),
    ];
    if !rect.into_iter().chain(page).all(f64::is_finite) || x1 == x0 || y1 == y0 {
        return false;
    }

    let (left, right) = (x0.min(x1), x0.max(x1));
    let (bottom, top) = (y0.min(y1), y0.max(y1));
    left.max(page[0]) < right.min(page[2]) && bottom.max(page[1]) < top.min(page[3])
}

/// Whether an annotation subtype marks up existing page text via `/QuadPoints`
/// (Highlight, Underline, StrikeOut, Squiggly).
fn is_text_markup(subtype: xberg_native_pdf::AnnotationSubtype) -> bool {
    matches!(
        subtype,
        xberg_native_pdf::AnnotationSubtype::Highlight
            | xberg_native_pdf::AnnotationSubtype::Underline
            | xberg_native_pdf::AnnotationSubtype::StrikeOut
            | xberg_native_pdf::AnnotationSubtype::Squiggly
    )
}

/// Reduce a `/QuadPoints` quad (`x1,y1, x2,y2, x3,y3, x4,y4`) to its axis-aligned
/// bounding box. The PDF spec does not fix the winding order of the four
/// points, so the min/max of each axis is taken rather than assuming a
/// particular corner order.
fn quad_to_bounding_box(quad: &[f64; 8]) -> BoundingBox {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    BoundingBox {
        x0: xs.iter().copied().fold(f64::INFINITY, f64::min),
        y0: ys.iter().copied().fold(f64::INFINITY, f64::min),
        x1: xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        y1: ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

/// Recover the page text underneath a text markup annotation's `/QuadPoints`
/// boxes, one per marked line/run, joined with a single space.
///
/// Returns `None` when no box yields any text (e.g. the region only covers
/// whitespace, or `xberg_native_pdf` fails to extract text for every box).
fn extract_marked_text(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    boxes: &[BoundingBox],
) -> Option<String> {
    let mut pieces = Vec::with_capacity(boxes.len());

    for bbox in boxes {
        let region = xberg_native_pdf::geometry::Rect::from_points(
            bbox.x0 as f32,
            bbox.y0 as f32,
            bbox.x1 as f32,
            bbox.y1 as f32,
        );

        match doc.extract_text_in_rect(page_index, region, xberg_native_pdf::layout::RectFilterMode::Intersects) {
            Ok(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    pieces.push(trimmed.to_string());
                }
            }
            Err(e) => {
                tracing::debug!(
                    page = page_index,
                    "xberg_native_pdf: failed to extract marked text for annotation: {e}"
                );
            }
        }
    }

    if pieces.is_empty() {
        None
    } else {
        Some(pieces.join(" "))
    }
}

/// Convert a PDF `/C` colour array (DeviceGray, DeviceRGB, or DeviceCMYK) to a
/// CSS-compatible `#rrggbb` hex string.
fn color_to_hex(components: &[f64]) -> Option<String> {
    let to_u8 = |v: f64| -> u8 { (v.clamp(0.0, 1.0) * 255.0).round() as u8 };

    match components {
        [gray] => {
            let g = to_u8(*gray);
            Some(format!("#{g:02x}{g:02x}{g:02x}"))
        }
        [r, g, b] => Some(format!("#{:02x}{:02x}{:02x}", to_u8(*r), to_u8(*g), to_u8(*b))),
        [c, m, y, k] => {
            let r = to_u8((1.0 - c) * (1.0 - k));
            let g = to_u8((1.0 - m) * (1.0 - k));
            let b = to_u8((1.0 - y) * (1.0 - k));
            Some(format!("#{r:02x}{g:02x}{b:02x}"))
        }
        _ => None,
    }
}

/// Build the warning for issue #72's document-wide failure mode: the page count
/// itself could not be determined, so no page could even be attempted.
pub(crate) fn page_count_failure_warning(error: &xberg_native_pdf::Error) -> ProcessingWarning {
    ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_annotations"),
        message: std::borrow::Cow::Owned(format!(
            "annotation extraction failed: could not determine page count ({error}); no annotations were extracted"
        )),
    }
}

/// Build the warning for issue #72's per-page failure mode: annotations on one
/// page could not be read, but the rest of the document is still processed.
fn page_annotations_failure_warning(page_number: u32, error: &xberg_native_pdf::Error) -> ProcessingWarning {
    ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_annotations"),
        message: std::borrow::Cow::Owned(format!(
            "annotation extraction failed for page {page_number}: {error}; annotations on this page were skipped"
        )),
    }
}

fn page_geometry_failure_warning(page_number: u32, error: &xberg_native_pdf::Error) -> ProcessingWarning {
    ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_annotations"),
        message: std::borrow::Cow::Owned(format!(
            "annotation visibility could not be determined for page {page_number}: {error}; \
             annotations on this page were skipped"
        )),
    }
}

/// Map a xberg_native_pdf annotation subtype to Xberg's `PdfAnnotationType`.
fn map_annotation_subtype(subtype: xberg_native_pdf::AnnotationSubtype) -> PdfAnnotationType {
    match subtype {
        xberg_native_pdf::AnnotationSubtype::Text | xberg_native_pdf::AnnotationSubtype::FreeText => {
            PdfAnnotationType::Text
        }
        xberg_native_pdf::AnnotationSubtype::Highlight => PdfAnnotationType::Highlight,
        xberg_native_pdf::AnnotationSubtype::Link => PdfAnnotationType::Link,
        xberg_native_pdf::AnnotationSubtype::Stamp => PdfAnnotationType::Stamp,
        xberg_native_pdf::AnnotationSubtype::Underline => PdfAnnotationType::Underline,
        xberg_native_pdf::AnnotationSubtype::StrikeOut => PdfAnnotationType::StrikeOut,
        xberg_native_pdf::AnnotationSubtype::Squiggly => PdfAnnotationType::Squiggly,
        xberg_native_pdf::AnnotationSubtype::Ink => PdfAnnotationType::Ink,
        xberg_native_pdf::AnnotationSubtype::Square => PdfAnnotationType::Square,
        xberg_native_pdf::AnnotationSubtype::Circle => PdfAnnotationType::Circle,
        xberg_native_pdf::AnnotationSubtype::Polygon => PdfAnnotationType::Polygon,
        xberg_native_pdf::AnnotationSubtype::PolyLine => PdfAnnotationType::PolyLine,
        xberg_native_pdf::AnnotationSubtype::Line => PdfAnnotationType::Line,
        xberg_native_pdf::AnnotationSubtype::Caret => PdfAnnotationType::Caret,
        xberg_native_pdf::AnnotationSubtype::FileAttachment => PdfAnnotationType::FileAttachment,
        xberg_native_pdf::AnnotationSubtype::Sound => PdfAnnotationType::Sound,
        xberg_native_pdf::AnnotationSubtype::Movie => PdfAnnotationType::Movie,
        _ => PdfAnnotationType::Other,
    }
}

/// Extract content text from a xberg_native_pdf annotation.
///
/// For Link annotations, attempts to retrieve the URI from the associated
/// action. Falls back to the generic `contents` field for all types.
fn extract_annotation_content(annot: &xberg_native_pdf::Annotation) -> Option<String> {
    if annot.subtype_enum == xberg_native_pdf::AnnotationSubtype::Link
        && let Some(ref action) = annot.action
    {
        match action {
            xberg_native_pdf::LinkAction::Uri(uri) if !uri.is_empty() => {
                return Some(uri.clone());
            }
            _ => {}
        }
    }

    annot.contents.as_ref().filter(|s| !s.is_empty()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #72: a document-wide page-count failure must produce a
    /// `ProcessingWarning` (not just a `tracing::debug!` line the caller can
    /// never see) naming the root cause.
    #[test]
    fn test_page_count_failure_warning_names_root_cause() {
        let error = xberg_native_pdf::Error::InvalidPdf("corrupt xref".to_string());
        let warning = page_count_failure_warning(&error);

        assert_eq!(warning.source.as_ref(), "pdf_annotations");
        assert_eq!(
            warning.message.as_ref(),
            "annotation extraction failed: could not determine page count (Invalid PDF: corrupt xref); \
             no annotations were extracted"
        );
    }

    /// Issue #72: a single page's annotation-read failure must produce a
    /// `ProcessingWarning` naming that page, while extraction continues.
    #[test]
    fn test_page_annotations_failure_warning_names_page() {
        let error = xberg_native_pdf::Error::InvalidPdf("malformed /Annots array".to_string());
        let warning = page_annotations_failure_warning(3, &error);

        assert_eq!(warning.source.as_ref(), "pdf_annotations");
        assert_eq!(
            warning.message.as_ref(),
            "annotation extraction failed for page 3: Invalid PDF: malformed /Annots array; \
             annotations on this page were skipped"
        );
    }

    #[test]
    fn test_page_geometry_failure_warning_names_page() {
        let error = xberg_native_pdf::Error::InvalidPdf("malformed /CropBox".to_string());
        let warning = page_geometry_failure_warning(4, &error);

        assert_eq!(warning.source.as_ref(), "pdf_annotations");
        assert_eq!(
            warning.message.as_ref(),
            "annotation visibility could not be determined for page 4: Invalid PDF: malformed /CropBox; \
             annotations on this page were skipped"
        );
    }

    #[test]
    fn test_map_annotation_subtype_text() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Text),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_subtype_free_text() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::FreeText),
            PdfAnnotationType::Text
        );
    }

    #[test]
    fn test_map_annotation_subtype_highlight() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Highlight),
            PdfAnnotationType::Highlight
        );
    }

    #[test]
    fn test_map_annotation_subtype_link() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Link),
            PdfAnnotationType::Link
        );
    }

    #[test]
    fn test_map_annotation_subtype_stamp() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Stamp),
            PdfAnnotationType::Stamp
        );
    }

    #[test]
    fn test_map_annotation_subtype_underline() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Underline),
            PdfAnnotationType::Underline
        );
    }

    #[test]
    fn test_map_annotation_subtype_strikeout() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::StrikeOut),
            PdfAnnotationType::StrikeOut
        );
    }

    #[test]
    fn test_map_annotation_subtype_other() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Screen),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Watermark),
            PdfAnnotationType::Other
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Redact),
            PdfAnnotationType::Other
        );
    }

    /// Issue #63: subtypes that were previously collapsed into `Other` now each
    /// round-trip as their own distinct variant.
    #[test]
    fn test_map_annotation_subtype_previously_collapsed_variants() {
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Ink),
            PdfAnnotationType::Ink
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Square),
            PdfAnnotationType::Square
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Circle),
            PdfAnnotationType::Circle
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Polygon),
            PdfAnnotationType::Polygon
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::PolyLine),
            PdfAnnotationType::PolyLine
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Squiggly),
            PdfAnnotationType::Squiggly
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Caret),
            PdfAnnotationType::Caret
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::FileAttachment),
            PdfAnnotationType::FileAttachment
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Sound),
            PdfAnnotationType::Sound
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Movie),
            PdfAnnotationType::Movie
        );
        assert_eq!(
            map_annotation_subtype(xberg_native_pdf::AnnotationSubtype::Line),
            PdfAnnotationType::Line
        );
    }

    #[test]
    fn test_color_to_hex_gray() {
        assert_eq!(color_to_hex(&[0.5]), Some("#808080".to_string()));
    }

    #[test]
    fn test_color_to_hex_rgb() {
        assert_eq!(color_to_hex(&[1.0, 1.0, 0.0]), Some("#ffff00".to_string()));
    }

    #[test]
    fn test_color_to_hex_cmyk() {
        // Pure black in CMYK (K=1) converts to RGB black.
        assert_eq!(color_to_hex(&[0.0, 0.0, 0.0, 1.0]), Some("#000000".to_string()));
    }

    #[test]
    fn test_color_to_hex_invalid_length_returns_none() {
        assert_eq!(color_to_hex(&[0.1, 0.2]), None);
        assert_eq!(color_to_hex(&[]), None);
    }

    #[test]
    fn test_quad_to_bounding_box_computes_min_max() {
        let quad = [10.0, 20.0, 30.0, 20.0, 10.0, 40.0, 30.0, 40.0];
        let bbox = quad_to_bounding_box(&quad);
        assert_eq!(bbox.x0, 10.0);
        assert_eq!(bbox.y0, 20.0);
        assert_eq!(bbox.x1, 30.0);
        assert_eq!(bbox.y1, 40.0);
    }

    #[test]
    fn test_extract_annotation_content_uri() {
        let annot = xberg_native_pdf::Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Link".to_string()),
            subtype_enum: xberg_native_pdf::AnnotationSubtype::Link,
            contents: None,
            rect: None,
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: Some(xberg_native_pdf::LinkAction::Uri("https://example.com".to_string())),
            quad_points: None,
            color: None,
            opacity: None,
            flags: xberg_native_pdf::AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        let content = extract_annotation_content(&annot);
        assert_eq!(content, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_extract_annotation_content_fallback() {
        let annot = xberg_native_pdf::Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Text".to_string()),
            subtype_enum: xberg_native_pdf::AnnotationSubtype::Text,
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
            flags: xberg_native_pdf::AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        let content = extract_annotation_content(&annot);
        assert_eq!(content, Some("A note".to_string()));
    }
}
