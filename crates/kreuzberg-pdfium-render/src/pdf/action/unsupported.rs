//! Defines the [PdfActionUnsupported] struct, exposing functionality related to a single
//! action of type `PdfActionType::Unsupported`.

use crate::bindgen::FPDF_ACTION;
use crate::bindings::PdfiumLibraryBindings;
use crate::pdf::action::private::internal::PdfActionPrivate;

pub struct PdfActionUnsupported<'a> {
    #[allow(dead_code)]
    handle: FPDF_ACTION,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfActionUnsupported<'a> {
    #[inline]
    pub(crate) fn from_pdfium(handle: FPDF_ACTION, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        PdfActionUnsupported { handle, bindings }
    }
}

impl<'a> PdfActionPrivate<'a> for PdfActionUnsupported<'a> {
    #[inline]
    fn handle(&self) -> &FPDF_ACTION {
        &self.handle
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }
}
