//! Defines the [PdfPageShadingObject] struct, exposing functionality related to a single
//! page object of type [PdfPageObjectType::Shading].

use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGEOBJECT};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectOwnership};
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::{create_transform_getters, create_transform_setters};

#[cfg(doc)]
use {crate::pdf::document::page::PdfPage, crate::pdf::document::page::object::PdfPageObjectType};

/// A single [PdfPageObject] of type [PdfPageObjectType::Shading].
pub struct PdfPageShadingObject<'a> {
    object_handle: FPDF_PAGEOBJECT,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageShadingObject<'a> {
    pub(crate) fn from_pdfium(
        object_handle: FPDF_PAGEOBJECT,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageShadingObject {
            object_handle,
            ownership,
            bindings,
        }
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "this [PdfPageShadingObject]",
        "this [PdfPageShadingObject].",
        "this [PdfPageShadingObject],"
    );

    // The transform_impl() function required by the create_transform_setters!() macro
    // is provided by the PdfPageObjectPrivate trait.

    create_transform_getters!(
        "this [PdfPageShadingObject]",
        "this [PdfPageShadingObject].",
        "this [PdfPageShadingObject],"
    );

    // The get_matrix_impl() function required by the create_transform_getters!() macro
    // is provided by the PdfPageObjectPrivate trait.
}

impl<'a> PdfPageObjectPrivate<'a> for PdfPageShadingObject<'a> {
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
        Err(PdfiumError::UnsupportedPdfPageObjectType)
    }
}

impl<'a> Drop for PdfPageShadingObject<'a> {
    /// Closes this [PdfPageShadingObject], releasing held memory.
    fn drop(&mut self) {
        self.drop_impl();
    }
}
