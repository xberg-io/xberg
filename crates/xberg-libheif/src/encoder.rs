use std::collections::HashMap;
use std::ffi::CString;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr;
use std::sync::Mutex;

use crate::lh;

use crate::utils::cstr_to_str;
use crate::{ColorConversionOptions, HeifError, HeifErrorCode, HeifErrorSubCode, ImageOrientation, Result};

static ENCODER_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[non_exhaustive]
#[repr(C)]
pub enum CompressionFormat {
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_undefined]
    Undefined = lh::heif_compression_format_heif_compression_undefined as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_HEVC]
    Hevc = lh::heif_compression_format_heif_compression_HEVC as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_AVC]
    Avc = lh::heif_compression_format_heif_compression_AVC as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_JPEG]
    Jpeg = lh::heif_compression_format_heif_compression_JPEG as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_AV1]
    Av1 = lh::heif_compression_format_heif_compression_AV1 as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_VVC]
    Vvc = lh::heif_compression_format_heif_compression_VVC as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_EVC]
    Evc = lh::heif_compression_format_heif_compression_EVC as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_JPEG2000]
    Jpeg2000 = lh::heif_compression_format_heif_compression_JPEG2000 as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_uncompressed]
    Uncompressed = lh::heif_compression_format_heif_compression_uncompressed as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_mask]
    Mask = lh::heif_compression_format_heif_compression_mask as _,
    /// Rust equivalent of [lh::heif_compression_format_heif_compression_HTJ2K]
    #[cfg(feature = "v1_18")]
    HtJ2k = lh::heif_compression_format_heif_compression_HTJ2K as _,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum EncoderParameterType {
    Int = lh::heif_encoder_parameter_type_heif_encoder_parameter_type_integer as _,
    Bool = lh::heif_encoder_parameter_type_heif_encoder_parameter_type_boolean as _,
    String = lh::heif_encoder_parameter_type_heif_encoder_parameter_type_string as _,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EncoderParameterValue {
    Int(i32),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EncoderQuality {
    LossLess,
    /// Value inside variant is a 'quality' factor (0-100).
    /// How this is mapped to actual encoding parameters is encoder dependent.
    Lossy(u8),
}

pub type EncoderParametersTypes = HashMap<String, EncoderParameterType>;

pub struct Encoder<'a> {
    pub(crate) inner: *mut lh::heif_encoder,
    pub(crate) parameters_types: EncoderParametersTypes,
    phantom: PhantomData<&'a mut lh::heif_encoder>,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(c_encoder: &'a mut lh::heif_encoder) -> Result<Self> {
        let parameters_types = parameters_types(c_encoder)?;
        Ok(Self {
            inner: c_encoder,
            parameters_types,
            phantom: PhantomData,
        })
    }
}

#[allow(unsafe_code)]
impl<'a> Drop for Encoder<'a> {
    fn drop(&mut self) {
        // SAFETY: `inner` is the encoder libheif allocated for this wrapper and released exactly once here;
        // nothing else owns it.
        unsafe { lh::heif_encoder_release(self.inner) };
    }
}

#[allow(unsafe_code)]
impl<'a> Encoder<'a> {
    /// Name of encoder.
    #[allow(unsafe_code)]
    pub fn name(&self) -> String {
        let _lock = ENCODER_MUTEX.lock();
        // SAFETY: The mutex serializes access to the encoder's plugin state and `inner` stays live for the
        // borrow; the returned name is owned by the encoder and copied before the lock is released.
        let res = unsafe { lh::heif_encoder_get_name(self.inner) };
        cstr_to_str(res).unwrap_or("").to_owned()
    }

    #[allow(unsafe_code)]
    pub fn set_quality(&mut self, quality: EncoderQuality) -> Result<()> {
        let err = match quality {
            // SAFETY: `self` is exclusively borrowed and the flag is passed by value into the live encoder.
            EncoderQuality::LossLess => unsafe { lh::heif_encoder_set_lossless(self.inner, 1) },
            // SAFETY: `self` is exclusively borrowed; the lossless flag is cleared first and the quality is
            // only set once that call succeeded.
            EncoderQuality::Lossy(value) => unsafe {
                let middle_err = lh::heif_encoder_set_lossless(self.inner, 0);
                HeifError::from_heif_error(middle_err)?;
                lh::heif_encoder_set_lossy_quality(self.inner, i32::from(value))
            },
        };
        HeifError::from_heif_error(err)
    }

    #[allow(unsafe_code)]
    fn parameter_value(&self, name: &str, parameter_type: EncoderParameterType) -> Result<EncoderParameterValue> {
        let c_param_name = CString::new(name).unwrap();
        let param_value = match parameter_type {
            EncoderParameterType::Int => {
                let mut value = 0;
                // SAFETY: `c_param_name` keeps its NUL-terminated buffer for the call and the out slot is an
                // initialized `c_int`; `inner` is live for the shared borrow.
                let err = unsafe {
                    lh::heif_encoder_get_parameter_integer(self.inner, c_param_name.as_ptr(), &mut value as _)
                };
                HeifError::from_heif_error(err)?;
                EncoderParameterValue::Int(value)
            }
            EncoderParameterType::Bool => {
                let mut value = 0;
                // SAFETY: `c_param_name` keeps its NUL-terminated buffer for the call and the out slot is an
                // initialized `c_int`; `inner` is live for the shared borrow.
                let err = unsafe {
                    lh::heif_encoder_get_parameter_boolean(self.inner, c_param_name.as_ptr(), &mut value as _)
                };
                HeifError::from_heif_error(err)?;
                EncoderParameterValue::Bool(value > 0)
            }
            EncoderParameterType::String => {
                let value: Vec<u8> = vec![0; 51];
                // SAFETY: `value` is a 51-byte buffer and libheif is told the writable capacity is 50, so it
                // writes at most 50 characters plus its own NUL terminator; `c_param_name` outlives the
                // call.
                let err = unsafe {
                    lh::heif_encoder_get_parameter_string(self.inner, c_param_name.as_ptr(), value.as_ptr() as _, 50)
                };
                HeifError::from_heif_error(err)?;
                EncoderParameterValue::String(cstr_to_str(value.as_ptr() as _).unwrap_or("").to_string())
            }
        };

        Ok(param_value)
    }

    pub fn parameters_names(&self) -> Vec<String> {
        self.parameters_types.keys().cloned().collect()
    }

    /// Get value of encoder's parameter.
    pub fn parameter(&self, name: &str) -> Result<Option<EncoderParameterValue>> {
        match self.parameters_types.get(name) {
            Some(param_type) => {
                let value = self.parameter_value(name, *param_type)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set value of encoder's parameter.
    #[allow(unsafe_code)]
    pub fn set_parameter_value(&self, name: &str, value: EncoderParameterValue) -> Result<()> {
        let c_param_name = CString::new(name).unwrap();
        let err = match value {
            // SAFETY: `c_param_name` keeps its NUL-terminated buffer for the call and the value is a scalar.
            // The encoder is mutated through its live native pointer.
            EncoderParameterValue::Bool(v) => unsafe {
                lh::heif_encoder_set_parameter_boolean(self.inner, c_param_name.as_ptr(), v.into())
            },
            // SAFETY: `c_param_name` keeps its NUL-terminated buffer for the call and the value is a scalar.
            // The encoder is mutated through its live native pointer.
            EncoderParameterValue::Int(v) => unsafe {
                lh::heif_encoder_set_parameter_integer(self.inner, c_param_name.as_ptr(), v)
            },
            // SAFETY: Both C strings keep their buffers for the call, during which libheif copies the value;
            // the encoder is mutated through its live native pointer.
            EncoderParameterValue::String(v) => unsafe {
                let c_param_value = CString::new(v).unwrap();
                lh::heif_encoder_set_parameter_string(self.inner, c_param_name.as_ptr(), c_param_value.as_ptr())
            },
        };
        HeifError::from_heif_error(err)?;
        Ok(())
    }
}

#[allow(unsafe_code)]
fn parameters_types(c_encoder: &mut lh::heif_encoder) -> Result<EncoderParametersTypes> {
    let mut res = EncoderParametersTypes::new();
    // SAFETY: `c_encoder` is exclusively borrowed by the caller, and the returned array is libheif's
    // own, terminated by a null entry; it stays valid while the encoder lives, and every name
    // is copied into a `String` before the walk advances.
    unsafe {
        let mut param_pointers = lh::heif_encoder_list_parameters(c_encoder);
        if !param_pointers.is_null() {
            while let Some(raw_param) = (*param_pointers).as_ref() {
                let c_param_type = lh::heif_encoder_parameter_get_type(raw_param);
                let param_type = match EncoderParameterType::n(c_param_type) {
                    Some(res) => res,
                    None => {
                        return Err(HeifError {
                            code: HeifErrorCode::EncoderPluginError,
                            sub_code: HeifErrorSubCode::UnsupportedParameter,
                            message: format!("{} is unknown type of parameter", c_param_type),
                        });
                    }
                };
                let c_param_name = lh::heif_encoder_parameter_get_name(raw_param);
                let name = cstr_to_str(c_param_name).unwrap_or("").to_string();
                res.insert(name, param_type);
                param_pointers = param_pointers.offset(1);
            }
        }
    }
    Ok(res)
}

#[derive(Debug)]
pub struct EncodingOptions {
    inner: ptr::NonNull<lh::heif_encoding_options>,
}

#[allow(unsafe_code)]
impl EncodingOptions {
    pub fn new() -> Result<Self> {
        // SAFETY: The allocator returns a fully defaulted struct, including `version`,
        // which this wrapper owns once the result is non-null.
        let inner_ptr = unsafe { lh::heif_encoding_options_alloc() };
        match ptr::NonNull::new(inner_ptr) {
            Some(inner) => Ok(Self { inner }),
            None => Err(HeifError {
                code: HeifErrorCode::MemoryAllocationError,
                sub_code: HeifErrorSubCode::Unspecified,
                message: Default::default(),
            }),
        }
    }
}

impl Default for EncodingOptions {
    fn default() -> Self {
        Self::new().expect("heif_encoding_options_alloc() returns a null pointer")
    }
}

#[allow(unsafe_code)]
impl Drop for EncodingOptions {
    fn drop(&mut self) {
        // SAFETY: This wrapper is the sole owner of the allocation and is not Clone, so it
        // is freed once; heif_encoding_options_free is the `delete` matching the `new` in
        // heif_encoding_options_alloc. The raw pointer from get_encoding_options_ptr does
        // not outlive it: every caller in context.rs takes EncodingOptions by value and
        // passes the pointer to one libheif encode call, which copies the options and
        // does not retain the pointer.
        unsafe {
            lh::heif_encoding_options_free(self.inner.as_ptr());
        }
    }
}

#[allow(unsafe_code)]
impl EncodingOptions {
    #[inline(always)]
    #[allow(unsafe_code)]
    fn inner_ref(&self) -> &lh::heif_encoding_options {
        // SAFETY: The pointer is the initialized allocation from heif_encoding_options_alloc,
        // which self keeps alive for this shared borrow. The libheif-sys bindings follow
        // the highest enabled `v1_*` feature, and the build requires libheif at or above
        // that version. libheif only appends fields, so a runtime at or above the build
        // floor allocates at least this layout; older runtimes are unsupported.
        unsafe { self.inner.as_ref() }
    }

    #[inline(always)]
    #[allow(unsafe_code)]
    fn inner_mut(&mut self) -> &mut lh::heif_encoding_options {
        // SAFETY: The pointer is the initialized allocation from heif_encoding_options_alloc,
        // which self owns, and &mut self grants exclusive access. The layout holds for
        // the reason given in inner_ref: the runtime libheif is at or above the build floor.
        unsafe { self.inner.as_mut() }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        self.inner_ref().version
    }

    #[inline]
    pub fn save_alpha_channel(&self) -> bool {
        self.inner_ref().save_alpha_channel != 0
    }

    #[inline]
    pub fn set_save_alpha_channel(&mut self, enable: bool) {
        self.inner_mut().save_alpha_channel = if enable { 1 } else { 0 };
    }

    #[inline]
    pub fn mac_os_compatibility_workaround(&self) -> bool {
        self.inner_ref().macOS_compatibility_workaround != 0
    }

    #[inline]
    pub fn set_mac_os_compatibility_workaround(&mut self, enable: bool) {
        self.inner_mut().macOS_compatibility_workaround = if enable { 1 } else { 0 };
    }

    #[inline]
    pub fn save_two_colr_boxes_when_icc_and_nclx_available(&self) -> bool {
        self.inner_ref().save_two_colr_boxes_when_ICC_and_nclx_available != 0
    }

    #[inline]
    pub fn set_save_two_colr_boxes_when_icc_and_nclx_available(&mut self, enable: bool) {
        self.inner_mut().save_two_colr_boxes_when_ICC_and_nclx_available = if enable { 1 } else { 0 };
    }

    #[inline]
    pub fn mac_os_compatibility_workaround_no_nclx_profile(&self) -> bool {
        self.inner_ref().macOS_compatibility_workaround_no_nclx_profile != 0
    }

    #[inline]
    pub fn set_mac_os_compatibility_workaround_no_nclx_profile(&mut self, enable: bool) {
        self.inner_mut().macOS_compatibility_workaround_no_nclx_profile = if enable { 1 } else { 0 };
    }

    #[inline]
    pub fn image_orientation(&self) -> ImageOrientation {
        let orientation = self.inner_ref().image_orientation;
        ImageOrientation::n(orientation).unwrap_or(ImageOrientation::Normal)
    }

    #[inline]
    pub fn set_image_orientation(&mut self, orientation: ImageOrientation) {
        self.inner_mut().image_orientation = orientation as _;
    }

    pub fn color_conversion_options(&self) -> ColorConversionOptions {
        let lh_options = self.inner_ref().color_conversion_options;
        ColorConversionOptions::from_cc_options(&lh_options)
    }

    pub fn set_color_conversion_options(&mut self, options: ColorConversionOptions) {
        let lh_options = &mut self.inner_mut().color_conversion_options;
        options.fill_cc_options(lh_options);
    }
}

/// Returns the raw options pointer, or null for `None`.
///
/// The pointer is valid only while `options` is alive. The caller must keep `options`
/// alive across the libheif call that receives it.
pub(crate) fn get_encoding_options_ptr(options: &Option<EncodingOptions>) -> *mut lh::heif_encoding_options {
    options.as_ref().map(|o| o.inner.as_ptr()).unwrap_or_else(ptr::null_mut)
}

#[derive(Copy, Clone)]
pub struct EncoderDescriptor<'a> {
    pub(crate) inner: &'a lh::heif_encoder_descriptor,
}

#[allow(unsafe_code)]
impl<'a> EncoderDescriptor<'a> {
    pub(crate) fn new(inner: &'a lh::heif_encoder_descriptor) -> Self {
        Self { inner }
    }

    /// A short, symbolic name for identifying the encoder.
    /// This name should stay constant over different encoder versions.
    #[allow(unsafe_code)]
    pub fn id(&self) -> &str {
        // SAFETY: The descriptor and the name string it points into are owned by libheif's encoder
        // registry, which outlives the descriptor borrow `'a`; the result borrows `&self`.
        let name = unsafe { lh::heif_encoder_descriptor_get_id_name(self.inner) };
        cstr_to_str(name).unwrap_or_default()
    }

    /// A long, descriptive name of the encoder
    /// (including version information).
    #[allow(unsafe_code)]
    pub fn name(&self) -> String {
        let _lock = ENCODER_MUTEX.lock();
        // SAFETY: The mutex serializes registry access, the descriptor and its name outlive the `'a`
        // borrow, and the name is copied into a `String` before the lock is released.
        let name = unsafe { lh::heif_encoder_descriptor_get_name(self.inner) };
        cstr_to_str(name).unwrap_or_default().to_owned()
    }

    #[allow(unsafe_code)]
    pub fn compression_format(&self) -> CompressionFormat {
        // SAFETY: The descriptor is borrowed from libheif's registry for `'a`; the call only reads its
        // format enum.
        let c_format = unsafe { lh::heif_encoder_descriptor_get_compression_format(self.inner) };
        match CompressionFormat::n(c_format) {
            Some(res) => res,
            None => CompressionFormat::Undefined,
        }
    }

    #[allow(unsafe_code)]
    pub fn supports_lossy_compression(&self) -> bool {
        // SAFETY: The descriptor is borrowed from libheif's registry for `'a`; the call only reads a
        // capability flag.
        unsafe { lh::heif_encoder_descriptor_supports_lossy_compression(self.inner) != 0 }
    }

    #[allow(unsafe_code)]
    pub fn supports_lossless_compression(&self) -> bool {
        // SAFETY: The descriptor is borrowed from libheif's registry for `'a`; the call only reads a
        // capability flag.
        unsafe { lh::heif_encoder_descriptor_supports_lossless_compression(self.inner) != 0 }
    }
}

impl<'a> Debug for EncoderDescriptor<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncoderDescriptor")
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
