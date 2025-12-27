//! Result accessor FFI module.
//!
//! Provides C-compatible functions to access fields from ExtractionResult without
//! requiring bindings to parse JSON. This eliminates JSON round-trips and allows
//! efficient field access in language bindings.
//!
//! All string-returning functions return pointers to C strings that MUST be freed
//! with `kreuzberg_free_string()`.

use crate::{clear_last_error, set_last_error};
use kreuzberg::types::ExtractionResult;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Get page count from extraction result.
///
/// Returns the total number of pages/slides/sheets detected in the document.
///
/// # Arguments
///
/// * `result` - Pointer to an ExtractionResult structure
///
/// # Returns
///
/// The page count (>= 0) if successful, or -1 on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `result` must be a valid pointer to an ExtractionResult
/// - `result` cannot be NULL
///
/// # Example (C)
///
/// ```c
/// ExtractionResult* result = kreuzberg_extract_file("document.pdf", NULL);
/// if (result != NULL) {
///     int page_count = kreuzberg_result_get_page_count(result);
///     if (page_count >= 0) {
///         printf("Document has %d pages\n", page_count);
///     }
///     kreuzberg_result_free(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_result_get_page_count(result: *const ExtractionResult) -> i32 {
    if result.is_null() {
        set_last_error("Result cannot be NULL".to_string());
        return -1;
    }

    clear_last_error();

    let result_ref = unsafe { &*result };

    if let Some(metadata) = &result_ref.metadata.pages {
        metadata.total_count as i32
    } else {
        0
    }
}

/// Get chunk count from extraction result.
///
/// Returns the number of text chunks when chunking is enabled, or 0 if chunking
/// was not performed.
///
/// # Arguments
///
/// * `result` - Pointer to an ExtractionResult structure
///
/// # Returns
///
/// The chunk count (>= 0) if successful, or -1 on error (check `kreuzberg_last_error`).
///
/// # Safety
///
/// - `result` must be a valid pointer to an ExtractionResult
/// - `result` cannot be NULL
///
/// # Example (C)
///
/// ```c
/// ExtractionResult* result = kreuzberg_extract_file("document.pdf", config);
/// if (result != NULL) {
///     int chunk_count = kreuzberg_result_get_chunk_count(result);
///     if (chunk_count >= 0) {
///         printf("Document has %d chunks\n", chunk_count);
///     }
///     kreuzberg_result_free(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_result_get_chunk_count(result: *const ExtractionResult) -> i32 {
    if result.is_null() {
        set_last_error("Result cannot be NULL".to_string());
        return -1;
    }

    clear_last_error();

    let result_ref = unsafe { &*result };

    if let Some(chunks) = &result_ref.chunks {
        chunks.len() as i32
    } else {
        0
    }
}

/// Get detected language from extraction result.
///
/// Returns the primary detected language as an ISO 639 language code.
/// If multiple languages were detected, returns the primary one.
///
/// # Arguments
///
/// * `result` - Pointer to an ExtractionResult structure
///
/// # Returns
///
/// A pointer to a C string containing the language code (e.g., "en", "de"),
/// or NULL if no language was detected or on error (check `kreuzberg_last_error`).
///
/// The returned pointer must be freed with `kreuzberg_free_string()`.
///
/// # Safety
///
/// - `result` must be a valid pointer to an ExtractionResult
/// - `result` cannot be NULL
/// - The returned pointer (if non-NULL) must be freed with `kreuzberg_free_string`
///
/// # Example (C)
///
/// ```c
/// ExtractionResult* result = kreuzberg_extract_file("document.pdf", NULL);
/// if (result != NULL) {
///     char* language = kreuzberg_result_get_detected_language(result);
///     if (language != NULL) {
///         printf("Detected language: %s\n", language);
///         kreuzberg_free_string(language);
///     }
///     kreuzberg_result_free(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_result_get_detected_language(result: *const ExtractionResult) -> *mut c_char {
    if result.is_null() {
        set_last_error("Result cannot be NULL".to_string());
        return ptr::null_mut();
    }

    clear_last_error();

    let result_ref = unsafe { &*result };

    let language = if let Some(lang) = &result_ref.metadata.language {
        lang.clone()
    } else if let Some(langs) = &result_ref.detected_languages {
        if !langs.is_empty() {
            langs[0].clone()
        } else {
            set_last_error("No language detected".to_string());
            return ptr::null_mut();
        }
    } else {
        set_last_error("No language detected".to_string());
        return ptr::null_mut();
    };

    match CString::new(language) {
        Ok(c_string) => c_string.into_raw(),
        Err(e) => {
            set_last_error(format!("Failed to convert language to C string: {}", e));
            ptr::null_mut()
        }
    }
}

/// Metadata field accessor structure
///
/// Returned by `kreuzberg_result_get_metadata_field()`. Contains the field value
/// as JSON and information about whether the field exists.
///
/// # Fields
///
/// * `name` - The field name requested (does not need to be freed)
/// * `json_value` - JSON representation of the field value, or NULL if field doesn't exist
/// * `is_null` - 1 if the field doesn't exist, 0 if it does
///
/// The `json_value` pointer (if non-NULL) must be freed with `kreuzberg_free_string()`.
#[repr(C)]
pub struct CMetadataField {
    pub name: *const c_char,
    pub json_value: *mut c_char,
    pub is_null: i32,
}

/// Get a metadata field by name.
///
/// Retrieves a metadata field from the extraction result and returns its value
/// as a JSON string. Supports nested fields with dot notation (e.g., "format.pages").
///
/// # Arguments
///
/// * `result` - Pointer to an ExtractionResult structure
/// * `field_name` - Null-terminated C string with the field name
///
/// # Returns
///
/// A CMetadataField structure containing:
/// - `name`: The field name (caller should not free)
/// - `json_value`: Pointer to field value as JSON string (must free with `kreuzberg_free_string`),
///   or NULL if field doesn't exist
/// - `is_null`: 1 if field doesn't exist, 0 if it does
///
/// # Safety
///
/// - `result` must be a valid pointer to an ExtractionResult
/// - `field_name` must be a valid null-terminated C string
/// - Neither parameter can be NULL
/// - The returned `json_value` (if non-NULL) must be freed with `kreuzberg_free_string`
///
/// # Example (C)
///
/// ```c
/// ExtractionResult* result = kreuzberg_extract_file("document.pdf", NULL);
/// if (result != NULL) {
///     CMetadataField title_field = kreuzberg_result_get_metadata_field(result, "title");
///     if (!title_field.is_null) {
///         printf("Title: %s\n", title_field.json_value);
///         kreuzberg_free_string(title_field.json_value);
///     }
///
///     CMetadataField author_field = kreuzberg_result_get_metadata_field(result, "authors");
///     if (!author_field.is_null) {
///         printf("Authors: %s\n", author_field.json_value);
///         kreuzberg_free_string(author_field.json_value);
///     }
///
///     kreuzberg_result_free(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_result_get_metadata_field(
    result: *const ExtractionResult,
    field_name: *const c_char,
) -> CMetadataField {
    if result.is_null() {
        set_last_error("Result cannot be NULL".to_string());
        return CMetadataField {
            name: field_name,
            json_value: ptr::null_mut(),
            is_null: 1,
        };
    }

    if field_name.is_null() {
        set_last_error("Field name cannot be NULL".to_string());
        return CMetadataField {
            name: ptr::null(),
            json_value: ptr::null_mut(),
            is_null: 1,
        };
    }

    clear_last_error();

    let field_str = match unsafe { std::ffi::CStr::from_ptr(field_name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in field name: {}", e));
            return CMetadataField {
                name: field_name,
                json_value: ptr::null_mut(),
                is_null: 1,
            };
        }
    };

    let result_ref = unsafe { &*result };

    let metadata_json = match serde_json::to_value(&result_ref.metadata) {
        Ok(val) => val,
        Err(e) => {
            set_last_error(format!("Failed to serialize metadata: {}", e));
            return CMetadataField {
                name: field_name,
                json_value: ptr::null_mut(),
                is_null: 1,
            };
        }
    };

    let mut current = &metadata_json;
    for part in field_str.split('.') {
        if let Some(obj) = current.as_object() {
            match obj.get(part) {
                Some(val) => current = val,
                None => {
                    return CMetadataField {
                        name: field_name,
                        json_value: ptr::null_mut(),
                        is_null: 1,
                    };
                }
            }
        } else {
            return CMetadataField {
                name: field_name,
                json_value: ptr::null_mut(),
                is_null: 1,
            };
        }
    }

    match serde_json::to_string(current) {
        Ok(json) => match CString::new(json) {
            Ok(c_string) => CMetadataField {
                name: field_name,
                json_value: c_string.into_raw(),
                is_null: 0,
            },
            Err(e) => {
                set_last_error(format!("Failed to convert field value to C string: {}", e));
                CMetadataField {
                    name: field_name,
                    json_value: ptr::null_mut(),
                    is_null: 1,
                }
            }
        },
        Err(e) => {
            set_last_error(format!("Failed to serialize field value: {}", e));
            CMetadataField {
                name: field_name,
                json_value: ptr::null_mut(),
                is_null: 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    fn create_test_result() -> ExtractionResult {
        use kreuzberg::types::{Metadata, PageStructure, PageUnitType};

        let mut metadata = Metadata::default();
        metadata.title = Some("Test Document".to_string());
        metadata.language = Some("en".to_string());

        let page_structure = PageStructure {
            total_count: 10,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: None,
        };
        metadata.pages = Some(page_structure);

        ExtractionResult {
            content: "Sample content for testing".to_string(),
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
    fn test_result_get_page_count() {
        let result = create_test_result();
        let result_ptr = Box::into_raw(Box::new(result));

        let page_count = unsafe { kreuzberg_result_get_page_count(result_ptr) };
        assert_eq!(page_count, 10);

        unsafe {
            let _ = Box::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_result_get_page_count_null() {
        let page_count = unsafe { kreuzberg_result_get_page_count(ptr::null()) };
        assert_eq!(page_count, -1);
    }

    #[test]
    fn test_result_get_chunk_count() {
        let result = create_test_result();
        let result_ptr = Box::into_raw(Box::new(result));

        let chunk_count = unsafe { kreuzberg_result_get_chunk_count(result_ptr) };
        assert_eq!(chunk_count, 2);

        unsafe {
            let _ = Box::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_result_get_chunk_count_null() {
        let chunk_count = unsafe { kreuzberg_result_get_chunk_count(ptr::null()) };
        assert_eq!(chunk_count, -1);
    }

    #[test]
    fn test_result_get_detected_language() {
        let result = create_test_result();
        let result_ptr = Box::into_raw(Box::new(result));

        let lang_ptr = unsafe { kreuzberg_result_get_detected_language(result_ptr) };
        assert!(!lang_ptr.is_null());

        let lang_str = unsafe { CStr::from_ptr(lang_ptr).to_str().unwrap() };
        assert_eq!(lang_str, "en");

        unsafe {
            crate::kreuzberg_free_string(lang_ptr);
            let _ = Box::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_result_get_detected_language_null() {
        let lang_ptr = unsafe { kreuzberg_result_get_detected_language(ptr::null()) };
        assert!(lang_ptr.is_null());
    }

    #[test]
    fn test_result_get_metadata_field_title() {
        let result = create_test_result();
        let result_ptr = Box::into_raw(Box::new(result));

        let field_name = std::ffi::CString::new("title").unwrap();
        let field = unsafe { kreuzberg_result_get_metadata_field(result_ptr, field_name.as_ptr()) };

        assert_eq!(field.is_null, 0);
        assert!(!field.json_value.is_null());

        let value_str = unsafe { CStr::from_ptr(field.json_value).to_str().unwrap() };
        assert_eq!(value_str, r#""Test Document""#);

        unsafe {
            crate::kreuzberg_free_string(field.json_value);
            let _ = Box::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_result_get_metadata_field_missing() {
        let result = create_test_result();
        let result_ptr = Box::into_raw(Box::new(result));

        let field_name = std::ffi::CString::new("nonexistent").unwrap();
        let field = unsafe { kreuzberg_result_get_metadata_field(result_ptr, field_name.as_ptr()) };

        assert_eq!(field.is_null, 1);
        assert!(field.json_value.is_null());

        unsafe {
            let _ = Box::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_result_get_metadata_field_null_result() {
        let field_name = std::ffi::CString::new("title").unwrap();
        let field = unsafe { kreuzberg_result_get_metadata_field(ptr::null(), field_name.as_ptr()) };

        assert_eq!(field.is_null, 1);
        assert!(field.json_value.is_null());
    }
}
