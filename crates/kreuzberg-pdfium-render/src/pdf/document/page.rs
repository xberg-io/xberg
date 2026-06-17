//! Defines the [PdfPage] struct, exposing functionality related to a single page in a
//! [PdfPages] collection.

pub mod annotation;
pub mod annotations;
pub mod boundaries;
pub mod extraction;
pub(crate) mod index_cache;
pub mod links;
pub mod object;
pub mod objects;
pub mod paragraph;
pub mod render_config;
pub mod size;
pub mod struct_element;
pub mod struct_tree;
pub mod text;

use object::ownership::PdfPageObjectOwnership;

use crate::bindgen::{
    FLAT_PRINT, FLATTEN_FAIL, FLATTEN_NOTHINGTODO, FLATTEN_SUCCESS, FPDF_DOCUMENT, FPDF_FORMHANDLE, FPDF_PAGE,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::create_transform_setters;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::bitmap::{PdfBitmap, PdfBitmapFormat, Pixels};
use crate::pdf::document::page::annotations::PdfPageAnnotations;
use crate::pdf::document::page::boundaries::PdfPageBoundaries;
use crate::pdf::document::page::index_cache::PdfPageIndexCache;
use crate::pdf::document::page::links::PdfPageLinks;
use crate::pdf::document::page::objects::PdfPageObjects;
use crate::pdf::document::page::objects::common::PdfPageObjectsCommon;
use crate::pdf::document::page::render_config::{PdfPageRenderSettings, PdfRenderConfig};
use crate::pdf::document::page::size::PdfPagePaperSize;
use crate::pdf::document::page::struct_tree::PdfStructTree;
use crate::pdf::document::page::text::PdfPageText;
use crate::pdf::font::PdfFont;
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use std::collections::{HashMap, hash_map::Entry};
use std::f32::consts::{FRAC_PI_2, PI};
use std::os::raw::{c_double, c_int};

#[cfg(doc)]
use crate::pdf::document::{PdfDocument, PdfPages};

/// The orientation of a [PdfPage].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfPageOrientation {
    Portrait,
    Landscape,
}

impl PdfPageOrientation {
    #[inline]
    pub(crate) fn from_width_and_height(width: PdfPoints, height: PdfPoints) -> Self {
        if width.value > height.value {
            PdfPageOrientation::Landscape
        } else {
            PdfPageOrientation::Portrait
        }
    }
}

/// A rotation transformation that should be applied to a [PdfPage] when it is rendered
/// into a [PdfBitmap].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfPageRenderRotation {
    None,
    Degrees90,
    Degrees180,
    Degrees270,
}

impl PdfPageRenderRotation {
    #[inline]
    pub(crate) fn from_pdfium(value: i32) -> Result<Self, PdfiumError> {
        match value {
            0 => Ok(PdfPageRenderRotation::None),
            1 => Ok(PdfPageRenderRotation::Degrees90),
            2 => Ok(PdfPageRenderRotation::Degrees180),
            3 => Ok(PdfPageRenderRotation::Degrees270),
            _ => Err(PdfiumError::UnknownBitmapRotation),
        }
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> i32 {
        match self {
            PdfPageRenderRotation::None => 0,
            PdfPageRenderRotation::Degrees90 => 1,
            PdfPageRenderRotation::Degrees180 => 2,
            PdfPageRenderRotation::Degrees270 => 3,
        }
    }

    /// Returns the equivalent clockwise rotation of this [PdfPageRenderRotation] variant, in degrees.
    #[inline]
    pub const fn as_degrees(&self) -> f32 {
        match self {
            PdfPageRenderRotation::None => 0.0,
            PdfPageRenderRotation::Degrees90 => 90.0,
            PdfPageRenderRotation::Degrees180 => 180.0,
            PdfPageRenderRotation::Degrees270 => 270.0,
        }
    }

    pub(crate) const DEGREES_90_AS_RADIANS: f32 = FRAC_PI_2;

    pub(crate) const DEGREES_180_AS_RADIANS: f32 = PI;

    pub(crate) const DEGREES_270_AS_RADIANS: f32 = FRAC_PI_2 + PI;

    /// Returns the equivalent clockwise rotation of this [PdfPageRenderRotation] variant, in radians.
    #[inline]
    pub const fn as_radians(&self) -> f32 {
        match self {
            PdfPageRenderRotation::None => 0.0,
            PdfPageRenderRotation::Degrees90 => Self::DEGREES_90_AS_RADIANS,
            PdfPageRenderRotation::Degrees180 => Self::DEGREES_180_AS_RADIANS,
            PdfPageRenderRotation::Degrees270 => Self::DEGREES_270_AS_RADIANS,
        }
    }
}

// TODO: AJRC - 19/6/23 - remove deprecated PdfBitmapRotation type in 0.9.0
// as part of tracking issue https://github.com/ajrcarey/pdfium-render/issues/36
#[deprecated(
    since = "0.8.6",
    note = "This enum has been renamed to better reflect its purpose. Use the PdfPageRenderRotation enum instead."
)]
#[doc(hidden)]
pub type PdfBitmapRotation = PdfPageRenderRotation;

/// Content regeneration strategies that instruct `pdfium-render` when, if ever, it should
/// automatically regenerate the content of a [PdfPage].
///
/// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
/// content is regenerated. If a page is reloaded or closed without regenerating the page's
/// content, any changes not applied are lost.
///
/// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
/// this removes the possibility of data loss, and ensures changes can be read back from other
/// data structures as soon as they are made. However, if many changes are made to a page at once,
/// then regenerating the content after every change is inefficient; it is faster to stage
/// all changes first, then regenerate the page's content just once. In this case,
/// changing the content regeneration strategy for a [PdfPage] can improve performance,
/// but you must be careful not to forget to commit your changes before the [PdfPage] moves out of scope.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfPageContentRegenerationStrategy {
    /// `pdfium-render` will call the [PdfPage::regenerate_content()] function on any
    /// change to this [PdfPage]. This is the default setting.
    AutomaticOnEveryChange,

    /// `pdfium-render` will call the [PdfPage::regenerate_content()] function only when
    /// this [PdfPage] is about to move out of scope.
    AutomaticOnDrop,

    /// `pdfium-render` will never call the [PdfPage::regenerate_content()] function.
    /// You must do so manually after staging your changes, or your changes will be lost
    /// when this [PdfPage] moves out of scope.
    Manual,
}

/// A single page in a `PdfDocument`.
///
/// In addition to its own intrinsic properties, a [PdfPage] serves as the entry point
/// to all object collections related to a single page in a document. These collections include:
/// * [PdfPage::annotations()], an immutable collection of all the user annotations attached to the [PdfPage].
/// * [PdfPage::annotations_mut()], a mutable collection of all the user annotations attached to the [PdfPage].
/// * [PdfPage::boundaries()], an immutable collection of the boundary boxes relating to the [PdfPage].
/// * [PdfPage::boundaries_mut()], a mutable collection of the boundary boxes relating to the [PdfPage].
/// * [PdfPage::links()], an immutable collection of the links on the [PdfPage].
/// * [PdfPage::links_mut()], a mutable collection of the links on the [PdfPage].
/// * [PdfPage::objects()], an immutable collection of all the displayable objects on the [PdfPage].
/// * [PdfPage::objects_mut()], a mutable collection of all the displayable objects on the [PdfPage].
pub struct PdfPage<'a> {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    form_handle: Option<FPDF_FORMHANDLE>,
    label: Option<String>,
    regeneration_strategy: PdfPageContentRegenerationStrategy,
    is_content_regeneration_required: bool,
    annotations: PdfPageAnnotations<'a>,
    boundaries: PdfPageBoundaries<'a>,
    links: PdfPageLinks<'a>,
    objects: PdfPageObjects<'a>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPage<'a> {
    /// The default content regeneration strategy used by `pdfium-render`. This can be overridden
    /// on a page-by-page basis using the [PdfPage::set_content_regeneration_strategy()] function.
    const DEFAULT_CONTENT_REGENERATION_STRATEGY: PdfPageContentRegenerationStrategy =
        PdfPageContentRegenerationStrategy::AutomaticOnEveryChange;

    #[inline]
    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        form_handle: Option<FPDF_FORMHANDLE>,
        label: Option<String>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        let mut result = PdfPage {
            document_handle,
            page_handle,
            form_handle,
            label,
            regeneration_strategy: PdfPageContentRegenerationStrategy::Manual,
            is_content_regeneration_required: false,
            annotations: PdfPageAnnotations::from_pdfium(document_handle, page_handle, form_handle, bindings),
            boundaries: PdfPageBoundaries::from_pdfium(page_handle, bindings),
            links: PdfPageLinks::from_pdfium(page_handle, document_handle, bindings),
            objects: PdfPageObjects::from_pdfium(document_handle, page_handle, bindings),
            bindings,
        };

        // Make sure the default content regeneration strategy is applied to child containers.

        result.set_content_regeneration_strategy(Self::DEFAULT_CONTENT_REGENERATION_STRATEGY);

        result
    }

    /// Returns the internal `FPDF_PAGE` handle for this [PdfPage].
    #[inline]
    pub(crate) fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    /// Returns the internal `FPDF_DOCUMENT` handle of the [PdfDocument] containing this [PdfPage].
    #[inline]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPage].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the label assigned to this [PdfPage], if any.
    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the width of this [PdfPage] in device-independent points.
    /// One point is 1/72 inches, roughly 0.358 mm.
    #[inline]
    pub fn width(&self) -> PdfPoints {
        PdfPoints::new(self.bindings.FPDF_GetPageWidthF(self.page_handle))
    }

    /// Returns the height of this [PdfPage] in device-independent points.
    /// One point is 1/72 inches, roughly 0.358 mm.
    #[inline]
    pub fn height(&self) -> PdfPoints {
        PdfPoints::new(self.bindings.FPDF_GetPageHeightF(self.page_handle))
    }

    /// Returns the width and height of this [PdfPage] expressed as a [PdfRect].
    #[inline]
    pub fn page_size(&self) -> PdfRect {
        PdfRect::new(PdfPoints::ZERO, PdfPoints::ZERO, self.height(), self.width())
    }

    /// Returns [PdfPageOrientation::Landscape] if the width of this [PdfPage]
    /// is greater than its height; otherwise returns [PdfPageOrientation::Portrait].
    #[inline]
    pub fn orientation(&self) -> PdfPageOrientation {
        PdfPageOrientation::from_width_and_height(self.width(), self.height())
    }

    /// Returns `true` if this [PdfPage] has orientation [PdfPageOrientation::Portrait].
    #[inline]
    pub fn is_portrait(&self) -> bool {
        self.orientation() == PdfPageOrientation::Portrait
    }

    /// Returns `true` if this [PdfPage] has orientation [PdfPageOrientation::Landscape].
    #[inline]
    pub fn is_landscape(&self) -> bool {
        self.orientation() == PdfPageOrientation::Landscape
    }

    /// Returns any intrinsic rotation encoded into this document indicating a rotation
    /// should be applied to this [PdfPage] during rendering.
    #[inline]
    pub fn rotation(&self) -> Result<PdfPageRenderRotation, PdfiumError> {
        PdfPageRenderRotation::from_pdfium(self.bindings.FPDFPage_GetRotation(self.page_handle))
    }

    /// Sets the intrinsic rotation that should be applied to this [PdfPage] during rendering.
    #[inline]
    pub fn set_rotation(&mut self, rotation: PdfPageRenderRotation) {
        self.bindings
            .FPDFPage_SetRotation(self.page_handle, rotation.as_pdfium());
    }

    /// Returns `true` if any object on the page contains transparency.
    #[inline]
    pub fn has_transparency(&self) -> bool {
        self.bindings
            .is_true(self.bindings.FPDFPage_HasTransparency(self.page_handle))
    }

    /// Returns the paper size of this [PdfPage].
    #[inline]
    pub fn paper_size(&self) -> PdfPagePaperSize {
        PdfPagePaperSize::from_points(self.width(), self.height())
    }

    /// Returns `true` if this [PdfPage] contains an embedded thumbnail.
    ///
    /// Embedded thumbnails can be generated as a courtesy by PDF generators to save PDF consumers
    /// the burden of having to render their own thumbnails on the fly. If a thumbnail for this page
    /// was not embedded at the time the document was created, one can easily be rendered using the
    /// standard rendering functions:
    ///
    /// ```
    ///     let thumbnail_desired_pixel_size = 128;
    ///
    ///     let thumbnail = page.render_with_config(
    ///         &PdfRenderConfig::thumbnail(thumbnail_desired_pixel_size)
    ///     )?; // Renders a 128 x 128 thumbnail of the page
    /// ```
    #[inline]
    pub fn has_embedded_thumbnail(&self) -> bool {
        // To determine whether the page includes a thumbnail, we ask Pdfium to return the
        // size of the thumbnail data. A non-zero value indicates a thumbnail exists.

        self.bindings
            .FPDFPage_GetRawThumbnailData(self.page_handle, std::ptr::null_mut(), 0)
            > 0
    }

    /// Returns the embedded thumbnail for this [PdfPage], if any.
    ///
    /// Embedded thumbnails can be generated as a courtesy by PDF generators to save PDF consumers
    /// the burden of having to render their own thumbnails on the fly. If a thumbnail for this page
    /// was not embedded at the time the document was created, one can easily be rendered using the
    /// standard rendering functions:
    ///
    /// ```
    ///     let thumbnail_desired_pixel_size = 128;
    ///
    ///     let thumbnail = page.render_with_config(
    ///         &PdfRenderConfig::thumbnail(thumbnail_desired_pixel_size)
    ///     )?; // Renders a 128 x 128 thumbnail of the page
    /// ```
    pub fn embedded_thumbnail(&self) -> Result<PdfBitmap<'_>, PdfiumError> {
        let thumbnail_handle = self.bindings().FPDFPage_GetThumbnailAsBitmap(self.page_handle);

        if thumbnail_handle.is_null() {
            // No thumbnail is available for this page.

            Err(PdfiumError::PageMissingEmbeddedThumbnail)
        } else {
            Ok(PdfBitmap::from_pdfium(thumbnail_handle, self.bindings))
        }
    }

    /// Returns the collection of text boxes contained within this [PdfPage].
    pub fn text(&self) -> Result<PdfPageText<'_>, PdfiumError> {
        let text_handle = self.bindings().FPDFText_LoadPage(self.page_handle);

        if text_handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfPageText::from_pdfium(text_handle, self, self.bindings))
        }
    }

    /// Returns the structure tree for this page, if the PDF is tagged.
    /// Returns `None` for untagged PDFs or if the structure tree cannot be loaded.
    ///
    /// The returned [PdfStructTree] provides access to semantic document structure
    /// including element types (paragraphs, headings, tables, etc.), alternative text,
    /// and marked content identifiers.
    pub fn struct_tree(&self) -> Option<PdfStructTree<'_>> {
        let tree_handle = self.bindings.FPDF_StructTree_GetForPage(self.page_handle);
        if tree_handle.is_null() {
            None
        } else {
            Some(PdfStructTree::from_pdfium(tree_handle, self.bindings))
        }
    }

    /// Returns an immutable collection of the annotations that have been added to this [PdfPage].
    pub fn annotations(&self) -> &PdfPageAnnotations<'a> {
        &self.annotations
    }

    /// Returns a mutable collection of the annotations that have been added to this [PdfPage].
    pub fn annotations_mut(&mut self) -> &mut PdfPageAnnotations<'a> {
        &mut self.annotations
    }

    /// Returns an immutable collection of the bounding boxes defining the extents of this [PdfPage].
    #[inline]
    pub fn boundaries(&self) -> &PdfPageBoundaries<'a> {
        &self.boundaries
    }

    /// Returns a mutable collection of the bounding boxes defining the extents of this [PdfPage].
    #[inline]
    pub fn boundaries_mut(&mut self) -> &mut PdfPageBoundaries<'a> {
        &mut self.boundaries
    }

    /// Returns an immutable collection of the links on this [PdfPage].
    #[inline]
    pub fn links(&self) -> &PdfPageLinks<'a> {
        &self.links
    }

    /// Returns a mutable collection of the links on this [PdfPage].
    #[inline]
    pub fn links_mut(&mut self) -> &mut PdfPageLinks<'a> {
        &mut self.links
    }

    /// Returns an immutable collection of all the page objects on this [PdfPage].
    pub fn objects(&self) -> &PdfPageObjects<'a> {
        &self.objects
    }

    /// Returns a mutable collection of all the page objects on this [PdfPage].
    pub fn objects_mut(&mut self) -> &mut PdfPageObjects<'a> {
        &mut self.objects
    }

    /// Returns a list of all the distinct [PdfFont] instances used by the page text objects
    /// on this [PdfPage], if any.
    pub fn fonts(&self) -> Vec<PdfFont<'_>> {
        let mut distinct_font_handles = HashMap::new();

        let mut result = Vec::new();

        for object in self.objects().iter() {
            if let Some(object) = object.as_text_object() {
                let font = object.font();

                if let Entry::Vacant(entry) = distinct_font_handles.entry(font.handle()) {
                    entry.insert(true);
                    result.push(font.handle());
                }
            }
        }

        result
            .into_iter()
            .map(|handle| PdfFont::from_pdfium(handle, self.bindings, None, false))
            .collect()
    }

    /// Converts from a bitmap coordinate system, measured in [Pixels] and with constraints
    /// and dimensions determined by the given [PdfRenderConfig] object, to the equivalent
    /// position on this page, measured in [PdfPoints].
    pub fn pixels_to_points(
        &self,
        x: Pixels,
        y: Pixels,
        config: &PdfRenderConfig,
    ) -> Result<(PdfPoints, PdfPoints), PdfiumError> {
        let mut page_x: c_double = 0.0;
        let mut page_y: c_double = 0.0;

        let settings = config.apply_to_page(self);

        if self.bindings.is_true(self.bindings.FPDF_DeviceToPage(
            self.page_handle,
            settings.clipping.left as c_int,
            settings.clipping.top as c_int,
            (settings.clipping.right - settings.clipping.left) as c_int,
            (settings.clipping.bottom - settings.clipping.top) as c_int,
            settings.rotate,
            x as c_int,
            y as c_int,
            &mut page_x,
            &mut page_y,
        )) {
            Ok((PdfPoints::new(page_x as f32), PdfPoints::new(page_y as f32)))
        } else {
            Err(PdfiumError::CoordinateConversionFunctionIndicatedError)
        }
    }

    /// Converts from the page coordinate system, measured in [PdfPoints], to the equivalent position
    /// in a bitmap coordinate system measured in [Pixels] and with constraints and dimensions
    /// defined by the given [PdfRenderConfig] object.
    pub fn points_to_pixels(
        &self,
        x: PdfPoints,
        y: PdfPoints,
        config: &PdfRenderConfig,
    ) -> Result<(Pixels, Pixels), PdfiumError> {
        let mut device_x: c_int = 0;
        let mut device_y: c_int = 0;

        let settings = config.apply_to_page(self);

        if self.bindings.is_true(self.bindings.FPDF_PageToDevice(
            self.page_handle,
            settings.clipping.left as c_int,
            settings.clipping.top as c_int,
            (settings.clipping.right - settings.clipping.left) as c_int,
            (settings.clipping.bottom - settings.clipping.top) as c_int,
            settings.rotate,
            x.value.into(),
            y.value.into(),
            &mut device_x,
            &mut device_y,
        )) {
            Ok((device_x as Pixels, device_y as Pixels))
        } else {
            Err(PdfiumError::CoordinateConversionFunctionIndicatedError)
        }
    }

    /// Renders this [PdfPage] into a [PdfBitmap] with the given pixel dimensions and page rotation.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio.
    ///
    /// See also [PdfPage::render_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    ///
    /// Each call to `PdfPage::render()` creates a new [PdfBitmap] object and allocates memory
    /// for it. To avoid repeated allocations, create a single [PdfBitmap] object
    /// using [PdfBitmap::empty()] and reuse it across multiple calls to [PdfPage::render_into_bitmap()].
    pub fn render(
        &self,
        width: Pixels,
        height: Pixels,
        rotation: Option<PdfPageRenderRotation>,
    ) -> Result<PdfBitmap<'_>, PdfiumError> {
        let mut bitmap = PdfBitmap::empty(width, height, PdfBitmapFormat::default(), self.bindings)?;

        let mut config = PdfRenderConfig::new().set_target_width(width).set_target_height(height);

        if let Some(rotation) = rotation {
            config = config.rotate(rotation, true);
        }

        self.render_into_bitmap_with_config(&mut bitmap, &config)?;

        Ok(bitmap)
    }

    /// Renders this [PdfPage] into a new [PdfBitmap] using pixel dimensions, page rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    ///
    /// Each call to `PdfPage::render_with_config()` creates a new [PdfBitmap] object and
    /// allocates memory for it. To avoid repeated allocations, create a single [PdfBitmap] object
    /// using [PdfBitmap::empty()] and reuse it across multiple calls to
    /// [PdfPage::render_into_bitmap_with_config()].
    pub fn render_with_config(&self, config: &PdfRenderConfig) -> Result<PdfBitmap<'_>, PdfiumError> {
        let settings = config.apply_to_page(self);

        let mut bitmap = PdfBitmap::empty(
            settings.width as Pixels,
            settings.height as Pixels,
            PdfBitmapFormat::from_pdfium(settings.format as u32).unwrap_or_else(|_| PdfBitmapFormat::default()),
            self.bindings,
        )?;

        self.render_into_bitmap_with_settings(&mut bitmap, settings)?;

        Ok(bitmap)
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using the given pixel dimensions
    /// and page rotation.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio. The size of the buffer backing the given bitmap
    /// must be sufficiently large to hold the rendered image or an error will be returned.
    ///
    /// See also [PdfPage::render_into_bitmap_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    pub fn render_into_bitmap(
        &self,
        bitmap: &mut PdfBitmap,
        width: Pixels,
        height: Pixels,
        rotation: Option<PdfPageRenderRotation>,
    ) -> Result<(), PdfiumError> {
        let mut config = PdfRenderConfig::new().set_target_width(width).set_target_height(height);

        if let Some(rotation) = rotation {
            config = config.rotate(rotation, true);
        }

        self.render_into_bitmap_with_config(bitmap, &config)
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using pixel dimensions, page rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    ///
    /// The size of the buffer backing the given bitmap must be sufficiently large to hold the
    /// rendered image or an error will be returned.
    #[inline]
    pub fn render_into_bitmap_with_config(
        &self,
        bitmap: &mut PdfBitmap,
        config: &PdfRenderConfig,
    ) -> Result<(), PdfiumError> {
        self.render_into_bitmap_with_settings(bitmap, config.apply_to_page(self))
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using the given [PdfRenderSettings].
    /// The size of the buffer backing the given bitmap must be sufficiently large to hold
    /// the rendered image or an error will be returned.
    pub(crate) fn render_into_bitmap_with_settings(
        &self,
        bitmap: &mut PdfBitmap,
        settings: PdfPageRenderSettings,
    ) -> Result<(), PdfiumError> {
        let bitmap_handle = bitmap.handle();

        if settings.do_clear_bitmap_before_rendering {
            // Clear the bitmap buffer by setting every pixel to a known color.

            self.bindings().FPDFBitmap_FillRect(
                bitmap_handle,
                0,
                0,
                settings.width,
                settings.height,
                settings.clear_color,
            );
        }

        if settings.do_render_form_data {
            // Render the PDF page into the bitmap buffer, ignoring any custom transformation matrix.
            // (Custom transforms cannot be applied to the rendering of form fields.)

            self.bindings.FPDF_RenderPageBitmap(
                bitmap_handle,
                self.page_handle,
                0,
                0,
                settings.width,
                settings.height,
                settings.rotate,
                settings.render_flags,
            );

            if let Some(form_handle) = self.form_handle {
                // Render user-supplied form data, if any, as an overlay on top of the page.

                if let Some(form_field_highlight) = settings.form_field_highlight.as_ref() {
                    for (form_field_type, (color, alpha)) in form_field_highlight.iter() {
                        self.bindings
                            .FPDF_SetFormFieldHighlightColor(form_handle, *form_field_type, *color);

                        self.bindings.FPDF_SetFormFieldHighlightAlpha(form_handle, *alpha);
                    }
                }

                self.bindings.FPDF_FFLDraw(
                    form_handle,
                    bitmap_handle,
                    self.page_handle,
                    0,
                    0,
                    settings.width,
                    settings.height,
                    settings.rotate,
                    settings.render_flags,
                );
            }
        } else {
            // Render the PDF page into the bitmap buffer, applying any custom transformation matrix.

            self.bindings.FPDF_RenderPageBitmapWithMatrix(
                bitmap_handle,
                self.page_handle,
                &settings.matrix,
                &settings.clipping,
                settings.render_flags,
            );
        }

        bitmap.set_byte_order_from_render_settings(&settings);

        Ok(())
    }

    // TODO: AJRC - 29/7/22 - remove deprecated PdfPage::get_bitmap_*() functions in 0.9.0
    // as part of tracking issue https://github.com/ajrcarey/pdfium-render/issues/36
    /// Renders this [PdfPage] into a new [PdfBitmap] using pixel dimensions, rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    #[deprecated(
        since = "0.7.12",
        note = "This function has been renamed to better reflect its purpose. Use the PdfPage::render_with_config() function instead."
    )]
    #[doc(hidden)]
    #[inline]
    pub fn get_bitmap_with_config(&self, config: &PdfRenderConfig) -> Result<PdfBitmap<'_>, PdfiumError> {
        self.render_with_config(config)
    }

    /// Renders this [PdfPage] into a new [PdfBitmap] with the given pixel dimensions and
    /// rotation setting.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio.
    ///
    /// See also [PdfPage::render_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    #[deprecated(
        since = "0.7.12",
        note = "This function has been renamed to better reflect its purpose. Use the PdfPage::render() function instead."
    )]
    #[doc(hidden)]
    pub fn get_bitmap(
        &self,
        width: Pixels,
        height: Pixels,
        rotation: Option<PdfPageRenderRotation>,
    ) -> Result<PdfBitmap<'_>, PdfiumError> {
        self.render(width, height, rotation)
    }

    /// Applies the given transformation, expressed as six values representing the six configurable
    /// elements of a nine-element 3x3 PDF transformation matrix, to the objects on this [PdfPage],
    /// restricting the effects of the transformation to the given clipping rectangle.
    ///
    /// To move, scale, rotate, or skew the objects on this [PdfPage], consider using one or more of
    /// the following functions. Internally they all use [PdfPage::transform()], but are
    /// probably easier to use (and certainly clearer in their intent) in most situations.
    ///
    /// * [PdfPage::translate()]: changes the position of each object on this [PdfPage].
    /// * [PdfPage::scale()]: changes the size of each object on this [PdfPage].
    /// * [PdfPage::flip_horizontally()]: flips each object on this [PdfPage] horizontally around
    ///   the page origin point.
    /// * [PdfPage::flip_vertically()]: flips each object on this [PdfPage] vertically around
    ///   the page origin point.
    /// * [PdfPage::rotate_clockwise_degrees()], [PdfPage::rotate_counter_clockwise_degrees()],
    ///   [PdfPage::rotate_clockwise_radians()], [PdfPage::rotate_counter_clockwise_radians()]:
    ///   rotates each object on this [PdfPage] around its origin.
    /// * [PdfPage::skew_degrees()], [PdfPage::skew_radians()]: skews each object
    ///   on this [PdfPage] relative to its axes.
    ///
    /// **The order in which transformations are applied is significant.**
    /// For example, the result of rotating _then_ translating an object may be vastly different
    /// from translating _then_ rotating the same object.
    ///
    /// An overview of PDF transformation matrices can be found in the PDF Reference Manual
    /// version 1.7 on page 204; a detailed description can be found in section 4.2.3 on page 207.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn transform_with_clip(
        &mut self,
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
        clip: PdfRect,
    ) -> Result<(), PdfiumError> {
        self.apply_matrix_with_clip(PdfMatrix::new(a, b, c, d, e, f), clip)
    }

    // TODO: AJRC - 3/11/23 - remove deprecated PdfPage::set_matrix_with_clip() function in 0.9.0
    // as part of tracking issue https://github.com/ajrcarey/pdfium-render/issues/36
    #[deprecated(
        since = "0.8.15",
        note = "This function has been renamed to better reflect its behaviour. Use the apply_matrix_with_clip() function instead."
    )]
    #[doc(hidden)]
    #[inline]
    pub fn set_matrix_with_clip(&mut self, matrix: PdfMatrix, clip: PdfRect) -> Result<(), PdfiumError> {
        self.apply_matrix_with_clip(matrix, clip)
    }

    /// Applies the given transformation, expressed as a [PdfMatrix], to this [PdfPage],
    /// restricting the effects of the transformation matrix to the given clipping rectangle.
    pub fn apply_matrix_with_clip(&mut self, matrix: PdfMatrix, clip: PdfRect) -> Result<(), PdfiumError> {
        if self.bindings().is_true(self.bindings().FPDFPage_TransFormWithClip(
            self.page_handle,
            &matrix.as_pdfium(),
            &clip.as_pdfium(),
        )) {
            // A probable bug in Pdfium means we must reload the page in order for the
            // transformation to take effect. For more information, see:
            // https://github.com/ajrcarey/pdfium-render/issues/93

            self.reload_in_place();
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "each object on this [PdfPage]",
        "each object on this [PdfPage].",
        "each object on this [PdfPage],",
        "",
        pub(self)
    ); // pub(self) visibility for the generated reset_matrix() function will effectively make it
    // private. This is what we want, since Pdfium does not expose a function to directly set
    // the transformation matrix of a page.

    #[inline]
    fn transform_impl(
        &mut self,
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
    ) -> Result<(), PdfiumError> {
        self.transform_with_clip(a, b, c, d, e, f, PdfRect::MAX)
    }

    // The reset_matrix() function created by the create_transform_setters!() macro
    // is not publicly visible, so this function should never be called.
    #[allow(dead_code)]
    fn reset_matrix_impl(&mut self, _: PdfMatrix) -> Result<(), PdfiumError> {
        unreachable!();
    }

    /// Flattens all annotations and form fields on this [PdfPage] into the page contents.
    pub fn flatten(&mut self) -> Result<(), PdfiumError> {
        // TODO: AJRC - 28/5/22 - consider allowing the caller to set the FLAT_NORMALDISPLAY or FLAT_PRINT flag.
        let flag = FLAT_PRINT;

        match self.bindings().FPDFPage_Flatten(self.page_handle, flag as c_int) as u32 {
            FLATTEN_SUCCESS => {
                self.regenerate_content()?;

                // As noted at https://bugs.chromium.org/p/pdfium/issues/detail?id=2055,
                // FPDFPage_Flatten() updates the underlying dictionaries and content streams for
                // the page, but does not update the FPDF_Page structure. We must reload the
                // page for the effects of the flatten operation to be visible. For more information, see:
                // https://github.com/ajrcarey/pdfium-render/issues/140

                self.reload_in_place();
                Ok(())
            }
            FLATTEN_NOTHINGTODO => Ok(()),
            FLATTEN_FAIL => Err(PdfiumError::PageFlattenFailure),
            _ => Err(PdfiumError::PageFlattenFailure),
        }
    }

    /// Deletes this [PdfPage] from its containing `PdfPages` collection, consuming this [PdfPage].
    pub fn delete(self) -> Result<(), PdfiumError> {
        let index = PdfPageIndexCache::get_index_for_page(self.document_handle, self.page_handle)
            .ok_or(PdfiumError::SourcePageIndexNotInCache)?;

        self.bindings.FPDFPage_Delete(self.document_handle, index as c_int);

        PdfPageIndexCache::delete_pages_at_index(self.document_handle, index, 1);

        Ok(())
    }

    /// Returns the strategy used by `pdfium-render` to regenerate the content of a [PdfPage].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying `PdfDocument` until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn content_regeneration_strategy(&self) -> PdfPageContentRegenerationStrategy {
        self.regeneration_strategy
    }

    /// Sets the strategy used by `pdfium-render` to regenerate the content of a [PdfPage].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying `PdfDocument` until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn set_content_regeneration_strategy(&mut self, strategy: PdfPageContentRegenerationStrategy) {
        self.regeneration_strategy = strategy;

        if let Some(index) = PdfPageIndexCache::get_index_for_page(self.document_handle(), self.page_handle()) {
            PdfPageIndexCache::cache_props_for_page(self.document_handle(), self.page_handle(), index, strategy);
        }
    }

    /// Commits any staged but unsaved changes to this [PdfPage] to the underlying [PdfDocument].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn regenerate_content(&mut self) -> Result<(), PdfiumError> {
        // This is a publicly-visible wrapper for the private regenerate_content_immut() function.
        // It is only available to callers who hold a mutable reference to the page.

        self.regenerate_content_immut()
    }

    /// Commits any staged but unsaved changes to this [PdfPage] to the underlying [PdfDocument].
    #[inline]
    pub(crate) fn regenerate_content_immut(&self) -> Result<(), PdfiumError> {
        Self::regenerate_content_immut_for_handle(self.page_handle, self.bindings)
    }

    /// Commits any staged but unsaved changes to the page identified by the given internal
    /// `FPDF_PAGE` handle to the underlying [PdfDocument] containing that page.
    ///
    /// This function always commits changes, irrespective of the page's currently set
    /// content regeneration strategy.
    pub(crate) fn regenerate_content_immut_for_handle(
        page: FPDF_PAGE,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<(), PdfiumError> {
        if bindings.is_true(bindings.FPDFPage_GenerateContent(page)) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Reloads the page transparently to any caller, forcing a refresh of all page data structures.
    /// This will replace this page's `FPDF_PAGE` handle. The page index cache will be updated.
    fn reload_in_place(&mut self) {
        if let Some(page_index) = PdfPageIndexCache::get_index_for_page(self.document_handle, self.page_handle) {
            self.drop_impl();

            self.page_handle = self.bindings.FPDF_LoadPage(self.document_handle, page_index as c_int);

            PdfPageIndexCache::cache_props_for_page(
                self.document_handle,
                self.page_handle,
                page_index,
                self.content_regeneration_strategy(),
            );
        }
    }

    /// Drops the page by calling `FPDF_ClosePage()`, freeing held memory. This will invalidate
    /// this page's `FPDF_PAGE` handle. The page index cache will be updated.
    fn drop_impl(&mut self) {
        if self.regeneration_strategy != PdfPageContentRegenerationStrategy::Manual
            && self.is_content_regeneration_required
        {
            // Regenerate page content now if necessary, before the PdfPage moves out of scope.

            let result = self.regenerate_content();

            debug_assert!(result.is_ok());
        }

        self.bindings.FPDF_ClosePage(self.page_handle);

        PdfPageIndexCache::remove_index_for_page(self.document_handle, self.page_handle);
    }
}

impl<'a> Drop for PdfPage<'a> {
    /// Closes this [PdfPage], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        self.drop_impl();
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};
    use image_025::{GenericImageView, ImageFormat};

    #[test]
    fn test_page_rendering_reusing_bitmap() -> Result<(), PdfiumError> {
        // Renders each page in the given test PDF file to a separate JPEG file
        // by re-using the same bitmap buffer for each render.

        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file(&test_fixture_path("export-test.pdf"), None)?;

        let render_config = PdfRenderConfig::new()
            .set_target_width(2000)
            .set_maximum_height(2000)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

        let mut bitmap = PdfBitmap::empty(2500, 2500, PdfBitmapFormat::default(), pdfium.bindings())?;

        for (index, page) in document.pages().iter().enumerate() {
            page.render_into_bitmap_with_config(&mut bitmap, &render_config)?; // Re-uses the same bitmap for rendering each page.

            bitmap
                .as_image()?
                .into_rgb8()
                .save_with_format(format!("test-page-{}.jpg", index), ImageFormat::Jpeg)
                .map_err(|_| PdfiumError::ImageError)?;
        }

        Ok(())
    }

    #[test]
    fn test_rendered_image_dimension() -> Result<(), PdfiumError> {
        // Checks that downscaled dimensions are rounded correctly during page rendering.
        // See: https://github.com/ajrcarey/pdfium-render/pull/87

        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file(&test_fixture_path("dimensions-test.pdf"), None)?;

        let render_config = PdfRenderConfig::new().set_target_width(500).set_maximum_height(500);

        for page in document.pages().iter() {
            let rendered_page = page.render_with_config(&render_config)?.as_image()?;

            let (width, _height) = rendered_page.dimensions();

            assert_eq!(width, 500);
        }

        Ok(())
    }
}
