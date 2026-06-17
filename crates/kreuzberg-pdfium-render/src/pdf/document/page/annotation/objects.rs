//! Defines the [PdfPageAnnotationObjects] struct, exposing functionality related to the
//! page objects contained within a single `PdfPageAnnotation`.

use crate::bindgen::{FPDF_ANNOTATION, FPDF_DOCUMENT, FPDF_PAGE};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::page::object::PdfPageObject;
use crate::pdf::document::page::object::ownership::PdfPageObjectOwnership;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::objects::common::{PdfPageObjectIndex, PdfPageObjectsCommon, PdfPageObjectsIterator};
use crate::pdf::document::page::objects::private::internal::PdfPageObjectsPrivate;
use std::os::raw::c_int;

/// The page objects contained within a single `PdfPageAnnotation`.
///
/// Content in an annotation is structured as a stream of [PdfPageObject] objects of different types:
/// text objects, image objects, path objects, and so on.
///
/// Note that Pdfium does not support or recognize all PDF page object types. For instance,
/// Pdfium does not currently support or recognize the External Object ("XObject") page object type
/// supported by Adobe Acrobat and Foxit's commercial PDF SDK. In these cases, Pdfium will return
/// `PdfPageObjectType::Unsupported`.
///
/// Page objects can be retrieved from any type of `PdfPageAnnotation`, but Pdfium currently
/// only permits adding new page objects to, or removing existing page objects from, annotations
/// of types `PdfPageAnnotationType::Ink` and `PdfPageAnnotationType::Stamp`. All other annotation
/// types are read-only.
pub struct PdfPageAnnotationObjects<'a> {
    annotation_handle: FPDF_ANNOTATION,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageAnnotationObjects<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        annotation_handle: FPDF_ANNOTATION,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        Self {
            annotation_handle,
            ownership: PdfPageObjectOwnership::owned_by_attached_annotation(
                document_handle,
                page_handle,
                annotation_handle,
            ),
            bindings,
        }
    }

    /// Returns the internal `FPDF_ANNOTATION` handle for the [PdfPageAnnotation] containing
    /// this [PdfPageAnnotationObjects] collection.
    #[inline]
    pub(crate) fn annotation_handle(&self) -> FPDF_ANNOTATION {
        self.annotation_handle
    }
}

impl<'a> PdfPageObjectsPrivate<'a> for PdfPageAnnotationObjects<'a> {
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
        self.bindings().FPDFAnnot_GetObjectCount(self.annotation_handle()) as PdfPageObjectIndex
    }

    fn get_impl(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object_handle = self
            .bindings()
            .FPDFAnnot_GetObject(self.annotation_handle(), index as c_int);

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
                self.bindings,
            ))
        }
    }

    #[inline]
    fn iter_impl(&'a self) -> PdfPageObjectsIterator<'a> {
        PdfPageObjectsIterator::new(self)
    }

    #[inline]
    fn add_object_impl(&mut self, mut object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        object.add_object_to_annotation(self).map(|_| object)
    }

    #[inline]
    fn remove_object_impl(&mut self, mut object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        object.remove_object_from_annotation().map(|_| object)
    }
}
