//! Type definitions for C FFI compatibility.
//!
//! This module contains all C-compatible struct definitions used across the FFI boundary.
//! These types must maintain strict memory layout guarantees to ensure compatibility with
//! other languages (Java via Panama FFI, Go via cgo, C# via P/Invoke, etc.).

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// RAII guard for C strings that ensures proper cleanup.
///
/// This guard owns a raw C string pointer and automatically frees it when dropped,
/// preventing memory leaks. It can also transfer ownership via `into_raw()`.
///
/// # Memory Safety
///
/// - The guard takes ownership of a `CString` and converts it to a raw pointer
/// - On drop, it reconstructs the `CString` and drops it, freeing the memory
/// - If `into_raw()` is called, ownership is transferred and the drop is skipped
pub struct CStringGuard {
    ptr: *mut c_char,
}

impl CStringGuard {
    /// Create a new guard from a CString, transferring ownership of the raw pointer
    pub fn new(s: CString) -> Self {
        Self { ptr: s.into_raw() }
    }

    /// Transfer ownership of the raw pointer to the caller, preventing cleanup
    pub fn into_raw(mut self) -> *mut c_char {
        let ptr = self.ptr;
        self.ptr = ptr::null_mut();
        ptr
    }
}

impl Drop for CStringGuard {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { drop(CString::from_raw(self.ptr)) };
        }
    }
}

/// C-compatible extraction result structure
///
/// This struct must maintain a stable ABI and memory layout for FFI compatibility.
///
/// # Memory Layout
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 22 pointers (8 bytes each) + 1 bool + 7 bytes padding = 184 bytes total
///
/// The `#[repr(C)]` attribute ensures the struct follows C's memory layout rules:
/// - Fields are laid out in order
/// - Padding is added to maintain alignment
/// - The struct has the same size and alignment on all platforms (for 64-bit)
///
/// # Memory Management
///
/// All pointer fields are owned by the caller and must be freed using `kreuzberg_free_string`.
/// The struct itself must be freed using `kreuzberg_free_extraction_result`.
#[repr(C)]
pub struct CExtractionResult {
    /// JSON-serialized PDF annotations array (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub annotations_json: *mut c_char,
    /// Text chunks as JSON array (null-terminated, or null pointer if not available, must be freed with kreuzberg_free_string)
    pub chunks_json: *mut c_char,
    /// JSON-serialized children extraction results array (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub children_json: *mut c_char,
    /// Extracted text content (null-terminated UTF-8 string, must be freed with kreuzberg_free_string)
    pub content: *mut c_char,
    /// Document date (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub date: *mut c_char,
    /// Detected languages as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub detected_languages_json: *mut c_char,
    /// JSON-serialized Djot content (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub djot_content_json: *mut c_char,
    /// Document structure as JSON object (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub document_json: *mut c_char,
    /// Semantic elements as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub elements_json: *mut c_char,
    /// JSON-serialized extracted keywords (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub extracted_keywords_json: *mut c_char,
    /// Extracted images as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub images_json: *mut c_char,
    /// Document language (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub language: *mut c_char,
    /// Metadata as JSON object (null-terminated string, or NULL if no metadata, must be freed with kreuzberg_free_string)
    pub metadata_json: *mut c_char,
    /// Detected MIME type (null-terminated string, must be freed with kreuzberg_free_string)
    pub mime_type: *mut c_char,
    /// OCR elements as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub ocr_elements_json: *mut c_char,
    /// Page structure as JSON object (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub page_structure_json: *mut c_char,
    /// Per-page content as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub pages_json: *mut c_char,
    /// JSON-serialized processing warnings array (null-terminated, or null pointer if empty, must be freed with kreuzberg_free_string)
    pub processing_warnings_json: *mut c_char,
    /// JSON-serialized quality score (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub quality_score_json: *mut c_char,
    /// Document subject (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub subject: *mut c_char,
    /// Tables as JSON array (null-terminated string, or NULL if no tables, must be freed with kreuzberg_free_string)
    pub tables_json: *mut c_char,
    /// JSON-serialized URIs/links array (null-terminated, or null pointer if none, must be freed with kreuzberg_free_string)
    pub uris_json: *mut c_char,
    /// Whether extraction was successful
    pub success: bool,
    /// Padding to match Java MemoryLayout (7 bytes padding to align to 8-byte boundary)
    pub _padding1: [u8; 7],
}

/// C-compatible structure for passing byte array with MIME type in batch operations
///
/// # Memory Layout
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 1 pointer (8 bytes) + 1 usize (8 bytes) + 1 pointer (8 bytes) = 24 bytes total
///
/// The `#[repr(C)]` attribute ensures consistent memory layout across languages.
///
/// # Usage
///
/// This struct is used to pass document data to batch extraction functions. The caller
/// retains ownership of the data and mime_type pointers.
#[repr(C)]
pub struct CBytesWithMime {
    /// Pointer to byte data
    pub data: *const u8,
    /// Length of byte data
    pub data_len: usize,
    /// MIME type as null-terminated C string
    pub mime_type: *const c_char,
}

/// C-compatible structure for batch extraction results
///
/// # Memory Layout
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 1 pointer (8 bytes) + 1 usize (8 bytes) + 1 bool + 7 bytes padding = 24 bytes total
///
/// The padding ensures the struct is properly aligned for 64-bit architectures.
///
/// # Memory Management
///
/// - The `results` array must be freed using `kreuzberg_free_batch_result`
/// - Each individual result in the array must also be freed
#[repr(C)]
pub struct CBatchResult {
    /// Number of results
    pub count: usize,
    /// Array of extraction results
    pub results: *mut *mut CExtractionResult,
    /// Whether batch operation was successful
    pub success: bool,
    /// Padding to match Java MemoryLayout (7 bytes padding to align to 8-byte boundary)
    pub _padding2: [u8; 7],
}

/// Compile-time layout assertions to ensure ABI stability.
///
/// These assertions verify that the struct layouts match the expected sizes and alignments.
/// If these fail at compile time, it indicates a breaking change in the memory layout.
#[allow(non_upper_case_globals)]
const _: () = {
    const fn assert_c_extraction_result_size() {
        const SIZE: usize = std::mem::size_of::<CExtractionResult>();
        const _: () = assert!(SIZE == 184, "CExtractionResult size must be 184 bytes");
    }

    const fn assert_c_extraction_result_alignment() {
        const ALIGN: usize = std::mem::align_of::<CExtractionResult>();
        const _: () = assert!(ALIGN == 8, "CExtractionResult alignment must be 8 bytes");
    }

    const fn assert_c_batch_result_size() {
        const SIZE: usize = std::mem::size_of::<CBatchResult>();
        const _: () = assert!(SIZE == 24, "CBatchResult size must be 24 bytes");
    }

    const fn assert_c_batch_result_alignment() {
        const ALIGN: usize = std::mem::align_of::<CBatchResult>();
        const _: () = assert!(ALIGN == 8, "CBatchResult alignment must be 8 bytes");
    }

    const fn assert_c_bytes_with_mime_size() {
        const SIZE: usize = std::mem::size_of::<CBytesWithMime>();
        const _: () = assert!(SIZE == 24, "CBytesWithMime size must be 24 bytes");
    }

    const fn assert_c_bytes_with_mime_alignment() {
        const ALIGN: usize = std::mem::align_of::<CBytesWithMime>();
        const _: () = assert!(ALIGN == 8, "CBytesWithMime alignment must be 8 bytes");
    }

    let _ = assert_c_extraction_result_size;
    let _ = assert_c_extraction_result_alignment;
    let _ = assert_c_batch_result_size;
    let _ = assert_c_batch_result_alignment;
    let _ = assert_c_bytes_with_mime_size;
    let _ = assert_c_bytes_with_mime_alignment;
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that CExtractionResult has the correct size
    #[test]
    fn test_c_extraction_result_size() {
        assert_eq!(
            std::mem::size_of::<CExtractionResult>(),
            184,
            "CExtractionResult must be exactly 184 bytes"
        );
    }

    /// Test that CExtractionResult has the correct alignment
    #[test]
    fn test_c_extraction_result_alignment() {
        assert_eq!(
            std::mem::align_of::<CExtractionResult>(),
            8,
            "CExtractionResult must be 8-byte aligned"
        );
    }

    /// Test that CBatchResult has the correct size
    #[test]
    fn test_c_batch_result_size() {
        assert_eq!(
            std::mem::size_of::<CBatchResult>(),
            24,
            "CBatchResult must be exactly 24 bytes"
        );
    }

    /// Test that CBatchResult has the correct alignment
    #[test]
    fn test_c_batch_result_alignment() {
        assert_eq!(
            std::mem::align_of::<CBatchResult>(),
            8,
            "CBatchResult must be 8-byte aligned"
        );
    }

    /// Test that CBytesWithMime has the correct size
    #[test]
    fn test_c_bytes_with_mime_size() {
        assert_eq!(
            std::mem::size_of::<CBytesWithMime>(),
            24,
            "CBytesWithMime must be exactly 24 bytes"
        );
    }

    /// Test that CBytesWithMime has the correct alignment
    #[test]
    fn test_c_bytes_with_mime_alignment() {
        assert_eq!(
            std::mem::align_of::<CBytesWithMime>(),
            8,
            "CBytesWithMime must be 8-byte aligned"
        );
    }

    /// Test CStringGuard RAII behavior - normal drop
    #[test]
    fn test_c_string_guard_drop() {
        let original = CString::new("test string").unwrap();
        let guard = CStringGuard::new(original);
        // Guard should automatically free the string when it goes out of scope
        drop(guard);
        // If this test completes without crashing, the RAII behavior is working
    }

    /// Test CStringGuard RAII behavior - into_raw transfer
    #[test]
    fn test_c_string_guard_into_raw() {
        let original = CString::new("test string").unwrap();
        let guard = CStringGuard::new(original);
        let ptr = guard.into_raw();

        assert!(!ptr.is_null(), "into_raw should return a non-null pointer");

        // Manually free the string since we took ownership
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }

    /// Test CStringGuard with empty string
    #[test]
    fn test_c_string_guard_empty() {
        let original = CString::new("").unwrap();
        let guard = CStringGuard::new(original);
        let ptr = guard.into_raw();

        assert!(!ptr.is_null(), "Empty string should still have a valid pointer");

        unsafe {
            let recovered = CString::from_raw(ptr);
            assert_eq!(recovered.to_str().unwrap(), "");
        }
    }

    /// Test that CStringGuard doesn't double-free
    #[test]
    fn test_c_string_guard_no_double_free() {
        let original = CString::new("test").unwrap();
        let mut guard = CStringGuard::new(original);

        // Manually set to null to simulate into_raw behavior
        let ptr = guard.ptr;
        guard.ptr = ptr::null_mut();

        // This should not attempt to free anything
        drop(guard);

        // Clean up the actual pointer
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }

    /// Verify field offsets in CExtractionResult match expectations
    #[test]
    fn test_c_extraction_result_field_offsets() {
        use std::mem::offset_of;

        // All pointer fields should be 8 bytes each
        assert_eq!(offset_of!(CExtractionResult, annotations_json), 0);
        assert_eq!(offset_of!(CExtractionResult, chunks_json), 8);
        assert_eq!(offset_of!(CExtractionResult, children_json), 16);
        assert_eq!(offset_of!(CExtractionResult, content), 24);
        assert_eq!(offset_of!(CExtractionResult, date), 32);
        assert_eq!(offset_of!(CExtractionResult, detected_languages_json), 40);
        assert_eq!(offset_of!(CExtractionResult, djot_content_json), 48);
        assert_eq!(offset_of!(CExtractionResult, document_json), 56);
        assert_eq!(offset_of!(CExtractionResult, elements_json), 64);
        assert_eq!(offset_of!(CExtractionResult, extracted_keywords_json), 72);
        assert_eq!(offset_of!(CExtractionResult, images_json), 80);
        assert_eq!(offset_of!(CExtractionResult, language), 88);
        assert_eq!(offset_of!(CExtractionResult, metadata_json), 96);
        assert_eq!(offset_of!(CExtractionResult, mime_type), 104);
        assert_eq!(offset_of!(CExtractionResult, ocr_elements_json), 112);
        assert_eq!(offset_of!(CExtractionResult, page_structure_json), 120);
        assert_eq!(offset_of!(CExtractionResult, pages_json), 128);
        assert_eq!(offset_of!(CExtractionResult, processing_warnings_json), 136);
        assert_eq!(offset_of!(CExtractionResult, quality_score_json), 144);
        assert_eq!(offset_of!(CExtractionResult, subject), 152);
        assert_eq!(offset_of!(CExtractionResult, tables_json), 160);
        assert_eq!(offset_of!(CExtractionResult, uris_json), 168);
        assert_eq!(offset_of!(CExtractionResult, success), 176);
    }

    /// Verify field offsets in CBatchResult match expectations
    #[test]
    fn test_c_batch_result_field_offsets() {
        use std::mem::offset_of;

        assert_eq!(offset_of!(CBatchResult, count), 0);
        assert_eq!(offset_of!(CBatchResult, results), 8);
        assert_eq!(offset_of!(CBatchResult, success), 16);
    }

    /// Verify field offsets in CBytesWithMime match expectations
    #[test]
    fn test_c_bytes_with_mime_field_offsets() {
        use std::mem::offset_of;

        assert_eq!(offset_of!(CBytesWithMime, data), 0);
        assert_eq!(offset_of!(CBytesWithMime, data_len), 8);
        assert_eq!(offset_of!(CBytesWithMime, mime_type), 16);
    }

    /// Test that all structs can be safely created with zeroed memory
    #[test]
    fn test_structs_can_be_zeroed() {
        unsafe {
            // These should not panic if the types are properly repr(C)
            let _result: CExtractionResult = std::mem::zeroed();
            let _batch: CBatchResult = std::mem::zeroed();
            let _bytes: CBytesWithMime = std::mem::zeroed();
        }
    }
}
