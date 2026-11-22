//! Page vector geometry output for the native PDF backend.
//!
//! [`xberg_native_pdf::document::PdfDocument::extract_paths`] already returns every painted
//! path with its CTM applied, its stroke width and dash pattern scaled to page space, and
//! its fill/stroke colours. This module maps that into the public [`PageGeometry`] shape
//! carried on `PageContent::geometry`.
//!
//! Nothing here runs unless `pdf_options.extract_vector_geometry` is set: a dense drawing
//! sheet can hold more than ten thousand paths.

use xberg_native_pdf::elements::{LineCap, LineJoin, PathContent, PathOperation};
use xberg_native_pdf::layout::Color;

use crate::types::{
    BoundingBox, PageContent, PageGeometry, VectorColor, VectorDashPattern, VectorLineCap, VectorLineJoin, VectorPath,
    VectorPoint, VectorSegment, VectorStroke, VectorSubpath,
};

/// Attach each page's vector geometry, in page order.
///
/// A page whose paths cannot be extracted keeps `geometry: None`; the failure is logged and
/// does not abort the rest of the extraction, matching how the other optional native stages
/// degrade.
pub(crate) fn attach_page_geometry(doc: &xberg_native_pdf::PdfDocument, pages: &mut [PageContent]) {
    for page in pages {
        let page_index = page.page_number.saturating_sub(1) as usize;
        match doc.extract_paths(page_index) {
            // A page that drew no path reports no geometry rather than an empty one: an
            // absent value is the same signal the flag being off gives, and the caller
            // knows which it asked for. ~keep
            Ok(paths) if paths.is_empty() => {
                page.geometry = None;
            }
            Ok(paths) => {
                page.geometry = Some(PageGeometry {
                    paths: paths.iter().map(VectorPath::from).collect(),
                    regions: Vec::new(),
                });
            }
            Err(error) => {
                tracing::warn!(
                    page = page.page_number,
                    "skipping PDF vector geometry that could not be extracted: {error}"
                );
            }
        }
    }
}

impl From<&PathContent> for VectorPath {
    fn from(path: &PathContent) -> Self {
        Self {
            subpaths: subpaths_from_operations(&path.operations),
            bounding_box: BoundingBox {
                x0: f64::from(path.bbox.x),
                y0: f64::from(path.bbox.y),
                x1: f64::from(path.bbox.x + path.bbox.width),
                y1: f64::from(path.bbox.y + path.bbox.height),
            },
            fill_color: path.fill_color.map(VectorColor::from),
            stroke: path.stroke_color.map(|color| VectorStroke {
                color: VectorColor::from(color),
                width: path.stroke_width,
                dash_pattern: path.dash_pattern.as_ref().map(|(dashes, phase)| VectorDashPattern {
                    dashes: dashes.clone(),
                    phase: *phase,
                }),
                line_cap: path.line_cap.into(),
                line_join: path.line_join.into(),
            }),
            layer: path.layer.clone(),
        }
    }
}

impl From<Color> for VectorColor {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

impl From<LineCap> for VectorLineCap {
    fn from(cap: LineCap) -> Self {
        match cap {
            LineCap::Butt => Self::Butt,
            LineCap::Round => Self::Round,
            LineCap::Square => Self::Square,
        }
    }
}

impl From<LineJoin> for VectorLineJoin {
    fn from(join: LineJoin) -> Self {
        match join {
            LineJoin::Miter => Self::Miter,
            LineJoin::Round => Self::Round,
            LineJoin::Bevel => Self::Bevel,
        }
    }
}

/// Rebuild the path's subpaths from its drawing operators.
///
/// The operators are already in page coordinates. A `Rectangle` becomes a closed four-sided
/// subpath, a `ClosePath` sets `closed` on the current subpath, and a `MoveTo` starts a new
/// one. Subpaths that paint no segment (a lone `MoveTo`) are dropped, as they draw nothing.
fn subpaths_from_operations(operations: &[PathOperation]) -> Vec<VectorSubpath> {
    let mut subpaths = Vec::new();
    let mut current: Option<VectorSubpath> = None;
    let mut position = VectorPoint::default();
    let mut subpath_start = VectorPoint::default();

    for operation in operations {
        match *operation {
            PathOperation::MoveTo(x, y) => {
                flush_subpath(&mut current, &mut subpaths);
                position = VectorPoint { x, y };
                subpath_start = position;
                current = Some(VectorSubpath {
                    start: position,
                    segments: Vec::new(),
                    closed: false,
                });
            }
            PathOperation::LineTo(x, y) => {
                let subpath = current.get_or_insert_with(|| open_subpath_at(position));
                position = VectorPoint { x, y };
                subpath.segments.push(VectorSegment::Line { to: position });
            }
            PathOperation::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                let subpath = current.get_or_insert_with(|| open_subpath_at(position));
                let segment = VectorSegment::Cubic {
                    control1: VectorPoint { x: c1x, y: c1y },
                    control2: VectorPoint { x: c2x, y: c2y },
                    to: VectorPoint { x, y },
                };
                position = VectorPoint { x, y };
                subpath.segments.push(segment);
            }
            PathOperation::Rectangle(x, y, width, height) => {
                flush_subpath(&mut current, &mut subpaths);
                // The extractor stores the transformed delta, so a flipping CTM can make the
                // extent negative; order the corners before emitting them.
                let (x0, x1) = (x.min(x + width), x.max(x + width));
                let (y0, y1) = (y.min(y + height), y.max(y + height));
                subpath_start = VectorPoint { x: x0, y: y0 };
                current = Some(VectorSubpath {
                    start: subpath_start,
                    segments: vec![
                        VectorSegment::Line {
                            to: VectorPoint { x: x1, y: y0 },
                        },
                        VectorSegment::Line {
                            to: VectorPoint { x: x1, y: y1 },
                        },
                        VectorSegment::Line {
                            to: VectorPoint { x: x0, y: y1 },
                        },
                    ],
                    closed: true,
                });
                position = subpath_start;
            }
            PathOperation::ClosePath => {
                if let Some(subpath) = current.as_mut() {
                    subpath.closed = true;
                }
                flush_subpath(&mut current, &mut subpaths);
                // After `h` the current point is the closed subpath's start.
                position = subpath_start;
            }
        }
    }

    flush_subpath(&mut current, &mut subpaths);
    subpaths
}

fn open_subpath_at(start: VectorPoint) -> VectorSubpath {
    VectorSubpath {
        start,
        segments: Vec::new(),
        closed: false,
    }
}

fn flush_subpath(current: &mut Option<VectorSubpath>, subpaths: &mut Vec<VectorSubpath>) {
    if let Some(subpath) = current.take()
        && !subpath.segments.is_empty()
    {
        subpaths.push(subpath);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xberg_native_pdf::elements::{LineCap, LineJoin};
    use xberg_native_pdf::geometry::Rect;

    fn point(x: f32, y: f32) -> VectorPoint {
        VectorPoint { x, y }
    }

    fn path(operations: Vec<PathOperation>) -> PathContent {
        PathContent::from_operations(operations)
    }

    #[test]
    fn a_line_path_becomes_one_open_subpath() {
        let converted = VectorPath::from(&path(vec![
            PathOperation::MoveTo(72.0, 700.0),
            PathOperation::LineTo(300.0, 700.0),
        ]));

        assert_eq!(
            converted.subpaths,
            vec![VectorSubpath {
                start: point(72.0, 700.0),
                segments: vec![VectorSegment::Line {
                    to: point(300.0, 700.0)
                }],
                closed: false,
            }]
        );
        assert_eq!(
            converted.bounding_box,
            BoundingBox {
                x0: 72.0,
                y0: 700.0,
                x1: 300.0,
                y1: 700.0
            }
        );
    }

    #[test]
    fn a_closed_curve_keeps_its_control_points_and_close_flag() {
        let converted = VectorPath::from(&path(vec![
            PathOperation::MoveTo(10.0, 10.0),
            PathOperation::CurveTo(10.0, 20.0, 20.0, 20.0, 20.0, 10.0),
            PathOperation::ClosePath,
        ]));

        let [subpath] = converted.subpaths.as_slice() else {
            panic!("expected one subpath, got {:?}", converted.subpaths);
        };
        assert!(subpath.closed);
        assert_eq!(
            subpath.segments,
            vec![VectorSegment::Cubic {
                control1: point(10.0, 20.0),
                control2: point(20.0, 20.0),
                to: point(20.0, 10.0),
            }]
        );
    }

    #[test]
    fn a_rectangle_becomes_a_closed_four_sided_subpath_with_ordered_corners() {
        // A flipping CTM can leave a negative extent on the operator.
        let converted = VectorPath::from(&path(vec![PathOperation::Rectangle(150.0, 60.0, -50.0, -10.0)]));

        let [subpath] = converted.subpaths.as_slice() else {
            panic!("expected one subpath, got {:?}", converted.subpaths);
        };
        assert_eq!(subpath.start, point(100.0, 50.0));
        assert!(subpath.closed);
        assert_eq!(
            subpath.segments,
            vec![
                VectorSegment::Line { to: point(150.0, 50.0) },
                VectorSegment::Line { to: point(150.0, 60.0) },
                VectorSegment::Line { to: point(100.0, 60.0) },
            ],
            "the close sets the fourth side instead of a duplicate segment"
        );
        assert_eq!(
            converted.bounding_box,
            BoundingBox {
                x0: 100.0,
                y0: 50.0,
                x1: 150.0,
                y1: 60.0
            }
        );
    }

    #[test]
    fn several_painted_subpaths_stay_in_one_path() {
        let converted = VectorPath::from(&path(vec![
            PathOperation::MoveTo(0.0, 0.0),
            PathOperation::LineTo(10.0, 0.0),
            PathOperation::Rectangle(20.0, 20.0, 5.0, 5.0),
            PathOperation::MoveTo(40.0, 0.0),
            PathOperation::LineTo(40.0, 10.0),
        ]));

        assert_eq!(converted.subpaths.len(), 3);
        assert!(!converted.subpaths[0].closed);
        assert!(converted.subpaths[1].closed);
        assert_eq!(converted.subpaths[2].start, point(40.0, 0.0));
    }

    #[test]
    fn a_lone_move_to_paints_nothing_and_is_dropped() {
        let converted = VectorPath::from(&path(vec![
            PathOperation::MoveTo(5.0, 5.0),
            PathOperation::MoveTo(6.0, 6.0),
            PathOperation::LineTo(7.0, 7.0),
        ]));

        let [subpath] = converted.subpaths.as_slice() else {
            panic!("expected one subpath, got {:?}", converted.subpaths);
        };
        assert_eq!(subpath.start, point(6.0, 6.0));
        assert_eq!(subpath.segments, vec![VectorSegment::Line { to: point(7.0, 7.0) }]);
    }

    #[test]
    fn fill_and_stroke_carry_every_paint_attribute() {
        let mut source = PathContent::new(Rect::new(0.0, 0.0, 10.0, 10.0));
        source.operations = vec![PathOperation::Rectangle(0.0, 0.0, 10.0, 10.0)];
        source.fill_color = Some(Color::new(0.2, 0.4, 0.6));
        source.stroke_color = Some(Color::new(1.0, 0.0, 0.0));
        source.stroke_width = 2.5;
        source.line_cap = LineCap::Round;
        source.line_join = LineJoin::Bevel;
        source.dash_pattern = Some((vec![3.0, 2.0], 1.5));
        source.layer = Some("A-WALL".to_string());

        let converted = VectorPath::from(&source);

        assert_eq!(converted.fill_color, Some(VectorColor { r: 0.2, g: 0.4, b: 0.6 }));
        let stroke = converted.stroke.expect("the stroke is present");
        assert_eq!(stroke.color, VectorColor { r: 1.0, g: 0.0, b: 0.0 });
        assert_eq!(stroke.width, 2.5);
        assert_eq!(
            stroke.dash_pattern,
            Some(VectorDashPattern {
                dashes: vec![3.0, 2.0],
                phase: 1.5
            })
        );
        assert_eq!(stroke.line_cap, VectorLineCap::Round);
        assert_eq!(stroke.line_join, VectorLineJoin::Bevel);
        assert_eq!(converted.layer.as_deref(), Some("A-WALL"));
    }

    #[test]
    fn an_unstroked_unfilled_path_has_neither_paint_style() {
        let mut source = PathContent::new(Rect::new(0.0, 0.0, 10.0, 10.0));
        source.operations = vec![PathOperation::Rectangle(0.0, 0.0, 10.0, 10.0)];
        source.stroke_color = None;
        source.fill_color = None;

        let converted = VectorPath::from(&source);

        assert!(converted.stroke.is_none());
        assert!(converted.fill_color.is_none());
    }
}
