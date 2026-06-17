//! Defines the [PdfPageObjectContentMarks] struct, exposing functionality related to the
//! collection of content marks associated with a [PdfPageObject].

use crate::bindgen::FPDF_PAGEOBJECT;
use crate::bindings::PdfiumLibraryBindings;
use crate::pdf::document::page::object::content_mark::PdfPageObjectContentMark;
use std::os::raw::c_ulong;

/// The collection of content marks associated with a `PdfPageObject`.
///
/// Content marks provide a bridge between page objects and the PDF structure tree.
/// Use the [PdfPageObjectContentMarks::iter()] method to iterate over all marks,
/// or [PdfPageObjectContentMarks::get()] to access a mark by index.
pub struct PdfPageObjectContentMarks<'a> {
    object_handle: FPDF_PAGEOBJECT,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageObjectContentMarks<'a> {
    pub(crate) fn from_pdfium(object_handle: FPDF_PAGEOBJECT, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self {
            object_handle,
            bindings,
        }
    }

    /// Returns the number of content marks associated with this page object.
    pub fn len(&self) -> usize {
        let count = self.bindings.FPDFPageObj_CountMarks(self.object_handle);

        if count < 0 { 0 } else { count as usize }
    }

    /// Returns `true` if this page object has no content marks.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the content mark at the given index, or `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<PdfPageObjectContentMark<'_>> {
        if index >= self.len() {
            return None;
        }

        let handle = self.bindings.FPDFPageObj_GetMark(self.object_handle, index as c_ulong);

        if handle.is_null() {
            None
        } else {
            Some(PdfPageObjectContentMark::from_pdfium(handle, self.bindings))
        }
    }

    /// Returns an iterator over all content marks associated with this page object.
    pub fn iter(&self) -> PdfPageObjectContentMarksIterator<'_> {
        PdfPageObjectContentMarksIterator { marks: self, index: 0 }
    }
}

/// An iterator over the content marks in a [PdfPageObjectContentMarks] collection.
pub struct PdfPageObjectContentMarksIterator<'a> {
    marks: &'a PdfPageObjectContentMarks<'a>,
    index: usize,
}

impl<'a> Iterator for PdfPageObjectContentMarksIterator<'a> {
    type Item = PdfPageObjectContentMark<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.marks.len() {
            None
        } else {
            let result = self.marks.get(self.index);
            self.index += 1;
            result
        }
    }
}
