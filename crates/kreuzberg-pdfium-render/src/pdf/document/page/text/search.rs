//! Defines the [PdfPageTextSearch] struct, exposing functionality related to searching
//! the collection of Unicode characters visible in a single [PdfPage].

use crate::bindgen::{FPDF_MATCHCASE, FPDF_MATCHWHOLEWORD, FPDF_SCHHANDLE};
use crate::bindings::PdfiumLibraryBindings;
use crate::pdf::document::page::text::PdfPageText;
use crate::pdf::document::page::text::chars::PdfPageTextCharIndex;
use crate::pdf::document::page::text::segments::PdfPageTextSegments;
use std::os::raw::c_ulong;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// Configures the search options that should be applied when creating a new [PdfPageTextSearch] object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfSearchOptions {
    match_case: bool,
    match_whole_word: bool,
}

impl PdfSearchOptions {
    /// Creates a new [PdfSearchOptions] object with all settings initialized with their default values.
    pub fn new() -> Self {
        PdfSearchOptions {
            match_case: false,
            match_whole_word: false,
        }
    }

    /// Controls whether the search should be limited to results that exactly match the
    /// case of the search target. The default is `false`.
    pub fn match_case(mut self, do_match_case: bool) -> Self {
        self.match_case = do_match_case;

        self
    }

    /// Controls whether the search should be limited to results where the search target
    /// is a complete word, surrounded by punctuation or whitespace. The default is `false`.
    pub fn match_whole_word(mut self, do_match_whole_word: bool) -> Self {
        self.match_whole_word = do_match_whole_word;

        self
    }

    pub(crate) fn as_pdfium(&self) -> c_ulong {
        let mut flag = 0;

        if self.match_case {
            flag |= FPDF_MATCHCASE;
        }
        if self.match_whole_word {
            flag |= FPDF_MATCHWHOLEWORD;
        }

        flag as c_ulong
    }
}

impl Default for PdfSearchOptions {
    #[inline]
    fn default() -> Self {
        PdfSearchOptions::new()
    }
}

/// The direction in which to search for the next result.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum PdfSearchDirection {
    SearchForward,
    SearchBackward,
}

/// Yields the results of searching for a given string within the collection of Unicode characters
/// visible on a single [PdfPage].
pub struct PdfPageTextSearch<'a> {
    search_handle: FPDF_SCHHANDLE,
    text_page: &'a PdfPageText<'a>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageTextSearch<'a> {
    pub(crate) fn from_pdfium(
        search_handle: FPDF_SCHHANDLE,
        text_page: &'a PdfPageText<'a>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageTextSearch {
            search_handle,
            text_page,
            bindings,
        }
    }

    /// Returns the internal `FPDF_SCHHANDLE` handle for this [PdfPageTextSearch] object.
    #[inline]
    pub(crate) fn search_handle(&self) -> FPDF_SCHHANDLE {
        self.search_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageTextSearch] object.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the next search result yielded by this [PdfPageTextSearch] object
    /// in the direction [PdfSearchDirection::SearchForward].
    #[inline]
    pub fn find_next(&self) -> Option<PdfPageTextSegments<'_>> {
        self.get_next_result(PdfSearchDirection::SearchForward)
    }

    /// Returns the next search result yielded by this [PdfPageTextSearch] object
    /// in the direction [PdfSearchDirection::SearchBackward].
    #[inline]
    pub fn find_previous(&self) -> Option<PdfPageTextSegments<'_>> {
        self.get_next_result(PdfSearchDirection::SearchBackward)
    }

    /// Returns the next search result yielded by this [PdfPageTextSearch] object
    /// in the given direction.
    pub fn get_next_result(&self, direction: PdfSearchDirection) -> Option<PdfPageTextSegments<'_>> {
        let has_next = if direction == PdfSearchDirection::SearchForward {
            self.bindings().FPDFText_FindNext(self.search_handle()) != 0
        } else {
            self.bindings().FPDFText_FindPrev(self.search_handle()) != 0
        };

        if has_next {
            let start_index = self.bindings().FPDFText_GetSchResultIndex(self.search_handle());
            let count = self.bindings().FPDFText_GetSchCount(self.search_handle());

            Some(
                self.text_page
                    .segments_subset(start_index as PdfPageTextCharIndex, count as PdfPageTextCharIndex),
            )
        } else {
            None
        }
    }

    /// Returns an iterator over all search results yielded by this [PdfPageTextSearch]
    /// object in the given direction.
    #[inline]
    pub fn iter(&self, direction: PdfSearchDirection) -> PdfPageTextSearchIterator<'_> {
        PdfPageTextSearchIterator::new(self, direction)
    }
}

impl<'a> Drop for PdfPageTextSearch<'a> {
    /// Closes this [PdfPageTextSearch] object, releasing held memory.
    #[inline]
    fn drop(&mut self) {
        self.bindings().FPDFText_FindClose(self.search_handle());
    }
}

/// An iterator over all the [PdfPageTextSegments] search results yielded by a [PdfPageTextSearch] object.
pub struct PdfPageTextSearchIterator<'a> {
    search: &'a PdfPageTextSearch<'a>,
    direction: PdfSearchDirection,
}

impl<'a> PdfPageTextSearchIterator<'a> {
    pub(crate) fn new(search: &'a PdfPageTextSearch<'a>, direction: PdfSearchDirection) -> Self {
        PdfPageTextSearchIterator { search, direction }
    }
}

impl<'a> Iterator for PdfPageTextSearchIterator<'a> {
    type Item = PdfPageTextSegments<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.search.get_next_result(self.direction)
    }
}
