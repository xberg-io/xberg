use std::ffi::CString;
use std::fmt::{Debug, Formatter};
use std::ptr;
use std::sync::Mutex;

use libheif_sys as lh;

#[cfg(feature = "v1_20")]
use crate::AlphaCompositionMode;
use crate::utils::{cstr_to_str, str_to_cstring};
use crate::{ChromaDownsamplingAlgorithm, ChromaUpsamplingAlgorithm, ColorProfileNCLX, HeifError};
static DECODER_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct DecodingOptions {
    inner: ptr::NonNull<lh::heif_decoding_options>,
    decoder_id: Option<CString>,
    #[allow(dead_code)]
    output_image_nclx_profile: Option<ColorProfileNCLX>,
}

#[allow(unsafe_code)]
impl DecodingOptions {
    pub fn new() -> Option<Self> {
        // SAFETY: The allocator returns a `new` struct filled with defaults, including
        // `version`, which this wrapper owns until Drop once the result is non-null.
        let inner_ptr = unsafe { lh::heif_decoding_options_alloc() };
        ptr::NonNull::new(inner_ptr).map(|inner| Self {
            inner,
            decoder_id: None,
            output_image_nclx_profile: None,
        })
    }
}

#[allow(unsafe_code)]
impl Drop for DecodingOptions {
    fn drop(&mut self) {
        #[cfg(feature = "v1_20")]
        {
            let inner_mut = self.inner_mut();
            if !inner_mut.color_conversion_options_ext.is_null() {
                // SAFETY: The only write of a non-null extension is set_alpha_composition_mode,
                // which stores heif_color_conversion_options_ext_alloc's result, so these options
                // own it. heif_decoding_options_free does not free it, and the pointer is cleared
                // after this free, so it is freed exactly once.
                unsafe { lh::heif_color_conversion_options_ext_free(inner_mut.color_conversion_options_ext) };
                inner_mut.color_conversion_options_ext = ptr::null_mut();
            }
        }
        // SAFETY: These options are the sole owner of the allocation, and
        // heif_decoding_options_free is the `delete` matching the alloc's `new`. Under v1_20
        // the block above has freed the extension; without v1_20 the field is not in the
        // bindings and stays null, because the alloc sets it and this code never writes it.
        unsafe {
            lh::heif_decoding_options_free(self.inner.as_ptr());
        }
    }
}

#[allow(unsafe_code)]
impl DecodingOptions {
    #[inline(always)]
    #[allow(unsafe_code)]
    fn inner_ref(&self) -> &lh::heif_decoding_options {
        // SAFETY: The pointer is the initialized allocation from heif_decoding_options_alloc,
        // which self keeps alive for this shared borrow. The reference is sized to the
        // bindings for the highest enabled `v1_*` feature, so the runtime libheif must have
        // allocated at least that struct version. libheif-sys's pkg-config floor checks the
        // libheif version only at build time, libheif only appends fields, and nothing checks
        // `version` at runtime: a runtime older than the build floor is unsupported.
        unsafe { self.inner.as_ref() }
    }

    #[inline(always)]
    #[allow(unsafe_code)]
    pub(crate) fn inner_mut(&mut self) -> &mut lh::heif_decoding_options {
        // SAFETY: The pointer is the initialized allocation from heif_decoding_options_alloc,
        // which self owns, and &mut self grants exclusive access. The layout holds only for
        // a runtime libheif at or above the build floor, as inner_ref explains.
        unsafe { self.inner.as_mut() }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        self.inner_ref().version
    }

    /// Ignore geometric transformations like cropping, rotation, mirroring.
    /// Default: false (do not ignore).
    #[inline]
    pub fn ignore_transformations(&self) -> bool {
        self.inner_ref().ignore_transformations != 0
    }

    #[inline]
    pub fn set_ignore_transformations(&mut self, enable: bool) {
        self.inner_mut().ignore_transformations = if enable { 1 } else { 0 }
    }

    #[inline]
    pub fn convert_hdr_to_8bit(&self) -> bool {
        self.inner_ref().convert_hdr_to_8bit != 0
    }

    #[inline]
    pub fn set_convert_hdr_to_8bit(&mut self, enable: bool) {
        self.inner_mut().convert_hdr_to_8bit = if enable { 1 } else { 0 }
    }

    /// When strict decoding is enabled, an error is returned for invalid input.
    /// Otherwise, it will try its best and add decoding warnings to
    /// the decoded `Image`. Default is non-strict.
    pub fn strict_decoding(&self) -> bool {
        self.inner_ref().strict_decoding != 0
    }

    pub fn set_strict_decoding(&mut self, enable: bool) {
        self.inner_mut().strict_decoding = if enable { 1 } else { 0 }
    }

    /// ID of the decoder to use for the decoding.
    /// If set to `None` (default), the highest priority decoder is chosen.
    /// The priority is defined in the plugin.
    pub fn decoder_id(&self) -> Option<&str> {
        cstr_to_str(self.inner_ref().decoder_id)
    }

    pub fn set_decoder_id(&mut self, decoder_id: Option<&str>) -> Result<(), HeifError> {
        if let Some(decoder_id) = decoder_id {
            let c_decoder_id = str_to_cstring(decoder_id, "decoder_id")?;
            self.inner_mut().decoder_id = c_decoder_id.as_ptr();
            self.decoder_id = Some(c_decoder_id);
        } else {
            self.inner_mut().decoder_id = ptr::null() as _;
            self.decoder_id = None;
        }
        Ok(())
    }

    pub fn color_conversion_options(&self) -> ColorConversionOptions {
        let inner = self.inner_ref();
        ColorConversionOptions::from_cc_options(&inner.color_conversion_options)
    }

    pub fn set_color_conversion_options(&mut self, options: ColorConversionOptions) {
        let inner = self.inner_mut();
        options.fill_cc_options(&mut inner.color_conversion_options);
    }

    #[cfg(feature = "v1_20")]
    pub fn alpha_composition_mode(&self) -> AlphaCompositionMode {
        let inner = self.inner_ref();
        AlphaCompositionMode::from_libheif(inner.color_conversion_options_ext)
    }

    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn set_alpha_composition_mode(&mut self, v: AlphaCompositionMode) {
        let inner = self.inner_mut();
        if inner.color_conversion_options_ext.is_null() {
            // SAFETY: The allocator returns a `new` extension filled with defaults, or null.
            // A non-null result is owned by these options and freed in Drop.
            inner.color_conversion_options_ext = unsafe { lh::heif_color_conversion_options_ext_alloc() };
        }
        v.fill_libheif_cc_options_ext(inner.color_conversion_options_ext);
    }

    #[cfg(feature = "v1_21")]
    /// If enabled, it will decode the media timeline,
    /// ignoring the sequence tracks edit-list.
    pub fn ignore_sequence_edit_list(&self) -> bool {
        let inner = self.inner_ref();
        inner.ignore_sequence_editlist != 0
    }

    #[cfg(feature = "v1_21")]
    /// If enabled, it will decode the media timeline,
    /// ignoring the sequence tracks edit-list.
    pub fn set_ignore_sequence_edit_list(&mut self, v: bool) {
        let inner = self.inner_mut();
        inner.ignore_sequence_editlist = v as _;
    }

    #[cfg(feature = "v1_21")]
    pub fn output_image_nclx_profile(&self) -> Option<&ColorProfileNCLX> {
        self.output_image_nclx_profile.as_ref()
    }

    #[cfg(feature = "v1_21")]
    pub fn set_output_image_nclx_profile(&mut self, v: Option<ColorProfileNCLX>) {
        self.output_image_nclx_profile = v;
        let profile_ptr = self
            .output_image_nclx_profile
            .as_ref()
            .map(|v| v.inner)
            .unwrap_or(ptr::null_mut());
        let inner = self.inner_mut();
        inner.output_image_nclx_profile = profile_ptr;
    }

    #[cfg(feature = "v1_21")]
    /// 0 lets libheif decide.
    pub fn num_library_threads(&self) -> u32 {
        let inner = self.inner_ref();
        inner.num_library_threads.max(0) as _
    }

    #[cfg(feature = "v1_21")]
    /// 0 lets libheif decide.
    pub fn set_num_library_threads(&mut self, v: u32) {
        let inner = self.inner_mut();
        inner.num_library_threads = v.min(i32::MAX as u32) as _;
    }

    #[cfg(feature = "v1_21")]
    /// 0 = use decoder default
    pub fn num_codec_threads(&self) -> u32 {
        let inner = self.inner_ref();
        inner.num_codec_threads.max(0) as _
    }

    #[cfg(feature = "v1_21")]
    /// 0 = use decoder default
    pub fn set_num_codec_threads(&mut self, v: u32) {
        let inner = self.inner_mut();
        inner.num_codec_threads = v.min(i32::MAX as u32) as _;
    }
}

/// This function makes sure the decoding options
/// won't be freed too early.
pub(crate) fn get_decoding_options_ptr(options: &Option<DecodingOptions>) -> *mut lh::heif_decoding_options {
    options.as_ref().map(|o| o.inner.as_ptr()).unwrap_or_else(ptr::null_mut)
}

#[derive(Debug, Copy, Clone)]
pub struct ColorConversionOptions {
    pub preferred_chroma_downsampling_algorithm: ChromaDownsamplingAlgorithm,
    pub preferred_chroma_upsampling_algorithm: ChromaUpsamplingAlgorithm,
    /// When set to `false`, libheif may also use a different algorithm
    /// if the preferred one is not available.
    pub only_use_preferred_chroma_algorithm: bool,
}

#[allow(unsafe_code)]
impl Default for ColorConversionOptions {
    fn default() -> Self {
        #[allow(unused_mut)]
        let mut cc_options = lh::heif_color_conversion_options {
            version: 1,
            preferred_chroma_downsampling_algorithm: 0,
            preferred_chroma_upsampling_algorithm: 0,
            only_use_preferred_chroma_algorithm: 0,
        };
        #[cfg(feature = "v1_19")]
        {
            // SAFETY: cc_options is a writable, initialized version-1 structure that
            // stays live while libheif sets its default fields.
            unsafe { lh::heif_color_conversion_options_set_defaults(&mut cc_options) };
        }
        Self::from_cc_options(&cc_options)
    }
}

impl ColorConversionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn from_cc_options(cc_options: &lh::heif_color_conversion_options) -> Self {
        let preferred_chroma_downsampling_algorithm =
            ChromaDownsamplingAlgorithm::n(cc_options.preferred_chroma_downsampling_algorithm)
                .unwrap_or(ChromaDownsamplingAlgorithm::Average);
        let preferred_chroma_upsampling_algorithm =
            ChromaUpsamplingAlgorithm::n(cc_options.preferred_chroma_upsampling_algorithm)
                .unwrap_or(ChromaUpsamplingAlgorithm::Bilinear);
        let only_use_preferred_chroma_algorithm = cc_options.only_use_preferred_chroma_algorithm != 0;
        Self {
            preferred_chroma_downsampling_algorithm,
            preferred_chroma_upsampling_algorithm,
            only_use_preferred_chroma_algorithm,
        }
    }

    pub(crate) fn fill_cc_options(&self, cc_options: &mut lh::heif_color_conversion_options) {
        cc_options.preferred_chroma_downsampling_algorithm = self.preferred_chroma_downsampling_algorithm as _;
        cc_options.preferred_chroma_upsampling_algorithm = self.preferred_chroma_upsampling_algorithm as _;
        cc_options.only_use_preferred_chroma_algorithm = self.only_use_preferred_chroma_algorithm as _;
    }
}

#[derive(Copy, Clone)]
pub struct DecoderDescriptor<'a> {
    inner: &'a lh::heif_decoder_descriptor,
}

#[allow(unsafe_code)]
impl<'a> DecoderDescriptor<'a> {
    pub(crate) fn new(inner: &'a lh::heif_decoder_descriptor) -> Self {
        Self { inner }
    }

    /// A short, symbolic name for identifying the decoder.
    /// This name should stay constant over different decoder versions.
    #[allow(unsafe_code)]
    pub fn id(&self) -> &str {
        // SAFETY: The descriptor is a plugin registered with libheif, and the borrow of
        // LibHeif behind 'a keeps libheif initialized, so the plugin stays loaded. The call
        // returns the plugin's static id_name, or null for plugin API versions below 3.
        let name = unsafe { lh::heif_decoder_descriptor_get_id_name(self.inner) };
        cstr_to_str(name).unwrap_or_default()
    }

    /// A long, descriptive name of the decoder
    /// (including version information).
    #[allow(unsafe_code)]
    pub fn name(&self) -> String {
        let _lock = DECODER_MUTEX.lock();
        // SAFETY: The descriptor stays loaded for 'a, as in id(). get_plugin_name may write
        // a buffer shared by all calls, so the call and the copy below run under
        // DECODER_MUTEX and the result is owned before the lock is released.
        let name = unsafe { lh::heif_decoder_descriptor_get_name(self.inner) };
        cstr_to_str(name).unwrap_or_default().to_owned()
    }
}

impl<'a> Debug for DecoderDescriptor<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderDescriptor")
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
