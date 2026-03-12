//! Helper utilities for FFI operations.
//!
//! This module contains shared helper functions for error handling, string conversion,
//! and type conversion between Rust and C types.

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::types::ExtractionResult;

use crate::panic_shield::{ErrorCode, StructuredError, clear_structured_error, set_structured_error};
use crate::types::{CExtractionResult, CStringGuard};

/// Type alias for FFI results that can fail with a String error message
pub type FfiResult<T> = std::result::Result<T, String>;

// Thread-local storage for the last error message (for backward compatibility)
thread_local! {
    pub(crate) static LAST_ERROR_C_STRING: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Set the last error message (convenience wrapper for backward compatibility)
pub fn set_last_error(err: String) {
    if let Ok(c_str) = CString::new(err.clone()) {
        LAST_ERROR_C_STRING.with(|last| *last.borrow_mut() = Some(c_str));
    }

    let structured_err = StructuredError::from_message(err, ErrorCode::GenericError);
    set_structured_error(structured_err);
}

/// Clear the last error message
pub fn clear_last_error() {
    LAST_ERROR_C_STRING.with(|last| *last.borrow_mut() = None);
    clear_structured_error();
}

/// Convert a Rust String to a C string pointer
pub fn string_to_c_string(value: String) -> std::result::Result<*mut c_char, String> {
    CString::new(value)
        .map(CString::into_raw)
        .map_err(|e| format!("Failed to create C string: {}", e))
}

/// Parse extraction configuration from JSON string
pub fn parse_extraction_config_from_json(config_str: &str) -> FfiResult<ExtractionConfig> {
    // html-to-markdown-rs v2.22.5+ has #[serde(default)] on ConversionOptions,
    // so serde can now handle partial deserialization with defaults for missing fields
    let config: ExtractionConfig =
        serde_json::from_str(config_str).map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    Ok(config)
}

/// Convert a Rust ExtractionResult to a C-compatible CExtractionResult
pub fn to_c_extraction_result(result: ExtractionResult) -> std::result::Result<*mut CExtractionResult, String> {
    let ExtractionResult {
        content,
        mime_type,
        metadata,
        tables,
        detected_languages,
        chunks,
        images,
        pages,
        elements,
        djot_content,
        ocr_elements,
        document,
        extracted_keywords,
        quality_score,
        processing_warnings,
        annotations,
    } = result;

    let sanitized_content = if content.contains('\0') {
        content.replace('\0', "\u{FFFD}")
    } else {
        content
    };

    let content_guard = CStringGuard::new(
        CString::new(sanitized_content).map_err(|e| format!("Failed to convert content to C string: {}", e))?,
    );

    let mime_type_guard = CStringGuard::new(
        CString::new(mime_type.to_string()).map_err(|e| format!("Failed to convert MIME type to C string: {}", e))?,
    );

    let language_guard = match &metadata.language {
        Some(lang) => Some(CStringGuard::new(
            CString::new(lang.as_str()).map_err(|e| format!("Failed to convert language to C string: {}", e))?,
        )),
        None => None,
    };

    let date_guard = match &metadata.created_at {
        Some(d) => {
            Some(CStringGuard::new(CString::new(d.as_str()).map_err(|e| {
                format!("Failed to convert created_at to C string: {}", e)
            })?))
        }
        None => None,
    };

    let subject_guard = match &metadata.subject {
        Some(subj) => Some(CStringGuard::new(
            CString::new(subj.as_str()).map_err(|e| format!("Failed to convert subject to C string: {}", e))?,
        )),
        None => None,
    };

    let tables_json_guard = if !tables.is_empty() {
        let json = serde_json::to_string(&tables).map_err(|e| format!("Failed to serialize tables to JSON: {}", e))?;
        Some(CStringGuard::new(CString::new(json).map_err(|e| {
            format!("Failed to convert tables JSON to C string: {}", e)
        })?))
    } else {
        None
    };

    let detected_languages_json_guard = match detected_languages {
        Some(langs) if !langs.is_empty() => {
            let json = serde_json::to_string(&langs)
                .map_err(|e| format!("Failed to serialize detected languages to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert detected languages JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let metadata_json_guard = {
        let json =
            serde_json::to_string(&metadata).map_err(|e| format!("Failed to serialize metadata to JSON: {}", e))?;

        Some(CStringGuard::new(CString::new(json).map_err(|e| {
            format!("Failed to convert metadata JSON to C string: {}", e)
        })?))
    };

    let chunks_json_guard = match chunks {
        Some(chunks) if !chunks.is_empty() => {
            let json =
                serde_json::to_string(&chunks).map_err(|e| format!("Failed to serialize chunks to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert chunks JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let images_json_guard = match images {
        Some(images) if !images.is_empty() => {
            let json =
                serde_json::to_string(&images).map_err(|e| format!("Failed to serialize images to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert images JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let page_structure_json_guard = match &metadata.pages {
        Some(page_structure) => {
            let json = serde_json::to_string(&page_structure)
                .map_err(|e| format!("Failed to serialize page structure to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert page structure JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let pages_json_guard = match pages {
        Some(pages) if !pages.is_empty() => {
            let json =
                serde_json::to_string(&pages).map_err(|e| format!("Failed to serialize pages to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert pages JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let elements_json_guard = match elements {
        Some(elements) if !elements.is_empty() => {
            let json =
                serde_json::to_string(&elements).map_err(|e| format!("Failed to serialize elements to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert elements JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let ocr_elements_json_guard = match ocr_elements {
        Some(els) if !els.is_empty() => {
            let json =
                serde_json::to_string(&els).map_err(|e| format!("Failed to serialize OCR elements to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert OCR elements JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let document_json_guard = match document {
        Some(doc) => {
            let json = serde_json::to_string(&doc)
                .map_err(|e| format!("Failed to serialize document structure to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert document structure JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let extracted_keywords_json_guard = match extracted_keywords {
        Some(keywords) if !keywords.is_empty() => {
            let json = serde_json::to_string(&keywords)
                .map_err(|e| format!("Failed to serialize extracted keywords to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert extracted keywords JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let quality_score_json_guard = match quality_score {
        Some(score) => {
            let json = serde_json::to_string(&score)
                .map_err(|e| format!("Failed to serialize quality score to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert quality score JSON to C string: {}", e)
            })?))
        }
        None => None,
    };

    let processing_warnings_json_guard = if !processing_warnings.is_empty() {
        let json = serde_json::to_string(&processing_warnings)
            .map_err(|e| format!("Failed to serialize processing warnings to JSON: {}", e))?;
        Some(CStringGuard::new(CString::new(json).map_err(|e| {
            format!("Failed to convert processing warnings JSON to C string: {}", e)
        })?))
    } else {
        None
    };

    let djot_content_json_guard = match djot_content {
        Some(djot) => {
            let json =
                serde_json::to_string(&djot).map_err(|e| format!("Failed to serialize djot content to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert djot content JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let annotations_json_guard = match annotations {
        Some(anns) if !anns.is_empty() => {
            let json =
                serde_json::to_string(&anns).map_err(|e| format!("Failed to serialize annotations to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert annotations JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    Ok(Box::into_raw(Box::new(CExtractionResult {
        annotations_json: annotations_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        chunks_json: chunks_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        content: content_guard.into_raw(),
        date: date_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        detected_languages_json: detected_languages_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        djot_content_json: djot_content_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        document_json: document_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        elements_json: elements_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        extracted_keywords_json: extracted_keywords_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        images_json: images_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        language: language_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        metadata_json: metadata_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        mime_type: mime_type_guard.into_raw(),
        ocr_elements_json: ocr_elements_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        page_structure_json: page_structure_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        pages_json: pages_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        processing_warnings_json: processing_warnings_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        quality_score_json: quality_score_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        subject: subject_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        tables_json: tables_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        success: true,
        _padding1: [0u8; 7],
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::types::{Chunk, ChunkMetadata, ExtractionResult, Metadata, Table};
    use std::borrow::Cow;
    use std::ffi::CStr;

    #[test]
    fn test_set_and_clear_error() {
        // Test that error functions don't panic
        set_last_error("test error".to_string());
        clear_last_error();
    }

    #[test]
    fn test_string_to_c_string_success() {
        let result = string_to_c_string("hello world".to_string());
        assert!(result.is_ok());

        // Clean up the allocated string
        if let Ok(ptr) = result {
            unsafe {
                let _ = CString::from_raw(ptr);
            }
        }
    }

    #[test]
    fn test_string_to_c_string_with_null_byte() {
        let result = string_to_c_string("hello\0world".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to create C string"));
    }

    #[test]
    fn test_parse_extraction_config_empty_json() {
        let result = parse_extraction_config_from_json("{}");
        assert!(result.is_ok());
        let config = result.unwrap();
        // Default config should be created
        assert!(config.html_options.is_none());
    }

    #[test]
    fn test_parse_extraction_config_with_basic_options() {
        let json = r#"{
            "use_cache": true,
            "enable_quality_processing": false
        }"#;

        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.use_cache);
        assert!(!config.enable_quality_processing);
    }

    #[test]
    fn test_parse_extraction_config_with_html_options() {
        let json = r#"{
            "html_options": {
                "headingStyle": "atx",
                "escapeAsterisks": true,
                "autolinks": false
            }
        }"#;

        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.html_options.is_some());

        let html_opts = config.html_options.unwrap();
        assert!(html_opts.escape_asterisks);
        assert!(!html_opts.autolinks);
    }

    #[test]
    fn test_parse_extraction_config_invalid_json() {
        let result = parse_extraction_config_from_json("not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse config JSON"));
    }

    #[test]
    fn test_parse_extraction_config_invalid_heading_style_uses_default() {
        // With #[serde(default)], invalid enum values use the default instead of failing
        let json = r#"{
            "html_options": {
                "headingStyle": "invalid_style"
            }
        }"#;

        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok(), "Parsing should succeed with default values");

        // Invalid enum values should be ignored and default value used
        let config = result.unwrap();
        assert!(config.html_options.is_some());
    }

    #[test]
    fn test_parse_extraction_config_all_heading_styles() {
        let styles = vec![
            ("atx", "Atx"),
            ("underlined", "Underlined"),
            ("atx_closed", "AtxClosed"),
        ];

        for (input, _expected) in styles {
            let json = format!(r#"{{"html_options": {{"headingStyle": "{}"}}}}"#, input);
            let result = parse_extraction_config_from_json(&json);
            assert!(result.is_ok(), "Failed to parse heading_style: {}", input);
        }
    }

    #[test]
    fn test_parse_extraction_config_preprocessing() {
        let json = r#"{
            "html_options": {
                "preprocessing": {
                    "enabled": true,
                    "preset": "aggressive",
                    "removeNavigation": true,
                    "removeForms": false
                }
            }
        }"#;

        let result = parse_extraction_config_from_json(json);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.html_options.is_some());

        let preprocessing = &config.html_options.unwrap().preprocessing;
        assert!(preprocessing.enabled);
        assert!(preprocessing.remove_navigation);
        assert!(!preprocessing.remove_forms);
    }

    #[test]
    fn test_to_c_extraction_result_basic() {
        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
        };

        let c_result = to_c_extraction_result(result);
        assert!(c_result.is_ok());

        // Clean up the allocated result
        if let Ok(ptr) = c_result {
            unsafe {
                let boxed = Box::from_raw(ptr);
                // Verify success flag
                assert!(boxed.success);

                // Clean up strings
                if !boxed.content.is_null() {
                    let _ = CString::from_raw(boxed.content);
                }
                if !boxed.mime_type.is_null() {
                    let _ = CString::from_raw(boxed.mime_type);
                }
                if !boxed.metadata_json.is_null() {
                    let _ = CString::from_raw(boxed.metadata_json);
                }
            }
        }
    }

    #[test]
    fn test_to_c_extraction_result_with_null_bytes() {
        let result = ExtractionResult {
            content: "Test\0content with null".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
        };

        let c_result = to_c_extraction_result(result);
        assert!(c_result.is_ok());

        // Clean up
        if let Ok(ptr) = c_result {
            unsafe {
                let boxed = Box::from_raw(ptr);

                // Verify null bytes were replaced with replacement character
                let content_str = CStr::from_ptr(boxed.content).to_str().unwrap();
                assert!(!content_str.contains('\0'));
                assert!(content_str.contains('\u{FFFD}'));

                // Clean up strings
                if !boxed.content.is_null() {
                    let _ = CString::from_raw(boxed.content);
                }
                if !boxed.mime_type.is_null() {
                    let _ = CString::from_raw(boxed.mime_type);
                }
                if !boxed.metadata_json.is_null() {
                    let _ = CString::from_raw(boxed.metadata_json);
                }
            }
        }
    }

    #[test]
    fn test_to_c_extraction_result_with_metadata() {
        let metadata = Metadata {
            language: Some("en".to_string()),
            created_at: Some("2024-01-01".to_string()),
            subject: Some("Test Subject".to_string()),
            ..Default::default()
        };

        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            tables: vec![],
            detected_languages: Some(vec!["en".to_string(), "de".to_string()]),
            chunks: None,
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
        };

        let c_result = to_c_extraction_result(result);
        assert!(c_result.is_ok());

        // Clean up
        if let Ok(ptr) = c_result {
            unsafe {
                let boxed = Box::from_raw(ptr);

                // Verify metadata fields are not null
                assert!(!boxed.language.is_null());
                assert!(!boxed.date.is_null());
                assert!(!boxed.subject.is_null());
                assert!(!boxed.detected_languages_json.is_null());

                // Clean up all allocated strings
                if !boxed.content.is_null() {
                    let _ = CString::from_raw(boxed.content);
                }
                if !boxed.mime_type.is_null() {
                    let _ = CString::from_raw(boxed.mime_type);
                }
                if !boxed.language.is_null() {
                    let _ = CString::from_raw(boxed.language);
                }
                if !boxed.date.is_null() {
                    let _ = CString::from_raw(boxed.date);
                }
                if !boxed.subject.is_null() {
                    let _ = CString::from_raw(boxed.subject);
                }
                if !boxed.metadata_json.is_null() {
                    let _ = CString::from_raw(boxed.metadata_json);
                }
                if !boxed.detected_languages_json.is_null() {
                    let _ = CString::from_raw(boxed.detected_languages_json);
                }
                if !boxed.djot_content_json.is_null() {
                    let _ = CString::from_raw(boxed.djot_content_json);
                }
            }
        }
    }

    #[test]
    fn test_to_c_extraction_result_with_tables_and_chunks() {
        let table = Table {
            cells: vec![
                vec!["Col1".to_string(), "Col2".to_string()],
                vec!["A1".to_string(), "A2".to_string()],
                vec!["B1".to_string(), "B2".to_string()],
            ],
            markdown: "| Col1 | Col2 |\n|------|------|\n| A1 | A2 |\n| B1 | B2 |".to_string(),
            page_number: 1,
            bounding_box: None,
        };

        let chunk = Chunk {
            content: "Chunk content".to_string(),
            embedding: None,
            metadata: ChunkMetadata {
                byte_start: 0,
                byte_end: 13,
                token_count: None,
                chunk_index: 0,
                total_chunks: 1,
                first_page: Some(1),
                last_page: Some(1),
                heading_context: None,
            },
        };

        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: Metadata::default(),
            tables: vec![table],
            detected_languages: None,
            chunks: Some(vec![chunk]),
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: vec![],
            annotations: None,
        };

        let c_result = to_c_extraction_result(result);
        assert!(c_result.is_ok());

        // Clean up
        if let Ok(ptr) = c_result {
            unsafe {
                let boxed = Box::from_raw(ptr);

                // Verify JSON fields are not null
                assert!(!boxed.tables_json.is_null());
                assert!(!boxed.chunks_json.is_null());

                // Clean up all allocated strings
                if !boxed.content.is_null() {
                    let _ = CString::from_raw(boxed.content);
                }
                if !boxed.mime_type.is_null() {
                    let _ = CString::from_raw(boxed.mime_type);
                }
                if !boxed.metadata_json.is_null() {
                    let _ = CString::from_raw(boxed.metadata_json);
                }
                if !boxed.tables_json.is_null() {
                    let _ = CString::from_raw(boxed.tables_json);
                }
                if !boxed.chunks_json.is_null() {
                    let _ = CString::from_raw(boxed.chunks_json);
                }
            }
        }
    }
}
