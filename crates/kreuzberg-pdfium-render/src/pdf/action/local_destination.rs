//! Defines the [PdfActionLocalDestination] struct, exposing functionality related to a single
//! action of type `PdfActionType::GoToDestinationInSameDocument`.

use crate::bindgen::{FPDF_ACTION, FPDF_DOCUMENT};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::action::private::internal::PdfActionPrivate;
use crate::pdf::destination::PdfDestination;

pub struct PdfActionLocalDestination<'a> {
    handle: FPDF_ACTION,
    document: FPDF_DOCUMENT,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfActionLocalDestination<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_ACTION,
        document: FPDF_DOCUMENT,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfActionLocalDestination {
            handle,
            document,
            bindings,
        }
    }

    /// Returns the target [PdfDestination] for this [PdfActionLocalDestination].
    pub fn destination(&self) -> Result<PdfDestination<'_>, PdfiumError> {
        let handle = self.bindings.FPDFAction_GetDest(self.document, self.handle);

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfDestination::from_pdfium(self.document, handle, self.bindings))
        }
    }
}

impl<'a> PdfActionPrivate<'a> for PdfActionLocalDestination<'a> {
    #[inline]
    fn handle(&self) -> &FPDF_ACTION {
        &self.handle
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }
}
