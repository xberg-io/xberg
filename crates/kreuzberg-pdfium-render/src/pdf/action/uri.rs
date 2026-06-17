//! Defines the [PdfActionUri] struct, exposing functionality related to a single
//! action of type `PdfActionType::Uri`.

use crate::bindgen::{FPDF_ACTION, FPDF_DOCUMENT};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::action::private::internal::PdfActionPrivate;
use crate::utils::mem::create_byte_buffer;
use std::ffi::{CString, c_void};

pub struct PdfActionUri<'a> {
    handle: FPDF_ACTION,
    document: FPDF_DOCUMENT,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfActionUri<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_ACTION,
        document: FPDF_DOCUMENT,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfActionUri {
            handle,
            document,
            bindings,
        }
    }

    /// Returns the URI path associated with this [PdfActionUri], if any.
    pub fn uri(&self) -> Result<String, PdfiumError> {
        // Retrieving the URI path from Pdfium is a two-step operation. First, we call
        // FPDFAction_GetURIPath() with a null buffer; this will retrieve the length of
        // the path in bytes. If the length is zero, then there is no path associated
        // with this action.

        // If the length is non-zero, then we reserve a byte buffer of the given
        // length and call FPDFAction_GetURIPath() again with a pointer to the buffer;
        // this will write the path to the buffer as an array of 7-bit ASCII characters.

        let buffer_length = self
            .bindings()
            .FPDFAction_GetURIPath(self.document, self.handle, std::ptr::null_mut(), 0);

        if buffer_length == 0 {
            // There is no URI path for this action.

            return Err(PdfiumError::NoUriForAction);
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings().FPDFAction_GetURIPath(
            self.document,
            self.handle,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        if let Ok(result) = CString::from_vec_with_nul(buffer) {
            result.into_string().map_err(PdfiumError::CStringConversionError)
        } else {
            Err(PdfiumError::NoUriForAction)
        }
    }
}

impl<'a> PdfActionPrivate<'a> for PdfActionUri<'a> {
    #[inline]
    fn handle(&self) -> &FPDF_ACTION {
        &self.handle
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }
}
