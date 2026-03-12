//! Defines the [PdfDocument] struct, the entry point to all Pdfium functionality
//! related to a single PDF file.

pub mod fonts;
pub mod form;
pub mod metadata;
pub mod page;
pub mod pages;
pub mod permissions;

use crate::bindgen::FPDF_DOCUMENT;
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::error::PdfiumInternalError;
use crate::pdf::document::fonts::PdfFonts;
use crate::pdf::document::form::PdfForm;
use crate::pdf::document::metadata::PdfMetadata;
use crate::pdf::document::page::index_cache::PdfPageIndexCache;
use crate::pdf::document::pages::PdfPages;
use crate::pdf::document::permissions::PdfPermissions;
use crate::utils::files::FpdfFileAccessExt;
use crate::utils::files::get_pdfium_file_writer_from_writer;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::io::Write;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Uint8Array};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use web_sys::Blob;

// The following dummy declaration is used only when running cargo doc.
// It allows documentation of WASM-specific functionality to be included
// in documentation generated on non-WASM targets.

#[cfg(doc)]
struct Blob;

/// The file version of a [PdfDocument].
///
/// A list of PDF file versions is available at <https://en.wikipedia.org/wiki/History_of_PDF>.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfDocumentVersion {
    /// No version information is available. This is the case if the [PdfDocument]
    /// was created via a call to `Pdfium::create_new_pdf()` rather than loaded from a file.
    Unset,

    /// PDF 1.0, first published in 1993, supported by Acrobat Reader Carousel (1.0) onwards.
    Pdf1_0,

    /// PDF 1.1, first published in 1994, supported by Acrobat Reader 2.0 onwards.
    Pdf1_1,

    /// PDF 1.2, first published in 1996, supported by Acrobat Reader 3.0 onwards.
    Pdf1_2,

    /// PDF 1.3, first published in 2000, supported by Acrobat Reader 4.0 onwards.
    Pdf1_3,

    /// PDF 1.4, first published in 2001, supported by Acrobat Reader 5.0 onwards.
    Pdf1_4,

    /// PDF 1.5, first published in 2003, supported by Acrobat Reader 6.0 onwards.
    Pdf1_5,

    /// PDF 1.6, first published in 2004, supported by Acrobat Reader 7.0 onwards.
    Pdf1_6,

    /// PDF 1.7, first published in 2006, supported by Acrobat Reader 8.0 onwards,
    /// adopted as ISO open standard 32000-1 in 2008. Certain proprietary Adobe
    /// extensions to PDF 1.7 are only fully supported in Acrobat Reader X (10.0)
    /// and later.
    Pdf1_7,

    /// PDF 2.0, first published in 2017, ISO open standard 32000-2.
    Pdf2_0,

    /// A two-digit raw file version number. For instance, a value of 21 would indicate
    /// PDF version 2.1, a value of 34 would indicate PDF version 3.4, and so on.
    /// Only used when the file version number is not directly recognized by
    /// pdfium-render.
    Other(i32),
}

impl PdfDocumentVersion {
    /// The default [PdfDocumentVersion] applied to new documents.
    pub const DEFAULT_VERSION: PdfDocumentVersion = PdfDocumentVersion::Pdf1_7;

    #[inline]
    pub(crate) fn from_pdfium(version: i32) -> Self {
        match version {
            10 => PdfDocumentVersion::Pdf1_0,
            11 => PdfDocumentVersion::Pdf1_1,
            12 => PdfDocumentVersion::Pdf1_2,
            13 => PdfDocumentVersion::Pdf1_3,
            14 => PdfDocumentVersion::Pdf1_4,
            15 => PdfDocumentVersion::Pdf1_5,
            16 => PdfDocumentVersion::Pdf1_6,
            17 => PdfDocumentVersion::Pdf1_7,
            20 => PdfDocumentVersion::Pdf2_0,
            _ => PdfDocumentVersion::Other(version),
        }
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> Option<i32> {
        match self {
            PdfDocumentVersion::Pdf1_0 => Some(10),
            PdfDocumentVersion::Pdf1_1 => Some(11),
            PdfDocumentVersion::Pdf1_2 => Some(12),
            PdfDocumentVersion::Pdf1_3 => Some(13),
            PdfDocumentVersion::Pdf1_4 => Some(14),
            PdfDocumentVersion::Pdf1_5 => Some(15),
            PdfDocumentVersion::Pdf1_6 => Some(16),
            PdfDocumentVersion::Pdf1_7 => Some(17),
            PdfDocumentVersion::Pdf2_0 => Some(20),
            PdfDocumentVersion::Other(value) => Some(*value),
            PdfDocumentVersion::Unset => None,
        }
    }
}

/// An entry point to all the various object collections contained in a single PDF file.
/// These collections include:
/// * [PdfDocument::fonts()], an immutable collection of all the [PdfFonts] in the document.
/// * [PdfDocument::fonts_mut()], a mutable collection of all the [PdfFonts] in the document.
/// * [PdfDocument::form()], an immutable reference to the [PdfForm] embedded in the document, if any.
/// * [PdfDocument::metadata()], an immutable collection of all the [PdfMetadata] tags in the document.
/// * [PdfDocument::pages()], an immutable collection of all the [PdfPages] in the document.
/// * [PdfDocument::pages_mut()], a mutable collection of all the [PdfPages] in the document.
/// * [PdfDocument::permissions()], settings relating to security handlers and document permissions
///   for the document.
pub struct PdfDocument<'a> {
    handle: FPDF_DOCUMENT,
    output_version: Option<PdfDocumentVersion>,
    form: Option<PdfForm<'a>>,
    fonts: PdfFonts<'a>,
    metadata: PdfMetadata<'a>,
    pages: PdfPages<'a>,
    permissions: PdfPermissions<'a>,
    bindings: &'a dyn PdfiumLibraryBindings,
    source_byte_buffer: Option<Vec<u8>>,

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    // This field is never used when compiling to WASM.
    file_access_reader: Option<Box<FpdfFileAccessExt<'a>>>,
}

impl<'a> PdfDocument<'a> {
    #[inline]
    pub(crate) fn from_pdfium(handle: FPDF_DOCUMENT, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        let form = PdfForm::from_pdfium(handle, bindings);

        let pages = PdfPages::from_pdfium(handle, form.as_ref().map(|form| form.handle()), bindings);

        PdfDocument {
            handle,
            output_version: None,
            form,
            fonts: PdfFonts::from_pdfium(handle, bindings),
            metadata: PdfMetadata::from_pdfium(handle, bindings),
            pages,
            permissions: PdfPermissions::from_pdfium(handle, bindings),
            bindings,
            source_byte_buffer: None,
            file_access_reader: None,
        }
    }

    /// Returns the internal `FPDF_DOCUMENT` handle for this [PdfDocument].
    #[inline]
    pub(crate) fn handle(&self) -> FPDF_DOCUMENT {
        self.handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfDocument].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Transfers ownership of the byte buffer containing the binary data of this [PdfDocument],
    /// so that it will always be available for Pdfium to read data from as needed.
    #[inline]
    pub(crate) fn set_source_byte_buffer(&mut self, bytes: Vec<u8>) {
        self.source_byte_buffer = Some(bytes);
    }

    /// Binds an `FPDF_FILEACCESS` reader to the lifetime of this [PdfDocument], so that
    /// it will always be available for Pdfium to read data from as needed.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    // This function is never used when compiling to WASM.
    #[inline]
    pub(crate) fn set_file_access_reader(&mut self, reader: Box<FpdfFileAccessExt<'a>>) {
        self.file_access_reader = Some(reader);
    }

    /// Returns the file version of this [PdfDocument].
    pub fn version(&self) -> PdfDocumentVersion {
        let mut version = 0;

        if self.bindings.FPDF_GetFileVersion(self.handle, &mut version) != 0 {
            PdfDocumentVersion::from_pdfium(version)
        } else {
            PdfDocumentVersion::Unset
        }
    }

    /// Sets the file version that will be used the next time this [PdfDocument] is saved.
    pub fn set_version(&mut self, version: PdfDocumentVersion) {
        self.output_version = Some(version);
    }

    /// Returns `true` if this [PdfDocument] is a tagged PDF.
    pub fn is_tagged(&self) -> bool {
        self.bindings.is_true(self.bindings.FPDFCatalog_IsTagged(self.handle))
    }

    /// Returns `true` if this [PdfDocument] has a valid cross-reference table.
    pub fn has_valid_cross_reference_table(&self) -> bool {
        self.bindings
            .is_true(self.bindings.FPDF_DocumentHasValidCrossReferenceTable(self.handle))
    }

    /// Returns an immutable reference to the [PdfForm] embedded in this [PdfDocument], if any.
    #[inline]
    pub fn form(&self) -> Option<&PdfForm<'_>> {
        self.form.as_ref()
    }

    /// Returns an immutable collection of all the [PdfFonts] in this [PdfDocument].
    #[inline]
    pub fn fonts(&self) -> &PdfFonts<'_> {
        &self.fonts
    }

    /// Returns a mutable collection of all the [PdfFonts] in this [PdfDocument].
    #[inline]
    pub fn fonts_mut(&mut self) -> &mut PdfFonts<'a> {
        &mut self.fonts
    }

    /// Returns an immutable collection of all the [PdfMetadata] tags in this [PdfDocument].
    #[inline]
    pub fn metadata(&self) -> &PdfMetadata<'_> {
        &self.metadata
    }

    /// Returns an immutable collection of all the [PdfPages] in this [PdfDocument].
    #[inline]
    pub fn pages(&self) -> &PdfPages<'a> {
        &self.pages
    }

    /// Returns a mutable collection of all the [PdfPages] in this [PdfDocument].
    #[inline]
    pub fn pages_mut(&mut self) -> &mut PdfPages<'a> {
        &mut self.pages
    }

    /// Returns an immutable collection of all the [PdfPermissions] applied to this [PdfDocument].
    #[inline]
    pub fn permissions(&self) -> &PdfPermissions<'_> {
        &self.permissions
    }

    /// Writes this [PdfDocument] to the given writer.
    pub fn save_to_writer<W: Write + 'static>(&self, writer: &mut W) -> Result<(), PdfiumError> {
        // TODO: AJRC - 25/5/22 - investigate supporting the FPDF_INCREMENTAL, FPDF_NO_INCREMENTAL,
        // and FPDF_REMOVE_SECURITY flags defined in fpdf_save.h. There's not a lot of information
        // on what they actually do, however.
        // Some small info at https://forum.patagames.com/posts/t155-PDF-SaveFlags.

        let flags = 0;

        let mut pdfium_file_writer = get_pdfium_file_writer_from_writer(writer);

        let result = match self.output_version {
            Some(version) => self.bindings.FPDF_SaveWithVersion(
                self.handle,
                pdfium_file_writer.as_fpdf_file_write_mut_ptr(),
                flags,
                version
                    .as_pdfium()
                    .unwrap_or_else(|| PdfDocumentVersion::DEFAULT_VERSION.as_pdfium().unwrap()),
            ),
            None => self
                .bindings
                .FPDF_SaveAsCopy(self.handle, pdfium_file_writer.as_fpdf_file_write_mut_ptr(), flags),
        };

        match self.bindings.is_true(result) {
            true => {
                // Pdfium's return value indicated success. Flush the buffer.

                pdfium_file_writer.flush().map_err(PdfiumError::IoError)
            }
            false => {
                // Pdfium's return value indicated failure.

                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        }
    }

    /// Writes this [PdfDocument] to the file at the given path.
    ///
    /// This function is not available when compiling to WASM. You have several options for
    /// saving your PDF document data in WASM:
    /// * Use either the [PdfDocument::save_to_writer()] or the [PdfDocument::save_to_bytes()] functions,
    ///   both of which are available when compiling to WASM.
    /// * Use the [PdfDocument::save_to_blob()] function to save document data directly into a new
    ///   Javascript `Blob` object. This function is only available when compiling to WASM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self, path: &(impl AsRef<Path> + ?Sized)) -> Result<(), PdfiumError> {
        self.save_to_writer(&mut File::create(path).map_err(PdfiumError::IoError)?)
    }

    /// Writes this [PdfDocument] to a new byte buffer, returning the byte buffer.
    pub fn save_to_bytes(&self) -> Result<Vec<u8>, PdfiumError> {
        let mut cursor = Cursor::new(Vec::new());

        self.save_to_writer(&mut cursor)?;

        Ok(cursor.into_inner())
    }

    /// Writes this [PdfDocument] to a new `Blob`, returning the `Blob`.
    ///
    /// This function is only available when compiling to WASM.
    #[cfg(any(doc, target_arch = "wasm32"))]
    pub fn save_to_blob(&self) -> Result<Blob, PdfiumError> {
        let bytes = self.save_to_bytes()?;

        let array = Uint8Array::new_with_length(bytes.len() as u32);

        array.copy_from(bytes.as_slice());

        let blob = Blob::new_with_u8_array_sequence(&JsValue::from(Array::of1(&JsValue::from(array))))
            .map_err(|_| PdfiumError::JsSysErrorConstructingBlobFromBytes)?;

        Ok(blob)
    }
}

impl<'a> Drop for PdfDocument<'a> {
    /// Closes this [PdfDocument], releasing held memory and, if the document was loaded
    /// from a file, the file handle on the document.
    #[inline]
    fn drop(&mut self) {
        // Drop this document's PdfForm, if any, before we close the document itself.
        // This ensures that FPDFDOC_ExitFormFillEnvironment() is called _before_ FPDF_CloseDocument(),
        // avoiding a segmentation fault when using Pdfium builds compiled with V8/XFA support.

        self.form = None;

        // Clear all cached page indices for this document before closing.
        // This prevents stale cache entries if Pdfium reuses the document handle.
        PdfPageIndexCache::clear_document(self.handle);

        self.bindings.FPDF_CloseDocument(self.handle);
    }
}

impl<'a> Debug for PdfDocument<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocument")
            .field("FPDF_DOCUMENT", &format!("{:?}", self.handle))
            .finish()
    }
}
