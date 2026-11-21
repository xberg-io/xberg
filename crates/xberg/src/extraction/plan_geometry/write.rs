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
/// the source context), or fewer than three vertices are supplied.
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
