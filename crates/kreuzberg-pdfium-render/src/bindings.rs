//! Defines the [PdfiumLibraryBindings] trait, containing run-time bindings to the FPDF_*
//! functions exported by the Pdfium library.
//!
//! By default, `pdfium-render` attempts to bind against the latest released version
//! of the Pdfium API. To explicitly bind against an older version, select one of the
//! crate's Pdfium version feature flags when taking `pdfium-render` as a dependency
//! in your project's `Cargo.toml`.
//!
//! Doc comments on functions in this trait are taken directly from the Pdfium header files
//! and as such are copyright by the Pdfium authors and Google. They are reproduced here
//! as a courtesy for API consumers. The original comments can be found in the Pdfium repository at:
//! <https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/>

// Include the appropriate implementation of the PdfiumLibraryBindings trait for the
// target architecture. Conditional compilation is used to compile different implementations
// of the PdfiumLibraryBindings trait depending on whether we are compiling to a WASM module,
// a native shared library, or a statically linked library.

#![allow(clippy::too_many_arguments)]

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(pdfium_use_static))]
pub(crate) mod dynamic_bindings;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(pdfium_use_static)]
pub(crate) mod static_bindings;

#[cfg(target_arch = "wasm32")]
#[allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::let_and_return,
    clippy::needless_borrow,
    clippy::derivable_impls,
    clippy::unnecessary_cast
)]
pub(crate) mod wasm_bindings;

// The following dummy declarations are used only when running cargo doc.
// They allow documentation of any target-specific functionality to be included
// in documentation generated on a different target.

#[cfg(doc)]
struct Uint8Array;

#[cfg(doc)]
struct HDC;

pub mod version;

pub use crate::bindgen::{
    FPDF_ACTION, FPDF_ANNOT_APPEARANCEMODE, FPDF_ANNOTATION, FPDF_ANNOTATION_SUBTYPE, FPDF_ATTACHMENT, FPDF_AVAIL,
    FPDF_BITMAP, FPDF_BOOKMARK, FPDF_BOOL, FPDF_CLIPPATH, FPDF_COLORSCHEME, FPDF_CharsetFontMap, FPDF_DEST,
    FPDF_DOCUMENT, FPDF_DUPLEXTYPE, FPDF_DWORD, FPDF_FILEACCESS, FPDF_FILEIDTYPE, FPDF_FILEWRITE, FPDF_FONT,
    FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_GLYPHPATH, FPDF_IMAGEOBJ_METADATA, FPDF_JAVASCRIPT_ACTION,
    FPDF_LIBRARY_CONFIG, FPDF_LINK, FPDF_OBJECT_TYPE, FPDF_PAGE, FPDF_PAGELINK, FPDF_PAGEOBJ_FORM, FPDF_PAGEOBJ_IMAGE,
    FPDF_PAGEOBJ_PATH, FPDF_PAGEOBJ_SHADING, FPDF_PAGEOBJ_TEXT, FPDF_PAGEOBJ_UNKNOWN, FPDF_PAGEOBJECT,
    FPDF_PAGEOBJECTMARK, FPDF_PAGERANGE, FPDF_PATHSEGMENT, FPDF_SCHHANDLE, FPDF_SIGNATURE, FPDF_STRUCTELEMENT,
    FPDF_STRUCTELEMENT_ATTR, FPDF_STRUCTTREE, FPDF_SYSFONTINFO, FPDF_TEXT_RENDERMODE, FPDF_TEXTPAGE, FPDF_WCHAR,
    FPDF_WIDESTRING, FPDF_XOBJECT, FPDFANNOT_COLORTYPE, FS_FLOAT, FS_MATRIX, FS_POINTF, FS_QUADPOINTSF, FS_RECTF,
    FS_SIZEF, FX_DOWNLOADHINTS, FX_FILEAVAIL, IFSDK_PAUSE,
};

pub use crate::bindgen::FPDF_STRUCTELEMENT_ATTR_VALUE;

use crate::bindgen::size_t;
use crate::bindings::version::PdfiumApiVersion;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::bitmap::PdfBitmap;
use crate::pdf::color::PdfColor;
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::PdfPage;
use crate::pdf::document::page::object::PdfPageObject;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::matrix::PdfMatrix;
use crate::pdf::rect::PdfRect;
use crate::prelude::PdfQuadPoints;
use crate::utils::pixels::{bgra_to_rgba, rgba_to_bgra, unaligned_bgr_to_rgba, unaligned_rgb_to_bgra};
use crate::utils::utf16le::{get_pdfium_utf16le_bytes_from_str, get_string_from_pdfium_utf16le_bytes};
use std::os::raw::{c_char, c_double, c_float, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void};

/// Platform-independent function bindings to an external Pdfium library.
/// On most platforms this will be an external shared library loaded dynamically
/// at runtime, either bundled alongside your compiled Rust application or provided as a system
/// library by the platform. On WASM, this will be a set of Javascript functions exposed by a
/// separate WASM module that is imported into the same browser context.
///
/// Pdfium's API uses three different string types: classic C-style null-terminated char arrays,
/// UTF-8 byte arrays, and a UTF-16LE byte array type named `FPDF_WIDESTRING`. For functions that take a
/// C-style string or a UTF-8 byte array, `pdfium-render`'s binding will take the standard Rust `&str` type.
/// For functions that take an `FPDF_WIDESTRING`, `pdfium-render` exposes two functions: the vanilla
/// `FPDF_*()` function that takes an `FPDF_WIDESTRING`, and an additional `FPDF_*_str()` helper function
/// that takes a standard Rust `&str` and converts it internally to an `FPDF_WIDESTRING` before calling
/// Pdfium. Examples of functions with additional `_str()` helpers include `FPDFBookmark_Find()`,
/// `FPDFAnnot_SetStringValue()`, and `FPDFText_SetText()`.
///
/// The [PdfiumLibraryBindings::get_pdfium_utf16le_bytes_from_str] and
/// [PdfiumLibraryBindings::get_string_from_pdfium_utf16le_bytes] functions are provided
/// for converting to and from UTF-16LE in your own code.
///
/// The following Pdfium functions have different signatures in this trait compared to their
/// native function signatures in Pdfium:
/// * [PdfiumLibraryBindings::FPDF_LoadDocument]: this function is not available when compiling to WASM.
/// * [PdfiumLibraryBindings::FPDFBitmap_GetBuffer]: this function is not available when compiling
///   to WASM. Use the globally-available [PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_vec]
///   or the WASM-specific [PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_array] functions instead.
#[allow(drop_bounds)] // We want every bindings implementation to also provide a Drop implementation.
pub trait PdfiumLibraryBindings: Send + Sync + Drop {
    /// Returns the canonical C-style boolean integer value 1, indicating `true`.
    #[inline]
    #[allow(non_snake_case)]
    fn TRUE(&self) -> FPDF_BOOL {
        1
    }

    /// Returns the canonical C-style boolean integer value 0, indicating `false`.
    #[inline]
    #[allow(non_snake_case)]
    fn FALSE(&self) -> FPDF_BOOL {
        0
    }

    /// Converts from a C-style boolean integer to a Rust `bool`.
    ///
    /// Assumes `PdfiumLibraryBindings::FALSE()` indicates `false` and any other value indicates `true`.
    #[inline]
    fn is_true(&self, bool: FPDF_BOOL) -> bool {
        bool != self.FALSE()
    }

    /// Converts the given Rust `bool` into a Pdfium `FPDF_BOOL`.
    #[inline]
    fn bool_to_pdfium(&self, bool: bool) -> FPDF_BOOL {
        if bool { self.TRUE() } else { self.FALSE() }
    }

    /// Converts from a C-style boolean integer to a Rust `Result`.
    ///
    /// Assumes `PdfiumLibraryBindings::FALSE()` indicates `false` and any other value indicates `true`.
    ///
    /// A value of `PdfiumLibraryBindings::FALSE()` will return a [PdfiumInternalError::Unknown].
    /// All other values will return `Ok(())`.
    #[inline]
    fn to_result(&self, bool: FPDF_BOOL) -> Result<(), PdfiumError> {
        if self.is_true(bool) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Converts the given Rust `&str` into an UTF16-LE encoded byte buffer.
    #[inline]
    fn get_pdfium_utf16le_bytes_from_str(&self, str: &str) -> Vec<u8> {
        get_pdfium_utf16le_bytes_from_str(str)
    }

    /// Converts the bytes in the given buffer from UTF16-LE to a standard Rust `String`.
    #[inline]
    #[allow(unused_mut)] // The buffer must be mutable when compiling to WASM.
    fn get_string_from_pdfium_utf16le_bytes(&self, mut buffer: Vec<u8>) -> Option<String> {
        get_string_from_pdfium_utf16le_bytes(buffer)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR,
    /// into pixel data encoded as four-channel RGBA. A new alpha channel is created with full opacity.
    #[inline]
    fn bgr_to_rgba(&self, bgr: &[u8]) -> Vec<u8> {
        unaligned_bgr_to_rgba(bgr)
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel BGRA,
    /// into pixel data encoded as four-channel RGBA.
    #[inline]
    fn bgra_to_rgba(&self, bgra: &[u8]) -> Vec<u8> {
        bgra_to_rgba(bgra)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel RGB,
    /// into pixel data encoded as four-channel BGRA. A new alpha channel is created with full opacity.
    #[inline]
    fn rgb_to_bgra(&self, rgb: &[u8]) -> Vec<u8> {
        unaligned_rgb_to_bgra(rgb)
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel RGBA,
    /// into pixel data encoded as four-channel BGRA.
    #[inline]
    fn rgba_to_bgra(&self, rgba: &[u8]) -> Vec<u8> {
        rgba_to_bgra(rgba)
    }

    /// Returns Pdfium's internal `FPDF_DOCUMENT` handle for the given [PdfDocument].
    #[inline]
    fn get_handle_from_document(&self, document: &PdfDocument) -> FPDF_DOCUMENT {
        document.handle()
    }

    /// Returns Pdfium's internal `FPDF_PAGE` handle for the given [PdfPage].
    #[inline]
    fn get_handle_from_page(&self, page: &PdfPage) -> FPDF_PAGE {
        page.page_handle()
    }

    /// Returns Pdfium's internal `FPDF_PAGEOBJECT` handle for the given [PdfPageObject].
    #[inline]
    fn get_handle_from_object(&self, object: &PdfPageObject) -> FPDF_PAGEOBJECT {
        object.object_handle()
    }

    /// Returns Pdfium's internal `FPDF_BITMAP` handle for the given [PdfBitmap].
    #[inline]
    fn get_handle_from_bitmap(&self, bitmap: &PdfBitmap) -> FPDF_BITMAP {
        bitmap.handle()
    }

    /// Returns Pdfium's internal `FS_MATRIX` struct representation for the given [PdfMatrix].
    #[inline]
    fn get_fs_matrix_from_matrix(&self, matrix: &PdfMatrix) -> FS_MATRIX {
        matrix.as_pdfium()
    }

    /// Returns Pdfium's internal `FS_RECTF` struct representation for the given [PdfRect].
    #[inline]
    fn get_fs_rect_from_rect(&self, rect: &PdfRect) -> FS_RECTF {
        rect.as_pdfium()
    }

    /// Returns Pdfium's internal `FS_QUADPOINTSF` struct representation for the given [PdfQuadPoints].
    #[inline]
    fn get_fs_quad_points_from_quad_points(&self, quad_points: &PdfQuadPoints) -> FS_QUADPOINTSF {
        quad_points.as_pdfium()
    }

    /// Returns the given color encoded as a 32-bit hexadecimal 0xAARRGGBB value,
    /// suitable for passing to Pdfium.
    #[inline]
    fn get_dword_from_color(&self, color: &PdfColor) -> FPDF_DWORD {
        color.as_pdfium_color()
    }

    /// Returns a tuple comprising the given color encoded as a 32-bit hexadecimal 0xFFRRGGBB
    /// value and the color's alpha channel encoded as an 8-bit value, suitable for passing
    /// to Pdfium.
    #[inline]
    fn get_dword_and_alpha_from_color(&self, color: &PdfColor) -> (FPDF_DWORD, u8) {
        color.as_pdfium_color_with_alpha()
    }

    /// Returns the API version of the Pdfium FPDF_* API currently in use.
    ///
    /// By default, `pdfium-render` attempts to bind against the latest released version
    /// of the Pdfium API. To explicitly bind against an older version, select one of the
    /// crate's Pdfium version feature flags when taking `pdfium-render` as a dependency
    /// in your project's `Cargo.toml`.
    #[inline]
    fn version(&self) -> PdfiumApiVersion {
        PdfiumApiVersion::current()
    }

    /// Initializes the PDFium library and allocate global resources for it.
    ///
    ///    `config` - configuration information.
    ///
    /// You have to call this function before you can call any PDF processing functions.
    #[allow(non_snake_case)]
    fn FPDF_InitLibraryWithConfig(&self, config: *const FPDF_LIBRARY_CONFIG);

    /// Initializes the PDFium library (alternative form).
    ///
    /// Convenience function to call [PdfiumLibraryBindings::FPDF_InitLibraryWithConfig]
    /// with a default configuration for backwards compatibility purposes. New code should
    /// call [PdfiumLibraryBindings::FPDF_InitLibraryWithConfig] instead.
    /// This will be deprecated in the future.
    #[allow(non_snake_case)]
    fn FPDF_InitLibrary(&self);

    /// Releases global resources allocated to the PDFium library by [PdfiumLibraryBindings::FPDF_InitLibrary]
    /// or [PdfiumLibraryBindings::FPDF_InitLibraryWithConfig]. After this function is called,
    /// you must not call any PDF processing functions. Calling this function does not automatically
    /// close other objects. It is recommended to close other objects before closing the library
    /// with this function.
    #[allow(non_snake_case)]
    fn FPDF_DestroyLibrary(&self);

    /// Sets the policy for the sandbox environment.
    ///
    ///    `policy` -   The specified policy for setting, for example `FPDF_POLICY_MACHINETIME_ACCESS`.
    ///
    ///    `enable` -   `true` to enable, `false` to disable the policy.
    #[allow(non_snake_case)]
    fn FPDF_SetSandBoxPolicy(&self, policy: FPDF_DWORD, enable: FPDF_BOOL);

    /// Gets the last error code when a function fails.
    ///
    /// Returns a 32-bit integer indicating error code as defined above.
    ///
    /// If the previous SDK call succeeded, the return value of this function is not defined.
    /// This function only works in conjunction with APIs that mention `FPDF_GetLastError`
    /// in their documentation.
    #[allow(non_snake_case)]
    fn FPDF_GetLastError(&self) -> c_ulong;

    /// Coalesces the given individual R, G, B, and alpha color components into
    // a 32-bit hexadecimal 0xAARRGGBB value, suitable for passing to Pdfium.
    #[allow(non_snake_case)]
    fn FPDF_ARGB(&self, a: u8, r: u8, g: u8, b: u8) -> FPDF_DWORD {
        PdfColor::new(r, g, b, a).as_pdfium_color()
    }

    /// Returns the blue component of the given color.
    #[allow(non_snake_case)]
    fn FPDF_GetBValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).blue()
    }

    /// Returns the green component of the given color.
    #[allow(non_snake_case)]
    fn FPDF_GetGValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).green()
    }

    /// Returns the red component of the given color.
    #[allow(non_snake_case)]
    fn FPDF_GetRValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).red()
    }

    /// Returns the alpha component of the given color.
    #[allow(non_snake_case)]
    fn FPDF_GetAValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).alpha()
    }

    /// Creates a new empty PDF document. Returns a handle to a new document, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDF_CreateNewDocument(&self) -> FPDF_DOCUMENT;

    #[cfg(not(target_arch = "wasm32"))]
    /// Opens and loads an existing PDF document.
    ///
    ///    `file_path` -  Path to the PDF file (including extension).
    ///
    ///    `password`  -  A string used as the password for the PDF file.
    ///                   If no password is needed, empty or `NULL` can be used.
    ///                   See comments below regarding the encoding.
    ///
    /// Returns a handle to the loaded document, or `NULL` on failure.
    ///
    /// The loaded document can be closed by [PdfiumLibraryBindings::FPDF_CloseDocument].
    /// If this function fails, you can use [PdfiumLibraryBindings::FPDF_GetLastError] to retrieve
    /// the reason why it failed. The encoding for `file_path` is UTF-8. The encoding for
    /// `password` can be either UTF-8 or Latin-1. PDFs, depending on the security handler
    /// revision, will only accept one or the other encoding. If `password`'s encoding and
    /// the PDF's expected encoding do not match, `FPDF_LoadDocument` will automatically
    /// convert `password` to the other encoding.
    ///
    /// This function is not available when compiling to WASM. You must use one of the
    /// [PdfiumLibraryBindings::FPDF_LoadMemDocument], [PdfiumLibraryBindings::FPDF_LoadMemDocument64],
    /// or [PdfiumLibraryBindings::FPDF_LoadCustomDocument] functions instead.
    #[allow(non_snake_case)]
    fn FPDF_LoadDocument(&self, file_path: &str, password: Option<&str>) -> FPDF_DOCUMENT;

    /// Opens and loads an existing PDF document from memory.
    ///
    ///    `data_buf`    -   Pointer to a buffer containing the PDF document.
    ///
    ///    `size`        -   Number of bytes in the PDF document.
    ///
    ///    `password`    -   A string used as the password for the PDF file.
    ///                      If no password is needed, empty or NULL can be used.
    ///
    /// Returns a handle to the loaded document, or `NULL` on failure.
    ///
    /// The memory buffer must remain valid when the document is open. The loaded document
    /// can be closed by [PdfiumLibraryBindings::FPDF_CloseDocument]. If this function fails,
    /// you can use [PdfiumLibraryBindings::FPDF_GetLastError] to retrieve the reason why it failed.
    ///
    /// See the comments for [PdfiumLibraryBindings::FPDF_LoadDocument] regarding the encoding for
    /// `password`.
    ///
    /// If PDFium is built with the XFA module, the application should call [PdfiumLibraryBindings::FPDF_LoadXFA]
    /// function after the PDF document is loaded to support XFA fields defined in the `fpdfformfill.h` file.
    ///
    /// Note that all calls to [PdfiumLibraryBindings::FPDF_LoadMemDocument] are
    /// internally upgraded to [PdfiumLibraryBindings::FPDF_LoadMemDocument64] by `pdfium-render`.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument(&self, bytes: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        self.FPDF_LoadMemDocument64(bytes, password)
    }

    /// Opens and loads an existing PDF document from memory.
    ///
    ///    `data_buf`    -   Pointer to a buffer containing the PDF document.
    ///
    ///    `size`        -   Number of bytes in the PDF document.
    ///
    ///    `password`    -   A string used as the password for the PDF file.
    ///                      If no password is needed, empty or `NULL` can be used.
    ///
    /// Returns a handle to the loaded document, or `NULL` on failure.
    ///
    /// The memory buffer must remain valid when the document is open. The loaded document
    /// can be closed by [PdfiumLibraryBindings::FPDF_CloseDocument]. If this function fails,
    /// you can use [PdfiumLibraryBindings::FPDF_GetLastError] to retrieve the reason why it failed.
    ///
    /// See the comments for [PdfiumLibraryBindings::FPDF_LoadDocument] regarding the encoding for
    /// `password`.
    ///
    /// If PDFium is built with the XFA module, the application should call [PdfiumLibraryBindings::FPDF_LoadXFA]
    /// function after the PDF document loaded to support XFA fields defined in the `fpdfformfill.h` file.
    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument64(&self, data_buf: &[u8], password: Option<&str>) -> FPDF_DOCUMENT;

    /// Loads a PDF document from a custom access descriptor.
    ///
    ///    `pFileAccess` -   A structure for accessing the file.
    ///
    ///    `password`    -   Optional password for decrypting the PDF file.
    ///
    /// Returns a handle to the loaded document, or `NULL` on failure.
    ///
    /// The application must keep the file resources `pFileAccess` points to valid until the
    /// returned `FPDF_DOCUMENT` is closed. `pFileAccess` itself does not need to outlive the
    /// `FPDF_DOCUMENT`. The loaded document can be closed with [PdfiumLibraryBindings::FPDF_CloseDocument].
    ///
    /// See the comments for [PdfiumLibraryBindings::FPDF_LoadDocument] regarding the encoding for
    /// `password`.
    ///
    /// If PDFium is built with the XFA module, the application should call [PdfiumLibraryBindings::FPDF_LoadXFA]
    /// function after the PDF document loaded to support XFA fields defined in the `fpdfformfill.h` file.
    #[allow(non_snake_case)]
    fn FPDF_LoadCustomDocument(&self, pFileAccess: *mut FPDF_FILEACCESS, password: Option<&str>) -> FPDF_DOCUMENT;

    /// Saves a copy of the specified document in a custom way.
    ///
    ///    `document`        -   Handle to document, as returned by [PdfiumLibraryBindings::FPDF_LoadDocument]
    ///                          or [PdfiumLibraryBindings::FPDF_CreateNewDocument].
    ///
    ///    `pFileWrite`      -   A pointer to a custom file write structure.
    ///
    ///    `flags`           -   The creating flags.
    ///
    /// Returns `true` on success, `false` on failure.
    #[allow(non_snake_case)]
    fn FPDF_SaveAsCopy(&self, document: FPDF_DOCUMENT, pFileWrite: *mut FPDF_FILEWRITE, flags: FPDF_DWORD)
    -> FPDF_BOOL;

    /// Same as [PdfiumLibraryBindings::FPDF_SaveAsCopy], except the file version of the
    /// saved document can be specified by the caller.
    ///
    ///    `document`        -   Handle to document.
    ///
    ///    `pFileWrite`      -   A pointer to a custom file write structure.
    ///
    ///    `flags`           -   The creating flags.
    ///
    ///    `fileVersion`     -   The PDF file version. File version: 14 for 1.4,
    ///                          15 for 1.5, etc.
    ///
    /// Returns `true` on success, `false` on failure.
    #[allow(non_snake_case)]
    fn FPDF_SaveWithVersion(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
        fileVersion: c_int,
    ) -> FPDF_BOOL;

    /// Creates a document availability provider.
    ///
    ///   `file_avail` - pointer to file availability interface.
    ///
    ///   `file`       - pointer to a file access interface.
    ///
    /// Returns a handle to the document availability provider, or `NULL` on error.
    ///
    /// [PdfiumLibraryBindings::FPDFAvail_Destroy] must be called when done with the
    /// availability provider.
    #[allow(non_snake_case)]
    fn FPDFAvail_Create(&self, file_avail: *mut FX_FILEAVAIL, file: *mut FPDF_FILEACCESS) -> FPDF_AVAIL;

    /// Destroys the `avail` document availability provider.
    ///
    ///   `avail` - handle to document availability provider to be destroyed.
    #[allow(non_snake_case)]
    fn FPDFAvail_Destroy(&self, avail: FPDF_AVAIL);

    /// Checks if the document is ready for loading; if not, gets download hints.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    ///   `hints` - pointer to a download hints interface.
    ///
    /// Returns one of:
    ///
    ///   `PDF_DATA_ERROR`: A common error is returned. Data availability unknown.
    ///
    ///   `PDF_DATA_NOTAVAIL`: Data not yet available.
    ///
    ///   `PDF_DATA_AVAIL`: Data available.
    ///
    /// Applications should call this function whenever new data arrives, and process
    /// all the generated download hints, if any, until the function returns
    /// `PDF_DATA_ERROR` or `PDF_DATA_AVAIL`.
    ///
    /// If `hints` is `NULL`, the function just checks current document availability.
    ///
    /// Once all data is available, call [PdfiumLibraryBindings::FPDFAvail_GetDocument] to get
    /// a document handle.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsDocAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int;

    /// Gets a document from the availability provider.
    ///
    ///   `avail`    - handle to document availability provider.
    ///
    ///   `password` - password for decrypting the PDF file. Optional.
    ///
    /// Returns a handle to the document.
    ///
    /// When [PdfiumLibraryBindings::FPDFAvail_IsDocAvail] returns `TRUE`, call
    /// [PdfiumLibraryBindings::FPDFAvail_GetDocument] to\n retrieve the document handle.
    /// See the comments for [PdfiumLibraryBindings::FPDF_LoadDocument] regarding the encoding
    /// for `password`.
    #[allow(non_snake_case)]
    fn FPDFAvail_GetDocument(&self, avail: FPDF_AVAIL, password: Option<&str>) -> FPDF_DOCUMENT;

    /// Gets the page number for the first available page in a linearized PDF.
    ///
    ///   `doc` - document handle.
    ///
    /// Returns the zero-based index for the first available page.
    ///
    /// For most linearized PDFs, the first available page will be the first page,
    /// however, some PDFs might make another page the first available page.
    ///
    /// For non-linearized PDFs, this function will always return zero.
    #[allow(non_snake_case)]
    fn FPDFAvail_GetFirstPageNum(&self, doc: FPDF_DOCUMENT) -> c_int;

    /// Checks if `page_index` is ready for loading, if not, get the `FX_DOWNLOADHINTS`.
    ///
    ///   `avail`      - handle to document availability provider.
    ///
    ///   `page_index` - index number of the page. Zero for the first page.
    ///
    ///   `hints`      - pointer to a download hints interface. Populated if
    ///                  `page_index` is not available.
    ///
    /// Returns one of:
    ///
    ///   `PDF_DATA_ERROR`: A common error is returned. Data availability unknown.
    ///
    ///   `PDF_DATA_NOTAVAIL`: Data not yet available.
    ///
    ///   `PDF_DATA_AVAIL`: Data available.
    ///
    /// This function can be called only after [PdfiumLibraryBindings::FPDFAvail_GetDocument]
    /// is called. Applications should call this function whenever new data arrives and process
    /// all the generated download `hints`, if any, until this function returns `PDF_DATA_ERROR`
    /// or `PDF_DATA_AVAIL`. Applications can then perform page loading.
    ///
    /// If `hints` is `NULL`, the function just check current availability of specified page.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsPageAvail(&self, avail: FPDF_AVAIL, page_index: c_int, hints: *mut FX_DOWNLOADHINTS) -> c_int;

    /// Checks if form data is ready for initialization; if not, get the `FX_DOWNLOADHINTS`.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    ///   `hints` - pointer to a download hints interface. Populated if form is not
    ///             ready for initialization.
    ///
    /// Returns one of:
    ///
    ///   `PDF_FORM_ERROR`: A common error, in general incorrect parameters.
    ///
    ///   `PDF_FORM_NOTAVAIL`: Data not available.
    ///
    ///   `PDF_FORM_AVAIL`: Data available.
    ///
    ///   `PDF_FORM_NOTEXIST`: No form data.
    ///
    /// This function can be called only after [PdfiumLibraryBindings::FPDFAvail_GetDocument]
    /// is called. The application should call this function whenever new data arrives and
    /// process all the generated download `hints`, if any, until the function returns
    /// `PDF_FORM_ERROR`, `PDF_FORM_AVAIL` or `PDF_FORM_NOTEXIST`.
    ///
    /// If `hints` is `NULL`, the function just check current form availability.
    ///
    /// Applications can then perform page loading. It is recommend to call
    /// [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment] when `PDF_FORM_AVAIL` is returned.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsFormAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int;

    /// Checks whether a document is a linearized PDF.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    /// Returns one of:
    ///
    ///   `PDF_LINEARIZED`
    ///
    ///   `PDF_NOT_LINEARIZED`
    ///
    ///   `PDF_LINEARIZATION_UNKNOWN`
    ///
    /// [PdfiumLibraryBindings::FPDFAvail_IsLinearized] will return `PDF_LINEARIZED` or
    /// `PDF_NOT_LINEARIZED` once we have received 1kb of data. If the file's size is less
    /// than 1kb, it returns `PDF_LINEARIZATION_UNKNOWN` as there is insufficient information
    // to determine if the PDF is linearlized.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsLinearized(&self, avail: FPDF_AVAIL) -> c_int;

    /// Closes a loaded PDF page.
    ///
    ///    `page`        -   Handle to the loaded page.
    #[allow(non_snake_case)]
    fn FPDF_ClosePage(&self, page: FPDF_PAGE);

    /// Closes a loaded PDF document.
    ///
    ///    `document`    -   Handle to the loaded document.
    #[allow(non_snake_case)]
    fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT);

    /// Converts the screen coordinates of a point to page coordinates.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`     -   Left pixel position of the display area in device coordinates.
    ///
    ///    `start_y`     -   Top pixel position of the display area in device coordinates.
    ///
    ///    `size_x`      -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`      -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`      -   Page orientation:
    ///                            0 (normal)
    ///                            1 (rotated 90 degrees clockwise)
    ///                            2 (rotated 180 degrees)
    ///                            3 (rotated 90 degrees counter-clockwise)
    ///
    ///    `device_x`    -   X value in device coordinates to be converted.
    ///
    ///    `device_y`    -   Y value in device coordinates to be converted.
    ///
    ///    `page_x`      -   A pointer to a double receiving the converted X
    ///                       value in page coordinates.
    ///
    ///    `page_y`      -   A pointer to a double receiving the converted Y
    ///                       value in page coordinates.
    ///
    /// Returns `true` if the conversion succeeds, and `page_x` and `page_y`
    /// successfully receives the converted coordinates.
    ///
    /// The page coordinate system has its origin at the left-bottom corner of the page,
    /// with the X-axis on the bottom going to the right, and the Y-axis on the left side going up.
    /// This coordinate system can be altered when you zoom, scroll, or rotate a page; however,
    /// a point on the page should always have the same coordinate values in the page coordinate
    /// system. The device coordinate system is device dependent. For screen device, its origin
    /// is at the left-top corner of the window. This origin can be altered by the Windows coordinate
    /// transformation utilities.
    ///
    /// You must make sure the `start_x`, `start_y`, `size_x`, `size_y` and `rotate` parameters
    /// have exactly same values as you used in the [PdfiumLibraryBindings::FPDF_RenderPage] function call.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_DeviceToPage(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> FPDF_BOOL;

    /// Converts the page coordinates of a point to screen coordinates.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`     -   Left pixel position of the display area in device coordinates.
    ///
    ///    `start_y`     -   Top pixel position of the display area in device coordinates.
    ///
    ///    `size_x`      -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`      -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`      -   Page orientation:
    ///                            0 (normal)
    ///                            1 (rotated 90 degrees clockwise)
    ///                            2 (rotated 180 degrees)
    ///                            3 (rotated 90 degrees counter-clockwise)
    ///
    ///    `page_x`      -   X value in page coordinates.
    ///
    ///    `page_y`      -   Y value in page coordinate.
    ///
    ///    `device_x`    -   A pointer to an integer receiving the result X value in device coordinates.
    ///
    ///    `device_y`    -   A pointer to an integer receiving the result Y value in device coordinates.
    ///
    /// Returns `true` if the conversion succeeds, and `device_x` and `device_y`
    /// successfully receives the converted coordinates.
    ///
    /// Refer to [PdfiumLibraryBindings::FPDF_DeviceToPage] for comments on coordinate systems.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_PageToDevice(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> FPDF_BOOL;

    /// Gets the file version of the given PDF document.
    ///
    ///    `doc`         -   Handle to a document.
    ///
    ///    `fileVersion` -   The PDF file version. File version: 14 for 1.4, 15
    ///                      for 1.5, etc.
    ///
    /// Returns `true` on success, `false` on failure.
    ///
    /// If the document was created by [PdfiumLibraryBindings::FPDF_CreateNewDocument],
    /// then this function will always fail.
    #[allow(non_snake_case)]
    fn FPDF_GetFileVersion(&self, doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL;

    /// Returns whether the document's cross reference table is valid or not.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns `true` if the PDF parser did not encounter problems parsing the cross reference table,
    /// or `false` if the parser could not parse the cross reference table and the table had to be
    /// rebuilt from other data within the document.
    ///
    /// The return value may change over time as the PDF parser evolves.
    #[allow(non_snake_case)]
    fn FPDF_DocumentHasValidCrossReferenceTable(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    /// Gets the byte offsets of trailer ends.
    ///
    ///    `document`    -   Handle to document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `buffer`      -   The address of a buffer that receives the byte offsets.
    ///
    ///    `length`      -   The size, in ints, of `buffer`.
    ///
    /// Returns the number of ints in the buffer on success, 0 on error.
    ///
    /// `buffer` is an array of integers that describes the exact byte offsets of the
    /// trailer ends in the document. If `length` is less than the returned length,
    /// or `document` or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_GetTrailerEnds(&self, document: FPDF_DOCUMENT, buffer: *mut c_uint, length: c_ulong) -> c_ulong;

    /// Gets the file permission flags of the document.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns a 32-bit integer indicating permission flags. Please refer to the PDF Reference
    /// for detailed descriptions. If the document is not protected or was unlocked
    /// by the owner, `0xffffffff` will be returned.
    #[allow(non_snake_case)]
    fn FPDF_GetDocPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong;

    /// Gets user file permission flags of the document.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns a 32-bit integer indicating permission flags. Please refer to the PDF Reference
    /// for detailed descriptions. If the document is not protected, `0xffffffff` will be returned.
    /// Always returns user permissions, even if the document was unlocked by the owner.
    #[allow(non_snake_case)]
    fn FPDF_GetDocUserPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong;

    /// Gets the revision for the security handler.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns the security handler revision number. Please refer to the PDF Reference
    /// for a detailed description. If the document is not protected, `-1` will be returned.
    #[allow(non_snake_case)]
    fn FPDF_GetSecurityHandlerRevision(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Gets the total number of pages in the document.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns the total number of pages in the document."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Loads a page inside the document.
    ///
    ///    `document`    -   Handle to a document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `page_index`  -   Index number of the page. `0` for the first page.
    ///
    /// Returns a handle to the loaded page, or `NULL` if page load fails.
    ///
    /// The loaded page can be rendered to devices using [PdfiumLibraryBindings::FPDF_RenderPage].
    /// The loaded page can be closed using [PdfiumLibraryBindings::FPDF_ClosePage].
    #[allow(non_snake_case)]
    fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE;

    /// Starts rendering page contents to a device independent bitmap progressively with a
    /// specified color scheme for the content.
    ///
    ///    `bitmap`       -   Handle to the device independent bitmap (as the
    ///                       output buffer). Bitmap handle can be created by
    ///                       [PdfiumLibraryBindings::FPDFBitmap_Create] function.
    ///
    ///    `page`         -   Handle to the page as returned by [PdfiumLibraryBindings::FPDF_LoadPage]
    ///                       function.
    ///
    ///    `start_x`      -   Left pixel position of the display area in the bitmap coordinate.
    ///
    ///    `start_y`      -   Top pixel position of the display area in the bitmap coordinate.
    ///
    ///    `size_x`       -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`       -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`       -   Page orientation: 0 (normal), 1 (rotated 90 degrees clockwise),
    ///                       2 (rotated 180 degrees), 3 (rotated 90 degrees counter-clockwise).
    ///
    ///    `flags`        -   0 for normal display, or combination of flags defined in `fpdfview.h`.
    ///                       With `FPDF_ANNOT` flag, it renders all annotations that does not require
    ///                       user-interaction, which are all annotations except widget and popup
    ///                       annotations.
    ///
    ///    `color_scheme` -   Color scheme to be used in rendering the `page`.
    ///                       If `NULL`, this function will work similar to [PdfiumLibraryBindings::FPDF_RenderPageBitmap_Start].
    ///
    ///    `pause`        -   The `IFSDK_PAUSE` interface, a callback mechanism allowing the
    ///                       page progressive rendering process to be paused.
    ///
    /// Returns the rendering status. See flags for progressive process status for details.
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithColorScheme_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        color_scheme: *const FPDF_COLORSCHEME,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int;

    /// Starts rendering page contents to a device independent bitmap progressively.
    ///
    ///    `bitmap`      -   Handle to the device independent bitmap (as the
    ///                      output buffer). Bitmap handle can be created by
    ///                      [PdfiumLibraryBindings::FPDFBitmap_Create].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`     -   Left pixel position of the display area in the
    ///                      bitmap coordinates.
    ///
    ///    `start_y`     -   Top pixel position of the display area in the bitmap coordinates.
    ///
    ///    `size_x`      -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`      -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`      -   Page orientation: 0 (normal), 1 (rotated 90 degrees clockwise),
    ///                      2 (rotated 180 degrees), 3 (rotated 90 degrees counter-clockwise).
    ///
    ///    `flags`       -   0 for normal display, or combination of flags defined in `fpdfview.h`.
    ///                      With `FPDF_ANNOT` flag, it renders all annotations that does not require
    ///                      user-interaction, which are all annotations except widget and popup annotations.
    ///
    ///    `pause`       -   The `IFSDK_PAUSE` interface, a callback mechanism allowing the
    ///                      page rendering process to be paused.
    ///
    /// Returns the rendering status. See flags for progressive process status for details.
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int;

    /// Continues rendering a PDF page.
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage()].
    ///
    ///    `pause`       -   The `IFSDK_PAUSE` interface, a callback mechanism allowing
    ///                      the page rendering process to be paused before it's finished.
    ///                      This can be `NULL` if you don't want to pause.
    ///
    /// Returns the rendering status. See flags for progressive process status for details.
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Continue(&self, page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int;

    /// Releases the resource allocate during page rendering. Needs to be called after finishing
    /// rendering or after cancelling the rendering.
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage()].
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Close(&self, page: FPDF_PAGE);

    /// Imports pages into a `FPDF_DOCUMENT`.
    ///
    ///    `dest_doc`     - The destination document for the pages.
    ///
    ///    `src_doc`      - The document to be imported.
    ///
    ///    `page_indices` - An array of page indices to be imported. The first page index is
    ///                     zero. If `page_indices` is `NULL`, all pages from `src_doc`
    ///                     are imported.
    ///
    ///    `length`       - The length of the `page_indices` array.
    ///
    ///    `index`        - The page index at which to insert the first imported page
    ///                     into `dest_doc`. The first page index is zero.
    ///
    /// Returns `true` on success. Returns `false` if any pages in `page_indices` are invalid.
    ///
    /// A [vec]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDF_ImportPagesByIndex_vec].
    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: *const c_int,
        length: c_ulong,
        index: c_int,
    ) -> FPDF_BOOL;

    /// A [vec]-friendly helper function for [PdfiumLibraryBindings::FPDF_ImportPagesByIndex].
    ///
    /// Imports pages into a `FPDF_DOCUMENT`.
    ///
    ///    `dest_doc`     - The destination document for the pages.
    ///
    ///    `src_doc`      - The document to be imported.
    ///
    ///    `page_indices` - A [vec] of page indices to be imported. The first page index is zero.
    ///
    ///    `index`        - The page index at which to insert the first imported page
    ///                     into `dest_doc`. The first page index is zero.
    ///
    /// Returns `true` on success. Returns `false` if any pages in `page_indices` are invalid.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex_vec(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: Vec<c_int>,
        index: c_int,
    ) -> FPDF_BOOL {
        self.FPDF_ImportPagesByIndex(
            dest_doc,
            src_doc,
            page_indices.as_ptr(),
            page_indices.len() as c_ulong,
            index,
        )
    }

    /// Imports pages into a `FPDF_DOCUMENT`.
    ///
    ///    `dest_doc`  - The destination document for the pages.
    ///
    ///    `src_doc`   - The document to be imported.
    ///
    ///    `pagerange` - A page range string, such as "1,3,5-7". The first page index is one.
    ///                  If `pagerange` is `NULL`, all pages from `src_doc` are imported.
    ///
    ///    `index`     - The page index at which to insert the first imported page into
    ///                  `dest_doc`. The first page index is zero.
    ///
    /// Returns `true` on success. Returns `false` if any pages in `pagerange` is invalid
    /// or if `pagerange` cannot be read.
    #[allow(non_snake_case)]
    fn FPDF_ImportPages(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        pagerange: &str,
        index: c_int,
    ) -> FPDF_BOOL;

    /// Creates a new document from `src_doc`. The pages of `src_doc` will be combined to provide
    /// `num_pages_on_x_axis`, `num_pages_on_y_axis` pages per `output_doc` page.
    ///
    ///    `src_doc`             - The document to be imported.
    ///
    ///    `output_width`        - The output page width in PDF "user space" units.
    ///
    ///    `output_height`       - The output page height in PDF "user space" units.
    ///
    ///    `num_pages_on_x_axis` - The number of pages on X Axis.
    ///
    ///    `num_pages_on_y_axis` - The number of pages on Y Axis.
    ///
    /// Returns a handle to the created document, or `NULL` on failure.
    ///
    /// The total number of pages per page = num_pages_on_x_axis * num_pages_on_y_axis.
    #[allow(non_snake_case)]
    fn FPDF_ImportNPagesToOne(
        &self,
        src_doc: FPDF_DOCUMENT,
        output_width: c_float,
        output_height: c_float,
        num_pages_on_x_axis: size_t,
        num_pages_on_y_axis: size_t,
    ) -> FPDF_DOCUMENT;

    /// Creates a template to generate form xobjects from `src_doc`'s page at `src_page_index`,
    /// for use in `dest_doc`.
    ///
    /// Returns a handle on success, or `NULL` on failure. Caller owns the newly created object.
    #[allow(non_snake_case)]
    fn FPDF_NewXObjectFromPage(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        src_page_index: c_int,
    ) -> FPDF_XOBJECT;

    /// Closes an `FPDF_XOBJECT` handle created by [PdfiumLibraryBindings::FPDF_NewXObjectFromPage].
    /// `FPDF_PAGEOBJECT`s created from the `FPDF_XOBJECT` handle are not affected.
    #[allow(non_snake_case)]
    fn FPDF_CloseXObject(&self, xobject: FPDF_XOBJECT);

    /// Creates a new form object from an `FPDF_XOBJECT` object.
    ///
    /// Returns a new form object on success, or `NULL` on failure. Caller owns the newly created object.
    #[allow(non_snake_case)]
    fn FPDF_NewFormObjectFromXObject(&self, xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT;

    /// Copies the viewer preferences from `src_doc` into `dest_doc`.
    ///
    ///    `dest_doc` - Document to write the viewer preferences into.
    ///
    ///    `src_doc`  - Document to read the viewer preferences from.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDF_CopyViewerPreferences(&self, dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT) -> FPDF_BOOL;

    /// Gets page width.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns the page width (excluding non-displayable area) measured in points.
    /// One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Note that changing the rotation of the page affects the return value.
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidthF(&self, page: FPDF_PAGE) -> c_float;

    /// Gets page width.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns the page width (excluding non-displayable area) measured in points.
    /// One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Note: prefer the [PdfiumLibraryBindings::FPDF_GetPageWidthF] function above.
    /// This function will be deprecated in the future.
    ///
    /// Note that changing the rotation of the page affects the return value.
    #[deprecated(
        since = "0.8.25",
        note = "Prefer FPDF_GetPageWidthF() over FPDF_GetPageWidth(). FPDF_GetPageWidth() is deprecated and will likely be removed in a future version of Pdfium."
    )]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidth(&self, page: FPDF_PAGE) -> f64;

    /// Gets page height.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns the page height (excluding non-displayable area) measured in points.
    /// One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Note that changing the rotation of the page affects the return value.
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeightF(&self, page: FPDF_PAGE) -> c_float;

    /// Gets page height.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns the page height (excluding non-displayable area) measured in points.
    /// One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Note: prefer the [PdfiumLibraryBindings::FPDF_GetPageHeightF] function above.
    /// This function will be deprecated in the future.
    ///
    /// Note that changing the rotation of the page affects the return value.
    #[deprecated(
        since = "0.8.25",
        note = "Prefer FPDF_GetPageHeightF() over FPDF_GetPageHeight(). FPDF_GetPageHeight() is deprecated and will likely be removed in a future version of Pdfium."
    )]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeight(&self, page: FPDF_PAGE) -> f64;

    /// Gets the character index in the `text_page` internal character list.
    ///
    ///    `text_page`  - a text page information structure.
    ///
    ///    `nTextIndex` - index of the text returned from [PdfiumLibraryBindings::FPDFText_GetText].
    ///
    /// Returns the index of the character in the internal character list, or `-1` for error.
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexFromTextIndex(&self, text_page: FPDF_TEXTPAGE, nTextIndex: c_int) -> c_int;

    /// Gets the text index in the `text_page` internal character list.
    ///
    ///    `text_page`  - a text page information structure.
    ///
    ///    `nCharIndex` - index of the character in internal character list.
    ///
    /// Returns the index of the text returned from [PdfiumLibraryBindings::FPDFText_GetText],
    /// or `-1` for error.
    #[allow(non_snake_case)]
    fn FPDFText_GetTextIndexFromCharIndex(&self, text_page: FPDF_TEXTPAGE, nCharIndex: c_int) -> c_int;

    /// Gets the total number of signatures in the document.
    ///
    ///    `document`    -   Handle to document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns the total number of signatures in the document on success, or `-1` on error.
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureCount(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Gets the nth signature in the document.
    ///
    ///    `document`    -   Handle to document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `index`       -   Index into the array of signatures of the document.
    ///
    /// Returns the handle to the signature, or `NULL` on failure. The caller
    /// does not take ownership of the returned `FPDF_SIGNATURE`. Instead, it
    /// remains valid until [PdfiumLibraryBindings::FPDF_CloseDocument] is called for the document.
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureObject(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE;

    /// Gets the contents of a signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    ///    `buffer`      -   The address of a buffer that receives the contents.
    ///
    ///    `length`      -   The size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the contents on success, or `0` on error.
    /// For public-key signatures, `buffer` is either a DER-encoded PKCS#1 binary or
    /// a DER-encoded PKCS#7 binary. If `length` is less than the returned length, or
    /// `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetContents(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong;

    /// Gets the byte range of a signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    ///    `buffer`      -   The address of a buffer that receives the byte range.
    ///
    ///    `length`      -   The size, in `int`s, of `buffer`.
    ///
    /// Returns the number of `int`s in the byte range on success, or `0` on error.
    /// `buffer` is an array of pairs of integers (starting byte offset, length in bytes)
    /// that describes the exact byte range for the digest calculation. If `length` is
    /// less than the returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetByteRange(&self, signature: FPDF_SIGNATURE, buffer: *mut c_int, length: c_ulong) -> c_ulong;

    /// Gets the encoding of the value of a signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    ///    `buffer`      -   The address of a buffer that receives the encoding.
    ///
    ///    `length`      -   The size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the encoding name (including the trailing `NUL` character)
    /// on success, or `0` on error. The `buffer` is always encoded in 7-bit ASCII.
    /// If `length` is less than the returned length, or `buffer` is `NULL`, `buffer` will
    // not be modified.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetSubFilter(&self, signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong)
    -> c_ulong;

    /// Gets the reason (comment) of the signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    ///    `buffer`      -   The address of a buffer that receives the reason.
    ///
    ///    `length`      -   The size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the reason on success, or `0` on error.
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding. The
    /// string is terminated by a UTF16 `NUL` character. If `length` is less than the
    /// returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetReason(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong;

    /// Gets the time of signing of a signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    ///    `buffer`      -   The address of a buffer that receives the time.
    ///
    ///    `length`      -   The size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the encoding name (including the
    /// trailing `NUL` character) on success, or `0` on error.
    /// The `buffer` is always encoded in 7-bit ASCII. If `length` is less than the
    /// returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    /// The format of time is expected to be `D:YYYYMMDDHHMMSS+XX'YY'`, i.e. its
    /// precision is seconds, with timezone information. This value should be used
    /// only when the time of signing is not available in the PKCS#7 binary signature.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetTime(&self, signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong;

    /// Gets the DocMDP (modification detection and prevention) permission of a signature object.
    ///
    ///    `signature`   -   Handle to the signature object. Returned by
    ///                      [PdfiumLibraryBindings::FPDF_GetSignatureObject].
    ///
    /// Returns the permission (`1`, `2`, or `3`) on success, or `0` on error.
    ///
    ///    `1`           -   No changes to the document are permitted; any change
    ///                      to the document invalidates the signature.
    ///
    ///    `2`           -   Permitted changes are filling in forms, instantiating page
    ///                      templates, and signing; other changes invalidate the signature.
    ///
    ///    `3`           -   Permitted changes are the same as for level 2, as well as
    ///                      annotation creation, deletion, and modification; other changes
    ///                      invalidate the signature.
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetDocMDPPermission(&self, signature: FPDF_SIGNATURE) -> c_uint;

    /// Gets the structure tree for a page.
    ///
    ///   `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Return value: a handle to the structure tree, or `NULL` on error. The caller owns the
    /// returned handle and must use [PdfiumLibraryBindings::FPDF_StructTree_Close] to release it.
    ///
    /// The handle should be released before `page` is released.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetForPage(&self, page: FPDF_PAGE) -> FPDF_STRUCTTREE;

    /// Releases a resource allocated by [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    #[allow(non_snake_case)]
    fn FPDF_StructTree_Close(&self, struct_tree: FPDF_STRUCTTREE);

    /// Counts the number of children for the structure tree.
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    /// Return value: the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_CountChildren(&self, struct_tree: FPDF_STRUCTTREE) -> c_int;

    /// Gets a child in the structure tree.
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    ///   `index`       -   The index for the child, 0-based.
    ///
    /// Return value: the child at the n-th index or `NULL` on error. The caller does not
    /// own the handle. The handle remains valid as long as `struct_tree` remains valid.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructTree_CountChildren] return value.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetChildAtIndex(&self, struct_tree: FPDF_STRUCTTREE, index: c_int) -> FPDF_STRUCTELEMENT;

    /// Gets the alt text for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the alt text. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the alt text, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAltText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the actual text for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the actual text. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the actual text, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetActualText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the ID for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the ID string. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the ID string, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the case-insensitive IETF BCP 47 language code for an element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the lang string. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the ID string, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetLang(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets a struct element attribute of type `name` or `string`.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `attr_name`      -   The name of the attribute to retrieve.
    ///
    ///   `buffer`         -   A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the attribute value, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetStringAttribute(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        attr_name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the marked content ID for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the marked content ID of the element. If no ID exists, returns 1.
    ///
    /// [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdAtIndex] may be able to
    /// extract more marked content IDs out of `struct_element`. This API may be deprecated
    /// in the future.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets the type (/S) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the type, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the object type (/Type) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the object type, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetObjType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the title (/T) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the title, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetTitle(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Counts the number of children for the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_CountChildren(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets a child in the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the child, 0-based.
    ///
    /// Returns the child at the n-th index, or `NULL` on error.
    ///
    /// If the child exists but is not an element, then this function will return `NULL`.
    /// This will also return `NULL` for out-of-bounds indices.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT;

    /// Gets the child's content id.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the child, 0-based.
    ///
    /// Returns the marked content ID of the child. If no ID exists, returns -1.
    ///
    /// If the child exists but is not a stream or object, then this function will return -1.
    /// This will also return -1 for out of bounds indices. Compared to
    /// [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdAtIndex],
    /// it is scoped to the current page.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int;

    /// Gets the parent of the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the parent structure element, or `NULL` on error.
    ///
    /// If structure element is StructTreeRoot, then this function will return `NULL`.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetParent(&self, struct_element: FPDF_STRUCTELEMENT) -> FPDF_STRUCTELEMENT;

    /// Counts the number of attributes for the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the number of attributes, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets an attribute object in the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the attribute object, 0-based.
    ///
    /// Returns the attribute object at the n-th index, or `NULL` on error.
    ///
    /// If the attribute object exists but is not a dict, then this function will return `NULL`.
    /// This will also return `NULL` for out-of-bounds indices. The caller does not own the handle.
    /// The handle remains valid as long as `struct_element` remains valid.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_GetAttributeCount]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR;

    /// Counts the number of attributes in a structure element attribute map.
    ///
    ///   `struct_attribute` - Handle to the struct element attribute.
    ///
    /// Returns the number of attributes, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetCount(&self, struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int;

    /// Gets the name of an attribute in a structure element attribute map.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `index`              - The index of attribute in the map.
    ///
    ///   `buffer`             - A buffer for output. May be `NULL`. This is only
    ///                          modified if `buflen` is longer than the length
    ///                          of the key. Optional, pass `NULL` to just
    ///                          retrieve the size of the buffer needed.
    ///
    ///   `buflen`             - The length of the buffer.
    ///
    ///   `out_buflen`         - A pointer to variable that will receive the
    ///                          minimum buffer size to contain the key. Not
    ///                          filled if `FALSE` is returned.
    ///
    /// Returns `true` if the operation was successful.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetName(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Gets a handle to a value for an attribute in a structure element attribute map.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    /// Returns a handle to the value associated with the input, if any. Returns `NULL`
    /// on failure. The caller does not own the handle.
    ///
    /// The handle remains valid as long as `struct_attribute` remains valid.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE;

    /// Gets the type of an attribute in a structure element attribute map.
    ///
    ///   `value` - Handle to the value.
    ///
    /// Returns the type of the value, or `FPDF_OBJECT_UNKNOWN` in case of failure. Note that
    /// this will never return `FPDF_OBJECT_REFERENCE`, as references are always dereferenced.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> FPDF_OBJECT_TYPE;

    /// Gets the value of a boolean attribute in an attribute map as `FPDF_BOOL`.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_BOOLEAN` for this property.
    ///
    ///   `value`     - Handle to the value.
    ///
    ///   `out_value` - A pointer to variable that will receive the value. Not
    ///                 filled if false is returned.
    ///
    /// Returns `true` if the attribute maps to a boolean value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL;

    /// Gets the value of a number attribute in an attribute map as float.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_NUMBER` for this property.
    ///
    ///   `value`     - Handle to the value.
    ///
    ///   `out_value` - A pointer to variable that will receive the value. Not
    ///                 filled if false is returned.
    ///
    /// Returns `true` if the attribute maps to a number value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut f32,
    ) -> FPDF_BOOL;

    /// Gets the value of a string attribute in an attribute map as string.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_STRING` or `FPDF_OBJECT_NAME` for this property.
    ///
    ///   `value`      - Handle to the value.
    ///
    ///   `buffer`     - A buffer for holding the returned key in UTF-16LE.
    ///                  This is only modified if `buflen` is longer than the
    ///                  length of the key. Optional, pass `NULL` to just
    ///                  retrieve the size of the buffer needed.
    ///
    ///   `buflen`     - The length of the buffer.
    ///
    ///   `out_buflen` - A pointer to variable that will receive the minimum
    ///                  buffer size to contain the key. Not filled if `false` is
    ///                  returned.
    ///
    /// Returns `true` if the attribute maps to a string value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetStringValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Gets the value of a blob attribute in an attribute map as string.
    ///
    ///   `value`      - Handle to the value.
    ///
    ///   `buffer`     - A buffer for holding the returned value. This is only
    ///                  modified if `buflen` is at least as long as the length
    ///                  of the value. Optional, pass `NULL` to just retrieve the
    ///                  size of the buffer needed.
    ///
    ///   `buflen`     - The length of the buffer.
    ///
    ///   `out_buflen` - A pointer to variable that will receive the minimum buffer size
    ///                  to contain the key. Not filled if `false` is returned.
    ///
    /// Returns `true` if the attribute maps to a string value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBlobValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Counts the number of children values in an attribute.
    ///
    ///   `value` - Handle to the value.
    ///
    /// Returns the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_CountChildren(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int;

    /// Gets a child from an attribute.
    ///
    ///   `value` - Handle to the value.
    ///
    ///   `index` - The index for the child, 0-based.
    ///
    /// Returns the child at the n-th index, or `NULL` on error.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_Attr_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetChildAtIndex(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE;

    /// Gets the count of marked content ids for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the count of marked content ids or -1 if none exists.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets the marked content id at a given index for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index of the marked content id, 0-based.
    ///
    /// Returns the marked content ID of the element. If no ID exists, returns -1.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdCount]
    /// return value.
    ///
    /// This function will likely supersede [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentID].
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdAtIndex(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int;

    /// Creates a new PDF page.
    ///
    ///   `document`   - handle to document.
    ///
    ///   `page_index` - suggested 0-based index of the page to create. If it is larger
    ///                  than document's current last index(L), the created page index
    ///                  is the next available index -- L+1.
    ///
    ///   `width`      - the page width in points.
    ///
    ///   `height`     - the page height in points.
    ///
    /// Returns the handle to the new page or `NULL` on failure.
    ///
    /// The page should be closed with [PdfiumLibraryBindings::FPDF_ClosePage()] when finished as
    /// with any other page in the document.
    #[allow(non_snake_case)]
    fn FPDFPage_New(&self, document: FPDF_DOCUMENT, page_index: c_int, width: c_double, height: c_double) -> FPDF_PAGE;

    /// Deletes the page at `page_index`.
    ///
    ///   `document`   - handle to document.
    ///
    ///   `page_index` - the index of the page to delete.
    #[allow(non_snake_case)]
    fn FPDFPage_Delete(&self, document: FPDF_DOCUMENT, page_index: c_int);

    /// Moves the given pages to a new index position.
    ///
    ///   `page_indices`     - the ordered list of pages to move. No duplicates allowed.
    ///
    ///   `page_indices_len` - the number of elements in `page_indices`
    ///
    ///   `dest_page_index`  - the new index position to which the pages in
    ///                        `page_indices` are moved.
    ///
    /// Returns `true` on success. If it returns `false`, the document may be left in an
    /// indeterminate state.
    ///
    /// Example: The PDF document starts out with pages [A, B, C, D], with indices
    /// [0, 1, 2, 3].
    ///
    /// >  Move(doc, [3, 2], 2, 1); // returns `true`. The document now has pages [A, D, C, B].
    /// >
    /// >  Move(doc, [0, 4, 3], 3, 1); // returns `false` because index 4 is out of range.
    /// >
    /// >  Move(doc, [0, 3, 1], 3, 2); // returns `false` because index 2 is out of range for 3 page indices.
    /// >
    /// >  Move(doc, [2, 2], 2, 0); // returns `false` because [2, 2] contains duplicates.
    #[allow(non_snake_case)]
    fn FPDF_MovePages(
        &self,
        document: FPDF_DOCUMENT,
        page_indices: *const c_int,
        page_indices_len: c_ulong,
        dest_page_index: c_int,
    ) -> FPDF_BOOL;

    /// Gets the rotation of `page`.
    ///
    ///   `page` - handle to a page
    ///
    /// Returns one of the following indicating the page rotation:
    ///
    ///   `0` - No rotation.
    ///
    ///   `1` - Rotated 90 degrees clockwise.
    ///
    ///   `2` - Rotated 180 degrees clockwise.
    ///
    ///   `3` - Rotated 270 degrees clockwise.
    #[allow(non_snake_case)]
    fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int;

    /// Sets rotation for `page`.
    ///
    ///   `page`   - handle to a page.
    ///
    ///   `rotate` - the rotation value, one of:
    ///
    ///              0 - No rotation.
    ///
    ///              1 - Rotated 90 degrees clockwise.
    ///
    ///              2 - Rotated 180 degrees clockwise.
    ///
    ///              3 - Rotated 270 degrees clockwise.
    #[allow(non_snake_case)]
    fn FPDFPage_SetRotation(&self, page: FPDF_PAGE, rotate: c_int);

    /// Gets the bounding box of the page. This is the intersection between
    /// its media box and its crop box.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `rect`        -   Pointer to a rect to receive the page bounding box.
    ///                      On an error, `rect` won't be filled.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDF_GetPageBoundingBox(&self, page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the size of the page at the given index.
    ///
    ///    `document`    -   Handle to document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `page_index`  -   Page index, zero for the first page.
    ///
    ///    `size`        -   Pointer to a `FS_SIZEF` to receive the page size (in points).
    ///
    /// Returns non-zero value for success, `0` for error (document or page not found).
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndexF(&self, document: FPDF_DOCUMENT, page_index: c_int, size: *mut FS_SIZEF) -> FPDF_BOOL;

    /// Gets the size of the page at the given index.
    ///
    ///    `document`    -   Handle to document. Returned by [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `page_index`  -   Page index, zero for the first page.
    ///
    ///    `width`       -   Pointer to a double to receive the page width (in points).
    ///
    ///    `height`      -   Pointer to a double to receive the page height (in points).
    ///
    /// Returns non-zero for success, `0` for error (document or page not found).
    ///
    /// Note: prefer [PdfiumLibraryBindings::FPDF_GetPageSizeByIndexF]. This function
    /// will be deprecated in the future.
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndex(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int;

    /// Gets "MediaBox" entry from the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - Pointer to a float value receiving the left of the rectangle.
    ///
    ///    `bottom` - Pointer to a float value receiving the bottom of the rectangle.
    ///
    ///    `right`  - Pointer to a float value receiving the right of the rectangle.
    ///
    ///    `top`    - Pointer to a float value receiving the top of the rectangle.
    ///
    /// On success, returns `true` and writes to the out parameters. Otherwise returns `false`
    /// and leaves the out parameters unmodified.
    #[allow(non_snake_case)]
    fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets "CropBox" entry from the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - Pointer to a float value receiving the left of the rectangle.
    ///
    ///    `bottom` - Pointer to a float value receiving the bottom of the rectangle.
    ///
    ///    `right`  - Pointer to a float value receiving the right of the rectangle.
    ///
    ///    `top`    - Pointer to a float value receiving the top of the rectangle.
    ///
    /// On success, returns `true` and writes to the out parameters. Otherwise returns `false`
    /// and leaves the out parameters unmodified.
    #[allow(non_snake_case)]
    fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets "BleedBox" entry from the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - Pointer to a float value receiving the left of the rectangle.
    ///
    ///    `bottom` - Pointer to a float value receiving the bottom of the rectangle.
    ///
    ///    `right`  - Pointer to a float value receiving the right of the rectangle.
    ///
    ///    `top`    - Pointer to a float value receiving the top of the rectangle.
    ///
    /// On success, returns `true` and writes to the out parameters. Otherwise returns `false`
    /// and leaves the out parameters unmodified.
    #[allow(non_snake_case)]
    fn FPDFPage_GetBleedBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets "TrimBox" entry from the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - Pointer to a float value receiving the left of the rectangle.
    ///
    ///    `bottom` - Pointer to a float value receiving the bottom of the rectangle.
    ///
    ///    `right`  - Pointer to a float value receiving the right of the rectangle.
    ///
    ///    `top`    - Pointer to a float value receiving the top of the rectangle.
    ///
    /// On success, returns `true` and writes to the out parameters. Otherwise returns `false`
    /// and leaves the out parameters unmodified.
    #[allow(non_snake_case)]
    fn FPDFPage_GetTrimBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets "ArtBox" entry from the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - Pointer to a float value receiving the left of the rectangle.
    ///
    ///    `bottom` - Pointer to a float value receiving the bottom of the rectangle.
    ///
    ///    `right`  - Pointer to a float value receiving the right of the rectangle.
    ///
    ///    `top`    - Pointer to a float value receiving the top of the rectangle.
    ///
    /// On success, returns `true` and writes to the out parameters. Otherwise returns `false`
    /// and leaves the out parameters unmodified.
    #[allow(non_snake_case)]
    fn FPDFPage_GetArtBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Sets "MediaBox" entry to the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - The left of the rectangle.
    ///
    ///    `bottom` - The bottom of the rectangle.
    ///
    ///    `right`  - The right of the rectangle.
    ///
    ///    `top`    - The top of the rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_SetMediaBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float);

    /// Sets "CropBox" entry in the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - The left of the rectangle.
    ///
    ///    `bottom` - The bottom of the rectangle.
    ///
    ///    `right`  - The right of the rectangle.
    ///
    ///    `top`    - The top of the rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_SetCropBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float);

    /// Sets "BleedBox" entry in the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - The left of the rectangle.
    ///
    ///    `bottom` - The bottom of the rectangle.
    ///
    ///    `right`  - The right of the rectangle.
    ///
    ///    `top`    - The top of the rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_SetBleedBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float);

    /// Sets "TrimBox" entry in the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - The left of the rectangle.
    ///
    ///    `bottom` - The bottom of the rectangle.
    ///
    ///    `right`  - The right of the rectangle.
    ///
    ///    `top`    - The top of the rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_SetTrimBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float);

    /// Sets "ArtBox" entry in the page dictionary.
    ///
    ///    `page`   - Handle to a page.
    ///
    ///    `left`   - The left of the rectangle.
    ///
    ///    `bottom` - The bottom of the rectangle.
    ///
    ///    `right`  - The right of the rectangle.
    ///
    ///    `top`    - The top of the rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_SetArtBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float);

    /// Applies transforms to `page`.
    ///
    /// If `matrix` is provided, it will be applied to transform the page.
    /// If `clipRect` is provided, it will be used to clip the resulting page.
    /// If neither `matrix` nor `clipRect` are provided, this method returns `false`.
    ///
    /// Returns `true` if transforms are applied. This function will transform the whole page,
    /// and will take effect on all the objects on the page.
    ///
    ///    `page`        - Page handle.
    ///
    ///    `matrix`      - Transform matrix.
    ///
    ///    `clipRect`    - Clipping rectangle.
    #[allow(non_snake_case)]
    fn FPDFPage_TransFormWithClip(
        &self,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipRect: *const FS_RECTF,
    ) -> FPDF_BOOL;

    /// Transforms (scale, rotate, shear, move) the clip path of page object.
    ///
    ///    `page_object` - Handle to a page object. Returned by [PdfiumLibraryBindings::FPDFPageObj_NewImageObj].
    ///
    ///    `a`  - The coefficient "a" of the transformation matrix.
    ///
    ///    `b`  - The coefficient "b" of the matrix.
    ///
    ///    `c`  - The coefficient "c" of the matrix.
    ///
    ///    `d`  - The coefficient "d" of the matrix.
    ///
    ///    `e`  - The coefficient "e" of the matrix.
    ///
    ///    `f`  - The coefficient "f" of the matrix.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPageObj_TransformClipPath(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    );

    /// Gets the clip path of the page object.
    ///
    ///    `page object` - Handle to a page object. Returned by e.g.
    ///                    [PdfiumLibraryBindings::FPDFPage_GetObject].
    ///
    /// Returns the handle to the clip path, or `NULL` on failure. The caller does not
    /// take ownership of the returned `FPDF_CLIPPATH`. Instead, it remains valid until
    /// [PdfiumLibraryBindings::FPDF_ClosePage] is called for the page containing `page_object`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetClipPath(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH;

    /// Gets the number of paths inside `clip_path`.
    ///
    ///    `clip_path` - handle to a clip path.
    ///
    /// Returns the number of objects in `clip_path`, or `-1` on failure.
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPaths(&self, clip_path: FPDF_CLIPPATH) -> c_int;

    /// Gets the number of segments inside one path of `clip_path`.
    ///
    ///    `clip_path`  - handle to a clip path.
    ///
    ///    `path_index` - index into the array of paths of the clip path.
    ///
    /// Returns the number of segments, or `-1` on failure.
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPathSegments(&self, clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int;

    /// Gets a specific segment in a specific path of `clip_path`.
    ///
    ///    `clip_path`     - handle to a clip path.
    ///
    ///    `path_index`    - the index of a path.
    ///
    ///    `segment_index` - the index of a segment.
    ///
    /// Returns the handle to the segment, or `NULL` on failure. The caller does not
    /// take ownership of the returned `FPDF_PATHSEGMENT`. Instead, it remains valid
    /// until [PdfiumLibraryBindings::FPDF_ClosePage] is called for the page containing `clip_path`.
    #[allow(non_snake_case)]
    fn FPDFClipPath_GetPathSegment(
        &self,
        clip_path: FPDF_CLIPPATH,
        path_index: c_int,
        segment_index: c_int,
    ) -> FPDF_PATHSEGMENT;

    /// Creates a new clip path, with a rectangle inserted.
    ///
    /// Caller takes ownership of the returned `FPDF_CLIPPATH`. It should be freed with
    /// [PdfiumLibraryBindings::FPDF_DestroyClipPath].
    ///
    ///    `left`   - The left of the clip box.
    ///
    ///    `bottom` - The bottom of the clip box.
    ///
    ///    `right`  - The right of the clip box.
    ///
    ///    `top`    - The top of the clip box.
    #[allow(non_snake_case)]
    fn FPDF_CreateClipPath(&self, left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH;

    /// Destroys a clip path.
    ///
    ///    `clipPath` - A handle to the clip path. It will be invalid after this call.
    #[allow(non_snake_case)]
    fn FPDF_DestroyClipPath(&self, clipPath: FPDF_CLIPPATH);

    /// Clips the page content. Content outside the clipping region will become invisible.
    ///
    /// A clip path will be inserted before the page content stream or content array.
    /// In this way, the page content will be clipped by this clip path.
    ///
    ///    `page`        - A page handle.
    ///
    ///    `clipPath`    - A handle to the clip path. The caller does not take ownership.
    #[allow(non_snake_case)]
    fn FPDFPage_InsertClipPath(&self, page: FPDF_PAGE, clipPath: FPDF_CLIPPATH);

    /// Checks if `page` contains transparency.
    ///
    ///    `page` - handle to a page.
    ///
    /// Returns `true` if `page` contains transparency.
    #[allow(non_snake_case)]
    fn FPDFPage_HasTransparency(&self, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Generates the content of `page`. Before you save `page` to a file or reload `page`,
    /// you must call this function or any changes to `page` will be lost.
    ///
    ///    `page` - handle to a page.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPage_GenerateContent(&self, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Transforms all annotations in `page`.
    ///
    ///    `page` - handle to a page.
    ///
    ///    `a`    - matrix value.
    ///
    ///    `b`    - matrix value.
    ///
    ///    `c`    - matrix value.
    ///
    ///    `d`    - matrix value.
    ///
    ///    `e`    - matrix value.
    ///
    ///    `f`    - matrix value.
    ///
    /// The matrix is composed as:
    ///
    ///    `|a c e|`
    ///
    ///    `|b d f|`
    ///
    /// and can be used to scale, rotate, shear, and translate the `page` annotations.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPage_TransformAnnots(&self, page: FPDF_PAGE, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64);

    /// Creates a device independent bitmap (FXDIB).
    ///
    ///   `width`       -   The number of pixels in width for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `height`      -   The number of pixels in height for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `alpha`       -   A flag indicating whether the alpha channel is used.
    ///                     Non-zero for using alpha, zero for not using.
    ///
    /// Returns the created bitmap handle, or `NULL` if a parameter error or out of
    /// memory.
    ///
    /// The bitmap always uses 4 bytes per pixel. The first byte is always double word aligned.
    /// The byte order is BGRx (the last byte unused if no alpha channel) or BGRA.
    /// The pixels in a horizontal line are stored side by side, with the left most pixel
    /// stored first (with lower memory address). Each line uses `width * 4` bytes.
    /// Lines are stored one after another, with the top most line stored first.
    /// There is no gap between adjacent lines.
    ///
    /// This function allocates enough memory for holding all pixels in the bitmap,
    /// but it doesn't initialize the buffer. Applications can use [PdfiumLibraryBindings::FPDFBitmap_FillRect]
    /// to fill the bitmap using any color. If the OS allows it, this function can allocate
    /// up to 4 GB of memory.
    #[allow(non_snake_case)]
    fn FPDFBitmap_Create(&self, width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP;

    /// Creates a device independent bitmap (FXDIB).
    ///
    ///   `width`       -   The number of pixels in width for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `height`      -   The number of pixels in height for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `format`      -   A number indicating for bitmap format, as defined above.
    ///
    ///   `first_scan`  -   A pointer to the first byte of the first line if
    ///                     using an external buffer. If this parameter is `NULL`,
    ///                     then a new buffer will be created.
    ///
    ///   `stride`      -   Number of bytes for each scan line. The value must
    ///                     be 0 or greater. When the value is 0,
    ///                     `FPDFBitmap_CreateEx()` will automatically calculate
    ///                     the appropriate value using `width` and `format`.
    ///                     When using an external buffer, it is recommended for the caller
    ///                     to pass in the value. When not using an external buffer, it is
    ///                     recommended for the caller to pass in 0.
    ///
    /// Returns the bitmap handle, or `NULL` if parameter error or out of memory.
    ///
    /// Similar to [PdfiumLibraryBindings::FPDFBitmap_Create] function, but allows for more
    /// formats and an external buffer is supported. The bitmap created by this function
    /// can be used in any place that a `FPDF_BITMAP` handle is required.
    ///
    /// If an external buffer is used, then the caller should destroy the buffer.
    /// [PdfiumLibraryBindings::FPDFBitmap_Destroy] will not destroy the buffer.
    ///
    /// It is recommended to use [PdfiumLibraryBindings::FPDFBitmap_GetStride to get the stride
    /// value.
    #[allow(non_snake_case)]
    fn FPDFBitmap_CreateEx(
        &self,
        width: c_int,
        height: c_int,
        format: c_int,
        first_scan: *mut c_void,
        stride: c_int,
    ) -> FPDF_BITMAP;

    /// Gets the format of the bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the format of the bitmap.
    ///
    /// Only formats supported by [PdfiumLibraryBindings::FPDFBitmap_CreateEx] are supported by this
    /// function; see the list of such formats above.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetFormat(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Fills a rectangle in a bitmap.
    ///
    ///   `bitmap`      -   The handle to the bitmap. Returned by
    ///                     [PdfiumLibraryBindings::FPDFBitmap_Create].
    ///
    ///   `left`        -   The left position. Starting from 0 at the left-most pixel.
    ///
    ///   `top`         -   The top position. Starting from 0 at the top-most line.
    ///
    ///   `width`       -   Width in pixels to be filled.
    ///
    ///   `height`      -   Height in pixels to be filled.
    ///
    ///   `color`       -   A 32-bit value specifying the color, in 8888 ARGB format.
    ///
    /// Returns whether the operation succeeded or not.
    ///
    /// This function sets the color and (optionally) alpha value in the specified region
    /// of the bitmap.
    ///
    /// Note: If the alpha channel is used, this function does _not_ composite the background
    /// with the source color, instead the background will be replaced by the source color
    /// and the alpha. If the alpha channel is not used, the alpha parameter is ignored.
    #[allow(non_snake_case)]
    fn FPDFBitmap_FillRect(
        &self,
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    ) -> FPDF_BOOL;

    #[cfg(not(target_arch = "wasm32"))]
    /// Note that this function is not available when compiling to WASM as it cannot be made
    /// memory safe. When compiling to WASM, Pdfium's internal pixel data buffer for a bitmap
    /// resides in a separate WASM memory module from your Rust application, so any buffer
    /// returned by this function is necessarily a copy; mutating that copy does not alter
    /// the buffer in Pdfium's WASM module and, since there is no way for `pdfium-render` to
    /// know when the caller has finished mutating the copied buffer, there is no reliable way
    /// for `pdfium-render` to transfer any changes made to the copy across to Pdfium's
    /// WASM module.
    ///
    /// To avoid having to maintain different code for different platform targets, it is
    /// recommended that all callers use the provided [PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_vec]
    /// and [PdfiumLibraryBindings::FPDFBitmap_SetBuffer] convenience functions to retrieve
    /// and update the pixel data of a bitmap, instead of directly mutating the buffer
    /// returned by this function.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the pointer to the first byte of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Applications can use this function to get the bitmap buffer pointer,
    /// then manipulate any color and/or alpha values for any pixels in the bitmap.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *mut c_void;

    // TODO: AJRC - 27/11/24 - remove deprecated item as part of #36
    #[deprecated(
        since = "0.8.27",
        note = "The WASM implementation of FPDFBitmap_GetBuffer() cannot be made memory-safe. Prefer FPDFBitmap_GetBuffer_as_vec() or FPDFBitmap_GetBuffer_as_array() instead."
    )]
    #[cfg(target_arch = "wasm32")]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *const c_void;

    #[cfg(not(target_arch = "wasm32"))]
    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as an
    /// alternative to directly mutating the data returned by
    /// [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// Replaces all pixel data for the given bitmap with the pixel data in the given buffer,
    /// returning `true` once the new pixel data has been applied. If the given buffer
    /// does not have the same length as the bitmap's current buffer then the current buffer
    /// will be unchanged and a value of `false` will be returned.
    #[allow(non_snake_case)]
    fn FPDFBitmap_SetBuffer(&self, bitmap: FPDF_BITMAP, buffer: &[u8]) -> bool {
        let buffer_length = (self.FPDFBitmap_GetStride(bitmap) * self.FPDFBitmap_GetHeight(bitmap)) as usize;

        if buffer.len() != buffer_length {
            return false;
        }

        let buffer_start = self.FPDFBitmap_GetBuffer(bitmap);

        let destination = unsafe { std::slice::from_raw_parts_mut(buffer_start as *mut u8, buffer_length) };

        destination.copy_from_slice(buffer);

        true
    }

    #[cfg(target_arch = "wasm32")]
    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as an
    /// alternative to directly mutating the data returned by
    /// [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// Replaces all pixel data of the given bitmap with the pixel data in the given buffer,
    /// returning `true` once the new pixel data has been applied. If the given buffer
    /// does not have the same length as the bitmap's current buffer then the current buffer
    /// will be unchanged and a value of `false` will be returned.
    #[allow(non_snake_case)]
    fn FPDFBitmap_SetBuffer(&self, bitmap: FPDF_BITMAP, buffer: &[u8]) -> bool;

    #[cfg(not(target_arch = "wasm32"))]
    /// Gets the data buffer of a bitmap as a Rust slice.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns a `&[u8]` slice containing the contents of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer_as_slice(&self, bitmap: FPDF_BITMAP) -> &[u8] {
        let buffer = self.FPDFBitmap_GetBuffer(bitmap);

        let len = self.FPDFBitmap_GetStride(bitmap) * self.FPDFBitmap_GetHeight(bitmap);

        unsafe { std::slice::from_raw_parts(buffer as *const u8, len as usize) }
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as a
    /// cross-platform neutral way of retrieving the pixel data owned by a bitmap. It is
    /// an alternative to Pdfium's [PdfiumLibraryBindings::FPDFBitmap_GetBuffer] function,
    /// which is not available when compiling to WASM.
    ///
    /// To maintain memory safety, this function must copy pixel data from the bitmap
    /// buffer into a new [Vec]. This has both memory usage and performance implications.
    /// For non-WASM targets, consider using the [PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_slice]
    /// function, which avoids allocation. When compiling to WASM, an equivalent function,
    /// [PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_array], is provided that similarily
    /// avoids the need to copy pixel data.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns a [Vec] containing a copy of the contents of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_SetBuffer] to apply any changes made
    /// to the returned [Vec] back to the originating bitmap.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer_as_vec(&self, bitmap: FPDF_BITMAP) -> Vec<u8> {
        Vec::from(self.FPDFBitmap_GetBuffer_as_slice(bitmap))
    }

    #[cfg(target_arch = "wasm32")]
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns a [Vec] containing the contents of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_SetBuffer] to apply any changes made
    /// to the returned [Vec] back to the originating bitmap.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer_as_vec(&self, bitmap: FPDF_BITMAP) -> Vec<u8> {
        self.FPDFBitmap_GetBuffer_as_array(bitmap).to_vec()
    }

    // TODO: AJRC - 27/11/24 - remove deprecated item as part of #36
    #[deprecated(
        since = "0.8.27",
        note = "This function has been renamed to better reflect its purpose. Prefer FPDFBitmap_GetBuffer_as_array() instead."
    )]
    #[cfg(target_arch = "wasm32")]
    #[doc(hidden)]
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetArray(&self, bitmap: FPDF_BITMAP) -> js_sys::Uint8Array {
        self.FPDFBitmap_GetBuffer_as_array(bitmap)
    }

    #[allow(non_snake_case)]
    #[cfg(target_arch = "wasm32")]
    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as a
    /// more performant WASM-specific variant of [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    /// Since it avoids a (potentially large) bitmap allocation and copy, it is both faster and
    /// more memory efficient than [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// This function is only available when compiling to WASM.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns a [js_sys::Uint8Array] containing the contents of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    ///
    /// Changes made to the returned array will directly mutate the pixel data of the bitmap.
    fn FPDFBitmap_GetBuffer_as_array(&self, bitmap: FPDF_BITMAP) -> js_sys::Uint8Array;

    #[cfg(doc)]
    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as a
    /// more performant WASM-specific variant of [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    /// Since it avoids a (potentially large) bitmap allocation and copy, it is both faster and
    /// more memory efficient than [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// This function is only available when compiling to WASM.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns a `js_sys::Uint8Array` containing the contents of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    ///
    /// Changes made to the returned array will directly mutate the pixel data of the bitmap.
    fn FPDFBitmap_GetBuffer_as_array(&self, bitmap: FPDF_BITMAP) -> Uint8Array {}

    /// Gets the width of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the width of the bitmap in pixels.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetWidth(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Gets the height of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the height of the bitmap in pixels.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetHeight(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Gets the number of bytes for each line in the bitmap buffer.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the number of bytes for each line in the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetStride(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Destroys a bitmap and releases all related buffers.
    ///
    ///    `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                      or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// This function will not destroy any external buffers provided when
    /// the bitmap was created.
    #[allow(non_snake_case)]
    fn FPDFBitmap_Destroy(&self, bitmap: FPDF_BITMAP);

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(doc)]
    /// Renders the contents of a page to a device (screen, bitmap, or printer).
    /// This function is only supported on Windows.
    ///
    ///    `dc`          -   Handle to the device context.
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`     -   Left pixel position of the display area in device coordinates.
    ///
    ///    `start_y`     -   Top pixel position of the display area in device coordinates.
    ///
    ///    `size_x`      -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`      -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`      -   Page orientation:
    ///                            0 (normal)
    ///                            1 (rotated 90 degrees clockwise)
    ///                            2 (rotated 180 degrees)
    ///                            3 (rotated 90 degrees counter-clockwise)
    ///
    ///    `flags`       -   0 for normal display, or combination of flags defined above.
    #[allow(non_snake_case)]
    fn FPDF_RenderPage(
        &self,
        dc: HDC,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ) {
    }

    /// Renders contents of a page to a device independent bitmap.
    ///
    ///    `bitmap`      -   Handle to the device independent bitmap (as the
    ///                      output buffer). The bitmap handle can be created
    ///                      by [PdfiumLibraryBindings::FPDFBitmap_Create] or retrieved from an image
    ///                      object by [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`     -   Left pixel position of the display area in bitmap coordinates.
    ///
    ///    `start_y`     -   Top pixel position of the display area in bitmap coordinates.
    ///
    ///    `size_x`      -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`      -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`      -   Page orientation:
    ///                            0 (normal)
    ///                            1 (rotated 90 degrees clockwise)
    ///                            2 (rotated 180 degrees)
    ///                            3 (rotated 90 degrees counter-clockwise)
    ///
    ///    `flags`       -   0 for normal display, or combination of the Page Rendering flags defined above.
    ///                      With the `FPDF_ANNOT` flag, it renders all annotations that do not require
    ///                      user-interaction, which are all annotations except widget and popup annotations.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_RenderPageBitmap(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    /// Renders contents of a page to a device independent bitmap.
    ///
    ///    `bitmap`      -   Handle to the device independent bitmap (as the
    ///                      output buffer). The bitmap handle can be created
    ///                      by [PdfiumLibraryBindings::FPDFBitmap_Create] or retrieved by
    ///                      [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `matrix`      -   The transform matrix, which must be invertible.
    ///                      See PDF Reference 1.7, 4.2.2 Common Transformations.
    ///
    ///    `clipping`    -   The rect to clip to in device coords.
    ///
    ///    `flags`       -   0 for normal display, or combination of the Page Rendering flags defined above.
    ///                      With the `FPDF_ANNOT` flag, it renders all annotations that do not require
    ///                      user-interaction, which are all annotations except widget and popup annotations.
    ///
    /// Note that behavior is undefined if det of `matrix` is 0.
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithMatrix(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipping: *const FS_RECTF,
        flags: c_int,
    );

    /// Checks if an annotation subtype is currently supported for creation.
    /// Currently supported subtypes:
    ///
    ///    - circle
    ///
    ///    - file attachment
    ///
    ///    - freetext
    ///
    ///    - highlight
    ///
    ///    - ink
    ///
    ///    - link
    ///
    ///    - popup
    ///
    ///    - square
    ///
    ///    - squiggly
    ///
    ///    - stamp
    ///
    ///    - strikeout
    ///
    ///    - text
    ///
    ///    - underline
    ///
    ///   `subtype`   - the subtype to be checked.
    ///
    /// Returns `true` if this subtype supported.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL;

    /// Creates an annotation in `page` of the subtype `subtype`. If the specified
    /// subtype is illegal or unsupported, then a new annotation will not be created.
    /// Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the annotation returned by this
    /// function is no longer needed.
    ///
    ///   `page`      - handle to a page.
    ///
    ///   `subtype`   - the subtype of the new annotation.
    ///
    /// Returns a handle to the new annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_CreateAnnot(&self, page: FPDF_PAGE, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_ANNOTATION;

    /// Gets the number of annotations in `page`.
    ///
    ///   `page`   - handle to a page.
    ///
    /// Returns the number of annotations in `page`.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotCount(&self, page: FPDF_PAGE) -> c_int;

    /// Gets annotation in `page` at `index`. Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the
    /// annotation returned by this function is no longer needed.
    ///
    ///   `page`  - handle to a page.
    ///
    ///   `index` - the index of the annotation.
    ///
    /// Returns a handle to the annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION;

    /// Gets the index of `annot` in `page`. This is the opposite of
    /// [PdfiumLibraryBindings::FPDFPage_GetAnnot].
    ///
    ///   `page`  - handle to the page that the annotation is on.
    ///
    ///   `annot` - handle to an annotation.
    ///
    /// Returns the index of `annot`, or -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotIndex(&self, page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int;

    /// Closes an annotation. Must be called when the annotation returned by
    /// [PdfiumLibraryBindings::FPDFPage_CreateAnnot] or [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    /// is no longer needed. This function does not remove the annotation from the document.
    ///
    ///   `annot`  - handle to an annotation.
    #[allow(non_snake_case)]
    fn FPDFPage_CloseAnnot(&self, annot: FPDF_ANNOTATION);

    /// Removes the annotation in `page` at `index`.
    ///
    ///   `page`  - handle to a page.
    ///
    ///   `index` - the index of the annotation.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL;

    /// Gets the subtype of an annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the annotation subtype.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetSubtype(&self, annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE;

    /// Checks if an annotation subtype is currently supported for object extraction,
    /// update, and removal.
    ///
    /// Currently supported subtypes: ink and stamp.
    ///
    ///   `subtype`   - the subtype to be checked.
    ///
    /// Returns `true` if this subtype supported.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsObjectSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL;

    /// Updates `obj` in `annot`. `obj` must be in `annot` already and must have
    /// been retrieved by [PdfiumLibraryBindings::FPDFAnnot_GetObject]. Currently, only ink and stamp
    /// annotations are supported by this API. Also note that only path, image, and
    /// text objects have APIs for modification; see `FPDFPath_*()`, `FPDFText_*()`, and
    /// `FPDFImageObj_*()`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `obj`    - handle to the object that `annot` needs to update.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_UpdateObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Adds a new InkStroke, represented by an array of points, to the InkList of
    /// `annot`. The API creates an InkList if one doesn't already exist in `annot`.
    /// This API works only for ink annotations. Please refer to ISO 32000-1:2008
    /// spec, section 12.5.6.13.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `points`      - pointer to a `FS_POINTF` array representing input points.
    ///
    ///   `point_count` - number of elements in `points` array. This should not exceed
    ///                   the maximum value that can be represented by an `int32_t`.
    ///
    /// Returns the 0-based index at which the new InkStroke is added in the InkList
    /// of the `annot`. Returns -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddInkStroke(&self, annot: FPDF_ANNOTATION, points: *const FS_POINTF, point_count: size_t) -> c_int;

    /// Removes an InkList in `annot`.
    ///
    /// This API works only for ink annotations.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Return true on successful removal of `/InkList` entry from context of the
    /// non-null ink `annot`. Returns `false` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveInkList(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Adds `obj` to `annot`. `obj` must have been created by
    /// [PdfiumLibraryBindings::FPDFPageObj_CreateNewPath], [PdfiumLibraryBindings::FPDFPageObj_CreateNewRect],
    /// [PdfiumLibraryBindings::FPDFPageObj_NewTextObj], or [PdfiumLibraryBindings::FPDFPageObj_NewImageObj], and
    /// will be owned by `annot`. Note that an `obj` cannot belong to more than one
    /// `annot`. Currently, only ink and stamp annotations are supported by this API.
    /// Also note that only path, image, and text objects have APIs for creation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `obj`    - handle to the object that is to be added to `annot`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Gets the total number of objects in `annot`, including path objects, text
    /// objects, external objects, image objects, and shading objects.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the number of objects in `annot`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObjectCount(&self, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the object in `annot` at `index`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `index`  - the index of the object.
    ///
    /// Returns a handle to the object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT;

    /// Removes the object in `annot` at `index`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `index`  - the index of the object to be removed.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL;

    /// Sets the color of an annotation. Fails when called on annotations with
    /// appearance streams already defined; instead use
    /// [PdfiumLibraryBindings::FPDFPageObj_SetStrokeColor] or [PdfiumLibraryBindings::FPDFPageObj_SetFillColor].
    ///
    ///   `annot`        - handle to an annotation.
    ///
    ///   `type`         - type of the color to be set.
    ///
    ///   `R`, `G`, `B`  - buffers to hold the RGB values of the color. Ranges from 0 to 255.
    ///
    ///   `A`            - buffers to hold the opacity. Ranges from 0 to 255.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    /// Gets the color of an annotation. If no color is specified, default to yellow
    /// for highlight annotation, black for all else. Fails when called on
    /// annotations with appearance streams already defined; instead use
    /// [PdfiumLibraryBindings::FPDFPageObj_GetStrokeColor] or [PdfiumLibraryBindings::FPDFPageObj_GetFillColor].
    ///
    ///   `annot`        - handle to an annotation.
    ///
    ///   `type`         - type of the color requested.
    ///
    ///   `R`, `G`, `B`  - buffers to hold the RGB values of the color. Ranges from 0 to 255.
    ///
    ///   `A`            - buffer to hold the opacity. Ranges from 0 to 255.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Checks if the annotation is of a type that has attachment points
    /// (i.e. quadpoints). Quadpoints are the vertices of the rectangle that
    /// encompasses the texts affected by the annotation. They provide the
    /// coordinates in the page where the annotation is attached. Only text markup
    /// annotations (i.e. highlight, strikeout, squiggly, and underline) and link
    /// annotations have quadpoints.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns `true` if the annotation is of a type that has quadpoints.
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Replaces the attachment points (i.e. quadpoints) set of an annotation at
    /// `quad_index`. This index needs to be within the result of
    /// [PdfiumLibraryBindings::FPDFAnnot_CountAttachmentPoints].
    ///
    /// If the annotation's appearance stream is defined and this annotation is of a
    /// type with quadpoints, then update the bounding box too if the new quadpoints
    /// define a bigger one.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_index`  - index of the set of quadpoints.
    ///
    ///   `quad_points` - the quadpoints to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Appends to the list of attachment points (i.e. quadpoints) of an annotation.
    /// If the annotation's appearance stream is defined and this annotation is of a
    /// type with quadpoints, then update the bounding box too if the new quadpoints
    /// define a bigger one.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_points` - the quadpoints to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendAttachmentPoints(&self, annot: FPDF_ANNOTATION, quad_points: *const FS_QUADPOINTSF)
    -> FPDF_BOOL;

    /// Gets the number of sets of quadpoints of an annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the number of sets of quadpoints, or 0 on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_CountAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> size_t;

    /// Gets the attachment points (i.e. quadpoints) of an annotation.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_index`  - index of the set of quadpoints.
    ///
    ///   `quad_points` - receives the quadpoints; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Sets the annotation rectangle defining the location of the annotation. If the
    /// annotation's appearance stream is defined and this annotation is of a type
    /// without quadpoints, then update the bounding box too if the new rectangle
    /// defines a bigger one.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `rect`   - the annotation rectangle to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetRect(&self, annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL;

    /// Gets the annotation rectangle defining the location of the annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `rect`   - receives the rectangle; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetRect(&self, annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the vertices of a polygon or polyline annotation. `buffer` is an array of
    /// points of the annotation. If `length` is less than the returned length, or
    /// `annot` or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `buffer` - buffer for holding the points.
    ///
    ///   `length` - length of the buffer in points.
    ///
    /// Returns the number of points if the annotation is of type polygon or
    /// polyline, 0 otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetVertices(&self, annot: FPDF_ANNOTATION, buffer: *mut FS_POINTF, length: c_ulong) -> c_ulong;

    /// Gets the number of paths in the ink list of an ink annotation.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    /// Returns the number of paths in the ink list if the annotation is of type ink,
    /// 0 otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListCount(&self, annot: FPDF_ANNOTATION) -> c_ulong;

    /// Gets a path in the ink list of an ink annotation. `buffer` is an array of
    /// points of the path. If `length` is less than the returned length, or `annot`
    /// or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `path_index` - index of the path.
    ///
    ///   `buffer` - buffer for holding the points.
    ///
    ///   `length` - length of the buffer in points.
    ///
    /// Returns the number of points of the path if the annotation is of type ink, 0
    /// otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListPath(
        &self,
        annot: FPDF_ANNOTATION,
        path_index: c_ulong,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong;

    /// Gets the starting and ending coordinates of a line annotation.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `start` - starting point
    ///
    ///   `end` - ending point
    ///
    /// Returns `true` if the annotation is of type line and `start` and `end` are not `NULL`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLine(&self, annot: FPDF_ANNOTATION, start: *mut FS_POINTF, end: *mut FS_POINTF) -> FPDF_BOOL;

    /// Sets the characteristics of the annotation's border (rounded rectangle).
    ///
    ///   `annot`              - handle to an annotation.
    ///
    ///   `horizontal_radius`  - horizontal corner radius, in default user space units.
    ///
    ///   `vertical_radius`    - vertical corner radius, in default user space units.
    ///
    ///   `border_width`       - border width, in default user space units.
    ///
    /// Returns `true` if setting the border for `annot` succeeds.
    ///
    /// If `annot` contains an appearance stream that overrides the border values,
    /// then the appearance stream will be removed on success.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: c_float,
        vertical_radius: c_float,
        border_width: c_float,
    ) -> FPDF_BOOL;

    /// Gets the characteristics of the annotation's border (rounded rectangle).
    ///
    ///   `annot`              - handle to an annotation.
    ///
    ///   `horizontal_radius`  - horizontal corner radius, in default user space units.
    ///
    ///   `vertical_radius`    - vertical corner radius, in default user space units.
    ///
    ///   `border_width`       - border width, in default user space units.
    ///
    /// Returns `true` if `horizontal_radius`, `vertical_radius` and `border_width` are not `NULL`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: *mut c_float,
        vertical_radius: *mut c_float,
        border_width: *mut c_float,
    ) -> FPDF_BOOL;

    /// Get the JavaScript of an event of the annotation's additional actions.
    ///
    /// `buffer` is only modified if `buflen` is large enough to hold the whole
    /// JavaScript string. If `buflen` is smaller, the total size of the JavaScript
    /// is still returned, but nothing is copied.  If there is no JavaScript for
    /// `event` in `annot`, an empty string is written to `buf` and 2 is returned,
    /// denoting the size of the null terminator in the buffer. On other errors,
    /// nothing is written to `buffer` and 0 is returned.
    ///
    ///   `form`        -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment()].
    ///
    ///   `annot`       -   handle to an interactive form annotation.
    ///
    ///   `event`       -   event type, one of the `FPDF_ANNOT_AACTION_*` values.
    ///
    ///   `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`     -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes, including the 2-byte null terminator.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormAdditionalActionJavaScript(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        event: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the alternate name of `annot`, which is an interactive form annotation.
    ///
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will be 0.
    /// Note that return value of empty string is 2 for `\0\0`.
    ///
    ///   `form`        -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment()].
    ///
    ///   `annot`       -   handle to an interactive form annotation.
    ///
    ///   `buffer`      -   buffer for holding the alternate name string, encoded in
    ///                     UTF-16LE.
    ///
    ///   `buflen`     -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAlternateName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Check if `annot`'s dictionary has `key` as a key.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to look for, encoded in UTF-8.
    ///
    /// Returns `true` if `key` exists.
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasKey(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_BOOL;

    /// Gets the type of the value corresponding to `key` in `annot`'s dictionary.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to look for, encoded in UTF-8.
    ///
    /// Returns the type of the dictionary value.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetValueType(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_OBJECT_TYPE;

    /// Sets the string value corresponding to `key` in `annot`'s dictionary,
    /// overwriting the existing value if any. The value type would be
    /// `FPDF_OBJECT_STRING` after this function call succeeds.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the dictionary entry to be set, encoded in UTF-8.
    ///
    ///   `value`  - the string value to be set, encoded in UTF-16LE.
    ///
    /// Returns `true` if successful.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_SetStringValue_str].
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue(&self, annot: FPDF_ANNOTATION, key: &str, value: FPDF_WIDESTRING) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_SetStringValue].
    ///
    /// Sets the string value corresponding to `key` in `annot`'s dictionary,
    /// overwriting the existing value if any. The value type would be
    /// `FPDF_OBJECT_STRING` after this function call succeeds.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the dictionary entry to be set.
    ///
    ///   `value`  - the string value to be set.
    ///
    /// Returns `true` if successful.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue_str(&self, annot: FPDF_ANNOTATION, key: &str, value: &str) -> FPDF_BOOL {
        self.FPDFAnnot_SetStringValue(
            annot,
            key,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the string value corresponding to `key` in `annot`'s dictionary. `buffer`
    /// is only modified if `buflen` is longer than the length of contents. Note that
    /// if `key` does not exist in the dictionary or if `key`'s corresponding value
    /// in the dictionary is not a string (i.e. the value is not of type
    /// `FPDF_OBJECT_STRING` or `FPDF_OBJECT_NAME`), then an empty string would be copied
    /// to `buffer` and the return value would be 2. On other errors, nothing would
    /// be added to `buffer` and the return value would be 0.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    ///   `buffer` - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen` - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetStringValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the float value corresponding to `key` in `annot`'s dictionary. Writes
    /// value to `value` and returns `true` if `key` exists in the dictionary and
    /// `key`'s corresponding value is a number (`FPDF_OBJECT_NUMBER`), `false`
    /// otherwise.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    ///   `value`  - receives the value, must not be `NULL`.
    ///
    /// Returns `true` if value found.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetNumberValue(&self, annot: FPDF_ANNOTATION, key: &str, value: *mut c_float) -> FPDF_BOOL;

    /// Sets the AP (appearance string) in `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to set the AP.
    ///
    ///   `value`          - the string value to be set, encoded in UTF-16LE. If
    ///                      `nullptr` is passed, the AP is cleared for that mode. If the
    ///                      mode is Normal, APs for all modes are cleared.
    ///
    /// Returns `true` if successful.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_SetAP_str].
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_SetAP].
    ///
    /// Sets the AP (appearance string) in `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to set the AP.
    ///
    ///   `value`          - the string value to be set.
    ///
    /// Returns `true` if successful.
    ///
    /// Note that this helper function cannot clear appearance strings, since it cannot pass
    /// a null pointer for `value`. To clear an appearance string, use [PdfiumLibraryBindings::FPDFAnnot_SetAP].
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP_str(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: &str,
    ) -> FPDF_BOOL {
        self.FPDFAnnot_SetAP(
            annot,
            appearanceMode,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the AP (appearance string) from `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    /// `buffer` is only modified if `buflen` is large enough to hold the whole AP
    /// string. If `buflen` is smaller, the total size of the AP is still returned,
    /// but nothing is copied.
    ///
    /// If there is no appearance stream for `annot` in `appearanceMode`, an empty
    /// string is written to `buf` and 2 is returned.
    ///
    /// On other errors, nothing is written to `buffer` and 0 is returned.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to get the AP.
    ///
    ///   `buffer`         - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`         - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the annotation corresponding to `key` in `annot`'s dictionary. Common
    /// keys for linking annotations include "IRT" and "Popup". Must call
    /// [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the annotation returned by this function
    /// is no longer needed.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    /// Returns a handle to the linked annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLinkedAnnot(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_ANNOTATION;

    /// Gets the annotation flags of `annot`.
    ///
    ///   `annot`    - handle to an annotation.
    ///
    /// Returns the annotation flags.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFlags(&self, annot: FPDF_ANNOTATION) -> c_int;

    /// Sets the `annot`'s flags to be of the value `flags`.
    ///
    ///   `annot`      - handle to an annotation.
    ///
    ///   `flags`      - the flag values to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFlags(&self, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL;

    /// Gets the annotation flags of `annot`.
    ///
    ///    `handle`    -   the handle to the form fill module, returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`     -   handle to an interactive form annotation.
    ///
    /// Returns the annotation flags specific to interactive forms.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Sets the form field flags for an interactive form annotation.
    ///
    ///   `form`         -   the handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`        -   handle to an interactive form annotation.
    ///
    ///   `flags`        -   the form field flags to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL;

    /// Retrieves an interactive form annotation whose rectangle contains a given
    /// point on a page. Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the
    /// annotation returned is no longer needed.
    ///
    ///    `form`       -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`       -   handle to the page, returned by [PdfiumLibraryBindings::FPDF_LoadPage] function.
    ///
    ///    `point`      -   position in PDF "user space".
    ///
    /// Returns the interactive form annotation whose rectangle contains the given
    /// coordinates on the page. If there is no such annotation, return `NULL`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        point: *const FS_POINTF,
    ) -> FPDF_ANNOTATION;

    /// Gets the name of `annot`, which is an interactive form annotation.
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will
    /// be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `form`        -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the name string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the form field type of `annot`, which is an interactive form annotation.
    ///
    ///    `form`        -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    /// Returns the type of the form field (one of the `FPDF_FORMFIELD_*` values) on
    /// success. Returns -1 on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldType(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the value of `annot`, which is an interactive form annotation.
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will
    /// be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `form`        -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the number of options in the `annot`'s "Opt" dictionary. Intended for
    /// use with listbox and combobox widget annotations.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns the number of options in "Opt" dictionary on success. Return value
    /// will be -1 if annotation does not have an "Opt" dictionary or other error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the string value for the label of the option at `index` in `annot`'s
    /// "Opt" dictionary. Intended for use with listbox and combobox widget
    /// annotations. `buffer` is only modified if `buflen` is longer than the length
    /// of contents. If index is out of range or in case of other error, nothing
    /// will be added to `buffer` and the return value will be 0. Note that
    /// return value of empty string is 2 for "\0\0".
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `index`   - numeric index of the option in the "Opt" array.
    ///
    ///   `buffer`  - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`  - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    /// If `annot` does not have an "Opt" array, `index` is out of range or if any
    /// other error occurs, returns 0.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionLabel(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Determines whether or not the option at `index` in `annot`'s "Opt" dictionary
    /// is selected. Intended for use with listbox and combobox widget annotations.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `index`   - numeric index of the option in the "Opt" array.
    ///
    /// Returns `true` if the option at `index` in `annot`'s "Opt" dictionary is selected.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsOptionSelected(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL;

    /// Gets the float value of the font size for an `annot` with variable text.
    /// If 0, the font is to be auto-sized: its size is computed as a function of
    /// the height of the annotation rectangle.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `value`   - Required. Float which will be set to font size on success.
    ///
    /// Returns `true` if the font size was set in `value`, `false` on error or if
    /// `value` not provided.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontSize(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, value: *mut c_float) -> FPDF_BOOL;

    /// Experimental API.
    /// Set the text color of an annotation.
    ///
    ///   `form`     - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`    - handle to an annotation.
    ///
    ///   `R`        - the red component for the text color.
    ///
    ///   `G`        - the green component for the text color.
    ///
    ///   `B`        - the blue component for the text color.
    ///
    /// Returns `true` if successful.
    ///
    /// Currently supported subtypes: freetext.
    /// The range for the color components is 0 to 255.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: c_uint,
        G: c_uint,
        B: c_uint,
    ) -> FPDF_BOOL;

    /// Gets the RGB value of the font color for an `annot` with variable text.
    ///
    ///   `form`         - handle to the form fill module, returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`        - handle to an annotation.
    ///
    ///   `R`, `G`, `B`  - buffer to hold the RGB value of the color. Ranges from 0 to 255.
    ///
    /// Returns `true` if the font color was set, `false` on error or if the font color
    /// was not provided.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Determines if `annot` is a form widget that is checked. Intended for use with
    /// checkbox and radio button widgets.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns `true` if `annot` is a form widget and is checked.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsChecked(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Sets the list of focusable annotation subtypes. Annotations of subtype
    /// `FPDF_ANNOT_WIDGET` are by default focusable. New subtypes set using this API
    /// will override the existing subtypes.
    ///
    ///   `form`     - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `subtypes` - list of annotation subtype which can be tabbed over.
    ///
    ///   `count`    - total number of annotation subtype in list.
    ///
    /// Returns `true` if list of annotation subtype is set successfully.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL;

    /// Gets the count of focusable annotation subtypes as set by host
    /// for a `form`.
    ///
    ///   `form`     - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// Returns the count of focusable annotation subtypes or `-1` on error.
    ///
    /// Note: Annotations of type `FPDF_ANNOT_WIDGET` are by default focusable.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypesCount(&self, form: FPDF_FORMHANDLE) -> c_int;

    /// Gets the list of focusable annotation subtype as set by host.
    ///
    ///   `form`     - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `subtypes` - receives the list of annotation subtype which can be tabbed
    ///                over. Caller must have allocated `subtypes` more than or
    ///                equal to the count obtained from
    ///                [PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypesCount] API.
    ///
    ///   `count`    - size of `subtypes`.
    ///
    /// Returns `true` on success and sets list of annotation subtype to `subtypes`.
    ///
    /// Note: Annotations of type `FPDF_ANNOT_WIDGET` are by default focusable.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL;

    /// Gets `FPDF_LINK` object for `annot`. Intended to use for link annotations.
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns `FPDF_LINK` from the `FPDF_ANNOTATION` and `NULL` on failure,
    /// if the input annot is `NULL`, or input annot's subtype is not link.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLink(&self, annot: FPDF_ANNOTATION) -> FPDF_LINK;

    /// Gets the count of annotations in the `annot`'s control group.
    ///
    /// A group of interactive form annotations is collectively called a form
    /// control group. Here, `annot`, an interactive form annotation, should be
    /// either a radio button or a checkbox.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns number of controls in its control group or `-1` on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the index of `annot` in `annot`'s control group.
    ///
    /// A group of interactive form annotations is collectively called a form
    /// control group. Here, `annot`, an interactive form annotation, should be
    /// either a radio button or a checkbox.
    ///
    ///   `form`    - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns index of a given `annot` in its control group or `-1` on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlIndex(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the export value of `annot` which is an interactive form annotation.
    ///
    /// Intended for use with radio button and checkbox widget annotations.
    ///
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value
    /// will be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `form`        -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldExportValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Add a URI action to `annot`, overwriting the existing action, if any.
    ///
    ///   `annot`  - handle to a link annotation.
    ///
    ///   `uri`    - the URI to be set, encoded in 7-bit ASCII.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetURI(&self, annot: FPDF_ANNOTATION, uri: &str) -> FPDF_BOOL;

    /// Get the attachment from `annot`.
    ///
    ///   `annot`  - handle to a file annotation.
    ///
    /// Returns the handle to the attachment object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFileAttachment(&self, annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT;

    /// Add an embedded file with `name` to `annot`.
    ///
    ///   `annot`    - handle to a file annotation.
    ///
    ///   `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or `NULL` on failure.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_AddFileAttachment_str].
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment(&self, annot: FPDF_ANNOTATION, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_AddFileAttachment].
    ///
    /// Add an embedded file with `name` to `annot`.
    ///
    ///   `annot`    - handle to a file annotation.
    ///
    ///   `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or `NULL` on failure.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment_str(&self, annot: FPDF_ANNOTATION, name: &str) -> FPDF_ATTACHMENT {
        self.FPDFAnnot_AddFileAttachment(
            annot,
            get_pdfium_utf16le_bytes_from_str(name).as_ptr() as FPDF_WIDESTRING,
        )
    }

    ///  Initializes the form fill environment.
    ///
    ///    `document` - Handle to document from [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `formInfo` - Pointer to a `FPDF_FORMFILLINFO` structure.
    ///
    /// Return Value:
    ///        Handle to the form fill module, or `NULL` on failure.
    ///
    /// Comments:
    ///        This function should be called before any form fill operation.
    ///        The `FPDF_FORMFILLINFO` passed in via `form_info` must remain valid until
    ///        the returned `FPDF_FORMHANDLE` is closed.
    #[allow(non_snake_case)]
    fn FPDFDOC_InitFormFillEnvironment(
        &self,
        document: FPDF_DOCUMENT,
        form_info: *mut FPDF_FORMFILLINFO,
    ) -> FPDF_FORMHANDLE;

    /// Takes ownership of `form` and exits the form fill environment.
    ///
    ///    `form`     -   Handle to the form fill module, as returned by
    ///                   [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// This function is a no-op when `form` is null.
    #[allow(non_snake_case)]
    fn FPDFDOC_ExitFormFillEnvironment(&self, form: FPDF_FORMHANDLE);

    /// This method is required for implementing all the form related
    /// functions. Should be invoked after user successfully loaded a
    /// PDF page, and [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment] has been invoked.
    ///
    ///    `form`      -   Handle to the form fill module, as returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    #[allow(non_snake_case)]
    fn FORM_OnAfterLoadPage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE);

    /// This method is required for implementing all the form related
    /// functions. Should be invoked before user closes the PDF page.
    ///
    ///    `page`      -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `form`      -   Handle to the form fill module, as returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    #[allow(non_snake_case)]
    fn FORM_OnBeforeClosePage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE);

    /// Gets the document's page mode.
    ///
    ///    `doc` - Handle to document.
    ///
    /// Returns one of the `PAGEMODE_*` flags defined above.
    /// The page mode defines how the document should be initially displayed.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetPageMode(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Flattens annotations and form fields into the page contents.
    ///
    ///    `page`  - handle to the page.
    ///
    ///    `nFlag` - One of the `FLAT_*` values denoting the page usage.
    ///
    /// Returns one of the `FLATTEN_*` values. Currently, all failures return `FLATTEN_FAIL`
    /// with no indication of the cause.
    #[allow(non_snake_case)]
    fn FPDFPage_Flatten(&self, page: FPDF_PAGE, nFlag: c_int) -> c_int;

    /// This method is required for performing document-level JavaScript actions.
    /// It should be invoked after the PDF document has been loaded.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// If there is document-level JavaScript action embedded in the document, this method
    /// will execute the JavaScript action. Otherwise, the method will do nothing.
    #[allow(non_snake_case)]
    fn FORM_DoDocumentJSAction(&self, form: FPDF_FORMHANDLE);

    /// This method is required for performing open-action when the document is opened.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// This method will do nothing if there are no open-actions embedded in the document.
    #[allow(non_snake_case)]
    fn FORM_DoDocumentOpenAction(&self, form: FPDF_FORMHANDLE);

    /// This method is required for performing the document's additional-action.
    ///
    ///    `form`        -   Handle to the form fill module. Returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `aaType`      -   The type of the additional-actions which are defined above.
    ///
    /// This method will do nothing if there is no document additional-action corresponding
    /// to the specified `aaType`.
    #[allow(non_snake_case)]
    fn FORM_DoDocumentAAction(&self, form: FPDF_FORMHANDLE, aaType: c_int);

    /// This method is required for performing the page object's additional-action when
    /// opened or closed.
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `aaType`      -   The type of the page object's additional-actions
    ///                      which are defined above.
    ///
    /// This method will do nothing if no additional-action corresponding to the specified
    /// `aaType` exists.
    #[allow(non_snake_case)]
    fn FORM_DoPageAAction(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE, aaType: c_int);

    /// Call this member function when the mouse cursor moves.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`    -   Indicates whether various virtual keys are down.
    ///
    ///    `page_x`      -   Specifies the x-coordinate of the cursor in PDF user space.
    ///
    ///    `page_y`      -   Specifies the y-coordinate of the cursor in PDF user space.
    ///
    /// Returns `true` on  success.
    #[allow(non_snake_case)]
    fn FORM_OnMouseMove(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Call this member function when the user scrolls the mouse wheel.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`    -   Indicates whether various virtual keys are down.
    ///
    ///    `page_coord`  -   Specifies the coordinates of the cursor in PDF user space.
    ///
    ///    `delta_x`     -   Specifies the amount of wheel movement on the x-axis,
    ///                      in units of platform-agnostic wheel deltas. Negative
    ///                      values mean left.
    ///
    ///    `delta_y`     -   Specifies the amount of wheel movement on the y-axis,
    ///                      in units of platform-agnostic wheel deltas. Negative
    ///                      values mean down.
    ///
    /// Returns `true` indicates success.
    ///
    /// For `delta_x` and `delta_y`, the caller must normalize platform-specific wheel deltas,
    /// e.g. on Windows, a delta value of `240` for a `WM_MOUSEWHEEL` event normalizes to `2`,
    /// since Windows defines `WHEEL_DELTA` as 120.
    #[allow(non_snake_case)]
    fn FORM_OnMouseWheel(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_coord: *const FS_POINTF,
        delta_x: c_int,
        delta_y: c_int,
    ) -> FPDF_BOOL;

    /// This function focuses the form annotation at a given point. If the annotation at the
    /// point already has focus, nothing happens. If there is no annotation at the point,
    /// removes form focus.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`    -   Indicates whether various virtual keys are down.
    ///
    ///    `page_x`      -   Specifies the x-coordinate of the cursor in PDF user space.
    ///
    ///    `page_y`      -   Specifies the y-coordinate of the cursor in PDF user space.
    ///
    /// Returns `true` if there is an annotation at the given point and it has focus.
    #[allow(non_snake_case)]
    fn FORM_OnFocus(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Call this member function when the user presses the left mouse button.
    ///
    ///    `form`       -   Handle to the form fill module, as returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`       -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`   -   Indicates whether various virtual keys are down.
    ///
    ///    `page_x`     -   Specifies the x-coordinate of the cursor in PDF user space.
    ///
    ///    `page_y`     -   Specifies the y-coordinate of the cursor in PDF user space.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Same as [PdfiumLibraryBindings::FORM_OnLButtonDown], execpt for the right mouse button.
    ///
    /// At the present time, has no effect except in XFA builds, but is included for the sake
    /// of symmetry.
    #[allow(non_snake_case)]
    fn FORM_OnRButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Call this member function when the user releases the left mouse button.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`    -   Indicates whether various virtual keys are down.
    ///
    ///    `page_x`      -   Specifies the x-coordinate of the cursor in device coordinates.
    ///
    ///    `page_y`      -   Specifies the y-coordinate of the cursor in device coordinates.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_OnLButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Same as [PdfiumLibraryBindings::FORM_OnLButtonUp], execpt for the right mouse button.
    ///
    /// At the present time, has no effect except in XFA builds, but is included for the sake
    /// of symmetry.
    #[allow(non_snake_case)]
    fn FORM_OnRButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Call this member function when the user double clicks the left mouse button.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `modifier`    -   Indicates whether various virtual keys are down.
    ///
    ///    `page_x`      -   Specifies the x-coordinate of the cursor in PDF user space.
    ///
    ///    `page_y`      -   Specifies the y-coordinate of the cursor in PDF user space.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDoubleClick(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    /// Call this member function when a non-system key is pressed.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `nKeyCode`    -   The virtual-key code of the given key (see `fpdf_fwlevent.h`
    ///                      for virtual key codes).
    ///
    ///    `modifier`    -   Mask of key flags (see `fpdf_fwlevent.h` for key flag values).
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_OnKeyDown(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL;

    /// Call this member function when a non-system key is released.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `nKeyCode`    -   The virtual-key code of the given key (see `fpdf_fwlevent.h`
    ///                      for virtual key codes).
    ///
    ///    `modifier`    -   Mask of key flags (see `fpdf_fwlevent.h` for key flag values).
    ///
    /// Returns `true` on success.
    ///
    /// Note: currently unimplemented, always returns `false`. PDFium reserves this API
    /// and may implement it in the future on an as-needed basis.
    #[allow(non_snake_case)]
    fn FORM_OnKeyUp(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL;

    /// Call this member function when a keystroke translates to a non-system character.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `nChar`       -   The character code value itself.
    ///
    ///    `modifier`    -   Mask of key flags (see `fpdf_fwlevent.h` for key flag values).
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_OnChar(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nChar: c_int, modifier: c_int) -> FPDF_BOOL;

    /// Call this function to obtain the text within the current focused field, if any.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `buffer`      -   Buffer for holding the form text, encoded in UTF-16LE.
    ///                      If `NULL`, `buffer` is not modified.
    ///
    ///    `buflen`      -   Length of `buffer` in bytes. If `buflen` is less than the length
    ///                      of the form text string, `buffer` is not modified.
    ///
    /// Returns the length in bytes of the text in the focused field.
    #[allow(non_snake_case)]
    fn FORM_GetFocusedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Call this function to obtain selected text within a form text field or
    /// form combo-box text field.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `buffer`      -   Buffer for holding the selected text, encoded in UTF-16LE.
    ///                      If `NULL`, `buffer` is not modified.
    ///
    ///    `buflen`      -   Length of `buffer` in bytes. If `buflen` is less than the length
    ///                      of the selected text string, `buffer` is not modified.
    ///
    /// Returns the length in bytes of selected text in form text field or form combo-box text field.
    #[allow(non_snake_case)]
    fn FORM_GetSelectedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Call this function to replace the selected text in a form text field or
    /// user-editable form combo-box text field with another text string
    /// (which can be empty or non-empty). If there is no selected text, this function will
    /// append the replacement text after the current caret position. After the insertion,
    /// the inserted text will be selected.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `wsText`      -   The text to be inserted, in UTF-16LE format.
    #[allow(non_snake_case)]
    fn FORM_ReplaceAndKeepSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING);

    /// Call this function to replace the selected text in a form text field or
    /// user-editable form combo-box text field with another text string
    /// (which can be empty or non-empty). If there is no selected text, this function
    /// will append the replacement text after the current caret position. After the insertion,
    /// the selection range will be set to empty.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `wsText`      -   The text to be inserted, in UTF-16LE format.
    #[allow(non_snake_case)]
    fn FORM_ReplaceSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING);

    /// Call this function to select all the text within the currently focused form text field
    /// or form combo-box text field.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FORM_SelectAllText(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Finds out if it is possible for the current focused widget in a given form to perform
    /// an undo operation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns `true` if it is possible to undo.
    #[allow(non_snake_case)]
    fn FORM_CanUndo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Finds out if it is possible for the current focused widget in a given form to perform
    /// a redo operation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns `true` if it is possible to redo.
    #[allow(non_snake_case)]
    fn FORM_CanRedo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Makes the current focused widget perform an undo operation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns `true` if the undo operation succeeded.
    #[allow(non_snake_case)]
    fn FORM_Undo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Makes the current focused widget perform a redo operation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns `true` if the redo operation succeeded.
    #[allow(non_snake_case)]
    fn FORM_Redo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    /// Calls this member function to force to kill the focus of the form field which has focus.
    /// If it would kill the focus of a form field, saves the value of form field if was
    /// changed by the user.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FORM_ForceToKillFocus(&self, form: FPDF_FORMHANDLE) -> FPDF_BOOL;

    /// Calls this member function to get the currently focused annotation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page_index`  -   Buffer to hold the index number of the page which contains
    ///                      the focused annotation. `0` for the first page. Can't be `NULL`.
    ///
    ///    `annot`       -   Buffer to hold the focused annotation. Can't be `NULL`.
    ///
    /// On success, returns `true` and writes to the out parameters.
    /// Otherwise returns `false` and leaves the out parameters unmodified.
    /// Will return `true` and set `page_index` to `-1` and `annot` to `NULL`
    /// if there is no focused annotation.
    ///
    /// Note: not currently supported for XFA forms - will report no focused annotation.
    /// Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the annotation returned
    /// in `annot` by this function is no longer needed.
    #[allow(non_snake_case)]
    fn FORM_GetFocusedAnnot(
        &self,
        form: FPDF_FORMHANDLE,
        page_index: *mut c_int,
        annot: *mut FPDF_ANNOTATION,
    ) -> FPDF_BOOL;

    /// Calls this member function to set the currently focused annotation.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   Handle to an annotation.
    ///
    /// Returns `true` on success.
    ///
    /// Note: `annot` must not be `NULL`. To kill focus, use
    /// [PdfiumLibraryBindings::FORM_ForceToKillFocus] instead.
    #[allow(non_snake_case)]
    fn FORM_SetFocusedAnnot(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Gets the form field type by point.
    ///
    ///    `form`        -   Handle to the form fill module. Returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `page_x`      -   X position in PDF user space.
    ///
    ///    `page_y`      -   Y position in PDF user space.
    ///
    /// Returns the type of the form field. `-1` indicates no field at the given point.
    /// See field types above.
    #[allow(non_snake_case)]
    fn FPDFPage_HasFormFieldAtPoint(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int;

    /// Gets the form field z-order by point.
    ///
    ///    `form`        -   Handle to the form fill module. Returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `page_x`      -   X position in PDF user space.
    ///
    ///    `page_y`      -   Y position in PDF user space.
    ///
    /// Returns the z-order of the form field. `-1` indicates no field.
    /// Higher numbers are closer to the front.
    #[allow(non_snake_case)]
    fn FPDFPage_FormFieldZOrderAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        page_x: f64,
        page_y: f64,
    ) -> c_int;

    /// Sets the highlight color of the specified (or all) form fields in the document.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `doc`         -   Handle to the document, as returned by
    ///                      [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `fieldType`   -   A 32-bit integer indicating the type of a form field (defined above).
    ///
    ///    `color`       -   The highlight color of the form field. Constructed by `0xxxrrggbb`.
    ///
    /// When the parameter `fieldType` is set to `FPDF_FORMFIELD_UNKNOWN`,
    /// the highlight color will be applied to all the form fields in the document.
    /// Please refresh the client window to show the highlight immediately if necessary.
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightColor(&self, form: FPDF_FORMHANDLE, field_type: c_int, color: FPDF_DWORD);

    /// Sets the transparency of the form field highlight color in the document.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `doc`         -   Handle to the document, as returned by
    ///                      [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `alpha`       -   The transparency of the form field highlight color, between `0` - `255`.
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightAlpha(&self, form: FPDF_FORMHANDLE, alpha: c_uchar);

    /// Removes the form field highlight color in the document.
    ///
    ///    `form`        -   Handle to the form fill module, as returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// Please refresh the client window to remove the highlight immediately if necessary.
    #[allow(non_snake_case)]
    fn FPDF_RemoveFormFieldHighlight(&self, form: FPDF_FORMHANDLE);

    /// Renders form fields and pop-up windows on a page to a device independent bitmap.
    ///
    ///    `form`         -   Handle to the form fill module, as returned by
    ///                       [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `bitmap`       -   Handle to the device independent bitmap (as the output
    ///                       buffer). Bitmap handles can be created by
    ///                       [PdfiumLibraryBindings::FPDFBitmap_Create].
    ///
    ///    `page`         -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `start_x`      -   Left pixel position of the display area in the device coordinates.
    ///
    ///    `start_y`      -   Top pixel position of the display area in the device coordinates.
    ///
    ///    `size_x`       -   Horizontal size (in pixels) for displaying the page.
    ///
    ///    `size_y`       -   Vertical size (in pixels) for displaying the page.
    ///
    ///    `rotate`       -   Page orientation: `0` (normal), `1` (rotated 90 degrees clockwise),
    ///                       `2` (rotated 180 degrees), `3` (rotated 90 degrees counter-clockwise).
    ///
    ///    `flags`        -   `0` for normal display, or combination of flags defined above.
    ///
    /// This function is designed to render annotations that are user-interactive,
    /// which are widget annotations (for form fields) and pop-up annotations.
    /// With the `FPDF_ANNOT` flag, this function will render a pop-up annotation
    /// when users mouse-hover on a non-widget annotation. Regardless of `FPDF_ANNOT` flag,
    /// this function will always render widget annotations for form fields.
    /// In order to implement the form fill functions, implementation should call this function
    /// after rendering functions, such as [PdfiumLibraryBindings::FPDF_RenderPageBitmap]
    /// or [PdfiumLibraryBindings::FPDF_RenderPageBitmap_Start], have finished rendering
    /// the page contents.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_FFLDraw(
        &self,
        form: FPDF_FORMHANDLE,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    /// Returns the type of form contained in the PDF document.
    ///
    ///    `document` - Handle to document.
    ///
    /// Returns an integer value representing one of the `FORMTYPE_*` values.
    /// If `document` is `NULL`, then the return value is `FORMTYPE_NONE`.
    #[allow(non_snake_case)]
    fn FPDF_GetFormType(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Selects or deselects the value at the given `index` of the focused annotation.
    ///
    ///    `form`        -   Handle to the form fill module. Returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `index`       -   `0`-based index of value to be set as selected or unselected.
    ///
    ///    `selected`    -   `true` to select, `false` to deselect.
    ///
    /// Returns `true` if the operation succeeded, `false` if the operation failed or
    /// the widget is not a supported type.
    ///
    /// Intended for use with listbox or combo-box widget types. Default implementation is
    /// a no-op that will return `false` for widget other types. Not currently supported for
    /// XFA forms - will return `false`. Combo-boxes have at most a single value selected at
    /// a time which cannot be deselected. Deselect on a combo-box is a no-op that returns `false`.
    #[allow(non_snake_case)]
    fn FORM_SetIndexSelected(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        index: c_int,
        selected: FPDF_BOOL,
    ) -> FPDF_BOOL;

    /// Returns whether or not the value at `index` of the focused annotation is currently selected.
    ///
    ///    `form`        -   Handle to the form fill module. Returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`        -   Handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `index`       -   `0`-based index of value to check.
    ///
    /// Returns `true`if value at `index` is currently selected, `false` if value at `index`
    /// is not selected or widget is not a supported type.
    ///
    /// Intended for use with listbox or combo-box widget types. Default implementation is
    /// a no-op that will return `false` for other types. Not currently supported for
    /// XFA forms - will return `false`.
    #[allow(non_snake_case)]
    fn FORM_IsIndexSelected(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL;

    /// If the document consists of XFA fields, call this method to attempt to load XFA fields.
    ///
    ///    `document`     -   Handle to document from [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    /// Returns `true` upon success. If XFA support is not built into
    /// PDFium, performs no action and always returns `false`.
    #[allow(non_snake_case)]
    fn FPDF_LoadXFA(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    /// Gets the number of JavaScript actions in `document`.
    ///
    ///    `document` - handle to a document.
    ///
    /// Returns the number of JavaScript actions in `document` or `-1` on error.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptActionCount(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Gets the JavaScript action at `index` in `document`.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `index`    - the index of the requested JavaScript action.
    ///
    /// Returns the handle to the JavaScript action, or `NULL` on failure.
    ///
    /// Caller owns the returned handle and must close it with
    /// [PdfiumLibraryBindings::FPDFDoc_CloseJavaScriptAction].
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptAction(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_JAVASCRIPT_ACTION;

    /// Closes a loaded `FPDF_JAVASCRIPT_ACTION` object.
    ///
    ///    `javascript` - Handle to a JavaScript action.
    #[allow(non_snake_case)]
    fn FPDFDoc_CloseJavaScriptAction(&self, javascript: FPDF_JAVASCRIPT_ACTION);

    /// Gets the name from the `javascript` handle. `buffer` is only modified if
    /// `buflen` is longer than the length of the name. On errors, `buffer` is
    /// unmodified and the returned length is `0`.
    ///
    ///    `javascript` - handle to an JavaScript action.
    ///
    ///    `buffer`     - buffer for holding the name, encoded in UTF-16LE.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    /// Returns the length of the JavaScript action name in bytes.
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetName(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the script from the `javascript` handle. `buffer` is only modified if
    /// `buflen` is longer than the length of the script. On errors, `buffer` is
    /// unmodified and the returned length is `0`.
    ///
    ///    `javascript` - handle to an JavaScript action.
    ///
    ///    `buffer`     - buffer for holding the name, encoded in UTF-16LE.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    /// Returns the length of the JavaScript action name in bytes.
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetScript(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Returns a pointer to the default character set to TT Font name map. The map is an array of
    /// FPDF_CharsetFontMap structs, with its end indicated by a { -1, NULL } entry.
    /// Returns a pointer to the Charset Font Map. Note: once [PdfiumLibraryBindings::FPDF_GetDefaultTTFMapCount]
    /// and [PdfiumLibraryBindings::FPDF_GetDefaultTTFMapEntry] are no longer experimental,
    /// this API will be marked as deprecated. See: <https://crbug.com/348468114>
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMap(&self) -> *const FPDF_CharsetFontMap;

    /// Returns the number of entries in the default character set to TT Font name map.
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapCount(&self) -> usize;

    /// Returns an entry in the default character set to TT Font name map.
    ///
    ///    `index`    -   The index to the entry in the map to retrieve.
    ///
    /// Returns a pointer to the entry, if it is in the map, or `NULL` if the index is out
    /// of bounds.
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapEntry(&self, index: usize) -> *const FPDF_CharsetFontMap;

    /// Adds a system font to the list in PDFium.
    ///
    /// This function is only called during the system font list building process.
    ///
    ///    `mapper`    -   Opaque pointer to Foxit font mapper.
    ///
    ///    `face`      -   The font face name.
    ///
    ///    `charset`   -   Font character set. See above defined constants.
    #[allow(non_snake_case)]
    fn FPDF_AddInstalledFont(&self, mapper: *mut c_void, face: &str, charset: c_int);

    /// Sets the system font info interface into PDFium.
    ///
    ///    `font_info` -   Pointer to a `FPDF_SYSFONTINFO` structure.
    ///
    /// Platform support implementation should implement required methods of
    /// `FFDF_SYSFONTINFO` interface, then call this function during PDFium initialization
    /// process.
    ///
    /// Call this with `NULL` to tell PDFium to stop using a previously set `FPDF_SYSFONTINFO`.
    #[allow(non_snake_case)]
    fn FPDF_SetSystemFontInfo(&self, font_info: *mut FPDF_SYSFONTINFO);

    /// Gets default system font info interface for current platform.
    ///
    /// Returns a pointer to a `FPDF_SYSFONTINFO` structure describing the default
    /// interface, or `NULL` if the platform doesn't have a default interface.
    ///
    /// Application should call [PdfiumLibraryBindings::FPDF_FreeDefaultSystemFontInfo]
    /// to free the returned pointer. For some platforms, PDFium implements a default version
    /// of system font info interface. The default implementation can be passed to
    /// [PdfiumLibraryBindings::FPDF_SetSystemFontInfo].
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultSystemFontInfo(&self) -> *mut FPDF_SYSFONTINFO;

    /// Frees a default system font info interface.
    ///
    ///    `font_info`   -   Pointer to a `FPDF_SYSFONTINFO` structure.
    ///
    /// This function should be called on the output from
    /// [PdfiumLibraryBindings::FPDF_GetDefaultSystemFontInfo] once it is no longer needed.
    #[allow(non_snake_case)]
    fn FPDF_FreeDefaultSystemFontInfo(&self, font_info: *mut FPDF_SYSFONTINFO);

    /// Gets the first child of `bookmark`, or the first top-level bookmark item.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the current bookmark. Pass `NULL` for the first top
    ///                level item.
    ///
    /// Returns a handle to the first child of `bookmark` or the first top-level
    /// bookmark item. `NULL` if no child or top-level bookmark found.
    /// Note that another name for the bookmarks is the document outline, as
    /// described in ISO 32000-1:2008, section 12.3.3.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetFirstChild(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK;

    /// Gets the next sibling of `bookmark`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the current bookmark.
    ///
    /// Returns a handle to the next sibling of `bookmark`, or `NULL` if this is the
    /// last bookmark at this level.
    ///
    /// Note that the caller is responsible for handling circular bookmark
    /// references, as may arise from malformed documents.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetNextSibling(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK;

    /// Gets the title of `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    ///   `buffer`   - buffer for the title. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer in bytes. May be 0.
    ///
    /// Returns the number of bytes in the title, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and
    /// `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding. The
    /// string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetTitle(&self, bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: c_ulong) -> c_ulong;

    /// Gets the number of children of `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns a signed integer that represents the number of sub-items the given
    /// bookmark has. If the value is positive, child items shall be shown by default
    /// (open state). If the value is negative, child items shall be hidden by
    /// default (closed state). Please refer to PDF 32000-1:2008, Table 153.
    /// Returns 0 if the bookmark has no children or is invalid.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetCount(&self, bookmark: FPDF_BOOKMARK) -> c_int;

    /// Finds the bookmark with `title` in `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `title`    - the UTF-16LE encoded Unicode title for which to search.
    ///
    /// Returns the handle to the bookmark, or `NULL` if `title` can't be found.
    ///
    /// `FPDFBookmark_Find()` will always return the first bookmark found even if
    /// multiple bookmarks have the same `title`.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFBookmark_Find_str].
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find(&self, document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFBookmark_Find].
    ///
    /// Finds the bookmark with `title` in `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `title`    - the title for which to search.
    ///
    /// Returns the handle to the bookmark, or `NULL` if `title` can't be found.
    ///
    /// `FPDFBookmark_Find_str()` will always return the first bookmark found even if
    /// multiple bookmarks have the same `title`.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find_str(&self, document: FPDF_DOCUMENT, title: &str) -> FPDF_BOOKMARK {
        self.FPDFBookmark_Find(
            document,
            get_pdfium_utf16le_bytes_from_str(title).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the destination associated with `bookmark`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns the handle to the destination data, or `NULL` if no destination is
    /// associated with `bookmark`.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetDest(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST;

    /// Gets the action associated with `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns the handle to the action data, or `NULL` if no action is associated
    /// with `bookmark`.
    ///
    /// If this function returns a valid handle, it is valid as long as `bookmark` is
    /// valid.
    ///
    /// If this function returns `NULL`, `FPDFBookmark_GetDest()` should be called to get
    /// the `bookmark` destination data.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetAction(&self, bookmark: FPDF_BOOKMARK) -> FPDF_ACTION;

    /// Gets the type of `action`.
    ///
    ///   `action` - handle to the action.
    ///
    /// Returns one of:
    ///   - `PDFACTION_UNSUPPORTED`
    ///   - `PDFACTION_GOTO`
    ///   - `PDFACTION_REMOTEGOTO`
    ///   - `PDFACTION_URI`
    ///   - `PDFACTION_LAUNCH`
    #[allow(non_snake_case)]
    fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong;

    /// Gets the destination of `action`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `action`   - handle to the action. `action` must be a `PDFACTION_GOTO` or
    ///                `PDFACTION_REMOTEGOTO`.
    ///
    /// Returns a handle to the destination data, or `NULL` on error, typically
    /// because the arguments were bad or the action was of the wrong type.
    ///
    /// In the case of `PDFACTION_REMOTEGOTO`, you must first call
    /// `FPDFAction_GetFilePath()`, then load the document at that path, then pass
    /// the document handle from that document as `document` to `FPDFAction_GetDest()`.
    #[allow(non_snake_case)]
    fn FPDFAction_GetDest(&self, document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST;

    /// Gets the file path of `action`.
    ///
    ///   `action` - handle to the action. `action` must be a `PDFACTION_LAUNCH` or
    ///              `PDFACTION_REMOTEGOTO`.
    ///
    ///   `buffer` - a buffer for output the path string. May be `NULL`.
    ///
    ///   `buflen` - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the file path, including the trailing `NUL`
    /// character, or 0 on error, typically because the arguments were bad or the
    /// action was of the wrong type.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-8 encoding.
    /// If `buflen` is less than the returned length, or `buffer` is `NULL`, `buffer`
    /// will not be modified.
    #[allow(non_snake_case)]
    fn FPDFAction_GetFilePath(&self, action: FPDF_ACTION, buffer: *mut c_void, buflen: c_ulong) -> c_ulong;

    /// Gets the URI path of `action`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `action`   - handle to the action. Must be a `PDFACTION_URI`.
    ///
    ///   `buffer`   - a buffer for the path string. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the URI path, including the trailing `NUL`
    /// character, or 0 on error, typically because the arguments were bad or the
    /// action was of the wrong type.
    ///
    /// The `buffer` may contain badly encoded data. The caller should validate the
    /// output, i.e. check to see if it is UTF-8.
    ///
    /// If `buflen` is less than the returned length, or `buffer` is `NULL`, buffer`
    /// will not be modified.
    ///
    /// Historically, the documentation for this API claimed `buffer` is always
    /// encoded in 7-bit ASCII, but did not actually enforce it.
    /// <https://pdfium.googlesource.com/pdfium.git/+/d609e84cee2e14a18333247485af91df48a40592>
    /// added that enforcement, but that did not work well for real world PDFs that
    /// used UTF-8. As of this writing, this API reverted back to its original
    /// behavior prior to commit d609e84cee.
    #[allow(non_snake_case)]
    fn FPDFAction_GetURIPath(
        &self,
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the page index of `dest`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `dest`     - handle to the destination.
    ///
    /// Returns the 0-based page index containing `dest`. Returns -1 on error.
    #[allow(non_snake_case)]
    fn FPDFDest_GetDestPageIndex(&self, document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int;

    /// Gets the view (fit type) specified by `dest`.
    ///
    ///   `dest`         - handle to the destination.
    ///
    ///   `pNumParams`   - receives the number of view parameters, which is at most 4.
    ///
    ///   `pParams`      - buffer to write the view parameters. Must be at least 4
    ///                    `FS_FLOAT`s long.
    ///
    /// Returns one of the `PDFDEST_VIEW_*` constants, or `PDFDEST_VIEW_UNKNOWN_MODE` if
    /// `dest` does not specify a view.
    #[allow(non_snake_case)]
    fn FPDFDest_GetView(&self, dest: FPDF_DEST, pNumParams: *mut c_ulong, pParams: *mut FS_FLOAT) -> c_ulong;

    /// Gets the (`x`, `y`, `zoom`) location of `dest` in the destination page, if the
    /// destination is in `page /XYZ x y zoom` syntax.
    ///
    ///   `dest`       - handle to the destination.
    ///
    ///   `hasXVal`    - out parameter; true if the `x` value is not null
    ///
    ///   `hasYVal`    - out parameter; true if the `y` value is not null
    ///
    ///   `hasZoomVal` - out parameter; true if the `zoom` value is not null
    ///
    ///   `x`          - out parameter; the `x` coordinate, in page coordinates.
    ///
    ///   `y`          - out parameter; the `y` coordinate, in page coordinates.
    ///
    ///   `zoom`       - out parameter; the `zoom` value.
    ///
    /// Returns `true` on successfully reading the `/XYZ` value.
    ///
    /// Note the `x`, `y`, `zoom` values are only set if the corresponding `hasXVal`,
    /// `hasYVal`, or `hasZoomVal` flags are true.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFDest_GetLocationInPage(
        &self,
        dest: FPDF_DEST,
        hasXVal: *mut FPDF_BOOL,
        hasYVal: *mut FPDF_BOOL,
        hasZoomVal: *mut FPDF_BOOL,
        x: *mut FS_FLOAT,
        y: *mut FS_FLOAT,
        zoom: *mut FS_FLOAT,
    ) -> FPDF_BOOL;

    /// Finds a link at point (`x`, `y`) on `page`.
    ///
    ///   `page` - handle to the document page.
    ///
    ///   `x`    - the `x` coordinate, in the page coordinate system.
    ///
    ///   `y`    - the `y` coordinate, in the page coordinate system.
    ///
    /// Returns a handle to the link, or `NULL` if no link found at the given point.
    ///
    /// You can convert coordinates from screen coordinates to page coordinates using
    /// `FPDF_DeviceToPage()`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK;

    /// Finds the Z-order of link at point (`x`, `y`) on `page`.
    ///
    ///   `page` - handle to the document page.
    ///
    ///   `x`    - the `x` coordinate, in the page coordinate system.
    ///
    ///   `y`    - the `y` coordinate, in the page coordinate system.
    ///
    /// Returns the Z-order of the link, or -1 if no link found at the given point.
    /// Larger Z-order numbers are closer to the front.
    ///
    /// You can convert coordinates from screen coordinates to page coordinates using
    /// `FPDF_DeviceToPage()`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkZOrderAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> c_int;

    /// Gets destination info for `link`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `link`     - handle to the link.
    ///
    /// Returns a handle to the destination, or `NULL` if there is no destination
    /// associated with the link. In this case, you should call `FPDFLink_GetAction()`
    /// to retrieve the action associated with `link`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetDest(&self, document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST;

    /// Gets action info for `link`.
    ///
    ///   `link` - handle to the link.
    ///
    /// Returns a handle to the action associated to `link`, or `NULL` if no action.
    /// If this function returns a valid handle, it is valid as long as `link` is
    /// valid.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION;

    /// Enumerates all the link annotations in `page`.
    ///
    ///   `page`       - handle to the page.
    ///
    ///   `start_pos`  - the start position, should initially be 0 and is updated with
    ///                  the next start position on return.
    ///
    ///   `link_annot` - the link handle for `startPos`.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_Enumerate(&self, page: FPDF_PAGE, start_pos: *mut c_int, link_annot: *mut FPDF_LINK) -> FPDF_BOOL;

    /// Gets `FPDF_ANNOTATION` object for `link_annot`.
    ///
    ///   `page`       - handle to the page in which `FPDF_LINK` object is present.
    ///
    ///   `link_annot` - handle to link annotation.
    ///
    /// Returns `FPDF_ANNOTATION` from the `FPDF_LINK` or `NULL` on failure,
    /// if the input link annot or page is `NULL`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnot(&self, page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION;

    /// Gets the rectangle for `link_annot`.
    ///
    ///   `link_annot` - handle to the link annotation.
    ///
    ///   `rect`       - the annotation rectangle.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the count of quadrilateral points to the `link_annot`.
    ///
    ///   `link_annot` - handle to the link annotation.
    ///
    /// Returns the count of quadrilateral points.
    #[allow(non_snake_case)]
    fn FPDFLink_CountQuadPoints(&self, link_annot: FPDF_LINK) -> c_int;

    /// Gets the quadrilateral points for the specified `quad_index` in `link_annot`.
    ///
    ///   `link_annot`  - handle to the link annotation.
    ///
    ///   `quad_index`  - the specified quad point index.
    ///
    ///   `quad_points` - receives the quadrilateral points.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_GetQuadPoints(
        &self,
        link_annot: FPDF_LINK,
        quad_index: c_int,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Gets an additional-action from `page`.
    ///
    ///   `page`      - handle to the page, as returned by `FPDF_LoadPage()`.
    ///
    ///   `aa_type`   - the type of the page object's additional-action, defined
    ///                 in `public/fpdf_formfill.h`
    ///
    ///   Returns the handle to the action data, or `NULL` if there is no
    ///   additional-action of type `aa_type`.
    ///
    ///   If this function returns a valid handle, it is valid as long as `page` is
    ///   valid.
    #[allow(non_snake_case)]
    fn FPDF_GetPageAAction(&self, page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION;

    /// Gets the file identifier defined in the trailer of `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `id_type`  - the file identifier type to retrieve.
    ///
    ///   `buffer`   - a buffer for the file identifier. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the file identifier, including the `NUL`
    /// terminator.
    ///
    /// The `buffer` is always a byte string. The `buffer` is followed by a `NUL`
    /// terminator.  If `buflen` is less than the returned length, or `buffer` is
    /// `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_GetFileIdentifier(
        &self,
        document: FPDF_DOCUMENT,
        id_type: FPDF_FILEIDTYPE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets meta-data `tag` content from `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `tag`      - the tag to retrieve. The tag can be one of:
    ///                Title, Author, Subject, Keywords, Creator, Producer,
    ///                CreationDate, or ModDate.
    ///                For detailed explanations of these tags and their respective
    ///                values, please refer to PDF Reference 1.6, section 10.2.1,
    ///                "Document Information Dictionary".
    ///
    ///   `buffer`   - a buffer for the tag. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the tag, including trailing zeros.
    ///
    /// The |buffer| is always encoded in UTF-16LE. The `buffer` is followed by two
    /// bytes of zeros indicating the end of the string.  If `buflen` is less than
    /// the returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    /// For linearized files, `FPDFAvail_IsFormAvail()` must be called before this, and
    /// it must have returned `PDF_FORM_AVAIL` or `PDF_FORM_NOTEXIST`. Before that, there
    /// is no guarantee the metadata has been loaded.
    #[allow(non_snake_case)]
    fn FPDF_GetMetaText(&self, document: FPDF_DOCUMENT, tag: &str, buffer: *mut c_void, buflen: c_ulong) -> c_ulong;

    /// Gets the page label for `page_index` from `document`.
    ///
    ///   `document`    - handle to the document.
    ///
    ///   `page_index`  - the 0-based index of the page.
    ///
    ///   `buffer`      - a buffer for the page label. May be `NULL`.
    ///
    ///   `buflen`      - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the page label, including trailing zeros.
    ///
    /// The `buffer` is always encoded in UTF-16LE. The `buffer` is followed by two
    /// bytes of zeros indicating the end of the string.  If `buflen` is less than
    /// the returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_GetPageLabel(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Prepares information about all characters in a page.
    ///
    ///    `page`    -   handle to the page. Returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Returns a handle to the text page information structure, or `NULL` if something goes wrong.
    ///
    /// Application must call [PdfiumLibraryBindings::FPDFText_ClosePage] to release the
    /// text page information.
    #[allow(non_snake_case)]
    fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE;

    /// Releases all resources allocated for a text page information structure.
    ///
    ///    `text_page`   -   handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    #[allow(non_snake_case)]
    fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE);

    /// Gets the number of characters in a page.
    ///
    ///    `text_page`   -   handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    /// Returns the number of characters in the page, or `-1` for error.
    /// Generated characters, like additional space characters, new line characters,
    /// are also counted.
    ///
    /// Characters in a page form a "stream"; inside the stream, each character has an index.
    /// We will use the index parameters in many of the `FPDFTEXT` functions. The
    /// first character in the page has an index value of zero.
    #[allow(non_snake_case)]
    fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int;

    /// Gets the Unicode of a character in a page.
    ///
    ///    `text_page`   -   handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   zero-based index of the character.
    ///
    /// Returns the Unicode of the particular character. If a character is not encoded in
    /// Unicode and Foxit engine can't convert to Unicode, the return value will be zero.
    #[allow(non_snake_case)]
    fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint;

    /// Gets the `FPDF_PAGEOBJECT` associated with a given character.
    ///
    ///    `text_page`   -   handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   zero-based index of the character.
    ///
    /// Returns the associated text object for the character at `index`, or `NULL` on error.
    /// The returned text object, if non-`NULL`, is of type `FPDF_PAGEOBJ_TEXT`.
    /// The caller does not own the returned object.
    #[allow(non_snake_case)]
    fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT;

    /// Returns whether or not a character in a page is generated by PDFium.
    ///
    ///    `text_page`   -   handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   zero-based index of the character.
    ///
    /// Returns `1` if the character is generated by PDFium, `0` if the character is not
    /// generated by PDFium, or `-1` if there was an error.
    #[allow(non_snake_case)]
    fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    // Returns whether or not a character in a page is a hyphen.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    /// Returns `1` if the character is a hyphen, `0` if the character is not a hyphen,
    /// or `-1` if there was an error.
    #[allow(non_snake_case)]
    fn FPDFText_IsHyphen(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    /// Returns whether or not a character in a page has an invalid unicode mapping.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    /// Returns `1` if the character has an invalid unicode mapping, `0` if the character
    /// has no known unicode mapping issues, or `-1` if there was an error.
    #[allow(non_snake_case)]
    fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    /// Gets the font size of a particular character.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    /// Returns the font size of the particular character, measured in points (about 1/72 inch).
    /// This is the typographic size of the font (so called "em size").
    #[allow(non_snake_case)]
    fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double;

    /// Gets the font name and flags of a particular character.
    ///
    ///    `text_page` - Handle to a text page information structure.
    ///                  Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`     - Zero-based index of the character.
    ///
    ///    `buffer`    - A buffer receiving the font name.
    ///
    ///    `buflen`    - The length of `buffer` in bytes.
    ///
    ///    `flags`     - Optional pointer to an int receiving the font flags.
    ///                  These flags should be interpreted per PDF spec 1.7
    ///                  Section 5.7.1, "Font Descriptor Flags".
    ///
    /// On success, returns the length of the font name, including the trailing `NUL` character,
    /// in bytes. If this length is less than or equal to `length`, `buffer` is set to the
    /// font name, `flags` is set to the font flags. `buffer` is in UTF-8 encoding.
    /// Returns `0` on failure.
    #[allow(non_snake_case)]
    fn FPDFText_GetFontInfo(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        flags: *mut c_int,
    ) -> c_ulong;

    /// Gets the font weight of a particular character.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    /// On success, returns the font weight of the particular character. If `text_page`
    /// is invalid, if `index` is out of bounds, or if the character's text object is
    /// undefined, return `-1`.
    #[allow(non_snake_case)]
    fn FPDFText_GetFontWeight(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    /// Gets the fill color of a particular character.
    ///
    ///    `text_page`      -   Handle to a text page information structure.
    ///                         Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`          -   Zero-based index of the character.
    ///
    ///    `R`              -   Pointer to an unsigned int number receiving the
    ///                         red value of the fill color.
    ///
    ///    `G`              -   Pointer to an unsigned int number receiving the
    ///                         green value of the fill color.
    ///
    ///    `B`              -   Pointer to an unsigned int number receiving the
    ///                         blue value of the fill color.
    ///
    ///    `A`              -   Pointer to an unsigned int number receiving the
    ///                         alpha value of the fill color.
    ///
    /// Returns whether the call succeeded. If false, `R`, `G`, `B` and `A` are unchanged.
    #[allow(non_snake_case)]
    fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Gets the stroke color of a particular character.
    ///
    ///    `text_page`      -   Handle to a text page information structure.
    ///                         Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`          -   Zero-based index of the character.
    ///
    ///    `R`              -   Pointer to an unsigned int number receiving the
    ///                         red value of the stroke color.
    ///
    ///    `G`              -   Pointer to an unsigned int number receiving the
    ///                         green value of the stroke color.
    ///
    ///    `B`              -   Pointer to an unsigned int number receiving the
    ///                         blue value of the stroke color.
    ///
    ///    `A`              -   Pointer to an unsigned int number receiving the
    ///                         alpha value of the stroke color.
    ///
    /// Returns whether the call succeeded. If false, `R`, `G`, `B` and `A` are unchanged.
    #[allow(non_snake_case)]
    fn FPDFText_GetStrokeColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Gets character rotation angle.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    /// On success, returns the angle value in radians. Value will always be greater or
    /// equal to `0`. If `text_page` is invalid, or if `index` is out of bounds,
    /// then returns `-1`.
    #[allow(non_snake_case)]
    fn FPDFText_GetCharAngle(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_float;

    /// Gets bounding box of a particular character.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    ///    `left`        -   Pointer to a double number receiving left position
    ///                      of the character box.
    ///
    ///    `right`       -   Pointer to a double number receiving right position
    ///                      of the character box.
    ///
    ///    `bottom`      -   Pointer to a double number receiving bottom position
    ///                      of the character box.
    ///
    ///    `top`         -   Pointer to a double number receiving top position of
    ///                      the character box.
    ///
    /// On success, returns `true` and fills in `left`, `right`, `bottom`, and `top`.
    /// If `text_page` is invalid, or if `index` is out of bounds, then returns `false`,
    /// and the out parameters remain unmodified.
    ///
    /// All positions are measured in PDF user space.
    #[allow(non_snake_case)]
    fn FPDFText_GetCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        left: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
        top: *mut c_double,
    ) -> FPDF_BOOL;

    /// Gets a "loose" bounding box of a particular character, i.e., covering the entire
    /// glyph bounds, without taking the actual glyph shape into account.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    ///    `rect`        -   Pointer to a `FS_RECTF` receiving the character box.
    ///
    /// On success, returns `true` and fills in `rect`. If `text_page` is invalid, or if
    /// `index` is out of bounds, then returns `false`, and the `rect` out parameter
    /// remains unmodified.
    ///
    /// All positions are measured in PDF "user space".
    #[allow(non_snake_case)]
    fn FPDFText_GetLooseCharBox(&self, text_page: FPDF_TEXTPAGE, index: c_int, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the effective transformation matrix for a particular character.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    ///    `matrix`      -   Pointer to a `FS_MATRIX` receiving the transformation matrix.
    ///
    /// On success, returns `true` and fills in `matrix`. If `text_page` is invalid, or if
    /// `index` is out of bounds, or if `matrix` is `NULL`, then returns `false`, and
    /// `matrix` remains unmodified.
    #[allow(non_snake_case)]
    fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL;

    /// Gets the origin of a particular character.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `index`       -   Zero-based index of the character.
    ///
    ///    `x`           -   Pointer to a double number receiving x coordinate of
    ///                      the character origin.
    ///
    ///    `y`           -   Pointer to a double number receiving y coordinate of
    ///                      the character origin.
    ///
    /// Returns whether the call succeeded. If `false`, `x` and `y` are unchanged.
    ///
    /// All positions are measured in PDF "user space".
    #[allow(non_snake_case)]
    fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL;

    /// Gets the index of a character at or nearby a certain position on the page.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `x`           -   X position in PDF "user space".
    ///
    ///    `y`           -   Y position in PDF "user space".
    ///
    ///    `xTolerance`  -   An x-axis tolerance value for character hit detection,
    ///                      in point units.
    ///
    ///    `yTolerance`  -   A y-axis tolerance value for character hit detection,
    ///                      in point units.
    ///
    /// Returns the zero-based index of the character at, or nearby, the point `(x, y)`.
    /// If there is no character at or nearby the point, the return value will be `-1`.
    /// If an error occurs, `-3` will be returned.
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexAtPos(
        &self,
        text_page: FPDF_TEXTPAGE,
        x: c_double,
        y: c_double,
        xTolerance: c_double,
        yTolerance: c_double,
    ) -> c_int;

    /// Extracts a unicode text string from the page.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `start_index` -   Index for the start characters.
    ///
    ///    `count`       -   Number of `UCS-2` values to be extracted.
    ///
    ///    `result`      -   A buffer (allocated by application) receiving the extracted
    ///                      `UCS-2` values. The buffer must be able to hold `count`
    ///                      `UCS-2` values plus a terminator.
    ///
    /// Returns the number of characters written into the `result` buffer, including the
    /// trailing terminator.
    ///
    /// This function ignores characters without `UCS-2` representations. It considers
    /// all characters on the page, even those that are not visible when the page has
    /// a cropbox. To filter out the characters outside of the cropbox, use
    /// [PdfiumLibraryBindings::FPDF_GetPageBoundingBox] and [PdfiumLibraryBindings::FPDFText_GetCharBox].
    #[allow(non_snake_case)]
    fn FPDFText_GetText(
        &self,
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int;

    /// Counts the number of rectangular areas occupied by a segment of text,
    /// and caches the result for subsequent [PdfiumLibraryBindings::FPDFText_GetRect] calls.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `start_index` -   Index for the start character.
    ///
    ///    `count`       -   Number of characters, or `-1` for all remaining.
    ///
    /// Returns the number of rectangles, `0` if `text_page` is `NULL`, or `-1` on bad `start_index`.
    ///
    /// This function, along with [PdfiumLibraryBindings::FPDFText_GetRect], can be used by
    /// applications to detect the position on the page for a text segment, so proper areas
    /// can be highlighted. The `FPDFText_*` functions will automatically merge small character
    /// boxes into bigger one if those characters are on the same line and use same font settings.
    #[allow(non_snake_case)]
    fn FPDFText_CountRects(&self, text_page: FPDF_TEXTPAGE, start_index: c_int, count: c_int) -> c_int;

    /// Gets a rectangular area from the result generated by
    /// [PdfiumLibraryBindings::FPDFText_CountRects].
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `rect_index`  -   Zero-based index for the rectangle.
    ///
    ///    `left`        -   Pointer to a double value receiving the rectangle left boundary.
    ///
    ///    `top`         -   Pointer to a double value receiving the rectangle top boundary.
    ///
    ///    `right`       -   Pointer to a double value receiving the rectangle right boundary.
    ///
    ///    `bottom`      -   Pointer to a double value receiving the rectangle bottom boundary.
    ///
    /// On success, returns `true` and fills in `left`, `top`, `right`, and `bottom`.
    /// If `text_page` is invalid then returns `false`, and the out parameters remain unmodified.
    /// If `text_page` is valid but `rect_index` is out of bounds, then returns `false`
    ///  and sets the out parameters to `0`.
    #[allow(non_snake_case)]
    fn FPDFText_GetRect(
        &self,
        text_page: FPDF_TEXTPAGE,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL;

    /// Extracts unicode text within a rectangular boundary on the page.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `left`        -   Left boundary.
    ///
    ///    `top`         -   Top boundary.
    ///
    ///    `right`       -   Right boundary.
    ///
    ///    `bottom`      -   Bottom boundary.
    ///
    ///    `buffer`      -   Caller-allocated buffer to receive `UTF-16` values.
    ///
    ///    `buflen`      -   Number of `UTF-16` values **(not bytes)** that `buffer`
    ///                      is capable of holding.
    ///
    /// If `buffer` is `NULL` or `buflen` is zero, then returns the number of `UTF-16`
    /// values **(not bytes)** of text present within the rectangle, excluding
    /// a terminating `NUL`. Generally you should pass a buffer at least one larger than this
    /// if you want a terminating `NUL`, which will be provided if space is available.
    /// Otherwise, return number of `UTF-16` values copied into the buffer, including the
    /// terminating `NUL` when space for it is available.
    ///
    /// If `buffer` is too small, as much text as will fit is copied into it. May return
    /// a split surrogate in that case.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFText_GetBoundedText(
        &self,
        text_page: FPDF_TEXTPAGE,
        left: c_double,
        top: c_double,
        right: c_double,
        bottom: c_double,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int;

    /// Starts a search.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `findwhat`    -   A unicode match pattern.
    ///
    ///    `flags`       -   Option flags.
    ///
    ///    `start_index` -   Start from this character. `-1` for end of the page.
    ///
    /// Returns a handle for the search context. [PdfiumLibraryBindings::FPDFText_FindClose]
    /// must be called to release this handle.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFText_FindStart_str].
    #[allow(non_snake_case)]
    fn FPDFText_FindStart(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: FPDF_WIDESTRING,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFText_FindStart].
    ///
    /// Starts a search.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage].
    ///
    ///    `findwhat`    -   A unicode match pattern.
    ///
    ///    `flags`       -   Option flags.
    ///
    ///    `start_index` -   Start from this character. `-1` for end of the page.
    ///
    /// Returns a handle for the search context. [PdfiumLibraryBindings::FPDFText_FindClose]
    /// must be called to release this handle.
    #[allow(non_snake_case)]
    fn FPDFText_FindStart_str(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: &str,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE {
        self.FPDFText_FindStart(
            text_page,
            get_pdfium_utf16le_bytes_from_str(findwhat).as_ptr() as FPDF_WIDESTRING,
            flags,
            start_index,
        )
    }

    /// Searches in the direction from page start to end.
    ///
    ///    `handle`      -   A search context handle returned by
    ///                      [PdfiumLibraryBindings::FPDFText_FindStart].
    ///
    /// Returns whether or not a match is found.
    #[allow(non_snake_case)]
    fn FPDFText_FindNext(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL;

    /// Searches in the direction from page end to start.
    ///
    ///    `handle`      -   A search context handle returned by
    ///                      [PdfiumLibraryBindings::FPDFText_FindStart].
    ///
    /// Returns whether or not a match is found.
    #[allow(non_snake_case)]
    fn FPDFText_FindPrev(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL;

    /// Gets the starting character index of the search result.
    ///
    ///    `handle`      -   A search context handle returned by
    ///                      [PdfiumLibraryBindings::FPDFText_FindStart].
    ///
    /// Returns the index for the starting character.
    #[allow(non_snake_case)]
    fn FPDFText_GetSchResultIndex(&self, handle: FPDF_SCHHANDLE) -> c_int;

    /// Gets the number of matched characters in the search result.
    ///
    ///    `handle`      -   A search context handle returned by
    ///                      [PdfiumLibraryBindings::FPDFText_FindStart].
    ///
    /// Returns the number of matched characters.
    #[allow(non_snake_case)]
    fn FPDFText_GetSchCount(&self, handle: FPDF_SCHHANDLE) -> c_int;

    /// Releases a search context.
    ///
    ///    `handle`      -   A search context handle returned by
    ///                      [PdfiumLibraryBindings::FPDFText_FindStart].
    #[allow(non_snake_case)]
    fn FPDFText_FindClose(&self, handle: FPDF_SCHHANDLE);

    /// Prepares information about weblinks in a page.
    ///
    ///    `text_page`   -   Handle to a text page information structure.
    ///                      Returned by [PdfiumLibraryBindings::FPDFText_LoadPage] function.
    ///
    /// Returns a handle to the page's links information structure, or `NULL` if something goes wrong.
    ///
    /// Weblinks are those links implicitly embedded in PDF pages. PDF also has a type of annotation
    /// called "link" (FPDFTEXT doesn't deal with that kind of link). FPDFTEXT weblink feature is
    /// useful for automatically detecting links in the page contents. For example, things like
    /// <https://www.example.com> will be detected, so applications can allow user to click on
    /// those characters to activate the link, even the PDF doesn't come with link annotations.
    ///
    /// [PdfiumLibraryBindings::FPDFLink_CloseWebLinks] must be called to release resources.
    #[allow(non_snake_case)]
    fn FPDFLink_LoadWebLinks(&self, text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK;

    /// Counts the number of detected web links.
    ///
    ///    `link_page`   -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    ///
    /// Returns the umber of detected web links.
    #[allow(non_snake_case)]
    fn FPDFLink_CountWebLinks(&self, link_page: FPDF_PAGELINK) -> c_int;

    /// Gets the URL information for a detected web link.
    ///
    ///    `link_page`   -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    ///
    ///    `link_index`  -   Zero-based index of the link.
    ///
    ///    `buffer`      -   A unicode buffer for the result.
    ///
    ///    `buflen`      -   Number of 16-bit code units (not bytes) for the buffer,
    ///                      including an additional terminator.
    ///
    /// If `buffer` is `NULL` or `buflen` is zero, returns the number of 16-bit code units
    /// (not bytes) needed to buffer the result (an additional terminator is included in this count).
    /// Otherwise, copies the result into `buffer`, truncating at `buflen` if the result is
    /// too large to fit, and returns the number of 16-bit code units actually copied into
    // the buffer (the additional terminator is also included in this count).
    ///
    /// If `link_index` does not correspond to a valid link, then the result is an empty string.
    #[allow(non_snake_case)]
    fn FPDFLink_GetURL(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int;

    /// Counts the number of rectangular areas for a given link.
    ///
    ///    `link_page`   -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    ///
    ///    `link_index`  -   Zero-based index of the link.
    ///
    /// Returns the number of rectangular areas for the link. If `link_index` does not
    /// correspond to a valid link, then returns `0`.
    #[allow(non_snake_case)]
    fn FPDFLink_CountRects(&self, link_page: FPDF_PAGELINK, link_index: c_int) -> c_int;

    /// Gets the boundaries of one rectangular area for a given link.
    ///
    ///    `link_page`   -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    ///
    ///    `link_index`  -   Zero-based index of the link.
    ///
    ///    `rect_index`  -   Zero-based index of the rectangle.
    ///
    ///    `left`        -   Pointer to a double value receiving the rectangle left boundary.
    ///
    ///    `top`         -   Pointer to a double value receiving the rectangle top boundary.
    ///
    ///    `right`       -   Pointer to a double value receiving the rectangle right boundary.
    ///
    ///    `bottom`      -   Pointer to a double value receiving the rectangle bottom boundary.
    ///
    /// On success, returns `true` and fills in `left`, `top`, `right`, and `bottom`.
    /// If `link_page` is invalid or if `link_index` does not correspond to a valid link,
    /// then returns `false`, and the out parameters remain unmodified.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFLink_GetRect(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL;

    /// Gets the start char index and char count for a link.
    ///
    ///    `link_page`         -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    ///
    ///    `link_index`        -   Zero-based index for the link.
    ///
    ///    `start_char_index`  -   pointer to int receiving the start char index
    ///
    ///    `char_count`        -   pointer to int receiving the char count
    ///
    /// On success, returns `true` and fills in `start_char_index` and `char_count`.
    /// If `link_page` is invalid or if `link_index` does not correspond to a valid link,
    /// then returns `false` and the out parameters remain unmodified.
    #[allow(non_snake_case)]
    fn FPDFLink_GetTextRange(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        start_char_index: *mut c_int,
        char_count: *mut c_int,
    ) -> FPDF_BOOL;

    /// Releases resources used by weblink feature.
    ///
    ///    `link_page`   -   Handle returned by [PdfiumLibraryBindings::FPDFLink_LoadWebLinks].
    #[allow(non_snake_case)]
    fn FPDFLink_CloseWebLinks(&self, link_page: FPDF_PAGELINK);

    /// Gets the decoded data from the thumbnail of `page`, if it exists.
    ///
    ///    `page`    - handle to a page.
    ///
    ///    `buffer`  - buffer for holding the decoded image data.
    ///
    ///    `buflen`  - length of the buffer in bytes.
    ///
    /// This only modifies `buffer` if `buflen` is less than or equal to the size of the
    /// decoded data. Returns the size of the decoded data, or `0` if thumbnail does not exist.
    /// Optionally, pass `NULL` to just retrieve the size of the buffer needed.
    #[allow(non_snake_case)]
    fn FPDFPage_GetDecodedThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong;

    /// Gets the raw data from the thumbnail of `page`, if it exists.
    ///
    ///    `page`    - handle to a page.
    ///
    ///    `buffer`  - buffer for holding the raw image data.
    ///
    ///    `buflen`  - length of the buffer in bytes.
    ///
    /// This only modifies `buffer` if `buflen` is less than or equal to the size of the
    /// raw data. Returns the size of the raw data, or `0` if thumbnail does not exist.
    /// Optionally, pass `NULL` to just retrieve the size of the buffer needed.
    #[allow(non_snake_case)]
    fn FPDFPage_GetRawThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong;

    /// Returns the thumbnail of `page` as a `FPDF_BITMAP`.
    ///
    ///    `page` - handle to a page.
    ///
    /// Returns `NULL` if unable to access the thumbnail's stream.
    #[allow(non_snake_case)]
    fn FPDFPage_GetThumbnailAsBitmap(&self, page: FPDF_PAGE) -> FPDF_BITMAP;

    /// Gets the number of page objects inside `form_object`.
    ///
    ///    `form_object` - handle to a form object.
    ///
    /// Returns the number of objects in `form_object` on success, or `-1` on error.
    #[allow(non_snake_case)]
    fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets the page object in `form_object` at `index`.
    ///
    ///    `form_object` - handle to a form object.
    ///
    ///    `index`       - the zero-based index of a page object.
    ///
    /// Returns the handle to the page object, or `NULL` on error.
    #[allow(non_snake_case)]
    fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT;

    /// Removes `page_object` from `form_object`.
    ///
    ///   `form_object` - handle to a form object.
    ///
    ///   `page_object` - handle to a page object to be removed from the form.
    ///
    /// Returns `true` on success.
    ///
    /// Ownership of the removed `page_object` is transferred to the caller.
    /// Call [PdfiumLibraryBindings::FPDFPageObj_Destroy]` on the removed page_object to free it.
    #[allow(non_snake_case)]
    fn FPDFFormObj_RemoveObject(&self, form_object: FPDF_PAGEOBJECT, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Creates a new text object using a loaded font.
    ///
    ///    `document`   - handle to the document.
    ///
    ///    `font`       - handle to the font object.
    ///
    ///    `font_size`  - the font size for the new text object.
    ///
    /// Returns a handle to a new text object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: FPDF_FONT,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT;

    /// Gets the text rendering mode of a text object.
    ///
    ///    `text`     - the handle to the text object.
    ///
    /// Returns one of the known `FPDF_TEXT_RENDERMODE` enum values on success,
    /// `FPDF_TEXTRENDERMODE_UNKNOWN` on error.
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetTextRenderMode(&self, text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE;

    /// Sets the text rendering mode of a text object.
    ///
    ///    `text`         - the handle to the text object.
    ///
    ///    `render_mode`  - the `FPDF_TEXT_RENDERMODE` enum value to be set (cannot set to
    ///                     `FPDF_TEXTRENDERMODE_UNKNOWN`).
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFTextObj_SetTextRenderMode(&self, text: FPDF_PAGEOBJECT, render_mode: FPDF_TEXT_RENDERMODE) -> FPDF_BOOL;

    /// Gets the text of a text object.
    ///
    ///    `text_object`      - the handle to the text object.
    ///
    ///    `text_page`        - the handle to the text page.
    ///
    ///    `buffer`           - the address of a buffer that receives the text.
    ///
    ///    `length`           - the size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the text (including the trailing `NUL` character)
    /// on success, `0` on error. Regardless of the platform, the `buffer` is always in
    /// UTF-16LE encoding. If `length` is less than the returned length, or `buffer` is
    /// `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetText(
        &self,
        text_object: FPDF_PAGEOBJECT,
        text_page: FPDF_TEXTPAGE,
        buffer: *mut FPDF_WCHAR,
        length: c_ulong,
    ) -> c_ulong;

    /// Gets a bitmap rasterization of `text_object`. To render correctly, the caller
    /// must provide the `document` associated with `text_object`. If there is a `page`
    /// associated with `text_object`, the caller should provide that as well.
    /// The returned bitmap will be owned by the caller, and [PdfiumLibraryBindings::FPDFBitmap_Destroy]
    /// must be called on the returned bitmap when it is no longer needed.
    ///
    ///    `document`    - handle to a document associated with `text_object`.
    ///
    ///    `page`        - handle to an optional page associated with `text_object`.
    ///
    ///    `text_object` - handle to a text object.
    ///
    ///    `scale`       - the scaling factor, which must be greater than `0`.
    ///
    /// Returns the bitmap or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        text_object: FPDF_PAGEOBJECT,
        scale: f32,
    ) -> FPDF_BITMAP;

    /// Gets the font of a text object.
    ///
    ///    `text` - the handle to the text object.
    ///
    /// Returns a handle to the font object held by `text` which retains ownership.
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT;

    /// Gets the font size of a text object.
    ///
    ///    `text` - handle to a text.
    ///
    ///    `size` - pointer to the font size of the text object, measured in points
    ///             (about 1/72 inch).
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFontSize(&self, text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL;

    /// Closes a loaded PDF font.
    ///
    ///    `font`   - Handle to the loaded font.
    #[allow(non_snake_case)]
    fn FPDFFont_Close(&self, font: FPDF_FONT);

    /// Moves a path's current point.
    ///
    ///    `path`   - the handle to the path object.
    ///
    ///    `x`      - the horizontal position of the new current point.
    ///
    ///    `y`      - the vertical position of the new current point.
    ///
    /// Note that no line will be created between the previous current point and the
    /// new one. Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPath_MoveTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL;

    /// Adds a line between the current point and a new point in the path.
    ///
    ///    `path`   - the handle to the path object.
    ///
    ///    `x`      - the horizontal position of the new point.
    ///
    ///    `y`      - the vertical position of the new point.
    ///
    /// The path's current point is changed to `(x, y)`. Returns `true` on success,
    /// `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFPath_LineTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL;

    /// Adds a cubic Bezier curve to the given path, starting at the current point.
    ///
    ///    `path`   - the handle to the path object.
    ///
    ///    `x1`     - the horizontal position of the first Bezier control point.
    ///
    ///    `y1`     - the vertical position of the first Bezier control point.
    ///
    ///    `x2`     - the horizontal position of the second Bezier control point.
    ///
    ///    `y2`     - the vertical position of the second Bezier control point.
    ///
    ///    `x3`     - the horizontal position of the ending point of the Bezier curve.
    ///
    ///    `y3`     - the vertical position of the ending point of the Bezier curve.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPath_BezierTo(
        &self,
        path: FPDF_PAGEOBJECT,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        x3: c_float,
        y3: c_float,
    ) -> FPDF_BOOL;

    /// Closes the current subpath of a given path.
    ///
    ///    `path`   - the handle to the path object.
    ///
    /// This will add a line between the current point and the initial point of the subpath,
    /// thus terminating the current subpath. Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPath_Close(&self, path: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Sets the drawing mode of a path.
    ///
    ///    `path`     - the handle to the path object.
    ///
    ///    `fillmode` - the filling mode to be set: one of the `FPDF_FILLMODE_*` flags.
    ///
    ///    `stroke`   - a boolean specifying if the path should be stroked or not.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPath_SetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: c_int, stroke: FPDF_BOOL) -> FPDF_BOOL;

    /// Gets the drawing mode of a path.
    ///
    ///    `path`     - the handle to the path object.
    ///
    ///    `fillmode` - the filling mode of the path: one of the `FPDF_FILLMODE_*` flags.
    ///
    ///    `stroke`   - a boolean specifying if the path is stroked or not.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPath_GetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: *mut c_int, stroke: *mut FPDF_BOOL) -> FPDF_BOOL;

    /// Creates a new text object using one of the standard PDF fonts.
    ///
    ///    `document`   - handle to the document.
    ///
    ///    `font`       - string containing the font name, without spaces.
    ///
    ///    `font_size`  - the font size for the new text object.
    ///
    /// Returns a handle to a new text object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewTextObj(&self, document: FPDF_DOCUMENT, font: &str, font_size: c_float) -> FPDF_PAGEOBJECT;

    /// Sets the text for a text object. If it had text, it will be replaced.
    ///
    ///    `text_object`  - handle to the text object.
    ///
    ///    `text`         - the UTF-16LE encoded string containing the text to be added.
    ///
    /// Returns `true` on success.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFText_SetText_str].
    #[allow(non_snake_case)]
    fn FPDFText_SetText(&self, text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFText_SetText].
    ///
    /// Sets the text for a text object. If it had text, it will be replaced.
    ///
    ///    `text_object`  - handle to the text object.
    ///
    ///    `text`         - the string containing the text to be added.
    ///
    /// Returns `true` on success.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetText_str(&self, text_object: FPDF_PAGEOBJECT, text: &str) -> FPDF_BOOL {
        self.FPDFText_SetText(
            text_object,
            get_pdfium_utf16le_bytes_from_str(text).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Sets the text using charcodes for a text object. If it had text, it will be replaced.
    ///
    ///    `text_object`  - handle to the text object.
    ///
    ///    `charcodes`    - pointer to an array of charcodes to be added.
    ///
    ///    `count`        - number of elements in |charcodes|.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFText_SetCharcodes(&self, text_object: FPDF_PAGEOBJECT, charcodes: *const c_uint, count: size_t)
    -> FPDF_BOOL;

    /// Returns a font object loaded from a stream of data. The font is loaded into the
    /// document. Various font data structures, such as the ToUnicode data, are auto-generated
    /// based on the inputs.
    ///
    ///    `document`  - handle to the document.
    ///
    ///    `data`      - the stream of font data, which will be copied by the font object.
    ///
    ///    `size`      - the size of the font data, in bytes.
    ///
    ///    `font_type` - `FPDF_FONT_TYPE1` or `FPDF_FONT_TRUETYPE` depending on the font type.
    ///
    ///    `cid`       - a boolean specifying if the font is a CID font or not.
    ///
    /// The loaded font can be closed using [PdfiumLibraryBindings::FPDFFont_Close].
    /// Returns `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFText_LoadFont(
        &self,
        document: FPDF_DOCUMENT,
        data: *const c_uchar,
        size: c_uint,
        font_type: c_int,
        cid: FPDF_BOOL,
    ) -> FPDF_FONT;

    /// Loads one of the standard 14 fonts per PDF spec 1.7 page 416. The preferred way
    /// of using font style is using a dash to separate the name from the style,
    /// for example `Helvetica-BoldItalic`.
    ///
    ///    `document`   - handle to the document.
    ///
    ///    `font`       - string containing the font name, without spaces.
    ///
    /// The loaded font can be closed using [PdfiumLibraryBindings::FPDFFont_Close].
    /// Returns `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFText_LoadStandardFont(&self, document: FPDF_DOCUMENT, font: &str) -> FPDF_FONT;

    /// Returns a font object loaded from a stream of data for a type 2 CID font. The font
    /// is loaded into the document. Unlike [PdfiumLibraryBindings::FPDFText_LoadFont],
    /// the ToUnicode data and the CIDToGIDMap data are caller provided, instead of being
    /// auto-generated.
    ///
    ///    `document`                   - handle to the document.
    ///
    ///    `font_data`                  - the stream of font data, which will be copied
    ///                                   by the font object.
    ///
    ///    `font_data_size`             - the size of the font data, in bytes.
    ///
    ///    `to_unicode_cmap`            - the ToUnicode data.
    ///
    ///    `cid_to_gid_map_data`        - the stream of CIDToGIDMap data.
    ///
    ///    `cid_to_gid_map_data_size`   - the size of the CIDToGIDMap data, in bytes.
    ///
    /// The loaded font can be closed using [PdfiumLibraryBindings::FPDFFont_Close].
    /// Returns `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFText_LoadCidType2Font(
        &self,
        document: FPDF_DOCUMENT,
        font_data: *const u8,
        font_data_size: u32,
        to_unicode_cmap: &str,
        cid_to_gid_map_data: *const u8,
        cid_to_gid_map_data_size: u32,
    ) -> FPDF_FONT;

    /// Inserts `page_object` into `page`.
    ///
    ///    `page`        - handle to a page
    ///
    ///    `page_object` - handle to a page object. The `page_object` will be
    ///                    automatically freed.
    #[allow(non_snake_case)]
    fn FPDFPage_InsertObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT);

    /// Inserts `page_object` into `page` at the specified `index`.
    ///
    ///    `page`        - handle to a page
    ///
    ///    `page_object` - handle to a page object as previously obtained by
    ///                    [PdfiumLibraryBindings::FPDFPageObj_CreateNewPath],
    ///                    [PdfiumLibraryBindings::FPDFPageObj_CreateNewRect],
    ///                    [PdfiumLibraryBindings::FPDFPageObj_NewTextObj], or
    ///                    [PdfiumLibraryBindings::FPDFPageObj_NewImageObj].
    ///                    Ownership of the object is transferred back to PDFium.
    ///
    ///    `index`       - the index position to insert the object at. If index equals
    ///                    the current object count, the object will be appended to the
    ///                    end. If index is greater than the object count, the function
    ///                    will fail and return false.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPage_InsertObjectAtIndex(&self, page: FPDF_PAGE, page_object: FPDF_PAGEOBJECT, index: usize) -> FPDF_BOOL;

    /// Removes `page_object` from `page`.
    ///
    ///    `page`        - handle to a page
    ///
    ///    `page_object` - handle to a page object to be removed.
    ///
    /// Returns `true` on success. Ownership is transferred to the caller.
    /// Call [PdfiumLibraryBindings::FPDFPageObj_Destroy] to free it. Note that when removing
    /// a `page_object` of type `FPDF_PAGEOBJ_TEXT`, all `FPDF_TEXTPAGE` handles for `page`
    /// are no longer valid.
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Gets the number of page objects inside `page`.
    ///
    ///    `page` - handle to a page.
    ///
    /// Returns the number of objects in `page`.
    #[allow(non_snake_case)]
    fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int;

    /// Gets the object in `page` at `index`.
    ///
    ///    `page`  - handle to a page.
    ///
    ///    `index` - the index of a page object.
    ///
    /// Returns the handle to the page object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT;

    /// Destroys `page_object` by releasing its resources. `page_object` must have
    /// been created by [PdfiumLibraryBindings::FPDFPageObj_CreateNewPath],
    /// [PdfiumLibraryBindings::FPDFPageObj_CreateNewRect],
    /// [PdfiumLibraryBindings::FPDFPageObj_NewTextObj] or
    /// [PdfiumLibraryBindings::FPDFPageObj_NewImageObj]. This function must be called
    /// on newly-created objects if they are not added to a page through
    /// [PdfiumLibraryBindings::FPDFPage_InsertObject] or to an annotation through
    /// [PdfiumLibraryBindings::FPDFAnnot_AppendObject].
    ///
    ///    `page_object` - handle to a page object.
    #[allow(non_snake_case)]
    fn FPDFPageObj_Destroy(&self, page_obj: FPDF_PAGEOBJECT);

    /// Checks if `page` contains transparency.
    ///
    ///    `page` - handle to a page.
    ///
    /// Returns `true` if `page` contains transparency.
    #[allow(non_snake_case)]
    fn FPDFPageObj_HasTransparency(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Gets the type of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    /// Returns one of the `FPDF_PAGEOBJ_*` values on success, or `FPDF_PAGEOBJ_UNKNOWN` on error.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets the active state for `page_object` within its containing page.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `active`      - pointer to variable that will receive `true` if the page object
    ///                    is active. This is a required parameter. Not filled if this
    ///                    function returns `false`.
    ///
    /// For page objects where `active` is filled with `false`, the `page_object` is
    /// treated as if it wasn't in the document even though it is still held
    /// internally.
    ///
    /// Returns `true` if the operation succeeded, `false` if it failed.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: *mut FPDF_BOOL) -> FPDF_BOOL;

    /// Sets whether `page_object` is active within its containing page.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `active`      - a boolean specifying if the object is active.
    ///
    /// Returns `true` on success.
    ///
    /// Page objects all start in the active state by default, and remain in that
    /// state unless this function is called.
    ///
    /// When `active` is false, this makes the `page_object` be treated as if it
    /// wasn't in the document even though it is still held internally.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: FPDF_BOOL) -> FPDF_BOOL;

    /// Transforms `page_object` by the given matrix.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `a`           - matrix value.
    ///
    ///    `b`           - matrix value.
    ///
    ///    `c`           - matrix value.
    ///
    ///    `d`           - matrix value.
    ///
    ///    `e`           - matrix value.
    ///
    ///    `f`           - matrix value.
    ///
    /// The matrix is composed as:
    ///
    ///    `a c e`
    ///
    ///    `b d f`
    ///
    /// and can be used to scale, rotate, shear and translate the `page_object`.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPageObj_Transform(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    );

    /// Transforms `page_object` by the given matrix.
    ///
    ///   `page_object` - handle to a page object.
    ///
    ///   `matrix`      - the transform matrix.
    ///
    /// Returns `true on success.
    ///
    /// This can be used to scale, rotate, shear and translate the `page_object`.
    /// It is an improved version of [PdfiumLibraryBindings::FPDFPageObj_Transform]
    /// that does not do unnecessary double to float conversions, and only uses 1 parameter
    /// for the matrix. It also returns whether the operation succeeded or not.
    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformF(&self, page_object: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL;

    /// Gets the transform matrix of a page object.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `matrix`      - pointer to struct to receive the matrix value.
    ///
    /// The matrix is composed as:
    ///
    ///    `a c e`
    ///
    ///    `b d f`
    ///
    /// and used to scale, rotate, shear and translate the page object. For page objects
    /// outside form objects, the matrix values are relative to the page that contains it.
    /// For page objects inside form objects, the matrix values are relative to the form
    /// that contains it.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL;

    /// Sets the transform matrix of a page object.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `matrix`      - pointer to struct with the matrix value.
    ///
    /// The matrix is composed as:
    ///
    ///    `a c e`
    ///
    ///    `b d f`
    ///
    /// and can be used to scale, rotate, shear and translate the page object.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetMatrix(&self, path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL;

    /// Creates a new image object.
    ///
    ///    `document` - handle to a document.
    ///
    /// Returns a handle to a new image object.
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewImageObj(&self, document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT;

    /// Gets the marked content ID for the object.
    ///
    ///   `page_object`   - handle to a page object.
    ///
    /// Returns the page object's marked content ID, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMarkedContentID(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets the number of content marks in `page_object`.
    ///
    ///    `page_object`   - handle to a page object.
    ///
    /// Returns the number of content marks in `page_object`, or -1 in case of failure.
    #[allow(non_snake_case)]
    fn FPDFPageObj_CountMarks(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets content mark in `page_object` at `index`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `index`       - the index of a page object.
    ///
    /// Returns the handle to the content mark, or `NULL` on failure. The handle is
    /// still owned by the library, and it should not be freed directly. It becomes
    /// invalid if the page object is destroyed, either directly or indirectly by
    /// unloading the page.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMark(&self, page_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECTMARK;

    /// Adds a new content mark to a `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `name`        - the name (tag) of the mark.
    ///
    /// Returns the handle to the content mark, or `NULL` on failure. The handle is
    /// still owned by the library, and it should not be freed directly. It becomes
    /// invalid if the page object is destroyed, either directly or indirectly by
    /// unloading the page.
    #[allow(non_snake_case)]
    fn FPDFPageObj_AddMark(&self, page_object: FPDF_PAGEOBJECT, name: &str) -> FPDF_PAGEOBJECTMARK;

    /// Removes a content `mark` from a `page_object`. The mark handle will be invalid
    /// after the removal.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `mark`        - handle to a content mark in that object to remove.
    ///
    /// Returns `true` if the operation succeeded, `false` if it failed.
    #[allow(non_snake_case)]
    fn FPDFPageObj_RemoveMark(&self, page_object: FPDF_PAGEOBJECT, mark: FPDF_PAGEOBJECTMARK) -> FPDF_BOOL;

    /// Gets the name of a content mark.
    ///
    ///    `mark`       - handle to a content mark.
    ///
    ///    `buffer`     - buffer for holding the returned name in UTF-16LE. This is only
    ///                   modified if `buflen` is large enough to store the name.
    ///                   Optional, pass `null` to just retrieve the size of the buffer
    ///                   needed.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    ///    `out_buflen` - pointer to variable that will receive the minimum buffer size
    ///                   in bytes to contain the name. This is a required parameter.
    ///                   Not filled if `false` is returned.
    ///
    /// Returns `true` if the operation succeeded, `false` if it failed.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetName(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Gets the number of key/value pair parameters in `mark`.
    ///
    ///    `mark`   - handle to a content mark.
    ///
    /// Returns the number of key/value pair parameters `mark`, or `-1` in case of failure.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_CountParams(&self, mark: FPDF_PAGEOBJECTMARK) -> c_int;

    /// Gets the key of a property in a content mark.
    ///
    ///    `mark`       - handle to a content mark.
    ///
    ///    `index`      - index of the property.
    ///
    ///    `buffer`     - buffer for holding the returned key in UTF-16LE. This is only
    ///                   modified if `buflen` is large enough to store the key.
    ///                   Optional, pass `null` to just retrieve the size of the buffer needed.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    ///    `out_buflen` - pointer to variable that will receive the minimum buffer size
    ///                   in bytes to contain the name. This is a required parameter.
    ///                   Not filled if `false` is returned.
    ///
    /// Returns `true` if the operation was successful.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamKey(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        index: c_ulong,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Gets the type of the value of a property in a content mark by key.
    ///
    ///    `mark`   - handle to a content mark.
    ///
    ///    `key`    - string key of the property.
    ///
    /// Returns the type of the value, or `FPDF_OBJECT_UNKNOWN` in case of failure.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamValueType(&self, mark: FPDF_PAGEOBJECTMARK, key: &str) -> FPDF_OBJECT_TYPE;

    /// Gets the value of a number property in a content mark by key as int.
    /// [PdfiumLibraryBindings::FPDFPageObjMark_GetParamValueType] should have returned
    /// `FPDF_OBJECT_NUMBER` for this property.
    ///
    ///    `mark`      - handle to a content mark.
    ///
    ///    `key`       - string key of the property.
    ///
    ///    `out_value` - pointer to variable that will receive the value. Not filled if
    ///                  `false` is returned.
    ///
    /// Returns `true` if the key maps to a number value.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamIntValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_int,
    ) -> FPDF_BOOL;

    /// Gets the value of a number property in a content mark by key as float.
    /// [PdfiumLibraryBindings::FPDFPageObjMark_GetParamValueType] should have returned
    /// `FPDF_OBJECT_NUMBER` for this property.
    ///
    ///    `mark`      - handle to a content mark.
    ///
    ///    `key`       - string key of the property.
    ///
    ///    `out_value` - pointer to variable that will receive the value. Not filled if
    ///                  `false` is returned.
    ///
    /// Returns `true` if the key maps to a number value.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamFloatValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut f32,
    ) -> FPDF_BOOL;

    /// Gets the value of a string property in a content mark by key.
    ///
    ///    `mark`       - handle to a content mark.
    ///
    ///    `key`        - string key of the property.
    ///
    ///    `buffer`     - buffer for holding the returned value in UTF-16LE. This is
    ///                   only modified if `buflen` is large enough to store the value.
    ///                   Optional, pass `null` to just retrieve the size of the buffer needed.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    ///    `out_buflen` - pointer to variable that will receive the minimum buffer size
    ///                   in bytes to contain the name. This is a required parameter.
    ///                   Not filled if `false` is returned.
    ///
    /// Returns `true` if the key maps to a string/blob value.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamStringValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Gets the value of a blob property in a content mark by key.
    ///
    ///    `mark`       - handle to a content mark.
    ///
    ///    `key`        - string key of the property.
    ///
    ///    `buffer`     - buffer for holding the returned value. This is only modified
    ///                   if `buflen` is large enough to store the value. Optional, pass `null`
    ///                   to just retrieve the size of the buffer needed.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    ///    `out_buflen` - pointer to variable that will receive the minimum buffer size
    ///                   in bytes to contain the name. This is a required parameter.
    ///                   Not filled if `false` is returned.
    ///
    /// Returns `true` if the key maps to a string/blob value.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamBlobValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut c_uchar,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Sets the value of an int property in a content mark by key. If a parameter
    /// with key `key` exists, its value is set to `value`. Otherwise, it is added as
    /// a new parameter.
    ///
    ///    `document`    - handle to the document.
    ///
    ///    `page_object` - handle to the page object with the mark.
    ///
    ///    `mark`        - handle to a content mark.
    ///
    ///    `key`         - string key of the property.
    ///
    ///    `value`       - int value to set.
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetIntParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: c_int,
    ) -> FPDF_BOOL;

    /// Sets the value of a float property in a content mark by key. If a parameter
    /// with key `key` exists, its value is set to `value`. Otherwise, it is added as
    /// a new parameter.
    ///
    ///    `document`    - handle to the document.
    ///
    ///    `page_object` - handle to the page object with the mark.
    ///
    ///    `mark`        - handle to a content mark.
    ///
    ///    `key`         - string key of the property.
    ///
    ///    `value`       - float value to set.
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetFloatParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: f32,
    ) -> FPDF_BOOL;

    /// Sets the value of a string property in a content mark by key. If a parameter
    /// with key `key` exists, its value is set to `value`. Otherwise, it is added as
    /// a new parameter.
    ///
    ///    `document`    - handle to the document.
    ///
    ///    `page_object` - handle to the page object with the mark.
    ///
    ///    `mark`        - handle to a content mark.
    ///
    ///    `key`         - string key of the property.
    ///
    ///    `value`       - string value to set.
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetStringParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL;

    /// Sets the value of a blob property in a content mark by key. If a parameter
    /// with key `key` exists, its value is set to `value`. Otherwise, it is added as
    /// a new parameter.
    ///
    ///    `document`    - handle to the document.
    ///
    ///    `page_object` - handle to the page object with the mark.
    ///
    ///    `mark`        - handle to a content mark.
    ///
    ///    `key`         - string key of the property.
    ///
    ///    `value`       - pointer to blob value to set.
    ///
    ///    `value_len`   - size in bytes of `value`.
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetBlobParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: *const c_uchar,
        value_len: c_ulong,
    ) -> FPDF_BOOL;

    /// Removes a property from a content mark by key.
    ///
    ///    `page_object` - handle to the page object with the mark.
    ///
    ///    `mark`        - handle to a content mark.
    ///
    ///    `key`         - string key of the property.
    ///
    /// Returns `true` if the operation succeeded.
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_RemoveParam(
        &self,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
    ) -> FPDF_BOOL;

    /// Loads an image from a JPEG image file and then set it into `image_object`.
    ///
    ///    `pages`        - pointer to the start of all loaded pages, may be `NULL`.
    ///
    ///    `count`        - number of `pages`, may be `0`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `file_access`  - file access handler which specifies the JPEG image file.
    ///
    /// Returns `true` on success.
    ///
    /// The image object might already have an associated image, which is shared and
    /// cached by the loaded pages. In that case, we need to clear the cached image
    /// for all the loaded pages. Pass `pages` and page count (`count`) to this API
    /// to clear the image cache. If the image is not previously shared, or `NULL` is a
    /// valid `pages` value.
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFile(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL;

    /// Loads an image from a JPEG image file and then set it into `image_object`.
    ///
    ///    `pages`        - pointer to the start of all loaded pages, may be `NULL`.
    ///
    ///    `count`        - number of `pages`, may be `0`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `file_access`  - file access handler which specifies the JPEG image file.
    ///
    /// Returns `true` on success.
    ///
    /// The image object might already have an associated image, which is shared and
    /// cached by the loaded pages. In that case, we need to clear the cached image
    /// for all the loaded pages. Pass `pages` and page count (`count`) to this API
    /// to clear the image cache. If the image is not previously shared, or `NULL` is a
    /// valid `pages` value. This function loads the JPEG image inline, so the image
    /// content is copied to the file. This allows `file_access` and its associated
    /// data to be deleted after this function returns.
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFileInline(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL;

    /// Sets the transform matrix of `image_object`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `a`            - matrix value.
    ///
    ///    `b`            - matrix value.
    ///
    ///    `c`            - matrix value.
    ///
    ///    `d`            - matrix value.
    ///
    ///    `e`            - matrix value.
    ///
    ///    `f`            - matrix value.
    ///
    /// The matrix is composed as:
    ///
    ///    `|a c e|`
    ///
    ///    `|b d f|`
    ///
    /// and can be used to scale, rotate, shear and translate the `image_object`.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    #[deprecated(
        note = "Prefer FPDFPageObj_SetMatrix() over FPDFImageObj_SetMatrix(). FPDFImageObj_SetMatrix() is deprecated and will likely be removed in a future version of Pdfium."
    )]
    fn FPDFImageObj_SetMatrix(
        &self,
        image_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) -> FPDF_BOOL;

    /// Sets `bitmap` to `image_object`.
    ///
    ///    `pages`        - pointer to the start of all loaded pages, may be `NULL`.
    ///
    ///    `count`        - number of `pages`, may be `0`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `bitmap`       - handle of the bitmap.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFImageObj_SetBitmap(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        bitmap: FPDF_BITMAP,
    ) -> FPDF_BOOL;

    /// Gets a bitmap rasterization of `image_object`. [PdfiumLibraryBindings::FPDFImageObj_GetBitmap]
    /// only operates on `image_object` and does not take the associated image mask into
    /// account. It also ignores the matrix for `image_object`. The returned bitmap will be
    /// owned by the caller, and [PdfiumLibraryBindings::FPDFBitmap_Destroy] must be called on
    /// the returned bitmap when it is no longer needed.
    ///
    ///    `image_object` - handle to an image object.
    ///
    /// Returns the bitmap.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetBitmap(&self, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP;

    /// Gets a bitmap rasterization of `image_object` that takes the image mask and
    /// image matrix into account. To render correctly, the caller must provide the
    /// `document` associated with `image_object`. If there is a `page` associated
    /// with `image_object`, the caller should provide that as well.
    ///
    /// The returned bitmap will be owned by the caller, and [PdfiumLibraryBindings::FPDFBitmap_Destroy]
    /// must be called on the returned bitmap when it is no longer needed.
    ///
    ///    `document`     - handle to a document associated with `image_object`.
    ///
    ///    `page`         - handle to an optional page associated with `image_object`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    /// Returns the bitmap or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        image_object: FPDF_PAGEOBJECT,
    ) -> FPDF_BITMAP;

    /// Gets the decoded image data of `image_object`. The decoded data is the uncompressed
    /// image data, i.e. the raw image data after having all filters applied. `buffer` is
    /// only modified if `buflen` is longer than the length of the decoded image data.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `buffer`       - buffer for holding the decoded image data.
    ///
    ///    `buflen`       - length of the buffer in bytes.
    ///
    /// Returns the length of the decoded image data.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the raw image data of `image_object`. The raw data is the image data as
    /// stored in the PDF without applying any filters. `buffer` is only modified if
    /// `buflen` is longer than the length of the raw image data.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `buffer`       - buffer for holding the raw image data.
    ///
    ///    `buflen`       - length of the buffer in bytes.
    ///
    /// Returns the length of the raw image data.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataRaw(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the number of filters (i.e. decoders) of the image in `image_object`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    /// Returns the number of `image_object`'s filters.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilterCount(&self, image_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets the filter at `index` of `image_object`'s list of filters. Note that the
    /// filters need to be applied in order, i.e. the first filter should be applied
    /// first, then the second, etc. `buffer` is only modified if `buflen` is longer
    /// than the length of the filter string.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `index`        - the index of the filter requested.
    ///
    ///    `buffer`       - buffer for holding filter string, encoded in UTF-8.
    ///
    ///    `buflen`       - length of the buffer.
    ///
    /// Returns the length of the filter string.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilter(
        &self,
        image_object: FPDF_PAGEOBJECT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the image metadata of `image_object`, including dimension, DPI, bits per pixel,
    /// and colorspace. If the `image_object` is not an image object or if it does not have
    /// an image, then the return value will be false. Otherwise, failure to retrieve any
    /// specific parameter would result in its value being `0`.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `page`         - handle to the page that `image_object` is on. Required for
    ///                     retrieving the image's bits per pixel and colorspace.
    ///
    ///    `metadata`     - receives the image metadata; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageMetadata(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL;

    /// Gets the image size in pixels. This is a faster method to get only image size.
    ///
    ///    `image_object` - handle to an image object.
    ///
    ///    `width`        - receives the image width in pixels; must not be `NULL`.
    ///
    ///    `height`       - receives the image height in pixels; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImagePixelSize(
        &self,
        image_object: FPDF_PAGEOBJECT,
        width: *mut c_uint,
        height: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Gets the ICC profile decoded data of `image_object`.
    ///
    /// If the given `image_object` is not an image object, or if it does not have
    /// an image, then the return value will be `false`. This function also returns
    /// `false` if the `image_object` has no ICC profile.
    ///
    /// `buffer` is only modified if ICC profile exists and `buflen` is longer than
    /// the length of the ICC profile decoded data.
    ///
    ///   `image_object` - handle to an image object; must not be `NULL`.
    ///
    ///   `page`         - handle to the page containing `image_object`; must not be
    ///                    `NULL`. Required for retrieving the image's colorspace.
    ///
    ///   `buffer`       - Buffer to receive ICC profile data; may be `NULL` if querying
    ///                    required size via `out_buflen`.
    ///
    ///   `buflen`       - Length of the buffer in bytes. Ignored if `buffer` is `NULL`.
    ///
    ///   `out_buflen`   - Pointer to receive the ICC profile data size in bytes; must
    ///                    not be `NULL`. Will be set if this function returns `true`.
    ///
    /// Returns `true` if `out_buflen` is not `NULL` and an ICC profile exists for the
    /// given `image_object`.
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetIccProfileDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        buffer: *mut u8,
        buflen: usize,
        out_buflen: *mut usize,
    ) -> FPDF_BOOL;

    /// Creates a new path object at an initial position.
    ///
    ///    `x` - initial horizontal position.
    ///
    ///    `y` - initial vertical position.
    ///
    /// Returns a handle to a new path object.
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewPath(&self, x: c_float, y: c_float) -> FPDF_PAGEOBJECT;

    /// Creates a closed path consisting of a rectangle.
    ///
    ///    `x` - horizontal position for the left boundary of the rectangle.
    ///
    ///    `y` - vertical position for the bottom boundary of the rectangle.
    ///
    ///    `w` - width of the rectangle.
    ///
    ///    `h` - height of the rectangle.
    ///
    /// Returns a handle to the new path object.
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewRect(&self, x: c_float, y: c_float, w: c_float, h: c_float) -> FPDF_PAGEOBJECT;

    /// Gets the bounding box of `page_object`.
    ///
    ///    `page_object`  - handle to a page object.
    ///
    ///    `left`         - pointer where the left coordinate will be stored.
    ///
    ///    `bottom`       - pointer where the bottom coordinate will be stored.
    ///
    ///    `right`        - pointer where the right coordinate will be stored.
    ///
    ///    `top`          - pointer where the top coordinate will be stored.
    ///
    /// On success, returns `true` and fills in the four coordinates.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets the quad points that bounds `page_object`.
    ///
    ///    `page_object`  - handle to a page object.
    ///
    ///    `quad_points`  - pointer where the quadrilateral points will be stored.
    ///
    /// On success, returns `true` and fills in `quad_points`.
    ///
    /// Similar to [PdfiumLibraryBindings::FPDFPageObj_GetBounds], this returns the bounds
    /// of a page object. When the object is rotated by a non-multiple of 90 degrees,
    /// this API returns a tighter bound that cannot be represented with just the four sides
    /// of a rectangle.
    ///
    /// Currently only works the following `page_object` types: `FPDF_PAGEOBJ_TEXT` and `FPDF_PAGEOBJ_IMAGE`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetRotatedBounds(&self, page_object: FPDF_PAGEOBJECT, quad_points: *mut FS_QUADPOINTSF)
    -> FPDF_BOOL;

    /// Sets the blend mode of `page_object`.
    ///
    ///    `page_object`  - handle to a page object.
    ///
    ///    `blend_mode`   - string containing the blend mode.
    ///
    /// Blend mode can be one of following: `Color`, `ColorBurn`, `ColorDodge`, `Darken`,
    /// `Difference`, `Exclusion`, `HardLight`, `Hue`, `Lighten`, `Luminosity`, `Multiply`,
    /// `Normal`, `Overlay`, `Saturation`, `Screen`, `SoftLight`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetBlendMode(&self, page_object: FPDF_PAGEOBJECT, blend_mode: &str);

    /// Sets the stroke RGBA of a page object. Range of values: `0` - `255`.
    ///
    ///    `page_object`  - the handle to the page object.
    ///
    ///    `R`            - the red component for the object's stroke color.
    ///
    ///    `G`            - the green component for the object's stroke color.
    ///
    ///    `B`            - the blue component for the object's stroke color.
    ///
    ///    `A`            - the stroke alpha for the object.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    /// Gets the stroke RGBA of a page object. Range of values: `0` - `255`.
    ///
    ///    `page_object`  - the handle to the page object.
    ///
    ///    `R`            - the red component of the path stroke color.
    ///
    ///    `G`            - the green component of the object's stroke color.
    ///
    ///    `B`            - the blue component of the object's stroke color.
    ///
    ///    `A`            - the stroke alpha of the object.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Sets the stroke width of a page object.
    ///
    ///    `path`   - the handle to the page object.
    ///
    ///    `width`  - the width of the stroke.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: c_float) -> FPDF_BOOL;

    /// Gets the stroke width of a page object.
    ///
    ///    `path`   - the handle to the page object.
    ///
    ///    `width`  - the width of the stroke.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: *mut c_float) -> FPDF_BOOL;

    /// Gets the line join of `page_object`.
    ///
    ///    `page_object`  - handle to a page object.
    ///
    /// Returns the line join, or `-1` on failure.
    ///
    /// Line join can be one of following: `FPDF_LINEJOIN_MITER`, `FPDF_LINEJOIN_ROUND`,
    /// `FPDF_LINEJOIN_BEVEL`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineJoin(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Sets the line join of `page_object`.
    ///
    ///    `page_object`  - handle to a page object.
    ///
    ///    `line_join`    - line join
    ///
    /// Line join can be one of following: `FPDF_LINEJOIN_MITER`, `FPDF_LINEJOIN_ROUND`,
    /// `FPDF_LINEJOIN_BEVEL`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineJoin(&self, page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL;

    /// Gets the line cap of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    /// Returns the line cap, or `-1` on failure.
    ///
    /// Line cap can be one of following: `FPDF_LINECAP_BUTT`, `FPDF_LINECAP_ROUND`,
    /// `FPDF_LINECAP_PROJECTING_SQUARE`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineCap(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Sets the line cap of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `line_cap`    - line cap
    ///
    /// Line cap can be one of following: `FPDF_LINECAP_BUTT`, `FPDF_LINECAP_ROUND`,
    /// `FPDF_LINECAP_PROJECTING_SQUARE`.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineCap(&self, page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL;

    /// Sets the fill RGBA of a page object. Range of values: `0` - `255`.
    ///
    ///    `page_object`  - the handle to the page object.
    ///
    ///    `R`            - the red component for the object's fill color.
    ///
    ///    `G`            - the green component for the object's fill color.
    ///
    ///    `B`            - the blue component for the object's fill color.
    ///
    ///    `A`            - the fill alpha for the object.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    /// Gets the fill RGBA of a page object. Range of values: `0` - `255`.
    ///
    ///    `page_object`  - the handle to the page object.
    ///
    ///    `R`            - the red component of the object's fill color.
    ///
    ///    `G`            - the green component of the object's fill color.
    ///
    ///    `B`            - the blue component of the object's fill color.
    ///
    ///    `A`            - the fill alpha of the object.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Gets the line dash `phase` of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `phase`       - pointer where the dashing phase will be stored.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: *mut c_float) -> FPDF_BOOL;

    /// Sets the line dash phase of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `phase`       - line dash phase.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL;

    /// Gets the line dash array of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    /// Returns the line dash array size, or `-1` on failure.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashCount(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    /// Gets the line dash array of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `dash_array`  - pointer where the dashing array will be stored.
    ///
    ///    `dash_count`  - number of elements in `dash_array`.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *mut c_float,
        dash_count: size_t,
    ) -> FPDF_BOOL;

    /// Sets the line dash array of `page_object`.
    ///
    ///    `page_object` - handle to a page object.
    ///
    ///    `dash_array`  - the dash array.
    ///
    ///    `dash_count`  - number of elements in `dash_array`.
    ///
    ///    `phase`       - the line dash phase.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *const c_float,
        dash_count: size_t,
        phase: c_float,
    ) -> FPDF_BOOL;

    /// Gets the number of segments inside `path`.
    ///
    ///    `path` - handle to a path.
    ///
    /// A segment is a single command, created by e.g. [PdfiumLibraryBindings::FPDFPath_MoveTo],
    /// [PdfiumLibraryBindings::FPDFPath_LineTo], or [PdfiumLibraryBindings::FPDFPath_BezierTo].
    ///
    /// Returns the number of objects in `path`, or `-1` on failure.
    #[allow(non_snake_case)]
    fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int;

    /// Gets segment in `path` at `index`.
    ///
    ///    `path`  - handle to a path.
    ///
    ///    `index` - the index of a segment.
    ///
    /// Returns the handle to the segment, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT;

    /// Gets coordinates of `segment`.
    ///
    ///    `segment`  - handle to a segment.
    ///
    ///    `x`        - the horizontal position of the segment.
    ///
    ///    `y`        - the vertical position of the segment.
    ///
    /// Returns `true` on success, otherwise `x` and `y` is not set.
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetPoint(&self, segment: FPDF_PATHSEGMENT, x: *mut c_float, y: *mut c_float) -> FPDF_BOOL;

    /// Gets the type of `segment`.
    ///
    ///    `segment` - handle to a segment.
    ///
    /// Returns one of the `FPDF_SEGMENT_*` values on success, or `FPDF_SEGMENT_UNKNOWN`
    /// on error.
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int;

    /// Indicates whether or not the `segment` closes the current subpath of a given path.
    ///
    ///    `segment` - handle to a segment.
    ///
    /// Returns close flag for non-`NULL` segment.
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL;

    /// Gets the base name of a font.
    ///
    ///    `font`   - the handle to the font object.
    ///
    ///    `buffer` - the address of a buffer that receives the base font name.
    ///
    ///    `length` - the size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the base name (including the trailing `NUL`
    /// character) on success, 0 on error. The base name is typically the font's
    /// PostScript name. See descriptions of "BaseFont" in ISO 32000-1:2008 spec.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-8 encoding.
    /// If `length` is less than the returned length, or `buffer` is `NULL`, `buffer`
    /// will not be modified.
    #[allow(non_snake_case)]
    fn FPDFFont_GetBaseFontName(&self, font: FPDF_FONT, buffer: *mut c_char, length: size_t) -> size_t;

    /// Gets the family name of a font.
    ///
    ///    `font`   - the handle to the font object.
    ///
    ///    `buffer` - the address of a buffer that receives the font name.
    ///
    ///    `length` - the size, in bytes, of `buffer`.
    ///
    /// Returns the number of bytes in the family name (including the trailing `NUL`
    /// character) on success, 0 on error.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-8 encoding.
    /// If `length` is less than the returned length, or `buffer` is `NULL`, `buffer`
    /// will not be modified.
    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(&self, font: FPDF_FONT, buffer: *mut c_char, length: size_t) -> size_t;

    /// Gets the decoded data from the `font` object.
    ///
    ///    `font`       - The handle to the font object. (Required)
    ///
    ///    `buffer`     - The address of a buffer that receives the font data.
    ///
    ///    `buflen`     - Length of the buffer.
    ///
    ///    `out_buflen` - Pointer to variable that will receive the minimum buffer size
    ///                   to contain the font data. Not filled if the return value is
    ///                   `false`. (Required)
    ///
    /// Returns `true` on success. In which case, `out_buflen` will be filled, and
    /// `buffer` will be filled if it is large enough. Returns `false` if any of the
    /// required parameters are `NULL`.
    ///
    /// The decoded data is the uncompressed font data. i.e. the raw font data after
    /// having all stream filters applied, when the data is embedded.
    ///
    /// If the font is not embedded, then this API will instead return the data for
    /// the substitution font it is using.
    #[allow(non_snake_case)]
    fn FPDFFont_GetFontData(
        &self,
        font: FPDF_FONT,
        buffer: *mut u8,
        buflen: size_t,
        out_buflen: *mut size_t,
    ) -> FPDF_BOOL;

    /// Gets whether `font` is embedded or not.
    ///
    ///    `font` - the handle to the font object.
    ///
    /// Returns 1 if the font is embedded, 0 if it not, or -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetIsEmbedded(&self, font: FPDF_FONT) -> c_int;

    /// Gets the descriptor flags of a font.
    ///
    ///    `font` - the handle to the font object.
    ///
    /// Returns the bit flags specifying various characteristics of the font as
    /// defined in ISO 32000-1:2008, table 123, or -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetFlags(&self, font: FPDF_FONT) -> c_int;

    /// Gets the font weight of a font.
    ///
    ///    `font` - the handle to the font object.
    ///
    /// Returns the font weight, or -1 on failure. Typical values include 400 (normal) and 700 (bold).
    #[allow(non_snake_case)]
    fn FPDFFont_GetWeight(&self, font: FPDF_FONT) -> c_int;

    /// Gets the italic angle of a font.
    ///
    ///    `font`  - the handle to the font object.
    ///
    ///    `angle` - pointer where the italic angle will be stored.
    ///
    /// The italic angle of a `font` is defined as degrees counterclockwise
    /// from vertical. For a font that slopes to the right, this will be negative.
    ///
    /// Returns `true` on success; `angle` unmodified on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetItalicAngle(&self, font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL;

    /// Gets ascent distance of a font.
    ///
    ///    `font`       - the handle to the font object.
    ///
    ///    `font_size`  - the size of the `font`.
    ///
    ///    `ascent`     - pointer where the font ascent will be stored.
    ///
    /// Ascent is the maximum distance in points above the baseline reached by the
    /// glyphs of the `font`. One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Returns `true` on success; `ascent` unmodified on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL;

    /// Gets descent distance of a font.
    ///
    ///    `font`       - the handle to the font object.
    ///
    ///    `font_size`  - the size of the `font`.
    ///
    ///    `descent`    - pointer where the font descent will be stored.
    ///
    /// Descent is the maximum distance in points below the baseline reached by the
    /// glyphs of the `font`. One point is 1/72 inch (around 0.3528 mm).
    ///
    /// Returns `true` on success; `descent` unmodified on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL;

    /// Gets the width of a glyph in a font.
    ///
    ///    `font`       - the handle to the font object.
    ///
    ///    `glyph`      - the glyph.
    ///
    ///    `font_size`  - the size of the font.
    ///
    ///    `width`      - pointer where the glyph width will be stored.
    ///
    /// Glyph width is the distance from the end of the prior glyph to the next
    /// glyph. This will be the vertical distance for vertical writing.
    ///
    /// Returns `true` on success; `width` unmodified on failure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphWidth(
        &self,
        font: FPDF_FONT,
        glyph: c_uint,
        font_size: c_float,
        width: *mut c_float,
    ) -> FPDF_BOOL;

    /// Gets the glyphpath describing how to draw a font glyph.
    ///
    ///    `font`       - the handle to the font object.
    ///
    ///    `glyph`      - the glyph being drawn.
    ///
    ///    `font_size`  - the size of the font.
    ///
    /// Returns the handle to the segment, or `NULL` on faiure.
    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphPath(&self, font: FPDF_FONT, glyph: c_uint, font_size: c_float) -> FPDF_GLYPHPATH;

    /// Gets the number of segments inside `glyphpath`.
    ///
    ///    `glyphpath` - handle to a glyph path.
    ///
    /// Returns the number of objects in `glyphpath` or -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_CountGlyphSegments(&self, glyphpath: FPDF_GLYPHPATH) -> c_int;

    /// Gets the segment in `glyphpath` at `index`.
    ///
    ///    `glyphpath`  - handle to a glyph path.
    ///
    ///    `index`      - the index of a segment.
    ///
    /// Returns the handle to the segment, or `NULL` on faiure.
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_GetGlyphPathSegment(&self, glyphpath: FPDF_GLYPHPATH, index: c_int) -> FPDF_PATHSEGMENT;

    /// Whether the PDF document prefers to be scaled or not.
    ///
    ///    `document`    -   Handle to the loaded document.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintScaling(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    /// Returns the number of copies to be printed.
    ///
    ///    `document`    -   Handle to the loaded document.
    ///
    /// Returns the number of copies to be printed.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetNumCopies(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Page numbers to initialize print dialog box when file is printed.
    ///
    ///    `document`    -   Handle to the loaded document.
    ///
    /// Returns the print page range to be used for printing.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRange(&self, document: FPDF_DOCUMENT) -> FPDF_PAGERANGE;

    /// Returns the number of elements in a `FPDF_PAGERANGE`.
    ///
    ///    `pagerange`   -   Handle to the page range.
    ///
    /// Returns the number of elements in the page range. Returns 0 on error.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeCount(&self, pagerange: FPDF_PAGERANGE) -> size_t;

    /// Returns an element from a `FPDF_PAGERANGE`.
    ///
    ///    `pagerange`   -   Handle to the page range.
    ///
    ///    `index`       -   Index of the element.
    ///
    /// Returns the value of the element in the page range at a given index.
    /// Returns -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeElement(&self, pagerange: FPDF_PAGERANGE, index: size_t) -> c_int;

    /// Returns the paper handling option to be used when printing from the print dialog.
    ///
    ///    `document`    -   Handle to the loaded document.
    ///
    /// Returns the paper handling option to be used when printing.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetDuplex(&self, document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE;

    /// Gets the contents for a viewer ref, with a given key. The value must
    /// be of type "name".
    ///
    ///    `document`    -   Handle to the loaded document.
    ///
    ///    `key`         -   Name of the key in the viewer pref dictionary,
    ///                      encoded in UTF-8.
    ///
    ///    `buffer`      -   Caller-allocate buffer to receive the key, or `NULL`
    ///                      to query the required length.
    ///
    ///    `length`      -   Length of the buffer.
    ///
    /// Returns the number of bytes in the contents, including the `NULL` terminator.
    /// Thus if the return value is 0, then that indicates an error, such
    /// as when `document` is invalid. If `length` is less than the required length, or
    /// `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetName(
        &self,
        document: FPDF_DOCUMENT,
        key: &str,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    /// Gets the count of named destinations in the PDF document.
    ///
    ///    `document`    -   Handle to a document
    ///
    /// Returns the count of named destinations.
    #[allow(non_snake_case)]
    fn FPDF_CountNamedDests(&self, document: FPDF_DOCUMENT) -> FPDF_DWORD;

    /// Gets a the destination handle for the given name.
    ///
    ///    `document`    -   Handle to the loaded document.
    ///
    ///    `name`        -   The name of a destination.
    ///
    /// Returns a handle to the destination.
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDestByName(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_DEST;

    /// Gets the named destination by index.
    ///
    ///    `document`        -   Handle to a document
    ///
    ///    `index`           -   The index of a named destination.
    ///
    ///    `buffer`          -   The buffer to store the destination name, used as `wchar_t*`.
    ///
    ///    `buflen [in/out]` -   Size of the buffer in bytes on input,
    ///                          length of the result in bytes on output
    ///                          or -1 if the buffer is too small.
    ///
    /// Returns the destination handle for a given index, or `NULL` if there is no
    /// named destination corresponding to `index`.
    ///
    /// Call this function twice to get the name of the named destination:
    /// * First time pass in `buffer` as `NULL` and get `buflen`.
    /// * Second time pass in allocated `buffer` and `buflen` to retrieve `buffer`,
    ///   which should be used as `wchar_t*`.
    ///
    /// If `buflen` is not sufficiently large, it will be set to -1 upon return.
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDest(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: *mut c_long,
    ) -> FPDF_DEST;

    /// Gets the number of embedded files in `document`.
    ///
    ///    `document` - handle to a document.
    ///
    /// Returns the number of embedded files in `document`.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachmentCount(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Adds an embedded file with `name` in `document`. If `name` is empty, or if
    /// `name` is the name of a existing embedded file in `document`, or if
    /// `document`'s embedded file name tree is too deep (i.e. `document` has too
    /// many embedded files already), then a new attachment will not be added.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or NULL on failure.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFDoc_AddAttachment_str].
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment(&self, document: FPDF_DOCUMENT, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFDoc_AddAttachment].
    ///
    /// Adds an embedded file with `name` in `document`. If `name` is empty, or if
    /// `name` is the name of a existing embedded file in `document`, or if
    /// `document`'s embedded file name tree is too deep (i.e. `document` has too
    /// many embedded files already), then a new attachment will not be added.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment_str(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_ATTACHMENT {
        self.FPDFDoc_AddAttachment(
            document,
            get_pdfium_utf16le_bytes_from_str(name).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the embedded attachment at `index` in `document`. Note that the returned
    /// attachment handle is only valid while `document` is open.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `index`    - the index of the requested embedded file.
    ///
    /// Returns the handle to the attachment object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT;

    /// Deletes the embedded attachment at `index` in `document`. Note that this does
    /// not remove the attachment data from the PDF file; it simply removes the
    /// file's entry in the embedded files name tree so that it does not appear in
    /// the attachment list. This behavior may change in the future.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `index`    - the index of the embedded file to be deleted.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFDoc_DeleteAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL;

    /// Gets the name of the `attachment` file. `buffer` is only modified if `buflen`
    /// is longer than the length of the file name. On errors, `buffer` is unmodified
    /// and the returned length is 0.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `buffer`     - buffer for holding the file name, encoded in UTF-16LE.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    /// Returns the length of the file name in bytes.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetName(&self, attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong;

    /// Checks if the params dictionary of `attachment` has `key` as a key.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `key`        - the key to look for, encoded in UTF-8.
    ///
    /// Returns `true` if `key` exists.
    #[allow(non_snake_case)]
    fn FPDFAttachment_HasKey(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_BOOL;

    /// Gets the type of the value corresponding to `key` in the params dictionary of
    /// the embedded `attachment`.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `key`        - the key to look for, encoded in UTF-8.
    ///
    /// Returns the type of the dictionary value.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetValueType(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_OBJECT_TYPE;

    /// Sets the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`, overwriting the existing value if any. The value
    /// type should be FPDF_OBJECT_STRING after this function call succeeds.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `key`        - the key to the dictionary entry, encoded in UTF-8.
    ///
    ///    `value`      - the string value to be set, encoded in UTF-16LE.
    ///
    /// Returns `true` on success.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAttachment_SetStringValue_str].
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAttachment_SetStringValue].
    ///
    /// Sets the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`, overwriting the existing value if any. The value
    /// type should be FPDF_OBJECT_STRING after this function call succeeds.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `key`        - the key to the dictionary entry.
    ///
    ///    `value`      - the string value to be set.
    ///
    /// Returns `true` on success.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue_str(&self, attachment: FPDF_ATTACHMENT, key: &str, value: &str) -> FPDF_BOOL {
        self.FPDFAttachment_SetStringValue(
            attachment,
            key,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`. `buffer` is only modified if `buflen` is longer
    /// than the length of the string value. Note that if `key` does not exist in the
    /// dictionary or if `key`'s corresponding value in the dictionary is not a
    /// string (i.e. the value is not of type FPDF_OBJECT_STRING or
    /// FPDF_OBJECT_NAME), then an empty string would be copied to `buffer` and the
    /// return value would be 2. On other errors, nothing would be added to `buffer`
    /// and the return value would be 0.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `key`        - the key to the requested string value, encoded in UTF-8.
    ///
    ///    `buffer`     - buffer for holding the string value encoded in UTF-16LE.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    /// Returns the length of the dictionary value string in bytes.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Sets the file data of `attachment`, overwriting the existing file data if any.
    /// The creation date and checksum will be updated, while all other dictionary
    /// entries will be deleted. Note that only contents with `len` smaller than
    /// INT_MAX is supported.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `contents`   - buffer holding the file data to write to `attachment`.
    ///
    ///    `len`        - length of file data in bytes.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        document: FPDF_DOCUMENT,
        contents: *const c_void,
        len: c_ulong,
    ) -> FPDF_BOOL;

    /// Gets the file data of `attachment`.
    ///
    /// When the attachment file data is readable, true is returned, and `out_buflen`
    /// is updated to indicate the file data size. `buffer` is only modified if
    /// `buflen` is non-null and long enough to contain the entire file data. Callers
    /// must check both the return value and the input `buflen` is no less than the
    /// returned `out_buflen` before using the data.
    ///
    /// Otherwise, when the attachment file data is unreadable or when `out_buflen`
    /// is null, false is returned and `buffer` and `out_buflen` remain unmodified.
    ///
    ///    `attachment` - handle to an attachment.
    ///
    ///    `buffer`     - buffer for holding the file data from `attachment`.
    ///
    ///    `buflen`     - length of the buffer in bytes.
    ///
    ///    `out_buflen` - pointer to the variable that will receive the minimum buffer
    ///                   size to contain the file data of `attachment`.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Get the MIME type (Subtype) of the embedded file `attachment`. `buffer` is
    /// only modified if `buflen` is longer than the length of the MIME type string.
    /// If the Subtype is not found or if there is no file stream, an empty string
    /// would be copied to `buffer` and the return value would be 2. On other errors,
    /// nothing would be added to `buffer` and the return value would be 0.
    ///
    ///   `attachment` - handle to an attachment.
    ///
    ///   `buffer`     - buffer for holding the MIME type string encoded in UTF-16LE.
    ///
    ///   `buflen`     - length of the buffer in bytes.
    ///
    /// Returns the length of the MIME type string in bytes.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetSubtype(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Determines if `document` represents a tagged PDF.
    ///
    /// For the definition of tagged PDF, see 10.7 "Tagged PDF" in PDF Reference 1.7.
    ///
    ///    `document` - handle to a document.
    ///
    /// Returns `true` if `document` is a tagged PDF.
    #[allow(non_snake_case)]
    fn FPDFCatalog_IsTagged(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    /// Sets the language of `document` to `language`.
    ///
    ///    `document` - handle to a document.
    ///
    ///    `language` - the language to set to.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFCatalog_SetLanguage(&self, document: FPDF_DOCUMENT, language: &str) -> FPDF_BOOL;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_is_true() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        assert!(!pdfium.bindings().is_true(0));
        assert!(pdfium.bindings().is_true(1));
        assert!(pdfium.bindings().is_true(-1));

        Ok(())
    }
}
