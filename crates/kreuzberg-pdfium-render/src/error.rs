//! Defines the [PdfiumError] enum, used to wrap Pdfium errors as `Err` values.

use crate::bindgen::{
    FPDF_ERR_FILE, FPDF_ERR_FORMAT, FPDF_ERR_PAGE, FPDF_ERR_PASSWORD, FPDF_ERR_SECURITY, FPDF_ERR_UNKNOWN,
};
use std::error::Error;
use std::ffi::IntoStringError;
use std::fmt::{Display, Formatter, Result};
use std::num::ParseIntError;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(doc)]
use crate::pdfium::Pdfium;

/// A wrapped internal library error from Pdfium's `FPDF_ERR_*` constant values.
///
/// Pdfium only provides detailed internal error information for document loading functions.
/// All other functions in the Pdfium API return a value indicating success or failure,
/// but otherwise detailed error information for failed API calls is not available. In these
/// cases, an error value of [PdfiumInternalError::Unknown] will be returned.
// For more information, see: https://github.com/ajrcarey/pdfium-render/issues/78
#[derive(Debug)]
pub enum PdfiumInternalError {
    /// The document could not be loaded due to a file system error.
    FileError = FPDF_ERR_FILE as isize,

    /// The document could not be loaded due to a format parsing error.
    FormatError = FPDF_ERR_FORMAT as isize,

    /// The document could not be loaded because the wrong password was supplied.
    PasswordError = FPDF_ERR_PASSWORD as isize,

    /// The document could not be loaded because of the document's security settings.
    SecurityError = FPDF_ERR_SECURITY as isize,

    /// The page could not be loaded due to an internal error.
    PageError = FPDF_ERR_PAGE as isize,

    /// A generic error value returned in all other unhandled situations.
    Unknown = FPDF_ERR_UNKNOWN as isize,
}

/// A wrapper enum for handling Pdfium errors as standard Rust `Err` values.
#[derive(Debug)]
pub enum PdfiumError {
    /// The Pdfium WASM module has not been initialized.
    ///
    /// It is essential that the exported `initialize_pdfium_render()` function be called
    /// from Javascript _before_ calling any `pdfium-render` function from within your Rust code.
    /// See: <https://github.com/ajrcarey/pdfium-render/blob/master/examples/index.html>
    #[cfg(target_arch = "wasm32")]
    PdfiumWasmModuleNotInitialized,

    /// An error occurred during dynamic binding to an external Pdfium library.
    #[cfg(not(target_arch = "wasm32"))]
    LoadLibraryError(libloading::Error),

    /// An error occurred during dynamic binding while converting an FPDF_* function name
    /// to a C string. The wrapped string value contains more information.
    #[cfg(not(target_arch = "wasm32"))]
    LoadLibraryFunctionNameError(String),

    /// The global library bindings have already been initialized based on the
    /// first call to [Pdfium::new]. Bindings initialization can only occur once
    /// during the lifetime of the program.
    PdfiumLibraryBindingsAlreadyInitialized,

    UnrecognizedPath,
    PdfClipPathSegmentIndexOutOfBounds,
    PageIndexOutOfBounds,
    LinkIndexOutOfBounds,
    UnknownBitmapFormat,
    UnknownBitmapRotation,
    UnknownFormType,
    UnknownFormFieldType,
    UnknownActionType,
    UnknownAppearanceMode,
    UnknownPageAnnotationVariableTextJustificationType,
    PageObjectIndexOutOfBounds,
    PageAnnotationIndexOutOfBounds,
    OwnershipNotAttachedToDocument,
    OwnershipNotAttachedToPage,
    OwnershipAlreadyAttachedToDifferentPage,
    OwnershipNotAttachedToAnnotation,
    FormFieldOptionIndexOutOfBounds,
    FormFieldAppearanceStreamUndefined,
    PageFlattenFailure,
    PageMissingEmbeddedThumbnail,
    UnknownPdfPageObjectType,
    UnknownPdfPageTextRenderMode,
    UnknownPdfPagePathFillMode,
    UnknownPdfAnnotationType,
    UnknownPdfDestinationViewType,
    UnknownPdfSecurityHandlerRevision,
    UnsupportedPdfPageObjectType,
    TextSegmentIndexOutOfBounds,
    TextSearchTargetIsEmpty,
    CharIndexOutOfBounds,
    NoCharsInPageObject,
    NoCharsInAnnotation,
    NoCharsInRect,
    ImageObjectFilterIndexOutOfBounds,
    ImageObjectFilterIndexInBoundsButFilterUndefined,
    UnknownPdfColorSpace,
    InvalidTransformationMatrix,
    FontGlyphIndexOutOfBounds,
    UnknownPathSegmentType,
    NoPagesInDocument,
    NoPageObjectsInCollection,
    NoPageLinksInCollection,
    NoAnnotationsInCollection,
    PageObjectNotCopyable,
    ImageObjectFiltersNotCopyable,
    PathObjectBezierControlPointsNotCopyable,
    PathObjectUnknownSegmentTypeNotCopyable,
    GroupContainsNonCopyablePageObjects,
    SourcePageIndexNotInCache,
    NoUriForAction,
    DestinationPageIndexNotAvailable,
    DestinationPageLocationNotAvailable,
    PageAnnotationAttachmentPointIndexOutOfBounds,
    NoAttachmentPointsInPageAnnotation,
    CoordinateConversionFunctionIndicatedError,

    /// Pdfium does not safely support moving page object ownership from one document to another.
    CannotMoveObjectAcrossDocuments,

    /// Pdfium does not support adding or removing page objects from the page objects
    /// collection inside a PdfPageXObjectFormObject object.
    PageObjectsCollectionIsImmutable,

    /// A call to `FPDFDest_GetView()` returned a valid `FPDFDEST_VIEW_*` value, but the number
    /// of view parameters returned does not match the PDF specification.
    PdfDestinationViewInvalidParameters,

    /// A [ParseIntError] occurred while attempting to parse a `PdfColor` from a hexadecimal string
    /// in `PdfColor::from_hex()`.
    ParseHexadecimalColorError(ParseIntError),

    /// The hexadecimal string given to `PdfColor::from_hex()` was not either exactly 7 or 9
    /// characters long.
    ParseHexadecimalColorUnexpectedLength,

    /// The leading `#` character was not found while attempting to parse a `PdfColor` from
    /// a hexadecimal string in `PdfColor::from_hex()`.
    ParseHexadecimalColorMissingLeadingHash,

    /// An error occurred converting a byte stream into a `CString`.
    CStringConversionError(IntoStringError),

    /// Two data buffers are expected to have the same size, but they do not.
    DataBufferLengthMismatch,

    /// The setting cannot be returned because this `PdfPageGroupObject` is empty.
    EmptyPageObjectGroup,

    /// A call to a internal Pdfium `FPDF_*` function returned a value indicating failure.
    ///
    /// For Pdfium functions that return enumerations, this means the function returned
    /// a value of -1 rather than a valid enumeration constant.
    ///
    /// For Pdfium functions that return C-style boolean integers, this means that the function
    /// returned a value other than `PdfiumLibraryBindings::TRUE`.
    PdfiumFunctionReturnValueIndicatedFailure,

    /// A call to a Pdfium function that returns a standard 8-bit color component value
    /// (for example, `FPDFPageObj_GetStrokeColor()` and `FPDFPageObj_GetStrokeColor()`)
    /// successfully returned a value, but the value could not be converted from a c_int
    /// to a standard Rust u8.
    UnableToConvertPdfiumColorValueToRustu8(std::num::TryFromIntError),

    /// The browser's built-in `Window` object could not be retrieved.
    WebSysWindowObjectNotAvailable,

    #[cfg(target_arch = "wasm32")]
    /// A JsValue returned from a function call was set to JsValue::UNDEFINED instead of
    /// a valid value of the expected type.
    JsValueUndefined,

    #[cfg(target_arch = "wasm32")]
    /// An error was returned when attempting to use the browser's built-in `fetch()` API.
    WebSysFetchError(JsValue),

    #[cfg(target_arch = "wasm32")]
    /// An invalid Response object was returned when attempting to use the browser's built-in `fetch()` API.
    WebSysInvalidResponseError,

    #[cfg(target_arch = "wasm32")]
    /// An error was returned when attempting to construct a `Blob` object from a byte buffer.
    JsSysErrorConstructingBlobFromBytes,

    #[cfg(target_arch = "wasm32")]
    /// An error occurred when attempting to retrieve the function table for the compiled
    /// Pdfium WASM module.
    JsSysErrorRetrievingFunctionTable(JsValue),

    #[cfg(target_arch = "wasm32")]
    /// An error occurred when attempting to retrieve an exported function from
    /// `pdfium-render`'s WASM module.
    JsSysErrorRetrievingFunction(JsValue),

    #[cfg(target_arch = "wasm32")]
    /// An error occurred when attempting to update an entry in Pdfium's WASM function table.
    JsSysErrorPatchingFunctionTable(JsValue),

    #[cfg(target_arch = "wasm32")]
    /// No previously cached function was available for a WASM function table restore operation.
    ///
    /// This error should never occur; if it does, it indicates a programming error in pdfium-render.
    /// Please file an issue: https://github.com/ajrcarey/pdfium-render/issues
    NoPreviouslyCachedFunctionSet,

    /// An error occurred during an image processing operation.
    ImageError,

    /// Dimensions of `Image::Image` are specified in `u32`, but bitmaps in Pdfium are sized in
    /// `c_int` (`i32`), meaning that an `Image::Image` can have dimensions that overflow
    /// the maximum size of a Pdfium bitmap. As a compromise, Image dimensions in `pdfium-render`
    /// are limited to `u16`.
    ///
    /// This error indicates that an `Image::Image` had a width or height larger than the maximum
    /// `u16` size allowed by `pdfium-render`.
    ImageSizeOutOfBounds,

    /// An I/O error occurred during a Pdfium file operation.
    IoError(std::io::Error),

    /// A wrapped internal library error from Pdfium's `FPDF_ERR_*` constant values.
    PdfiumLibraryInternalError(PdfiumInternalError),
}

impl Display for PdfiumError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{self:#?}")
    }
}

impl Error for PdfiumError {}
