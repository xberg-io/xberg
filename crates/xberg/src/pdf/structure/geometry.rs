//! Shared spatial geometry primitives for the PDF markdown pipeline.
//!
//! All operations are coordinate-system agnostic: they work with any
//! axis-aligned rectangle regardless of whether y=0 is at the top
//! (image coordinates) or bottom (PDF coordinates). Callers are
//! responsible for consistent coordinate systems when comparing rects.

/// Axis-aligned bounding box.
///
/// Fields use the naming convention `left/right` for X and `y_min/y_max`
/// for Y to remain agnostic about coordinate system direction.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub left: f32,
    pub right: f32,
    /// Smaller Y value.
    pub y_min: f32,
    /// Larger Y value.
    pub y_max: f32,
}

impl Rect {
    /// Create a rect, normalizing so left <= right and y_min <= y_max.
    pub(crate) fn new(left: f32, right: f32, y_min: f32, y_max: f32) -> Self {
        Self {
            left: left.min(right),
            right: left.max(right),
            y_min: y_min.min(y_max),
            y_max: y_min.max(y_max),
        }
    }

    /// Create from PDF coordinate convention (left, bottom, right, top).
    pub(crate) fn from_lbrt(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self::new(left, right, bottom, top)
    }

    /// Create from image coordinate convention (left, top, right, bottom).
    pub(crate) fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self::new(left, right, top, bottom)
    }

    /// Create from (x, y, width, height) where y is the smaller Y value.
    pub(crate) fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(x, x + width, y, y + height)
    }

    pub(crate) fn width(&self) -> f32 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    pub(crate) fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub(crate) fn center_x(&self) -> f32 {
        (self.left + self.right) * 0.5
    }

    pub(crate) fn center_y(&self) -> f32 {
        (self.y_min + self.y_max) * 0.5
    }

    /// Intersection area with another rect. Returns 0.0 if no overlap.
    pub(crate) fn intersection_area(&self, other: &Rect) -> f32 {
        let inter_left = self.left.max(other.left);
        let inter_right = self.right.min(other.right);
        let inter_y_min = self.y_min.max(other.y_min);
        let inter_y_max = self.y_max.min(other.y_max);

        if inter_left >= inter_right || inter_y_min >= inter_y_max {
            return 0.0;
        }

        (inter_right - inter_left) * (inter_y_max - inter_y_min)
    }

    /// Fraction of `self`'s area that overlaps with `container`.
    ///
    /// Returns 0.0 if self has zero area or no overlap exists.
    pub(crate) fn intersection_over_self(&self, container: &Rect) -> f32 {
        let self_area = self.area();
        if self_area <= 0.0 {
            return 0.0;
        }
        self.intersection_area(container) / self_area
    }

    /// Whether a point lies inside (or on the boundary of) this rect.
    pub(crate) fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.y_min && y <= self.y_max
    }
}

/// The smallest `(left, bottom, right, top)` box covering both boxes, or the one that exists.
pub(crate) fn union_bbox(
    a: Option<(f32, f32, f32, f32)>,
    b: Option<(f32, f32, f32, f32)>,
) -> Option<(f32, f32, f32, f32)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        (bbox, None) | (None, bbox) => bbox,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area() {
        let r = Rect::new(0.0, 10.0, 0.0, 5.0);
        assert!((r.area() - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zero_area() {
        let r = Rect::new(5.0, 5.0, 0.0, 10.0);
        assert!((r.area() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_area_overlap() {
        let a = Rect::new(0.0, 10.0, 0.0, 10.0);
        let b = Rect::new(5.0, 15.0, 5.0, 15.0);
        assert!((a.intersection_area(&b) - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_area_no_overlap() {
        let a = Rect::new(0.0, 5.0, 0.0, 5.0);
        let b = Rect::new(10.0, 15.0, 10.0, 15.0);
        assert!((a.intersection_area(&b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_area_contained() {
        let outer = Rect::new(0.0, 20.0, 0.0, 20.0);
        let inner = Rect::new(5.0, 10.0, 5.0, 10.0);
        assert!((inner.intersection_area(&outer) - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_over_self_full() {
        let inner = Rect::new(5.0, 10.0, 5.0, 10.0);
        let outer = Rect::new(0.0, 20.0, 0.0, 20.0);
        assert!((inner.intersection_over_self(&outer) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_over_self_half() {
        let a = Rect::new(0.0, 10.0, 0.0, 10.0);
        let b = Rect::new(5.0, 15.0, 0.0, 10.0);
        assert!((a.intersection_over_self(&b) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_over_self_zero_area() {
        let zero = Rect::new(5.0, 5.0, 0.0, 10.0);
        let other = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!((zero.intersection_over_self(&other) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contains_point_inside() {
        let r = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!(r.contains_point(5.0, 5.0));
    }

    #[test]
    fn test_contains_point_boundary() {
        let r = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!(r.contains_point(0.0, 0.0));
        assert!(r.contains_point(10.0, 10.0));
    }

    #[test]
    fn test_contains_point_outside() {
        let r = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!(!r.contains_point(11.0, 5.0));
        assert!(!r.contains_point(5.0, -1.0));
    }

    #[test]
    fn test_from_lbrt() {
        let r = Rect::from_lbrt(10.0, 20.0, 50.0, 80.0);
        assert!((r.left - 10.0).abs() < f32::EPSILON);
        assert!((r.y_min - 20.0).abs() < f32::EPSILON);
        assert!((r.right - 50.0).abs() < f32::EPSILON);
        assert!((r.y_max - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_from_xywh() {
        let r = Rect::from_xywh(10.0, 20.0, 30.0, 40.0);
        assert!((r.left - 10.0).abs() < f32::EPSILON);
        assert!((r.y_min - 20.0).abs() < f32::EPSILON);
        assert!((r.right - 40.0).abs() < f32::EPSILON);
        assert!((r.y_max - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalizes_inverted() {
        let r = Rect::new(10.0, 0.0, 10.0, 0.0);
        assert!((r.left - 0.0).abs() < f32::EPSILON);
        assert!((r.right - 10.0).abs() < f32::EPSILON);
        assert!((r.y_min - 0.0).abs() < f32::EPSILON);
        assert!((r.y_max - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_width_and_height() {
        let r = Rect::new(10.0, 30.0, 5.0, 25.0);
        assert!((r.width() - 20.0).abs() < f32::EPSILON);
        assert!((r.height() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_center_x_and_y() {
        let r = Rect::new(0.0, 10.0, 0.0, 20.0);
        assert!((r.center_x() - 5.0).abs() < f32::EPSILON);
        assert!((r.center_y() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_from_ltrb() {
        let r = Rect::from_ltrb(10.0, 80.0, 50.0, 20.0);
        assert!((r.left - 10.0).abs() < f32::EPSILON);
        assert!((r.right - 50.0).abs() < f32::EPSILON);
        assert!((r.y_min - 20.0).abs() < f32::EPSILON);
        assert!((r.y_max - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zero_width_rect() {
        let r = Rect::new(5.0, 5.0, 0.0, 10.0);
        assert!((r.width() - 0.0).abs() < f32::EPSILON);
        assert!((r.area() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zero_height_rect() {
        let r = Rect::new(0.0, 10.0, 5.0, 5.0);
        assert!((r.height() - 0.0).abs() < f32::EPSILON);
        assert!((r.area() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_identical_rects_intersection() {
        let a = Rect::new(0.0, 10.0, 0.0, 10.0);
        let b = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!((a.intersection_area(&b) - 100.0).abs() < f32::EPSILON);
        assert!((a.intersection_over_self(&b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_adjacent_rects_no_intersection() {
        let a = Rect::new(0.0, 10.0, 0.0, 10.0);
        let b = Rect::new(10.0, 20.0, 0.0, 10.0);
        assert!((a.intersection_area(&b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contains_point_just_outside() {
        let r = Rect::new(0.0, 10.0, 0.0, 10.0);
        assert!(!r.contains_point(-0.001, 5.0));
        assert!(!r.contains_point(10.001, 5.0));
        assert!(!r.contains_point(5.0, -0.001));
        assert!(!r.contains_point(5.0, 10.001));
    }

    #[test]
    fn test_intersection_over_self_no_overlap() {
        let a = Rect::new(0.0, 5.0, 0.0, 5.0);
        let b = Rect::new(100.0, 200.0, 100.0, 200.0);
        assert!((a.intersection_over_self(&b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intersection_area_symmetric() {
        let a = Rect::new(0.0, 10.0, 0.0, 10.0);
        let b = Rect::new(5.0, 15.0, 5.0, 15.0);
        assert!((a.intersection_area(&b) - b.intersection_area(&a)).abs() < f32::EPSILON);
    }
}
