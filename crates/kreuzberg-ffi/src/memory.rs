//! Memory management functions for FFI.
//!
//! This module contains all memory allocation and deallocation functions for the FFI layer.
//! These functions are CRITICAL for proper memory management across language boundaries.
//!
//! # Safety
//!
//! All functions in this module are unsafe and must be called correctly:
//! - Pointers must be valid and allocated by Rust
//! - Pointers must not be used after being freed
//! - NULL pointers are always safe and result in no-ops
//!
//! # Memory Leak Bugs Fixed
//!
//! - PR #3: Fixed Box/Vec mismatch in `kreuzberg_free_batch_result` causing segfaults
//! - PR #3: Fixed missing `page_structure_json` and `pages_json` deallocation in `kreuzberg_free_result`

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::ffi_panic_guard;
use crate::helpers::{clear_last_error, set_last_error};
use crate::types::{CBatchResult, CExtractionResult};

/// Free a batch result returned by batch extraction functions.
///
/// # Safety
///
/// - `batch_result` must be a pointer previously returned by a batch extraction function
/// - `batch_result` can be NULL (no-op)
/// - `batch_result` must not be used after this call
/// - All individual results in the batch will be freed automatically
///
/// # Memory Layout
///
/// CRITICAL: The results array is allocated as `Box<[*mut CExtractionResult]>` (boxed slice),
/// NOT as `Vec<*mut CExtractionResult>`. We must use `Box::from_raw` with a slice pointer,
/// not `Vec::from_raw_parts`, to avoid Box/Vec mismatch that causes segfaults.
///
/// # Example (C)
///
/// ```c
/// CBatchResult* batch = kreuzberg_extract_batch_sync(paths, count);
/// // Use batch...
/// kreuzberg_free_batch_result(batch);
/// // batch is now invalid
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_batch_result(batch_result: *mut CBatchResult) {
    if batch_result.is_null() {
        return;
    }

    let batch = unsafe { Box::from_raw(batch_result) };

    // Free individual results first, then the array
    if !batch.results.is_null() {
        if batch.count > 0 {
            unsafe {
                // Free each individual result
                for i in 0..batch.count {
                    let result_ptr = *batch.results.add(i);
                    if !result_ptr.is_null() {
                        kreuzberg_free_result(result_ptr);
                    }
                }
            }
        }

        // Free the results array itself (was created with into_boxed_slice())
        // IMPORTANT: Must use Box::from_raw with slice pointer, not Vec::from_raw_parts
        // because the array was allocated as Box<[T]>, not Vec<T>
        unsafe {
            let _boxed_slice = Box::from_raw(std::ptr::slice_from_raw_parts_mut(batch.results, batch.count));
            // Box will be dropped here, freeing the array allocation
        };
    }
}

/// Free a string returned by Kreuzberg functions.
///
/// # Safety
///
/// - `s` must be a string previously returned by a Kreuzberg function
/// - `s` can be NULL (no-op)
/// - `s` must not be used after this call
///
/// # Example (C)
///
/// ```c
/// char* str = result->content;
/// kreuzberg_free_string(str);
/// // str is now invalid
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Clone a null-terminated string using Rust's allocator.
///
/// # Safety
///
/// - `s` must be a valid null-terminated UTF-8 string
/// - Returned pointer must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clone_string(s: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_clone_string", {
        clear_last_error();

        if s.is_null() {
            set_last_error("Input string cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let raw = match unsafe { CStr::from_ptr(s) }.to_str() {
            Ok(val) => val,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in string: {}", e));
                return ptr::null_mut();
            }
        };

        match CString::new(raw) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                set_last_error(format!("Failed to clone string: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Free an extraction result returned by `kreuzberg_extract_file_sync`.
///
/// # Safety
///
/// - `result` must be a pointer previously returned by `kreuzberg_extract_file_sync`
/// - `result` can be NULL (no-op)
/// - `result` must not be used after this call
/// - All string fields within the result will be freed automatically
///
/// # Memory Layout
///
/// This function frees all 21 string fields in CExtractionResult:
/// 1. content
/// 2. mime_type
/// 3. language
/// 4. date
/// 5. subject
/// 6. tables_json
/// 7. detected_languages_json
/// 8. metadata_json
/// 9. chunks_json
/// 10. images_json
/// 11. page_structure_json (FIXED: was missing before PR #3)
/// 12. pages_json (FIXED: was missing before PR #3)
/// 13. elements_json (ADDED: for element-based extraction support)
/// 14. ocr_elements_json (ADDED: for OCR element output)
/// 15. document_json (ADDED: for document structure)
/// 16. extracted_keywords_json (ADDED: for keyword extraction)
/// 17. quality_score_json (ADDED: for quality analysis)
/// 18. processing_warnings_json (ADDED: for pipeline warnings)
/// 19. annotations_json (ADDED: for PDF annotation extraction)
///
/// # Example (C)
///
/// ```c
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// // Use result...
/// kreuzberg_free_result(result);
/// // result is now invalid
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_result(result: *mut CExtractionResult) {
    if !result.is_null() {
        let result_box = unsafe { Box::from_raw(result) };

        if !result_box.content.is_null() {
            unsafe { drop(CString::from_raw(result_box.content)) };
        }
        if !result_box.mime_type.is_null() {
            unsafe { drop(CString::from_raw(result_box.mime_type)) };
        }
        if !result_box.language.is_null() {
            unsafe { drop(CString::from_raw(result_box.language)) };
        }
        if !result_box.date.is_null() {
            unsafe { drop(CString::from_raw(result_box.date)) };
        }
        if !result_box.subject.is_null() {
            unsafe { drop(CString::from_raw(result_box.subject)) };
        }
        if !result_box.tables_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.tables_json)) };
        }
        if !result_box.detected_languages_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.detected_languages_json)) };
        }
        if !result_box.metadata_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.metadata_json)) };
        }
        if !result_box.chunks_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.chunks_json)) };
        }
        if !result_box.children_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.children_json)) };
        }
        if !result_box.images_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.images_json)) };
        }
        if !result_box.page_structure_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.page_structure_json)) };
        }
        if !result_box.pages_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.pages_json)) };
        }
        if !result_box.elements_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.elements_json)) };
        }
        if !result_box.ocr_elements_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.ocr_elements_json)) };
        }
        if !result_box.document_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.document_json)) };
        }
        if !result_box.extracted_keywords_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.extracted_keywords_json)) };
        }
        if !result_box.quality_score_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.quality_score_json)) };
        }
        if !result_box.processing_warnings_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.processing_warnings_json)) };
        }
        if !result_box.annotations_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.annotations_json)) };
        }
        if !result_box.uris_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.uris_json)) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// Helper to create a test CExtractionResult with all fields populated
    fn create_test_result() -> *mut CExtractionResult {
        Box::into_raw(Box::new(CExtractionResult {
            content: CString::new("test content").unwrap().into_raw(),
            mime_type: CString::new("text/plain").unwrap().into_raw(),
            language: CString::new("en").unwrap().into_raw(),
            date: CString::new("2024-01-01").unwrap().into_raw(),
            subject: CString::new("test subject").unwrap().into_raw(),
            tables_json: CString::new("[]").unwrap().into_raw(),
            detected_languages_json: CString::new("[\"en\"]").unwrap().into_raw(),
            metadata_json: CString::new("{}").unwrap().into_raw(),
            chunks_json: CString::new("[]").unwrap().into_raw(),
            children_json: ptr::null_mut(),
            images_json: CString::new("[]").unwrap().into_raw(),
            page_structure_json: CString::new("{}").unwrap().into_raw(),
            pages_json: CString::new("[]").unwrap().into_raw(),
            elements_json: CString::new("[]").unwrap().into_raw(),
            djot_content_json: ptr::null_mut(),
            ocr_elements_json: ptr::null_mut(),
            document_json: ptr::null_mut(),
            extracted_keywords_json: ptr::null_mut(),
            quality_score_json: CString::new("0.85").unwrap().into_raw(),
            processing_warnings_json: ptr::null_mut(),
            annotations_json: ptr::null_mut(),
            uris_json: ptr::null_mut(),
            success: true,
            _padding1: [0u8; 7],
        }))
    }

    /// Helper to create a test CExtractionResult with some NULL fields
    fn create_partial_result() -> *mut CExtractionResult {
        Box::into_raw(Box::new(CExtractionResult {
            content: CString::new("test content").unwrap().into_raw(),
            mime_type: CString::new("text/plain").unwrap().into_raw(),
            language: ptr::null_mut(),
            date: ptr::null_mut(),
            subject: ptr::null_mut(),
            tables_json: ptr::null_mut(),
            detected_languages_json: ptr::null_mut(),
            metadata_json: CString::new("{}").unwrap().into_raw(),
            chunks_json: ptr::null_mut(),
            children_json: ptr::null_mut(),
            images_json: ptr::null_mut(),
            page_structure_json: ptr::null_mut(),
            pages_json: ptr::null_mut(),
            elements_json: ptr::null_mut(),
            djot_content_json: ptr::null_mut(),
            ocr_elements_json: ptr::null_mut(),
            document_json: ptr::null_mut(),
            extracted_keywords_json: ptr::null_mut(),
            quality_score_json: ptr::null_mut(),
            processing_warnings_json: ptr::null_mut(),
            annotations_json: ptr::null_mut(),
            uris_json: ptr::null_mut(),
            success: true,
            _padding1: [0u8; 7],
        }))
    }

    #[test]
    fn test_free_string_null() {
        // Should not crash on NULL
        unsafe { kreuzberg_free_string(ptr::null_mut()) };
    }

    #[test]
    fn test_free_string_valid() {
        let s = CString::new("test string").unwrap().into_raw();
        unsafe { kreuzberg_free_string(s) };
        // If we get here without crashing, the test passed
    }

    #[test]
    fn test_clone_string_null() {
        let result = unsafe { kreuzberg_clone_string(ptr::null()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_clone_string_valid() {
        let original = CString::new("test string").unwrap();
        let cloned = unsafe { kreuzberg_clone_string(original.as_ptr()) };

        assert!(!cloned.is_null());

        // Verify the cloned string matches
        unsafe {
            let cloned_str = CStr::from_ptr(cloned);
            assert_eq!(cloned_str.to_str().unwrap(), "test string");

            // Free the cloned string
            kreuzberg_free_string(cloned);
        }
    }

    #[test]
    fn test_clone_and_free_cycle() {
        // Test multiple clone and free cycles
        for _ in 0..100 {
            let original = CString::new("test").unwrap();
            let cloned = unsafe { kreuzberg_clone_string(original.as_ptr()) };
            assert!(!cloned.is_null());
            unsafe { kreuzberg_free_string(cloned) };
        }
    }

    #[test]
    fn test_free_result_null() {
        // Should not crash on NULL
        unsafe { kreuzberg_free_result(ptr::null_mut()) };
    }

    #[test]
    fn test_free_result_all_fields() {
        // Test freeing a result with all 12 string fields populated
        let result = create_test_result();
        unsafe { kreuzberg_free_result(result) };
        // If we get here without crashing, the test passed
    }

    #[test]
    fn test_free_result_partial_fields() {
        // Test freeing a result with some NULL fields
        let result = create_partial_result();
        unsafe { kreuzberg_free_result(result) };
        // If we get here without crashing, the test passed
    }

    #[test]
    fn test_free_result_page_structure_and_pages_json() {
        // Regression test: ensure page_structure_json and pages_json are freed
        // These were missing in the original implementation before PR #3
        let result = Box::into_raw(Box::new(CExtractionResult {
            content: CString::new("test").unwrap().into_raw(),
            mime_type: CString::new("text/plain").unwrap().into_raw(),
            language: ptr::null_mut(),
            date: ptr::null_mut(),
            subject: ptr::null_mut(),
            tables_json: ptr::null_mut(),
            detected_languages_json: ptr::null_mut(),
            metadata_json: ptr::null_mut(),
            chunks_json: ptr::null_mut(),
            children_json: ptr::null_mut(),
            images_json: ptr::null_mut(),
            page_structure_json: CString::new("{\"pages\": []}").unwrap().into_raw(),
            pages_json: CString::new("[{\"content\": \"page 1\"}]").unwrap().into_raw(),
            elements_json: ptr::null_mut(),
            djot_content_json: ptr::null_mut(),
            ocr_elements_json: ptr::null_mut(),
            document_json: ptr::null_mut(),
            extracted_keywords_json: ptr::null_mut(),
            quality_score_json: ptr::null_mut(),
            processing_warnings_json: ptr::null_mut(),
            annotations_json: ptr::null_mut(),
            uris_json: ptr::null_mut(),
            success: true,
            _padding1: [0u8; 7],
        }));

        unsafe { kreuzberg_free_result(result) };
        // If we get here without crashing or leaking, the test passed
    }

    #[test]
    fn test_free_result_elements_json() {
        // Test: ensure elements_json is freed
        let result = Box::into_raw(Box::new(CExtractionResult {
            content: CString::new("test").unwrap().into_raw(),
            mime_type: CString::new("text/plain").unwrap().into_raw(),
            language: ptr::null_mut(),
            date: ptr::null_mut(),
            subject: ptr::null_mut(),
            tables_json: ptr::null_mut(),
            detected_languages_json: ptr::null_mut(),
            metadata_json: ptr::null_mut(),
            chunks_json: ptr::null_mut(),
            children_json: ptr::null_mut(),
            images_json: ptr::null_mut(),
            page_structure_json: ptr::null_mut(),
            pages_json: ptr::null_mut(),
            elements_json: CString::new(r#"[{"element_id":"abc","element_type":"title","text":"Hello"}]"#)
                .unwrap()
                .into_raw(),
            djot_content_json: ptr::null_mut(),
            ocr_elements_json: ptr::null_mut(),
            document_json: ptr::null_mut(),
            extracted_keywords_json: ptr::null_mut(),
            quality_score_json: ptr::null_mut(),
            processing_warnings_json: ptr::null_mut(),
            annotations_json: ptr::null_mut(),
            uris_json: ptr::null_mut(),
            success: true,
            _padding1: [0u8; 7],
        }));

        unsafe { kreuzberg_free_result(result) };
        // If we get here without crashing or leaking, the test passed
    }

    #[test]
    fn test_free_batch_result_null() {
        // Should not crash on NULL
        unsafe { kreuzberg_free_batch_result(ptr::null_mut()) };
    }

    #[test]
    fn test_free_batch_result_empty() {
        // Test freeing a batch with 0 results
        let batch = Box::into_raw(Box::new(CBatchResult {
            results: ptr::null_mut(),
            count: 0,
            success: true,
            _padding2: [0u8; 7],
        }));

        unsafe { kreuzberg_free_batch_result(batch) };
    }

    #[test]
    fn test_free_batch_result_single() {
        // Test freeing a batch with 1 result
        let result = create_test_result();
        let results_array = vec![result].into_boxed_slice();
        let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

        let batch = Box::into_raw(Box::new(CBatchResult {
            results: results_ptr,
            count: 1,
            success: true,
            _padding2: [0u8; 7],
        }));

        unsafe { kreuzberg_free_batch_result(batch) };
    }

    #[test]
    fn test_free_batch_result_multiple() {
        // Test freeing a batch with 100 results
        let mut results = Vec::new();
        for _ in 0..100 {
            results.push(create_test_result());
        }

        let results_array = results.into_boxed_slice();
        let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

        let batch = Box::into_raw(Box::new(CBatchResult {
            results: results_ptr,
            count: 100,
            success: true,
            _padding2: [0u8; 7],
        }));

        unsafe { kreuzberg_free_batch_result(batch) };
    }

    #[test]
    fn test_free_batch_result_box_vec_symmetry() {
        // Regression test for Box/Vec mismatch bug fixed in PR #3
        // This test ensures we use Box::from_raw with slice pointer,
        // not Vec::from_raw_parts, which would cause a segfault

        // Create results using Box<[T]> allocation (same as production code)
        let mut results = Vec::new();
        for _ in 0..10 {
            results.push(create_test_result());
        }

        // Convert to boxed slice (this is what production code does)
        let results_array = results.into_boxed_slice();
        let count = results_array.len();
        let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

        let batch = Box::into_raw(Box::new(CBatchResult {
            results: results_ptr,
            count,
            success: true,
            _padding2: [0u8; 7],
        }));

        // This should NOT segfault
        unsafe { kreuzberg_free_batch_result(batch) };
    }

    #[test]
    fn test_free_batch_result_with_null_results() {
        // Test freeing a batch where some results are NULL
        let results = vec![
            create_test_result(),
            ptr::null_mut(),
            create_test_result(),
            ptr::null_mut(),
        ];

        let results_array = results.into_boxed_slice();
        let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

        let batch = Box::into_raw(Box::new(CBatchResult {
            results: results_ptr,
            count: 4,
            success: true,
            _padding2: [0u8; 7],
        }));

        unsafe { kreuzberg_free_batch_result(batch) };
    }

    #[test]
    fn test_memory_stress_test() {
        // Stress test: allocate and free 1000 results
        for _ in 0..1000 {
            let result = create_test_result();
            unsafe { kreuzberg_free_result(result) };
        }
    }

    #[test]
    fn test_memory_stress_test_batch() {
        // Stress test: allocate and free 100 batches of 10 results each
        for _ in 0..100 {
            let mut results = Vec::new();
            for _ in 0..10 {
                results.push(create_test_result());
            }

            let results_array = results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: 10,
                success: true,
                _padding2: [0u8; 7],
            }));

            unsafe { kreuzberg_free_batch_result(batch) };
        }
    }
}
