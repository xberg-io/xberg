//! Zero-copy result view FFI module.
//!
//! Provides direct read-only access to ExtractionResult fields without copying.
//! This eliminates memory allocation and JSON serialization overhead for common
//! field access patterns in language bindings.
//!
//! # Safety Model
//!
//! Views are **borrowed references** to ExtractionResult data. The caller MUST ensure:
//! 1. The source ExtractionResult outlives all views created from it
//! 2. Views are not used after the source result is freed
//! 3. Multi-threaded access requires external synchronization
//!
//! # Performance Benefits
//!
//! Zero-copy views eliminate:
//! - String allocation overhead (no `CString::new()` calls)
//! - JSON serialization for metadata/tables/chunks
//! - UTF-8 validation (already validated in Rust)
//! - Memory copying from Rust String → C string
//!
//! Expected performance improvement: 10-20% for large documents with many fields.

use crate::{clear_last_error, set_last_error};
use kreuzberg::types::ExtractionResult;
use std::ptr;

/// Zero-copy view into an ExtractionResult.
///
/// Provides direct pointers to string data without allocation or copying.
/// All pointers are valid UTF-8 byte slices (not null-terminated).
///
/// # Lifetime Safety
///
/// This structure contains borrowed pointers. The caller MUST ensure:
/// - The source `ExtractionResult` outlives this view
/// - No use after the source result is freed with `kreuzberg_result_free()`
///
/// # Memory Layout
///
/// Field order: 6 ptr+len pairs (96 bytes) + 5 counts (40 bytes) = 136 bytes on 64-bit systems
/// All pointers are either valid UTF-8 data or NULL (with corresponding len=0).
///
/// # Thread Safety
///
/// Views are NOT thread-safe. External synchronization required for concurrent access.
#[repr(C)]
pub struct CExtractionResultView {
    /// Direct pointer to content bytes (UTF-8, not null-terminated)
    pub content_ptr: *const u8,
    /// Length of content in bytes
    pub content_len: usize,

    /// Direct pointer to MIME type bytes (UTF-8, not null-terminated)
    pub mime_type_ptr: *const u8,
    /// Length of MIME type in bytes
    pub mime_type_len: usize,

    /// Direct pointer to language bytes (UTF-8, not null-terminated), or NULL
    pub language_ptr: *const u8,
    /// Length of language in bytes (0 if NULL)
    pub language_len: usize,

    /// Direct pointer to date bytes (UTF-8, not null-terminated), or NULL
    pub date_ptr: *const u8,
    /// Length of date in bytes (0 if NULL)
    pub date_len: usize,

    /// Direct pointer to subject bytes (UTF-8, not null-terminated), or NULL
    pub subject_ptr: *const u8,
    /// Length of subject in bytes (0 if NULL)
    pub subject_len: usize,

    /// Direct pointer to title bytes (UTF-8, not null-terminated), or NULL
    pub title_ptr: *const u8,
    /// Length of title in bytes (0 if NULL)
    pub title_len: usize,

    /// Number of tables extracted
    pub table_count: usize,

    /// Number of chunks (0 if chunking not enabled)
    pub chunk_count: usize,

    /// Number of detected languages (0 if language detection not enabled)
    pub detected_language_count: usize,

    /// Number of extracted images (0 if no images)
    pub image_count: usize,

    /// Total page count (0 if not applicable)
    pub page_count: usize,
}

/// Get a zero-copy view of an extraction result.
///
/// Creates a view structure with direct pointers to result data without allocation.
/// The view is valid only while the source `result` remains valid.
///
/// # Arguments
///
/// * `result` - Pointer to an ExtractionResult structure
/// * `out_view` - Pointer to a CExtractionResultView structure to populate
///
/// # Returns
///
/// 0 on success, -1 on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `result` must be a valid pointer to an ExtractionResult
/// - `out_view` must be a valid pointer to writable memory
/// - Neither parameter can be NULL
/// - The returned view is valid ONLY while `result` is not freed
/// - Caller MUST NOT use the view after calling `kreuzberg_result_free(result)`
///
/// # Lifetime Safety
///
/// ```text
/// ExtractionResult lifetime: |-------------------------------------|
/// View lifetime:              |----------------------|
///                                   SAFE             FREE → INVALID
/// ```
///
/// # Example (C)
///
/// ```c
/// ExtractionResult* result = kreuzberg_extract_file("document.pdf", NULL);
/// if (result != NULL) {
///     CExtractionResultView view;
///     if (kreuzberg_get_result_view(result, &view) == 0) {
///         // Direct access to content without copying
///         printf("Content length: %zu bytes\n", view.content_len);
///         printf("MIME type: %.*s\n", (int)view.mime_type_len, view.mime_type_ptr);
///         printf("Tables: %zu, Chunks: %zu\n", view.table_count, view.chunk_count);
///
///         // No need to free the view (no allocations)
///     }
///
///     kreuzberg_result_free(result); // After this, view is INVALID
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_get_result_view(
    result: *const ExtractionResult,
    out_view: *mut CExtractionResultView,
) -> i32 {
    if result.is_null() {
        set_last_error("Result cannot be NULL".to_string());
        return -1;
    }

    if out_view.is_null() {
        set_last_error("Output view cannot be NULL".to_string());
        return -1;
    }

    clear_last_error();

    let result_ref = unsafe { &*result };

    unsafe {
        let content_bytes = result_ref.content.as_bytes();
        (*out_view).content_ptr = content_bytes.as_ptr();
        (*out_view).content_len = content_bytes.len();

        let mime_bytes = result_ref.mime_type.as_bytes();
        (*out_view).mime_type_ptr = mime_bytes.as_ptr();
        (*out_view).mime_type_len = mime_bytes.len();

        if let Some(ref language) = result_ref.metadata.language {
            let lang_bytes = language.as_bytes();
            (*out_view).language_ptr = lang_bytes.as_ptr();
            (*out_view).language_len = lang_bytes.len();
        } else {
            (*out_view).language_ptr = ptr::null();
            (*out_view).language_len = 0;
        }

        if let Some(ref created_at) = result_ref.metadata.created_at {
            let created_at_bytes = created_at.as_bytes();
            (*out_view).date_ptr = created_at_bytes.as_ptr();
            (*out_view).date_len = created_at_bytes.len();
        } else {
            (*out_view).date_ptr = ptr::null();
            (*out_view).date_len = 0;
        }

        if let Some(ref subject) = result_ref.metadata.subject {
            let subject_bytes = subject.as_bytes();
            (*out_view).subject_ptr = subject_bytes.as_ptr();
            (*out_view).subject_len = subject_bytes.len();
        } else {
            (*out_view).subject_ptr = ptr::null();
            (*out_view).subject_len = 0;
        }

        if let Some(ref title) = result_ref.metadata.title {
            let title_bytes = title.as_bytes();
            (*out_view).title_ptr = title_bytes.as_ptr();
            (*out_view).title_len = title_bytes.len();
        } else {
            (*out_view).title_ptr = ptr::null();
            (*out_view).title_len = 0;
        }

        (*out_view).table_count = result_ref.tables.len();
        (*out_view).chunk_count = result_ref.chunks.as_ref().map_or(0, |c| c.len());
        (*out_view).detected_language_count = result_ref.detected_languages.as_ref().map_or(0, |l| l.len());
        (*out_view).image_count = result_ref.images.as_ref().map_or(0, |i| i.len());
        (*out_view).page_count = result_ref.metadata.pages.as_ref().map_or(0, |p| p.total_count);
    }

    0
}

/// Internal helper: create a zero-copy view by value (for internal use).
///
/// This is a convenience function for internal FFI modules that need to
/// create views without having to allocate and populate an output structure.
///
/// # Safety
///
/// - `result` must be a valid reference to ExtractionResult
/// - Returned view is only valid while `result` is alive
pub(crate) fn create_result_view(result: &ExtractionResult) -> CExtractionResultView {
    let mut view = CExtractionResultView {
        content_ptr: ptr::null(),
        content_len: 0,
        mime_type_ptr: ptr::null(),
        mime_type_len: 0,
        language_ptr: ptr::null(),
        language_len: 0,
        date_ptr: ptr::null(),
        date_len: 0,
        subject_ptr: ptr::null(),
        subject_len: 0,
        title_ptr: ptr::null(),
        title_len: 0,
        table_count: 0,
        chunk_count: 0,
        detected_language_count: 0,
        image_count: 0,
        page_count: 0,
    };

    let content_bytes = result.content.as_bytes();
    view.content_ptr = content_bytes.as_ptr();
    view.content_len = content_bytes.len();

    let mime_bytes = result.mime_type.as_bytes();
    view.mime_type_ptr = mime_bytes.as_ptr();
    view.mime_type_len = mime_bytes.len();

    if let Some(ref language) = result.metadata.language {
        let lang_bytes = language.as_bytes();
        view.language_ptr = lang_bytes.as_ptr();
        view.language_len = lang_bytes.len();
    }

    if let Some(ref created_at) = result.metadata.created_at {
        let created_at_bytes = created_at.as_bytes();
        view.date_ptr = created_at_bytes.as_ptr();
        view.date_len = created_at_bytes.len();
    }

    if let Some(ref subject) = result.metadata.subject {
        let subject_bytes = subject.as_bytes();
        view.subject_ptr = subject_bytes.as_ptr();
        view.subject_len = subject_bytes.len();
    }

    if let Some(ref title) = result.metadata.title {
        let title_bytes = title.as_bytes();
        view.title_ptr = title_bytes.as_ptr();
        view.title_len = title_bytes.len();
    }

    view.table_count = result.tables.len();
    view.chunk_count = result.chunks.as_ref().map_or(0, |c| c.len());
    view.detected_language_count = result.detected_languages.as_ref().map_or(0, |l| l.len());
    view.image_count = result.images.as_ref().map_or(0, |i| i.len());
    view.page_count = result.metadata.pages.as_ref().map_or(0, |p| p.total_count);

    view
}

/// Get direct access to content from a result view.
///
/// Helper function to retrieve content as a slice without copying.
///
/// # Arguments
///
/// * `view` - Pointer to a CExtractionResultView structure
/// * `out_ptr` - Pointer to receive the content pointer
/// * `out_len` - Pointer to receive the content length
///
/// # Returns
///
/// 0 on success, -1 on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `view` must be a valid pointer to a CExtractionResultView
/// - `out_ptr` and `out_len` must be valid writable pointers
/// - The returned content pointer is valid only while the source ExtractionResult is valid
///
/// # Example (C)
///
/// ```c
/// const uint8_t* content;
/// size_t content_len;
/// if (kreuzberg_view_get_content(&view, &content, &content_len) == 0) {
///     // Process content directly without copying
///     fwrite(content, 1, content_len, stdout);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_view_get_content(
    view: *const CExtractionResultView,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if view.is_null() {
        set_last_error("View cannot be NULL".to_string());
        return -1;
    }

    if out_ptr.is_null() || out_len.is_null() {
        set_last_error("Output pointers cannot be NULL".to_string());
        return -1;
    }

    clear_last_error();

    unsafe {
        *out_ptr = (*view).content_ptr;
        *out_len = (*view).content_len;
    }

    0
}

/// Get direct access to MIME type from a result view.
///
/// # Arguments
///
/// * `view` - Pointer to a CExtractionResultView structure
/// * `out_ptr` - Pointer to receive the MIME type pointer
/// * `out_len` - Pointer to receive the MIME type length
///
/// # Returns
///
/// 0 on success, -1 on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `view` must be a valid pointer to a CExtractionResultView
/// - `out_ptr` and `out_len` must be valid writable pointers
/// - The returned MIME type pointer is valid only while the source ExtractionResult is valid
///
/// # Example (C)
///
/// ```c
/// const uint8_t* mime_type;
/// size_t mime_len;
/// if (kreuzberg_view_get_mime_type(&view, &mime_type, &mime_len) == 0) {
///     printf("MIME: %.*s\n", (int)mime_len, mime_type);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_view_get_mime_type(
    view: *const CExtractionResultView,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if view.is_null() {
        set_last_error("View cannot be NULL".to_string());
        return -1;
    }

    if out_ptr.is_null() || out_len.is_null() {
        set_last_error("Output pointers cannot be NULL".to_string());
        return -1;
    }

    clear_last_error();

    unsafe {
        *out_ptr = (*view).mime_type_ptr;
        *out_len = (*view).mime_type_len;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::types::{Metadata, PageStructure, PageUnitType};
    use std::mem;

    fn create_test_result() -> ExtractionResult {
        let mut metadata = Metadata::default();
        metadata.title = Some("Test Document".to_string());
        metadata.language = Some("en".to_string());
        metadata.created_at = Some("2025-01-01".to_string());
        metadata.subject = Some("Test Subject".to_string());

        let page_structure = PageStructure {
            total_count: 10,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: None,
        };
        metadata.pages = Some(page_structure);

        ExtractionResult {
            content: "Sample content for zero-copy testing".to_string(),
            mime_type: "text/plain".to_string(),
            metadata,
            tables: vec![],
            detected_languages: Some(vec!["en".to_string(), "de".to_string()]),
            chunks: Some(vec![
                kreuzberg::types::Chunk {
                    content: "Chunk 1".to_string(),
                    embedding: None,
                    metadata: kreuzberg::types::ChunkMetadata {
                        byte_start: 0,
                        byte_end: 7,
                        token_count: None,
                        chunk_index: 0,
                        total_chunks: 2,
                        first_page: None,
                        last_page: None,
                    },
                },
                kreuzberg::types::Chunk {
                    content: "Chunk 2".to_string(),
                    embedding: None,
                    metadata: kreuzberg::types::ChunkMetadata {
                        byte_start: 8,
                        byte_end: 15,
                        token_count: None,
                        chunk_index: 1,
                        total_chunks: 2,
                        first_page: None,
                        last_page: None,
                    },
                },
            ]),
            images: None,
            pages: None,
        }
    }

    #[test]
    fn test_result_view_structure_size() {
        let size = mem::size_of::<CExtractionResultView>();
        assert_eq!(
            size, 136,
            "View structure size should be 136 bytes (6 ptr+len pairs + 5 counts)"
        );
    }

    #[test]
    fn test_get_result_view_success() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0, "Should return success");

        assert!(!view.content_ptr.is_null());
        assert_eq!(view.content_len, result.content.len());

        assert!(!view.mime_type_ptr.is_null());
        assert_eq!(view.mime_type_len, result.mime_type.len());

        assert!(!view.language_ptr.is_null());
        assert_eq!(view.language_len, 2);

        assert!(!view.title_ptr.is_null());
        assert_eq!(view.title_len, "Test Document".len());

        assert_eq!(view.chunk_count, 2);
        assert_eq!(view.detected_language_count, 2);
        assert_eq!(view.page_count, 10);
        assert_eq!(view.table_count, 0);
        assert_eq!(view.image_count, 0);

        let content_slice = unsafe { std::slice::from_raw_parts(view.content_ptr, view.content_len) };
        assert_eq!(content_slice, result.content.as_bytes());

        let mime_slice = unsafe { std::slice::from_raw_parts(view.mime_type_ptr, view.mime_type_len) };
        assert_eq!(mime_slice, result.mime_type.as_bytes());
    }

    #[test]
    fn test_get_result_view_null_result() {
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };
        let ret = unsafe { kreuzberg_get_result_view(ptr::null(), &mut view) };
        assert_eq!(ret, -1, "Should return error for NULL result");
    }

    #[test]
    fn test_get_result_view_null_output() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let ret = unsafe { kreuzberg_get_result_view(result_ptr, ptr::null_mut()) };
        assert_eq!(ret, -1, "Should return error for NULL output");
    }

    #[test]
    fn test_view_get_content() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

        let mut content_ptr: *const u8 = ptr::null();
        let mut content_len: usize = 0;

        let ret = unsafe { kreuzberg_view_get_content(&view, &mut content_ptr, &mut content_len) };

        assert_eq!(ret, 0, "Should return success");
        assert!(!content_ptr.is_null());
        assert_eq!(content_len, result.content.len());

        let content_slice = unsafe { std::slice::from_raw_parts(content_ptr, content_len) };
        assert_eq!(content_slice, result.content.as_bytes());
    }

    #[test]
    fn test_view_get_mime_type() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

        let mut mime_ptr: *const u8 = ptr::null();
        let mut mime_len: usize = 0;

        let ret = unsafe { kreuzberg_view_get_mime_type(&view, &mut mime_ptr, &mut mime_len) };

        assert_eq!(ret, 0, "Should return success");
        assert!(!mime_ptr.is_null());
        assert_eq!(mime_len, result.mime_type.len());

        let mime_slice = unsafe { std::slice::from_raw_parts(mime_ptr, mime_len) };
        assert_eq!(mime_slice, result.mime_type.as_bytes());
    }

    #[test]
    fn test_view_optional_fields_null() {
        let mut result = create_test_result();
        result.metadata.language = None;
        result.metadata.title = None;

        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0);

        assert!(view.language_ptr.is_null());
        assert_eq!(view.language_len, 0);

        assert!(view.title_ptr.is_null());
        assert_eq!(view.title_len, 0);
    }

    #[test]
    fn test_view_lifetime_safety_pattern() {
        let result = create_test_result();
        let expected_content = result.content.clone();

        {
            let result_ptr = &result as *const ExtractionResult;
            let mut view: CExtractionResultView = unsafe { mem::zeroed() };

            unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

            let content_slice = unsafe { std::slice::from_raw_parts(view.content_ptr, view.content_len) };
            assert_eq!(content_slice, expected_content.as_bytes());
        }

        assert_eq!(result.content, expected_content);
    }

    #[test]
    fn test_zero_copy_no_allocation() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;

        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

        let content_start = result.content.as_ptr() as usize;
        let content_end = content_start + result.content.len();
        let view_ptr = view.content_ptr as usize;

        assert!(
            view_ptr >= content_start && view_ptr < content_end,
            "View pointer should point into result's memory"
        );
    }

    #[test]
    fn test_view_get_content_null_view() {
        let mut content_ptr: *const u8 = ptr::null();
        let mut content_len: usize = 0;

        let ret = unsafe { kreuzberg_view_get_content(ptr::null(), &mut content_ptr, &mut content_len) };

        assert_eq!(ret, -1, "Should return error for NULL view");
    }

    #[test]
    fn test_view_get_content_null_outputs() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

        let mut content_len: usize = 0;
        let ret = unsafe { kreuzberg_view_get_content(&view, ptr::null_mut(), &mut content_len) };
        assert_eq!(ret, -1, "Should return error for NULL out_ptr");

        let mut content_ptr: *const u8 = ptr::null();
        let ret = unsafe { kreuzberg_view_get_content(&view, &mut content_ptr, ptr::null_mut()) };
        assert_eq!(ret, -1, "Should return error for NULL out_len");
    }

    #[test]
    fn test_view_get_mime_type_null_view() {
        let mut mime_ptr: *const u8 = ptr::null();
        let mut mime_len: usize = 0;

        let ret = unsafe { kreuzberg_view_get_mime_type(ptr::null(), &mut mime_ptr, &mut mime_len) };

        assert_eq!(ret, -1, "Should return error for NULL view");
    }

    #[test]
    fn test_view_empty_content() {
        let mut result = create_test_result();
        result.content = String::new();

        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0);

        assert!(
            !view.content_ptr.is_null(),
            "Empty string should still have valid pointer"
        );
        assert_eq!(view.content_len, 0);
    }

    #[test]
    fn test_view_large_content() {
        let mut result = create_test_result();
        result.content = "x".repeat(10 * 1024 * 1024);
        let expected_len = result.content.len();

        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0);

        assert_eq!(view.content_len, expected_len);
        assert!(!view.content_ptr.is_null());

        let content_slice = unsafe { std::slice::from_raw_parts(view.content_ptr, view.content_len) };
        assert_eq!(content_slice, result.content.as_bytes());
    }

    #[test]
    fn test_view_unicode_content() {
        let mut result = create_test_result();
        result.content = "Hello 世界 🌍 Привет مرحبا".to_string();
        result.metadata.title = Some("Título español 中文标题".to_string());

        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0);

        let content_slice = unsafe { std::slice::from_raw_parts(view.content_ptr, view.content_len) };
        assert_eq!(content_slice, result.content.as_bytes());

        let title_slice = unsafe { std::slice::from_raw_parts(view.title_ptr, view.title_len) };
        let title_str = std::str::from_utf8(title_slice).unwrap();
        assert_eq!(title_str, "Título español 中文标题");
    }

    #[test]
    fn test_view_all_counts_zero() {
        let result = ExtractionResult {
            content: "Minimal content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        };

        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        let ret = unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };
        assert_eq!(ret, 0);

        assert_eq!(view.table_count, 0);
        assert_eq!(view.chunk_count, 0);
        assert_eq!(view.detected_language_count, 0);
        assert_eq!(view.image_count, 0);
        assert_eq!(view.page_count, 0);
    }

    #[test]
    fn test_view_multiple_views_same_result() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;

        let mut view1: CExtractionResultView = unsafe { mem::zeroed() };
        let mut view2: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe {
            kreuzberg_get_result_view(result_ptr, &mut view1);
            kreuzberg_get_result_view(result_ptr, &mut view2);
        }

        assert_eq!(view1.content_ptr, view2.content_ptr);
        assert_eq!(view1.content_len, view2.content_len);
        assert_eq!(view1.mime_type_ptr, view2.mime_type_ptr);
        assert_eq!(view1.mime_type_len, view2.mime_type_len);
        assert_eq!(view1.table_count, view2.table_count);
        assert_eq!(view1.chunk_count, view2.chunk_count);
    }

    #[test]
    fn test_view_field_isolation() {
        let result = create_test_result();
        let result_ptr = &result as *const ExtractionResult;
        let mut view: CExtractionResultView = unsafe { mem::zeroed() };

        unsafe { kreuzberg_get_result_view(result_ptr, &mut view) };

        assert_ne!(view.content_ptr, view.mime_type_ptr);

        if !view.language_ptr.is_null() && !view.title_ptr.is_null() {
            assert_ne!(view.language_ptr, view.title_ptr);
        }

        assert_eq!(view.mime_type_len, "text/plain".len());
        assert_eq!(view.language_len, "en".len());
    }
}
