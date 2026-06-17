//! Defines the [PdfPageAnnotationAttachmentPoints] struct, a collection of all the
//! attachment points that visually associate a `PdfPageAnnotation` object with one or more
//! `PdfPageObject` objects on a `PdfPage`.

use crate::bindgen::FPDF_ANNOTATION;
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::quad_points::PdfQuadPoints;
use std::ops::{Range, RangeInclusive};

/// The zero-based index of a single attachment point inside its containing
/// [PdfPageAnnotationAttachmentPoints] collection.
pub type PdfPageAnnotationAttachmentPointIndex = usize;

/// A set of all the attachment points that visually connect a `PdfPageAnnotation` object
/// to one or more `PdfPageObject` objects on a `PdfPage`.
pub struct PdfPageAnnotationAttachmentPoints<'a> {
    annotation_handle: FPDF_ANNOTATION,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageAnnotationAttachmentPoints<'a> {
    #[inline]
    pub(crate) fn from_pdfium(annotation_handle: FPDF_ANNOTATION, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        PdfPageAnnotationAttachmentPoints {
            annotation_handle,
            bindings,
        }
    }

    /// Returns the number of attachment points in this [PdfPageAnnotationAttachmentPoints] collection.
    pub fn len(&self) -> PdfPageAnnotationAttachmentPointIndex {
        if self
            .bindings
            .is_true(self.bindings.FPDFAnnot_HasAttachmentPoints(self.annotation_handle))
        {
            self.bindings.FPDFAnnot_CountAttachmentPoints(self.annotation_handle)
                as PdfPageAnnotationAttachmentPointIndex
        } else {
            // Attachment points are not supported for this annotation type.

            0
        }
    }

    /// Returns `true` if this [PdfPageAnnotationAttachmentPoints] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of attachment points)` for this
    /// [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageAnnotationAttachmentPointIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of attachment points - 1)` for this
    /// [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfPageAnnotationAttachmentPointIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single attachment point, expressed as a set of [PdfQuadPoints], from this
    /// [PdfPageAnnotationAttachmentPoints] collection.
    pub fn get(&self, index: PdfPageAnnotationAttachmentPointIndex) -> Result<PdfQuadPoints, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageAnnotationAttachmentPointIndexOutOfBounds);
        }

        let mut result = PdfQuadPoints::ZERO.as_pdfium();

        if self.bindings.is_true(self.bindings.FPDFAnnot_GetAttachmentPoints(
            self.annotation_handle,
            index,
            &mut result,
        )) {
            Ok(PdfQuadPoints::from_pdfium(result))
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns the first attachment point, expressed as a set of [PdfQuadPoints],
    /// in this [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn first(&self) -> Result<PdfQuadPoints, PdfiumError> {
        if !self.is_empty() {
            self.get(0)
        } else {
            Err(PdfiumError::NoAttachmentPointsInPageAnnotation)
        }
    }

    /// Returns the last attachment point, expressed as a set of [PdfQuadPoints],
    /// in this [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn last(&self) -> Result<PdfQuadPoints, PdfiumError> {
        if !self.is_empty() {
            self.get(self.len() - 1)
        } else {
            Err(PdfiumError::NoAttachmentPointsInPageAnnotation)
        }
    }

    /// Creates a new attachment point from the given set of [PdfQuadPoints],
    /// and appends it to the end of this [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn create_attachment_point_at_end(&mut self, attachment_point: PdfQuadPoints) -> Result<(), PdfiumError> {
        if self.bindings.is_true(
            self.bindings
                .FPDFAnnot_AppendAttachmentPoints(self.annotation_handle, &attachment_point.as_pdfium()),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Replaces the attachment at the given index in this [PdfPageAnnotationAttachmentPoints]
    /// collection with the given updated set of [PdfQuadPoints].
    pub fn set_attachment_point_at_index(
        &mut self,
        index: PdfPageAnnotationAttachmentPointIndex,
        attachment_point: PdfQuadPoints,
    ) -> Result<(), PdfiumError> {
        if self.bindings.is_true(self.bindings.FPDFAnnot_SetAttachmentPoints(
            self.annotation_handle,
            index,
            &attachment_point.as_pdfium(),
        )) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns an iterator over all the attachment points in this [PdfPageAnnotationAttachmentPoints] collection.
    #[inline]
    pub fn iter(&self) -> PdfPageAnnotationAttachmentPointsIterator<'_> {
        PdfPageAnnotationAttachmentPointsIterator::new(self)
    }
}

/// An iterator over all the attachment points in a [PdfPageAnnotationAttachmentPoints] collection.
pub struct PdfPageAnnotationAttachmentPointsIterator<'a> {
    attachment_points: &'a PdfPageAnnotationAttachmentPoints<'a>,
    next_index: PdfPageAnnotationAttachmentPointIndex,
}

impl<'a> PdfPageAnnotationAttachmentPointsIterator<'a> {
    #[inline]
    pub(crate) fn new(attachment_points: &'a PdfPageAnnotationAttachmentPoints<'a>) -> Self {
        PdfPageAnnotationAttachmentPointsIterator {
            attachment_points,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PdfPageAnnotationAttachmentPointsIterator<'a> {
    type Item = PdfQuadPoints;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.attachment_points.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
