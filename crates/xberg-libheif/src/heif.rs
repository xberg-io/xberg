use std::ffi::CString;
use std::path::Path;
use std::ptr;

use libheif_sys as lh;

use crate::decoder::get_decoding_options_ptr;
use crate::utils::path_to_cstring;
use crate::{
    ColorSpace, CompressionFormat, DecoderDescriptor, DecodingOptions, Encoder, EncoderDescriptor, HeifError, Image,
    ImageHandle, Result,
};

/// Guard structure used for `libheif` initialization, working with plugins,
/// getting encoders, decode images, etc.
///
/// You may use one instance of this structure for all parts of your code.
/// Methods of the structure use static variables inside `libheif`.
/// So any changes that they make in internals of `libheif`
/// will have side effect to all instances of the structure.
///
/// For example, if you load some plugins by one `LibHeif` instance,
/// all those plugins will be available to all other instances.
pub struct LibHeif(());

#[allow(unsafe_code)]
impl LibHeif {
    #[allow(unsafe_code)]
    pub fn new() -> Self {
        let mut init_params = lh::heif_init_params { version: 0 };
        // SAFETY: The local version-zero parameter block stays live through initialization.
        unsafe {
            lh::heif_init(&mut init_params as _);
        }
        Self(())
    }

    /// Use this method if you need to make sure that external plugins
    /// have loaded successfully.
    ///
    /// Default paths of directories with external plugins may be
    /// changed with help of environment variable `LIBHEIF_PLUGIN_PATH`.
    #[allow(unsafe_code)]
    pub fn new_checked() -> Result<Self> {
        let mut init_params = lh::heif_init_params { version: 0 };
        // SAFETY: The local version-zero parameter block stays live through initialization.
        let error = unsafe { lh::heif_init(&mut init_params as _) };
        HeifError::from_heif_error(error)?;
        Ok(Self(()))
    }
}

#[allow(unsafe_code)]
impl Drop for LibHeif {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY requirement: Deinitialization balances a successful initialization.
        // new_checked enforces success before constructing a guard; new ignores the result.
        unsafe {
            lh::heif_deinit();
        }
    }
}

#[allow(unsafe_code)]
impl Default for LibHeif {
    #[allow(unsafe_code)]
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unsafe_code)]
impl LibHeif {
    /// Version of linked `libheif` as an array of numbers in format:
    ///
    /// `[<major>, <minor>, <maintenance>]`
    #[allow(unsafe_code)]
    pub fn version(&self) -> [u8; 3] {
        // SAFETY: The version query takes no pointers and returns a value.
        let version: u32 = unsafe { lh::heif_get_version_number() };
        let parts = version.to_be_bytes();
        [parts[0], parts[1], parts[2]]
    }

    /// Load all plugins from given path of directory.
    ///
    /// Returns number of loaded plugins.
    #[allow(unsafe_code)]
    pub fn load_plugins(&self, dir_path: impl AsRef<Path>) -> Result<usize> {
        self._load_plugins(dir_path.as_ref())
    }

    #[allow(unsafe_code)]
    fn _load_plugins(&self, dir_path: &Path) -> Result<usize> {
        let dir_path = path_to_cstring(dir_path);
        let mut plugins_loaded: libc::c_int = 0;
        // SAFETY: The directory CString and count out slot live for the call. Passing
        // null with zero capacity requests no plugin-info output array.
        let err =
            unsafe { lh::heif_load_plugins(dir_path.as_ptr(), ptr::null_mut() as _, &mut plugins_loaded as _, 0) };
        HeifError::from_heif_error(err)?;
        Ok(plugins_loaded as usize)
    }

    /// Decode an image handle into the actual pixel image and also carry out
    /// all geometric transformations specified in the HEIF file (rotation, cropping, mirroring).
    ///
    /// If `color_space` is set to [`ColorSpace::Undefined`],
    /// respectively, the original colorspace is taken.
    #[allow(unsafe_code)]
    pub fn decode(
        &self,
        image_handle: &ImageHandle,
        color_space: ColorSpace,
        decoding_options: Option<DecodingOptions>,
    ) -> Result<Image> {
        let mut c_image: *mut lh::heif_image = ptr::null_mut();
        // SAFETY: The image handle and optional decoding settings outlive the call,
        // and c_image is a writable slot for the owned native result.
        let err = unsafe {
            lh::heif_decode_image(
                image_handle.inner,
                &mut c_image,
                color_space.heif_color_space(),
                color_space.heif_chroma(),
                get_decoding_options_ptr(&decoding_options),
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Image::from_heif_image(c_image))
    }

    /// Get a list of available decoders.
    /// You can filter the decoders by compression format.
    ///
    /// The returned list of decoders is sorted by their priority
    /// (which is a plugin property).
    #[allow(unsafe_code)]
    pub fn decoder_descriptors(
        &self,
        max_count: usize,
        format_filter: Option<CompressionFormat>,
    ) -> Vec<DecoderDescriptor<'_>> {
        let format_filter = format_filter.unwrap_or(CompressionFormat::Undefined);
        let max_count = max_count.min(libc::c_int::MAX as usize);

        let mut descriptors_ptr = Vec::with_capacity(max_count);
        // SAFETY: The Vec has capacity for max_count pointers, which fits in a C int.
        // Libheif returns the number of entries it initialized, bounded by max_count.
        unsafe {
            let count =
                lh::heif_get_decoder_descriptors(format_filter as _, descriptors_ptr.as_mut_ptr(), max_count as _);
            descriptors_ptr.set_len(count as usize);
        }

        descriptors_ptr
            .into_iter()
            // SAFETY: Non-null entries are library-owned descriptors. Their Rust borrows
            // are tied to this initialization guard, which keeps the registry available.
            .filter_map(|d_ptr| unsafe { d_ptr.as_ref().map(DecoderDescriptor::new) })
            .collect()
    }

    /// Get a list of available encoders.
    /// You can filter the encoders by compression format and name.
    ///
    /// The returned list of encoders is sorted by their priority
    /// (which is a plugin property).
    ///
    /// Note: to get the actual encoder from the descriptors returned here,
    /// use [`LibHeif::encoder`] method.
    #[allow(unsafe_code)]
    pub fn encoder_descriptors(
        &self,
        max_count: usize,
        format_filter: Option<CompressionFormat>,
        name_filter: Option<&str>,
    ) -> Vec<EncoderDescriptor<'_>> {
        let format_filter = format_filter.unwrap_or(CompressionFormat::Undefined);
        let max_count = max_count.min(libc::c_int::MAX as usize);
        let name_filter = name_filter.map(|s| CString::new(s).ok()).unwrap_or_default();
        let name_filter_ptr = name_filter.map(|s| s.as_ptr()).unwrap_or(ptr::null());

        let mut descriptors_ptr = Vec::with_capacity(max_count);
        // SAFETY requirement: Libheif writes at most max_count entries, which fits
        // the Vec. A present name filter must also remain live, but the consuming
        // map above drops its CString before this call.
        unsafe {
            let count = lh::heif_get_encoder_descriptors(
                format_filter as _,
                name_filter_ptr,
                descriptors_ptr.as_mut_ptr(),
                max_count as _,
            );
            descriptors_ptr.set_len(count as usize);
        }

        descriptors_ptr
            .into_iter()
            // SAFETY: Non-null entries are library-owned descriptors. Their Rust borrows
            // are tied to this initialization guard, which keeps the registry available.
            .filter_map(|d_ptr| unsafe { d_ptr.as_ref().map(EncoderDescriptor::new) })
            .collect()
    }

    /// Get an encoder instance that can be used to actually
    /// encode images from a descriptor.
    #[allow(unsafe_code)]
    pub fn encoder(&self, descriptor: EncoderDescriptor) -> Result<Encoder<'_>> {
        let mut c_encoder: *mut lh::heif_encoder = ptr::null_mut();
        // SAFETY: Libheif accepts a null context for this registry operation. The
        // descriptor is borrowed and c_encoder is a writable out slot.
        let err = unsafe { lh::heif_context_get_encoder(ptr::null_mut(), descriptor.inner, &mut c_encoder) };
        HeifError::from_heif_error(err)?;
        // SAFETY: Success returns a fresh encoder allocation. Encoder owns it if construction succeeds.
        let encoder = Encoder::new(unsafe { &mut *c_encoder })?;
        Ok(encoder)
    }

    /// Get an encoder for the given compression format.
    /// If there are several encoder plugins for this format,
    /// the encoder with the highest plugin priority will be returned.
    #[allow(unsafe_code)]
    pub fn encoder_for_format(&self, format: CompressionFormat) -> Result<Encoder<'_>> {
        let mut c_encoder: *mut lh::heif_encoder = ptr::null_mut();
        // SAFETY: Libheif accepts a null context for this registry operation and
        // can initialize the caller-owned out slot.
        let err = unsafe { lh::heif_context_get_encoder_for_format(ptr::null_mut() as _, format as _, &mut c_encoder) };
        HeifError::from_heif_error(err)?;
        // SAFETY: Success returns a fresh encoder allocation. Encoder owns it if construction succeeds.
        let encoder = Encoder::new(unsafe { &mut *c_encoder })?;
        Ok(encoder)
    }
}
