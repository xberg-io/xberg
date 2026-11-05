use super::{Edge, SNAP_TOL};
use crate::elements::PathContent;
use crate::geometry::Rect;

fn edge_covers(edges: &[Edge], coordinate: f32, start: f32, end: f32) -> bool {
    edges.iter().any(|edge| {
        (edge.coord - coordinate).abs() <= SNAP_TOL && edge.start <= start + SNAP_TOL && edge.end >= end - SNAP_TOL
    })
}

fn independently_closed(bounds: Rect, horizontal: &[Edge], vertical: &[Edge]) -> bool {
    bounds.width > 0.0
        && bounds.height > 0.0
        && [bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
            .iter()
            .all(|value| value.is_finite())
        && edge_covers(horizontal, bounds.top(), bounds.left(), bounds.right())
        && edge_covers(horizontal, bounds.bottom(), bounds.left(), bounds.right())
        && edge_covers(vertical, bounds.left(), bounds.top(), bounds.bottom())
        && edge_covers(vertical, bounds.right(), bounds.top(), bounds.bottom())
}

/// Text-line backgrounds inside one independently ruled cell do not add inner grid boundaries.
/// The outer fill must be closed by thin rules; nested fills cannot establish their own proof. ~keep
pub(super) fn nested_inside_ruled_fill(
    path: &PathContent,
    rectangles: &[&PathContent],
    horizontal: &[Edge],
    vertical: &[Edge],
) -> bool {
    if !path.has_fill() || path.has_stroke() {
        return false;
    }
    rectangles.iter().any(|outer| {
        outer.has_fill()
            && !outer.has_stroke()
            && outer.fill_color == path.fill_color
            && outer.bbox.contains_rect(&path.bbox)
            && outer.bbox.left() < path.bbox.left() - SNAP_TOL
            && outer.bbox.right() > path.bbox.right() + SNAP_TOL
            && independently_closed(outer.bbox, horizontal, vertical)
    })
}
