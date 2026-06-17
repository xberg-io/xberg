//! Defines the [PdfPageAnnotations] struct, exposing functionality related to the
//! annotations that have been added to a single `PdfPage`.

use crate::bindgen::{FPDF_ANNOTATION, FPDF_DOCUMENT, FPDF_FORMHANDLE, FPDF_PAGE};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::color::PdfColor;
use crate::pdf::document::page::annotation::free_text::PdfPageFreeTextAnnotation;
use crate::pdf::document::page::annotation::highlight::PdfPageHighlightAnnotation;
use crate::pdf::document::page::annotation::ink::PdfPageInkAnnotation;
use crate::pdf::document::page::annotation::link::PdfPageLinkAnnotation;
use crate::pdf::document::page::annotation::private::internal::PdfPageAnnotationPrivate;
use crate::pdf::document::page::annotation::square::PdfPageSquareAnnotation;
use crate::pdf::document::page::annotation::squiggly::PdfPageSquigglyAnnotation;
use crate::pdf::document::page::annotation::stamp::PdfPageStampAnnotation;
use crate::pdf::document::page::annotation::strikeout::PdfPageStrikeoutAnnotation;
use crate::pdf::document::page::annotation::text::PdfPageTextAnnotation;
use crate::pdf::document::page::annotation::underline::PdfPageUnderlineAnnotation;
use crate::pdf::document::page::annotation::{PdfPageAnnotation, PdfPageAnnotationCommon, PdfPageAnnotationType};
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectCommon};
use crate::pdf::document::page::{PdfPage, PdfPageContentRegenerationStrategy, PdfPageIndexCache};
use crate::pdf::quad_points::PdfQuadPoints;
use chrono::prelude::*;
use std::ops::Range;
use std::os::raw::c_int;

/// The zero-based index of a single [PdfPageAnnotation] inside its containing
/// [PdfPageAnnotations] collection.
pub type PdfPageAnnotationIndex = usize;

/// The annotations that have been added to a single `PdfPage`.
pub struct PdfPageAnnotations<'a> {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    form_handle: Option<FPDF_FORMHANDLE>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageAnnotations<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        form_handle: Option<FPDF_FORMHANDLE>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageAnnotations {
            document_handle,
            page_handle,
            form_handle,
            bindings,
        }
    }

    /// Returns the internal `FPDF_DOCUMENT` handle of the [PdfDocument] containing this
    /// [PdfPageAnnotations] collection.
    #[inline]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the internal `FPDF_PAGE` handle of the [PdfPage] containing this
    /// [PdfPageAnnotations] collection.
    #[inline]
    pub(crate) fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageAnnotations] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the total number of annotations that have been added to the containing `PdfPage`.
    #[inline]
    pub fn len(&self) -> PdfPageAnnotationIndex {
        self.bindings().FPDFPage_GetAnnotCount(self.page_handle) as PdfPageAnnotationIndex
    }

    /// Returns true if this [PdfPageAnnotations] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from 0..(number of annotations) for this [PdfPageAnnotations] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageAnnotationIndex> {
        0..self.len()
    }

    /// Returns a single [PdfPageAnnotation] from this [PdfPageAnnotations] collection.
    pub fn get(&self, index: PdfPageAnnotationIndex) -> Result<PdfPageAnnotation<'a>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageAnnotationIndexOutOfBounds);
        }

        let annotation_handle = self.bindings().FPDFPage_GetAnnot(self.page_handle, index as c_int);

        if annotation_handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfPageAnnotation::from_pdfium(
                self.document_handle,
                self.page_handle,
                annotation_handle,
                self.form_handle,
                self.bindings,
            ))
        }
    }

    /// Returns the first [PdfPageAnnotation] in this [PdfPageAnnotations] collection.
    #[inline]
    pub fn first(&self) -> Result<PdfPageAnnotation<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(0)
        } else {
            Err(PdfiumError::NoAnnotationsInCollection)
        }
    }

    /// Returns the last [PdfPageAnnotation] in this [PdfPageAnnotations] collection.
    #[inline]
    pub fn last(&self) -> Result<PdfPageAnnotation<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(self.len() - 1)
        } else {
            Err(PdfiumError::NoAnnotationsInCollection)
        }
    }

    /// Returns an iterator over all the annotations in this [PdfPageAnnotations] collection.
    #[inline]
    pub fn iter(&self) -> PdfPageAnnotationsIterator<'_> {
        PdfPageAnnotationsIterator::new(self)
    }

    /// Creates a new annotation of the given [PdfPageAnnotationType] by passing the result of calling
    /// `FPDFPage_CreateAnnot()` to an annotation constructor function.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    pub(crate) fn create_annotation<T: PdfPageAnnotationCommon>(
        &mut self,
        annotation_type: PdfPageAnnotationType,
        constructor: fn(FPDF_DOCUMENT, FPDF_PAGE, FPDF_ANNOTATION, &'a dyn PdfiumLibraryBindings) -> T,
    ) -> Result<T, PdfiumError> {
        let handle = self
            .bindings()
            .FPDFPage_CreateAnnot(self.page_handle(), annotation_type.as_pdfium());

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            let mut annotation = constructor(self.document_handle(), self.page_handle(), handle, self.bindings());

            annotation
                .set_creation_date(Utc::now())
                .and_then(|()| {
                    if let Some(content_regeneration_strategy) =
                        PdfPageIndexCache::get_content_regeneration_strategy_for_page(
                            self.document_handle(),
                            self.page_handle(),
                        )
                    {
                        if content_regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange {
                            PdfPage::regenerate_content_immut_for_handle(self.page_handle(), self.bindings())
                        } else {
                            Ok(())
                        }
                    } else {
                        Err(PdfiumError::SourcePageIndexNotInCache)
                    }
                })
                .map(|()| annotation)
        }
    }

    /// Creates a new [PdfPageFreeTextAnnotation] containing the given text in this
    /// [PdfPageAnnotations] collection, returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_free_text_annotation(&mut self, text: &str) -> Result<PdfPageFreeTextAnnotation<'a>, PdfiumError> {
        let mut annotation =
            self.create_annotation(PdfPageAnnotationType::FreeText, PdfPageFreeTextAnnotation::from_pdfium)?;

        annotation.set_contents(text)?;

        Ok(annotation)
    }

    /// Creates a new [PdfPageHighlightAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_highlight_annotation(&mut self) -> Result<PdfPageHighlightAnnotation<'a>, PdfiumError> {
        self.create_annotation(
            PdfPageAnnotationType::Highlight,
            PdfPageHighlightAnnotation::from_pdfium,
        )
    }

    /// Creates a new [PdfPageInkAnnotation] in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_ink_annotation(&mut self) -> Result<PdfPageInkAnnotation<'a>, PdfiumError> {
        self.create_annotation(PdfPageAnnotationType::Ink, PdfPageInkAnnotation::from_pdfium)
    }

    /// Creates a new [PdfPageLinkAnnotation] with the given URI in this [PdfPageAnnotations]
    /// collection, returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    pub fn create_link_annotation(&mut self, uri: &str) -> Result<PdfPageLinkAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_annotation(PdfPageAnnotationType::Link, PdfPageLinkAnnotation::from_pdfium)?;

        annotation.set_link(uri)?;

        Ok(annotation)
    }

    /// Creates a new [PdfPageSquareAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_square_annotation(&mut self) -> Result<PdfPageSquareAnnotation<'a>, PdfiumError> {
        self.create_annotation(PdfPageAnnotationType::Square, PdfPageSquareAnnotation::from_pdfium)
    }

    /// Creates a new [PdfPageSquigglyAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_squiggly_annotation(&mut self) -> Result<PdfPageSquigglyAnnotation<'a>, PdfiumError> {
        self.create_annotation(PdfPageAnnotationType::Squiggly, PdfPageSquigglyAnnotation::from_pdfium)
    }

    /// Creates a new [PdfPageStampAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_stamp_annotation(&mut self) -> Result<PdfPageStampAnnotation<'a>, PdfiumError> {
        self.create_annotation(PdfPageAnnotationType::Stamp, PdfPageStampAnnotation::from_pdfium)
    }

    /// Creates a new [PdfPageStrikeoutAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_strikeout_annotation(&mut self) -> Result<PdfPageStrikeoutAnnotation<'a>, PdfiumError> {
        self.create_annotation(
            PdfPageAnnotationType::Strikeout,
            PdfPageStrikeoutAnnotation::from_pdfium,
        )
    }

    /// Creates a new [PdfPageTextAnnotation] containing the given text in this [PdfPageAnnotations]
    /// collection, returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_text_annotation(&mut self, text: &str) -> Result<PdfPageTextAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_annotation(PdfPageAnnotationType::Text, PdfPageTextAnnotation::from_pdfium)?;

        annotation.set_contents(text)?;

        Ok(annotation)
    }

    /// Creates a new [PdfPageUnderlineAnnotation] annotation in this [PdfPageAnnotations] collection,
    /// returning the newly created annotation.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_underline_annotation(&mut self) -> Result<PdfPageUnderlineAnnotation<'a>, PdfiumError> {
        self.create_annotation(
            PdfPageAnnotationType::Underline,
            PdfPageUnderlineAnnotation::from_pdfium,
        )
    }

    // Convenience functions for creating and positioning markup annotations
    // in a single function call.

    /// Creates a new [PdfPageSquigglyAnnotation] annotation and positions it underneath the given
    /// [PdfPageObject], coloring it with the given [PdfColor].
    ///
    /// If the given contents string is supplied, the annotation will be additionally configured
    /// so that when the given [PdfPageObject] is clicked in a conforming PDF viewer, the given
    /// contents string will be displayed in a popup window.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_squiggly_annotation_under_object(
        &mut self,
        object: &PdfPageObject,
        color: PdfColor,
        contents: Option<&str>,
    ) -> Result<PdfPageSquigglyAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_squiggly_annotation()?;

        // The annotation will not display if it is not positioned.

        let bounds = object.bounds()?;

        annotation.set_position(bounds.left(), bounds.bottom())?;
        annotation.set_stroke_color(color)?;

        const SQUIGGLY_HEIGHT: f32 = 12.0;

        let annotation_top = bounds.bottom().value - 5.0;
        let annotation_bottom = annotation_top - SQUIGGLY_HEIGHT;

        annotation
            .attachment_points_mut()
            .create_attachment_point_at_end(PdfQuadPoints::new_from_values(
                bounds.left().value,
                annotation_bottom,
                bounds.right().value,
                annotation_bottom,
                bounds.right().value,
                annotation_top,
                bounds.left().value,
                annotation_top,
            ))?;

        if let Some(contents) = contents {
            annotation.set_width(bounds.width())?;
            annotation.set_height(bounds.height())?;
            annotation.set_contents(contents)?;
        }

        Ok(annotation)
    }

    /// Creates a new [PdfPageUnderlineAnnotation] annotation and positions it underneath the given
    /// [PdfPageObject], coloring it with the given [PdfColor].
    ///
    /// If the given contents string is supplied, the annotation will be additionally configured
    /// so that when the given [PdfPageObject] is clicked in a conforming PDF viewer, the given
    /// contents string will be displayed in a popup window.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_underline_annotation_under_object(
        &mut self,
        object: &PdfPageObject,
        color: PdfColor,
        contents: Option<&str>,
    ) -> Result<PdfPageUnderlineAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_underline_annotation()?;

        // The annotation will not display if it is not positioned.

        let bounds = object.bounds()?;

        annotation.set_position(bounds.left(), bounds.bottom())?;
        annotation.set_stroke_color(color)?;
        annotation
            .attachment_points_mut()
            .create_attachment_point_at_end(bounds)?;

        if let Some(contents) = contents {
            annotation.set_width(bounds.width())?;
            annotation.set_height(bounds.height())?;
            annotation.set_contents(contents)?;
        }

        Ok(annotation)
    }

    /// Creates a new [PdfPageStrikeoutAnnotation] annotation and vertically positions it in the
    /// center the given [PdfPageObject], coloring it with the given [PdfColor].
    ///
    /// If the given contents string is supplied, the annotation will be additionally configured
    /// so that when the given [PdfPageObject] is clicked in a conforming PDF viewer, the given
    /// contents string will be displayed in a popup window.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_strikeout_annotation_through_object(
        &mut self,
        object: &PdfPageObject,
        color: PdfColor,
        contents: Option<&str>,
    ) -> Result<PdfPageStrikeoutAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_strikeout_annotation()?;

        // The annotation will not display if it is not positioned.

        let bounds = object.bounds()?;

        annotation.set_position(bounds.left(), bounds.bottom())?;
        annotation.set_stroke_color(color)?;
        annotation
            .attachment_points_mut()
            .create_attachment_point_at_end(bounds)?;

        if let Some(contents) = contents {
            annotation.set_width(bounds.width())?;
            annotation.set_height(bounds.height())?;
            annotation.set_contents(contents)?;
        }

        Ok(annotation)
    }

    /// Creates a new [PdfPageHighlightAnnotation] annotation and positions it so as to cover
    /// the given [PdfPageObject], coloring it with the given [PdfColor].
    ///
    /// If the given contents string is supplied, the annotation will be additionally configured
    /// so that when the given [PdfPageObject] is clicked in a conforming PDF viewer, the given
    /// contents string will be displayed in a popup window.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    pub fn create_highlight_annotation_over_object(
        &mut self,
        object: &PdfPageObject,
        color: PdfColor,
        contents: Option<&str>,
    ) -> Result<PdfPageHighlightAnnotation<'a>, PdfiumError> {
        let mut annotation = self.create_highlight_annotation()?;

        // The annotation will not display if it is not positioned.

        let bounds = object.bounds()?;

        annotation.set_position(bounds.left(), bounds.bottom())?;
        annotation.set_stroke_color(color)?;
        annotation
            .attachment_points_mut()
            .create_attachment_point_at_end(bounds)?;

        if let Some(contents) = contents {
            annotation.set_width(bounds.width())?;
            annotation.set_height(bounds.height())?;
            annotation.set_contents(contents)?;
        }

        Ok(annotation)
    }

    /// Removes the given [PdfPageAnnotation] from this [PdfPageAnnotations] collection,
    /// consuming the [PdfPageAnnotation].
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    pub fn delete_annotation(&mut self, annotation: PdfPageAnnotation<'a>) -> Result<(), PdfiumError> {
        let index = self
            .bindings()
            .FPDFPage_GetAnnotIndex(self.page_handle(), annotation.handle());

        if index == -1 {
            return Err(PdfiumError::PageAnnotationIndexOutOfBounds);
        }

        if self
            .bindings()
            .is_true(self.bindings().FPDFPage_RemoveAnnot(self.page_handle(), index))
        {
            if let Some(content_regeneration_strategy) = PdfPageIndexCache::get_content_regeneration_strategy_for_page(
                self.document_handle(),
                self.page_handle(),
            ) {
                if content_regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange {
                    PdfPage::regenerate_content_immut_for_handle(self.page_handle(), self.bindings())
                } else {
                    Ok(())
                }
            } else {
                Err(PdfiumError::SourcePageIndexNotInCache)
            }
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }
}

/// An iterator over all the [PdfPageAnnotation] objects in a [PdfPageAnnotations] collection.
pub struct PdfPageAnnotationsIterator<'a> {
    annotations: &'a PdfPageAnnotations<'a>,
    next_index: PdfPageAnnotationIndex,
}

impl<'a> PdfPageAnnotationsIterator<'a> {
    #[inline]
    pub(crate) fn new(annotations: &'a PdfPageAnnotations<'a>) -> Self {
        PdfPageAnnotationsIterator {
            annotations,
            next_index: 0,
        }
    }
}

impl<'a> Iterator for PdfPageAnnotationsIterator<'a> {
    type Item = PdfPageAnnotation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.annotations.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
