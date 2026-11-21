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
    /// Squiggly (wavy) underline text markup
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Squiggly,
    /// Freehand drawing (ink) annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Ink,
    /// Rectangle/box shape annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Square,
    /// Ellipse/oval shape annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Circle,
    /// Closed polygon shape annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Polygon,
    /// Open polyline shape annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    PolyLine,
    /// Line annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Line,
    /// Caret (text-insertion marker) annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Caret,
    /// Embedded file attachment annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    FileAttachment,
    /// Embedded sound annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Sound,
    /// Embedded movie annotation
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Movie,
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
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub bounding_box: Option<BoundingBox>,
    /// Author/creator of the annotation (PDF `/T` entry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub author: Option<String>,
    /// Last modification date of the annotation (PDF `/M` entry), as a raw
    /// PDF date string (e.g. `"D:20240115120000Z"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub modified: Option<String>,
    /// Annotation colour (PDF `/C` entry), normalised to a CSS-compatible
    /// `#rrggbb` hex string. Gray and CMYK colour spaces are converted to RGB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub color: Option<String>,
    /// Subject of the annotation (PDF `/Subj` entry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub subject: Option<String>,
    /// Per-line bounding boxes derived from the annotation's `/QuadPoints`
    /// entry. Present for text markup annotations (Highlight, Underline,
    /// StrikeOut, Squiggly), one box per marked line/run of text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub quad_points: Option<Vec<BoundingBox>>,
    /// The document text covered by [`Self::quad_points`], recovered from the
    /// page content underneath the marked-up region. Populated for
    /// Highlight, Underline, StrikeOut, and Squiggly annotations when the
    /// underlying text could be recovered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub marked_text: Option<String>,
    /// Vertices of a Line, Polygon or PolyLine annotation.
    ///
    /// A `Line` annotation carries its two `/L` endpoints; a `Polygon` or
    /// `PolyLine` carries its `/Vertices`. Coordinates are in PDF page points
    /// with y increasing upwards, the same frame as [`Self::bounding_box`].
    /// `None` for every other subtype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.5.0"))]
    pub vertices: Option<Vec<AnnotationPoint>>,
    /// One point list per stroke of an Ink annotation (PDF `/InkList`), each in
    /// PDF page points with y increasing upwards. `None` for every other
    /// subtype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.5.0"))]
    pub ink_list: Option<Vec<Vec<AnnotationPoint>>>,
    /// Measurement dictionary of a geometric annotation (PDF `/Measure`).
    ///
    /// Measured takeoff tools save their work as a polygon with a measure
    /// dictionary; reading it lets a caller turn the polygon's page-space
    /// coordinates into a real-world quantity. `None` when the annotation
    /// carries no `/Measure`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.5.0"))]
    pub measure: Option<MeasureData>,
}

/// One point of an annotation's geometry, in PDF page points with y increasing
/// upwards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct AnnotationPoint {
    /// Horizontal coordinate in PDF points.
    pub x: f64,
    /// Vertical coordinate in PDF points, increasing upwards.
    pub y: f64,
}

/// A PDF measurement dictionary (PDF `/Measure`, ISO 32000-1 §12.9).
///
/// Number format arrays are exposed as their flattened numeric entries because
/// the PDF specification allows nested arrays and the meaning of the entries is
/// producer-specific. [`scale_ratio`](Self::scale_ratio) keeps the producer's
/// own scale text, which the drawing-set parser can turn into a number.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct MeasureData {
    /// Measure subtype: `RL` for a rectilinear drawing, `GEO` for geospatial
    /// data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// The producer's scale-ratio text (PDF `/R`), for example
    /// `"3/8 in = 1 ft"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_ratio: Option<String>,
    /// Flattened numeric entries of the `/X` number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Vec<f64>>,
    /// Flattened numeric entries of the `/Y` number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Vec<f64>>,
    /// Flattened numeric entries of the `/D` distance number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<Vec<f64>>,
    /// Flattened numeric entries of the `/A` area number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Vec<f64>>,
    /// Flattened numeric entries of the `/T` angle number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<Vec<f64>>,
    /// Flattened numeric entries of the `/S` slope number format array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slope: Option<Vec<f64>>,
    /// Name of the coordinate origin (PDF `/O`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

impl From<&xberg_native_pdf::MeasureDictionary> for MeasureData {
    fn from(measure: &xberg_native_pdf::MeasureDictionary) -> Self {
        Self {
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
}

/// A page viewport (PDF `/VP`, ISO 32000-1 §12.5.6.19).
///
/// Construction drawings use one viewport per plan, detail or elevation, and
/// the viewport's measure dictionary carries that view's scale.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PageViewport {
    /// Viewport bounds in PDF page points with y increasing upwards.
    pub bounding_box: BoundingBox,
    /// Viewport name, read from `/Name`, then `/NM`, then `/T`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Measurement dictionary for this viewport, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasureData>,
}

impl From<&xberg_native_pdf::PageViewport> for PageViewport {
    fn from(viewport: &xberg_native_pdf::PageViewport) -> Self {
        Self {
            bounding_box: BoundingBox {
                x0: viewport.bbox[0],
                y0: viewport.bbox[1],
                x1: viewport.bbox[2],
                y1: viewport.bbox[3],
            },
            name: viewport.name.clone(),
            measure: viewport.measure.as_ref().map(MeasureData::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn annotation() -> PdfAnnotation {
        PdfAnnotation {
            annotation_type: PdfAnnotationType::Polygon,
            content: Some("Room 101".to_string()),
            page_number: 1,
            bounding_box: None,
            author: None,
            modified: None,
            color: None,
            subject: None,
            quad_points: None,
            marked_text: None,
            vertices: None,
            ink_list: None,
            measure: None,
        }
    }

    #[test]
    fn an_annotation_without_geometry_serializes_without_the_new_keys() {
        let value = serde_json::to_value(annotation()).expect("the annotation serializes");
        assert_eq!(
            value,
            json!({
                "annotation_type": "polygon",
                "content": "Room 101",
                "page_number": 1,
            }),
            "absent optional geometry must not produce keys"
        );
    }

    #[test]
    fn vertices_ink_and_measure_serialize_in_page_coordinates() {
        let mut annotation = annotation();
        annotation.vertices = Some(vec![
            AnnotationPoint { x: 10.0, y: 20.0 },
            AnnotationPoint { x: 30.0, y: 20.0 },
            AnnotationPoint { x: 30.0, y: 40.0 },
        ]);
        annotation.ink_list = Some(vec![vec![AnnotationPoint { x: 1.0, y: 2.0 }]]);
        annotation.measure = Some(MeasureData {
            subtype: Some("RL".to_string()),
            scale_ratio: Some("1/4 in = 1 ft".to_string()),
            area: Some(vec![1.0, 144.0]),
            ..MeasureData::default()
        });

        let value = serde_json::to_value(annotation).expect("the annotation serializes");
        assert_eq!(
            value["vertices"],
            json!([{"x": 10.0, "y": 20.0}, {"x": 30.0, "y": 20.0}, {"x": 30.0, "y": 40.0}])
        );
        assert_eq!(value["ink_list"], json!([[{"x": 1.0, "y": 2.0}]]));
        assert_eq!(value["measure"]["scale_ratio"], json!("1/4 in = 1 ft"));
        assert_eq!(value["measure"]["area"], json!([1.0, 144.0]));
    }

    #[test]
    fn a_viewport_carries_bounds_name_and_measure() {
        let viewport = PageViewport {
            bounding_box: BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 300.0,
                y1: 400.0,
            },
            name: Some("PLAN".to_string()),
            measure: Some(MeasureData {
                scale_ratio: Some("3/8 in = 1 ft".to_string()),
                ..MeasureData::default()
            }),
        };
        assert_eq!(
            serde_json::to_value(viewport).expect("the viewport serializes"),
            json!({
                "bounding_box": {"x0": 10.0, "y0": 20.0, "x1": 300.0, "y1": 400.0},
                "name": "PLAN",
                "measure": {"scale_ratio": "3/8 in = 1 ft"},
            })
        );
    }
}
