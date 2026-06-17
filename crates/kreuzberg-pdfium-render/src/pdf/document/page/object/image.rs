//! Defines the [PdfPageImageObject] struct, exposing functionality related to a single page
//! object defining an image, where the image data is sourced from a [PdfBitmap] buffer.

use crate::bindgen::{FPDF_DOCUMENT, FPDF_IMAGEOBJ_METADATA, FPDF_PAGE, FPDF_PAGEOBJECT, fpdf_page_t__};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::bitmap::PdfBitmap;
use crate::pdf::bitmap::Pixels;
use crate::pdf::color_space::PdfColorSpace;
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectOwnership};
use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
use crate::pdf::points::PdfPoints;
use crate::utils::mem::create_byte_buffer;
use crate::{create_transform_getters, create_transform_setters};
use std::convert::TryInto;
use std::ops::{Range, RangeInclusive};
use std::os::raw::{c_int, c_void};

#[cfg(not(target_arch = "wasm32"))]
use {
    crate::utils::files::get_pdfium_file_accessor_from_reader,
    std::fs::File,
    std::io::{Read, Seek},
    std::path::Path,
};

#[cfg(feature = "image_025")]
use {
    crate::pdf::bitmap::PdfBitmapFormat,
    crate::utils::pixels::{aligned_bgr_to_rgba, aligned_grayscale_to_unaligned, bgra_to_rgba, rgba_to_bgra},
    image_025::{DynamicImage, EncodableLayout, GrayImage, RgbaImage},
};

#[cfg(doc)]
use {
    crate::pdf::document::page::PdfPage, crate::pdf::document::page::object::PdfPageObjectType,
    crate::pdf::document::page::objects::common::PdfPageObjectsCommon,
};

/// A single [PdfPageObject] of type [PdfPageObjectType::Image]. The page object defines a
/// single image, where the image data is sourced from a [PdfBitmap] buffer.
///
/// Page objects can be created either attached to a [PdfPage] (in which case the page object's
/// memory is owned by the containing page) or detached from any page (in which case the page
/// object's memory is owned by the object). Page objects are not rendered until they are
/// attached to a page; page objects that are never attached to a page will be lost when they
/// fall out of scope.
///
/// The simplest way to create a page image object that is immediately attached to a page
/// is to call the [PdfPageObjectsCommon::create_image_object()] function.
///
/// Creating a detached page image object offers more scope for customization, but you must
/// add the object to a containing [PdfPage] manually. To create a detached page image object,
/// use the [PdfPageImageObject::new()] or [PdfPageImageObject::new_from_jpeg_file()] functions.
/// The detached page image object can later be attached to a page by using the
/// [PdfPageObjectsCommon::add_image_object()] function.
pub struct PdfPageImageObject<'a> {
    object_handle: FPDF_PAGEOBJECT,
    ownership: PdfPageObjectOwnership,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageImageObject<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        object_handle: FPDF_PAGEOBJECT,
        ownership: PdfPageObjectOwnership,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageImageObject {
            object_handle,
            ownership,
            bindings,
        }
    }

    /// Creates a new [PdfPageImageObject] from the given [DynamicImage]. The returned
    /// page object will not be rendered until it is added to a [PdfPage] using the
    /// [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// The returned page object will have its width and height both set to 1.0 points.
    /// Use the [PdfPageImageObject::scale()] function to apply a horizontal and vertical scale
    /// to the object after it is created, or use one of the [PdfPageImageObject::new_with_width()],
    /// [PdfPageImageObject::new_with_height()], or [PdfPageImageObject::new_with_size()] functions
    /// to scale the page object to a specific width and/or height at the time the object is created.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn new(document: &PdfDocument<'a>, image: &DynamicImage) -> Result<Self, PdfiumError> {
        let mut result = Self::new_from_handle(document.handle(), document.bindings());

        if let Ok(result) = result.as_mut() {
            result.set_image(image)?;
        }

        result
    }

    /// Creates a new [PdfPageImageObject]. The returned page object will not be
    /// rendered until it is added to a [PdfPage] using the
    /// [PdfPageObjects::add_image_object()] function.
    ///
    /// Use the [PdfPageImageObject::set_bitmap()] function to apply image data to
    /// the empty object.
    ///
    /// The returned page object will have its width and height both set to 1.0 points.
    /// Use the [WriteTransforms::scale()] function to apply a horizontal and vertical scale
    /// to the object after it is created.
    #[cfg(not(feature = "image_025"))]
    pub fn new(document: &PdfDocument<'a>) -> Result<Self, PdfiumError> {
        Self::new_from_handle(document.handle(), document.bindings())
    }

    /// Creates a new [PdfPageImageObject] containing JPEG image data loaded from the
    /// given file path. The returned page object will not be rendered until it is added to
    /// a [PdfPage] using the [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// The returned page object will have its width and height both set to 1.0 points.
    /// Use the [PdfPageImageObject::scale] function to apply a horizontal and vertical scale
    /// to the object after it is created, or use one of the [PdfPageImageObject::new_with_width()],
    /// [PdfPageImageObject::new_with_height()], or [PdfPageImageObject::new_with_size()] functions
    /// to scale the page object to a specific width and/or height at the time the object is created.
    ///
    /// This function is not available when compiling to WASM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_from_jpeg_file(
        document: &PdfDocument<'a>,
        path: &(impl AsRef<Path> + ?Sized),
    ) -> Result<Self, PdfiumError> {
        Self::new_from_jpeg_reader(document, File::open(path).map_err(PdfiumError::IoError)?)
    }

    /// Creates a new [PdfPageImageObject] containing JPEG image data loaded from the
    /// given reader. Because Pdfium must know the total content length in advance prior to
    /// loading any portion of it, the given reader must implement the [Seek] trait
    /// as well as the [Read] trait.
    ///
    /// The returned page object will not be rendered until it is added to
    /// a [PdfPage] using the [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// The returned page object will have its width and height both set to 1.0 points.
    /// Use the [PdfPageImageObject::scale] function to apply a horizontal and vertical scale
    /// to the object after it is created, or use one of the [PdfPageImageObject::new_with_width()],
    /// [PdfPageImageObject::new_with_height()], or [PdfPageImageObject::new_with_size()] functions
    /// to scale the page object to a specific width and/or height at the time the object is created.
    ///
    /// This function is not available when compiling to WASM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_from_jpeg_reader<R: Read + Seek>(document: &PdfDocument<'a>, reader: R) -> Result<Self, PdfiumError> {
        let object = Self::new_from_handle(document.handle(), document.bindings())?;

        let mut reader = get_pdfium_file_accessor_from_reader(reader);

        let result = document.bindings().FPDFImageObj_LoadJpegFileInline(
            std::ptr::null_mut(),
            0,
            object.object_handle(),
            reader.as_fpdf_file_access_mut_ptr(),
        );

        if object.bindings.is_true(result) {
            Ok(object)
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    // Takes a raw `FPDF_DOCUMENT` handle to avoid cascading lifetime problems
    // associated with borrowing `PdfDocument<'a>`.
    pub(crate) fn new_from_handle(
        document: FPDF_DOCUMENT,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Result<Self, PdfiumError> {
        let handle = bindings.FPDFPageObj_NewImageObj(document);

        if handle.is_null() {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        } else {
            Ok(PdfPageImageObject {
                object_handle: handle,
                ownership: PdfPageObjectOwnership::unowned(),
                bindings,
            })
        }
    }

    /// Creates a new [PdfPageImageObject] from the given arguments. The page object will be scaled
    /// horizontally to match the given width; its height will be adjusted to maintain the aspect
    /// ratio of the given image. The returned page object will not be rendered until it is
    /// added to a [PdfPage] using the [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    pub fn new_with_width(
        document: &PdfDocument<'a>,
        image: &DynamicImage,
        width: PdfPoints,
    ) -> Result<Self, PdfiumError> {
        let aspect_ratio = image.height() as f32 / image.width() as f32;

        let height = width * aspect_ratio;

        Self::new_with_size(document, image, width, height)
    }

    /// Creates a new [PdfPageImageObject] from the given arguments. The page object will be scaled
    /// vertically to match the given height; its width will be adjusted to maintain the aspect
    /// ratio of the given image. The returned page object will not be rendered until it is
    /// added to a [PdfPage] using the [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    pub fn new_with_height(
        document: &PdfDocument<'a>,
        image: &DynamicImage,
        height: PdfPoints,
    ) -> Result<Self, PdfiumError> {
        let aspect_ratio = image.height() as f32 / image.width() as f32;

        let width = height / aspect_ratio;

        Self::new_with_size(document, image, width, height)
    }

    /// Creates a new [PdfPageImageObject] from the given arguments. The page object will be scaled to
    /// match the given width and height. The returned page object will not be rendered until it is
    /// added to a [PdfPage] using the [PdfPageObjectsCommon::add_image_object()] function.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn new_with_size(
        document: &PdfDocument<'a>,
        image: &DynamicImage,
        width: PdfPoints,
        height: PdfPoints,
    ) -> Result<Self, PdfiumError> {
        let mut result = Self::new(document, image)?;

        result.scale(width.value, height.value)?;

        Ok(result)
    }

    /// Returns a new [PdfBitmap] created from the bitmap buffer backing
    /// this [PdfPageImageObject], ignoring any image filters, image mask, or object
    /// transforms applied to this page object.
    pub fn get_raw_bitmap(&self) -> Result<PdfBitmap<'_>, PdfiumError> {
        Ok(PdfBitmap::from_pdfium(
            self.bindings().FPDFImageObj_GetBitmap(self.object_handle()),
            self.bindings(),
        ))
    }

    /// Returns a new [DynamicImage] created from the bitmap buffer backing
    /// this [PdfPageImageObject], ignoring any image filters, image mask, or object
    /// transforms applied to this page object.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn get_raw_image(&self) -> Result<DynamicImage, PdfiumError> {
        self.get_image_from_bitmap(&self.get_raw_bitmap()?)
    }

    /// Returns a new [PdfBitmap] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    #[inline]
    pub fn get_processed_bitmap(&self, document: &PdfDocument) -> Result<PdfBitmap<'_>, PdfiumError> {
        let (width, height) = self.get_current_width_and_height_from_metadata()?;

        self.get_processed_bitmap_with_size(document, width, height)
    }

    /// Returns a new [DynamicImage] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn get_processed_image(&self, document: &PdfDocument) -> Result<DynamicImage, PdfiumError> {
        let (width, height) = self.get_current_width_and_height_from_metadata()?;

        self.get_processed_image_with_size(document, width, height)
    }

    /// Returns a new [PdfBitmap] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned bitmap will be scaled during rendering so its width matches the given target width.
    #[inline]
    pub fn get_processed_bitmap_with_width(
        &self,
        document: &PdfDocument,
        width: Pixels,
    ) -> Result<PdfBitmap<'_>, PdfiumError> {
        let (current_width, current_height) = self.get_current_width_and_height_from_metadata()?;

        let aspect_ratio = current_width as f32 / current_height as f32;

        self.get_processed_bitmap_with_size(
            document,
            width,
            ((width as f32 / aspect_ratio) as u32)
                .try_into()
                .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?,
        )
    }

    /// Returns a new [DynamicImage] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned image will be scaled during rendering so its width matches the given target width.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn get_processed_image_with_width(
        &self,
        document: &PdfDocument,
        width: Pixels,
    ) -> Result<DynamicImage, PdfiumError> {
        let (current_width, current_height) = self.get_current_width_and_height_from_metadata()?;

        let aspect_ratio = current_width as f32 / current_height as f32;

        self.get_processed_image_with_size(
            document,
            width,
            ((width as f32 / aspect_ratio) as u32)
                .try_into()
                .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?,
        )
    }

    /// Returns a new [PdfBitmap] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned bitmap will be scaled during rendering so its height matches the given target height.
    #[inline]
    pub fn get_processed_bitmap_with_height(
        &self,
        document: &PdfDocument,
        height: Pixels,
    ) -> Result<PdfBitmap<'_>, PdfiumError> {
        let (current_width, current_height) = self.get_current_width_and_height_from_metadata()?;

        let aspect_ratio = current_width as f32 / current_height as f32;

        self.get_processed_bitmap_with_size(
            document,
            ((height as f32 * aspect_ratio) as u32)
                .try_into()
                .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?,
            height,
        )
    }

    /// Returns a new [DynamicImage] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned image will be scaled during rendering so its height matches the given target height.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn get_processed_image_with_height(
        &self,
        document: &PdfDocument,
        height: Pixels,
    ) -> Result<DynamicImage, PdfiumError> {
        let (current_width, current_height) = self.get_current_width_and_height_from_metadata()?;

        let aspect_ratio = current_width as f32 / current_height as f32;

        self.get_processed_image_with_size(
            document,
            ((height as f32 * aspect_ratio) as u32)
                .try_into()
                .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?,
            height,
        )
    }

    /// Returns a new [PdfBitmap] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned bitmap will be scaled during rendering so its width and height match
    /// the given target dimensions.
    pub fn get_processed_bitmap_with_size(
        &self,
        document: &PdfDocument,
        width: Pixels,
        height: Pixels,
    ) -> Result<PdfBitmap<'_>, PdfiumError> {
        // We attempt to work around two separate problems in Pdfium's
        // FPDFImageObj_GetRenderedBitmap() function.

        // First, the call to FPDFImageObj_GetRenderedBitmap() can fail, returning
        // a null FPDF_BITMAP handle, if the image object's transformation matrix includes
        // negative values for either the matrix.a or matrix.d values. We flip those values
        // in the transformation matrix if they are negative, and we make sure we restore
        // the original values before we return to the caller.

        // Second, Pdfium seems to often return a rendered bitmap that is much smaller
        // than the image object's metadata suggests. We look at the dimensions of the bitmap
        // returned from FPDFImageObj_GetRenderedBitmap(), and we apply a scale factor to the
        // image object's transformation matrix if the bitmap is not the expected size.

        // For more details, see: https://github.com/ajrcarey/pdfium-render/issues/52

        let mut matrix = self.matrix()?;

        let original_matrix = matrix; // We'll restore the matrix to this before we return.

        // Ensure the matrix.a and matrix.d values are not negative.

        if matrix.a() < 0f32 {
            matrix.set_a(-matrix.a());
            self.reset_matrix_impl(matrix)?;
        }

        if matrix.d() < 0f32 {
            matrix.set_d(-matrix.d());
            self.reset_matrix_impl(matrix)?;
        }

        let page_handle = match self.ownership() {
            PdfPageObjectOwnership::Page(ownership) => Some(ownership.page_handle()),
            PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.page_handle()),
            _ => None,
        };

        let bitmap_handle = match page_handle {
            Some(page_handle) => {
                self.bindings()
                    .FPDFImageObj_GetRenderedBitmap(document.handle(), page_handle, self.object_handle())
            }
            None => self.bindings.FPDFImageObj_GetRenderedBitmap(
                document.handle(),
                std::ptr::null_mut::<fpdf_page_t__>(),
                self.object_handle(),
            ),
        };

        if bitmap_handle.is_null() {
            // Restore the original transformation matrix values before we return the error
            // to the caller.

            self.reset_matrix_impl(original_matrix)?;
            return Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown));
        }

        let result = PdfBitmap::from_pdfium(bitmap_handle, self.bindings());

        if width == result.width() && height == result.height() {
            // The bitmap generated by Pdfium is already at the caller's requested dimensions.
            // Restore the original transformation matrix values before we return to the caller.

            self.reset_matrix_impl(original_matrix)?;

            Ok(result)
        } else {
            // The bitmap generated by Pdfium is not at the caller's requested dimensions.
            // We apply a scale transform to the page object to encourage Pdfium to generate
            // a bitmap matching the caller's requested dimensions.

            self.transform_impl(
                width as PdfMatrixValue / result.width() as PdfMatrixValue,
                0.0,
                0.0,
                height as PdfMatrixValue / result.height() as PdfMatrixValue,
                0.0,
                0.0,
            )?;

            // Generate the bitmap again at the new scale.

            let result = PdfBitmap::from_pdfium(
                match page_handle {
                    Some(page_handle) => self.bindings().FPDFImageObj_GetRenderedBitmap(
                        document.handle(),
                        page_handle,
                        self.object_handle(),
                    ),
                    None => self.bindings.FPDFImageObj_GetRenderedBitmap(
                        document.handle(),
                        std::ptr::null_mut::<fpdf_page_t__>(),
                        self.object_handle(),
                    ),
                },
                self.bindings,
            );

            // Restore the original transformation matrix values before we return to the caller.

            self.reset_matrix_impl(original_matrix)?;

            Ok(result)
        }
    }

    /// Returns a new [DynamicImage] created from the bitmap buffer backing
    /// this [PdfPageImageObject], taking into account any image filters, image mask, and
    /// object transforms applied to this page object.
    ///
    /// The returned image will be scaled during rendering so its width and height match
    /// the given target dimensions.
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    #[inline]
    pub fn get_processed_image_with_size(
        &self,
        document: &PdfDocument,
        width: Pixels,
        height: Pixels,
    ) -> Result<DynamicImage, PdfiumError> {
        self.get_processed_bitmap_with_size(document, width, height)
            .and_then(|bitmap| self.get_image_from_bitmap(&bitmap))
    }

    #[cfg(feature = "image_025")]
    pub(crate) fn get_image_from_bitmap(&self, bitmap: &PdfBitmap) -> Result<DynamicImage, PdfiumError> {
        let handle = bitmap.handle();

        let width = self.bindings.FPDFBitmap_GetWidth(handle);

        let height = self.bindings.FPDFBitmap_GetHeight(handle);

        let stride = self.bindings.FPDFBitmap_GetStride(handle);

        let format = PdfBitmapFormat::from_pdfium(self.bindings.FPDFBitmap_GetFormat(handle) as u32)?;

        #[cfg(not(target_arch = "wasm32"))]
        let buffer = self.bindings.FPDFBitmap_GetBuffer_as_slice(handle);

        #[cfg(target_arch = "wasm32")]
        let buffer_vec = self.bindings.FPDFBitmap_GetBuffer_as_vec(handle);
        #[cfg(target_arch = "wasm32")]
        let buffer = buffer_vec.as_slice();

        match format {
            #[allow(deprecated)]
            PdfBitmapFormat::BGRA | PdfBitmapFormat::BRGx | PdfBitmapFormat::BGRx => {
                RgbaImage::from_raw(width as u32, height as u32, bgra_to_rgba(buffer)).map(DynamicImage::ImageRgba8)
            }
            PdfBitmapFormat::BGR => RgbaImage::from_raw(
                width as u32,
                height as u32,
                aligned_bgr_to_rgba(buffer, width as usize, stride as usize),
            )
            .map(DynamicImage::ImageRgba8),
            PdfBitmapFormat::Gray => GrayImage::from_raw(
                width as u32,
                height as u32,
                aligned_grayscale_to_unaligned(buffer, width as usize, stride as usize),
            )
            .map(DynamicImage::ImageLuma8),
        }
        .ok_or(PdfiumError::ImageError)
    }

    /// Returns the raw image data backing this [PdfPageImageObject] exactly as it is stored
    /// in the containing PDF without applying any of the image's filters.
    ///
    /// The returned byte buffer may be empty if the image object does not contain any data.
    pub fn get_raw_image_data(&self) -> Result<Vec<u8>, PdfiumError> {
        let buffer_length = self
            .bindings()
            .FPDFImageObj_GetImageDataRaw(self.object_handle(), std::ptr::null_mut(), 0);

        if buffer_length == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings().FPDFImageObj_GetImageDataRaw(
            self.object_handle(),
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        Ok(buffer)
    }

    /// Returns the expected pixel width and height of the processed image from Pdfium's metadata.
    pub(crate) fn get_current_width_and_height_from_metadata(&self) -> Result<(Pixels, Pixels), PdfiumError> {
        let width = self
            .get_raw_metadata()
            .and_then(|metadata| metadata.width.try_into().map_err(|_| PdfiumError::ImageSizeOutOfBounds))?;

        let height = self.get_raw_metadata().and_then(|metadata| {
            metadata
                .height
                .try_into()
                .map_err(|_| PdfiumError::ImageSizeOutOfBounds)
        })?;

        Ok((width, height))
    }

    /// Returns the expected pixel width of the processed image for this [PdfPageImageObject],
    /// taking into account any image filters, image mask, and object transforms applied
    /// to this page object.
    #[inline]
    pub fn width(&self) -> Result<Pixels, PdfiumError> {
        self.get_current_width_and_height_from_metadata()
            .map(|(width, _height)| width)
    }

    /// Returns the expected pixel height of the processed image for this [PdfPageImageObject],
    /// taking into account any image filters, image mask, and object transforms applied
    /// to this page object.
    #[inline]
    pub fn height(&self) -> Result<Pixels, PdfiumError> {
        self.get_current_width_and_height_from_metadata()
            .map(|(_width, height)| height)
    }

    /// Applies the byte data in the given [DynamicImage] to this [PdfPageImageObject].
    ///
    /// This function is only available when this crate's `image` feature is enabled.
    #[cfg(feature = "image_025")]
    pub fn set_image(&mut self, image: &DynamicImage) -> Result<(), PdfiumError> {
        let width: Pixels = image
            .width()
            .try_into()
            .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?;

        let height: Pixels = image
            .height()
            .try_into()
            .map_err(|_| PdfiumError::ImageSizeOutOfBounds)?;

        let bitmap = PdfBitmap::empty(width, height, PdfBitmapFormat::BGRA, self.bindings)?;

        let buffer = if let Some(image) = image.as_rgba8() {
            // The given image is already in RGBA format.

            rgba_to_bgra(image.as_bytes())
        } else {
            // The image must be converted to RGBA first.

            let image = image.to_rgba8();

            rgba_to_bgra(image.as_bytes())
        };

        if !self.bindings.FPDFBitmap_SetBuffer(bitmap.handle(), buffer.as_slice()) {
            return Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown));
        }

        self.set_bitmap(&bitmap)
    }

    /// Applies the byte data in the given [PdfBitmap] to this [PdfPageImageObject].
    pub fn set_bitmap(&mut self, bitmap: &PdfBitmap) -> Result<(), PdfiumError> {
        if self.bindings.is_true(self.bindings().FPDFImageObj_SetBitmap(
            std::ptr::null_mut::<FPDF_PAGE>(),
            0,
            self.object_handle(),
            bitmap.handle(),
        )) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns all internal metadata for this [PdfPageImageObject].
    pub(crate) fn get_raw_metadata(&self) -> Result<FPDF_IMAGEOBJ_METADATA, PdfiumError> {
        let mut metadata = FPDF_IMAGEOBJ_METADATA {
            width: 0,
            height: 0,
            horizontal_dpi: 0.0,
            vertical_dpi: 0.0,
            bits_per_pixel: 0,
            colorspace: 0,
            marked_content_id: 0,
        };

        let page_handle = match self.ownership() {
            PdfPageObjectOwnership::Page(ownership) => Some(ownership.page_handle()),
            PdfPageObjectOwnership::AttachedAnnotation(ownership) => Some(ownership.page_handle()),
            _ => None,
        };

        let result = self.bindings().FPDFImageObj_GetImageMetadata(
            self.object_handle(),
            match page_handle {
                Some(page_handle) => page_handle,
                None => std::ptr::null_mut::<fpdf_page_t__>(),
            },
            &mut metadata,
        );

        if self.bindings().is_true(result) {
            Ok(metadata)
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
        }
    }

    /// Returns the horizontal dots per inch resolution of the image assigned to this
    /// [PdfPageImageObject], based on the intrinsic resolution of the assigned image
    /// and the dimensions of this object.
    #[inline]
    pub fn horizontal_dpi(&self) -> Result<f32, PdfiumError> {
        self.get_raw_metadata().map(|metadata| metadata.horizontal_dpi)
    }

    /// Returns the vertical dots per inch resolution of the image assigned to this
    /// [PdfPageImageObject], based on the intrinsic resolution of the assigned image
    /// and the dimensions of this object.
    #[inline]
    pub fn vertical_dpi(&self) -> Result<f32, PdfiumError> {
        self.get_raw_metadata().map(|metadata| metadata.vertical_dpi)
    }

    /// Returns the bits per pixel for the image assigned to this [PdfPageImageObject].
    ///
    /// This value is not available if this object has not been attached to a `PdfPage`.
    #[inline]
    pub fn bits_per_pixel(&self) -> Result<u8, PdfiumError> {
        self.get_raw_metadata().map(|metadata| metadata.bits_per_pixel as u8)
    }

    /// Returns the color space for the image assigned to this [PdfPageImageObject].
    ///
    /// This value is not available if this object has not been attached to a `PdfPage`.
    #[inline]
    pub fn color_space(&self) -> Result<PdfColorSpace, PdfiumError> {
        self.get_raw_metadata()
            .and_then(|metadata| PdfColorSpace::from_pdfium(metadata.colorspace as u32))
    }

    /// Returns the collection of image filters currently applied to this [PdfPageImageObject].
    #[inline]
    pub fn filters(&self) -> PdfPageImageObjectFilters<'_> {
        PdfPageImageObjectFilters::new(self)
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "this [PdfPageImageObject]",
        "this [PdfPageImageObject].",
        "this [PdfPageImageObject],"
    );

    // The transform_impl() function required by the create_transform_setters!() macro
    // is provided by the PdfPageObjectPrivate trait.

    create_transform_getters!(
        "this [PdfPageImageObject]",
        "this [PdfPageImageObject].",
        "this [PdfPageImageObject],"
    );

    // The get_matrix_impl() function required by the create_transform_getters!() macro
    // is provided by the PdfPageObjectPrivate trait.
}

impl<'a> PdfPageObjectPrivate<'a> for PdfPageImageObject<'a> {
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
        // Image filters cannot be copied.

        self.filters().is_empty()
    }

    #[inline]
    fn try_copy_impl<'b>(
        &self,
        document: FPDF_DOCUMENT,
        bindings: &'b dyn PdfiumLibraryBindings,
    ) -> Result<PdfPageObject<'b>, PdfiumError> {
        if !self.filters().is_empty() {
            // Image filters cannot be copied.

            return Err(PdfiumError::ImageObjectFiltersNotCopyable);
        }

        let mut copy = PdfPageImageObject::new_from_handle(document, bindings)?;

        copy.set_bitmap(&self.get_raw_bitmap()?)?;
        copy.reset_matrix(self.matrix()?)?;

        Ok(PdfPageObject::Image(copy))
    }
}

/// The zero-based index of a single [PdfPageImageObjectFilter] inside its containing
/// [PdfPageImageObjectFilters] collection.
pub type PdfPageImageObjectFilterIndex = usize;

/// A collection of all the image filters applied to a [PdfPageImageObject].
pub struct PdfPageImageObjectFilters<'a> {
    object: &'a PdfPageImageObject<'a>,
}

impl<'a> PdfPageImageObjectFilters<'a> {
    #[inline]
    pub(crate) fn new(object: &'a PdfPageImageObject<'a>) -> Self {
        PdfPageImageObjectFilters { object }
    }

    /// Returns the number of image filters applied to the parent [PdfPageImageObject].
    pub fn len(&self) -> usize {
        self.object
            .bindings()
            .FPDFImageObj_GetImageFilterCount(self.object.object_handle()) as usize
    }

    /// Returns true if this [PdfPageImageObjectFilters] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a Range from `0..(number of filters)` for this [PdfPageImageObjectFilters] collection.
    #[inline]
    pub fn as_range(&self) -> Range<PdfPageImageObjectFilterIndex> {
        0..self.len()
    }

    /// Returns an inclusive Range from `0..=(number of filters - 1)` for this [PdfPageImageObjectFilters] collection.
    #[inline]
    pub fn as_range_inclusive(&self) -> RangeInclusive<PdfPageImageObjectFilterIndex> {
        if self.is_empty() { 0..=0 } else { 0..=(self.len() - 1) }
    }

    /// Returns a single [PdfPageImageObjectFilter] from this [PdfPageImageObjectFilters] collection.
    pub fn get(&self, index: PdfPageImageObjectFilterIndex) -> Result<PdfPageImageObjectFilter, PdfiumError> {
        if index >= self.len() {
            return Err(PdfiumError::ImageObjectFilterIndexOutOfBounds);
        }

        // Retrieving the image filter name from Pdfium is a two-step operation. First, we call
        // FPDFImageObj_GetImageFilter() with a null buffer; this will retrieve the length of
        // the image filter name in bytes. If the length is zero, then there is no image filter name.

        // If the length is non-zero, then we reserve a byte buffer of the given
        // length and call FPDFImageObj_GetImageFilter() again with a pointer to the buffer;
        // this will write the font name into the buffer. Unlike most text handling in
        // Pdfium, image filter names are returned in UTF-8 format.

        let buffer_length = self.object.bindings().FPDFImageObj_GetImageFilter(
            self.object.object_handle(),
            index as c_int,
            std::ptr::null_mut(),
            0,
        );

        if buffer_length == 0 {
            // The image filter name is not present.

            return Err(PdfiumError::ImageObjectFilterIndexInBoundsButFilterUndefined);
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.object.bindings().FPDFImageObj_GetImageFilter(
            self.object.object_handle(),
            index as c_int,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        Ok(PdfPageImageObjectFilter::new(
            String::from_utf8(buffer)
                // Trim any trailing nulls. All strings returned from Pdfium are generally terminated
                // by one null byte.
                .map(|str| str.trim_end_matches(char::from(0)).to_owned())
                .unwrap_or_default(),
        ))
    }

    /// Returns an iterator over all the [PdfPageImageObjectFilter] objects in this
    /// [PdfPageImageObjectFilters] collection.
    #[inline]
    pub fn iter(&self) -> PdfPageImageObjectFiltersIterator<'_> {
        PdfPageImageObjectFiltersIterator::new(self)
    }
}

/// A single image filter applied to a [PdfPageImageObject].
pub struct PdfPageImageObjectFilter {
    name: String,
}

impl PdfPageImageObjectFilter {
    #[inline]
    pub(crate) fn new(name: String) -> Self {
        PdfPageImageObjectFilter { name }
    }

    /// Returns the name of this [PdfPageImageObjectFilter].
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// An iterator over all the [PdfPageImageObjectFilter] objects in a
/// [PdfPageImageObjectFilters] collection.
pub struct PdfPageImageObjectFiltersIterator<'a> {
    filters: &'a PdfPageImageObjectFilters<'a>,
    next_index: PdfPageImageObjectFilterIndex,
}

impl<'a> PdfPageImageObjectFiltersIterator<'a> {
    #[inline]
    pub(crate) fn new(filters: &'a PdfPageImageObjectFilters<'a>) -> Self {
        PdfPageImageObjectFiltersIterator { filters, next_index: 0 }
    }
}

impl<'a> Iterator for PdfPageImageObjectFiltersIterator<'a> {
    type Item = PdfPageImageObjectFilter;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.filters.get(self.next_index);

        self.next_index += 1;

        next.ok()
    }
}

impl<'a> Drop for PdfPageImageObject<'a> {
    /// Closes this [PdfPageImageObject], releasing held memory.
    fn drop(&mut self) {
        self.drop_impl();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};

    #[test]
    fn test_page_image_object_retains_format() -> Result<(), PdfiumError> {
        // Make sure the format of the image we pass into a new PdfPageImageObject is the
        // same when we later retrieve it.

        let pdfium = test_bind_to_pdfium();

        let image = pdfium
            .load_pdf_from_file(&test_fixture_path("path-test.pdf"), None)?
            .pages()
            .get(0)?
            .render_with_config(&PdfRenderConfig::new().set_target_width(1000))?
            .as_image()?;

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;

        let object = page.objects_mut().create_image_object(
            PdfPoints::new(100.0),
            PdfPoints::new(100.0),
            &image,
            Some(PdfPoints::new(image.width() as f32)),
            Some(PdfPoints::new(image.height() as f32)),
        )?;

        // Since the object has no image filters applied, both the raw and processed images should
        // be identical to the source image we assigned to the object. The processed image will
        // take the object's scale factors into account, but we made sure to set those to the actual
        // pixel dimensions of the source image.

        // A visual inspection can be carried out by uncommenting the PNG save commands below.

        let raw_image = object.as_image_object().unwrap().get_raw_image()?;

        // raw_image
        //     .save_with_format("./test/1.png", ImageFormat::Png)
        //     .unwrap();

        let processed_image = object.as_image_object().unwrap().get_processed_image(&document)?;

        // processed_image
        //     .save_with_format("./test/2.png", ImageFormat::Png)
        //     .unwrap();

        assert!(compare_equality_of_byte_arrays(
            image.as_bytes(),
            raw_image.into_rgba8().as_raw().as_slice()
        ));

        assert!(compare_equality_of_byte_arrays(
            image.as_bytes(),
            processed_image.into_rgba8().as_raw().as_slice()
        ));

        Ok(())
    }

    fn compare_equality_of_byte_arrays(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        for index in 0..a.len() {
            if a[index] != b[index] {
                return false;
            }
        }

        true
    }

    #[test]
    fn test_image_scaling_keeps_aspect_ratio() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;

        let image = DynamicImage::new_rgb8(100, 200);

        let object = page.objects_mut().create_image_object(
            PdfPoints::new(0.0),
            PdfPoints::new(0.0),
            &image,
            Some(PdfPoints::new(image.width() as f32)),
            Some(PdfPoints::new(image.height() as f32)),
        )?;

        let image_object = object.as_image_object().unwrap();

        assert_eq!(
            image_object.get_processed_bitmap_with_width(&document, 50)?.height(),
            100
        );
        assert_eq!(
            image_object.get_processed_image_with_width(&document, 50)?.height(),
            100
        );
        assert_eq!(
            image_object.get_processed_bitmap_with_height(&document, 50)?.width(),
            25
        );
        assert_eq!(image_object.get_processed_image_with_height(&document, 50)?.width(), 25);

        Ok(())
    }
}
