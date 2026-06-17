//! Defines the [PdfMatrix] struct, a container for six floating-point values that represent
//! the six configurable elements of a nine-element 3x3 PDF transformation matrix.

use crate::bindgen::FS_MATRIX;
use crate::error::PdfiumError;
use crate::pdf::points::PdfPoints;
use crate::{create_transform_getters, create_transform_setters};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Mul, Sub};
use vecmath::{Matrix3, mat3_add, mat3_det, mat3_inv, mat3_sub, mat3_transposed, row_mat3_mul};

/// The floating-point data type used internally by a [PdfMatrix].
pub type PdfMatrixValue = f32;

/// Six floating-point values, labelled `a`, `b`, `c`, `d`, `e`, and `f`, that represent
/// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
///
/// Applying the matrix to any transformable object containing a `set_matrix()` function - such as
/// a page, clip path, individual page object, or page object group - will result in a
/// transformation of that object. Depending on the values specified in the matrix, the object
/// can be moved, scaled, rotated, or skewed.
///
/// **It is rare that a matrix needs to be used directly.** All transformable objects provide
/// convenient and expressive access to the most commonly used transformation operations without
/// requiring a matrix.
///
/// However, a matrix can be convenient when the same transformation values need to be applied
/// to a large set of transformable objects.
///
/// An overview of PDF transformation matrices can be found in the PDF Reference Manual
/// version 1.7 on page 204; a detailed description can be founded in section 4.2.3 on page 207.
#[derive(Debug, Copy, Clone)]
pub struct PdfMatrix {
    matrix: Matrix3<PdfMatrixValue>,
}

impl PdfMatrix {
    /// A [PdfMatrix] object with all matrix values set to 0.0.
    pub const ZERO: PdfMatrix = Self::zero();

    /// A [PdfMatrix] object with matrix values a and d set to 1.0
    /// and all other values set to 0.0.
    pub const IDENTITY: PdfMatrix = Self::identity();

    #[inline]
    pub(crate) fn from_pdfium(matrix: FS_MATRIX) -> Self {
        Self::new(matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f)
    }

    /// Creates a new [PdfMatrix] with the given matrix values.
    #[inline]
    pub const fn new(
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
    ) -> Self {
        Self {
            matrix: [[a, b, 0.0], [c, d, 0.0], [e, f, 1.0]],
        }
    }

    /// Creates a new [PdfMatrix] object with all matrix values set to 0.0.
    ///
    /// The return value of this function is identical to the constant [PdfMatrix::ZERO].
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Creates a new [PdfMatrix] object with matrix values a and d set to 1.0
    /// and all other values set to 0.0.
    ///
    /// The return value of this function is identical to the constant [PdfMatrix::IDENTITY].
    #[inline]
    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    /// Returns the value of `a`, the first of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn a(&self) -> PdfMatrixValue {
        self.matrix[0][0]
    }

    /// Sets the value of `a`, the first of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_a(&mut self, a: PdfMatrixValue) {
        self.matrix[0][0] = a;
    }

    /// Returns the value of `b`, the second of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn b(&self) -> PdfMatrixValue {
        self.matrix[0][1]
    }

    /// Sets the value of `b`, the second of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_b(&mut self, b: PdfMatrixValue) {
        self.matrix[0][1] = b;
    }

    /// Returns the value of `c`, the third of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn c(&self) -> PdfMatrixValue {
        self.matrix[1][0]
    }

    /// Sets the value of `c`, the third of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_c(&mut self, c: PdfMatrixValue) {
        self.matrix[1][0] = c;
    }

    /// Returns the value of `d`, the fourth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn d(&self) -> PdfMatrixValue {
        self.matrix[1][1]
    }

    /// Sets the value of `d`, the fourth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_d(&mut self, d: PdfMatrixValue) {
        self.matrix[1][1] = d;
    }

    /// Returns the value of `e`, the fifth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn e(&self) -> PdfMatrixValue {
        self.matrix[2][0]
    }

    /// Sets the value of `e`, the fifth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_e(&mut self, e: PdfMatrixValue) {
        self.matrix[2][0] = e;
    }

    /// Returns the value of `f`, the sixth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn f(&self) -> PdfMatrixValue {
        self.matrix[2][1]
    }

    /// Sets the value of `f`, the sixth of six floating-point values that represent
    /// the six configurable elements of a nine-element 3x3 PDF transformation matrix.
    #[inline]
    pub fn set_f(&mut self, f: PdfMatrixValue) {
        self.matrix[2][1] = f;
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> FS_MATRIX {
        FS_MATRIX {
            a: self.a(),
            b: self.b(),
            c: self.c(),
            d: self.d(),
            e: self.e(),
            f: self.f(),
        }
    }

    /// Returns the inverse of this [PdfMatrix].
    #[inline]
    pub fn invert(&self) -> PdfMatrix {
        Self {
            matrix: mat3_inv(self.matrix),
        }
    }

    /// Returns the transpose of this [PdfMatrix].
    #[inline]
    pub fn transpose(&self) -> PdfMatrix {
        Self {
            matrix: mat3_transposed(self.matrix),
        }
    }

    /// Returns the determinant of this [PdfMatrix].
    #[inline]
    pub fn determinant(&self) -> PdfMatrixValue {
        mat3_det(self.matrix)
    }

    /// Returns the result of adding the given [PdfMatrix] to this [PdfMatrix].
    #[inline]
    pub fn add(&self, other: PdfMatrix) -> PdfMatrix {
        Self {
            matrix: mat3_add(self.matrix, other.matrix),
        }
    }

    /// Returns the result of subtracting the given [PdfMatrix] from this [PdfMatrix].
    #[inline]
    pub fn subtract(&self, other: PdfMatrix) -> PdfMatrix {
        Self {
            matrix: mat3_sub(self.matrix, other.matrix),
        }
    }

    /// Returns the result of multiplying this [PdfMatrix] by the given [PdfMatrix].
    #[inline]
    pub fn multiply(&self, other: PdfMatrix) -> PdfMatrix {
        Self {
            matrix: row_mat3_mul(self.matrix, other.matrix),
        }
    }

    /// Returns the result of applying this [PdfMatrix] to the given coordinate pair expressed
    /// as [PdfPoints].
    #[inline]
    pub fn apply_to_points(&self, x: PdfPoints, y: PdfPoints) -> (PdfPoints, PdfPoints) {
        // The formula for applying transform to coordinates is provided in
        // The PDF Reference Manual, version 1.7, on page 208.

        (
            PdfPoints::new(self.a() * x.value + self.c() * y.value + self.e()),
            PdfPoints::new(self.b() * x.value + self.d() * y.value + self.f()),
        )
    }

    create_transform_setters!(
        Self,
        Result<Self, PdfiumError>,
        "this [PdfMatrix]",
        "this [PdfMatrix].",
        "this [PdfMatrix],"
    );

    // The internal implementation of the transform() function used by the create_transform_setters!() macro.
    fn transform_impl(
        mut self,
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
    ) -> Result<Self, PdfiumError> {
        let result = row_mat3_mul(self.matrix, [[a, b, 0.0], [c, d, 0.0], [e, f, 1.0]]);

        if mat3_det(result) == 0.0 {
            Err(PdfiumError::InvalidTransformationMatrix)
        } else {
            self.matrix = result;

            Ok(self)
        }
    }

    // The internal implementation of the reset_matrix() function used by the create_transform_setters!() macro.
    fn reset_matrix_impl(mut self, matrix: PdfMatrix) -> Result<Self, PdfiumError> {
        self.set_a(matrix.a());
        self.set_b(matrix.b());
        self.set_c(matrix.c());
        self.set_d(matrix.d());
        self.set_e(matrix.e());
        self.set_f(matrix.f());

        Ok(self)
    }

    create_transform_getters!("this [PdfMatrix]", "this [PdfMatrix].", "this [PdfMatrix],");

    // The internal implementation of the get_matrix_impl() function used by the create_transform_getters!() macro.
    #[inline]
    fn get_matrix_impl(&self) -> Result<PdfMatrix, PdfiumError> {
        Ok(*self)
    }
}

// We could derive PartialEq automatically, but it's good practice to implement PartialEq
// by hand when implementing Hash.

impl PartialEq for PdfMatrix {
    fn eq(&self, other: &Self) -> bool {
        (self.a() - other.a()).abs() < 0.0001
            && (self.b() - other.b()).abs() < 0.0001
            && (self.c() - other.c()).abs() < 0.0001
            && (self.d() - other.d()).abs() < 0.0001
            && (self.e() - other.e()).abs() < 0.0001
            && (self.f() - other.f()).abs() < 0.0001
    }
}

// The PdfMatrixValue values inside PdfMatrix will never be NaN or Infinity, so these implementations
// of Eq and Hash are safe.

impl Eq for PdfMatrix {}

impl Hash for PdfMatrix {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.a().to_bits());
        state.write_u32(self.b().to_bits());
        state.write_u32(self.c().to_bits());
        state.write_u32(self.d().to_bits());
        state.write_u32(self.e().to_bits());
        state.write_u32(self.f().to_bits());
    }
}

impl Add for PdfMatrix {
    type Output = PdfMatrix;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        // Add::add() shadows Self::add(), so we must be explicit about which function to call.
        Self::add(&self, rhs)
    }
}

impl Sub for PdfMatrix {
    type Output = PdfMatrix;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.subtract(rhs)
    }
}

impl Mul for PdfMatrix {
    type Output = PdfMatrix;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply(rhs)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_matrix_apply_to_points() {
        let delta_x = PdfPoints::new(50.0);
        let delta_y = PdfPoints::new(-25.0);

        let matrix = PdfMatrix::identity().translate(delta_x, delta_y).unwrap();

        let x = PdfPoints::new(300.0);
        let y = PdfPoints::new(400.0);

        let result = matrix.apply_to_points(x, y);

        assert_eq!(result.0, x + delta_x);
        assert_eq!(result.1, y + delta_y);
    }
}
