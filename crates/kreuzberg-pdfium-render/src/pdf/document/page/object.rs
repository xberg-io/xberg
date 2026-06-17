//! Defines the [PdfPageObject] enum, exposing functionality related to a single renderable page object.

pub(crate) mod content_mark;
pub(crate) mod content_marks;
pub(crate) mod group;
pub(crate) mod image;
pub(crate) mod ownership;
pub(crate) mod path;
pub(crate) mod private; // Keep private so that the PdfPageObjectPrivate trait is not exposed.
pub(crate) mod shading;
pub(crate) mod text;
pub(crate) mod unsupported;
pub(crate) mod x_object_form;

use crate::bindgen::{
    FPDF_DOCUMENT, FPDF_LINECAP_BUTT, FPDF_LINECAP_PROJECTING_SQUARE, FPDF_LINECAP_ROUND, FPDF_LINEJOIN_BEVEL,
    FPDF_LINEJOIN_MITER, FPDF_LINEJOIN_ROUND, FPDF_PAGEOBJ_FORM, FPDF_PAGEOBJ_IMAGE, FPDF_PAGEOBJ_PATH,
    FPDF_PAGEOBJ_SHADING, FPDF_PAGEOBJ_TEXT, FPDF_PAGEOBJ_UNKNOWN, FPDF_PAGEOBJECT,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::color::PdfColor;
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::annotation::objects::PdfPageAnnotationObjects;
use crate::pdf::document::page::annotation::private::internal::PdfPageAnnotationPrivate;
use crate::pdf::document::page::annotation::{PdfPageAnnotation, PdfPageAnnotationCommon};
use crate::pdf::document::page::object::content_marks::PdfPageObjectContentMarks;
use crate::pdf::document::page::object::image::PdfPageImageObject;
use crate::pdf::document::page::object::path::PdfPagePathObject;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::shading::PdfPageShadingObject;
use crate::pdf::document::page::object::text::PdfPageTextObject;
use crate::pdf::document::page::object::unsupported::PdfPageUnsupportedObject;
use crate::pdf::document::page::object::x_object_form::PdfPageXObjectFormObject;
use crate::pdf::document::page::objects::PdfPageObjects;
use crate::pdf::document::page::{PdfPage, PdfPageObjectOwnership};
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::path::clip_path::PdfClipPath;
use crate::pdf::points::PdfPoints;
use crate::pdf::quad_points::PdfQuadPoints;
use crate::pdf::rect::PdfRect;
use crate::{create_transform_getters, create_transform_setters};
use std::convert::TryInto;
use std::os::raw::{c_int, c_uint};

use crate::error::PdfiumInternalError;

/// The type of a single renderable [PdfPageObject].
///
/// Note that Pdfium does not support or recognize all PDF page object types. For instance,
/// Pdfium does not currently support or recognize all types of External Object ("XObject")
/// page object types supported by Adobe Acrobat and Foxit's commercial PDF SDK. In these cases,
/// Pdfium will return [PdfPageObjectType::Unsupported].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub enum PdfPageObjectType {
    /// Any External Object ("XObject") page object type not directly supported by Pdfium.
    Unsupported = FPDF_PAGEOBJ_UNKNOWN as isize,

    /// A page object containing renderable text.
    Text = FPDF_PAGEOBJ_TEXT as isize,

    /// A page object containing a renderable vector path.
    Path = FPDF_PAGEOBJ_PATH as isize,

    /// A page object containing a renderable bitmapped image.
    Image = FPDF_PAGEOBJ_IMAGE as isize,

    /// A page object containing a renderable geometric shape whose color is an arbitrary
    /// function of position within the shape.
    Shading = FPDF_PAGEOBJ_SHADING as isize,

    /// A page object containing a content stream that itself may consist of multiple other page
    /// objects. When this page object is rendered, it renders all its constituent page objects,
    /// effectively serving as a template or stamping object.
    ///
    /// Despite the page object name including "form", this page object type bears no relation
    /// to an interactive form containing form fields.
    XObjectForm = FPDF_PAGEOBJ_FORM as isize,
}

impl PdfPageObjectType {
    pub(crate) fn from_pdfium(value: u32) -> Result<PdfPageObjectType, PdfiumError> {
        match value {
            FPDF_PAGEOBJ_UNKNOWN => Ok(PdfPageObjectType::Unsupported),
            FPDF_PAGEOBJ_TEXT => Ok(PdfPageObjectType::Text),
            FPDF_PAGEOBJ_PATH => Ok(PdfPageObjectType::Path),
            FPDF_PAGEOBJ_IMAGE => Ok(PdfPageObjectType::Image),
            FPDF_PAGEOBJ_SHADING => Ok(PdfPageObjectType::Shading),
            FPDF_PAGEOBJ_FORM => Ok(PdfPageObjectType::XObjectForm),
            _ => Err(PdfiumError::UnknownPdfPageObjectType),
        }
    }
}

/// The method used to combine overlapping colors when painting one [PdfPageObject] on top of
/// another.
///
/// The color being newly painted is the source color; the existing color being painted onto is the
/// backdrop color.
///
/// A formal definition of these blend modes can be found in Section 7.2.4 of
/// the PDF Reference Manual, version 1.7, on page 520.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPageObjectBlendMode {
    /// Selects the source color, ignoring the backdrop.
    Normal,

    /// Multiplies the backdrop and source color values. The resulting color is always at least
    /// as dark as either of the two constituent colors. Multiplying any color with black
    /// produces black; multiplying with white leaves the original color unchanged.
    /// Painting successive overlapping objects with a color other than black or white
    /// produces progressively darker colors.
    Multiply,

    /// Multiplies the complements of the backdrop and source color values, then complements
    /// the result.

    /// The result color is always at least as light as either of the two constituent colors.
    /// Screening any color with white produces white; screening with black leaves the original
    /// color unchanged. The effect is similar to projecting multiple photographic slides
    /// simultaneously onto a single screen.
    Screen,

    /// Multiplies or screens the colors, depending on the backdrop color value. Source colors
    /// overlay the backdrop while preserving its highlights and shadows. The backdrop color is
    /// not replaced but is mixed with the source color to reflect the lightness or darkness of
    /// the backdrop.
    Overlay,

    /// Selects the darker of the backdrop and source colors. The backdrop is replaced with the
    /// source where the source is darker; otherwise, it is left unchanged.
    Darken,

    /// Selects the lighter of the backdrop and source colors. The backdrop is replaced with the
    /// source where the source is lighter; otherwise, it is left unchanged.
    Lighten,

    /// Brightens the backdrop color to reflect the source color. Painting with black produces no
    /// changes.
    ColorDodge,

    /// Darkens the backdrop color to reflect the source color. Painting with white produces no
    /// change.
    ColorBurn,

    /// Multiplies or screens the colors, depending on the source color value. The effect is similar
    /// to shining a harsh spotlight on the backdrop.
    HardLight,

    /// Darkens or lightens the colors, depending on the source color value. The effect is similar
    /// to shining a diffused spotlight on the backdrop.
    SoftLight,

    /// Subtracts the darker of the two constituent colors from the lighter color.
    /// Painting with white inverts the backdrop color; painting with black produces no change.
    Difference,

    /// Produces an effect similar to that of the Difference mode but lower in contrast.
    /// Painting with white inverts the backdrop color; painting with black produces no change.
    Exclusion,

    /// Preserves the luminosity of the backdrop color while adopting the hue and saturation
    /// of the source color.
    HSLColor,

    /// Preserves the luminosity and saturation of the backdrop color while adopting the hue
    /// of the source color.
    HSLHue,

    /// Preserves the hue and saturation of the backdrop color while adopting the luminosity
    /// of the source color.
    HSLLuminosity,

    /// Preserves the luminosity and hue of the backdrop color while adopting the saturation
    /// of the source color.
    HSLSaturation,
}

impl PdfPageObjectBlendMode {
    pub(crate) fn as_pdfium(&self) -> &str {
        match self {
            PdfPageObjectBlendMode::HSLColor => "Color",
            PdfPageObjectBlendMode::ColorBurn => "ColorBurn",
            PdfPageObjectBlendMode::ColorDodge => "ColorDodge",
            PdfPageObjectBlendMode::Darken => "Darken",
            PdfPageObjectBlendMode::Difference => "Difference",
            PdfPageObjectBlendMode::Exclusion => "Exclusion",
            PdfPageObjectBlendMode::HardLight => "HardLight",
            PdfPageObjectBlendMode::HSLHue => "Hue",
            PdfPageObjectBlendMode::Lighten => "Lighten",
            PdfPageObjectBlendMode::HSLLuminosity => "Luminosity",
            PdfPageObjectBlendMode::Multiply => "Multiply",
            PdfPageObjectBlendMode::Normal => "Normal",
            PdfPageObjectBlendMode::Overlay => "Overlay",
            PdfPageObjectBlendMode::HSLSaturation => "Saturation",
            PdfPageObjectBlendMode::Screen => "Screen",
            PdfPageObjectBlendMode::SoftLight => "SoftLight",
        }
    }
}

/// The shape that should be used at the corners of stroked paths.
///
/// Join styles are significant only at points where consecutive segments of a path
/// connect at an angle; segments that meet or intersect fortuitously receive no special treatment.
///
/// A formal definition of these styles can be found in Section 4.3.2 of
/// the PDF Reference Manual, version 1.7, on page 216.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPageObjectLineJoin {
    /// The outer edges of the strokes for the two path segments are extended
    /// until they meet at an angle, as in a picture frame. If the segments meet at too
    /// sharp an angle, a bevel join is used instead.
    Miter = FPDF_LINEJOIN_MITER as isize,

    /// An arc of a circle with a diameter equal to the line width is drawn
    /// around the point where the two path segments meet, connecting the outer edges of
    /// the strokes for the two segments. This pie-slice-shaped figure is filled in,
    /// producing a rounded corner.
    Round = FPDF_LINEJOIN_ROUND as isize,

    /// The two path segments are finished with butt caps and the resulting notch
    /// beyond the ends of the segments is filled with a triangle.
    Bevel = FPDF_LINEJOIN_BEVEL as isize,
}

impl PdfPageObjectLineJoin {
    pub(crate) fn from_pdfium(value: c_int) -> Option<Self> {
        match value as u32 {
            FPDF_LINEJOIN_MITER => Some(Self::Miter),
            FPDF_LINEJOIN_ROUND => Some(Self::Round),
            FPDF_LINEJOIN_BEVEL => Some(Self::Bevel),
            _ => None,
        }
    }

    pub(crate) fn as_pdfium(&self) -> u32 {
        match self {
            PdfPageObjectLineJoin::Miter => FPDF_LINEJOIN_MITER,
            PdfPageObjectLineJoin::Round => FPDF_LINEJOIN_ROUND,
            PdfPageObjectLineJoin::Bevel => FPDF_LINEJOIN_BEVEL,
        }
    }
}

/// The shape that should be used at the ends of open stroked paths.
///
/// A formal definition of these styles can be found in Section 4.3.2 of
/// the PDF Reference Manual, version 1.7, on page 216.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPageObjectLineCap {
    /// The stroke is squared off at the endpoint of the path. There is no
    /// projection beyond the end of the path.
    Butt = FPDF_LINECAP_BUTT as isize,

    /// A semicircular arc with a diameter equal to the line width is
    /// drawn around the endpoint and filled in.
    Round = FPDF_LINECAP_ROUND as isize,

    /// The stroke continues beyond the endpoint of the path
    /// for a distance equal to half the line width and is squared off.
    Square = FPDF_LINECAP_PROJECTING_SQUARE as isize,
}

impl PdfPageObjectLineCap {
    pub(crate) fn from_pdfium(value: c_int) -> Option<Self> {
        match value as u32 {
            FPDF_LINECAP_BUTT => Some(Self::Butt),
            FPDF_LINECAP_ROUND => Some(Self::Round),
            FPDF_LINECAP_PROJECTING_SQUARE => Some(Self::Square),
            _ => None,
        }
    }

    pub(crate) fn as_pdfium(&self) -> u32 {
        match self {
            PdfPageObjectLineCap::Butt => FPDF_LINECAP_BUTT,
            PdfPageObjectLineCap::Round => FPDF_LINECAP_ROUND,
            PdfPageObjectLineCap::Square => FPDF_LINECAP_PROJECTING_SQUARE,
        }
    }
}

/// A single renderable object on a [PdfPage].
pub enum PdfPageObject<'a> {
    /// A page object containing renderable text.
    Text(PdfPageTextObject<'a>),

    /// A page object containing a renderable vector path.
    Path(PdfPagePathObject<'a>),

    /// A page object containing a renderable bitmapped image.
    Image(PdfPageImageObject<'a>),

    /// A page object containing a renderable geometric shape whose color is an arbitrary
    /// function of position within the shape.
    Shading(PdfPageShadingObject<'a>),

    /// A page object containing a content stream that itself may consist of multiple other page
    /// objects. When this page object is rendered, it renders all its constituent page objects,
    /// effectively serving as a template or stamping object.
    ///
    /// Despite the page object name including "form", this page object type bears no relation
    /// to an interactive form containing form fields.
    XObjectForm(PdfPageXObjectFormObject<'a>),

    /// Any External Object ("XObject") page object type not directly supported by Pdfium.
    ///
    /// Common properties shared by all [PdfPageObject] types can still be accessed for
    /// page objects not recognized by Pdfium, but object-specific functionality
    /// will be unavailable.
    Unsupported(PdfPageUnsupportedObject<'a>),
}

impl<'a> PdfPageObject<'a> {
    pub(crate) fn from_pdfium(
        object_handle: FPDF_PAGEOBJECT,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        match PdfPageObjectType::from_pdfium(bindings.FPDFPageObj_GetType(object_handle) as u32)
            .unwrap_or(PdfPageObjectType::Unsupported)
        {
            PdfPageObjectType::Unsupported => PdfPageObject::Unsupported(PdfPageUnsupportedObject::from_pdfium(
                object_handle,
                ownership,
                bindings,
            )),
            PdfPageObjectType::Text => {
                PdfPageObject::Text(PdfPageTextObject::from_pdfium(object_handle, ownership, bindings))
            }
            PdfPageObjectType::Path => {
                PdfPageObject::Path(PdfPagePathObject::from_pdfium(object_handle, ownership, bindings))
            }
            PdfPageObjectType::Image => {
                PdfPageObject::Image(PdfPageImageObject::from_pdfium(object_handle, ownership, bindings))
            }
            PdfPageObjectType::Shading => {
                PdfPageObject::Shading(PdfPageShadingObject::from_pdfium(object_handle, ownership, bindings))
            }
            PdfPageObjectType::XObjectForm => PdfPageObject::XObjectForm(PdfPageXObjectFormObject::from_pdfium(
                object_handle,
                ownership,
                bindings,
            )),
        }
    }

    #[inline]
    pub(crate) fn unwrap_as_trait(&self) -> &dyn PdfPageObjectPrivate<'a> {
        match self {
            PdfPageObject::Text(object) => object,
            PdfPageObject::Path(object) => object,
            PdfPageObject::Image(object) => object,
            PdfPageObject::Shading(object) => object,
            PdfPageObject::XObjectForm(object) => object,
            PdfPageObject::Unsupported(object) => object,
        }
    }

    #[inline]
    pub(crate) fn unwrap_as_trait_mut(&mut self) -> &mut dyn PdfPageObjectPrivate<'a> {
        match self {
            PdfPageObject::Text(object) => object,
            PdfPageObject::Path(object) => object,
            PdfPageObject::Image(object) => object,
            PdfPageObject::Shading(object) => object,
            PdfPageObject::XObjectForm(object) => object,
            PdfPageObject::Unsupported(object) => object,
        }
    }

    /// The object type of this [PdfPageObject].
    ///
    /// Note that Pdfium does not support or recognize all PDF page object types. For instance,
    /// Pdfium does not currently support or recognize the External Object ("XObject") page object
    /// type supported by Adobe Acrobat and Foxit's commercial PDF SDK. In these cases, Pdfium
    /// will return `PdfPageObjectType::Unsupported`.
    #[inline]
    pub fn object_type(&self) -> PdfPageObjectType {
        match self {
            PdfPageObject::Text(_) => PdfPageObjectType::Text,
            PdfPageObject::Path(_) => PdfPageObjectType::Path,
            PdfPageObject::Image(_) => PdfPageObjectType::Image,
            PdfPageObject::Shading(_) => PdfPageObjectType::Shading,
            PdfPageObject::XObjectForm(_) => PdfPageObjectType::XObjectForm,
            PdfPageObject::Unsupported(_) => PdfPageObjectType::Unsupported,
        }
    }

    /// Returns `true` if this [PdfPageObject] has an object type other than [PdfPageObjectType::Unsupported].
    ///
    /// The [PdfPageObject::as_text_object()], [PdfPageObject::as_path_object()], [PdfPageObject::as_image_object()],
    /// [PdfPageObject::as_shading_object()], and [PdfPageObject::as_x_object_form_object()] functions
    /// can be used to access properties and functions pertaining to a specific page object type.
    #[inline]
    pub fn is_supported(&self) -> bool {
        !self.is_unsupported()
    }

    /// Returns `true` if this [PdfPageObject] has an object type of [PdfPageObjectType::Unsupported].
    ///
    /// Common properties shared by all [PdfPageObject] types can still be accessed for
    /// page objects not recognized by Pdfium, but object-specific functionality
    /// will be unavailable.
    #[inline]
    pub fn is_unsupported(&self) -> bool {
        self.object_type() == PdfPageObjectType::Unsupported
    }

    /// Returns an immutable reference to the underlying [PdfPageTextObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Text].
    #[inline]
    pub fn as_text_object(&self) -> Option<&PdfPageTextObject<'_>> {
        match self {
            PdfPageObject::Text(object) => Some(object),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageTextObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Text].
    #[inline]
    pub fn as_text_object_mut(&mut self) -> Option<&mut PdfPageTextObject<'a>> {
        match self {
            PdfPageObject::Text(object) => Some(object),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPagePathObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Path].
    #[inline]
    pub fn as_path_object(&self) -> Option<&PdfPagePathObject<'_>> {
        match self {
            PdfPageObject::Path(object) => Some(object),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPagePathObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Path].
    #[inline]
    pub fn as_path_object_mut(&mut self) -> Option<&mut PdfPagePathObject<'a>> {
        match self {
            PdfPageObject::Path(object) => Some(object),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageImageObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Image].
    #[inline]
    pub fn as_image_object(&self) -> Option<&PdfPageImageObject<'_>> {
        match self {
            PdfPageObject::Image(object) => Some(object),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageImageObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Image].
    #[inline]
    pub fn as_image_object_mut(&mut self) -> Option<&mut PdfPageImageObject<'a>> {
        match self {
            PdfPageObject::Image(object) => Some(object),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageShadingObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Shading].
    #[inline]
    pub fn as_shading_object(&self) -> Option<&PdfPageShadingObject<'_>> {
        match self {
            PdfPageObject::Shading(object) => Some(object),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageShadingObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::Shading].
    #[inline]
    pub fn as_shading_object_mut(&mut self) -> Option<&mut PdfPageShadingObject<'a>> {
        match self {
            PdfPageObject::Shading(object) => Some(object),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfPageXObjectFormObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::XObjectForm].
    #[inline]
    pub fn as_x_object_form_object(&self) -> Option<&PdfPageXObjectFormObject<'_>> {
        match self {
            PdfPageObject::XObjectForm(object) => Some(object),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfPageXObjectFormObject] for this [PdfPageObject],
    /// if this page object has an object type of [PdfPageObjectType::XObjectForm].
    #[inline]
    pub fn as_x_object_form_object_mut(&mut self) -> Option<&mut PdfPageXObjectFormObject<'a>> {
        match self {
            PdfPageObject::XObjectForm(object) => Some(object),
            _ => None,
        }
    }

    /// Returns the clip path for this object, if any.
    pub fn get_clip_path(&self) -> Option<PdfClipPath<'_>> {
        let path_handle = self.bindings().FPDFPageObj_GetClipPath(self.object_handle());

        if path_handle.is_null() {
            return None;
        }

        Some(PdfClipPath::from_pdfium(
            path_handle,
            *self.ownership(),
            self.bindings(),
        ))
    }

    /// Marks this [PdfPageObject] as active on its containing page. All page objects
    /// start in the active state by default.
    pub fn set_active(&mut self) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetIsActive(self.object_handle(), self.bindings().TRUE()),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns `true` if this [PdfPageObject] is marked as active on its containing page.
    pub fn is_active(&self) -> Result<bool, PdfiumError> {
        let mut result = self.bindings().FALSE();

        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_GetIsActive(self.object_handle(), &mut result),
        ) {
            Ok(self.bindings().is_true(result))
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Marks this [PdfPageObject] as inactive on its containing page. The page object will
    /// be treated as if it were not in the document, even though it exists internally.
    pub fn set_inactive(&mut self) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetIsActive(self.object_handle(), self.bindings().FALSE()),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns `true` if this [PdfPageObject] is marked as inactive on its containing page.
    #[inline]
    pub fn is_inactive(&self) -> Result<bool, PdfiumError> {
        self.is_active().map(|result| !result)
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "this [PdfPageObject]",
        "this [PdfPageObject].",
        "this [PdfPageObject],"
    );

    // The transform_impl() and reset_matrix_impl() functions required by the
    // create_transform_setters!() macro are provided by the PdfPageObjectPrivate trait.

    create_transform_getters!("this [PdfPageObject]", "this [PdfPageObject].", "this [PdfPageObject],");

    // The get_matrix_impl() function required by the create_transform_getters!() macro
    // is provided by the PdfPageObjectPrivate trait.
}

/// Functionality common to all [PdfPageObject] objects, regardless of their [PdfPageObjectType].
pub trait PdfPageObjectCommon<'a> {
    /// Returns `true` if this [PdfPageObject] contains transparency.
    fn has_transparency(&self) -> bool;

    /// Returns the bounding box of this [PdfPageObject] as a quadrilateral.
    ///
    /// For text objects, the bottom of the bounding box is set to the font baseline. Any characters
    /// in the text object that have glyph shapes that descends below the font baseline will extend
    /// beneath the bottom of this bounding box. To measure the distance of the maximum descent of
    /// any glyphs, use the [PdfPageTextObject::descent()] function.
    fn bounds(&self) -> Result<PdfQuadPoints, PdfiumError>;

    /// Returns the width of this [PdfPageObject].
    #[inline]
    fn width(&self) -> Result<PdfPoints, PdfiumError> {
        Ok(self.bounds()?.width())
    }

    /// Returns the height of this [PdfPageObject].
    #[inline]
    fn height(&self) -> Result<PdfPoints, PdfiumError> {
        Ok(self.bounds()?.height())
    }

    /// Returns `true` if the bounds of this [PdfPageObject] lie entirely within the given rectangle.
    #[inline]
    fn is_inside_rect(&self, rect: &PdfRect) -> bool {
        self.bounds()
            .map(|bounds| bounds.to_rect().is_inside(rect))
            .unwrap_or(false)
    }

    /// Returns `true` if the bounds of this [PdfPageObject] lie at least partially within
    /// the given rectangle.
    #[inline]
    fn does_overlap_rect(&self, rect: &PdfRect) -> bool {
        self.bounds()
            .map(|bounds| bounds.to_rect().does_overlap(rect))
            .unwrap_or(false)
    }

    /// Transforms this [PdfPageObject] by applying the transformation matrix read from the given [PdfPageObject].
    ///
    /// Any translation, rotation, scaling, or skewing transformations currently applied to the
    /// given [PdfPageObject] will be immediately applied to this [PdfPageObject].
    fn transform_from(&mut self, other: &PdfPageObject) -> Result<(), PdfiumError>;

    /// Sets the blend mode that will be applied when painting this [PdfPageObject].
    ///
    /// Note that Pdfium does not currently expose a function to read the currently set blend mode.
    fn set_blend_mode(&mut self, blend_mode: PdfPageObjectBlendMode) -> Result<(), PdfiumError>;

    /// Returns the color of any filled paths in this [PdfPageObject].
    fn fill_color(&self) -> Result<PdfColor, PdfiumError>;

    /// Sets the color of any filled paths in this [PdfPageObject].
    fn set_fill_color(&mut self, fill_color: PdfColor) -> Result<(), PdfiumError>;

    /// Returns the color of any stroked paths in this [PdfPageObject].
    fn stroke_color(&self) -> Result<PdfColor, PdfiumError>;

    /// Sets the color of any stroked paths in this [PdfPageObject].
    ///
    /// Even if this object's path is set with a visible color and a non-zero stroke width,
    /// the object's stroke mode must be set in order for strokes to actually be visible.
    fn set_stroke_color(&mut self, stroke_color: PdfColor) -> Result<(), PdfiumError>;

    /// Returns the width of any stroked lines in this [PdfPageObject].
    fn stroke_width(&self) -> Result<PdfPoints, PdfiumError>;

    /// Sets the width of any stroked lines in this [PdfPageObject].
    ///
    /// A line width of 0 denotes the thinnest line that can be rendered at device resolution:
    /// 1 device pixel wide. However, some devices cannot reproduce 1-pixel lines,
    /// and on high-resolution devices, they are nearly invisible. Since the results of rendering
    /// such zero-width lines are device-dependent, their use is not recommended.
    ///
    /// Even if this object's path is set with a visible color and a non-zero stroke width,
    /// the object's stroke mode must be set in order for strokes to actually be visible.
    fn set_stroke_width(&mut self, stroke_width: PdfPoints) -> Result<(), PdfiumError>;

    /// Returns the line join style that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    fn line_join(&self) -> Result<PdfPageObjectLineJoin, PdfiumError>;

    /// Sets the line join style that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    fn set_line_join(&mut self, line_join: PdfPageObjectLineJoin) -> Result<(), PdfiumError>;

    /// Returns the line cap style that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    fn line_cap(&self) -> Result<PdfPageObjectLineCap, PdfiumError>;

    /// Sets the line cap style that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    fn set_line_cap(&mut self, line_cap: PdfPageObjectLineCap) -> Result<(), PdfiumError>;

    /// Returns the line dash phase that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    ///
    /// A page object's line dash pattern controls the pattern of dashes and gaps used to stroke
    /// paths, as specified by a _dash array_ and a _dash phase_. The dash array's elements are
    /// [PdfPoints] values that specify the lengths of alternating dashes and gaps; all values
    /// must be non-zero and non-negative. The dash phase specifies the distance into the dash pattern
    /// at which to start the dash.
    ///
    /// For more information on stroked dash patterns, refer to the PDF Reference Manual,
    /// version 1.7, pages 217 - 218.
    ///
    /// Note that dash pattern save support in Pdfium was not fully stabilized until release
    /// `chromium/5772` (May 2023). Versions of Pdfium older than this can load and render
    /// dash patterns, but will not save dash patterns to PDF files.
    fn dash_phase(&self) -> Result<PdfPoints, PdfiumError>;

    /// Sets the line dash phase that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    ///
    /// A page object's line dash pattern controls the pattern of dashes and gaps used to stroke
    /// paths, as specified by a _dash array_ and a _dash phase_. The dash array's elements are
    /// [PdfPoints] values that specify the lengths of alternating dashes and gaps; all values
    /// must be non-zero and non-negative. The dash phase specifies the distance into the dash pattern
    /// at which to start the dash.
    ///
    /// For more information on stroked dash patterns, refer to the PDF Reference Manual,
    /// version 1.7, pages 217 - 218.
    ///
    /// Note that dash pattern save support in Pdfium was not fully stabilized until release
    /// `chromium/5772` (May 2023). Versions of Pdfium older than this can load and render
    /// dash patterns, but will not save dash patterns to PDF files.
    fn set_dash_phase(&mut self, dash_phase: PdfPoints) -> Result<(), PdfiumError>;

    /// Returns the line dash array that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    ///
    /// A page object's line dash pattern controls the pattern of dashes and gaps used to stroke
    /// paths, as specified by a _dash array_ and a _dash phase_. The dash array's elements are
    /// [PdfPoints] values that specify the lengths of alternating dashes and gaps; all values
    /// must be non-zero and non-negative. The dash phase specifies the distance into the dash pattern
    /// at which to start the dash.
    ///
    /// For more information on stroked dash patterns, refer to the PDF Reference Manual,
    /// version 1.7, pages 217 - 218.
    ///
    /// Note that dash pattern save support in Pdfium was not fully stabilized until release
    /// `chromium/5772` (May 2023). Versions of Pdfium older than this can load and render
    /// dash patterns, but will not save dash patterns to PDF files.
    fn dash_array(&self) -> Result<Vec<PdfPoints>, PdfiumError>;

    /// Sets the line dash array that will be used when painting stroked path segments
    /// in this [PdfPageObject].
    ///
    /// A page object's line dash pattern controls the pattern of dashes and gaps used to stroke
    /// paths, as specified by a _dash array_ and a _dash phase_. The dash array's elements are
    /// [PdfPoints] values that specify the lengths of alternating dashes and gaps; all values
    /// must be non-zero and non-negative. The dash phase specifies the distance into the dash pattern
    /// at which to start the dash.
    ///
    /// For more information on stroked dash patterns, refer to the PDF Reference Manual,
    /// version 1.7, pages 217 - 218.
    ///
    /// Note that dash pattern save support in Pdfium was not fully stabilized until release
    /// `chromium/5772` (May 2023). Versions of Pdfium older than this can load and render
    /// dash patterns, but will not save dash patterns to PDF files.
    fn set_dash_array(&mut self, array: &[PdfPoints], phase: PdfPoints) -> Result<(), PdfiumError>;

    #[deprecated(
        since = "0.8.32",
        note = "This function has been retired in favour of the PdfPageObject::copy_to_page() function."
    )]
    /// Returns `true` if this [PdfPageObject] can be successfully copied by calling its
    /// `try_copy()` function.
    ///
    /// Not all page objects can be successfully copied. The following restrictions apply:
    ///
    /// * For path objects, it is not possible to copy a path object that contains a Bézier path
    ///   segment, because Pdfium does not currently provide any way to retrieve the control points of a
    ///   Bézier curve of an existing path object.
    /// * For text objects, the font used by the object must be present in the destination document,
    ///   or text rendering behaviour will be unpredictable. While text objects refer to fonts,
    ///   font data is embedded into documents separately from text objects.
    /// * For image objects, Pdfium allows iterating over the list of image filters applied
    ///   to an image object, but currently provides no way to set a new object's image filters.
    ///   As a result, it is not possible to copy an image object that has any image filters applied.
    ///
    /// Pdfium currently allows setting the blend mode for a page object, but provides no way
    /// to retrieve an object's current blend mode. As a result, the blend mode setting of the
    /// original object will not be transferred to the copy.
    fn is_copyable(&self) -> bool;

    #[deprecated(
        since = "0.8.32",
        note = "This function has been retired in favour of the PdfPageObject::copy_to_page() function."
    )]
    /// Attempts to copy this [PdfPageObject] by creating a new page object and copying across
    /// all the properties of this [PdfPageObject] to the new page object.
    ///
    /// Not all page objects can be successfully copied. The following restrictions apply:
    ///
    /// * For path objects, it is not possible to copy a path object that contains a Bézier path
    ///   segment, because Pdfium does not currently provide any way to retrieve the control points of a
    ///   Bézier curve of an existing path object.
    /// * For text objects, the font used by the object must be present in the destination document,
    ///   or text rendering behaviour will be unpredictable. While text objects refer to fonts,
    ///   font data is embedded into documents separately from text objects.
    /// * For image objects, Pdfium allows iterating over the list of image filters applied
    ///   to an image object, but currently provides no way to set a new object's image filters.
    ///   As a result, it is not possible to copy an image object that has any image filters applied.
    ///
    /// Pdfium currently allows setting the blend mode for a page object, but provides no way
    /// to retrieve an object's current blend mode. As a result, the blend mode setting of the
    /// original object will not be transferred to the copy.
    ///
    /// The returned page object will be detached from any existing [PdfPage]. Its lifetime
    /// will be bound to the lifetime of the given destination [PdfDocument].
    fn try_copy<'b>(&self, document: &'b PdfDocument<'b>) -> Result<PdfPageObject<'b>, PdfiumError>;

    /// Copies this [PdfPageObject] object into a new [PdfPageXObjectFormObject], then adds
    /// the new form object to the page objects collection of the given [PdfPage],
    /// returning the new form object.
    fn copy_to_page<'b>(&mut self, page: &mut PdfPage<'b>) -> Result<PdfPageObject<'b>, PdfiumError>;

    /// Moves the ownership of this [PdfPageObject] to the given [PdfPage], regenerating
    /// page content as necessary.
    ///
    /// An error will be returned if the destination page is in a different [PdfDocument]
    /// than this object. Pdfium only supports safely moving objects within the
    /// same document, not across documents.
    fn move_to_page(&mut self, page: &mut PdfPage) -> Result<(), PdfiumError>;

    /// Moves the ownership of this [PdfPageObject] to the given [PdfPageAnnotation],
    /// regenerating page content as necessary.
    ///
    /// An error will be returned if the destination annotation is in a different [PdfDocument]
    /// than this object. Pdfium only supports safely moving objects within the
    /// same document, not across documents.
    fn move_to_annotation(&mut self, annotation: &mut PdfPageAnnotation) -> Result<(), PdfiumError>;

    /// Returns the marked content ID (MCID) for this page object, if any.
    ///
    /// The MCID links this object to an element in the page's structure tree,
    /// providing a bridge between visual content and semantic document structure.
    /// Returns `None` if this object has no associated marked content ID.
    fn marked_content_id(&self) -> Option<i32>;

    /// Returns the collection of content marks associated with this page object.
    ///
    /// Content marks provide metadata about page objects (e.g., "P", "Span", "Artifact")
    /// and can contain key-value parameters. Use the returned collection to iterate
    /// over marks or access them by index.
    fn content_marks(&self) -> PdfPageObjectContentMarks<'_>;
}

// Blanket implementation for all PdfPageObject types.

impl<'a, T> PdfPageObjectCommon<'a> for T
where
    T: PdfPageObjectPrivate<'a>,
{
    #[inline]
    fn has_transparency(&self) -> bool {
        self.has_transparency_impl()
    }

    #[inline]
    fn bounds(&self) -> Result<PdfQuadPoints, PdfiumError> {
        self.bounds_impl()
    }

    #[inline]
    fn transform_from(&mut self, other: &PdfPageObject) -> Result<(), PdfiumError> {
        self.reset_matrix_impl(other.matrix()?)
    }

    #[inline]
    fn set_blend_mode(&mut self, blend_mode: PdfPageObjectBlendMode) -> Result<(), PdfiumError> {
        self.bindings()
            .FPDFPageObj_SetBlendMode(self.object_handle(), blend_mode.as_pdfium());

        Ok(())
    }

    #[inline]
    fn fill_color(&self) -> Result<PdfColor, PdfiumError> {
        let mut r = 0;

        let mut g = 0;

        let mut b = 0;

        let mut a = 0;

        if self.bindings().is_true(self.bindings().FPDFPageObj_GetFillColor(
            self.object_handle(),
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

    #[inline]
    fn set_fill_color(&mut self, fill_color: PdfColor) -> Result<(), PdfiumError> {
        if self.bindings().is_true(self.bindings().FPDFPageObj_SetFillColor(
            self.object_handle(),
            fill_color.red() as c_uint,
            fill_color.green() as c_uint,
            fill_color.blue() as c_uint,
            fill_color.alpha() as c_uint,
        )) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn stroke_color(&self) -> Result<PdfColor, PdfiumError> {
        let mut r = 0;

        let mut g = 0;

        let mut b = 0;

        let mut a = 0;

        if self.bindings().is_true(self.bindings().FPDFPageObj_GetStrokeColor(
            self.object_handle(),
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

    #[inline]
    fn set_stroke_color(&mut self, stroke_color: PdfColor) -> Result<(), PdfiumError> {
        if self.bindings().is_true(self.bindings().FPDFPageObj_SetStrokeColor(
            self.object_handle(),
            stroke_color.red() as c_uint,
            stroke_color.green() as c_uint,
            stroke_color.blue() as c_uint,
            stroke_color.alpha() as c_uint,
        )) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn stroke_width(&self) -> Result<PdfPoints, PdfiumError> {
        let mut width = 0.0;

        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_GetStrokeWidth(self.object_handle(), &mut width),
        ) {
            Ok(PdfPoints::new(width))
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn set_stroke_width(&mut self, stroke_width: PdfPoints) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetStrokeWidth(self.object_handle(), stroke_width.value),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn line_join(&self) -> Result<PdfPageObjectLineJoin, PdfiumError> {
        PdfPageObjectLineJoin::from_pdfium(self.bindings().FPDFPageObj_GetLineJoin(self.object_handle()))
            .ok_or(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
    }

    #[inline]
    fn set_line_join(&mut self, line_join: PdfPageObjectLineJoin) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetLineJoin(self.object_handle(), line_join.as_pdfium() as c_int),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn line_cap(&self) -> Result<PdfPageObjectLineCap, PdfiumError> {
        PdfPageObjectLineCap::from_pdfium(self.bindings().FPDFPageObj_GetLineCap(self.object_handle()))
            .ok_or(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
    }

    #[inline]
    fn set_line_cap(&mut self, line_cap: PdfPageObjectLineCap) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetLineCap(self.object_handle(), line_cap.as_pdfium() as c_int),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn dash_phase(&self) -> Result<PdfPoints, PdfiumError> {
        let mut phase = 0.0;

        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_GetDashPhase(self.object_handle(), &mut phase),
        ) {
            Ok(PdfPoints::new(phase))
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn set_dash_phase(&mut self, dash_phase: PdfPoints) -> Result<(), PdfiumError> {
        if self.bindings().is_true(
            self.bindings()
                .FPDFPageObj_SetDashPhase(self.object_handle(), dash_phase.value),
        ) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn dash_array(&self) -> Result<Vec<PdfPoints>, PdfiumError> {
        let dash_count = self.bindings().FPDFPageObj_GetDashCount(self.object_handle()) as usize;

        let mut dash_array = vec![0.0; dash_count];

        if self.bindings().is_true(self.bindings().FPDFPageObj_GetDashArray(
            self.object_handle(),
            dash_array.as_mut_ptr(),
            dash_count,
        )) {
            Ok(dash_array.iter().map(|dash| PdfPoints::new(*dash)).collect())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    fn set_dash_array(&mut self, array: &[PdfPoints], phase: PdfPoints) -> Result<(), PdfiumError> {
        let dash_array = array.iter().map(|dash| dash.value).collect::<Vec<_>>();

        if self.bindings().is_true(self.bindings().FPDFPageObj_SetDashArray(
            self.object_handle(),
            dash_array.as_ptr(),
            dash_array.len(),
            phase.value,
        )) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumFunctionReturnValueIndicatedFailure)
        }
    }

    #[inline]
    fn is_copyable(&self) -> bool {
        self.is_copyable_impl()
    }

    #[inline]
    fn try_copy<'b>(&self, document: &'b PdfDocument<'b>) -> Result<PdfPageObject<'b>, PdfiumError> {
        self.try_copy_impl(document.handle(), document.bindings())
    }

    #[inline]
    fn copy_to_page<'b>(&mut self, page: &mut PdfPage<'b>) -> Result<PdfPageObject<'b>, PdfiumError> {
        self.copy_to_page_impl(page)
    }

    fn move_to_page(&mut self, page: &mut PdfPage) -> Result<(), PdfiumError> {
        match self.ownership() {
            PdfPageObjectOwnership::Document(ownership) => {
                if ownership.document_handle() != page.document_handle() {
                    return Err(PdfiumError::CannotMoveObjectAcrossDocuments);
                }
            }
            PdfPageObjectOwnership::Page(_) => self.remove_object_from_page()?,
            PdfPageObjectOwnership::AttachedAnnotation(_) | PdfPageObjectOwnership::UnattachedAnnotation(_) => {
                self.remove_object_from_annotation()?
            }
            PdfPageObjectOwnership::Unowned => {}
        }

        self.add_object_to_page(page.objects_mut())
    }

    fn move_to_annotation(&mut self, annotation: &mut PdfPageAnnotation) -> Result<(), PdfiumError> {
        match self.ownership() {
            PdfPageObjectOwnership::Document(ownership) => {
                let annotation_document_handle = match annotation.ownership() {
                    PdfPageObjectOwnership::Document(ownership) => Some(ownership.document_handle()),
                    PdfPageObjectOwnership::Page(ownership) => Some(ownership.document_handle()),
                    PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.document_handle()),
                    PdfPageObjectOwnership::UnattachedAnnotation(_) | PdfPageObjectOwnership::Unowned => None,
                };

                if let Some(annotation_document_handle) = annotation_document_handle
                    && ownership.document_handle() != annotation_document_handle
                {
                    return Err(PdfiumError::CannotMoveObjectAcrossDocuments);
                }
            }
            PdfPageObjectOwnership::Page(_) => self.remove_object_from_page()?,
            PdfPageObjectOwnership::AttachedAnnotation(_) | PdfPageObjectOwnership::UnattachedAnnotation(_) => {
                self.remove_object_from_annotation()?
            }
            PdfPageObjectOwnership::Unowned => {}
        }

        self.add_object_to_annotation(annotation.objects())
    }

    #[inline]
    fn marked_content_id(&self) -> Option<i32> {
        let mcid = self.bindings().FPDFPageObj_GetMarkedContentID(self.object_handle());

        if mcid == -1 { None } else { Some(mcid) }
    }

    #[inline]
    fn content_marks(&self) -> PdfPageObjectContentMarks<'_> {
        PdfPageObjectContentMarks::from_pdfium(self.object_handle(), self.bindings())
    }
}

impl<'a> PdfPageObjectPrivate<'a> for PdfPageObject<'a> {
    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.unwrap_as_trait().bindings()
    }

    #[inline]
    fn object_handle(&self) -> FPDF_PAGEOBJECT {
        self.unwrap_as_trait().object_handle()
    }

    #[inline]
    fn ownership(&self) -> &PdfPageObjectOwnership {
        self.unwrap_as_trait().ownership()
    }

    #[inline]
    fn set_ownership(&mut self, ownership: PdfPageObjectOwnership) {
        self.unwrap_as_trait_mut().set_ownership(ownership);
    }

    #[inline]
    fn add_object_to_page(&mut self, page_objects: &mut PdfPageObjects) -> Result<(), PdfiumError> {
        self.unwrap_as_trait_mut().add_object_to_page(page_objects)
    }

    #[inline]
    fn remove_object_from_page(&mut self) -> Result<(), PdfiumError> {
        self.unwrap_as_trait_mut().remove_object_from_page()
    }

    #[inline]
    fn add_object_to_annotation(&mut self, annotation_objects: &PdfPageAnnotationObjects) -> Result<(), PdfiumError> {
        self.unwrap_as_trait_mut().add_object_to_annotation(annotation_objects)
    }

    #[inline]
    fn remove_object_from_annotation(&mut self) -> Result<(), PdfiumError> {
        self.unwrap_as_trait_mut().remove_object_from_annotation()
    }

    #[inline]
    fn is_copyable_impl(&self) -> bool {
        self.unwrap_as_trait().is_copyable_impl()
    }

    #[inline]
    fn try_copy_impl<'b>(
        &self,
        document: FPDF_DOCUMENT,
        bindings: &'b dyn PdfiumLibraryBindings,
    ) -> Result<PdfPageObject<'b>, PdfiumError> {
        self.unwrap_as_trait().try_copy_impl(document, bindings)
    }

    #[inline]
    fn copy_to_page_impl<'b>(&mut self, page: &mut PdfPage<'b>) -> Result<PdfPageObject<'b>, PdfiumError> {
        self.unwrap_as_trait_mut().copy_to_page_impl(page)
    }
}

impl<'a> From<PdfPageXObjectFormObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPageXObjectFormObject<'a>) -> Self {
        Self::XObjectForm(object)
    }
}

impl<'a> From<PdfPageImageObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPageImageObject<'a>) -> Self {
        Self::Image(object)
    }
}

impl<'a> From<PdfPagePathObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPagePathObject<'a>) -> Self {
        Self::Path(object)
    }
}

impl<'a> From<PdfPageShadingObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPageShadingObject<'a>) -> Self {
        Self::Shading(object)
    }
}

impl<'a> From<PdfPageTextObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPageTextObject<'a>) -> Self {
        Self::Text(object)
    }
}

impl<'a> From<PdfPageUnsupportedObject<'a>> for PdfPageObject<'a> {
    #[inline]
    fn from(object: PdfPageUnsupportedObject<'a>) -> Self {
        Self::Unsupported(object)
    }
}

impl<'a> Drop for PdfPageObject<'a> {
    /// Closes this [PdfPageObject], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        // The documentation for FPDFPageObj_Destroy() states that we only need
        // call the function for page objects created by FPDFPageObj_CreateNew*() or
        // FPDFPageObj_New*Obj() _and_ where the newly-created object was _not_ subsequently
        // added to a PdfPage or PdfPageAnnotation via a call to FPDFPage_InsertObject() or
        // FPDFAnnot_AppendObject().

        // In other words, retrieving a page object that already exists in a document evidently
        // does not allocate any additional resources, so we don't need to free anything.
        // (Indeed, if we try to, Pdfium segfaults.)

        if !self.ownership().is_owned() {
            self.bindings().FPDFPageObj_Destroy(self.object_handle());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_apply_matrix() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let font = document.fonts_mut().times_roman();

        let mut object = page.objects_mut().create_text_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            "My new text object",
            font,
            PdfPoints::new(10.0),
        )?;

        // Apply some basic transformations to the object...

        object.translate(PdfPoints::new(100.0), PdfPoints::new(100.0))?;
        object.flip_vertically()?;
        object.rotate_clockwise_degrees(45.0)?;
        object.scale(3.0, 4.0)?;

        let previous_matrix = object.matrix()?;

        // _Applying_ the identity matrix should not alter the current matrix.

        object.apply_matrix(PdfMatrix::IDENTITY)?;

        assert_eq!(previous_matrix, object.matrix()?);

        Ok(())
    }

    #[test]
    fn test_reset_matrix_to_identity() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let font = document.fonts_mut().times_roman();

        let mut object = page.objects_mut().create_text_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            "My new text object",
            font,
            PdfPoints::new(10.0),
        )?;

        // Apply some basic transformations to the object...

        object.translate(PdfPoints::new(100.0), PdfPoints::new(100.0))?;
        object.flip_vertically()?;
        object.rotate_clockwise_degrees(45.0)?;
        object.scale(3.0, 4.0)?;

        let previous_matrix = object.matrix()?;

        // _Resetting_ the object's matrix back to the identity matrix should wipe out
        // the current matrix.

        object.reset_matrix_to_identity()?;

        assert_ne!(previous_matrix, object.matrix()?);
        assert_eq!(object.matrix()?, PdfMatrix::IDENTITY);

        Ok(())
    }

    #[test]
    fn test_transform_captured_in_content_regeneration() -> Result<(), PdfiumError> {
        // The purpose of the test is to confirm that object transformations are correctly
        // applied to the page's content streams by automatic content regeneration.
        // In pdfium-render versions 0.8.27 and earlier, this was not reliably the case.
        // See: https://github.com/ajrcarey/pdfium-render/issues/168

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let x = PdfPoints::new(100.0);
        let y = PdfPoints::new(400.0);

        let object_matrix_before_rotation = {
            let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            let font = document.fonts_mut().new_built_in(PdfFontBuiltin::TimesRoman);

            let mut object = page
                .objects_mut()
                .create_text_object(x, y, "Hello world!", font, PdfPoints::new(20.0))?;

            let object_matrix_before_rotation = object.matrix()?;

            // In pdfium-render versions 0.8.27 and earlier, the following transformation
            // will not be captured by automatic content regeneration. It will be lost
            // when the page falls out of scope.
            object.rotate_clockwise_degrees(45.0)?;

            object_matrix_before_rotation
        };

        // The page has now been dropped since it has fallen out of scope. Re-open it,
        // retrieve the page object, and test its transformation matrix. It should be
        // correctly rotated, indicating the transformation was correctly captured by
        // automatic content regeneration.

        assert_eq!(
            object_matrix_before_rotation.rotate_clockwise_degrees(45.0)?,
            document.pages().first()?.objects().first()?.matrix()?
        );

        Ok(())
    }
}
