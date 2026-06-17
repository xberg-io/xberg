//! Defines the [PdfPageAnnotation] struct, exposing functionality related to a single annotation.

pub mod attachment_points;
pub mod circle;
pub mod free_text;
pub mod highlight;
pub mod ink;
pub mod link;
pub mod objects;
pub(crate) mod private; // Keep private so that the PdfPageAnnotationPrivate trait is not exposed.
pub mod square;
pub mod squiggly;
pub mod stamp;
pub mod strikeout;
pub mod text;
pub mod underline;
pub mod unsupported;

use crate::bindgen::{
    FPDF_ANNOT_CARET, FPDF_ANNOT_CIRCLE, FPDF_ANNOT_FILEATTACHMENT, FPDF_ANNOT_FREETEXT, FPDF_ANNOT_HIGHLIGHT,
    FPDF_ANNOT_INK, FPDF_ANNOT_LINE, FPDF_ANNOT_LINK, FPDF_ANNOT_MOVIE, FPDF_ANNOT_POLYGON, FPDF_ANNOT_POLYLINE,
    FPDF_ANNOT_POPUP, FPDF_ANNOT_PRINTERMARK, FPDF_ANNOT_REDACT, FPDF_ANNOT_RICHMEDIA, FPDF_ANNOT_SCREEN,
    FPDF_ANNOT_SOUND, FPDF_ANNOT_SQUARE, FPDF_ANNOT_SQUIGGLY, FPDF_ANNOT_STAMP, FPDF_ANNOT_STRIKEOUT, FPDF_ANNOT_TEXT,
    FPDF_ANNOT_THREED, FPDF_ANNOT_TRAPNET, FPDF_ANNOT_UNDERLINE, FPDF_ANNOT_UNKNOWN, FPDF_ANNOT_WATERMARK,
    FPDF_ANNOT_WIDGET, FPDF_ANNOT_XFAWIDGET, FPDF_ANNOTATION, FPDF_ANNOTATION_SUBTYPE, FPDF_DOCUMENT, FPDF_FORMHANDLE,
    FPDF_PAGE,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::color::PdfColor;
use crate::pdf::document::page::annotation::attachment_points::PdfPageAnnotationAttachmentPoints;
use crate::pdf::document::page::annotation::circle::PdfPageCircleAnnotation;
use crate::pdf::document::page::annotation::free_text::PdfPageFreeTextAnnotation;
use crate::pdf::document::page::annotation::highlight::PdfPageHighlightAnnotation;
use crate::pdf::document::page::annotation::ink::PdfPageInkAnnotation;
use crate::pdf::document::page::annotation::link::PdfPageLinkAnnotation;
use crate::pdf::document::page::annotation::objects::PdfPageAnnotationObjects;
use crate::pdf::document::page::annotation::private::internal::{PdfAnnotationFlags, PdfPageAnnotationPrivate};
use crate::pdf::document::page::annotation::square::PdfPageSquareAnnotation;
use crate::pdf::document::page::annotation::squiggly::PdfPageSquigglyAnnotation;
use crate::pdf::document::page::annotation::stamp::PdfPageStampAnnotation;
use crate::pdf::document::page::annotation::strikeout::PdfPageStrikeoutAnnotation;
use crate::pdf::document::page::annotation::text::PdfPageTextAnnotation;
use crate::pdf::document::page::annotation::underline::PdfPageUnderlineAnnotation;
use crate::pdf::document::page::annotation::unsupported::PdfPageUnsupportedAnnotation;
use crate::pdf::document::page::object::ownership::PdfPageObjectOwnership;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use chrono::prelude::*;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// The type of a single [PdfPageAnnotation], as defined in table 8.20 of the PDF Reference,
/// version 1.7, on page 615.
///
/// Not all PDF annotation types are supported by Pdfium. For example, Pdfium does not
/// currently support embedded sound or movie file annotations, embedded 3D animations, or
/// annotations containing embedded file attachments.
///
/// Pdfium currently supports creating, editing, and rendering the following types of annotations:
///
/// * [PdfPageAnnotationType::Circle]
/// * [PdfPageAnnotationType::FreeText]
/// * [PdfPageAnnotationType::Highlight]
/// * [PdfPageAnnotationType::Ink]
/// * [PdfPageAnnotationType::Link]
/// * [PdfPageAnnotationType::Popup]
/// * [PdfPageAnnotationType::Redacted]
/// * [PdfPageAnnotationType::Square]
/// * [PdfPageAnnotationType::Squiggly]
/// * [PdfPageAnnotationType::Stamp]
/// * [PdfPageAnnotationType::Strikeout]
/// * [PdfPageAnnotationType::Text]
/// * [PdfPageAnnotationType::Underline]
/// * [PdfPageAnnotationType::Widget]
/// * [PdfPageAnnotationType::XfaWidget]
///
/// Note that a `FreeText` annotation is rendered directly on the page, whereas a `Text` annotation
/// floats over the page inside its own enclosed area. Adobe often uses the term "sticky note"
/// in reference to `Text` annotations to distinguish them from `FreeText` annotations.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum PdfPageAnnotationType {
    Unknown = FPDF_ANNOT_UNKNOWN as isize,
    Text = FPDF_ANNOT_TEXT as isize,
    Link = FPDF_ANNOT_LINK as isize,
    FreeText = FPDF_ANNOT_FREETEXT as isize,
    Line = FPDF_ANNOT_LINE as isize,
    Square = FPDF_ANNOT_SQUARE as isize,
    Circle = FPDF_ANNOT_CIRCLE as isize,
    Polygon = FPDF_ANNOT_POLYGON as isize,
    Polyline = FPDF_ANNOT_POLYLINE as isize,
    Highlight = FPDF_ANNOT_HIGHLIGHT as isize,
    Underline = FPDF_ANNOT_UNDERLINE as isize,
    Squiggly = FPDF_ANNOT_SQUIGGLY as isize,
    Strikeout = FPDF_ANNOT_STRIKEOUT as isize,
    Stamp = FPDF_ANNOT_STAMP as isize,
    Caret = FPDF_ANNOT_CARET as isize,
    Ink = FPDF_ANNOT_INK as isize,
    Popup = FPDF_ANNOT_POPUP as isize,
    FileAttachment = FPDF_ANNOT_FILEATTACHMENT as isize,
    Sound = FPDF_ANNOT_SOUND as isize,
    Movie = FPDF_ANNOT_MOVIE as isize,
    Widget = FPDF_ANNOT_WIDGET as isize,
    Screen = FPDF_ANNOT_SCREEN as isize,
    PrinterMark = FPDF_ANNOT_PRINTERMARK as isize,
    TrapNet = FPDF_ANNOT_TRAPNET as isize,
    Watermark = FPDF_ANNOT_WATERMARK as isize,
    ThreeD = FPDF_ANNOT_THREED as isize,
    RichMedia = FPDF_ANNOT_RICHMEDIA as isize,
    XfaWidget = FPDF_ANNOT_XFAWIDGET as isize,
    Redacted = FPDF_ANNOT_REDACT as isize,
}

impl PdfPageAnnotationType {
    pub(crate) fn from_pdfium(value: FPDF_ANNOTATION_SUBTYPE) -> Result<PdfPageAnnotationType, PdfiumError> {
        match value as u32 {
            FPDF_ANNOT_UNKNOWN => Ok(PdfPageAnnotationType::Unknown),
            FPDF_ANNOT_TEXT => Ok(PdfPageAnnotationType::Text),
            FPDF_ANNOT_LINK => Ok(PdfPageAnnotationType::Link),
            FPDF_ANNOT_FREETEXT => Ok(PdfPageAnnotationType::FreeText),
            FPDF_ANNOT_LINE => Ok(PdfPageAnnotationType::Line),
            FPDF_ANNOT_SQUARE => Ok(PdfPageAnnotationType::Square),
            FPDF_ANNOT_CIRCLE => Ok(PdfPageAnnotationType::Circle),
            FPDF_ANNOT_POLYGON => Ok(PdfPageAnnotationType::Polygon),
            FPDF_ANNOT_POLYLINE => Ok(PdfPageAnnotationType::Polyline),
            FPDF_ANNOT_HIGHLIGHT => Ok(PdfPageAnnotationType::Highlight),
            FPDF_ANNOT_UNDERLINE => Ok(PdfPageAnnotationType::Underline),
            FPDF_ANNOT_SQUIGGLY => Ok(PdfPageAnnotationType::Squiggly),
            FPDF_ANNOT_STRIKEOUT => Ok(PdfPageAnnotationType::Strikeout),
            FPDF_ANNOT_STAMP => Ok(PdfPageAnnotationType::Stamp),
            FPDF_ANNOT_CARET => Ok(PdfPageAnnotationType::Caret),
            FPDF_ANNOT_INK => Ok(PdfPageAnnotationType::Ink),
            FPDF_ANNOT_POPUP => Ok(PdfPageAnnotationType::Popup),
            FPDF_ANNOT_FILEATTACHMENT => Ok(PdfPageAnnotationType::FileAttachment),
            FPDF_ANNOT_SOUND => Ok(PdfPageAnnotationType::Sound),
            FPDF_ANNOT_MOVIE => Ok(PdfPageAnnotationType::Movie),
            FPDF_ANNOT_WIDGET => Ok(PdfPageAnnotationType::Widget),
            FPDF_ANNOT_SCREEN => Ok(PdfPageAnnotationType::Screen),
            FPDF_ANNOT_PRINTERMARK => Ok(PdfPageAnnotationType::PrinterMark),
            FPDF_ANNOT_TRAPNET => Ok(PdfPageAnnotationType::TrapNet),
            FPDF_ANNOT_WATERMARK => Ok(PdfPageAnnotationType::Watermark),
            FPDF_ANNOT_THREED => Ok(PdfPageAnnotationType::ThreeD),
            FPDF_ANNOT_RICHMEDIA => Ok(PdfPageAnnotationType::RichMedia),
            FPDF_ANNOT_XFAWIDGET => Ok(PdfPageAnnotationType::XfaWidget),
            FPDF_ANNOT_REDACT => Ok(PdfPageAnnotationType::Redacted),
            _ => Err(PdfiumError::UnknownPdfAnnotationType),
        }
    }

    #[allow(dead_code)]
    // The as_pdfium() function is not currently used, but we expect it to be in future
    pub(crate) fn as_pdfium(&self) -> FPDF_ANNOTATION_SUBTYPE {
        (match self {
            PdfPageAnnotationType::Unknown => FPDF_ANNOT_UNKNOWN,
            PdfPageAnnotationType::Text => FPDF_ANNOT_TEXT,
            PdfPageAnnotationType::Link => FPDF_ANNOT_LINK,
            PdfPageAnnotationType::FreeText => FPDF_ANNOT_FREETEXT,
            PdfPageAnnotationType::Line => FPDF_ANNOT_LINE,
            PdfPageAnnotationType::Square => FPDF_ANNOT_SQUARE,
            PdfPageAnnotationType::Circle => FPDF_ANNOT_CIRCLE,
            PdfPageAnnotationType::Polygon => FPDF_ANNOT_POLYGON,
            PdfPageAnnotationType::Polyline => FPDF_ANNOT_POLYLINE,
            PdfPageAnnotationType::Highlight => FPDF_ANNOT_HIGHLIGHT,
            PdfPageAnnotationType::Underline => FPDF_ANNOT_UNDERLINE,
            PdfPageAnnotationType::Squiggly => FPDF_ANNOT_SQUIGGLY,
            PdfPageAnnotationType::Strikeout => FPDF_ANNOT_STRIKEOUT,
            PdfPageAnnotationType::Stamp => FPDF_ANNOT_STAMP,
            PdfPageAnnotationType::Caret => FPDF_ANNOT_CARET,
            PdfPageAnnotationType::Ink => FPDF_ANNOT_INK,
            PdfPageAnnotationType::Popup => FPDF_ANNOT_POPUP,
            PdfPageAnnotationType::FileAttachment => FPDF_ANNOT_FILEATTACHMENT,
            PdfPageAnnotationType::Sound => FPDF_ANNOT_SOUND,
            PdfPageAnnotationType::Movie => FPDF_ANNOT_MOVIE,
            PdfPageAnnotationType::Widget => FPDF_ANNOT_WIDGET,
            PdfPageAnnotationType::Screen => FPDF_ANNOT_SCREEN,
            PdfPageAnnotationType::PrinterMark => FPDF_ANNOT_PRINTERMARK,
            PdfPageAnnotationType::TrapNet => FPDF_ANNOT_TRAPNET,
            PdfPageAnnotationType::Watermark => FPDF_ANNOT_WATERMARK,
            PdfPageAnnotationType::ThreeD => FPDF_ANNOT_THREED,
            PdfPageAnnotationType::RichMedia => FPDF_ANNOT_RICHMEDIA,
            PdfPageAnnotationType::XfaWidget => FPDF_ANNOT_XFAWIDGET,
            PdfPageAnnotationType::Redacted => FPDF_ANNOT_REDACT,
        }) as FPDF_ANNOTATION_SUBTYPE
    }
}

/// A single user annotation on a [PdfPage].
pub enum PdfPageAnnotation<'a> {
    Circle(PdfPageCircleAnnotation<'a>),
    FreeText(PdfPageFreeTextAnnotation<'a>),
    Highlight(PdfPageHighlightAnnotation<'a>),
    Ink(PdfPageInkAnnotation<'a>),
    Link(PdfPageLinkAnnotation<'a>),
    Square(PdfPageSquareAnnotation<'a>),
    Squiggly(PdfPageSquigglyAnnotation<'a>),
    Stamp(PdfPageStampAnnotation<'a>),
    Strikeout(PdfPageStrikeoutAnnotation<'a>),
    Text(PdfPageTextAnnotation<'a>),
    Underline(PdfPageUnderlineAnnotation<'a>),

    /// Common properties shared by all [PdfPageAnnotation] types can still be accessed for
    /// annotations not supported by Pdfium, but annotation-specific functionality
    /// will be unavailable.
    Unsupported(PdfPageUnsupportedAnnotation<'a>),
}

impl<'a> PdfPageAnnotation<'a> {
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        annotation_handle: FPDF_ANNOTATION,
        _form_handle: Option<FPDF_FORMHANDLE>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        let annotation_type = PdfPageAnnotationType::from_pdfium(bindings.FPDFAnnot_GetSubtype(annotation_handle))
            .unwrap_or(PdfPageAnnotationType::Unknown);

        match annotation_type {
            PdfPageAnnotationType::Circle => PdfPageAnnotation::Circle(PdfPageCircleAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::FreeText => PdfPageAnnotation::FreeText(PdfPageFreeTextAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Highlight => PdfPageAnnotation::Highlight(PdfPageHighlightAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Ink => PdfPageAnnotation::Ink(PdfPageInkAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Link => PdfPageAnnotation::Link(PdfPageLinkAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Square => PdfPageAnnotation::Square(PdfPageSquareAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Squiggly => PdfPageAnnotation::Squiggly(PdfPageSquigglyAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Stamp => PdfPageAnnotation::Stamp(PdfPageStampAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Strikeout => PdfPageAnnotation::Strikeout(PdfPageStrikeoutAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Text => PdfPageAnnotation::Text(PdfPageTextAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            PdfPageAnnotationType::Underline => PdfPageAnnotation::Underline(PdfPageUnderlineAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                bindings,
            )),
            _ => PdfPageAnnotation::Unsupported(PdfPageUnsupportedAnnotation::from_pdfium(
                document_handle,
                page_handle,
                annotation_handle,
                annotation_type,
                bindings,
            )),
        }
    }

    #[inline]
    pub(crate) fn unwrap_as_trait(&self) -> &dyn PdfPageAnnotationPrivate<'a> {
        match self {
            PdfPageAnnotation::Circle(annotation) => annotation,
            PdfPageAnnotation::FreeText(annotation) => annotation,
            PdfPageAnnotation::Highlight(annotation) => annotation,
            PdfPageAnnotation::Ink(annotation) => annotation,
            PdfPageAnnotation::Link(annotation) => annotation,
            PdfPageAnnotation::Square(annotation) => annotation,
            PdfPageAnnotation::Squiggly(annotation) => annotation,
            PdfPageAnnotation::Stamp(annotation) => annotation,
            PdfPageAnnotation::Strikeout(annotation) => annotation,
            PdfPageAnnotation::Text(annotation) => annotation,
            PdfPageAnnotation::Underline(annotation) => annotation,
            PdfPageAnnotation::Unsupported(annotation) => annotation,
        }
    }

    #[inline]
    #[allow(dead_code)] // We don't currently use unwrap_as_trait_mut(), but we expect to in the future
    pub(crate) fn unwrap_as_trait_mut(&mut self) -> &mut dyn PdfPageAnnotationPrivate<'a> {
        match self {
            PdfPageAnnotation::Circle(annotation) => annotation,
            PdfPageAnnotation::FreeText(annotation) => annotation,
            PdfPageAnnotation::Highlight(annotation) => annotation,
            PdfPageAnnotation::Ink(annotation) => annotation,
            PdfPageAnnotation::Link(annotation) => annotation,
            PdfPageAnnotation::Square(annotation) => annotation,
            PdfPageAnnotation::Squiggly(annotation) => annotation,
            PdfPageAnnotation::Stamp(annotation) => annotation,
            PdfPageAnnotation::Strikeout(annotation) => annotation,
            PdfPageAnnotation::Text(annotation) => annotation,
            PdfPageAnnotation::Underline(annotation) => annotation,
            PdfPageAnnotation::Unsupported(annotation) => annotation,
        }
    }

    /// The type of this [PdfPageAnnotation].
    ///
    /// Not all PDF annotation types are supported by Pdfium. For example, Pdfium does not
    /// currently support embedded sound or movie file annotations, embedded 3D animations, or
    /// annotations containing embedded file attachments.
    ///
    /// Pdfium currently supports creating, editing, and rendering the following types of annotations:
    ///
    /// * [PdfPageAnnotationType::Circle]
    /// * [PdfPageAnnotationType::FreeText]
    /// * [PdfPageAnnotationType::Highlight]
    /// * [PdfPageAnnotationType::Ink]
    /// * [PdfPageAnnotationType::Link]
    /// * [PdfPageAnnotationType::Popup]
    /// * [PdfPageAnnotationType::Redacted]
    /// * [PdfPageAnnotationType::Square]
    /// * [PdfPageAnnotationType::Squiggly]
    /// * [PdfPageAnnotationType::Stamp]
    /// * [PdfPageAnnotationType::Strikeout]
    /// * [PdfPageAnnotationType::Text]
    /// * [PdfPageAnnotationType::Underline]
    /// * [PdfPageAnnotationType::Widget]
    /// * [PdfPageAnnotationType::XfaWidget]
    #[inline]
    pub fn annotation_type(&self) -> PdfPageAnnotationType {
        match self {
            PdfPageAnnotation::Circle(_) => PdfPageAnnotationType::Circle,
            PdfPageAnnotation::FreeText(_) => PdfPageAnnotationType::FreeText,
            PdfPageAnnotation::Highlight(_) => PdfPageAnnotationType::Highlight,
            PdfPageAnnotation::Ink(_) => PdfPageAnnotationType::Ink,
            PdfPageAnnotation::Link(_) => PdfPageAnnotationType::Link,
            PdfPageAnnotation::Square(_) => PdfPageAnnotationType::Square,
            PdfPageAnnotation::Squiggly(_) => PdfPageAnnotationType::Squiggly,
            PdfPageAnnotation::Stamp(_) => PdfPageAnnotationType::Stamp,
            PdfPageAnnotation::Strikeout(_) => PdfPageAnnotationType::Strikeout,
            PdfPageAnnotation::Text(_) => PdfPageAnnotationType::Text,
            PdfPageAnnotation::Underline(_) => PdfPageAnnotationType::Underline,
            PdfPageAnnotation::Unsupported(annotation) => annotation.get_type(),
        }
    }

    /// Returns `true` if Pdfium supports creating, editing, and rendering this type of
    /// [PdfPageAnnotation].
    ///
    /// Not all PDF annotation types are supported by Pdfium. For example, Pdfium does not
    /// currently support embedded sound or movie file annotations, embedded 3D animations, or
    /// annotations containing embedded file attachments.
    ///
    /// Pdfium currently supports creating, editing, and rendering the following types of annotations:
    ///
    /// * [PdfPageAnnotationType::Circle]
    /// * [PdfPageAnnotationType::FreeText]
    /// * [PdfPageAnnotationType::Highlight]
    /// * [PdfPageAnnotationType::Ink]
    /// * [PdfPageAnnotationType::Link]
    /// * [PdfPageAnnotationType::Popup]
    /// * [PdfPageAnnotationType::Redacted]
    /// * [PdfPageAnnotationType::Square]
    /// * [PdfPageAnnotationType::Squiggly]
    /// * [PdfPageAnnotationType::Stamp]
    /// * [PdfPageAnnotationType::Strikeout]
    /// * [PdfPageAnnotationType::Text]
    /// * [PdfPageAnnotationType::Underline]
    /// * [PdfPageAnnotationType::Widget]
    /// * [PdfPageAnnotationType::XfaWidget]
    #[inline]
    pub fn is_supported(&self) -> bool {
        !self.is_unsupported()
    }

    /// Returns `true` if Pdfium does _not_ support creating, editing, and rendering this type of
    /// [PdfPageAnnotation].
    ///
    /// Not all PDF annotation types are supported by Pdfium. For example, Pdfium does not
    /// currently support embedded sound or movie file annotations, embedded 3D animations, or
    /// annotations containing embedded file attachments.
    ///
    /// Pdfium currently supports creating, editing, and rendering the following types of annotations:
    ///
    /// * [PdfPageAnnotationType::Circle]
    /// * [PdfPageAnnotationType::FreeText]
    /// * [PdfPageAnnotationType::Highlight]
    /// * [PdfPageAnnotationType::Ink]
    /// * [PdfPageAnnotationType::Link]
    /// * [PdfPageAnnotationType::Popup]
    /// * [PdfPageAnnotationType::Redacted]
    /// * [PdfPageAnnotationType::Square]
    /// * [PdfPageAnnotationType::Squiggly]
    /// * [PdfPageAnnotationType::Stamp]
    /// * [PdfPageAnnotationType::Strikeout]
    /// * [PdfPageAnnotationType::Text]
    /// * [PdfPageAnnotationType::Underline]
    /// * [PdfPageAnnotationType::Widget]
    /// * [PdfPageAnnotationType::XfaWidget]
    #[inline]
    pub fn is_unsupported(&self) -> bool {
        matches!(self, PdfPageAnnotation::Unsupported(_))
    }

    /// Returns an immutable reference to the underlying [PdfPageCircleAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Circle].
    #[inline]
    pub fn as_circle_annotation(&self) -> Option<&PdfPageCircleAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Circle(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageCircleAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Circle].
    #[inline]
    pub fn as_circle_annotation_mut(&mut self) -> Option<&mut PdfPageCircleAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Circle(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageFreeTextAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::FreeText].
    #[inline]
    pub fn as_free_text_annotation(&self) -> Option<&PdfPageFreeTextAnnotation<'_>> {
        match self {
            PdfPageAnnotation::FreeText(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageFreeTextAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::FreeText].
    #[inline]
    pub fn as_free_text_annotation_mut(&mut self) -> Option<&mut PdfPageFreeTextAnnotation<'a>> {
        match self {
            PdfPageAnnotation::FreeText(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageHighlightAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Highlight].
    #[inline]
    pub fn as_highlight_annotation(&self) -> Option<&PdfPageHighlightAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Highlight(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageHighlightAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Highlight].
    #[inline]
    pub fn as_highlight_annotation_mut(&mut self) -> Option<&mut PdfPageHighlightAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Highlight(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageInkAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Ink].
    #[inline]
    pub fn as_ink_annotation(&self) -> Option<&PdfPageInkAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Ink(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageInkAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Ink].
    #[inline]
    pub fn as_ink_annotation_mut(&mut self) -> Option<&mut PdfPageInkAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Ink(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageLinkAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Link].
    #[inline]
    pub fn as_link_annotation(&self) -> Option<&PdfPageLinkAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Link(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageLinkAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Link].
    #[inline]
    pub fn as_link_annotation_mut(&mut self) -> Option<&mut PdfPageLinkAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Link(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageSquareAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Square].
    #[inline]
    pub fn as_square_annotation(&self) -> Option<&PdfPageSquareAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Square(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageSquareAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Square].
    #[inline]
    pub fn as_square_annotation_mut(&mut self) -> Option<&mut PdfPageSquareAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Square(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageSquigglyAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Squiggly].
    #[inline]
    pub fn as_squiggly_annotation(&self) -> Option<&PdfPageSquigglyAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Squiggly(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageSquigglyAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Squiggly].
    #[inline]
    pub fn as_squiggly_annotation_mut(&mut self) -> Option<&mut PdfPageSquigglyAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Squiggly(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageStampAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Stamp].
    #[inline]
    pub fn as_stamp_annotation(&self) -> Option<&PdfPageStampAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Stamp(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageStampAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Stamp].
    #[inline]
    pub fn as_stamp_annotation_mut(&mut self) -> Option<&mut PdfPageStampAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Stamp(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageStrikeoutAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Strikeout].
    #[inline]
    pub fn as_strikeout_annotation(&self) -> Option<&PdfPageStrikeoutAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Strikeout(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageStrikeoutAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Strikeout].
    #[inline]
    pub fn as_strikeout_annotation_mut(&mut self) -> Option<&mut PdfPageStrikeoutAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Strikeout(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageTextAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Text].
    #[inline]
    pub fn as_text_annotation(&self) -> Option<&PdfPageTextAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Text(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageTextAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Text].
    #[inline]
    pub fn as_text_annotation_mut(&mut self) -> Option<&mut PdfPageTextAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Text(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageUnderlineAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Underline].
    #[inline]
    pub fn as_underline_annotation(&self) -> Option<&PdfPageUnderlineAnnotation<'_>> {
        match self {
            PdfPageAnnotation::Underline(annotation) => Some(annotation),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageUnderlineAnnotation]
    /// for this [PdfPageAnnotation], if this annotation has an annotation type of
    /// [PdfPageAnnotationType::Underline].
    #[inline]
    pub fn as_underline_annotation_mut(&mut self) -> Option<&mut PdfPageUnderlineAnnotation<'a>> {
        match self {
            PdfPageAnnotation::Underline(annotation) => Some(annotation),
            _ => None,
        }
    }
}

/// Functionality common to all [PdfPageAnnotation] objects, regardless of their [PdfPageAnnotationType].
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
    fn set_creation_date(&mut self, date: DateTime<Utc>) -> Result<(), PdfiumError>;

    /// Returns the date and time when this [PdfPageAnnotation] was last modified, if any.
    fn modification_date(&self) -> Option<String>;

    /// Sets the date and time when this [PdfPageAnnotation] was last modified.
    fn set_modification_date(&mut self, date: DateTime<Utc>) -> Result<(), PdfiumError>;

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

// Blanket implementation for all PdfPageAnnotation types.

impl<'a, T> PdfPageAnnotationCommon for T
where
    T: PdfPageAnnotationPrivate<'a>,
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
    fn set_creation_date(&mut self, date: DateTime<Utc>) -> Result<(), PdfiumError> {
        self.set_creation_date_impl(date)
    }

    #[inline]
    fn modification_date(&self) -> Option<String> {
        self.modification_date_impl()
    }

    #[inline]
    fn set_modification_date(&mut self, date: DateTime<Utc>) -> Result<(), PdfiumError> {
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

impl<'a> PdfPageAnnotationPrivate<'a> for PdfPageAnnotation<'a> {
    #[inline]
    fn handle(&self) -> FPDF_ANNOTATION {
        self.unwrap_as_trait().handle()
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.unwrap_as_trait().bindings()
    }

    #[inline]
    fn ownership(&self) -> &PdfPageObjectOwnership {
        self.unwrap_as_trait().ownership()
    }

    #[inline]
    fn objects_impl(&self) -> &PdfPageAnnotationObjects<'_> {
        self.unwrap_as_trait().objects_impl()
    }

    #[inline]
    fn attachment_points_impl(&self) -> &PdfPageAnnotationAttachmentPoints<'_> {
        self.unwrap_as_trait().attachment_points_impl()
    }
}

impl<'a> Drop for PdfPageAnnotation<'a> {
    /// Closes this [PdfPageAnnotation], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        self.bindings().FPDFPage_CloseAnnot(self.handle());
    }
}
