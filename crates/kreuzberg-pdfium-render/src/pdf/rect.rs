//! Defines the [PdfRect] struct, a rectangle measured in [PdfPoints].

use crate::bindgen::{FPDF_BOOL, FS_RECTF};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::matrix::PdfMatrix;
use crate::pdf::points::PdfPoints;
use crate::pdf::quad_points::PdfQuadPoints;
use itertools::{max, min};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// A rectangle measured in [PdfPoints].
///
/// The coordinate space of a `PdfPage` has its origin (0,0) at the bottom left of the page,
/// with x values increasing as coordinates move horizontally to the right and
/// y values increasing as coordinates move vertically up.
#[derive(Debug, Copy, Clone)]
pub struct PdfRect {
    // TODO: AJRC - 28/12/24 - direct field access to be removed as part of release 0.9.0.
    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfRect::bottom() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub bottom: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfRect::left() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub left: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfRect::top() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub top: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfRect::left() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub right: PdfPoints,
}

impl PdfRect {
    /// A [PdfRect] object with the identity value (0.0, 0.0, 0.0, 0.0).
    pub const ZERO: PdfRect = PdfRect::zero();

    /// A [PdfRect] object that encloses the entire addressable `PdfPage` coordinate space of
    /// ([-PdfPoints::MAX], [-PdfPoints::MAX], [PdfPoints::MAX], [PdfPoints::MAX]).
    pub const MAX: PdfRect = PdfRect::new(PdfPoints::MIN, PdfPoints::MIN, PdfPoints::MAX, PdfPoints::MAX);

    #[inline]
    pub(crate) fn from_pdfium(rect: FS_RECTF) -> Self {
        Self::new_from_values(rect.bottom, rect.left, rect.top, rect.right)
    }

    #[inline]
    pub(crate) fn from_pdfium_as_result(
        result: FPDF_BOOL,
        rect: FS_RECTF,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<PdfRect, PdfiumError> {
        if !bindings.is_true(result) {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfRect::from_pdfium(rect))
        }
    }

    /// Creates a new [PdfRect] from the given [PdfPoints] measurements.
    ///
    /// The coordinate space of a `PdfPage` has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    pub const fn new(bottom: PdfPoints, left: PdfPoints, top: PdfPoints, right: PdfPoints) -> Self {
        // Check all given points to ensure they are ordered in accordance with the PDF
        // coordinate system, i.e. bottom should always be <= top and left should always
        // be <= right. See: https://github.com/ajrcarey/pdfium-render/issues/223

        let (ordered_bottom, ordered_top) = if bottom.value > top.value {
            (top, bottom)
        } else {
            (bottom, top)
        };

        let (ordered_left, ordered_right) = if left.value > right.value {
            (right, left)
        } else {
            (left, right)
        };

        #[allow(deprecated)]
        Self {
            bottom: ordered_bottom,
            left: ordered_left,
            top: ordered_top,
            right: ordered_right,
        }
    }

    /// Creates a new [PdfRect] from the given raw points values.
    ///
    /// The coordinate space of a `PdfPage` has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    pub const fn new_from_values(bottom: f32, left: f32, top: f32, right: f32) -> Self {
        Self::new(
            PdfPoints::new(bottom),
            PdfPoints::new(left),
            PdfPoints::new(top),
            PdfPoints::new(right),
        )
    }

    /// Creates a new [PdfRect] object with all values set to 0.0.
    ///
    /// Consider using the compile-time constant value [PdfRect::ZERO]
    /// rather than calling this function directly.
    #[inline]
    pub const fn zero() -> Self {
        Self::new_from_values(0.0, 0.0, 0.0, 0.0)
    }

    /// Returns the left-most extent of this [PdfRect].
    #[inline]
    pub const fn left(&self) -> PdfPoints {
        #[allow(deprecated)]
        self.left
    }

    /// Returns the right-most extent of this [PdfRect].
    #[inline]
    pub const fn right(&self) -> PdfPoints {
        #[allow(deprecated)]
        self.right
    }

    /// Returns the bottom-most extent of this [PdfRect].
    #[inline]
    pub const fn bottom(&self) -> PdfPoints {
        #[allow(deprecated)]
        self.bottom
    }

    /// Returns the top-most extent of this [PdfRect].
    #[inline]
    pub const fn top(&self) -> PdfPoints {
        #[allow(deprecated)]
        self.top
    }

    /// Returns the width of this [PdfRect].
    #[inline]
    pub fn width(&self) -> PdfPoints {
        self.right() - self.left()
    }

    /// Returns the height of this [PdfRect].
    #[inline]
    pub fn height(&self) -> PdfPoints {
        self.top() - self.bottom()
    }

    #[inline]
    /// Returns `true` if the given point lies inside this [PdfRect].
    pub fn contains(&self, x: PdfPoints, y: PdfPoints) -> bool {
        self.contains_x(x) && self.contains_y(y)
    }

    /// Returns `true` if the given horizontal coordinate lies inside this [PdfRect].
    #[inline]
    pub fn contains_x(&self, x: PdfPoints) -> bool {
        self.left() <= x && self.right() >= x
    }

    /// Returns `true` if the given vertical coordinate lies inside this [PdfRect].
    #[inline]
    pub fn contains_y(&self, y: PdfPoints) -> bool {
        self.bottom() <= y && self.top() >= y
    }

    /// Returns `true` if the bounds of this [PdfRect] lie entirely within the given rectangle.
    #[inline]
    pub fn is_inside(&self, other: &PdfRect) -> bool {
        self.left() >= other.left()
            && self.right() <= other.right()
            && self.top() <= other.top()
            && self.bottom() >= other.bottom()
    }

    /// Returns `true` if the bounds of this [PdfRect] lie at least partially within
    /// the given rectangle.
    #[inline]
    pub fn does_overlap(&self, other: &PdfRect) -> bool {
        // As per https://stackoverflow.com/questions/306316/determine-if-two-rectangles-overlap-each-other

        self.left() < other.right()
            && self.right() > other.left()
            && self.top() > other.bottom()
            && self.bottom() < other.top()
    }

    /// Returns the result of applying the given [PdfMatrix] to each corner point of this [PdfRect].
    #[inline]
    pub fn transform(&self, matrix: PdfMatrix) -> PdfRect {
        let (x1, y1) = matrix.apply_to_points(self.left(), self.top());
        let (x2, y2) = matrix.apply_to_points(self.left(), self.bottom());
        let (x3, y3) = matrix.apply_to_points(self.right(), self.top());
        let (x4, y4) = matrix.apply_to_points(self.right(), self.bottom());

        PdfRect::new(
            min([y1, y2, y3, y4]).unwrap_or(PdfPoints::ZERO),
            min([x1, x2, x3, x4]).unwrap_or(PdfPoints::ZERO),
            max([y1, y2, y3, y4]).unwrap_or(PdfPoints::ZERO),
            max([x1, x2, x3, x4]).unwrap_or(PdfPoints::ZERO),
        )
    }

    /// Returns the [PdfQuadPoints] quadrilateral representation of this [PdfRect].
    #[inline]
    pub fn to_quad_points(&self) -> PdfQuadPoints {
        PdfQuadPoints::from_rect(self)
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> FS_RECTF {
        FS_RECTF {
            left: self.left().value,
            top: self.top().value,
            right: self.right().value,
            bottom: self.bottom().value,
        }
    }
}

// We could derive PartialEq automatically, but it's good practice to implement PartialEq
// by hand when implementing Hash.

impl PartialEq for PdfRect {
    fn eq(&self, other: &Self) -> bool {
        self.bottom() == other.bottom()
            && self.left() == other.left()
            && self.top() == other.top()
            && self.right() == other.right()
    }
}

// The f32 values inside PdfRect will never be NaN or Infinity, so these implementations
// of Eq and Hash are safe.

impl Eq for PdfRect {}

impl Hash for PdfRect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.bottom().value.to_bits());
        state.write_u32(self.left().value.to_bits());
        state.write_u32(self.top().value.to_bits());
        state.write_u32(self.right().value.to_bits());
    }
}

impl Display for PdfRect {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "PdfRect(bottom: {}, left: {}, top: {}, right: {})",
            self.bottom().value,
            self.left().value,
            self.top().value,
            self.right().value
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_rect_is_inside() {
        assert!(
            PdfRect::new_from_values(3.0, 3.0, 9.0, 9.0).is_inside(&PdfRect::new_from_values(2.0, 2.0, 10.0, 10.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 10.0, 10.0).is_inside(&PdfRect::new_from_values(3.0, 3.0, 9.0, 9.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).is_inside(&PdfRect::new_from_values(5.0, 4.0, 10.0, 10.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).is_inside(&PdfRect::new_from_values(8.0, 4.0, 10.0, 10.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).is_inside(&PdfRect::new_from_values(5.0, 8.0, 10.0, 10.0))
        );
    }

    #[test]
    fn test_rect_does_overlap() {
        assert!(
            PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).does_overlap(&PdfRect::new_from_values(5.0, 4.0, 10.0, 10.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).does_overlap(&PdfRect::new_from_values(8.0, 4.0, 10.0, 10.0))
        );

        assert!(
            !PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0).does_overlap(&PdfRect::new_from_values(5.0, 8.0, 10.0, 10.0))
        );
    }

    #[test]
    fn test_transform_rect() {
        let delta_x = PdfPoints::new(50.0);
        let delta_y = PdfPoints::new(-25.0);

        let matrix = PdfMatrix::identity().translate(delta_x, delta_y).unwrap();

        let bottom = PdfPoints::new(100.0);
        let top = PdfPoints::new(200.0);
        let left = PdfPoints::new(300.0);
        let right = PdfPoints::new(400.0);

        let rect = PdfRect::new(bottom, left, top, right);

        let result = rect.transform(matrix);

        assert_eq!(result.bottom(), bottom + delta_y);
        assert_eq!(result.top(), top + delta_y);
        assert_eq!(result.left(), left + delta_x);
        assert_eq!(result.right(), right + delta_x);
    }

    #[test]
    fn test_coordinate_space_order_guard() {
        // We create a rectangle with the horizontal and vertical coordinates
        // around the wrong way...

        let result = PdfRect::new_from_values(
            149.0, 544.0, 73.0, // Note: top < bottom but should be bottom <= top
            48.0, // Note: right < left but should be left <= right
        );

        // ... and confirm that the rectangle returns the coordinates in
        // the correct order.

        assert_eq!(result.bottom().value, 73.0);
        assert_eq!(result.top().value, 149.0);
        assert_eq!(result.left().value, 48.0);
        assert_eq!(result.right().value, 544.0);
    }
}
