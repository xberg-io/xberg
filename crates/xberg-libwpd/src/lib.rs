//! WordPerfect text extraction for Xberg.
//!
//! Thin, safe wrapper over [libwpd](https://libwpd.sourceforge.net/) and its
//! document-model dependency librevenge, both built from source against their
//! MPL-2.0 arm (see `build.rs`). libwpd covers the whole WordPerfect binary
//! family (WP 4.2 through the X-series).
//!
//! libwpd has no `extract()` entry point; it drives a librevenge callback
//! interface. A hand-written C++ shim (`src/shim.cpp`) implements that
//! interface, accumulates a plain-text rendering, and exposes a flat C API this
//! crate wraps. WordPerfect support is desktop-only (Linux and macOS); on other
//! platforms the functions return [`WpdError::UnsupportedPlatform`].

mod error;

pub use error::WpdError;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod imp {
    use crate::WpdError;
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int, c_uchar, c_ulong};
    use std::ptr;

    unsafe extern "C" {
        fn xberg_wpd_is_supported(data: *const c_uchar, len: c_ulong) -> c_int;
        fn xberg_wpd_extract_text(data: *const c_uchar, len: c_ulong, out_text: *mut *mut c_char) -> c_int;
        fn xberg_wpd_free_string(s: *mut c_char);
    }

    /// Returns true if `data` looks like a WordPerfect document libwpd can parse.
    pub fn is_supported(data: &[u8]) -> bool {
        if data.is_empty() || data.len() > u32::MAX as usize {
            return false;
        }
        // SAFETY: `data` is a valid slice of `len` bytes; the shim only reads it
        // and catches any C++ exception internally.
        unsafe { xberg_wpd_is_supported(data.as_ptr(), data.len() as c_ulong) != 0 }
    }

    /// Extract the text of a WordPerfect document held entirely in memory.
    pub fn extract_text(data: &[u8]) -> Result<String, WpdError> {
        if data.is_empty() || data.len() > u32::MAX as usize {
            return Err(WpdError::InvalidArgs);
        }

        let mut out: *mut c_char = ptr::null_mut();
        // SAFETY: `data` is a valid slice of `len` bytes; `out` is a valid
        // out-pointer. The shim catches any C++ exception and reports it via the
        // return code. On a zero return it hands back a malloc'd, NUL-terminated
        // buffer whose ownership transfers to us.
        let code = unsafe { xberg_wpd_extract_text(data.as_ptr(), data.len() as c_ulong, &mut out) };
        if code != 0 {
            return Err(WpdError::from_code(code));
        }
        if out.is_null() {
            return Err(WpdError::Internal);
        }

        // SAFETY: `out` is the non-null buffer the shim allocated; we copy it out
        // and free it through the matching deallocator before returning.
        let text = unsafe {
            let owned = CStr::from_ptr(out).to_str().map(str::to_owned);
            xberg_wpd_free_string(out);
            owned
        };
        text.map_err(|_| WpdError::InvalidUtf8)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod imp {
    use crate::WpdError;

    /// WordPerfect extraction is desktop-only; unavailable on this target.
    pub fn is_supported(_data: &[u8]) -> bool {
        false
    }

    /// WordPerfect extraction is desktop-only; unavailable on this target.
    pub fn extract_text(_data: &[u8]) -> Result<String, WpdError> {
        Err(WpdError::UnsupportedPlatform)
    }
}

pub use imp::{extract_text, is_supported};
