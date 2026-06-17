//! Defines the [PdfPathSegments] trait, a collection of all the `PdfPathSegment` objects in a
//! path page object, a font glyph path, or a clip path.

use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::path::segment::PdfPathSegment;
use std::ops::{Range, RangeInclusive};

/// The zero-based index of a single [PdfPathSegment] inside its containing [PdfPathSegments] collection.
pub type PdfPathSegmentIndex = u32;

/// The collection of [PdfPathSegment] objects inside a path page object, a font glyph path,
/// or a clip path.
pub trait PdfPathSegments<'a> {
    /// Returns the [PdfiumLibraryBindings] used by this [PdfPathSegments] collection.
    fn bindings(&self) -> &'a dyn PdfiumLibraryBindings;

    /// Returns the number of path segments in this [PdfPathSegments] collection.
    fn len(&self) -> PdfPathSegmentIndex;

    /// Returns `true` if this [PdfPathSegments] collection is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of path segments)` for this [PdfPathSegments] collection.
    #[inline]
    fn as_range(&self) -> Range<PdfPathSegmentIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of path segments - 1)` for this [PdfPathSegments] collection.
    #[inline]
    fn as_range_inclusive(&self) -> RangeInclusive<PdfPathSegmentIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfPathSegment] from this [PdfPathSegments] collection.
    fn get(&self, index: PdfPathSegmentIndex) -> Result<PdfPathSegment<'a>, PdfiumError>;

    /// Returns an iterator over all the path segments in this [PdfPathSegments] collection.
    fn iter(&'a self) -> PdfPathSegmentsIterator<'a>;
}

/// An iterator over all the [PdfPathSegment] objects in a [PdfPathSegments] collection.
pub struct PdfPathSegmentsIterator<'a> {
    segments: &'a dyn PdfPathSegments<'a>,
    next_index: PdfPathSegmentIndex,
}

impl<'a> PdfPathSegmentsIterator<'a> {
    #[inline]
    pub(crate) fn new(segments: &'a dyn PdfPathSegments<'a>) -> Self {
        PdfPathSegmentsIterator {
            segments,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PdfPathSegmentsIterator<'a> {
    type Item = PdfPathSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.segments.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
