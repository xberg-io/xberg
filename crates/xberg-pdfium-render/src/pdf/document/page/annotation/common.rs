//! Defines the [PdfPageAnnotationCommon] trait, exposing functionality common to all
//! [PdfPageAnnotation] objects, regardless of their [PdfPageAnnotationType].

use crate::error::PdfiumError;
use crate::pdf::color::PdfColor;
use crate::pdf::document::page::annotation::attachment_points::PdfPageAnnotationAttachmentPoints;
use crate::pdf::document::page::annotation::objects::PdfPageAnnotationObjects;
use crate::pdf::document::page::annotation::private::internal::{PdfAnnotationFlags, PdfPageAnnotationPrivateStyle};
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;
#[cfg(doc)]
use crate::pdf::document::page::annotation::{PdfPageAnnotation, PdfPageAnnotationType};

pub trait PdfPageAnnotationCommon {
    /// Returns the name of this [PdfPageAnnotation], if any. This is a text string uniquely identifying
    /// this annotation among all the annotations attached to the containing page.
    fn name(&self) -> Option<String>;

    /// Returns `true` if this [PdfPageAnnotation] supports applying text markup to the page
    /// by setting the annotation contents using the [PdfPageAnnotationCommon::set_contents()]
    /// function.
    fn is_markup_annotation(&self) -> bool;

    /// Returns `true` if this [PdfPageAnnotation] supports setting attachment points that
    /// visually associate it with a `PdfPageObject`.
    fn has_attachment_points(&self) -> bool;

    /// Returns the bounding box of this [PdfPageAnnotation].
    fn bounds(&self) -> Result<PdfRect, PdfiumError>;

    /// Sets the bounding box of this [PdfPageAnnotation].
    ///
    /// This sets the position, the width, and the height of the annotation in a single operation.
    /// To set these properties separately, use the [PdfPageAnnotationCommon::set_position()],
    /// [PdfPageAnnotationCommon::set_width()], and [PdfPageAnnotationCommon::set_height()] functions.
    fn set_bounds(&mut self, bounds: PdfRect) -> Result<(), PdfiumError>;

    /// Sets the bottom right corner of this [PdfPageAnnotation] to the given values.
    ///
    /// To set the position, the width, and the height of the annotation in a single operation,
    /// use the [PdfPageAnnotationCommon::set_bounds()] function.
    fn set_position(&mut self, x: PdfPoints, y: PdfPoints) -> Result<(), PdfiumError>;

    /// Sets the width of this [PdfPageAnnotation] to the given value.
    ///
    /// To set the position, the width, and the height of the annotation in a single operation,
    /// use the [PdfPageAnnotationCommon::set_bounds()] function.
    fn set_width(&mut self, width: PdfPoints) -> Result<(), PdfiumError>;

    /// Sets the height of this [PdfPageAnnotation] to the given value.
    ///
    /// To set the position, the width, and the height of the annotation in a single operation,
    /// use the [PdfPageAnnotationCommon::set_bounds()] function.
    fn set_height(&mut self, width: PdfPoints) -> Result<(), PdfiumError>;

    /// Returns the text to be displayed for this [PdfPageAnnotation], or, if this type of annotation
    /// does not display text, an alternate description of the annotation's contents in human-readable
    /// form. In either case this text is useful when extracting the document's contents in support
    /// of accessibility to users with disabilities or for other purposes.
    fn contents(&self) -> Option<String>;

    /// Sets the text to be displayed for this [PdfPageAnnotation], or, if this type of annotation
    /// does not display text, an alternate description of the annotation's contents in human-readable
    /// form for providing accessibility to users with disabilities or for other purposes.
    fn set_contents(&mut self, contents: &str) -> Result<(), PdfiumError>;

    /// Returns the name of the creator of this [PdfPageAnnotation], if any.
    fn creator(&self) -> Option<String>;

    /// Sets the name of the creator of this [PdfPageAnnotation].
    fn set_creator(&mut self, creator: &str) -> Result<(), PdfiumError>;

    /// Returns the date and time when this [PdfPageAnnotation] was originally created, if any.
    fn creation_date(&self) -> Option<String>;

    /// Sets the date and time when this [PdfPageAnnotation] was originally created.
    fn set_creation_date(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError>;

    /// Returns the date and time when this [PdfPageAnnotation] was last modified, if any.
    fn modification_date(&self) -> Option<String>;

    /// Sets the date and time when this [PdfPageAnnotation] was last modified.
    fn set_modification_date(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError>;

    /// Returns the color of any filled paths in this [PdfPageAnnotation].
    fn fill_color(&self) -> Result<PdfColor, PdfiumError>;

    /// Sets the color of any filled paths in this [PdfPageAnnotation].
    fn set_fill_color(&mut self, fill_color: PdfColor) -> Result<(), PdfiumError>;

    /// Returns the color of any stroked paths in this [PdfPageAnnotation].
    fn stroke_color(&self) -> Result<PdfColor, PdfiumError>;

    /// Sets the color of any stroked paths in this [PdfPageAnnotation].
    fn set_stroke_color(&mut self, stroke_color: PdfColor) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] should not be displayed to the user
    /// if it does not belong to one of the standard annotation types and no annotation
    /// handler is available that supports it.
    fn is_invisible_if_unsupported(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] should be displayed to the user
    /// if it does not belong to one of the standard annotation types and no annotation
    /// handler is available that supports it.
    fn set_is_invisible_if_unsupported(&mut self, is_invisible: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] should not be displayed or printed,
    /// nor allowed to interact with the user, regardless of its annotation type or whether
    /// an annotation handler is available that supports it.
    ///
    /// This flag was added in PDF version 1.2.
    fn is_hidden(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] should be displayed, printed,
    /// and allowed to interact with the user, regardless of its annotation type or whether
    /// an annotation handler is available that supports it.
    ///
    /// This flag was added in PDF version 1.2.
    fn set_is_hidden(&mut self, is_hidden: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] should be printed when the
    /// page is printed.
    ///
    /// This can be useful, for example, for annotations representing interactive
    /// push buttons, which would serve no meaningful purpose on the printed page.
    ///
    /// This flag was added in PDF version 1.2.
    fn is_printed(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] should be printed when the
    /// page is printed.
    ///
    /// This can be useful, for example, for annotations representing interactive
    /// push buttons, which would serve no meaningful purpose on the printed page.
    ///
    /// This flag was added in PDF version 1.2.
    fn set_is_printed(&mut self, is_printed: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if the appearance of this [PdfPageAnnotation] should scale to match
    /// the magnification of the page. If `false`, the location of the annotation on the
    /// page (defined by the upper-left corner of its annotation rectangle) will remain fixed,
    /// regardless of the page magnification.
    ///
    /// This flag was added in PDF version 1.3.
    fn is_zoomable(&self) -> bool;

    /// Controls whether or not the appearance of this [PdfPageAnnotation] should scale to
    /// match the magnification of the page.
    ///
    /// This flag was added in PDF version 1.3.
    fn set_is_zoomable(&mut self, is_zoomable: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if the appearance of this [PdfPageAnnotation] should rotate to match
    /// the rotation of the page. If `false`, the upper-left corner of the annotation rectangle
    /// will remain in a fixed location on the page, regardless of the page rotation.
    ///
    /// This flag was added in PDF version 1.3.
    fn is_rotatable(&self) -> bool;

    /// Controls whether or not the appearance of this [PdfPageAnnotation] should rotate
    /// to match the rotation of the page.
    ///
    /// This flag was added in PDF version 1.3.
    fn set_is_rotatable(&mut self, is_rotatable: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] should not be displayed to, or allowed to
    /// interact with, the user. The annotation may be printed (depending on the setting of
    /// the [PdfPageAnnotationCommon::is_printed()] flag) but should be considered
    /// hidden for purposes of on-screen display and user interaction.
    ///
    /// This flag was added in PDF version 1.3.
    fn is_printable_but_not_viewable(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] should be displayed to, and allowed
    /// to interact with, the user. Whether or not the annotation should be printed is
    /// controlled separately by the [PdfPageAnnotationCommon::set_is_printed()] function.
    ///
    /// This flag was added in PDF version 1.3.
    fn set_is_printable_but_not_viewable(&mut self, is_printable_but_not_viewable: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] should not be allowed to interact
    /// with the user. The annotation may be displayed or printed (depending on the settings
    /// of the [PdfPageAnnotationCommon::is_printed()] and [PdfPageAnnotationCommon::is_printable_but_not_viewable()]
    /// flags) but should not respond to mouse clicks or change its appearance
    /// in response to mouse motions.
    ///
    /// This flag is ignored for widget annotations; its function is subsumed by
    /// the [PdfFormFieldCommon::is_read_only()] flag of the associated form field.
    ///
    /// THis flag was added in PDF version 1.3.
    fn is_read_only(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] should be allowed to interact
    /// with the user.
    ///
    /// This flag is ignored for widget annotations; its function is subsumed by
    /// the [PdfFormFieldCommon::is_read_only()] flag of the associated form field.
    ///
    /// THis flag was added in PDF version 1.3.
    fn set_is_read_only(&mut self, is_read_only: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if this [PdfPageAnnotation] is locked. Locked annotations cannot be
    /// deleted, repositioned, or resized by the user. The content of a locked annotation
    /// may still be editable, depending on the setting of the [PdfPageAnnotationCommon::is_editable()]
    /// flag.
    fn is_locked(&self) -> bool;

    /// Controls whether or not this [PdfPageAnnotation] is locked. Locked annotations cannot be
    /// deleted, repositioned, or resized by the user. The content of a locked annotation
    /// may still be editable, depending on the setting of the [PdfPageAnnotationCommon::set_is_editable()]
    /// function.
    fn set_is_locked(&mut self, is_locked: bool) -> Result<(), PdfiumError>;

    /// Returns `true` if the contents of this [PdfPageAnnotation] can be edited by the user.
    /// This setting does not control whether or not the annotation can be deleted,
    /// repositioned, or resized; those properties are controlled by the
    /// [PdfPageAnnotationCommon::is_locked()] flag.
    fn is_editable(&self) -> bool;

    /// Controls whether or not the contents of this [PdfPageAnnotation] can be edited by the user.
    /// This setting does not control whether or not the annotation can be deleted,
    /// repositioned, or resized; those properties are controlled by the
    /// [PdfPageAnnotationCommon::set_is_locked()] function.
    fn set_is_editable(&mut self, is_editable: bool) -> Result<(), PdfiumError>;

    /// Returns an immutable collection of all the page objects in this [PdfPageAnnotation].
    ///
    /// Page objects can be retrieved from any type of [PdfPageAnnotation], but Pdfium currently
    /// only permits adding new page objects to, or removing existing page objects from, annotations
    /// of types [PdfPageAnnotationType::Ink] and [PdfPageAnnotationType::Stamp]. All other annotation
    /// types are read-only.
    ///
    /// To gain access to the mutable collection of page objects inside an ink or stamp annotation,
    /// you must first unwrap the annotation, like so:
    /// ```
    /// annotation.as_stamp_annotation_mut().unwrap().objects_mut();
    /// ```
    fn objects(&self) -> &PdfPageAnnotationObjects<'_>;

    /// Returns an immutable collection of the attachment points that visually associate
    /// this [PdfPageAnnotation] with one or more `PdfPageObject` objects on this `PdfPage`.
    ///
    /// This collection is provided for all annotation types, but it will always be empty
    /// if the annotation does not support attachment points. Pdfium supports attachment points
    /// for all markup annotations and the Link annotation, but not for any other annotation type.
    /// The [PdfPageAnnotationCommon::has_attachment_points()] function will return `true`
    /// if the annotation supports attachment points.
    ///
    /// To gain access to the mutable collection of attachment points inside a supported
    /// annotation, you must first unwrap the annotation, like so:
    /// ```
    /// annotation.as_link_annotation_mut().unwrap().attachment_points_mut();
    /// ```
    fn attachment_points(&self) -> &PdfPageAnnotationAttachmentPoints<'_>;
}

impl<'a, T> PdfPageAnnotationCommon for T
where
    T: PdfPageAnnotationPrivateStyle<'a>,
{
    #[inline]
    fn name(&self) -> Option<String> {
        self.name_impl()
    }

    #[inline]
    fn is_markup_annotation(&self) -> bool {
        self.is_markup_annotation_impl()
    }

    #[inline]
    fn has_attachment_points(&self) -> bool {
        self.has_attachment_points_impl()
    }

    #[inline]
    fn bounds(&self) -> Result<PdfRect, PdfiumError> {
        self.bounds_impl()
    }

    #[inline]
    fn set_bounds(&mut self, bounds: PdfRect) -> Result<(), PdfiumError> {
        self.set_bounds_impl(bounds)
    }

    #[inline]
    fn set_position(&mut self, x: PdfPoints, y: PdfPoints) -> Result<(), PdfiumError> {
        self.set_position_impl(x, y)
    }

    #[inline]
    fn set_width(&mut self, width: PdfPoints) -> Result<(), PdfiumError> {
        self.set_width_impl(width)
    }

    #[inline]
    fn set_height(&mut self, height: PdfPoints) -> Result<(), PdfiumError> {
        self.set_height_impl(height)
    }

    #[inline]
    fn contents(&self) -> Option<String> {
        self.contents_impl()
    }

    #[inline]
    fn set_contents(&mut self, contents: &str) -> Result<(), PdfiumError> {
        self.set_contents_impl(contents)
    }

    #[inline]
    fn creator(&self) -> Option<String> {
        self.creator_impl()
    }

    #[inline]
    fn set_creator(&mut self, creator: &str) -> Result<(), PdfiumError> {
        self.set_creator_impl(creator)
    }

    #[inline]
    fn creation_date(&self) -> Option<String> {
        self.creation_date_impl()
    }

    #[inline]
    fn set_creation_date(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError> {
        self.set_creation_date_impl(date)
    }

    #[inline]
    fn modification_date(&self) -> Option<String> {
        self.modification_date_impl()
    }

    #[inline]
    fn set_modification_date(&mut self, date: jiff::Timestamp) -> Result<(), PdfiumError> {
        self.set_modification_date_impl(date)
    }

    #[inline]
    fn fill_color(&self) -> Result<PdfColor, PdfiumError> {
        self.fill_color_impl()
    }

    #[inline]
    fn set_fill_color(&mut self, fill_color: PdfColor) -> Result<(), PdfiumError> {
        self.set_fill_color_impl(fill_color)
    }

    #[inline]
    fn stroke_color(&self) -> Result<PdfColor, PdfiumError> {
        self.stroke_color_impl()
    }

    #[inline]
    fn set_stroke_color(&mut self, stroke_color: PdfColor) -> Result<(), PdfiumError> {
        self.set_stroke_color_impl(stroke_color)
    }

    #[inline]
    fn objects(&self) -> &PdfPageAnnotationObjects<'_> {
        self.objects_impl()
    }

    #[inline]
    fn attachment_points(&self) -> &PdfPageAnnotationAttachmentPoints<'_> {
        self.attachment_points_impl()
    }

    #[inline]
    fn is_invisible_if_unsupported(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::Invisible)
    }

    #[inline]
    fn set_is_invisible_if_unsupported(&mut self, is_invisible: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::Invisible, is_invisible)
    }

    #[inline]
    fn is_hidden(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::Hidden)
    }

    #[inline]
    fn set_is_hidden(&mut self, is_hidden: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::Hidden, is_hidden)
    }

    #[inline]
    fn is_printed(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::Print)
    }

    #[inline]
    fn set_is_printed(&mut self, is_printed: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::Print, is_printed)
    }

    #[inline]
    fn is_zoomable(&self) -> bool {
        !self.get_flags_impl().contains(PdfAnnotationFlags::NoZoom)
    }

    #[inline]
    fn set_is_zoomable(&mut self, is_zoomable: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::NoZoom, !is_zoomable)
    }

    #[inline]
    fn is_rotatable(&self) -> bool {
        !self.get_flags_impl().contains(PdfAnnotationFlags::NoRotate)
    }

    #[inline]
    fn set_is_rotatable(&mut self, is_rotatable: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::NoRotate, !is_rotatable)
    }

    #[inline]
    fn is_printable_but_not_viewable(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::NoView)
    }

    #[inline]
    fn set_is_printable_but_not_viewable(&mut self, is_printable_but_not_viewable: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::NoView, is_printable_but_not_viewable)
    }

    #[inline]
    fn is_read_only(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::ReadOnly)
    }

    #[inline]
    fn set_is_read_only(&mut self, is_read_only: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::ReadOnly, is_read_only)
    }

    #[inline]
    fn is_locked(&self) -> bool {
        self.get_flags_impl().contains(PdfAnnotationFlags::Locked)
    }

    #[inline]
    fn set_is_locked(&mut self, is_locked: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::Locked, is_locked)
    }

    #[inline]
    fn is_editable(&self) -> bool {
        !self.get_flags_impl().contains(PdfAnnotationFlags::LockedContents)
    }

    #[inline]
    fn set_is_editable(&mut self, is_editable: bool) -> Result<(), PdfiumError> {
        self.update_one_flag_impl(PdfAnnotationFlags::LockedContents, !is_editable)
    }
}
