//! Defines the [PdfActionRemoteDestination] struct, exposing functionality related to a single
//! action of type `PdfActionType::GoToDestinationInRemoteDocument`.

use crate::bindgen::FPDF_ACTION;
use crate::bindings::PdfiumLibraryBindings;
use crate::pdf::action::private::internal::PdfActionPrivate;

pub struct PdfActionRemoteDestination<'a> {
    #[allow(dead_code)]
    handle: FPDF_ACTION,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfActionRemoteDestination<'a> {
    #[inline]
    pub(crate) fn from_pdfium(handle: FPDF_ACTION, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        PdfActionRemoteDestination { handle, bindings }
    }
}

impl<'a> PdfActionPrivate<'a> for PdfActionRemoteDestination<'a> {
    #[inline]
    fn handle(&self) -> &FPDF_ACTION {
        &self.handle
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }
}
