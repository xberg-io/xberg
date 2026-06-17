//! Defines the [PdfPageTextChar] struct, exposing functionality related to a single character
//! in a [PdfPageTextChars] collection.

use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGE, FPDF_TEXTPAGE, FS_MATRIX, FS_RECTF};
use crate::bindings::PdfiumLibraryBindings;
use crate::create_transform_getters;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::color::PdfColor;
use crate::pdf::document::page::object::text::PdfPageTextRenderMode;
use crate::pdf::document::page::text::chars::PdfPageTextCharIndex;
use crate::pdf::font::{FpdfFontDescriptorFlags, PdfFontWeight};
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use crate::utils::mem::create_byte_buffer;
use std::convert::TryInto;
use std::ffi::c_void;

use {crate::pdf::document::page::PdfPageObjectOwnership, crate::pdf::document::page::object::text::PdfPageTextObject};

#[cfg(doc)]
use crate::pdf::document::page::text::PdfPageTextChars;

/// A single character in a [PdfPageTextChars] collection.
pub struct PdfPageTextChar<'a> {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    text_page_handle: FPDF_TEXTPAGE,
    index: i32,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageTextChar<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        text_page_handle: FPDF_TEXTPAGE,
        index: i32,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageTextChar {
            document_handle,
            page_handle,
            text_page_handle,
            index,
            bindings,
        }
    }

    /// Returns the internal `FPDF_DOCUMENT` handle of the [PdfDocument] containing this [PdfPageTextChar].
    #[inline]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the internal `FPDF_PAGE` handle of the [PdfPage] containing this [PdfPageTextChar].
    #[inline]
    pub(crate) fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    /// Returns the internal `FPDF_TEXTPAGE` handle for this [PdfPageTextChar].
    #[inline]
    pub(crate) fn text_page_handle(&self) -> FPDF_TEXTPAGE {
        self.text_page_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageTextChar].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    #[inline]
    pub fn index(&self) -> PdfPageTextCharIndex {
        self.index as PdfPageTextCharIndex
    }

    /// Returns the raw Unicode literal value for this character.
    ///
    /// To return Rust's Unicode `char` representation of this Unicode literal, use the
    /// [PdfPageTextChar::unicode_char] function. To return the string representation of this
    /// Unicode literal, use the [PdfPageTextChar::unicode_string] function.
    #[inline]
    pub fn unicode_value(&self) -> u32 {
        self.bindings.FPDFText_GetUnicode(self.text_page_handle, self.index)
    }

    /// Returns Rust's Unicode `char` representation for this character, if available.
    ///
    /// To return the raw Unicode literal value for this character,
    /// use the [PdfPageTextChar::unicode_value] function. To return the string representation of
    /// this `char`, use the [PdfPageTextChar::unicode_string] function.
    #[inline]
    pub fn unicode_char(&self) -> Option<char> {
        char::from_u32(self.unicode_value())
    }

    /// Returns a string containing Rust's Unicode `char` representation for this character,
    /// if available.
    ///
    /// To return the raw Unicode literal value for this character,
    /// use the [PdfPageTextChar::unicode_value] function. To return Rust's Unicode `char`
    /// representation of this Unicode literal, use the [PdfPageTextChar::unicode_char] function.
    #[inline]
    pub fn unicode_string(&self) -> Option<String> {
        self.unicode_char().map(|char| char.to_string())
    }

    /// Returns the effective size of this character when rendered, taking into account both the
    /// font size applied to the character as well as any vertical scale factor applied
    /// to the character's transformation matrix.
    ///
    /// To retrieve only the specified font size, ignoring any vertical scaling, use the
    /// [PdfPageTextChar::unscaled_font_size] function.
    #[inline]
    pub fn scaled_font_size(&self) -> PdfPoints {
        PdfPoints::new(self.unscaled_font_size().value * self.get_vertical_scale())
    }

    /// Returns the font size applied to this character.
    ///
    /// Note that the effective size of the character when rendered may differ from the font size
    /// if a scaling factor has been applied to this character's transformation matrix.
    /// To retrieve the effective font size, taking vertical scaling into account, use the
    /// [PdfPageTextChar::scaled_font_size] function.
    #[inline]
    pub fn unscaled_font_size(&self) -> PdfPoints {
        PdfPoints::new(self.bindings.FPDFText_GetFontSize(self.text_page_handle, self.index) as f32)
    }

    /// Returns the font name and raw font descriptor flags for the font applied to this character.
    fn font(&self) -> (Option<String>, FpdfFontDescriptorFlags) {
        // Retrieving the font name from Pdfium is a two-step operation. First, we call
        // FPDFText_GetFontInfo() with a null buffer; this will retrieve the length of
        // the font name in bytes. If the length is zero, then there is no font name.

        // If the length is non-zero, then we reserve a byte buffer of the given
        // length and call FPDFText_GetFontInfo() again with a pointer to the buffer;
        // this will write the font name into the buffer. Unlike most text handling in
        // Pdfium, font names are returned in UTF-8 format.

        let mut flags = 0;

        let buffer_length =
            self.bindings
                .FPDFText_GetFontInfo(self.text_page_handle, self.index, std::ptr::null_mut(), 0, &mut flags);

        if buffer_length == 0 {
            // The font name is not present.

            return (None, FpdfFontDescriptorFlags::from_bits_truncate(flags as u32));
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings.FPDFText_GetFontInfo(
            self.text_page_handle,
            self.index,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
            &mut flags,
        );

        assert_eq!(result, buffer_length);

        (
            String::from_utf8(buffer)
                // Trim any trailing nulls. All strings returned from Pdfium are generally terminated
                // by one null byte.
                .map(|str| str.trim_end_matches(char::from(0)).to_owned())
                .ok(),
            FpdfFontDescriptorFlags::from_bits_truncate(flags as u32),
        )
    }

    /// Returns the name of the font applied to this character.
    #[inline]
    pub fn font_name(&self) -> String {
        self.font().0.unwrap_or_default()
    }

    /// Returns the weight of the font applied to this character.
    ///
    /// Pdfium may not reliably return the correct value of this property for built-in fonts.
    #[inline]
    pub fn font_weight(&self) -> Option<PdfFontWeight> {
        PdfFontWeight::from_pdfium(self.bindings.FPDFText_GetFontWeight(self.text_page_handle, self.index))
    }

    /// Returns the raw font descriptor bitflags for the font applied to this character.
    #[inline]
    fn font_flags_bits(&self) -> FpdfFontDescriptorFlags {
        self.font().1
    }

    /// Returns `true` if all the glyphs in the font applied to this character have the same width.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_fixed_pitch(&self) -> bool {
        self.font_flags_bits()
            .contains(FpdfFontDescriptorFlags::FIXED_PITCH_BIT_1)
    }

    /// Returns `true` if the glyphs in the font applied to this character have variable widths.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    #[inline]
    pub fn font_is_proportional_pitch(&self) -> bool {
        !self.font_is_fixed_pitch()
    }

    /// Returns `true` if one or more glyphs in the font applied to this character have serifs -
    /// short strokes drawn at an angle on the top or bottom of glyph stems to decorate the glyphs.
    /// For example, Times New Roman is a serif font.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_serif(&self) -> bool {
        self.font_flags_bits().contains(FpdfFontDescriptorFlags::SERIF_BIT_2)
    }

    /// Returns `true` if no glyphs in the font applied to this character have serifs -
    /// short strokes drawn at an angle on the top or bottom of glyph stems to decorate the glyphs.
    /// For example, Helvetica is a sans-serif font.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    #[inline]
    pub fn font_is_sans_serif(&self) -> bool {
        !self.font_is_serif()
    }

    /// Returns `true` if the font applied to this character contains glyphs outside the
    /// Adobe standard Latin character set.
    ///
    /// This classification of non-symbolic and symbolic fonts is peculiar to PDF. A font may
    /// contain additional characters that are used in Latin writing systems but are outside the
    /// Adobe standard Latin character set; PDF considers such a font to be symbolic.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_symbolic(&self) -> bool {
        // This flag bit and the non-symbolic flag bit cannot both be set or both be clear.

        self.font_flags_bits().contains(FpdfFontDescriptorFlags::SYMBOLIC_BIT_3)
    }

    /// Returns `true` if the font applied to this character does not contain glyphs outside the
    /// Adobe standard Latin character set.
    ///
    /// This classification of non-symbolic and symbolic fonts is peculiar to PDF. A font may
    /// contain additional characters that are used in Latin writing systems but are outside the
    /// Adobe standard Latin character set; PDF considers such a font to be symbolic.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_non_symbolic(&self) -> bool {
        // This flag bit and the symbolic flag bit cannot both be set or both be clear.

        self.font_flags_bits()
            .contains(FpdfFontDescriptorFlags::NON_SYMBOLIC_BIT_6)
    }

    /// Returns `true` if the glyphs in the font applied to this character are designed to resemble
    /// cursive handwriting.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_cursive(&self) -> bool {
        self.font_flags_bits().contains(FpdfFontDescriptorFlags::SCRIPT_BIT_4)
    }

    /// Returns `true` if the glyphs in the font applied to this character include dominant
    /// vertical strokes that are slanted.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_italic(&self) -> bool {
        self.font_flags_bits().contains(FpdfFontDescriptorFlags::ITALIC_BIT_7)
    }

    /// Returns `true` if the font applied to this character contains no lowercase letters by design.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_all_caps(&self) -> bool {
        self.font_flags_bits().contains(FpdfFontDescriptorFlags::ALL_CAP_BIT_17)
    }

    /// Returns `true` if the lowercase letters in the font applied to this character have the
    /// same shapes as the corresponding uppercase letters but are sized proportionally
    /// so they have the same size and stroke weight as lowercase glyphs in the same typeface family.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_small_caps(&self) -> bool {
        self.font_flags_bits()
            .contains(FpdfFontDescriptorFlags::SMALL_CAP_BIT_18)
    }

    /// Returns `true` if bold glyphs in the font applied to this character are painted with
    /// extra pixels at very small font sizes.
    ///
    /// Typically when glyphs are painted at small sizes on low-resolution devices, individual strokes
    /// of bold glyphs may appear only one pixel wide. Because this is the minimum width of a pixel
    /// based device, individual strokes of non-bold glyphs may also appear as one pixel wide
    /// and therefore cannot be distinguished from bold glyphs. If this flag is set, individual
    /// strokes of bold glyphs may be thickened at small font sizes.
    ///
    /// Pdfium may not reliably return the correct value of this flag for built-in fonts.
    pub fn font_is_bold_reenforced(&self) -> bool {
        self.font_flags_bits()
            .contains(FpdfFontDescriptorFlags::FORCE_BOLD_BIT_19)
    }

    /// Returns combined font style information from a single FFI call.
    ///
    /// This is more efficient than calling `font_name()`, `font_is_bold_reenforced()`,
    /// and `font_is_italic()` separately, as those each independently invoke the
    /// underlying `FPDFText_GetFontInfo()` FFI function.
    ///
    /// Returns a tuple of (font_name, is_bold, is_italic).
    pub fn font_info(&self) -> (String, bool, bool) {
        let (name, flags) = self.font();
        let name = name.unwrap_or_default();
        let is_bold =
            flags.contains(FpdfFontDescriptorFlags::FORCE_BOLD_BIT_19) || name.to_ascii_lowercase().contains("bold");
        let is_italic = flags.contains(FpdfFontDescriptorFlags::ITALIC_BIT_7);
        (name, is_bold, is_italic)
    }

    /// Returns the page text object that contains this character.
    pub fn text_object(&self) -> Result<PdfPageTextObject<'_>, PdfiumError> {
        let object_handle = self
            .bindings()
            .FPDFText_GetTextObject(self.text_page_handle(), self.index);

        if object_handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfPageTextObject::from_pdfium(
                object_handle,
                PdfPageObjectOwnership::owned_by_page(self.document_handle(), self.page_handle()),
                self.bindings(),
            ))
        }
    }

    /// Returns the text rendering mode for this character.
    pub fn render_mode(&self) -> Result<PdfPageTextRenderMode, PdfiumError> {
        self.text_object().map(|text_object| text_object.render_mode())
    }

    /// Returns the fill color applied to this character.
    pub fn fill_color(&self) -> Result<PdfColor, PdfiumError> {
        let mut r = 0;

        let mut g = 0;

        let mut b = 0;

        let mut a = 0;

        if self.bindings.is_true(self.bindings.FPDFText_GetFillColor(
            self.text_page_handle,
            self.index,
            &mut r,
            &mut g,
            &mut b,
            &mut a,
        )) {
            Ok(PdfColor::new(
                r.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                g.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                b.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                a.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
            ))
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    /// Returns the stroke color applied to this character.
    pub fn stroke_color(&self) -> Result<PdfColor, PdfiumError> {
        let mut r = 0;

        let mut g = 0;

        let mut b = 0;

        let mut a = 0;

        if self.bindings().is_true(self.bindings.FPDFText_GetStrokeColor(
            self.text_page_handle(),
            self.index,
            &mut r,
            &mut g,
            &mut b,
            &mut a,
        )) {
            Ok(PdfColor::new(
                r.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                g.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                b.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
                a.try_into()
                    .map_err(PdfiumError::UnableToConvertPdfiumColorValueToRustu8)?,
            ))
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    /// Returns the rotation angle of this character, expressed in degrees.
    #[inline]
    pub fn angle_degrees(&self) -> Result<f32, PdfiumError> {
        self.angle_radians().map(|result| result.to_degrees())
    }

    /// Returns the rotation angle of this character, expressed in radians.
    #[inline]
    pub fn angle_radians(&self) -> Result<f32, PdfiumError> {
        let result = self.bindings.FPDFText_GetCharAngle(self.text_page_handle, self.index);

        if result == -1.0 {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        } else {
            Ok(result)
        }
    }

    /// Returns a precise bounding box for this character, taking the character's specific
    /// shape into account.
    ///
    /// To return a loose bounding box that contains the entire glyph bounds, use the
    /// [PdfPageTextChar::loose_bounds] function.
    pub fn tight_bounds(&self) -> Result<PdfRect, PdfiumError> {
        let mut left = 0.0;

        let mut bottom = 0.0;

        let mut right = 0.0;

        let mut top = 0.0;

        let result = self.bindings().FPDFText_GetCharBox(
            self.text_page_handle(),
            self.index,
            &mut left,
            &mut right,
            &mut bottom,
            &mut top,
        );

        PdfRect::from_pdfium_as_result(
            result,
            FS_RECTF {
                left: left as f32,
                top: top as f32,
                right: right as f32,
                bottom: bottom as f32,
            },
            self.bindings(),
        )
    }

    /// Returns a loose bounding box for this character, containing the entire glyph bounds.
    ///
    /// To return a tight bounding box that takes this character's specific shape into
    /// account, use the [PdfPageTextChar::tight_bounds] function.
    pub fn loose_bounds(&self) -> Result<PdfRect, PdfiumError> {
        let mut bounds = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };

        let result = self
            .bindings
            .FPDFText_GetLooseCharBox(self.text_page_handle(), self.index, &mut bounds);

        PdfRect::from_pdfium_as_result(result, bounds, self.bindings())
    }

    /// Returns the current raw transformation matrix for this character.
    fn get_matrix_impl(&self) -> Result<PdfMatrix, PdfiumError> {
        let mut matrix = FS_MATRIX {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };

        if self.bindings().is_true(
            self.bindings()
                .FPDFText_GetMatrix(self.text_page_handle(), self.index, &mut matrix),
        ) {
            Ok(PdfMatrix::from_pdfium(matrix))
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    create_transform_getters!(
        "this [PdfPageTextChar]",
        "this [PdfPageTextChar].",
        "this [PdfPageTextChar],"
    );

    /// Returns the origin x and y positions of this character relative to its containing page.
    pub fn origin(&self) -> Result<(PdfPoints, PdfPoints), PdfiumError> {
        let mut x = 0.0;

        let mut y = 0.0;

        if self.bindings().is_true(self.bindings().FPDFText_GetCharOrigin(
            self.text_page_handle(),
            self.index,
            &mut x,
            &mut y,
        )) {
            Ok((PdfPoints::new(x as f32), PdfPoints::new(y as f32)))
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns the origin x position of this character relative to its containing page.
    #[inline]
    pub fn origin_x(&self) -> Result<PdfPoints, PdfiumError> {
        self.origin().map(|result| result.0)
    }

    /// Returns the origin y position of this character relative to its containing page.
    #[inline]
    pub fn origin_y(&self) -> Result<PdfPoints, PdfiumError> {
        self.origin().map(|result| result.1)
    }

    /// Returns `true` if the glyph shape of this character descends below the font baseline.
    #[inline]
    pub fn has_descender(&self) -> bool {
        self.tight_bounds().map(|bounds| bounds.bottom().value).unwrap_or(0.0)
            < self.loose_bounds().map(|bounds| bounds.bottom().value).unwrap_or(0.0)
    }

    /// Returns `true` if this character was generated by Pdfium. This can be the case for
    /// certain spacing, breaking, and justification-related characters.
    #[inline]
    pub fn is_generated(&self) -> Result<bool, PdfiumError> {
        match self
            .bindings()
            .FPDFText_IsGenerated(self.text_page_handle(), self.index)
        {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown)),
        }
    }

    /// Returns `true` if this character is recognized as a hyphen by Pdfium.
    #[inline]
    pub fn is_hyphen(&self) -> Result<bool, PdfiumError> {
        match self.bindings().FPDFText_IsHyphen(self.text_page_handle(), self.index) {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown)),
        }
    }

    /// Returns `true` if this character has an invalid unicode mapping in the PDF font.
    ///
    /// This indicates the font's ToUnicode CMap is broken or missing for this glyph,
    /// meaning the extracted text for this character is unreliable (tofu/garbage).
    #[inline]
    pub fn has_unicode_map_error(&self) -> Result<bool, PdfiumError> {
        match self
            .bindings()
            .FPDFText_HasUnicodeMapError(self.text_page_handle(), self.index)
        {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown)),
        }
    }
}
