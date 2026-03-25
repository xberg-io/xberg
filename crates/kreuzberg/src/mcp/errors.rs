//! MCP error mapping.
//!
//! This module provides functions to map Kreuzberg errors to MCP error responses.

use crate::KreuzbergError;
use rmcp::ErrorData as McpError;
use std::fmt::Write;

/// Map Kreuzberg errors to MCP error responses with appropriate error codes.
///
/// This function ensures different error types are properly differentiated in MCP responses:
/// - `Validation` errors → `INVALID_PARAMS` (-32602)
/// - `UnsupportedFormat` errors → `INVALID_PARAMS` (-32602)
/// - `Parsing` errors → `PARSE_ERROR` (-32700)
/// - `Io` errors → `INTERNAL_ERROR` (-32603) with context preserved
/// - All other errors → `INTERNAL_ERROR` (-32603)
///
/// The error message and source chain are preserved to aid debugging.
#[doc(hidden)]
pub fn map_kreuzberg_error_to_mcp(error: KreuzbergError) -> McpError {
    match error {
        KreuzbergError::Validation { message, source } => {
            let mut error_message = format!("Validation error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::invalid_params(error_message, None)
        }

        KreuzbergError::UnsupportedFormat(mime_type) => {
            McpError::invalid_params(format!("Unsupported format: {}", mime_type), None)
        }

        KreuzbergError::MissingDependency(dep) => McpError::invalid_params(
            format!(
                "Missing required dependency: {}. Please install it to use this feature.",
                dep
            ),
            None,
        ),

        KreuzbergError::Parsing { message, source } => {
            let mut error_message = format!("Parsing error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::parse_error(error_message, None)
        }

        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        KreuzbergError::Io(io_err) => McpError::internal_error(format!("System I/O error: {}", io_err), None),

        KreuzbergError::Ocr { message, source } => {
            let mut error_message = format!("OCR processing error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Cache { message, source } => {
            let mut error_message = format!("Cache error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::ImageProcessing { message, source } => {
            let mut error_message = format!("Image processing error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Serialization { message, source } => {
            let mut error_message = format!("Serialization error: {}", message);
            if let Some(src) = source {
                let _ = write!(error_message, " (caused by: {})", src);
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Plugin { message, plugin_name } => {
            McpError::internal_error(format!("Plugin '{}' error: {}", plugin_name, message), None)
        }

        KreuzbergError::LockPoisoned(msg) => McpError::internal_error(format!("Internal lock poisoned: {}", msg), None),

        KreuzbergError::Timeout { elapsed_ms, limit_ms } => McpError::internal_error(
            format!("Extraction timed out after {elapsed_ms}ms (limit: {limit_ms}ms)"),
            None,
        ),

        KreuzbergError::Other(msg) => McpError::internal_error(msg, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_validation_error_to_invalid_params() {
        let error = KreuzbergError::validation("invalid file path");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Validation error"));
        assert!(mcp_error.message.contains("invalid file path"));
    }

    #[test]
    fn test_map_validation_error_with_source_preserves_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad param");
        let error = KreuzbergError::validation_with_source("invalid configuration", source);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Validation error"));
        assert!(mcp_error.message.contains("invalid configuration"));
        assert!(mcp_error.message.contains("caused by"));
    }

    #[test]
    fn test_map_unsupported_format_to_invalid_params() {
        let error = KreuzbergError::UnsupportedFormat("application/unknown".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Unsupported format"));
        assert!(mcp_error.message.contains("application/unknown"));
    }

    #[test]
    fn test_map_missing_dependency_to_invalid_params() {
        let error = KreuzbergError::MissingDependency("tesseract".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Missing required dependency"));
        assert!(mcp_error.message.contains("tesseract"));
        assert!(mcp_error.message.contains("Please install"));
    }

    #[test]
    fn test_map_parsing_error_to_parse_error() {
        let error = KreuzbergError::parsing("corrupt PDF file");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32700);
        assert!(mcp_error.message.contains("Parsing error"));
        assert!(mcp_error.message.contains("corrupt PDF file"));
    }

    #[test]
    fn test_map_parsing_error_with_source_preserves_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "malformed data");
        let error = KreuzbergError::parsing_with_source("failed to parse document", source);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32700);
        assert!(mcp_error.message.contains("Parsing error"));
        assert!(mcp_error.message.contains("failed to parse document"));
        assert!(mcp_error.message.contains("caused by"));
    }

    #[test]
    fn test_map_io_error_to_internal_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = KreuzbergError::Io(io_error);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("System I/O error"));
        assert!(mcp_error.message.contains("file not found"));
    }

    #[test]
    fn test_map_ocr_error_to_internal_error() {
        let error = KreuzbergError::ocr("tesseract failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("OCR processing error"));
        assert!(mcp_error.message.contains("tesseract failed"));
    }

    #[test]
    fn test_map_cache_error_to_internal_error() {
        let error = KreuzbergError::cache("cache write failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Cache error"));
        assert!(mcp_error.message.contains("cache write failed"));
    }

    #[test]
    fn test_map_image_processing_error_to_internal_error() {
        let error = KreuzbergError::image_processing("resize failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Image processing error"));
        assert!(mcp_error.message.contains("resize failed"));
    }

    #[test]
    fn test_map_serialization_error_to_internal_error() {
        let error = KreuzbergError::serialization("JSON encode failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Serialization error"));
        assert!(mcp_error.message.contains("JSON encode failed"));
    }

    #[test]
    fn test_map_plugin_error_to_internal_error() {
        let error = KreuzbergError::Plugin {
            message: "extraction failed".to_string(),
            plugin_name: "pdf-extractor".to_string(),
        };
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Plugin 'pdf-extractor' error"));
        assert!(mcp_error.message.contains("extraction failed"));
    }

    #[test]
    fn test_map_lock_poisoned_error_to_internal_error() {
        let error = KreuzbergError::LockPoisoned("registry lock poisoned".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Internal lock poisoned"));
        assert!(mcp_error.message.contains("registry lock poisoned"));
    }

    #[test]
    fn test_map_other_error_to_internal_error() {
        let error = KreuzbergError::Other("unexpected error".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("unexpected error"));
    }

    #[test]
    fn test_error_type_differentiation() {
        let validation = KreuzbergError::validation("test");
        let parsing = KreuzbergError::parsing("test");
        let io = KreuzbergError::Io(std::io::Error::other("test"));

        let val_mcp = map_kreuzberg_error_to_mcp(validation);
        let parse_mcp = map_kreuzberg_error_to_mcp(parsing);
        let io_mcp = map_kreuzberg_error_to_mcp(io);

        assert_eq!(val_mcp.code.0, -32602);
        assert_eq!(parse_mcp.code.0, -32700);
        assert_eq!(io_mcp.code.0, -32603);

        assert_ne!(val_mcp.code.0, parse_mcp.code.0);
        assert_ne!(val_mcp.code.0, io_mcp.code.0);
        assert_ne!(parse_mcp.code.0, io_mcp.code.0);
    }

    #[test]
    fn test_error_mapping_preserves_error_context() {
        let validation_error = KreuzbergError::validation("invalid file path");
        let mcp_error = map_kreuzberg_error_to_mcp(validation_error);

        assert!(mcp_error.message.contains("invalid file path"));
    }

    #[test]
    fn test_io_errors_bubble_up_as_internal() {
        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let kreuzberg_error = KreuzbergError::Io(io_error);
        let mcp_error = map_kreuzberg_error_to_mcp(kreuzberg_error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("System I/O error"));
    }

    #[test]
    fn test_all_error_variants_have_mappings() {
        let errors = vec![
            KreuzbergError::validation("test"),
            KreuzbergError::UnsupportedFormat("test/unknown".to_string()),
            KreuzbergError::MissingDependency("test-dep".to_string()),
            KreuzbergError::parsing("test"),
            KreuzbergError::Io(std::io::Error::other("test")),
            KreuzbergError::ocr("test"),
            KreuzbergError::cache("test"),
            KreuzbergError::image_processing("test"),
            KreuzbergError::serialization("test"),
            KreuzbergError::Plugin {
                message: "test".to_string(),
                plugin_name: "test-plugin".to_string(),
            },
            KreuzbergError::LockPoisoned("test".to_string()),
            KreuzbergError::Other("test".to_string()),
        ];

        for error in errors {
            let mcp_error = map_kreuzberg_error_to_mcp(error);

            assert!(mcp_error.code.0 < 0, "Error code should be negative");

            assert!(!mcp_error.message.is_empty());
        }
    }
}
