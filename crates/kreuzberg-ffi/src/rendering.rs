//! PDF page rendering FFI functions.
//!
//! Provides C-compatible functions for rendering PDF pages to PNG byte buffers.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use crate::ffi_panic_guard;
use crate::helpers::{clear_last_error, set_last_error};

/// A single rendered page image (PNG bytes).
#[repr(C)]
pub struct CPageImage {
    /// Pointer to PNG data. Owned by this struct; freed via `kreuzberg_free_render_page_result`.
    pub data: *mut u8,
    /// Length of PNG data in bytes.
    pub len: usize,
}

/// Result of rendering all pages of a PDF.
#[repr(C)]
pub struct CRenderResult {
    /// Array of page images. Owned; freed via `kreuzberg_free_render_result`.
    pub pages: *mut CPageImage,
    /// Number of pages.
    pub page_count: usize,
    /// Error message if the operation failed (NULL on success).
    pub error: *mut c_char,
}

/// Render all pages of a PDF file to PNG byte buffers.
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned pointer must be freed with `kreuzberg_free_render_result`
/// - Returns NULL on panic (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_render_pdf_pages(file_path: *const c_char, dpi: i32) -> *mut CRenderResult {
    ffi_panic_guard!("kreuzberg_render_pdf_pages", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        let dpi_opt = if dpi <= 0 { None } else { Some(dpi) };
        let path = std::path::Path::new(path_str);

        match kreuzberg::pdf::render_pdf_file_to_png_pages(path, dpi_opt, None) {
            Ok(pages) => {
                let mut c_pages: Vec<CPageImage> = pages
                    .into_iter()
                    .map(|png| {
                        let mut boxed = png.into_boxed_slice();
                        let data = boxed.as_mut_ptr();
                        let len = boxed.len();
                        std::mem::forget(boxed);
                        CPageImage { data, len }
                    })
                    .collect();

                let page_count = c_pages.len();
                let pages_ptr = c_pages.as_mut_ptr();
                std::mem::forget(c_pages);

                Box::into_raw(Box::new(CRenderResult {
                    pages: pages_ptr,
                    page_count,
                    error: ptr::null_mut(),
                }))
            }
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Render a single page of a PDF file to a PNG byte buffer.
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned pointer must be freed with `kreuzberg_free_render_page_result`
/// - Returns NULL on panic (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_render_pdf_page(
    file_path: *const c_char,
    page_index: usize,
    dpi: i32,
) -> *mut CPageImage {
    ffi_panic_guard!("kreuzberg_render_pdf_page", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        let dpi_opt = if dpi <= 0 { None } else { Some(dpi) };
        let pdf_bytes = match std::fs::read(path_str) {
            Ok(b) => b,
            Err(e) => {
                set_last_error(format!("Failed to read file: {}", e));
                return ptr::null_mut();
            }
        };

        match kreuzberg::pdf::render_pdf_page_to_png(&pdf_bytes, page_index, dpi_opt, None) {
            Ok(png) => {
                let mut boxed = png.into_boxed_slice();
                let data = boxed.as_mut_ptr();
                let len = boxed.len();
                std::mem::forget(boxed);

                Box::into_raw(Box::new(CPageImage { data, len }))
            }
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Free a render result returned by `kreuzberg_render_pdf_pages`.
///
/// # Safety
///
/// - `result` must be a pointer returned by `kreuzberg_render_pdf_pages`, or NULL (no-op)
/// - `result` must not be used after this call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_render_result(result: *mut CRenderResult) {
    if result.is_null() {
        return;
    }

    let result_box = unsafe { Box::from_raw(result) };

    if !result_box.pages.is_null() && result_box.page_count > 0 {
        for i in 0..result_box.page_count {
            let page = unsafe { &*result_box.pages.add(i) };
            if !page.data.is_null() {
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(page.data, page.len)));
                }
            }
        }
        // Free the pages array itself (allocated via Vec)
        unsafe {
            drop(Vec::from_raw_parts(
                result_box.pages,
                result_box.page_count,
                result_box.page_count,
            ));
        }
    }

    if !result_box.error.is_null() {
        unsafe {
            drop(std::ffi::CString::from_raw(result_box.error));
        }
    }
}

/// Free a single page result returned by `kreuzberg_render_pdf_page`.
///
/// # Safety
///
/// - `page` must be a pointer returned by `kreuzberg_render_pdf_page`, or NULL (no-op)
/// - `page` must not be used after this call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_render_page_result(page: *mut CPageImage) {
    if page.is_null() {
        return;
    }

    let page_box = unsafe { Box::from_raw(page) };

    if !page_box.data.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                page_box.data,
                page_box.len,
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_page_image_size() {
        assert_eq!(
            std::mem::size_of::<CPageImage>(),
            16,
            "CPageImage must be 16 bytes (ptr + usize)"
        );
    }

    #[test]
    fn test_c_render_result_size() {
        assert_eq!(
            std::mem::size_of::<CRenderResult>(),
            24,
            "CRenderResult must be 24 bytes"
        );
    }

    #[test]
    fn test_free_render_result_null() {
        unsafe { kreuzberg_free_render_result(ptr::null_mut()) };
    }

    #[test]
    fn test_free_render_page_result_null() {
        unsafe { kreuzberg_free_render_page_result(ptr::null_mut()) };
    }

    #[test]
    fn test_render_pdf_pages_null_path() {
        let result = unsafe { kreuzberg_render_pdf_pages(ptr::null(), 150) };
        assert!(result.is_null());
    }

    #[test]
    fn test_render_pdf_page_null_path() {
        let result = unsafe { kreuzberg_render_pdf_page(ptr::null(), 0, 150) };
        assert!(result.is_null());
    }
}
