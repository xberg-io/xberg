//! Append a measured Polygon annotation to an existing PDF.
//!
//! This is the xberg-facing wrapper around the native writer. It takes the
//! public annotation shape, copies the measure dictionary verbatim, and returns
//! the complete PDF bytes as an incremental update: the original bytes and every
//! other object are preserved.

use crate::types::{AnnotationPoint, BoundingBox, MeasureData};
use crate::{Result, XbergError};

/// A Polygon annotation to append to an existing PDF.
#[derive(Debug, Clone, Default)]
pub struct PolygonAnnotationInput {
    /// Vertices in PDF page points with y increasing upwards. At least three are
    /// required.
    pub vertices: Vec<AnnotationPoint>,
    /// Explicit rectangle; `None` computes the tight bounds of the vertices.
    pub rect: Option<BoundingBox>,
    /// Annotation text (PDF `/Contents`).
    pub contents: Option<String>,
    /// Annotation subject (PDF `/Subj`).
    pub subject: Option<String>,
    /// Measure dictionary copied onto the annotation (PDF `/Measure`).
    pub measure: Option<MeasureData>,
    /// Colour components (PDF `/C`): gray, RGB or CMYK values in `0.0..=1.0`.
    pub color: Option<Vec<f64>>,
}

/// Append `annotation` to `page_index` (zero-based) and return the complete PDF.
///
/// # Errors
///
/// Returns an error when the input is not a readable PDF, the page does not
/// exist, the document is encrypted (writing unencrypted bytes would not open in
/// the source context), or the geometry is not representable: fewer than three
/// vertices, a non-finite vertex or rect, or a colour component outside
/// `0.0..=1.0`.
pub fn append_polygon_annotation(
    pdf: &[u8],
    page_index: usize,
    annotation: &PolygonAnnotationInput,
) -> Result<Vec<u8>> {
    let document = xberg_native_pdf::PdfDocument::from_bytes(pdf.to_vec())
        .map_err(|error| XbergError::Other(format!("could not open the PDF for annotation writing: {error}")))?;

    let mut native =
        xberg_native_pdf::PolygonAnnotation::new(annotation.vertices.iter().map(|point| [point.x, point.y]).collect());
    native.rect = annotation
        .rect
        .map(|bounds| [bounds.x0, bounds.y0, bounds.x1, bounds.y1]);
    native.contents = annotation.contents.clone();
    native.subject = annotation.subject.clone();
    native.color = annotation.color.clone();
    native.measure = annotation.measure.as_ref().map(to_native_measure);

    document
        .append_polygon_annotation(page_index, &native)
        .map_err(|error| XbergError::Other(format!("could not append the polygon annotation: {error}")))
}

fn to_native_measure(measure: &MeasureData) -> xberg_native_pdf::MeasureDictionary {
    xberg_native_pdf::MeasureDictionary {
        subtype: measure.subtype.clone(),
        scale_ratio: measure.scale_ratio.clone(),
        x: measure.x.clone(),
        y: measure.y.clone(),
        distance: measure.distance.clone(),
        area: measure.area.clone(),
        angle: measure.angle.clone(),
        slope: measure.slope.clone(),
        origin: measure.origin.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal one-page PDF: catalog, pages node, page and empty content stream.
    fn skeleton_pdf() -> Vec<u8> {
        let objects = [
            (1u32, "<< /Type /Catalog /Pages 2 0 R >>"),
            (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>",
            ),
            (4, "<< /Length 0 >>\nstream\n\nendstream"),
        ];
        let mut pdf = b"%PDF-1.6\n".to_vec();
        let mut offsets = Vec::new();
        for (id, body) in objects {
            offsets.push((id, pdf.len()));
            pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref_offset = pdf.len();
        let size = offsets.iter().map(|(id, _)| *id).max().unwrap_or(0) + 1;
        pdf.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for (_, offset) in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
        );
        pdf
    }

    fn triangle() -> Vec<AnnotationPoint> {
        vec![
            AnnotationPoint { x: 0.0, y: 0.0 },
            AnnotationPoint { x: 10.0, y: 0.0 },
            AnnotationPoint { x: 10.0, y: 10.0 },
        ]
    }

    #[test]
    fn a_non_finite_vertex_is_rejected() {
        let pdf = skeleton_pdf();
        let mut annotation = PolygonAnnotationInput {
            vertices: triangle(),
            ..PolygonAnnotationInput::default()
        };
        annotation.vertices[1] = AnnotationPoint { x: f64::NAN, y: 0.0 };
        assert!(
            append_polygon_annotation(&pdf, 0, &annotation).is_err(),
            "a NaN vertex must not reach the PDF"
        );
    }

    #[test]
    fn a_non_finite_rect_is_rejected() {
        let pdf = skeleton_pdf();
        let annotation = PolygonAnnotationInput {
            vertices: triangle(),
            rect: Some(BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: f64::INFINITY,
                y1: 10.0,
            }),
            ..PolygonAnnotationInput::default()
        };
        assert!(
            append_polygon_annotation(&pdf, 0, &annotation).is_err(),
            "an infinite rect must not reach the PDF"
        );
    }

    #[test]
    fn a_colour_outside_the_unit_range_is_rejected() {
        let pdf = skeleton_pdf();
        let annotation = PolygonAnnotationInput {
            vertices: triangle(),
            color: Some(vec![0.0, 1.5, 0.0]),
            ..PolygonAnnotationInput::default()
        };
        assert!(
            append_polygon_annotation(&pdf, 0, &annotation).is_err(),
            "a colour component above 1.0 must not reach the PDF"
        );
    }
}
