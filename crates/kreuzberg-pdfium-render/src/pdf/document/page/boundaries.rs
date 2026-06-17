//! Defines the [PdfPageBoundaries] struct, exposing functionality related to the
//! boundary boxes of a single [PdfPage].

use crate::bindgen::{FPDF_BOOL, FPDF_PAGE, FS_RECTF};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::rect::PdfRect;
use std::os::raw::c_float;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// The box type of a single boundary box in a [PdfPage].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPageBoundaryBoxType {
    Media,
    Art,
    Bleed,
    Trim,
    Crop,
    Bounding,
}

/// The type and bounds of a single boundary box in a [PdfPage].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PdfPageBoundaryBox {
    pub box_type: PdfPageBoundaryBoxType,
    pub bounds: PdfRect,
}

impl PdfPageBoundaryBox {
    #[inline]
    pub(crate) fn new(boundary: PdfPageBoundaryBoxType, bounds: PdfRect) -> Self {
        Self {
            box_type: boundary,
            bounds,
        }
    }
}

/// The page boundaries of a single [PdfPage].
///
/// The content of a page can be bounded by up to six different boxes:
///
/// * Media box: the full page size, equivalent to the target paper size when the document is printed.
///   All other page boundaries must fit inside the Media box.
/// * Art box: the maximum extent of out-of-bleed page art when offset printing.
///   Typically cropped out when viewing the document on-screen.
/// * Bleed box: the maximum extent of outside-trim page bleeds when offset printing.
///   Typically cropped out when viewing the document on-screen.
/// * Trim box: the maximum extent of page trims when offset printing.
///   Typically cropped out when viewing the document on-screen.
/// * Crop box: the maximum extent of user-visible content when viewing the document on-screen.
/// * Bounding box ("BBox"): the smallest rectangle that can enclose all the content contained in the page.
///
/// These boundaries are concentric, i.e. the Bounding box must fit within the Crop box,
/// which must fit within the Trim box, and so on. The Media box therefore contains all other boxes.
/// Not all boxes are guaranteed to exist for all pages.
///
/// For more information, see section 10.10.1 on page 962 of the PDF Reference Manual version 1.7,
/// or visit: <https://www.pdfscripting.com/public/PDF-Page-Coordinates.cfm#UserSpace>
pub struct PdfPageBoundaries<'a> {
    page_handle: FPDF_PAGE,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageBoundaries<'a> {
    #[inline]
    pub(crate) fn from_pdfium(page_handle: FPDF_PAGE, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self { page_handle, bindings }
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageBoundaries] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the boundary box defined for the containing [PdfPage] matching the
    /// given [PdfPageBoundaryBoxType], if any.
    #[inline]
    pub fn get(&self, boundary: PdfPageBoundaryBoxType) -> Result<PdfPageBoundaryBox, PdfiumError> {
        match boundary {
            PdfPageBoundaryBoxType::Media => self.media(),
            PdfPageBoundaryBoxType::Art => self.art(),
            PdfPageBoundaryBoxType::Bleed => self.bleed(),
            PdfPageBoundaryBoxType::Trim => self.trim(),
            PdfPageBoundaryBoxType::Crop => self.crop(),
            PdfPageBoundaryBoxType::Bounding => self.bounding(),
        }
    }

    /// Sets the boundary box matching the given [PdfPageBoundaryBoxType] to the given [PdfRect]
    /// for the containing [PdfPage].
    #[inline]
    pub fn set(&mut self, box_type: PdfPageBoundaryBoxType, rect: PdfRect) -> Result<(), PdfiumError> {
        match box_type {
            PdfPageBoundaryBoxType::Media => self.set_media(rect),
            PdfPageBoundaryBoxType::Art => self.set_art(rect),
            PdfPageBoundaryBoxType::Bleed => self.set_bleed(rect),
            PdfPageBoundaryBoxType::Trim => self.set_trim(rect),
            PdfPageBoundaryBoxType::Crop => self.set_crop(rect),
            PdfPageBoundaryBoxType::Bounding => Ok(()), // The bounding box is implicit and cannot be set directly.
        }
    }

    /// Returns the Media boundary box defined for the containing [PdfPage], if any.
    /// The Media box is the full page size, equivalent to the target paper size when the document
    /// is printed.
    #[inline]
    pub fn media(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        self.get_bounding_box_rect(|page, left, bottom, right, top| {
            self.bindings.FPDFPage_GetMediaBox(page, left, bottom, right, top)
        })
        .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Media, rect))
    }

    /// Sets the Media boundary box for the containing [PdfPage] to the given [PdfRect].
    pub fn set_media(&mut self, rect: PdfRect) -> Result<(), PdfiumError> {
        self.bindings.FPDFPage_SetMediaBox(
            self.page_handle,
            rect.left().value,
            rect.bottom().value,
            rect.right().value,
            rect.top().value,
        );

        Ok(())
    }

    /// Returns the Art boundary box defined for the containing [PdfPage], if any.
    /// The Art box is the maximum extent of out-of-bleed page art when offset printing.
    /// It is typically cropped out when viewing the document on-screen.
    #[inline]
    pub fn art(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        self.get_bounding_box_rect(|page, left, bottom, right, top| {
            self.bindings.FPDFPage_GetArtBox(page, left, bottom, right, top)
        })
        .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Art, rect))
    }

    /// Sets the Art boundary box for the containing [PdfPage] to the given [PdfRect].
    pub fn set_art(&mut self, rect: PdfRect) -> Result<(), PdfiumError> {
        self.bindings.FPDFPage_SetArtBox(
            self.page_handle,
            rect.left().value,
            rect.bottom().value,
            rect.right().value,
            rect.top().value,
        );

        Ok(())
    }

    /// Returns the Bleed boundary box defined for the containing [PdfPage], if any.
    /// The Bleed box is the maximum extent of outside-trim page bleeds when offset printing.
    /// It is typically cropped out when viewing the document on-screen.
    #[inline]
    pub fn bleed(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        self.get_bounding_box_rect(|page, left, bottom, right, top| {
            self.bindings.FPDFPage_GetBleedBox(page, left, bottom, right, top)
        })
        .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Bleed, rect))
    }

    /// Sets the Bleed boundary box for the containing [PdfPage] to the given [PdfRect].
    pub fn set_bleed(&mut self, rect: PdfRect) -> Result<(), PdfiumError> {
        self.bindings.FPDFPage_SetBleedBox(
            self.page_handle,
            rect.left().value,
            rect.bottom().value,
            rect.right().value,
            rect.top().value,
        );

        Ok(())
    }

    /// Returns the Trim boundary box defined for the containing [PdfPage], if any.
    /// The Trim box is the maximum extent of page trims when offset printing.
    /// It is typically cropped out when viewing the document on-screen.
    #[inline]
    pub fn trim(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        self.get_bounding_box_rect(|page, left, bottom, right, top| {
            self.bindings.FPDFPage_GetTrimBox(page, left, bottom, right, top)
        })
        .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Trim, rect))
    }

    /// Sets the Trim boundary box for the containing [PdfPage] to the given [PdfRect].
    pub fn set_trim(&mut self, rect: PdfRect) -> Result<(), PdfiumError> {
        self.bindings.FPDFPage_SetTrimBox(
            self.page_handle,
            rect.left().value,
            rect.bottom().value,
            rect.right().value,
            rect.top().value,
        );

        Ok(())
    }

    /// Returns the Crop boundary box defined for the containing [PdfPage], if any.
    /// The Crop box is the maximum extent of user-visible content when viewing the document on-screen.
    #[inline]
    pub fn crop(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        self.get_bounding_box_rect(|page, left, bottom, right, top| {
            self.bindings.FPDFPage_GetCropBox(page, left, bottom, right, top)
        })
        .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Crop, rect))
    }

    /// Sets the Crop boundary box for the containing [PdfPage] to the given [PdfRect].
    pub fn set_crop(&mut self, rect: PdfRect) -> Result<(), PdfiumError> {
        self.bindings.FPDFPage_SetCropBox(
            self.page_handle,
            rect.left().value,
            rect.bottom().value,
            rect.right().value,
            rect.top().value,
        );

        Ok(())
    }

    /// Returns the Bounding box ("BBox") defined for the containing [PdfPage], if any.
    /// The BBox is the smallest rectangle that can enclose all the content contained in the page.
    /// Unlike other boundary boxes, the BBox is computed dynamically on request and cannot
    /// be set explicitly.
    #[inline]
    pub fn bounding(&self) -> Result<PdfPageBoundaryBox, PdfiumError> {
        let mut rect = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };

        let result = self.bindings.FPDF_GetPageBoundingBox(self.page_handle, &mut rect);

        PdfRect::from_pdfium_as_result(result, rect, self.bindings)
            .map(|rect| PdfPageBoundaryBox::new(PdfPageBoundaryBoxType::Bounding, rect))
    }

    /// Returns the [PdfRect] obtained from calling the given `FPDF_*Box()` function.
    #[inline]
    fn get_bounding_box_rect<F>(&self, f: F) -> Result<PdfRect, PdfiumError>
    where
        F: FnOnce(FPDF_PAGE, *mut c_float, *mut c_float, *mut c_float, *mut c_float) -> FPDF_BOOL,
    {
        let mut left = 0_f32;
        let mut bottom = 0_f32;
        let mut right = 0_f32;
        let mut top = 0_f32;

        let result = f(self.page_handle, &mut left, &mut bottom, &mut right, &mut top);

        PdfRect::from_pdfium_as_result(
            result,
            FS_RECTF {
                left,
                top,
                right,
                bottom,
            },
            self.bindings,
        )
    }

    /// Returns an iterator over all defined [PdfPageBoundaryBox] boxes in the containing `PdfPage`.
    /// Not all boxes are guaranteed to exist for all pages, but where they are defined they will
    /// be returned strictly in enclosing order from outermost to innermost:
    /// Media, Art, Bleed, Trim, Crop, Bounding.
    pub fn iter(&'a self) -> PageBoundaryIterator<'a> {
        PageBoundaryIterator::new(self)
    }
}

/// An iterator over all the [PdfPageBoundaryBox] objects defined for a [PdfPage].
/// Not all boxes are guaranteed to exist for all pages, but where they are defined they will
/// be returned strictly in enclosing order from outermost to innermost:
/// Media, Art, Bleed, Trim, Crop, Bounding.
pub struct PageBoundaryIterator<'a> {
    boundaries: &'a PdfPageBoundaries<'a>,
    next_index: usize,
}

impl<'a> PageBoundaryIterator<'a> {
    #[inline]
    pub(crate) fn new(boundaries: &'a PdfPageBoundaries<'a>) -> Self {
        Self {
            boundaries,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PageBoundaryIterator<'a> {
    type Item = PdfPageBoundaryBox;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next = None;

        while self.next_index < 5 && next.is_none() {
            next = match self.next_index {
                0 => self.boundaries.get(PdfPageBoundaryBoxType::Media).ok(),
                1 => self.boundaries.get(PdfPageBoundaryBoxType::Art).ok(),
                2 => self.boundaries.get(PdfPageBoundaryBoxType::Bleed).ok(),
                3 => self.boundaries.get(PdfPageBoundaryBoxType::Trim).ok(),
                4 => self.boundaries.get(PdfPageBoundaryBoxType::Crop).ok(),
                5 => self.boundaries.get(PdfPageBoundaryBoxType::Bounding).ok(),
                _ => None,
            };

            self.next_index += 1;
        }

        next
    }
}
