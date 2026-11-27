//! libheif linked at build time (no `dynamic` feature): the `libheif-sys` declarations, with
//! the root functions of `dynamic.rs` returning `crate::Result` like their runtime-loaded
//! wrappers, so every call site is the same in both builds. A root function missing here or
//! there fails to compile at its call sites in that build. Linked, they never fail.
#![allow(unsafe_code)]
#![allow(
    non_snake_case,
    reason = "parameters keep libheif's names, such as out_nPluginsLoaded"
)]

pub use libheif_sys::*;

macro_rules! roots {
    (@ret) => { () };
    (@ret $ret:ty) => { $ret };
    ($( $(#[$attr:meta])* fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)?; )*) => {
        $(
            $(#[$attr])*
            #[allow(dead_code, reason = "the root set covers every feature combination; each build calls a subset")]
            #[inline]
            pub unsafe fn $name($($arg: $ty),*) -> crate::Result<roots!(@ret $($ret)?)> {
                // SAFETY: The caller upholds the contract of the libheif function of this name.
                Ok(unsafe { libheif_sys::$name($($arg),*) })
            }
        )*
    };
}

roots! {
    fn heif_check_filetype(data: *const u8, len: libc::c_int) -> heif_filetype_result;
    fn heif_context_alloc() -> *mut heif_context;
    fn heif_context_get_encoder_for_format(
        context: *mut heif_context,
        format: heif_compression_format,
        arg1: *mut *mut heif_encoder,
    ) -> heif_error;
    fn heif_decoding_options_alloc() -> *mut heif_decoding_options;
    fn heif_deinit();
    fn heif_encoding_options_alloc() -> *mut heif_encoding_options;
    fn heif_get_decoder_descriptors(
        format_filter: heif_compression_format,
        out_decoders: *mut *const heif_decoder_descriptor,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_get_encoder_descriptors(
        format_filter: heif_compression_format,
        name_filter: *const libc::c_char,
        out_encoders: *mut *const heif_encoder_descriptor,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_get_version_number() -> u32;
    fn heif_image_create(
        width: libc::c_int,
        height: libc::c_int,
        colorspace: heif_colorspace,
        chroma: heif_chroma,
        out_image: *mut *mut heif_image,
    ) -> heif_error;
    fn heif_init(arg1: *mut heif_init_params) -> heif_error;
    fn heif_load_plugins(
        directory: *const libc::c_char,
        out_plugins: *mut *const heif_plugin_info,
        out_nPluginsLoaded: *mut libc::c_int,
        output_array_size: libc::c_int,
    ) -> heif_error;
    fn heif_nclx_color_profile_alloc() -> *mut heif_color_profile_nclx;
    #[cfg(feature = "v1_19")]
    fn heif_color_conversion_options_set_defaults(arg1: *mut heif_color_conversion_options);
    #[cfg(feature = "v1_19")]
    fn heif_get_global_security_limits() -> *const heif_security_limits;
    #[cfg(feature = "v1_20")]
    fn heif_color_conversion_options_ext_alloc() -> *mut heif_color_conversion_options_ext;
}
