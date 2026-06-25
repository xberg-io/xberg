//! Framework adapter implementations

pub mod external;
pub mod subprocess;
pub mod xberg;

pub use external::{
    create_docling_adapter, create_liteparse_adapter, create_markitdown_adapter, create_mineru_adapter,
    create_pymupdf4llm_adapter, create_tika_adapter, create_unstructured_adapter,
};
pub use subprocess::SubprocessAdapter;
pub use xberg::create_xberg_adapter;

/// Returns the OCR flag string based on the provided boolean
pub(crate) fn ocr_flag(ocr_enabled: bool) -> String {
    if ocr_enabled {
        "--ocr".to_string()
    } else {
        "--no-ocr".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_flag_when_enabled() {
        let result = ocr_flag(true);
        assert_eq!(result, "--ocr", "Should return '--ocr' when enabled");
    }

    #[test]
    fn test_ocr_flag_when_disabled() {
        let result = ocr_flag(false);
        assert_eq!(result, "--no-ocr", "Should return '--no-ocr' when disabled");
    }
}
