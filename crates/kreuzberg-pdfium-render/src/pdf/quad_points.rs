//! Defines the [PdfQuadPoints] struct, a set of four coordinates expressed in [PdfPoints]
//! that outline the bounds of a four-sided quadrilateral.

use crate::bindgen::{FPDF_BOOL, FS_QUADPOINTSF};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::matrix::PdfMatrix;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// A set of four coordinates expressed in [PdfPoints] that outline the bounds of a
/// four-sided quadrilateral. The coordinates specify the quadrilateral's four vertices
/// in counter-clockwise order:
/// ```
/// (x4, y4)                     (x3, y3)
///        ._____________________.
///        |                     |
///        |                     |
///        !_____________________!
/// (x1, y1)                     (x2, y2)
/// ```
/// More information on quad points can be found in Section 8.30 of the PDF Reference Manual,
/// version 1.7, on page 634.
#[derive(Debug, Copy, Clone)]
pub struct PdfQuadPoints {
    pub x1: PdfPoints,
    pub y1: PdfPoints,
    pub x2: PdfPoints,
    pub y2: PdfPoints,
    pub x3: PdfPoints,
    pub y3: PdfPoints,
    pub x4: PdfPoints,
    pub y4: PdfPoints,
    // TODO: AJRC - 28/12/24 - directly available fields are in place purely to achieve
    // API-compatibility with PdfRect as part of changes to bounds() function implementation
    // in page objects. Direct field access should be removed in favour of accessor functions
    // as part of release 0.9.0.
    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfQuadPoints::bottom() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub bottom: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfQuadPoints::left() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub left: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfQuadPoints::top() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub top: PdfPoints,

    #[deprecated(
        since = "0.8.28",
        note = "Use the PdfQuadPoints::right() function instead of direct field access. Direct field access will be removed in release 0.9.0."
    )]
    pub right: PdfPoints,
}

impl PdfQuadPoints {
    /// A [PdfQuadPoints] object with the identity value (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0).
    #[allow(deprecated)]
    pub const ZERO: PdfQuadPoints = PdfQuadPoints {
        // TODO: AJRC - 28/12/24 - this in-place initialiser can be reduced to
        // PdfQuadPoints::zero() once deprecated features are removed as part of release 0.9.0.
        x1: PdfPoints::zero(),
        y1: PdfPoints::zero(),
        x2: PdfPoints::zero(),
        y2: PdfPoints::zero(),
        x3: PdfPoints::zero(),
        y3: PdfPoints::zero(),
        x4: PdfPoints::zero(),
        y4: PdfPoints::zero(),
        bottom: PdfPoints::zero(),
        left: PdfPoints::zero(),
        top: PdfPoints::zero(),
        right: PdfPoints::zero(),
    };

    #[inline]
    pub(crate) fn from_pdfium(points: FS_QUADPOINTSF) -> Self {
        PdfQuadPoints::new_from_values(
            points.x1, points.y1, points.x2, points.y2, points.x3, points.y3, points.x4, points.y4,
        )
    }

    #[inline]
    pub(crate) fn from_pdfium_as_result(
        result: FPDF_BOOL,
        points: FS_QUADPOINTSF,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<PdfQuadPoints, PdfiumError> {
        if !bindings.is_true(result) {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfQuadPoints::from_pdfium(points))
        }
    }

    /// Creates a new [PdfQuadPoints] from the given [PdfPoints] coordinate pairs.
    ///
    /// The coordinate space of a `PdfPage` has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    // TODO: AJRC - 28/12/24 - return to const function once left, right, bottom, top fields
    // removed as part of release 0.9.0.
    pub fn new(
        x1: PdfPoints,
        y1: PdfPoints,
        x2: PdfPoints,
        y2: PdfPoints,
        x3: PdfPoints,
        y3: PdfPoints,
        x4: PdfPoints,
        y4: PdfPoints,
    ) -> Self {
        #[allow(deprecated)]
        Self {
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
            x4,
            y4,
            left: *[x1, x2, x3, x4].iter().min().unwrap(),
            right: *[x1, x2, x3, x4].iter().max().unwrap(),
            bottom: *[y1, y2, y3, y4].iter().min().unwrap(),
            top: *[y1, y2, y3, y4].iter().max().unwrap(),
        }
    }

    /// Creates a new [PdfQuadPoints] from the given raw points values.
    ///
    /// The coordinate space of a `PdfPage` has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    // TODO: AJRC - 28/12/24 - return to const function once left, right, bottom, top fields
    // removed as part of release 0.9.0.
    pub fn new_from_values(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32) -> Self {
        Self::new(
            PdfPoints::new(x1),
            PdfPoints::new(y1),
            PdfPoints::new(x2),
            PdfPoints::new(y2),
            PdfPoints::new(x3),
            PdfPoints::new(y3),
            PdfPoints::new(x4),
            PdfPoints::new(y4),
        )
    }

    /// Creates a new [PdfQuadPoints] object with all values set to 0.0.
    ///
    /// Consider using the compile-time constant value [PdfQuadPoints::ZERO]
    /// rather than calling this function directly.
    #[inline]
    // TODO: AJRC - 28/12/24 - return to const function once left, right, bottom, top fields
    // removed as part of release 0.9.0.
    pub fn zero() -> Self {
        Self::new_from_values(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Creates a new [PdfQuadPoints] from the given [PdfRect].
    #[inline]
    pub fn from_rect(rect: &PdfRect) -> Self {
        PdfQuadPoints::new(
            rect.left(),
            rect.bottom(),
            rect.right(),
            rect.bottom(),
            rect.right(),
            rect.top(),
            rect.left(),
            rect.top(),
        )
    }

    /// Returns the left-most extent of this [PdfQuadPoints].
    pub fn left(&self) -> PdfPoints {
        #[allow(deprecated)]
        // TODO: AJRC - 28/12/24 - replace with inline calculation for release 0.9.0.
        self.left
        // *vec![self.x1, self.x2, self.x3, self.x4]
        //     .iter()
        //     .min()
        //     .unwrap()
    }

    /// Returns the right-most extent of this [PdfQuadPoints].
    pub fn right(&self) -> PdfPoints {
        #[allow(deprecated)]
        // TODO: AJRC - 28/12/24 - replace with inline calculation for release 0.9.0.
        self.right
        // *vec![self.x1, self.x2, self.x3, self.x4]
        //     .iter()
        //     .max()
        //     .unwrap()
    }

    /// Returns the bottom-most extent of this [PdfQuadPoints].
    pub fn bottom(&self) -> PdfPoints {
        #[allow(deprecated)]
        // TODO: AJRC - 28/12/24 - replace with inline calculation for release 0.9.0.
        self.bottom
        // *vec![self.y1, self.y2, self.y3, self.y4]
        //     .iter()
        //     .min()
        //     .unwrap()
    }

    /// Returns the top-most extent of this [PdfQuadPoints].
    pub fn top(&self) -> PdfPoints {
        #[allow(deprecated)]
        // TODO: AJRC - 28/12/24 - replace with inline calculation for release 0.9.0.
        self.top
        // *vec![self.y1, self.y2, self.y3, self.y4]
        //     .iter()
        //     .max()
        //     .unwrap()
    }

    /// Returns the width of this [PdfQuadPoints].
    #[inline]
    pub fn width(&self) -> PdfPoints {
        self.right() - self.left()
    }

    /// Returns the height of this [PdfQuadPoints].
    #[inline]
    pub fn height(&self) -> PdfPoints {
        self.top() - self.bottom()
    }

    /// Returns the result of applying the given [PdfMatrix] to each corner point
    // of this [PdfQuadPoints].
    #[inline]
    pub fn transform(&self, matrix: PdfMatrix) -> PdfQuadPoints {
        let (x1, y1) = matrix.apply_to_points(self.x1, self.y1);
        let (x2, y2) = matrix.apply_to_points(self.x2, self.y2);
        let (x3, y3) = matrix.apply_to_points(self.x3, self.y3);
        let (x4, y4) = matrix.apply_to_points(self.x4, self.y4);

        PdfQuadPoints::new(x1, y1, x2, y2, x3, y3, x4, y4)
    }

    /// Returns the smallest [PdfRect] that can completely enclose the quadrilateral
    /// outlined by this [PdfQuadPoints].
    pub fn to_rect(&self) -> PdfRect {
        let xs = [self.x1, self.x2, self.x3, self.x4];
        let ys = [self.y1, self.y2, self.y3, self.y4];

        PdfRect::new(
            *ys.iter().min().unwrap(),
            *xs.iter().min().unwrap(),
            *ys.iter().max().unwrap(),
            *xs.iter().max().unwrap(),
        )
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> FS_QUADPOINTSF {
        FS_QUADPOINTSF {
            x1: self.x1.value,
            y1: self.y1.value,
            x2: self.x2.value,
            y2: self.y2.value,
            x3: self.x3.value,
            y3: self.y3.value,
            x4: self.x4.value,
            y4: self.y4.value,
        }
    }
}

// We could derive PartialEq automatically, but it's good practice to implement PartialEq
// by hand when implementing Hash.

impl PartialEq for PdfQuadPoints {
    fn eq(&self, other: &Self) -> bool {
        self.x1 == other.x1
            && self.y1 == other.y1
            && self.x2 == other.x2
            && self.y2 == other.y2
            && self.x3 == other.x3
            && self.y3 == other.y3
            && self.x4 == other.x4
            && self.y4 == other.y4
    }
}

// The f32 values inside PdfQuadPoints will never be NaN or Infinity, so these implementations
// of Eq and Hash are safe.

impl Eq for PdfQuadPoints {}

impl Hash for PdfQuadPoints {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.x1.value.to_bits());
        state.write_u32(self.y1.value.to_bits());
        state.write_u32(self.x2.value.to_bits());
        state.write_u32(self.y2.value.to_bits());
        state.write_u32(self.x3.value.to_bits());
        state.write_u32(self.y3.value.to_bits());
        state.write_u32(self.x4.value.to_bits());
        state.write_u32(self.y4.value.to_bits());
    }
}

impl Display for PdfQuadPoints {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "PdfQuadPoints(x1: {}, y1: {}, x2: {}, y2: {}, x3: {}, y3: {}, x4: {}, y4: {})",
            self.x1.value,
            self.y1.value,
            self.x2.value,
            self.y2.value,
            self.x3.value,
            self.y3.value,
            self.x4.value,
            self.y4.value
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_quadpoints_extents() {
        let r = PdfRect::new_from_values(50.0, 100.0, 300.0, 200.0);

        assert_eq!(r.to_quad_points().left().value, 100.0);
        assert_eq!(r.to_quad_points().right().value, 200.0);
        assert_eq!(r.to_quad_points().top().value, 300.0);
        assert_eq!(r.to_quad_points().bottom().value, 50.0);

        assert_eq!(r.to_quad_points().width().value, 100.0);
        assert_eq!(r.to_quad_points().height().value, 250.0);
    }

    #[test]
    fn test_quadpoints_to_rect() {
        let r = PdfRect::new_from_values(100.0, 100.0, 200.0, 200.0);
        assert_eq!(r.to_quad_points().to_rect(), r);

        let m = PdfMatrix::identity().rotate_clockwise_degrees(45.0).unwrap();
        let r45 = r.transform(m);

        let q = r.to_quad_points();
        let q45 = q.transform(m);

        assert_eq!(q.to_rect(), r);
        assert_eq!(q45.to_rect(), r45);

        // It would be incredibly elegant to test

        // assert_eq!(q45.transform(m.invert()).to_rect(), r);

        // but sadly floating point rounding errors means the double-transformed values
        // are ever-so-slightly off (by a fraction of a PdfPoint). Let's test manually
        // so we can apply a comparison threshold.

        let s = q45.transform(m.invert()).to_rect();
        let threshold = PdfPoints::new(0.001);
        assert!((s.top() - r.top()).abs() < threshold);
        assert!((s.bottom() - r.bottom()).abs() < threshold);
        assert!((s.left() - r.left()).abs() < threshold);
        assert!((s.right() - r.right()).abs() < threshold);
    }
}
