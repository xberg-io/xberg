pub(crate) mod internal {

    use crate::bindgen::{
        FPDF_ANNOT_FLAG_HIDDEN, FPDF_ANNOT_FLAG_INVISIBLE, FPDF_ANNOT_FLAG_LOCKED, FPDF_ANNOT_FLAG_NONE,
        FPDF_ANNOT_FLAG_NOROTATE, FPDF_ANNOT_FLAG_NOVIEW, FPDF_ANNOT_FLAG_NOZOOM, FPDF_ANNOT_FLAG_PRINT,
        FPDF_ANNOT_FLAG_READONLY, FPDF_ANNOT_FLAG_TOGGLENOVIEW, FPDF_ANNOTATION, FPDF_OBJECT_STRING, FPDF_PAGEOBJECT,
        FPDF_WCHAR, FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_Color,
        FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_InteriorColor, FS_RECTF,
    };
    use crate::bindings::PdfiumLibraryBindings;
    use crate::error::{PdfiumError, PdfiumInternalError};
    use crate::pdf::color::PdfColor;
    use crate::pdf::document::page::annotation::attachment_points::PdfPageAnnotationAttachmentPoints;
    use crate::pdf::document::page::annotation::objects::PdfPageAnnotationObjects;
    use crate::pdf::document::page::annotation::{PdfPageAnnotationCommon, PdfPageAnnotationType};
    use crate::pdf::document::page::object::ownership::PdfPageObjectOwnership;
    use crate::pdf::points::PdfPoints;
    use crate::pdf::rect::PdfRect;
    use crate::utils::dates::timestamp_to_pdf_string;
    use crate::utils::mem::create_byte_buffer;
    use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
    use bitflags::bitflags;
    use std::os::raw::{c_int, c_uint};

    bitflags! {
        /// Flags specifying various characteristics of one annotation. For more details,
        /// refer to Section 8.4.2 of The PDF Reference (Sixth Edition, PDF Format 1.7),
        /// starting on page 608.
        pub struct PdfAnnotationFlags: u32 {
            /// No flags are set for this annotation.
            const None = FPDF_ANNOT_FLAG_NONE;

            /// If set, do not display the annotation if it does not belong to one of the
            /// standard annotation types and no annotation handler is available. If clear,
            /// display such an unknown annotation using an appearance stream specified by
            /// its appearance dictionary, if any.
            const Invisible = FPDF_ANNOT_FLAG_INVISIBLE;

            /// If set, do not display or print the annotation or allow it to interact
            /// with the user, regardless of its annotation type or whether an annotation
            /// handler is available.
            ///
            /// In cases where screen space is limited, the ability to hide and show annotations
            /// selectively can be used in combination with appearance streams to display
            /// auxiliary pop-up information, similar in function to online help systems.
            ///
            /// This flag was added in PDF version 1.2.
            const Hidden = FPDF_ANNOT_FLAG_HIDDEN;

            /// If set, print the annotation when the page is printed. If clear, never
            /// print the annotation, regardless of whether it is displayed on the screen.
            ///
            /// This can be useful, for example, for annotations representing interactive
            /// push buttons, which would serve no meaningful purpose on the printed page.
            ///
            /// This flag was added in PDF version 1.2.
            const Print = FPDF_ANNOT_FLAG_PRINT;

            /// If set, do not scale the annotation's appearance to match the magnification
            /// of the page. The location of the annotation on the page (defined by
            /// the upper-left corner of its annotation rectangle) remains fixed,
            /// regardless of the page magnification.
            ///
            /// This flag was added in PDF version 1.3.
            const NoZoom = FPDF_ANNOT_FLAG_NOZOOM;

            /// If set, do not rotate the annotation's appearance to match the rotation
            /// of the page. The upper-left corner of the annotation rectangle remains in
            /// a fixed location on the page, regardless of the page rotation.
            ///
            /// This flag was added in PDF version 1.3.
            const NoRotate = FPDF_ANNOT_FLAG_NOROTATE;

            /// If set, do not display the annotation on the screen or allow it to
            /// interact with the user. The annotation may be printed (depending on the
            /// setting of the Print flag) but should be considered hidden for purposes
            /// of on-screen display and user interaction.
            ///
            /// This flag was added in PDF version 1.3.
            const NoView = FPDF_ANNOT_FLAG_NOVIEW;

            /// If set, do not allow the annotation to interact with the user. The
            /// annotation may be displayed or printed (depending on the settings of the
            /// `NoView` and `Print` flags) but should not respond to mouse clicks or
            /// change its appearance in response to mouse motions.
            ///
            /// This flag is ignored for widget annotations; its function is subsumed by
            /// the `ReadOnly` flag of the associated form field.
            ///
            /// THis flag was added in PDF version 1.3.
            const ReadOnly = FPDF_ANNOT_FLAG_READONLY;

            /// If set, do not allow the annotation to be deleted or its properties
            /// (including position and size) to be modified by the user. However, this flag
            /// does not restrict changes to the annotation's contents, such as the value
            /// of a form field.
            ///
            /// THis flag was added in PDF version 1.4.
            const Locked = FPDF_ANNOT_FLAG_LOCKED;

            /// If set, invert the interpretation of the `NoView` flag for certain
            /// events. A typical use is to have an annotation that appears only when a
            /// mouse cursor is held over it.
            ///
            /// This flag was added in PDF version 1.5.
            const ToggleNoView = FPDF_ANNOT_FLAG_TOGGLENOVIEW;

            /// If set, do not allow the contents of the annotation to be modified by
            /// the user. This flag does not restrict deletion of the annotation or
            /// changes to other annotation properties, such as position and size.
            ///
            /// This flag was added in PDF version 1.7.
            const LockedContents = 1 << 10;
        }
    }

    /// Internal crate-specific functionality common to all [PdfPageAnnotation] objects.
    ///
    /// This trait carries only the handful of primitives every concrete annotation type must
    /// supply itself; the default-implemented functionality built on top of them lives in
    /// [PdfPageAnnotationPrivateDefaults] and [PdfPageAnnotationPrivateStyle], both of which are
    /// blanket-implemented for every type that implements this trait. ~keep
    pub(crate) trait PdfPageAnnotationPrivate<'a>: PdfPageAnnotationCommon {
        /// Returns the internal `FPDF_ANNOTATION` handle for this [PdfPageAnnotation].
        fn handle(&self) -> FPDF_ANNOTATION;

        /// Returns the [PdfiumLibraryBindings] used by this [PdfPageAnnotation].
        fn bindings(&self) -> &dyn PdfiumLibraryBindings;

        /// Returns the ownership hierarchy for this [PdfPageAnnotation].
        fn ownership(&self) -> &PdfPageObjectOwnership;

        /// Internal implementation of [PdfPageAnnotationCommon::objects()].
        fn objects_impl(&self) -> &PdfPageAnnotationObjects<'_>;

        /// Internal implementation of [PdfPageAnnotationCommon::attachment_points()].
        fn attachment_points_impl(&self) -> &PdfPageAnnotationAttachmentPoints<'_>;
    }

    /// Default-implemented crate-specific functionality for reading and writing the string-valued
    /// and geometric properties common to all [PdfPageAnnotation] objects. Blanket-implemented for
    /// every type that implements [PdfPageAnnotationPrivate]. ~keep
    pub(crate) trait PdfPageAnnotationPrivateDefaults<'a>: PdfPageAnnotationPrivate<'a> {
        /// Returns the [PdfPageAnnotationType] of this [PdfPageAnnotation].
        fn get_annotation_type(&self) -> PdfPageAnnotationType {
            PdfPageAnnotationType::from_pdfium(self.bindings().FPDFAnnot_GetSubtype(self.handle()))
                .unwrap_or(PdfPageAnnotationType::Unknown)
        }

        /// Returns the string value associated with the given key in the annotation dictionary
        /// of this [PdfPageAnnotation], if any.
        fn get_string_value(&self, key: &str) -> Option<String> {
            if !self
                .bindings()
                .is_true(self.bindings().FPDFAnnot_HasKey(self.handle(), key))
            {
                return None;
            }

            if self.bindings().FPDFAnnot_GetValueType(self.handle(), key) as u32 != FPDF_OBJECT_STRING {
                return None;
            }

            let buffer_length = self
                .bindings()
                .FPDFAnnot_GetStringValue(self.handle(), key, std::ptr::null_mut(), 0);

            if buffer_length <= 2 {
                return None;
            }

            let mut buffer = create_byte_buffer(buffer_length as usize);

            let result = self.bindings().FPDFAnnot_GetStringValue(
                self.handle(),
                key,
                buffer.as_mut_ptr() as *mut FPDF_WCHAR,
                buffer_length,
            );

            assert_eq!(result, buffer_length);

            Some(get_string_from_pdfium_utf16le_bytes(buffer).unwrap_or_default())
        }

        /// Sets the string value associated with the given key in the annotation dictionary
        /// of this [PdfPageAnnotation].
        fn set_string_value(&mut self, key: &str, value: &str) -> Result<(), PdfiumError> {
            #[allow(clippy::collapsible_if)]
            if key != "M" {
                self.set_string_value("M", &timestamp_to_pdf_string(jiff::Timestamp::now()))?;
            }

            if self
                .bindings()
                .is_true(self.bindings().FPDFAnnot_SetStringValue_str(self.handle(), key, value))
            {
                Ok(())
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        }

        /// Internal implementation of [PdfPageAnnotationCommon::name()].
        #[inline]
        fn name_impl(&self) -> Option<String> {
            self.get_string_value("NM")
        }

        /// Internal implementation of [PdfPageAnnotationCommon::bounds()].
        #[inline]
        fn bounds_impl(&self) -> Result<PdfRect, PdfiumError> {
            let mut rect = FS_RECTF {
                left: 0_f32,
                bottom: 0_f32,
                right: 0_f32,
                top: 0_f32,
            };

            let result = self.bindings().FPDFAnnot_GetRect(self.handle(), &mut rect);

            PdfRect::from_pdfium_as_result(result, rect, self.bindings())
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_bounds()].
        #[inline]
        fn set_bounds_impl(&mut self, bounds: PdfRect) -> Result<(), PdfiumError> {
            if self
                .bindings()
                .is_true(self.bindings().FPDFAnnot_SetRect(self.handle(), &bounds.as_pdfium()))
            {
                self.set_string_value("M", &timestamp_to_pdf_string(jiff::Timestamp::now()))
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_position()].
        fn set_position_impl(&mut self, x: PdfPoints, y: PdfPoints) -> Result<(), PdfiumError> {
            let bounds = self.bounds().unwrap_or(PdfRect::new_from_values(0.0, 0.0, 1.0, 1.0));

            let width = bounds.width();

            let height = bounds.height();

            self.set_bounds(PdfRect::new(y, x, y + height, x + width))
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_width()].
        fn set_width_impl(&mut self, width: PdfPoints) -> Result<(), PdfiumError> {
            let bounds = self.bounds().unwrap_or(PdfRect::new_from_values(0.0, 0.0, 1.0, 1.0));

            let height = bounds.height();

            self.set_bounds(PdfRect::new(
                bounds.bottom(),
                bounds.left(),
                bounds.bottom() + height,
                bounds.left() + width,
            ))
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_height()].
        fn set_height_impl(&mut self, height: PdfPoints) -> Result<(), PdfiumError> {
            let bounds = self.bounds().unwrap_or(PdfRect::new_from_values(0.0, 0.0, 1.0, 1.0));

            let width = bounds.width();

            self.set_bounds(PdfRect::new(
                bounds.bottom(),
                bounds.left(),
                bounds.bottom() + height,
                bounds.left() + width,
            ))
        }

        /// Internal implementation of [PdfPageAnnotationCommon::contents()].
        #[inline]
        fn contents_impl(&self) -> Option<String> {
            self.get_string_value("Contents")
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_contents()].
        #[inline]
        fn set_contents_impl(&mut self, contents: &str) -> Result<(), PdfiumError> {
            self.set_string_value("Contents", contents)
        }

        /// Internal implementation of [PdfPageAnnotationCommon::creator()].
        #[inline]
        fn creator_impl(&self) -> Option<String> {
            self.get_string_value("T")
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_creator()].
        #[inline]
        fn set_creator_impl(&mut self, creator: &str) -> Result<(), PdfiumError> {
            self.set_string_value("T", creator)
        }

        /// Internal implementation of [PdfPageAnnotationCommon::creation_date()].
        #[inline]
        fn creation_date_impl(&self) -> Option<String> {
            self.get_string_value("CreationDate")
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_creation_date()].
        #[inline]
        fn set_creation_date_impl(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError> {
            self.set_string_value("CreationDate", &timestamp_to_pdf_string(date))
        }

        /// Internal implementation of [PdfPageAnnotationCommon::modification_date()].
        #[inline]
        fn modification_date_impl(&self) -> Option<String> {
            self.get_string_value("M")
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_modification_date()].
        #[inline]
        fn set_modification_date_impl(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError> {
            self.set_string_value("M", &timestamp_to_pdf_string(date))
        }
    }

    impl<'a, T> PdfPageAnnotationPrivateDefaults<'a> for T where T: PdfPageAnnotationPrivate<'a> {}

    /// Default-implemented crate-specific functionality for reading and writing the markup,
    /// color, and flag-based properties common to all [PdfPageAnnotation] objects.
    /// Blanket-implemented for every type that implements [PdfPageAnnotationPrivate]. ~keep
    pub(crate) trait PdfPageAnnotationPrivateStyle<'a>: PdfPageAnnotationPrivateDefaults<'a> {
        /// Internal implementation of [PdfPageAnnotationCommon::is_markup_annotation()].
        #[inline]
        fn is_markup_annotation_impl(&self) -> bool {
            self.has_attachment_points_impl() && self.get_annotation_type() != PdfPageAnnotationType::Link
        }

        /// Internal implementation of [PdfPageAnnotationCommon::has_attachment_points()].
        #[inline]
        fn has_attachment_points_impl(&self) -> bool {
            self.bindings()
                .is_true(self.bindings().FPDFAnnot_HasAttachmentPoints(self.handle()))
        }

        /// Internal implementation of [PdfPageAnnotationCommon::fill_color()].
        #[inline]
        fn fill_color_impl(&self) -> Result<PdfColor, PdfiumError> {
            let mut r: c_uint = 0;

            let mut g: c_uint = 0;

            let mut b: c_uint = 0;

            let mut a: c_uint = 0;

            if self.bindings().is_true(self.bindings().FPDFAnnot_GetColor(
                self.handle(),
                FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_InteriorColor,
                &mut r,
                &mut g,
                &mut b,
                &mut a,
            )) {
                Ok(PdfColor::new(r as u8, g as u8, b as u8, a as u8))
            } else {
                if self.bindings().is_true(self.bindings().FPDFPageObj_GetFillColor(
                    self.handle() as FPDF_PAGEOBJECT,
                    &mut r,
                    &mut g,
                    &mut b,
                    &mut a,
                )) {
                    Ok(PdfColor::new(r as u8, g as u8, b as u8, a as u8))
                } else {
                    Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                }
            }
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_fill_color()].
        #[inline]
        fn set_fill_color_impl(&mut self, fill_color: PdfColor) -> Result<(), PdfiumError> {
            if self.bindings().is_true(self.bindings().FPDFAnnot_SetColor(
                self.handle(),
                FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_InteriorColor,
                fill_color.red() as c_uint,
                fill_color.green() as c_uint,
                fill_color.blue() as c_uint,
                fill_color.alpha() as c_uint,
            )) {
                Ok(())
            } else {
                if self.bindings().is_true(self.bindings().FPDFPageObj_SetFillColor(
                    self.handle() as FPDF_PAGEOBJECT,
                    fill_color.red() as c_uint,
                    fill_color.green() as c_uint,
                    fill_color.blue() as c_uint,
                    fill_color.alpha() as c_uint,
                )) {
                    Ok(())
                } else {
                    Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                }
            }
        }

        /// Internal implementation of [PdfPageAnnotationCommon::stroke_color()].
        #[inline]
        fn stroke_color_impl(&self) -> Result<PdfColor, PdfiumError> {
            let mut r: c_uint = 0;

            let mut g: c_uint = 0;

            let mut b: c_uint = 0;

            let mut a: c_uint = 0;

            if self.bindings().is_true(self.bindings().FPDFAnnot_GetColor(
                self.handle(),
                FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_Color,
                &mut r,
                &mut g,
                &mut b,
                &mut a,
            )) {
                Ok(PdfColor::new(r as u8, g as u8, b as u8, a as u8))
            } else {
                if self.bindings().is_true(self.bindings().FPDFPageObj_GetStrokeColor(
                    self.handle() as FPDF_PAGEOBJECT,
                    &mut r,
                    &mut g,
                    &mut b,
                    &mut a,
                )) {
                    Ok(PdfColor::new(r as u8, g as u8, b as u8, a as u8))
                } else {
                    Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                }
            }
        }

        /// Internal implementation of [PdfPageAnnotationCommon::set_stroke_color()].
        #[inline]
        fn set_stroke_color_impl(&mut self, stroke_color: PdfColor) -> Result<(), PdfiumError> {
            if self.bindings().is_true(self.bindings().FPDFAnnot_SetColor(
                self.handle(),
                FPDFANNOT_COLORTYPE_FPDFANNOT_COLORTYPE_Color,
                stroke_color.red() as c_uint,
                stroke_color.green() as c_uint,
                stroke_color.blue() as c_uint,
                stroke_color.alpha() as c_uint,
            )) {
                Ok(())
            } else {
                if self.bindings().is_true(self.bindings().FPDFPageObj_SetStrokeColor(
                    self.handle() as FPDF_PAGEOBJECT,
                    stroke_color.red() as c_uint,
                    stroke_color.green() as c_uint,
                    stroke_color.blue() as c_uint,
                    stroke_color.alpha() as c_uint,
                )) {
                    Ok(())
                } else {
                    Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                }
            }
        }

        /// Returns all the flags currently set on this annotation.
        #[inline]
        fn get_flags_impl(&self) -> PdfAnnotationFlags {
            PdfAnnotationFlags::from_bits_truncate(self.bindings().FPDFAnnot_GetFlags(self.handle()) as u32)
        }

        /// Sets all the flags on this annotation.
        #[inline]
        fn set_flags_impl(&mut self, flags: PdfAnnotationFlags) -> bool {
            self.bindings()
                .is_true(self.bindings().FPDFAnnot_SetFlags(self.handle(), flags.bits() as c_int))
        }

        /// Sets or clears a single flag on this annotation.
        fn update_one_flag_impl(&mut self, flag: PdfAnnotationFlags, value: bool) -> Result<(), PdfiumError> {
            let mut flags = self.get_flags_impl();

            flags.set(flag, value);

            if self.set_flags_impl(flags) {
                Ok(())
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(
                    crate::error::PdfiumInternalError::Unknown,
                ))
            }
        }
    }

    impl<'a, T> PdfPageAnnotationPrivateStyle<'a> for T where T: PdfPageAnnotationPrivateDefaults<'a> {}
}

#[cfg(test)]
mod tests {}
