//! Defines the [PdfClipPath] struct, exposing functionality related to a clip path.

use crate::bindgen::FPDF_CLIPPATH;
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::page::object::ownership::PdfPageObjectOwnership;
use crate::pdf::path::segment::PdfPathSegment;
use crate::pdf::path::segments::{PdfPathSegmentIndex, PdfPathSegments, PdfPathSegmentsIterator};
use std::convert::TryInto;
use std::ops::{Range, RangeInclusive};
use std::os::raw::c_int;

/// The zero-based index of a single [PdfClipPathSegments] path object inside its
/// containing [PdfClipPath] instance.
pub type PdfClipPathSegmentIndex = u16;

/// A single clip path, containing zero or more path objects.
pub struct PdfClipPath<'a> {
    handle: FPDF_CLIPPATH,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfClipPath<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_CLIPPATH,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        Self {
            handle,
            ownership,
            bindings,
        }
    }

    /// Returns the internal `FPDF_CLIPPATH` handle for this [PdfPathSegment].
    #[inline]
    pub(crate) fn handle(&self) -> FPDF_CLIPPATH {
        self.handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfClipPath] instance.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the number of path objects inside this [PdfClipPath] instance.
    #[inline]
    pub fn len(&self) -> PdfClipPathSegmentIndex {
        self.bindings().FPDFClipPath_CountPaths(self.handle()) as PdfClipPathSegmentIndex
    }

    /// Returns `true` if this [PdfClipPath] instance is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of path objects)` for this [PdfClipPath] instance.
    #[inline]
    pub fn as_range(&self) -> Range<PdfClipPathSegmentIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of path objects - 1)` for this [PdfClipPath] instance.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfClipPathSegmentIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfClipPathSegments] path object from this [PdfClipPath] instance.
    pub fn get(&self, index: PdfClipPathSegmentIndex) -> Result<PdfClipPathSegments<'a>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PdfClipPathSegmentIndexOutOfBounds);
        }

        Ok(PdfClipPathSegments::from_pdfium(self.handle(), index, self.bindings()))
    }

    /// Returns an iterator over all the path objects in this [PdfClipPath] instance.
    #[inline]
    pub fn iter(&self) -> PdfClipPathIterator<'_> {
        PdfClipPathIterator::new(self)
    }
}

impl<'a> Drop for PdfClipPath<'a> {
    /// Closes this [PdfClipPath], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        if !self.ownership.is_owned() {
            // Responsibility for de-allocation lies with us, not Pdfium, since
            // the clip path is not attached to a page, a page object, or an annotation.

            self.bindings.FPDF_DestroyClipPath(self.handle)
        }
    }
}

/// An iterator over all the [PdfPathSegments] path objects in a [PdfClipPath] instance.
pub struct PdfClipPathIterator<'a> {
    clip_path: &'a PdfClipPath<'a>,
    next_index: PdfClipPathSegmentIndex,
}

impl<'a> PdfClipPathIterator<'a> {
    #[inline]
    pub(crate) fn new(clip_path: &'a PdfClipPath<'a>) -> Self {
        PdfClipPathIterator {
            clip_path,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PdfClipPathIterator<'a> {
    type Item = PdfClipPathSegments<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.clip_path.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}

/// The collection of [PdfPathSegment] objects inside a single path within a clip path.
pub struct PdfClipPathSegments<'a> {
    handle: FPDF_CLIPPATH,
    index: PdfClipPathSegmentIndex,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfClipPathSegments<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_CLIPPATH,
        path_index: PdfClipPathSegmentIndex,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        Self {
            handle,
            index: path_index,
            bindings,
        }
    }
}

impl<'a> PdfPathSegments<'a> for PdfClipPathSegments<'a> {
    #[inline]
    fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    #[inline]
    fn len(&self) -> PdfPathSegmentIndex {
        self.bindings()
            .FPDFClipPath_CountPathSegments(self.handle, self.index as i32)
            .try_into()
            .unwrap_or(0)
    }

    fn get(&self, index: PdfPathSegmentIndex) -> Result<PdfPathSegment<'a>, PdfiumError> {
        let handle = self
            .bindings()
            .FPDFClipPath_GetPathSegment(self.handle, self.index as i32, index as c_int);

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfPathSegment::from_pdfium(handle, None, self.bindings()))
        }
    }

    #[inline]
    fn iter(&'a self) -> PdfPathSegmentsIterator<'a> {
        PdfPathSegmentsIterator::new(self)
    }
}
