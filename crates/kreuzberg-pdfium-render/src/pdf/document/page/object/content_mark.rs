//! Defines the [PdfPageObjectContentMark] struct, exposing functionality related to a single
//! content mark associated with a [PdfPageObject].

use crate::bindgen::{FPDF_PAGEOBJECTMARK, FPDF_WCHAR};
use crate::bindings::PdfiumLibraryBindings;
use crate::utils::mem::create_byte_buffer;
use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
use std::os::raw::{c_int, c_ulong};

/// A single content mark associated with a `PdfPageObject`.
///
/// Content marks provide a bridge between page objects (geometric/visual elements)
/// and the PDF structure tree (semantic document structure). Each mark has a name
/// (e.g., "P", "Span", "Artifact") and optional key-value parameters.
pub struct PdfPageObjectContentMark<'a> {
    mark_handle: FPDF_PAGEOBJECTMARK,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageObjectContentMark<'a> {
    pub(crate) fn from_pdfium(mark_handle: FPDF_PAGEOBJECTMARK, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self { mark_handle, bindings }
    }

    /// Returns the name of this content mark (e.g., "P", "Span", "Artifact").
    pub fn name(&self) -> Option<String> {
        // Retrieving the mark name from Pdfium is a two-step operation. First, we call
        // FPDFPageObjMark_GetName() with a null buffer to retrieve the required buffer
        // size in bytes. Then we allocate a buffer of that size and call the function
        // again to retrieve the actual name.

        let mut name_length: c_ulong = 0;

        if !self.bindings.is_true(self.bindings.FPDFPageObjMark_GetName(
            self.mark_handle,
            std::ptr::null_mut(),
            0,
            &mut name_length,
        )) {
            return None;
        }

        if name_length == 0 {
            return None;
        }

        let mut buffer = create_byte_buffer(name_length as usize);

        let mut actual_length: c_ulong = 0;

        if self.bindings.is_true(self.bindings.FPDFPageObjMark_GetName(
            self.mark_handle,
            buffer.as_mut_ptr() as *mut FPDF_WCHAR,
            name_length,
            &mut actual_length,
        )) {
            get_string_from_pdfium_utf16le_bytes(buffer)
        } else {
            None
        }
    }

    /// Returns the number of key-value parameters in this content mark.
    pub fn param_count(&self) -> usize {
        let count = self.bindings.FPDFPageObjMark_CountParams(self.mark_handle);

        if count < 0 { 0 } else { count as usize }
    }

    /// Returns the key name of the parameter at the given index.
    pub fn param_key(&self, index: usize) -> Option<String> {
        let mut key_length: c_ulong = 0;

        if !self.bindings.is_true(self.bindings.FPDFPageObjMark_GetParamKey(
            self.mark_handle,
            index as c_ulong,
            std::ptr::null_mut(),
            0,
            &mut key_length,
        )) {
            return None;
        }

        if key_length == 0 {
            return None;
        }

        let mut buffer = create_byte_buffer(key_length as usize);

        let mut actual_length: c_ulong = 0;

        if self.bindings.is_true(self.bindings.FPDFPageObjMark_GetParamKey(
            self.mark_handle,
            index as c_ulong,
            buffer.as_mut_ptr() as *mut FPDF_WCHAR,
            key_length,
            &mut actual_length,
        )) {
            get_string_from_pdfium_utf16le_bytes(buffer)
        } else {
            None
        }
    }

    /// Returns the integer value of the parameter with the given key, if the parameter
    /// exists and is a number type.
    pub fn param_int_value(&self, key: &str) -> Option<i32> {
        let mut value: c_int = 0;

        if self.bindings.is_true(
            self.bindings
                .FPDFPageObjMark_GetParamIntValue(self.mark_handle, key, &mut value),
        ) {
            Some(value as i32)
        } else {
            None
        }
    }

    /// Returns the float value of the parameter with the given key, if the parameter
    /// exists and is a number type.
    pub fn param_float_value(&self, key: &str) -> Option<f32> {
        let mut value: f32 = 0.0;

        if self.bindings.is_true(
            self.bindings
                .FPDFPageObjMark_GetParamFloatValue(self.mark_handle, key, &mut value),
        ) {
            Some(value)
        } else {
            None
        }
    }

    /// Returns the string value of the parameter with the given key, if the parameter
    /// exists and is a string type.
    pub fn param_string_value(&self, key: &str) -> Option<String> {
        let mut value_length: c_ulong = 0;

        if !self.bindings.is_true(self.bindings.FPDFPageObjMark_GetParamStringValue(
            self.mark_handle,
            key,
            std::ptr::null_mut(),
            0,
            &mut value_length,
        )) {
            return None;
        }

        if value_length == 0 {
            return None;
        }

        let mut buffer = create_byte_buffer(value_length as usize);

        let mut actual_length: c_ulong = 0;

        if self.bindings.is_true(self.bindings.FPDFPageObjMark_GetParamStringValue(
            self.mark_handle,
            key,
            buffer.as_mut_ptr() as *mut FPDF_WCHAR,
            value_length,
            &mut actual_length,
        )) {
            get_string_from_pdfium_utf16le_bytes(buffer)
        } else {
            None
        }
    }
}
