//! Defines the [PdfPageObjectsCommon] trait, providing functionality common to all
//! containers of multiple [PdfPageObject] objects.

use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::color::PdfColor;
use crate::pdf::document::fonts::ToPdfFontToken;
use crate::pdf::document::page::PdfPageObjectOwnership;
use crate::pdf::document::page::object::image::PdfPageImageObject;
use crate::pdf::document::page::object::path::PdfPagePathObject;
use crate::pdf::document::page::object::text::PdfPageTextObject;
use crate::pdf::document::page::object::x_object_form::PdfPageXObjectFormObject;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectCommon};
use crate::pdf::document::page::objects::private::internal::PdfPageObjectsPrivate;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use std::ops::{Range, RangeInclusive};

#[cfg(feature = "image_025")]
use image_025::DynamicImage;

#[cfg(doc)]
use {
    crate::pdf::document::page::PdfPage, crate::pdf::document::page::PdfPageContentRegenerationStrategy,
    crate::pdf::document::page::PdfPageObjects,
};

/// The zero-based index of a single [PdfPageObject] inside its containing [PdfPageObjects] collection.
pub type PdfPageObjectIndex = usize;

/// Functionality common to all containers of multiple [PdfPageObject] objects.
/// Both pages and annotations can contain page objects.
pub trait PdfPageObjectsCommon<'a> {
    /// Returns the total number of page objects in the collection.
    fn len(&self) -> PdfPageObjectIndex;

    /// Returns true if this page objects collection is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of objects)` for this page objects collection.
    #[inline]
    fn as_range(&self) -> Range<PdfPageObjectIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of objects - 1)` for this page objects collection.
    #[inline]
    fn as_range_inclusive(&self) -> RangeInclusive<PdfPageObjectIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfPageObject] from this page objects collection.
    fn get(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Returns the first [PdfPageObject] in this page objects collection.
    #[inline]
    fn first(&self) -> Result<PdfPageObject<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(0)
        } else {
            Err(PdfiumError::NoPageObjectsInCollection)
        }
    }

    /// Returns the last [PdfPageObject] in this page objects collection.
    #[inline]
    fn last(&self) -> Result<PdfPageObject<'a>, PdfiumError> {
        if !self.is_empty() {
            self.get(self.len() - 1)
        } else {
            Err(PdfiumError::NoPageObjectsInCollection)
        }
    }

    /// Returns an iterator over all the [PdfPageObject] objects in this page objects collection.
    fn iter(&'a self) -> PdfPageObjectsIterator<'a>;

    /// Returns the smallest bounding box that contains all the [PdfPageObject] objects in this
    /// page objects collection.
    fn bounds(&'a self) -> PdfRect {
        let mut bottom: f32 = 0.0;
        let mut top: f32 = 0.0;
        let mut left: f32 = 0.0;
        let mut right: f32 = 0.0;

        for object in self.iter() {
            if let Ok(bounds) = object.bounds() {
                bottom = bottom.min(bounds.bottom().value);
                top = top.max(bounds.top().value);
                left = left.min(bounds.left().value);
                right = right.max(bounds.right().value);
            }
        }

        PdfRect::new_from_values(bottom, left, top, right)
    }

    /// Adds the given [PdfPageObject] to this page objects collection. The object's
    /// memory ownership will be transferred to the [PdfPage] containing this page objects
    /// collection, and the updated page object will be returned.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn add_object(&mut self, object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Adds the given [PdfPageTextObject] to this page objects collection,
    /// returning the text object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    #[inline]
    fn add_text_object(&mut self, object: PdfPageTextObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.add_object(PdfPageObject::Text(object))
    }

    /// Creates a new [PdfPageTextObject] at the given x and y page co-ordinates
    /// from the given arguments and adds it to this page objects collection,
    /// returning the text object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_text_object(
        &mut self,
        x: PdfPoints,
        y: PdfPoints,
        text: impl ToString,
        font: impl ToPdfFontToken,
        font_size: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Adds the given [PdfPagePathObject] to this page objects collection,
    /// returning the path object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    #[inline]
    fn add_path_object(&mut self, object: PdfPagePathObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.add_object(PdfPageObject::Path(object))
    }

    /// Adds the given [PdfPageXObjectFormObject] to this page objects collection,
    /// returning the XObject form object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing `PdfPage` has a content regeneration strategy of
    /// `PdfPageContentRegenerationStrategy::AutomaticOnEveryChange` then content regeneration
    /// will be triggered on the page.
    #[inline]
    fn add_x_object_form_object(
        &mut self,
        object: PdfPageXObjectFormObject<'a>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.add_object(PdfPageObject::XObjectForm(object))
    }

    /// Creates a new [PdfPagePathObject] for the given line, with the given
    /// stroke settings applied. The new path object will be added to this page objects collection
    /// and then returned, wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_path_object_line(
        &mut self,
        x1: PdfPoints,
        y1: PdfPoints,
        x2: PdfPoints,
        y2: PdfPoints,
        stroke_color: PdfColor,
        stroke_width: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject] for the given cubic Bézier curve, with the given
    /// stroke settings applied. The new path object will be added to this page objects collection
    /// and then returned, wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    #[allow(clippy::too_many_arguments)]
    fn create_path_object_bezier(
        &mut self,
        x1: PdfPoints,
        y1: PdfPoints,
        x2: PdfPoints,
        y2: PdfPoints,
        control1_x: PdfPoints,
        control1_y: PdfPoints,
        control2_x: PdfPoints,
        control2_y: PdfPoints,
        stroke_color: PdfColor,
        stroke_width: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject] for the given rectangle, with the given
    /// fill and stroke settings applied. Both the stroke color and the stroke width must be
    /// provided for the rectangle to be stroked. The new path object will be added to
    /// this page objects collection and then returned, wrapped inside a generic
    /// [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_path_object_rect(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject]. The new path will be created with a circle that fills
    /// the given rectangle, with the given fill and stroke settings applied. Both the stroke color
    /// and the stroke width must be provided for the circle to be stroked. The new path object
    /// will be added to this page objects collection and then returned, wrapped inside a generic
    /// [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_path_object_circle(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject]. The new path will be created with a circle centered
    /// at the given coordinates, with the given radius, and with the given fill and stroke settings
    /// applied. Both the stroke color and the stroke width must be provided for the circle to be
    /// stroked. The new path object will be added to this page objects collection and then
    /// returned, wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_path_object_circle_at(
        &mut self,
        center_x: PdfPoints,
        center_y: PdfPoints,
        radius: PdfPoints,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject]. The new path will be created with an ellipse that fills
    /// the given rectangle, with the given fill and stroke settings applied. Both the stroke color
    /// and the stroke width must be provided for the ellipse to be stroked. The new path object
    /// will be added to this page objects collection and then returned, wrapped inside a generic
    /// [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn create_path_object_ellipse(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Creates a new [PdfPagePathObject]. The new path will be created with an ellipse centered
    /// at the given coordinates, with the given radii, and with the given fill and stroke settings
    /// applied. Both the stroke color and the stroke width must be provided for the ellipse to be
    /// stroked. The new path object will be added to this page objects collection and then
    /// returned, wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    #[allow(clippy::too_many_arguments)]
    fn create_path_object_ellipse_at(
        &mut self,
        center_x: PdfPoints,
        center_y: PdfPoints,
        x_radius: PdfPoints,
        y_radius: PdfPoints,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Adds the given [PdfPageImageObject] to this page objects collection,
    /// returning the image object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    #[inline]
    fn add_image_object(&mut self, object: PdfPageImageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.add_object(PdfPageObject::Image(object))
    }

    /// Creates a new [PdfPageImageObject] at the given x and y page co-ordinates
    /// from the given arguments and adds it to this page objects collection,
    /// returning the image object wrapped inside a generic [PdfPageObject] wrapper.
    ///
    /// By default, new image objects have their width and height both set to 1.0 points.
    /// If provided, the given width and/or height will be applied to the newly created object to
    /// scale its size.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    fn create_image_object(
        &mut self,
        x: PdfPoints,
        y: PdfPoints,
        image: &DynamicImage,
        width: Option<PdfPoints>,
        height: Option<PdfPoints>,
    ) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Removes the given [PdfPageObject] from this page objects collection. The object's
    /// memory ownership will be removed from the [PdfPage] containing this page objects
    /// collection, and the updated page object will be returned. It can be added back to a
    /// page objects collection or dropped, at which point the memory owned by the object will
    /// be freed.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn remove_object(&mut self, object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError>;

    /// Removes the [PdfPageObject] at the given index from this page objects collection.
    /// The object's memory ownership will be removed from the [PdfPage] containing this page objects
    /// collection, and the updated page object will be returned. It can be added back into a
    /// page objects collection or discarded, at which point the memory owned by the object will
    /// be freed.
    ///
    /// If the containing [PdfPage] has a content regeneration strategy of
    /// [PdfPageContentRegenerationStrategy::AutomaticOnEveryChange] then content regeneration
    /// will be triggered on the page.
    fn remove_object_at_index(&mut self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError>;
}

// Blanket implementation for all PdfPageObjects collection types.

impl<'a, T> PdfPageObjectsCommon<'a> for T
where
    T: PdfPageObjectsPrivate<'a>,
{
    #[inline]
    fn len(&self) -> PdfPageObjectIndex {
        self.len_impl()
    }

    #[inline]
    fn get(&self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.get_impl(index)
    }

    #[inline]
    fn iter(&'a self) -> PdfPageObjectsIterator<'a> {
        self.iter_impl()
    }

    #[inline]
    fn add_object(&mut self, object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.add_object_impl(object)
    }

    #[inline]
    fn create_text_object(
        &mut self,
        x: PdfPoints,
        y: PdfPoints,
        text: impl ToString,
        font: impl ToPdfFontToken,
        font_size: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let document_handle = match self.ownership() {
            PdfPageObjectOwnership::Page(ownership) => Some(ownership.document_handle()),
            PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.document_handle()),
            PdfPageObjectOwnership::UnattachedAnnotation(ownership) => Some(ownership.document_handle()),
            _ => None,
        };

        if let Some(document_handle) = document_handle {
            let mut object = PdfPageTextObject::new_from_handles(
                document_handle,
                text,
                font.token().handle(),
                font_size,
                self.bindings(),
            )?;

            object.translate(x, y)?;

            self.add_text_object(object)
        } else {
            Err(PdfiumError::OwnershipNotAttachedToPage)
        }
    }

    #[inline]
    fn create_path_object_line(
        &mut self,
        x1: PdfPoints,
        y1: PdfPoints,
        x2: PdfPoints,
        y2: PdfPoints,
        stroke_color: PdfColor,
        stroke_width: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object =
            PdfPagePathObject::new_line_from_bindings(self.bindings(), x1, y1, x2, y2, stroke_color, stroke_width)?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_bezier(
        &mut self,
        x1: PdfPoints,
        y1: PdfPoints,
        x2: PdfPoints,
        y2: PdfPoints,
        control1_x: PdfPoints,
        control1_y: PdfPoints,
        control2_x: PdfPoints,
        control2_y: PdfPoints,
        stroke_color: PdfColor,
        stroke_width: PdfPoints,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object = PdfPagePathObject::new_bezier_from_bindings(
            self.bindings(),
            x1,
            y1,
            x2,
            y2,
            control1_x,
            control1_y,
            control2_x,
            control2_y,
            stroke_color,
            stroke_width,
        )?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_rect(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object =
            PdfPagePathObject::new_rect_from_bindings(self.bindings(), rect, stroke_color, stroke_width, fill_color)?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_circle(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object =
            PdfPagePathObject::new_circle_from_bindings(self.bindings(), rect, stroke_color, stroke_width, fill_color)?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_circle_at(
        &mut self,
        center_x: PdfPoints,
        center_y: PdfPoints,
        radius: PdfPoints,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object = PdfPagePathObject::new_circle_at_from_bindings(
            self.bindings(),
            center_x,
            center_y,
            radius,
            stroke_color,
            stroke_width,
            fill_color,
        )?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_ellipse(
        &mut self,
        rect: PdfRect,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object = PdfPagePathObject::new_ellipse_from_bindings(
            self.bindings(),
            rect,
            stroke_color,
            stroke_width,
            fill_color,
        )?;

        self.add_path_object(object)
    }

    #[inline]
    fn create_path_object_ellipse_at(
        &mut self,
        center_x: PdfPoints,
        center_y: PdfPoints,
        x_radius: PdfPoints,
        y_radius: PdfPoints,
        stroke_color: Option<PdfColor>,
        stroke_width: Option<PdfPoints>,
        fill_color: Option<PdfColor>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let object = PdfPagePathObject::new_ellipse_at_from_bindings(
            self.bindings(),
            center_x,
            center_y,
            x_radius,
            y_radius,
            stroke_color,
            stroke_width,
            fill_color,
        )?;

        self.add_path_object(object)
    }

    #[cfg(feature = "image_025")]
    fn create_image_object(
        &mut self,
        x: PdfPoints,
        y: PdfPoints,
        image: &DynamicImage,
        width: Option<PdfPoints>,
        height: Option<PdfPoints>,
    ) -> Result<PdfPageObject<'a>, PdfiumError> {
        let document_handle = match self.ownership() {
            PdfPageObjectOwnership::Page(ownership) => Some(ownership.document_handle()),
            PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.document_handle()),
            PdfPageObjectOwnership::UnattachedAnnotation(ownership) => Some(ownership.document_handle()),
            _ => None,
        };

        if let Some(document_handle) = document_handle {
            let image_width = image.width();

            let image_height = image.height();

            let mut object = PdfPageImageObject::new_from_handle(document_handle, self.bindings())?;

            object.set_image(image)?;

            // Apply specified dimensions, if provided.

            match (width, height) {
                (Some(width), Some(height)) => {
                    object.scale(width.value, height.value)?;
                }
                (Some(width), None) => {
                    let aspect_ratio = image_height as f32 / image_width as f32;

                    let height = width * aspect_ratio;

                    object.scale(width.value, height.value)?;
                }
                (None, Some(height)) => {
                    let aspect_ratio = image_height as f32 / image_width as f32;

                    let width = height / aspect_ratio;

                    object.scale(width.value, height.value)?;
                }
                (None, None) => {}
            }

            object.translate(x, y)?;

            self.add_image_object(object)
        } else {
            Err(PdfiumError::OwnershipNotAttachedToPage)
        }
    }

    #[inline]
    fn remove_object(&mut self, object: PdfPageObject<'a>) -> Result<PdfPageObject<'a>, PdfiumError> {
        self.remove_object_impl(object)
    }

    fn remove_object_at_index(&mut self, index: PdfPageObjectIndex) -> Result<PdfPageObject<'a>, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::PageObjectIndexOutOfBounds);
        }

        if let Ok(object) = self.get(index) {
            self.remove_object(object)
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }
}

/// An iterator over all the [PdfPageObject] objects in a page objects collection.
pub struct PdfPageObjectsIterator<'a> {
    objects: &'a dyn PdfPageObjectsPrivate<'a>,
    next_index: PdfPageObjectIndex,
}

impl<'a> PdfPageObjectsIterator<'a> {
    #[inline]
    pub(crate) fn new(objects: &'a dyn PdfPageObjectsPrivate<'a>) -> Self {
        PdfPageObjectsIterator { objects, next_index: 0 }
    }
}

impl<'a> Iterator for PdfPageObjectsIterator<'a> {
    type Item = PdfPageObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.objects.get_impl(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}
