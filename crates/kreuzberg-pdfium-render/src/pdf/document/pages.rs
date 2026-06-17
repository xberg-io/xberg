//! Defines the [PdfPages] struct, a collection of all the `PdfPage` objects in a
//! `PdfDocument`.

use crate::bindgen::{
    FPDF_DOCUMENT, FPDF_FORMHANDLE, FPDF_PAGE, FS_SIZEF, PAGEMODE_FULLSCREEN, PAGEMODE_UNKNOWN,
    PAGEMODE_USEATTACHMENTS, PAGEMODE_USENONE, PAGEMODE_USEOC, PAGEMODE_USEOUTLINES, PAGEMODE_USETHUMBS, size_t,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::PdfPage;
use crate::pdf::document::page::index_cache::PdfPageIndexCache;
use crate::pdf::document::page::object::group::PdfPageGroupObject;
use crate::pdf::document::page::size::PdfPagePaperSize;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use crate::utils::mem::create_byte_buffer;
use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
use std::ops::{Range, RangeInclusive};
use std::os::raw::{c_double, c_int, c_void};

/// The zero-based index of a single [PdfPage] inside its containing [PdfPages] collection.
pub type PdfPageIndex = c_int;

/// A hint to a PDF document reader (such as Adobe Acrobat) as to how the creator intended
/// the [PdfPage] objects in a [PdfDocument] to be displayed to the viewer when the document is opened.
#[derive(Debug, Copy, Clone)]
pub enum PdfPageMode {
    /// No known page mode is set for this [PdfDocument].
    UnsetOrUnknown = PAGEMODE_UNKNOWN as isize,

    /// No page mode, i.e. neither the document outline nor thumbnail images should be visible,
    /// no side panels should be visible, and the document should not be displayed in full screen mode.
    None = PAGEMODE_USENONE as isize,

    /// Outline page mode: the document outline should be visible.
    ShowDocumentOutline = PAGEMODE_USEOUTLINES as isize,

    /// Thumbnail page mode: page thumbnails should be visible.
    ShowPageThumbnails = PAGEMODE_USETHUMBS as isize,

    /// Fullscreen page mode: no menu bar, window controls, or other windows should be visible.
    Fullscreen = PAGEMODE_FULLSCREEN as isize,

    /// The optional content group panel should be visible.
    ShowContentGroupPanel = PAGEMODE_USEOC as isize,

    /// The attachments panel should be visible.
    ShowAttachmentsPanel = PAGEMODE_USEATTACHMENTS as isize,
}

impl PdfPageMode {
    #[inline]
    pub(crate) fn from_pdfium(page_mode: i32) -> Option<Self> {
        // The PAGEMODE_* enum constants are a mixture of i32 and u32 values :/

        if page_mode == PAGEMODE_UNKNOWN {
            return Some(PdfPageMode::UnsetOrUnknown);
        }

        match page_mode as u32 {
            PAGEMODE_USENONE => Some(PdfPageMode::None),
            PAGEMODE_USEOUTLINES => Some(PdfPageMode::ShowDocumentOutline),
            PAGEMODE_USETHUMBS => Some(PdfPageMode::ShowPageThumbnails),
            PAGEMODE_FULLSCREEN => Some(PdfPageMode::Fullscreen),
            PAGEMODE_USEOC => Some(PdfPageMode::ShowContentGroupPanel),
            PAGEMODE_USEATTACHMENTS => Some(PdfPageMode::ShowAttachmentsPanel),
            _ => None,
        }
    }
}

/// The collection of [PdfPage] objects inside a [PdfDocument].
pub struct PdfPages<'a> {
    document_handle: FPDF_DOCUMENT,
    form_handle: Option<FPDF_FORMHANDLE>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPages<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        form_handle: Option<FPDF_FORMHANDLE>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPages {
            document_handle,
            form_handle,
            bindings,
        }
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPages] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the number of pages in this [PdfPages] collection.
    pub fn len(&self) -> PdfPageIndex {
        self.bindings.FPDF_GetPageCount(self.document_handle) as PdfPageIndex
    }

    /// Returns `true` if this [PdfPages] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of pages)` for this [PdfPages] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of pages - 1)` for this [PdfPages] collection.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfPageIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfPage] from this [PdfPages] collection.
    pub fn get(&self, index: PdfPageIndex) -> Result<PdfPage<'a>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageIndexOutOfBounds);
        }

        let page_handle = self.bindings.FPDF_LoadPage(self.document_handle, index as c_int);

        let result = self.pdfium_page_handle_to_result(index, page_handle);

        if let Ok(page) = result.as_ref() {
            PdfPageIndexCache::cache_props_for_page(
                self.document_handle,
                page_handle,
                index,
                page.content_regeneration_strategy(),
            );
        }

        result
    }

    /// Returns the size of a single [PdfPage] without loading it into memory.
    /// This is considerably faster than loading the page first via [PdfPages::get()] and then
    /// retrieving the page size using [PdfPage::page_size()].
    pub fn page_size(&self, index: PdfPageIndex) -> Result<PdfRect, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageIndexOutOfBounds);
        }

        let mut size = FS_SIZEF {
            width: 0.0,
            height: 0.0,
        };

        if self.bindings.is_true(
            self.bindings
                .FPDF_GetPageSizeByIndexF(self.document_handle, index, &mut size),
        ) {
            Ok(PdfRect::new(
                PdfPoints::ZERO,
                PdfPoints::ZERO,
                PdfPoints::new(size.height),
                PdfPoints::new(size.width),
            ))
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns the size of every [PdfPage] in this [PdfPages] collection.
    #[inline]
    pub fn page_sizes(&self) -> Result<Vec<PdfRect>, PdfiumError> {
        let mut sizes = Vec::with_capacity(self.len() as usize);

        for i in self.as_range() {
            sizes.push(self.page_size(i)?);
        }

        Ok(sizes)
    }

    /// Returns the first [PdfPage] in this [PdfPages] collection.
    #[inline]
    pub fn first(&self) -> Result<PdfPage<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(0)
        } else {
            Err(PdfiumError::NoPagesInDocument)
        }
    }

    /// Returns the last [PdfPage] in this [PdfPages] collection.
    #[inline]
    pub fn last(&self) -> Result<PdfPage<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(self.len() - 1)
        } else {
            Err(PdfiumError::NoPagesInDocument)
        }
    }

    /// Creates a new, empty [PdfPage] with the given [PdfPagePaperSize] and inserts it
    /// at the start of this [PdfPages] collection, shuffling down all other pages.
    #[inline]
    pub fn create_page_at_start(&mut self, size: PdfPagePaperSize) -> Result<PdfPage<'a>, PdfiumError> {
        self.create_page_at_index(size, 0)
    }

    /// Creates a new, empty [PdfPage] with the given [PdfPagePaperSize] and adds it
    /// to the end of this [PdfPages] collection.
    #[inline]
    pub fn create_page_at_end(&mut self, size: PdfPagePaperSize) -> Result<PdfPage<'a>, PdfiumError> {
        self.create_page_at_index(size, self.len())
    }

    /// Creates a new, empty [PdfPage] with the given [PdfPagePaperSize] and inserts it
    /// into this [PdfPages] collection at the given page index.
    pub fn create_page_at_index(
        &mut self,
        size: PdfPagePaperSize,
        index: PdfPageIndex,
    ) -> Result<PdfPage<'a>, PdfiumError> {
        let result = self.pdfium_page_handle_to_result(
            index,
            self.bindings.FPDFPage_New(
                self.document_handle,
                index as c_int,
                size.width().value as c_double,
                size.height().value as c_double,
            ),
        );

        if let Ok(page) = result.as_ref() {
            PdfPageIndexCache::insert_pages_at_index(self.document_handle, index, 1);
            PdfPageIndexCache::cache_props_for_page(
                self.document_handle,
                page.page_handle(),
                index,
                page.content_regeneration_strategy(),
            );
        }

        result
    }

    // TODO: AJRC - 5/2/23 - remove deprecated PdfPages::delete_page_range() function in 0.9.0
    // as part of tracking issue: https://github.com/ajrcarey/pdfium-render/issues/36
    // TODO: AJRC - 5/2/23 - if PdfDocument::pages() returned a &PdfPages reference (rather than an
    // owned PdfPages instance), and if PdfPages::get() returned a &PdfPage reference (rather than an
    // owned PdfPage instance), then it might be possible to reinstate this function, as Rust
    // would be able to manage the reference lifetimes safely. Tracking issue:
    // https://github.com/ajrcarey/pdfium-render/issues/47
    /// Deletes the page at the given index from this [PdfPages] collection.
    #[deprecated(
        since = "0.7.30",
        note = "This function has been deprecated. Use the PdfPage::delete() function instead."
    )]
    #[doc(hidden)]
    pub fn delete_page_at_index(&mut self, index: PdfPageIndex) -> Result<(), PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageIndexOutOfBounds);
        }

        self.bindings.FPDFPage_Delete(self.document_handle, index as c_int);

        PdfPageIndexCache::delete_pages_at_index(self.document_handle, index, 1);

        Ok(())
    }

    // TODO: AJRC - 5/2/23 - remove deprecated PdfPages::delete_page_range() function in 0.9.0
    // as part of tracking issue: https://github.com/ajrcarey/pdfium-render/issues/36
    // TODO: AJRC - 5/2/23 - if PdfDocument::pages() returned a &PdfPages reference (rather than an
    // owned PdfPages instance), and if PdfPages::get() returned a &PdfPage reference (rather than an
    // owned PdfPage instance), then it might be possible to reinstate this function, as Rust
    // would be able to manage the reference lifetimes safely. Tracking issue:
    // https://github.com/ajrcarey/pdfium-render/issues/47
    /// Deletes all pages in the given range from this [PdfPages] collection.
    #[deprecated(
        since = "0.7.30",
        note = "This function has been deprecated. Use the PdfPage::delete() function instead."
    )]
    #[doc(hidden)]
    pub fn delete_page_range(&mut self, range: Range<PdfPageIndex>) -> Result<(), PdfiumError> {
        for index in range.rev() {
            #[allow(deprecated)] // Both functions will be removed at the same time.
            self.delete_page_at_index(index)?;
        }

        Ok(())
    }

    /// Copies a single page with the given source page index from the given
    /// source [PdfDocument], inserting it at the given destination page index
    /// in this [PdfPages] collection.
    pub fn copy_page_from_document(
        &mut self,
        source: &PdfDocument,
        source_page_index: PdfPageIndex,
        destination_page_index: PdfPageIndex,
    ) -> Result<(), PdfiumError> {
        self.copy_page_range_from_document(source, source_page_index..=source_page_index, destination_page_index)
    }

    /// Copies one or more pages, specified using a user-friendly page range string,
    /// from the given source [PdfDocument], inserting the pages sequentially starting at the given
    /// destination page index in this [PdfPages] collection.
    ///
    /// The page range string should be in a comma-separated list of indexes and ranges,
    /// for example \"1,3,5-7\". Pages are indexed starting at one, not zero.
    #[inline]
    pub fn copy_pages_from_document(
        &mut self,
        source: &PdfDocument,
        pages: &str,
        destination_page_index: PdfPageIndex,
    ) -> Result<(), PdfiumError> {
        Self::copy_pages_between_documents(
            source.handle(),
            pages,
            self.document_handle,
            destination_page_index,
            self.bindings(),
        )
    }

    /// Copies one or more pages, specified using a user-friendly page range string,
    /// from one raw document handle to another, inserting the pages sequentially
    /// starting at the given destination page index.
    pub(crate) fn copy_pages_between_documents(
        source: FPDF_DOCUMENT,
        pages: &str,
        destination: FPDF_DOCUMENT,
        destination_page_index: PdfPageIndex,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<(), PdfiumError> {
        let destination_page_count_before_import = bindings.FPDF_GetPageCount(destination);

        if bindings.is_true(bindings.FPDF_ImportPages(destination, source, pages, destination_page_index as c_int)) {
            let destination_page_count_after_import = bindings.FPDF_GetPageCount(destination);

            PdfPageIndexCache::insert_pages_at_index(
                destination,
                destination_page_index,
                (destination_page_count_after_import - destination_page_count_before_import) as PdfPageIndex,
            );

            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Copies one or more pages with the given range of indices from the given
    /// source [PdfDocument], inserting the pages sequentially starting at the given
    /// destination page index in this [PdfPages] collection.
    #[inline]
    pub fn copy_page_range_from_document(
        &mut self,
        source: &PdfDocument,
        source_page_range: RangeInclusive<PdfPageIndex>,
        destination_page_index: PdfPageIndex,
    ) -> Result<(), PdfiumError> {
        Self::copy_page_range_between_documents(
            source.handle(),
            source_page_range,
            self.document_handle,
            destination_page_index,
            self.bindings(),
        )
    }

    /// Copies one or more pages with the given range of indices from one raw document handle
    /// to another, inserting the pages sequentially starting at the given destination page index.
    pub(crate) fn copy_page_range_between_documents(
        source: FPDF_DOCUMENT,
        source_page_range: RangeInclusive<PdfPageIndex>,
        destination: FPDF_DOCUMENT,
        destination_page_index: PdfPageIndex,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<(), PdfiumError> {
        let no_of_pages_to_import = (source_page_range.end() - source_page_range.start() + 1) as PdfPageIndex;

        if bindings.is_true(bindings.FPDF_ImportPagesByIndex_vec(
            destination,
            source,
            source_page_range.collect::<Vec<_>>(),
            destination_page_index,
        )) {
            PdfPageIndexCache::insert_pages_at_index(destination, destination_page_index, no_of_pages_to_import);

            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Copies all pages in the given source [PdfDocument], appending them sequentially
    /// to the end of this [PdfPages] collection.
    ///
    /// For finer control over which pages are imported, and where they should be inserted,
    /// use one of the [PdfPages::copy_page_from_document()], [PdfPages::copy_pages_from_document()],
    ///  or [PdfPages::copy_page_range_from_document()] functions.
    #[inline]
    pub fn append(&mut self, document: &PdfDocument) -> Result<(), PdfiumError> {
        self.copy_page_range_from_document(document, document.pages().as_range_inclusive(), self.len())
    }

    /// Creates a new [PdfDocument] by copying the pages in this [PdfPages] collection
    /// into tiled grids, the size of each tile shrinking or expanding as necessary to fit
    /// the given [PdfPagePaperSize].
    ///
    /// For example, to output all pages in a [PdfPages] collection into a new
    /// A3 landscape document with six source pages tiled on each destination page arranged
    /// into a 2 row x 3 column grid, you would call:
    ///
    /// ```
    /// PdfPages::tile_into_new_document(2, 3, PdfPagePaperSize::a3().to_landscape())
    /// ```
    pub fn tile_into_new_document(
        &self,
        rows_per_page: u8,
        columns_per_row: u8,
        size: PdfPagePaperSize,
    ) -> Result<PdfDocument<'_>, PdfiumError> {
        let handle = self.bindings.FPDF_ImportNPagesToOne(
            self.document_handle,
            size.width().value,
            size.height().value,
            columns_per_row as size_t,
            rows_per_page as size_t,
        );

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfDocument::from_pdfium(handle, self.bindings))
        }
    }

    /// Returns a [PdfPage] from the given `FPDF_PAGE` handle, if possible.
    pub(crate) fn pdfium_page_handle_to_result(
        &self,
        index: PdfPageIndex,
        page_handle: FPDF_PAGE,
    ) -> Result<PdfPage<'a>, PdfiumError> {
        if page_handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            // The page's label (if any) is retrieved by index rather than by using the
            // FPDF_PAGE handle. Since the index of any particular page can change
            // (if other pages are inserted or removed), it's better if we don't treat the
            // page index as an immutable property of the PdfPage; instead, we look up the label now.

            // (Pdfium does not currently include an FPDF_SetPageLabel() function, so the label
            // _will_ be an immutable property of the PdfPage for its entire lifetime.)

            let label = {
                // Retrieving the label text from Pdfium is a two-step operation. First, we call
                // FPDF_GetPageLabel() with a null buffer; this will retrieve the length of
                // the label text in bytes. If the length is zero, then there is no such tag.

                // If the length is non-zero, then we reserve a byte buffer of the given
                // length and call FPDF_GetPageLabel() again with a pointer to the buffer;
                // this will write the label text to the buffer in UTF16LE format.

                let buffer_length =
                    self.bindings
                        .FPDF_GetPageLabel(self.document_handle, index as c_int, std::ptr::null_mut(), 0);

                if buffer_length == 0 {
                    // The label is not present.

                    None
                } else {
                    let mut buffer = create_byte_buffer(buffer_length as usize);

                    let result = self.bindings.FPDF_GetPageLabel(
                        self.document_handle,
                        index as c_int,
                        buffer.as_mut_ptr() as *mut c_void,
                        buffer_length,
                    );

                    debug_assert_eq!(result, buffer_length);

                    get_string_from_pdfium_utf16le_bytes(buffer)
                }
            };

            Ok(PdfPage::from_pdfium(
                self.document_handle,
                page_handle,
                self.form_handle,
                label,
                self.bindings,
            ))
        }
    }

    /// Returns the [PdfPageMode] setting embedded in the containing [PdfDocument].
    pub fn page_mode(&self) -> PdfPageMode {
        PdfPageMode::from_pdfium(self.bindings.FPDFDoc_GetPageMode(self.document_handle))
            .unwrap_or(PdfPageMode::UnsetOrUnknown)
    }

    /// Applies the given watermarking closure to each [PdfPage] in this [PdfPages] collection.
    ///
    /// The closure receives four arguments:
    /// * An empty [PdfPageGroupObject] for you to populate with the page objects that make up your watermark.
    /// * The zero-based index of the [PdfPage] currently being processed.
    /// * The width of the [PdfPage] currently being processed, in [PdfPoints].
    /// * The height of the [PdfPage] currently being processed, in [PdfPoints].
    ///
    /// If the current page should not be watermarked, simply leave the group empty.
    ///
    /// The closure can return a `Result<(), PdfiumError>`; this makes it easy to use the `?` unwrapping
    /// operator within the closure.
    ///
    /// For example, the following snippet adds a page number to the very top of every page in a document
    /// except for the first page.
    ///
    /// ```
    ///     document.pages().watermark(|group, index, width, height| {
    ///         if index == 0 {
    ///             // Don't watermark the first page.
    ///
    ///             Ok(())
    ///         } else {
    ///             let mut page_number = PdfPageTextObject::new(
    ///                 &document,
    ///                 format!("Page {}", index + 1),
    ///                 &PdfFont::helvetica(&document),
    ///                 PdfPoints::new(14.0),
    ///             )?;
    ///
    ///             page_number.translate(
    ///                 (width - page_number.width()?) / 2.0, // Horizontally center the page number...
    ///                 height - page_number.height()?, // ... and vertically position it at the page top.
    ///             )?;
    ///
    ///             group.push(&mut page_number.into())
    ///         }
    ///     })?;
    /// ```
    pub fn watermark<F>(&self, watermarker: F) -> Result<(), PdfiumError>
    where
        F: Fn(&mut PdfPageGroupObject<'a>, PdfPageIndex, PdfPoints, PdfPoints) -> Result<(), PdfiumError>,
    {
        for (index, page) in self.iter().enumerate() {
            let mut group = PdfPageGroupObject::from_pdfium(self.document_handle, page.page_handle(), self.bindings);

            watermarker(&mut group, index as PdfPageIndex, page.width(), page.height())?;
        }

        Ok(())
    }

    /// Returns an iterator over all the pages in this [PdfPages] collection.
    #[inline]
    pub fn iter(&self) -> PdfPagesIterator<'_> {
        PdfPagesIterator::new(self)
    }
}

/// An iterator over all the [PdfPage] objects in a [PdfPages] collection.
pub struct PdfPagesIterator<'a> {
    pages: &'a PdfPages<'a>,
    next_index: PdfPageIndex,
}

impl<'a> PdfPagesIterator<'a> {
    #[inline]
    pub(crate) fn new(pages: &'a PdfPages<'a>) -> Self {
        PdfPagesIterator { pages, next_index: 0 }
    }
}

impl<'a> Iterator for PdfPagesIterator<'a> {
    type Item = PdfPage<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.pages.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};

    #[test]
    fn test_page_size() -> Result<(), PdfiumError> {
        // Tests the dimensions of each page in a sample file.

        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file(&test_fixture_path("page-sizes-test.pdf"), None)?;

        assert_eq!(document.pages().page_size(0)?, expected_page_0_size());
        assert_eq!(document.pages().page_size(1)?, expected_page_1_size());
        assert_eq!(document.pages().page_size(2)?, expected_page_2_size());
        assert_eq!(document.pages().page_size(3)?, expected_page_3_size());
        assert_eq!(document.pages().page_size(4)?, expected_page_4_size());
        assert!(document.pages().page_size(5).is_err());

        Ok(())
    }

    #[test]
    fn test_page_sizes() -> Result<(), PdfiumError> {
        // Tests the dimensions of all pages in a sample file.

        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file(&test_fixture_path("page-sizes-test.pdf"), None)?;

        assert_eq!(
            document.pages().page_sizes()?,
            vec!(
                expected_page_0_size(),
                expected_page_1_size(),
                expected_page_2_size(),
                expected_page_3_size(),
                expected_page_4_size(),
            ),
        );

        Ok(())
    }

    const fn expected_page_0_size() -> PdfRect {
        PdfRect::new_from_values(0.0, 0.0, 841.8898, 595.30396)
    }

    const fn expected_page_1_size() -> PdfRect {
        PdfRect::new_from_values(0.0, 0.0, 595.30396, 841.8898)
    }

    const fn expected_page_2_size() -> PdfRect {
        PdfRect::new_from_values(0.0, 0.0, 1190.5511, 841.8898)
    }

    const fn expected_page_3_size() -> PdfRect {
        PdfRect::new_from_values(0.0, 0.0, 419.5559, 595.30396)
    }

    const fn expected_page_4_size() -> PdfRect {
        expected_page_0_size()
    }

    #[test]
    fn copy_page_range_from_document() -> Result<(), PdfiumError> {
        // Tests that copy_page_range_from_document() copies the expected
        // number of pages.

        let pdfium = test_bind_to_pdfium();

        let max_page_count = 200;

        for i in 0..(max_page_count / 2) {
            let mut source = pdfium.create_new_pdf()?;

            for _ in 0..max_page_count {
                source.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
            }

            let mut destination = pdfium.create_new_pdf()?;

            for _ in 0..i {
                destination.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
            }

            let destination_page_index = destination.pages().len() / 2;

            let source_from_page_index = source.pages().len() / 2 - i;
            let source_to_page_index = source.pages().len() / 2 + i;
            let source_page_range_len = source_to_page_index - source_from_page_index + 1; // Page ranges are inclusive

            destination.pages_mut().copy_page_range_from_document(
                &source,
                source_from_page_index..=source_to_page_index,
                destination_page_index,
            )?;

            assert_eq!(destination.pages().len(), i + source_page_range_len);
        }

        Ok(())
    }
}
