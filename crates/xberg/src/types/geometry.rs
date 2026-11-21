//! Vector geometry of a PDF page.
//!
//! Returned on [`PageContent::geometry`](super::PageContent) when
//! `pdf_options.extract_vector_geometry` is set. Every coordinate is in the page's default
//! user space: PDF points with y increasing upwards, measured from the page's lower-left
//! corner. This is the same frame as text bounds in
//! [`ElementMetadata::coordinates`](super::ElementMetadata), so a label's box and the shape
//! it annotates can be compared directly. Content-stream `cm` transforms and Form XObject
//! `/Matrix` transforms are already applied; page `/Rotate` and rendering DPI are not.
//!
//! Paths are reported in drawing order and are not simplified, merged or classified: a
//! construction drawing keeps its individual rules, curves and filled shapes.

use serde::{Deserialize, Serialize};

use super::extraction::BoundingBox;

/// Vector geometry of one page.
///
/// Only populated for PDF pages, and only when
/// [`PdfConfig::extract_vector_geometry`](crate::core::config::PdfConfig) is on.
/// A dense drawing sheet can hold more than ten thousand paths, so this output is off by
/// default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PageGeometry {
    /// Painted paths, in content-stream drawing order.
    ///
    /// Clipping paths alone are not painted and are therefore not listed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<VectorPath>,

    /// Closed faces recovered from the page's line segments.
    ///
    /// Set only when `pdf_options.plan_regions` is enabled and the
    /// `plan-regions` feature is compiled in. Each face carries its polygon, its
    /// area in square page points, and the page text inside it. Ordering is
    /// deterministic: largest area first, then by the face's upper-left corner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "api", schema(required = false))]
    pub regions: Vec<PlanRegion>,
}

/// One closed face recovered from a page's line segments.
///
/// Coordinates are in PDF page points with y increasing upwards. The area is in
/// square page points; convert it with the view's scale to get a real-world
/// quantity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PlanRegion {
    /// Polygon vertices in drawing order, without a repeated closing point.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub polygon: Vec<VectorPoint>,

    /// Area of the face in square page points (always non-negative).
    pub area: f64,

    /// Axis-aligned bounds of the face.
    pub bounding_box: BoundingBox,

    /// Page text whose span centre lies inside the face, in reading order and
    /// joined by single spaces. Empty when no text is inside.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
}

/// One painted vector path, in page coordinates.
///
/// A single painting operator can carry several subpaths (for example a table rule and a
/// rectangle drawn before one `f`), so a path is a list of [`VectorSubpath`]s sharing one
/// paint style.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorPath {
    /// Subpaths, in the order the path built them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subpaths: Vec<VectorSubpath>,

    /// Axis-aligned bounds of the path's geometry, without the stroke extension.
    ///
    /// This is the geometry as constructed: a cubic segment's control points are included,
    /// so the box bounds the curve without being tight to it. A stroke straddles the path by
    /// half its width on each side, which this box does not include.
    pub bounding_box: BoundingBox,

    /// Fill colour, or `None` when the path is not filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub fill_color: Option<VectorColor>,

    /// Stroke style, or `None` when the path is not stroked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub stroke: Option<VectorStroke>,

    /// Name of the Optional Content Group (PDF "layer") the path was drawn under, when the
    /// producer tagged it. CAD and BIM exports commonly name layers such as `A-WALL`, and a
    /// consumer can use the name to select the drawing discipline it wants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub layer: Option<String>,
}

/// One continuous run of a [`VectorPath`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorSubpath {
    /// Point the subpath starts at.
    pub start: VectorPoint,

    /// Segments leaving [`start`](Self::start), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<VectorSegment>,

    /// Whether the subpath is closed.
    ///
    /// A closed subpath paints one extra straight segment from the end of its last segment
    /// back to [`start`](Self::start), as the PDF `h` operator does.
    pub closed: bool,
}

/// One segment of a [`VectorSubpath`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VectorSegment {
    /// A straight segment.
    Line {
        /// End point of the segment.
        to: VectorPoint,
    },

    /// A cubic Bézier segment.
    Cubic {
        /// First control point.
        control1: VectorPoint,
        /// Second control point.
        control2: VectorPoint,
        /// End point of the segment.
        to: VectorPoint,
    },
}

/// A point in page coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorPoint {
    /// Horizontal coordinate in PDF points.
    pub x: f32,
    /// Vertical coordinate in PDF points, increasing upwards.
    pub y: f32,
}

/// An RGB colour with channels in `0.0..=1.0` in the page's colour space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorColor {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
}

/// Stroke style of a [`VectorPath`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorStroke {
    /// Stroke colour.
    pub color: VectorColor,

    /// Line width in points, in page coordinates (any content-stream scaling already applied).
    pub width: f32,

    /// Dash pattern, or `None` for a solid line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub dash_pattern: Option<VectorDashPattern>,

    /// How the ends of an open subpath are painted.
    pub line_cap: VectorLineCap,

    /// How corners between segments are painted.
    pub line_join: VectorLineJoin,
}

/// Dash pattern of a [`VectorStroke`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct VectorDashPattern {
    /// Alternating on/off lengths in points, starting with an on length.
    pub dashes: Vec<f32>,

    /// Distance into the pattern at which the stroke starts.
    pub phase: f32,
}

/// How the ends of an open stroked subpath are painted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum VectorLineCap {
    /// The stroke ends at the end point.
    #[default]
    Butt,
    /// A semicircle extends past the end point.
    Round,
    /// A half square extends past the end point.
    Square,
}

/// How corners of a stroked subpath are painted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum VectorLineJoin {
    /// The outer edges meet at a sharp corner.
    #[default]
    Miter,
    /// The outer edges meet at a circular arc.
    Round,
    /// The corner is cut off with a straight edge.
    Bevel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line_path() -> VectorPath {
        VectorPath {
            subpaths: vec![VectorSubpath {
                start: VectorPoint { x: 72.0, y: 700.0 },
                segments: vec![VectorSegment::Line {
                    to: VectorPoint { x: 300.0, y: 700.0 },
                }],
                closed: false,
            }],
            bounding_box: BoundingBox {
                x0: 72.0,
                y0: 700.0,
                x1: 300.0,
                y1: 700.0,
            },
            fill_color: None,
            stroke: Some(VectorStroke {
                color: VectorColor { r: 1.0, g: 0.0, b: 0.0 },
                width: 2.0,
                dash_pattern: Some(VectorDashPattern {
                    dashes: vec![3.0, 2.0],
                    phase: 1.0,
                }),
                line_cap: VectorLineCap::Round,
                line_join: VectorLineJoin::Bevel,
            }),
            layer: Some("A-WALL".to_string()),
        }
    }

    #[test]
    fn a_geometry_path_serializes_with_stable_snake_case_names() {
        let value = serde_json::to_value(line_path()).expect("the path serializes");

        assert_eq!(
            value,
            json!({
                "subpaths": [{
                    "start": {"x": 72.0, "y": 700.0},
                    "segments": [{"type": "line", "to": {"x": 300.0, "y": 700.0}}],
                    "closed": false,
                }],
                "bounding_box": {"x0": 72.0, "y0": 700.0, "x1": 300.0, "y1": 700.0},
                "stroke": {
                    "color": {"r": 1.0, "g": 0.0, "b": 0.0},
                    "width": 2.0,
                    "dash_pattern": {"dashes": [3.0, 2.0], "phase": 1.0},
                    "line_cap": "round",
                    "line_join": "bevel",
                },
                "layer": "A-WALL",
            }),
            "absent fill and present stroke must keep their documented key names"
        );
    }

    #[test]
    fn a_curve_segment_serializes_its_controls_under_a_type_tag() {
        let segment = VectorSegment::Cubic {
            control1: VectorPoint { x: 1.0, y: 2.0 },
            control2: VectorPoint { x: 3.0, y: 4.0 },
            to: VectorPoint { x: 5.0, y: 6.0 },
        };

        assert_eq!(
            serde_json::to_value(segment).expect("the segment serializes"),
            json!({
                "type": "cubic",
                "control1": {"x": 1.0, "y": 2.0},
                "control2": {"x": 3.0, "y": 4.0},
                "to": {"x": 5.0, "y": 6.0},
            })
        );
    }

    #[test]
    fn an_empty_page_geometry_serializes_without_a_paths_key() {
        assert_eq!(
            serde_json::to_value(PageGeometry::default()).expect("empty geometry serializes"),
            json!({}),
            "an empty path list must not produce a key"
        );
    }
}
