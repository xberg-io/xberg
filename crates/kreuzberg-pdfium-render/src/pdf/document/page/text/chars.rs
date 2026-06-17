//! Defines the [PdfPageTextChars] struct, a collection of nominated [PdfPageTextChar]
//! characters selected from a single [PdfPage].

use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGE, FPDF_TEXTPAGE};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::page::PdfPageText;
use crate::pdf::document::page::text::char::PdfPageTextChar;
use crate::pdf::points::PdfPoints;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// The zero-based index of a single [PdfPageTextChar] inside its containing [PdfPageTextChars] collection.
pub type PdfPageTextCharIndex = usize;

/// A collection of nominated [PdfPageTextChar] characters selected from a single [PdfPage].
pub struct PdfPageTextChars<'a> {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    text_page_handle: FPDF_TEXTPAGE,
    char_indices: Vec<i32>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageTextChars<'a> {
    #[inline]
    pub(crate) fn new(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        text_page_handle: FPDF_TEXTPAGE,
        char_indices: Vec<i32>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageTextChars {
            document_handle,
            page_handle,
            text_page_handle,
            char_indices,
            bindings,
        }
    }

    /// Returns the internal `FPDF_DOCUMENT` handle of the [PdfDocument] containing this
    /// [PdfPageTextChars] collection.
    #[inline]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the internal `FPDF_PAGE` handle of the [PdfPage] containing this
    /// [PdfPageTextChars] collection.
    #[inline]
    pub(crate) fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    /// Returns the internal `FPDF_TEXTPAGE` handle for this [PdfPageTextChars] collection.
    #[inline]
    pub(crate) fn text_page_handle(&self) -> FPDF_TEXTPAGE {
        self.text_page_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageTextChars] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the index in the containing [PdfPage] of the first character in this
    /// [PdfPageTextChars] collection, if any.
    #[inline]
    pub fn first_char_index(&self) -> Option<PdfPageTextCharIndex> {
        self.char_indices.first().map(|index| *index as PdfPageTextCharIndex)
    }

    /// Returns the number of individual characters in this [PdfPageTextChars] collection.
    #[inline]
    pub fn len(&self) -> PdfPageTextCharIndex {
        self.char_indices.len()
    }

    /// Returns the index in the containing [PdfPage] of the last character in this
    /// [PdfPageTextChars] collection, if any.
    #[inline]
    pub fn last_char_index(&self) -> Option<PdfPageTextCharIndex> {
        self.char_indices.last().map(|index| *index as PdfPageTextCharIndex)
    }

    /// Returns `true` if this [PdfPageTextChars] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a single [PdfPageTextChar] from this [PdfPageTextChars] collection.
    #[inline]
    pub fn get(&self, index: PdfPageTextCharIndex) -> Result<PdfPageTextChar<'_>, PdfiumError> {
        match self.char_indices.get(index) {
            Some(index) => Ok(PdfPageTextChar::from_pdfium(
                self.document_handle(),
                self.page_handle(),
                self.text_page_handle(),
                *index,
                self.bindings(),
            )),
            None => Err(PdfiumError::CharIndexOutOfBounds),
        }
    }

    /// Returns the character at the given x and y positions on the containing [PdfPage], if any.
    #[inline]
    pub fn get_char_at_point(&self, x: PdfPoints, y: PdfPoints) -> Option<PdfPageTextChar<'_>> {
        self.get_char_near_point(x, PdfPoints::ZERO, y, PdfPoints::ZERO)
    }

    /// Returns the character near to the given x and y positions on the containing [PdfPage],
    /// if any. The returned character will be no further from the given positions than the given
    /// tolerance values.
    #[inline]
    pub fn get_char_near_point(
        &self,
        x: PdfPoints,
        tolerance_x: PdfPoints,
        y: PdfPoints,
        tolerance_y: PdfPoints,
    ) -> Option<PdfPageTextChar<'_>> {
        PdfPageText::get_char_index_near_point(self.text_page_handle(), x, tolerance_x, y, tolerance_y, self.bindings())
            .ok_or(PdfiumError::CharIndexOutOfBounds)
            .and_then(|index| self.get(index))
            .ok()
    }

    /// Returns an iterator over all the characters in this [PdfPageTextChars] collection.
    #[inline]
    pub fn iter(&self) -> PdfPageTextCharsIterator<'_> {
        PdfPageTextCharsIterator::new(self)
    }
}

/// An iterator over all the [PdfPageTextChar] objects in a [PdfPageTextChars] collection.
pub struct PdfPageTextCharsIterator<'a> {
    chars: &'a PdfPageTextChars<'a>,
    next_index: PdfPageTextCharIndex,
}

impl<'a> PdfPageTextCharsIterator<'a> {
    #[inline]
    pub(crate) fn new(chars: &'a PdfPageTextChars) -> Self {
        PdfPageTextCharsIterator { chars, next_index: 0 }
    }
}

impl<'a> Iterator for PdfPageTextCharsIterator<'a> {
    type Item = PdfPageTextChar<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.chars.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
