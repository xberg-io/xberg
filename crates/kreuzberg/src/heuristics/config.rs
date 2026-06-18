//! Configuration for document-processing heuristics.
//!
//! All thresholds are public fields with [`Default`] impls.  Tuned values for
//! specific deployment environments belong in downstream consumers, not here.
//! No environment-variable reads occur in this crate; callers that need
//! `HEURISTICS_*` env-var support should implement `from_env()` on their own
//! config wrapper and populate the appropriate fields.

use crate::heuristics::error::{HeuristicsError, Result};
use serde::{Deserialize, Serialize};

/// Configuration for document chunking and analysis heuristics.
///
/// Every threshold is a public field so callers can override any subset via
/// struct-update syntax: `HeuristicsConfig { text_layer_threshold: 0.5, ..Default::default() }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicsConfig {
    /// Enable PDF text-layer detection heuristics.
    ///
    /// When `true`, PDFs with a substantial text layer will skip chunking.
    /// Default: `true`.
    pub enable_pdf_text_heuristics: bool,

    /// Minimum fraction of pages that must have text to skip chunking.
    ///
    /// Range `0.0..=1.0`. Default: `0.7` (70 % of pages).
    pub text_layer_threshold: f32,

    /// File size threshold in bytes for considering chunking.
    ///
    /// Files smaller than this are processed without chunking.
    /// Default: 10 MiB (10 × 1 024 × 1 024).
    pub file_size_threshold_bytes: u64,

    /// Page count threshold for considering chunking.
    ///
    /// Documents with fewer pages are processed without chunking.
    /// Default: 50.
    pub page_count_threshold: u32,

    /// Target number of pages per chunk for optimal parallel processing.
    ///
    /// Default: 10.
    pub target_pages_per_chunk: u32,

    /// Hard cap on pages per chunk.
    ///
    /// No chunk will exceed this limit. Must be ≥ `target_pages_per_chunk`.
    /// Default: 25.
    pub max_pages_per_chunk: u32,

    /// File size threshold for disk-based processing.
    ///
    /// Files larger than this are buffered to disk to prevent OOM.
    /// Default: 50 MiB (50 × 1 024 × 1 024).
    pub disk_processing_threshold_bytes: u64,

    /// Minimum characters per page to consider a page as having text.
    ///
    /// Default: 50.
    pub min_chars_per_page: u32,

    /// Maximum sheet count allowed in an XLSX workbook.
    ///
    /// Workbooks beyond this are rejected pre-extraction to avoid OOM /
    /// abusive billing inflation. Default: 200.
    pub max_xlsx_sheet_count: u32,

    /// Maximum cell count (sheets × rows × columns approximation) in an XLSX workbook.
    ///
    /// Default: 5 000 000 (≈ 200 sheets × 25 k cells).
    pub max_xlsx_workbook_cells: u64,

    /// Maximum number of OLE-embedded objects extractable from a single PPTX or DOCX.
    ///
    /// Protects against zip-bomb-style nested-document abuse. Default: 50.
    pub max_pptx_embedded_count: u32,
}

impl Default for HeuristicsConfig {
    fn default() -> Self {
        Self {
            enable_pdf_text_heuristics: true,
            text_layer_threshold: 0.7,
            file_size_threshold_bytes: 10 * 1024 * 1024, // 10 MiB
            page_count_threshold: 50,
            target_pages_per_chunk: 10,
            max_pages_per_chunk: 25,
            disk_processing_threshold_bytes: 50 * 1024 * 1024, // 50 MiB
            min_chars_per_page: 50,
            max_xlsx_sheet_count: 200,
            max_xlsx_workbook_cells: 5_000_000,
            max_pptx_embedded_count: 50,
        }
    }
}

impl HeuristicsConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`HeuristicsError::ConfigError`] when:
    /// - `target_pages_per_chunk` is 0
    /// - `max_pages_per_chunk` < `target_pages_per_chunk`
    /// - `file_size_threshold_bytes` is 0
    pub fn validate(&self) -> Result<()> {
        if self.target_pages_per_chunk == 0 {
            return Err(HeuristicsError::ConfigError(
                "target_pages_per_chunk must be greater than 0".to_string(),
            ));
        }

        if self.max_pages_per_chunk < self.target_pages_per_chunk {
            return Err(HeuristicsError::ConfigError(
                "max_pages_per_chunk must be >= target_pages_per_chunk".to_string(),
            ));
        }

        if self.file_size_threshold_bytes == 0 {
            return Err(HeuristicsError::ConfigError(
                "file_size_threshold_bytes must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Create a configuration suitable for unit tests (smaller thresholds).
    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            enable_pdf_text_heuristics: true,
            text_layer_threshold: 0.5,
            file_size_threshold_bytes: 1024, // 1 KiB
            page_count_threshold: 5,
            target_pages_per_chunk: 2,
            max_pages_per_chunk: 5,
            disk_processing_threshold_bytes: 10 * 1024, // 10 KiB
            min_chars_per_page: 10,
            max_xlsx_sheet_count: 10,
            max_xlsx_workbook_cells: 50_000,
            max_pptx_embedded_count: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HeuristicsConfig::default();
        assert!(config.enable_pdf_text_heuristics);
        assert!((config.text_layer_threshold - 0.7).abs() < f32::EPSILON);
        assert_eq!(config.file_size_threshold_bytes, 10 * 1024 * 1024);
        assert_eq!(config.page_count_threshold, 50);
        assert_eq!(config.target_pages_per_chunk, 10);
        assert_eq!(config.max_pages_per_chunk, 25);
        assert_eq!(config.disk_processing_threshold_bytes, 50 * 1024 * 1024);
        assert_eq!(config.max_xlsx_sheet_count, 200);
        assert_eq!(config.max_xlsx_workbook_cells, 5_000_000);
        assert_eq!(config.max_pptx_embedded_count, 50);
    }

    #[test]
    fn test_config_validation_passes() {
        let config = HeuristicsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_fails_zero_target() {
        let config = HeuristicsConfig {
            target_pages_per_chunk: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_fails_max_less_than_target() {
        let config = HeuristicsConfig {
            target_pages_per_chunk: 20,
            max_pages_per_chunk: 10,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_fails_zero_file_size_threshold() {
        let config = HeuristicsConfig {
            file_size_threshold_bytes: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(HeuristicsError::ConfigError(msg)) = result {
            assert!(msg.contains("file_size_threshold_bytes"));
            assert!(msg.contains("greater than 0"));
        } else {
            panic!("Expected ConfigError");
        }
    }

    #[test]
    fn test_new_returns_default_values() {
        let config = HeuristicsConfig::new();
        let default_config = HeuristicsConfig::default();
        assert_eq!(
            config.enable_pdf_text_heuristics,
            default_config.enable_pdf_text_heuristics
        );
        assert!((config.text_layer_threshold - default_config.text_layer_threshold).abs() < f32::EPSILON);
        assert_eq!(
            config.file_size_threshold_bytes,
            default_config.file_size_threshold_bytes
        );
        assert_eq!(config.page_count_threshold, default_config.page_count_threshold);
    }

    #[test]
    fn test_default_trait_implementation() {
        let config: HeuristicsConfig = Default::default();
        assert!(config.validate().is_ok());
        assert!(config.enable_pdf_text_heuristics);
        assert!((config.text_layer_threshold - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_test_config_has_all_fields_set() {
        let config = HeuristicsConfig::test_config();
        assert!(config.enable_pdf_text_heuristics);
        assert!((config.text_layer_threshold - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.file_size_threshold_bytes, 1024);
        assert_eq!(config.page_count_threshold, 5);
        assert_eq!(config.target_pages_per_chunk, 2);
        assert_eq!(config.max_pages_per_chunk, 5);
        assert_eq!(config.disk_processing_threshold_bytes, 10 * 1024);
        assert_eq!(config.min_chars_per_page, 10);
        assert_eq!(config.max_xlsx_sheet_count, 10);
        assert_eq!(config.max_xlsx_workbook_cells, 50_000);
        assert_eq!(config.max_pptx_embedded_count, 5);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_test_config_differs_from_default() {
        let test_config = HeuristicsConfig::test_config();
        let default_config = HeuristicsConfig::default();
        assert!(test_config.file_size_threshold_bytes < default_config.file_size_threshold_bytes);
        assert!(test_config.page_count_threshold < default_config.page_count_threshold);
        assert!(test_config.target_pages_per_chunk < default_config.target_pages_per_chunk);
        assert!(test_config.max_pages_per_chunk < default_config.max_pages_per_chunk);
        assert!(test_config.disk_processing_threshold_bytes < default_config.disk_processing_threshold_bytes);
    }
}
