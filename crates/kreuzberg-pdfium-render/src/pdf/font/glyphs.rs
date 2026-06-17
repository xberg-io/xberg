//! Defines the [PdfFontGlyphs] struct, a collection of all the [PdfFontGlyph] objects in a
//! [PdfFont].

use crate::bindgen::FPDF_FONT;
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::font::glyph::PdfFontGlyph;
use std::cell::Cell;
use std::ops::{Range, RangeInclusive};
use std::os::raw::c_uint;

#[cfg(doc)]
use crate::pdf::font::PdfFont;

/// The zero-based index of a single [PdfFontGlyph] inside its containing [PdfFontGlyphs] collection.
pub type PdfFontGlyphIndex = u16;

/// A collection of all the [PdfFontGlyph] objects in a [PdfFont].
pub struct PdfFontGlyphs<'a> {
    handle: FPDF_FONT,
    len: Cell<Option<PdfFontGlyphIndex>>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfFontGlyphs<'a> {
    #[inline]
    pub(crate) fn from_pdfium(handle: FPDF_FONT, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self {
            handle,
            len: Cell::new(None),
            bindings,
        }
    }

    /// Initializes the length of this [PdfFontGlyphs] collection.
    ///
    /// We avoid doing this on instantiation as it may not take constant time.
    /// We only incur the cost of initializing this value if the user actually requests the
    /// [PdfFontGlyphs] collection by calling the [PdfFont::glyphs] function.
    #[inline]
    pub(crate) fn initialize_len(&self) {
        if self.len.get().is_none() {
            // Pdfium does not provide a function that returns the number of glyphs in a font.
            // We use a binary search algorithm to determine the number of glyphs as efficiently
            // as possible.

            let len = self.find_maximum_valid_glyph_index(u16::MIN, u16::MAX).unwrap_or(0);

            self.len.replace(Some(len));
        }
    }

    /// Returns the highest index position of an extant glyph within the given index range.
    fn find_maximum_valid_glyph_index(&self, min: u16, max: u16) -> Option<u16> {
        // Exit immediately if the maximum valid glyph index lies outside the given index boundaries.

        if !self
            .bindings
            .FPDFFont_GetGlyphPath(self.handle, max as c_uint, 1.0)
            .is_null()
        {
            return Some(max);
        }

        if self
            .bindings
            .FPDFFont_GetGlyphPath(self.handle, min as c_uint, 1.0)
            .is_null()
        {
            return None;
        }

        // Partition the given index boundaries and recursively search.

        let mid = min + (max - min) / 2;

        if self
            .bindings
            .FPDFFont_GetGlyphPath(self.handle, mid as c_uint, 1.0)
            .is_null()
        {
            // The maximum valid glyph index must lie before the partition mid point.

            if mid > min {
                self.find_maximum_valid_glyph_index(min, mid - 1)
            } else {
                None
            }
        } else {
            // The maximum valid glyph index must lie after the partition mid point.

            if mid < max {
                self.find_maximum_valid_glyph_index(mid + 1, max)
            } else {
                Some(max)
            }
        }
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfFontGlyphs] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the number of glyphs in this [PdfFontGlyphs] collection.
    #[inline]
    pub fn len(&self) -> PdfFontGlyphIndex {
        self.len.get().unwrap_or(0)
    }

    /// Returns `true` if this [PdfFontGlyphs] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of glyphs)` for this [PdfFontGlyphs] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfFontGlyphIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of glyphs - 1)` for this [PdfFontGlyphs] collection.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfFontGlyphIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfFontGlyph] from this [PdfFontGlyphs] collection.
    pub fn get(&self, index: PdfFontGlyphIndex) -> Result<PdfFontGlyph<'a>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::FontGlyphIndexOutOfBounds);
        }

        Ok(PdfFontGlyph::from_pdfium(self.handle, index, self.bindings))
    }

    /// Returns an iterator over all the glyphs in this [PdfFontGlyphs] collection.
    #[inline]
    pub fn iter(&self) -> PdfFontGlyphsIterator<'_> {
        PdfFontGlyphsIterator::new(self)
    }
}

/// An iterator over all the [PdfFontGlyph] objects in a [PdfFontGlyphs] collection.
pub struct PdfFontGlyphsIterator<'a> {
    glyphs: &'a PdfFontGlyphs<'a>,
    next_index: PdfFontGlyphIndex,
}

impl<'a> PdfFontGlyphsIterator<'a> {
    #[inline]
    pub(crate) fn new(glyphs: &'a PdfFontGlyphs<'a>) -> Self {
        PdfFontGlyphsIterator { glyphs, next_index: 0 }
    }
}

impl<'a> Iterator for PdfFontGlyphsIterator<'a> {
    type Item = PdfFontGlyph<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.glyphs.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
