//! Defines the [PdfPageTextObject] struct, exposing functionality related to a single
//! page object defining a piece of formatted text.

use crate::bindgen::{
    FPDF_DOCUMENT, FPDF_FONT, FPDF_PAGEOBJECT, FPDF_TEXT_RENDERMODE, FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_CLIP,
    FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL, FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_CLIP,
    FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE, FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE_CLIP,
    FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_INVISIBLE, FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE,
    FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE_CLIP, FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_UNKNOWN, FPDF_WCHAR,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::document::fonts::ToPdfFontToken;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectCommon, PdfPageObjectOwnership};

use crate::pdf::document::PdfDocument;
use crate::pdf::font::PdfFont;
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::utils::mem::create_byte_buffer;
use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
use crate::{create_transform_getters, create_transform_setters};

use {crate::pdf::document::page::text::PdfPageText, crate::pdf::document::page::text::chars::PdfPageTextChars};

#[cfg(doc)]
use {
    crate::pdf::document::page::PdfPage, crate::pdf::document::page::object::PdfPageObjectType,
    crate::pdf::document::page::objects::common::PdfPageObjectsCommon,
};

/// The text rendering modes supported by the PDF standard, as listed in table 5.3
/// on page 402 in the PDF Reference manual version 1.7.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum PdfPageTextRenderMode {
    /// The text render mode is not recognized by Pdfium.
    Unknown = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_UNKNOWN as isize,

    /// The text will be filled, but not stroked.
    FilledUnstroked = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL as isize,

    /// The text will be stroked, but not filled.
    StrokedUnfilled = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE as isize,

    /// The text will be filled, then stroked.
    FilledThenStroked = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE as isize,

    /// The text will be neither filled nor stroked. It will still take up size in the layout, however.
    Invisible = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_INVISIBLE as isize,

    /// The text will be filled and added to the path for clipping.
    FilledUnstrokedClipping = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_CLIP as isize,

    /// The text will be stroked and added to the path for clipping.
    StrokedUnfilledClipping = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE_CLIP as isize,

    /// The text will be filled, then stroked, and added to the path for clipping.
    FilledThenStrokedClipping = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE_CLIP as isize,

    /// The text will be neither filled nor stroked, only added to the path for clipping.
    InvisibleClipping = FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_CLIP as isize,
}

impl PdfPageTextRenderMode {
    #[inline]
    pub(crate) fn from_pdfium(value: i32) -> Result<PdfPageTextRenderMode, PdfiumError> {
        match value {
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_UNKNOWN => Ok(PdfPageTextRenderMode::Unknown),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL => Ok(PdfPageTextRenderMode::FilledUnstroked),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE => Ok(PdfPageTextRenderMode::StrokedUnfilled),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE => Ok(PdfPageTextRenderMode::FilledThenStroked),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_INVISIBLE => Ok(PdfPageTextRenderMode::Invisible),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_CLIP => Ok(PdfPageTextRenderMode::FilledUnstrokedClipping),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE_CLIP => Ok(PdfPageTextRenderMode::StrokedUnfilledClipping),
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE_CLIP => {
                Ok(PdfPageTextRenderMode::FilledThenStrokedClipping)
            }
            FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_CLIP => Ok(PdfPageTextRenderMode::InvisibleClipping),
            _ => Err(PdfiumError::UnknownPdfPageTextRenderMode),
        }
    }

    #[inline]
    #[allow(dead_code)]
    // The as_pdfium() function is not currently used, but we expect it to be in future
    pub(crate) fn as_pdfium(&self) -> FPDF_TEXT_RENDERMODE {
        match self {
            PdfPageTextRenderMode::Unknown => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_UNKNOWN,
            PdfPageTextRenderMode::FilledUnstroked => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL,
            PdfPageTextRenderMode::StrokedUnfilled => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE,
            PdfPageTextRenderMode::FilledThenStroked => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE,
            PdfPageTextRenderMode::Invisible => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_INVISIBLE,
            PdfPageTextRenderMode::FilledUnstrokedClipping => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_CLIP,
            PdfPageTextRenderMode::StrokedUnfilledClipping => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_STROKE_CLIP,
            PdfPageTextRenderMode::FilledThenStrokedClipping => {
                FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_FILL_STROKE_CLIP
            }
            PdfPageTextRenderMode::InvisibleClipping => FPDF_TEXT_RENDERMODE_FPDF_TEXTRENDERMODE_CLIP,
        }
    }
}

/// A single [PdfPageObject] of type [PdfPageObjectType::Text]. The page object defines a single
/// piece of formatted text.
///
/// Page objects can be created either attached to a [PdfPage] (in which case the page object's
/// memory is owned by the containing page) or detached from any page (in which case the page
/// object's memory is owned by the object). Page objects are not rendered until they are
/// attached to a page; page objects that are never attached to a page will be lost when they
/// fall out of scope.
///
/// The simplest way to create a page text object that is immediately attached to a page
/// is to call the [PdfPageObjectsCommon::create_text_object()] function.
///
/// Creating a detached page text object offers more scope for customization, but you must
/// add the object to a containing [PdfPage] manually. To create a detached page text object,
/// use the [PdfPageTextObject::new()] function. The detached page text object can later
/// be attached to a page by using the [PdfPageObjectsCommon::add_text_object()] function.
pub struct PdfPageTextObject<'a> {
    object_handle: FPDF_PAGEOBJECT,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageTextObject<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        object_handle: FPDF_PAGEOBJECT,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageTextObject {
            object_handle,
            ownership,
            bindings,
        }
    }

    /// Creates a new [PdfPageTextObject] from the given arguments. The returned page object
    /// will not be rendered until it is added to a [PdfPage] using the
    /// [PdfPageObjectsCommon::add_text_object()] function.
    ///
    /// A single space will be used if the given text is empty, in order to avoid
    /// unexpected behaviour from Pdfium when dealing with empty strings.
    // Specifically, `FPDFPageObj_SetText()` will crash if we try to apply an empty string to a
    // text object, and `FPDFText_LoadPage()` will crash if any text object on the page contains
    // an empty string (so it isn't enough to avoid calling `FPDFPageObj_SetText()` for an empty
    // text object, we _have_ to set a non-empty string to avoid segfaults).
    #[inline]
    pub fn new(
        document: &PdfDocument<'a>,
        text: impl ToString,
        font: impl ToPdfFontToken,
        font_size: PdfPoints,
    ) -> Result<Self, PdfiumError> {
        Self::new_from_handles(
            document.handle(),
            text,
            font.token().handle(),
            font_size,
            document.bindings(),
        )
    }

    // Take raw `FPDF_DOCUMENT` and `FPDF_FONT` handles to avoid cascading lifetime problems
    // associated with borrowing `PdfDocument<'a>` and/or `PdfFont<'a>`.
    pub(crate) fn new_from_handles(
        document: FPDF_DOCUMENT,
        text: impl ToString,
        font: FPDF_FONT,
        font_size: PdfPoints,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Result<Self, PdfiumError> {
        let handle = bindings.FPDFPageObj_CreateTextObj(document, font, font_size.value);

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            let mut result = PdfPageTextObject {
                object_handle: handle,
                ownership: PdfPageObjectOwnership::unowned(),
                bindings,
            };

            result.set_text(text)?;

            Ok(result)
        }
    }

    /// Returns the text rendering mode for the text contained within this [PdfPageTextObject].
    pub fn render_mode(&self) -> PdfPageTextRenderMode {
        PdfPageTextRenderMode::from_pdfium(self.bindings().FPDFTextObj_GetTextRenderMode(self.object_handle))
            .unwrap_or(PdfPageTextRenderMode::Unknown)
    }

    /// Returns the effective size of the text when rendered, taking into account both the
    /// font size specified in this text object as well as any vertical scale factor applied
    /// to the text object's transformation matrix.
    ///
    /// To retrieve only the specified font size, ignoring any vertical scaling, use the
    /// [PdfPageTextObject::unscaled_font_size()] function.
    #[inline]
    pub fn scaled_font_size(&self) -> PdfPoints {
        PdfPoints::new(self.unscaled_font_size().value * self.get_vertical_scale())
    }

    /// Returns the font size of the text specified in this [PdfPageTextObject].
    ///
    /// Note that the effective size of the text when rendered may differ from the font size
    /// if a scaling factor has been applied to this text object's transformation matrix.
    /// To retrieve the effective font size, taking vertical scaling into account, use the
    /// [PdfPageTextObject::scaled_font_size()] function.
    pub fn unscaled_font_size(&self) -> PdfPoints {
        let mut result = 0.0;

        if self
            .bindings()
            .is_true(self.bindings().FPDFTextObj_GetFontSize(self.object_handle, &mut result))
        {
            PdfPoints::new(result)
        } else {
            PdfPoints::ZERO
        }
    }

    /// Returns the [PdfFont] used to render the text contained within this [PdfPageTextObject].
    pub fn font(&self) -> PdfFont<'_> {
        PdfFont::from_pdfium(
            self.bindings().FPDFTextObj_GetFont(self.object_handle),
            self.bindings(),
            None,
            false,
        )
    }

    /// Returns the text contained within this [PdfPageTextObject].
    ///
    /// Text retrieval in Pdfium is handled by the [PdfPageText] object owned by the [PdfPage]
    /// containing this [PdfPageTextObject]. If this text object has not been attached to a page
    /// then text retrieval will be unavailable and an empty string will be returned.
    ///
    /// When retrieving the text from many [PdfPageTextObject] objects (for instance, as part of
    /// a loop or an iterator), it may be faster to open the [PdfPageText] object once and keep
    /// it open while processing the text objects, like so:
    ///
    /// ```
    /// let text_page = page.text()?; // Opens the text page once.
    ///
    /// for object in <some object iterator> {
    ///     let object_text = text_page.for_object(object)?;
    /// }
    /// ```
    ///
    /// The [PdfPageText] object will be closed when the binding to it (`text_page` in the example above)
    /// falls out of scope.
    pub fn text(&self) -> String {
        // Retrieving the text from Pdfium is a two-step operation. First, we call
        // FPDFTextObj_GetText() with a null buffer; this will retrieve the length of
        // the text in bytes. If the length is zero, then there is no text associated
        // with the page object.

        // If the length is non-zero, then we reserve a byte buffer of the given
        // length and call FPDFTextObj_GetText() again with a pointer to the buffer;
        // this will write the text to the buffer in UTF16-LE format.

        let page_handle = match self.ownership() {
            PdfPageObjectOwnership::Page(ownership) => Some(ownership.page_handle()),
            PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.page_handle()),
            _ => None,
        };

        if let Some(page_handle) = page_handle {
            let text_handle = self.bindings().FPDFText_LoadPage(page_handle);

            if !text_handle.is_null() {
                let buffer_length =
                    self.bindings()
                        .FPDFTextObj_GetText(self.object_handle(), text_handle, std::ptr::null_mut(), 0);

                if buffer_length == 0 {
                    // There is no text.

                    return String::new();
                }

                let mut buffer = create_byte_buffer(buffer_length as usize);

                let result = self.bindings().FPDFTextObj_GetText(
                    self.object_handle(),
                    text_handle,
                    buffer.as_mut_ptr() as *mut FPDF_WCHAR,
                    buffer_length,
                );

                assert_eq!(result, buffer_length);

                self.bindings.FPDFText_ClosePage(text_handle);

                get_string_from_pdfium_utf16le_bytes(buffer).unwrap_or_default()
            } else {
                // The PdfPage containing this page object does not have an associated
                // FPDF_TEXTPAGE object.

                String::new()
            }
        } else {
            // This page object is not contained by a PdfPage.

            String::new()
        }
    }

    /// Sets the text contained within this [PdfPageTextObject], replacing any existing text.
    ///
    /// A single space will be used if the given text is empty, in order to avoid
    /// unexpected behaviour from Pdfium when dealing with an empty string.
    pub fn set_text(&mut self, text: impl ToString) -> Result<(), PdfiumError> {
        let text = text.to_string();

        let text = if text.is_empty() { " " } else { text.as_str() };

        if self
            .bindings()
            .is_true(self.bindings().FPDFText_SetText_str(self.object_handle(), text))
        {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Sets the text rendering mode for the text contained within this [PdfPageTextObject].
    pub fn set_render_mode(&mut self, render_mode: PdfPageTextRenderMode) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFTextObj_SetTextRenderMode(self.object_handle(), render_mode.as_pdfium()),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns a collection of the characters contained within this [PdfPageTextObject],
    /// using character retrieval functionality provided by the given [PdfPageText] object.
    #[inline]
    pub fn chars(&self, text: &'a PdfPageText<'a>) -> Result<PdfPageTextChars<'a>, PdfiumError> {
        text.chars_for_object(self)
    }

    /// Returns `true` if any of the characters contained within this [PdfPageTextObject] have a
    /// glyph shape that descends below the font baseline.
    ///
    /// Character retrieval functionality is provided by the given [PdfPageText] object.
    #[inline]
    pub fn has_descenders(&self, text: &PdfPageText) -> Result<bool, PdfiumError> {
        self.chars(text)
            .map(|chars| chars.iter().any(|char| char.has_descender()))
    }

    /// Returns the descent of this [PdfPageTextObject]. The descent is the maximum distance below
    /// the baseline reached by any glyph in any of the characters contained in this text object,
    /// expressed as a negative points value.
    ///
    /// Character retrieval and bounds measurement is provided by the given [PdfPageText] object.
    pub fn descent(&self, text: &PdfPageText) -> Result<PdfPoints, PdfiumError> {
        let object_bottom = self.get_vertical_translation();

        let mut maximum_descent = object_bottom;

        for char in self.chars(text)?.iter() {
            let char_bottom = char.tight_bounds()?.bottom();

            if char_bottom < maximum_descent {
                maximum_descent = char_bottom;
            }
        }

        Ok(maximum_descent - object_bottom)
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "this [PdfPageTextObject]",
        "this [PdfPageTextObject].",
        "this [PdfPageTextObject],"
    );

    // The transform_impl() function required by the create_transform_setters!() macro
    // is provided by the PdfPageObjectPrivate trait.

    create_transform_getters!(
        "this [PdfPageTextObject]",
        "this [PdfPageTextObject].",
        "this [PdfPageTextObject],"
    );

    // The get_matrix_impl() function required by the create_transform_getters!() macro
    // is provided by the PdfPageObjectPrivate trait.
}

impl<'a> PdfPageObjectPrivate<'a> for PdfPageTextObject<'a> {
    #[inline]
    fn object_handle(&self) -> FPDF_PAGEOBJECT {
        self.object_handle
    }

    #[inline]
    fn ownership(&self) -> &PdfPageObjectOwnership {
        &self.ownership
    }

    #[inline]
    fn set_ownership(&mut self, ownership: PdfPageObjectOwnership) {
        self.ownership = ownership;
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }

    #[inline]
    fn is_copyable_impl(&self) -> bool {
        true
    }

    #[inline]
    fn try_copy_impl<'b>(
        &self,
        document: FPDF_DOCUMENT,
        bindings: &'b dyn PdfiumLibraryBindings,
    ) -> Result<PdfPageObject<'b>, PdfiumError> {
        let mut copy = PdfPageTextObject::new_from_handles(
            document,
            self.text(),
            self.font().handle(),
            self.unscaled_font_size(),
            bindings,
        )?;

        copy.set_fill_color(self.fill_color()?)?;
        copy.set_stroke_color(self.stroke_color()?)?;
        copy.set_stroke_width(self.stroke_width()?)?;
        copy.set_line_join(self.line_join()?)?;
        copy.set_line_cap(self.line_cap()?)?;
        copy.reset_matrix(self.matrix()?)?;

        Ok(PdfPageObject::Text(copy))
    }
}

impl<'a> Drop for PdfPageTextObject<'a> {
    /// Closes this [PdfPageTextObject], releasing held memory.
    fn drop(&mut self) {
        self.drop_impl();
    }
}
