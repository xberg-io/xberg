//! C FFI bindings for Kreuzberg document intelligence library.
//!
//! Provides a C-compatible API that can be consumed by Java (Panama FFI),
//! Go (cgo), C# (P/Invoke), Zig, and other languages with C FFI support.

mod batch_streaming;
mod cancellation;
mod config;
mod config_builder;
mod embedding;
mod error;
mod extraction;
mod helpers;
mod html_options;
mod memory;
mod mime;
mod panic_shield;
mod plugins;
mod rendering;
mod result;
mod result_pool;
mod result_view;

mod string_intern;
mod types;
mod util;
mod validation;

pub use batch_streaming::{
    ErrorCallback, ResultCallback, kreuzberg_extract_batch_parallel, kreuzberg_extract_batch_streaming,
};
pub use cancellation::{
    CancellationToken as CFfiCancellationToken, kreuzberg_cancel_token_cancel, kreuzberg_cancel_token_free,
    kreuzberg_cancel_token_is_cancelled, kreuzberg_cancel_token_new,
};
pub use embedding::kreuzberg_embed;

pub use config::{
    kreuzberg_config_discover, kreuzberg_config_free, kreuzberg_config_from_file, kreuzberg_config_from_json,
    kreuzberg_config_get_field, kreuzberg_config_is_valid, kreuzberg_config_merge, kreuzberg_config_to_json,
    kreuzberg_get_embedding_preset, kreuzberg_list_embedding_presets, kreuzberg_load_extraction_config_from_file,
};
pub use config_builder::kreuzberg_config_builder_set_layout;
pub use config_builder::{
    kreuzberg_config_builder_build, kreuzberg_config_builder_free, kreuzberg_config_builder_new,
    kreuzberg_config_builder_set_acceleration, kreuzberg_config_builder_set_chunking,
    kreuzberg_config_builder_set_image_extraction, kreuzberg_config_builder_set_include_document_structure,
    kreuzberg_config_builder_set_language_detection, kreuzberg_config_builder_set_ocr,
    kreuzberg_config_builder_set_pdf, kreuzberg_config_builder_set_post_processor,
    kreuzberg_config_builder_set_use_cache,
};
pub use error::ErrorCode as KreuzbergErrorCode;
pub use error::{
    CErrorDetails, kreuzberg_classify_error, kreuzberg_error_code_cancelled, kreuzberg_error_code_count,
    kreuzberg_error_code_description, kreuzberg_error_code_internal, kreuzberg_error_code_io,
    kreuzberg_error_code_missing_dependency, kreuzberg_error_code_name, kreuzberg_error_code_ocr,
    kreuzberg_error_code_parsing, kreuzberg_error_code_plugin, kreuzberg_error_code_unsupported_format,
    kreuzberg_error_code_validation, kreuzberg_get_error_details,
};
pub use extraction::{
    kreuzberg_batch_extract_bytes_sync, kreuzberg_batch_extract_files_sync, kreuzberg_extract_bytes_sync,
    kreuzberg_extract_bytes_sync_with_config, kreuzberg_extract_file_sync, kreuzberg_extract_file_sync_with_config,
};
pub use helpers::*;
pub use html_options::{
    kreuzberg_code_block_style_to_string, kreuzberg_heading_style_to_string, kreuzberg_highlight_style_to_string,
    kreuzberg_list_indent_type_to_string, kreuzberg_newline_style_to_string, kreuzberg_parse_code_block_style,
    kreuzberg_parse_heading_style, kreuzberg_parse_highlight_style, kreuzberg_parse_list_indent_type,
    kreuzberg_parse_newline_style, kreuzberg_parse_preprocessing_preset, kreuzberg_parse_whitespace_mode,
    kreuzberg_preprocessing_preset_to_string, kreuzberg_whitespace_mode_to_string,
};
pub use memory::{kreuzberg_clone_string, kreuzberg_free_batch_result, kreuzberg_free_result, kreuzberg_free_string};
pub use mime::{
    kreuzberg_detect_mime_type, kreuzberg_detect_mime_type_from_bytes, kreuzberg_detect_mime_type_from_path,
    kreuzberg_get_extensions_for_mime, kreuzberg_validate_mime_type,
};
pub use panic_shield::{
    ErrorCode, StructuredError, clear_structured_error, get_last_error_code, get_last_error_message,
    get_last_panic_context, set_structured_error,
};
pub use plugins::*;
pub use rendering::{
    CPageImage, CPageIterResult, CPdfPageIterator, kreuzberg_free_render_page_result, kreuzberg_pdf_page_iterator_free,
    kreuzberg_pdf_page_iterator_free_result, kreuzberg_pdf_page_iterator_new, kreuzberg_pdf_page_iterator_next,
    kreuzberg_pdf_page_iterator_page_count, kreuzberg_render_pdf_page,
};
pub use result::{
    CMetadataField, kreuzberg_result_get_chunk_count, kreuzberg_result_get_detected_language,
    kreuzberg_result_get_metadata_field, kreuzberg_result_get_page_count,
};
pub use result_pool::{
    CResultPoolStats, ResultPool, kreuzberg_extract_file_into_pool, kreuzberg_extract_file_into_pool_view,
    kreuzberg_result_pool_free, kreuzberg_result_pool_new, kreuzberg_result_pool_reset, kreuzberg_result_pool_stats,
};
pub use result_view::{
    CExtractionResultView, kreuzberg_get_result_view, kreuzberg_view_get_content, kreuzberg_view_get_mime_type,
};

pub use string_intern::{
    CStringInternStats, kreuzberg_free_interned_string, kreuzberg_intern_string, kreuzberg_string_intern_reset,
    kreuzberg_string_intern_stats,
};
pub use types::*;
pub use util::{kreuzberg_last_error, kreuzberg_last_error_code, kreuzberg_last_panic_context, kreuzberg_version};
pub use validation::*;

#[ctor::ctor]
fn setup_onnx_runtime_path() {
    kreuzberg::ort_discovery::ensure_ort_available();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::ptr;

    #[test]
    fn test_version() {
        unsafe {
            let version = kreuzberg_version();
            assert!(!version.is_null());
            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert!(!version_str.is_empty());
        }
    }

    #[test]
    fn test_null_path() {
        unsafe {
            let result = kreuzberg_extract_file_sync(ptr::null());
            assert!(result.is_null());

            let error = kreuzberg_last_error();
            assert!(!error.is_null());
            let error_str = CStr::from_ptr(error).to_str().unwrap();
            assert!(error_str.contains("NULL"));
        }
    }

    #[test]
    fn test_nonexistent_file() {
        unsafe {
            let path = CString::new("/nonexistent/file.pdf").unwrap();
            let result = kreuzberg_extract_file_sync(path.as_ptr());
            assert!(result.is_null());

            let error = kreuzberg_last_error();
            assert!(!error.is_null());
        }
    }

    // ==================== Struct Layout Tests ====================

    #[test]
    fn test_cextraction_result_layout() {
        // Test size
        assert_eq!(
            std::mem::size_of::<CExtractionResult>(),
            208,
            "CExtractionResult must be exactly 208 bytes"
        );

        // Test alignment
        assert_eq!(
            std::mem::align_of::<CExtractionResult>(),
            8,
            "CExtractionResult must be 8-byte aligned"
        );
    }

    #[test]
    fn test_cbatch_result_layout() {
        // Test size
        assert_eq!(
            std::mem::size_of::<CBatchResult>(),
            24,
            "CBatchResult must be exactly 24 bytes"
        );

        // Test alignment
        assert_eq!(
            std::mem::align_of::<CBatchResult>(),
            8,
            "CBatchResult must be 8-byte aligned"
        );
    }

    #[test]
    fn test_cbytes_with_mime_layout() {
        // Test size
        assert_eq!(
            std::mem::size_of::<CBytesWithMime>(),
            24,
            "CBytesWithMime must be exactly 24 bytes"
        );

        // Test alignment
        assert_eq!(
            std::mem::align_of::<CBytesWithMime>(),
            8,
            "CBytesWithMime must be 8-byte aligned"
        );
    }

    // ==================== Memory Safety Tests ====================

    /// Helper function to create a mock CExtractionResult for testing
    fn create_mock_extraction_result() -> *mut CExtractionResult {
        Box::into_raw(Box::new(CExtractionResult {
            content: CString::new("test content").unwrap().into_raw(),
            mime_type: CString::new("text/plain").unwrap().into_raw(),
            language: CString::new("en").unwrap().into_raw(),
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
            elements_json: ptr::null_mut(),
            djot_content_json: ptr::null_mut(),
            ocr_elements_json: ptr::null_mut(),
            document_json: ptr::null_mut(),
            extracted_keywords_json: ptr::null_mut(),
            quality_score_json: ptr::null_mut(),
            processing_warnings_json: ptr::null_mut(),
            annotations_json: ptr::null_mut(),
            uris_json: ptr::null_mut(),
            llm_usage_json: ptr::null_mut(),
            code_intelligence_json: ptr::null_mut(),
            structured_output_json: ptr::null_mut(),
            success: true,
            _padding1: [0u8; 7],
        }))
    }

    #[test]
    fn test_batch_result_allocation_deallocation() {
        unsafe {
            // Simulate the exact allocation pattern from kreuzberg_batch_extract_files_sync
            let c_results = vec![
                create_mock_extraction_result(),
                create_mock_extraction_result(),
                create_mock_extraction_result(),
            ];

            let actual_count = c_results.len();

            // This is the exact pattern used in kreuzberg_batch_extract_files_sync
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: actual_count,
                success: true,
                _padding2: [0u8; 7],
            }));

            // Verify the batch result is valid
            assert!(!batch_result.is_null());
            assert_eq!((*batch_result).count, 3);
            assert!((*batch_result).success);

            // Now free it using the public API
            kreuzberg_free_batch_result(batch_result);

            // If we got here without crashing, the allocation/deallocation pattern is correct
        }
    }

    #[test]
    fn test_free_null_batch() {
        unsafe {
            // Freeing NULL batch should not crash
            kreuzberg_free_batch_result(ptr::null_mut());
        }
    }

    #[test]
    fn test_free_null_result() {
        unsafe {
            // Freeing NULL result should not crash
            kreuzberg_free_result(ptr::null_mut());
        }
    }

    #[test]
    fn test_free_null_string() {
        unsafe {
            // Freeing NULL string should not crash
            kreuzberg_free_string(ptr::null_mut());
        }
    }

    #[test]
    fn test_batch_result_with_empty_results() {
        unsafe {
            // Test batch result with zero results
            let c_results: Vec<*mut CExtractionResult> = Vec::new();
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: 0,
                success: true,
                _padding2: [0u8; 7],
            }));

            assert!(!batch_result.is_null());
            assert_eq!((*batch_result).count, 0);

            // Free should handle empty batch gracefully
            kreuzberg_free_batch_result(batch_result);
        }
    }

    #[test]
    fn test_batch_result_with_null_elements() {
        unsafe {
            // Test batch result where some elements are NULL
            let c_results = vec![
                create_mock_extraction_result(),
                ptr::null_mut(), // NULL element
                create_mock_extraction_result(),
            ];

            let actual_count = c_results.len();
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: actual_count,
                success: true,
                _padding2: [0u8; 7],
            }));

            // Free should handle NULL elements gracefully
            kreuzberg_free_batch_result(batch_result);
        }
    }

    #[test]
    fn test_batch_result_single_element() {
        unsafe {
            // Test batch allocation/deallocation with exactly 1 result
            // This is important for boundary condition testing
            let c_results = vec![create_mock_extraction_result()];

            let actual_count = c_results.len();
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: actual_count,
                success: true,
                _padding2: [0u8; 7],
            }));

            // Verify the batch result is valid
            assert!(!batch_result.is_null());
            assert_eq!((*batch_result).count, 1);
            assert!((*batch_result).success);

            // Free should handle single-element batch correctly
            kreuzberg_free_batch_result(batch_result);
        }
    }

    #[test]
    fn test_batch_result_large_size() {
        unsafe {
            // Test batch with 100 elements to catch boundary conditions
            // This verifies the system can handle larger batches without memory corruption
            let mut c_results = Vec::with_capacity(100);

            for _ in 0..100 {
                c_results.push(create_mock_extraction_result());
            }

            let actual_count = c_results.len();
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: actual_count,
                success: true,
                _padding2: [0u8; 7],
            }));

            // Verify the batch result is valid
            assert!(!batch_result.is_null());
            assert_eq!((*batch_result).count, 100);
            assert!((*batch_result).success);

            // Free should handle large batch correctly without memory issues
            kreuzberg_free_batch_result(batch_result);
        }
    }

    #[test]
    fn test_repeated_allocation_deallocation() {
        unsafe {
            // Stress test: 1000 iterations of allocation/deallocation
            // This catches memory leaks, corruption, and use-after-free issues
            for _ in 0..1000 {
                let result = create_mock_extraction_result();

                // Verify the result is valid
                assert!(!result.is_null());
                assert!((*result).success);

                // Free the result
                kreuzberg_free_result(result);
            }

            // If we got here without crashing or leaking, the memory management is sound
        }
    }

    // ==================== Box/Vec Symmetry Test ====================

    #[test]
    fn test_box_vec_symmetry() {
        unsafe {
            // This test verifies the fix for the allocation/deallocation bug
            // Create a Vec, convert to boxed slice, then verify correct deallocation

            let mut vec = Vec::with_capacity(5);
            vec.push(42u32);
            vec.push(100u32);
            vec.push(255u32);

            let len = vec.len();

            // Convert to boxed slice (this is what kreuzberg_batch_extract_files_sync does)
            let boxed_slice = vec.into_boxed_slice();
            let raw_ptr = Box::into_raw(boxed_slice) as *mut u32;

            // Verify we can read the values
            assert_eq!(*raw_ptr.add(0), 42);
            assert_eq!(*raw_ptr.add(1), 100);
            assert_eq!(*raw_ptr.add(2), 255);

            // Now deallocate using the correct method (from kreuzberg_free_batch_result)
            // IMPORTANT: Must use Box::from_raw with slice pointer, not Vec::from_raw_parts
            let _boxed_slice = Box::from_raw(std::ptr::slice_from_raw_parts_mut(raw_ptr, len));

            // If we got here without crashing, the symmetry is correct
        }
    }

    #[test]
    fn test_box_vec_symmetry_pointers() {
        unsafe {
            // Test with pointer types (as used in CBatchResult)
            let vec: Vec<*mut CExtractionResult> = vec![
                create_mock_extraction_result(),
                create_mock_extraction_result(),
                create_mock_extraction_result(),
            ];

            let len = vec.len();

            // Convert to boxed slice
            let boxed_slice = vec.into_boxed_slice();
            let raw_ptr = Box::into_raw(boxed_slice) as *mut *mut CExtractionResult;

            // Free individual results first
            for i in 0..len {
                let result_ptr = *raw_ptr.add(i);
                if !result_ptr.is_null() {
                    kreuzberg_free_result(result_ptr);
                }
            }

            // Now free the array itself
            let _boxed_slice = Box::from_raw(std::ptr::slice_from_raw_parts_mut(raw_ptr, len));

            // If we got here without crashing, the symmetry is correct
        }
    }

    // ==================== FFI Function Smoke Tests ====================

    #[test]
    fn test_version_not_null() {
        unsafe {
            let version = kreuzberg_version();
            assert!(!version.is_null(), "Version string should not be NULL");

            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert!(!version_str.is_empty(), "Version string should not be empty");

            // Verify it looks like a version string (has dots or numbers)
            assert!(
                version_str.contains('.') || version_str.chars().any(|c| c.is_numeric()),
                "Version string should contain version info"
            );
        }
    }

    #[test]
    fn test_null_config_handling() {
        unsafe {
            // Test that functions handle NULL config gracefully
            // kreuzberg_batch_extract_files_sync with NULL config should use defaults

            // Create a valid file paths array
            let path1 = CString::new("/tmp/test1.txt").unwrap();
            let path2 = CString::new("/tmp/test2.txt").unwrap();
            let paths = [path1.as_ptr(), path2.as_ptr()];

            // Create a file_config_jsons array with NULL entries (use base config for all)
            let file_configs: [*const c_char; 2] = [ptr::null(), ptr::null()];

            // This should not crash with NULL config (though it may fail due to missing files)
            let result = kreuzberg_batch_extract_files_sync(paths.as_ptr(), file_configs.as_ptr(), 2, ptr::null());

            // Result might be NULL due to file not existing, but it shouldn't crash
            if !result.is_null() {
                kreuzberg_free_batch_result(result);
            }
        }
    }

    #[test]
    fn test_extraction_result_free_with_null_fields() {
        unsafe {
            // Test freeing a result where most fields are NULL
            let result = Box::into_raw(Box::new(CExtractionResult {
                content: CString::new("content").unwrap().into_raw(),
                mime_type: CString::new("text/plain").unwrap().into_raw(),
                language: ptr::null_mut(),
                date: ptr::null_mut(),
                subject: ptr::null_mut(),
                tables_json: ptr::null_mut(),
                detected_languages_json: ptr::null_mut(),
                djot_content_json: ptr::null_mut(),
                metadata_json: ptr::null_mut(),
                chunks_json: ptr::null_mut(),
                children_json: ptr::null_mut(),
                images_json: ptr::null_mut(),
                page_structure_json: ptr::null_mut(),
                pages_json: ptr::null_mut(),
                elements_json: ptr::null_mut(),
                ocr_elements_json: ptr::null_mut(),
                document_json: ptr::null_mut(),
                extracted_keywords_json: ptr::null_mut(),
                quality_score_json: ptr::null_mut(),
                processing_warnings_json: ptr::null_mut(),
                annotations_json: ptr::null_mut(),
                uris_json: ptr::null_mut(),
                llm_usage_json: ptr::null_mut(),
                code_intelligence_json: ptr::null_mut(),
                structured_output_json: ptr::null_mut(),
                success: true,
                _padding1: [0u8; 7],
            }));

            // Should not crash when freeing result with NULL fields
            kreuzberg_free_result(result);
        }
    }

    #[test]
    fn test_extraction_result_free_all_fields_allocated() {
        unsafe {
            // Test freeing a result where ALL 21 string fields are allocated
            // This verifies that kreuzberg_free_result properly frees all fields
            let result = Box::into_raw(Box::new(CExtractionResult {
                content: CString::new("test content").unwrap().into_raw(),
                mime_type: CString::new("application/pdf").unwrap().into_raw(),
                language: CString::new("en").unwrap().into_raw(),
                date: CString::new("2024-01-01").unwrap().into_raw(),
                subject: CString::new("Test Subject").unwrap().into_raw(),
                tables_json: CString::new("[]").unwrap().into_raw(),
                detected_languages_json: CString::new("[\"en\"]").unwrap().into_raw(),
                djot_content_json: ptr::null_mut(),
                metadata_json: CString::new("{}").unwrap().into_raw(),
                chunks_json: CString::new("[{\"text\":\"chunk1\"}]").unwrap().into_raw(),
                children_json: ptr::null_mut(),
                images_json: CString::new("[{\"data\":\"base64\"}]").unwrap().into_raw(),
                page_structure_json: CString::new("{\"pages\":1}").unwrap().into_raw(),
                pages_json: CString::new("[{\"page\":1,\"content\":\"test\"}]").unwrap().into_raw(),
                elements_json: CString::new("[]").unwrap().into_raw(),
                ocr_elements_json: ptr::null_mut(),
                document_json: ptr::null_mut(),
                extracted_keywords_json: CString::new("[{\"text\":\"test\",\"score\":0.5}]").unwrap().into_raw(),
                quality_score_json: CString::new("0.85").unwrap().into_raw(),
                processing_warnings_json: CString::new("[{\"source\":\"chunking\",\"message\":\"warn\"}]")
                    .unwrap()
                    .into_raw(),
                annotations_json: ptr::null_mut(),
                uris_json: ptr::null_mut(),
                llm_usage_json: ptr::null_mut(),
                code_intelligence_json: ptr::null_mut(),
                structured_output_json: ptr::null_mut(),
                success: true,
                _padding1: [0u8; 7],
            }));

            // Should properly free all 20 allocated string fields without leaking memory
            kreuzberg_free_result(result);
        }
    }

    #[test]
    fn test_string_allocation_deallocation() {
        unsafe {
            // Test string cloning and freeing
            let original = CString::new("test string").unwrap();
            let cloned = kreuzberg_clone_string(original.as_ptr());

            assert!(!cloned.is_null(), "Cloned string should not be NULL");

            let cloned_str = CStr::from_ptr(cloned).to_str().unwrap();
            assert_eq!(cloned_str, "test string", "Cloned string should match original");

            // Free the cloned string
            kreuzberg_free_string(cloned);
        }
    }

    #[test]
    fn test_clone_null_string() {
        unsafe {
            // Cloning NULL should return NULL and set error
            clear_last_error();
            let cloned = kreuzberg_clone_string(ptr::null());

            assert!(cloned.is_null(), "Cloning NULL should return NULL");

            let error = kreuzberg_last_error();
            assert!(!error.is_null(), "Error should be set");
            let error_str = CStr::from_ptr(error).to_str().unwrap();
            assert!(error_str.contains("NULL"), "Error should mention NULL");
        }
    }

    #[test]
    fn test_batch_result_success_field() {
        unsafe {
            // Test that success field is properly set
            let c_results: Vec<*mut CExtractionResult> = Vec::new();
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            let batch_result = Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count: 0,
                success: true,
                _padding2: [0u8; 7],
            }));

            assert!((*batch_result).success, "Success field should be true");

            kreuzberg_free_batch_result(batch_result);
        }
    }

    #[test]
    fn test_last_error_cleared() {
        unsafe {
            // Test that clear_last_error works
            set_last_error("test error".to_string());

            let error = kreuzberg_last_error();
            assert!(!error.is_null());

            clear_last_error();

            let error_after = kreuzberg_last_error();
            assert!(error_after.is_null(), "Error should be cleared");
        }
    }

    // ==================== Additional Safety Net Tests (PR #1) ====================

    /// Test CExtractionResult size exactly matches FFI contract
    #[test]
    fn test_c_extraction_result_size() {
        assert_eq!(std::mem::size_of::<CExtractionResult>(), 208);
        assert_eq!(std::mem::align_of::<CExtractionResult>(), 8);
    }

    /// Test CBatchResult size exactly matches FFI contract
    #[test]
    fn test_c_batch_result_size() {
        assert_eq!(std::mem::size_of::<CBatchResult>(), 24);
        assert_eq!(std::mem::align_of::<CBatchResult>(), 8);
    }

    /// Test CBytesWithMime size exactly matches FFI contract
    #[test]
    fn test_c_bytes_with_mime_size() {
        assert_eq!(std::mem::size_of::<CBytesWithMime>(), 24);
        assert_eq!(std::mem::align_of::<CBytesWithMime>(), 8);
    }

    /// Test that kreuzberg_extract_bytes_sync handles NULL data pointer
    #[test]
    fn test_extract_bytes_null_data() {
        unsafe {
            let mime = CString::new("text/plain").unwrap();
            let result = kreuzberg_extract_bytes_sync(ptr::null(), 0, mime.as_ptr());
            assert!(result.is_null(), "Should return NULL for NULL data pointer");
        }
    }

    /// Test that kreuzberg_extract_bytes_sync handles NULL mime type
    #[test]
    fn test_extract_bytes_null_mime() {
        unsafe {
            let data = b"test data";
            let result = kreuzberg_extract_bytes_sync(data.as_ptr(), data.len(), ptr::null());
            assert!(result.is_null(), "Should return NULL for NULL mime type");
        }
    }

    /// Test that kreuzberg_batch_extract_files_sync handles NULL paths pointer
    #[test]
    fn test_batch_extract_null_paths() {
        unsafe {
            let result = kreuzberg_batch_extract_files_sync(ptr::null(), ptr::null(), 0, ptr::null());
            assert!(result.is_null(), "Should return NULL for NULL paths pointer");
        }
    }

    /// Test that kreuzberg_batch_extract_bytes_sync handles NULL bytes pointer
    #[test]
    fn test_batch_extract_bytes_null() {
        unsafe {
            let result = kreuzberg_batch_extract_bytes_sync(ptr::null(), ptr::null(), 0, ptr::null());
            assert!(result.is_null(), "Should return NULL for NULL bytes pointer");
        }
    }

    /// Test that kreuzberg_register_ocr_backend handles NULL name
    #[test]
    fn test_register_ocr_backend_null_name() {
        unsafe {
            extern "C" fn dummy_callback(_: *const u8, _: usize, _: *const c_char) -> *mut c_char {
                ptr::null_mut()
            }
            let result = kreuzberg_register_ocr_backend(ptr::null(), dummy_callback);
            assert!(!result, "Should return false for NULL backend name");
        }
    }

    /// Test that kreuzberg_unregister_ocr_backend handles NULL name
    #[test]
    fn test_unregister_ocr_backend_null_name() {
        unsafe {
            let result = kreuzberg_unregister_ocr_backend(ptr::null());
            assert!(!result, "Should return false for NULL backend name");
        }
    }

    /// Test that kreuzberg_register_post_processor handles NULL name
    #[test]
    fn test_register_post_processor_null_name() {
        unsafe {
            extern "C" fn dummy_callback(_: *const c_char) -> *mut c_char {
                ptr::null_mut()
            }
            let result = kreuzberg_register_post_processor(ptr::null(), dummy_callback, 0);
            assert!(!result, "Should return false for NULL processor name");
        }
    }

    /// Test that kreuzberg_unregister_post_processor handles NULL name
    #[test]
    fn test_unregister_post_processor_null_name() {
        unsafe {
            let result = kreuzberg_unregister_post_processor(ptr::null());
            assert!(!result, "Should return false for NULL processor name");
        }
    }

    /// Test that kreuzberg_register_validator handles NULL name
    #[test]
    fn test_register_validator_null_name() {
        unsafe {
            extern "C" fn dummy_callback(_: *const c_char) -> *mut c_char {
                ptr::null_mut()
            }
            let result = kreuzberg_register_validator(ptr::null(), dummy_callback, 0);
            assert!(!result, "Should return false for NULL validator name");
        }
    }

    /// Test that kreuzberg_unregister_validator handles NULL name
    #[test]
    fn test_unregister_validator_null_name() {
        unsafe {
            let result = kreuzberg_unregister_validator(ptr::null());
            assert!(!result, "Should return false for NULL validator name");
        }
    }

    /// Test that kreuzberg_get_ocr_languages handles NULL backend
    #[test]
    fn test_get_ocr_languages_null_backend() {
        unsafe {
            let result = kreuzberg_get_ocr_languages(ptr::null());
            assert!(result.is_null(), "Should return NULL for NULL backend name");
        }
    }

    /// Test that kreuzberg_is_language_supported handles NULL backend
    #[test]
    fn test_is_language_supported_null_backend() {
        unsafe {
            let lang = CString::new("en").unwrap();
            let result = kreuzberg_is_language_supported(ptr::null(), lang.as_ptr());
            assert_eq!(result, 0, "Should return 0 (false) for NULL backend");
        }
    }

    /// Test that kreuzberg_is_language_supported handles NULL language
    #[test]
    fn test_is_language_supported_null_language() {
        unsafe {
            let backend = CString::new("tesseract").unwrap();
            let result = kreuzberg_is_language_supported(backend.as_ptr(), ptr::null());
            assert_eq!(result, 0, "Should return 0 (false) for NULL language");
        }
    }

    /// Test that kreuzberg_validate_binarization_method handles NULL
    #[test]
    fn test_validate_binarization_method_null() {
        unsafe {
            let result = kreuzberg_validate_binarization_method(ptr::null());
            assert_eq!(result, 0, "Should return 0 (invalid) for NULL method");
        }
    }

    /// Test that kreuzberg_validate_token_reduction_level handles NULL
    #[test]
    fn test_validate_token_reduction_level_null() {
        unsafe {
            let result = kreuzberg_validate_token_reduction_level(ptr::null());
            assert_eq!(result, 0, "Should return 0 (invalid) for NULL level");
        }
    }

    /// Test that kreuzberg_validate_ocr_backend handles NULL
    #[test]
    fn test_validate_ocr_backend_null() {
        unsafe {
            let result = kreuzberg_validate_ocr_backend(ptr::null());
            assert_eq!(result, 0, "Should return 0 (invalid) for NULL backend");
        }
    }

    /// Test that kreuzberg_validate_language_code handles NULL
    #[test]
    fn test_validate_language_code_null() {
        unsafe {
            let result = kreuzberg_validate_language_code(ptr::null());
            assert_eq!(result, 0, "Should return 0 (invalid) for NULL language code");
        }
    }

    /// Test that kreuzberg_validate_output_format handles NULL
    #[test]
    fn test_validate_output_format_null() {
        unsafe {
            let result = kreuzberg_validate_output_format(ptr::null());
            assert_eq!(result, 0, "Should return 0 (invalid) for NULL format");
        }
    }

    /// Test that kreuzberg_version returns non-null
    #[test]
    fn test_version_returns_non_null() {
        unsafe {
            let version = kreuzberg_version();
            assert!(!version.is_null(), "kreuzberg_version should never return NULL");
            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert!(!version_str.is_empty(), "Version string should not be empty");
        }
    }

    /// Test that kreuzberg_last_error returns NULL when no error
    #[test]
    fn test_last_error_null_when_no_error() {
        unsafe {
            clear_last_error();
            let error = kreuzberg_last_error();
            assert!(error.is_null(), "Should return NULL when no error is set");
        }
    }

    /// Test that kreuzberg_clone_string returns non-null for valid input
    #[test]
    fn test_clone_string_returns_non_null() {
        unsafe {
            let input = CString::new("test").unwrap();
            let cloned = kreuzberg_clone_string(input.as_ptr());
            assert!(!cloned.is_null(), "Clone should return non-NULL for valid input");
            kreuzberg_free_string(cloned);
        }
    }

    /// Test clearing OCR backends doesn't crash
    #[test]
    fn test_clear_ocr_backends_doesnt_crash() {
        unsafe {
            // This should not crash even if called multiple times
            kreuzberg_clear_ocr_backends();
            kreuzberg_clear_ocr_backends();
        }
    }

    /// Test clearing post processors doesn't crash
    #[test]
    fn test_clear_post_processors_doesnt_crash() {
        unsafe {
            // This should not crash even if called multiple times
            kreuzberg_clear_post_processors();
            kreuzberg_clear_post_processors();
        }
    }

    /// Test clearing validators doesn't crash
    #[test]
    fn test_clear_validators_doesnt_crash() {
        unsafe {
            // This should not crash even if called multiple times
            kreuzberg_clear_validators();
            kreuzberg_clear_validators();
        }
    }

    /// Test clearing document extractors doesn't crash
    #[test]
    fn test_clear_document_extractors_doesnt_crash() {
        unsafe {
            // This should not crash even if called multiple times
            kreuzberg_clear_document_extractors();
            kreuzberg_clear_document_extractors();
        }
    }

    /// Test that list functions return non-null JSON arrays
    #[test]
    fn test_list_functions_return_non_null() {
        unsafe {
            // All list functions should return non-NULL JSON arrays (even if empty)
            let ocr = kreuzberg_list_ocr_backends();
            assert!(!ocr.is_null(), "list_ocr_backends should return non-NULL");
            kreuzberg_free_string(ocr);

            let processors = kreuzberg_list_post_processors();
            assert!(!processors.is_null(), "list_post_processors should return non-NULL");
            kreuzberg_free_string(processors);

            let validators = kreuzberg_list_validators();
            assert!(!validators.is_null(), "list_validators should return non-NULL");
            kreuzberg_free_string(validators);

            let extractors = kreuzberg_list_document_extractors();
            assert!(!extractors.is_null(), "list_document_extractors should return non-NULL");
            kreuzberg_free_string(extractors);

            let backends_with_langs = kreuzberg_list_ocr_backends_with_languages();
            assert!(
                !backends_with_langs.is_null(),
                "list_ocr_backends_with_languages should return non-NULL"
            );
            kreuzberg_free_string(backends_with_langs);
        }
    }

    /// Test numeric validation functions with edge cases
    #[test]
    fn test_numeric_validation_edge_cases() {
        // Test Tesseract PSM validation with invalid values
        assert_eq!(
            kreuzberg_validate_tesseract_psm(-1),
            0,
            "Negative PSM should be invalid"
        );
        assert_eq!(kreuzberg_validate_tesseract_psm(0), 1, "PSM 0 should be valid");
        assert_eq!(kreuzberg_validate_tesseract_psm(13), 1, "PSM 13 should be valid");
        assert_eq!(kreuzberg_validate_tesseract_psm(14), 0, "PSM 14 should be invalid");

        // Test Tesseract OEM validation
        assert_eq!(
            kreuzberg_validate_tesseract_oem(-1),
            0,
            "Negative OEM should be invalid"
        );
        assert_eq!(kreuzberg_validate_tesseract_oem(0), 1, "OEM 0 should be valid");
        assert_eq!(kreuzberg_validate_tesseract_oem(3), 1, "OEM 3 should be valid");
        assert_eq!(kreuzberg_validate_tesseract_oem(4), 0, "OEM 4 should be invalid");

        // Test confidence validation
        assert_eq!(
            kreuzberg_validate_confidence(-0.1),
            0,
            "Negative confidence should be invalid"
        );
        assert_eq!(kreuzberg_validate_confidence(0.0), 1, "0.0 confidence should be valid");
        assert_eq!(kreuzberg_validate_confidence(0.5), 1, "0.5 confidence should be valid");
        assert_eq!(kreuzberg_validate_confidence(1.0), 1, "1.0 confidence should be valid");
        assert_eq!(
            kreuzberg_validate_confidence(1.1),
            0,
            "1.1 confidence should be invalid"
        );

        // Test DPI validation
        assert_eq!(kreuzberg_validate_dpi(0), 0, "0 DPI should be invalid");
        assert_eq!(kreuzberg_validate_dpi(-1), 0, "-1 DPI should be invalid");
        assert_eq!(kreuzberg_validate_dpi(1), 1, "1 DPI should be valid");
        assert_eq!(kreuzberg_validate_dpi(72), 1, "72 DPI should be valid");
        assert_eq!(kreuzberg_validate_dpi(300), 1, "300 DPI should be valid");
        assert_eq!(kreuzberg_validate_dpi(2400), 1, "2400 DPI should be valid");
        assert_eq!(kreuzberg_validate_dpi(2401), 0, "2401 DPI should be invalid");

        // Test chunking params validation
        assert_eq!(
            kreuzberg_validate_chunking_params(0, 0),
            0,
            "0 max_chars should be invalid"
        );
        assert_eq!(
            kreuzberg_validate_chunking_params(100, 0),
            1,
            "Valid params should pass"
        );
        assert_eq!(
            kreuzberg_validate_chunking_params(100, 50),
            1,
            "Valid overlap should pass"
        );
        assert_eq!(
            kreuzberg_validate_chunking_params(100, 100),
            0,
            "Overlap >= max_chars should be invalid"
        );
        assert_eq!(
            kreuzberg_validate_chunking_params(100, 101),
            0,
            "Overlap > max_chars should be invalid"
        );
    }
}
