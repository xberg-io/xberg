//! libheif opened at run time instead of linked (`dynamic` feature).
//!
//! The crate reaches libheif only through `crate::lh`. With this feature that is this module:
//! the types and constants of `libheif-sys`, and a wrapper for every libheif function the crate
//! calls. A wrapper shadows the `libheif-sys` declaration of the same name, so no libheif symbol
//! is referenced at link time and the dynamic loader does not map libheif and its codecs at
//! process start. The library is opened, and every function below resolved, the first time
//! [`load`] runs. Every public entry point that reaches libheif without a context or an image
//! checks it first (`crate::ensure_libheif`), so a missing library is returned as an error; a
//! wrapper called while the library is not loadable panics.
//!
//! The search is the one `xberg`'s `ort_discovery::library` uses for ONNX Runtime: an absolute
//! override, the executable's directory, `<prefix>/lib` for a `<prefix>/bin` executable, fixed
//! system paths, each discovered file passing [`trusted_library`]. The feature exists for the
//! release CLI, which ships libheif beside the binary, so there is no fallback to the platform
//! loader's search path.
#![allow(unsafe_code)]
#![allow(
    non_snake_case,
    reason = "wrappers keep libheif's names, such as heif_context_add_XMP_metadata"
)]

use std::ffi::{CString, OsStr};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use libheif_sys::*;

#[cfg(not(unix))]
compile_error!("the `dynamic` feature of xberg-libheif loads libheif with dlopen and supports Unix targets only");

/// Explicit libheif library to load; authoritative when set and non-empty.
pub(crate) const LIBHEIF_PATH_ENV_VAR: &str = "XBERG_LIBHEIF_PATH";

/// File names the library ships under, searched in this order within each directory.
#[cfg(target_os = "macos")]
const LIBRARY_NAMES: &[&str] = &["libheif.1.dylib", "libheif.dylib"];
#[cfg(not(target_os = "macos"))]
const LIBRARY_NAMES: &[&str] = &["libheif.so.1", "libheif.so"];

/// System install locations, searched after the executable's own layout.
#[cfg(target_os = "macos")]
const SYSTEM_CANDIDATES: &[&str] = &["/opt/homebrew/lib/libheif.1.dylib", "/usr/local/lib/libheif.1.dylib"];
#[cfg(not(target_os = "macos"))]
const SYSTEM_CANDIDATES: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libheif.so.1",
    "/usr/lib/aarch64-linux-gnu/libheif.so.1",
    "/usr/lib64/libheif.so.1",
    "/usr/lib/libheif.so.1",
    "/usr/local/lib/libheif.so.1",
];

static API: OnceLock<Result<Api, String>> = OnceLock::new();

/// Open libheif and resolve every function this module wraps, once per process.
///
/// The outcome, success or failure, is cached, so a missing library costs one search.
pub(crate) fn load() -> Result<(), &'static str> {
    API.get_or_init(open_library)
        .as_ref()
        .map(|_| ())
        .map_err(String::as_str)
}

/// [`load`] as a [`crate::HeifError`], for the entry points that return one.
pub(crate) fn ensure_loaded() -> crate::Result<()> {
    load().map_err(|reason| crate::HeifError {
        code: crate::HeifErrorCode::PluginLoadingError,
        sub_code: crate::HeifErrorSubCode::PluginLoadingError,
        message: reason.to_string(),
    })
}

fn api() -> &'static Api {
    match API.get_or_init(open_library) {
        Ok(api) => api,
        Err(reason) => panic!("libheif was called before a successful load: {reason}"),
    }
}

fn open_library() -> Result<Api, String> {
    if let Some(explicit) = explicit_library(std::env::var_os(LIBHEIF_PATH_ENV_VAR).as_deref()) {
        return open_path(&explicit?);
    }
    let executable = running_executable();
    open_first(&library_candidates(executable.as_deref(), SYSTEM_CANDIDATES))
}

/// `XBERG_LIBHEIF_PATH` when set and non-empty: the only library tried, so a mistyped override
/// fails instead of loading another libheif. A relative value is refused, since the loader would
/// resolve it against the current directory.
fn explicit_library(explicit: Option<&OsStr>) -> Option<Result<PathBuf, String>> {
    let path = PathBuf::from(explicit.filter(|value| !value.is_empty())?);
    Some(if path.is_absolute() {
        Ok(path)
    } else {
        Err(format!(
            "{LIBHEIF_PATH_ENV_VAR} is the relative path `{}`; set it to the absolute path of libheif",
            path.display()
        ))
    })
}

/// The running executable with symlinks resolved, so a symlinked install still finds the
/// library shipped beside the real binary.
fn running_executable() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    Some(std::fs::canonicalize(&executable).unwrap_or(executable))
}

/// The libraries to look for, in order, without touching the file system: [`LIBRARY_NAMES`] in
/// the executable's directory, then in `<prefix>/lib` when that directory is `<prefix>/bin`,
/// then `system`. The parent of any other directory may be shared, as `/tmp` is for
/// `/tmp/x/xberg`, and is not searched. Every candidate is absolute, so neither the current
/// directory nor the loader's search path (`DYLD_*`, `LD_LIBRARY_PATH`) can supply the library.
fn library_candidates(executable: Option<&Path>, system: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(directory) = executable.and_then(Path::parent) {
        let mut directories = vec![directory.to_path_buf()];
        if directory.file_name() == Some(OsStr::new("bin"))
            && let Some(prefix) = directory.parent()
        {
            directories.push(prefix.join("lib"));
        }
        for directory in &directories {
            candidates.extend(LIBRARY_NAMES.iter().map(|name| directory.join(name)));
        }
    }
    candidates.extend(system.iter().map(PathBuf::from));
    candidates
}

/// Load the first candidate that exists, passes [`trusted_library`] and opens with every wrapped
/// function. A candidate that fails either check is skipped; when none loads, the first load
/// failure is reported, else every path searched and every file skipped.
fn open_first(candidates: &[PathBuf]) -> Result<Api, String> {
    let mut skipped = Vec::new();
    let mut first_failure = None;
    for candidate in candidates.iter().filter(|candidate| candidate.is_file()) {
        let real = match trusted_library(candidate) {
            Ok(real) => real,
            Err(reason) => {
                skipped.push(format!("`{}` ({reason})", candidate.display()));
                continue;
            }
        };
        match open_path(&real) {
            Ok(api) => return Ok(api),
            Err(failure) => {
                if first_failure.is_none() {
                    first_failure = Some(failure);
                }
            }
        }
    }
    if let Some(failure) = first_failure {
        return Err(failure);
    }
    let searched: Vec<String> = candidates.iter().map(|path| format!("`{}`", path.display())).collect();
    let skipped = if skipped.is_empty() {
        String::new()
    } else {
        format!("; skipped {}, which another user could have placed", skipped.join(", "))
    };
    Err(format!(
        "libheif not found; searched {}{skipped}. Reinstall the release archive, which ships libheif next to \
         the xberg executable, or set {LIBHEIF_PATH_ENV_VAR} to the absolute path of libheif",
        searched.join(", ")
    ))
}

/// The library at `path` with symlinks resolved, if another local user cannot have placed or
/// replaced it: the file and the directory that holds it must be owned by root or the effective
/// user and not writable by group or others. The resolved path is what gets opened, so a
/// symlink swapped after this check is not followed. Ancestor directories are not checked; a
/// shared one such as `/tmp` is sticky, so another user cannot rename what is below it.
///
/// `xberg`'s `ort_discovery::library::trusted_library` applies the same rule; keep the two in
/// step. ~keep
fn trusted_library(path: &Path) -> Result<PathBuf, String> {
    use std::os::unix::fs::MetadataExt;

    let real = std::fs::canonicalize(path).map_err(|error| format!("cannot resolve it: {error}"))?;
    // SAFETY: geteuid takes no arguments, cannot fail and has no side effects.
    let euid = unsafe { libc::geteuid() };
    for subject in [Some(real.as_path()), real.parent()].into_iter().flatten() {
        let metadata = std::fs::metadata(subject).map_err(|error| format!("`{}`: {error}", subject.display()))?;
        if metadata.uid() != 0 && metadata.uid() != euid {
            return Err(format!(
                "`{}` is owned by uid {}, not root or uid {euid}",
                subject.display(),
                metadata.uid()
            ));
        }
        if metadata.mode() & 0o022 != 0 {
            return Err(format!(
                "`{}` is writable by group or others (mode {:o})",
                subject.display(),
                metadata.mode() & 0o777
            ));
        }
    }
    Ok(real)
}

fn open_path(path: &Path) -> Result<Api, String> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("libheif path `{}` contains a NUL byte", path.display()))?;
    // SAFETY: `c_path` is NUL-terminated and outlives the call. Opening runs libheif's static
    // initialisers, which a link-time dependency would have run at process start.
    let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if handle.is_null() {
        return Err(format!(
            "could not load libheif `{}`: {}. Point {LIBHEIF_PATH_ENV_VAR} at a working libheif",
            path.display(),
            last_dl_error()
        ));
    }
    // A loaded handle is never closed: the resolved functions stay valid for the process. ~keep
    // SAFETY: `handle` is the live handle just opened.
    unsafe { Api::resolve(handle) }.map_err(|missing| {
        // SAFETY: Nothing resolved from `handle` outlives this failed resolution.
        unsafe { libc::dlclose(handle) };
        format!(
            "libheif `{}` lacks `{missing}`; xberg needs a libheif at least as new as the one it was built against",
            path.display()
        )
    })
}

fn last_dl_error() -> String {
    // SAFETY: `dlerror` returns null or a NUL-terminated message owned by the loader, copied here
    // before any other loader call on this thread.
    let message = unsafe { libc::dlerror() };
    if message.is_null() {
        return String::from("unknown dynamic loader error");
    }
    // SAFETY: Non-null `dlerror` results are NUL-terminated.
    unsafe { std::ffi::CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

/// Declares, for each libheif function, an [`Api`] field resolved by `dlsym` and a wrapper with
/// the `libheif-sys` signature that calls through it. Signatures are copied from the
/// `libheif-sys` bindings; each is gated on the first libheif version that declares it.
macro_rules! libheif_functions {
    ($( $(#[$attr:meta])* fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)?; )*) => {
        #[allow(dead_code, reason = "the wrapper set covers every feature combination; each build calls a subset")]
        struct Api {
            $( $(#[$attr])* $name: unsafe extern "C" fn($($ty),*) $(-> $ret)?, )*
        }

        impl Api {
            /// Resolve every function, or name the first one the library lacks.
            ///
            /// # Safety
            ///
            /// `handle` must be a live `dlopen` handle of libheif, whose exported functions have
            /// the C signatures `libheif-sys` declares.
            unsafe fn resolve(handle: *mut libc::c_void) -> Result<Self, &'static str> {
                Ok(Self {
                    $( $(#[$attr])* $name: {
                        // SAFETY: `handle` is live and the name is NUL-terminated.
                        let symbol =
                            unsafe { libc::dlsym(handle, concat!(stringify!($name), "\0").as_ptr().cast()) };
                        if symbol.is_null() {
                            return Err(stringify!($name));
                        }
                        // SAFETY: libheif exports this function with the signature declared here.
                        unsafe {
                            std::mem::transmute::<*mut libc::c_void, unsafe extern "C" fn($($ty),*) $(-> $ret)?>(
                                symbol,
                            )
                        }
                    }, )*
                })
            }
        }

        $(
            $(#[$attr])*
            #[allow(clippy::too_many_arguments)]
            #[allow(dead_code, reason = "the wrapper set covers every feature combination; each build calls a subset")]
            #[inline]
            pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? {
                // SAFETY: The caller upholds the contract of the libheif function of this name.
                unsafe { (api().$name)($($arg),*) }
            }
        )*
    };
}

libheif_functions! {
    fn heif_check_filetype(data: *const u8, len: libc::c_int) -> heif_filetype_result;
    fn heif_context_add_XMP_metadata(
        arg1: *mut heif_context,
        image_handle: *const heif_image_handle,
        data: *const libc::c_void,
        size: libc::c_int,
    ) -> heif_error;
    fn heif_context_add_exif_metadata(
        arg1: *mut heif_context,
        image_handle: *const heif_image_handle,
        data: *const libc::c_void,
        size: libc::c_int,
    ) -> heif_error;
    fn heif_context_add_generic_metadata(
        ctx: *mut heif_context,
        image_handle: *const heif_image_handle,
        data: *const libc::c_void,
        size: libc::c_int,
        item_type: *const libc::c_char,
        content_type: *const libc::c_char,
    ) -> heif_error;
    fn heif_context_alloc() -> *mut heif_context;
    fn heif_context_assign_thumbnail(
        arg1: *mut heif_context,
        master_image: *const heif_image_handle,
        thumbnail_image: *const heif_image_handle,
    ) -> heif_error;
    fn heif_context_encode_image(
        arg1: *mut heif_context,
        image: *const heif_image,
        encoder: *mut heif_encoder,
        options: *const heif_encoding_options,
        out_image_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_context_encode_thumbnail(
        arg1: *mut heif_context,
        image: *const heif_image,
        master_image_handle: *const heif_image_handle,
        encoder: *mut heif_encoder,
        options: *const heif_encoding_options,
        bbox_size: libc::c_int,
        out_thumb_image_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_context_free(arg1: *mut heif_context);
    fn heif_context_get_encoder(
        context: *mut heif_context,
        arg1: *const heif_encoder_descriptor,
        out_encoder: *mut *mut heif_encoder,
    ) -> heif_error;
    fn heif_context_get_encoder_for_format(
        context: *mut heif_context,
        format: heif_compression_format,
        arg1: *mut *mut heif_encoder,
    ) -> heif_error;
    fn heif_context_get_image_handle(
        ctx: *mut heif_context,
        id: heif_item_id,
        arg1: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_context_get_list_of_top_level_image_IDs(
        ctx: *mut heif_context,
        ID_array: *mut heif_item_id,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_context_get_number_of_top_level_images(ctx: *mut heif_context) -> libc::c_int;
    fn heif_context_get_primary_image_handle(
        ctx: *mut heif_context,
        arg1: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_context_read_from_file(
        arg1: *mut heif_context,
        filename: *const libc::c_char,
        arg2: *const heif_reading_options,
    ) -> heif_error;
    fn heif_context_read_from_memory_without_copy(
        arg1: *mut heif_context,
        mem: *const libc::c_void,
        size: usize,
        arg2: *const heif_reading_options,
    ) -> heif_error;
    fn heif_context_read_from_reader(
        arg1: *mut heif_context,
        reader: *const heif_reader,
        userdata: *mut libc::c_void,
        arg2: *const heif_reading_options,
    ) -> heif_error;
    fn heif_context_set_max_decoding_threads(ctx: *mut heif_context, max_threads: libc::c_int);
    fn heif_context_set_primary_image(arg1: *mut heif_context, image_handle: *mut heif_image_handle) -> heif_error;
    fn heif_context_write(
        arg1: *mut heif_context,
        writer: *mut heif_writer,
        userdata: *mut libc::c_void,
    ) -> heif_error;
    fn heif_context_write_to_file(arg1: *mut heif_context, filename: *const libc::c_char) -> heif_error;
    fn heif_decode_image(
        in_handle: *const heif_image_handle,
        out_img: *mut *mut heif_image,
        colorspace: heif_colorspace,
        chroma: heif_chroma,
        options: *const heif_decoding_options,
    ) -> heif_error;
    fn heif_decoder_descriptor_get_id_name(arg1: *const heif_decoder_descriptor) -> *const libc::c_char;
    fn heif_decoder_descriptor_get_name(arg1: *const heif_decoder_descriptor) -> *const libc::c_char;
    fn heif_decoding_options_alloc() -> *mut heif_decoding_options;
    fn heif_decoding_options_free(arg1: *mut heif_decoding_options);
    fn heif_deinit();
    fn heif_encoder_descriptor_get_compression_format(
        arg1: *const heif_encoder_descriptor,
    ) -> heif_compression_format;
    fn heif_encoder_descriptor_get_id_name(arg1: *const heif_encoder_descriptor) -> *const libc::c_char;
    fn heif_encoder_descriptor_get_name(arg1: *const heif_encoder_descriptor) -> *const libc::c_char;
    fn heif_encoder_descriptor_supports_lossless_compression(arg1: *const heif_encoder_descriptor) -> libc::c_int;
    fn heif_encoder_descriptor_supports_lossy_compression(arg1: *const heif_encoder_descriptor) -> libc::c_int;
    fn heif_encoder_get_name(arg1: *const heif_encoder) -> *const libc::c_char;
    fn heif_encoder_get_parameter_boolean(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: *mut libc::c_int,
    ) -> heif_error;
    fn heif_encoder_get_parameter_integer(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: *mut libc::c_int,
    ) -> heif_error;
    fn heif_encoder_get_parameter_string(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: *mut libc::c_char,
        value_size: libc::c_int,
    ) -> heif_error;
    fn heif_encoder_list_parameters(arg1: *mut heif_encoder) -> *const *const heif_encoder_parameter;
    fn heif_encoder_parameter_get_name(arg1: *const heif_encoder_parameter) -> *const libc::c_char;
    fn heif_encoder_parameter_get_type(arg1: *const heif_encoder_parameter) -> heif_encoder_parameter_type;
    fn heif_encoder_release(arg1: *mut heif_encoder);
    fn heif_encoder_set_lossless(arg1: *mut heif_encoder, enable: libc::c_int) -> heif_error;
    fn heif_encoder_set_lossy_quality(arg1: *mut heif_encoder, quality: libc::c_int) -> heif_error;
    fn heif_encoder_set_parameter_boolean(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: libc::c_int,
    ) -> heif_error;
    fn heif_encoder_set_parameter_integer(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: libc::c_int,
    ) -> heif_error;
    fn heif_encoder_set_parameter_string(
        arg1: *mut heif_encoder,
        parameter_name: *const libc::c_char,
        value: *const libc::c_char,
    ) -> heif_error;
    fn heif_encoding_options_alloc() -> *mut heif_encoding_options;
    fn heif_encoding_options_free(arg1: *mut heif_encoding_options);
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
    fn heif_image_add_plane(
        image: *mut heif_image,
        channel: heif_channel,
        width: libc::c_int,
        height: libc::c_int,
        bit_depth: libc::c_int,
    ) -> heif_error;
    fn heif_image_create(
        width: libc::c_int,
        height: libc::c_int,
        colorspace: heif_colorspace,
        chroma: heif_chroma,
        out_image: *mut *mut heif_image,
    ) -> heif_error;
    fn heif_image_get_bits_per_pixel(arg1: *const heif_image, channel: heif_channel) -> libc::c_int;
    fn heif_image_get_bits_per_pixel_range(arg1: *const heif_image, channel: heif_channel) -> libc::c_int;
    fn heif_image_get_chroma_format(arg1: *const heif_image) -> heif_chroma;
    fn heif_image_get_color_profile_type(image: *const heif_image) -> heif_color_profile_type;
    fn heif_image_get_colorspace(arg1: *const heif_image) -> heif_colorspace;
    fn heif_image_get_height(img: *const heif_image, channel: heif_channel) -> libc::c_int;
    fn heif_image_get_nclx_color_profile(
        image: *const heif_image,
        out_data: *mut *mut heif_color_profile_nclx,
    ) -> heif_error;
    fn heif_image_get_pixel_aspect_ratio(arg1: *const heif_image, aspect_h: *mut u32, aspect_v: *mut u32);
    fn heif_image_get_plane(arg1: *mut heif_image, channel: heif_channel, out_stride: *mut libc::c_int) -> *mut u8;
    fn heif_image_get_plane_readonly(
        arg1: *const heif_image,
        channel: heif_channel,
        out_stride: *mut libc::c_int,
    ) -> *const u8;
    fn heif_image_get_primary_height(img: *const heif_image) -> libc::c_int;
    fn heif_image_get_primary_width(img: *const heif_image) -> libc::c_int;
    fn heif_image_get_raw_color_profile(image: *const heif_image, out_data: *mut libc::c_void) -> heif_error;
    fn heif_image_get_raw_color_profile_size(image: *const heif_image) -> usize;
    fn heif_image_get_width(img: *const heif_image, channel: heif_channel) -> libc::c_int;
    fn heif_image_handle_get_auxiliary_image_handle(
        main_image_handle: *const heif_image_handle,
        auxiliary_id: heif_item_id,
        out_auxiliary_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_image_handle_get_auxiliary_type(
        handle: *const heif_image_handle,
        out_type: *mut *const libc::c_char,
    ) -> heif_error;
    fn heif_image_handle_get_chroma_bits_per_pixel(arg1: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_color_profile_type(handle: *const heif_image_handle) -> heif_color_profile_type;
    fn heif_image_handle_get_depth_image_handle(
        handle: *const heif_image_handle,
        depth_image_id: heif_item_id,
        out_depth_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_image_handle_get_height(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_ispe_height(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_ispe_width(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_item_id(handle: *const heif_image_handle) -> heif_item_id;
    fn heif_image_handle_get_list_of_auxiliary_image_IDs(
        handle: *const heif_image_handle,
        aux_filter: libc::c_int,
        ids: *mut heif_item_id,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_image_handle_get_list_of_depth_image_IDs(
        handle: *const heif_image_handle,
        ids: *mut heif_item_id,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_image_handle_get_list_of_metadata_block_IDs(
        handle: *const heif_image_handle,
        type_filter: *const libc::c_char,
        ids: *mut heif_item_id,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_image_handle_get_list_of_thumbnail_IDs(
        handle: *const heif_image_handle,
        ids: *mut heif_item_id,
        count: libc::c_int,
    ) -> libc::c_int;
    fn heif_image_handle_get_luma_bits_per_pixel(arg1: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_metadata(
        handle: *const heif_image_handle,
        metadata_id: heif_item_id,
        out_data: *mut libc::c_void,
    ) -> heif_error;
    fn heif_image_handle_get_metadata_content_type(
        handle: *const heif_image_handle,
        metadata_id: heif_item_id,
    ) -> *const libc::c_char;
    fn heif_image_handle_get_metadata_item_uri_type(
        handle: *const heif_image_handle,
        metadata_id: heif_item_id,
    ) -> *const libc::c_char;
    fn heif_image_handle_get_metadata_size(handle: *const heif_image_handle, metadata_id: heif_item_id) -> usize;
    fn heif_image_handle_get_metadata_type(
        handle: *const heif_image_handle,
        metadata_id: heif_item_id,
    ) -> *const libc::c_char;
    fn heif_image_handle_get_nclx_color_profile(
        handle: *const heif_image_handle,
        out_data: *mut *mut heif_color_profile_nclx,
    ) -> heif_error;
    fn heif_image_handle_get_number_of_auxiliary_images(
        handle: *const heif_image_handle,
        aux_filter: libc::c_int,
    ) -> libc::c_int;
    fn heif_image_handle_get_number_of_depth_images(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_number_of_metadata_blocks(
        handle: *const heif_image_handle,
        type_filter: *const libc::c_char,
    ) -> libc::c_int;
    fn heif_image_handle_get_number_of_thumbnails(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_get_preferred_decoding_colorspace(
        image_handle: *const heif_image_handle,
        out_colorspace: *mut heif_colorspace,
        out_chroma: *mut heif_chroma,
    ) -> heif_error;
    fn heif_image_handle_get_raw_color_profile(
        handle: *const heif_image_handle,
        out_data: *mut libc::c_void,
    ) -> heif_error;
    fn heif_image_handle_get_raw_color_profile_size(handle: *const heif_image_handle) -> usize;
    fn heif_image_handle_get_thumbnail(
        main_image_handle: *const heif_image_handle,
        thumbnail_id: heif_item_id,
        out_thumbnail_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    fn heif_image_handle_get_width(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_has_alpha_channel(arg1: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_has_depth_image(arg1: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_is_premultiplied_alpha(arg1: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_is_primary_image(handle: *const heif_image_handle) -> libc::c_int;
    fn heif_image_handle_release(arg1: *const heif_image_handle);
    fn heif_image_handle_release_auxiliary_type(
        handle: *const heif_image_handle,
        out_type: *mut *const libc::c_char,
    );
    fn heif_image_has_channel(arg1: *const heif_image, channel: heif_channel) -> libc::c_int;
    fn heif_image_is_premultiplied_alpha(image: *mut heif_image) -> libc::c_int;
    fn heif_image_release(arg1: *const heif_image);
    fn heif_image_scale_image(
        input: *const heif_image,
        output: *mut *mut heif_image,
        width: libc::c_int,
        height: libc::c_int,
        options: *const heif_scaling_options,
    ) -> heif_error;
    fn heif_image_set_nclx_color_profile(
        image: *mut heif_image,
        color_profile: *const heif_color_profile_nclx,
    ) -> heif_error;
    fn heif_image_set_pixel_aspect_ratio(arg1: *mut heif_image, aspect_h: u32, aspect_v: u32);
    fn heif_image_set_premultiplied_alpha(image: *mut heif_image, is_premultiplied_alpha: libc::c_int);
    fn heif_image_set_raw_color_profile(
        image: *mut heif_image,
        profile_type_fourcc_string: *const libc::c_char,
        profile_data: *const libc::c_void,
        profile_size: usize,
    ) -> heif_error;
    fn heif_init(arg1: *mut heif_init_params) -> heif_error;
    fn heif_load_plugins(
        directory: *const libc::c_char,
        out_plugins: *mut *const heif_plugin_info,
        out_nPluginsLoaded: *mut libc::c_int,
        output_array_size: libc::c_int,
    ) -> heif_error;
    fn heif_nclx_color_profile_alloc() -> *mut heif_color_profile_nclx;
    fn heif_nclx_color_profile_free(nclx_profile: *mut heif_color_profile_nclx);
    #[cfg(feature = "v1_18")]
    fn heif_context_encode_grid(
        ctx: *mut heif_context,
        tiles: *mut *mut heif_image,
        rows: u16,
        columns: u16,
        encoder: *mut heif_encoder,
        input_options: *const heif_encoding_options,
        out_image_handle: *mut *mut heif_image_handle,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_context_get_region_item(
        context: *const heif_context,
        region_item_id: heif_item_id,
        out: *mut *mut heif_region_item,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_image_handle_add_region_item(
        image_handle: *mut heif_image_handle,
        reference_width: u32,
        reference_height: u32,
        out_region_item: *mut *mut heif_region_item,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_image_handle_get_list_of_region_item_ids(
        image_handle: *const heif_image_handle,
        region_item_ids_array: *mut heif_item_id,
        max_count: libc::c_int,
    ) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_image_handle_get_number_of_region_items(image_handle: *const heif_image_handle) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_ellipse(
        region: *const heif_region,
        out_x: *mut i32,
        out_y: *mut i32,
        out_radius_x: *mut u32,
        out_radius_y: *mut u32,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_ellipse_transformed(
        region: *const heif_region,
        image_id: heif_item_id,
        out_x: *mut f64,
        out_y: *mut f64,
        out_radius_x: *mut f64,
        out_radius_y: *mut f64,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_inline_mask_data(
        region: *const heif_region,
        out_x: *mut i32,
        out_y: *mut i32,
        out_width: *mut u32,
        out_height: *mut u32,
        out_mask_data: *mut u8,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_inline_mask_data_len(region: *const heif_region) -> usize;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_point(region: *const heif_region, out_x: *mut i32, out_y: *mut i32) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_point_transformed(
        region: *const heif_region,
        image_id: heif_item_id,
        out_x: *mut f64,
        out_y: *mut f64,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polygon_num_points(region: *const heif_region) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polygon_points(region: *const heif_region, out_pts_array: *mut i32) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polygon_points_transformed(
        region: *const heif_region,
        image_id: heif_item_id,
        out_pts_array: *mut f64,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polyline_num_points(region: *const heif_region) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polyline_points(region: *const heif_region, out_pts_array: *mut i32) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_polyline_points_transformed(
        region: *const heif_region,
        image_id: heif_item_id,
        out_pts_array: *mut f64,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_rectangle(
        region: *const heif_region,
        out_x: *mut i32,
        out_y: *mut i32,
        out_width: *mut u32,
        out_height: *mut u32,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_rectangle_transformed(
        region: *const heif_region,
        image_id: heif_item_id,
        out_x: *mut f64,
        out_y: *mut f64,
        out_width: *mut f64,
        out_height: *mut f64,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_referenced_mask_ID(
        region: *const heif_region,
        out_x: *mut i32,
        out_y: *mut i32,
        out_width: *mut u32,
        out_height: *mut u32,
        out_mask_item_id: *mut heif_item_id,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_get_type(region: *const heif_region) -> heif_region_type;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_ellipse(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        radius_x: u32,
        radius_y: u32,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_inline_mask(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        image: *mut heif_image,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_inline_mask_data(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        mask_data: *const u8,
        mask_data_len: usize,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_point(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_polygon(
        region_item: *mut heif_region_item,
        pts_array: *const i32,
        nPoints: libc::c_int,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_polyline(
        region_item: *mut heif_region_item,
        pts_array: *const i32,
        nPoints: libc::c_int,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_rectangle(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_add_region_referenced_mask(
        region_item: *mut heif_region_item,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        mask_item_id: heif_item_id,
        out_region: *mut *mut heif_region,
    ) -> heif_error;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_get_id(region_item: *mut heif_region_item) -> heif_item_id;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_get_list_of_regions(
        region_item: *const heif_region_item,
        out_regions_array: *mut *mut heif_region,
        max_count: libc::c_int,
    ) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_get_number_of_regions(region_item: *const heif_region_item) -> libc::c_int;
    #[cfg(feature = "v1_18")]
    fn heif_region_item_get_reference_size(arg1: *mut heif_region_item, out_width: *mut u32, out_height: *mut u32);
    #[cfg(feature = "v1_18")]
    fn heif_region_item_release(region_item: *mut heif_region_item);
    #[cfg(feature = "v1_18")]
    fn heif_region_release(region: *const heif_region);
    #[cfg(feature = "v1_19")]
    fn heif_color_conversion_options_set_defaults(arg1: *mut heif_color_conversion_options);
    #[cfg(feature = "v1_19")]
    fn heif_context_get_security_limits(arg1: *const heif_context) -> *mut heif_security_limits;
    #[cfg(feature = "v1_19")]
    fn heif_context_set_security_limits(arg1: *mut heif_context, arg2: *const heif_security_limits) -> heif_error;
    #[cfg(feature = "v1_19")]
    fn heif_get_global_security_limits() -> *const heif_security_limits;
    #[cfg(feature = "v1_20")]
    fn heif_color_conversion_options_ext_alloc() -> *mut heif_color_conversion_options_ext;
    #[cfg(feature = "v1_20")]
    fn heif_color_conversion_options_ext_free(arg1: *mut heif_color_conversion_options_ext);
    #[cfg(feature = "v1_20")]
    fn heif_context_get_sequence_duration(arg1: *const heif_context) -> u64;
    #[cfg(feature = "v1_20")]
    fn heif_context_get_sequence_timescale(arg1: *const heif_context) -> u32;
    #[cfg(feature = "v1_20")]
    fn heif_context_get_track(arg1: *const heif_context, id: u32) -> *mut heif_track;
    #[cfg(feature = "v1_20")]
    fn heif_context_get_track_ids(ctx: *const heif_context, out_track_id_array: *mut [u32; 0usize]);
    #[cfg(feature = "v1_20")]
    fn heif_context_has_sequence(arg1: *const heif_context) -> libc::c_int;
    #[cfg(feature = "v1_20")]
    fn heif_context_number_of_sequence_tracks(arg1: *const heif_context) -> libc::c_int;
    #[cfg(feature = "v1_20")]
    fn heif_image_get_duration(arg1: *const heif_image) -> u32;
    #[cfg(feature = "v1_20")]
    fn heif_image_get_plane2(arg1: *mut heif_image, channel: heif_channel, out_stride: *mut usize) -> *mut u8;
    #[cfg(feature = "v1_20")]
    fn heif_image_get_plane_readonly2(
        arg1: *const heif_image,
        channel: heif_channel,
        out_stride: *mut usize,
    ) -> *const u8;
    #[cfg(feature = "v1_20")]
    fn heif_image_set_duration(arg1: *mut heif_image, duration: u32);
    #[cfg(feature = "v1_20")]
    fn heif_track_decode_next_image(
        track: *mut heif_track,
        out_img: *mut *mut heif_image,
        colorspace: heif_colorspace,
        chroma: heif_chroma,
        options: *const heif_decoding_options,
    ) -> heif_error;
    #[cfg(feature = "v1_20")]
    fn heif_track_get_id(track: *const heif_track) -> u32;
    #[cfg(feature = "v1_20")]
    fn heif_track_get_image_resolution(
        arg1: *const heif_track,
        out_width: *mut u16,
        out_height: *mut u16,
    ) -> heif_error;
    #[cfg(feature = "v1_20")]
    fn heif_track_get_timescale(arg1: *const heif_track) -> u32;
    #[cfg(feature = "v1_20")]
    fn heif_track_get_track_handler_type(arg1: *const heif_track) -> heif_track_type;
    #[cfg(feature = "v1_20")]
    fn heif_track_release(arg1: *mut heif_track);
    #[cfg(feature = "v1_21")]
    fn heif_track_has_alpha_channel(arg1: *const heif_track) -> libc::c_int;
}

#[cfg(test)]
mod tests;
