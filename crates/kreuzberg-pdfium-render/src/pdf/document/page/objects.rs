//! Defines the [PdfPageObjects] struct, exposing functionality related to the
//! page objects contained within a single [PdfPage].

pub mod common;
pub(crate) mod private; // Keep private so that the PdfPageObjectsPrivate trait is not exposed.

use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGE};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::PdfPageIndexCache;
use crate::pdf::document::page::object::PdfPageObject;
use crate::pdf::document::page::object::group::PdfPageGroupObject;
use crate::pdf::document::page::object::ownership::PdfPageObjectOwnership;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::x_object_form::PdfPageXObjectFormObject;
use crate::pdf::document::page::objects::common::{PdfPageObjectIndex, PdfPageObjectsCommon, PdfPageObjectsIterator};
use crate::pdf::document::page::objects::private::internal::PdfPageObjectsPrivate;
use std::os::raw::c_int;

#[cfg(doc)]
use {
    crate::pdf::document::page::PdfPage, crate::pdf::document::page::PdfPageContentRegenerationStrategy,
    crate::pdf::document::page::object::PdfPageObjectType,
};

/// The page objects contained within a single [PdfPage].
///
/// Content on a page is structured as a stream of [PdfPageObject] objects of different types:
/// text objects, image objects, path objects, and so on.
///
/// Note that Pdfium does not support or recognize all PDF page object types. For instance,
/// Pdfium does not currently support or recognize the External Object ("XObject") page object type
/// supported by Adobe Acrobat and Foxit's commercial PDF SDK. In these cases, Pdfium will return
/// [PdfPageObjectType::Unsupported].
pub struct PdfPageObjects<'a> {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageObjects<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        Self {
            document_handle,
            page_handle,
            ownership: PdfPageObjectOwnership::owned_by_page(document_handle, page_handle),
            bindings,
        }
    }

    /// Returns the internal `FPDF_DOCUMENT` handle for this page objects collection.
    #[inline]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the internal `FPDF_PAGE` handle for this page objects collection.
    #[inline]
    pub(crate) fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    /// Creates a new [PdfPageGroupObject] object group that includes any page objects in this
    /// [PdfPageObjects] collection matching the given predicate function.
    pub fn create_group<F>(&'a self, predicate: F) -> Result<PdfPageGroupObject<'a>, PdfiumError>
    where
        F: Fn(&PdfPageObject) -> bool,
    {
        let mut result = self.create_empty_group();

        for mut object in self.iter().filter(predicate) {
            result.push(&mut object)?;
        }

        Ok(result)
    }

    /// Creates a new [PdfPageGroupObject] object group that can accept any [PdfPageObject]
    /// in this [PdfPageObjects] collection. The newly created group will be empty;
    /// you will need to manually add to it the objects you want to manipulate.
    #[inline]
    pub fn create_empty_group(&self) -> PdfPageGroupObject<'a> {
        PdfPageGroupObject::from_pdfium(self.document_handle(), self.page_handle(), self.bindings())
    }

    /// Creates a new [PdfPageXObjectFormObject] object from the page objects on this [PdfPage],
    /// ready to use in the given destination [PdfDocument].
    pub fn copy_into_x_object_form_object(
        &self,
        destination: &mut PdfDocument<'a>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let page_index = PdfPageIndexCache::get_index_for_page(self.document_handle(), self.page_handle());

        match page_index {
            Some(page_index) => {
                let x_object = self.bindings().FPDF_NewXObjectFromPage(
                    destination.handle(),
                    self.document_handle(),
                    page_index as c_int,
                );

                let object_handle = self.bindings().FPDF_NewFormObjectFromXObject(x_object);
                if object_handle.is_null() {
                    return Err(PdfiumError::PdfiumLibraryInternalError(
                        crate::error::PdfiumInternalError::Unknown,
                    ));
                }

                let object = PdfPageXObjectFormObject::from_pdfium(
                    object_handle,
                    PdfPageObjectOwnership::owned_by_document(destination.handle()),
                    self.bindings(),
                );

                self.bindings().FPDF_CloseXObject(x_object);

                Ok(PdfPageObject::XObjectForm(object))
            }
            None => Err(PdfiumError::SourcePageIndexNotInCache),
        }
    }
}

impl<'a> PdfPageObjectsPrivate<'a> for PdfPageObjects<'a> {
    #[inline]
    fn ownership(&self) -> &PdfPageObjectOwnership {
        &self.ownership
    }

    #[inline]
    fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    #[inline]
    fn len_impl(&self) -> PdfPageObjectIndex {
        self.bindings.FPDFPage_CountObjects(self.page_handle) as PdfPageObjectIndex
    }

    fn get_impl(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object_handle = self.bindings.FPDFPage_GetObject(self.page_handle, index as c_int);

        if object_handle.is_null() {
            if index >= self.len() {
                Err(PdfiumError::PageObjectIndexOutOfBounds)
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        } else {
            Ok(PdfPageObject::from_pdfium(
                object_handle,
                *self.ownership(),
                self.bindings(),
            ))
        }
    }

    #[inline]
    fn iter_impl(&'a self) -> PdfPageObjectsIterator<'a> {
        PdfPageObjectsIterator::new(self)
    }

    #[inline]
    fn add_object_impl(&mut self, mut object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        object.add_object_to_page(self).map(|_| object)
    }

    #[inline]
    fn remove_object_impl(&mut self, mut object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        object.remove_object_from_page().map(|_| object)
    }
}
