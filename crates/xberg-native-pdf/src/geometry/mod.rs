//! Geometric primitives for layout analysis.
//!
//! This module provides basic geometric types and operations used throughout
//! the layout analysis algorithms.

#![forbid(unsafe_code)]

/// A 2D point in document space.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Default)]
pub struct Point {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl Point {
    /// Create a new point.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Point;
    ///
    /// let point = Point::new(10.0, 20.0);
    /// assert_eq!(point.x, 10.0);
    /// assert_eq!(point.y, 20.0);
    /// ```
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A rectangle in document space.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Default)]
pub struct Rect {
    /// X coordinate of top-left corner
    pub x: f32,
    /// Y coordinate of top-left corner
    pub y: f32,
    /// Width of rectangle
    pub width: f32,
    /// Height of rectangle
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle from position and dimensions.
    /// Normalizes dimensions so width and height are always non-negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let rect = Rect::new(0.0, 50.0, 100.0, -50.0);
    /// assert_eq!(rect.y, 0.0);
    /// assert_eq!(rect.height, 50.0);
    /// ```
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        let (nx, nw) = if width < 0.0 { (x + width, -width) } else { (x, width) };
        let (ny, nh) = if height < 0.0 {
            (y + height, -height)
        } else {
            (y, height)
        };
        Self {
            x: nx,
            y: ny,
            width: nw,
            height: nh,
        }
    }

    /// Ensure width and height are non-negative.
    pub fn normalize(self) -> Self {
        Self::new(self.x, self.y, self.width, self.height)
    }

    /// Create a rectangle from two corner points.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let rect = Rect::from_points(10.0, 20.0, 110.0, 70.0);
    /// assert_eq!(rect.x, 10.0);
    /// assert_eq!(rect.y, 20.0);
    /// assert_eq!(rect.width, 100.0);
    /// assert_eq!(rect.height, 50.0);
    ///
    /// let swapped = Rect::from_points(110.0, 70.0, 10.0, 20.0);
    /// assert_eq!((swapped.x, swapped.y, swapped.width, swapped.height), (10.0, 20.0, 100.0, 50.0));
    /// ```
    pub fn from_points(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// Get the left edge x-coordinate.
    pub fn left(&self) -> f32 {
        self.x
    }

    /// Get the right edge x-coordinate.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the top edge y-coordinate.
    pub fn top(&self) -> f32 {
        self.y
    }

    /// Get the bottom edge y-coordinate.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Get the center point of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    /// let center = rect.center();
    /// assert_eq!(center.x, 50.0);
    /// assert_eq!(center.y, 25.0);
    /// ```
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    /// Check if this rectangle intersects with another.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    /// let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
    /// let r3 = Rect::new(200.0, 200.0, 100.0, 100.0);
    ///
    /// assert!(r1.intersects(&r2));
    /// assert!(!r1.intersects(&r3));
    /// ```
    pub fn intersects(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    /// Compute the intersection of this rectangle with another.
    ///
    /// Returns the overlapping region as a new rectangle, or None if they don't overlap.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    /// let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
    /// let r3 = Rect::new(200.0, 200.0, 100.0, 100.0);
    ///
    /// let inter = r1.intersection(&r2).unwrap();
    /// assert_eq!(inter.x, 50.0);
    /// assert_eq!(inter.y, 50.0);
    /// assert_eq!(inter.width, 50.0);
    /// assert_eq!(inter.height, 50.0);
    ///
    /// assert!(r1.intersection(&r3).is_none());
    /// ```
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.left().max(other.left());
        let y = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Rect::new(x, y, right - x, bottom - y))
    }

    /// Check if this rectangle contains a point.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::{Rect, Point};
    ///
    /// let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    /// let p1 = Point::new(50.0, 50.0);
    /// let p2 = Point::new(150.0, 150.0);
    ///
    /// assert!(rect.contains_point(&p1));
    /// assert!(!rect.contains_point(&p2));
    /// ```
    pub fn contains_point(&self, p: &Point) -> bool {
        p.x >= self.left() && p.x <= self.right() && p.y >= self.top() && p.y <= self.bottom()
    }

    /// Check if this rectangle fully contains another rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    /// let r2 = Rect::new(10.0, 10.0, 50.0, 50.0);
    /// let r3 = Rect::new(50.0, 50.0, 100.0, 100.0);
    ///
    /// assert!(r1.contains_rect(&r2));
    /// assert!(!r1.contains_rect(&r3));
    /// ```
    pub fn contains_rect(&self, other: &Rect) -> bool {
        other.left() >= self.left()
            && other.right() <= self.right()
            && other.top() >= self.top()
            && other.bottom() <= self.bottom()
    }

    /// Compute the union of this rectangle with another.
    ///
    /// Returns the smallest rectangle that contains both rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let r1 = Rect::new(0.0, 0.0, 50.0, 50.0);
    /// let r2 = Rect::new(25.0, 25.0, 50.0, 50.0);
    /// let union = r1.union(&r2);
    ///
    /// assert_eq!(union.x, 0.0);
    /// assert_eq!(union.y, 0.0);
    /// assert_eq!(union.right(), 75.0);
    /// assert_eq!(union.bottom(), 75.0);
    /// ```
    pub fn union(&self, other: &Rect) -> Rect {
        let x0 = self.left().min(other.left());
        let y0 = self.top().min(other.top());
        let x1 = self.right().max(other.right());
        let y1 = self.bottom().max(other.bottom());
        Rect::from_points(x0, y0, x1, y1)
    }

    /// Compute the area of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::geometry::Rect;
    ///
    /// let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    /// assert_eq!(rect.area(), 5000.0);
    /// ```
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

/// Compute the Euclidean distance between two points.
///
/// # Examples
///
/// ```
/// use xberg_native_pdf::geometry::{Point, euclidean_distance};
///
/// let p1 = Point::new(0.0, 0.0);
/// let p2 = Point::new(3.0, 4.0);
///
/// assert_eq!(euclidean_distance(&p1, &p2), 5.0);
/// ```
pub fn euclidean_distance(p1: &Point, p2: &Point) -> f32 {
    ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_from_points_normalises_opposite_diagonal() {
        let r = Rect::from_points(110.0, 70.0, 10.0, 20.0);
        assert_eq!((r.x, r.y, r.width, r.height), (10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn test_point_creation() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn test_rect_creation() {
        let r = Rect::new(5.0, 10.0, 100.0, 50.0);
        assert_eq!(r.x, 5.0);
        assert_eq!(r.y, 10.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn test_rect_from_points() {
        let r = Rect::from_points(10.0, 20.0, 110.0, 70.0);
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn test_rect_edges() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.left(), 10.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.top(), 20.0);
        assert_eq!(r.bottom(), 70.0);
    }

    #[test]
    fn test_rect_center() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        let center = r.center();
        assert_eq!(center.x, 50.0);
        assert_eq!(center.y, 25.0);
    }

    #[test]
    fn test_rect_intersects() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let r3 = Rect::new(200.0, 200.0, 100.0, 100.0);

        assert!(r1.intersects(&r2));
        assert!(r2.intersects(&r1));
        assert!(!r1.intersects(&r3));
        assert!(!r3.intersects(&r1));
    }

    #[test]
    fn test_rect_contains_point() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let p1 = Point::new(50.0, 50.0);
        let p2 = Point::new(150.0, 150.0);
        let p3 = Point::new(0.0, 0.0);
        let p4 = Point::new(100.0, 100.0);

        assert!(r.contains_point(&p1));
        assert!(!r.contains_point(&p2));
        assert!(r.contains_point(&p3));
        assert!(r.contains_point(&p4));
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0.0, 0.0, 50.0, 50.0);
        let r2 = Rect::new(25.0, 25.0, 50.0, 50.0);
        let union = r1.union(&r2);

        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.right(), 75.0);
        assert_eq!(union.bottom(), 75.0);
    }

    #[test]
    fn test_rect_area() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(r.area(), 5000.0);
    }

    #[test]
    fn test_euclidean_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(euclidean_distance(&p1, &p2), 5.0);

        let p3 = Point::new(1.0, 1.0);
        let p4 = Point::new(1.0, 1.0);
        assert_eq!(euclidean_distance(&p3, &p4), 0.0);
    }
}
