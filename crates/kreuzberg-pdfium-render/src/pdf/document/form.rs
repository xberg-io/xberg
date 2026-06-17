//! Defines the [PdfForm] struct, exposing functionality related to a form
//! embedded in a `PdfDocument`.

use crate::bindgen::{
    FORMTYPE_ACRO_FORM, FORMTYPE_NONE, FORMTYPE_XFA_FOREGROUND, FORMTYPE_XFA_FULL, FPDF_DOCUMENT, FPDF_FORMFILLINFO,
    FPDF_FORMHANDLE,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::pages::PdfPages;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::pin::Pin;
use std::ptr::null_mut;

#[cfg(doc)]
use crate::pdf::document::PdfDocument;

/// The internal definition type of a [PdfForm] embedded in a [PdfDocument].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfFormType {
    // The FORMTYPE_COUNT constant simply specifies the number of form types supported
    // by Pdfium; we do not need to expose it.
    None = FORMTYPE_NONE as isize,
    Acrobat = FORMTYPE_ACRO_FORM as isize,
    XfaFull = FORMTYPE_XFA_FULL as isize,
    XfaForeground = FORMTYPE_XFA_FOREGROUND as isize,
}

impl PdfFormType {
    #[inline]
    pub(crate) fn from_pdfium(form_type: u32) -> Result<PdfFormType, PdfiumError> {
        match form_type {
            FORMTYPE_NONE => Ok(PdfFormType::None),
            FORMTYPE_ACRO_FORM => Ok(PdfFormType::Acrobat),
            FORMTYPE_XFA_FULL => Ok(PdfFormType::XfaFull),
            FORMTYPE_XFA_FOREGROUND => Ok(PdfFormType::XfaForeground),
            _ => Err(PdfiumError::UnknownFormType),
        }
    }

    #[inline]
    #[allow(dead_code)]
    // The as_pdfium() function is not currently used, but we expect it to be in future
    pub(crate) fn as_pdfium(&self) -> u32 {
        match self {
            PdfFormType::None => FORMTYPE_NONE,
            PdfFormType::Acrobat => FORMTYPE_ACRO_FORM,
            PdfFormType::XfaFull => FORMTYPE_XFA_FULL,
            PdfFormType::XfaForeground => FORMTYPE_XFA_FOREGROUND,
        }
    }
}

/// The [PdfForm] embedded inside a [PdfDocument].
///
/// Form fields in Pdfium are exposed as page annotations of type `PdfPageAnnotationType::Widget`
/// or `PdfPageAnnotationType::XfaWidget`, depending on the type of form embedded inside the
/// document. To retrieve the user-specified form field values, iterate over each annotation
/// on each page in the document, filtering out annotations that do not contain a valid form field:
///
/// ```
/// for page in document.pages.iter() {
///     for annotation in page.annotations.iter() {
///         if let Some(field) = annotation.as_form_field() {
///             // We can now unwrap the specific type of form field
///             // and access its properties, including any user-specified value.
///         }
///     }
/// }
/// ```
///
/// Alternatively, use the [PdfForm::field_values()] function to eagerly retrieve the values of all
/// fields in the document as a map of (field name, field value) pairs.
pub struct PdfForm<'a> {
    form_handle: FPDF_FORMHANDLE,
    document_handle: FPDF_DOCUMENT,

    #[allow(dead_code)]
    // The form_fill_info field is not currently used, but we expect it to be in future
    form_fill_info: Pin<Box<FPDF_FORMFILLINFO>>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfForm<'a> {
    /// Attempts to bind to an embedded form, if any, inside the document with the given
    /// document handle.
    #[inline]
    pub(crate) fn from_pdfium(document_handle: FPDF_DOCUMENT, bindings: &'a dyn PdfiumLibraryBindings) -> Option<Self> {
        // Pdfium does not load form field data or widgets (and therefore will not
        // render them) until a call has been made to the
        // FPDFDOC_InitFormFillEnvironment() function. This function takes a large
        // struct, FPDF_FORMFILLINFO, which Pdfium uses to store a variety of form
        // configuration information - mostly callback functions that should be called
        // when the user interacts with a form field widget. Since pdfium-render has
        // no concept of interactivity, we can leave all these set to None.

        // We allocate the FPDF_FORMFILLINFO struct on the heap and pin its pointer location
        // so Rust will not move it around. Pdfium retains the pointer location
        // when we call FPDFDOC_InitFormFillEnvironment() and expects the pointer
        // location to still be valid when we later call FPDFDOC_ExitFormFillEnvironment()
        // during drop(); if we don't pin the struct's location it may move, and the
        // call to FPDFDOC_ExitFormFillEnvironment() will segfault.

        let mut form_fill_info = Box::pin(FPDF_FORMFILLINFO {
            version: 2,
            Release: None,
            FFI_Invalidate: None,
            FFI_OutputSelectedRect: None,
            FFI_SetCursor: None,
            FFI_SetTimer: None,
            FFI_KillTimer: None,
            FFI_GetLocalTime: None,
            FFI_OnChange: None,
            FFI_GetPage: None,
            FFI_GetCurrentPage: None,
            FFI_GetRotation: None,
            FFI_ExecuteNamedAction: None,
            FFI_SetTextFieldFocus: None,
            FFI_DoURIAction: None,
            FFI_DoGoToAction: None,
            m_pJsPlatform: null_mut(),
            xfa_disabled: 0,
            FFI_DisplayCaret: None,
            FFI_GetCurrentPageIndex: None,
            FFI_SetCurrentPage: None,
            FFI_GotoURL: None,
            FFI_GetPageViewRect: None,
            FFI_PageEvent: None,
            FFI_PopupMenu: None,
            FFI_OpenFile: None,
            FFI_EmailTo: None,
            FFI_UploadTo: None,
            FFI_GetPlatform: None,
            FFI_GetLanguage: None,
            FFI_DownloadFromURL: None,
            FFI_PostRequestURL: None,
            FFI_PutRequestURL: None,
            FFI_OnFocusChange: None,
            FFI_DoURIActionWithKeyboardModifier: None,
        });

        let form_handle = bindings.FPDFDOC_InitFormFillEnvironment(document_handle, form_fill_info.deref_mut());

        if !form_handle.is_null() {
            // There is a form embedded in this document, and we retrieved a valid handle to it.

            let form = PdfForm {
                form_handle,
                document_handle,
                form_fill_info,
                bindings,
            };

            if let Ok(form_type) = form.form_type() {
                if form_type != PdfFormType::None {
                    Some(form)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // There is no form embedded in this document.

            None
        }
    }

    /// Returns the internal `FPDF_FORMHANDLE` handle for this [PdfForm].
    #[inline]
    pub(crate) fn handle(&self) -> FPDF_FORMHANDLE {
        self.form_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfForm].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the [PdfFormType] of this [PdfForm].
    #[inline]
    pub fn form_type(&self) -> Result<PdfFormType, PdfiumError> {
        PdfFormType::from_pdfium(self.bindings.FPDF_GetFormType(self.document_handle) as u32)
    }

    /// Captures a string representation of the value of every form field on every page of
    /// the given [PdfPages] collection, returning a map of (field name, field value) pairs.
    ///
    /// This function assumes that all form fields in the document have unique field names
    /// except for radio button and checkbox control groups.
    ///
    /// Note: form field extraction via widget annotations is not supported in this build.
    /// Returns an empty map.
    pub fn field_values(&self, _pages: &'a PdfPages<'a>) -> HashMap<String, Option<String>> {
        HashMap::new()
    }
}

impl<'a> Drop for PdfForm<'a> {
    /// Closes this [PdfForm], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        self.bindings.FPDFDOC_ExitFormFillEnvironment(self.form_handle);
    }
}
