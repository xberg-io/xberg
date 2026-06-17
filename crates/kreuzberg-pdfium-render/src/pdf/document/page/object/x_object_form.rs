//! Defines the [PdfPageXObjectFormObject] struct, exposing functionality related to a single
//! page object of type `PdfPageObjectType::XObjectForm`.

use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGEOBJECT};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectOwnership};
use crate::pdf::document::page::objects::common::{PdfPageObjectIndex, PdfPageObjectsIterator};
use crate::pdf::document::page::objects::private::internal::PdfPageObjectsPrivate;
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::{create_transform_getters, create_transform_setters};
use std::ops::{Range, RangeInclusive};
use std::os::raw::c_ulong;

#[cfg(doc)]
use {
    crate::pdf::document::page::object::PdfPageObjectType,
    crate::pdf::document::page::object::group::PdfPageGroupObject, crate::pdf::document::page::objects::PdfPageObjects,
};

/// A single [PdfPageObject] of type [PdfPageObjectType::XObjectForm]. The page object contains a
/// content stream that itself may consist of multiple other page objects. When this page object
/// is rendered, it renders all its constituent page objects, effectively serving as a template or
/// stamping object.
///
/// Despite the page object name including "form", this page object type bears no relation
/// to an interactive form containing form fields.
///
/// New [PdfPageObjectType::XObjectForm] objects can be created by calling either the
/// [PdfPageObjects::copy_into_x_object_form_object()] function or the
/// [PdfPageGroupObject::copy_into_x_object_form_object()] function.
pub struct PdfPageXObjectFormObject<'a> {
    object_handle: FPDF_PAGEOBJECT,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageXObjectFormObject<'a> {
    pub(crate) fn from_pdfium(
        object_handle: FPDF_PAGEOBJECT,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageXObjectFormObject {
            object_handle,
            ownership,
            bindings,
        }
    }

    /// Returns the total number of child page objects in this [PdfPageXObjectFormObject].
    #[inline]
    pub fn len(&self) -> PdfPageObjectIndex {
        self.len_impl()
    }

    /// Returns `true` if this page objects collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of objects)` for the child page objects in
    /// this [PdfPageXObjectFormObject].
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageObjectIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of objects - 1)` for the child page objects
    /// in this [PdfPageXObjectFormObject].
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfPageObjectIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single child [PdfPageObject] from this [PdfPageXObjectFormObject].
    #[inline]
    pub fn get(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.get_impl(index)
    }

    /// Returns the first child [PdfPageObject] in this [PdfPageXObjectFormObject].
    #[inline]
    pub fn first(&self) -> Result<PdfPageObject<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(0)
        } else {
            Err(PdfiumError::NoPageObjectsInCollection)
        }
    }

    /// Returns the last child [PdfPageObject] in this [PdfPageXObjectFormObject].
    #[inline]
    pub fn last(&self) -> Result<PdfPageObject<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(self.len() - 1)
        } else {
            Err(PdfiumError::NoPageObjectsInCollection)
        }
    }

    /// Returns an iterator over all the child [PdfPageObject] objects in this [PdfPageXObjectFormObject].
    #[inline]
    pub fn iter(&'a self) -> PdfPageObjectsIterator<'a> {
        self.iter_impl()
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "this [PdfPageXObjectFormObject]",
        "this [PdfPageXObjectFormObject].",
        "this [PdfPageXObjectFormObject],"
    );

    // The transform_impl() function required by the create_transform_setters!() macro
    // is provided by the PdfPageObjectPrivate trait.

    create_transform_getters!(
        "this [PdfPageXObjectFormObject]",
        "this [PdfPageXObjectFormObject].",
        "this [PdfPageXObjectFormObject],"
    );

    // The get_matrix_impl() function required by the create_transform_getters!() macro
    // is provided by the PdfPageObjectPrivate trait.
}

impl<'a> PdfPageObjectPrivate<'a> for PdfPageXObjectFormObject<'a> {
    #[inline]
    fn object_handle(&self) -> FPDF_PAGEOBJECT {
        self.object_handle
    }

    #[inline]
    fn ownership(&self) -> &PdfPageObjectOwnership {
        &self.ownership
    }

    #[inline]
    fn set_ownership(&mut self, ownership: PdfPageObjectOwnership) {
        self.ownership = ownership;
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }

    #[inline]
    fn is_copyable_impl(&self) -> bool {
        false
    }

    #[inline]
    fn try_copy_impl<'b>(
        &self,
        _: FPDF_DOCUMENT,
        _: &'b dyn PdfiumLibraryBindings,
    ) -> Result<PdfPageObject<'b>, PdfiumError> {
        Err(PdfiumError::PageObjectNotCopyable)
    }
}

impl<'a> PdfPageObjectsPrivate<'a> for PdfPageXObjectFormObject<'a> {
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
        self.bindings.FPDFFormObj_CountObjects(self.object_handle) as PdfPageObjectIndex
    }

    fn get_impl(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object_handle = self
            .bindings
            .FPDFFormObj_GetObject(self.object_handle, index as c_ulong);

        if object_handle.is_null() {
            if index >= self.len() {
                Err(PdfiumError::PageObjectIndexOutOfBounds)
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        } else {
            Ok(PdfPageObject::from_pdfium(
                object_handle,
                *PdfPageObjectPrivate::ownership(self),
                PdfPageObjectsPrivate::bindings(self),
            ))
        }
    }

    #[inline]
    fn iter_impl(&'a self) -> PdfPageObjectsIterator<'a> {
        PdfPageObjectsIterator::new(self)
    }

    // The child objects collection is read-only.

    fn add_object_impl(&mut self, _object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        Err(PdfiumError::PageObjectsCollectionIsImmutable)
    }

    fn remove_object_impl(&mut self, _object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        Err(PdfiumError::PageObjectsCollectionIsImmutable)
    }
}

impl<'a> Drop for PdfPageXObjectFormObject<'a> {
    /// Closes this [PdfPageXObjectFormObject], releasing held memory.
    fn drop(&mut self) {
        self.drop_impl();
    }
}
