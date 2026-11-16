//! WordPerfect structured document extraction for Xberg.
//!
//! Thin, safe wrapper over [libwpd](https://libwpd.sourceforge.net/) and its
//! document-model dependency librevenge, both built from source against their
//! MPL-2.0 arm (see `build.rs`). libwpd covers the whole WordPerfect binary
//! family (WP 4.2 through the X-series).
//!
//! libwpd has no `extract()` entry point; it drives a librevenge callback
//! interface. A hand-written C++ shim (`src/shim.cpp`) implements that
//! interface, records a flat, format-agnostic internal document as libwpd
//! walks the input, and serializes that one document into a versioned binary
//! blob exposed through a flat C API this crate wraps. [`extract_document`]
//! decodes that blob into a typed [`WpdDocument`]: an ordered [`WpdEvent`]
//! stream (text runs, formatting spans, list items, table structure with
//! column/row spans and header-row flags, hyperlinks, fields, footnotes and
//! endnotes kept as distinct sequences, headers/footers, and comment/text-box
//! asides) plus [`WpdMetadata`] (title, author, subject, keywords, and every
//! raw key/value pair libwpd reported). This crate performs no text or
//! Markdown rendering; producing a flattened string from the structured model
//! is left to the caller. WordPerfect support targets Linux, macOS and
//! Windows; on other platforms [`extract_document`] returns
//! [`WpdError::UnsupportedPlatform`].

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![allow(unsafe_code, reason = "libwpd is reached only through its C++ shim")]
#![cfg_attr(test, allow(clippy::print_stdout, clippy::print_stderr))]

mod dto;
mod error;

pub use dto::{WpdDocument, WpdEvent, WpdMetadata};
pub use error::WpdError;

// libz-sys is linked for the vendored librevenge zip stream and never named in Rust; this marks
// it used for cargo's unused_dependencies lint. ~keep
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use libz_sys as _;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod imp {
    use crate::{WpdDocument, WpdError, dto};
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int, c_uchar, c_ulong};
    use std::{ptr, slice};

    unsafe extern "C" {
        fn xberg_wpd_is_supported(data: *const c_uchar, len: c_ulong) -> c_int;
        fn xberg_wpd_extract_document(
            data: *const c_uchar,
            len: c_ulong,
            out_buf: *mut *mut c_char,
            out_len: *mut c_ulong,
            out_err: *mut *mut c_char,
        ) -> c_int;
        fn xberg_wpd_free_string(s: *mut c_char);
        #[cfg(test)]
        fn xberg_wpd_self_test_separation() -> c_int;
        #[cfg(test)]
        fn xberg_wpd_self_test_features() -> c_int;
    }

    /// Returns true if `data` looks like a WordPerfect document libwpd can parse.
    pub fn is_supported(data: &[u8]) -> bool {
        if data.is_empty() || data.len() > u32::MAX as usize {
            return false;
        }
        // SAFETY: `data` is a valid slice of `len` bytes; the shim only reads it
        // and catches any C++ exception internally. ~keep
        unsafe { xberg_wpd_is_supported(data.as_ptr(), data.len() as c_ulong) != 0 }
    }

    /// Extract the structured document model of a WordPerfect document held
    /// entirely in memory.
    pub fn extract_document(data: &[u8]) -> Result<WpdDocument, WpdError> {
        if data.is_empty() || data.len() > u32::MAX as usize {
            return Err(WpdError::InvalidArgs);
        }

        let mut out: *mut c_char = ptr::null_mut();
        let mut out_len: c_ulong = 0;
        let mut out_err: *mut c_char = ptr::null_mut();
        // SAFETY: `data` is a valid slice of `len` bytes; `out`/`out_len`/`out_err`
        // are valid out-pointers. The shim catches any C++ exception and reports
        // it via the return code (plus, optionally, a detail message). On a zero
        // return it hands back a malloc'd buffer of exactly `out_len` bytes whose
        // ownership transfers to us. ~keep
        let code = unsafe {
            xberg_wpd_extract_document(
                data.as_ptr(),
                data.len() as c_ulong,
                &mut out,
                &mut out_len,
                &mut out_err,
            )
        };
        if !out_err.is_null() {
            // SAFETY: `out_err` is a malloc'd, NUL-terminated buffer the shim
            // handed us; freed unconditionally right after reading it. ~keep
            let detail = unsafe {
                let msg = CStr::from_ptr(out_err).to_string_lossy().into_owned();
                xberg_wpd_free_string(out_err);
                msg
            };
            tracing::warn!(code, error = %detail, "libwpd raised an exception during extraction");
        }
        if code != 0 {
            // Defensive: the FFI contract is that `out` stays null on any
            // non-zero return, but a future shim regression that sets it
            // anyway must not leak the buffer it allocated. ~keep
            if !out.is_null() {
                // SAFETY: `out` would only be non-null here if the shim
                // violated its own contract by allocating a buffer on an
                // error path; if so it is still the same malloc'd buffer
                // `xberg_wpd_free_string` is designed to free. ~keep
                unsafe { xberg_wpd_free_string(out) };
            }
            return Err(WpdError::from_code(code));
        }
        if out.is_null() {
            return Err(WpdError::Internal);
        }

        // SAFETY: `out` is the non-null buffer the shim allocated, exactly
        // `out_len` bytes long; we copy it out and free it through the matching
        // deallocator before returning. Using the explicit length (rather than
        // scanning for a NUL terminator) means the binary blob's embedded
        // length-prefixed strings can't be silently truncated at an embedded
        // NUL. ~keep
        let bytes = unsafe {
            let bytes = slice::from_raw_parts(out as *const u8, out_len as usize).to_vec();
            xberg_wpd_free_string(out);
            bytes
        };
        dto::decode(&bytes)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn collector_separates_asides_from_body() {
            // SAFETY: takes no arguments and only touches its own stack-local state. ~keep
            assert_eq!(unsafe { xberg_wpd_self_test_separation() }, 1);
        }

        #[test]
        fn collector_captures_links_tables_fields_and_notes() {
            // SAFETY: takes no arguments and only touches its own stack-local state. ~keep
            assert_eq!(unsafe { xberg_wpd_self_test_features() }, 1);
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod imp {
    /// WordPerfect extraction is desktop-only; unavailable on this target.
    pub fn is_supported(_data: &[u8]) -> bool {
        false
    }

    /// WordPerfect extraction is desktop-only; unavailable on this target.
    pub fn extract_document(_data: &[u8]) -> Result<super::WpdDocument, super::WpdError> {
        Err(super::WpdError::UnsupportedPlatform)
    }
}

pub use imp::{extract_document, is_supported};
