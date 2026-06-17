//! Defines the [PdfPageTextSegments] struct, a collection of all the distinct rectangular
//! areas of a single [PdfPage] occupied by spans of text that share a common text style.

use crate::bindgen::FS_RECTF;
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::page::text::PdfPageText;
use crate::pdf::document::page::text::segment::PdfPageTextSegment;
use crate::pdf::rect::PdfRect;
use std::ops::{Range, RangeInclusive};
use std::os::raw::c_int;

#[cfg(doc)]
use {crate::pdf::document::page::PdfPage, crate::pdf::document::page::object::text::PdfPageTextObject};

/// The zero-based index of a single [PdfPageTextSegment] inside its containing
/// [PdfPageTextSegments] collection.
pub type PdfPageTextSegmentIndex = usize;

/// A collection of all the distinct rectangular areas of a single [PdfPage] occupied by
/// spans of text that share a common text style.
///
/// Pdfium automatically merges smaller text boxes into larger ones if all enclosed characters
/// are on the same line and share the same font settings.
pub struct PdfPageTextSegments<'a> {
    text: &'a PdfPageText<'a>,
    start: i32,
    characters: i32,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageTextSegments<'a> {
    #[inline]
    pub(crate) fn new(
        text: &'a PdfPageText<'a>,
        start: i32,
        characters: i32,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageTextSegments {
            text,
            start,
            characters,
            bindings,
        }
    }

    /// Returns the number of distinct rectangular areas occupied by text in the containing [PdfPage].
    ///
    /// Pdfium automatically merges smaller text boxes into larger ones if all enclosed characters
    /// are on the same line and share the same font settings. The number of rectangular text segments
    /// returned by this function therefore indicates the minimum number of spans of text that
    /// share text styles on the page. The number of individual [PdfPageTextObject] objects on
    /// the page may be much larger than the number of text segments.
    #[inline]
    pub fn len(&self) -> PdfPageTextSegmentIndex {
        self.bindings
            .FPDFText_CountRects(self.text.text_page_handle(), self.start, self.characters)
            as PdfPageTextSegmentIndex
    }

    /// Returns `true` if this [PdfPageTextSegments] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of segments)` for this [PdfPageTextSegments] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageTextSegmentIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of segments - 1)` for this
    /// [PdfPageTextSegments] collection.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfPageTextSegmentIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfPageTextSegment] from this [PdfPageTextSegments] collection.
    #[inline]
    pub fn get(&self, index: PdfPageTextSegmentIndex) -> Result<PdfPageTextSegment<'_>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::TextSegmentIndexOutOfBounds);
        }

        let mut left = 0.0;

        let mut bottom = 0.0;

        let mut right = 0.0;

        let mut top = 0.0;

        let result = self.bindings.FPDFText_GetRect(
            self.text.text_page_handle(),
            index as c_int,
            &mut left,
            &mut top,
            &mut right,
            &mut bottom,
        );

        PdfRect::from_pdfium_as_result(
            result,
            FS_RECTF {
                left: left as f32,
                top: top as f32,
                right: right as f32,
                bottom: bottom as f32,
            },
            self.bindings,
        )
        .map(|rect| PdfPageTextSegment::from_pdfium(self.text, rect))
    }

    /// Returns an iterator over all the text segments in this [PdfPageTextSegments] collection.
    ///
    /// Pdfium automatically merges smaller text boxes into larger text segments if all
    /// enclosed characters are on the same line and share the same font settings. The number of
    /// individual [PdfPageTextObject] objects on the page may be much larger than the number of
    /// text segments.
    #[inline]
    pub fn iter(&self) -> PdfPageTextSegmentsIterator<'_> {
        PdfPageTextSegmentsIterator::new(self)
    }
}

/// An iterator over all the [PdfPageTextSegment] objects in a [PdfPageTextSegments] collection.
pub struct PdfPageTextSegmentsIterator<'a> {
    segments: &'a PdfPageTextSegments<'a>,
    next_index: PdfPageTextSegmentIndex,
}

impl<'a> PdfPageTextSegmentsIterator<'a> {
    #[inline]
    pub(crate) fn new(segments: &'a PdfPageTextSegments<'a>) -> Self {
        PdfPageTextSegmentsIterator {
            segments,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PdfPageTextSegmentsIterator<'a> {
    type Item = PdfPageTextSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.segments.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
