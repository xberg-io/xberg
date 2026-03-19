//! PDF-specific configuration.
//!
//! Defines PDF extraction options including metadata handling, image extraction,
//! password management, and hierarchy extraction for document structure analysis.

use serde::{Deserialize, Serialize};

/// PDF-specific configuration.
#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    /// Extract images from PDF
    #[serde(default)]
    pub extract_images: bool,

    /// List of passwords to try when opening encrypted PDFs
    #[serde(default)]
    pub passwords: Option<Vec<String>>,

    /// Extract PDF metadata
    #[serde(default = "default_true")]
    pub extract_metadata: bool,

    /// Hierarchy extraction configuration (None = hierarchy extraction disabled)
    #[serde(default)]
    pub hierarchy: Option<HierarchyConfig>,

    /// Extract PDF annotations (text notes, highlights, links, stamps).
    /// Default: false
    #[serde(default)]
    pub extract_annotations: bool,

    /// Top margin fraction (0.0–1.0) of page height to exclude headers/running heads.
    /// Default: 0.06 (6%)
    #[serde(default)]
    pub top_margin_fraction: Option<f32>,

    /// Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers.
    /// Default: 0.05 (5%)
    #[serde(default)]
    pub bottom_margin_fraction: Option<f32>,

    /// Allow single-column pseudo tables in extraction results.
    ///
    /// By default, tables with fewer than 2 columns (layout-guided) or 3 columns
    /// (heuristic) are rejected. When `true`, the minimum column count is relaxed
    /// to 1, allowing single-column structured data (glossaries, itemized lists)
    /// to be emitted as tables. Other quality filters (density, sparsity, prose
    /// detection) still apply.
    #[serde(default)]
    pub allow_single_column_tables: bool,
}

/// Hierarchy extraction configuration for PDF text structure analysis.
///
/// Enables extraction of document hierarchy levels (H1-H6) based on font size
/// clustering and semantic analysis. When enabled, hierarchical blocks are
/// included in page content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyConfig {
    /// Enable hierarchy extraction
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Number of font size clusters to use for hierarchy levels (1-7)
    ///
    /// Default: 6, which provides H1-H6 heading levels with body text.
    /// Larger values create more fine-grained hierarchy levels.
    #[serde(default = "default_k_clusters")]
    pub k_clusters: usize,

    /// Include bounding box information in hierarchy blocks
    #[serde(default = "default_true")]
    pub include_bbox: bool,

    /// OCR coverage threshold for smart OCR triggering (0.0-1.0)
    ///
    /// Determines when OCR should be triggered based on text block coverage.
    /// OCR is triggered when text blocks cover less than this fraction of the page.
    /// Default: 0.5 (trigger OCR if less than 50% of page has text)
    #[serde(default = "default_ocr_coverage_threshold")]
    pub ocr_coverage_threshold: Option<f32>,
}

#[cfg(feature = "pdf")]
impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            extract_images: false,
            passwords: None,
            extract_metadata: true,
            hierarchy: None,
            extract_annotations: false,
            top_margin_fraction: None,
            bottom_margin_fraction: None,
            allow_single_column_tables: false,
        }
    }
}

impl Default for HierarchyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            k_clusters: 3,
            include_bbox: true,
            ocr_coverage_threshold: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_k_clusters() -> usize {
    3
}

fn default_ocr_coverage_threshold() -> Option<f32> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "pdf")]
    fn test_hierarchy_config_default() {
        use super::*;
        let config = HierarchyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.k_clusters, 3);
        assert!(config.include_bbox);
        assert!(config.ocr_coverage_threshold.is_none());
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_hierarchy_config_disabled() {
        use super::*;
        let config = HierarchyConfig {
            enabled: false,
            k_clusters: 3,
            include_bbox: false,
            ocr_coverage_threshold: Some(0.7),
        };
        assert!(!config.enabled);
        assert_eq!(config.k_clusters, 3);
        assert!(!config.include_bbox);
        assert_eq!(config.ocr_coverage_threshold, Some(0.7));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_config_custom_margins() {
        use super::*;
        let config = PdfConfig {
            extract_images: false,
            passwords: None,
            extract_metadata: true,
            hierarchy: None,
            extract_annotations: false,
            top_margin_fraction: Some(0.10),
            bottom_margin_fraction: Some(0.08),
            allow_single_column_tables: false,
        };
        assert_eq!(config.top_margin_fraction, Some(0.10));
        assert_eq!(config.bottom_margin_fraction, Some(0.08));
    }
}
