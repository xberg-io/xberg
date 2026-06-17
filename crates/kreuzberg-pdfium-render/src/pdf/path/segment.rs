//! Defines the [PdfPathSegment] struct, exposing functionality related to a single
//! path segment in a `PdfPathSegments` collection.

use crate::bindgen::{
    FPDF_PATHSEGMENT, FPDF_SEGMENT_BEZIERTO, FPDF_SEGMENT_LINETO, FPDF_SEGMENT_MOVETO, FPDF_SEGMENT_UNKNOWN,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::matrix::PdfMatrix;
use crate::pdf::points::PdfPoints;
use std::os::raw::c_float;

/// The type of a single [PdfPathSegment].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum PdfPathSegmentType {
    Unknown = FPDF_SEGMENT_UNKNOWN as isize,
    LineTo = FPDF_SEGMENT_LINETO as isize,
    BezierTo = FPDF_SEGMENT_BEZIERTO as isize,
    MoveTo = FPDF_SEGMENT_MOVETO as isize,
}

impl PdfPathSegmentType {
    #[inline]
    pub(crate) fn from_pdfium(segment_type: i32) -> Result<PdfPathSegmentType, PdfiumError> {
        if segment_type == FPDF_SEGMENT_UNKNOWN {
            return Ok(PdfPathSegmentType::Unknown);
        }

        match segment_type as u32 {
            FPDF_SEGMENT_LINETO => Ok(PdfPathSegmentType::LineTo),
            FPDF_SEGMENT_BEZIERTO => Ok(PdfPathSegmentType::BezierTo),
            FPDF_SEGMENT_MOVETO => Ok(PdfPathSegmentType::MoveTo),
            _ => Err(PdfiumError::UnknownPathSegmentType),
        }
    }
}

/// A single [PdfPathSegment] in a `PdfPathSegments` collection.
pub struct PdfPathSegment<'a> {
    handle: FPDF_PATHSEGMENT,
    matrix: Option<PdfMatrix>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPathSegment<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_PATHSEGMENT,
        matrix: Option<PdfMatrix>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        Self {
            handle,
            matrix,
            bindings,
        }
    }

    /// Returns the internal `FPDF_PATHSEGMENT` handle for this [PdfPathSegment].
    #[inline]
    pub(crate) fn handle(&self) -> FPDF_PATHSEGMENT {
        self.handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPathSegment].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the [PdfPathSegmentType] of this [PdfPathSegment].
    #[inline]
    pub fn segment_type(&self) -> PdfPathSegmentType {
        PdfPathSegmentType::from_pdfium(self.bindings().FPDFPathSegment_GetType(self.handle))
            .unwrap_or(PdfPathSegmentType::Unknown)
    }

    /// Returns `true` if this [PdfPathSegment] closes the current sub-path.
    #[inline]
    pub fn is_close(&self) -> bool {
        self.bindings()
            .is_true(self.bindings().FPDFPathSegment_GetClose(self.handle()))
    }

    /// Returns the horizontal and vertical destination positions of this [PdfPathSegment].
    pub fn point(&self) -> (PdfPoints, PdfPoints) {
        let mut x: c_float = 0.0;

        let mut y: c_float = 0.0;

        if self
            .bindings()
            .is_true(self.bindings().FPDFPathSegment_GetPoint(self.handle(), &mut x, &mut y))
        {
            let x = PdfPoints::new(x as f32);

            let y = PdfPoints::new(y as f32);

            match self.matrix.as_ref() {
                None => (x, y),
                Some(matrix) => matrix.apply_to_points(x, y),
            }
        } else {
            (PdfPoints::ZERO, PdfPoints::ZERO)
        }
    }

    /// Returns the horizontal destination position of this [PdfPathSegment].
    #[inline]
    pub fn x(&self) -> PdfPoints {
        self.point().0
    }

    /// Returns the vertical destination position of this [PdfPathSegment].
    #[inline]
    pub fn y(&self) -> PdfPoints {
        self.point().1
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_point_transform() {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf().unwrap();

        let mut page = document
            .pages_mut()
            .create_page_at_start(PdfPagePaperSize::a4())
            .unwrap();

        let object = page
            .objects_mut()
            .create_path_object_line(
                PdfPoints::new(100.0),
                PdfPoints::new(200.0),
                PdfPoints::new(300.0),
                PdfPoints::new(400.0),
                PdfColor::BEIGE,
                PdfPoints::new(1.0),
            )
            .unwrap();

        let delta_x = PdfPoints::new(50.0);
        let delta_y = PdfPoints::new(-25.0);

        let matrix = PdfMatrix::identity().translate(delta_x, delta_y).unwrap();

        let raw_segment_0 = object.as_path_object().unwrap().segments().get(0).unwrap();
        let raw_segment_1 = object.as_path_object().unwrap().segments().get(1).unwrap();

        let transformed_segment_0 = object
            .as_path_object()
            .unwrap()
            .segments()
            .transform(matrix)
            .get(0)
            .unwrap();

        let transformed_segment_1 = object
            .as_path_object()
            .unwrap()
            .segments()
            .transform(matrix)
            .get(1)
            .unwrap();

        assert_eq!(transformed_segment_0.x(), raw_segment_0.x() + delta_x);
        assert_eq!(transformed_segment_0.y(), raw_segment_0.y() + delta_y);
        assert_eq!(transformed_segment_1.x(), raw_segment_1.x() + delta_x);
        assert_eq!(transformed_segment_1.y(), raw_segment_1.y() + delta_y);
    }

    #[test]
    fn test_point_transform_during_iteration() {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf().unwrap();

        let mut page = document
            .pages_mut()
            .create_page_at_start(PdfPagePaperSize::a4())
            .unwrap();

        let object = page
            .objects_mut()
            .create_path_object_line(
                PdfPoints::new(100.0),
                PdfPoints::new(200.0),
                PdfPoints::new(300.0),
                PdfPoints::new(400.0),
                PdfColor::BEIGE,
                PdfPoints::new(1.0),
            )
            .unwrap();

        let raw_points: Vec<(PdfPoints, PdfPoints)> = object
            .as_path_object()
            .unwrap()
            .segments()
            .iter()
            .map(|segment| segment.point())
            .collect();

        let delta_x = PdfPoints::new(50.0);
        let delta_y = PdfPoints::new(-25.0);

        let matrix = PdfMatrix::identity().translate(delta_x, delta_y).unwrap();

        let transformed_points: Vec<(PdfPoints, PdfPoints)> = object
            .as_path_object()
            .unwrap()
            .segments()
            .transform(matrix)
            .iter()
            .map(|segment| segment.point())
            .collect();

        for (raw, transformed) in raw_points.iter().zip(transformed_points) {
            assert_eq!(transformed.0, raw.0 + delta_x);
            assert_eq!(transformed.1, raw.1 + delta_y);
        }
    }
}
